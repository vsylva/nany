//! Module and symbol lookup, straight from the `windows` crate.
//!
//! Nothing here walks the PEB, parses an export table or converts strings by
//! hand: `GetModuleHandleW`, `LoadLibraryW` and `GetProcAddress` are the typed
//! `windows` imports and the names are `w!`/`s!` literals supplied by the
//! caller.
//!
//! The lookup stays dynamic on purpose. d3d11, d3d12, dxgi, opengl32, vulkan-1
//! and d3dcompiler_47 must not become load-time dependencies of an injected
//! DLL: a game that never loads them would otherwise fail to load us, and on
//! systems without them (d3d12.dll on Windows 7) injection would not work.

use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
    },
    core::{PCSTR, PCWSTR},
};

/// Base address of an already loaded module, or 0. `PCWSTR::null()` means the
/// module of the calling process.
pub unsafe fn module_base(module: PCWSTR,) -> usize {
    match GetModuleHandleW(module,)
    {
        Ok(h,) => h.0 as usize,
        Err(_,) => 0,
    }
}

pub unsafe fn proc_address(module: usize, name: PCSTR,) -> *const c_void {
    if module == 0
    {
        return std::ptr::null();
    }
    match GetProcAddress(HMODULE(module as *mut c_void,), name,)
    {
        Some(f,) => f as *const c_void,
        None => std::ptr::null(),
    }
}

/// Address of `name` inside an **already loaded** `module`, or 0.
///
/// This deliberately does not load anything: the backend probes use it to ask
/// "is this API even present in this process?", and forcing d3d11/d3d12/dxgi in
/// would both slow injection down and build probe objects in games that never
/// touch those APIs. Callers that do want to load a library say so with
/// [`load_library`].
pub unsafe fn resolve(module: PCWSTR, name: PCSTR,) -> usize {
    proc_address(module_base(module,), name,) as usize
}

/// Cheap self check before hooking: the loader path everything else depends on
/// actually works in this process.
pub unsafe fn ready() -> bool {
    use windows::core::{s, w};

    module_base(w!("kernel32.dll"),) != 0 && resolve(w!("kernel32.dll"), s!("GetProcAddress"),) != 0
}

#[cfg(test)]
mod tests {
    use windows::core::{s, w};

    use super::*;

    #[test]
    fn finds_kernel32() {
        unsafe {
            let base = module_base(w!("kernel32.dll"),);
            assert!(base != 0, "kernel32 not in the module list");

            assert_eq!(std::ptr::read_unaligned(base as *const u16,), 0x5A4D);
        }
    }

    #[test]
    fn null_name_is_the_calling_module() {
        unsafe {
            let base = module_base(PCWSTR::null(),);
            assert!(base != 0, "GetModuleHandleW(NULL) returned nothing");
            assert_eq!(std::ptr::read_unaligned(base as *const u16,), 0x5A4D);
        }
    }

    #[test]
    fn resolves_exports() {
        unsafe {
            let gpa = resolve(w!("kernel32.dll"), s!("GetProcAddress"),);
            assert!(gpa != 0, "GetProcAddress did not resolve");
            assert_eq!(resolve(w!("kernel32.dll"), s!("nany_no_such_export"),), 0);
            assert_eq!(proc_address(0, s!("GetProcAddress"),), std::ptr::null(),);
        }
    }

    #[test]
    fn resolve_only_looks_at_loaded_modules() {
        unsafe {
            // a module that is not loaded resolves to nothing: the backend
            // probes rely on that, they must not pull d3d11/d3d12 in
            assert_eq!(resolve(w!("nany_not_a_module.dll"), s!("Nope"),), 0);
            assert_eq!(module_base(w!("nany_not_a_module.dll"),), 0);
        }
    }

    #[test]
    fn loader_path_is_ready() {
        unsafe {
            assert!(ready(), "the windows-crate loader path is not usable");
        }
    }
}
