use std::{ffi::c_void, sync::OnceLock};

use crate::{log_error, log_info};

static READY: OnceLock<bool,> = OnceLock::new();

unsafe fn init() -> bool {
    *READY.get_or_init(|| {
        if !min_hook_rs::is_supported()
        {
            log_info!("detour", "inline hook is not supported on this platform");
            return false;
        }
        match min_hook_rs::initialize()
        {
            Ok((),) =>
            {
                log_info!("detour", format!("min_hook_rs {} ready", min_hook_rs::get_version()));
                true
            }
            Err(e,) =>
            {
                log_error!("detour", format!("initialize failed: {e:?}"));
                false
            }
        }
    },)
}

pub unsafe fn hook(target: usize, detour: usize,) -> Option<usize,> {
    if target == 0 || detour == 0
    {
        return None;
    }
    if !init()
    {
        return None;
    }
    let target_ptr = target as *mut c_void;
    match min_hook_rs::create_hook(target_ptr, detour as *mut c_void,)
    {
        Ok(tramp,) => match min_hook_rs::enable_hook(target_ptr,)
        {
            Ok((),) => Some(tramp as usize,),
            Err(e,) =>
            {
                log_error!("detour", format!("enable 0x{target:X} failed: {e:?}"));
                let _ = min_hook_rs::remove_hook(target_ptr,);
                None
            }
        },
        Err(e,) =>
        {
            log_error!("detour", format!("create 0x{target:X} failed: {e:?}"));
            None
        }
    }
}

pub unsafe fn unhook(target: usize,) {
    if target == 0
    {
        return;
    }
    let _ = min_hook_rs::disable_hook(target as *mut c_void,);
    let _ = min_hook_rs::remove_hook(target as *mut c_void,);
}
