use egui::ClippedPrimitive;

pub const VERTEX_SIZE: usize = 20;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug,)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv:  [f32; 2],

    pub col: [u8; 4],
}

#[derive(Clone, Copy, Debug,)]
pub struct Draw {
    pub idx_start: u32,
    pub idx_count: u32,

    pub scissor: [i32; 4],
    pub tex:     u64,
}

#[derive(Default, Debug,)]
pub struct Batch {
    pub verts:  Vec<Vertex,>,
    pub idx:    Vec<u32,>,
    pub draws:  Vec<Draw,>,
    pub screen: (u32, u32,),
}

impl Batch {
    pub unsafe fn is_empty(&self,) -> bool {
        self.draws.is_empty() || self.idx.is_empty()
    }

    pub unsafe fn vertex_bytes(&self,) -> &[u8] {
        std::slice::from_raw_parts(self.verts.as_ptr() as *const u8, self.verts.len() * VERTEX_SIZE,)
    }

    pub unsafe fn index_bytes(&self,) -> &[u8] {
        std::slice::from_raw_parts(self.idx.as_ptr() as *const u8, self.idx.len() * 4,)
    }
}

pub unsafe fn tex_key(id: egui::TextureId,) -> u64 {
    match id
    {
        egui::TextureId::Managed(i,) => i,
        egui::TextureId::User(i,) => 0x8000_0000_0000_0000 | i,
    }
}

pub unsafe fn build(prims: &[ClippedPrimitive], screen: (u32, u32,),) -> Batch {
    let mut b = Batch { screen, ..Default::default() };

    for cp in prims
    {
        let egui::epaint::Primitive::Mesh(mesh,) = &cp.primitive
        else
        {
            continue;
        };
        if mesh.indices.is_empty() || mesh.vertices.is_empty()
        {
            continue;
        }
        let base = b.verts.len() as u32;
        b.verts.extend(mesh.vertices.iter().map(|v| Vertex {
            pos: [v.pos.x, v.pos.y,],
            uv:  [v.uv.x, v.uv.y,],
            col: [v.color.r(), v.color.g(), v.color.b(), v.color.a(),],
        },),);
        let idx_start = b.idx.len() as u32;
        b.idx.extend(mesh.indices.iter().map(|i| i + base,),);

        let r = cp.clip_rect;
        let scissor = [
            r.min.x.max(0.0,) as i32,
            r.min.y.max(0.0,) as i32,
            (r.max.x.min(screen.0 as f32,) as i32).max(0,),
            (r.max.y.min(screen.1 as f32,) as i32).max(0,),
        ];
        if scissor[2] <= scissor[0] || scissor[3] <= scissor[1]
        {
            continue;
        }
        b.draws
            .push(Draw { idx_start, idx_count: mesh.indices.len() as u32, scissor, tex: tex_key(mesh.texture_id,), },);
    }

    b
}

pub unsafe fn alpha_of(image: &egui::ColorImage,) -> Vec<u8,> {
    image.pixels.iter().map(|c| c.a(),).collect()
}
