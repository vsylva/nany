use std::ffi::{CString, c_void};

use windows::{Win32::Graphics::Direct3D::ID3DBlob, core::PCSTR};

use crate::win;

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
    *mut Option<ID3DBlob,>,
    *mut Option<ID3DBlob,>,
) -> i32;

pub unsafe fn compile(hlsl: &str, entry: &str, target: &str,) -> Result<Vec<u8,>, String,> {
    let entry_c = CString::new(entry,).map_err(|e| e.to_string(),)?;
    let target_c = CString::new(target,).map_err(|e| e.to_string(),)?;

    let lib = win::load_library_bytes(b"d3dcompiler_47.dll\0",);
    if lib == 0
    {
        return Err("d3dcompiler_47.dll not found".into(),);
    }
    let f = win::proc_address(lib, b"D3DCompile\0",);
    if f.is_null()
    {
        return Err("D3DCompile is not exported".into(),);
    }
    let f: D3DCompileFn = std::mem::transmute(f,);

    let mut blob: Option<ID3DBlob,> = None;
    let mut err: Option<ID3DBlob,> = None;
    let hr = f(
        hlsl.as_ptr(),
        hlsl.len(),
        PCSTR::null(),
        std::ptr::null(),
        std::ptr::null_mut(),
        PCSTR::from_raw(entry_c.as_ptr() as *const u8,),
        PCSTR::from_raw(target_c.as_ptr() as *const u8,),
        0,
        0,
        &mut blob,
        &mut err,
    );
    if hr < 0
    {
        if let Some(e,) = err
        {
            let n = e.GetBufferSize();
            let msg = std::slice::from_raw_parts(e.GetBufferPointer() as *const u8, n,);
            return Err(String::from_utf8_lossy(&msg[..msg.len().min(2000,)],).into_owned(),);
        }
        return Err(format!("D3DCompile hr=0x{hr:08X}"),);
    }
    let Some(blob,) = blob
    else
    {
        return Err("D3DCompile returned a null blob".into(),);
    };
    let n = blob.GetBufferSize();
    let p = blob.GetBufferPointer() as *const u8;
    Ok(std::slice::from_raw_parts(p, n,).to_vec(),)
}
