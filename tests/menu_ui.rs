#![allow(deprecated)]

use egui::{Align2, Color32, Context, FontId, Pos2, RawInput, Rect, Stroke, Vec2};

const W: f32 = 892.0;
const H: f32 = 376.0;
const ENEMIES: usize = 9;

#[derive(Clone, Copy,)]
struct Enemy {
    pos:      Pos2,
    health:   f32,
    distance: f32,
    visible:  bool,
}

struct Model {
    open:    bool,
    esp:     bool,
    phase:   f32,
    enemies: [Enemy; ENEMIES],
}

impl Default for Model {
    fn default() -> Self {
        Self {
            open:    true,
            esp:     true,
            phase:   0.0,
            enemies: [Enemy { pos: Pos2::ZERO, health: 100.0, distance: 0.0, visible: true, }; ENEMIES],
        }
    }
}

impl Model {
    unsafe fn animate(&mut self, dt: f32, screen: Vec2,) {
        self.phase += dt;
        let mut i = 0;
        while i < ENEMIES
        {
            let k = i as f32;
            let t = self.phase * 0.55 + k * 0.7;
            let e = &mut self.enemies[i];
            e.pos = Pos2::new(
                screen.x * (0.52 + 0.44 * ((t * 0.9 + k * 0.31).sin() * 0.5 + 0.5)),
                screen.y * (0.52 + 0.42 * ((t * 0.6 + k * 0.53).cos() * 0.5 + 0.5)),
            );
            e.distance = 12.0 + k * 4.5 + 3.0 * (t * 1.3).sin();
            e.health = 100.0 * (0.35 + 0.65 * ((t * 0.8).sin() * 0.5 + 0.5));
            e.visible = i % 4 != 3;
            i += 1;
        }
    }

    unsafe fn menu_rect(&self, screen: Vec2,) -> Rect {
        let pos = Pos2::new(((screen.x - W) * 0.5).max(0.0,), ((screen.y - H) * 0.5).max(0.0,),);
        Rect::from_min_size(pos, Vec2::new(W, H,),)
    }

    unsafe fn draw(&self, ctx: &Context, screen: Vec2,) {
        if self.esp
        {
            let p = ctx.layer_painter(egui::LayerId::background(),);
            let mut i = 0;
            while i < ENEMIES
            {
                let e = self.enemies[i];
                i += 1;
                if !e.visible
                {
                    continue;
                }
                let h = 92.0 - e.distance.min(90.0,) * 0.42;
                let r = Rect::from_center_size(e.pos, Vec2::new(h * 0.46, h,),);
                p.rect_stroke(r, 0.0, Stroke::new(1.6, Color32::RED,), egui::StrokeKind::Inside,);
                p.text(
                    Pos2::new(r.center().x, r.top() - 4.0,),
                    Align2::CENTER_BOTTOM,
                    format!("{:.0}m", e.distance),
                    FontId::monospace(10.0,),
                    Color32::WHITE,
                );
            }
        }

        if !self.open
        {
            return;
        }
        let rect = self.menu_rect(screen,);
        {
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("menu_layer",),),);
            p.rect_filled(rect, 6.0, Color32::from_rgb(11, 14, 20,),);
            p.rect_stroke(rect, 6.0, Stroke::new(1.0, Color32::from_rgb(32, 40, 54,),), egui::StrokeKind::Inside,);
            let mut y = rect.top() + 24.0;
            while y < rect.bottom() - 24.0
            {
                let row = Rect::from_min_size(Pos2::new(rect.left() + 16.0, y,), Vec2::new(200.0, 18.0,),);
                p.rect_filled(row, 4.0, Color32::from_rgb(19, 24, 34,),);
                p.text(
                    row.min + Vec2::new(6.0, 2.0,),
                    Align2::LEFT_TOP,
                    "nav item",
                    FontId::monospace(11.0,),
                    Color32::from_rgb(230, 236, 245,),
                );
                y += 26.0;
            }
        }
    }
}

unsafe fn run_once(model: &Model, ctx: &Context, screen: Vec2,) -> usize {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen,),), ..Default::default() };
    let mut out = ctx.run_ui(input, |ctx| model.draw(ctx, screen,),);
    out.textures_delta.clear();
    let shapes = out.shapes.len();
    let prims = ctx.tessellate(out.shapes, 1.0,);
    let verts: usize = prims
        .iter()
        .map(|p| match &p.primitive
        {
            egui::epaint::Primitive::Mesh(m,) => m.vertices.len(),
            _ => 0,
        },)
        .sum();
    assert!(verts > 0, "pass produced no vertices");
    shapes
}

#[test]
fn menu_and_esp_both_produce_geometry() {
    unsafe {
        let ctx = Context::default();
        let mut model = Model::default();
        model.animate(0.25, Vec2::new(1280.0, 720.0,),);
        let with_esp = run_once(&model, &ctx, Vec2::new(1280.0, 720.0,),);

        model.esp = false;
        let without_esp = run_once(&model, &ctx, Vec2::new(1280.0, 720.0,),);
        assert!(with_esp > without_esp, "esp added no shapes: {with_esp} vs {without_esp}");
    }
}

#[test]
fn closed_menu_keeps_esp_running() {
    unsafe {
        let ctx = Context::default();
        let mut model = Model::default();
        model.animate(0.1, Vec2::new(1280.0, 720.0,),);
        let open = run_once(&model, &ctx, Vec2::new(1280.0, 720.0,),);
        model.open = false;
        let closed = run_once(&model, &ctx, Vec2::new(1280.0, 720.0,),);
        assert!(closed > 0, "esp must keep drawing with the menu closed");
        assert!(open > closed, "menu added no shapes: {open} vs {closed}");
    }
}

#[test]
fn menu_rect_is_centered_and_bounded() {
    unsafe {
        let model = Model::default();
        for screen in [Vec2::new(1280.0, 720.0,), Vec2::new(1920.0, 1080.0,), Vec2::new(800.0, 600.0,),]
        {
            let r = model.menu_rect(screen,);
            assert_eq!(r.width(), W);
            assert_eq!(r.height(), H);
            assert!(r.min.x >= 0.0 && r.min.y >= 0.0);
            assert!(r.max.x <= screen.x + W && r.max.y <= screen.y + H);
        }
    }
}

#[test]
fn esp_enemies_stay_finite_and_on_screen() {
    unsafe {
        let mut model = Model::default();
        let screen = Vec2::new(1280.0, 720.0,);
        let mut step = 0;
        while step < 600
        {
            model.animate(0.016, screen,);
            let mut i = 0;
            while i < ENEMIES
            {
                let e = model.enemies[i];
                i += 1;
                assert!(e.pos.x.is_finite() && e.pos.y.is_finite(), "nan position at step {step}");
                assert!(e.pos.x >= 0.0 && e.pos.x <= screen.x, "x off screen at step {step}");
                assert!(e.pos.y >= 0.0 && e.pos.y <= screen.y, "y off screen at step {step}");
                assert!((0.0..=100.0).contains(&e.health), "health out of range: {}", e.health);
                assert!(e.distance > 0.0, "distance must stay positive");
            }
            step += 1;
        }
    }
}
