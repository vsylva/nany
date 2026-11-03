#![allow(non_snake_case, clippy::missing_transmute_annotations)]

mod support;

use std::ffi::c_void;

type Hglrc = *mut c_void;
type Hdc = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy,)]
struct PixelFormatDescriptor {
    size:             u16,
    version:          u16,
    flags:            u32,
    pixel_type:       u8,
    color_bits:       u8,
    red_bits:         u8,
    red_shift:        u8,
    green_bits:       u8,
    green_shift:      u8,
    blue_bits:        u8,
    blue_shift:       u8,
    alpha_bits:       u8,
    alpha_shift:      u8,
    accum_bits:       u8,
    accum_red_bits:   u8,
    accum_green_bits: u8,
    accum_blue_bits:  u8,
    accum_alpha_bits: u8,
    depth_bits:       u8,
    stencil_bits:     u8,
    aux_bits:         u8,
    layer_type:       u8,
    reserved:         u8,
    layer_mask:       u32,
    visible_mask:     u32,
    damage_mask:      u32,
}

unsafe extern "system" {
    fn GetDC(h: support::Hwnd,) -> Hdc;
    fn ChoosePixelFormat(dc: Hdc, pfd: *const PixelFormatDescriptor,) -> i32;
    fn SetPixelFormat(dc: Hdc, format: i32, pfd: *const PixelFormatDescriptor,) -> i32;
    fn LoadLibraryA(name: *const u8,) -> isize;
    fn GetProcAddress(module: isize, name: *const u8,) -> *const c_void;
}

type WglCreateContextFn = unsafe extern "system" fn(Hdc,) -> Hglrc;
type WglMakeCurrentFn = unsafe extern "system" fn(Hdc, Hglrc,) -> i32;
type WglDeleteContextFn = unsafe extern "system" fn(Hglrc,) -> i32;
type WglSwapBuffersFn = unsafe extern "system" fn(Hdc,) -> i32;
type GlClearColorFn = unsafe extern "system" fn(f32, f32, f32, f32,);
type GlClearFn = unsafe extern "system" fn(u32,);
type GlViewportFn = unsafe extern "system" fn(i32, i32, i32, i32,);

unsafe fn sym(module: isize, name: &[u8],) -> *const c_void {
    GetProcAddress(module, name.as_ptr(),)
}

#[test]
fn gl_survives_window_transitions() {
    unsafe {
        let mut win = match support::TestWindow::create("gl", 1280, 720,)
        {
            Some(w,) => w,
            None =>
            {
                eprintln!("[gl] window creation failed, skipping");
                return;
            }
        };
        let gl = LoadLibraryA(b"opengl32.dll\0".as_ptr(),);
        if gl == 0
        {
            eprintln!("[gl] opengl32.dll not available, skipping");
            return;
        }
        let create: WglCreateContextFn = std::mem::transmute(sym(gl, b"wglCreateContext\0",),);
        let make: WglMakeCurrentFn = std::mem::transmute(sym(gl, b"wglMakeCurrent\0",),);
        let delete: WglDeleteContextFn = std::mem::transmute(sym(gl, b"wglDeleteContext\0",),);
        let swap: WglSwapBuffersFn = std::mem::transmute(sym(gl, b"wglSwapBuffers\0",),);
        let clear_color: GlClearColorFn = std::mem::transmute(sym(gl, b"glClearColor\0",),);
        let clear: GlClearFn = std::mem::transmute(sym(gl, b"glClear\0",),);
        let viewport: GlViewportFn = std::mem::transmute(sym(gl, b"glViewport\0",),);

        let dc = GetDC(win.hwnd,);
        if dc.is_null()
        {
            eprintln!("[gl] GetDC failed, skipping");
            return;
        }
        let pfd = PixelFormatDescriptor {
            size:             std::mem::size_of::<PixelFormatDescriptor,>() as u16,
            version:          1,
            flags:            0x0000_0004 | 0x0000_0020 | 0x0000_0001,
            pixel_type:       0,
            color_bits:       32,
            red_bits:         8,
            red_shift:        16,
            green_bits:       8,
            green_shift:      8,
            blue_bits:        8,
            blue_shift:       0,
            alpha_bits:       8,
            alpha_shift:      24,
            accum_bits:       0,
            accum_red_bits:   0,
            accum_green_bits: 0,
            accum_blue_bits:  0,
            accum_alpha_bits: 0,
            depth_bits:       24,
            stencil_bits:     8,
            aux_bits:         0,
            layer_type:       0,
            reserved:         0,
            layer_mask:       0,
            visible_mask:     0,
            damage_mask:      0,
        };
        let fmt = ChoosePixelFormat(dc, &pfd,);
        if fmt == 0 || SetPixelFormat(dc, fmt, &pfd,) == 0
        {
            eprintln!("[gl] SetPixelFormat failed, skipping");
            return;
        }
        let rc = create(dc,);
        if rc.is_null() || make(dc, rc,) == 0
        {
            eprintln!("[gl] context creation failed, skipping");
            return;
        }

        let counter = support::Counter::new();
        nany::init("test_gl", Box::new(support::SceneProvider::new(counter.clone(),),),);

        for step in support::window_steps()
        {
            win.pump(8,);
            (step.apply)(&mut win,);
            let (w, h,) = win.client_size();
            let before = counter.frames();
            for _ in 0..24
            {
                viewport(0, 0, w as i32, h as i32,);
                clear_color(0.06, 0.07, 0.12, 1.0,);
                clear(0x4000 | 0x100,);
                swap(dc,);
                win.pump(1,);
                std::thread::sleep(std::time::Duration::from_millis(4,),);
            }
            let after = counter.frames();
            support::report("gl", step.name, before, after,);
            assert!(after >= before, "gl overlay frame counter went backwards on {}", step.name);
        }
        assert!(counter.frames() > 0, "gl overlay never drew a frame");

        make(dc, std::ptr::null_mut(),);
        delete(rc,);
    }
}
