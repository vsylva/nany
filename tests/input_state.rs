use egui::Event;
use nany::input;

mod support;

unsafe extern "system" {
    fn SendMessageW(hwnd: usize, msg: u32, wparam: usize, lparam: isize,) -> isize;
}

const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_KILLFOCUS: u32 = 0x0008;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;

unsafe fn fake_hwnd() -> nany::win::Hwnd {
    1usize as nany::win::Hwnd
}

unsafe fn guard() -> std::sync::MutexGuard<'static, (),> {
    static LOCK: std::sync::Mutex<(),> = std::sync::Mutex::new((),);
    let g = match LOCK.lock()
    {
        Ok(g,) => g,
        Err(p,) => p.into_inner(),
    };
    let mut drain = input::Input::default();
    let _ = drain.poll(fake_hwnd(), (1280, 720,), false,);
    let _ = input::hotkey(0x2D,);
    g
}

#[test]
fn hotkey_survives_inactive_frames() {
    unsafe {
        let _guard = guard();
        input::on_key(0x2D, true,);
        input::on_key(0x2D, false,);
        let mut state = input::Input::default();
        let _ = state.poll(std::ptr::null_mut(), (1280, 720,), false,);
        assert!(input::hotkey(0x2D), "hotkey was swallowed while the ui was inactive");
        assert!(!input::hotkey(0x2D), "hotkey must be consumed exactly once");
    }
}

#[test]
fn wndproc_records_generic_control_and_alt_for_modifiers() {
    unsafe {
        let _guard = guard();
        let win = support::TestWindow::create("mods", 200, 200,).expect("test window",);
        nany::hook::wndproc::install(win.hwnd,);

        SendMessageW(win.hwnd as usize, WM_KEYDOWN, VK_CONTROL as usize, 0,);
        assert!(input::held(VK_CONTROL,), "generic VK_CONTROL was never recorded from WM_KEYDOWN");
        assert!(input::modifiers().ctrl, "ctrl modifier is not visible to egui");
        SendMessageW(win.hwnd as usize, WM_KEYUP, VK_CONTROL as usize, 0,);
        assert!(!input::modifiers().ctrl, "ctrl modifier stuck after WM_KEYUP");

        SendMessageW(win.hwnd as usize, WM_KEYDOWN, VK_MENU as usize, 0,);
        assert!(input::held(VK_MENU,), "generic VK_MENU was never recorded from WM_KEYDOWN");
        assert!(input::modifiers().alt, "alt modifier is not visible to egui");
        SendMessageW(win.hwnd as usize, WM_KEYUP, VK_MENU as usize, 0,);
        assert!(!input::modifiers().alt, "alt modifier stuck after WM_KEYUP");

        nany::hook::wndproc::uninstall();
    }
}

#[test]
fn kill_focus_releases_held_keys_and_buttons() {
    unsafe {
        let _guard = guard();
        let win = support::TestWindow::create("focus", 200, 200,).expect("test window",);
        nany::hook::wndproc::install(win.hwnd,);

        SendMessageW(win.hwnd as usize, WM_KEYDOWN, 0x57, 0,);
        assert!(input::held(0x57,),);
        input::on_mouse_button(0, true,);

        SendMessageW(win.hwnd as usize, WM_KILLFOCUS, 0, 0,);
        assert!(!input::held(0x57,), "key stuck after WM_KILLFOCUS");
        assert!(!input::button_down(0,), "button stuck after WM_KILLFOCUS");

        let mut st = input::Input::default();
        let ev = st.poll(fake_hwnd(), (1280, 720,), true,);
        assert!(ev.iter().any(|e| matches!(e, Event::WindowFocused(false,))),);

        nany::hook::wndproc::uninstall();
    }
}

#[test]
fn pointer_is_scaled_from_client_to_render_size() {
    unsafe {
        let _guard = guard();
        input::on_mouse_move(640, 360,);
        nany::win::set_client_size(fake_hwnd(), 1280, 720,);
        let mut state = input::Input::default();
        let events = state.poll(fake_hwnd(), (2560, 1440,), true,);
        let moved = events.iter().find_map(|e| match e
        {
            Event::PointerMoved(p,) => Some(*p,),
            _ => None,
        },);
        let moved = moved.expect("no PointerMoved emitted",);
        assert!((moved.x - 1280.0).abs() < 0.5, "unexpected x: {}", moved.x);
        assert!((moved.y - 720.0).abs() < 0.5, "unexpected y: {}", moved.y);
    }
}

#[test]
fn ime_double_send_yields_one_text_event() {
    unsafe {
        let _guard = guard();
        input::push_char_utf16(0x662F,);
        input::push_char_utf16(0x662F,);
        let mut state = input::Input::default();
        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        let texts: Vec<_,> = events
            .iter()
            .filter_map(|e| match e
            {
                Event::Text(t,) => Some(t.clone(),),
                _ => None,
            },)
            .collect();
        assert_eq!(texts.len(), 1, "duplicate ime text: {texts:?}");
        assert_eq!(texts[0], "\u{662f}");
    }
}

#[test]
fn surrogate_pair_is_combined() {
    unsafe {
        let _guard = guard();
        input::push_char_utf16(0xD83D,);
        input::push_char_utf16(0xDE00,);
        let mut state = input::Input::default();
        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        let texts: Vec<_,> = events
            .iter()
            .filter_map(|e| match e
            {
                Event::Text(t,) => Some(t.clone(),),
                _ => None,
            },)
            .collect();
        assert_eq!(texts, vec!["\u{1F600}".to_string()]);
    }
}

#[test]
fn buttons_and_keys_reach_egui_once() {
    unsafe {
        let _guard = guard();
        input::on_mouse_move(100, 100,);
        nany::win::set_client_size(fake_hwnd(), 1280, 720,);
        input::on_mouse_button(0, true,);
        input::on_key(0x0D, true,);
        let mut state = input::Input::default();
        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        assert!(events.iter().any(|e| matches!(e, Event::PointerButton { pressed: true, .. })));
        assert!(events.iter().any(|e| matches!(e, Event::Key { key: egui::Key::Enter, pressed: true, .. })));
    }
}

#[test]
fn clipboard_events_are_forwarded() {
    unsafe {
        let _guard = guard();
        input::paste_text("hello".to_string(),);
        input::clipboard_copy();
        let mut state = input::Input::default();
        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        assert!(events.iter().any(|e| matches!(e, Event::Paste(t) if t == "hello")));
        assert!(events.iter().any(|e| matches!(e, Event::Copy)));

        input::clipboard_cut();
        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        assert!(events.iter().any(|e| matches!(e, Event::Cut)));
    }
}

#[test]
fn losing_focus_clears_pointer_and_buttons() {
    unsafe {
        let _guard = guard();
        input::on_mouse_move(10, 10,);
        input::on_mouse_button(0, true,);
        input::on_focus(false,);
        input::on_mouse_leave();
        let mut state = input::Input::default();
        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        assert!(events.iter().any(|e| matches!(e, Event::WindowFocused(false))));
        assert!(events.iter().any(|e| matches!(e, Event::PointerGone)));
        assert!(state.cursor().is_none());

        let events = state.poll(fake_hwnd(), (1280, 720,), true,);
        assert!(!events.iter().any(|e| matches!(e, Event::PointerButton { .. })), "stale button edge leaked");
    }
}
