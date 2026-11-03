use std::{collections::HashMap, ffi::c_void};

use super::mesh::{self, Batch, VERTEX_SIZE};
use crate::{
    hook::vk::{DeviceInfo, SwapchainInfo, VkHandle, device_info, swapchain_info},
    log_error, log_info, log_warn,
};

include!(concat!(env!("OUT_DIR"), "/vk_spv.rs"));

const S_TYPE_SUBMIT_INFO: i32 = 4;
const S_TYPE_MEMORY_ALLOCATE_INFO: i32 = 5;
const S_TYPE_FENCE_CREATE_INFO: i32 = 8;
const S_TYPE_SEMAPHORE_CREATE_INFO: i32 = 9;
const S_TYPE_BUFFER_CREATE_INFO: i32 = 12;
const S_TYPE_IMAGE_CREATE_INFO: i32 = 14;
const S_TYPE_IMAGE_VIEW_CREATE_INFO: i32 = 15;
const S_TYPE_SHADER_MODULE_CREATE_INFO: i32 = 16;
const S_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO: i32 = 18;
const S_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO: i32 = 19;
const S_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO: i32 = 20;
const S_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO: i32 = 22;
const S_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO: i32 = 23;
const S_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO: i32 = 24;
const S_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO: i32 = 26;
const S_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO: i32 = 27;
const S_TYPE_GRAPHICS_PIPELINE_CREATE_INFO: i32 = 28;
const S_TYPE_PIPELINE_LAYOUT_CREATE_INFO: i32 = 30;
const S_TYPE_SAMPLER_CREATE_INFO: i32 = 31;
const S_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO: i32 = 32;
const S_TYPE_DESCRIPTOR_POOL_CREATE_INFO: i32 = 33;
const S_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO: i32 = 34;
const S_TYPE_WRITE_DESCRIPTOR_SET: i32 = 35;
const S_TYPE_FRAMEBUFFER_CREATE_INFO: i32 = 37;
const S_TYPE_RENDER_PASS_CREATE_INFO: i32 = 38;
const S_TYPE_COMMAND_POOL_CREATE_INFO: i32 = 39;
const S_TYPE_COMMAND_BUFFER_ALLOCATE_INFO: i32 = 40;
const S_TYPE_COMMAND_BUFFER_BEGIN_INFO: i32 = 42;
const S_TYPE_IMAGE_MEMORY_BARRIER: i32 = 45;

const FORMAT_R8_UNORM: i32 = 9;

unsafe fn is_srgb_format(fmt: i32,) -> bool {
    matches!(fmt, 43 | 50 | 64 | 65 | 74 | 75 | 84 | 85 | 94 | 95 | 104 | 107 | 120 | 121)
}

const SHADER_STAGE_VERTEX: u32 = 0x1;
const SHADER_STAGE_FRAGMENT: u32 = 0x10;
const TOPOLOGY_TRIANGLE_LIST: i32 = 3;
const VERTEX_INPUT_RATE_VERTEX: i32 = 0;
const POLYGON_MODE_FILL: i32 = 0;
const CULL_MODE_NONE: i32 = 0;
const FRONT_FACE_CCW: i32 = 0;
const SAMPLE_COUNT_1: u32 = 1;
const BLEND_FACTOR_ONE: i32 = 1;
const BLEND_FACTOR_ONE_MINUS_SRC_ALPHA: i32 = 7;
const BLEND_OP_ADD: i32 = 0;
const COLOR_COMPONENT_ALL: u32 = 0xF;
const DYNAMIC_STATE_SCISSOR: i32 = 1;
const PIPELINE_BIND_POINT_GRAPHICS: i32 = 0;
const ATTACHMENT_LOAD_OP_LOAD: i32 = 0;
const ATTACHMENT_STORE_OP_STORE: i32 = 0;
const ATTACHMENT_LOAD_OP_DONT_CARE: i32 = 2;
const ATTACHMENT_STORE_OP_DONT_CARE: i32 = 1;
const IMAGE_LAYOUT_UNDEFINED: i32 = 0;
const IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: i32 = 2;
const IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL: i32 = 5;
const IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: i32 = 7;
const IMAGE_LAYOUT_PRESENT_SRC: i32 = 1_000_001_002;
const ACCESS_SHADER_READ: u32 = 0x20;

const ACCESS_COLOR_ATTACHMENT_WRITE: u32 = 0x100;
const ACCESS_TRANSFER_WRITE: u32 = 0x1000;
const STAGE_COLOR_ATTACHMENT_OUTPUT: u32 = 0x400;
const STAGE_TRANSFER: u32 = 0x1000;
const STAGE_BOTTOM_OF_PIPE: u32 = 0x2000;
const DESC_TYPE_SAMPLER: i32 = 0;
const DESC_TYPE_SAMPLED_IMAGE: i32 = 2;
const DESC_TYPE_UNIFORM_BUFFER: i32 = 6;
const IMAGE_VIEW_TYPE_2D: i32 = 1;
const IMAGE_TYPE_2D: i32 = 1;
const IMAGE_TILING_OPTIMAL: i32 = 0;
const IMAGE_USAGE_TRANSFER_DST: u32 = 0x2;
const IMAGE_USAGE_SAMPLED: u32 = 0x4;
const SHARING_MODE_EXCLUSIVE: i32 = 0;
const IMAGE_ASPECT_COLOR: u32 = 1;
const BUF_USAGE_TRANSFER_SRC: u32 = 0x1;
const BUF_USAGE_UNIFORM: u32 = 0x10;
const BUF_USAGE_INDEX: u32 = 0x40;
const BUF_USAGE_VERTEX: u32 = 0x80;
const MEM_DEVICE_LOCAL: u32 = 0x1;
const MEM_HOST_VISIBLE: u32 = 0x2;
const MEM_HOST_COHERENT: u32 = 0x4;
const MIPMAP_MODE_NEAREST: i32 = 0;
const FILTER_LINEAR: i32 = 1;
const ADDRESS_MODE_CLAMP: i32 = 2;
const BORDER_FLOAT_TRANSPARENT_BLACK: i32 = 0;
const SUBPASS_CONTENTS_INLINE: u32 = 0;
const INDEX_TYPE_UINT32: i32 = 1;

const FRAMES: usize = 3;

#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct Extent2D {
    width:  u32,
    height: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct Extent3D {
    width:  u32,
    height: u32,
    depth:  u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct Offset2D {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct Offset3D {
    x: i32,
    y: i32,
    z: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct Rect2D {
    offset: Offset2D,
    extent: Extent2D,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct Viewport {
    x:         f32,
    y:         f32,
    width:     f32,
    height:    f32,
    min_depth: f32,
    max_depth: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct SubresourceRange {
    aspect_mask:      u32,
    base_mip_level:   u32,
    level_count:      u32,
    base_array_layer: u32,
    layer_count:      u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct SubresourceLayers {
    aspect_mask:      u32,
    mip_level:        u32,
    base_array_layer: u32,
    layer_count:      u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct ComponentMapping {
    r: i32,
    g: i32,
    b: i32,
    a: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct BufferImageCopy {
    buffer_offset:       u64,
    buffer_row_length:   u32,
    buffer_image_height: u32,
    image_subresource:   SubresourceLayers,
    image_offset:        Offset3D,
    image_extent:        Extent3D,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct ImageMemoryBarrier {
    s_type:            i32,
    p_next:            *const c_void,
    src_access_mask:   u32,
    dst_access_mask:   u32,
    old_layout:        i32,
    new_layout:        i32,
    src_queue_family:  u32,
    dst_queue_family:  u32,
    image:             VkHandle,
    subresource_range: SubresourceRange,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct MemoryType {
    property_flags: u32,
    heap_index:     u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct MemoryHeap {
    size:  u64,
    flags: u32,
    _pad:  u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct MemoryProperties {
    memory_type_count: u32,
    memory_types:      [MemoryType; 32],
    memory_heap_count: u32,
    memory_heaps:      [MemoryHeap; 16],
}
#[repr(C)]
#[derive(Clone, Copy, Default,)]
struct MemoryRequirements {
    size:             u64,
    alignment:        u64,
    memory_type_bits: u32,
}

type PfnGdpa = unsafe extern "system" fn(VkHandle, *const u8,) -> *const c_void;

#[derive(Clone, Copy,)]
struct Fns {
    create_shader_module: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    destroy_shader_module: unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    create_pipeline_layout: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    create_graphics_pipelines:
        unsafe extern "system" fn(VkHandle, VkHandle, u32, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    destroy_pipeline: unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    create_render_pass: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    destroy_render_pass: unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    create_framebuffer: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    destroy_framebuffer: unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    create_image_view: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    destroy_image_view: unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    create_sampler: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    create_buffer: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    get_buffer_memory_requirements: unsafe extern "system" fn(VkHandle, VkHandle, *mut MemoryRequirements,),
    create_image: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    get_image_memory_requirements: unsafe extern "system" fn(VkHandle, VkHandle, *mut MemoryRequirements,),
    allocate_memory: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    bind_buffer_memory: unsafe extern "system" fn(VkHandle, VkHandle, VkHandle, u64,) -> i32,
    bind_image_memory: unsafe extern "system" fn(VkHandle, VkHandle, VkHandle, u64,) -> i32,
    map_memory: unsafe extern "system" fn(VkHandle, VkHandle, u64, u64, u32, *mut *mut c_void,) -> i32,
    get_physical_device_memory_properties: unsafe extern "system" fn(VkHandle, *mut MemoryProperties,),
    create_descriptor_set_layout:
        unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    create_descriptor_pool: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    allocate_descriptor_sets: unsafe extern "system" fn(VkHandle, *const c_void, *mut VkHandle,) -> i32,
    free_descriptor_sets: unsafe extern "system" fn(VkHandle, VkHandle, u32, *const VkHandle,),
    update_descriptor_sets: unsafe extern "system" fn(VkHandle, u32, *const c_void, u32, *const c_void,),
    create_command_pool: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    allocate_command_buffers: unsafe extern "system" fn(VkHandle, *const c_void, *mut VkHandle,) -> i32,
    begin_command_buffer: unsafe extern "system" fn(VkHandle, *const c_void,) -> i32,
    end_command_buffer: unsafe extern "system" fn(VkHandle,) -> i32,
    reset_command_buffer: unsafe extern "system" fn(VkHandle, u32,) -> i32,
    cmd_pipeline_barrier:
        unsafe extern "system" fn(VkHandle, u32, u32, u32, u32, *const c_void, u32, *const c_void, u32, *const c_void,),
    cmd_copy_buffer_to_image: unsafe extern "system" fn(VkHandle, VkHandle, VkHandle, i32, u32, *const BufferImageCopy,),
    cmd_begin_render_pass: unsafe extern "system" fn(VkHandle, *const c_void, u32,),
    cmd_end_render_pass: unsafe extern "system" fn(VkHandle,),
    cmd_bind_pipeline: unsafe extern "system" fn(VkHandle, i32, VkHandle,),
    cmd_bind_vertex_buffers: unsafe extern "system" fn(VkHandle, u32, u32, *const VkHandle, *const u64,),
    cmd_bind_index_buffer: unsafe extern "system" fn(VkHandle, VkHandle, u64, i32,),
    cmd_bind_descriptor_sets:
        unsafe extern "system" fn(VkHandle, i32, VkHandle, u32, u32, *const VkHandle, u32, *const u32,),

    cmd_set_scissor:  unsafe extern "system" fn(VkHandle, u32, u32, *const Rect2D,),
    cmd_draw_indexed: unsafe extern "system" fn(VkHandle, u32, u32, u32, i32, u32,),
    queue_submit:     unsafe extern "system" fn(VkHandle, u32, *const c_void, VkHandle,) -> i32,
    create_fence:     unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,
    wait_for_fences:  unsafe extern "system" fn(VkHandle, u32, *const VkHandle, u32, u64,) -> i32,
    reset_fences:     unsafe extern "system" fn(VkHandle, u32, *const VkHandle,) -> i32,
    create_semaphore: unsafe extern "system" fn(VkHandle, *const c_void, *const c_void, *mut VkHandle,) -> i32,

    destroy_buffer:                unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_image:                 unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    free_memory:                   unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_fence:                 unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_semaphore:             unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_command_pool:          unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_descriptor_pool:       unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_descriptor_set_layout: unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_pipeline_layout:       unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    destroy_sampler:               unsafe extern "system" fn(VkHandle, VkHandle, *const c_void,),
    get_swapchain_images:          unsafe extern "system" fn(VkHandle, VkHandle, *mut u32, *mut VkHandle,) -> i32,
    device_wait_idle:              unsafe extern "system" fn(VkHandle,) -> i32,
}

unsafe fn resolve<T: Copy,>(gdpa: PfnGdpa, device: VkHandle, name: &str,) -> Option<T,> {
    let c = std::ffi::CString::new(name,).ok()?;
    let p = gdpa(device, c.as_ptr() as *const u8,);
    if p.is_null()
    {
        log_error!("vk", format!("device function {name} resolve failed (device={device:#x})"));
        None
    }
    else
    {
        Some(std::mem::transmute_copy(&p,),)
    }
}

struct GpuBuffer {
    buffer: VkHandle,

    _memory: VkHandle,
    size:    u64,
    mapped:  *mut u8,
}

unsafe impl Send for GpuBuffer {}

struct TexEntry {
    image: VkHandle,

    memory: VkHandle,
    view:   VkHandle,
    w:      u32,
    h:      u32,
}

#[derive(Clone, Copy,)]
enum DeferKind {
    Framebuffer,
    View,
    RenderPass,
    Pipeline,
}

#[derive(Default,)]
struct Retired {
    tex: Vec<(u64, TexEntry,),>,
    buf: Vec<(u64, GpuBuffer,),>,
    vk:  Vec<(u64, VkHandle, DeferKind,),>,

    sets: Vec<(u64, VkHandle,),>,
}

impl TexEntry {
    unsafe fn destroy(&self, f: &Fns, device: VkHandle,) {
        if self.view != 0
        {
            (f.destroy_image_view)(device, self.view, std::ptr::null(),);
        }
        if self.image != 0
        {
            (f.destroy_image)(device, self.image, std::ptr::null(),);
        }
        if self.memory != 0
        {
            (f.free_memory)(device, self.memory, std::ptr::null(),);
        }
    }
}

struct FrameRes {
    cmd:       VkHandle,
    fence:     VkHandle,
    ubo:       GpuBuffer,
    vbo:       GpuBuffer,
    ibo:       GpuBuffer,
    ubo_set:   VkHandle,
    in_flight: bool,
}

pub struct PainterVK {
    f:            Fns,
    device:       VkHandle,
    physical:     VkHandle,
    queue_family: u32,
    swapchain:    VkHandle,
    format:       i32,
    width:        u32,
    height:       u32,
    render_pass:  VkHandle,
    views:        Vec<VkHandle,>,

    img_ready: Vec<bool,>,

    retired:         Retired,
    frame_no:        u64,
    framebuffers:    Vec<VkHandle,>,
    pipeline:        VkHandle,
    pipeline_layout: VkHandle,
    set_layout_ubo:  VkHandle,
    set_layout_tex:  VkHandle,
    pool:            VkHandle,
    cmd_pool:        VkHandle,
    frames:          Vec<FrameRes,>,
    frame_idx:       usize,
    tex:             HashMap<u64, TexEntry,>,
    tex_sets:        HashMap<u64, VkHandle,>,
    sampler:         VkHandle,

    pending: Vec<Upload,>,

    semaphores: Vec<VkHandle,>,
    staging:    Option<GpuBuffer,>,
    screen:     (u32, u32,),
    warned:     bool,
}

unsafe impl Send for PainterVK {}

const RETIRE_AGE: u64 = 8;

impl PainterVK {
    unsafe fn drain_retired_aged(&mut self,) {
        let now = self.frame_no;
        let aged = |f: u64| now.saturating_sub(f,) >= RETIRE_AGE;

        let tex: Vec<TexEntry,> = {
            let mut keep = Vec::new();
            let mut kill = Vec::new();
            for (f, t,) in std::mem::take(&mut self.retired.tex,)
            {
                if aged(f,) { kill.push(t,) } else { keep.push((f, t,),) }
            }
            self.retired.tex = keep;
            kill
        };
        for t in tex
        {
            t.destroy(&self.f, self.device,);
        }

        let bufs: Vec<GpuBuffer,> = {
            let mut keep = Vec::new();
            let mut kill = Vec::new();
            for (f, b,) in std::mem::take(&mut self.retired.buf,)
            {
                if aged(f,) { kill.push(b,) } else { keep.push((f, b,),) }
            }
            self.retired.buf = keep;
            kill
        };
        for b in bufs
        {
            if b.buffer != 0
            {
                (self.f.destroy_buffer)(self.device, b.buffer, std::ptr::null(),);
            }
            if b._memory != 0
            {
                (self.f.free_memory)(self.device, b._memory, std::ptr::null(),);
            }
        }

        let objs: Vec<(VkHandle, DeferKind,),> = {
            let mut keep = Vec::new();
            let mut kill = Vec::new();
            for (f, h, k,) in std::mem::take(&mut self.retired.vk,)
            {
                if aged(f,) { kill.push((h, k,),) } else { keep.push((f, h, k,),) }
            }
            self.retired.vk = keep;
            kill
        };
        for (h, k,) in objs
        {
            match k
            {
                DeferKind::Framebuffer => (self.f.destroy_framebuffer)(self.device, h, std::ptr::null(),),
                DeferKind::View => (self.f.destroy_image_view)(self.device, h, std::ptr::null(),),
                DeferKind::RenderPass => (self.f.destroy_render_pass)(self.device, h, std::ptr::null(),),
                DeferKind::Pipeline => (self.f.destroy_pipeline)(self.device, h, std::ptr::null(),),
            }
        }

        let sets: Vec<VkHandle,> = {
            let mut keep = Vec::new();
            let mut kill = Vec::new();
            for (f, s,) in std::mem::take(&mut self.retired.sets,)
            {
                if aged(f,) { kill.push(s,) } else { keep.push((f, s,),) }
            }
            self.retired.sets = keep;
            kill
        };
        for s in sets
        {
            if self.pool != 0 && s != 0
            {
                (self.f.free_descriptor_sets)(self.device, self.pool, 1, &s,);
            }
        }
    }
}

impl Drop for PainterVK {
    fn drop(&mut self,) {
        unsafe {
            if self.device == 0
            {
                return;
            }

            (self.f.device_wait_idle)(self.device,);

            for (_, t,) in std::mem::take(&mut self.retired.tex,)
            {
                t.destroy(&self.f, self.device,);
            }
            for (_, b,) in std::mem::take(&mut self.retired.buf,)
            {
                if b.buffer != 0
                {
                    (self.f.destroy_buffer)(self.device, b.buffer, std::ptr::null(),);
                }
                if b._memory != 0
                {
                    (self.f.free_memory)(self.device, b._memory, std::ptr::null(),);
                }
            }
            for (_, h, kind,) in std::mem::take(&mut self.retired.vk,)
            {
                match kind
                {
                    DeferKind::Framebuffer => (self.f.destroy_framebuffer)(self.device, h, std::ptr::null(),),
                    DeferKind::View => (self.f.destroy_image_view)(self.device, h, std::ptr::null(),),
                    DeferKind::RenderPass => (self.f.destroy_render_pass)(self.device, h, std::ptr::null(),),
                    DeferKind::Pipeline => (self.f.destroy_pipeline)(self.device, h, std::ptr::null(),),
                }
            }

            for fb in self.framebuffers.drain(..,)
            {
                (self.f.destroy_framebuffer)(self.device, fb, std::ptr::null(),);
            }
            for v in self.views.drain(..,)
            {
                (self.f.destroy_image_view)(self.device, v, std::ptr::null(),);
            }
            if self.render_pass != 0
            {
                (self.f.destroy_render_pass)(self.device, self.render_pass, std::ptr::null(),);
            }
            if self.pipeline != 0
            {
                (self.f.destroy_pipeline)(self.device, self.pipeline, std::ptr::null(),);
            }
            if self.pipeline_layout != 0
            {
                (self.f.destroy_pipeline_layout)(self.device, self.pipeline_layout, std::ptr::null(),);
            }
            for l in [self.set_layout_ubo, self.set_layout_tex,]
            {
                if l != 0
                {
                    (self.f.destroy_descriptor_set_layout)(self.device, l, std::ptr::null(),);
                }
            }
            if self.sampler != 0
            {
                (self.f.destroy_sampler)(self.device, self.sampler, std::ptr::null(),);
            }
            if self.pool != 0
            {
                (self.f.destroy_descriptor_pool)(self.device, self.pool, std::ptr::null(),);
            }
            if self.cmd_pool != 0
            {
                (self.f.destroy_command_pool)(self.device, self.cmd_pool, std::ptr::null(),);
            }
            for se in self.semaphores.drain(..,)
            {
                (self.f.destroy_semaphore)(self.device, se, std::ptr::null(),);
            }

            let free_buf = |b: &GpuBuffer| {
                if b.buffer != 0
                {
                    (self.f.destroy_buffer)(self.device, b.buffer, std::ptr::null(),);
                }
                if b._memory != 0
                {
                    (self.f.free_memory)(self.device, b._memory, std::ptr::null(),);
                }
            };
            for f in self.frames.drain(..,)
            {
                if f.fence != 0
                {
                    (self.f.destroy_fence)(self.device, f.fence, std::ptr::null(),);
                }
                free_buf(&f.ubo,);
                free_buf(&f.vbo,);
                free_buf(&f.ibo,);
            }
            if let Some(s,) = self.staging.take()
            {
                free_buf(&s,);
            }
            for (_, t,) in self.tex.drain()
            {
                if t.view != 0
                {
                    (self.f.destroy_image_view)(self.device, t.view, std::ptr::null(),);
                }
                if t.image != 0
                {
                    (self.f.destroy_image)(self.device, t.image, std::ptr::null(),);
                }
                if t.memory != 0
                {
                    (self.f.free_memory)(self.device, t.memory, std::ptr::null(),);
                }
            }
        }
    }
}

struct Upload {
    key:   u64,
    x:     u32,
    y:     u32,
    w:     u32,
    h:     u32,
    bytes: Vec<u8,>,

    fresh: bool,
}

impl PainterVK {
    pub unsafe fn new(sc: VkHandle,) -> Option<Self,> {
        let dev: DeviceInfo = device_info()?;
        let info: SwapchainInfo = swapchain_info(sc,)?;
        let module = crate::win::module_base_bytes(b"vulkan-1.dll\0",);
        if module == 0
        {
            return None;
        }
        let raw = crate::win::proc_address(module as isize, b"vkGetDeviceProcAddr\0",);
        if raw.is_null()
        {
            return None;
        }
        let gdpa: PfnGdpa = std::mem::transmute(raw,);

        let inst = crate::hook::vk::instance()?;
        let gipa_raw = crate::win::proc_address(module as isize, b"vkGetInstanceProcAddr\0",);
        if gipa_raw.is_null()
        {
            return None;
        }
        let gipa: PfnGdpa = std::mem::transmute(gipa_raw,);
        let get_mem_props: unsafe extern "system" fn(VkHandle, *mut MemoryProperties,) =
            resolve(gipa, inst, "vkGetPhysicalDeviceMemoryProperties",)?;
        let d = dev.device;
        let f = Fns {
            create_shader_module: resolve(gdpa, d, "vkCreateShaderModule",)?,
            destroy_shader_module: resolve(gdpa, d, "vkDestroyShaderModule",)?,
            create_pipeline_layout: resolve(gdpa, d, "vkCreatePipelineLayout",)?,
            create_graphics_pipelines: resolve(gdpa, d, "vkCreateGraphicsPipelines",)?,
            destroy_pipeline: resolve(gdpa, d, "vkDestroyPipeline",)?,
            create_render_pass: resolve(gdpa, d, "vkCreateRenderPass",)?,
            destroy_render_pass: resolve(gdpa, d, "vkDestroyRenderPass",)?,
            create_framebuffer: resolve(gdpa, d, "vkCreateFramebuffer",)?,
            destroy_framebuffer: resolve(gdpa, d, "vkDestroyFramebuffer",)?,
            create_image_view: resolve(gdpa, d, "vkCreateImageView",)?,
            destroy_image_view: resolve(gdpa, d, "vkDestroyImageView",)?,
            create_sampler: resolve(gdpa, d, "vkCreateSampler",)?,
            create_buffer: resolve(gdpa, d, "vkCreateBuffer",)?,
            get_buffer_memory_requirements: resolve(gdpa, d, "vkGetBufferMemoryRequirements",)?,
            create_image: resolve(gdpa, d, "vkCreateImage",)?,
            get_image_memory_requirements: resolve(gdpa, d, "vkGetImageMemoryRequirements",)?,
            allocate_memory: resolve(gdpa, d, "vkAllocateMemory",)?,
            bind_buffer_memory: resolve(gdpa, d, "vkBindBufferMemory",)?,
            bind_image_memory: resolve(gdpa, d, "vkBindImageMemory",)?,
            map_memory: resolve(gdpa, d, "vkMapMemory",)?,
            get_physical_device_memory_properties: get_mem_props,
            create_descriptor_set_layout: resolve(gdpa, d, "vkCreateDescriptorSetLayout",)?,
            create_descriptor_pool: resolve(gdpa, d, "vkCreateDescriptorPool",)?,
            allocate_descriptor_sets: resolve(gdpa, d, "vkAllocateDescriptorSets",)?,
            free_descriptor_sets: resolve(gdpa, d, "vkFreeDescriptorSets",)?,
            update_descriptor_sets: resolve(gdpa, d, "vkUpdateDescriptorSets",)?,
            create_command_pool: resolve(gdpa, d, "vkCreateCommandPool",)?,
            allocate_command_buffers: resolve(gdpa, d, "vkAllocateCommandBuffers",)?,
            begin_command_buffer: resolve(gdpa, d, "vkBeginCommandBuffer",)?,
            end_command_buffer: resolve(gdpa, d, "vkEndCommandBuffer",)?,
            reset_command_buffer: resolve(gdpa, d, "vkResetCommandBuffer",)?,
            cmd_pipeline_barrier: resolve(gdpa, d, "vkCmdPipelineBarrier",)?,
            cmd_copy_buffer_to_image: resolve(gdpa, d, "vkCmdCopyBufferToImage",)?,
            cmd_begin_render_pass: resolve(gdpa, d, "vkCmdBeginRenderPass",)?,
            cmd_end_render_pass: resolve(gdpa, d, "vkCmdEndRenderPass",)?,
            cmd_bind_pipeline: resolve(gdpa, d, "vkCmdBindPipeline",)?,
            cmd_bind_vertex_buffers: resolve(gdpa, d, "vkCmdBindVertexBuffers",)?,
            cmd_bind_index_buffer: resolve(gdpa, d, "vkCmdBindIndexBuffer",)?,
            cmd_bind_descriptor_sets: resolve(gdpa, d, "vkCmdBindDescriptorSets",)?,
            cmd_set_scissor: resolve(gdpa, d, "vkCmdSetScissor",)?,
            cmd_draw_indexed: resolve(gdpa, d, "vkCmdDrawIndexed",)?,
            queue_submit: resolve(gdpa, d, "vkQueueSubmit",)?,
            create_fence: resolve(gdpa, d, "vkCreateFence",)?,
            wait_for_fences: resolve(gdpa, d, "vkWaitForFences",)?,
            reset_fences: resolve(gdpa, d, "vkResetFences",)?,
            create_semaphore: resolve(gdpa, d, "vkCreateSemaphore",)?,
            destroy_buffer: resolve(gdpa, d, "vkDestroyBuffer",)?,
            destroy_image: resolve(gdpa, d, "vkDestroyImage",)?,
            free_memory: resolve(gdpa, d, "vkFreeMemory",)?,
            destroy_fence: resolve(gdpa, d, "vkDestroyFence",)?,
            destroy_semaphore: resolve(gdpa, d, "vkDestroySemaphore",)?,
            destroy_command_pool: resolve(gdpa, d, "vkDestroyCommandPool",)?,
            destroy_descriptor_pool: resolve(gdpa, d, "vkDestroyDescriptorPool",)?,
            destroy_descriptor_set_layout: resolve(gdpa, d, "vkDestroyDescriptorSetLayout",)?,
            destroy_pipeline_layout: resolve(gdpa, d, "vkDestroyPipelineLayout",)?,
            destroy_sampler: resolve(gdpa, d, "vkDestroySampler",)?,
            get_swapchain_images: resolve(gdpa, d, "vkGetSwapchainImagesKHR",)?,
            device_wait_idle: resolve(gdpa, d, "vkDeviceWaitIdle",)?,
        };

        let mut p = PainterVK {
            f,
            device: d,
            physical: dev.physical,
            queue_family: dev.queue_family,
            swapchain: 0,
            format: info.format,
            width: 0,
            height: 0,
            render_pass: 0,
            views: Vec::new(),
            img_ready: Vec::new(),
            retired: Retired::default(),
            frame_no: 0,
            framebuffers: Vec::new(),
            pipeline: 0,
            pipeline_layout: 0,
            set_layout_ubo: 0,
            set_layout_tex: 0,
            pool: 0,
            cmd_pool: 0,
            frames: Vec::new(),
            frame_idx: 0,
            tex: HashMap::new(),
            tex_sets: HashMap::new(),
            sampler: 0,
            pending: Vec::new(),
            semaphores: Vec::new(),
            staging: None,
            screen: (0, 0,),
            warned: false,
        };
        if !p.create_sampler()
        {
            log_error!("vk", "sampler creation failed");
            return None;
        }
        if !p.create_descriptor_layouts() || !p.create_pipeline_layout()
        {
            return None;
        }
        if !p.create_pool()
        {
            return None;
        }
        if !p.ensure_swapchain(sc, info,)
        {
            return None;
        }
        if !p.create_pipeline()
        {
            return None;
        }
        if !p.create_frames()
        {
            return None;
        }
        if !p.make_white()
        {
            return None;
        }
        log_info!("vk", format!("painter ready: {}x{} fmt={} images={}", p.width, p.height, p.format, p.views.len()));
        Some(p,)
    }

    pub unsafe fn tex_count(&self,) -> usize {
        self.tex.len()
    }

    unsafe fn mem_type(&self, bits: u32, want: u32,) -> Option<u32,> {
        let mut mp = MemoryProperties::default();
        (self.f.get_physical_device_memory_properties)(self.physical, &mut mp,);
        for i in 0..mp.memory_type_count.min(32,)
        {
            let t = mp.memory_types[i as usize];
            if bits & (1 << i) != 0 && (t.property_flags & want) == want
            {
                return Some(i,);
            }
        }
        None
    }

    unsafe fn make_buffer(&mut self, size: u64, usage: u32, host: bool,) -> Option<GpuBuffer,> {
        let mut buffer: VkHandle = 0;
        #[repr(C)]
        struct BufferCreateInfo {
            s_type:                 i32,
            p_next:                 *const c_void,
            flags:                  u32,
            size:                   u64,
            usage:                  u32,
            sharing_mode:           i32,
            queue_family_count:     u32,
            p_queue_family_indices: *const u32,
        }
        let bci = BufferCreateInfo {
            s_type: S_TYPE_BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            size,
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let r = (self.f.create_buffer)(self.device, &bci as *const _ as *const c_void, std::ptr::null(), &mut buffer,);
        if r != 0
        {
            log_error!("vk", format!("vkCreateBuffer({size}) failed result={r}"));
            return None;
        }
        let mut req = MemoryRequirements::default();
        (self.f.get_buffer_memory_requirements)(self.device, buffer, &mut req,);
        let want = if host { MEM_HOST_VISIBLE | MEM_HOST_COHERENT } else { MEM_DEVICE_LOCAL };
        let ty = self.mem_type(req.memory_type_bits, want,)?;
        #[repr(C)]
        struct AllocInfo {
            s_type:            i32,
            p_next:            *const c_void,
            size:              u64,
            memory_type_index: u32,
        }
        let ai = AllocInfo {
            s_type:            S_TYPE_MEMORY_ALLOCATE_INFO,
            p_next:            std::ptr::null(),
            size:              req.size,
            memory_type_index: ty,
        };
        let mut memory: VkHandle = 0;
        let r = (self.f.allocate_memory)(self.device, &ai as *const _ as *const c_void, std::ptr::null(), &mut memory,);
        if r != 0
        {
            log_error!("vk", format!("vkAllocateMemory failed result={r}"));
            return None;
        }
        if (self.f.bind_buffer_memory)(self.device, buffer, memory, 0,) != 0
        {
            log_error!("vk", "vkBindBufferMemory failed");
            return None;
        }
        let mut mapped: *mut c_void = std::ptr::null_mut();
        if host
        {
            let r = (self.f.map_memory)(self.device, memory, 0, req.size, 0, &mut mapped,);
            if r != 0
            {
                log_error!("vk", format!("vkMapMemory failed result={r}"));
                return None;
            }
        }
        Some(GpuBuffer { buffer, _memory: memory, size: req.size, mapped: mapped as *mut u8, },)
    }

    unsafe fn write_buffer(&self, b: &GpuBuffer, data: &[u8],) -> bool {
        if b.mapped.is_null() || data.len() as u64 > b.size
        {
            if data.len() as u64 > b.size
            {
                log_warn!("vk", "write_buffer: data larger than buffer, frame skipped");
            }
            return false;
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), b.mapped, data.len(),);
        true
    }

    unsafe fn make_image(&mut self, w: u32, h: u32, format: i32, usage: u32,) -> Option<(VkHandle, VkHandle,),> {
        #[repr(C)]
        struct ImageCreateInfo {
            s_type:                 i32,
            p_next:                 *const c_void,
            flags:                  u32,
            image_type:             i32,
            format:                 i32,
            extent:                 Extent3D,
            mip_levels:             u32,
            array_layers:           u32,
            samples:                u32,
            tiling:                 i32,
            usage:                  u32,
            sharing_mode:           i32,
            queue_family_count:     u32,
            p_queue_family_indices: *const u32,
            initial_layout:         i32,
        }
        let ici = ImageCreateInfo {
            s_type: S_TYPE_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format,
            extent: Extent3D { width: w.max(1,), height: h.max(1,), depth: 1, },
            mip_levels: 1,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        let mut image: VkHandle = 0;
        let r = (self.f.create_image)(self.device, &ici as *const _ as *const c_void, std::ptr::null(), &mut image,);
        if r != 0
        {
            log_error!("vk", format!("vkCreateImage({w}x{h}) failed result={r}"));
            return None;
        }
        let mut req = MemoryRequirements::default();
        (self.f.get_image_memory_requirements)(self.device, image, &mut req,);
        let ty = self.mem_type(req.memory_type_bits, MEM_DEVICE_LOCAL,)?;
        #[repr(C)]
        struct AllocInfo {
            s_type:            i32,
            p_next:            *const c_void,
            size:              u64,
            memory_type_index: u32,
        }
        let ai = AllocInfo {
            s_type:            S_TYPE_MEMORY_ALLOCATE_INFO,
            p_next:            std::ptr::null(),
            size:              req.size,
            memory_type_index: ty,
        };
        let mut memory: VkHandle = 0;
        let r = (self.f.allocate_memory)(self.device, &ai as *const _ as *const c_void, std::ptr::null(), &mut memory,);
        if r != 0
        {
            log_error!("vk", format!("vkAllocateMemory(image) failed result={r}"));
            return None;
        }
        if (self.f.bind_image_memory)(self.device, image, memory, 0,) != 0
        {
            log_error!("vk", "vkBindImageMemory failed");
            return None;
        }
        Some((image, memory,),)
    }

    unsafe fn make_view(&self, image: VkHandle, format: i32,) -> Option<VkHandle,> {
        #[repr(C)]
        struct ImageViewCreateInfo {
            s_type:            i32,
            p_next:            *const c_void,
            flags:             u32,
            image:             VkHandle,
            view_type:         i32,
            format:            i32,
            components:        ComponentMapping,
            subresource_range: SubresourceRange,
        }
        let vci = ImageViewCreateInfo {
            s_type: S_TYPE_IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image,
            view_type: IMAGE_VIEW_TYPE_2D,
            format,
            components: ComponentMapping { r: 0, g: 0, b: 0, a: 0, },
            subresource_range: SubresourceRange {
                aspect_mask:      IMAGE_ASPECT_COLOR,
                base_mip_level:   0,
                level_count:      1,
                base_array_layer: 0,
                layer_count:      1,
            },
        };
        let mut view: VkHandle = 0;
        let r =
            (self.f.create_image_view)(self.device, &vci as *const _ as *const c_void, std::ptr::null(), &mut view,);
        if r != 0
        {
            log_error!("vk", format!("vkCreateImageView failed result={r}"));
            return None;
        }
        Some(view,)
    }

    unsafe fn create_sampler(&mut self,) -> bool {
        #[repr(C)]
        struct SamplerCreateInfo {
            s_type:            i32,
            p_next:            *const c_void,
            flags:             u32,
            mag_filter:        i32,
            min_filter:        i32,
            mipmap_mode:       i32,
            address_u:         i32,
            address_v:         i32,
            address_w:         i32,
            mip_lod_bias:      f32,
            anisotropy_enable: u32,
            max_anisotropy:    f32,
            compare_enable:    u32,
            compare_op:        i32,
            min_lod:           f32,
            max_lod:           f32,
            border_color:      i32,
            unnormalized:      u32,
        }
        let s = SamplerCreateInfo {
            s_type:            S_TYPE_SAMPLER_CREATE_INFO,
            p_next:            std::ptr::null(),
            flags:             0,
            mag_filter:        FILTER_LINEAR,
            min_filter:        FILTER_LINEAR,
            mipmap_mode:       MIPMAP_MODE_NEAREST,
            address_u:         ADDRESS_MODE_CLAMP,
            address_v:         ADDRESS_MODE_CLAMP,
            address_w:         ADDRESS_MODE_CLAMP,
            mip_lod_bias:      0.0,
            anisotropy_enable: 0,
            max_anisotropy:    1.0,
            compare_enable:    0,
            compare_op:        0,
            min_lod:           0.0,
            max_lod:           1.0,
            border_color:      BORDER_FLOAT_TRANSPARENT_BLACK,
            unnormalized:      0,
        };
        let mut sampler: VkHandle = 0;
        let r = (self.f.create_sampler)(self.device, &s as *const _ as *const c_void, std::ptr::null(), &mut sampler,);
        if r != 0
        {
            return false;
        }
        self.sampler = sampler;
        true
    }

    unsafe fn create_descriptor_layouts(&mut self,) -> bool {
        #[repr(C)]
        struct SetLayoutBinding {
            binding:              u32,
            descriptor_type:      i32,
            descriptor_count:     u32,
            stage_flags:          u32,
            p_immutable_samplers: *const VkHandle,
        }
        #[repr(C)]
        struct SetLayoutCreateInfo {
            s_type:        i32,
            p_next:        *const c_void,
            flags:         u32,
            binding_count: u32,
            p_bindings:    *const SetLayoutBinding,
        }
        let ubo = [SetLayoutBinding {
            binding:              0,
            descriptor_type:      DESC_TYPE_UNIFORM_BUFFER,
            descriptor_count:     1,
            stage_flags:          SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
            p_immutable_samplers: std::ptr::null(),
        },];
        let ci = SetLayoutCreateInfo {
            s_type:        S_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next:        std::ptr::null(),
            flags:         0,
            binding_count: 1,
            p_bindings:    ubo.as_ptr(),
        };
        let mut layout: VkHandle = 0;
        if (self.f.create_descriptor_set_layout)(
            self.device,
            &ci as *const _ as *const c_void,
            std::ptr::null(),
            &mut layout,
        ) != 0
        {
            return false;
        }
        self.set_layout_ubo = layout;

        let tex = [
            SetLayoutBinding {
                binding:              0,
                descriptor_type:      DESC_TYPE_SAMPLED_IMAGE,
                descriptor_count:     1,
                stage_flags:          SHADER_STAGE_FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
            },
            SetLayoutBinding {
                binding:              1,
                descriptor_type:      DESC_TYPE_SAMPLER,
                descriptor_count:     1,
                stage_flags:          SHADER_STAGE_FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
            },
        ];
        let ci2 = SetLayoutCreateInfo {
            s_type:        S_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next:        std::ptr::null(),
            flags:         0,
            binding_count: 2,
            p_bindings:    tex.as_ptr(),
        };
        let mut layout2: VkHandle = 0;
        if (self.f.create_descriptor_set_layout)(
            self.device,
            &ci2 as *const _ as *const c_void,
            std::ptr::null(),
            &mut layout2,
        ) != 0
        {
            return false;
        }
        self.set_layout_tex = layout2;
        true
    }

    unsafe fn create_pipeline_layout(&mut self,) -> bool {
        #[repr(C)]
        struct PipelineLayoutCreateInfo {
            s_type:                    i32,
            p_next:                    *const c_void,
            flags:                     u32,
            set_layout_count:          u32,
            p_set_layouts:             *const VkHandle,
            push_constant_range_count: u32,
            p_push_constant_ranges:    *const c_void,
        }
        let sets = [self.set_layout_ubo, self.set_layout_tex,];
        let ci = PipelineLayoutCreateInfo {
            s_type:                    S_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
            p_next:                    std::ptr::null(),
            flags:                     0,
            set_layout_count:          2,
            p_set_layouts:             sets.as_ptr(),
            push_constant_range_count: 0,
            p_push_constant_ranges:    std::ptr::null(),
        };
        let mut layout: VkHandle = 0;
        let r = (self.f.create_pipeline_layout)(
            self.device,
            &ci as *const _ as *const c_void,
            std::ptr::null(),
            &mut layout,
        );
        if r != 0
        {
            log_error!("vk", format!("vkCreatePipelineLayout failed result={r}"));
            return false;
        }
        self.pipeline_layout = layout;
        true
    }

    unsafe fn create_pool(&mut self,) -> bool {
        #[repr(C)]
        struct PoolSize {
            descriptor_type:  i32,
            descriptor_count: u32,
        }
        #[repr(C)]
        struct PoolCreateInfo {
            s_type:          i32,
            p_next:          *const c_void,
            flags:           u32,
            max_sets:        u32,
            pool_size_count: u32,
            p_pool_sizes:    *const PoolSize,
        }
        let sizes = [
            PoolSize { descriptor_type: DESC_TYPE_UNIFORM_BUFFER, descriptor_count: 8, },
            PoolSize { descriptor_type: DESC_TYPE_SAMPLED_IMAGE, descriptor_count: 64, },
            PoolSize { descriptor_type: DESC_TYPE_SAMPLER, descriptor_count: 64, },
        ];
        let ci = PoolCreateInfo {
            s_type:          S_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
            p_next:          std::ptr::null(),
            flags:           0,
            max_sets:        4096,
            pool_size_count: sizes.len() as u32,
            p_pool_sizes:    sizes.as_ptr(),
        };
        let mut pool: VkHandle = 0;
        if (self.f.create_descriptor_pool)(self.device, &ci as *const _ as *const c_void, std::ptr::null(), &mut pool,)
            != 0
        {
            return false;
        }
        self.pool = pool;

        #[repr(C)]
        struct CommandPoolCreateInfo {
            s_type:             i32,
            p_next:             *const c_void,
            flags:              u32,
            queue_family_index: u32,
        }
        let cci = CommandPoolCreateInfo {
            s_type: S_TYPE_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),

            flags:              0x2,
            queue_family_index: self.queue_family,
        };
        let mut cpool: VkHandle = 0;
        if (self.f.create_command_pool)(self.device, &cci as *const _ as *const c_void, std::ptr::null(), &mut cpool,)
            != 0
        {
            return false;
        }
        self.cmd_pool = cpool;
        true
    }

    unsafe fn ensure_swapchain(&mut self, sc: VkHandle, info: SwapchainInfo,) -> bool {
        if self.swapchain == sc && self.format == info.format && self.width == info.width && self.height == info.height
        {
            return true;
        }
        if self.swapchain != 0
        {
            for fb in self.framebuffers.drain(..,)
            {
                self.retired.vk.push((self.frame_no, fb, DeferKind::Framebuffer,),);
            }
            for v in self.views.drain(..,)
            {
                self.retired.vk.push((self.frame_no, v, DeferKind::View,),);
            }

            self.img_ready.clear();
            if self.render_pass != 0
            {
                self.retired.vk.push((self.frame_no, self.render_pass, DeferKind::RenderPass,),);
                self.render_pass = 0;
            }
            if self.pipeline != 0
            {
                self.retired.vk.push((self.frame_no, self.pipeline, DeferKind::Pipeline,),);
                self.pipeline = 0;
            }
        }

        self.swapchain = sc;
        self.format = info.format;
        self.width = info.width;
        self.height = info.height;
        self.img_ready = vec![false; info.images.max(1) as usize + 2];

        #[repr(C)]
        struct AttachmentDescription {
            flags:            u32,
            format:           i32,
            samples:          u32,
            load_op:          i32,
            store_op:         i32,
            stencil_load_op:  i32,
            stencil_store_op: i32,
            initial_layout:   i32,
            final_layout:     i32,
        }
        #[repr(C)]
        struct AttachmentReference {
            attachment: u32,
            layout:     i32,
        }
        #[repr(C)]
        struct SubpassDescription {
            flags: u32,
            pipeline_bind_point: i32,
            input_attachment_count: u32,
            p_input_attachments: *const AttachmentReference,
            color_attachment_count: u32,
            p_color_attachments: *const AttachmentReference,
            p_resolve_attachments: *const AttachmentReference,
            p_depth_stencil_attachment: *const c_void,
            preserve_attachment_count: u32,
            p_preserve_attachments: *const u32,
        }
        #[repr(C)]
        struct RenderPassCreateInfo {
            s_type:           i32,
            p_next:           *const c_void,
            flags:            u32,
            attachment_count: u32,
            p_attachments:    *const AttachmentDescription,
            subpass_count:    u32,
            p_subpasses:      *const SubpassDescription,
            dependency_count: u32,
            p_dependencies:   *const c_void,
        }
        let att = [AttachmentDescription {
            flags:            0,
            format:           info.format,
            samples:          SAMPLE_COUNT_1,
            load_op:          ATTACHMENT_LOAD_OP_LOAD,
            store_op:         ATTACHMENT_STORE_OP_STORE,
            stencil_load_op:  ATTACHMENT_LOAD_OP_DONT_CARE,
            stencil_store_op: ATTACHMENT_STORE_OP_DONT_CARE,
            initial_layout:   IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            final_layout:     IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        },];
        let color_ref = [AttachmentReference { attachment: 0, layout: IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL, },];
        let sub = [SubpassDescription {
            flags: 0,
            pipeline_bind_point: PIPELINE_BIND_POINT_GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: color_ref.as_ptr(),
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
        },];
        let rpci = RenderPassCreateInfo {
            s_type:           S_TYPE_RENDER_PASS_CREATE_INFO,
            p_next:           std::ptr::null(),
            flags:            0,
            attachment_count: 1,
            p_attachments:    att.as_ptr(),
            subpass_count:    1,
            p_subpasses:      sub.as_ptr(),
            dependency_count: 0,
            p_dependencies:   std::ptr::null(),
        };
        let mut rp: VkHandle = 0;
        let r =
            (self.f.create_render_pass)(self.device, &rpci as *const _ as *const c_void, std::ptr::null(), &mut rp,);
        if r != 0
        {
            log_error!("vk", format!("vkCreateRenderPass failed result={r}"));
            return false;
        }
        self.render_pass = rp;

        let mut count = 0u32;
        if (self.f.get_swapchain_images)(self.device, sc, &mut count, std::ptr::null_mut(),) != 0 || count == 0
        {
            log_error!("vk", "vkGetSwapchainImagesKHR failed");
            return false;
        }
        let mut images = vec![0 as VkHandle; count as usize];
        if (self.f.get_swapchain_images)(self.device, sc, &mut count, images.as_mut_ptr(),) != 0
        {
            return false;
        }
        #[repr(C)]
        struct FramebufferCreateInfo {
            s_type:           i32,
            p_next:           *const c_void,
            flags:            u32,
            render_pass:      VkHandle,
            attachment_count: u32,
            p_attachments:    *const VkHandle,
            width:            u32,
            height:           u32,
            layers:           u32,
        }
        for img in &images
        {
            let Some(view,) = self.make_view(*img, info.format,)
            else
            {
                return false;
            };
            self.views.push(view,);
            let fci = FramebufferCreateInfo {
                s_type:           S_TYPE_FRAMEBUFFER_CREATE_INFO,
                p_next:           std::ptr::null(),
                flags:            0,
                render_pass:      self.render_pass,
                attachment_count: 1,
                p_attachments:    &view,
                width:            self.width,
                height:           self.height,
                layers:           1,
            };
            let mut fb: VkHandle = 0;
            let r =
                (self.f.create_framebuffer)(self.device, &fci as *const _ as *const c_void, std::ptr::null(), &mut fb,);
            if r != 0
            {
                log_error!("vk", format!("vkCreateFramebuffer failed result={r}"));
                return false;
            }
            self.framebuffers.push(fb,);
        }
        log_info!("vk", format!("swapchain views/framebuffers: {}", self.framebuffers.len()));
        true
    }

    unsafe fn create_pipeline(&mut self,) -> bool {
        if self.pipeline != 0
        {
            return true;
        }
        #[repr(C)]
        struct ShaderModuleCreateInfo {
            s_type:    i32,
            p_next:    *const c_void,
            flags:     u32,
            code_size: usize,
            p_code:    *const u32,
        }
        let smci = ShaderModuleCreateInfo {
            s_type:    S_TYPE_SHADER_MODULE_CREATE_INFO,
            p_next:    std::ptr::null(),
            flags:     0,
            code_size: VK_SPV.len() * 4,
            p_code:    VK_SPV.as_ptr(),
        };
        let mut module: VkHandle = 0;
        let r = (self.f.create_shader_module)(
            self.device,
            &smci as *const _ as *const c_void,
            std::ptr::null(),
            &mut module,
        );
        if r != 0
        {
            log_error!("vk", format!("vkCreateShaderModule failed result={r} (bad SPIR-V?)"));
            return false;
        }

        #[repr(C)]
        struct PipelineShaderStage {
            s_type:           i32,
            p_next:           *const c_void,
            flags:            u32,
            stage:            u32,
            module:           VkHandle,
            p_name:           *const u8,
            p_specialization: *const c_void,
        }
        let vs_name = b"vs_main\0";
        let ps_name = b"ps_main\0";
        let stages = [
            PipelineShaderStage {
                s_type: S_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_VERTEX,
                module,
                p_name: vs_name.as_ptr(),
                p_specialization: std::ptr::null(),
            },
            PipelineShaderStage {
                s_type: S_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_FRAGMENT,
                module,
                p_name: ps_name.as_ptr(),
                p_specialization: std::ptr::null(),
            },
        ];

        #[repr(C)]
        struct VertexBinding {
            binding:    u32,
            stride:     u32,
            input_rate: i32,
        }
        #[repr(C)]
        struct VertexAttribute {
            location: u32,
            binding:  u32,
            format:   i32,
            offset:   u32,
        }
        #[repr(C)]
        struct PipelineVertexInput {
            s_type:          i32,
            p_next:          *const c_void,
            flags:           u32,
            binding_count:   u32,
            p_bindings:      *const VertexBinding,
            attribute_count: u32,
            p_attributes:    *const VertexAttribute,
        }
        let binding =
            [VertexBinding { binding: 0, stride: VERTEX_SIZE as u32, input_rate: VERTEX_INPUT_RATE_VERTEX, },];
        let attrs = [
            VertexAttribute { location: 0, binding: 0, format: 103, offset: 0, },
            VertexAttribute { location: 1, binding: 0, format: 103, offset: 8, },
            VertexAttribute { location: 2, binding: 0, format: 37, offset: 16, },
        ];
        let vk_in = PipelineVertexInput {
            s_type:          S_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next:          std::ptr::null(),
            flags:           0,
            binding_count:   1,
            p_bindings:      binding.as_ptr(),
            attribute_count: 3,
            p_attributes:    attrs.as_ptr(),
        };

        #[repr(C)]
        struct PipelineInputAssembly {
            s_type:                   i32,
            p_next:                   *const c_void,
            flags:                    u32,
            topology:                 i32,
            primitive_restart_enable: u32,
        }
        let ia = PipelineInputAssembly {
            s_type:                   S_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next:                   std::ptr::null(),
            flags:                    0,
            topology:                 TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };

        #[repr(C)]
        struct PipelineViewportState {
            s_type:         i32,
            p_next:         *const c_void,
            flags:          u32,
            viewport_count: u32,
            p_viewports:    *const Viewport,
            scissor_count:  u32,
            p_scissors:     *const Rect2D,
        }
        let vs = PipelineViewportState {
            s_type:         S_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next:         std::ptr::null(),
            flags:          0,
            viewport_count: 1,

            p_viewports:   &Viewport {
                x:         0.0,
                y:         0.0,
                width:     self.width as f32,
                height:    self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            },
            scissor_count: 1,
            p_scissors:    &Rect2D {
                offset: Offset2D { x: 0, y: 0, },
                extent: Extent2D { width: self.width, height: self.height, },
            },
        };

        #[repr(C)]
        struct PipelineRasterization {
            s_type:              i32,
            p_next:              *const c_void,
            flags:               u32,
            depth_clamp:         u32,
            rasterizer_discard:  u32,
            polygon_mode:        i32,
            cull_mode:           i32,
            front_face:          i32,
            depth_bias_enable:   u32,
            depth_bias_constant: f32,
            depth_bias_clamp:    f32,
            depth_bias_slope:    f32,
            line_width:          f32,
        }
        let raster = PipelineRasterization {
            s_type:              S_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next:              std::ptr::null(),
            flags:               0,
            depth_clamp:         0,
            rasterizer_discard:  0,
            polygon_mode:        POLYGON_MODE_FILL,
            cull_mode:           CULL_MODE_NONE,
            front_face:          FRONT_FACE_CCW,
            depth_bias_enable:   0,
            depth_bias_constant: 0.0,
            depth_bias_clamp:    0.0,
            depth_bias_slope:    0.0,
            line_width:          1.0,
        };

        #[repr(C)]
        struct PipelineMultisample {
            s_type:                   i32,
            p_next:                   *const c_void,
            flags:                    u32,
            rasterization_samples:    u32,
            sample_shading_enable:    u32,
            min_sample_shading:       f32,
            p_sample_mask:            *const u32,
            alpha_to_coverage_enable: u32,
            alpha_to_one_enable:      u32,
        }
        let ms = PipelineMultisample {
            s_type:                   S_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next:                   std::ptr::null(),
            flags:                    0,
            rasterization_samples:    SAMPLE_COUNT_1,
            sample_shading_enable:    0,
            min_sample_shading:       0.0,
            p_sample_mask:            std::ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable:      0,
        };

        #[repr(C)]
        struct BlendAttachment {
            blend_enable: u32,
            src_color:    i32,
            dst_color:    i32,
            color_op:     i32,
            src_alpha:    i32,
            dst_alpha:    i32,
            alpha_op:     i32,
            write_mask:   u32,
        }
        #[repr(C)]
        struct PipelineColorBlend {
            s_type:           i32,
            p_next:           *const c_void,
            flags:            u32,
            logic_op_enable:  u32,
            logic_op:         i32,
            attachment_count: u32,
            p_attachments:    *const BlendAttachment,
            blend_constants:  [f32; 4],
        }
        let ba = [BlendAttachment {
            blend_enable: 1,
            src_color:    BLEND_FACTOR_ONE,
            dst_color:    BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
            color_op:     BLEND_OP_ADD,
            src_alpha:    BLEND_FACTOR_ONE,
            dst_alpha:    BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
            alpha_op:     BLEND_OP_ADD,
            write_mask:   COLOR_COMPONENT_ALL,
        },];
        let blend = PipelineColorBlend {
            s_type:           S_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next:           std::ptr::null(),
            flags:            0,
            logic_op_enable:  0,
            logic_op:         0,
            attachment_count: 1,
            p_attachments:    ba.as_ptr(),
            blend_constants:  [0.0; 4],
        };

        #[repr(C)]
        struct PipelineDynamicState {
            s_type:              i32,
            p_next:              *const c_void,
            flags:               u32,
            dynamic_state_count: u32,
            p_dynamic_states:    *const i32,
        }
        let dyn_states = [DYNAMIC_STATE_SCISSOR,];
        let dynamic = PipelineDynamicState {
            s_type:              S_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next:              std::ptr::null(),
            flags:               0,
            dynamic_state_count: dyn_states.len() as u32,
            p_dynamic_states:    dyn_states.as_ptr(),
        };

        #[repr(C)]
        struct GraphicsPipelineCreateInfo {
            s_type:              i32,
            p_next:              *const c_void,
            flags:               u32,
            stage_count:         u32,
            p_stages:            *const PipelineShaderStage,
            p_vertex_input:      *const PipelineVertexInput,
            p_input_assembly:    *const PipelineInputAssembly,
            p_tessellation:      *const c_void,
            p_viewport:          *const PipelineViewportState,
            p_rasterization:     *const PipelineRasterization,
            p_multisample:       *const PipelineMultisample,
            p_depth_stencil:     *const c_void,
            p_color_blend:       *const PipelineColorBlend,
            p_dynamic:           *const PipelineDynamicState,
            layout:              VkHandle,
            render_pass:         VkHandle,
            subpass:             u32,
            base_pipeline:       VkHandle,
            base_pipeline_index: i32,
        }
        let gpci = GraphicsPipelineCreateInfo {
            s_type:              S_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            p_next:              std::ptr::null(),
            flags:               0,
            stage_count:         2,
            p_stages:            stages.as_ptr(),
            p_vertex_input:      &vk_in,
            p_input_assembly:    &ia,
            p_tessellation:      std::ptr::null(),
            p_viewport:          &vs,
            p_rasterization:     &raster,
            p_multisample:       &ms,
            p_depth_stencil:     std::ptr::null(),
            p_color_blend:       &blend,
            p_dynamic:           &dynamic,
            layout:              self.pipeline_layout,
            render_pass:         self.render_pass,
            subpass:             0,
            base_pipeline:       0,
            base_pipeline_index: -1,
        };
        let mut pipeline: VkHandle = 0;
        let r = (self.f.create_graphics_pipelines)(
            self.device,
            0,
            1,
            &gpci as *const _ as *const c_void,
            std::ptr::null(),
            &mut pipeline,
        );
        (self.f.destroy_shader_module)(self.device, module, std::ptr::null(),);
        if r != 0 || pipeline == 0
        {
            log_error!("vk", format!("vkCreateGraphicsPipelines failed result={r}"));
            return false;
        }
        self.pipeline = pipeline;
        true
    }

    unsafe fn create_frames(&mut self,) -> bool {
        if !self.frames.is_empty()
        {
            return true;
        }
        #[repr(C)]
        struct CmdBufferAllocateInfo {
            s_type:       i32,
            p_next:       *const c_void,
            command_pool: VkHandle,
            level:        i32,
            count:        u32,
        }
        #[repr(C)]
        struct FenceCreateInfo {
            s_type: i32,
            p_next: *const c_void,
            flags:  u32,
        }
        let mut frames = Vec::with_capacity(FRAMES,);
        for _ in 0..FRAMES
        {
            let ai = CmdBufferAllocateInfo {
                s_type:       S_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                p_next:       std::ptr::null(),
                command_pool: self.cmd_pool,
                level:        0,
                count:        1,
            };
            let mut cmd: VkHandle = 0;
            if (self.f.allocate_command_buffers)(self.device, &ai as *const _ as *const c_void, &mut cmd,) != 0
            {
                return false;
            }
            let fci = FenceCreateInfo { s_type: S_TYPE_FENCE_CREATE_INFO, p_next: std::ptr::null(), flags: 0x1, };
            let mut fence: VkHandle = 0;
            if (self.f.create_fence)(self.device, &fci as *const _ as *const c_void, std::ptr::null(), &mut fence,) != 0
            {
                return false;
            }
            let Some(ubo,) = self.make_buffer(16, BUF_USAGE_UNIFORM, true,)
            else
            {
                return false;
            };
            let Some(vbo,) = self.make_buffer(256 * 1024, BUF_USAGE_VERTEX | BUF_USAGE_TRANSFER_SRC, true,)
            else
            {
                return false;
            };
            let Some(ibo,) = self.make_buffer(512 * 1024, BUF_USAGE_INDEX | BUF_USAGE_TRANSFER_SRC, true,)
            else
            {
                return false;
            };
            frames.push(FrameRes { cmd, fence, ubo, vbo, ibo, ubo_set: 0, in_flight: false, },);
        }
        self.frames = frames;

        let sem_count = FRAMES.max(self.img_ready.len(),);
        #[repr(C)]
        struct SemCreateInfo {
            s_type: i32,
            p_next: *const c_void,
            flags:  u32,
        }
        let sci = SemCreateInfo { s_type: S_TYPE_SEMAPHORE_CREATE_INFO, p_next: std::ptr::null(), flags: 0, };
        for _ in 0..sem_count
        {
            let mut sem: VkHandle = 0;
            if (self.f.create_semaphore)(self.device, &sci as *const _ as *const c_void, std::ptr::null(), &mut sem,)
                != 0
            {
                return false;
            }
            self.semaphores.push(sem,);
        }

        let layouts = [self.set_layout_ubo; FRAMES];
        #[repr(C)]
        struct SetAllocateInfo {
            s_type:        i32,
            p_next:        *const c_void,
            pool:          VkHandle,
            count:         u32,
            p_set_layouts: *const VkHandle,
        }
        let ai = SetAllocateInfo {
            s_type:        S_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next:        std::ptr::null(),
            pool:          self.pool,
            count:         FRAMES as u32,
            p_set_layouts: layouts.as_ptr(),
        };
        let mut sets = [0 as VkHandle; FRAMES];
        if (self.f.allocate_descriptor_sets)(self.device, &ai as *const _ as *const c_void, sets.as_mut_ptr(),) != 0
        {
            return false;
        }
        for (i, f,) in self.frames.iter_mut().enumerate()
        {
            f.ubo_set = sets[i];
        }

        #[repr(C)]
        struct DescriptorBufferInfo {
            buffer: VkHandle,
            offset: u64,
            range:  u64,
        }
        #[repr(C)]
        struct WriteDescriptorSet {
            s_type:            i32,
            p_next:            *const c_void,
            dst_set:           VkHandle,
            dst_binding:       u32,
            dst_array_element: u32,
            descriptor_count:  u32,
            descriptor_type:   i32,
            p_image_info:      *const c_void,
            p_buffer_info:     *const DescriptorBufferInfo,
            p_texel_view:      *const c_void,
        }
        for f in &self.frames
        {
            let bi = DescriptorBufferInfo { buffer: f.ubo.buffer, offset: 0, range: 16, };
            let w = WriteDescriptorSet {
                s_type:            S_TYPE_WRITE_DESCRIPTOR_SET,
                p_next:            std::ptr::null(),
                dst_set:           f.ubo_set,
                dst_binding:       0,
                dst_array_element: 0,
                descriptor_count:  1,
                descriptor_type:   DESC_TYPE_UNIFORM_BUFFER,
                p_image_info:      std::ptr::null(),
                p_buffer_info:     &bi,
                p_texel_view:      std::ptr::null(),
            };
            (self.f.update_descriptor_sets)(self.device, 1, &w as *const _ as *const c_void, 0, std::ptr::null(),);
        }
        true
    }

    unsafe fn make_white(&mut self,) -> bool {
        self.pending.push(Upload {
            key:   WHITE_KEY,
            x:     0,
            y:     0,
            w:     1,
            h:     1,
            bytes: vec![255u8],
            fresh: true,
        },);
        true
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
                        self.pending.push(Upload {
                            key,
                            x: 0,
                            y: 0,
                            w: w as u32,
                            h: h as u32,
                            bytes: alpha,
                            fresh: true,
                        },);
                    }
                    Some([x, y,],) => self.pending.push(Upload {
                        key,
                        x: x as u32,
                        y: y as u32,
                        w: w as u32,
                        h: h as u32,
                        bytes: alpha,
                        fresh: false,
                    },),
                }
            }
        }
        for id in &delta.free
        {
            let key = mesh::tex_key(*id,);

            if let Some(old_set,) = self.tex_sets.remove(&key,)
            {
                self.retired.sets.push((self.frame_no, old_set,),);
            }
            if let Some(old,) = self.tex.remove(&key,)
            {
                self.retired.tex.push((self.frame_no, old,),);
            }
        }
    }

    unsafe fn ensure_frame_buffers(&mut self, slot: usize, vneed: u64, ineed: u64,) -> bool {
        let need_v = self.frames[slot].vbo.size < vneed;
        let need_i = self.frames[slot].ibo.size < ineed;
        if !need_v && !need_i
        {
            return true;
        }
        let grow = |n: u64| {
            let mut cap = 1u64 << 18;
            while cap < n
            {
                cap *= 2;
            }
            cap
        };
        if need_v
        {
            let Some(nb,) = self.make_buffer(grow(vneed,), BUF_USAGE_VERTEX | BUF_USAGE_TRANSFER_SRC, true,)
            else
            {
                return false;
            };
            let old = std::mem::replace(&mut self.frames[slot].vbo, nb,);
            self.retired.buf.push((self.frame_no, old,),);
        }
        if need_i
        {
            let Some(nb,) = self.make_buffer(grow(ineed,), BUF_USAGE_INDEX | BUF_USAGE_TRANSFER_SRC, true,)
            else
            {
                return false;
            };
            let old = std::mem::replace(&mut self.frames[slot].ibo, nb,);
            self.retired.buf.push((self.frame_no, old,),);
        }
        true
    }

    unsafe fn ensure_staging(&mut self, need: u64,) -> bool {
        if let Some(s,) = self.staging.as_ref()
        {
            if s.size >= need
            {
                return true;
            }
        }
        let mut cap = 1u64 << 20;
        while cap < need && cap < (64u64 << 20)
        {
            cap *= 2;
        }
        match self.make_buffer(cap, BUF_USAGE_TRANSFER_SRC, true,)
        {
            Some(b,) =>
            {
                if let Some(old,) = self.staging.replace(b,)
                {
                    self.retired.buf.push((self.frame_no, old,),);
                }
                true
            }
            None => false,
        }
    }

    pub unsafe fn paint(
        &mut self,
        sc: VkHandle,
        queue: VkHandle,
        image_index: u32,
        prims: &[egui::ClippedPrimitive],
        screen: (u32, u32,),
    ) -> Option<VkHandle,> {
        let info = swapchain_info(sc,)?;
        if !self.ensure_swapchain(sc, info,) || !self.create_pipeline()
        {
            return None;
        }
        if image_index as usize >= self.framebuffers.len()
        {
            return None;
        }
        self.screen = screen;
        let batch: Batch = mesh::build(prims, screen,);

        let total: u64 = self.pending.iter().fold(0u64, |acc, u| (acc + u.bytes.len() as u64 + 63) & !63,);
        if total > 0 && !self.ensure_staging(total.max(4,),)
        {
            log_error!("vk", "staging buffer unavailable, frame skipped (delta kept for next frame)");
            return None;
        }
        let uploads = std::mem::take(&mut self.pending,);
        if let Some(st,) = self.staging.as_ref()
        {
            let mut off = 0u64;
            for u in &uploads
            {
                if st.mapped.is_null()
                {
                    break;
                }
                std::ptr::copy_nonoverlapping(u.bytes.as_ptr(), st.mapped.add(off as usize,), u.bytes.len(),);
                off = (off + u.bytes.len() as u64 + 63) & !63;
            }
        }

        let slot = self.frame_idx % self.frames.len().max(1,);
        self.frame_idx = self.frame_idx.wrapping_add(1,);
        self.frame_no = self.frame_no.wrapping_add(1,);

        self.drain_retired_aged();
        {
            let f = &self.frames[slot];
            if f.in_flight
            {
                let fences = [f.fence,];

                if (self.f.wait_for_fences)(self.device, 1, fences.as_ptr(), 1, 8_000_000,) != 0
                {
                    return None;
                }
            }
        }
        let (cmd, fence,) = (self.frames[slot].cmd, self.frames[slot].fence,);
        (self.f.reset_fences)(self.device, 1, &fence,);

        #[repr(C)]
        struct CmdBeginInfo {
            s_type:        i32,
            p_next:        *const c_void,
            flags:         u32,
            p_inheritance: *const c_void,
        }
        let bi = CmdBeginInfo {
            s_type:        S_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            p_next:        std::ptr::null(),
            flags:         0,
            p_inheritance: std::ptr::null(),
        };
        (self.f.reset_command_buffer)(cmd, 0,);
        if (self.f.begin_command_buffer)(cmd, &bi as *const _ as *const c_void,) != 0
        {
            return None;
        }

        let range = SubresourceRange {
            aspect_mask:      IMAGE_ASPECT_COLOR,
            base_mip_level:   0,
            level_count:      1,
            base_array_layer: 0,
            layer_count:      1,
        };

        let mut off = 0u64;
        for u in &uploads
        {
            if u.fresh
            {
                let pair = self.make_image(u.w, u.h, FORMAT_R8_UNORM, IMAGE_USAGE_SAMPLED | IMAGE_USAGE_TRANSFER_DST,);
                let Some((image, memory,),) = pair
                else
                {
                    continue;
                };
                let Some(view,) = self.make_view(image, FORMAT_R8_UNORM,)
                else
                {
                    continue;
                };
                let entry = TexEntry { image, memory, view, w: u.w, h: u.h, };
                if let Some(old,) = self.tex.insert(u.key, entry,)
                {
                    self.retired.tex.push((self.frame_no, old,),);
                }

                if let Some(old_set,) = self.tex_sets.remove(&u.key,)
                {
                    self.retired.sets.push((self.frame_no, old_set,),);
                }
                if !self.bind_texture_set(u.key,)
                {
                    continue;
                }
                let b = ImageMemoryBarrier {
                    s_type: S_TYPE_IMAGE_MEMORY_BARRIER,
                    p_next: std::ptr::null(),
                    src_access_mask: 0,
                    dst_access_mask: ACCESS_TRANSFER_WRITE,
                    old_layout: IMAGE_LAYOUT_UNDEFINED,
                    new_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    src_queue_family: 0xFFFF_FFFF,
                    dst_queue_family: 0xFFFF_FFFF,
                    image,
                    subresource_range: range,
                };
                (self.f.cmd_pipeline_barrier)(
                    cmd,
                    STAGE_TOP_OF_PIPE_FALLBACK,
                    STAGE_TRANSFER,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    1,
                    &b as *const _ as *const c_void,
                );
            }
            else
            {
                let Some(entry,) = self.tex.get(&u.key,)
                else
                {
                    continue;
                };
                if u.x + u.w > entry.w || u.y + u.h > entry.h
                {
                    continue;
                }
                let image = entry.image;
                let b = ImageMemoryBarrier {
                    s_type: S_TYPE_IMAGE_MEMORY_BARRIER,
                    p_next: std::ptr::null(),
                    src_access_mask: ACCESS_SHADER_READ,
                    dst_access_mask: ACCESS_TRANSFER_WRITE,
                    old_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                    new_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    src_queue_family: 0xFFFF_FFFF,
                    dst_queue_family: 0xFFFF_FFFF,
                    image,
                    subresource_range: range,
                };
                (self.f.cmd_pipeline_barrier)(
                    cmd,
                    STAGE_FRAGMENT_SHADER_FALLBACK,
                    STAGE_TRANSFER,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    1,
                    &b as *const _ as *const c_void,
                );
            }

            let Some(entry,) = self.tex.get(&u.key,)
            else
            {
                continue;
            };
            let Some(st,) = self.staging.as_ref()
            else
            {
                continue;
            };
            let region = BufferImageCopy {
                buffer_offset:       off,
                buffer_row_length:   u.w,
                buffer_image_height: u.h,
                image_subresource:   SubresourceLayers {
                    aspect_mask:      IMAGE_ASPECT_COLOR,
                    mip_level:        0,
                    base_array_layer: 0,
                    layer_count:      1,
                },
                image_offset:        Offset3D { x: u.x as i32, y: u.y as i32, z: 0, },
                image_extent:        Extent3D { width: u.w, height: u.h, depth: 1, },
            };
            (self.f.cmd_copy_buffer_to_image)(
                cmd,
                st.buffer,
                entry.image,
                IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                1,
                &region,
            );

            let b2 = ImageMemoryBarrier {
                s_type:            S_TYPE_IMAGE_MEMORY_BARRIER,
                p_next:            std::ptr::null(),
                src_access_mask:   ACCESS_TRANSFER_WRITE,
                dst_access_mask:   ACCESS_SHADER_READ,
                old_layout:        IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                new_layout:        IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                src_queue_family:  0xFFFF_FFFF,
                dst_queue_family:  0xFFFF_FFFF,
                image:             entry.image,
                subresource_range: range,
            };
            (self.f.cmd_pipeline_barrier)(
                cmd,
                STAGE_TRANSFER,
                STAGE_FRAGMENT_SHADER_FALLBACK,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &b2 as *const _ as *const c_void,
            );
            off = (off + u.bytes.len() as u64 + 63) & !63;
        }

        if !batch.is_empty()
        {
            let f = &self.frames[slot];
            if batch.vertex_bytes().len() as u64 > f.vbo.size || batch.index_bytes().len() as u64 > f.ibo.size
            {
                if !self.warned
                {
                    log_warn!(
                        "vk",
                        format!(
                            "vertex data exceeds buffer ({}B / {}B), frame skipped",
                            batch.vertex_bytes().len(),
                            batch.index_bytes().len()
                        )
                    );
                    self.warned = true;
                }
            }
            else
            {
                let srgb_flag = if is_srgb_format(self.format,) { 1.0f32 } else { 0.0f32 };
                let ubo = [screen.0 as f32, screen.1 as f32, srgb_flag, 0.0f32,];
                let ubo_bytes = std::slice::from_raw_parts(ubo.as_ptr() as *const u8, 16,);
                self.write_buffer(&self.frames[slot].ubo, ubo_bytes,);
                if !self
                    .ensure_frame_buffers(slot, batch.vertex_bytes().len() as u64, batch.index_bytes().len() as u64,)
                {
                    self.pending = uploads;
                    return None;
                }
                self.write_buffer(&self.frames[slot].vbo, batch.vertex_bytes(),);
                self.write_buffer(&self.frames[slot].ibo, batch.index_bytes(),);

                let img = self.swapchain_image(image_index,)?;
                let idx = image_index as usize;
                let ready = self.img_ready.get(idx,).copied().unwrap_or(false,);
                if idx < self.img_ready.len()
                {
                    self.img_ready[idx] = true;
                }
                let to_color = ImageMemoryBarrier {
                    s_type:            S_TYPE_IMAGE_MEMORY_BARRIER,
                    p_next:            std::ptr::null(),
                    src_access_mask:   0,
                    dst_access_mask:   ACCESS_COLOR_ATTACHMENT_WRITE,
                    old_layout:        if ready { IMAGE_LAYOUT_PRESENT_SRC } else { IMAGE_LAYOUT_UNDEFINED },
                    new_layout:        IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                    src_queue_family:  0xFFFF_FFFF,
                    dst_queue_family:  0xFFFF_FFFF,
                    image:             img,
                    subresource_range: range,
                };
                (self.f.cmd_pipeline_barrier)(
                    cmd,
                    STAGE_COLOR_ATTACHMENT_OUTPUT,
                    STAGE_COLOR_ATTACHMENT_OUTPUT,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    1,
                    &to_color as *const _ as *const c_void,
                );

                #[repr(C)]
                struct RenderPassBegin {
                    s_type:      i32,
                    p_next:      *const c_void,
                    render_pass: VkHandle,
                    framebuffer: VkHandle,
                    render_area: Rect2D,
                    clear_count: u32,
                    p_clears:    *const c_void,
                }
                let rp = RenderPassBegin {
                    s_type:      43,
                    p_next:      std::ptr::null(),
                    render_pass: self.render_pass,
                    framebuffer: self.framebuffers[image_index as usize],
                    render_area: Rect2D {
                        offset: Offset2D { x: 0, y: 0, },
                        extent: Extent2D { width: self.width, height: self.height, },
                    },
                    clear_count: 0,
                    p_clears:    std::ptr::null(),
                };
                (self.f.cmd_begin_render_pass)(cmd, &rp as *const _ as *const c_void, SUBPASS_CONTENTS_INLINE,);
                (self.f.cmd_bind_pipeline)(cmd, PIPELINE_BIND_POINT_GRAPHICS, self.pipeline,);
                let vb = [self.frames[slot].vbo.buffer,];
                let voff = [0u64,];
                (self.f.cmd_bind_vertex_buffers)(cmd, 0, 1, vb.as_ptr(), voff.as_ptr(),);
                (self.f.cmd_bind_index_buffer)(cmd, self.frames[slot].ibo.buffer, 0, INDEX_TYPE_UINT32,);

                let ubo_set = self.frames[slot].ubo_set;
                for d in &batch.draws
                {
                    let sc_rect = Rect2D {
                        offset: Offset2D { x: d.scissor[0], y: d.scissor[1], },
                        extent: Extent2D {
                            width:  (d.scissor[2] - d.scissor[0]).max(0,) as u32,
                            height: (d.scissor[3] - d.scissor[1]).max(0,) as u32,
                        },
                    };
                    (self.f.cmd_set_scissor)(cmd, 0, 1, &sc_rect,);
                    let tex_set = self.tex_sets.get(&d.tex,).copied().unwrap_or(0,);
                    if tex_set == 0
                    {
                        continue;
                    }
                    let sets = [ubo_set, tex_set,];
                    (self.f.cmd_bind_descriptor_sets)(
                        cmd,
                        PIPELINE_BIND_POINT_GRAPHICS,
                        self.pipeline_layout,
                        0,
                        2,
                        sets.as_ptr(),
                        0,
                        std::ptr::null(),
                    );
                    (self.f.cmd_draw_indexed)(cmd, d.idx_count, 1, d.idx_start, 0, 0,);
                }
                (self.f.cmd_end_render_pass)(cmd,);

                let to_present = ImageMemoryBarrier {
                    s_type:            S_TYPE_IMAGE_MEMORY_BARRIER,
                    p_next:            std::ptr::null(),
                    src_access_mask:   ACCESS_COLOR_ATTACHMENT_WRITE,
                    dst_access_mask:   0,
                    old_layout:        IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                    new_layout:        IMAGE_LAYOUT_PRESENT_SRC,
                    src_queue_family:  0xFFFF_FFFF,
                    dst_queue_family:  0xFFFF_FFFF,
                    image:             img,
                    subresource_range: range,
                };
                (self.f.cmd_pipeline_barrier)(
                    cmd,
                    STAGE_COLOR_ATTACHMENT_OUTPUT,
                    STAGE_BOTTOM_OF_PIPE,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    1,
                    &to_present as *const _ as *const c_void,
                );
            }
        }

        if (self.f.end_command_buffer)(cmd,) != 0
        {
            return None;
        }

        let sem = self.semaphores[image_index as usize % self.semaphores.len().max(1,)];
        #[repr(C)]
        struct SubmitInfo {
            s_type:        i32,
            p_next:        *const c_void,
            wait_count:    u32,
            p_waits:       *const VkHandle,
            p_wait_stages: *const u32,
            cmd_count:     u32,
            p_cmds:        *const VkHandle,
            signal_count:  u32,
            p_signals:     *const VkHandle,
        }
        let si = SubmitInfo {
            s_type:        S_TYPE_SUBMIT_INFO,
            p_next:        std::ptr::null(),
            wait_count:    0,
            p_waits:       std::ptr::null(),
            p_wait_stages: std::ptr::null(),
            cmd_count:     1,
            p_cmds:        &cmd,
            signal_count:  1,
            p_signals:     &sem,
        };
        let r = (self.f.queue_submit)(queue, 1, &si as *const _ as *const c_void, fence,);
        if r != 0
        {
            if !self.warned
            {
                log_error!("vk", format!("vkQueueSubmit failed result={r}"));
                self.warned = true;
            }

            self.pending = uploads;
            return None;
        }
        self.frames[slot].in_flight = true;
        Some(sem,)
    }

    unsafe fn swapchain_image(&self, index: u32,) -> Option<VkHandle,> {
        let mut count = 0u32;
        if (self.f.get_swapchain_images)(self.device, self.swapchain, &mut count, std::ptr::null_mut(),) != 0
            || count == 0
        {
            return None;
        }
        let mut images = vec![0 as VkHandle; count as usize];
        if (self.f.get_swapchain_images)(self.device, self.swapchain, &mut count, images.as_mut_ptr(),) != 0
        {
            return None;
        }
        images.get(index as usize,).copied()
    }

    unsafe fn bind_texture_set(&mut self, key: u64,) -> bool {
        let Some(entry,) = self.tex.get(&key,)
        else
        {
            return false;
        };
        if self.tex_sets.contains_key(&key,)
        {
            return true;
        }
        #[repr(C)]
        struct SetAllocateInfo {
            s_type:        i32,
            p_next:        *const c_void,
            pool:          VkHandle,
            count:         u32,
            p_set_layouts: *const VkHandle,
        }
        let layout = self.set_layout_tex;
        let ai = SetAllocateInfo {
            s_type:        S_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next:        std::ptr::null(),
            pool:          self.pool,
            count:         1,
            p_set_layouts: &layout,
        };
        let mut set: VkHandle = 0;
        if (self.f.allocate_descriptor_sets)(self.device, &ai as *const _ as *const c_void, &mut set,) != 0
        {
            return false;
        }
        #[repr(C)]
        struct DescriptorImageInfo {
            sampler:      VkHandle,
            image_view:   VkHandle,
            image_layout: i32,
        }
        #[repr(C)]
        struct WriteDescriptorSet {
            s_type:            i32,
            p_next:            *const c_void,
            dst_set:           VkHandle,
            dst_binding:       u32,
            dst_array_element: u32,
            descriptor_count:  u32,
            descriptor_type:   i32,
            p_image_info:      *const DescriptorImageInfo,
            p_buffer_info:     *const c_void,
            p_texel_view:      *const c_void,
        }
        let img = DescriptorImageInfo {
            sampler:      0,
            image_view:   entry.view,
            image_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
        };
        let smp = DescriptorImageInfo { sampler: self.sampler, image_view: 0, image_layout: 0, };
        let writes = [
            WriteDescriptorSet {
                s_type:            S_TYPE_WRITE_DESCRIPTOR_SET,
                p_next:            std::ptr::null(),
                dst_set:           set,
                dst_binding:       0,
                dst_array_element: 0,
                descriptor_count:  1,
                descriptor_type:   DESC_TYPE_SAMPLED_IMAGE,
                p_image_info:      &img,
                p_buffer_info:     std::ptr::null(),
                p_texel_view:      std::ptr::null(),
            },
            WriteDescriptorSet {
                s_type:            S_TYPE_WRITE_DESCRIPTOR_SET,
                p_next:            std::ptr::null(),
                dst_set:           set,
                dst_binding:       1,
                dst_array_element: 0,
                descriptor_count:  1,
                descriptor_type:   DESC_TYPE_SAMPLER,
                p_image_info:      &smp,
                p_buffer_info:     std::ptr::null(),
                p_texel_view:      std::ptr::null(),
            },
        ];
        (self.f.update_descriptor_sets)(
            self.device,
            writes.len() as u32,
            writes.as_ptr() as *const c_void,
            0,
            std::ptr::null(),
        );
        self.tex_sets.insert(key, set,);
        true
    }
}

const STAGE_TOP_OF_PIPE_FALLBACK: u32 = 0x1;
const STAGE_FRAGMENT_SHADER_FALLBACK: u32 = 0x80;
const WHITE_KEY: u64 = u64::MAX;
