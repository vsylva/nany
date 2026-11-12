use std::{collections::HashMap, ffi::c_void};

use crate::{log_error, log_info};

type SerializeRootSignatureFn = unsafe extern "system" fn(
    *const D3D12_ROOT_SIGNATURE_DESC,
    D3D_ROOT_SIGNATURE_VERSION,
    *mut c_void,
    *mut c_void,
) -> i32;
use std::mem::ManuallyDrop;

use windows::{
    Win32::{
        Foundation::{BOOL, HANDLE, RECT},
        Graphics::{
            Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            Direct3D12::*,
            Dxgi::{
                Common::{
                    DXGI_FORMAT, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R32_UINT,
                    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
                },
                IDXGISwapChain, IDXGISwapChain3,
            },
        },
    },
    core::{Interface, Result},
};

use super::{
    mesh::{self, Batch, VERTEX_SIZE},
    shader,
};
use crate::win;

const SRV_HEAP: u32 = 512;
const RTV_HEAP: u32 = 16;
const FRAMES: usize = 3;
const UPLOAD_MIN: u64 = 2 * 1024 * 1024;

const TEX_PITCH_ALIGN: u64 = 256;

const TEX_PLACE_ALIGN: u64 = 512;

struct TexEntry {
    w:   usize,
    h:   usize,
    res: ID3D12Resource,

    _upload: ID3D12Resource,
    idx:     u32,
}

struct Pending {
    key:       u64,
    w:         usize,
    h:         usize,
    row_pitch: u32,
    res:       ID3D12Resource,
    upload:    ID3D12Resource,
    idx:       u32,
}

struct Patch {
    key:   u64,
    x:     usize,
    y:     usize,
    w:     usize,
    h:     usize,
    bytes: Vec<u8,>,
}

struct Slot {
    alloc: ID3D12CommandAllocator,
    fence: u64,
}

pub struct Painter12 {
    device:     ID3D12Device,
    queue:      ID3D12CommandQueue,
    root:       ID3D12RootSignature,
    pso:        ID3D12PipelineState,
    pso_format: DXGI_FORMAT,
    srv_heap:   ID3D12DescriptorHeap,
    srv_inc:    u32,
    srv_cpu0:   D3D12_CPU_DESCRIPTOR_HANDLE,
    srv_gpu0:   D3D12_GPU_DESCRIPTOR_HANDLE,
    next_desc:  u32,

    free_desc:      Vec<u32,>,
    tex:            HashMap<u64, TexEntry,>,
    pending:        Vec<Pending,>,
    patches:        Vec<Patch,>,
    white_res:      Option<ID3D12Resource,>,
    rtv_heap:       ID3D12DescriptorHeap,
    rtv_inc:        u32,
    rtv_cpu0:       D3D12_CPU_DESCRIPTOR_HANDLE,
    rtv_map:        HashMap<usize, u32,>,
    next_rtv:       u32,
    slots:          [Slot; FRAMES],
    slot:           usize,
    list:           ID3D12GraphicsCommandList,
    fence:          ID3D12Fence,
    fence_value:    u64,
    event:          HANDLE,
    _upload:        ID3D12Resource,
    upload_ptr:     *mut u8,
    upload_gpu:     u64,
    upload_size:    u64,
    upload_off:     u64,
    retire:         Vec<(u64, Vec<ID3D12Resource,>,),>,
    retire_pso:     Vec<(u64, ID3D12PipelineState,),>,
    size:           (u32, u32,),
    format:         DXGI_FORMAT,
    warned:         bool,
    warned_patch:   bool,
    logged_patch:   bool,
    frames:         u64,
    pub device_raw: usize,
}

unsafe impl Send for Painter12 {}

unsafe fn borrow(res: &ID3D12Resource,) -> ManuallyDrop<Option<ID3D12Resource,>,> {
    ManuallyDrop::new(Some(ID3D12Resource::from_raw(res.as_raw(),),),)
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
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource:   borrow(res,),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter:  after,
            },),
        },
    }
}

unsafe fn buf_desc(size: u64,) -> D3D12_RESOURCE_DESC {
    D3D12_RESOURCE_DESC {
        Dimension:        D3D12_RESOURCE_DIMENSION_BUFFER,
        Alignment:        0,
        Width:            size,
        Height:           1,
        DepthOrArraySize: 1,
        MipLevels:        1,
        Format:           DXGI_FORMAT_UNKNOWN,
        SampleDesc:       DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
        Layout:           D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags:            D3D12_RESOURCE_FLAG_NONE,
    }
}

unsafe fn tex_desc(w: usize, h: usize,) -> D3D12_RESOURCE_DESC {
    D3D12_RESOURCE_DESC {
        Dimension:        D3D12_RESOURCE_DIMENSION_TEXTURE2D,
        Alignment:        0,
        Width:            w as u64,
        Height:           h as u32,
        DepthOrArraySize: 1,
        MipLevels:        1,
        Format:           DXGI_FORMAT_R8_UNORM,
        SampleDesc:       DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },

        Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
        Flags:  D3D12_RESOURCE_FLAG_NONE,
    }
}

unsafe fn heap_props(ty: D3D12_HEAP_TYPE,) -> D3D12_HEAP_PROPERTIES {
    D3D12_HEAP_PROPERTIES {
        Type:                 ty,
        CPUPageProperty:      D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
        CreationNodeMask:     1,
        VisibleNodeMask:      1,
    }
}

unsafe fn blob_bytes(blob: &windows::Win32::Graphics::Direct3D::ID3DBlob,) -> Vec<u8,> {
    let n = blob.GetBufferSize();
    let p = blob.GetBufferPointer() as *const u8;
    std::slice::from_raw_parts(p, n,).to_vec()
}

impl Painter12 {
    pub unsafe fn new(device: &ID3D12Device, queue: &ID3D12CommandQueue, format: DXGI_FORMAT,) -> Result<Self,> {
        let ranges = [D3D12_DESCRIPTOR_RANGE {
            RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
            NumDescriptors: 1,
            BaseShaderRegister: 0,
            RegisterSpace: 0,
            OffsetInDescriptorsFromTableStart: 0,
        },];
        let params = [
            D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
                Anonymous:     D3D12_ROOT_PARAMETER_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR { ShaderRegister: 0, RegisterSpace: 0, },
                },

                ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
            },
            D3D12_ROOT_PARAMETER {
                ParameterType:    D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                Anonymous:        D3D12_ROOT_PARAMETER_0 {
                    DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                        NumDescriptorRanges: 1,
                        pDescriptorRanges:   ranges.as_ptr(),
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
            },
        ];
        let sampler = D3D12_STATIC_SAMPLER_DESC {
            Filter:           D3D12_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU:         D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            AddressV:         D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            AddressW:         D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            MipLODBias:       0.0,
            MaxAnisotropy:    1,
            ComparisonFunc:   D3D12_COMPARISON_FUNC_ALWAYS,
            BorderColor:      D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK,
            MinLOD:           0.0,
            MaxLOD:           f32::MAX,
            ShaderRegister:   0,
            RegisterSpace:    0,
            ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
        };
        let rs_desc = D3D12_ROOT_SIGNATURE_DESC {
            NumParameters:     params.len() as u32,
            pParameters:       params.as_ptr(),
            NumStaticSamplers: 1,
            pStaticSamplers:   &sampler,
            Flags:             D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
        };
        let mut blob: Option<windows::Win32::Graphics::Direct3D::ID3DBlob,> = None;
        let mut err: Option<windows::Win32::Graphics::Direct3D::ID3DBlob,> = None;

        let raw = crate::api::resolve("d3d12.dll", b"D3D12SerializeRootSignature",);
        if raw == 0
        {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(0x8000_4005u32 as i32,),
                "d3d12.dll!D3D12SerializeRootSignature not resolved",
            ),);
        }
        let serialize: SerializeRootSignatureFn = std::mem::transmute(raw,);
        let hr = serialize(
            &rs_desc,
            D3D_ROOT_SIGNATURE_VERSION_1,
            &mut blob as *mut _ as *mut c_void,
            &mut err as *mut _ as *mut c_void,
        );
        if hr < 0
        {
            let detail = err.map(|b| String::from_utf8_lossy(&blob_bytes(&b,),).into_owned(),).unwrap_or_default();
            return Err(windows::core::Error::new(
                windows::core::HRESULT(hr,),
                format!("SerializeRootSignature: {detail}"),
            ),);
        }
        let Some(blob,) = blob
        else
        {
            return Err(windows::core::Error::new(windows::core::HRESULT(0x8000_4005u32 as i32,), "empty root blob",),);
        };
        let root: ID3D12RootSignature = device.CreateRootSignature(0, &blob_bytes(&blob,),)?;

        let pso = Self::create_pso(device, &root, format,)?;

        let srv_heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            Type:           D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            NumDescriptors: SRV_HEAP,
            Flags:          D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask:       0,
        },)?;
        let rtv_heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            Type:           D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: RTV_HEAP,
            Flags:          D3D12_DESCRIPTOR_HEAP_FLAGS(0,),
            NodeMask:       0,
        },)?;

        let slots: [Slot; FRAMES] = [
            Slot {
                alloc: device.CreateCommandAllocator::<ID3D12CommandAllocator>(D3D12_COMMAND_LIST_TYPE_DIRECT,)?,
                fence: 0,
            },
            Slot {
                alloc: device.CreateCommandAllocator::<ID3D12CommandAllocator>(D3D12_COMMAND_LIST_TYPE_DIRECT,)?,
                fence: 0,
            },
            Slot {
                alloc: device.CreateCommandAllocator::<ID3D12CommandAllocator>(D3D12_COMMAND_LIST_TYPE_DIRECT,)?,
                fence: 0,
            },
        ];
        let list: ID3D12GraphicsCommandList =
            device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &slots[0].alloc, &pso,)?;
        list.Close()?;

        let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE,)?;
        let event_raw = win::CreateEventW(std::ptr::null(), 0, 0, std::ptr::null(),);
        if event_raw.is_null()
        {
            return Err(windows::core::Error::from_win32(),);
        }
        let event = HANDLE(event_raw,);

        let mut upload_res: Option<ID3D12Resource,> = None;
        device.CreateCommittedResource(
            &heap_props(D3D12_HEAP_TYPE_UPLOAD,),
            D3D12_HEAP_FLAG_NONE,
            &buf_desc(UPLOAD_MIN,),
            D3D12_RESOURCE_STATE_GENERIC_READ,
            None,
            &mut upload_res,
        )?;
        let upload = upload_res.ok_or_else(windows::core::Error::from_win32,)?;
        let mut ptr: *mut c_void = std::ptr::null_mut();
        upload.Map(0, None, Some(&mut ptr,),)?;
        let upload_gpu = upload.GetGPUVirtualAddress();

        let mut p = Painter12 {
            device: device.clone(),
            queue: queue.clone(),
            root,
            pso,
            pso_format: format,
            srv_heap,
            srv_inc: device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,),
            srv_cpu0: D3D12_CPU_DESCRIPTOR_HANDLE::default(),
            srv_gpu0: D3D12_GPU_DESCRIPTOR_HANDLE::default(),
            next_desc: 1,
            free_desc: Vec::new(),
            tex: HashMap::new(),
            pending: Vec::new(),
            patches: Vec::new(),
            white_res: None,
            rtv_heap,
            rtv_inc: device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV,),
            rtv_cpu0: D3D12_CPU_DESCRIPTOR_HANDLE::default(),
            rtv_map: HashMap::new(),
            next_rtv: 0,
            slots,
            slot: 0,
            list,
            fence,
            fence_value: 0,
            event,
            _upload: upload,
            upload_ptr: ptr as *mut u8,
            upload_gpu,
            upload_size: UPLOAD_MIN,
            upload_off: 0,
            retire: Vec::new(),
            retire_pso: Vec::new(),
            size: (0, 0,),
            format,
            warned: false,
            warned_patch: false,
            logged_patch: false,
            frames: 0,
            device_raw: device.as_raw() as usize,
        };
        p.srv_cpu0 = p.srv_heap.GetCPUDescriptorHandleForHeapStart();
        p.srv_gpu0 = p.srv_heap.GetGPUDescriptorHandleForHeapStart();
        p.rtv_cpu0 = p.rtv_heap.GetCPUDescriptorHandleForHeapStart();

        if !p.init_white()
        {
            log_error!("d3d12", "white texture init failed");
        }
        log_info!("d3d12", format!("ready, pso format {}", format.0));
        Ok(p,)
    }

    unsafe fn create_pso(
        device: &ID3D12Device,
        root: &ID3D12RootSignature,
        format: DXGI_FORMAT,
    ) -> Result<ID3D12PipelineState,> {
        let vs = shader::compile(super::HLSL, "vs_main", "vs_5_0",)
            .map_err(|e| windows::core::Error::new(windows::core::HRESULT(0x8000_4005u32 as i32,), e,),)?;
        let ps = shader::compile(super::HLSL, "ps_main", "ps_5_0",)
            .map_err(|e| windows::core::Error::new(windows::core::HRESULT(0x8000_4005u32 as i32,), e,),)?;

        let elems = [
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName:         windows::core::s!("POSITION"),
                SemanticIndex:        0,
                Format:               DXGI_FORMAT_R32G32_FLOAT,
                InputSlot:            0,
                AlignedByteOffset:    0,
                InputSlotClass:       D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName:         windows::core::s!("TEXCOORD"),
                SemanticIndex:        0,
                Format:               DXGI_FORMAT_R32G32_FLOAT,
                InputSlot:            0,
                AlignedByteOffset:    8,
                InputSlotClass:       D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName:         windows::core::s!("COLOR"),
                SemanticIndex:        0,
                Format:               DXGI_FORMAT_R8G8B8A8_UNORM,
                InputSlot:            0,
                AlignedByteOffset:    16,
                InputSlotClass:       D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];

        let mut bd = D3D12_BLEND_DESC::default();
        bd.RenderTarget[0].BlendEnable = BOOL(1,);
        bd.RenderTarget[0].SrcBlend = D3D12_BLEND_ONE;
        bd.RenderTarget[0].DestBlend = D3D12_BLEND_INV_SRC_ALPHA;
        bd.RenderTarget[0].BlendOp = D3D12_BLEND_OP_ADD;
        bd.RenderTarget[0].SrcBlendAlpha = D3D12_BLEND_ONE;
        bd.RenderTarget[0].DestBlendAlpha = D3D12_BLEND_INV_SRC_ALPHA;
        bd.RenderTarget[0].BlendOpAlpha = D3D12_BLEND_OP_ADD;
        bd.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8;

        let rd = D3D12_RASTERIZER_DESC {
            FillMode: D3D12_FILL_MODE_SOLID,
            CullMode: D3D12_CULL_MODE_NONE,
            DepthClipEnable: BOOL(1,),
            ..Default::default()
        };

        let ds = D3D12_DEPTH_STENCIL_DESC {
            DepthEnable: BOOL(0,),
            StencilEnable: BOOL(0,),
            DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
            DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
            ..Default::default()
        };

        let mut formats = [DXGI_FORMAT_UNKNOWN; 8];
        formats[0] = format;

        let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            pRootSignature:        ManuallyDrop::new(Some(root.clone(),),),
            VS:                    D3D12_SHADER_BYTECODE {
                pShaderBytecode: vs.as_ptr() as *const c_void,
                BytecodeLength:  vs.len(),
            },
            PS:                    D3D12_SHADER_BYTECODE {
                pShaderBytecode: ps.as_ptr() as *const c_void,
                BytecodeLength:  ps.len(),
            },
            DS:                    D3D12_SHADER_BYTECODE::default(),
            HS:                    D3D12_SHADER_BYTECODE::default(),
            GS:                    D3D12_SHADER_BYTECODE::default(),
            StreamOutput:          D3D12_STREAM_OUTPUT_DESC::default(),
            BlendState:            bd,
            SampleMask:            u32::MAX,
            RasterizerState:       rd,
            DepthStencilState:     ds,
            InputLayout:           D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: elems.as_ptr(),
                NumElements:        elems.len() as u32,
            },
            IBStripCutValue:       D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
            PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            NumRenderTargets:      1,
            RTVFormats:            formats,
            DSVFormat:             DXGI_FORMAT_UNKNOWN,
            SampleDesc:            DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            NodeMask:              0,
            CachedPSO:             D3D12_CACHED_PIPELINE_STATE::default(),
            Flags:                 D3D12_PIPELINE_STATE_FLAG_NONE,
        };
        device.CreateGraphicsPipelineState(&desc,)
    }

    unsafe fn srv_cpu(&self, idx: u32,) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        D3D12_CPU_DESCRIPTOR_HANDLE { ptr: self.srv_cpu0.ptr + (idx * self.srv_inc) as usize, }
    }

    unsafe fn srv_gpu(&self, idx: u32,) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        D3D12_GPU_DESCRIPTOR_HANDLE { ptr: self.srv_gpu0.ptr + (idx as u64 * self.srv_inc as u64), }
    }

    unsafe fn rtv_cpu(&self, idx: u32,) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        D3D12_CPU_DESCRIPTOR_HANDLE { ptr: self.rtv_cpu0.ptr + (idx * self.rtv_inc) as usize, }
    }

    unsafe fn make_srv(&self, res: &ID3D12Resource, idx: u32,) {
        let desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format:                  DXGI_FORMAT_R8_UNORM,
            ViewDimension:           D3D12_SRV_DIMENSION_TEXTURE2D,
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            Anonymous:               D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_SRV {
                    MostDetailedMip:     0,
                    MipLevels:           1,
                    PlaneSlice:          0,
                    ResourceMinLODClamp: 0.0,
                },
            },
        };
        self.device.CreateShaderResourceView(res, Some(&desc,), self.srv_cpu(idx,),)
    }

    unsafe fn new_texture(&self, w: usize, h: usize,) -> Option<(ID3D12Resource, ID3D12Resource, u32,),> {
        let mut res: Option<ID3D12Resource,> = None;
        if let Err(e,) = self.device.CreateCommittedResource(
            &heap_props(D3D12_HEAP_TYPE_DEFAULT,),
            D3D12_HEAP_FLAG_NONE,
            &tex_desc(w, h,),
            D3D12_RESOURCE_STATE_COPY_DEST,
            None,
            &mut res,
        )
        {
            log_error!("d3d12", format!("CreateCommittedResource(tex {w}x{h}) failed: 0x{:08X}", e.code().0));
            return None;
        }
        let res = res?;
        let row_pitch = ((w + 255) & !255) as u32;
        let total = row_pitch as u64 * h as u64;
        let mut up: Option<ID3D12Resource,> = None;
        if let Err(e,) = self.device.CreateCommittedResource(
            &heap_props(D3D12_HEAP_TYPE_UPLOAD,),
            D3D12_HEAP_FLAG_NONE,
            &buf_desc(total,),
            D3D12_RESOURCE_STATE_GENERIC_READ,
            None,
            &mut up,
        )
        {
            log_error!("d3d12", format!("CreateCommittedResource(upload {total}) failed: 0x{:08X}", e.code().0));
            return None;
        }
        Some((res, up?, row_pitch,),)
    }

    unsafe fn fill_upload(&self, upload: &ID3D12Resource, w: usize, h: usize, row_pitch: u32, data: &[u8],) {
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if upload.Map(0, None, Some(&mut ptr,),).is_err()
        {
            return;
        }
        let base = ptr as *mut u8;
        for row in 0..h
        {
            let src = &data[row * w..row * w + w];
            std::ptr::copy_nonoverlapping(src.as_ptr(), base.add(row * row_pitch as usize,), w,);
        }
        upload.Unmap(0, None,);
    }

    unsafe fn init_white(&mut self,) -> bool {
        let Some((res, upload, row_pitch,),) = self.new_texture(1, 1,)
        else
        {
            log_error!("d3d12", "white: resource creation failed");
            return false;
        };
        self.fill_upload(&upload, 1, 1, row_pitch, &[255u8,],);

        if let Err(e,) = self.slots[0].alloc.Reset()
        {
            log_error!("d3d12", format!("white: allocator reset failed 0x{:08X}", e.code().0));
            return false;
        }
        if let Err(e,) = self.list.Reset(&self.slots[0].alloc, &self.pso,)
        {
            log_error!("d3d12", format!("white: list reset failed 0x{:08X}", e.code().0));
            return false;
        }
        let dst = D3D12_TEXTURE_COPY_LOCATION {
            pResource: borrow(&res,),
            Type:      D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0, },
        };
        let src = D3D12_TEXTURE_COPY_LOCATION {
            pResource: borrow(&upload,),
            Type:      D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                    Offset:    0,
                    Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                        Format:   DXGI_FORMAT_R8_UNORM,
                        Width:    1,
                        Height:   1,
                        Depth:    1,
                        RowPitch: row_pitch,
                    },
                },
            },
        };
        self.list.CopyTextureRegion(&dst, 0, 0, 0, &src, None,);
        let b = transition(&res, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,);
        self.list.ResourceBarrier(&[b,],);
        if let Err(e,) = self.list.Close()
        {
            log_error!("d3d12", format!("white: list close failed 0x{:08X}", e.code().0));
            return false;
        }
        let Ok(cmd,) = self.list.cast::<ID3D12CommandList>()
        else
        {
            return false;
        };
        self.queue.ExecuteCommandLists(&[Some(cmd,),],);
        self.fence_value += 1;
        let v = self.fence_value;
        let _ = self.queue.Signal(&self.fence, v,);
        let _ = self.fence.SetEventOnCompletion(v, self.event,);
        win::WaitForSingleObject(self.event.0, u32::MAX,);

        self.make_srv(&res, 0,);
        self.white_res = Some(res,);
        let _ = upload;
        true
    }

    unsafe fn wait_value(&self, value: u64,) {
        if self.fence.GetCompletedValue() >= value
        {
            return;
        }
        if self.fence.SetEventOnCompletion(value, self.event,).is_ok()
        {
            win::WaitForSingleObject(self.event.0, u32::MAX,);
        }
    }

    unsafe fn ensure_upload(&mut self, need: u64,) -> bool {
        if need <= self.upload_size
        {
            return true;
        }
        self.wait_value(self.fence_value,);

        let mut cap = UPLOAD_MIN;
        while cap < need
        {
            cap = cap.saturating_mul(2,);
        }
        let mut res: Option<ID3D12Resource,> = None;
        let ok = self.device.CreateCommittedResource(
            &heap_props(D3D12_HEAP_TYPE_UPLOAD,),
            D3D12_HEAP_FLAG_NONE,
            &buf_desc(cap,),
            D3D12_RESOURCE_STATE_GENERIC_READ,
            None,
            &mut res,
        );
        if ok.is_err()
        {
            return false;
        }
        let Some(res,) = res
        else
        {
            return false;
        };
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if res.Map(0, None, Some(&mut ptr,),).is_err()
        {
            return false;
        }
        self.upload_gpu = res.GetGPUVirtualAddress();
        self.upload_ptr = ptr as *mut u8;
        self.upload_size = cap;
        self.upload_off = 0;
        self._upload = res;
        log_info!("d3d12", format!("upload ring grown to {} MB", cap / 1024 / 1024));
        true
    }

    unsafe fn reserve_upload(&mut self, need: u64,) -> bool {
        if need > self.upload_size && !self.ensure_upload(need,)
        {
            return false;
        }
        if self.upload_off + need > self.upload_size
        {
            self.wait_value(self.fence_value,);
            self.upload_off = 0;
        }
        true
    }

    unsafe fn upload_bytes(&mut self, bytes: &[u8],) -> u64 {
        let len = ((bytes.len() as u64) + 255) & !255;
        let va = self.upload_gpu + self.upload_off;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.upload_ptr.add(self.upload_off as usize,), bytes.len(),);
        self.upload_off += len;
        va
    }

    unsafe fn upload_slice(&mut self, w: usize, h: usize, bytes: &[u8],) -> Option<(u64, u64,),> {
        let pitch = ((w as u64) + TEX_PITCH_ALIGN - 1) & !(TEX_PITCH_ALIGN - 1);
        let size = (pitch * h as u64 + TEX_PLACE_ALIGN - 1) & !(TEX_PLACE_ALIGN - 1);
        let off = (self.upload_off + TEX_PLACE_ALIGN - 1) & !(TEX_PLACE_ALIGN - 1);
        if bytes.len() < w * h || off + size > self.upload_size
        {
            return None;
        }
        let dst = self.upload_ptr.add(off as usize,);
        for row in 0..h
        {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().add(row * w,), dst.add(row * pitch as usize,), w,);
        }
        self.upload_off = off + size;
        Some((off, pitch,),)
    }

    pub unsafe fn on_resize_begin(&mut self,) {
        self.wait_value(self.fence_value,);

        if self.list.Reset(&self.slots[0].alloc, None::<&ID3D12PipelineState,>,).is_ok()
        {
            let _ = self.list.Close();
        }
        self.rtv_map.clear();
        self.next_rtv = 0;
        self.size = (0, 0,);
        log_info!("d3d12", "gpu idle, command list reset, rtv cache dropped");
    }

    pub unsafe fn queue_raw(&self,) -> usize {
        self.queue.as_raw() as usize
    }

    pub unsafe fn ensure_target(&mut self, sc: &IDXGISwapChain, w: u32, h: u32,) {
        let Ok(desc,) = sc.GetDesc()
        else
        {
            return;
        };
        let format = desc.BufferDesc.Format;
        if self.size == (w, h,) && self.format == format
        {
            return;
        }
        self.size = (w, h,);
        self.format = format;
        self.rtv_map.clear();
        self.next_rtv = 0;
        if format != self.pso_format
        {
            if let Ok(pso,) = Self::create_pso(&self.device, &self.root, format,)
            {
                let old = std::mem::replace(&mut self.pso, pso,);
                // in-flight command lists still reference the old pipeline;
                // d3d12 requires the app to keep it alive until they finished
                self.retire_pso.push((self.fence_value + 1, old,),);
                self.pso_format = format;
                log_info!("d3d12", format!("pso rebuilt for format {}", format.0));
            }
        }
        log_info!("d3d12", format!("target {w}x{h} fmt={}", format.0));
    }

    unsafe fn rtv_for(&mut self, res: &ID3D12Resource,) -> Option<u32,> {
        let key = res.as_raw() as usize;
        if let Some(&i,) = self.rtv_map.get(&key,)
        {
            return Some(i,);
        }
        if self.next_rtv >= RTV_HEAP
        {
            self.rtv_map.clear();
            self.next_rtv = 0;
        }
        let idx = self.next_rtv;
        let desc = D3D12_RENDER_TARGET_VIEW_DESC {
            Format:        self.format,
            ViewDimension: D3D12_RTV_DIMENSION_TEXTURE2D,
            Anonymous:     D3D12_RENDER_TARGET_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_RTV { MipSlice: 0, PlaneSlice: 0, },
            },
        };
        self.device.CreateRenderTargetView(res, Some(&desc,), self.rtv_cpu(idx,),);
        self.rtv_map.insert(key, idx,);
        self.next_rtv += 1;
        Some(idx,)
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
                if w == 0 || h == 0 || alpha.len() < w * h
                {
                    continue;
                }

                match image_delta.pos
                {
                    None =>
                    {
                        let Some((res, upload, row_pitch,),) = self.new_texture(w, h,)
                        else
                        {
                            continue;
                        };
                        let idx = match self.free_desc.pop()
                        {
                            Some(i,) => i,
                            None =>
                            {
                                let i = self.next_desc;
                                if i >= SRV_HEAP
                                {
                                    log_info!("d3d12", "srv heap exhausted");
                                    continue;
                                }
                                self.next_desc += 1;
                                i
                            }
                        };
                        self.fill_upload(&upload, w, h, row_pitch, &alpha,);
                        self.pending.push(Pending { key, w, h, row_pitch, res, upload, idx, },);
                    }

                    Some([x, y,],) =>
                    {
                        if self.tex.contains_key(&key,) || self.pending.iter().any(|p| p.key == key,)
                        {
                            self.patches.push(Patch { key, x, y, w, h, bytes: alpha, },);
                        }
                        else if !self.warned_patch
                        {
                            log_info!(
                                "d3d12",
                                format!("patch for unknown texture {key:#x} at ({x},{y}) {w}x{h}, dropped")
                            );
                            self.warned_patch = true;
                        }
                    }
                }
            }
        }
        for id in &delta.free
        {
            if let Some(old,) = self.tex.remove(&mesh::tex_key(*id,),)
            {
                self.free_desc.push(old.idx,);
                self.retire.push((self.fence_value + 1, vec![old.res, old._upload],),);
            }
        }
    }

    pub unsafe fn paint(&mut self, sc: &IDXGISwapChain, prims: &[egui::ClippedPrimitive], screen: (u32, u32,),) {
        let batch = mesh::build(prims, screen,);
        if batch.is_empty() && self.pending.is_empty() && self.patches.is_empty()
        {
            return;
        }
        if let Err(e,) = self.record_and_submit(sc, &batch,)
        {
            if !self.warned
            {
                log_info!("d3d12", format!("frame error: {e}"));
                self.warned = true;
            }
        }
    }

    unsafe fn record_and_submit(&mut self, sc: &IDXGISwapChain, batch: &Batch,) -> Result<(),> {
        let done = self.fence.GetCompletedValue();
        self.retire.retain(|(v, _,)| *v > done,);
        self.retire_pso.retain(|(v, _,)| *v > done,);

        let sc3: IDXGISwapChain3 = sc.cast()?;
        let idx = sc3.GetCurrentBackBufferIndex();
        let bb: ID3D12Resource = sc3.GetBuffer(idx,)?;
        let Some(rtv_idx,) = self.rtv_for(&bb,)
        else
        {
            return Ok((),);
        };
        let rtv = self.rtv_cpu(rtv_idx,);

        let completed = self.fence.GetCompletedValue();
        let slot = match (0..self.slots.len()).find(|i| self.slots[*i].fence <= completed,)
        {
            Some(i,) => i,
            None =>
            {
                let oldest = (0..self.slots.len()).min_by_key(|i| self.slots[*i].fence,).unwrap_or(0,);
                self.wait_value(self.slots[oldest].fence,);
                oldest
            }
        };
        self.slot = (slot + 1) % self.slots.len();
        self.slots[slot].alloc.Reset()?;
        self.list.Reset(&self.slots[slot].alloc, &self.pso,)?;
        self.list.SetGraphicsRootSignature(&self.root,);
        self.list.SetPipelineState(&self.pso,);
        self.list.SetDescriptorHeaps(&[Some(self.srv_heap.clone(),),],);

        let pending = std::mem::take(&mut self.pending,);
        let patches = std::mem::take(&mut self.patches,);

        let mut patch_bytes = 0u64;
        for p in &patches
        {
            let pitch = ((p.w as u64) + TEX_PITCH_ALIGN - 1) & !(TEX_PITCH_ALIGN - 1);
            // + TEX_PLACE_ALIGN covers the offset alignment upload_slice applies per rect
            patch_bytes += ((pitch * p.h as u64 + TEX_PLACE_ALIGN - 1) & !(TEX_PLACE_ALIGN - 1)) + TEX_PLACE_ALIGN;
        }
        let geo_bytes = if batch.is_empty()
        {
            0
        }
        else
        {
            // upload_bytes rounds every chunk up to 256 bytes; reserve with
            // the same rounding so the ring is never written past its end
            ((batch.vertex_bytes().len() as u64 + 255) & !255) + ((batch.index_bytes().len() as u64 + 255) & !255) + 256
        };
        let need = patch_bytes + geo_bytes;
        if need > 0 && !self.reserve_upload(need,)
        {
            return Ok((),);
        }

        for p in pending
        {
            let dst = D3D12_TEXTURE_COPY_LOCATION {
                pResource: borrow(&p.res,),
                Type:      D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0, },
            };
            let src = D3D12_TEXTURE_COPY_LOCATION {
                pResource: borrow(&p.upload,),
                Type:      D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset:    0,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format:   DXGI_FORMAT_R8_UNORM,
                            Width:    p.w as u32,
                            Height:   p.h as u32,
                            Depth:    1,
                            RowPitch: p.row_pitch,
                        },
                    },
                },
            };
            self.list.CopyTextureRegion(&dst, 0, 0, 0, &src, None,);
            let b = transition(&p.res, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,);
            self.list.ResourceBarrier(&[b,],);
            self.make_srv(&p.res, p.idx,);

            let entry = TexEntry { w: p.w, h: p.h, res: p.res, _upload: p.upload, idx: p.idx, };
            if let Some(old,) = self.tex.insert(p.key, entry,)
            {
                self.free_desc.push(old.idx,);
                self.retire.push((self.fence_value + 1, vec![old.res, old._upload],),);
            }
        }

        let mut i = 0;
        let mut patched = 0usize;
        while i < patches.len()
        {
            let key = patches[i].key;
            let mut j = i + 1;
            while j < patches.len() && patches[j].key == key
            {
                j += 1;
            }
            let group: Vec<&Patch,> = {
                let Some((tw, th,),) = self.tex.get(&key,).map(|e| (e.w, e.h,),)
                else
                {
                    i = j;
                    continue;
                };
                patches[i..j].iter().filter(|p| p.x + p.w <= tw && p.y + p.h <= th,).collect()
            };
            if group.is_empty()
            {
                i = j;
                continue;
            }
            let Some(res,) = self.tex.get(&key,).map(|e| e.res.clone(),)
            else
            {
                i = j;
                continue;
            };

            let to_copy = transition(&res, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_COPY_DEST,);
            self.list.ResourceBarrier(&[to_copy,],);
            for p in group
            {
                let Some((off, pitch,),) = self.upload_slice(p.w, p.h, &p.bytes,)
                else
                {
                    continue;
                };
                let dst = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: borrow(&res,),
                    Type:      D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0, },
                };
                let src = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: borrow(&self._upload,),
                    Type:      D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                        PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                            Offset:    off,
                            Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                                Format:   DXGI_FORMAT_R8_UNORM,
                                Width:    p.w as u32,
                                Height:   p.h as u32,
                                Depth:    1,
                                RowPitch: pitch as u32,
                            },
                        },
                    },
                };
                self.list.CopyTextureRegion(&dst, p.x as u32, p.y as u32, 0, &src, None,);
                patched += 1;
            }
            let back = transition(&res, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,);
            self.list.ResourceBarrier(&[back,],);
            i = j;
        }
        if patched > 0 && !self.logged_patch
        {
            log_info!("d3d12", format!("texture patch path active: {patched} rect(s) this frame via upload ring"));
            self.logged_patch = true;
        }

        if !batch.is_empty()
        {
            let (w, h,) = batch.screen;
            let vbytes = batch.vertex_bytes();
            let ibytes = batch.index_bytes();
            let va = self.upload_bytes(vbytes,);
            let ia = self.upload_bytes(ibytes,);

            let to_rt = transition(&bb, D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,);
            self.list.ResourceBarrier(&[to_rt,],);

            let vp = D3D12_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width:    w as f32,
                Height:   h as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };
            self.list.RSSetViewports(&[vp,],);
            self.list.OMSetRenderTargets(1, Some(&rtv,), BOOL(0,), None,);
            self.list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,);

            let srgb_flag = if matches!(self.pso_format.0, 29 | 91) { 1.0f32 } else { 0.0f32 };
            let scr = [w as f32, h as f32, srgb_flag, 0.0f32,];
            let scr_bytes = std::slice::from_raw_parts(scr.as_ptr() as *const u8, 16,);
            let cb_va = self.upload_bytes(scr_bytes,);
            self.list.SetGraphicsRootConstantBufferView(0, cb_va,);

            let vbv = D3D12_VERTEX_BUFFER_VIEW {
                BufferLocation: va,
                SizeInBytes:    vbytes.len() as u32,
                StrideInBytes:  VERTEX_SIZE as u32,
            };
            self.list.IASetVertexBuffers(0, Some(&[vbv,],),);
            let ibv = D3D12_INDEX_BUFFER_VIEW {
                BufferLocation: ia,
                SizeInBytes:    ibytes.len() as u32,
                Format:         DXGI_FORMAT_R32_UINT,
            };
            self.list.IASetIndexBuffer(Some(&ibv,),);

            for d in &batch.draws
            {
                let srv = self.tex.get(&d.tex,).map(|t| t.idx,).unwrap_or(0,);
                self.list.SetGraphicsRootDescriptorTable(1, self.srv_gpu(srv,),);
                self.list.RSSetScissorRects(&[RECT {
                    left:   d.scissor[0],
                    top:    d.scissor[1],
                    right:  d.scissor[2],
                    bottom: d.scissor[3],
                },],);
                self.list.DrawIndexedInstanced(d.idx_count, 1, d.idx_start, 0, 0,);
            }

            let to_present = transition(&bb, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_PRESENT,);
            self.list.ResourceBarrier(&[to_present,],);
        }

        self.list.Close()?;
        let Ok(cmd,) = self.list.cast::<ID3D12CommandList>()
        else
        {
            return Ok((),);
        };
        self.queue.ExecuteCommandLists(&[Some(cmd,),],);
        self.fence_value += 1;
        let v = self.fence_value;
        self.queue.Signal(&self.fence, v,)?;
        self.slots[slot].fence = v;
        self.slot = (self.slot + 1) % FRAMES;

        if self.frames == 1
        {
            log_info!(
                "d3d12",
                format!(
                    "first submit: draws={} verts={} idx={} screen={:?} bb={}",
                    batch.draws.len(),
                    batch.verts.len(),
                    batch.idx.len(),
                    batch.screen,
                    idx
                )
            );
            self.dump_debug_messages();
        }
        self.frames += 1;
        Ok((),)
    }

    unsafe fn dump_debug_messages(&self,) {
        let Ok(q,) = self.device.cast::<ID3D12InfoQueue>()
        else
        {
            log_info!("d3d12", "[dbg] no ID3D12InfoQueue (debug layer off)");
            return;
        };
        let n = q.GetNumStoredMessages();
        log_info!("d3d12", format!("[dbg] infoqueue ok, {n} messages"));
        if n == 0
        {
            return;
        }
        for i in 0..n.min(25,)
        {
            let mut len = 0usize;
            if q.GetMessage(i, None, &mut len,).is_err() || len == 0
            {
                continue;
            }
            let mut buf = vec![0u8; len];
            let msg = buf.as_mut_ptr() as *mut D3D12_MESSAGE;
            if q.GetMessage(i, Some(msg,), &mut len,).is_err()
            {
                continue;
            }
            let m = &*msg;
            let desc = std::slice::from_raw_parts(m.pDescription, m.DescriptionByteLength,);
            let text = String::from_utf8_lossy(desc,).replace('\n', " ",);
            log_info!("d3d12", format!("[dbg] {} {:?}", m.ID.0, text));
        }
    }

    pub unsafe fn tex_count(&self,) -> usize {
        self.tex.len()
    }
}
