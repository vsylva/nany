#![allow(deprecated)]
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke};

use super::menu::Theme;

const ENEMY_COUNT: usize = 9;
const TEAM_ENEMY: Color32 = Color32::from_rgb(255, 92, 92,);
const TEAM_VISIBLE: Color32 = Color32::from_rgb(255, 196, 64,);
const HEALTH_HI: Color32 = Color32::from_rgb(72, 219, 166,);
const HEALTH_MID: Color32 = Color32::from_rgb(255, 196, 64,);
const HEALTH_LOW: Color32 = Color32::from_rgb(255, 92, 92,);

#[derive(Clone, Copy,)]
pub struct Enemy {
    pub pos:        (f32, f32,),
    pub health:     f32,
    pub max_health: f32,
    pub distance:   f32,
    pub visible:    bool,
    pub name:       &'static str,
}

pub struct Esp {
    pub enabled:     bool,
    pub boxes:       bool,
    pub health_bars: bool,
    pub names:       bool,
    pub skeletons:   bool,
    pub distance:    bool,
    phase:           f32,
    enemies:         [Enemy; ENEMY_COUNT],
}

const NAMES: [&str; ENEMY_COUNT] =
    ["grunt-01", "grunt-02", "scout-11", "scout-12", "heavy-21", "heavy-22", "boss-31", "drone-41", "drone-42",];

impl Default for Esp {
    fn default() -> Self {
        let mut enemies = [Enemy {
            pos:        (0.0, 0.0,),
            health:     100.0,
            max_health: 100.0,
            distance:   0.0,
            visible:    true,
            name:       "grunt",
        }; ENEMY_COUNT];
        for (i, e,) in enemies.iter_mut().enumerate()
        {
            e.name = NAMES[i];
        }
        let mut esp = Self {
            enabled: true,
            boxes: true,
            health_bars: true,
            names: true,
            skeletons: true,
            distance: true,
            phase: 0.0,
            enemies,
        };
        unsafe { esp.animate(0.0, (1280, 720,),) };
        esp
    }
}

impl Esp {
    pub unsafe fn animate(&mut self, dt: f32, screen: (u32, u32,),) {
        self.phase += dt.max(0.0,);
        let (w, h,) = (screen.0 as f32, screen.1 as f32,);
        for (i, e,) in self.enemies.iter_mut().enumerate()
        {
            let k = i as f32;
            let t = self.phase * 0.55 + k * 0.7;
            let x = w * (0.62 + 0.3 * (t * 0.9).sin() * (0.4 + k * 0.03));
            let y = h * (0.30 + 0.42 * (t * 0.6 + k * 0.5).cos().abs());
            e.pos = (x, y,);
            e.distance = 12.0 + (k * 4.5) + 3.0 * (t * 1.3).sin();
            e.max_health = 100.0;
            e.health = (100.0 * (0.35 + 0.65 * ((t * 0.8).sin() * 0.5 + 0.5))).clamp(1.0, 100.0,);
            e.visible = (i % 4) != 3;
        }
    }

    pub unsafe fn draw(&self, ctx: &egui::Context, screen: (u32, u32,),) {
        if !self.enabled
        {
            return;
        }
        let (sw, sh,) = (screen.0 as f32, screen.1 as f32,);
        if sw < 64.0 || sh < 64.0
        {
            return;
        }
        let area = egui::Area::new(egui::Id::new("esp_background",),)
            .order(egui::Order::Background,)
            .fixed_pos(Pos2::ZERO,)
            .interactable(false,);
        area.show(ctx, |ui| {
            let p = ui.painter();
            for e in self.enemies.iter()
            {
                if !e.visible
                {
                    continue;
                }
                let h = 92.0 - e.distance.min(90.0,) * 0.42;
                let w = h * 0.46;
                let center = Pos2::new(e.pos.0, e.pos.1,);
                let rect = Rect::from_center_size(center, egui::vec2(w, h,),);
                let color = if e.health > 55.0 { TEAM_ENEMY } else { TEAM_VISIBLE };

                if self.boxes
                {
                    let c = 14.0;
                    let s = Stroke::new(1.6, color,);
                    for (a, b,) in [
                        (rect.left_top(), Pos2::new(rect.left() + c, rect.top(),),),
                        (rect.left_top(), Pos2::new(rect.left(), rect.top() + c,),),
                        (rect.right_top(), Pos2::new(rect.right() - c, rect.top(),),),
                        (rect.right_top(), Pos2::new(rect.right(), rect.top() + c,),),
                        (rect.left_bottom(), Pos2::new(rect.left() + c, rect.bottom(),),),
                        (rect.left_bottom(), Pos2::new(rect.left(), rect.bottom() - c,),),
                        (rect.right_bottom(), Pos2::new(rect.right() - c, rect.bottom(),),),
                        (rect.right_bottom(), Pos2::new(rect.right(), rect.bottom() - c,),),
                    ]
                    {
                        p.line_segment([a, b,], s,);
                    }
                }

                if self.skeletons
                {
                    let s = Stroke::new(1.2, color.gamma_multiply(0.75,),);
                    let head = Pos2::new(center.x, rect.top() + h * 0.12,);
                    p.circle_stroke(head, h * 0.09, s,);
                    let neck = Pos2::new(center.x, rect.top() + h * 0.21,);
                    let hip = Pos2::new(center.x, rect.top() + h * 0.58,);
                    p.line_segment([neck, hip,], s,);
                    p.line_segment([neck, Pos2::new(rect.left() + w * 0.18, rect.top() + h * 0.42,),], s,);
                    p.line_segment([neck, Pos2::new(rect.right() - w * 0.18, rect.top() + h * 0.42,),], s,);
                    p.line_segment([hip, Pos2::new(rect.left() + w * 0.26, rect.bottom(),),], s,);
                    p.line_segment([hip, Pos2::new(rect.right() - w * 0.26, rect.bottom(),),], s,);
                }

                if self.health_bars
                {
                    let bar_w = w.max(46.0,);
                    let bar = Rect::from_min_size(
                        Pos2::new(center.x - bar_w * 0.5, rect.top() - 12.0,),
                        egui::vec2(bar_w, 4.0,),
                    );
                    p.rect_filled(bar, 1.0, Color32::from_black_alpha(190,),);
                    let frac = (e.health / e.max_health).clamp(0.0, 1.0,);
                    let fill = if frac > 0.55
                    {
                        HEALTH_HI
                    }
                    else if frac > 0.25
                    {
                        HEALTH_MID
                    }
                    else
                    {
                        HEALTH_LOW
                    };
                    p.rect_filled(
                        Rect::from_min_size(bar.min, egui::vec2(bar.width() * frac, bar.height(),),),
                        1.0,
                        fill,
                    );
                }

                if self.names
                {
                    p.text(
                        Pos2::new(center.x, rect.top() - 20.0,),
                        Align2::CENTER_BOTTOM,
                        e.name,
                        FontId::monospace(11.0,),
                        Theme::TEXT,
                    );
                }

                if self.distance
                {
                    p.text(
                        Pos2::new(center.x, rect.bottom() + 4.0,),
                        Align2::CENTER_TOP,
                        format!("{:.0}m", e.distance),
                        FontId::monospace(10.0,),
                        Theme::MUTED,
                    );
                }
            }
        },);
    }
}
