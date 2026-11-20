#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
compile_error!("nany supports Windows on x86_64 and x86");

pub mod api;
pub mod debug_panel;
pub mod hook;
pub mod input;
pub mod log;

pub mod overlay;
pub mod render;
pub mod ui;
pub mod win;

use std::time::Duration;

pub use overlay::{Frame, Provider};

pub unsafe fn init(tag: &str, provider: Box<dyn Provider,>,) -> bool {
    log::init(tag,);
    log_info!("init", format!("pid={} module={}", std::process::id(), win::module_path()));
    overlay::set_provider(provider,);
    let ok = hook::dxgi::install();
    let d3d9_ok = hook::d3d9::install();
    let gl_ok = hook::gl::install();
    let vk_ok = hook::vk::install();
    log_info!("init", format!("hooks installed = dxgi:{ok} d3d9:{d3d9_ok} gl:{gl_ok} vk:{vk_ok}"));
    overlay::ensure_wndproc();
    ok
}

pub unsafe fn install_hooks_only() -> bool {
    if !log::is_ready()
    {
        log::init("nany",);
    }
    hook::d3d9::ensure();
    hook::gl::ensure();
    hook::vk::ensure();
    hook::dxgi::ensure()
}

pub unsafe fn init_async(delay_ms: u64, provider: Box<dyn Provider,>,) {
    std::thread::spawn(move || {
        if delay_ms > 0
        {
            std::thread::sleep(Duration::from_millis(delay_ms,),);
        }
        init("nany", provider,);
    },);
}

pub unsafe fn on_dll_attach(provider: Box<dyn Provider,>,) {
    on_dll_attach_tagged("nany", provider,)
}

pub unsafe fn on_dll_attach_tagged(tag: &str, provider: Box<dyn Provider,>,) {
    overlay::set_provider(provider,);
    let tag = tag.to_string();
    std::thread::spawn(move || {
        log::init(&tag,);
        log_info!("init", format!("worker start (tag={tag})"));

        if api::try_api().is_none()
        {
            log_error!("init", "Win32 export table bootstrap failed, aborting hook install");
            return;
        }
        log_info!("init", "api table ready");
        let _ = hook::dxgi::install();
        {
            let d3d9 = hook::d3d9::install();
            log_info!("init", format!("d3d9 hooks installed = {d3d9}"));
        }
        {
            let gl = hook::gl::install();
            log_info!("init", format!("gl hooks installed = {gl}"));
        }
        {
            let vk = hook::vk::install();
            log_info!("init", format!("vk hooks installed = {vk}"));
        }

        overlay::ensure_wndproc();
        log_info!("init", "hook install finished");
    },);
}

pub unsafe fn shutdown() -> usize {
    let n = hook::uninstall_all();
    log::flush();
    n
}

pub unsafe fn ensure_late_hooks() {
    if !hook::dxgi::installed()
    {
        let _ = hook::dxgi::install();
    }
    if !hook::d3d9::installed()
    {
        let _ = hook::d3d9::install();
    }
    if !hook::gl::installed()
    {
        let _ = hook::gl::install();
    }
    if !hook::vk::installed()
    {
        let _ = hook::vk::install();
    }
}

pub unsafe fn install_hooks() -> bool {
    install_hooks_only()
}

pub unsafe fn hooks_ready() -> bool {
    hook::dxgi::installed()
}
