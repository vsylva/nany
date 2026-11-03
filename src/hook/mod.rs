use crate::{log_error, log_info};

pub mod detour;
pub mod dxgi;
pub mod gl;
pub mod vk;
pub mod wndproc;

static INSTALLED: std::sync::Mutex<Vec<usize,>,> = std::sync::Mutex::new(Vec::new(),);

pub unsafe fn uninstall_all() -> usize {
    let targets: Vec<usize,> = match INSTALLED.lock()
    {
        Ok(mut g,) => std::mem::take(&mut *g,),
        Err(p,) => std::mem::take(&mut *p.into_inner(),),
    };
    let n = targets.len();
    for t in targets
    {
        detour::unhook(t,);
    }
    wndproc::uninstall();
    log_info!("hook", format!("uninstalled {n} hook(s)"));
    n
}
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) unsafe fn install_one(
    tag: &str,
    name: &str,
    target: usize,
    detour_fn: usize,
    slot: &AtomicUsize,
    targets: &std::sync::Mutex<Vec<usize,>,>,
) -> bool {
    if target == 0
    {
        log_error!(tag, format!("{name}: address unavailable, skipped"));
        return false;
    }
    match detour::hook(target, detour_fn,)
    {
        Some(tramp,) =>
        {
            slot.store(tramp, Ordering::Release,);
            if let Ok(mut g,) = targets.lock()
            {
                g.push(target,);
            }
            if let Ok(mut g,) = INSTALLED.lock()
            {
                g.push(target,);
            }
            log_info!(tag, format!("{name} @0x{target:X} inline hooked"));
            true
        }
        None =>
        {
            log_error!(tag, format!("{name}: inline hook failed"));
            false
        }
    }
}
