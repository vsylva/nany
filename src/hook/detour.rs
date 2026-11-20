//! Inline detours, backed by `naly`, with a registry so a hook can be removed
//! by its target address.

use std::ffi::c_void;
use std::sync::Mutex;

use crate::{log_error, log_info};

/// Live hooks keyed by target address. The `Hook` owns the trampoline and
/// removes the patch when dropped.
static HOOKS: Mutex<Vec<(usize, naly::Hook,)>,> = Mutex::new(Vec::new(),);

unsafe fn registry() -> std::sync::MutexGuard<'static, Vec<(usize, naly::Hook,),>,> {
    match HOOKS.lock()
    {
        Ok(g,) => g,
        Err(p,) => p.into_inner(),
    }
}

/// Patch `target` so calls land in `detour`; returns the trampoline address.
pub unsafe fn hook(target: usize, detour: usize,) -> Option<usize,> {
    if target == 0 || detour == 0
    {
        log_error!("detour", "hook with a null address, skipped");
        return None;
    }
    match naly::Hook::attach(target as *mut c_void, detour as *mut c_void,)
    {
        Ok(h,) =>
        {
            let tramp = h.trampoline() as usize;
            let stolen = h.stolen_len();
            registry().push((target, h,),);
            log_info!("detour", format!("{target:#x} -> {detour:#x} trampoline {tramp:#x} ({stolen} bytes)"));
            Some(tramp,)
        }
        Err(e,) =>
        {
            log_error!("detour", format!("{target:#x}: {e}"));
            None
        }
    }
}

/// Remove the hook installed on `target`, if any.
pub unsafe fn unhook(target: usize,) {
    if target == 0
    {
        return;
    }
    let removed = {
        let mut hooks = registry();
        hooks.iter().position(|(t, _,)| *t == target,).map(|i| hooks.remove(i,),)
    };
    if let Some((t, hook,),) = removed
    {
        drop(hook,);
        log_info!("detour", format!("unhooked {t:#x}"));
    }
}

/// Remove every hook. Returns how many were live.
pub unsafe fn unhook_all() -> usize {
    let hooks: Vec<(usize, naly::Hook,),> = std::mem::take(&mut *registry(),);
    let count = hooks.len();
    drop(hooks,);
    count
}

/// Address where the original of a hooked function can still be called.
pub unsafe fn trampoline(target: usize,) -> Option<usize,> {
    let hooks = registry();
    hooks.iter().find(|(t, _,)| *t == target,).map(|(_, h,)| h.trampoline() as usize,)
}

/// Number of live hooks.
pub unsafe fn count() -> usize {
    registry().len()
}
