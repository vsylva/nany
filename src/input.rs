#![allow(dead_code)]

use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicI8, AtomicU64, Ordering},
    },
};

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2};

use crate::win::{self, Hwnd};

struct St {
    chars: VecDeque<char,>,

    last_char: Option<(char, u128,),>,

    pending_high: u32,

    wheel:   f32,
    wheel_h: f32,

    hotkeys: Vec<i32,>,

    mouse_pos: Option<(i32, i32,),>,

    mouse_down: [bool; 5],

    mouse_edges: Vec<(u8, bool,),>,

    key_edges: Vec<(u32, bool, bool,),>,

    paste: Option<String,>,
}

impl St {
    const fn new() -> Self {
        Self {
            chars:        VecDeque::new(),
            last_char:    None,
            pending_high: 0,
            wheel:        0.0,
            wheel_h:      0.0,
            hotkeys:      Vec::new(),
            mouse_pos:    None,
            mouse_down:   [false; 5],
            mouse_edges:  Vec::new(),
            key_edges:    Vec::new(),
            paste:        None,
        }
    }
}

static ST: Mutex<St,> = Mutex::new(St::new(),);

unsafe fn with<R,>(f: impl FnOnce(&mut St,) -> R,) -> R {
    let mut g = match ST.lock()
    {
        Ok(g,) => g,
        Err(p,) => p.into_inner(),
    };
    f(&mut g,)
}

static HELD_BITS: [AtomicU64; 4] = [AtomicU64::new(0,), AtomicU64::new(0,), AtomicU64::new(0,), AtomicU64::new(0,),];

static POINTER_LEFT: AtomicBool = AtomicBool::new(false,);

static FOCUS_EVENT: AtomicI8 = AtomicI8::new(0,);

static CLIP_OP: AtomicI8 = AtomicI8::new(0,);

const CHAR_DEDUPE_MS: u128 = 12;

unsafe fn held_set(vk: i32, down: bool,) {
    if !(0..256).contains(&vk,)
    {
        return;
    }
    let (word, bit,) = (vk as usize / 64, 1u64 << (vk as usize % 64),);
    let a = &HELD_BITS[word];
    if down
    {
        a.fetch_or(bit, Ordering::AcqRel,);
    }
    else
    {
        a.fetch_and(!bit, Ordering::AcqRel,);
    }
}

unsafe fn held_get(vk: i32,) -> bool {
    if !(0..256).contains(&vk,)
    {
        return false;
    }
    let (word, bit,) = (vk as usize / 64, 1u64 << (vk as usize % 64),);
    HELD_BITS[word].load(Ordering::Acquire,) & bit != 0
}

pub unsafe fn on_key(vk: i32, down: bool,) {
    let was = held_get(vk,);
    held_set(vk, down,);
    with(|s| {
        if down && !was && s.hotkeys.len() < 32
        {
            s.hotkeys.push(vk,);
        }
        if s.key_edges.len() < 128
        {
            s.key_edges.push((vk as u32, down, false,),);
        }
    },);
}

pub unsafe fn on_key_repeat(vk: i32,) {
    with(|s| {
        if s.key_edges.len() < 128
        {
            s.key_edges.push((vk as u32, true, true,),);
        }
    },);
}

pub unsafe fn hotkey(vk: i32,) -> bool {
    with(|s| match s.hotkeys.iter().position(|k| *k == vk,)
    {
        Some(pos,) =>
        {
            s.hotkeys.remove(pos,);
            true
        }
        None => false,
    },)
}

pub unsafe fn held(vk: i32,) -> bool {
    held_get(vk,)
}

pub unsafe fn modifiers() -> Modifiers {
    Modifiers {
        alt:     held(win::VK_MENU,),
        ctrl:    held(win::VK_CONTROL,),
        shift:   held(win::VK_SHIFT,),
        mac_cmd: false,
        command: held(win::VK_CONTROL,),
    }
}

pub unsafe fn on_mouse_leave() {
    POINTER_LEFT.store(true, Ordering::Relaxed,);
    with(|s| s.mouse_pos = None,);
}

pub unsafe fn on_focus(focused: bool,) {
    FOCUS_EVENT.store(if focused { 1 } else { 2 }, Ordering::Relaxed,);
}

pub unsafe fn on_mouse_move(x: i32, y: i32,) {
    with(|s| s.mouse_pos = Some((x, y,),),);
}

pub unsafe fn on_mouse_button(idx: u8, down: bool,) {
    if idx > 4
    {
        return;
    }
    with(|s| {
        if s.mouse_down[idx as usize] == down
        {
            return;
        }
        s.mouse_down[idx as usize] = down;
        if s.mouse_edges.len() < 64
        {
            s.mouse_edges.push((idx, down,),);
        }
    },);
}

pub unsafe fn clear_buttons() {
    with(|s| {
        s.mouse_down = [false; 5];
        s.mouse_edges.clear();
    },);
}

pub unsafe fn button_down(idx: u8,) -> bool {
    if idx > 4
    {
        return false;
    }
    with(|s| s.mouse_down[idx as usize],)
}

pub unsafe fn button_mask() -> u8 {
    with(|s| {
        let mut m = 0u8;
        let mut i = 0usize;
        while i < 5
        {
            if s.mouse_down[i]
            {
                m |= 1 << i;
            }
            i += 1;
        }
        m
    },)
}

pub unsafe fn pointer() -> Option<(i32, i32,),> {
    with(|s| s.mouse_pos,)
}

pub unsafe fn push_wheel_h(delta: f32,) {
    with(|s| s.wheel_h += delta,);
}

pub unsafe fn push_wheel(delta: f32,) {
    with(|s| s.wheel += delta,);
}

pub unsafe fn push_char_utf16(unit: u16,) {
    if (0xD800..0xDC00).contains(&unit,)
    {
        with(|s| s.pending_high = unit as u32,);
        return;
    }

    if (0xDC00..0xE000).contains(&unit,)
    {
        let hi = with(|s| std::mem::replace(&mut s.pending_high, 0,),);
        if hi != 0
        {
            let cp = 0x1_0000 + ((hi - 0xD800) << 10) + (unit as u32 - 0xDC00);
            if let Some(c,) = char::from_u32(cp,)
            {
                push_char(c,);
            }
        }
        return;
    }
    with(|s| s.pending_high = 0,);
    if let Some(c,) = char::from_u32(unit as u32,)
    {
        if !c.is_control()
        {
            push_char(c,);
        }
    }
}

pub unsafe fn push_char(c: char,) {
    let now = crate::overlay::now_ms();
    with(|s| {
        if let Some((prev, t,),) = s.last_char
        {
            if prev == c && now.saturating_sub(t,) < CHAR_DEDUPE_MS
            {
                return;
            }
        }
        s.last_char = Some((c, now,),);
        if s.chars.len() < 128
        {
            s.chars.push_back(c,);
        }
    },);
}

unsafe fn drain_chars() -> Vec<char,> {
    with(|s| s.chars.drain(..,).collect(),)
}

pub unsafe fn paste_text(s: String,) {
    with(|st| st.paste = Some(s,),);
}

pub unsafe fn clipboard_copy() {
    CLIP_OP.store(1, Ordering::Relaxed,);
}

pub unsafe fn clipboard_cut() {
    CLIP_OP.store(2, Ordering::Relaxed,);
}

unsafe fn vk_to_key(vk: u32,) -> Option<Key,> {
    Some(match vk
    {
        0x08 => Key::Backspace,
        0x09 => Key::Tab,
        0x0D => Key::Enter,
        0x1B => Key::Escape,
        0x20 => Key::Space,
        0x21 => Key::PageUp,
        0x22 => Key::PageDown,
        0x23 => Key::End,
        0x24 => Key::Home,
        0x25 => Key::ArrowLeft,
        0x26 => Key::ArrowUp,
        0x27 => Key::ArrowRight,
        0x28 => Key::ArrowDown,
        0x2E => Key::Delete,
        _ => return None,
    },)
}

#[derive(Default,)]
pub struct Input {
    cursor: Option<Pos2,>,
}

#[derive(Default,)]
struct Drained {
    mouse_pos:   Option<(i32, i32,),>,
    mouse_edges: Vec<(u8, bool,),>,
    key_edges:   Vec<(u32, bool, bool,),>,
    chars:       Vec<char,>,
    wheel:       f32,
    wheel_h:     f32,
    paste:       Option<String,>,
}

unsafe fn drain(keep_accumulators: bool,) -> Drained {
    with(|s| {
        let d = Drained {
            mouse_pos:   s.mouse_pos,
            mouse_edges: std::mem::take(&mut s.mouse_edges,),
            key_edges:   std::mem::take(&mut s.key_edges,),
            chars:       s.chars.drain(..,).collect(),
            wheel:       std::mem::replace(&mut s.wheel, 0.0,),
            wheel_h:     std::mem::replace(&mut s.wheel_h, 0.0,),
            paste:       s.paste.take(),
        };
        if !keep_accumulators
        {
            s.mouse_pos = None;
            s.mouse_down = [false; 5];
        }
        d
    },)
}

impl Input {
    pub unsafe fn poll(&mut self, hwnd: Hwnd, screen: (u32, u32,), active: bool,) -> Vec<Event,> {
        let mut ev: Vec<Event,> = Vec::new();

        if !active
        {
            // only announce the pointer loss once, on the transition; a
            // PointerGone every frame would force a full egui pass per frame
            let had_cursor = self.cursor.is_some();
            self.cursor = None;
            let _ = drain(false,);
            if had_cursor
            {
                ev.push(Event::PointerGone,);
            }
            return ev;
        }

        let size = win::client_size_cached(hwnd,).or_else(|| {
            let s = win::client_size(hwnd,);
            if let Some((w, h,),) = s
            {
                win::set_client_size(hwnd, w, h,);
            }
            s
        },);

        let d = drain(true,);

        if let Some(t,) = d.paste
        {
            ev.push(Event::Paste(t,),);
        }
        match CLIP_OP.swap(0, Ordering::Relaxed,)
        {
            1 => ev.push(Event::Copy,),
            2 => ev.push(Event::Cut,),
            _ =>
            {}
        }

        if POINTER_LEFT.swap(false, Ordering::Relaxed,)
        {
            self.cursor = None;
            ev.push(Event::PointerGone,);
        }

        match FOCUS_EVENT.swap(0, Ordering::Relaxed,)
        {
            1 => ev.push(Event::WindowFocused(true,),),
            2 => ev.push(Event::WindowFocused(false,),),
            _ =>
            {}
        }

        if let (Some((cx, cy,),), Some((cw, ch,),),) = (d.mouse_pos, size,)
        {
            let sx = screen.0 as f32 / cw as f32;
            let sy = screen.1 as f32 / ch as f32;
            let pos = egui::pos2(cx as f32 * sx, cy as f32 * sy,);
            if self.cursor != Some(pos,)
            {
                self.cursor = Some(pos,);
                ev.push(Event::PointerMoved(pos,),);
            }
        }

        let mods = modifiers();

        for (idx, down,) in d.mouse_edges
        {
            let button = match idx
            {
                0 => PointerButton::Primary,
                1 => PointerButton::Secondary,
                2 => PointerButton::Middle,
                3 => PointerButton::Extra1,
                _ => PointerButton::Extra2,
            };
            let pos = self.cursor.unwrap_or(egui::Pos2::ZERO,);
            ev.push(Event::PointerButton { pos, button, pressed: down, modifiers: mods, },);
        }

        if d.wheel.abs() > f32::EPSILON
        {
            ev.push(Event::MouseWheel {
                unit:      egui::MouseWheelUnit::Point,
                delta:     Vec2::new(0.0, d.wheel,),
                phase:     egui::TouchPhase::Move,
                source:    egui::MouseWheelSource::Unknown,
                modifiers: mods,
            },);
        }

        if d.wheel_h.abs() > f32::EPSILON
        {
            ev.push(Event::MouseWheel {
                unit:      egui::MouseWheelUnit::Point,
                delta:     Vec2::new(d.wheel_h, 0.0,),
                phase:     egui::TouchPhase::Move,
                source:    egui::MouseWheelSource::Unknown,
                modifiers: mods,
            },);
        }

        for (vk, down, repeat,) in d.key_edges
        {
            if let Some(key,) = vk_to_key(vk,)
            {
                ev.push(Event::Key { key, physical_key: None, pressed: down, repeat, modifiers: mods, },);
            }
        }

        for c in d.chars
        {
            if !c.is_control()
            {
                ev.push(Event::Text(c.to_string(),),);
            }
        }

        ev
    }

    pub unsafe fn cursor(&self,) -> Option<Pos2,> {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::{CHAR_DEDUPE_MS, drain_chars, push_char};

    unsafe fn reset() -> std::sync::MutexGuard<'static, (),> {
        static L: std::sync::Mutex<(),> = std::sync::Mutex::new((),);
        let g = match L.lock()
        {
            Ok(g,) => g,
            Err(p,) => p.into_inner(),
        };
        std::thread::sleep(std::time::Duration::from_millis(CHAR_DEDUPE_MS as u64 + 5,),);
        let _ = drain_chars();
        g
    }

    #[test]
    fn ime_double_send_yields_one_char() {
        unsafe {
            let _g = reset();
            push_char('\u{662f}',);
            push_char('\u{662f}',);
            assert_eq!(drain_chars(), vec!['\u{662f}']);
        }
    }

    #[test]
    fn same_char_after_window_is_kept() {
        unsafe {
            let _g = reset();
            push_char('\u{662f}',);
            std::thread::sleep(std::time::Duration::from_millis(CHAR_DEDUPE_MS as u64 + 5,),);
            push_char('\u{662f}',);
            assert_eq!(drain_chars(), vec!['\u{662f}', '\u{662f}']);
        }
    }

    #[test]
    fn key_bitmap_tracks_state() {
        unsafe {
            let _g = reset();
            super::on_key(0x41, true,);
            assert!(super::held(0x41));
            super::on_key(0x41, false,);
            assert!(!super::held(0x41));
        }
    }

    #[test]
    fn hotkey_survives_inactive_poll() {
        unsafe {
            let _g = reset();
            let _ = super::hotkey(0x2D,);
            super::on_key(0x2D, true,);
            super::on_key(0x2D, false,);

            let mut input = super::Input::default();
            let _ = input.poll(std::ptr::null_mut(), (100, 100,), false,);
            assert!(super::hotkey(0x2D), "hotkey was swallowed after the menu closed");
        }
    }

    #[test]
    fn hotkey_consumes_once() {
        unsafe {
            let _g = reset();
            let _ = super::hotkey(0x42,);
            super::on_key(0x42, true,);
            super::on_key(0x42, false,);
            assert!(super::hotkey(0x42));
            assert!(!super::hotkey(0x42));
        }
    }
}
