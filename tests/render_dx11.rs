#![allow(non_camel_case_types)]

mod support;

use windows::{
    Win32::{
        Foundation::{BOOL, HWND, RECT},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL},
            Direct3D11::*,
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
                    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
                },
                DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
                IDXGISwapChain, IDXGISwapChain3,
            },
        },
    },
    core::Interface,
};

unsafe fn clear_frame(sc: &IDXGISwapChain, ctx: &ID3D11DeviceContext, dev: &ID3D11Device,) {
    let mut rtv: Option<ID3D11RenderTargetView,> = None;
    if let Ok(bb,) = sc.GetBuffer::<ID3D11Texture2D>(0,)
    {
        let res: ID3D11Resource = bb.cast().unwrap();
        if dev.CreateRenderTargetView(&res, None, Some(&mut rtv,),).is_err()
        {
            return;
        }
        let mut vp = [D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width:    1280.0,
            Height:   720.0,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        },];
        let mut r = RECT { left: 0, top: 0, right: 1280, bottom: 720, };
        if let Ok(bb2,) = sc.GetBuffer::<ID3D11Texture2D>(0,)
        {
            let mut d = D3D11_TEXTURE2D_DESC::default();
            bb2.GetDesc(&mut d,);
            vp[0].Width = d.Width as f32;
            vp[0].Height = d.Height as f32;
            r.right = d.Width as i32;
            r.bottom = d.Height as i32;
        }
        ctx.RSSetViewports(Some(&vp[..],),);
        ctx.OMSetRenderTargets(Some(&[rtv.clone(),],), None::<&ID3D11DepthStencilView,>,);
        if let Some(v,) = rtv.as_ref()
        {
            ctx.ClearRenderTargetView(v, &[0.06, 0.07, 0.12, 1.0,],);
        }
        let _ = r;
        let _ = sc.cast::<IDXGISwapChain3>().map(|s| s.GetCurrentBackBufferIndex(),);
    }
}

#[test]
fn dx11_survives_window_transitions() {
    unsafe {
        let mut win = match support::TestWindow::create("dx11", 1280, 720,)
        {
            Some(w,) => w,
            None =>
            {
                eprintln!("[dx11] window creation failed, skipping");
                return;
            }
        };
        let (w, h,) = win.client_size();
        let desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc:   DXGI_MODE_DESC {
                Width:            w,
                Height:           h,
                RefreshRate:      DXGI_RATIONAL { Numerator: 60, Denominator: 1, },
                Format:           DXGI_FORMAT_R8G8B8A8_UNORM,
                ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                Scaling:          DXGI_MODE_SCALING_UNSPECIFIED,
            },
            SampleDesc:   DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            BufferUsage:  DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount:  2,
            OutputWindow: HWND(win.hwnd,),
            Windowed:     BOOL(1,),
            SwapEffect:   DXGI_SWAP_EFFECT_FLIP_DISCARD,
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
        if hr.is_err()
        {
            eprintln!("[dx11] device creation failed: {hr:?}, skipping");
            return;
        }
        let (sc, dev, ctx,) = (sc.unwrap(), dev.unwrap(), ctx.unwrap(),);

        let counter = support::Counter::new();
        nany::init("test_dx11", Box::new(support::SceneProvider::new(counter.clone(),),),);

        for step in support::window_steps()
        {
            win.pump(8,);
            (step.apply)(&mut win,);
            let before = counter.frames();
            for _ in 0..24
            {
                clear_frame(&sc, &ctx, &dev,);
                let _ = sc.Present(1, DXGI_PRESENT(0,),);
                win.pump(1,);
                std::thread::sleep(std::time::Duration::from_millis(4,),);
            }
            let after = counter.frames();
            support::report("dx11", step.name, before, after,);
            assert!(after >= before, "dx11 overlay frame counter went backwards on step {}", step.name);
        }
        assert!(counter.frames() > 0, "dx11 overlay never drew a frame");
    }
}
