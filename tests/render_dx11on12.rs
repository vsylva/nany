#![allow(non_snake_case, clippy::missing_transmute_annotations)]

mod support;

use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D11::*,
            Direct3D11on12::{D3D11On12CreateDevice, ID3D11On12Device},
            Direct3D12::*,
            Dxgi::{Common::*, *},
        },
    },
    core::Interface,
};

struct On12 {
    dev11:     ID3D11Device,
    ctx11:     ID3D11DeviceContext,
    swapchain: IDXGISwapChain3,
    on12:      ID3D11On12Device,
    rtvs:      [Option<(usize, ID3D11RenderTargetView,),>; 2],
}

impl On12 {
    unsafe fn new(hwnd: HWND, w: u32, h: u32,) -> Option<Self,> {
        let mut dev12: Option<ID3D12Device,> = None;
        D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut dev12 as *mut _ as *mut _,).ok()?;
        let dev12 = dev12?;
        let queue: ID3D12CommandQueue = dev12
            .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                Type:     D3D12_COMMAND_LIST_TYPE_DIRECT,
                Priority: 0,
                Flags:    D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            },)
            .ok()?;

        let mut dev11: Option<ID3D11Device,> = None;
        let mut ctx11: Option<ID3D11DeviceContext,> = None;
        let mut level = D3D_FEATURE_LEVEL_11_0;
        D3D11On12CreateDevice(
            &dev12,
            0,
            Some(&[D3D_FEATURE_LEVEL_11_0,],),
            Some(&[Some(queue.cast::<windows::core::IUnknown>().ok()?,),],),
            0,
            Some(&mut dev11,),
            Some(&mut ctx11,),
            Some(&mut level,),
        )
        .ok()?;
        let dev11 = dev11?;
        let ctx11 = ctx11?;

        let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0,),).ok()?;
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width:       w,
            Height:      h,
            Format:      DXGI_FORMAT_R8G8B8A8_UNORM,
            Stereo:      windows::Win32::Foundation::BOOL(0,),
            SampleDesc:  DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling:     DXGI_SCALING_STRETCH,
            SwapEffect:  DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode:   DXGI_ALPHA_MODE_IGNORE,
            Flags:       0,
        };
        let sc1: IDXGISwapChain1 = factory.CreateSwapChainForHwnd(&dev11, hwnd, &desc, None, None,).ok()?;
        let swapchain: IDXGISwapChain3 = sc1.cast().ok()?;
        let on12: ID3D11On12Device = dev11.cast().ok()?;

        Some(Self { dev11, ctx11, swapchain, on12, rtvs: [None, None,], },)
    }

    unsafe fn resize(&mut self, w: u32, h: u32,) {
        self.rtvs = [None, None,];
        let _ = self.swapchain.ResizeBuffers(2, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0,),);
    }

    unsafe fn frame(&mut self,) {
        let idx = (self.swapchain.GetCurrentBackBufferIndex() as usize).min(1,);
        let Ok(bb,) = self.swapchain.GetBuffer::<ID3D11Texture2D>(idx as u32,)
        else
        {
            return;
        };
        let key = bb.as_raw() as usize;
        if self.rtvs[idx].as_ref().map(|(k, _,)| *k,) != Some(key,)
        {
            let mut v: Option<ID3D11RenderTargetView,> = None;
            let _ = self.dev11.CreateRenderTargetView(&bb, None, Some(&mut v,),);
            self.rtvs[idx] = v.map(|r| (key, r,),);
        }
        let Some((_, rtv,),) = self.rtvs[idx].as_ref()
        else
        {
            return;
        };
        let Ok(res,) = bb.cast::<ID3D11Resource>()
        else
        {
            return;
        };
        self.on12.AcquireWrappedResources(&[Some(res.clone(),),],);
        self.ctx11.OMSetRenderTargets(Some(&[Some(rtv.clone(),),],), None::<&ID3D11DepthStencilView,>,);
        self.ctx11.ClearRenderTargetView(rtv, &[0.06, 0.07, 0.12, 1.0,],);
        self.on12.ReleaseWrappedResources(&[Some(res,),],);
        self.ctx11.Flush();
        let _ = self.swapchain.Present(1, DXGI_PRESENT(0,),);
    }
}

#[test]
fn dx11on12_survives_window_transitions() {
    unsafe {
        let mut win = match support::TestWindow::create("dx11on12", 1280, 720,)
        {
            Some(w,) => w,
            None =>
            {
                eprintln!("[11on12] window creation failed, skipping");
                return;
            }
        };
        let (w, h,) = win.client_size();
        let mut dx = match On12::new(HWND(win.hwnd,), w, h,)
        {
            Some(d,) => d,
            None =>
            {
                eprintln!("[11on12] device creation failed, skipping");
                return;
            }
        };

        let counter = support::Counter::new();
        nany::init("test_dx11on12", Box::new(support::SceneProvider::new(counter.clone(),),),);

        let mut last = (w, h,);
        for step in support::window_steps()
        {
            win.pump(8,);
            (step.apply)(&mut win,);
            let size = win.client_size();
            if size != last
            {
                dx.resize(size.0, size.1,);
                last = size;
            }
            let before = counter.frames();
            for _ in 0..24
            {
                dx.frame();
                win.pump(1,);
                std::thread::sleep(std::time::Duration::from_millis(4,),);
            }
            let after = counter.frames();
            support::report("11on12", step.name, before, after,);
            assert!(after >= before, "11on12 overlay frame counter went backwards on {}", step.name);
        }
        assert!(counter.frames() > 0, "11on12 overlay never drew a frame");
    }
}
