use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

use windows::{
    Win32::Graphics::{
        Direct3D11::ID3D11Device, Direct3D11on12::ID3D11On12Device, Direct3D12::ID3D12CommandQueue,
        Dxgi::IDXGISwapChain,
    },
    core::Interface,
};

use crate::{
    input::Input,
    log, log_error, log_info,
    render::{self, Backend},
    win,
};

pub struct Frame<'a,> {
    pub ctx: &'a egui::Context,

    pub screen: (u32, u32,),

    pub backend: &'static str,

    pub frame_ms: f32,

    pub frames: u64,

    pub dt: f32,

    pub pointer: Option<(f32, f32,),>,
}

pub trait Provider: Send + Sync {
    unsafe fn init(&self, _ctx: &egui::Context,) {}

    unsafe fn ui(&self, frame: &Frame<'_,>,) -> bool;
}

impl Provider for () {
    unsafe fn ui(&self, _frame: &Frame<'_,>,) -> bool {
        false
    }
}

#[allow(clippy::large_enum_variant)]
enum Renderer {
    Dx11(render::d3d11::Painter11,),
    Dx12(render::d3d12::Painter12,),
    Gl(render::gl::PainterGL,),
    Vk(render::vk::PainterVK,),
    #[allow(dead_code)]
    None,
}

struct Overlay {
    ctx:             egui::Context,
    renderer:        Renderer,
    renderer_failed: bool,
    input:           Input,
    hwnd:            usize,
    frames:          u64,
    last_ms:         f32,
    last_frame_t:    Instant,

    start_t:  Instant,
    last_log: Instant,
    warned:   bool,

    logged_hybrid: bool,
    ever_rendered: bool,
    pending_tex:   Vec<egui::TexturesDelta,>,
    capture:       bool,

    fail_since: Option<Instant,>,

    last_ok_at: Instant,

    ok_uptime: f32,

    next_pass_at: Instant,

    last_prims: Option<Vec<egui::ClippedPrimitive,>,>,

    last_capture: bool,

    prims_size: (u32, u32,),

    pass_ms: f32,

    backend: Option<Backend,>,
}

unsafe fn apply_all(deltas: &mut [egui::TexturesDelta], mut apply: impl FnMut(&egui::TexturesDelta,),) {
    for d in deltas.iter_mut()
    {
        apply(d,);
        d.clear();
    }
}

static ENTRIES: Mutex<Vec<Entry,>,> = Mutex::new(Vec::new(),);

const PASS_DUTY_DEFAULT: f32 = 0.05;

const LOG_PERIOD_MS: u128 = 5000;

static SC_COUNT: AtomicUsize = AtomicUsize::new(0,);

const MAX_ENTRIES: usize = 8;
static CAPTURE: AtomicBool = AtomicBool::new(false,);
static PROVIDER: OnceLock<Box<dyn Provider,>,> = OnceLock::new();

static ENTRY_TICK: AtomicU64 = AtomicU64::new(1,);

struct Entry {
    key:       usize,
    ov:        Overlay,
    last_used: u64,
}

unsafe fn detect_backend(sc: &IDXGISwapChain,) -> Option<Backend,> {
    let dev11: Option<ID3D11Device,> = sc.GetDevice::<ID3D11Device>().ok();
    let has11 = dev11.is_some();
    let on12: Option<ID3D11On12Device,> = dev11.as_ref().and_then(|d| d.cast::<ID3D11On12Device>().ok(),);
    let has12 = sc.GetDevice::<windows::Win32::Graphics::Direct3D12::ID3D12Device>().is_ok();
    if has12
    {
        Some(Backend::D3D12,)
    }
    else if has11
    {
        Some(if on12.is_some() { Backend::D3D11On12 } else { Backend::D3D11 },)
    }
    else
    {
        None
    }
}
unsafe fn lock_entries() -> std::sync::MutexGuard<'static, Vec<Entry,>,> {
    match ENTRIES.lock()
    {
        Ok(g,) => g,
        Err(p,) => p.into_inner(),
    }
}

unsafe fn with_entry<R,>(key: usize, f: impl FnOnce(&mut Overlay,) -> R,) -> R {
    let tick = ENTRY_TICK.fetch_add(1, Ordering::Relaxed,);
    let mut g = lock_entries();
    let idx = match g.iter().position(|e| e.key == key,)
    {
        Some(i,) => i,
        None =>
        {
            if g.len() >= MAX_ENTRIES
            {
                if let Some((victim, _,),) = g.iter().enumerate().min_by_key(|(_, e,)| e.last_used,)
                {
                    g.remove(victim,);
                }
            }
            g.push(Entry { key, ov: Overlay::new(), last_used: tick, },);
            SC_COUNT.store(g.len(), Ordering::Relaxed,);
            g.len() - 1
        }
    };
    g[idx].last_used = tick;
    f(&mut g[idx].ov,)
}

unsafe fn with_existing<R,>(key: usize, f: impl FnOnce(&mut Overlay,) -> R,) -> Option<R,> {
    let tick = ENTRY_TICK.fetch_add(1, Ordering::Relaxed,);
    let mut g = lock_entries();
    let idx = g.iter().position(|e| e.key == key,)?;
    g[idx].last_used = tick;
    Some(f(&mut g[idx].ov,),)
}

pub unsafe fn now_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH,).map(|d| d.as_millis(),).unwrap_or(0,)
}

pub unsafe fn set_provider(p: Box<dyn Provider,>,) {
    let _ = PROVIDER.set(p,);
}

unsafe fn provider_ref() -> &'static dyn Provider {
    match PROVIDER.get()
    {
        Some(b,) => b.as_ref(),
        None => &(),
    }
}

pub unsafe fn input_captured() -> bool {
    CAPTURE.load(Ordering::Relaxed,)
}

impl Overlay {
    unsafe fn make_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        provider_ref().init(&ctx,);
        ctx
    }

    unsafe fn new() -> Self {
        if !log::is_ready()
        {
            log::init("nany",);
        }
        let ctx = Self::make_ctx();
        log_info!("ui", "egui context ready (provider init done)");

        Self {
            ctx,
            renderer: Renderer::None,
            renderer_failed: false,
            input: Input::default(),
            hwnd: 0,
            frames: 0,
            last_ms: 0.0,
            last_frame_t: Instant::now(),
            start_t: Instant::now(),
            last_log: Instant::now(),
            warned: false,
            logged_hybrid: false,
            ever_rendered: false,
            pending_tex: Vec::new(),
            capture: false,
            fail_since: None,
            last_ok_at: Instant::now(),
            ok_uptime: 0.0,
            next_pass_at: Instant::now(),
            last_prims: None,
            last_capture: false,
            prims_size: (0, 0,),
            pass_ms: 0.0,
            backend: None,
        }
    }

    unsafe fn before_rebuild(&mut self,) {
        if !self.ever_rendered
        {
            return;
        }
        log_info!("ui", "backend rebuilt, resetting egui context for full texture upload");
        self.ctx = Self::make_ctx();
        self.pending_tex.clear();
        // the old tessellation references texture ids of the old context
        self.last_prims = None;
    }

    unsafe fn stash_pending(&mut self, deltas: Vec<egui::TexturesDelta,>,) {
        for mut old in std::mem::take(&mut self.pending_tex,)
        {
            old.clear();
        }
        self.pending_tex = deltas;
        while self.pending_tex.len() > 32
        {
            let victim = self
                .pending_tex
                .iter()
                .position(|d| d.set.iter().all(|(_, v,)| v.iter().all(|i| i.pos.is_some(),),),)
                .unwrap_or(0,);
            let mut dropped = self.pending_tex.remove(victim,);
            dropped.clear();
        }
    }

    unsafe fn tick(&mut self,) -> (Instant, f32,) {
        let t0 = Instant::now();
        self.frames += 1;
        let dt = t0.saturating_duration_since(self.last_frame_t,).as_secs_f32();
        self.last_frame_t = t0;
        (t0, dt,)
    }

    unsafe fn note_ok(&mut self,) {
        let now = Instant::now();
        self.ok_uptime += now.saturating_duration_since(self.last_ok_at,).as_secs_f32();
        self.last_ok_at = now;
        self.fail_since = None;
    }

    unsafe fn note_fail(&mut self,) -> bool {
        let now = Instant::now();
        let since = *self.fail_since.get_or_insert(now,);
        let fail_secs = now.saturating_duration_since(since,).as_secs_f32();
        fail_secs > self.ok_uptime
    }

    unsafe fn ui_pass(&mut self, w: u32, h: u32, label: &'static str, dt: f32,) -> (Vec<egui::TexturesDelta,>, bool,) {
        let captured = self.capture;
        CAPTURE.store(captured, Ordering::Relaxed,);
        let events = self.input.poll(self.hwnd as win::Hwnd, (w, h,), captured,);
        let had_input = !events.is_empty();

        let must_pass = !self.ever_rendered
            || had_input
            || self.capture != self.last_capture
            || self.ctx.has_requested_repaint()
            || Instant::now() >= self.next_pass_at;

        let geometry_same = self.prims_size == (w, h,);
        let can_reuse = !must_pass && geometry_same && self.last_prims.is_some();

        let mut full: Option<egui::FullOutput,> = None;
        if !can_reuse
        {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w as f32, h as f32,),),),
                time: Some(self.start_t.elapsed().as_secs_f64(),),
                events,
                ..Default::default()
            };
            let frame_ctx = Frame {
                ctx: &self.ctx,
                screen: (w, h,),
                backend: label,
                frame_ms: self.last_ms,
                frames: self.frames,
                dt,
                pointer: self.input.cursor().map(|p| (p.x, p.y,),),
            };
            self.ctx.begin_pass(raw,);
            let pass_t0 = Instant::now();
            let want_capture = provider_ref().ui(&frame_ctx,);
            maybe_debug_panel(&self.ctx, label, (w, h,), self.last_ms, self.frames, want_capture,);
            let out = self.ctx.end_pass();

            for cmd in &out.platform_output.commands
            {
                if let egui::OutputCommand::CopyText(text,) = cmd
                {
                    win::set_clipboard_text(text,);
                }
            }

            let pass_ms = pass_t0.elapsed().as_secs_f32() * 1000.0;
            self.pass_ms = pass_ms;
            self.capture = want_capture;
            self.last_capture = want_capture;

            let duty = PASS_DUTY_DEFAULT;
            let cadence_ms = if duty > 0.0 { self.pass_ms / duty } else { 0.0 };
            self.next_pass_at = Instant::now() + std::time::Duration::from_secs_f32(cadence_ms / 1000.0,);
            full = Some(out,);
        }

        let mut tex_deltas = std::mem::take(&mut self.pending_tex,);
        if let Some(out,) = full
        {
            tex_deltas.push(out.textures_delta,);

            let prims = self.ctx.tessellate(out.shapes, out.pixels_per_point,);
            self.last_prims = Some(prims,);
            self.prims_size = (w, h,);
        }
        (tex_deltas, captured,)
    }

    unsafe fn frame(&mut self, sc_raw: *mut c_void,) {
        let sc: ManuallyDrop<IDXGISwapChain,> = ManuallyDrop::new(IDXGISwapChain::from_raw(sc_raw,),);

        let (t0, dt,) = self.tick();

        let mut input_hwnd = self.hwnd;
        if let Ok(sc1,) = sc.cast::<windows::Win32::Graphics::Dxgi::IDXGISwapChain1>()
        {
            if let Ok(h,) = sc1.GetHwnd()
            {
                if !h.0.is_null()
                {
                    input_hwnd = h.0 as usize;
                }
            }
        }
        if input_hwnd != 0 && input_hwnd != self.hwnd
        {
            self.hwnd = input_hwnd;
            log_info!("win", format!("swapchain window {input_hwnd:#x}"));
            crate::hook::wndproc::install(input_hwnd as win::Hwnd,);
        }
        else if self.hwnd == 0
        {
            let h = win::find_main_window();
            if !h.is_null()
            {
                self.hwnd = h as usize;
                log_info!("win", format!("fallback window '{}'", win::window_title(h)));
                crate::hook::wndproc::install(h,);
            }
        }

        let backend = match self.backend
        {
            Some(b,) => b,
            None =>
            {
                let Some(b,) = detect_backend(&sc,)
                else
                {
                    if !self.warned
                    {
                        log_info!("frame", "unknown swapchain device, skip");
                        self.warned = true;
                    }
                    return;
                };
                self.backend = Some(b,);
                log_info!("backend", format!("detected {} (once per swapchain)", b.label()));
                b
            }
        };

        if backend == Backend::D3D11On12 && !self.logged_hybrid
        {
            log_info!(
                "backend",
                "d3d11on12 interop backend: back buffer handled as wrapped resource (acquire/release + flush)"
            );
            self.logged_hybrid = true;
        }

        let (w, h,) = match backend
        {
            Backend::D3D11 => match sc.GetBuffer::<windows::Win32::Graphics::Direct3D11::ID3D11Texture2D>(0,)
            {
                Ok(bb,) =>
                {
                    let mut d = windows::Win32::Graphics::Direct3D11::D3D11_TEXTURE2D_DESC::default();
                    bb.GetDesc(&mut d,);
                    (d.Width.max(1,), d.Height.max(1,),)
                }
                Err(_,) => (0, 0,),
            },
            _ => match sc.GetDesc()
            {
                Ok(d,) => (d.BufferDesc.Width.max(1,), d.BufferDesc.Height.max(1,),),
                Err(_,) => (0, 0,),
            },
        };

        if w < 8 || h < 8
        {
            return;
        }

        let (mut tex_deltas, captured,) = self.ui_pass(w, h, backend.label(), dt,);

        if backend.is_d3d11() && !self.renderer_failed
        {
            let dev11: Option<ID3D11Device,> = sc.GetDevice::<ID3D11Device>().ok();
            let on12: Option<ID3D11On12Device,> = dev11.as_ref().and_then(|d| d.cast::<ID3D11On12Device>().ok(),);
            let need = match &self.renderer
            {
                Renderer::Dx11(p,) => Some(p.device_raw,) != dev11.as_ref().map(|d| d.as_raw() as usize,),
                _ => true,
            };
            if need
            {
                self.before_rebuild();
                if let Some(dev,) = dev11.as_ref().cloned()
                {
                    if let Ok(ctx,) = dev.GetImmediateContext()
                    {
                        match render::d3d11::Painter11::new(&dev, &ctx,)
                        {
                            Ok(mut p,) =>
                            {
                                p.set_on12(on12.clone(),);
                                log_info!(
                                    "d3d11",
                                    if on12.is_some()
                                    {
                                        "painter created (11on12 wrapped resource)"
                                    }
                                    else
                                    {
                                        "painter created"
                                    }
                                );
                                self.renderer = Renderer::Dx11(p,);
                                self.ever_rendered = true;
                            }
                            Err(e,) =>
                            {
                                log_error!("d3d11", format!("painter failed: {e}"));
                                self.renderer_failed = true;
                            }
                        }
                    }
                }
            }
        }

        if backend == Backend::D3D12 && !self.renderer_failed
        {
            let cur_q = crate::hook::dxgi::queue().unwrap_or(0,);
            let need = match &self.renderer
            {
                Renderer::Dx12(p,) => p.queue_raw() != cur_q,
                _ => true,
            };
            if need && cur_q != 0
            {
                self.before_rebuild();
                let queue: ManuallyDrop<ID3D12CommandQueue,> =
                    ManuallyDrop::new(ID3D12CommandQueue::from_raw(cur_q as *mut c_void,),);
                let mut dev: Option<windows::Win32::Graphics::Direct3D12::ID3D12Device,> = None;
                let _ = queue.GetDevice(&mut dev,);
                if let Some(dev,) = dev
                {
                    let format = sc
                        .GetDesc()
                        .map(|d| d.BufferDesc.Format,)
                        .unwrap_or(windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,);
                    match render::d3d12::Painter12::new(&dev, &queue, format,)
                    {
                        Ok(p,) =>
                        {
                            log_info!("d3d12", "painter created");
                            self.renderer = Renderer::Dx12(p,);
                            self.ever_rendered = true;
                        }
                        Err(e,) =>
                        {
                            log_error!("d3d12", format!("painter failed: {e}"));
                            self.renderer_failed = true;
                        }
                    }
                }
            }
        }

        let prims: &[egui::ClippedPrimitive] = match &self.last_prims
        {
            Some(p,) => p,
            None => &[],
        };

        let mut consumed = false;

        match &mut self.renderer
        {
            Renderer::Dx11(p,) if backend.is_d3d11() =>
            {
                p.ensure_target(&sc, w, h,);
                apply_all(&mut tex_deltas, |d| p.apply_textures(d,),);
                p.paint(prims, (w, h,),);
                consumed = true;
            }
            Renderer::Dx12(p,) if backend == Backend::D3D12 =>
            {
                p.ensure_target(&sc, w, h,);
                apply_all(&mut tex_deltas, |d| p.apply_textures(d,),);
                p.paint(&sc, prims, (w, h,),);
                consumed = true;
            }
            _ =>
            {}
        }
        if !consumed
        {
            self.stash_pending(tex_deltas,);
        }

        self.finish_frame(t0, backend, w, h, captured,);
    }

    unsafe fn finish_frame(&mut self, t0: Instant, backend: Backend, w: u32, h: u32, captured: bool,) {
        self.last_ms = t0.elapsed().as_secs_f32() * 1000.0;
        if self.last_log.elapsed().as_millis() < LOG_PERIOD_MS
        {
            return;
        }
        self.last_log = Instant::now();

        let tex = match &self.renderer
        {
            Renderer::Dx11(p,) => p.tex_count(),
            Renderer::Dx12(p,) => p.tex_count(),
            Renderer::Gl(p,) => p.tex_count(),
            Renderer::Vk(p,) => p.tex_count(),
            Renderer::None => 0,
        };
        log_info!(
            "frame",
            format!("{} {}x{} capture={} tex={} ms={:.2}", backend.label(), w, h, captured as u8, tex, self.last_ms)
        );
    }

    unsafe fn frame_gl(&mut self, hwnd: usize, w: u32, h: u32,) {
        let (t0, dt,) = self.tick();

        if hwnd != 0 && hwnd != self.hwnd
        {
            self.hwnd = hwnd;
            log_info!("win", format!("gl window {hwnd:#x}"));
            crate::hook::wndproc::install(hwnd as win::Hwnd,);
        }
        else if self.hwnd == 0
        {
            let h = win::find_main_window();
            if !h.is_null()
            {
                self.hwnd = h as usize;
                log_info!("win", format!("fallback window '{}'", win::window_title(h)));
                crate::hook::wndproc::install(h,);
            }
        }

        let (mut tex_deltas, captured,) = self.ui_pass(w, h, Backend::OpenGL.label(), dt,);

        if !matches!(self.renderer, Renderer::Gl(_)) && !self.renderer_failed
        {
            self.before_rebuild();
            match render::gl::PainterGL::new()
            {
                Some(p,) =>
                {
                    self.renderer = Renderer::Gl(p,);
                    self.ever_rendered = true;
                    log_info!("gl", "painter created");
                }
                None =>
                {
                    log_error!("gl", "painter creation failed: incomplete GL entry points or no current context");
                    self.renderer_failed = true;
                }
            }
        }

        let prims: &[egui::ClippedPrimitive] = match &self.last_prims
        {
            Some(p,) => p,
            None => &[],
        };
        let mut consumed = false;
        if let Renderer::Gl(p,) = &mut self.renderer
        {
            apply_all(&mut tex_deltas, |d| p.apply_textures(d,),);
            p.paint(prims, (w, h,),);
            consumed = true;
        }
        if !consumed
        {
            self.stash_pending(tex_deltas,);
        }

        self.finish_frame(t0, Backend::OpenGL, w, h, captured,);
    }

    unsafe fn frame_vk(&mut self, queue: u64, swapchain: u64, image_index: u32,) -> Option<u64,> {
        let info = crate::hook::vk::swapchain_info(swapchain,)?;
        let (w, h,) = (info.width, info.height,);

        if w < 8 || h < 8
        {
            return None;
        }
        let hwnd = if self.hwnd == 0
        {
            let h = win::find_main_window();
            if !h.is_null()
            {
                self.hwnd = h as usize;
                crate::hook::wndproc::install(h,);
            }
            self.hwnd
        }
        else
        {
            self.hwnd
        };
        let _ = hwnd;

        let (_t0, dt,) = self.tick();
        let (mut tex_deltas, captured,) = self.ui_pass(w, h, Backend::Vulkan.label(), dt,);

        if !matches!(self.renderer, Renderer::Vk(_)) && !self.renderer_failed
        {
            self.before_rebuild();
            match render::vk::PainterVK::new(swapchain,)
            {
                Some(p,) =>
                {
                    self.renderer = Renderer::Vk(p,);
                    self.ever_rendered = true;
                    log_info!("vk", "painter created");
                }
                None =>
                {
                    log_error!("vk", "painter creation failed: device or swapchain info missing");
                    self.renderer_failed = true;
                }
            }
        }

        let prims: &[egui::ClippedPrimitive] = match &self.last_prims
        {
            Some(p,) => p,
            None => &[],
        };
        let mut sem = None;
        let mut consumed = false;
        if let Renderer::Vk(p,) = &mut self.renderer
        {
            apply_all(&mut tex_deltas, |d| p.apply_textures(d,),);
            sem = p.paint(swapchain, queue, image_index, prims, (w, h,),);
            consumed = sem.is_some();
        }
        if !consumed
        {
            self.stash_pending(tex_deltas,);
        }

        self.finish_frame(_t0, Backend::Vulkan, w, h, captured,);
        sem
    }
}

pub unsafe fn on_vk_present(queue: u64, swapchain: u64, image_index: u32,) -> Option<u64,> {
    let key = match crate::hook::vk::swapchain_hwnd(swapchain,)
    {
        Some(h,) if h != 0 => h | (2usize << 47),
        _ => (swapchain as usize) | (2usize << 47),
    };
    if log::enabled() && self::ENTRY_TICK.load(Ordering::Relaxed,) % 60 == 0
    {
        log_info!("vk", format!("[d] key={key:#x} entries={}", swapchain_count()));
    }
    let mut sem = None;
    with_entry(key, |st| {
        sem = st.frame_vk(queue, swapchain, image_index,);
        st.note_ok();
    },);
    sem
}

pub unsafe fn on_gl_present(hdc: *mut c_void,) {
    let hwnd = win::window_from_dc(hdc,);
    let Some((w, h,),) = win::client_size_cached(hwnd,).or_else(|| {
        let s = win::client_size(hwnd,);
        if let Some((cw, ch,),) = s
        {
            win::set_client_size(hwnd, cw, ch,);
        }
        s
    },)
    else
    {
        return;
    };
    if w <= 0 || h <= 0
    {
        return;
    }

    let key = (hdc as usize) | (1usize << 47);
    with_entry(key, |st| {
        st.frame_gl(hwnd as usize, w as u32, h as u32,);
        st.note_ok();
    },);
}

pub unsafe fn on_present(sc_raw: *mut c_void,) {
    if sc_raw.is_null()
    {
        return;
    }
    let key = sc_raw as usize;
    with_entry(key, |st| {
        st.frame(sc_raw,);
        st.note_ok();
    },);
}

pub unsafe fn on_resize(sc_raw: *mut c_void,) {
    with_existing(sc_raw as usize, |st| match &mut st.renderer
    {
        Renderer::Dx11(p,) =>
        {
            p.release_target();
            log_info!("resize", "dx11 rtv released before ResizeBuffers");
        }
        Renderer::Dx12(p,) =>
        {
            p.on_resize_begin();
            log_info!("resize", "dx12 waited for gpu, rtv cache cleared");
        }
        _ =>
        {}
    },);
}

pub unsafe fn on_device_lost(sc_raw: *mut c_void,) {
    with_existing(sc_raw as usize, |st| {
        if matches!(st.renderer, Renderer::None)
        {
            return;
        }
        log_info!("device", "lost/reset, dropping backend");
        st.renderer = Renderer::None;
        st.renderer_failed = false;
        st.pending_tex.clear();
        st.last_prims = None;
        st.ctx = Overlay::make_ctx();
        if st.note_fail()
        {
            log_error!("device", "repeated device losses, circuit breaker tripped and hooks unloaded");
            crate::hook::dxgi::unhook_all();
        }
    },);
}

pub unsafe fn ensure_wndproc() {
    let hwnd = win::find_main_window();
    if !hwnd.is_null()
    {
        crate::hook::wndproc::install(hwnd,);
    }
}

pub unsafe fn swapchain_count() -> usize {
    SC_COUNT.load(Ordering::Relaxed,)
}

unsafe fn maybe_debug_panel(
    ctx: &egui::Context,
    backend: &'static str,
    screen: (u32, u32,),
    frame_ms: f32,
    frames: u64,
    capture: bool,
) {
    if !crate::debug_panel::enabled()
    {
        return;
    }
    let info = crate::debug_panel::Info {
        backend,
        width: screen.0,
        height: screen.1,
        frame_ms,
        frames,
        capture,
        swapchains: swapchain_count(),
        hooks_ok: crate::hook::dxgi::installed(),
        node_mask: crate::hook::dxgi::node_mask(),
    };
    crate::debug_panel::draw(ctx, &info,);
}
