//! Direct3D 9 device hook: `Present` and `Reset` on `IDirect3DDevice9`.
//!
//! D3D9 has no swapchain object to interrogate, so the vtable addresses come
//! from a throw-away device built on a hidden window; every device of the same
//! driver shares those entries.

use std::{
    ffi::c_void,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use windows::Win32::{
    Foundation::{BOOL, HWND},
    Graphics::Direct3D9::{
        D3DADAPTER_DEFAULT, D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL,
        D3DDEVTYPE_REF, D3DFMT_A8R8G8B8, D3DMULTISAMPLE_NONE, D3DPRESENT_INTERVAL_IMMEDIATE, D3DPRESENT_PARAMETERS,
        D3DSWAPEFFECT_DISCARD, Direct3DCreate9, IDirect3DDevice9, IDirect3DDevice9_Vtbl,
    },
};

use windows::core::Interface;

use super::detour;
use crate::{log_error, log_info, log_warn, overlay, win};

static INSTALLED: AtomicBool = AtomicBool::new(false,);
static ORIG_PRESENT: AtomicUsize = AtomicUsize::new(0,);
static ORIG_RESET: AtomicUsize = AtomicUsize::new(0,);
static TARGETS: Mutex<Vec<usize,>,> = Mutex::new(Vec::new(),);

/// Slots are derived from the generated vtable layout, so they stay right on
/// both architectures instead of relying on a hand-counted constant.
const PRESENT_SLOT: usize = std::mem::offset_of!(IDirect3DDevice9_Vtbl, Present) / std::mem::size_of::<usize,>();
const RESET_SLOT: usize = std::mem::offset_of!(IDirect3DDevice9_Vtbl, Reset) / std::mem::size_of::<usize,>();

/// D3D_SDK_VERSION from d3d9.h.
const D3D_SDK_VERSION: u32 = 32;

pub const D3DERR_DEVICELOST: i32 = 0x8876_0868u32 as i32;
pub const D3DERR_DEVICENOTRESET: i32 = 0x8876_0869u32 as i32;

type PresentFn = unsafe extern "system" fn(*mut c_void, *const c_void, *const c_void, *mut c_void, *const c_void) -> i32;
type ResetFn = unsafe extern "system" fn(*mut c_void, *mut c_void,) -> i32;

pub unsafe fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire,)
}

pub unsafe fn is_device_lost(hr: i32,) -> bool {
    hr == D3DERR_DEVICELOST || hr == D3DERR_DEVICENOTRESET
}

unsafe extern "system" fn detour_present(
    device: *mut c_void,
    source: *const c_void,
    dest: *const c_void,
    override_window: *mut c_void,
    dirty: *const c_void,
) -> i32 {
    overlay::on_present_d3d9(device,);
    let orig = ORIG_PRESENT.load(Ordering::Acquire,);
    if orig == 0
    {
        return 0;
    }
    let f: PresentFn = std::mem::transmute(orig,);
    let hr = f(device, source, dest, override_window, dirty,);
    if is_device_lost(hr,)
    {
        overlay::on_lost_d3d9(device,);
    }
    hr
}

unsafe extern "system" fn detour_reset(device: *mut c_void, params: *mut c_void,) -> i32 {
    // default-pool resources (our vertex/index buffers) must go before Reset
    overlay::on_reset_d3d9(device,);
    let orig = ORIG_RESET.load(Ordering::Acquire,);
    if orig == 0
    {
        return 0;
    }
    let f: ResetFn = std::mem::transmute(orig,);
    f(device, params,)
}

unsafe fn vtbl_entry(object: *mut c_void, index: usize,) -> usize {
    if object.is_null()
    {
        return 0;
    }
    let vtable = *(object as *const *const usize);
    if vtable.is_null()
    {
        return 0;
    }
    *vtable.add(index,)
}

unsafe fn hook_one(name: &str, target: usize, detour_fn: usize, slot: &AtomicUsize,) -> bool {
    crate::hook::install_one("hook", name, target, detour_fn, slot, &TARGETS,)
}

unsafe fn probe_device() -> Option<IDirect3DDevice9,> {
    let class = c"nany_probe_d3d9";
    let title = c"nany probe 9";
    let hwnd = win::create_dummy_window(class, title,);
    if hwnd.is_null()
    {
        log_error!("hook", "d3d9 probe window creation failed");
        return None;
    }

    let built = (|| {
        let d3d = Direct3DCreate9(D3D_SDK_VERSION,)?;
        let mut params = D3DPRESENT_PARAMETERS {
            BackBufferWidth:      8,
            BackBufferHeight:     8,
            BackBufferFormat:     D3DFMT_A8R8G8B8,
            BackBufferCount:      1,
            MultiSampleType:      D3DMULTISAMPLE_NONE,
            SwapEffect:           D3DSWAPEFFECT_DISCARD,
            hDeviceWindow:        HWND(hwnd,),
            Windowed:             BOOL(1,),
            PresentationInterval: D3DPRESENT_INTERVAL_IMMEDIATE as u32,
            ..Default::default()
        };

        let attempts = [
            (D3DDEVTYPE_HAL, D3DCREATE_HARDWARE_VERTEXPROCESSING),
            (D3DDEVTYPE_HAL, D3DCREATE_SOFTWARE_VERTEXPROCESSING),
            (D3DDEVTYPE_REF, D3DCREATE_SOFTWARE_VERTEXPROCESSING),
        ];
        for (kind, flags,) in attempts
        {
            let mut device: Option<IDirect3DDevice9,> = None;
            let hr = d3d.CreateDevice(D3DADAPTER_DEFAULT, kind, HWND(hwnd,), flags as u32, &mut params, &mut device,);
            if hr.is_ok()
            {
                if let Some(device,) = device
                {
                    return Some(device,);
                }
            }
        }
        None
    })();

    if built.is_none()
    {
        log_warn!("hook", "IDirect3D9::CreateDevice failed for every device type");
    }
    win::destroy_window(hwnd, class,);
    built
}

pub unsafe fn install() -> bool {
    if INSTALLED.swap(true, Ordering::SeqCst,)
    {
        return true;
    }
    if crate::api::module_base("d3d9.dll",) == 0
    {
        INSTALLED.store(false, Ordering::Release,);
        return false;
    }

    let Some(device,) = probe_device()
    else
    {
        INSTALLED.store(false, Ordering::Release,);
        return false;
    };

    let raw = device.as_raw();
    let mut ok = false;
    ok |= hook_one(
        "IDirect3DDevice9::Present",
        vtbl_entry(raw, PRESENT_SLOT,),
        detour_present as *const () as usize,
        &ORIG_PRESENT,
    );
    ok |= hook_one(
        "IDirect3DDevice9::Reset",
        vtbl_entry(raw, RESET_SLOT,),
        detour_reset as *const () as usize,
        &ORIG_RESET,
    );
    drop(device,);

    log_info!("hook", format!("d3d9 hooks installed = {ok}"));
    ok
}

pub unsafe fn ensure() -> bool {
    installed() || install()
}

pub unsafe fn unhook_all() {
    let targets: Vec<usize,> = match TARGETS.lock()
    {
        Ok(g,) => g.clone(),
        Err(p,) => p.into_inner().clone(),
    };
    for t in &targets
    {
        detour::unhook(*t,);
    }
    ORIG_PRESENT.store(0, Ordering::Release,);
    ORIG_RESET.store(0, Ordering::Release,);
    INSTALLED.store(false, Ordering::Release,);
    log_warn!("hook", format!("d3d9: unloaded {} inline hook(s)", targets.len()));
}
