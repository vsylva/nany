use std::{
    ffi::c_void,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
    },
};

use windows::{
    Win32::Graphics::{
        Direct3D12::{D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, ID3D12CommandQueue, ID3D12Device},
        Dxgi::{
            Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_RATIONAL, DXGI_SAMPLE_DESC},
            DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2,
            IDXGISwapChain, IDXGISwapChain1, IDXGISwapChain3,
        },
    },
    core::{GUID, Interface},
};

use super::detour;
use crate::{log_error, log_info, log_warn, overlay, win};

static INSTALLED: AtomicBool = AtomicBool::new(false,);
static QUEUE: AtomicUsize = AtomicUsize::new(0,);

static ORIG_PRESENT: AtomicUsize = AtomicUsize::new(0,);
static ORIG_PRESENT1: AtomicUsize = AtomicUsize::new(0,);
static ORIG_RESIZE: AtomicUsize = AtomicUsize::new(0,);
static ORIG_RESIZE1: AtomicUsize = AtomicUsize::new(0,);
static ORIG_EXEC: AtomicUsize = AtomicUsize::new(0,);

static LAST_NODE_MASK: AtomicU32 = AtomicU32::new(u32::MAX,);

static TARGETS: Mutex<Vec<usize,>,> = Mutex::new(Vec::new(),);

const DXGI_ERROR_DEVICE_REMOVED: i32 = 0x887A_0005u32 as i32;
const DXGI_ERROR_DEVICE_HUNG: i32 = 0x887A_0006u32 as i32;
const DXGI_ERROR_DEVICE_RESET: i32 = 0x887A_0007u32 as i32;

pub unsafe fn queue() -> Option<usize,> {
    match QUEUE.load(Ordering::Acquire,)
    {
        0 => None,
        q => Some(q,),
    }
}

pub unsafe fn node_mask() -> Option<u32,> {
    match LAST_NODE_MASK.load(Ordering::Acquire,)
    {
        u32::MAX => None,
        v => Some(v,),
    }
}

pub unsafe fn set_queue(q: usize,) {
    QUEUE.store(q, Ordering::Release,);
}

pub unsafe fn is_device_lost(hr: i32,) -> bool {
    hr == DXGI_ERROR_DEVICE_REMOVED || hr == DXGI_ERROR_DEVICE_RESET || hr == DXGI_ERROR_DEVICE_HUNG
}

pub unsafe fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire,)
}

type PresentFn = unsafe extern "system" fn(*mut c_void, u32, u32,) -> i32;
type Present1Fn = unsafe extern "system" fn(*mut c_void, u32, *const c_void,) -> i32;
type ResizeFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32,) -> i32;
type Resize1Fn =
    unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32, *const u32, *const *mut c_void,) -> i32;
type ExecFn = unsafe extern "system" fn(*mut c_void, u32, *const *mut c_void,);

unsafe extern "system" fn detour_present(this: *mut c_void, sync: u32, flags: u32,) -> i32 {
    overlay::on_present(this,);
    let orig = ORIG_PRESENT.load(Ordering::Acquire,);
    if orig == 0
    {
        return 0;
    }
    let f: PresentFn = std::mem::transmute(orig,);
    let hr = f(this, sync, flags,);
    if is_device_lost(hr,)
    {
        overlay::on_device_lost(this,);
    }
    hr
}

unsafe extern "system" fn detour_present1(this: *mut c_void, sync: u32, params: *const c_void,) -> i32 {
    overlay::on_present(this,);
    let orig = ORIG_PRESENT1.load(Ordering::Acquire,);
    if orig == 0
    {
        return 0;
    }
    let f: Present1Fn = std::mem::transmute(orig,);
    let hr = f(this, sync, params,);
    if is_device_lost(hr,)
    {
        overlay::on_device_lost(this,);
    }
    hr
}

unsafe extern "system" fn detour_resize(this: *mut c_void, count: u32, w: u32, h: u32, fmt: u32, flags: u32,) -> i32 {
    overlay::on_resize(this,);
    let orig = ORIG_RESIZE.load(Ordering::Acquire,);
    if orig == 0
    {
        return 0;
    }
    let f: ResizeFn = std::mem::transmute(orig,);
    f(this, count, w, h, fmt, flags,)
}

unsafe fn or_node_masks(masks: *const u32, count: u32,) -> u32 {
    if masks.is_null() || count == 0
    {
        return 0;
    }

    let n = count.min(64,) as usize;
    let mut or = 0u32;
    for i in 0..n
    {
        or |= *masks.add(i,);
    }
    or
}

unsafe fn resize_buffer_count(this: *mut c_void, count: u32,) -> u32 {
    if count != 0
    {
        return count;
    }
    if this.is_null()
    {
        return 0;
    }

    let sc = std::mem::ManuallyDrop::new(IDXGISwapChain::from_raw(this,),);
    match sc.cast::<IDXGISwapChain3>()
    {
        Ok(sc3,) => match sc3.GetDesc1()
        {
            Ok(d,) => d.BufferCount.min(16,),
            Err(_,) => 0,
        },
        Err(_,) => 0,
    }
}

unsafe extern "system" fn detour_resize1(
    this: *mut c_void,
    count: u32,
    w: u32,
    h: u32,
    fmt: u32,
    flags: u32,
    node_mask: *const u32,
    queues: *const *mut c_void,
) -> i32 {
    let n = resize_buffer_count(this, count,);
    let mask = or_node_masks(node_mask, n,);
    if LAST_NODE_MASK.swap(mask, Ordering::AcqRel,) != mask
    {
        let mut list = String::new();
        if !node_mask.is_null()
        {
            for i in 0..n.min(8,) as usize
            {
                if i > 0
                {
                    list.push(',',);
                }
                list.push_str(&format!("{:#x}", *node_mask.add(i)),);
            }
        }
        log_info!("hook", format!("ResizeBuffers1 {w}x{h} count={count} actual={n} nodes=[{list}] or={mask:#x}"));

        if mask != 0 && mask & 1 == 0
        {
            log_info!("hook", "node mask excludes node 0, overlay only draws on node 0");
        }
    }
    overlay::on_resize(this,);
    let orig = ORIG_RESIZE1.load(Ordering::Acquire,);
    if orig == 0
    {
        return 0;
    }
    let f: Resize1Fn = std::mem::transmute(orig,);
    f(this, count, w, h, fmt, flags, node_mask, queues,)
}

unsafe extern "system" fn detour_exec(this: *mut c_void, num: u32, lists: *const *mut c_void,) {
    if QUEUE.load(Ordering::Relaxed,) == 0 && is_direct_queue(this,)
    {
        set_queue(this as usize,);
        log_info!("hook", format!("d3d12 queue {:#x}", this as usize));
    }
    let orig = ORIG_EXEC.load(Ordering::Acquire,);
    if orig == 0
    {
        return;
    }
    let f: ExecFn = std::mem::transmute(orig,);
    f(this, num, lists,)
}

unsafe fn is_direct_queue(q: *mut c_void,) -> bool {
    if q.is_null()
    {
        return false;
    }
    let queue = std::mem::ManuallyDrop::new(ID3D12CommandQueue::from_raw(q,),);
    let desc: D3D12_COMMAND_QUEUE_DESC = queue.GetDesc();
    desc.Type == D3D12_COMMAND_LIST_TYPE_DIRECT
}

unsafe fn vtbl_entry(obj: *mut c_void, idx: usize,) -> usize {
    if obj.is_null()
    {
        return 0;
    }
    let vtbl = *(obj as *const *const usize);
    if vtbl.is_null()
    {
        return 0;
    }
    *vtbl.add(idx,)
}

unsafe fn hook_one(name: &str, target: usize, detour_fn: usize, slot: &AtomicUsize,) -> bool {
    crate::hook::install_one("hook", name, target, detour_fn, slot, &TARGETS,)
}

type CreateDeviceAndSwapChainFn = unsafe extern "system" fn(
    *mut c_void,
    u32,
    *mut c_void,
    u32,
    *const i32,
    u32,
    u32,
    *const DXGI_SWAP_CHAIN_DESC,
    *mut *mut c_void,
    *mut *mut c_void,
    *mut i32,
    *mut *mut c_void,
) -> i32;

type D3D12CreateDeviceFn = unsafe extern "system" fn(*mut c_void, u32, *const GUID, *mut *mut c_void,) -> i32;

unsafe fn probe_swapchain_d3d12() -> *mut c_void {
    let class = c"nany_probe_cls2";
    let title = c"nany probe2";
    let hwnd = win::create_dummy_window(class, title,);
    if hwnd.is_null()
    {
        return std::ptr::null_mut();
    }

    let raw_dev = crate::api::resolve("d3d12.dll", b"D3D12CreateDevice",);
    if raw_dev == 0
    {
        win::destroy_window(hwnd, class,);
        return std::ptr::null_mut();
    }
    let create_dev: D3D12CreateDeviceFn = std::mem::transmute(raw_dev,);
    let mut dev: *mut c_void = std::ptr::null_mut();
    if create_dev(std::ptr::null_mut(), 0xB000, &ID3D12Device::IID, &mut dev,) < 0 || dev.is_null()
    {
        win::destroy_window(hwnd, class,);
        return std::ptr::null_mut();
    }
    let device = ID3D12Device::from_raw(dev,);
    let qdesc = D3D12_COMMAND_QUEUE_DESC::default();
    let queue = match device.CreateCommandQueue::<ID3D12CommandQueue>(&qdesc,)
    {
        Ok(q,) => q,
        Err(_,) =>
        {
            win::destroy_window(hwnd, class,);
            return std::ptr::null_mut();
        }
    };

    let raw_f = crate::api::resolve("dxgi.dll", b"CreateDXGIFactory2",);
    if raw_f == 0
    {
        win::destroy_window(hwnd, class,);
        return std::ptr::null_mut();
    }
    let create_f: unsafe extern "system" fn(u32, *const GUID, *mut *mut c_void,) -> i32 = std::mem::transmute(raw_f,);
    let mut f_raw: *mut c_void = std::ptr::null_mut();
    if create_f(0, &IDXGIFactory2::IID, &mut f_raw,) < 0 || f_raw.is_null()
    {
        win::destroy_window(hwnd, class,);
        return std::ptr::null_mut();
    }

    type CreateForHwndFn = unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        windows::Win32::Foundation::HWND,
        *const windows::Win32::Graphics::Dxgi::DXGI_SWAP_CHAIN_DESC1,
        *const c_void,
        *const c_void,
        *mut *mut c_void,
    ) -> i32;
    let slot15 = vtbl_entry(f_raw, 15,);
    let mut sc: *mut c_void = std::ptr::null_mut();
    if slot15 != 0
    {
        let create_sc: CreateForHwndFn = std::mem::transmute(slot15,);
        let d1 = windows::Win32::Graphics::Dxgi::DXGI_SWAP_CHAIN_DESC1 {
            Width:       8,
            Height:      8,
            Format:      DXGI_FORMAT_R8G8B8A8_UNORM,
            Stereo:      windows::Win32::Foundation::BOOL(0,),
            SampleDesc:  DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling:     windows::Win32::Graphics::Dxgi::DXGI_SCALING_STRETCH,
            SwapEffect:  windows::Win32::Graphics::Dxgi::DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode:   windows::Win32::Graphics::Dxgi::Common::DXGI_ALPHA_MODE_IGNORE,
            Flags:       0,
        };
        let hr = create_sc(
            f_raw,
            queue.as_raw(),
            windows::Win32::Foundation::HWND(hwnd,),
            &d1,
            std::ptr::null(),
            std::ptr::null(),
            &mut sc,
        );
        if hr < 0
        {
            log_error!("hook", format!("d3d12 swapchain probe failed hr=0x{hr:08X}"));
            sc = std::ptr::null_mut();
        }
    }

    if sc.is_null()
    {
        drop(IDXGIFactory2::from_raw(f_raw,),);
        win::destroy_window(hwnd, class,);
        return std::ptr::null_mut();
    }

    drop(IDXGIFactory2::from_raw(f_raw,),);
    win::destroy_window(hwnd, class,);
    sc
}

unsafe fn probe_swapchain() -> *mut c_void {
    let class = c"nany_probe_cls";
    let title = c"nany probe";
    let hwnd = win::create_dummy_window(class, title,);
    if hwnd.is_null()
    {
        log_error!("hook", "probe window creation failed (RegisterClassA/CreateWindowExA)");
        return std::ptr::null_mut();
    }
    let raw = crate::api::resolve("d3d11.dll", b"D3D11CreateDeviceAndSwapChain",);
    if raw == 0
    {
        win::destroy_window(hwnd, class,);
        log_warn!("hook", "d3d11.dll not loaded, skipping d3d11 probe");
        return std::ptr::null_mut();
    }

    let create: CreateDeviceAndSwapChainFn = std::mem::transmute(raw,);

    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc:   DXGI_MODE_DESC {
            Width:            8,
            Height:           8,
            RefreshRate:      DXGI_RATIONAL { Numerator: 60, Denominator: 1, },
            Format:           DXGI_FORMAT_R8G8B8A8_UNORM,
            ScanlineOrdering: Default::default(),
            Scaling:          Default::default(),
        },
        SampleDesc:   DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
        BufferUsage:  DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount:  1,
        OutputWindow: windows::Win32::Foundation::HWND(hwnd,),
        Windowed:     windows::Win32::Foundation::BOOL(1,),
        SwapEffect:   DXGI_SWAP_EFFECT_DISCARD,
        Flags:        0,
    };

    let mut sc: *mut c_void = std::ptr::null_mut();
    let hr = create(
        std::ptr::null_mut(),
        1,
        std::ptr::null_mut(),
        0,
        std::ptr::null(),
        0,
        7,
        &desc,
        &mut sc,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    win::destroy_window(hwnd, class,);
    if hr < 0 || sc.is_null()
    {
        log_error!("hook", format!("swapchain probe failed hr=0x{hr:08X}"));
        return std::ptr::null_mut();
    }
    sc
}

unsafe fn probe_queue() -> *mut c_void {
    let raw = crate::api::resolve("d3d12.dll", b"D3D12CreateDevice",);
    if raw == 0
    {
        log_warn!("hook", "d3d12.dll not loaded, skipping d3d12 queue probe");
        return std::ptr::null_mut();
    }
    let create: D3D12CreateDeviceFn = std::mem::transmute(raw,);
    let mut dev: *mut c_void = std::ptr::null_mut();
    let hr = create(std::ptr::null_mut(), 0xB000, &ID3D12Device::IID, &mut dev,);
    if hr < 0 || dev.is_null()
    {
        log_error!("hook", format!("d3d12 device probe failed hr=0x{hr:08X}"));
        return std::ptr::null_mut();
    }
    let device = ID3D12Device::from_raw(dev,);
    let desc = D3D12_COMMAND_QUEUE_DESC::default();
    match device.CreateCommandQueue::<ID3D12CommandQueue>(&desc,)
    {
        Ok(q,) =>
        {
            let p = q.as_raw();
            std::mem::forget(q,);
            p
        }
        Err(_,) => std::ptr::null_mut(),
    }
}

pub unsafe fn install() -> bool {
    if INSTALLED.swap(true, Ordering::SeqCst,)
    {
        return true;
    }
    crate::api::init();

    let mut ok = false;

    let mut sc = probe_swapchain();
    if sc.is_null()
    {
        sc = probe_swapchain_d3d12();
    }
    if !sc.is_null()
    {
        let probe = std::mem::ManuallyDrop::new(IDXGISwapChain::from_raw(sc,),);
        let has1 = probe.cast::<IDXGISwapChain1>().is_ok();
        let has3 = probe.cast::<IDXGISwapChain3>().is_ok();
        ok |= hook_one("Present", vtbl_entry(sc, 8,), detour_present as *const () as usize, &ORIG_PRESENT,);
        ok |= hook_one("ResizeBuffers", vtbl_entry(sc, 13,), detour_resize as *const () as usize, &ORIG_RESIZE,);
        if has1
        {
            ok |= hook_one("Present1", vtbl_entry(sc, 22,), detour_present1 as *const () as usize, &ORIG_PRESENT1,);
        }
        else
        {
            log_warn!("hook", "skipping Present1: swapchain has no IDXGISwapChain1");
        }
        if has3
        {
            ok |= hook_one("ResizeBuffers1", vtbl_entry(sc, 39,), detour_resize1 as *const () as usize, &ORIG_RESIZE1,);
        }
        else
        {
            log_warn!("hook", "skipping ResizeBuffers1: swapchain has no IDXGISwapChain3");
        }
        drop(IDXGISwapChain::from_raw(sc,),);
    }

    let q = probe_queue();
    if !q.is_null()
    {
        ok |= hook_one("ExecuteCommandLists", vtbl_entry(q, 10,), detour_exec as *const () as usize, &ORIG_EXEC,);
        drop(ID3D12CommandQueue::from_raw(q,),);
    }

    log_info!("hook", format!("inline hooks installed = {ok}"));
    ok
}

pub unsafe fn ensure() -> bool {
    if installed()
    {
        return true;
    }
    install()
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
    ORIG_PRESENT1.store(0, Ordering::Release,);
    ORIG_RESIZE.store(0, Ordering::Release,);
    ORIG_RESIZE1.store(0, Ordering::Release,);
    ORIG_EXEC.store(0, Ordering::Release,);
    LAST_NODE_MASK.store(u32::MAX, Ordering::Release,);
    INSTALLED.store(false, Ordering::Release,);
    log_warn!("hook", format!("circuit breaker: unloaded {} inline hook(s)", targets.len()));
}

pub unsafe fn _types(sc1: Option<&IDXGISwapChain1,>, sc3: Option<&IDXGISwapChain3,>,) {
    let _ = (sc1.map(|s| s.as_raw(),), sc3.map(|s| s.as_raw(),),);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_masks_are_ored() {
        unsafe {
            let same = [1u32, 1, 1,];
            assert_eq!(or_node_masks(same.as_ptr(), 3), 1);

            let two_nodes = [1u32, 2, 2,];
            assert_eq!(or_node_masks(two_nodes.as_ptr(), 3), 3);
        }
    }

    #[test]
    fn node_masks_degenerate() {
        unsafe {
            let one = [0x4u32,];
            assert_eq!(or_node_masks(std::ptr::null(), 4), 0);
            assert_eq!(or_node_masks(one.as_ptr(), 0), 0);
            assert_eq!(or_node_masks(one.as_ptr(), 1), 0x4);
        }
    }
}
