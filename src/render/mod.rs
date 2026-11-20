pub mod mesh;
pub mod shader;

pub mod d3d9;
pub mod d3d11;
pub mod d3d12;
pub mod gl;
pub mod vk;

#[derive(Clone, Copy, PartialEq, Eq, Debug,)]
pub enum Backend {
    D3D9,

    D3D11,

    D3D11On12,
    D3D12,

    OpenGL,

    Vulkan,
}

impl Backend {
    pub unsafe fn label(self,) -> &'static str {
        match self
        {
            Backend::D3D9 => "D3D9",
            Backend::D3D11 => "D3D11",
            Backend::D3D11On12 => "D3D11on12",
            Backend::D3D12 => "D3D12",
            Backend::OpenGL => "OpenGL",
            Backend::Vulkan => "Vulkan",
        }
    }

    pub unsafe fn is_d3d11(self,) -> bool {
        matches!(self, Backend::D3D11 | Backend::D3D11On12)
    }
}
