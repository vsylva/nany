#![allow(dead_code)]

use std::{ffi::c_void, sync::OnceLock};

pub type GetProcAddr = unsafe extern "system" fn(usize, *const u8,) -> *const c_void;
pub type LoadLibrary = unsafe extern "system" fn(*const u8,) -> usize;

#[derive(Clone, Copy,)]
pub struct Win32 {
    pub get_proc_address:     GetProcAddr,
    pub load_library:         LoadLibrary,
    pub get_module_handle:    unsafe extern "system" fn(*const u8,) -> usize,
    pub get_module_file_name: unsafe extern "system" fn(usize, *mut u16, u32,) -> u32,
    pub get_module_handle_ex: unsafe extern "system" fn(u32, *const u16, *mut isize,) -> i32,
    pub virtual_protect:      unsafe extern "system" fn(*const c_void, usize, u32, *mut u32,) -> i32,
    pub read_process_memory:  unsafe extern "system" fn(isize, *const c_void, *mut c_void, usize, *mut usize,) -> i32,
    pub get_current_process:  unsafe extern "system" fn() -> isize,
    pub create_event:         unsafe extern "system" fn(*const c_void, i32, i32, *const u16,) -> *mut c_void,
    pub wait_for_single:      unsafe extern "system" fn(*mut c_void, u32,) -> u32,

    pub client_to_screen: unsafe extern "system" fn(*mut c_void, *mut [i32; 2],) -> i32,

    pub open_clipboard:                unsafe extern "system" fn(*mut c_void,) -> i32,
    pub close_clipboard:               unsafe extern "system" fn() -> i32,
    pub empty_clipboard:               unsafe extern "system" fn() -> i32,
    pub is_clipboard_format_available: unsafe extern "system" fn(u32,) -> i32,
    pub get_clipboard_data:            unsafe extern "system" fn(u32,) -> *mut c_void,
    pub set_clipboard_data:            unsafe extern "system" fn(u32, *mut c_void,) -> *mut c_void,
    pub global_alloc:                  unsafe extern "system" fn(u32, usize,) -> *mut c_void,
    pub global_lock:                   unsafe extern "system" fn(*mut c_void,) -> *mut c_void,
    pub global_unlock:                 unsafe extern "system" fn(*mut c_void,) -> i32,

    pub imm_get_context:            unsafe extern "system" fn(*mut c_void,) -> *mut c_void,
    pub imm_release_context:        unsafe extern "system" fn(*mut c_void, *mut c_void,) -> i32,
    pub imm_set_composition_window: unsafe extern "system" fn(*mut c_void, *const c_void,) -> i32,
    pub screen_to_client:           unsafe extern "system" fn(*mut c_void, *mut [i32; 2],) -> i32,

    pub track_mouse_event: unsafe extern "system" fn(*const c_void,) -> i32,
    pub get_client_rect:   unsafe extern "system" fn(*mut c_void, *mut [i32; 4],) -> i32,

    pub window_from_dc:          unsafe extern "system" fn(*mut c_void,) -> *mut c_void,
    pub get_foreground_window:   unsafe extern "system" fn() -> *mut c_void,
    pub get_window_thread_pid:   unsafe extern "system" fn(*mut c_void, *mut u32,) -> u32,
    pub enum_windows: unsafe extern "system" fn(unsafe extern "system" fn(*mut c_void, isize,) -> i32, isize,) -> i32,
    pub is_window_unicode:       unsafe extern "system" fn(isize,) -> i32,
    pub multi_byte_to_wide_char: unsafe extern "system" fn(u32, u32, *const u8, i32, *mut u16, i32,) -> i32,
    pub get_key_state:           unsafe extern "system" fn(i32,) -> i16,
    pub get_capture:             unsafe extern "system" fn() -> isize,
    pub set_capture:             unsafe extern "system" fn(isize,) -> isize,
    pub release_capture:         unsafe extern "system" fn() -> i32,
    pub is_window_visible:       unsafe extern "system" fn(*mut c_void,) -> i32,
    pub get_window_text_w:       unsafe extern "system" fn(*mut c_void, *mut u16, i32,) -> i32,
    pub set_window_long_ptr:     unsafe extern "system" fn(*mut c_void, i32, isize,) -> isize,
    pub call_window_proc:        unsafe extern "system" fn(isize, *mut c_void, u32, usize, isize,) -> isize,
    pub def_window_proc:         unsafe extern "system" fn(*mut c_void, u32, usize, isize,) -> isize,
    pub register_class:          unsafe extern "system" fn(*const u8,) -> u16,
    pub unregister_class:        unsafe extern "system" fn(*const u8, usize,) -> i32,
    #[allow(clippy::too_many_arguments)]
    pub create_window_ex: unsafe extern "system" fn(
        u32,
        *const u8,
        *const u8,
        u32,
        i32,
        i32,
        i32,
        i32,
        *mut c_void,
        isize,
        usize,
        *mut c_void,
    ) -> *mut c_void,
    pub destroy_window:          unsafe extern "system" fn(*mut c_void,) -> i32,
    pub get_system_metrics:      unsafe extern "system" fn(i32,) -> i32,
}

static API: OnceLock<Option<Win32,>,> = OnceLock::new();

pub unsafe fn api() -> &'static Win32 {
    match API.get_or_init(|| build(),)
    {
        Some(w,) => w,
        None => panic!("Win32 export table bootstrap failed: kernel32 not found via PEB"),
    }
}

pub unsafe fn try_api() -> Option<&'static Win32,> {
    API.get_or_init(|| build(),).as_ref()
}

pub unsafe fn init() {
    let _ = try_api();
}

pub unsafe fn modules() -> Vec<(String, usize,),> {
    #[repr(C)]
    struct ListEntry {
        flink: *mut ListEntry,
        blink: *mut ListEntry,
    }
    #[repr(C)]
    struct UnicodeString {
        len:     u16,
        max_len: u16,
        buf:     *mut u16,
    }
    const PEB_LDR: usize = 0x18;
    const LDR_LIST: usize = 0x10;
    const ENTRY_BASE: usize = 0x30;
    const ENTRY_NAME: usize = 0x58;

    let mut out = Vec::new();
    #[cfg(target_arch = "x86_64")]
    let peb: *const u8;
    std::arch::asm!("mov {}, gs:[0x60]", out(reg) peb, options(nostack, readonly));
    if peb.is_null()
    {
        return out;
    }
    let ldr = *(peb.add(PEB_LDR,) as *const *const u8);
    if ldr.is_null()
    {
        return out;
    }
    let head = ldr.add(LDR_LIST,) as *const ListEntry;
    let mut cur = (*head).flink;
    let mut guard = 0;
    while !cur.is_null() && !std::ptr::eq(cur, head,) && guard < 4096
    {
        guard += 1;
        let e = cur as *const u8;
        let base = *(e.add(ENTRY_BASE,) as *const usize);
        let us = &*(e.add(ENTRY_NAME,) as *const UnicodeString);
        if !us.buf.is_null() && us.len > 0 && us.len <= 1024 && base != 0
        {
            let name = String::from_utf16_lossy(std::slice::from_raw_parts(us.buf, (us.len / 2) as usize,),);
            out.push((name, base,),);
        }
        cur = (*cur).flink;
    }
    out
}

pub unsafe fn module_base(name: &str,) -> usize {
    modules().into_iter().find(|(n, _,)| n.eq_ignore_ascii_case(name,),).map(|(_, b,)| b,).unwrap_or(0,)
}

unsafe fn rd_u16(base: usize, off: usize,) -> u16 {
    std::ptr::read_unaligned((base + off) as *const u16,)
}
unsafe fn rd_u32(base: usize, off: usize,) -> u32 {
    std::ptr::read_unaligned((base + off) as *const u32,)
}
unsafe fn cstr_eq(addr: usize, want: &[u8],) -> bool {
    for (i, b,) in want.iter().enumerate()
    {
        let c = *(addr as *const u8).add(i,);
        if c != *b
        {
            return false;
        }
    }
    true
}

pub unsafe fn export_addr(base: usize, name: &[u8],) -> usize {
    if base == 0 || rd_u16(base, 0,) != 0x5A4D
    {
        return 0;
    }
    let pe = rd_u32(base, 0x3C,) as usize;
    if rd_u32(base, pe,) != 0x0000_4550
    {
        return 0;
    }
    let opt = pe + 24;
    let magic = rd_u16(base, opt,);
    let dir_off = match magic
    {
        0x20B => opt + 112,
        0x10B => opt + 96,
        _ => return 0,
    };
    let exp_rva = rd_u32(base, dir_off,) as usize;
    if exp_rva == 0
    {
        return 0;
    }
    let exp = base + exp_rva;
    let num_funcs = rd_u32(exp, 0x14,) as usize;
    let num_names = rd_u32(exp, 0x18,) as usize;
    let addr_funcs = base + rd_u32(exp, 0x1C,) as usize;
    let addr_names = base + rd_u32(exp, 0x20,) as usize;
    let addr_ords = base + rd_u32(exp, 0x24,) as usize;
    if num_names > 65536 || num_funcs > 65536
    {
        return 0;
    }
    for i in 0..num_names
    {
        let name_rva = rd_u32(addr_names, i * 4,) as usize;
        if name_rva != 0 && cstr_eq(base + name_rva, name,)
        {
            let ord = rd_u16(addr_ords, i * 2,) as usize;
            if ord >= num_funcs
            {
                return 0;
            }
            return base + rd_u32(addr_funcs, ord * 4,) as usize;
        }
    }
    0
}

unsafe fn build() -> Option<Win32,> {
    let k32 = module_base("kernel32.dll",);
    let gpa_raw = export_addr(k32, b"GetProcAddress",);
    let lla_raw = export_addr(k32, b"LoadLibraryA",);
    if gpa_raw == 0 || lla_raw == 0
    {
        return None;
    }
    let gpa: GetProcAddr = std::mem::transmute(gpa_raw,);
    let lla: LoadLibrary = std::mem::transmute(lla_raw,);

    let k = |n: &[u8]| gpa(k32, n.as_ptr(),);
    let mut u32dll = module_base("user32.dll",);
    if u32dll == 0
    {
        u32dll = lla(b"user32.dll\0".as_ptr(),);
    }
    let u = |n: &[u8]| {
        if u32dll == 0 { std::ptr::null() } else { gpa(u32dll, n.as_ptr(),) }
    };

    let mut imm32dll = module_base("imm32.dll",);
    if imm32dll == 0
    {
        imm32dll = lla(b"imm32.dll\0".as_ptr(),);
    }
    let im = |n: &[u8]| {
        if imm32dll == 0 { std::ptr::null() } else { gpa(imm32dll, n.as_ptr(),) }
    };

    Some(Win32 {
        get_proc_address:     gpa,
        load_library:         lla,
        get_module_handle:    std::mem::transmute(k(b"GetModuleHandleA\0",),),
        get_module_file_name: std::mem::transmute(k(b"GetModuleFileNameW\0",),),
        get_module_handle_ex: std::mem::transmute(k(b"GetModuleHandleExW\0",),),
        virtual_protect:      std::mem::transmute(k(b"VirtualProtect\0",),),
        read_process_memory:  std::mem::transmute(k(b"ReadProcessMemory\0",),),
        get_current_process:  std::mem::transmute(k(b"GetCurrentProcess\0",),),
        create_event:         std::mem::transmute(k(b"CreateEventW\0",),),
        wait_for_single:      std::mem::transmute(k(b"WaitForSingleObject\0",),),

        client_to_screen:              std::mem::transmute(u(b"ClientToScreen\0",),),
        open_clipboard:                std::mem::transmute(u(b"OpenClipboard\0",),),
        close_clipboard:               std::mem::transmute(u(b"CloseClipboard\0",),),
        empty_clipboard:               std::mem::transmute(u(b"EmptyClipboard\0",),),
        is_clipboard_format_available: std::mem::transmute(u(b"IsClipboardFormatAvailable\0",),),
        get_clipboard_data:            std::mem::transmute(u(b"GetClipboardData\0",),),
        set_clipboard_data:            std::mem::transmute(u(b"SetClipboardData\0",),),
        global_alloc:                  std::mem::transmute(k(b"GlobalAlloc\0",),),
        global_lock:                   std::mem::transmute(k(b"GlobalLock\0",),),
        global_unlock:                 std::mem::transmute(k(b"GlobalUnlock\0",),),
        imm_get_context:               std::mem::transmute(im(b"ImmGetContext\0",),),
        imm_release_context:           std::mem::transmute(im(b"ImmReleaseContext\0",),),
        imm_set_composition_window:    std::mem::transmute(im(b"ImmSetCompositionWindow\0",),),
        screen_to_client:              std::mem::transmute(u(b"ScreenToClient\0",),),
        track_mouse_event:             std::mem::transmute(u(b"TrackMouseEvent\0",),),
        get_client_rect:               std::mem::transmute(u(b"GetClientRect\0",),),
        window_from_dc:                std::mem::transmute(u(b"WindowFromDC\0",),),
        get_foreground_window:         std::mem::transmute(u(b"GetForegroundWindow\0",),),
        get_window_thread_pid:         std::mem::transmute(u(b"GetWindowThreadProcessId\0",),),
        enum_windows:                  std::mem::transmute(u(b"EnumWindows\0",),),
        is_window_visible:             std::mem::transmute(u(b"IsWindowVisible\0",),),
        is_window_unicode:             std::mem::transmute(u(b"IsWindowUnicode\0",),),
        multi_byte_to_wide_char:       std::mem::transmute(k(b"MultiByteToWideChar\0",),),
        get_key_state:                 std::mem::transmute(u(b"GetKeyState\0",),),
        get_capture:                   std::mem::transmute(u(b"GetCapture\0",),),
        set_capture:                   std::mem::transmute(u(b"SetCapture\0",),),
        release_capture:               std::mem::transmute(u(b"ReleaseCapture\0",),),
        get_window_text_w:             std::mem::transmute(u(b"GetWindowTextW\0",),),
        set_window_long_ptr:           std::mem::transmute(u(b"SetWindowLongPtrW\0",),),
        call_window_proc:              std::mem::transmute(u(b"CallWindowProcW\0",),),
        def_window_proc:               std::mem::transmute(u(b"DefWindowProcA\0",),),
        register_class:                std::mem::transmute(u(b"RegisterClassA\0",),),
        unregister_class:              std::mem::transmute(u(b"UnregisterClassA\0",),),
        create_window_ex:              std::mem::transmute(u(b"CreateWindowExA\0",),),
        destroy_window:                std::mem::transmute(u(b"DestroyWindow\0",),),
        get_system_metrics:            std::mem::transmute(u(b"GetSystemMetrics\0",),),
    },)
}

pub unsafe fn resolve(module: &str, name: &[u8],) -> usize {
    let base = module_base(module,);
    if base == 0
    {
        return 0;
    }
    export_addr(base, name,)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_kernel32() {
        unsafe {
            let base = module_base("kernel32.dll",);
            assert!(base != 0, "kernel32 not found in PEB");

            assert_eq!(std::ptr::read_unaligned(base as *const u16), 0x5A4D);
        }
    }

    #[test]
    fn resolves_exports() {
        unsafe {
            let k = module_base("kernel32.dll",);
            assert!(k != 0);
            let gpa = export_addr(k, b"GetProcAddress",);
            assert!(gpa != 0, "GetProcAddress did not resolve");

            let pe = std::ptr::read_unaligned((k + 0x3C) as *const u32,) as usize;
            let size = std::ptr::read_unaligned((k + pe + 24 + 56) as *const u32,) as usize;
            assert!(gpa >= k && gpa < k + size, "resolved address is outside the kernel32 image");

            assert_eq!(export_addr(k, b"nany_no_such_export"), 0);
        }
    }

    #[test]
    fn module_list_sane() {
        unsafe {
            let mods = modules();
            assert!(mods.len() >= 3, "module list too short: {}", mods.len());
            let has = |n: &str| mods.iter().any(|(m, _,)| m.eq_ignore_ascii_case(n,),);
            assert!(has("kernel32.dll"));
            assert!(has("ntdll.dll"));
        }
    }

    #[test]
    fn api_table_ready() {
        unsafe {
            let a = api();
            assert!((a.get_proc_address as usize) != 0);
            assert!((a.load_library as usize) != 0);
            assert!((a.virtual_protect as usize) != 0);

            if crate::api::module_base("user32.dll",) != 0
            {}
        }
    }
}
