//! Manual D3D9 probe: a real 900x600 window presenting every frame, with the
//! overlay menu drawn on top, so the result can be screenshotted.

#![allow(dead_code)]

use std::sync::Mutex;
use std::time::Instant;

use nany::overlay::{Frame, Provider};
use windows::Win32::{
    Foundation::{BOOL, HWND},
    Graphics::Direct3D9::{
        D3DCLEAR_TARGET, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_A8R8G8B8, D3DMULTISAMPLE_NONE,
        D3DPRESENT_INTERVAL_IMMEDIATE, D3DPRESENT_PARAMETERS, D3DSWAPEFFECT_DISCARD, Direct3DCreate9, IDirect3DDevice9,
    },
};

#[path = "../tests/support/mod.rs"]
mod support;

/// `D3D_SDK_VERSION` from d3d9.h.
const D3D_SDK_VERSION: u32 = 32;

struct MenuProbe {
    state: Mutex<support::menu::State,>,
    t0:    Instant,
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
        if let Some((x, y,),) = f.pointer
        {
            support::menu::draw_cursor(f.ctx, egui::pos2(x, y,),);
        }
        true
    }
}

fn present_params(hwnd: HWND, w: u32, h: u32,) -> D3DPRESENT_PARAMETERS {
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

fn main() {
    unsafe {
        let win = match support::TestWindow::create("d3d9probe", 1280, 720,)
        {
            Some(w,) => w,
            None =>
            {
                eprintln!("window creation failed");
                return;
            }
        };
        let Some(d3d,) = Direct3DCreate9(D3D_SDK_VERSION,)
        else
        {
            eprintln!("Direct3DCreate9 failed");
            return;
        };
        let (w, h,) = win.client_size();
        let mut pp = present_params(HWND(win.hwnd,), w, h,);
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
            eprintln!("CreateDevice failed");
            return;
        }
        let Some(device,) = device
        else
        {
            eprintln!("no device");
            return;
        };

        nany::debug_panel::set_enabled(false);
        nany::init(
            "d3d9_probe",
            Box::new(MenuProbe { state: Mutex::new(support::menu::State::default(),), t0: Instant::now() },),
        );
        println!("log: {}", nany::log::path());

        let started = Instant::now();
        while started.elapsed().as_secs_f32() < 25.0
        {
            let t = started.elapsed().as_secs_f32();
            // a moving game-layer colour so it is obvious what the overlay added
            let r = (40.0 + 30.0 * t.sin()) as u32;
            let g = (60.0 + 30.0 * (t * 1.7).cos()) as u32;
            let b = (90.0 + 30.0 * (t * 2.3).sin()) as u32;
            let color = (r << 16) | (g << 8) | b;
            let _ = device.Clear(0, std::ptr::null(), D3DCLEAR_TARGET as u32, color, 1.0, 0,);
            let _ = device.BeginScene();
            let _ = device.EndScene();
            let _ = device.Present(std::ptr::null(), std::ptr::null(), None::<&HWND,>, std::ptr::null(),);
            win.pump(1,);
            std::thread::sleep(std::time::Duration::from_millis(16,),);
        }
    }
}
