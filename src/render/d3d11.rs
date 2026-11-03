use std::{collections::HashMap, ffi::c_void};

use windows::{
    Win32::Graphics::{
        Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        Direct3D11::*,
        Direct3D11on12::ID3D11On12Device,
        Dxgi::{
            Common::{
                DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R32_UINT, DXGI_FORMAT_R32G32_FLOAT,
                DXGI_SAMPLE_DESC,
            },
            IDXGISwapChain, IDXGISwapChain3,
        },
    },
    core::{Interface, Result},
};

use super::{
    mesh::{self, Batch, VERTEX_SIZE},
    shader,
};
use crate::log_info;

const HLSL: &str = r#"
cbuffer Params : register(b0) { float2 uScreen; float uSrgb; float uPad; };
struct VSIn  { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PSIn  { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
PSIn vs_main(VSIn i) {
    PSIn o;
    float2 p = i.pos / uScreen * 2.0 - 1.0;
    o.pos = float4(p.x, -p.y, 0.0, 1.0);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}
Texture2D    tex : register(t0);
SamplerState smp : register(s0);

float3 srgb_to_linear(float3 c) {
    float3 lo = c / 12.92;
    float3 hi = pow((c + 0.055) / 1.055, 2.4);
    float3 k = step(c, 0.04045);
    return lerp(hi, lo, k);
}

float4 ps_main(PSIn i) : SV_Target {
    float a = tex.Sample(smp, i.uv).r;
    float4 c = i.col * a;
    if (uSrgb > 0.5) { c.rgb = srgb_to_linear(c.rgb); }
    return c;
}
"#;

struct TexEntry {
    w:    usize,
    h:    usize,
    data: Vec<u8,>,
    srv:  ID3D11ShaderResourceView,
    _tex: ID3D11Texture2D,
}

struct DynBuf {
    buf:   Option<ID3D11Buffer,>,
    cap:   usize,
    bind:  u32,
    label: &'static str,
}

impl DynBuf {
    unsafe fn new(bind: u32, label: &'static str,) -> Self {
        Self { buf: None, cap: 0, bind, label, }
    }

    unsafe fn ensure(&mut self, device: &ID3D11Device, need: usize,) -> bool {
        if self.buf.is_some() && self.cap >= need
        {
            return true;
        }

        let want = (need + 0xFFFF) & !0xFFFF;
        let cap = want.max(self.cap * 2,).max(64 * 1024,);
        let desc = D3D11_BUFFER_DESC {
            ByteWidth:           cap as u32,
            Usage:               D3D11_USAGE_DYNAMIC,
            BindFlags:           self.bind,
            CPUAccessFlags:      D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags:           0,
            StructureByteStride: 0,
        };
        let mut buf: Option<ID3D11Buffer,> = None;
        if device.CreateBuffer(&desc, None, Some(&mut buf as *mut _,),).is_err()
        {
            return false;
        }
        let Some(buf,) = buf
        else
        {
            return false;
        };
        self.buf = Some(buf,);
        self.cap = cap;
        log_info!("d3d11", format!("{} buffer grown to {} KB", self.label, cap / 1024));
        true
    }

    unsafe fn raw(&self,) -> Option<&ID3D11Buffer,> {
        self.buf.as_ref()
    }
}

struct SavedState11 {
    rtvs: [Option<ID3D11RenderTargetView,>; 8],
    dsv:  Option<ID3D11DepthStencilView,>,

    viewports:    [D3D11_VIEWPORT; 16],
    n_vp:         u32,
    scissors:     [windows::Win32::Foundation::RECT; 16],
    n_sc:         u32,
    raster:       Option<ID3D11RasterizerState,>,
    blend:        Option<ID3D11BlendState,>,
    blend_factor: [f32; 4],
    sample_mask:  u32,
    layout:       Option<ID3D11InputLayout,>,
    topology:     windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY,
    vs:           Option<ID3D11VertexShader,>,
    ps:           Option<ID3D11PixelShader,>,
    vs_cb0:       Option<ID3D11Buffer,>,
    ps_cb0:       Option<ID3D11Buffer,>,
    ps_sampler0:  Option<ID3D11SamplerState,>,
    ps_srv0:      Option<ID3D11ShaderResourceView,>,
    vb0:          Option<ID3D11Buffer,>,
    vb0_stride:   u32,
    vb0_offset:   u32,
    ib:           Option<ID3D11Buffer,>,
    ib_format:    windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT,
    ib_offset:    u32,
}

impl SavedState11 {
    unsafe fn save(ctx: &ID3D11DeviceContext,) -> Self {
        let mut rtvs: [Option<ID3D11RenderTargetView,>; 8] = Default::default();
        let mut dsv: Option<ID3D11DepthStencilView,> = None;
        ctx.OMGetRenderTargets(Some(&mut rtvs,), Some(&mut dsv,),);

        let mut n_vp = 16u32;
        let mut vp = [D3D11_VIEWPORT::default(); 16];
        ctx.RSGetViewports(&mut n_vp, Some(vp.as_mut_ptr(),),);
        let mut n_sc = 16u32;
        let mut sc = [windows::Win32::Foundation::RECT::default(); 16];
        ctx.RSGetScissorRects(&mut n_sc, Some(sc.as_mut_ptr(),),);

        let mut blend: Option<ID3D11BlendState,> = None;
        let mut factor = [0f32; 4];
        let mut mask = 0u32;
        ctx.OMGetBlendState(Some(&mut blend,), Some(&mut factor,), Some(&mut mask,),);

        let mut shader_vs: Option<ID3D11VertexShader,> = None;
        ctx.VSGetShader(&mut shader_vs, None, None,);
        let mut shader_ps: Option<ID3D11PixelShader,> = None;
        ctx.PSGetShader(&mut shader_ps, None, None,);

        let mut vs_cb = [None; 1];
        ctx.VSGetConstantBuffers(0, Some(&mut vs_cb,),);
        let mut ps_cb = [None; 1];
        ctx.PSGetConstantBuffers(0, Some(&mut ps_cb,),);
        let mut ps_smp = [None; 1];
        ctx.PSGetSamplers(0, Some(&mut ps_smp,),);
        let mut ps_srv = [None; 1];
        ctx.PSGetShaderResources(0, Some(&mut ps_srv,),);

        let mut vb0: Option<ID3D11Buffer,> = None;
        let (mut stride, mut offset,) = (0u32, 0u32,);
        ctx.IAGetVertexBuffers(0, 1, Some(&mut vb0,), Some(&mut stride,), Some(&mut offset,),);
        let mut ib: Option<ID3D11Buffer,> = None;
        let mut ib_format = windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
        let mut ib_offset = 0u32;
        ctx.IAGetIndexBuffer(Some(&mut ib,), Some(&mut ib_format,), Some(&mut ib_offset,),);

        SavedState11 {
            rtvs,
            dsv,
            viewports: vp,
            n_vp,
            scissors: sc,
            n_sc,
            raster: ctx.RSGetState().ok(),
            blend,
            blend_factor: factor,
            sample_mask: mask,
            layout: ctx.IAGetInputLayout().ok(),
            topology: ctx.IAGetPrimitiveTopology(),
            vs: shader_vs,
            ps: shader_ps,
            vs_cb0: vs_cb[0].take(),
            ps_cb0: ps_cb[0].take(),
            ps_sampler0: ps_smp[0].take(),
            ps_srv0: ps_srv[0].take(),
            vb0,
            vb0_stride: stride,
            vb0_offset: offset,
            ib,
            ib_format,
            ib_offset,
        }
    }

    unsafe fn restore(&self, ctx: &ID3D11DeviceContext,) {
        if self.n_vp > 0
        {
            ctx.RSSetViewports(Some(&self.viewports[..self.n_vp as usize],),);
        }
        if self.n_sc > 0
        {
            ctx.RSSetScissorRects(Some(&self.scissors[..self.n_sc as usize],),);
        }
        ctx.RSSetState(self.raster.as_ref(),);
        ctx.OMSetBlendState(self.blend.as_ref(), Some(&self.blend_factor,), self.sample_mask,);
        ctx.IASetInputLayout(self.layout.as_ref(),);
        ctx.IASetPrimitiveTopology(self.topology,);
        ctx.VSSetShader(self.vs.as_ref(), None,);
        ctx.PSSetShader(self.ps.as_ref(), None,);

        ctx.VSSetConstantBuffers(0, Some(std::slice::from_ref(&self.vs_cb0,),),);
        ctx.PSSetConstantBuffers(0, Some(std::slice::from_ref(&self.ps_cb0,),),);
        ctx.PSSetSamplers(0, Some(std::slice::from_ref(&self.ps_sampler0,),),);
        ctx.PSSetShaderResources(0, Some(std::slice::from_ref(&self.ps_srv0,),),);
        let vb0 = [self.vb0.clone(),];
        ctx.IASetVertexBuffers(0, 1, Some(vb0.as_ptr(),), Some(&self.vb0_stride,), Some(&self.vb0_offset,),);
        ctx.IASetIndexBuffer(self.ib.as_ref(), self.ib_format, self.ib_offset,);
        ctx.OMSetRenderTargets(Some(&self.rtvs,), self.dsv.as_ref(),);
    }
}

pub struct Painter11 {
    device: ID3D11Device,
    ctx:    ID3D11DeviceContext,
    vs:     ID3D11VertexShader,
    ps:     ID3D11PixelShader,
    layout: ID3D11InputLayout,
    vb:     DynBuf,
    ib:     DynBuf,

    last_vb: Vec<u8,>,
    last_ib: Vec<u8,>,
    cb:      ID3D11Buffer,
    blend:   ID3D11BlendState,
    raster:  ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
    white:   ID3D11ShaderResourceView,
    rtv:     Option<ID3D11RenderTargetView,>,
    rtv_key: (usize, u32, u32,),

    srgb: bool,

    bb_res: Option<ID3D11Resource,>,

    on12:           Option<ID3D11On12Device,>,
    tex:            HashMap<u64, TexEntry,>,
    size:           (u32, u32,),
    warned:         bool,
    pub device_raw: usize,
}

unsafe impl Send for Painter11 {}

impl Painter11 {
    pub unsafe fn new(device: &ID3D11Device, ctx: &ID3D11DeviceContext,) -> Result<Self,> {
        let fail = |e: String| windows::core::Error::new(windows::core::HRESULT(0x8000_4005u32 as i32,), e,);
        let vs_code = shader::compile(HLSL, "vs_main", "vs_5_0",).map_err(fail,)?;
        let ps_code = shader::compile(HLSL, "ps_main", "ps_5_0",).map_err(fail,)?;

        let mut vs: Option<ID3D11VertexShader,> = None;
        device.CreateVertexShader(&vs_code, None::<&ID3D11ClassLinkage,>, Some(&mut vs as *mut _,),)?;
        let mut ps: Option<ID3D11PixelShader,> = None;
        device.CreatePixelShader(&ps_code, None::<&ID3D11ClassLinkage,>, Some(&mut ps as *mut _,),)?;

        let elems = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName:         windows::core::s!("POSITION"),
                SemanticIndex:        0,
                Format:               DXGI_FORMAT_R32G32_FLOAT,
                InputSlot:            0,
                AlignedByteOffset:    0,
                InputSlotClass:       D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName:         windows::core::s!("TEXCOORD"),
                SemanticIndex:        0,
                Format:               DXGI_FORMAT_R32G32_FLOAT,
                InputSlot:            0,
                AlignedByteOffset:    8,
                InputSlotClass:       D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName:         windows::core::s!("COLOR"),
                SemanticIndex:        0,
                Format:               DXGI_FORMAT_R8G8B8A8_UNORM,
                InputSlot:            0,
                AlignedByteOffset:    16,
                InputSlotClass:       D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        let mut layout: Option<ID3D11InputLayout,> = None;
        device.CreateInputLayout(&elems, &vs_code, Some(&mut layout as *mut _,),)?;

        let cb_desc = D3D11_BUFFER_DESC {
            ByteWidth:           16,
            Usage:               D3D11_USAGE_DYNAMIC,
            BindFlags:           D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags:      D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags:           0,
            StructureByteStride: 0,
        };
        let mut cb: Option<ID3D11Buffer,> = None;
        device.CreateBuffer(&cb_desc, None, Some(&mut cb as *mut _,),)?;

        let mut bd = D3D11_BLEND_DESC::default();
        bd.RenderTarget[0].BlendEnable = windows::Win32::Foundation::BOOL(1,);
        bd.RenderTarget[0].SrcBlend = D3D11_BLEND_ONE;
        bd.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
        bd.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
        bd.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_ONE;
        bd.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
        bd.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
        bd.RenderTarget[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8;
        let mut blend: Option<ID3D11BlendState,> = None;
        device.CreateBlendState(&bd, Some(&mut blend as *mut _,),)?;

        let rd = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            ScissorEnable: windows::Win32::Foundation::BOOL(1,),
            DepthClipEnable: windows::Win32::Foundation::BOOL(1,),
            ..Default::default()
        };
        let mut raster: Option<ID3D11RasterizerState,> = None;
        device.CreateRasterizerState(&rd, Some(&mut raster as *mut _,),)?;

        let sd = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let mut sampler: Option<ID3D11SamplerState,> = None;
        device.CreateSamplerState(&sd, Some(&mut sampler as *mut _,),)?;

        let white_px = [255u8,];
        let wdesc = D3D11_TEXTURE2D_DESC {
            Width:          1,
            Height:         1,
            MipLevels:      1,
            ArraySize:      1,
            Format:         DXGI_FORMAT_R8_UNORM,
            SampleDesc:     DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            Usage:          D3D11_USAGE_IMMUTABLE,
            BindFlags:      D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags:      0,
        };
        let winit = D3D11_SUBRESOURCE_DATA {
            pSysMem:          white_px.as_ptr() as *const c_void,
            SysMemPitch:      1,
            SysMemSlicePitch: 0,
        };
        let mut wtex: Option<ID3D11Texture2D,> = None;
        device.CreateTexture2D(&wdesc, Some(&winit,), Some(&mut wtex as *mut _,),)?;

        let missing = || windows::core::Error::from_win32();
        let wtex = wtex.ok_or_else(missing,)?;
        let wres = wtex.cast::<ID3D11Resource>()?;
        let mut white: Option<ID3D11ShaderResourceView,> = None;
        device.CreateShaderResourceView(&wres, None, Some(&mut white as *mut _,),)?;

        Ok(Painter11 {
            device:     device.clone(),
            ctx:        ctx.clone(),
            vs:         vs.ok_or_else(missing,)?,
            ps:         ps.ok_or_else(missing,)?,
            layout:     layout.ok_or_else(missing,)?,
            vb:         DynBuf::new(D3D11_BIND_VERTEX_BUFFER.0 as u32, "vb",),
            ib:         DynBuf::new(D3D11_BIND_INDEX_BUFFER.0 as u32, "ib",),
            last_vb:    Vec::new(),
            last_ib:    Vec::new(),
            cb:         cb.ok_or_else(missing,)?,
            blend:      blend.ok_or_else(missing,)?,
            raster:     raster.ok_or_else(missing,)?,
            sampler:    sampler.ok_or_else(missing,)?,
            white:      white.ok_or_else(missing,)?,
            rtv:        None,
            rtv_key:    (0, 0, 0,),
            srgb:       false,
            bb_res:     None,
            on12:       None,
            tex:        HashMap::new(),
            size:       (0, 0,),
            warned:     false,
            device_raw: device.as_raw() as usize,
        },)
    }

    pub unsafe fn size(&self,) -> (u32, u32,) {
        self.size
    }

    pub unsafe fn release_target(&mut self,) {
        self.rtv = None;
        self.rtv_key = (0, 0, 0,);
        self.bb_res = None;
        self.size = (0, 0,);
    }

    pub unsafe fn set_on12(&mut self, dev: Option<ID3D11On12Device,>,) {
        self.on12 = dev;
    }

    pub unsafe fn ensure_target(&mut self, sc: &IDXGISwapChain, w: u32, h: u32,) {
        let idx = sc.cast::<IDXGISwapChain3>().map(|s| s.GetCurrentBackBufferIndex(),).unwrap_or(0,);
        let Ok(bb,) = sc.GetBuffer::<ID3D11Texture2D>(idx,)
        else
        {
            return;
        };
        let key = (bb.as_raw() as usize, w, h,);
        if self.rtv_key == key && self.rtv.is_some()
        {
            return;
        }
        let Ok(res,) = bb.cast::<ID3D11Resource>()
        else
        {
            return;
        };

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        bb.GetDesc(&mut desc,);
        self.srgb = matches!(desc.Format.0, 29 | 91);
        let mut rtv: Option<ID3D11RenderTargetView,> = None;
        if self.device.CreateRenderTargetView(&res, None, Some(&mut rtv as *mut _,),).is_ok()
        {
            self.rtv = rtv;
            self.rtv_key = key;
            self.bb_res = Some(res,);
            self.size = (w, h,);
            log_info!("d3d11", format!("rtv {w}x{h} buffer#{idx}"));
        }
    }

    unsafe fn make_tex(&self, w: usize, h: usize, data: &[u8],) -> Option<TexEntry,> {
        if w == 0 || h == 0 || data.len() < w * h
        {
            return None;
        }
        let desc = D3D11_TEXTURE2D_DESC {
            Width:          w as u32,
            Height:         h as u32,
            MipLevels:      1,
            ArraySize:      1,
            Format:         DXGI_FORMAT_R8_UNORM,
            SampleDesc:     DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            Usage:          D3D11_USAGE_DEFAULT,
            BindFlags:      D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags:      0,
        };
        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem:          data.as_ptr() as *const c_void,
            SysMemPitch:      w as u32,
            SysMemSlicePitch: 0,
        };
        let mut tex: Option<ID3D11Texture2D,> = None;
        self.device.CreateTexture2D(&desc, Some(&init,), Some(&mut tex as *mut _,),).ok()?;
        let tex = tex?;
        let res = tex.cast::<ID3D11Resource>().ok()?;
        let mut srv: Option<ID3D11ShaderResourceView,> = None;
        self.device.CreateShaderResourceView(&res, None, Some(&mut srv as *mut _,),).ok()?;
        Some(TexEntry { w, h, data: data.to_vec(), srv: srv?, _tex: tex, },)
    }

    pub unsafe fn apply_textures(&mut self, delta: &egui::TexturesDelta,) {
        for (id, deltas,) in &delta.set
        {
            for image_delta in deltas.iter()
            {
                let key = mesh::tex_key(*id,);
                let egui::ImageData::Color(img,) = &image_delta.image;
                let alpha = mesh::alpha_of(img,);
                let (w, h,) = (img.size[0], img.size[1],);

                match image_delta.pos
                {
                    None =>
                    {
                        if let Some(e,) = self.make_tex(w, h, &alpha,)
                        {
                            self.tex.insert(key, e,);
                        }
                    }
                    Some([x, y,],) =>
                    {
                        let ctx = self.ctx.clone();
                        if let Some(entry,) = self.tex.get_mut(&key,)
                        {
                            let Ok(res,) = entry._tex.cast::<ID3D11Resource>()
                            else
                            {
                                continue;
                            };
                            let bx = windows::Win32::Graphics::Direct3D11::D3D11_BOX {
                                left:   x as u32,
                                top:    y as u32,
                                front:  0,
                                right:  (x + w) as u32,
                                bottom: (y + h) as u32,
                                back:   1,
                            };
                            ctx.UpdateSubresource(&res, 0, Some(&bx,), alpha.as_ptr() as *const c_void, w as u32, 0,);
                            for row in 0..h
                            {
                                let dst = (y + row) * entry.w + x;
                                if dst + w <= entry.data.len()
                                {
                                    entry.data[dst..dst + w].copy_from_slice(&alpha[row * w..(row + 1) * w],);
                                }
                            }
                            continue;
                        }
                        let merged = self.tex.get(&key,).map(|old| {
                            let mut buf = old.data.clone();
                            for row in 0..h
                            {
                                let dst = (y + row) * old.w + x;
                                let src = row * w;
                                if dst + w <= buf.len()
                                {
                                    buf[dst..dst + w].copy_from_slice(&alpha[src..src + w],);
                                }
                            }
                            (old.w, old.h, buf,)
                        },);
                        if let Some((ow, oh, buf,),) = merged
                        {
                            if let Some(e,) = self.make_tex(ow, oh, &buf,)
                            {
                                self.tex.insert(key, e,);
                            }
                        }
                    }
                }
            }
        }
        for id in &delta.free
        {
            self.tex.remove(&mesh::tex_key(*id,),);
        }
    }

    pub unsafe fn paint(&mut self, prims: &[egui::ClippedPrimitive], screen: (u32, u32,),) {
        let Some(rtv,) = self.rtv.clone()
        else
        {
            if !self.warned
            {
                log_info!("d3d11", "no rtv, skip frame");
                self.warned = true;
            }
            return;
        };
        let batch = mesh::build(prims, screen,);
        if batch.is_empty()
        {
            return;
        }
        self.draw(&rtv, &batch,)
    }

    unsafe fn draw(&mut self, rtv: &ID3D11RenderTargetView, batch: &Batch,) {
        let on12 = self.on12.clone();
        let bb = self.bb_res.clone();
        if let (Some(dev,), Some(res,),) = (&on12, &bb,)
        {
            dev.AcquireWrappedResources(&[Some(res.clone(),),],);
        }
        let saved = SavedState11::save(&self.ctx,);
        self.draw_inner(rtv, batch,);
        saved.restore(&self.ctx,);
        if let (Some(dev,), Some(res,),) = (&on12, &bb,)
        {
            dev.ReleaseWrappedResources(&[Some(res.clone(),),],);
        }
        if on12.is_some()
        {
            self.ctx.Flush();
        }
    }

    unsafe fn draw_inner(&mut self, rtv: &ID3D11RenderTargetView, batch: &Batch,) {
        let ctx = &self.ctx;
        ctx.OMSetRenderTargets(Some(&[Some(rtv.clone(),),],), None::<&ID3D11DepthStencilView,>,);
        let vp = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width:    batch.screen.0 as f32,
            Height:   batch.screen.1 as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        ctx.RSSetViewports(Some(&[vp,],),);
        ctx.RSSetState(&self.raster,);
        ctx.OMSetBlendState(&self.blend, None, 0xffffffff,);
        ctx.IASetInputLayout(&self.layout,);
        ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,);
        ctx.VSSetShader(&self.vs, None,);
        ctx.PSSetShader(&self.ps, None,);
        ctx.PSSetSamplers(0, Some(&[Some(self.sampler.clone(),),],),);

        let srgb_flag = if self.srgb { 1.0f32 } else { 0.0f32 };
        let data = [batch.screen.0 as f32, batch.screen.1 as f32, srgb_flag, 0.0f32,];
        let Ok(cbres,) = self.cb.cast::<ID3D11Resource>()
        else
        {
            return;
        };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if ctx.Map(&cbres, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped,),).is_ok()
        {
            std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, mapped.pData as *mut u8, 16,);
            ctx.Unmap(&cbres, 0,);
        }
        ctx.VSSetConstantBuffers(0, Some(&[Some(self.cb.clone(),),],),);

        ctx.PSSetConstantBuffers(0, Some(&[Some(self.cb.clone(),),],),);

        let vbytes = batch.vertex_bytes();
        let ibytes = batch.index_bytes();

        if !self.vb.ensure(&self.device, vbytes.len(),) || !self.ib.ensure(&self.device, ibytes.len(),)
        {
            return;
        }
        let (Some(vb,), Some(ib,),) = (self.vb.raw().cloned(), self.ib.raw().cloned(),)
        else
        {
            return;
        };
        let Ok(vbres,) = vb.cast::<ID3D11Resource>()
        else
        {
            return;
        };
        let Ok(ibres,) = ib.cast::<ID3D11Resource>()
        else
        {
            return;
        };

        let same = self.last_vb == vbytes && self.last_ib == ibytes;
        if !same
        {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if ctx.Map(&vbres, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m,),).is_ok()
            {
                std::ptr::copy_nonoverlapping(vbytes.as_ptr(), m.pData as *mut u8, vbytes.len(),);
                ctx.Unmap(&vbres, 0,);
            }
            if ctx.Map(&ibres, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m,),).is_ok()
            {
                std::ptr::copy_nonoverlapping(ibytes.as_ptr(), m.pData as *mut u8, ibytes.len(),);
                ctx.Unmap(&ibres, 0,);
            }
            self.last_vb.clear();
            self.last_vb.extend_from_slice(vbytes,);
            self.last_ib.clear();
            self.last_ib.extend_from_slice(ibytes,);
        }

        let stride = VERTEX_SIZE as u32;
        let vbs = [Some(vb.clone(),),];
        ctx.IASetVertexBuffers(0, 1, Some(vbs.as_ptr(),), Some(&stride,), Some(&0u32,),);
        ctx.IASetIndexBuffer(&ib, DXGI_FORMAT_R32_UINT, 0,);

        for d in &batch.draws
        {
            let srv = self.tex.get(&d.tex,).map(|t| t.srv.clone(),).unwrap_or_else(|| self.white.clone(),);
            ctx.PSSetShaderResources(0, Some(&[Some(srv,),],),);
            let r = windows::Win32::Foundation::RECT {
                left:   d.scissor[0],
                top:    d.scissor[1],
                right:  d.scissor[2],
                bottom: d.scissor[3],
            };
            ctx.RSSetScissorRects(Some(&[r,],),);
            ctx.DrawIndexed(d.idx_count, d.idx_start, 0,);
        }
    }

    pub unsafe fn tex_count(&self,) -> usize {
        self.tex.len()
    }
}
