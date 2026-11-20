#![allow(non_snake_case)]

//! D3D9 backend test: the real overlay menu must land in the back buffer.
//!
//! The test draws a frame, reads the back buffer back *before* Present (after a
//! discard-swap the content is undefined), checks the menu's dark theme and its
//! accent colour, and writes the frame to `target/d3d9-test.bmp` so the result
//! can be looked at.

mod support;

use std::sync::Mutex;

use nany::overlay::{Frame, Provider};
use windows::{
    Win32::{
        Foundation::{BOOL, HWND},
        Graphics::Direct3D9::{
            D3DBACKBUFFER_TYPE_MONO, D3DCLEAR_TARGET, D3DCREATE_HARDWARE_VERTEXPROCESSING,
            D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_A8R8G8B8, D3DLOCK_READONLY, D3DLOCKED_RECT,
            D3DMULTISAMPLE_NONE, D3DPOOL_SYSTEMMEM, D3DPRESENT_INTERVAL_ONE, D3DPRESENT_PARAMETERS,
            D3DSWAPEFFECT_DISCARD, Direct3DCreate9, IDirect3DDevice9, IDirect3DSurface9,
        },
    },
    core::Interface,
};

use support::menu;

/// `D3D_SDK_VERSION` from d3d9.h.
const D3D_SDK_VERSION: u32 = 32;
/// What every frame clears to; the overlay never uses this colour.
const CLEAR: u32 = 0x0010_1020;

struct MenuProbe {
    state:  Mutex<menu::State>,
    t0:     std::time::Instant,
    frames: std::sync::atomic::AtomicU64,
}

impl MenuProbe {
    fn frames(&self) -> u64 {
        self.frames.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Provider for MenuProbe {
    unsafe fn init(&self, ctx: &egui::Context) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\msyh.ttc");
        ctx.all_styles_mut(|s| {
            s.visuals = egui::Visuals::dark();
            s.visuals.window_fill = menu::Theme::BG;
            s.visuals.panel_fill = menu::Theme::BG;
            s.visuals.extreme_bg_color = menu::Theme::INPUT_BG;
            s.visuals.faint_bg_color = menu::Theme::CARD;
            s.visuals.override_text_color = Some(menu::Theme::TEXT);
        });
    }

    unsafe fn ui(&self, f: &Frame<'_>) -> bool {
        self.frames.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let info = menu::Info {
            backend:  f.backend,
            screen:   f.screen,
            frame_ms: f.frame_ms,
            frames:   f.frames,
            hotkey:   0x2D,
            t:        self.t0.elapsed().as_secs_f32(),
        };
        menu::draw(f.ctx, &mut state, &info);
        true
    }
}

fn present_params(hwnd: HWND, w: u32, h: u32) -> D3DPRESENT_PARAMETERS {
    D3DPRESENT_PARAMETERS {
        BackBufferWidth: w,
        BackBufferHeight: h,
        BackBufferFormat: D3DFMT_A8R8G8B8,
        BackBufferCount: 1,
        MultiSampleType: D3DMULTISAMPLE_NONE,
        SwapEffect: D3DSWAPEFFECT_DISCARD,
        hDeviceWindow: hwnd,
        Windowed: BOOL(1),
        // vsync, like the other backend tests: roughly one frame per 16 ms
        PresentationInterval: D3DPRESENT_INTERVAL_ONE as u32,
        ..Default::default()
    }
}

unsafe fn make_device(win: &support::TestWindow) -> Option<IDirect3DDevice9> {
    let d3d = Direct3DCreate9(D3D_SDK_VERSION)?;
    let (w, h) = win.client_size();
    let mut pp = present_params(HWND(win.hwnd), w, h);
    let mut device: Option<IDirect3DDevice9> = None;
    for flags in [D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DCREATE_SOFTWARE_VERTEXPROCESSING] {
        if d3d.CreateDevice(0, D3DDEVTYPE_HAL, HWND(win.hwnd), flags as u32, &mut pp as *mut _, &mut device).is_ok()
            && device.is_some()
        {
            return device;
        }
        device = None;
    }
    None
}

unsafe fn present(device: &IDirect3DDevice9) {
    let _ = device.Present(std::ptr::null(), std::ptr::null(), None::<&HWND>, std::ptr::null());
}

/// One game frame: clear, open and close the scene, then present.
unsafe fn game_frame(device: &IDirect3DDevice9) {
    let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, CLEAR, 1.0, 0);
    let _ = device.BeginScene();
    let _ = device.EndScene();
    present(device);
}

struct Shot {
    w:      u32,
    h:      u32,
    pixels: Vec<u8>,
}

/// Read the back buffer (BGRA, top-down).
unsafe fn read_back(device: &IDirect3DDevice9, w: u32, h: u32) -> Option<Shot> {
    let mut surface: Option<IDirect3DSurface9> = None;
    device
        .CreateOffscreenPlainSurface(w, h, D3DFMT_A8R8G8B8, D3DPOOL_SYSTEMMEM, &mut surface, std::ptr::null_mut())
        .ok()?;
    let surface = surface?;
    let backbuffer = device.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO).ok()?;
    device.GetRenderTargetData(&backbuffer, &surface).ok()?;

    let mut locked = D3DLOCKED_RECT::default();
    surface.LockRect(&mut locked, std::ptr::null(), D3DLOCK_READONLY as u32).ok()?;
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    if !locked.pBits.is_null() {
        let pitch = locked.Pitch as usize;
        for y in 0..h as usize {
            let row = std::slice::from_raw_parts((locked.pBits as *const u8).add(y * pitch), (w * 4) as usize);
            pixels.extend_from_slice(row);
        }
    }
    let _ = surface.UnlockRect();
    Some(Shot { w, h, pixels })
}

impl Shot {
    fn at(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let i = ((y * self.w + x) * 4) as usize;
        (self.pixels[i + 2], self.pixels[i + 1], self.pixels[i])
    }

    /// Pixels that differ from the clear colour, i.e. anything the overlay added.
    fn painted(&self) -> u32 {
        // (r, g, b): the clear value is 0x00RRGGBB
        let clear = ((CLEAR >> 16) as u8, (CLEAR >> 8) as u8, CLEAR as u8);
        let mut n = 0;
        for y in 0..self.h {
            for x in 0..self.w {
                let p = self.at(x, y);
                let diff = p.0.abs_diff(clear.0) as u32 + p.1.abs_diff(clear.1) as u32 + p.2.abs_diff(clear.2) as u32;
                if diff > 24 {
                    n += 1;
                }
            }
        }
        n
    }

    fn count(&self, f: impl Fn((u8, u8, u8)) -> bool) -> u32 {
        self.count_rect(0, 0, self.w, self.h, f)
    }

    fn count_rect(&self, x0: u32, y0: u32, x1: u32, y1: u32, f: impl Fn((u8, u8, u8)) -> bool) -> u32 {
        let mut n = 0;
        for y in y0..y1.min(self.h) {
            for x in x0..x1.min(self.w) {
                if f(self.at(x, y)) {
                    n += 1;
                }
            }
        }
        n
    }

    /// Minimal 32-bit top-down BMP so the frame can be opened in any viewer.
    fn save_bmp(&self, path: &str) {
        let row = (self.w * 4) as usize;
        let mut out = Vec::with_capacity(54 + row * self.h as usize);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&((54 + row * self.h as usize) as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(self.w as i32).to_le_bytes());
        out.extend_from_slice(&(-(self.h as i32)).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&((row * self.h as usize) as u32).to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&self.pixels);
        let _ = std::fs::write(path, out);
    }
}

/// Size of the device's real back buffer.
unsafe fn backbuffer_size(device: &IDirect3DDevice9) -> Option<(u32, u32)> {
    let backbuffer = device.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO).ok()?;
    let mut desc = windows::Win32::Graphics::Direct3D9::D3DSURFACE_DESC::default();
    backbuffer.GetDesc(&mut desc).ok()?;
    Some((desc.Width, desc.Height))
}

/// Tests in this binary share the process-wide overlay registry (the menu test
/// installs the hooks), so they run one after another.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// A 32x32 atlas with a one-texel white cross at (9, 9), plus the quad that
/// draws it 1:1 into the top-left corner of the back buffer.
unsafe fn cross_painter(
    device: &IDirect3DDevice9,
) -> (nany::render::d3d9::Painter9, [egui::ClippedPrimitive; 1]) {
    let mut painter = nany::render::d3d9::Painter9::new(device).expect("d3d9 painter");

    let mut pixels = vec![egui::Color32::TRANSPARENT; 32 * 32];
    for i in 0..32 {
        pixels[9 * 32 + i] = egui::Color32::WHITE;
        pixels[i * 32 + 9] = egui::Color32::WHITE;
    }
    let atlas = egui::ColorImage::new([32, 32], pixels);

    let mut deltas = egui::TexturesDelta::default();
    deltas.set.entry(egui::TextureId::Managed(0)).or_default().push(egui::epaint::ImageDelta::full(
        egui::ImageData::Color(std::sync::Arc::new(atlas)),
        egui::TextureOptions::LINEAR,
    ));
    painter.apply_textures(&deltas);
    deltas.clear();

    let mut mesh = egui::Mesh { texture_id: egui::TextureId::Managed(0), ..Default::default() };
    mesh.add_rect_with_uv(
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(32.0, 32.0)),
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let prims = [egui::ClippedPrimitive {
        clip_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 240.0)),
        primitive: egui::epaint::Primitive::Mesh(mesh),
    }];
    (painter, prims)
}

/// Clear, draw and read the cross back: (row through y=20, column through x=20).
unsafe fn cross_pixels(
    painter: &mut nany::render::d3d9::Painter9,
    device: &IDirect3DDevice9,
    prims: &[egui::ClippedPrimitive],
) -> (Vec<u8>, Vec<u8>) {
    let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, 0, 1.0, 0);
    painter.paint(prims, (320, 240));
    let shot = read_back(device, 320, 240).expect("back buffer readback");
    let row: Vec<u8> = (0..32).map(|x| shot.at(x, 20).0).collect();
    let col: Vec<u8> = (0..32).map(|y| shot.at(20, y).0).collect();
    (row, col)
}

fn assert_cross(row: &[u8], col: &[u8], what: &str) {
    println!("[d3d9] {what} row y=20 {row:?}");
    println!("[d3d9] {what} col x=20 {col:?}");
    assert!(row[9] > 200, "{what}: the atlas column did not land on x=9: {row:?}");
    assert!(col[9] > 200, "{what}: the atlas row did not land on y=9: {col:?}");
    let bleed_row: u32 = row.iter().enumerate().filter(|(i, _)| *i != 9).map(|(_, v)| *v as u32).sum();
    let bleed_col: u32 = col.iter().enumerate().filter(|(i, _)| *i != 9).map(|(_, v)| *v as u32).sum();
    assert!(bleed_row < 24, "{what}: the atlas column is sampled between texels: {row:?}");
    assert!(bleed_col < 24, "{what}: the atlas row is sampled between texels: {col:?}");
}

/// The painter has to map mesh pixels onto back-buffer pixels 1:1. When the
/// atlas is sampled half a texel off, every glyph edge is spread over two
/// pixels, which is exactly the "d3d9 text looks soft" report: a one-texel
/// cross in the atlas must land on one pixel, not on a 2x2 blend.
#[test]
fn d3d9_samples_the_atlas_texel_exact() {
    let _serial = serial();
    unsafe {
        let win = match support::TestWindow::create("d3d9atlas", 320, 240) {
            Some(w) => w,
            None => panic!("window creation failed"),
        };
        let device = make_device(&win).expect("D3D9 device");
        let (mut painter, prims) = cross_painter(&device);
        let (row, col) = cross_pixels(&mut painter, &device, &prims);
        assert_cross(&row, &col, "first paint");
    }
}

/// A `Reset` (alt-tab back into a fullscreen game, a resolution change, a device
/// loss the game recovered from) invalidates every device object of the painter.
/// The atlas has to come back from the CPU copy: egui handed its texture deltas
/// over once and will not send them again, so a painter that waits for a new
/// delta draws white blocks instead of text - or, if the rebuild never happens,
/// nothing at all.
#[test]
fn d3d9_rebuild_restores_the_atlas_after_reset() {
    let _serial = serial();
    unsafe {
        let win = match support::TestWindow::create("d3d9rebuild", 320, 240) {
            Some(w) => w,
            None => panic!("window creation failed"),
        };
        let device = make_device(&win).expect("D3D9 device");
        let (mut painter, prims) = cross_painter(&device);
        let (row, col) = cross_pixels(&mut painter, &device, &prims);
        assert_cross(&row, &col, "before reset");

        // exactly what the Reset hook does around the original Reset call
        painter.release_target();
        assert!(!painter.ready(), "the painter must be stale after release_target");
        let (row, _col) = cross_pixels(&mut painter, &device, &prims);
        assert!(row.iter().all(|v| *v == 0), "a stale painter must not draw: {row:?}");

        assert!(painter.rebuild(), "the painter refused to rebuild");
        assert!(painter.ready());
        let (row, col) = cross_pixels(&mut painter, &device, &prims);
        assert_cross(&row, &col, "after rebuild");
    }
}

#[test]
fn d3d9_renders_the_menu() {
    let _serial = serial();
    unsafe {
        let mut win = match support::TestWindow::create("d3d9", 1280, 720) {
            Some(w) => w,
            None => panic!("window creation failed"),
        };
        let device = make_device(&win).expect("D3D9 device");

        nany::debug_panel::set_enabled(false);
        let probe = std::sync::Arc::new(MenuProbe {
            state:  Mutex::new(menu::State::default()),
            t0:     std::time::Instant::now(),
            frames: std::sync::atomic::AtomicU64::new(0),
        });
        struct Shared(std::sync::Arc<MenuProbe>);
        impl Provider for Shared {
            unsafe fn init(&self, ctx: &egui::Context) {
                <MenuProbe as Provider>::init(&self.0, ctx);
            }
            unsafe fn ui(&self, f: &Frame<'_>) -> bool {
                <MenuProbe as Provider>::ui(&self.0, f);
                true
            }
        }
        nany::init("test_d3d9", Box::new(Shared(probe.clone())));

        // warm up: lets the overlay build its painter through the Present hook
        for _ in 0..20 {
            game_frame(&device);
            win.pump(1);
            std::thread::sleep(std::time::Duration::from_millis(4));
        }

        // paint one frame and read it back before Present discards it
        let (w, h) = win.client_size();
        let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, CLEAR, 1.0, 0);
        let _ = device.BeginScene();
        let _ = device.EndScene();
        nany::overlay::on_present_d3d9(device.as_raw());

        let shot = read_back(&device, w, h).expect("back buffer readback");
        let painted = shot.painted();
        // the menu occupies pos (80, 80) with size 892x376; sample its interior
        let dark = shot.count_rect(90, 90, 960, 450, |c| c.0 < 40 && c.1 < 45 && c.2 < 60);
        let accent = shot.count(|c| c.2 > 190 && c.0 < 150 && c.1 > 80);
        // labels and widget text: light grey, never pure white
        let text = shot.count_rect(90, 90, 960, 450, |c| c.0 > 170 && c.1 > 170 && c.2 > 170);
        let white = shot.count(|c| c.0 > 240 && c.1 > 240 && c.2 > 240);
        shot.save_bmp("target\\d3d9-test.bmp");
        present(&device);

        println!("[d3d9] {w}x{h} painted={painted} dark={dark} text={text} accent={accent} white={white} -> target\\d3d9-test.bmp");
        println!("[d3d9] menu corner sample (200,200) = {:?}", shot.at(200, 200));

        let inside = shot.at(200, 200);
        let outside = shot.at(1240, 660);
        assert!(
            inside.0 < 40 && inside.1 < 45 && inside.2 < 60,
            "the menu interior is not the dark theme: {inside:?}"
        );
        assert!(
            (outside.0 as i32 - 16).abs() <= 6 && (outside.2 as i32 - 32).abs() <= 8,
            "the menu covered the whole screen instead of its own rect: {outside:?}"
        );
        assert!(dark > 100_000, "the menu frame is missing its dark theme (dark {dark} pixels)");
        assert!(text > 500, "no text/glyph pixels inside the menu (text {text})");
        // the unnormalised vertex-colour bug turned every colour white
        assert!(white * 10 < dark, "most of the menu came out white (white {white}, dark {dark})");

        // window transitions and Reset: same cadence as the other backend tests
        // (24 presents per step at vsync, so the phase is visible on screen)
        let mut last_size = (1280u32, 720u32);
        for step in support::window_steps() {
            win.pump(8);
            (step.apply)(&mut win);

            // a minimized or mid-transition window can report a degenerate
            // client rect; keep the last sane size for the Reset
            let (cw, ch) = win.client_size();
            if cw >= 64 && ch >= 64 {
                last_size = (cw, ch);
            }
            let (rw, rh) = last_size;
            let mut pp = present_params(HWND(win.hwnd), rw, rh);
            let reset = device.Reset(&mut pp as *mut _);

            let before = probe.frames();
            for _ in 0..24 {
                game_frame(&device);
                win.pump(1);
                std::thread::sleep(std::time::Duration::from_millis(4));
            }
            let after = probe.frames();
            support::report("d3d9", step.name, before, after);
            assert!(after >= before, "the overlay frame counter went backwards on step {}", step.name);

            // what the device really holds, which is what the readback can copy
            let (bw, bh) = backbuffer_size(&device).unwrap_or((0, 0));
            if reset.is_err() || bw < 960 || bh < 480 {
                println!("[d3d9] {:<26} {bw}x{bh} overlay check skipped", step.name);
                continue;
            }

            let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, CLEAR, 1.0, 0);
            let _ = device.BeginScene();
            let _ = device.EndScene();
            nany::overlay::on_present_d3d9(device.as_raw());
            let shot = read_back(&device, bw, bh).expect("back buffer readback");
            present(&device);
            let dark = shot.count_rect(90, 90, 960, 450, |c| c.0 < 40 && c.1 < 45 && c.2 < 60);
            println!(
                "[d3d9] {:<26} {bw}x{bh} menu pixels {dark} {}",
                step.name,
                if dark > 100_000 { "ok" } else { "MISSING" }
            );
            assert!(dark > 100_000, "step {} lost the menu (dark {dark} pixels)", step.name);
        }

        let (w, h) = win.client_size();
        let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, CLEAR, 1.0, 0);
        let _ = device.BeginScene();
        let _ = device.EndScene();
        nany::overlay::on_present_d3d9(device.as_raw());
        let shot = read_back(&device, w, h).expect("back buffer readback");
        present(&device);
        shot.save_bmp("target\\d3d9-test-after-resize.bmp");
        let painted = shot.painted();
        let dark = shot.count_rect(90, 90, 960, 450, |c| c.0 < 40 && c.1 < 45 && c.2 < 60);
        println!("[d3d9] after transitions dark={dark} -> target\\d3d9-test-after-resize.bmp");
        let _ = painted;
        assert!(dark > 100_000, "the menu stopped painting after the transition run (dark {dark})");
    }
}
