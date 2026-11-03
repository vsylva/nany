use std::{
    collections::HashMap,
    ffi::{CString, c_void},
};

use super::mesh::{self, Batch, VERTEX_SIZE};
use crate::{log_error, log_info, log_warn};

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_RED: u32 = 0x1903;
const GL_R8: u32 = 0x8229;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_UNSIGNED_INT: u32 = 0x1405;
const GL_FLOAT: u32 = 0x1406;
const GL_TRIANGLES: u32 = 0x0004;
const GL_ARRAY_BUFFER: u32 = 0x8892;
const GL_ELEMENT_ARRAY_BUFFER: u32 = 0x8893;
const GL_STREAM_DRAW: u32 = 0x88E0;
const GL_TEXTURE0: u32 = 0x84C0;
const GL_BLEND: u32 = 0x0BE2;
const GL_SCISSOR_TEST: u32 = 0x0C11;
const GL_DEPTH_TEST: u32 = 0x0B71;
const GL_CULL_FACE: u32 = 0x0B44;
const GL_ONE: u32 = 1;
const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
const GL_FUNC_ADD: u32 = 0x8006;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: u32 = 0x2601;
const GL_CLAMP_TO_EDGE: u32 = 0x812F;
const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;
const GL_UNPACK_ROW_LENGTH: u32 = 0x0CF2;
const GL_VERTEX_SHADER: u32 = 0x8B31;
const GL_FRAGMENT_SHADER: u32 = 0x8B30;
const GL_COMPILE_STATUS: u32 = 0x8B81;
const GL_LINK_STATUS: u32 = 0x8B82;
const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;
const GL_DRAW_FRAMEBUFFER_BINDING: u32 = 0x8CA6;
const GL_READ_FRAMEBUFFER_BINDING: u32 = 0x8CAA;
const GL_DRAW_FRAMEBUFFER: u32 = 0x8CA9;
const GL_READ_FRAMEBUFFER: u32 = 0x8CA8;
const GL_POLYGON_MODE: u32 = 0x0B40;
const GL_FRONT_AND_BACK: u32 = 0x0408;
const GL_FILL: u32 = 0x1B02;
const GL_PRIMITIVE_RESTART: u32 = 0x8F9D;
const GL_FRAMEBUFFER_SRGB: u32 = 0x8DB9;
const GL_RGBA: u32 = 0x1908;
const GL_PACK_ALIGNMENT: u32 = 0x0D05;

const GL_CURRENT_PROGRAM: u32 = 0x8B8D;
const GL_VERTEX_ARRAY_BINDING: u32 = 0x85B5;
const GL_ARRAY_BUFFER_BINDING: u32 = 0x8894;
const GL_ELEMENT_ARRAY_BUFFER_BINDING: u32 = 0x8895;
const GL_TEXTURE_BINDING_2D: u32 = 0x8069;
const GL_ACTIVE_TEXTURE: u32 = 0x84E0;
const GL_VIEWPORT: u32 = 0x0BA2;
const GL_SCISSOR_BOX: u32 = 0x0C10;
const GL_BLEND_SRC_RGB: u32 = 0x80C9;
const GL_BLEND_DST_RGB: u32 = 0x80C8;
const GL_BLEND_SRC_ALPHA: u32 = 0x80CB;
const GL_BLEND_DST_ALPHA: u32 = 0x80CA;
const GL_BLEND_EQUATION_RGB: u32 = 0x8009;
const GL_BLEND_EQUATION_ALPHA: u32 = 0x883D;

const VERSION: u32 = 0x1F02;
const SHADING_LANGUAGE_VERSION: u32 = 0x8B8C;

const VS: &str = r#"#version 130
in vec2 aPos;
in vec2 aUV;
in vec4 aCol;
uniform vec2 uScreen;
out vec2 vUV;
out vec4 vCol;
void main() {
    vec2 p = aPos / uScreen * 2.0 - 1.0;
    gl_Position = vec4(p.x, -p.y, 0.0, 1.0);
    vUV = aUV;
    vCol = aCol;
}
"#;

const FS: &str = r#"#version 130
in vec2 vUV;
in vec4 vCol;
uniform sampler2D uTex;
out vec4 oColor;
void main() {
    float a = texture(uTex, vUV).r;
    oColor = vCol * a;
}
"#;

type Gpa = unsafe extern "system" fn(*const u8,) -> *const c_void;

#[derive(Default,)]
struct Fns {
    get_integerv:               Option<unsafe extern "system" fn(u32, *mut i32,),>,
    get_string:                 Option<unsafe extern "system" fn(u32,) -> *const u8,>,
    get_error:                  Option<unsafe extern "system" fn() -> u32,>,
    enable:                     Option<unsafe extern "system" fn(u32,),>,
    disable:                    Option<unsafe extern "system" fn(u32,),>,
    blend_func_separate:        Option<unsafe extern "system" fn(u32, u32, u32, u32,),>,
    blend_equation:             Option<unsafe extern "system" fn(u32,),>,
    viewport:                   Option<unsafe extern "system" fn(i32, i32, i32, i32,),>,
    scissor:                    Option<unsafe extern "system" fn(i32, i32, i32, i32,),>,
    pixel_storei:               Option<unsafe extern "system" fn(u32, i32,),>,
    active_texture:             Option<unsafe extern "system" fn(u32,),>,
    gen_textures:               Option<unsafe extern "system" fn(i32, *mut u32,),>,
    delete_textures:            Option<unsafe extern "system" fn(i32, *const u32,),>,
    bind_texture:               Option<unsafe extern "system" fn(u32, u32,),>,
    tex_parameteri:             Option<unsafe extern "system" fn(u32, u32, i32,),>,
    tex_image_2d: Option<unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void,),>,
    tex_sub_image_2d: Option<unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void,),>,
    bind_framebuffer:           Option<unsafe extern "system" fn(u32, u32,),>,
    gen_buffers:                Option<unsafe extern "system" fn(i32, *mut u32,),>,
    delete_buffers:             Option<unsafe extern "system" fn(i32, *const u32,),>,
    bind_buffer:                Option<unsafe extern "system" fn(u32, u32,),>,
    buffer_data:                Option<unsafe extern "system" fn(u32, isize, *const c_void, u32,),>,
    gen_vertex_arrays:          Option<unsafe extern "system" fn(i32, *mut u32,),>,
    delete_vertex_arrays:       Option<unsafe extern "system" fn(i32, *const u32,),>,
    bind_vertex_array:          Option<unsafe extern "system" fn(u32,),>,
    enable_vertex_attrib_array: Option<unsafe extern "system" fn(u32,),>,
    vertex_attrib_pointer:      Option<unsafe extern "system" fn(u32, i32, u32, i8, i32, *const c_void,),>,
    draw_elements:              Option<unsafe extern "system" fn(u32, i32, u32, *const c_void,),>,
    create_shader:              Option<unsafe extern "system" fn(u32,) -> u32,>,
    shader_source:              Option<unsafe extern "system" fn(u32, i32, *const *const i8, *const i32,),>,
    compile_shader:             Option<unsafe extern "system" fn(u32,),>,
    get_shaderiv:               Option<unsafe extern "system" fn(u32, u32, *mut i32,),>,
    get_shader_info_log:        Option<unsafe extern "system" fn(u32, i32, *mut i32, *mut i8,),>,
    create_program:             Option<unsafe extern "system" fn() -> u32,>,
    attach_shader:              Option<unsafe extern "system" fn(u32, u32,),>,
    bind_attrib_location:       Option<unsafe extern "system" fn(u32, u32, *const i8,),>,
    link_program:               Option<unsafe extern "system" fn(u32,),>,
    get_programiv:              Option<unsafe extern "system" fn(u32, u32, *mut i32,),>,
    get_program_info_log:       Option<unsafe extern "system" fn(u32, i32, *mut i32, *mut i8,),>,
    use_program:                Option<unsafe extern "system" fn(u32,),>,
    delete_program:             Option<unsafe extern "system" fn(u32,),>,
    delete_shader:              Option<unsafe extern "system" fn(u32,),>,
    get_uniform_location:       Option<unsafe extern "system" fn(u32, *const i8,) -> i32,>,
    uniform_2f:                 Option<unsafe extern "system" fn(i32, f32, f32,),>,
    uniform_1i:                 Option<unsafe extern "system" fn(i32, i32,),>,
    bind_sampler:               Option<unsafe extern "system" fn(u32, u32,),>,
    polygon_mode:               Option<unsafe extern "system" fn(u32, u32,),>,
    is_enabled:                 Option<unsafe extern "system" fn(u32,) -> u8,>,
    read_pixels:                Option<unsafe extern "system" fn(i32, i32, i32, i32, u32, u32, *mut c_void,),>,
}

unsafe fn resolve<T: Copy,>(module: isize, gpa: Gpa, name: &str,) -> Option<T,> {
    let c = CString::new(name,).ok()?;
    let mut p = gpa(c.as_ptr() as *const u8,);
    if p.is_null()
    {
        p = crate::win::proc_address(module, c.as_bytes_with_nul(),);
    }
    if p.is_null() { None } else { Some(std::mem::transmute_copy(&p,),) }
}

impl Fns {
    unsafe fn load() -> Option<Self,> {
        let module = crate::win::load_library_bytes(b"opengl32.dll\0",);
        if module == 0
        {
            return None;
        }
        let raw = crate::win::proc_address(module, b"wglGetProcAddress\0",);
        if raw.is_null()
        {
            return None;
        }
        let gpa: Gpa = std::mem::transmute(raw,);

        let mut f = Fns {
            get_integerv:               resolve(module, gpa, "glGetIntegerv",),
            get_string:                 resolve(module, gpa, "glGetString",),
            get_error:                  resolve(module, gpa, "glGetError",),
            enable:                     resolve(module, gpa, "glEnable",),
            disable:                    resolve(module, gpa, "glDisable",),
            blend_func_separate:        resolve(module, gpa, "glBlendFuncSeparate",),
            blend_equation:             resolve(module, gpa, "glBlendEquation",),
            viewport:                   resolve(module, gpa, "glViewport",),
            scissor:                    resolve(module, gpa, "glScissor",),
            pixel_storei:               resolve(module, gpa, "glPixelStorei",),
            active_texture:             resolve(module, gpa, "glActiveTexture",),
            gen_textures:               resolve(module, gpa, "glGenTextures",),
            delete_textures:            resolve(module, gpa, "glDeleteTextures",),
            bind_texture:               resolve(module, gpa, "glBindTexture",),
            tex_parameteri:             resolve(module, gpa, "glTexParameteri",),
            tex_image_2d:               resolve(module, gpa, "glTexImage2D",),
            tex_sub_image_2d:           resolve(module, gpa, "glTexSubImage2D",),
            bind_framebuffer:           resolve(module, gpa, "glBindFramebuffer",),
            gen_buffers:                resolve(module, gpa, "glGenBuffers",),
            delete_buffers:             resolve(module, gpa, "glDeleteBuffers",),
            bind_buffer:                resolve(module, gpa, "glBindBuffer",),
            buffer_data:                resolve(module, gpa, "glBufferData",),
            gen_vertex_arrays:          resolve(module, gpa, "glGenVertexArrays",),
            delete_vertex_arrays:       resolve(module, gpa, "glDeleteVertexArrays",),
            bind_vertex_array:          resolve(module, gpa, "glBindVertexArray",),
            enable_vertex_attrib_array: resolve(module, gpa, "glEnableVertexAttribArray",),
            vertex_attrib_pointer:      resolve(module, gpa, "glVertexAttribPointer",),
            draw_elements:              resolve(module, gpa, "glDrawElements",),
            create_shader:              resolve(module, gpa, "glCreateShader",),
            shader_source:              resolve(module, gpa, "glShaderSource",),
            compile_shader:             resolve(module, gpa, "glCompileShader",),
            get_shaderiv:               resolve(module, gpa, "glGetShaderiv",),
            get_shader_info_log:        resolve(module, gpa, "glGetShaderInfoLog",),
            create_program:             resolve(module, gpa, "glCreateProgram",),
            attach_shader:              resolve(module, gpa, "glAttachShader",),
            bind_attrib_location:       resolve(module, gpa, "glBindAttribLocation",),
            link_program:               resolve(module, gpa, "glLinkProgram",),
            get_programiv:              resolve(module, gpa, "glGetProgramiv",),
            get_program_info_log:       resolve(module, gpa, "glGetProgramInfoLog",),
            use_program:                resolve(module, gpa, "glUseProgram",),
            delete_program:             resolve(module, gpa, "glDeleteProgram",),
            delete_shader:              resolve(module, gpa, "glDeleteShader",),
            get_uniform_location:       resolve(module, gpa, "glGetUniformLocation",),
            uniform_2f:                 resolve(module, gpa, "glUniform2f",),
            uniform_1i:                 resolve(module, gpa, "glUniform1i",),
            bind_sampler:               resolve(module, gpa, "glBindSampler",),
            polygon_mode:               resolve(module, gpa, "glPolygonMode",),
            is_enabled:                 resolve(module, gpa, "glIsEnabled",),
            read_pixels:                resolve(module, gpa, "glReadPixels",),
        };

        let need = f.create_program.is_some()
            && f.create_shader.is_some()
            && f.link_program.is_some()
            && f.draw_elements.is_some()
            && f.gen_buffers.is_some()
            && f.get_integerv.is_some()
            && f.viewport.is_some()
            && f.scissor.is_some();
        if !need
        {
            return None;
        }

        f.gen_vertex_arrays = f.gen_vertex_arrays.filter(|_| f.bind_vertex_array.is_some(),);
        Some(f,)
    }
}

struct TexEntry {
    w:   usize,
    h:   usize,
    tex: u32,
}

pub struct PainterGL {
    f:    Fns,
    prog: u32,

    vao:      [u32; 3],
    vbo:      [u32; 3],
    ibo:      [u32; 3],
    buf_idx:  usize,
    u_screen: i32,
    u_tex:    i32,
    tex:      HashMap<u64, TexEntry,>,
    white:    u32,
    size:     (u32, u32,),
    warned:   bool,

    diag_last_err_ms: u128,

    diag_last_read_ms: u128,

    diag_last_state_ms: u128,
}

unsafe impl Send for PainterGL {}

impl Drop for PainterGL {
    fn drop(&mut self,) {
        unsafe {
            if let Some(d,) = self.f.delete_textures
            {
                if self.white != 0
                {
                    d(1, &self.white,);
                }
                for (_, t,) in self.tex.drain()
                {
                    if t.tex != 0
                    {
                        d(1, &t.tex,);
                    }
                }
            }
            if let Some(d,) = self.f.delete_program
            {
                if self.prog != 0
                {
                    d(self.prog,);
                }
            }
            if let Some(d,) = self.f.delete_buffers
            {
                let mut all = [0u32; 6];
                all[..3].copy_from_slice(&self.vbo,);
                all[3..].copy_from_slice(&self.ibo,);
                d(6, all.as_ptr(),);
            }
            if let Some(d,) = self.f.delete_vertex_arrays
            {
                d(3, self.vao.as_ptr(),);
            }
        }
    }
}

struct SavedGl {
    prog:         i32,
    vao:          i32,
    vbo:          i32,
    ibo:          i32,
    unit:         i32,
    tex2d:        i32,
    unpack_align: i32,
    unpack_row:   i32,
}

impl SavedGl {
    unsafe fn capture(f: &Fns,) -> Self {
        let gi = |cap: u32| -> i32 {
            let mut v = 0i32;
            if let Some(g,) = f.get_integerv
            {
                g(cap, &mut v,);
            }
            v
        };
        let unit = gi(GL_ACTIVE_TEXTURE,);
        Self {
            prog: gi(GL_CURRENT_PROGRAM,),
            vao: gi(GL_VERTEX_ARRAY_BINDING,),
            vbo: gi(GL_ARRAY_BUFFER_BINDING,),
            ibo: gi(GL_ELEMENT_ARRAY_BUFFER_BINDING,),
            unit,
            tex2d: gi(GL_TEXTURE_BINDING_2D,),
            unpack_align: gi(GL_UNPACK_ALIGNMENT,),
            unpack_row: gi(GL_UNPACK_ROW_LENGTH,),
        }
    }

    unsafe fn restore(&self, f: &Fns,) {
        if let Some(v,) = f.bind_vertex_array
        {
            v(self.vao.max(0,) as u32,);
        }
        if let Some(b,) = f.bind_buffer
        {
            b(GL_ARRAY_BUFFER, self.vbo.max(0,) as u32,);
            b(GL_ELEMENT_ARRAY_BUFFER, self.ibo.max(0,) as u32,);
        }
        if let Some(u,) = f.use_program
        {
            u(self.prog.max(0,) as u32,);
        }
        if let Some(at,) = f.active_texture
        {
            at(self.unit.max(0,) as u32,);
        }
        if let Some(bt,) = f.bind_texture
        {
            bt(GL_TEXTURE_2D, self.tex2d.max(0,) as u32,);
        }
        if let Some(ps,) = f.pixel_storei
        {
            ps(GL_UNPACK_ALIGNMENT, self.unpack_align,);
            ps(GL_UNPACK_ROW_LENGTH, self.unpack_row,);
        }
    }
}
impl PainterGL {
    pub unsafe fn new() -> Option<Self,> {
        let f = Fns::load()?;
        let saved = SavedGl::capture(&f,);
        let out = Self::new_inner(f,);

        match &out
        {
            Some(p,) => saved.restore(&p.f,),
            None => saved.restore(&Fns::load()?,),
        }
        out
    }

    unsafe fn new_inner(f: Fns,) -> Option<Self,> {
        let (get_string, _,) = (f.get_string, f.get_integerv?,);

        let mut version = String::from("?",);
        if let Some(gs,) = get_string
        {
            let p = gs(VERSION,);
            if !p.is_null()
            {
                version = std::ffi::CStr::from_ptr(p as *const i8,).to_string_lossy().into_owned();
            }
        }
        let mut glsl = String::from("?",);
        if let Some(gs,) = get_string
        {
            let p = gs(SHADING_LANGUAGE_VERSION,);
            if !p.is_null()
            {
                glsl = std::ffi::CStr::from_ptr(p as *const i8,).to_string_lossy().into_owned();
            }
        }

        let prog = Self::build_program(&f,)?;
        let mut vao = [0u32; 3];
        let mut vbos = [0u32; 3];
        let mut ibos = [0u32; 3];
        let bind_buffer = f.bind_buffer?;
        if let (Some(gen_vao,), Some(bind_vao,),) = (f.gen_vertex_arrays, f.bind_vertex_array,)
        {
            gen_vao(3, vao.as_mut_ptr(),);
            (f.gen_buffers?)(3, vbos.as_mut_ptr(),);
            (f.gen_buffers?)(3, ibos.as_mut_ptr(),);

            if let (Some(vap,), Some(ena,),) = (f.vertex_attrib_pointer, f.enable_vertex_attrib_array,)
            {
                for k in 0..3
                {
                    bind_vao(vao[k],);
                    bind_buffer(GL_ARRAY_BUFFER, vbos[k],);
                    bind_buffer(GL_ELEMENT_ARRAY_BUFFER, ibos[k],);
                    ena(0,);
                    vap(0, 2, GL_FLOAT, 0, VERTEX_SIZE as i32, std::ptr::null::<c_void,>(),);
                    ena(1,);
                    vap(1, 2, GL_FLOAT, 0, VERTEX_SIZE as i32, 8 as *const c_void,);
                    ena(2,);
                    vap(2, 4, GL_UNSIGNED_BYTE, 1, VERTEX_SIZE as i32, 16 as *const c_void,);
                }
            }
        }
        else
        {
            (f.gen_buffers?)(2, vbos.as_mut_ptr(),);
        }

        let white = Self::make_texture(&f, 1, 1, &[255u8,],)?;
        log_info!("gl", format!("painter ready (GL {version}, GLSL {glsl}) state-iso=2"));
        Some(PainterGL {
            u_screen: (f.get_uniform_location?)(prog, b"uScreen\0".as_ptr() as *const i8,),
            u_tex: (f.get_uniform_location?)(prog, b"uTex\0".as_ptr() as *const i8,),
            f,
            prog,
            vao,
            vbo: vbos,
            ibo: ibos,
            buf_idx: 0,
            tex: HashMap::new(),
            white,
            size: (0, 0,),
            warned: false,
            diag_last_err_ms: 0,
            diag_last_read_ms: 0,
            diag_last_state_ms: 0,
        },)
    }

    unsafe fn build_program(f: &Fns,) -> Option<u32,> {
        let compile = |kind: u32, src: &str| -> Option<u32,> {
            let sh = (f.create_shader?)(kind,);
            if sh == 0
            {
                return None;
            }
            let c = CString::new(src,).ok()?;
            let ptr = c.as_ptr();
            (f.shader_source?)(sh, 1, &ptr, std::ptr::null(),);
            (f.compile_shader?)(sh,);
            let mut ok = 0i32;
            (f.get_shaderiv?)(sh, GL_COMPILE_STATUS, &mut ok,);
            if ok == 0
            {
                let mut n = 0i32;
                (f.get_shaderiv?)(sh, 0x8B84, &mut n,);
                let mut buf = vec![0u8; n.max(1) as usize];
                (f.get_shader_info_log?)(sh, n, std::ptr::null_mut(), buf.as_mut_ptr() as *mut i8,);
                let text = String::from_utf8_lossy(&buf,);
                log_error!("gl", format!("shader compile failed: {}", text.trim_end_matches('\0')));
                (f.delete_shader?)(sh,);
                return None;
            }
            Some(sh,)
        };

        let vs = compile(GL_VERTEX_SHADER, VS,)?;
        let fs = compile(GL_FRAGMENT_SHADER, FS,)?;
        let prog = (f.create_program?)();
        if prog == 0
        {
            return None;
        }
        (f.attach_shader?)(prog, vs,);
        (f.attach_shader?)(prog, fs,);
        (f.bind_attrib_location?)(prog, 0, b"aPos\0".as_ptr() as *const i8,);
        (f.bind_attrib_location?)(prog, 1, b"aUV\0".as_ptr() as *const i8,);
        (f.bind_attrib_location?)(prog, 2, b"aCol\0".as_ptr() as *const i8,);
        (f.link_program?)(prog,);
        let mut ok = 0i32;
        (f.get_programiv?)(prog, GL_LINK_STATUS, &mut ok,);
        if ok == 0
        {
            let mut n = 0i32;
            (f.get_programiv?)(prog, 0x8B84, &mut n,);
            let mut buf = vec![0u8; n.max(1) as usize];
            (f.get_program_info_log?)(prog, n, std::ptr::null_mut(), buf.as_mut_ptr() as *mut i8,);
            log_error!("gl", format!("program link failed: {}", String::from_utf8_lossy(&buf).trim_end_matches('\0')));
            (f.delete_program?)(prog,);
            return None;
        }
        (f.delete_shader?)(vs,);
        (f.delete_shader?)(fs,);
        Some(prog,)
    }

    unsafe fn make_texture(f: &Fns, w: usize, h: usize, data: &[u8],) -> Option<u32,> {
        let mut tex = 0u32;
        (f.gen_textures?)(1, &mut tex,);
        if tex == 0
        {
            return None;
        }
        (f.bind_texture?)(GL_TEXTURE_2D, tex,);
        (f.tex_parameteri?)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32,);
        (f.tex_parameteri?)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32,);
        (f.tex_parameteri?)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE as i32,);
        (f.tex_parameteri?)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE as i32,);
        (f.pixel_storei?)(GL_UNPACK_ALIGNMENT, 1,);
        (f.pixel_storei?)(GL_UNPACK_ROW_LENGTH, 0,);
        (f.tex_image_2d?)(
            GL_TEXTURE_2D,
            0,
            GL_R8 as i32,
            w as i32,
            h as i32,
            0,
            GL_RED,
            GL_UNSIGNED_BYTE,
            data.as_ptr() as *const c_void,
        );
        Some(tex,)
    }

    pub unsafe fn tex_count(&self,) -> usize {
        self.tex.len()
    }

    pub unsafe fn apply_textures(&mut self, delta: &egui::TexturesDelta,) {
        let Some(bind,) = self.f.bind_texture
        else
        {
            return;
        };
        let Some(sub,) = self.f.tex_sub_image_2d
        else
        {
            return;
        };
        let Some(ps,) = self.f.pixel_storei
        else
        {
            return;
        };
        let del = self.f.delete_textures;

        let saved = SavedGl::capture(&self.f,);
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
                        if let Some(tex,) = Self::make_texture(&self.f, w, h, &alpha,)
                        {
                            if let Some(old,) = self.tex.insert(key, TexEntry { w, h, tex, },)
                            {
                                if let Some(d,) = del
                                {
                                    d(1, &old.tex,);
                                }
                            }
                        }
                    }
                    Some([x, y,],) =>
                    {
                        let Some(e,) = self.tex.get(&key,)
                        else
                        {
                            continue;
                        };
                        if x + w > e.w || y + h > e.h
                        {
                            continue;
                        }
                        let tex = e.tex;
                        bind(GL_TEXTURE_2D, tex,);
                        ps(GL_UNPACK_ALIGNMENT, 1,);
                        ps(GL_UNPACK_ROW_LENGTH, 0,);
                        sub(
                            GL_TEXTURE_2D,
                            0,
                            x as i32,
                            y as i32,
                            w as i32,
                            h as i32,
                            GL_RED,
                            GL_UNSIGNED_BYTE,
                            alpha.as_ptr() as *const c_void,
                        );
                    }
                }
            }
        }
        for id in &delta.free
        {
            if let Some(old,) = self.tex.remove(&mesh::tex_key(*id,),)
            {
                if let Some(d,) = del
                {
                    d(1, &old.tex,);
                }
            }
        }

        saved.restore(&self.f,);
    }

    pub unsafe fn paint(&mut self, prims: &[egui::ClippedPrimitive], screen: (u32, u32,),) {
        let batch = mesh::build(prims, screen,);
        if batch.is_empty()
        {
            return;
        }
        self.size = screen;
        self.draw(&batch,);
    }

    unsafe fn draw(&mut self, batch: &Batch,) {
        let f = &self.f;
        let (Some(get_iv,), Some(bind_vao,), Some(bind_buf,), Some(buf_data,), Some(draw,), Some(use_prog,),) =
            (f.get_integerv, f.bind_vertex_array, f.bind_buffer, f.buffer_data, f.draw_elements, f.use_program,)
        else
        {
            if !self.warned
            {
                log_warn!("gl", "missing required GL entry points (VAO/VBO/Draw), skipping draw");
                self.warned = true;
            }
            return;
        };

        let gi = |cap: u32| -> i32 {
            let mut v = 0i32;
            get_iv(cap, &mut v,);
            v
        };
        let saved_prog = gi(GL_CURRENT_PROGRAM,);
        let saved_vao = gi(GL_VERTEX_ARRAY_BINDING,);
        let saved_vbo = gi(GL_ARRAY_BUFFER_BINDING,);
        let saved_ibo = gi(GL_ELEMENT_ARRAY_BUFFER_BINDING,);
        let saved_tex = gi(GL_TEXTURE_BINDING_2D,);
        let saved_unit = gi(GL_ACTIVE_TEXTURE,);
        let saved_fbo = gi(GL_FRAMEBUFFER_BINDING,);
        let mut vp = [0i32; 4];
        get_iv(GL_VIEWPORT, vp.as_mut_ptr(),);
        let mut sc = [0i32; 4];
        get_iv(GL_SCISSOR_BOX, sc.as_mut_ptr(),);
        let saved_blend = Self::get_flag(f, GL_BLEND,);
        let saved_scissor = Self::get_flag(f, GL_SCISSOR_TEST,);
        let saved_depth = Self::get_flag(f, GL_DEPTH_TEST,);
        let saved_cull = Self::get_flag(f, GL_CULL_FACE,);
        let bp = [gi(GL_BLEND_SRC_RGB,), gi(GL_BLEND_DST_RGB,), gi(GL_BLEND_SRC_ALPHA,), gi(GL_BLEND_DST_ALPHA,),];
        let eq_rgb = gi(GL_BLEND_EQUATION_RGB,);
        let eq_alpha = gi(GL_BLEND_EQUATION_ALPHA,);

        let mut pm = [GL_FILL as i32; 2];
        get_iv(GL_POLYGON_MODE, pm.as_mut_ptr(),);

        let saved_restart = Self::is_flag(f, GL_PRIMITIVE_RESTART,);

        let saved_draw_fb = gi(GL_DRAW_FRAMEBUFFER_BINDING,);
        let saved_read_fb = gi(GL_READ_FRAMEBUFFER_BINDING,);

        let saved_fb_srgb = Self::is_flag(f, GL_FRAMEBUFFER_SRGB,);
        Self::set_flag(f, GL_FRAMEBUFFER_SRGB, false,);
        Self::set_flag(f, GL_PRIMITIVE_RESTART, false,);
        if let Some(pm_fn,) = f.polygon_mode
        {
            pm_fn(GL_FRONT_AND_BACK, GL_FILL,);
        }
        let saved_unpack_align = gi(GL_UNPACK_ALIGNMENT,);
        let saved_row_len = gi(GL_UNPACK_ROW_LENGTH,);

        let now = crate::overlay::now_ms();
        if now.saturating_sub(self.diag_last_state_ms,) >= 2000
        {
            self.diag_last_state_ms = now;
            log_info!(
                "gl",
                format!(
                    "pre-state fbo={saved_draw_fb}/{saved_read_fb} vao={saved_vao} prog={saved_prog} vbo={saved_vbo} ibo={saved_ibo} blend={saved_blend} scissor={saved_scissor} depth={saved_depth} cull={saved_cull} srgb={saved_fb_srgb} poly={:?} vp={:?}",
                    pm, vp
                )
            );
        }

        if let Some(bind_fbo,) = f.bind_framebuffer
        {
            bind_fbo(GL_FRAMEBUFFER, 0,);
        }
        if let Some(v,) = f.viewport
        {
            v(0, 0, self.size.0 as i32, self.size.1 as i32,);
        }
        if let Some(e,) = f.enable
        {
            e(GL_BLEND,);
            e(GL_SCISSOR_TEST,);
        }
        if let Some(d,) = f.disable
        {
            d(GL_DEPTH_TEST,);
            d(GL_CULL_FACE,);
        }
        if let Some(bf,) = f.blend_func_separate
        {
            bf(GL_ONE, GL_ONE_MINUS_SRC_ALPHA, GL_ONE, GL_ONE_MINUS_SRC_ALPHA,);
        }
        if let Some(be,) = f.blend_equation
        {
            be(GL_FUNC_ADD,);
        }

        let saved_sampler = gi(0x8919,);
        if let Some(bs,) = f.bind_sampler
        {
            bs(0, 0,);
        }

        let k = self.buf_idx;
        self.buf_idx = (self.buf_idx + 1) % 3;
        bind_vao(self.vao[k],);
        bind_buf(GL_ARRAY_BUFFER, self.vbo[k],);
        let vbytes = batch.vertex_bytes();
        buf_data(GL_ARRAY_BUFFER, vbytes.len() as isize, vbytes.as_ptr() as *const c_void, GL_STREAM_DRAW,);
        bind_buf(GL_ELEMENT_ARRAY_BUFFER, self.ibo[k],);
        let ibytes = batch.index_bytes();
        buf_data(GL_ELEMENT_ARRAY_BUFFER, ibytes.len() as isize, ibytes.as_ptr() as *const c_void, GL_STREAM_DRAW,);

        use_prog(self.prog,);
        if let Some(u,) = f.uniform_2f
        {
            u(self.u_screen, self.size.0 as f32, self.size.1 as f32,);
        }
        if let Some(u,) = f.uniform_1i
        {
            u(self.u_tex, 0,);
        }
        if let Some(at,) = f.active_texture
        {
            at(GL_TEXTURE0,);
        }

        let sh = self.size.1 as i32;
        for d in &batch.draws
        {
            let tex = self.tex.get(&d.tex,).map(|t| t.tex,).unwrap_or(self.white,);
            if let Some(bt,) = f.bind_texture
            {
                bt(GL_TEXTURE_2D, tex,);
            }
            if let Some(s,) = f.scissor
            {
                let (l, t, r, b,) = (d.scissor[0], d.scissor[1], d.scissor[2], d.scissor[3],);
                s(l, sh - b, (r - l).max(0,), (b - t).max(0,),);
            }
            draw(GL_TRIANGLES, d.idx_count as i32, GL_UNSIGNED_INT, (d.idx_start as usize * 4) as *const c_void,);
        }

        if let (Some(rp,), Some(ps,),) = (f.read_pixels, f.pixel_storei,)
        {
            let now = crate::overlay::now_ms();
            if now.saturating_sub(self.diag_last_read_ms,) >= 2000
            {
                self.diag_last_read_ms = now;
                ps(GL_PACK_ALIGNMENT, 1,);
                let mut a = [0u8; 4];
                let mut b = [0u8; 4];
                let (cx, cy,) = (self.size.0 as i32 / 2, self.size.1 as i32 / 2,);
                let (qx, qy,) = (self.size.0.min(200,) as i32, self.size.1.min(300,) as i32,);
                rp(cx, cy, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, a.as_mut_ptr() as *mut c_void,);
                rp(qx, qy, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, b.as_mut_ptr() as *mut c_void,);
                ps(GL_UNPACK_ALIGNMENT, saved_unpack_align,);
                let black = |p: [u8; 4]| (p[0] | p[1] | p[2]) == 0;
                log_info!(
                    "gl",
                    format!(
                        "readback center=#{:02X}{:02X}{:02X} corner=#{:02X}{:02X}{:02X} -> {}",
                        a[0],
                        a[1],
                        a[2],
                        b[0],
                        b[1],
                        b[2],
                        if black(a) && black(b) { "ALL-BLACK (our draw did not land)" } else { "ok" }
                    )
                );
            }
        }

        bind_vao(saved_vao as u32,);
        bind_buf(GL_ARRAY_BUFFER, saved_vbo as u32,);
        bind_buf(GL_ELEMENT_ARRAY_BUFFER, saved_ibo as u32,);
        if let Some(bind_fbo,) = f.bind_framebuffer
        {
            bind_fbo(GL_READ_FRAMEBUFFER, saved_read_fb.max(0,) as u32,);
            bind_fbo(GL_DRAW_FRAMEBUFFER, saved_draw_fb.max(0,) as u32,);

            if saved_read_fb != saved_draw_fb
            {
                bind_fbo(GL_READ_FRAMEBUFFER, saved_read_fb.max(0,) as u32,);
            }
            else
            {
                bind_fbo(GL_FRAMEBUFFER, saved_fbo.max(0,) as u32,);
            }
        }
        if let Some(v,) = f.viewport
        {
            v(vp[0], vp[1], vp[2], vp[3],);
        }
        if let Some(s,) = f.scissor
        {
            s(sc[0], sc[1], sc[2], sc[3],);
        }
        if let Some(bf,) = f.blend_func_separate
        {
            bf(bp[0] as u32, bp[1] as u32, bp[2] as u32, bp[3] as u32,);
        }
        if let Some(be,) = f.blend_equation
        {
            be(eq_rgb as u32,);
            be(eq_alpha as u32,);
        }
        if let Some(pm_fn,) = f.polygon_mode
        {
            let _ = pm;
            pm_fn(GL_FRONT_AND_BACK, GL_FILL,);
        }
        Self::set_flag(f, GL_PRIMITIVE_RESTART, saved_restart,);
        Self::set_flag(f, GL_FRAMEBUFFER_SRGB, saved_fb_srgb,);
        Self::set_flag(f, GL_BLEND, saved_blend,);
        Self::set_flag(f, GL_SCISSOR_TEST, saved_scissor,);
        Self::set_flag(f, GL_DEPTH_TEST, saved_depth,);
        Self::set_flag(f, GL_CULL_FACE, saved_cull,);
        if let Some(at,) = f.active_texture
        {
            at(saved_unit as u32,);
        }
        if let Some(bt,) = f.bind_texture
        {
            bt(GL_TEXTURE_2D, saved_tex as u32,);
        }
        if let Some(ps,) = f.pixel_storei
        {
            ps(GL_UNPACK_ALIGNMENT, saved_unpack_align,);
            ps(GL_UNPACK_ROW_LENGTH, saved_row_len,);
        }
        if let Some(bs,) = f.bind_sampler
        {
            bs(0, saved_sampler as u32,);
        }
        use_prog(saved_prog as u32,);

        if let Some(ge,) = f.get_error
        {
            let mut first = 0u32;
            for _ in 0..8
            {
                let e = ge();
                if e == 0
                {
                    break;
                }
                if first == 0
                {
                    first = e;
                }
            }
            if first != 0
            {
                let now = crate::overlay::now_ms();
                if now.saturating_sub(self.diag_last_err_ms,) > 1000
                {
                    self.diag_last_err_ms = now;
                    log_info!("gl", format!("glGetError=0x{first:X} after overlay draw (state restored anyway)"));
                }
            }
        }
    }

    unsafe fn get_flag(f: &Fns, cap: u32,) -> bool {
        Self::is_flag(f, cap,)
    }

    unsafe fn is_flag(f: &Fns, cap: u32,) -> bool {
        if let Some(q,) = f.is_enabled
        {
            return q(cap,) != 0;
        }
        Self::get_flag_raw(f, cap,)
    }

    unsafe fn get_flag_raw(f: &Fns, cap: u32,) -> bool {
        let mut v = 0i32;
        if let Some(g,) = f.get_integerv
        {
            g(cap, &mut v,);
        }
        v != 0
    }

    unsafe fn set_flag(f: &Fns, cap: u32, on: bool,) {
        if on
        {
            if let Some(e,) = f.enable
            {
                e(cap,);
            }
        }
        else if let Some(d,) = f.disable
        {
            d(cap,);
        }
    }

    pub unsafe fn last_error(&self,) -> u32 {
        self.f.get_error.map(|f| f(),).unwrap_or(0,)
    }
}
