#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::{input, log_error, log_info, overlay, win};

static ORIG_WNDPROC: AtomicUsize = AtomicUsize::new(0,);
static HOOKED_HWND: AtomicUsize = AtomicUsize::new(0,);
static STATE: AtomicBool = AtomicBool::new(false,);

const WM_MOUSEHWHEEL: u32 = 0x020E;

const WM_IME_SETCONTEXT: u32 = 0x0281;
const WM_IME_CHAR: u32 = 0x0286;
const WM_IME_COMPOSITION: u32 = 0x010F;
const WM_UNICHAR: u32 = 0x0109;
const GCS_RESULTSTR: usize = 0x0800;
const CP_ACP: u32 = 0;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_RBUTTONDBLCLK: u32 = 0x0206;
const WM_MBUTTONDBLCLK: u32 = 0x0209;
const WM_XBUTTONDBLCLK: u32 = 0x020D;

const WHEEL_DELTA: i32 = 120;
const XBUTTON1: u16 = 0x0001;

const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const VK_LSHIFT: i32 = 0xA0;
const VK_RSHIFT: i32 = 0xA1;
const VK_LCONTROL: i32 = 0xA2;
const VK_RCONTROL: i32 = 0xA3;
const VK_LMENU: i32 = 0xA4;
const VK_RMENU: i32 = 0xA5;

#[inline]
unsafe fn is_dbcs_lead(b: u8,) -> bool {
    (0x81..=0xFE).contains(&b,)
}

#[inline]
unsafe fn hiword(l: u32,) -> u16 {
    (l >> 16) as u16
}

#[inline]
unsafe fn hiwordi(l: u32,) -> i16 {
    ((l >> 16) as u16) as i16
}

#[inline]
unsafe fn loword(l: u32,) -> u16 {
    (l & 0xFFFF) as u16
}

#[inline]
unsafe fn lowordi(l: u32,) -> i16 {
    (l & 0xFFFF) as u16 as i16
}

unsafe fn swallow(msg: u32,) -> bool {
    matches!(
        msg,
        win::WM_KEYDOWN
            | win::WM_KEYUP
            | win::WM_SYSKEYDOWN
            | win::WM_SYSKEYUP
            | win::WM_CHAR
            | WM_IME_CHAR
            | WM_UNICHAR
            | win::WM_MOUSEMOVE
            | win::WM_LBUTTONDOWN
            | win::WM_LBUTTONUP
            | win::WM_RBUTTONDOWN
            | win::WM_RBUTTONUP
            | win::WM_MBUTTONDOWN
            | win::WM_MBUTTONUP
            | win::WM_MOUSEWHEEL
            | WM_MOUSEHWHEEL
            | win::WM_XBUTTONDOWN
            | win::WM_XBUTTONUP
            | win::WM_INPUT
    )
}

unsafe extern "system" fn wnd_proc(hwnd: win::Hwnd, msg: u32, w: usize, l: isize,) -> isize {
    handle(hwnd, msg, w, l,)
}

#[inline]
unsafe fn map_vkey(wparam: u32, lparam: usize,) -> i32 {
    // SHIFT/CONTROL/MENU must be recorded under their generic VK codes so
    // input::modifiers() sees them; handle_input() additionally tracks the
    // left/right variants via GetKeyState.
    let _ = lparam;
    wparam as i32
}

unsafe fn handle_input(state: u32, wparam: u32, lparam: usize,) {
    let down = state == win::WM_KEYDOWN || state == win::WM_SYSKEYDOWN;
    let vk = map_vkey(wparam, lparam,);

    input::on_key(vk, down,);

    if down && (lparam & (1 << 30)) != 0
    {
        input::on_key_repeat(vk,);
    }

    if vk == VK_SHIFT
    {
        if win::key_down(VK_LSHIFT,) == down
        {
            input::on_key(VK_LSHIFT, down,);
        }
        if win::key_down(VK_RSHIFT,) == down
        {
            input::on_key(VK_RSHIFT, down,);
        }
    }
    else if vk == VK_CONTROL
    {
        if win::key_down(VK_LCONTROL,) == down
        {
            input::on_key(VK_LCONTROL, down,);
        }
        if win::key_down(VK_RCONTROL,) == down
        {
            input::on_key(VK_RCONTROL, down,);
        }
    }
    else if vk == VK_MENU
    {
        if win::key_down(VK_LMENU,) == down
        {
            input::on_key(VK_LMENU, down,);
        }
        if win::key_down(VK_RMENU,) == down
        {
            input::on_key(VK_RMENU, down,);
        }
    }
}

unsafe fn handle(hwnd: win::Hwnd, msg: u32, w: usize, l: isize,) -> isize {
    let open = overlay::input_captured();

    if matches!(msg, win::WM_SIZE | win::WM_SHOWWINDOW | win::WM_ACTIVATE)
    {
        crate::ensure_late_hooks();
    }

    match msg
    {
        win::WM_INPUT =>
        {}

        win::WM_KEYDOWN | win::WM_SYSKEYDOWN | win::WM_KEYUP | win::WM_SYSKEYUP =>
        {
            if (w as u32) < 256
            {
                handle_input(msg, w as u32, l as usize,);
            }
        }

        win::WM_LBUTTONDOWN | WM_LBUTTONDBLCLK =>
        {
            input::on_mouse_button(0, true,);
        }
        win::WM_RBUTTONDOWN | WM_RBUTTONDBLCLK =>
        {
            input::on_mouse_button(1, true,);
        }
        win::WM_MBUTTONDOWN | WM_MBUTTONDBLCLK =>
        {
            input::on_mouse_button(2, true,);
        }
        win::WM_XBUTTONDOWN | WM_XBUTTONDBLCLK =>
        {
            let idx = if hiword(w as u32,) == XBUTTON1 { 3 } else { 4 };
            input::on_mouse_button(idx, true,);
        }
        win::WM_LBUTTONUP =>
        {
            input::on_mouse_button(0, false,);
        }
        win::WM_RBUTTONUP =>
        {
            input::on_mouse_button(1, false,);
        }
        win::WM_MBUTTONUP =>
        {
            input::on_mouse_button(2, false,);
        }
        win::WM_XBUTTONUP =>
        {
            let idx = if hiword(w as u32,) == XBUTTON1 { 3 } else { 4 };
            input::on_mouse_button(idx, false,);
        }

        win::WM_MOUSEWHEEL =>
        {
            let delta = hiwordi(w as u32,) as f32 / WHEEL_DELTA as f32;
            input::push_wheel(delta,);
        }
        WM_MOUSEHWHEEL =>
        {
            let delta = hiwordi(w as u32,) as f32 / WHEEL_DELTA as f32;
            input::push_wheel_h(delta,);
        }

        win::WM_MOUSEMOVE =>
        {
            input::on_mouse_move(lowordi(l as u32,) as i32, hiwordi(l as u32,) as i32,);
        }

        win::WM_CHAR =>
        {
            if win::is_window_unicode(hwnd,)
            {
                input::push_char_utf16(w as u16,);
            }
            else if let Some(u,) = win::ansi_to_wide(CP_ACP, &(w as u16).to_le_bytes(),)
            {
                input::push_char_utf16(u,);
            }
        }
        WM_UNICHAR =>
        {
            if let Some(c,) = char::from_u32(w as u32,)
            {
                if !c.is_control()
                {
                    input::push_char(c,);
                }
            }
            return 1;
        }

        // 中文输入法：组合结果与 IME 字符
        WM_IME_COMPOSITION =>
        {
            let r = win::call_wnd_proc(ORIG_WNDPROC.load(Ordering::Relaxed,) as isize, hwnd, msg, w, l,);
            if (l as usize & GCS_RESULTSTR) != 0
            {
                return 1;
            }
            return r;
        }
        WM_IME_CHAR =>
        {
            if win::is_window_unicode(hwnd,)
            {
                input::push_char_utf16(w as u16,);
            }
            else
            {
                let bytes = (w as u16).to_le_bytes();
                let swapped = if bytes[1] != 0 && is_dbcs_lead(bytes[1],) { [bytes[1], bytes[0],] } else { bytes };
                if let Some(u,) = win::ansi_to_wide(CP_ACP, &swapped,)
                {
                    input::push_char_utf16(u,);
                }
                return 1;
            }
        }
        WM_IME_SETCONTEXT =>
        {}

        win::WM_SIZE =>
        {
            win::set_client_size(hwnd, loword(l as u32,) as i32, hiword(l as u32,) as i32,);
        }

        _ =>
        {}
    }

    if open && swallow(msg,)
    {
        return 0;
    }

    win::call_wnd_proc(ORIG_WNDPROC.load(Ordering::Relaxed,) as isize, hwnd, msg, w, l,)
}

pub unsafe fn install(hwnd: win::Hwnd,) {
    if hwnd.is_null()
    {
        return;
    }
    if STATE.load(Ordering::Relaxed,)
    {
        if HOOKED_HWND.load(Ordering::Relaxed,) == hwnd as usize
        {
            return;
        }

        let old = HOOKED_HWND.load(Ordering::Relaxed,) as win::Hwnd;
        if !old.is_null()
        {
            win::subclass(old, ORIG_WNDPROC.load(Ordering::Relaxed,),);
        }
    }
    let prev = win::subclass(hwnd, wnd_proc as *const () as usize,);
    if prev == 0
    {
        log_error!("input", "SetWindowLongPtrW failed");
        return;
    }
    ORIG_WNDPROC.store(prev as usize, Ordering::Release,);
    HOOKED_HWND.store(hwnd as usize, Ordering::Release,);
    STATE.store(true, Ordering::Relaxed,);

    win::seed_window_state(hwnd,);
    log_info!("input", format!("wndproc installed hwnd={hwnd:p}"));
}

pub unsafe fn uninstall() {
    if !STATE.load(Ordering::Relaxed,)
    {
        return;
    }
    let hwnd = HOOKED_HWND.load(Ordering::Relaxed,) as win::Hwnd;
    let orig = ORIG_WNDPROC.load(Ordering::Relaxed,);
    if !hwnd.is_null() && orig != 0
    {
        win::subclass(hwnd, orig,);
    }
    STATE.store(false, Ordering::Relaxed,);
    HOOKED_HWND.store(0, Ordering::Relaxed,);
    ORIG_WNDPROC.store(0, Ordering::Relaxed,);
    log_info!("input", "wndproc restored");
}

#[inline]
pub unsafe fn installed_hwnd() -> Option<win::Hwnd,> {
    if !STATE.load(Ordering::Relaxed,)
    {
        return None;
    }
    let h = HOOKED_HWND.load(Ordering::Relaxed,) as win::Hwnd;
    if h.is_null() { None } else { Some(h,) }
}
