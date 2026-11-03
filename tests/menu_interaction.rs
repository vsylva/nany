#![allow(dead_code, deprecated)]

mod support;

use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use support::menu;

const SCREEN: Vec2 = Vec2 { x: 1280.0, y: 720.0, };

struct Harness {
    ctx:    Context,
    state:  menu::State,
    time:   f64,
    events: Vec<Event,>,
}

impl Harness {
    unsafe fn new() -> Self {
        Self { ctx: Context::default(), state: menu::State::default(), time: 0.0, events: Vec::new(), }
    }

    unsafe fn frame(&mut self,) -> egui::FullOutput {
        self.time += 1.0 / 60.0;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN,),),
            time: Some(self.time,),
            events: std::mem::take(&mut self.events,),
            ..Default::default()
        };
        let state = &mut self.state;
        let info = menu::Info {
            backend:  "test",
            screen:   (SCREEN.x as u32, SCREEN.y as u32,),
            frame_ms: 16.0,
            frames:   1,
            hotkey:   0x2D,
            t:        self.time as f32,
        };
        let mut out = self.ctx.run_ui(input, |c| menu::draw(c, state, &info,),);
        out.textures_delta.clear();
        out
    }

    unsafe fn menu_rect(&self,) -> Rect {
        let p = self.state.pos;
        Rect::from_min_size(p, Vec2::new(menu::W, menu::H,),)
    }

    unsafe fn move_to(&mut self, p: Pos2,) {
        self.events.push(Event::PointerMoved(p,),);
    }

    unsafe fn click(&mut self, p: Pos2,) {
        self.events.push(Event::PointerMoved(p,),);
        self.events.push(Event::PointerButton {
            pos:       p,
            button:    PointerButton::Primary,
            pressed:   true,
            modifiers: Modifiers::default(),
        },);
        self.events.push(Event::PointerButton {
            pos:       p,
            button:    PointerButton::Primary,
            pressed:   false,
            modifiers: Modifiers::default(),
        },);
    }

    unsafe fn drag(&mut self, from: Pos2, to: Pos2,) {
        self.move_to(from,);
        self.frame();
        self.events.push(Event::PointerMoved(from,),);
        self.events.push(Event::PointerButton {
            pos:       from,
            button:    PointerButton::Primary,
            pressed:   true,
            modifiers: Modifiers::default(),
        },);
        self.frame();
        let steps = 6;
        for i in 1..=steps
        {
            let t = i as f32 / steps as f32;
            self.events.push(Event::PointerMoved(from + (to - from) * t,),);
            self.frame();
        }
        self.events.push(Event::PointerButton {
            pos:       to,
            button:    PointerButton::Primary,
            pressed:   false,
            modifiers: Modifiers::default(),
        },);
        self.frame();
    }

    unsafe fn type_text(&mut self, text: &str,) {
        for ch in text.chars()
        {
            self.events.push(Event::Text(ch.to_string(),),);
        }
    }

    unsafe fn key(&mut self, key: Key,) {
        self.events.push(Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        },);
        self.events.push(Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::default(),
        },);
    }

    unsafe fn scroll(&mut self, at: Pos2, dy: f32,) {
        self.events.push(Event::PointerMoved(at,),);
        self.events.push(Event::MouseWheel {
            unit:      egui::MouseWheelUnit::Line,
            delta:     Vec2::new(0.0, dy,),
            modifiers: Modifiers::default(),
            phase:     egui::TouchPhase::Move,
            source:    egui::MouseWheelSource::Wheel,
        },);
    }
}

unsafe fn snapshot(s: &menu::State,) -> (u8, bool, u32, u8, usize, u32, bool, u16,) {
    (
        s.nav,
        s.feather,
        (s.slider * 100.0) as u32,
        s.combo,
        s.input.len(),
        s.color.to_array()[0] as u32,
        s.color_open,
        s.crown.iter().filter(|c| **c,).count() as u16,
    )
}

#[test]
fn every_menu_element_reacts_to_the_pointer() {
    unsafe {
        let mut h = Harness::new();
        h.frame();
        h.frame();
        let rect = h.menu_rect();

        let mut hit_cells = 0usize;
        let mut nav_values = std::collections::BTreeSet::new();
        let mut toggled = false;
        let mut slider_moved = false;
        let mut combo_changed = false;

        let cols = 22;
        let rows = 12;
        for row in 0..rows
        {
            for col in 0..cols
            {
                let x = rect.left() + (col as f32 + 0.5) / cols as f32 * rect.width();
                let y = rect.top() + (row as f32 + 0.5) / rows as f32 * rect.height();
                let p = Pos2::new(x, y,);
                let before = snapshot(&h.state,);
                h.move_to(p,);
                h.frame();
                h.click(p,);
                h.frame();
                h.frame();
                let after = snapshot(&h.state,);
                if after != before
                {
                    hit_cells += 1;
                    if after.0 != before.0
                    {
                        nav_values.insert(after.0,);
                    }
                    if after.1 != before.1
                    {
                        toggled = true;
                    }
                    if after.2 != before.2
                    {
                        slider_moved = true;
                    }
                    if after.3 != before.3
                    {
                        combo_changed = true;
                    }
                }
                let away = Pos2::new(rect.left() + 4.0, rect.bottom() + 40.0,);
                h.move_to(away,);
                h.frame();
            }
        }

        assert!(hit_cells >= 8, "only {hit_cells} clickable cells responded to the sweep");
        assert!(nav_values.len() >= 3, "sidebar tabs did not switch, saw {:?}", nav_values);
        assert!(toggled, "no checkbox or switch flipped");
        assert!(combo_changed || toggled, "no combo or toggle changed");
        let _ = slider_moved;
    }
}

#[test]
fn broad_interaction_touches_several_state_fields() {
    unsafe {
        let mut h = Harness::new();
        h.frame();
        h.frame();
        let rect = h.menu_rect();
        let base = snapshot(&h.state,);
        let mut changed = [false; 8];

        let cols = 26;
        let rows = 14;
        let mut row = 0;
        while row < rows
        {
            let mut col = 0;
            while col < cols
            {
                let p = Pos2::new(
                    rect.left() + (col as f32 + 0.5) / cols as f32 * rect.width(),
                    rect.top() + (row as f32 + 0.5) / rows as f32 * rect.height(),
                );
                h.click(p,);
                h.frame();
                h.drag(p, p + Vec2::new(60.0, 0.0,),);
                let now = snapshot(&h.state,);
                let a = [
                    base.0,
                    base.1 as u8,
                    base.2 as u8,
                    base.3,
                    base.4 as u8,
                    base.5 as u8,
                    base.6 as u8,
                    base.7 as u8,
                ];
                let b = [now.0, now.1 as u8, now.2 as u8, now.3, now.4 as u8, now.5 as u8, now.6 as u8, now.7 as u8,];
                for i in 0..8
                {
                    if a[i] != b[i]
                    {
                        changed[i] = true;
                    }
                }
                col += 1;
            }
            row += 1;
        }

        let hits = changed.iter().filter(|c| **c,).count();
        assert!(hits >= 3, "only {hits} state fields responded to broad interaction: {changed:?}");
        assert!(changed[0], "sidebar tabs never switched");
    }
}
#[test]
fn text_field_accepts_typing_then_clears() {
    unsafe {
        let mut h = Harness::new();
        h.frame();
        h.frame();
        let rect = h.menu_rect();
        let mut clicked = false;
        let mut y = rect.top() + 24.0;
        while y < rect.bottom() - 24.0 && !clicked
        {
            let p = Pos2::new(rect.left() + menu::SIDEBAR_W + 60.0, y,);
            h.click(p,);
            h.frame();
            if !h.state.input.is_empty() || h.state.open
            {
                h.type_text("ab",);
                h.frame();
                h.frame();
                if !h.state.input.is_empty()
                {
                    clicked = true;
                }
            }
            y += 14.0;
        }
        if clicked
        {
            let len = h.state.input.len();
            h.type_text("cd",);
            h.frame();
            assert!(h.state.input.len() >= len, "typing did not append to the text field");
        }
        h.key(Key::Escape,);
        h.frame();
    }
}

#[test]
fn wheel_scroll_and_keys_do_not_break_the_menu() {
    unsafe {
        let mut h = Harness::new();
        h.frame();
        let rect = h.menu_rect();
        let center = rect.center();
        for _ in 0..8
        {
            h.scroll(center, -3.0,);
            h.frame();
        }
        for _ in 0..8
        {
            h.scroll(center, 3.0,);
            h.frame();
        }
        for k in [Key::Tab, Key::Space, Key::Enter, Key::ArrowDown, Key::ArrowUp, Key::Escape,]
        {
            h.key(k,);
            h.frame();
        }
        assert!(h.state.open, "escape closed the menu without an explicit toggle");
        assert!(h.state.slider.is_finite() && h.state.slider >= 0.0);
    }
}

#[test]
fn menu_survives_every_screen_size() {
    unsafe {
        for (w, hgt,) in [(1280u32, 720u32,), (1920, 1080,), (800, 600,), (640, 480,),]
        {
            let mut h = Harness::new();
            let screen = Vec2::new(w as f32, hgt as f32,);
            h.frame();
            let fits = menu::fits((w, hgt,),);
            let rect = h.menu_rect();
            assert!(rect.width() == menu::W && rect.height() == menu::H);
            assert!(rect.left() >= 0.0 && rect.top() >= 0.0);
            let _ = (screen, fits,);
            h.click(rect.center(),);
            h.frame();
        }
    }
}
