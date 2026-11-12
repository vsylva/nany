#![allow(non_snake_case)]

//! Runtime verification of the raw vtable slots this crate hooks.
//!
//! windows-rs computes every COM call through the generated `*_Vtbl` struct,
//! so reading a live object's vtable and locating the binding field address
//! proves which slot is which function on this machine — no guessing.

mod support;

use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::{BOOL, HWND},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{
                D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11CreateDeviceAndSwapChain, ID3D11Device,
                ID3D11DeviceContext,
            },
            Direct3D12::{
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
                D3D12CreateDevice, ID3D12CommandQueue, ID3D12CommandQueue_Vtbl, ID3D12Device,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_RATIONAL,
                    DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_PRESENT_PARAMETERS,
                DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_DISCARD,
                DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2, IDXGIFactory2_Vtbl,
                IDXGISwapChain, IDXGISwapChain_Vtbl, IDXGISwapChain1, IDXGISwapChain1_Vtbl, IDXGISwapChain3,
                IDXGISwapChain3_Vtbl,
            },
        },
    },
    core::Interface,
};

unsafe fn slot(obj: *const c_void, idx: usize,) -> usize {
    let vtbl = *(obj as *const *const usize);
    assert!(!vtbl.is_null());
    *vtbl.add(idx,)
}

unsafe fn find_slot(obj: *const c_void, want: usize, max: usize,) -> usize {
    for i in 0..max
    {
        if slot(obj, i,) == want
        {
            return i;
        }
    }
    usize::MAX
}

/// Function pointer stored at `vtbl + offset` in a live object's vtable.
unsafe fn fn_at(vtbl: usize, off: usize,) -> usize {
    *((vtbl + off) as *const usize)
}

unsafe fn make_legacy_swapchain(hwnd: HWND,) -> IDXGISwapChain {
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
        OutputWindow: hwnd,
        Windowed:     BOOL(1,),
        SwapEffect:   DXGI_SWAP_EFFECT_DISCARD,
        Flags:        0,
    };
    let mut sc: Option<IDXGISwapChain,> = None;
    let mut dev: Option<ID3D11Device,> = None;
    let mut ctx: Option<ID3D11DeviceContext,> = None;
    let mut level = D3D_FEATURE_LEVEL(0,);
    let hr = D3D11CreateDeviceAndSwapChain(
        None::<&windows::Win32::Graphics::Dxgi::IDXGIAdapter,>,
        D3D_DRIVER_TYPE_HARDWARE,
        windows::Win32::Foundation::HMODULE::default(),
        D3D11_CREATE_DEVICE_FLAG(0,),
        None,
        D3D11_SDK_VERSION,
        Some(&desc,),
        Some(&mut sc as *mut _,),
        Some(&mut dev as *mut _,),
        Some(&mut level as *mut _,),
        Some(&mut ctx as *mut _,),
    );
    assert!(hr.is_ok(), "D3D11CreateDeviceAndSwapChain failed: {hr:?}");
    drop(ctx,);
    drop(dev,);
    sc.unwrap()
}

unsafe fn make_flip_swapchain(hwnd: HWND,) -> (IDXGISwapChain3, ID3D12CommandQueue,) {
    let mut dev: Option<ID3D12Device,> = None;
    D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut dev as *mut _ as *mut _,).expect("D3D12CreateDevice",);
    let dev = dev.unwrap();
    let queue: ID3D12CommandQueue = dev
        .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
            Type:     D3D12_COMMAND_LIST_TYPE_DIRECT,
            Priority: 0,
            Flags:    D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0,
        },)
        .expect("CreateCommandQueue",);

    let factory: IDXGIFactory2 =
        CreateDXGIFactory2::<IDXGIFactory2,>(DXGI_CREATE_FACTORY_FLAGS(0,),).expect("CreateDXGIFactory2",);
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width:       8,
        Height:      8,
        Format:      DXGI_FORMAT_R8G8B8A8_UNORM,
        Stereo:      BOOL(0,),
        SampleDesc:  DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling:     DXGI_SCALING_STRETCH,
        SwapEffect:  DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode:   DXGI_ALPHA_MODE_IGNORE,
        Flags:       0,
    };
    let sc: IDXGISwapChain1 =
        factory.CreateSwapChainForHwnd(&queue, hwnd, &desc, None, None,).expect("CreateSwapChainForHwnd",);
    (sc.cast().expect("cast IDXGISwapChain3",), queue,)
}

#[test]
fn dxgi_vtable_slots_are_what_we_hook() {
    unsafe {
        let win = support::TestWindow::create("vtbl", 64, 64,).expect("test window",);
        let hwnd = HWND(win.hwnd,);

        // The legacy bitblt-model swapchain is what hook/dxgi.rs probes; the
        // flip-model one is what D3D12 games actually present through.
        let sc11 = make_legacy_swapchain(hwnd,);
        let (sc3, queue,) = make_flip_swapchain(hwnd,);

        let v11 = *(sc11.as_raw() as *const usize);
        let vflip = *(sc3.as_raw() as *const usize);

        // Binding function pointers read from each live vtable through the
        // windows-rs generated Vtbl struct layout.
        let present11 = fn_at(v11, std::mem::offset_of!(IDXGISwapChain_Vtbl, Present),);
        let resize11 = fn_at(v11, std::mem::offset_of!(IDXGISwapChain_Vtbl, ResizeBuffers),);
        let present1_11 = fn_at(v11, std::mem::offset_of!(IDXGISwapChain1_Vtbl, Present1),);
        let present_flip = fn_at(vflip, std::mem::offset_of!(IDXGISwapChain_Vtbl, Present),);
        let resize_flip = fn_at(vflip, std::mem::offset_of!(IDXGISwapChain_Vtbl, ResizeBuffers),);
        let present1_flip = fn_at(vflip, std::mem::offset_of!(IDXGISwapChain1_Vtbl, Present1),);
        let resize1_flip = fn_at(vflip, std::mem::offset_of!(IDXGISwapChain3_Vtbl, ResizeBuffers1),);

        let rows = [
            ("Present (legacy)", find_slot(sc11.as_raw(), present11, 48,), present11,),
            ("ResizeBuffers (legacy)", find_slot(sc11.as_raw(), resize11, 48,), resize11,),
            ("Present1 (legacy)", find_slot(sc11.as_raw(), present1_11, 48,), present1_11,),
            ("Present (flip)", find_slot(sc3.as_raw(), present_flip, 48,), present_flip,),
            ("ResizeBuffers (flip)", find_slot(sc3.as_raw(), resize_flip, 48,), resize_flip,),
            ("Present1 (flip)", find_slot(sc3.as_raw(), present1_flip, 48,), present1_flip,),
            ("ResizeBuffers1 (flip)", find_slot(sc3.as_raw(), resize1_flip, 48,), resize1_flip,),
        ];
        for (name, s, addr,) in rows
        {
            println!("{name:<24} binding 0x{addr:016X} -> slot {s}");
        }
        println!(
            "legacy/flip share Present={} ResizeBuffers={} Present1={}",
            present11 == present_flip,
            resize11 == resize_flip,
            present1_11 == present1_flip
        );

        // The numbers hook/dxgi.rs must use (assert against live objects).
        assert_eq!(find_slot(sc11.as_raw(), present11, 48), 8, "Present slot");
        assert_eq!(find_slot(sc11.as_raw(), resize11, 48), 13, "ResizeBuffers slot");
        assert_eq!(find_slot(sc11.as_raw(), present1_11, 48), 22, "Present1 slot");
        assert_eq!(find_slot(sc3.as_raw(), present_flip, 48), 8, "Present slot (flip)");
        assert_eq!(find_slot(sc3.as_raw(), resize_flip, 48), 13, "ResizeBuffers slot (flip)");
        assert_eq!(find_slot(sc3.as_raw(), present1_flip, 48), 22, "Present1 slot (flip)");
        // IDXGISwapChain1 has 4 extra methods past GetRestrictToOutput
        // (Set/GetBackgroundColor, Set/GetRotation), so ResizeBuffers1 lands
        // on slot 39, not the 36-ish a naive count would suggest.
        assert_eq!(find_slot(sc3.as_raw(), resize1_flip, 48), 39, "ResizeBuffers1 slot (flip)");
        if sc11.cast::<IDXGISwapChain3>().is_ok()
        {
            assert_eq!(slot(sc11.as_raw(), 39), resize1_flip, "ResizeBuffers1 slot (legacy probe object)");
        }

        // Factory probe slot: CreateSwapChainForHwnd on IDXGIFactory2.
        let factory: IDXGIFactory2 =
            CreateDXGIFactory2::<IDXGIFactory2,>(DXGI_CREATE_FACTORY_FLAGS(0,),).expect("factory",);
        let vfac = *(factory.as_raw() as *const usize);
        let cfh = fn_at(vfac, std::mem::offset_of!(IDXGIFactory2_Vtbl, CreateSwapChainForHwnd),);
        let cfh_slot = find_slot(factory.as_raw(), cfh, 32,);
        println!("CreateSwapChainForHwnd    binding 0x{cfh:016X} -> slot {cfh_slot}");
        assert_eq!(cfh_slot, 15, "CreateSwapChainForHwnd slot");

        // ExecuteCommandLists slot on the D3D12 queue.
        let vq = *(queue.as_raw() as *const usize);
        let ecl = fn_at(vq, std::mem::offset_of!(ID3D12CommandQueue_Vtbl, ExecuteCommandLists),);
        let ecl_slot = find_slot(queue.as_raw(), ecl, 32,);
        println!("ExecuteCommandLists       binding 0x{ecl:016X} -> slot {ecl_slot}");
        assert_eq!(ecl_slot, 10, "ExecuteCommandLists slot");

        // Behavioral proof on fresh flip-model objects (one resize per object,
        // no held back-buffer refs). Each slot is invoked exactly the way the
        // ABI of the function living there expects.
        type ResizeFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32,) -> i32;
        type Resize1Fn =
            unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32, *const u32, *const *mut c_void,) -> i32;
        type Present1Fn = unsafe extern "system" fn(*mut c_void, u32, u32, *const DXGI_PRESENT_PARAMETERS,) -> i32;

        let fmt = DXGI_FORMAT_R8G8B8A8_UNORM.0 as u32;
        // Flip-model swapchains bind exclusively to their window, so each
        // behavioral call gets a fresh window + swapchain.
        let win_rb = support::TestWindow::create("vtblrb", 64, 64,).expect("rb window",);
        let (sc_rb, _,) = make_flip_swapchain(HWND(win_rb.hwnd,),);
        let hr = {
            let f: ResizeFn = std::mem::transmute(slot(sc_rb.as_raw(), 13,),);
            f(sc_rb.as_raw(), 2, 8, 8, fmt, 0,)
        };
        println!("flip slot 13 called as ResizeBuffers   -> hr=0x{hr:08X}");
        assert_eq!(hr, 0, "slot 13 did not behave as ResizeBuffers");

        let win_rb1 = support::TestWindow::create("vtblrb1", 64, 64,).expect("rb1 window",);
        let (sc_rb1, queue1,) = make_flip_swapchain(HWND(win_rb1.hwnd,),);
        let hr = {
            let f: Resize1Fn = std::mem::transmute(slot(sc_rb1.as_raw(), 39,),);
            // pCreationNodeMask and ppPresentQueue are per-back-buffer arrays
            // sized by BufferCount.
            let mask = [0u32, 0u32,];
            let queues = [queue1.as_raw(), queue1.as_raw(),];
            f(sc_rb1.as_raw(), 2, 8, 8, fmt, 0, mask.as_ptr(), queues.as_ptr(),)
        };
        println!("flip slot 39 called as ResizeBuffers1  -> hr=0x{hr:08X}");
        assert_eq!(hr, 0, "slot 39 did not behave as ResizeBuffers1");

        let win_p1 = support::TestWindow::create("vtblp1", 64, 64,).expect("p1 window",);
        let (sc_p1, _,) = make_flip_swapchain(HWND(win_p1.hwnd,),);
        let hr = {
            let f: Present1Fn = std::mem::transmute(slot(sc_p1.as_raw(), 22,),);
            let params = DXGI_PRESENT_PARAMETERS::default();
            f(sc_p1.as_raw(), 0, 0, &params,)
        };
        println!("flip slot 22 called as Present1        -> hr=0x{hr:08X}");
        assert_eq!(hr, 0, "slot 22 did not behave as Present1");

        let _ = sc3.Present(0, DXGI_PRESENT(0,),);
    }
}
