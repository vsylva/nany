#![allow(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::UpdateWindow,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWL_STYLE, GetClientRect,
            GetSystemMetrics, HMENU, HWND_NOTOPMOST, HWND_TOP, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SM_CXSCREEN,
            SM_CYSCREEN, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
            SET_WINDOW_POS_FLAGS, SetWindowPos, ShowWindow, TranslateMessage, UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE,
            WNDCLASSW, WNDCLASS_STYLES, WS_OVERLAPPEDWINDOW, WS_POPUP,
        },
    },
    core::{HSTRING, PCWSTR},
};

#[cfg(target_arch = "x86_64")]
use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW};
#[cfg(target_arch = "x86")]
use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, SetWindowLongW};

pub mod esp;
pub mod menu;

use nany::overlay::{Frame, Provider};

pub type Hwnd = *mut std::ffi::c_void;

#[cfg(target_arch = "x86_64")]
unsafe fn style_get(hwnd: HWND,) -> WINDOW_STYLE {
    WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE,) as u32,)
}
#[cfg(target_arch = "x86")]
unsafe fn style_get(hwnd: HWND,) -> WINDOW_STYLE {
    WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE,) as u32,)
}

#[cfg(target_arch = "x86_64")]
unsafe fn style_set(hwnd: HWND, style: WINDOW_STYLE,) {
    SetWindowLongPtrW(hwnd, GWL_STYLE, style.0 as isize,);
}
#[cfg(target_arch = "x86")]
unsafe fn style_set(hwnd: HWND, style: WINDOW_STYLE,) {
    SetWindowLongW(hwnd, GWL_STYLE, style.0 as i32,);
}

/// WNDCLASSW::lpfnWndProc is a C callback, so the class needs a real
/// extern "system" function; everything inside it is a direct windows-rs call.
unsafe extern "system" fn default_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam,)
}

fn message_pump() {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE,).as_bool()
        {
            let _ = TranslateMessage(&msg,);
            DispatchMessageW(&msg,);
        }
    }
}

pub struct TestWindow {
    pub hwnd:   Hwnd,
    class:      HSTRING,
    pub width:  u32,
    pub height: u32,
}

impl TestWindow {
    pub unsafe fn create(tag: &str, width: u32, height: u32,) -> Option<Self,> {
        let class = HSTRING::from(format!("NANY_TEST_{tag}"),);
        let title = HSTRING::from(format!("nany test - {tag}"),);
        let inst = GetModuleHandleW(PCWSTR::null(),).ok()?;

        let wc = WNDCLASSW {
            style:         WNDCLASS_STYLES(0,),
            lpfnWndProc:   Some(default_wnd_proc,),
            hInstance:     HINSTANCE(inst.0,),
            lpszClassName: PCWSTR(class.as_ptr(),),
            ..Default::default()
        };
        RegisterClassW(&wc,);

        let mut rc = RECT { left: 0, top: 0, right: width as i32, bottom: height as i32, };
        AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, false,).ok()?;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0,),
            PCWSTR(class.as_ptr(),),
            PCWSTR(title.as_ptr(),),
            WS_OVERLAPPEDWINDOW,
            60,
            60,
            rc.right - rc.left,
            rc.bottom - rc.top,
            HWND::default(),
            HMENU::default(),
            HINSTANCE(inst.0,),
            None,
        )
        .ok()?;
        let _ = ShowWindow(hwnd, SW_SHOW,);
        let _ = UpdateWindow(hwnd,);
        Some(Self { hwnd: hwnd.0, class, width, height, },)
    }

    pub unsafe fn client_size(&self,) -> (u32, u32,) {
        let mut rc = RECT::default();
        if GetClientRect(HWND(self.hwnd,), &mut rc,).is_err()
        {
            return (self.width.max(1,), self.height.max(1,),);
        }
        (((rc.right - rc.left).max(1,)) as u32, ((rc.bottom - rc.top).max(1,)) as u32,)
    }

    pub unsafe fn resize(&mut self, w: u32, h: u32,) {
        self.width = w.max(1,);
        self.height = h.max(1,);
        let mut rc = RECT { left: 0, top: 0, right: self.width as i32, bottom: self.height as i32, };
        let _ = AdjustWindowRect(&mut rc, style_get(HWND(self.hwnd,),), false,);
        let _ = SetWindowPos(
            HWND(self.hwnd,),
            HWND_TOP,
            60,
            60,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_NOZORDER.0,),
        );
        message_pump();
    }

    pub unsafe fn set_style(&mut self, style: WINDOW_STYLE,) {
        style_set(HWND(self.hwnd,), style,);
        let _ = SetWindowPos(
            HWND(self.hwnd,),
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0 | SWP_FRAMECHANGED.0,),
        );
        message_pump();
    }

    pub unsafe fn borderless_fullscreen(&mut self,) {
        let w = GetSystemMetrics(SM_CXSCREEN,).max(640,) as u32;
        let h = GetSystemMetrics(SM_CYSCREEN,).max(480,) as u32;
        style_set(HWND(self.hwnd,), WS_POPUP,);
        let _ = SetWindowPos(
            HWND(self.hwnd,),
            HWND_TOP,
            0,
            0,
            w as i32,
            h as i32,
            SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_FRAMECHANGED.0 | SWP_SHOWWINDOW.0,),
        );
        self.width = w;
        self.height = h;
        message_pump();
    }

    pub unsafe fn windowed(&mut self, w: u32, h: u32,) {
        style_set(HWND(self.hwnd,), WS_OVERLAPPEDWINDOW,);
        self.width = w;
        self.height = h;
        let mut rc = RECT { left: 0, top: 0, right: w as i32, bottom: h as i32, };
        let _ = AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, false,);
        let _ = SetWindowPos(
            HWND(self.hwnd,),
            HWND_TOP,
            60,
            60,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0 | SWP_FRAMECHANGED.0 | SWP_SHOWWINDOW.0,),
        );
        message_pump();
    }

    pub unsafe fn minimize(&mut self,) {
        let _ = ShowWindow(HWND(self.hwnd,), SW_MINIMIZE,);
    }

    pub unsafe fn restore(&mut self,) {
        let _ = ShowWindow(HWND(self.hwnd,), SW_RESTORE,);
    }

    pub unsafe fn pump(&self, rounds: usize,) {
        for _ in 0..rounds
        {
            message_pump();
        }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self,) {
        unsafe {
            let _ = DestroyWindow(HWND(self.hwnd,),);
            if let Ok(inst,) = GetModuleHandleW(PCWSTR::null(),)
            {
                let _ = UnregisterClassW(PCWSTR(self.class.as_ptr(),), HINSTANCE(inst.0,),);
            }
        }
    }
}

pub struct Counter {
    frames: AtomicU64,
    shapes: AtomicU64,
}

impl Counter {
    pub unsafe fn new() -> Arc<Self,> {
        Arc::new(Self { frames: AtomicU64::new(0,), shapes: AtomicU64::new(0,), },)
    }

    pub unsafe fn frames(&self,) -> u64 {
        self.frames.load(Ordering::Relaxed,)
    }

    pub unsafe fn reset(&self,) {
        self.shapes.store(0, Ordering::Relaxed,);
    }

    pub unsafe fn draws(&self,) -> u64 {
        self.shapes.load(Ordering::Relaxed,)
    }
}

pub struct CountProvider {
    counter: Arc<Counter,>,
}

impl CountProvider {
    pub unsafe fn new(counter: Arc<Counter,>,) -> Self {
        Self { counter, }
    }
}

impl Provider for CountProvider {
    unsafe fn init(&self, ctx: &egui::Context,) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\msyh.ttc",);
        ctx.set_visuals(egui::Visuals::dark(),);
    }

    unsafe fn ui(&self, f: &Frame<'_,>,) -> bool {
        self.counter.frames.fetch_add(1, Ordering::Relaxed,);
        let (w, h,) = (f.screen.0 as f32, f.screen.1 as f32,);
        let p = f.ctx.layer_painter(egui::LayerId::background(),);
        p.rect_filled(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h,),),
            0.0,
            egui::Color32::from_rgb(18, 22, 30,),
        );
        for i in 0..4
        {
            let x = 40.0 + i as f32 * 60.0;
            let r = egui::Rect::from_min_size(egui::pos2(x, 40.0,), egui::vec2(48.0, 48.0,),);
            p.rect_filled(r, 6.0, egui::Color32::from_rgb(91, 140, 255,),);
        }
        self.counter.shapes.fetch_add(1, Ordering::Relaxed,);
        false
    }
}

pub struct Step {
    pub name:  &'static str,
    pub apply: fn(&mut TestWindow,),
}

pub unsafe fn window_steps() -> Vec<Step,> {
    vec![
        Step { name: "windowed 1280x720", apply: |w| w.windowed(1280, 720,), },
        Step { name: "resize 1600x900", apply: |w| w.resize(1600, 900,), },
        Step { name: "resize 800x450", apply: |w| w.resize(800, 450,), },
        Step { name: "borderless fullscreen", apply: |w| w.borderless_fullscreen(), },
        Step { name: "back to windowed 1280x720", apply: |w| w.windowed(1280, 720,), },
        Step { name: "minimize", apply: |w| w.minimize(), },
        Step { name: "restore", apply: |w| w.restore(), },
        Step { name: "resolution 1920x1080", apply: |w| w.resize(1920, 1080,), },
        Step { name: "back to 1280x720", apply: |w| w.resize(1280, 720,), },
    ]
}

pub unsafe fn report(backend: &str, step: &str, before: u64, after: u64,) {
    println!("[{backend}] {step:<26} frames {before} -> {after} (+{})", after.saturating_sub(before));
}

pub struct SceneProvider {
    counter: Arc<Counter,>,
    state:   std::sync::Mutex<menu::State,>,
    targets: std::sync::Mutex<esp::Esp,>,
    last:    std::sync::Mutex<u128,>,
    t0:      std::time::Instant,
}

impl SceneProvider {
    pub unsafe fn new(counter: Arc<Counter,>,) -> Self {
        Self {
            counter,
            state: std::sync::Mutex::new(menu::State::default(),),
            targets: std::sync::Mutex::new(esp::Esp::default(),),
            last: std::sync::Mutex::new(0,),
            t0: std::time::Instant::now(),
        }
    }
}

impl Provider for SceneProvider {
    unsafe fn init(&self, ctx: &egui::Context,) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\msyh.ttc",);
        ctx.all_styles_mut(|s| {
            s.visuals = egui::Visuals::dark();
            s.visuals.window_fill = menu::Theme::BG;
            s.visuals.panel_fill = menu::Theme::BG;
            s.visuals.extreme_bg_color = menu::Theme::INPUT_BG;
            s.visuals.faint_bg_color = menu::Theme::CARD;
            s.visuals.override_text_color = Some(menu::Theme::TEXT,);
            s.visuals.widgets.inactive.bg_fill = menu::Theme::INPUT_BG;
            s.visuals.widgets.hovered.bg_fill = menu::Theme::CARD_HI;
            s.visuals.widgets.active.bg_fill = menu::Theme::ACCENT;
        },);
    }

    unsafe fn ui(&self, f: &Frame<'_,>,) -> bool {
        let now = nany::overlay::now_ms();
        let dt = {
            let mut last = match self.last.lock()
            {
                Ok(g,) => g,
                Err(p,) => p.into_inner(),
            };
            let dt = if *last == 0 { 0.016 } else { (now.saturating_sub(*last,)) as f32 / 1000.0 };
            *last = now;
            dt.clamp(0.001, 0.1,)
        };

        {
            let mut esp = match self.targets.lock()
            {
                Ok(g,) => g,
                Err(p,) => p.into_inner(),
            };
            esp.animate(dt, f.screen,);
            esp.draw(f.ctx, f.screen,);
        }

        let mut st = match self.state.lock()
        {
            Ok(g,) => g,
            Err(p,) => p.into_inner(),
        };
        if st.open
        {
            let info = menu::Info {
                backend:  f.backend,
                screen:   f.screen,
                frame_ms: f.frame_ms,
                frames:   f.frames,
                hotkey:   0x2D,
                t:        self.t0.elapsed().as_secs_f32(),
            };
            menu::draw(f.ctx, &mut st, &info,);
            if let Some((x, y,),) = f.pointer
            {
                menu::draw_cursor(f.ctx, egui::pos2(x, y,),);
            }
        }
        drop(st,);
        self.counter.shapes.fetch_add(1, Ordering::Relaxed,);
        self.counter.frames.fetch_add(1, Ordering::Relaxed,);
        false
    }
}
