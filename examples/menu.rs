#![allow(dead_code)]

use std::sync::Mutex;

use nany::overlay::{Frame, Provider};

#[path = "../tests/support/menu.rs"]
mod menu;

const VK_INSERT: i32 = 0x2D;
const DLL_PROCESS_ATTACH: u32 = 1;

struct MenuShell {
    state: Mutex<menu::State,>,
    t0:    std::time::Instant,
}

impl Provider for MenuShell {
    unsafe fn init(&self, ctx: &egui::Context,) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\msyh.ttc",);
        ctx.all_styles_mut(|s| {
            s.visuals = egui::Visuals::dark();
            s.visuals.window_fill = menu::Theme::BG;
            s.visuals.panel_fill = menu::Theme::BG;
            s.visuals.extreme_bg_color = menu::Theme::INPUT_BG;
            s.visuals.faint_bg_color = menu::Theme::CARD;
            s.visuals.override_text_color = Some(menu::Theme::TEXT,);
            s.visuals.widgets.inactive.bg_fill = menu::Theme::INPUT_BG;
            s.visuals.widgets.hovered.bg_fill = menu::Theme::CARD_HI;
            s.visuals.widgets.active.bg_fill = menu::Theme::ACCENT;
        },);
    }

    unsafe fn ui(&self, f: &Frame<'_,>,) -> bool {
        let mut st = match self.state.lock()
        {
            Ok(g,) => g,
            Err(p,) => p.into_inner(),
        };

        if nany::input::hotkey(VK_INSERT,)
        {
            st.open = !st.open;
            nany::log::emit_info("menu", if st.open { "menu opened" } else { "menu closed" },);
        }

        if !st.open
        {
            return false;
        }

        let info = menu::Info {
            backend:  f.backend,
            screen:   f.screen,
            frame_ms: f.frame_ms,
            frames:   f.frames,
            hotkey:   VK_INSERT,
            t:        self.t0.elapsed().as_secs_f32(),
        };
        menu::draw(f.ctx, &mut st, &info,);
        if let Some((x, y,),) = f.pointer
        {
            menu::draw_cursor(f.ctx, egui::pos2(x, y,),);
        }
        true
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_h: *mut std::ffi::c_void, reason: u32, _r: *mut std::ffi::c_void,) -> i32 {
    if reason == DLL_PROCESS_ATTACH
    {
        let shell =
            Box::new(MenuShell { state: Mutex::new(menu::State::default(),), t0: std::time::Instant::now(), },);
        unsafe { nany::on_dll_attach_tagged("menu", shell,) };
    }
    1
}
