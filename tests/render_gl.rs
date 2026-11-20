#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case, clippy::missing_transmute_annotations)]

mod support;

use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            Gdi::{GetDC, ReleaseDC},
            OpenGL::{
                ChoosePixelFormat, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_FLAGS, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA,
                PIXELFORMATDESCRIPTOR, SetPixelFormat, SwapBuffers, glClear, glClearColor, glViewport, wglCreateContext,
                wglDeleteContext, wglMakeCurrent,
            },
        },
    },
};

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
        let window = HWND(win.hwnd,);

        let dc = GetDC(window,);
        if dc.is_invalid()
        {
            eprintln!("[gl] GetDC failed, skipping");
            return;
        }
        let pfd = PIXELFORMATDESCRIPTOR {
            nSize:       std::mem::size_of::<PIXELFORMATDESCRIPTOR,>() as u16,
            nVersion:    1,
            dwFlags:     PFD_FLAGS(PFD_DRAW_TO_WINDOW.0 | PFD_SUPPORT_OPENGL.0 | PFD_DOUBLEBUFFER.0,),
            iPixelType:  PFD_TYPE_RGBA,
            cColorBits:  32,
            cRedBits:    8,
            cRedShift:   16,
            cGreenBits:  8,
            cGreenShift: 8,
            cBlueBits:   8,
            cBlueShift:  0,
            cAlphaBits:  8,
            cAlphaShift: 24,
            cDepthBits:  24,
            cStencilBits: 8,
            ..Default::default()
        };
        let format = ChoosePixelFormat(dc, &pfd,);
        if format == 0 || SetPixelFormat(dc, format, &pfd,).is_err()
        {
            eprintln!("[gl] SetPixelFormat failed, skipping");
            let _ = ReleaseDC(window, dc,);
            return;
        }
        let Ok(rc,) = wglCreateContext(dc,)
        else
        {
            eprintln!("[gl] wglCreateContext failed, skipping");
            let _ = ReleaseDC(window, dc,);
            return;
        };
        if wglMakeCurrent(dc, rc,).is_err()
        {
            eprintln!("[gl] wglMakeCurrent failed, skipping");
            let _ = wglDeleteContext(rc,);
            let _ = ReleaseDC(window, dc,);
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
                glViewport(0, 0, w as i32, h as i32,);
                glClearColor(0.06, 0.07, 0.12, 1.0,);
                glClear(0x4000 | 0x100,);
                let _ = SwapBuffers(dc,);
                win.pump(1,);
                std::thread::sleep(std::time::Duration::from_millis(4,),);
            }
            let after = counter.frames();
            support::report("gl", step.name, before, after,);
            assert!(after >= before, "gl overlay frame counter went backwards on {}", step.name);
        }
        assert!(counter.frames() > 0, "gl overlay never drew a frame");

        let none: windows::Win32::Graphics::OpenGL::HGLRC = Default::default();
        let _ = wglMakeCurrent(dc, none,);
        let _ = wglDeleteContext(rc,);
        let _ = ReleaseDC(window, dc,);
    }
}
