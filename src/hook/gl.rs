use std::{
    cell::Cell,
    ffi::c_void,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::detour;
use crate::{log_error, log_info, log_warn, overlay, win};

static INSTALLED: AtomicBool = AtomicBool::new(false,);
static ORIG_WGL_SWAP: AtomicUsize = AtomicUsize::new(0,);
static ORIG_GDI_SWAP: AtomicUsize = AtomicUsize::new(0,);
static TARGETS: std::sync::Mutex<Vec<usize,>,> = std::sync::Mutex::new(Vec::new(),);

type SwapFn = unsafe extern "system" fn(*mut c_void,) -> i32;

thread_local! {

    static IN_PRESENT: Cell<bool> = const { Cell::new(false) };
}

pub unsafe fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire,)
}

static SWAPS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0,);
static SWAP_T0: std::sync::OnceLock<std::time::Instant,> = std::sync::OnceLock::new();

static SWAP_MARK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0,);

unsafe fn note_swap() {
    SWAPS.fetch_add(1, Ordering::Relaxed,);
    let t0 = SWAP_T0.get_or_init(std::time::Instant::now,);
    let now = t0.elapsed().as_millis() as u64;
    let last = SWAP_MARK.load(Ordering::Relaxed,);
    let span = now.saturating_sub(last,);
    if span >= 2000
    {
        let n = SWAPS.swap(0, Ordering::Relaxed,);
        SWAP_MARK.store(now, Ordering::Relaxed,);
        log_info!("gl", format!("swaps/s = {:.1} ({n} swaps / {span} ms)", n as f64 * 1000.0 / span as f64));
    }
}
unsafe fn present(hdc: *mut c_void,) {
    if hdc.is_null()
    {
        return;
    }
    note_swap();
    overlay::on_gl_present(hdc,);
}

unsafe fn swap(orig_slot: &AtomicUsize, hdc: *mut c_void,) -> i32 {
    let first = !IN_PRESENT.with(|f| f.replace(true,),);
    if first
    {
        present(hdc,);
    }
    let out = {
        let orig = orig_slot.load(Ordering::Acquire,);
        if orig == 0
        {
            1
        }
        else
        {
            let f: SwapFn = std::mem::transmute(orig,);
            f(hdc,)
        }
    };
    if first
    {
        IN_PRESENT.with(|f| f.set(false,),);
    }
    out
}

unsafe extern "system" fn detour_wgl_swap(hdc: *mut c_void,) -> i32 {
    swap(&ORIG_WGL_SWAP, hdc,)
}

unsafe extern "system" fn detour_gdi_swap(hdc: *mut c_void,) -> i32 {
    swap(&ORIG_GDI_SWAP, hdc,)
}

unsafe fn hook_one(name: &str, target: usize, detour_fn: usize, slot: &AtomicUsize,) -> bool {
    if target == 0
    {
        log_error!("gl", format!("{name}: address unavailable, skipped"));
        return false;
    }
    match detour::hook(target, detour_fn,)
    {
        Some(tramp,) =>
        {
            slot.store(tramp, Ordering::Release,);
            if let Ok(mut g,) = TARGETS.lock()
            {
                g.push(target,);
            }
            log_info!("gl", format!("{name} @0x{target:X} inline hooked"));
            true
        }
        None =>
        {
            log_error!("gl", format!("{name} @0x{target:X} inline hook failed"));
            false
        }
    }
}

pub unsafe fn install() -> bool {
    if INSTALLED.swap(true, Ordering::SeqCst,)
    {
        return true;
    }
    let gl = win::module_base_bytes(b"opengl32.dll\0",);
    if gl == 0
    {
        INSTALLED.store(false, Ordering::Release,);
        return false;
    }
    let mut ok = false;
    {
        let p = win::proc_address(gl as isize, b"wglSwapBuffers\0",) as usize;
        ok |= hook_one("wglSwapBuffers", p, detour_wgl_swap as *const () as usize, &ORIG_WGL_SWAP,);

        let gdi = win::module_base_bytes(b"gdi32.dll\0",);
        if gdi != 0
        {
            let p = win::proc_address(gdi as isize, b"SwapBuffers\0",) as usize;
            ok |= hook_one("SwapBuffers", p, detour_gdi_swap as *const () as usize, &ORIG_GDI_SWAP,);
        }
    }
    log_info!("gl", format!("hooks installed = {ok}"));
    ok
}

pub unsafe fn ensure() -> bool {
    installed() || install()
}

pub unsafe fn unhook_all() {
    if !installed()
    {
        return;
    }
    let targets: Vec<usize,> = match TARGETS.lock()
    {
        Ok(g,) => g.clone(),
        Err(p,) => p.into_inner().clone(),
    };
    for t in &targets
    {
        detour::unhook(*t,);
    }
    ORIG_WGL_SWAP.store(0, Ordering::Release,);
    ORIG_GDI_SWAP.store(0, Ordering::Release,);
    INSTALLED.store(false, Ordering::Release,);
    log_warn!("gl", format!("circuit breaker: unloaded {} inline hook(s)", targets.len()));
}
