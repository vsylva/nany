use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

use super::detour;
use crate::{log_info, log_warn, win};

pub type VkHandle = u64;

#[repr(C)]
#[derive(Clone, Copy,)]
pub struct VkExtent2D {
    pub width:  u32,
    pub height: u32,
}

#[repr(C)]
pub struct VkDeviceQueueCreateInfo {
    pub s_type:             i32,
    pub p_next:             *const c_void,
    pub flags:              u32,
    pub queue_family_index: u32,
    pub queue_count:        u32,
    pub p_queue_priorities: *const f32,
}

#[repr(C)]
pub struct VkDeviceCreateInfo {
    pub s_type: i32,
    pub p_next: *const c_void,
    pub flags: u32,
    pub queue_create_info_count: u32,
    pub p_queue_create_infos: *const VkDeviceQueueCreateInfo,
    pub enabled_layer_count: u32,
    pub pp_enabled_layer_names: *const *const u8,
    pub enabled_extension_count: u32,
    pub pp_enabled_extension_names: *const *const u8,
    pub p_enabled_features: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy,)]
pub struct VkSwapchainCreateInfoKHR {
    pub s_type:                   i32,
    pub p_next:                   *const c_void,
    pub flags:                    u32,
    pub surface:                  VkHandle,
    pub min_image_count:          u32,
    pub image_format:             i32,
    pub image_color_space:        i32,
    pub image_extent:             VkExtent2D,
    pub image_array_layers:       u32,
    pub image_usage:              u32,
    pub image_sharing_mode:       i32,
    pub queue_family_index_count: u32,
    pub p_queue_family_indices:   *const u32,
    pub pre_transform:            u32,
    pub composite_alpha:          i32,
    pub present_mode:             i32,
    pub clipped:                  u32,
    pub old_swapchain:            VkHandle,
}

#[repr(C)]
pub struct VkPresentInfoKHR {
    pub s_type:               i32,
    pub p_next:               *const c_void,
    pub wait_semaphore_count: u32,
    pub p_wait_semaphores:    *const VkHandle,
    pub swapchain_count:      u32,
    pub p_swapchains:         *const VkHandle,
    pub p_image_indices:      *const u32,
    pub p_results:            *mut i32,
}

type PfnCreateDevice =
    unsafe extern "system" fn(VkHandle, *const VkDeviceCreateInfo, *const c_void, *mut VkHandle,) -> i32;
type PfnCreateSwapchain =
    unsafe extern "system" fn(VkHandle, *const VkSwapchainCreateInfoKHR, *const c_void, *mut VkHandle,) -> i32;

#[repr(C)]
pub struct VkWin32SurfaceCreateInfoKHR {
    pub s_type:    i32,
    pub p_next:    *const c_void,
    pub flags:     u32,
    pub hinstance: *mut c_void,
    pub hwnd:      *mut c_void,
}
type PfnCreateWin32Surface =
    unsafe extern "system" fn(VkHandle, *const VkWin32SurfaceCreateInfoKHR, *const c_void, *mut VkHandle,) -> i32;
type PfnDestroySwapchain = unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,);
type PfnPresent = unsafe extern "system" fn(VkHandle, *const VkPresentInfoKHR,) -> i32;

static INSTALLED: AtomicBool = AtomicBool::new(false,);
static ORIG_PRESENT: AtomicUsize = AtomicUsize::new(0,);
static ORIG_CREATE_SWAPCHAIN: AtomicUsize = AtomicUsize::new(0,);
static ORIG_CREATE_WIN32_SURFACE: AtomicUsize = AtomicUsize::new(0,);
static ORIG_DESTROY_SWAPCHAIN: AtomicUsize = AtomicUsize::new(0,);
static ORIG_CREATE_DEVICE: AtomicUsize = AtomicUsize::new(0,);
static ORIG_CREATE_INSTANCE: AtomicUsize = AtomicUsize::new(0,);
static ORIG_GIPA: AtomicUsize = AtomicUsize::new(0,);
static ORIG_GDPA: AtomicUsize = AtomicUsize::new(0,);
static TARGETS: Mutex<Vec<usize,>,> = Mutex::new(Vec::new(),);

static SWAPCHAINS: Mutex<Option<HashMap<VkHandle, SwapchainInfo,>,>,> = Mutex::new(None,);

static SURFACES: Mutex<Option<HashMap<VkHandle, usize,>,>,> = Mutex::new(None,);

static DEVICE: Mutex<Option<DeviceInfo,>,> = Mutex::new(None,);

static INSTANCE: AtomicU64 = AtomicU64::new(0,);

pub unsafe fn instance() -> Option<VkHandle,> {
    match INSTANCE.load(Ordering::Acquire,)
    {
        0 => None,
        v => Some(v,),
    }
}

#[derive(Clone, Copy,)]
pub struct DeviceInfo {
    pub device: VkHandle,

    pub physical: VkHandle,

    pub queue_family: u32,
    pub queue_count:  u32,
}

#[derive(Clone, Copy, Debug,)]
pub struct SwapchainInfo {
    pub format:          i32,
    pub color_space:     i32,
    pub width:           u32,
    pub height:          u32,
    pub images:          u32,
    pub usage:           u32,
    pub present_mode:    i32,
    pub composite_alpha: i32,
    pub pre_transform:   u32,
    pub surface:         VkHandle,
}

pub unsafe fn swapchain_info(sc: VkHandle,) -> Option<SwapchainInfo,> {
    let g = SWAPCHAINS.lock().ok()?;
    g.as_ref()?.get(&sc,).copied()
}

pub unsafe fn swapchain_hwnd(sc: VkHandle,) -> Option<usize,> {
    let surface = swapchain_info(sc,)?.surface;
    if surface == 0
    {
        return None;
    }
    let g = SURFACES.lock().ok()?;
    g.as_ref()?.get(&surface,).copied()
}

pub unsafe fn device_info() -> Option<DeviceInfo,> {
    *DEVICE.lock().ok()?
}

pub unsafe fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire,)
}

unsafe extern "system" fn detour_present(queue: VkHandle, info: *const VkPresentInfoKHR,) -> i32 {
    let orig = ORIG_PRESENT.load(Ordering::Acquire,);
    let real = |pi: *const VkPresentInfoKHR| -> i32 {
        if orig == 0
        {
            return 0;
        }
        let f: PfnPresent = std::mem::transmute(orig,);
        f(queue, pi,)
    };

    if info.is_null()
    {
        return real(info,);
    }
    let pi = &*info;
    if pi.swapchain_count == 0 || pi.p_swapchains.is_null()
    {
        return real(info,);
    }
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static ONCE: AtomicBool = AtomicBool::new(false,);
        if !ONCE.swap(true, Ordering::AcqRel,)
        {}
    }
    let sc = *pi.p_swapchains;
    let idx = if pi.p_image_indices.is_null() { 0 } else { *pi.p_image_indices };

    let sem = crate::overlay::on_vk_present(queue, sc, idx,);

    static LOGGED: AtomicBool = AtomicBool::new(false,);
    if !LOGGED.swap(true, Ordering::AcqRel,)
    {
        match swapchain_info(sc,)
        {
            Some(s,) => log_info!(
                "vk",
                format!(
                    "present: queue={queue:#x} swapchain={sc:#x} image={idx} {}x{} fmt={} images={} overlay_sem={}",
                    s.width,
                    s.height,
                    s.format,
                    s.images,
                    if sem.is_some() { 1 } else { 0 }
                )
            ),
            None => log_info!(
                "vk",
                format!("present: queue={queue:#x} swapchain={sc:#x} image={idx} (creation params not captured)")
            ),
        }
    }

    match sem
    {
        Some(s,) =>
        {
            let n = pi.wait_semaphore_count as usize;
            let mut waits: Vec<VkHandle,> = Vec::with_capacity(n + 1,);
            if n > 0 && !pi.p_wait_semaphores.is_null()
            {
                for i in 0..n
                {
                    waits.push(*pi.p_wait_semaphores.add(i,),);
                }
            }
            waits.push(s,);
            let patched = VkPresentInfoKHR {
                s_type:               pi.s_type,
                p_next:               pi.p_next,
                wait_semaphore_count: waits.len() as u32,
                p_wait_semaphores:    waits.as_ptr(),
                swapchain_count:      pi.swapchain_count,
                p_swapchains:         pi.p_swapchains,
                p_image_indices:      pi.p_image_indices,
                p_results:            pi.p_results,
            };
            real(&patched,)
        }
        None => real(info,),
    }
}

unsafe extern "system" fn detour_create_win32_surface(
    instance: VkHandle,
    ci: *const VkWin32SurfaceCreateInfoKHR,
    alloc: *const c_void,
    out: *mut VkHandle,
) -> i32 {
    let orig = ORIG_CREATE_WIN32_SURFACE.load(Ordering::Acquire,);
    if orig == 0 || ci.is_null() || out.is_null()
    {
        return -1;
    }
    let f: PfnCreateWin32Surface = std::mem::transmute(orig,);
    let hr = f(instance, ci, alloc, out,);
    if hr == 0
    {
        let hwnd = (*ci).hwnd as usize;
        let surface = *out;
        if hwnd != 0 && surface != 0
        {
            if let Ok(mut g,) = SURFACES.lock()
            {
                let m = g.get_or_insert_with(HashMap::new,);
                if m.len() >= 32
                {
                    m.clear();
                }
                m.insert(surface, hwnd,);
            }
            log_info!("vk", format!("surface {surface:#x} -> hwnd {hwnd:#x}"));
        }
    }
    hr
}

unsafe extern "system" fn detour_create_swapchain(
    device: VkHandle,
    ci: *const VkSwapchainCreateInfoKHR,
    alloc: *const c_void,
    out: *mut VkHandle,
) -> i32 {
    let hr = {
        let orig = ORIG_CREATE_SWAPCHAIN.load(Ordering::Acquire,);
        if orig == 0
        {
            return -1;
        }
        let f: PfnCreateSwapchain = std::mem::transmute(orig,);
        f(device, ci, alloc, out,)
    };
    if hr == 0 && !ci.is_null() && !out.is_null()
    {
        let sc = *out;
        let c = &*ci;
        let info = SwapchainInfo {
            format:          c.image_format,
            color_space:     c.image_color_space,
            width:           c.image_extent.width,
            height:          c.image_extent.height,
            images:          c.min_image_count,
            usage:           c.image_usage,
            present_mode:    c.present_mode,
            composite_alpha: c.composite_alpha,
            pre_transform:   c.pre_transform,
            surface:         c.surface,
        };
        if let Ok(mut g,) = SWAPCHAINS.lock()
        {
            g.get_or_insert_with(HashMap::new,).insert(sc, info,);
        }
        log_info!(
            "vk",
            format!(
                "swapchain {sc:#x}: {}x{} fmt={} cs={} images={} usage={:#x} mode={} alpha={} transform={}",
                info.width,
                info.height,
                info.format,
                info.color_space,
                info.images,
                info.usage,
                info.present_mode,
                info.composite_alpha,
                info.pre_transform
            )
        );
    }
    hr
}

unsafe extern "system" fn detour_destroy_swapchain(device: VkHandle, sc: VkHandle, alloc: *const c_void,) {
    if let Ok(mut g,) = SWAPCHAINS.lock()
    {
        if let Some(map,) = g.as_mut()
        {
            map.remove(&sc,);
        }
    }
    let orig = ORIG_DESTROY_SWAPCHAIN.load(Ordering::Acquire,);
    if orig != 0
    {
        let f: PfnDestroySwapchain = std::mem::transmute(orig,);
        f(device, sc, alloc,);
    }
}

type PfnCreateInstance = unsafe extern "system" fn(*const VkInstanceCreateInfo, *const c_void, *mut VkHandle,) -> i32;

#[repr(C)]
pub struct VkInstanceCreateInfo {
    pub s_type: i32,
    pub p_next: *const c_void,
    pub flags: u32,
    pub p_application_info: *const c_void,
    pub enabled_layer_count: u32,
    pub pp_enabled_layer_names: *const *const u8,
    pub enabled_extension_count: u32,
    pub pp_enabled_extension_names: *const *const u8,
}

unsafe extern "system" fn detour_create_instance(
    ci: *const VkInstanceCreateInfo,
    alloc: *const c_void,
    out: *mut VkHandle,
) -> i32 {
    let orig = ORIG_CREATE_INSTANCE.load(Ordering::Acquire,);
    if orig == 0
    {
        return -1;
    }
    let f: PfnCreateInstance = std::mem::transmute(orig,);
    let hr = f(ci, alloc, out,);
    if hr == 0 && !out.is_null()
    {
        let inst = *out;
        INSTANCE.store(inst, Ordering::Release,);
        log_info!("vk", format!("instance {inst:#x}"));
    }
    hr
}

unsafe extern "system" fn detour_create_device(
    physical: VkHandle,
    ci: *const VkDeviceCreateInfo,
    alloc: *const c_void,
    out: *mut VkHandle,
) -> i32 {
    let hr = {
        let orig = ORIG_CREATE_DEVICE.load(Ordering::Acquire,);
        if orig == 0
        {
            return -1;
        }
        let f: PfnCreateDevice = std::mem::transmute(orig,);
        f(physical, ci, alloc, out,)
    };
    if hr == 0 && !ci.is_null() && !out.is_null()
    {
        let device = *out;
        let c = &*ci;
        let mut family = 0u32;
        let mut queues = 1u32;
        if c.queue_create_info_count > 0 && !c.p_queue_create_infos.is_null()
        {
            let q = &*c.p_queue_create_infos;
            family = q.queue_family_index;
            queues = q.queue_count.max(1,);
        }
        if let Ok(mut g,) = DEVICE.lock()
        {
            *g = Some(DeviceInfo { device, physical, queue_family: family, queue_count: queues, },);
        }
        log_info!(
            "vk",
            format!(
                "device {device:#x}: queue family {family}, {} queue(s), {} ext",
                queues, c.enabled_extension_count
            )
        );
    }
    hr
}

unsafe fn name_is(name: *const u8, want: &[u8],) -> bool {
    if name.is_null()
    {
        return false;
    }
    let mut i = 0usize;
    loop
    {
        let c = *name.add(i,);
        if c == 0
        {
            return i == want.len();
        }
        if i >= want.len() || c != want[i]
        {
            return false;
        }
        i += 1;
    }
}

type PfnGpa = unsafe extern "system" fn(VkHandle, *const u8,) -> *const c_void;

unsafe extern "system" fn detour_gipa(instance: VkHandle, name: *const u8,) -> *const c_void {
    if name_is(name, b"vkQueuePresentKHR",)
    {
        return detour_present as *const () as *const c_void;
    }
    if name_is(name, b"vkCreateDevice",)
    {
        return detour_create_device as *const () as *const c_void;
    }
    if name_is(name, b"vkCreateSwapchainKHR",)
    {
        return detour_create_swapchain as *const () as *const c_void;
    }
    if name_is(name, b"vkDestroySwapchainKHR",)
    {
        return detour_destroy_swapchain as *const () as *const c_void;
    }
    let orig = ORIG_GIPA.load(Ordering::Acquire,);
    if orig == 0
    {
        return std::ptr::null();
    }
    let f: PfnGpa = std::mem::transmute(orig,);
    f(instance, name,)
}

unsafe extern "system" fn detour_gdpa(device: VkHandle, name: *const u8,) -> *const c_void {
    if name_is(name, b"vkQueuePresentKHR",)
    {
        return detour_present as *const () as *const c_void;
    }
    if name_is(name, b"vkCreateSwapchainKHR",)
    {
        return detour_create_swapchain as *const () as *const c_void;
    }
    if name_is(name, b"vkDestroySwapchainKHR",)
    {
        return detour_destroy_swapchain as *const () as *const c_void;
    }
    let orig = ORIG_GDPA.load(Ordering::Acquire,);
    if orig == 0
    {
        return std::ptr::null();
    }
    let f: PfnGpa = std::mem::transmute(orig,);
    f(device, name,)
}

unsafe fn hook_one(name: &str, target: usize, detour_fn: usize, slot: &AtomicUsize,) -> bool {
    crate::hook::install_one("vk", name, target, detour_fn, slot, &TARGETS,)
}

pub unsafe fn install() -> bool {
    if INSTALLED.swap(true, Ordering::SeqCst,)
    {
        return true;
    }
    let module = win::module_base_bytes(b"vulkan-1.dll\0",);
    if module == 0
    {
        INSTALLED.store(false, Ordering::Release,);
        return false;
    }
    let mut ok = false;

    let p = win::proc_address(module as isize, b"vkGetInstanceProcAddr\0",) as usize;
    ok |= hook_one("vkGetInstanceProcAddr", p, detour_gipa as *const () as usize, &ORIG_GIPA,);
    let p = win::proc_address(module as isize, b"vkGetDeviceProcAddr\0",) as usize;
    ok |= hook_one("vkGetDeviceProcAddr", p, detour_gdpa as *const () as usize, &ORIG_GDPA,);

    let p = win::proc_address(module as isize, b"vkCreateInstance\0",) as usize;
    ok |= hook_one("vkCreateInstance", p, detour_create_instance as *const () as usize, &ORIG_CREATE_INSTANCE,);
    let p = win::proc_address(module as isize, b"vkQueuePresentKHR\0",) as usize;
    ok |= hook_one("vkQueuePresentKHR", p, detour_present as *const () as usize, &ORIG_PRESENT,);

    let p = win::proc_address(module as isize, b"vkCreateSwapchainKHR\0",) as usize;
    ok |= hook_one("vkCreateSwapchainKHR", p, detour_create_swapchain as *const () as usize, &ORIG_CREATE_SWAPCHAIN,);
    let p = win::proc_address(module as isize, b"vkDestroySwapchainKHR\0",) as usize;
    ok |=
        hook_one("vkDestroySwapchainKHR", p, detour_destroy_swapchain as *const () as usize, &ORIG_DESTROY_SWAPCHAIN,);
    let p = win::proc_address(module as isize, b"vkCreateDevice\0",) as usize;
    ok |= hook_one("vkCreateDevice", p, detour_create_device as *const () as usize, &ORIG_CREATE_DEVICE,);

    let p = win::proc_address(module as isize, b"vkCreateWin32SurfaceKHR\0",) as usize;
    if p != 0
    {
        let _ = hook_one(
            "vkCreateWin32SurfaceKHR",
            p,
            detour_create_win32_surface as *const () as usize,
            &ORIG_CREATE_WIN32_SURFACE,
        );
    }

    log_info!("vk", format!("hooks installed = {ok}"));
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
    ORIG_PRESENT.store(0, Ordering::Release,);
    ORIG_CREATE_SWAPCHAIN.store(0, Ordering::Release,);
    ORIG_DESTROY_SWAPCHAIN.store(0, Ordering::Release,);
    ORIG_CREATE_DEVICE.store(0, Ordering::Release,);
    ORIG_CREATE_INSTANCE.store(0, Ordering::Release,);
    INSTANCE.store(0, Ordering::Release,);
    ORIG_GIPA.store(0, Ordering::Release,);
    ORIG_GDPA.store(0, Ordering::Release,);
    INSTALLED.store(false, Ordering::Release,);
    log_warn!("vk", format!("circuit breaker: unloaded {} inline hook(s)", targets.len()));
}
