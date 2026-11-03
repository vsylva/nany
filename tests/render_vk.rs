#![allow(non_snake_case, non_camel_case_types, dead_code, clippy::missing_transmute_annotations)]

mod support;

use std::ffi::c_void;

type VkHandle = u64;
type PfnGpa = unsafe extern "system" fn(VkHandle, *const u8,) -> *const c_void;

#[repr(C)]
struct VkApplicationInfo {
    s_type:              i32,
    p_next:              *const c_void,
    p_application_name:  *const u8,
    application_version: u32,
    p_engine_name:       *const u8,
    engine_version:      u32,
    api_version:         u32,
}

#[repr(C)]
struct VkInstanceCreateInfo {
    s_type: i32,
    p_next: *const c_void,
    flags: u32,
    p_application_info: *const VkApplicationInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const u8,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct VkExtent2D {
    width:  u32,
    height: u32,
}

#[repr(C)]
struct VkWin32SurfaceCreateInfoKHR {
    s_type:    i32,
    p_next:    *const c_void,
    flags:     u32,
    hinstance: isize,
    hwnd:      *mut c_void,
}

#[repr(C)]
struct VkDeviceQueueCreateInfo {
    s_type:             i32,
    p_next:             *const c_void,
    flags:              u32,
    queue_family_index: u32,
    queue_count:        u32,
    p_queue_priorities: *const f32,
}

#[repr(C)]
struct VkDeviceCreateInfo {
    s_type: i32,
    p_next: *const c_void,
    flags: u32,
    queue_create_info_count: u32,
    p_queue_create_infos: *const VkDeviceQueueCreateInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const u8,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const u8,
    p_enabled_features: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct VkSurfaceCapabilitiesKHR {
    min_image_count:           u32,
    max_image_count:           u32,
    current_extent:            VkExtent2D,
    min_image_extent:          VkExtent2D,
    max_image_extent:          VkExtent2D,
    max_image_array_layers:    u32,
    supported_transforms:      u32,
    current_transform:         i32,
    supported_composite_alpha: u32,
    supported_usage_flags:     u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct VkSurfaceFormatKHR {
    format:      i32,
    color_space: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct VkExtent3D {
    width:  u32,
    height: u32,
    depth:  u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct VkQueueFamilyProperties {
    queue_flags:                    u32,
    queue_count:                    u32,
    timestamp_valid_bits:           u32,
    min_image_transfer_granularity: VkExtent3D,
}

#[repr(C)]
struct VkSwapchainCreateInfoKHR {
    s_type:                   i32,
    p_next:                   *const c_void,
    flags:                    u32,
    surface:                  VkHandle,
    min_image_count:          u32,
    image_format:             i32,
    image_color_space:        i32,
    image_extent:             VkExtent2D,
    image_array_layers:       u32,
    image_usage:              u32,
    image_sharing_mode:       i32,
    queue_family_index_count: u32,
    p_queue_family_indices:   *const u32,
    pre_transform:            u32,
    composite_alpha:          i32,
    present_mode:             i32,
    clipped:                  u32,
    old_swapchain:            VkHandle,
}

#[repr(C)]
struct VkCommandPoolCreateInfo {
    s_type:             i32,
    p_next:             *const c_void,
    flags:              u32,
    queue_family_index: u32,
}

#[repr(C)]
struct VkCommandBufferAllocateInfo {
    s_type:               i32,
    p_next:               *const c_void,
    command_pool:         VkHandle,
    level:                i32,
    command_buffer_count: u32,
}

#[repr(C)]
struct VkCommandBufferBeginInfo {
    s_type:             i32,
    p_next:             *const c_void,
    flags:              u32,
    p_inheritance_info: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy,)]
struct VkImageSubresourceRange {
    aspect_mask:      u32,
    base_mip_level:   u32,
    level_count:      u32,
    base_array_layer: u32,
    layer_count:      u32,
}

#[repr(C)]
struct VkImageMemoryBarrier {
    s_type:                 i32,
    p_next:                 *const c_void,
    src_access_mask:        u32,
    dst_access_mask:        u32,
    old_layout:             i32,
    new_layout:             i32,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    image:                  VkHandle,
    subresource_range:      VkImageSubresourceRange,
}

#[repr(C)]
struct VkSubmitInfo {
    s_type:                 i32,
    p_next:                 *const c_void,
    wait_semaphore_count:   u32,
    p_wait_semaphores:      *const VkHandle,
    p_wait_dst_stage_mask:  *const u32,
    command_buffer_count:   u32,
    p_command_buffers:      *const VkHandle,
    signal_semaphore_count: u32,
    p_signal_semaphores:    *const VkHandle,
}

#[repr(C)]
struct VkPresentInfoKHR {
    s_type:               i32,
    p_next:               *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores:    *const VkHandle,
    swapchain_count:      u32,
    p_swapchains:         *const VkHandle,
    p_image_indices:      *const u32,
    p_results:            *mut i32,
}

#[repr(C)]
struct VkSemaphoreCreateInfo {
    s_type: i32,
    p_next: *const c_void,
    flags:  u32,
}

#[repr(C)]
struct VkFenceCreateInfo {
    s_type: i32,
    p_next: *const c_void,
    flags:  u32,
}

const S_APP: i32 = 0;
const S_INSTANCE: i32 = 1;
const S_DEVICE_QUEUE: i32 = 2;
const S_DEVICE: i32 = 3;
const S_SUBMIT: i32 = 4;
const S_FENCE: i32 = 8;
const S_SEMAPHORE: i32 = 9;
const S_COMMAND_POOL: i32 = 39;
const S_COMMAND_BUFFER_ALLOCATE: i32 = 40;
const S_COMMAND_BUFFER_BEGIN: i32 = 42;
const S_IMAGE_MEMORY_BARRIER: i32 = 45;
const S_WIN32_SURFACE: i32 = 1_000_009_000;
const S_SWAPCHAIN: i32 = 1_000_001_000;
const S_PRESENT: i32 = 1_000_001_001;

const QUEUE_GRAPHICS: u32 = 0x0001;
const IMAGE_USAGE_COLOR_ATTACHMENT: u32 = 0x10;
const SHARING_EXCLUSIVE: i32 = 0;
const COMPOSITE_OPAQUE: i32 = 1;
const TRANSFORM_IDENTITY: u32 = 1;
const PRESENT_FIFO: i32 = 2;
const LEVEL_PRIMARY: i32 = 0;
const CMD_BUFFER_ONE_TIME: u32 = 0x0000_0001;
const CMD_POOL_RESET: u32 = 0x0000_0002;

const ACCESS_TRANSFER_WRITE: u32 = 0x1000;
const STAGE_TOP: u32 = 0x1;
const STAGE_TRANSFER: u32 = 0x1000;
const STAGE_BOTTOM: u32 = 0x2000;
const LAYOUT_UNDEFINED: i32 = 0;
const LAYOUT_TRANSFER_DST: i32 = 7;
const LAYOUT_PRESENT: i32 = 1_000_001_002;
const ASPECT_COLOR: u32 = 1;
const WHOLE_SIZE: u32 = u32::MAX;

type FnCreateInstance = unsafe extern "system" fn(*const VkInstanceCreateInfo, *const c_void, *mut VkHandle,) -> i32;
type FnCreateSurface =
    unsafe extern "system" fn(VkHandle, *const VkWin32SurfaceCreateInfoKHR, *const c_void, *mut VkHandle,) -> i32;
type FnEnumPhys = unsafe extern "system" fn(VkHandle, *mut u32, *mut VkHandle,) -> i32;
type FnGetCaps = unsafe extern "system" fn(VkHandle, VkHandle, *mut VkSurfaceCapabilitiesKHR,) -> i32;
type FnGetFormats = unsafe extern "system" fn(VkHandle, VkHandle, *mut u32, *mut VkSurfaceFormatKHR,) -> i32;
type FnGetQueueProps = unsafe extern "system" fn(VkHandle, *mut u32, *mut VkQueueFamilyProperties,) -> ();
type FnCreateDevice =
    unsafe extern "system" fn(VkHandle, *const VkDeviceCreateInfo, *const c_void, *mut VkHandle,) -> i32;
type FnGetDeviceQueue = unsafe extern "system" fn(VkHandle, u32, u32, *mut VkHandle,) -> ();
type FnCreateSwapchain =
    unsafe extern "system" fn(VkHandle, *const VkSwapchainCreateInfoKHR, *const c_void, *mut VkHandle,) -> i32;
type FnGetImages = unsafe extern "system" fn(VkHandle, VkHandle, *mut u32, *mut VkHandle,) -> i32;
type FnDestroySwapchain = unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,) -> ();
type FnCreatePool =
    unsafe extern "system" fn(VkHandle, *const VkCommandPoolCreateInfo, *const c_void, *mut VkHandle,) -> i32;
type FnAllocate = unsafe extern "system" fn(VkHandle, *const VkCommandBufferAllocateInfo, *mut VkHandle,) -> i32;
type FnBegin = unsafe extern "system" fn(VkHandle, *const VkCommandBufferBeginInfo,) -> i32;
type FnEnd = unsafe extern "system" fn(VkHandle,) -> i32;
type FnReset = unsafe extern "system" fn(u64, u32,) -> i32;
type FnCmdBarrier = unsafe extern "system" fn(
    u64,
    u32,
    u32,
    u32,
    u32,
    *const c_void,
    u32,
    *const c_void,
    u32,
    *const VkImageMemoryBarrier,
);
type FnCmdClear = unsafe extern "system" fn(u64, u64, i32, *const f32, u32, *const VkImageSubresourceRange,);
type FnAcquire = unsafe extern "system" fn(VkHandle, VkHandle, u64, VkHandle, VkHandle, *mut u32,) -> i32;
type FnQueueSubmit = unsafe extern "system" fn(VkHandle, u32, *const VkSubmitInfo, VkHandle,) -> i32;
type FnQueuePresent = unsafe extern "system" fn(VkHandle, *const VkPresentInfoKHR,) -> i32;
type FnCreateSemaphore =
    unsafe extern "system" fn(VkHandle, *const VkSemaphoreCreateInfo, *const c_void, *mut VkHandle,) -> i32;
type FnCreateFence =
    unsafe extern "system" fn(VkHandle, *const VkFenceCreateInfo, *const c_void, *mut VkHandle,) -> i32;
type FnWaitForFence = unsafe extern "system" fn(VkHandle, u32, *const VkHandle, u32, u64,) -> i32;
type FnResetFence = unsafe extern "system" fn(VkHandle, u32, *const VkHandle,) -> i32;
type FnDeviceWaitIdle = unsafe extern "system" fn(VkHandle,) -> i32;

struct Vk {
    instance:     VkHandle,
    surface:      VkHandle,
    phys:         VkHandle,
    device:       VkHandle,
    queue:        VkHandle,
    queue_family: u32,
    swapchain:    VkHandle,
    images:       Vec<VkHandle,>,
    extent:       VkExtent2D,
    format:       i32,
    color_space:  i32,
    pool:         VkHandle,
    cmd:          VkHandle,
    sem_avail:    VkHandle,
    sem_done:     VkHandle,
    fence:        VkHandle,
    f:            VkFns,
}

struct VkFns {
    create_surface:    FnCreateSurface,
    get_caps:          FnGetCaps,
    get_formats:       FnGetFormats,
    get_queue_props:   FnGetQueueProps,
    create_device:     FnCreateDevice,
    get_device_queue:  FnGetDeviceQueue,
    create_swapchain:  FnCreateSwapchain,
    get_images:        FnGetImages,
    destroy_swapchain: FnDestroySwapchain,
    create_pool:       FnCreatePool,
    allocate:          FnAllocate,
    begin:             FnBegin,
    end:               FnEnd,
    reset_cmd:         FnReset,
    cmd_barrier:       FnCmdBarrier,
    cmd_clear:         FnCmdClear,
    acquire:           FnAcquire,
    submit:            FnQueueSubmit,
    present:           FnQueuePresent,
    create_semaphore:  FnCreateSemaphore,
    create_fence:      FnCreateFence,
    wait_for_fence:    FnWaitForFence,
    reset_fence:       FnResetFence,
    device_wait_idle:  FnDeviceWaitIdle,
}

unsafe fn sym(module: isize, name: &[u8],) -> *const c_void {
    support::GetProcAddress(module, name.as_ptr(),)
}

unsafe fn load(vk: isize, instance: VkHandle, gipa: PfnGpa, name: &[u8],) -> *const c_void {
    let p = gipa(instance, name.as_ptr(),);
    if p.is_null() { sym(vk, name,) } else { p }
}

unsafe fn create_swapchain(v: &mut Vk, old: VkHandle, size: VkExtent2D,) -> bool {
    let mut caps = VkSurfaceCapabilitiesKHR::default();
    if (v.f.get_caps)(v.phys, v.surface, &mut caps,) != 0
    {
        return false;
    }
    let mut count = 4u32;
    let mut fmts = [VkSurfaceFormatKHR::default(); 4];
    let _ = (v.f.get_formats)(v.phys, v.surface, &mut count, fmts.as_mut_ptr(),);
    if count == 0
    {
        return false;
    }
    v.format = fmts[0].format;
    v.color_space = fmts[0].color_space;
    let extent = if caps.current_extent.width != u32::MAX
    {
        caps.current_extent
    }
    else
    {
        VkExtent2D {
            width:  size.width.clamp(caps.min_image_extent.width, caps.max_image_extent.width,),
            height: size.height.clamp(caps.min_image_extent.height, caps.max_image_extent.height,),
        }
    };
    let info = VkSwapchainCreateInfoKHR {
        s_type:                   S_SWAPCHAIN,
        p_next:                   std::ptr::null(),
        flags:                    0,
        surface:                  v.surface,
        min_image_count:          (caps.min_image_count + 1).max(2,),
        image_format:             v.format,
        image_color_space:        v.color_space,
        image_extent:             extent,
        image_array_layers:       1,
        image_usage:              IMAGE_USAGE_COLOR_ATTACHMENT,
        image_sharing_mode:       SHARING_EXCLUSIVE,
        queue_family_index_count: 0,
        p_queue_family_indices:   std::ptr::null(),
        pre_transform:            TRANSFORM_IDENTITY,
        composite_alpha:          COMPOSITE_OPAQUE,
        present_mode:             PRESENT_FIFO,
        clipped:                  1,
        old_swapchain:            old,
    };
    let mut sc: VkHandle = 0;
    if (v.f.create_swapchain)(v.device, &info, std::ptr::null(), &mut sc,) != 0
    {
        return false;
    }
    let mut n = 0u32;
    let _ = (v.f.get_images)(v.device, sc, &mut n, std::ptr::null_mut(),);
    let mut imgs = vec![0u64; n.max(1) as usize];
    let _ = (v.f.get_images)(v.device, sc, &mut n, imgs.as_mut_ptr(),);
    imgs.truncate(n as usize,);
    v.swapchain = sc;
    v.images = imgs;
    v.extent = extent;
    true
}

unsafe fn frame(v: &mut Vk,) -> bool {
    let mut idx = 0u32;
    if (v.f.acquire)(v.device, v.swapchain, u64::MAX, v.sem_avail, 0, &mut idx,) != 0
    {
        return false;
    }
    let image = match v.images.get(idx as usize,)
    {
        Some(i,) => *i,
        None => return false,
    };
    let _ = (v.f.reset_fence)(v.device, 1, &v.fence,);
    let _ = (v.f.reset_cmd)(v.cmd, 0,);
    let begin = VkCommandBufferBeginInfo {
        s_type:             S_COMMAND_BUFFER_BEGIN,
        p_next:             std::ptr::null(),
        flags:              CMD_BUFFER_ONE_TIME,
        p_inheritance_info: std::ptr::null(),
    };
    if (v.f.begin)(v.cmd, &begin,) != 0
    {
        return false;
    }
    let range = VkImageSubresourceRange {
        aspect_mask:      ASPECT_COLOR,
        base_mip_level:   0,
        level_count:      1,
        base_array_layer: 0,
        layer_count:      1,
    };
    let to_dst = VkImageMemoryBarrier {
        s_type: S_IMAGE_MEMORY_BARRIER,
        p_next: std::ptr::null(),
        src_access_mask: 0,
        dst_access_mask: ACCESS_TRANSFER_WRITE,
        old_layout: LAYOUT_UNDEFINED,
        new_layout: LAYOUT_TRANSFER_DST,
        src_queue_family_index: WHOLE_SIZE,
        dst_queue_family_index: WHOLE_SIZE,
        image,
        subresource_range: range,
    };
    (v.f.cmd_barrier)(v.cmd, STAGE_TOP, STAGE_TRANSFER, 0, 0, std::ptr::null(), 0, std::ptr::null(), 1, &to_dst,);
    let color = [0.06f32, 0.07, 0.12, 1.0,];
    (v.f.cmd_clear)(v.cmd, image, LAYOUT_TRANSFER_DST, color.as_ptr(), 1, &range,);
    let to_present = VkImageMemoryBarrier {
        s_type: S_IMAGE_MEMORY_BARRIER,
        p_next: std::ptr::null(),
        src_access_mask: ACCESS_TRANSFER_WRITE,
        dst_access_mask: 0,
        old_layout: LAYOUT_TRANSFER_DST,
        new_layout: LAYOUT_PRESENT,
        src_queue_family_index: WHOLE_SIZE,
        dst_queue_family_index: WHOLE_SIZE,
        image,
        subresource_range: range,
    };
    (v.f.cmd_barrier)(v.cmd, STAGE_TRANSFER, STAGE_BOTTOM, 0, 0, std::ptr::null(), 0, std::ptr::null(), 1, &to_present,);
    let _ = (v.f.end)(v.cmd,);

    let waits = [v.sem_avail,];
    let signals = [v.sem_done,];
    let cmds = [v.cmd,];
    let stage = STAGE_TRANSFER;
    let submit = VkSubmitInfo {
        s_type:                 S_SUBMIT,
        p_next:                 std::ptr::null(),
        wait_semaphore_count:   1,
        p_wait_semaphores:      waits.as_ptr(),
        p_wait_dst_stage_mask:  &stage,
        command_buffer_count:   1,
        p_command_buffers:      cmds.as_ptr(),
        signal_semaphore_count: 1,
        p_signal_semaphores:    signals.as_ptr(),
    };
    if (v.f.submit)(v.queue, 1, &submit, v.fence,) != 0
    {
        return false;
    }
    let _ = (v.f.wait_for_fence)(v.device, 1, &v.fence, 1, 5_000_000_000,);

    let scs = [v.swapchain,];
    let present = VkPresentInfoKHR {
        s_type:               S_PRESENT,
        p_next:               std::ptr::null(),
        wait_semaphore_count: 1,
        p_wait_semaphores:    signals.as_ptr(),
        swapchain_count:      1,
        p_swapchains:         scs.as_ptr(),
        p_image_indices:      &idx,
        p_results:            std::ptr::null_mut(),
    };
    (v.f.present)(v.queue, &present,) == 0
}

#[test]
fn vulkan_survives_window_transitions() {
    unsafe {
        let mut win = match support::TestWindow::create("vk", 1280, 720,)
        {
            Some(w,) => w,
            None =>
            {
                eprintln!("[vk] window creation failed, skipping");
                return;
            }
        };
        eprintln!("[vk] step1 window ok");
        let module = support::LoadLibraryA(b"vulkan-1.dll\0".as_ptr(),);
        if module == 0
        {
            eprintln!("[vk] vulkan-1.dll not available, skipping");
            return;
        }
        let counter = support::Counter::new();
        nany::init("test_vk", Box::new(support::SceneProvider::new(counter.clone(),),),);
        eprintln!("[vk] step2 module={module}");
        let create_instance: FnCreateInstance = std::mem::transmute(sym(module, b"vkCreateInstance\0",),);
        let gipa: PfnGpa = std::mem::transmute(sym(module, b"vkGetInstanceProcAddr\0",),);

        let app_name = b"nany_test\0";
        let engine = b"nany\0";
        let app = VkApplicationInfo {
            s_type:              S_APP,
            p_next:              std::ptr::null(),
            p_application_name:  app_name.as_ptr(),
            application_version: 1,
            p_engine_name:       engine.as_ptr(),
            engine_version:      1,
            api_version:         (1 << 22) | (1 << 12),
        };
        let exts: [*const u8; 2] = [b"VK_KHR_surface\0".as_ptr(), b"VK_KHR_win32_surface\0".as_ptr(),];
        let info = VkInstanceCreateInfo {
            s_type: S_INSTANCE,
            p_next: std::ptr::null(),
            flags: 0,
            p_application_info: &app,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: 2,
            pp_enabled_extension_names: exts.as_ptr(),
        };
        let mut instance: VkHandle = 0;
        if create_instance(&info, std::ptr::null(), &mut instance,) != 0
        {
            eprintln!("[vk] vkCreateInstance failed, skipping");
            return;
        }

        eprintln!("[vk] step3 instance={instance}");
        let create_surface: FnCreateSurface =
            std::mem::transmute(load(module, instance, gipa, b"vkCreateWin32SurfaceKHR\0",),);
        let inst_h = support::GetModuleHandleW(std::ptr::null(),);
        let sci = VkWin32SurfaceCreateInfoKHR {
            s_type:    S_WIN32_SURFACE,
            p_next:    std::ptr::null(),
            flags:     0,
            hinstance: inst_h,
            hwnd:      win.hwnd,
        };
        let mut surface: VkHandle = 0;
        if create_surface(instance, &sci, std::ptr::null(), &mut surface,) != 0
        {
            eprintln!("[vk] surface creation failed, skipping");
            return;
        }

        eprintln!("[vk] step4 surface={surface}");
        let enum_phys: FnEnumPhys =
            std::mem::transmute(load(module, instance, gipa, b"vkEnumeratePhysicalDevices\0",),);
        let mut count = 1u32;
        let mut phys: VkHandle = 0;
        if enum_phys(instance, &mut count, &mut phys,) != 0 || phys == 0
        {
            eprintln!("[vk] no physical device, skipping");
            return;
        }
        let get_queue_props: FnGetQueueProps =
            std::mem::transmute(load(module, instance, gipa, b"vkGetPhysicalDeviceQueueFamilyProperties\0",),);
        let mut qcount = 0u32;
        get_queue_props(phys, &mut qcount, std::ptr::null_mut(),);
        let mut props = vec![VkQueueFamilyProperties::default(); qcount.max(1) as usize];
        get_queue_props(phys, &mut qcount, props.as_mut_ptr(),);
        let Some(queue_family,) = props.iter().position(|p| p.queue_flags & QUEUE_GRAPHICS != 0,)
        else
        {
            eprintln!("[vk] no graphics queue, skipping");
            return;
        };

        eprintln!("[vk] step5 phys={phys} qf={queue_family}");
        let create_device: FnCreateDevice = std::mem::transmute(load(module, instance, gipa, b"vkCreateDevice\0",),);
        let prio = [1.0f32,];
        let qinfo = VkDeviceQueueCreateInfo {
            s_type:             S_DEVICE_QUEUE,
            p_next:             std::ptr::null(),
            flags:              0,
            queue_family_index: queue_family as u32,
            queue_count:        1,
            p_queue_priorities: prio.as_ptr(),
        };
        let exts_dev: [*const u8; 1] = [b"VK_KHR_swapchain\0".as_ptr(),];
        let dinfo = VkDeviceCreateInfo {
            s_type: S_DEVICE,
            p_next: std::ptr::null(),
            flags: 0,
            queue_create_info_count: 1,
            p_queue_create_infos: &qinfo,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: 1,
            pp_enabled_extension_names: exts_dev.as_ptr(),
            p_enabled_features: std::ptr::null(),
        };
        let mut device: VkHandle = 0;
        if create_device(phys, &dinfo, std::ptr::null(), &mut device,) != 0
        {
            eprintln!("[vk] vkCreateDevice failed, skipping");
            return;
        }
        let get_device_queue: FnGetDeviceQueue =
            std::mem::transmute(load(module, instance, gipa, b"vkGetDeviceQueue\0",),);
        let mut queue: VkHandle = 0;
        get_device_queue(device, queue_family as u32, 0, &mut queue,);

        eprintln!("[vk] step6 device={device} queue={queue}");
        let f = VkFns {
            create_surface,
            get_caps: std::mem::transmute(
                load(module, instance, gipa, b"vkGetPhysicalDeviceSurfaceCapabilitiesKHR\0",),
            ),
            get_formats: std::mem::transmute(load(module, instance, gipa, b"vkGetPhysicalDeviceSurfaceFormatsKHR\0",),),
            get_queue_props,
            create_device,
            get_device_queue,
            create_swapchain: std::mem::transmute(load(module, instance, gipa, b"vkCreateSwapchainKHR\0",),),
            get_images: std::mem::transmute(load(module, instance, gipa, b"vkGetSwapchainImagesKHR\0",),),
            destroy_swapchain: std::mem::transmute(load(module, instance, gipa, b"vkDestroySwapchainKHR\0",),),
            create_pool: std::mem::transmute(load(module, instance, gipa, b"vkCreateCommandPool\0",),),
            allocate: std::mem::transmute(load(module, instance, gipa, b"vkAllocateCommandBuffers\0",),),
            begin: std::mem::transmute(load(module, instance, gipa, b"vkBeginCommandBuffer\0",),),
            end: std::mem::transmute(load(module, instance, gipa, b"vkEndCommandBuffer\0",),),
            reset_cmd: std::mem::transmute(load(module, instance, gipa, b"vkResetCommandBuffer\0",),),
            cmd_barrier: std::mem::transmute(load(module, instance, gipa, b"vkCmdPipelineBarrier\0",),),
            cmd_clear: std::mem::transmute(load(module, instance, gipa, b"vkCmdClearColorImage\0",),),
            acquire: std::mem::transmute(load(module, instance, gipa, b"vkAcquireNextImageKHR\0",),),
            submit: std::mem::transmute(load(module, instance, gipa, b"vkQueueSubmit\0",),),
            present: std::mem::transmute(load(module, instance, gipa, b"vkQueuePresentKHR\0",),),
            create_semaphore: std::mem::transmute(load(module, instance, gipa, b"vkCreateSemaphore\0",),),
            create_fence: std::mem::transmute(load(module, instance, gipa, b"vkCreateFence\0",),),
            wait_for_fence: std::mem::transmute(load(module, instance, gipa, b"vkWaitForFences\0",),),
            reset_fence: std::mem::transmute(load(module, instance, gipa, b"vkResetFences\0",),),
            device_wait_idle: std::mem::transmute(load(module, instance, gipa, b"vkDeviceWaitIdle\0",),),
        };

        eprintln!("[vk] step7 fns ready");
        let pool_info = VkCommandPoolCreateInfo {
            s_type:             S_COMMAND_POOL,
            p_next:             std::ptr::null(),
            flags:              CMD_POOL_RESET,
            queue_family_index: queue_family as u32,
        };
        let mut pool: VkHandle = 0;
        if (f.create_pool)(device, &pool_info, std::ptr::null(), &mut pool,) != 0
        {
            eprintln!("[vk] command pool failed, skipping");
            return;
        }
        let alloc = VkCommandBufferAllocateInfo {
            s_type:               S_COMMAND_BUFFER_ALLOCATE,
            p_next:               std::ptr::null(),
            command_pool:         pool,
            level:                LEVEL_PRIMARY,
            command_buffer_count: 1,
        };
        let mut cmd: VkHandle = 0;
        let _ = (f.allocate)(device, &alloc, &mut cmd,);

        let sem_info = VkSemaphoreCreateInfo { s_type: S_SEMAPHORE, p_next: std::ptr::null(), flags: 0, };
        let mut sem_avail: VkHandle = 0;
        let mut sem_done: VkHandle = 0;
        let _ = (f.create_semaphore)(device, &sem_info, std::ptr::null(), &mut sem_avail,);
        let _ = (f.create_semaphore)(device, &sem_info, std::ptr::null(), &mut sem_done,);
        let fence_info = VkFenceCreateInfo { s_type: S_FENCE, p_next: std::ptr::null(), flags: 1, };
        let mut fence: VkHandle = 0;
        let _ = (f.create_fence)(device, &fence_info, std::ptr::null(), &mut fence,);

        let (w, h,) = win.client_size();
        let mut v = Vk {
            instance,
            surface,
            phys,
            device,
            queue,
            queue_family: queue_family as u32,
            swapchain: 0,
            images: Vec::new(),
            extent: VkExtent2D { width: w, height: h, },
            format: 0,
            color_space: 0,
            pool,
            cmd,
            sem_avail,
            sem_done,
            fence,
            f,
        };
        if !create_swapchain(&mut v, 0, VkExtent2D { width: w, height: h, },)
        {
            eprintln!("[vk] swapchain creation failed, skipping");
            return;
        }

        eprintln!("[vk] step8 swapchain={} images={}", v.swapchain, v.images.len());

        let mut last = (w, h,);
        for step in support::window_steps()
        {
            win.pump(8,);
            (step.apply)(&mut win,);
            let size = win.client_size();
            if size != last
            {
                (v.f.device_wait_idle)(v.device,);
                let old = v.swapchain;
                let ext = VkExtent2D { width: size.0, height: size.1, };
                if create_swapchain(&mut v, old, ext,)
                {
                    (v.f.destroy_swapchain)(v.device, old, std::ptr::null(),);
                }
                last = size;
            }
            let before = counter.frames();
            for _ in 0..24
            {
                frame(&mut v,);
                win.pump(1,);
                std::thread::sleep(std::time::Duration::from_millis(4,),);
            }
            let after = counter.frames();
            support::report("vk", step.name, before, after,);
            assert!(after >= before, "vk overlay frame counter went backwards on {}", step.name);
        }
        assert!(counter.frames() > 0, "vk overlay never drew a frame");
        let _ = &v.instance;
        let _ = &v.color_space;
        let _ = &v.format;
    }
}
