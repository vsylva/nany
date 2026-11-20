//! Direct3D 9 painter: draws the egui mesh with a small shader pair and a
//! managed alpha atlas.

use std::{collections::HashMap, ffi::c_void, mem::ManuallyDrop};

use windows::{
    Win32::{
        Foundation::RECT,
        Graphics::Direct3D9::{
            D3DBACKBUFFER_TYPE_MONO, D3DBLEND_INVSRCALPHA, D3DBLEND_ONE, D3DBLENDOP_ADD, D3DCULL_NONE,
            D3DDECLMETHOD_DEFAULT, D3DDECLTYPE_FLOAT2, D3DDECLTYPE_UBYTE4N, D3DDECLUSAGE_COLOR, D3DDECLUSAGE_POSITION,
            D3DDECLUSAGE_TEXCOORD, D3DFMT_INDEX32, D3DFMT_L8, D3DLOCK_DISCARD, D3DLOCKED_RECT, D3DPOOL_DEFAULT,
            D3DPOOL_MANAGED, D3DPT_TRIANGLELIST, D3DRS_ALPHABLENDENABLE, D3DRS_ALPHATESTENABLE, D3DRS_BLENDOP,
            D3DRS_BLENDOPALPHA, D3DRS_CLIPPING, D3DRS_COLORVERTEX, D3DRS_CULLMODE, D3DRS_DESTBLEND,
            D3DRS_DESTBLENDALPHA, D3DRS_FOGENABLE, D3DRS_LIGHTING, D3DRS_SCISSORTESTENABLE,
            D3DRS_SEPARATEALPHABLENDENABLE, D3DRS_SRCBLEND, D3DRS_SRCBLENDALPHA, D3DRS_SRGBWRITEENABLE,
            D3DRS_STENCILENABLE, D3DRS_ZENABLE, D3DRS_ZWRITEENABLE, D3DSAMP_ADDRESSU,
            D3DSAMP_ADDRESSV, D3DSAMP_ADDRESSW, D3DSAMP_MAGFILTER, D3DSAMP_MINFILTER, D3DSAMP_MIPFILTER, D3DSBT_ALL,
            D3DSURFACE_DESC, D3DTADDRESS_CLAMP, D3DTEXF_LINEAR, D3DTEXF_NONE, D3DUSAGE_DYNAMIC, D3DUSAGE_WRITEONLY,
            D3DVERTEXELEMENT9, D3DVIEWPORT9, D3DZB_FALSE, IDirect3DDevice9, IDirect3DIndexBuffer9, IDirect3DPixelShader9,
            IDirect3DTexture9, IDirect3DVertexBuffer9, IDirect3DVertexDeclaration9, IDirect3DVertexShader9,
        },
    },
    core::{Error, HRESULT, Interface, Result},
};

use super::{
    mesh::{self, Batch, VERTEX_SIZE},
    shader,
};
use crate::log_info;

/// Shader model 3.0 covers anything from 2004 on; 2.0 is the fallback for the
/// rare old driver.
const HLSL: &str = r#"
float4 uParams : register(c0);

struct VSIn  { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PSIn  { float4 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };

PSIn vs_main(VSIn i) {
    PSIn o;
    float2 p = i.pos / uParams.xy * 2.0 - 1.0;
    o.pos = float4(p.x, -p.y, 0.0, 1.0);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}

sampler2D smp : register(s0);

/* D3D9 back buffers are plain 8-bit colour targets, so there is no sRGB decode
   to do here: egui already produced display-space colours. */
float4 ps_main(PSIn i) : COLOR {
    float a = tex2D(smp, i.uv).r;
    return i.col * a;
}
"#;

/// `D3DDECLTYPE_UNUSED`, i.e. the end marker of a vertex declaration.
const DECL_UNUSED: u8 = 17;

const DECL_END: D3DVERTEXELEMENT9 =
    D3DVERTEXELEMENT9 { Stream: 0xFF, Offset: 0, Type: DECL_UNUSED, Method: 0, Usage: 0, UsageIndex: 0 };

struct TexEntry {
    w:    usize,
    h:    usize,
    data: Vec<u8,>,
    tex:  IDirect3DTexture9,
}

pub struct Painter9 {
    device: ManuallyDrop<IDirect3DDevice9,>,
    vs:     IDirect3DVertexShader9,
    ps:     IDirect3DPixelShader9,
    decl:   IDirect3DVertexDeclaration9,
    vb:     Option<IDirect3DVertexBuffer9,>,
    ib:     Option<IDirect3DIndexBuffer9,>,
    vb_cap: usize,
    ib_cap: usize,
    white:  IDirect3DTexture9,
    tex:    HashMap<u64, TexEntry,>,
    size:   (u32, u32,),
    warned: bool,
    pub device_raw: usize,
}

unsafe impl Send for Painter9 {}

unsafe fn set_state(
    device: &ManuallyDrop<IDirect3DDevice9,>,
    state: windows::Win32::Graphics::Direct3D9::D3DRENDERSTATETYPE,
    value: i32,
) {
    let _ = device.SetRenderState(state, value as u32,);
}

unsafe fn compile_pair() -> Option<(Vec<u8,>, Vec<u8,>)> {
    for (vs, ps,) in [("vs_3_0", "ps_3_0",), ("vs_2_0", "ps_2_0",)]
    {
        let (Ok(vs_code,), Ok(ps_code,),) =
            (shader::compile(HLSL, "vs_main", vs,), shader::compile(HLSL, "ps_main", ps,),)
        else
        {
            continue;
        };
        return Some((vs_code, ps_code,));
    }
    None
}

unsafe fn make_texture(device: &IDirect3DDevice9, w: usize, h: usize, data: &[u8],) -> Option<IDirect3DTexture9,> {
    if w == 0 || h == 0 || data.len() < w * h
    {
        return None;
    }
    let mut tex: Option<IDirect3DTexture9,> = None;
    // managed pool keeps the CPU copy across device resets
    device.CreateTexture(w as u32, h as u32, 1, 0, D3DFMT_L8, D3DPOOL_MANAGED, &mut tex, std::ptr::null_mut(),).ok()?;
    let tex = tex?;

    let mut locked = D3DLOCKED_RECT::default();
    if tex.LockRect(0, &mut locked, std::ptr::null(), 0,).is_err()
    {
        return None;
    }
    if !locked.pBits.is_null()
    {
        let pitch = locked.Pitch as isize;
        for row in 0..h
        {
            let dst = (locked.pBits as *mut u8).offset(row as isize * pitch,);
            std::ptr::copy_nonoverlapping(data.as_ptr().add(row * w,), dst, w,);
        }
    }
    let _ = tex.UnlockRect(0,);
    Some(tex,)
}

impl Painter9 {
    pub unsafe fn new(device: &IDirect3DDevice9,) -> Result<Self,> {
        let fail = |msg: String| Error::new(HRESULT(0x8000_4005u32 as i32,), msg,);
        let (vs_code, ps_code,) = compile_pair().ok_or_else(|| fail("shader compile failed".into(),),)?;

        let vs = device.CreateVertexShader(vs_code.as_ptr() as *const u32,).map_err(|e| fail(e.message(),),)?;
        let ps = device.CreatePixelShader(ps_code.as_ptr() as *const u32,).map_err(|e| fail(e.message(),),)?;

        let elements = [
            D3DVERTEXELEMENT9 {
                Stream:     0,
                Offset:     0,
                Type:       D3DDECLTYPE_FLOAT2.0 as u8,
                Method:     D3DDECLMETHOD_DEFAULT.0 as u8,
                Usage:      D3DDECLUSAGE_POSITION.0 as u8,
                UsageIndex: 0,
            },
            D3DVERTEXELEMENT9 {
                Stream:     0,
                Offset:     8,
                Type:       D3DDECLTYPE_FLOAT2.0 as u8,
                Method:     D3DDECLMETHOD_DEFAULT.0 as u8,
                Usage:      D3DDECLUSAGE_TEXCOORD.0 as u8,
                UsageIndex: 0,
            },
            D3DVERTEXELEMENT9 {
                Stream:     0,
                Offset:     16,
                Type:       D3DDECLTYPE_UBYTE4N.0 as u8,
                Method:     D3DDECLMETHOD_DEFAULT.0 as u8,
                Usage:      D3DDECLUSAGE_COLOR.0 as u8,
                UsageIndex: 0,
            },
            DECL_END,
        ];
        let decl = device.CreateVertexDeclaration(elements.as_ptr(),).map_err(|e| fail(e.message(),),)?;

        let white = make_texture(device, 1, 1, &[255u8,],).ok_or_else(|| fail("white texture failed".into(),),)?;
        let mut size = (0u32, 0u32,);
        if let Ok(backbuffer,) = device.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO,)
        {
            let mut desc = D3DSURFACE_DESC::default();
            if backbuffer.GetDesc(&mut desc,).is_ok()
            {
                size = (desc.Width, desc.Height,);
            }
        }

        Ok(Painter9 {
            device: ManuallyDrop::new(IDirect3DDevice9::from_raw(device.as_raw(),),),
            vs,
            ps,
            decl,
            vb: None,
            ib: None,
            vb_cap: 0,
            ib_cap: 0,
            white,
            tex: HashMap::new(),
            size,
            warned: false,
            device_raw: device.as_raw() as usize,
        },)
    }

    pub unsafe fn size(&self,) -> (u32, u32,) {
        self.size
    }

    /// Drop default-pool buffers; called from the `Reset` hook.
    pub unsafe fn release_target(&mut self,) {
        self.vb = None;
        self.ib = None;
        self.vb_cap = 0;
        self.ib_cap = 0;
        self.size = (0, 0,);
    }

    pub unsafe fn ensure_target(&mut self, w: u32, h: u32,) {
        self.size = (w, h,);
    }

    pub unsafe fn apply_textures(&mut self, delta: &egui::TexturesDelta,) {
        for (id, deltas,) in &delta.set
        {
            for image_delta in deltas.iter()
            {
                let key = mesh::tex_key(*id,);
                let egui::ImageData::Color(image,) = &image_delta.image;
                let alpha = mesh::alpha_of(image,);
                let (w, h,) = (image.size[0], image.size[1],);

                match image_delta.pos
                {
                    None =>
                    {
                        if let Some(tex,) = make_texture(&self.device, w, h, &alpha,)
                        {
                            self.tex.insert(key, TexEntry { w, h, data: alpha, tex, },);
                        }
                    }
                    Some([x, y,],) =>
                    {
                        // blend the patch into the stored copy, then re-upload;
                        // the glyph atlas is small and one code path is enough
                        let Some(old,) = self.tex.get(&key,)
                        else
                        {
                            continue;
                        };
                        let mut buf = old.data.clone();
                        for row in 0..h
                        {
                            let dst = (y + row) * old.w + x;
                            if dst + w <= buf.len()
                            {
                                buf[dst..dst + w].copy_from_slice(&alpha[row * w..(row + 1) * w],);
                            }
                        }
                        let (ow, oh,) = (old.w, old.h,);
                        if let Some(tex,) = make_texture(&self.device, ow, oh, &buf,)
                        {
                            self.tex.insert(key, TexEntry { w: ow, h: oh, data: buf, tex, },);
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

    pub unsafe fn tex_count(&self,) -> usize {
        self.tex.len()
    }

    unsafe fn ensure_vb(&mut self, need: usize,) -> bool {
        if self.vb.is_some() && self.vb_cap >= need
        {
            return true;
        }
        let cap = ((need + 0xFFFF) & !0xFFFF).max(self.vb_cap * 2,).max(64 * 1024,);
        let usage = (D3DUSAGE_DYNAMIC | D3DUSAGE_WRITEONLY) as u32;
        let mut vb: Option<IDirect3DVertexBuffer9,> = None;
        if self.device.CreateVertexBuffer(cap as u32, usage, 0, D3DPOOL_DEFAULT, &mut vb, std::ptr::null_mut(),).is_err()
        {
            return false;
        }
        let Some(vb,) = vb
        else
        {
            return false;
        };
        self.vb = Some(vb,);
        self.vb_cap = cap;
        log_info!("d3d9", format!("vb grown to {} KB", cap / 1024));
        true
    }

    unsafe fn ensure_ib(&mut self, need: usize,) -> bool {
        if self.ib.is_some() && self.ib_cap >= need
        {
            return true;
        }
        let cap = ((need + 0xFFFF) & !0xFFFF).max(self.ib_cap * 2,).max(64 * 1024,);
        let usage = (D3DUSAGE_DYNAMIC | D3DUSAGE_WRITEONLY) as u32;
        let mut ib: Option<IDirect3DIndexBuffer9,> = None;
        if self
            .device
            .CreateIndexBuffer(cap as u32, usage, D3DFMT_INDEX32, D3DPOOL_DEFAULT, &mut ib, std::ptr::null_mut(),)
            .is_err()
        {
            return false;
        }
        let Some(ib,) = ib
        else
        {
            return false;
        };
        self.ib = Some(ib,);
        self.ib_cap = cap;
        log_info!("d3d9", format!("ib grown to {} KB", cap / 1024));
        true
    }

    pub unsafe fn paint(&mut self, prims: &[egui::ClippedPrimitive], screen: (u32, u32,),) {
        if self.device.TestCooperativeLevel().is_err()
        {
            if !self.warned
            {
                log_info!("d3d9", "device not ready, holding off until Reset");
                self.warned = true;
            }
            return;
        }
        self.warned = false;

        let batch = mesh::build(prims, screen,);
        if batch.is_empty()
        {
            return;
        }

        // D3D9 rejects every draw call outside of a BeginScene/EndScene pair, and
        // Present runs after the game closed its scene. Open one when needed; when
        // a game leaves its own scene open, draw into that instead.
        let own_scene = self.device.BeginScene().is_ok();

        // one state block per frame keeps every touched state restorable
        let state = self.device.CreateStateBlock(D3DSBT_ALL,).ok();
        self.draw(&batch,);
        if let Some(state,) = state
        {
            let _ = state.Apply();
        }

        if own_scene
        {
            let _ = self.device.EndScene();
        }
    }

    unsafe fn draw(&mut self, batch: &Batch,) {
        let vertex_bytes = batch.vertex_bytes();
        let index_bytes = batch.index_bytes();
        if !self.ensure_vb(vertex_bytes.len(),) || !self.ensure_ib(index_bytes.len(),)
        {
            return;
        }
        let (Some(vb,), Some(ib,),) = (self.vb.clone(), self.ib.clone(),)
        else
        {
            return;
        };
        // a fresh wrapper over the same device, so self stays free for the
        // buffer helpers above
        let device: ManuallyDrop<IDirect3DDevice9,> = ManuallyDrop::new(IDirect3DDevice9::from_raw(self.device.as_raw(),),);

        let Ok(backbuffer,) = device.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO,)
        else
        {
            return;
        };
        let _ = device.SetRenderTarget(0, &backbuffer,);

        let viewport = D3DVIEWPORT9 {
            X:      0,
            Y:      0,
            Width:  batch.screen.0,
            Height: batch.screen.1,
            MinZ:   0.0,
            MaxZ:   1.0,
        };
        let _ = device.SetViewport(&viewport,);

        set_state(&device, D3DRS_ZENABLE, D3DZB_FALSE.0,);
        set_state(&device, D3DRS_ZWRITEENABLE, 0,);
        set_state(&device, D3DRS_CULLMODE, D3DCULL_NONE.0,);
        set_state(&device, D3DRS_LIGHTING, 0,);
        set_state(&device, D3DRS_ALPHATESTENABLE, 0,);
        set_state(&device, D3DRS_STENCILENABLE, 0,);
        set_state(&device, D3DRS_FOGENABLE, 0,);
        set_state(&device, D3DRS_CLIPPING, 0,);
        set_state(&device, D3DRS_COLORVERTEX, 1,);
        set_state(&device, D3DRS_SRGBWRITEENABLE, 0,);
        set_state(&device, D3DRS_SCISSORTESTENABLE, 1,);
        set_state(&device, D3DRS_ALPHABLENDENABLE, 1,);
        set_state(&device, D3DRS_SRCBLEND, D3DBLEND_ONE.0,);
        set_state(&device, D3DRS_DESTBLEND, D3DBLEND_INVSRCALPHA.0,);
        set_state(&device, D3DRS_BLENDOP, D3DBLENDOP_ADD.0,);
        set_state(&device, D3DRS_SEPARATEALPHABLENDENABLE, 1,);
        set_state(&device, D3DRS_SRCBLENDALPHA, D3DBLEND_ONE.0,);
        set_state(&device, D3DRS_DESTBLENDALPHA, D3DBLEND_INVSRCALPHA.0,);
        set_state(&device, D3DRS_BLENDOPALPHA, D3DBLENDOP_ADD.0,);

        let _ = device.SetSamplerState(0, D3DSAMP_MINFILTER, D3DTEXF_LINEAR.0 as u32,);
        let _ = device.SetSamplerState(0, D3DSAMP_MAGFILTER, D3DTEXF_LINEAR.0 as u32,);
        let _ = device.SetSamplerState(0, D3DSAMP_MIPFILTER, D3DTEXF_NONE.0 as u32,);
        let _ = device.SetSamplerState(0, D3DSAMP_ADDRESSU, D3DTADDRESS_CLAMP.0 as u32,);
        let _ = device.SetSamplerState(0, D3DSAMP_ADDRESSV, D3DTADDRESS_CLAMP.0 as u32,);
        let _ = device.SetSamplerState(0, D3DSAMP_ADDRESSW, D3DTADDRESS_CLAMP.0 as u32,);

        let params = [batch.screen.0 as f32, batch.screen.1 as f32, 0.0f32, 0.0f32,];
        let _ = device.SetVertexShaderConstantF(0, params.as_ptr(), 1,);

        let _ = device.SetVertexDeclaration(&self.decl,);
        let _ = device.SetVertexShader(&self.vs,);
        let _ = device.SetPixelShader(&self.ps,);

        let mut data: *mut c_void = std::ptr::null_mut();
        if vb.Lock(0, 0, &mut data, D3DLOCK_DISCARD as u32,).is_err() || data.is_null()
        {
            return;
        }
        std::ptr::copy_nonoverlapping(vertex_bytes.as_ptr(), data as *mut u8, vertex_bytes.len(),);
        let _ = vb.Unlock();

        let mut data: *mut c_void = std::ptr::null_mut();
        if ib.Lock(0, 0, &mut data, D3DLOCK_DISCARD as u32,).is_err() || data.is_null()
        {
            return;
        }
        std::ptr::copy_nonoverlapping(index_bytes.as_ptr(), data as *mut u8, index_bytes.len(),);
        let _ = ib.Unlock();

        let _ = device.SetStreamSource(0, &vb, 0, VERTEX_SIZE as u32,);
        let _ = device.SetIndices(&ib,);

        let vertices = batch.verts.len() as u32;
        let mut failed = false;
        for draw in &batch.draws
        {
            let texture = match self.tex.get(&draw.tex,)
            {
                Some(entry,) => entry.tex.clone(),
                None => self.white.clone(),
            };
            let _ = device.SetTexture(0, &texture,);
            let scissor = RECT {
                left:   draw.scissor[0],
                top:    draw.scissor[1],
                right:  draw.scissor[2],
                bottom: draw.scissor[3],
            };
            let _ = device.SetScissorRect(&scissor,);
            if let Err(e,) =
                device.DrawIndexedPrimitive(D3DPT_TRIANGLELIST, 0, 0, vertices, draw.idx_start, draw.idx_count / 3,)
            {
                if !failed
                {
                    // one line per frame at most: a rejected state tells us exactly
                    // which precondition (scene, stream, declaration) is missing
                    log_info!("d3d9", format!("draw rejected: {}", e.message()));
                    failed = true;
                }
            }
        }
    }
}
