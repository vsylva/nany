#![allow(unsafe_op_in_unsafe_fn)]
//! Debug: run the D3D9 painter directly and dump the back buffer to a BMP, so
//! the exact geometry it produced can be inspected.

#![allow(dead_code)]

use std::{sync::Mutex, time::Instant};

use nany::overlay::{Frame, Provider};
use windows::core::Interface;
use windows::Win32::{
    Foundation::{BOOL, HWND},
    Graphics::Direct3D9::{
        D3DBACKBUFFER_TYPE_MONO, D3DCLEAR_TARGET, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_A8R8G8B8,
        D3DLOCK_READONLY, D3DLOCKED_RECT, D3DMULTISAMPLE_NONE, D3DPOOL_SYSTEMMEM, D3DPRESENT_INTERVAL_IMMEDIATE,
        D3DPRESENT_PARAMETERS, D3DSWAPEFFECT_DISCARD, Direct3DCreate9, IDirect3DDevice9, IDirect3DSurface9,
    },
};

#[path = "../tests/support/mod.rs"]
mod support;

const D3D_SDK_VERSION: u32 = 32;

struct MenuProbe {
    state: Mutex<support::menu::State,>,
    t0: Instant,
}

impl Provider for MenuProbe {
    unsafe fn init(&self, ctx: &egui::Context,) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\msyh.ttc",);
        ctx.all_styles_mut(|s| {
            s.visuals = egui::Visuals::dark();
            s.visuals.window_fill = support::menu::Theme::BG;
            s.visuals.panel_fill = support::menu::Theme::BG;
            s.visuals.extreme_bg_color = support::menu::Theme::INPUT_BG;
            s.visuals.faint_bg_color = support::menu::Theme::CARD;
            s.visuals.override_text_color = Some(support::menu::Theme::TEXT,);
        },);
    }

    unsafe fn ui(&self, f: &Frame<'_,>,) -> bool {
        let mut state = match self.state.lock()
        {
            Ok(g,) => g,
            Err(p,) => p.into_inner(),
        };
        let info = support::menu::Info {
            backend:  f.backend,
            screen:   f.screen,
            frame_ms: f.frame_ms,
            frames:   f.frames,
            hotkey:   0x2D,
            t:        self.t0.elapsed().as_secs_f32(),
        };
        support::menu::draw(f.ctx, &mut state, &info,);
        true
    }
}
fn params(hwnd: HWND, w: u32, h: u32,) -> D3DPRESENT_PARAMETERS {
    D3DPRESENT_PARAMETERS {
        BackBufferWidth: w,
        BackBufferHeight: h,
        BackBufferFormat: D3DFMT_A8R8G8B8,
        BackBufferCount: 1,
        MultiSampleType: D3DMULTISAMPLE_NONE,
        SwapEffect: D3DSWAPEFFECT_DISCARD,
        hDeviceWindow: hwnd,
        Windowed: BOOL(1,),
        PresentationInterval: D3DPRESENT_INTERVAL_IMMEDIATE as u32,
        ..Default::default()
    }
}

/// 32-bit top-down BGRA BMP.
unsafe fn write_bmp(path: &str, w: u32, h: u32, pixels: *const u8, pitch: usize,) {
    let row = (w * 4) as usize;
    let mut out = Vec::with_capacity(54 + row * h as usize);
    out.extend_from_slice(b"BM",);
    let size = 54 + row * h as usize;
    out.extend_from_slice(&(size as u32).to_le_bytes(),);
    out.extend_from_slice(&0u32.to_le_bytes(),);
    out.extend_from_slice(&54u32.to_le_bytes(),);
    out.extend_from_slice(&40u32.to_le_bytes(),);
    out.extend_from_slice(&(w as i32).to_le_bytes(),);
    out.extend_from_slice(&(-(h as i32)).to_le_bytes(),);
    out.extend_from_slice(&1u16.to_le_bytes(),);
    out.extend_from_slice(&32u16.to_le_bytes(),);
    out.extend_from_slice(&0u32.to_le_bytes(),);
    out.extend_from_slice(&((row * h as usize) as u32).to_le_bytes(),);
    out.extend_from_slice(&2835i32.to_le_bytes(),);
    out.extend_from_slice(&2835i32.to_le_bytes(),);
    out.extend_from_slice(&0u32.to_le_bytes(),);
    out.extend_from_slice(&0u32.to_le_bytes(),);
    for y in 0..h as usize
    {
        let src = pixels.add(y * pitch,);
        out.extend_from_slice(std::slice::from_raw_parts(src, row,),);
    }
    let _ = std::fs::write(path, out,);
    println!("wrote {path}");
}

unsafe fn dump(device: &IDirect3DDevice9, w: u32, h: u32, path: &str,) {
    let mut surface: Option<IDirect3DSurface9,> = None;
    if device
        .CreateOffscreenPlainSurface(w, h, D3DFMT_A8R8G8B8, D3DPOOL_SYSTEMMEM, &mut surface, std::ptr::null_mut(),)
        .is_err()
    {
        println!("offscreen surface failed");
        return;
    }
    let Some(surface,) = surface
    else
    {
        return;
    };
    let Ok(backbuffer,) = device.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO,)
    else
    {
        return;
    };
    if device.GetRenderTargetData(&backbuffer, &surface,).is_err()
    {
        println!("GetRenderTargetData failed");
        return;
    }
    let mut locked = D3DLOCKED_RECT::default();
    if surface.LockRect(&mut locked, std::ptr::null(), D3DLOCK_READONLY as u32,).is_err()
    {
        return;
    }
    if !locked.pBits.is_null()
    {
        write_bmp(path, w, h, locked.pBits as *const u8, locked.Pitch as usize,);
    }
    let _ = surface.UnlockRect();
}

fn main() {
    unsafe {
        let Some(win,) = support::TestWindow::create("d3d9dump", 1280, 720,)
        else
        {
            println!("window failed");
            return;
        };
        let Some(d3d,) = Direct3DCreate9(D3D_SDK_VERSION,)
        else
        {
            println!("Direct3DCreate9 failed");
            return;
        };
        let (w, h,) = win.client_size();
        let mut pp = params(HWND(win.hwnd,), w, h,);
        let mut device: Option<IDirect3DDevice9,> = None;
        if d3d
            .CreateDevice(
                0,
                D3DDEVTYPE_HAL,
                HWND(win.hwnd,),
                D3DCREATE_SOFTWARE_VERTEXPROCESSING as u32,
                &mut pp as *mut _,
                &mut device,
            )
            .is_err()
        {
            println!("CreateDevice failed");
            return;
        }
        let Some(device,) = device
        else
        {
            println!("no device");
            return;
        };

        nany::debug_panel::set_enabled(false);
        nany::init("d3d9_dump", Box::new(MenuProbe { state: Mutex::new(support::menu::State::default(),), t0: Instant::now() },),);
        println!("screen {w}x{h}");

        // let the overlay build its painter through the normal Present hook
        for _ in 0..4
        {
            let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, 0x00FF_00FF, 1.0, 0,);
            let _ = device.BeginScene();
            let _ = device.EndScene();
            let _ = device.Present(std::ptr::null(), std::ptr::null(), None::<&HWND,>, std::ptr::null(),);
            win.pump(1,);
            std::thread::sleep(std::time::Duration::from_millis(20,),);
        }

        // now draw and dump before any Present can discard the buffer
        let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, 0x00FF_00FF, 1.0, 0,);
        let _ = device.BeginScene();
        let _ = device.EndScene();
        nany::overlay::on_present_d3d9(device.as_raw(),);
        dump(&device, w, h, "target\\d3d9-backbuffer.bmp",);
    }
}
