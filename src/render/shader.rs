//! Shader bytecode, compiled by `build.rs` and embedded here.
//!
//! Nothing is compiled at runtime: an injected DLL that calls
//! `LoadLibrary("d3dcompiler_47.dll")` + `D3DCompile` on every painter rebuild
//! shows up in module/import scans, drags a dependency into the game that may
//! not ship that DLL, and costs a few hundred microseconds per resize. The
//! sources are `src/render/shaders/*.hlsl`; the build script turns them into
//! DXBC (plus the Vulkan SPIR-V module, which lives in `super::vk_spv`).

/// D3D9 pair, shader model 3.0 (2004+) with a 2.0 fallback for old drivers.
pub const D3D9_VS_3_0: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/d3d9_vs30.bin"));
pub const D3D9_PS_3_0: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/d3d9_ps30.bin"));
pub const D3D9_VS_2_0: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/d3d9_vs20.bin"));
pub const D3D9_PS_2_0: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/d3d9_ps20.bin"));

/// Shared D3D11 / D3D12 pair, shader model 5.
pub const SM5_VS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sm5_vs.bin"));
pub const SM5_PS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sm5_ps.bin"));

/// The D3D9 pair to use on this device.
pub fn d3d9_pair() -> Option<(&'static [u8], &'static [u8])> {
    if !D3D9_VS_3_0.is_empty() && !D3D9_PS_3_0.is_empty() {
        return Some((D3D9_VS_3_0, D3D9_PS_3_0));
    }
    if !D3D9_VS_2_0.is_empty() && !D3D9_PS_2_0.is_empty() {
        return Some((D3D9_VS_2_0, D3D9_PS_2_0));
    }
    None
}
