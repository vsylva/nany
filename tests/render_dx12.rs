#![allow(non_snake_case, clippy::missing_transmute_annotations)]

mod support;

use windows::{
    Win32::{
        Foundation::{BOOL, HWND, RECT},
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::*,
            Dxgi::{Common::*, *},
        },
    },
    core::Interface,
};

struct Dx12 {
    device:      ID3D12Device,
    queue:       ID3D12CommandQueue,
    swapchain:   IDXGISwapChain3,
    allocator:   ID3D12CommandAllocator,
    list:        ID3D12GraphicsCommandList,
    rtv_heap:    ID3D12DescriptorHeap,
    fence:       ID3D12Fence,
    fence_value: u64,
    rtv_size:    u32,
    frames:      usize,
}

impl Dx12 {
    unsafe fn new(hwnd: HWND, w: u32, h: u32,) -> Option<Self,> {
        let mut device: Option<ID3D12Device,> = None;
        if D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device as *mut _ as *mut _,).is_err()
        {
            return None;
        }
        let device = device?;

        let queue: ID3D12CommandQueue = device
            .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type:     D3D12_COMMAND_LIST_TYPE_DIRECT,
                Priority: 0,
                Flags:    D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            },)
            .ok()?;

        let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0,),).ok()?;
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width:       w,
            Height:      h,
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
        let sc1: IDXGISwapChain1 = factory.CreateSwapChainForHwnd(&queue, hwnd, &desc, None, None,).ok()?;
        let swapchain: IDXGISwapChain3 = sc1.cast().ok()?;

        let allocator: ID3D12CommandAllocator = device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT,).ok()?;
        let list: ID3D12GraphicsCommandList =
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None,).ok()?;
        list.Close().ok()?;

        let rtv_heap: ID3D12DescriptorHeap = device
            .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type:           D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: 2,
                Flags:          D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
                NodeMask:       0,
            },)
            .ok()?;
        let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE,).ok()?;

        Some(Self {
            device: device.clone(),
            queue,
            swapchain,
            allocator,
            list,
            rtv_heap,
            fence,
            fence_value: 0,
            rtv_size: device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV,),
            frames: 0,
        },)
    }

    unsafe fn frame(&mut self,) {
        let idx = self.swapchain.GetCurrentBackBufferIndex();
        let _ = self.allocator.Reset();
        let _ = self.list.Reset(&self.allocator, None,);

        let Ok(bb,) = self.swapchain.GetBuffer::<ID3D12Resource>(idx,)
        else
        {
            return;
        };
        let cpu = self.rtv_heap.GetCPUDescriptorHandleForHeapStart();
        let handle = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: cpu.ptr + (idx as usize * self.rtv_size as usize), };
        self.device.CreateRenderTargetView(&bb, None, handle,);

        let to_rt = transition(&bb, D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,);
        self.list.ResourceBarrier(&[to_rt,],);
        self.list.OMSetRenderTargets(1, Some(&handle,), BOOL(0,), None,);
        self.list.ClearRenderTargetView(handle, &[0.06, 0.07, 0.12, 1.0,], None,);
        let to_present = transition(&bb, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_PRESENT,);
        self.list.ResourceBarrier(&[to_present,],);
        let _ = self.list.Close();
        if let Ok(cmd,) = self.list.cast::<ID3D12CommandList>()
        {
            self.queue.ExecuteCommandLists(&[Some(cmd,),],);
        }
        let _ = self.swapchain.Present(1, DXGI_PRESENT(0,),);

        self.fence_value += 1;
        if self.queue.Signal(&self.fence, self.fence_value,).is_ok()
        {
            while self.fence.GetCompletedValue() < self.fence_value
            {
                std::thread::sleep(std::time::Duration::from_millis(1,),);
            }
        }
        self.frames += 1;
    }
}

unsafe fn transition(
    res: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type:      D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags:     D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource:   std::mem::ManuallyDrop::new(Some(res.clone(),),),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter:  after,
            },),
        },
    }
}

#[test]
fn dx12_survives_window_transitions() {
    unsafe {
        let mut win = match support::TestWindow::create("dx12", 1280, 720,)
        {
            Some(w,) => w,
            None =>
            {
                eprintln!("[dx12] window creation failed, skipping");
                return;
            }
        };
        let (w, h,) = win.client_size();
        let mut dx = match Dx12::new(HWND(win.hwnd,), w, h,)
        {
            Some(d,) => d,
            None =>
            {
                eprintln!("[dx12] device creation failed, skipping");
                return;
            }
        };

        let counter = support::Counter::new();
        nany::init("test_dx12", Box::new(support::SceneProvider::new(counter.clone(),),),);

        let mut last_size = (w, h,);
        for step in support::window_steps()
        {
            win.pump(8,);
            (step.apply)(&mut win,);
            let size = win.client_size();
            if size != last_size
            {
                let _ = dx.swapchain.ResizeBuffers(2, size.0, size.1, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0,),);
                last_size = size;
            }
            let before = counter.frames();
            for _ in 0..24
            {
                dx.frame();
                win.pump(1,);
                std::thread::sleep(std::time::Duration::from_millis(4,),);
            }
            let after = counter.frames();
            support::report("dx12", step.name, before, after,);
            assert!(after >= before, "dx12 overlay frame counter went backwards on {}", step.name);
        }
        assert!(dx.frames > 0, "dx12 presented no frames");
        assert!(counter.frames() > 0, "dx12 overlay never drew a frame");
        let _ = RECT { left: 0, top: 0, right: 0, bottom: 0, };
    }
}
