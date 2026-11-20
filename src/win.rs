#![allow(dead_code)]
#![allow(non_snake_case)]

//! Thin semantic layer over the `windows` crate: everything that is a plain
//! Win32 call is made through the typed `windows` import directly, and only
//! state that is genuinely ours (client size cache, main window lookup, the
//! wndproc subclass) lives here.

use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::{BOOL, HANDLE, HGLOBAL, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Globalization::{MULTI_BYTE_TO_WIDE_CHAR_FLAGS, MultiByteToWideChar},
        Graphics::Gdi::{ClientToScreen, HDC, ScreenToClient, WindowFromDC},
        Security::SECURITY_ATTRIBUTES,
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
            },
            Diagnostics::Debug::ReadProcessMemory,
            LibraryLoader::{GetModuleFileNameW, GetModuleHandleExW, GetModuleHandleW},
            Memory::{GLOBAL_ALLOC_FLAGS, GlobalAlloc, GlobalLock, GlobalUnlock},
        },
        UI::{
            Input::{
                Ime::{COMPOSITIONFORM, HIMC, ImmGetContext, ImmReleaseContext, ImmSetCompositionWindow},
                KeyboardAndMouse::{
                    GetCapture, GetKeyState, ReleaseCapture, SetCapture, TRACKMOUSEEVENT, TRACKMOUSEEVENT_FLAGS,
                    TrackMouseEvent,
                },
            },
            WindowsAndMessaging::{
                CallWindowProcW, CreateWindowExW, DefWindowProcW, DestroyWindow, EnumWindows, GWLP_WNDPROC, GetClientRect,
                GetForegroundWindow, GetSystemMetrics, GetWindowTextW, GetWindowThreadProcessId, HMENU, IsWindowUnicode,
                IsWindowVisible, RegisterClassW, SYSTEM_METRICS_INDEX, UnregisterClassW, WINDOW_EX_STYLE,
                WINDOW_LONG_PTR_INDEX, WINDOW_STYLE, WNDCLASSW, WNDCLASS_STYLES, WNDPROC,
            },
        },
    },
    core::PCWSTR,
};

#[cfg(target_arch = "x86_64")]
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
#[cfg(target_arch = "x86")]
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongW;

pub type Hwnd = *mut c_void;

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

#[inline]
fn to_hwnd(h: Hwnd,) -> HWND {
    HWND(h,)
}

/// The `*Ptr` entry point only exists on 64-bit user32; 32-bit code calls the
/// `LONG` pair and windows-rs mirrors that split.
unsafe fn set_window_long(h: Hwnd, index: WINDOW_LONG_PTR_INDEX, value: isize,) -> isize {
    #[cfg(target_arch = "x86_64")]
    {
        SetWindowLongPtrW(to_hwnd(h,), index, value,)
    }
    #[cfg(target_arch = "x86")]
    {
        SetWindowLongW(to_hwnd(h,), index, value as i32,) as isize
    }
}

pub unsafe fn module_path() -> String {
    let mut buf = [0u16; 512];
    let n = GetModuleFileNameW(GetModuleHandleW(PCWSTR::null(),).unwrap_or_default(), &mut buf,);
    if n == 0
    {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize],)
}

pub unsafe fn module_of_address(address: usize,) -> String {
    const FROM_ADDRESS: u32 = 0x0000_0004;
    const UNCHANGED_REFCOUNT: u32 = 0x0000_0002;

    let mut h = HMODULE::default();
    if GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, PCWSTR(address as *const u16,), &mut h,).is_err()
    {
        return String::new();
    }
    let mut buf = [0u16; 512];
    let n = GetModuleFileNameW(h, &mut buf,);
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
    let mut r = RECT::default();
    if GetClientRect(to_hwnd(hwnd), &mut r,).is_err()
    {
        return None;
    }
    Some(((r.right - r.left).max(1,), (r.bottom - r.top).max(1,),),)
}

pub unsafe fn clipboard_text() -> Option<String,> {
    const CF_UNICODETEXT: u32 = 13;

    OpenClipboard(HWND::default(),).ok()?;
    let mut out = None;
    if IsClipboardFormatAvailable(CF_UNICODETEXT,).is_ok()
    {
        if let Ok(h,) = GetClipboardData(CF_UNICODETEXT,)
        {
            let p = GlobalLock(HGLOBAL(h.0,),) as *const u16;
            if !p.is_null()
            {
                let mut len = 0usize;
                while len < 65536 && *p.add(len,) != 0
                {
                    len += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, len,),),);
                let _ = GlobalUnlock(HGLOBAL(h.0,),);
            }
        }
    }
    let _ = CloseClipboard();
    out
}

pub unsafe fn set_clipboard_text(s: &str,) {
    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    if s.is_empty()
    {
        return;
    }
    if OpenClipboard(HWND::default(),).is_err()
    {
        return;
    }
    let _ = EmptyClipboard();
    let wide: Vec<u16,> = s.encode_utf16().chain(std::iter::once(0,),).collect();
    if let Ok(h,) = GlobalAlloc(GLOBAL_ALLOC_FLAGS(GMEM_MOVEABLE,), wide.len() * 2,)
    {
        let p = GlobalLock(h,) as *mut u16;
        if !p.is_null()
        {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len(),);
            let _ = GlobalUnlock(h,);
            let _ = SetClipboardData(CF_UNICODETEXT, HANDLE(h.0,),);
        }
    }
    let _ = CloseClipboard();
}

pub unsafe fn arm_mouse_leave(hwnd: Hwnd,) {
    const TME_LEAVE: u32 = 0x0000_0002;

    if hwnd.is_null()
    {
        return;
    }
    let mut t = TRACKMOUSEEVENT {
        cbSize:      std::mem::size_of::<TRACKMOUSEEVENT,>() as u32,
        dwFlags:     TRACKMOUSEEVENT_FLAGS(TME_LEAVE,),
        hwndTrack:   to_hwnd(hwnd,),
        dwHoverTime: 0,
    };
    let _ = TrackMouseEvent(&mut t,);
}

pub unsafe fn ansi_to_wide(code_page: u32, bytes: &[u8],) -> Option<u16,> {
    if bytes.is_empty()
    {
        return None;
    }
    let mut out = [0u16; 1];
    let n = MultiByteToWideChar(code_page, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0x0000_0400,), bytes, Some(&mut out,),);
    if n == 1 { Some(out[0],) } else { None }
}

pub unsafe fn window_pid(hwnd: Hwnd,) -> u32 {
    if hwnd.is_null()
    {
        return 0;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(to_hwnd(hwnd), Some(&mut pid,),);
    pid
}

pub unsafe fn window_title(hwnd: Hwnd,) -> String {
    let mut buf = [0u16; 256];
    let n = GetWindowTextW(to_hwnd(hwnd), &mut buf,);
    if n <= 0 { String::new() } else { String::from_utf16_lossy(&buf[..n as usize],) }
}

struct FindCtx {
    pid:  u32,
    best: Hwnd,
    area: i64,
}

unsafe extern "system" fn enum_cb(h: HWND, l: LPARAM,) -> BOOL {
    let ctx = &mut *(l.0 as *mut FindCtx);
    if window_pid(h.0,) == ctx.pid && IsWindowVisible(h,).as_bool()
    {
        if let Some((w, hh,),) = client_size(h.0,)
        {
            let a = w as i64 * hh as i64;
            if a > ctx.area
            {
                ctx.area = a;
                ctx.best = h.0;
            }
        }
    }
    BOOL(1,)
}

pub unsafe fn find_main_window() -> Hwnd {
    let mut ctx = FindCtx { pid: std::process::id(), best: std::ptr::null_mut(), area: 0, };
    let _ = EnumWindows(Some(enum_cb,), LPARAM(&mut ctx as *mut _ as isize,),);
    ctx.best
}

pub unsafe fn subclass(hwndw: Hwnd, new_proc: usize,) -> isize {
    if hwndw.is_null()
    {
        return 0;
    }
    set_window_long(hwndw, GWLP_WNDPROC, new_proc as isize,)
}

pub unsafe fn call_wnd_proc(prev: isize, hwndw: Hwnd, msg: u32, w: usize, l: isize,) -> isize {
    if prev == 0
    {
        return DefWindowProcW(to_hwnd(hwndw), msg, WPARAM(w,), LPARAM(l,),).0;
    }
    let proc: WNDPROC = Some(std::mem::transmute(prev,));
    CallWindowProcW(proc, to_hwnd(hwndw), msg, WPARAM(w,), LPARAM(l,),).0
}

pub unsafe extern "system" fn dummy_wnd_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM,) -> LRESULT {
    DefWindowProcW(h, m, w, l,)
}

pub unsafe fn create_dummy_window(class: PCWSTR, title: PCWSTR,) -> Hwnd {
    let inst = GetModuleHandleW(PCWSTR::null(),).unwrap_or_default();
    let wc = WNDCLASSW {
        style:         WNDCLASS_STYLES(0,),
        lpfnWndProc:   Some(dummy_wnd_proc,),
        hInstance:     HINSTANCE(inst.0,),
        lpszClassName: class,
        ..Default::default()
    };
    RegisterClassW(&wc,);
    CreateWindowExW(
        WINDOW_EX_STYLE(0,),
        class,
        title,
        WINDOW_STYLE(0,),
        0,
        0,
        8,
        8,
        HWND::default(),
        HMENU::default(),
        HINSTANCE(inst.0,),
        None,
    )
    .map(|h| h.0,)
    .unwrap_or(std::ptr::null_mut(),)
}

pub unsafe fn destroy_window(hwndw: Hwnd, class: PCWSTR,) {
    if hwndw.is_null()
    {
        return;
    }
    let _ = DestroyWindow(to_hwnd(hwndw),);
    let inst = GetModuleHandleW(PCWSTR::null(),).unwrap_or_default();
    let _ = UnregisterClassW(class, HINSTANCE(inst.0,),);
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
