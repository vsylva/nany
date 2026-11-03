#![allow(dead_code, deprecated)]

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub mod esp;
pub mod menu;

use nany::overlay::{Frame, Provider};

pub type Hwnd = *mut std::ffi::c_void;

pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const SW_SHOW: i32 = 5;
pub const SW_MINIMIZE: i32 = 6;
pub const SW_RESTORE: i32 = 9;
pub const HWND_TOP: isize = 0;
pub const HWND_NOTOPMOST: isize = -2;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_FRAMECHANGED: u32 = 0x0020;
pub const SWP_SHOWWINDOW: u32 = 0x0040;
pub const GWL_STYLE: i32 = -16;

#[repr(C)]
pub struct Rect {
    pub left:   i32,
    pub top:    i32,
    pub right:  i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct Msg {
    pub hwnd:    Hwnd,
    pub message: u32,
    pub wparam:  usize,
    pub lparam:  isize,
    pub time:    u32,
    pub pt_x:    i32,
    pub pt_y:    i32,
}

#[repr(C)]
pub struct WndClassW {
    pub style:           u32,
    pub lpfn_wnd_proc:   Option<unsafe extern "system" fn(Hwnd, u32, usize, isize,) -> isize,>,
    pub cb_cls_extra:    i32,
    pub cb_wnd_extra:    i32,
    pub h_instance:      isize,
    pub h_icon:          isize,
    pub h_cursor:        isize,
    pub hbr_background:  isize,
    pub lpsz_menu_name:  *const u8,
    pub lpsz_class_name: *const u8,
}

unsafe extern "system" {
    pub fn GetModuleHandleW(name: *const u16,) -> isize;
    pub fn RegisterClassA(cls: *const WndClassW,) -> u16;
    pub fn CreateWindowExA(
        ex_style: u32,
        class_name: *const u8,
        window_name: *const u8,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: Hwnd,
        menu: isize,
        instance: isize,
        param: *mut std::ffi::c_void,
    ) -> Hwnd;
    pub fn DefWindowProcA(h: Hwnd, m: u32, w: usize, l: isize,) -> isize;
    pub fn ShowWindow(h: Hwnd, cmd: i32,) -> i32;
    pub fn UpdateWindow(h: Hwnd,) -> i32;
    pub fn DestroyWindow(h: Hwnd,) -> i32;
    pub fn AdjustWindowRect(r: *mut Rect, style: u32, menu: i32,) -> i32;
    pub fn GetClientRect(h: Hwnd, r: *mut Rect,) -> i32;
    pub fn SetWindowPos(h: Hwnd, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32,) -> i32;
    pub fn SetWindowLongPtrA(h: Hwnd, index: i32, value: isize,) -> isize;
    pub fn GetWindowLongPtrA(h: Hwnd, index: i32,) -> isize;
    pub fn PeekMessageW(m: *mut Msg, h: Hwnd, min: u32, max: u32, remove: u32,) -> i32;
    pub fn TranslateMessage(m: *const Msg,) -> i32;
    pub fn DispatchMessageW(m: *const Msg,) -> isize;
    pub fn GetSystemMetrics(index: i32,) -> i32;
    pub fn LoadLibraryA(name: *const u8,) -> isize;
    pub fn GetProcAddress(module: isize, name: *const u8,) -> *const std::ffi::c_void;
}

pub const PM_REMOVE: u32 = 1;

pub unsafe extern "system" fn wnd_proc(h: Hwnd, m: u32, w: usize, l: isize,) -> isize {
    DefWindowProcA(h, m, w, l,)
}

pub struct TestWindow {
    pub hwnd:   Hwnd,
    class:      std::ffi::CString,
    pub width:  u32,
    pub height: u32,
}

impl TestWindow {
    pub unsafe fn create(tag: &str, width: u32, height: u32,) -> Option<Self,> {
        let class = std::ffi::CString::new(format!("NANY_TEST_{tag}"),).ok()?;
        let title = std::ffi::CString::new(format!("nany test - {tag}"),).ok()?;
        let inst = GetModuleHandleW(std::ptr::null(),);
        let wc = WndClassW {
            style:           0,
            lpfn_wnd_proc:   Some(wnd_proc,),
            cb_cls_extra:    0,
            cb_wnd_extra:    0,
            h_instance:      inst,
            h_icon:          0,
            h_cursor:        0,
            hbr_background:  0,
            lpsz_menu_name:  std::ptr::null(),
            lpsz_class_name: class.as_ptr() as *const u8,
        };
        RegisterClassA(&wc,);
        let mut rc = Rect { left: 0, top: 0, right: width as i32, bottom: height as i32, };
        AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, 0,);
        let hwnd = CreateWindowExA(
            0,
            class.as_ptr() as *const u8,
            title.as_ptr() as *const u8,
            WS_OVERLAPPEDWINDOW,
            60,
            60,
            rc.right - rc.left,
            rc.bottom - rc.top,
            std::ptr::null_mut(),
            0,
            inst,
            std::ptr::null_mut(),
        );
        if hwnd.is_null()
        {
            return None;
        }
        ShowWindow(hwnd, SW_SHOW,);
        UpdateWindow(hwnd,);
        Some(Self { hwnd, class, width, height, },)
    }

    pub unsafe fn client_size(&self,) -> (u32, u32,) {
        let mut rc = Rect { left: 0, top: 0, right: 0, bottom: 0, };
        if GetClientRect(self.hwnd, &mut rc,) == 0
        {
            return (self.width.max(1,), self.height.max(1,),);
        }
        (((rc.right - rc.left).max(1,)) as u32, ((rc.bottom - rc.top).max(1,)) as u32,)
    }

    pub unsafe fn resize(&mut self, w: u32, h: u32,) {
        self.width = w.max(1,);
        self.height = h.max(1,);
        let mut rc = Rect { left: 0, top: 0, right: self.width as i32, bottom: self.height as i32, };
        let style = GetWindowLongPtrA(self.hwnd, GWL_STYLE,) as u32;
        AdjustWindowRect(&mut rc, style, 0,);
        SetWindowPos(
            self.hwnd,
            HWND_TOP,
            60,
            60,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        self.pump(4,);
    }

    pub unsafe fn set_style(&mut self, style: u32,) {
        SetWindowLongPtrA(self.hwnd, GWL_STYLE, style as isize,);
        SetWindowPos(self.hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,);
        self.pump(2,);
    }

    pub unsafe fn borderless_fullscreen(&mut self,) {
        let w = GetSystemMetrics(0,).max(640,) as u32;
        let h = GetSystemMetrics(1,).max(480,) as u32;
        SetWindowLongPtrA(self.hwnd, GWL_STYLE, WS_POPUP as isize,);
        SetWindowPos(self.hwnd, HWND_TOP, 0, 0, w as i32, h as i32, SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,);
        self.width = w;
        self.height = h;
        self.pump(4,);
    }

    pub unsafe fn windowed(&mut self, w: u32, h: u32,) {
        SetWindowLongPtrA(self.hwnd, GWL_STYLE, WS_OVERLAPPEDWINDOW as isize,);
        self.width = w;
        self.height = h;
        let mut rc = Rect { left: 0, top: 0, right: w as i32, bottom: h as i32, };
        AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, 0,);
        SetWindowPos(
            self.hwnd,
            HWND_TOP,
            60,
            60,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        );
        self.pump(4,);
    }

    pub unsafe fn minimize(&mut self,) {
        ShowWindow(self.hwnd, SW_MINIMIZE,);
    }

    pub unsafe fn restore(&mut self,) {
        ShowWindow(self.hwnd, SW_RESTORE,);
    }

    pub unsafe fn pump(&self, rounds: usize,) {
        let mut msg = std::mem::zeroed::<Msg,>();
        for _ in 0..rounds
        {
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE,) != 0
            {
                TranslateMessage(&msg,);
                DispatchMessageW(&msg,);
            }
        }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self,) {
        unsafe {
            DestroyWindow(self.hwnd,);
            let _ = &self.class;
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
