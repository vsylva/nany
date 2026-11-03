#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::{CStr, c_void};

use windows::core::PCSTR;

pub type Hwnd = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug,)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug,)]
pub struct Rect {
    pub left:   i32,
    pub top:    i32,
    pub right:  i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct WndClassW {
    pub style:           u32,
    pub lpfn_wnd_proc:   Option<unsafe extern "system" fn(Hwnd, u32, usize, isize,) -> isize,>,
    pub cb_cls_extra:    i32,
    pub cb_wnd_extra:    i32,
    pub h_instance:      isize,
    pub h_icon:          isize,
    pub h_cursor:        isize,
    pub hbr_background:  isize,
    pub lpsz_menu_name:  PCSTR,
    pub lpsz_class_name: PCSTR,
}

pub const GWLP_WNDPROC: i32 = -4;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;

pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_XBUTTONDOWN: u32 = 0x020B;
pub const WM_XBUTTONUP: u32 = 0x020C;
pub const WM_INPUT: u32 = 0x00FF;
pub const WM_KILLFOCUS: u32 = 0x0008;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_SHOWWINDOW: u32 = 0x0018;
pub const WM_ACTIVATE: u32 = 0x0006;

pub const VK_LBUTTON: i32 = 0x01;
pub const VK_RBUTTON: i32 = 0x02;
pub const VK_MBUTTON: i32 = 0x04;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_INSERT: i32 = 0x2D;

#[repr(C)]
pub struct SecurityAttributes {
    pub n_length:               u32,
    pub lp_security_descriptor: *mut c_void,
    pub b_inherit_handle:       i32,
}

#[inline]
unsafe fn a() -> &'static crate::api::Win32 {
    crate::api::api()
}

unsafe fn GetCurrentProcess() -> isize {
    (a().get_current_process)()
}
unsafe fn ReadProcessMemory(h: isize, addr: *const c_void, buf: *mut c_void, size: usize, read: *mut usize,) -> i32 {
    (a().read_process_memory)(h, addr, buf, size, read,)
}

unsafe fn ClientToScreen(hwnd: Hwnd, p: *mut Point,) -> i32 {
    (a().client_to_screen)(hwnd, p as *mut [i32; 2],)
}
unsafe fn GetClientRect(hwnd: Hwnd, r: *mut Rect,) -> i32 {
    (a().get_client_rect)(hwnd, r as *mut [i32; 4],)
}

pub unsafe fn window_from_dc(hdc: *mut c_void,) -> Hwnd {
    if hdc.is_null()
    {
        return std::ptr::null_mut();
    }
    (a().window_from_dc)(hdc,)
}

unsafe fn GetForegroundWindow() -> Hwnd {
    (a().get_foreground_window)()
}
unsafe fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32,) -> u32 {
    (a().get_window_thread_pid)(hwnd, pid,)
}
unsafe fn EnumWindows(cb: unsafe extern "system" fn(Hwnd, isize,) -> i32, l: isize,) -> i32 {
    (a().enum_windows)(cb, l,)
}
unsafe fn IsWindowVisible(hwnd: Hwnd,) -> i32 {
    (a().is_window_visible)(hwnd,)
}
unsafe fn GetWindowTextW(hwnd: Hwnd, buf: *mut u16, n: i32,) -> i32 {
    (a().get_window_text_w)(hwnd, buf, n,)
}

unsafe fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize,) -> isize {
    (a().set_window_long_ptr)(hwnd, index, value,)
}
unsafe fn CallWindowProcW(prev: isize, hwnd: Hwnd, msg: u32, w: usize, l: isize,) -> isize {
    (a().call_window_proc)(prev, hwnd, msg, w, l,)
}
unsafe fn DefWindowProcW(hwnd: Hwnd, msg: u32, w: usize, l: isize,) -> isize {
    (a().def_window_proc)(hwnd, msg, w, l,)
}

unsafe fn GetModuleHandleA(name: PCSTR,) -> isize {
    (a().get_module_handle)(name.as_ptr(),) as isize
}
unsafe fn GetModuleHandleW(_name: *const u16,) -> isize {
    crate::api::module_base("",) as isize
}
unsafe fn GetModuleFileNameW(h: isize, buf: *mut u16, n: u32,) -> u32 {
    (a().get_module_file_name)(h as usize, buf, n,)
}

unsafe fn LoadLibraryA(name: PCSTR,) -> isize {
    (a().load_library)(name.as_ptr(),) as isize
}
unsafe fn LoadLibraryW(name: *const u16,) -> isize {
    let mut buf = Vec::new();
    let mut i = 0;
    loop
    {
        let c = *name.add(i,);
        if c == 0
        {
            break;
        }
        buf.push(c as u8,);
        i += 1;
    }
    buf.push(0,);
    (a().load_library)(buf.as_ptr(),) as isize
}
unsafe fn GetProcAddress(m: isize, name: PCSTR,) -> *const c_void {
    (a().get_proc_address)(m as usize, name.as_ptr(),)
}

unsafe fn VirtualProtect(addr: *const c_void, size: usize, new: u32, old: *mut u32,) -> i32 {
    (a().virtual_protect)(addr, size, new, old,)
}

unsafe fn RegisterClassW(cls: *const WndClassW,) -> u16 {
    if (a().register_class as usize) == 0
    {
        return 0;
    }
    (a().register_class)(cls as *const u8,)
}
unsafe fn UnregisterClassW(name: PCSTR, inst: isize,) -> i32 {
    (a().unregister_class)(name.as_ptr(), inst as usize,)
}
unsafe fn CreateWindowExW(
    ex: u32,
    class: PCSTR,
    name: PCSTR,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    parent: Hwnd,
    menu: isize,
    inst: isize,
    param: *mut c_void,
) -> Hwnd {
    (a().create_window_ex)(ex, class.as_ptr(), name.as_ptr(), style, x, y, w, h, parent, menu, inst as usize, param,)
}
unsafe fn DestroyWindow(h: Hwnd,) -> i32 {
    (a().destroy_window)(h,)
}
unsafe fn GetSystemMetrics(index: i32,) -> i32 {
    (a().get_system_metrics)(index,)
}

pub unsafe fn CreateEventW(attr: *const c_void, manual: i32, initial: i32, name: *const u16,) -> *mut c_void {
    (a().create_event)(attr, manual, initial, name,)
}
pub unsafe fn WaitForSingleObject(h: *mut c_void, ms: u32,) -> u32 {
    (a().wait_for_single)(h, ms,)
}

pub unsafe fn module_base_bytes(name: &[u8],) -> u64 {
    GetModuleHandleA(PCSTR::from_raw(name.as_ptr(),),) as u64
}

pub unsafe fn module_base(name: &CStr,) -> u64 {
    module_base_bytes(name.to_bytes_with_nul(),)
}

pub unsafe fn module_path() -> String {
    let mut buf = [0u16; 512];
    let n = GetModuleFileNameW(GetModuleHandleW(std::ptr::null(),), buf.as_mut_ptr(), buf.len() as u32,);
    if n == 0
    {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize],)
}

pub unsafe fn module_of_address(address: usize,) -> String {
    const FROM_ADDRESS: u32 = 0x0000_0004;
    const UNCHANGED_REFCOUNT: u32 = 0x0000_0002;
    let mut h: isize = 0;
    let ok = (a().get_module_handle_ex)(FROM_ADDRESS | UNCHANGED_REFCOUNT, address as *const u16, &mut h,);
    if ok == 0 || h == 0
    {
        return String::new();
    }
    let mut buf = [0u16; 512];
    let n = GetModuleFileNameW(h, buf.as_mut_ptr(), buf.len() as u32,);
    if n == 0
    {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize],)
}

pub unsafe fn own_module_path() -> String {
    let f: unsafe fn(usize,) -> String = module_of_address;
    module_of_address(f as usize,)
}

pub unsafe fn file_stem(path: &str,) -> String {
    std::path::Path::new(path,).file_stem().map(|s| s.to_string_lossy().into_owned(),).unwrap_or_default()
}

pub unsafe fn exe_dir() -> Option<std::path::PathBuf,> {
    let p = module_path();
    if p.is_empty()
    {
        return None;
    }
    std::path::Path::new(&p,).parent().map(|d| d.to_path_buf(),)
}

pub unsafe fn load_library_bytes(name: &[u8],) -> isize {
    LoadLibraryA(PCSTR::from_raw(name.as_ptr(),),)
}

pub unsafe fn proc_address(module: isize, name: &[u8],) -> *const c_void {
    GetProcAddress(module, PCSTR::from_raw(name.as_ptr(),),)
}

pub unsafe fn read<T: Copy,>(va: u64,) -> Option<T,> {
    if va < 0x10000
    {
        return None;
    }
    let mut out = std::mem::MaybeUninit::<T,>::uninit();
    let mut got = 0usize;
    let ok = ReadProcessMemory(
        GetCurrentProcess(),
        va as *const c_void,
        out.as_mut_ptr() as *mut c_void,
        std::mem::size_of::<T,>(),
        &mut got,
    );
    if ok != 0 && got == std::mem::size_of::<T,>() { Some(out.assume_init(),) } else { None }
}

static WIN_STATE: std::sync::Mutex<Vec<(usize, i32, i32, u64,),>,> = std::sync::Mutex::new(Vec::new(),);

static WIN_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1,);

unsafe fn win_state_update(hwnd: Hwnd, f: impl FnOnce(&mut (usize, i32, i32, u64,),),) {
    if hwnd.is_null()
    {
        return;
    }
    let key = hwnd as usize;
    let tick = WIN_TICK.fetch_add(1, std::sync::atomic::Ordering::Relaxed,);
    let Ok(mut g,) = WIN_STATE.lock()
    else
    {
        return;
    };
    if let Some(slot,) = g.iter_mut().find(|s| s.0 == key,)
    {
        f(slot,);
        slot.3 = tick;
        return;
    }
    let mut slot = (key, 0, 0, tick,);
    f(&mut slot,);
    g.push(slot,);
}

pub unsafe fn set_client_size(hwnd: Hwnd, w: i32, h: i32,) {
    win_state_update(hwnd, |s| {
        s.1 = w.max(1,);
        s.2 = h.max(1,);
    },);
}

pub unsafe fn client_size_cached(hwnd: Hwnd,) -> Option<(i32, i32,),> {
    let key = hwnd as usize;
    let g = WIN_STATE.lock().ok()?;
    g.iter().find(|s| s.0 == key && s.1 > 0 && s.2 > 0,).map(|s| (s.1, s.2,),)
}

pub unsafe fn seed_window_state(hwnd: Hwnd,) {
    if hwnd.is_null()
    {
        return;
    }
    if let Some((w, h,),) = client_size(hwnd,)
    {
        set_client_size(hwnd, w, h,);
    }
}

pub unsafe fn client_size(hwnd: Hwnd,) -> Option<(i32, i32,),> {
    if hwnd.is_null()
    {
        return None;
    }
    let mut r = Rect::default();
    if GetClientRect(hwnd, &mut r,) == 0
    {
        return None;
    }
    Some(((r.right - r.left).max(1,), (r.bottom - r.top).max(1,),),)
}

pub unsafe fn client_to_screen(hwnd: Hwnd, x: i32, y: i32,) -> Option<(i32, i32,),> {
    let mut p = Point { x, y, };
    let ok = ClientToScreen(hwnd, &mut p,);
    if ok == 0 { None } else { Some((p.x, p.y,),) }
}

pub unsafe fn clipboard_text() -> Option<String,> {
    const CF_UNICODETEXT: u32 = 13;
    let a = a();
    if (a.open_clipboard)(std::ptr::null_mut(),) == 0
    {
        return None;
    }
    let mut out = None;
    if (a.is_clipboard_format_available)(CF_UNICODETEXT,) != 0
    {
        let h = (a.get_clipboard_data)(CF_UNICODETEXT,);
        if !h.is_null()
        {
            let p = (a.global_lock)(h,) as *const u16;
            if !p.is_null()
            {
                let mut len = 0usize;
                while len < 65536 && *p.add(len,) != 0
                {
                    len += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, len,),),);
                (a.global_unlock)(h,);
            }
        }
    }
    (a.close_clipboard)();
    out
}

pub unsafe fn set_clipboard_text(s: &str,) {
    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;
    if s.is_empty()
    {
        return;
    }
    let a = a();
    if (a.open_clipboard)(std::ptr::null_mut(),) == 0
    {
        return;
    }
    (a.empty_clipboard)();
    let wide: Vec<u16,> = s.encode_utf16().chain(std::iter::once(0,),).collect();
    let h = (a.global_alloc)(GMEM_MOVEABLE, wide.len() * 2,);
    if !h.is_null()
    {
        let p = (a.global_lock)(h,) as *mut u16;
        if !p.is_null()
        {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len(),);
            (a.global_unlock)(h,);
            (a.set_clipboard_data)(CF_UNICODETEXT, h,);
        }
    }
    (a.close_clipboard)();
}

pub unsafe fn ime_move_candidate(hwnd: Hwnd, x: i32, y: i32,) {
    #[repr(C)]
    struct CompositionForm {
        style:      u32,
        pt_current: [i32; 2],
        rc_area:    [i32; 4],
    }
    const CFS_POINT: u32 = 0x0002;
    let a = a();
    if (a.imm_get_context as usize) == 0 || hwnd.is_null()
    {
        return;
    }
    let Some((sx, sy,),) = client_to_screen(hwnd, x, y,)
    else
    {
        return;
    };
    let himc = (a.imm_get_context)(hwnd,);
    if himc.is_null()
    {
        return;
    }
    let cf = CompositionForm { style: CFS_POINT, pt_current: [sx, sy,], rc_area: [0; 4], };
    if (a.imm_set_composition_window as usize) != 0
    {
        (a.imm_set_composition_window)(himc, &cf as *const _ as *const c_void,);
    }
    (a.imm_release_context)(hwnd, himc,);
}

pub unsafe fn screen_to_client_xy(hwnd: Hwnd, x: i32, y: i32,) -> Option<(i32, i32,),> {
    let a = a();
    let f = a.screen_to_client;
    if (f as usize) == 0 || hwnd.is_null()
    {
        return None;
    }
    let mut p = [x, y,];
    let ok = f(hwnd, &mut p,);
    if ok == 0 { None } else { Some((p[0], p[1],),) }
}

pub unsafe fn arm_mouse_leave(hwnd: Hwnd,) {
    #[repr(C)]
    struct TrackMouseEventIn {
        cb_size:    u32,
        flags:      u32,
        hwnd_track: Hwnd,
        hover_time: u32,
    }
    const TME_LEAVE: u32 = 0x0000_0002;
    let a = a();
    let f = a.track_mouse_event;
    if (f as usize) == 0 || hwnd.is_null()
    {
        return;
    }
    let t = TrackMouseEventIn {
        cb_size:    std::mem::size_of::<TrackMouseEventIn,>() as u32,
        flags:      TME_LEAVE,
        hwnd_track: hwnd,
        hover_time: 0,
    };
    f(&t as *const _ as *const c_void,);
}

pub unsafe fn arm_mouse_track(hwnd: Hwnd, non_client: bool,) {
    #[repr(C)]
    struct TrackMouseEventIn {
        cb_size:    u32,
        flags:      u32,
        hwnd_track: Hwnd,
        hover_time: u32,
    }
    const TME_LEAVE: u32 = 0x0000_0002;
    const TME_NONCLIENT: u32 = 0x0000_0010;
    let a = a();
    let f = a.track_mouse_event;
    if (f as usize) == 0 || hwnd.is_null()
    {
        return;
    }
    let mut flags = TME_LEAVE;
    if non_client
    {
        flags |= TME_NONCLIENT;
    }
    let t = TrackMouseEventIn {
        cb_size: std::mem::size_of::<TrackMouseEventIn,>() as u32,
        flags,
        hwnd_track: hwnd,
        hover_time: 0,
    };
    f(&t as *const _ as *const c_void,);
}

pub unsafe fn cancel_mouse_track(hwnd: Hwnd,) {
    #[repr(C)]
    struct TrackMouseEventIn {
        cb_size:    u32,
        flags:      u32,
        hwnd_track: Hwnd,
        hover_time: u32,
    }
    const TME_CANCEL: u32 = 0x8000_0000;
    let a = a();
    let f = a.track_mouse_event;
    if (f as usize) == 0 || hwnd.is_null()
    {
        return;
    }
    let t = TrackMouseEventIn {
        cb_size:    std::mem::size_of::<TrackMouseEventIn,>() as u32,
        flags:      TME_CANCEL,
        hwnd_track: hwnd,
        hover_time: 0,
    };
    f(&t as *const _ as *const c_void,);
}

pub unsafe fn is_window_unicode(hwnd: Hwnd,) -> bool {
    let a = a();
    let f = a.is_window_unicode;
    (f as usize) != 0 && !hwnd.is_null() && f(hwnd as isize,) != 0
}

pub unsafe fn ansi_to_wide(code_page: u32, bytes: &[u8],) -> Option<u16,> {
    let a = a();
    let f = a.multi_byte_to_wide_char;
    if (f as usize) == 0 || bytes.is_empty()
    {
        return None;
    }
    let mut out = [0u16; 1];
    let n = f(code_page, 0x0000_0400, bytes.as_ptr(), bytes.len() as i32, out.as_mut_ptr(), 1,);
    if n == 1 { Some(out[0],) } else { None }
}

pub unsafe fn key_down(vk: i32,) -> bool {
    let a = a();
    let f = a.get_key_state;
    (f as usize) != 0 && (f(vk,) as u16 & 0x8000) != 0
}

pub unsafe fn capture_window() -> Hwnd {
    let a = a();
    let f = a.get_capture;
    if (f as usize) == 0
    {
        return std::ptr::null_mut();
    }
    f() as Hwnd
}

pub unsafe fn set_capture(hwnd: Hwnd,) -> Hwnd {
    let a = a();
    let f = a.set_capture;
    if (f as usize) == 0 || hwnd.is_null()
    {
        return std::ptr::null_mut();
    }
    f(hwnd as isize,) as Hwnd
}

pub unsafe fn release_capture() {
    let a = a();
    let f = a.release_capture;
    if (f as usize) != 0
    {
        f();
    }
}

pub unsafe fn foreground() -> Hwnd {
    GetForegroundWindow()
}

pub unsafe fn window_pid(hwnd: Hwnd,) -> u32 {
    let mut pid = 0u32;
    if hwnd.is_null()
    {
        return 0;
    }
    GetWindowThreadProcessId(hwnd, &mut pid,);
    pid
}

pub unsafe fn window_title(hwnd: Hwnd,) -> String {
    let mut buf = [0u16; 256];
    let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32,);
    if n <= 0 { String::new() } else { String::from_utf16_lossy(&buf[..n as usize],) }
}

struct FindCtx {
    pid:  u32,
    best: Hwnd,
    area: i64,
}

unsafe extern "system" fn enum_cb(h: Hwnd, l: isize,) -> i32 {
    let ctx = &mut *(l as *mut FindCtx);
    if window_pid(h,) == ctx.pid && IsWindowVisible(h,) != 0
    {
        if let Some((w, hh,),) = client_size(h,)
        {
            let a = w as i64 * hh as i64;
            if a > ctx.area
            {
                ctx.area = a;
                ctx.best = h;
            }
        }
    }
    1
}

pub unsafe fn find_main_window() -> Hwnd {
    let mut ctx = FindCtx { pid: std::process::id(), best: std::ptr::null_mut(), area: 0, };
    EnumWindows(enum_cb, &mut ctx as *mut _ as isize,);
    ctx.best
}

pub unsafe fn subclass(hwnd: Hwnd, new_proc: usize,) -> isize {
    if hwnd.is_null()
    {
        return 0;
    }
    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, new_proc as isize,)
}

pub unsafe fn call_wnd_proc(prev: isize, hwnd: Hwnd, msg: u32, w: usize, l: isize,) -> isize {
    if prev == 0 { DefWindowProcW(hwnd, msg, w, l,) } else { CallWindowProcW(prev, hwnd, msg, w, l,) }
}

pub unsafe extern "system" fn dummy_wnd_proc(h: Hwnd, m: u32, w: usize, l: isize,) -> isize {
    DefWindowProcW(h, m, w, l,)
}

pub unsafe fn create_dummy_window(class: &CStr, title: &CStr,) -> Hwnd {
    let inst = GetModuleHandleA(PCSTR::null(),);
    let wc = WndClassW {
        style:           0,
        lpfn_wnd_proc:   Some(dummy_wnd_proc,),
        cb_cls_extra:    0,
        cb_wnd_extra:    0,
        h_instance:      inst,
        h_icon:          0,
        h_cursor:        0,
        hbr_background:  0,
        lpsz_menu_name:  PCSTR::null(),
        lpsz_class_name: PCSTR::from_raw(class.as_ptr() as *const u8,),
    };
    RegisterClassW(&wc,);
    CreateWindowExW(
        0,
        PCSTR::from_raw(class.as_ptr() as *const u8,),
        PCSTR::from_raw(title.as_ptr() as *const u8,),
        0,
        0,
        0,
        8,
        8,
        std::ptr::null_mut(),
        0,
        inst,
        std::ptr::null_mut(),
    )
}

pub unsafe fn destroy_window(hwnd: Hwnd, class: &CStr,) {
    if hwnd.is_null()
    {
        return;
    }
    DestroyWindow(hwnd,);
    UnregisterClassW(PCSTR::from_raw(class.as_ptr() as *const u8,), GetModuleHandleA(PCSTR::null(),),);
}

#[cfg(test)]
mod clipboard_tests {

    #[test]
    fn clipboard_roundtrip() {
        unsafe {
            let s = "nany-clip-test-42";
            super::set_clipboard_text(s,);
            let got = super::clipboard_text();
            assert_eq!(got.as_deref(), Some(s), "clipboard roundtrip failed: {got:?}");
        }
    }
}
