#![allow(unsafe_op_in_unsafe_fn)]

//! Build-time asset pipeline.
//!
//! 1. `src/render/vk_shader.wgsl` -> SPIR-V words (naga), embedded by `render`
//! 2. `src/render/shaders/*.hlsl` -> DXBC blobs, embedded by `render::shader`
//!
//! Both happen here on the build machine on purpose: the injected DLL must not
//! call `LoadLibrary` + `D3DCompile` at runtime (a game without
//! `d3dcompiler_47.dll` would lose the overlay, and dynamic compiler loads are
//! exactly the sort of thing overlay scanners look at).

use std::{ffi::c_void, path::{Path, PathBuf}};

use windows::{
    Win32::{
        Graphics::Direct3D::ID3DBlob,
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
    core::{PCSTR, s, w},
};

type D3DCompileFn = unsafe extern "system" fn(
    *const u8,
    usize,
    PCSTR,
    *const c_void,
    *mut c_void,
    PCSTR,
    PCSTR,
    u32,
    u32,
    *mut Option<ID3DBlob>,
    *mut Option<ID3DBlob>,
) -> i32;

fn main() {
    println!("cargo:rerun-if-changed=src/render/vk_shader.wgsl");
    println!("cargo:rerun-if-changed=src/render/shaders/d3d9.hlsl");
    println!("cargo:rerun-if-changed=src/render/shaders/sm5.hlsl");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    vk_spirv(&out);
    shaders(&out);
}

fn vk_spirv(out: &Path) {
    let src = match std::fs::read_to_string("src/render/vk_shader.wgsl") {
        Ok(s) => s,
        Err(e) => {
            println!("cargo:warning=读不到 vk_shader.wgsl: {e}");
            return;
        }
    };
    let module = match naga::front::wgsl::parse_str(&src) {
        Ok(m) => m,
        Err(e) => panic!("WGSL 解析失败: {e:?}"),
    };
    let info = match naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
    {
        Ok(i) => i,
        Err(e) => panic!("WGSL 校验失败: {e:?}"),
    };
    let options = naga::back::spv::Options { lang_version: (1, 0), ..Default::default() };
    let words = match naga::back::spv::write_vec(&module, &info, &options, None) {
        Ok(w) => w,
        Err(e) => panic!("SPIR-V 生成失败: {e:?}"),
    };

    let mut text = String::with_capacity(words.len() * 12);
    text.push_str("// 由 build.rs 从 vk_shader.wgsl 生成（naga → SPIR-V），不要手改\n");
    text.push_str(&format!("pub const VK_SPV_WORDS: usize = {};\n", words.len()));
    text.push_str("pub const VK_SPV: &[u32] = &[\n");
    for (i, word) in words.iter().enumerate() {
        if i % 8 == 0 {
            text.push_str("    ");
        }
        text.push_str(&format!("{word:#010x}, "));
        if i % 8 == 7 {
            text.push('\n');
        }
    }
    text.push_str("\n];\n");

    std::fs::write(out.join("vk_spv.rs"), text).expect("写 vk_spv.rs");
}

/// Compile the two HLSL pairs the D3D9 and D3D11/D3D12 painters need.
fn shaders(out: &Path) {
    let lib = unsafe { LoadLibraryW(w!("d3dcompiler_47.dll")) };
    let lib = match lib {
        Ok(h) => h,
        Err(e) => panic!("编译着色器需要 d3dcompiler_47.dll（Windows 8.1+ 自带）: {e}"),
    };
    let compile = unsafe { GetProcAddress(lib, s!("D3DCompile")) };
    let Some(compile) = compile else {
        panic!("d3dcompiler_47.dll 没有导出 D3DCompile");
    };
    let compile: D3DCompileFn = unsafe { std::mem::transmute(compile) };

    let d3d9 = std::fs::read_to_string("src/render/shaders/d3d9.hlsl").expect("d3d9.hlsl");
    let sm5 = std::fs::read_to_string("src/render/shaders/sm5.hlsl").expect("sm5.hlsl");

    // the 2.0 pair is the fallback for pre-shader-model-3 hardware only
    for (name, src, entry, target) in [
        ("d3d9_vs30.bin", &d3d9, "vs_main", "vs_3_0"),
        ("d3d9_ps30.bin", &d3d9, "ps_main", "ps_3_0"),
        ("d3d9_vs20.bin", &d3d9, "vs_main", "vs_2_0"),
        ("d3d9_ps20.bin", &d3d9, "ps_main", "ps_2_0"),
        ("sm5_vs.bin", &sm5, "vs_main", "vs_5_0"),
        ("sm5_ps.bin", &sm5, "ps_main", "ps_5_0"),
    ] {
        let bytes = unsafe { dxbc(compile, src, entry, target) }
            .unwrap_or_else(|e| panic!("编译 {target} {entry} 失败: {e}"));
        std::fs::write(out.join(name), bytes).expect("写着色器字节码");
    }
}

unsafe fn dxbc(compile: D3DCompileFn, src: &str, entry: &str, target: &str) -> Result<Vec<u8>, String> {
    let entry = std::ffi::CString::new(entry).map_err(|e| e.to_string())?;
    let target = std::ffi::CString::new(target).map_err(|e| e.to_string())?;
    let mut blob: Option<ID3DBlob> = None;
    let mut err: Option<ID3DBlob> = None;
    let hr = compile(
        src.as_ptr(),
        src.len(),
        PCSTR::null(),
        std::ptr::null(),
        std::ptr::null_mut(),
        PCSTR::from_raw(entry.as_ptr() as *const u8),
        PCSTR::from_raw(target.as_ptr() as *const u8),
        0,
        0,
        &mut blob,
        &mut err,
    );
    if hr < 0 {
        if let Some(e) = err {
            let n = e.GetBufferSize();
            let msg = std::slice::from_raw_parts(e.GetBufferPointer() as *const u8, n);
            return Err(String::from_utf8_lossy(&msg[..msg.len().min(4000)]).into_owned());
        }
        return Err(format!("D3DCompile hr=0x{hr:08X}"));
    }
    let Some(blob) = blob else {
        return Err("D3DCompile returned a null blob".into());
    };
    let n = blob.GetBufferSize();
    let p = blob.GetBufferPointer() as *const u8;
    Ok(std::slice::from_raw_parts(p, n).to_vec())
}
