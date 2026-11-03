#![allow(dead_code)]

mod support;

use nany::input;

fn fake_hwnd() -> nany::win::Hwnd {
    1usize as nany::win::Hwnd
}

fn guard() -> std::sync::MutexGuard<'static, (),> {
    static LOCK: std::sync::Mutex<(),> = std::sync::Mutex::new((),);
    let g = match LOCK.lock()
    {
        Ok(g,) => g,
        Err(p,) => p.into_inner(),
    };
    unsafe {
        input::clear_buttons();
        let mut drain = input::Input::default();
        let _ = drain.poll(fake_hwnd(), (1280, 720,), false,);
    }
    g
}

#[test]
fn user_pointer_is_forwarded_to_egui() {
    let _g = guard();
    unsafe {
        nany::win::set_client_size(fake_hwnd(), 1280, 720,);
        input::on_mouse_move(320, 180,);
        assert_eq!(input::pointer(), Some((320, 180)));

        let mut st = input::Input::default();
        let events = st.poll(fake_hwnd(), (1280, 720,), true,);
        let moved = events.iter().find_map(|e| match e
        {
            egui::Event::PointerMoved(p,) => Some(*p,),
            _ => None,
        },);
        let moved = moved.expect("no PointerMoved",);
        assert!((moved.x - 320.0).abs() < 0.5 && (moved.y - 180.0).abs() < 0.5);
    }
}

#[test]
fn user_button_state_produces_one_edge_per_change() {
    let _g = guard();
    unsafe {
        input::on_mouse_button(0, true,);
        assert!(input::button_down(0));
        assert_eq!(input::button_mask(), 0b0000_0001);

        let mut st = input::Input::default();
        let presses = st
            .poll(fake_hwnd(), (1280, 720,), true,)
            .iter()
            .filter(|e| matches!(e, egui::Event::PointerButton { pressed: true, .. }),)
            .count();
        assert_eq!(presses, 1);

        input::on_mouse_button(0, true,);
        let again = st
            .poll(fake_hwnd(), (1280, 720,), true,)
            .iter()
            .filter(|e| matches!(e, egui::Event::PointerButton { .. }),)
            .count();
        assert_eq!(again, 0, "same state must not re-emit");
    }
}

#[test]
fn user_key_state_feeds_hotkey() {
    let _g = guard();
    unsafe {
        let down = nany::win::key_down(0x2D,);
        input::on_key(0x2D, down,);
        assert_eq!(input::hotkey(0x2D), down);
    }
}

#[test]
fn wheel_from_user_is_forwarded() {
    let _g = guard();
    unsafe {
        input::push_wheel(50.0,);
        input::push_wheel_h(-30.0,);
        let mut st = input::Input::default();
        let events = st.poll(fake_hwnd(), (1280, 720,), true,);
        let v: f32 = events
            .iter()
            .filter_map(|e| match e
            {
                egui::Event::MouseWheel { delta, .. } if delta.x == 0.0 => Some(delta.y,),
                _ => None,
            },)
            .sum();
        let h: f32 = events
            .iter()
            .filter_map(|e| match e
            {
                egui::Event::MouseWheel { delta, .. } if delta.y == 0.0 => Some(delta.x,),
                _ => None,
            },)
            .sum();
        assert!((v - 50.0).abs() < 0.01 && (h + 30.0).abs() < 0.01);
    }
}
