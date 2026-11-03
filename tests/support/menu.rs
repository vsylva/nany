use egui::{Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

pub struct Theme;

impl Theme {
    pub const BG: Color32 = Color32::from_rgb(11, 14, 20,);
    pub const SIDE: Color32 = Color32::from_rgb(14, 18, 26,);
    pub const CARD: Color32 = Color32::from_rgb(19, 24, 34,);
    pub const CARD_HI: Color32 = Color32::from_rgb(24, 30, 42,);
    pub const LINE: Color32 = Color32::from_rgb(32, 40, 54,);
    pub const TEXT: Color32 = Color32::from_rgb(230, 236, 245,);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(176, 184, 198,);
    pub const MUTED: Color32 = Color32::from_rgb(124, 135, 152,);
    pub const ACCENT: Color32 = Color32::from_rgb(91, 140, 255,);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(42, 58, 92,);
    pub const PILL: Color32 = Color32::from_rgb(26, 33, 46,);
    pub const INPUT_BG: Color32 = Color32::from_rgb(15, 19, 27,);

    pub const TRACK_OFF: Color32 = Color32::from_rgb(44, 52, 68,);
}

#[derive(Clone, Copy, PartialEq,)]
pub enum Icon {
    Crosshair,
    Diamond,
    Triangle,
    Person,
    Grid,
    Gear,
    Code,
    Pen,
    Crown,
}

pub struct State {
    pub open: bool,
    pub pos:  egui::Pos2,

    pub nav: u8,

    pub feather: bool,
    pub slider:  f32,
    pub combo:   u8,
    pub input:   String,
    pub color:   Color32,

    pub color_open: bool,

    pub crown: [bool; 11],
}

impl Default for State {
    fn default() -> Self {
        Self {
            open:       true,
            pos:        egui::pos2(80.0, 80.0,),
            nav:        0,
            feather:    true,
            slider:     42.0,
            combo:      0,
            input:      String::new(),
            color:      Color32::from_rgb(91, 140, 255,),
            color_open: false,
            crown:      [true; 11],
        }
    }
}

pub struct Info {
    pub backend:  &'static str,
    pub screen:   (u32, u32,),
    pub frame_ms: f32,
    pub frames:   u64,
    pub hotkey:   i32,
    pub t:        f32,
}

pub const W: f32 = 892.0;
pub const H: f32 = 376.0;
pub const SIDEBAR_W: f32 = 224.0;
pub const CARD_W: f32 = 306.0;
pub const CARD_H: f32 = 336.0;

const CHIP: f32 = 26.0;
const CHIP_GAP: f32 = 6.0;
const COLS: usize = 6;

const GROUPS: [(&str, &[(&str, Icon,)],); 3] = [
    ("Aimbot", &[("Ragebot", Icon::Crosshair,), ("Anti-aim", Icon::Diamond,), ("Legitbot", Icon::Triangle,),],),
    ("Visuals", &[("Players", Icon::Person,), ("World", Icon::Grid,),],),
    ("Miscellaneous", &[("Main", Icon::Gear,), ("Scripts", Icon::Code,),],),
];

pub unsafe fn fits(screen: (u32, u32,),) -> bool {
    (screen.0 as f32) > W + 40.0 && (screen.1 as f32) > H + 40.0
}

pub unsafe fn draw_icon(p: &egui::Painter, c: Pos2, icon: Icon, color: Color32,) {
    let s = Stroke::new(1.4, color,);
    let r = 6.0;
    match icon
    {
        Icon::Crosshair =>
        {
            p.circle_stroke(c, 4.6, s,);
            p.line_segment([Pos2::new(c.x, c.y - r,), Pos2::new(c.x, c.y - 2.4,),], s,);
            p.line_segment([Pos2::new(c.x, c.y + 2.4,), Pos2::new(c.x, c.y + r,),], s,);
            p.line_segment([Pos2::new(c.x - r, c.y,), Pos2::new(c.x - 2.4, c.y,),], s,);
            p.line_segment([Pos2::new(c.x + 2.4, c.y,), Pos2::new(c.x + r, c.y,),], s,);
            p.circle_filled(c, 1.1, color,);
        }
        Icon::Diamond =>
        {
            let pts = vec![
                Pos2::new(c.x, c.y - r,),
                Pos2::new(c.x + r, c.y,),
                Pos2::new(c.x, c.y + r,),
                Pos2::new(c.x - r, c.y,),
            ];
            p.add(egui::Shape::closed_line(pts, s,),);
        }
        Icon::Triangle =>
        {
            let pts = vec![
                Pos2::new(c.x, c.y - r + 0.5,),
                Pos2::new(c.x + r - 0.5, c.y + r - 1.0,),
                Pos2::new(c.x - r + 0.5, c.y + r - 1.0,),
            ];
            p.add(egui::Shape::closed_line(pts, s,),);
        }
        Icon::Person =>
        {
            p.circle_stroke(Pos2::new(c.x, c.y - 3.2,), 2.6, s,);
            let pts = vec![
                Pos2::new(c.x - 4.6, c.y + 6.0,),
                Pos2::new(c.x - 3.0, c.y + 1.4,),
                Pos2::new(c.x + 3.0, c.y + 1.4,),
                Pos2::new(c.x + 4.6, c.y + 6.0,),
            ];
            p.add(egui::Shape::line(pts, s,),);
        }
        Icon::Grid =>
        {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(11.0,),),
                CornerRadius::same(2,),
                s,
                StrokeKind::Inside,
            );
            p.line_segment([Pos2::new(c.x - 5.5, c.y,), Pos2::new(c.x + 5.5, c.y,),], s,);
            p.line_segment([Pos2::new(c.x, c.y - 5.5,), Pos2::new(c.x, c.y + 5.5,),], s,);
        }
        Icon::Gear =>
        {
            p.circle_stroke(c, 3.4, s,);
            for i in 0..8
            {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let (sa, ca,) = (a.sin(), a.cos(),);
                p.line_segment(
                    [Pos2::new(c.x + ca * 4.4, c.y + sa * 4.4,), Pos2::new(c.x + ca * 6.2, c.y + sa * 6.2,),],
                    s,
                );
            }
        }
        Icon::Code =>
        {
            let pts_l =
                vec![Pos2::new(c.x - 2.0, c.y - 4.4,), Pos2::new(c.x - 5.6, c.y,), Pos2::new(c.x - 2.0, c.y + 4.4,)];
            let pts_r =
                vec![Pos2::new(c.x + 2.0, c.y - 4.4,), Pos2::new(c.x + 5.6, c.y,), Pos2::new(c.x + 2.0, c.y + 4.4,)];
            p.add(egui::Shape::line(pts_l, s,),);
            p.add(egui::Shape::line(pts_r, s,),);
        }
        Icon::Pen =>
        {
            p.line_segment([Pos2::new(c.x - 4.6, c.y + 4.6,), Pos2::new(c.x + 3.2, c.y - 3.4,),], s,);
            let nib = vec![
                Pos2::new(c.x + 3.2, c.y - 3.4,),
                Pos2::new(c.x + 5.6, c.y - 5.6,),
                Pos2::new(c.x + 4.2, c.y - 2.2,),
            ];
            p.add(egui::Shape::closed_line(nib, s,),);
            p.line_segment([Pos2::new(c.x - 5.0, c.y + 5.0,), Pos2::new(c.x - 2.4, c.y + 4.2,),], s,);
        }
        Icon::Crown =>
        {
            let pts = vec![
                Pos2::new(c.x - 5.4, c.y + 3.6,),
                Pos2::new(c.x - 4.2, c.y - 3.8,),
                Pos2::new(c.x - 1.4, c.y + 0.6,),
                Pos2::new(c.x, c.y - 4.6,),
                Pos2::new(c.x + 1.4, c.y + 0.6,),
                Pos2::new(c.x + 4.2, c.y - 3.8,),
                Pos2::new(c.x + 5.4, c.y + 3.6,),
            ];
            p.add(egui::Shape::line(pts, s,),);
            p.line_segment([Pos2::new(c.x - 5.4, c.y + 3.6,), Pos2::new(c.x + 5.4, c.y + 3.6,),], s,);
        }
    }
}

unsafe fn switch(ui: &mut Ui, on: &mut bool,) -> egui::Response {
    let (rect, mut resp,) = ui.allocate_exact_size(Vec2::new(36.0, 20.0,), Sense::click(),);
    if resp.clicked()
    {
        *on = !*on;
        resp.mark_changed();
    }
    let k = ui.ctx().animate_bool_with_time(resp.id, *on, 0.12,);
    let p = ui.painter();
    let track = if *on { Theme::ACCENT.gamma_multiply(0.85,) } else { Theme::TRACK_OFF };
    p.rect_filled(rect, CornerRadius::same(10,), track,);
    let cx = rect.left() + 10.0 + k * (rect.width() - 20.0);
    p.circle_filled(egui::pos2(cx, rect.center().y,), 7.0, Color32::WHITE,);
    resp
}

unsafe fn nav_item(ui: &mut Ui, icon: Icon, text: &str, active: bool,) -> egui::Response {
    let (rect, resp,) = ui.allocate_exact_size(Vec2::new(SIDEBAR_W - 20.0, 36.0,), Sense::click(),);
    let hovered = resp.hovered();
    let p = ui.painter();
    if active
    {
        p.rect_filled(rect, CornerRadius::same(9,), Theme::PILL,);
        p.rect_filled(
            Rect::from_center_size(egui::pos2(rect.left() + 6.0, rect.center().y,), Vec2::new(3.0, 16.0,),),
            CornerRadius::same(2,),
            Theme::ACCENT,
        );
    }
    else if hovered
    {
        p.rect_filled(rect, CornerRadius::same(9,), Theme::CARD,);
    }
    let fg = if active
    {
        Theme::ACCENT
    }
    else if hovered
    {
        Theme::TEXT_DIM
    }
    else
    {
        Theme::MUTED
    };
    draw_icon(p, egui::pos2(rect.left() + 24.0, rect.center().y,), icon, fg,);
    p.text(
        egui::pos2(rect.left() + 42.0, rect.center().y,),
        egui::Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.5,),
        if active { Theme::TEXT } else { fg },
    );
    resp
}

unsafe fn mix(a: Color32, b: Color32, t: f32,) -> Color32 {
    let t = t.clamp(0.0, 1.0,);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(f(a.r(), b.r(),), f(a.g(), b.g(),), f(a.b(), b.b(),), 255,)
}

unsafe fn pill_button(ui: &mut Ui, text: &str, width: f32, filled: bool, pulse: f32,) -> egui::Response {
    let (rect, resp,) = ui.allocate_exact_size(Vec2::new(width, 32.0,), Sense::click(),);
    let p = ui.painter();

    let hover = ui.ctx().animate_bool_with_time(resp.id.with("hover",), resp.hovered(), 0.12,);

    let pressed = resp.is_pointer_button_down_on();

    let glow = 0.85 + 0.15 * (pulse * 2.0).sin().abs();

    let (idle, hot,) =
        if filled { (Theme::ACCENT_DIM.gamma_multiply(glow,), Theme::ACCENT,) } else { (Theme::PILL, Theme::CARD_HI,) };
    let mut bg = mix(idle, hot, hover,);
    if pressed
    {
        bg = bg.gamma_multiply(0.82,);
    }

    let edge = mix(Theme::LINE, if filled { Theme::ACCENT } else { Theme::ACCENT.gamma_multiply(0.55,) }, hover,);
    p.rect_filled(rect, CornerRadius::same(8,), bg,);
    p.rect_stroke(rect, CornerRadius::same(8,), Stroke::new(1.0, edge,), StrokeKind::Inside,);

    if pressed
    {
        p.rect_stroke(
            rect.shrink(1.0,),
            CornerRadius::same(7,),
            Stroke::new(1.0, Color32::from_black_alpha(60,),),
            StrokeKind::Inside,
        );
    }
    let text_pos = rect.center() + egui::vec2(0.0, if pressed { 1.0 } else { 0.0 },);
    p.text(
        text_pos,
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0,),
        mix(Theme::TEXT, Color32::WHITE, hover * 0.6,),
    );
    resp
}

unsafe fn card(ui: &mut Ui, icon: Icon, title: &str, add: impl FnOnce(&mut Ui,),) {
    ui.vertical(|ui| {
        ui.set_width(CARD_W,);
        egui::Frame::new()
            .fill(Theme::CARD,)
            .corner_radius(CornerRadius::same(12,),)
            .stroke(Stroke::new(1.0, Theme::LINE,),)
            .inner_margin(Margin::same(16,),)
            .show(ui, |ui| {
                ui.set_width(CARD_W - 32.0,);
                ui.set_min_height(CARD_H - 32.0,);
                ui.horizontal(|ui| {
                    let (ir, _,) = ui.allocate_exact_size(Vec2::splat(14.0,), Sense::hover(),);
                    draw_icon(ui.painter(), ir.center(), icon, Theme::MUTED,);
                    ui.add_space(2.0,);
                    ui.add(
                        egui::Label::new(RichText::new(title,).size(14.0,).color(Theme::TEXT_DIM,),).selectable(false,),
                    );
                },);
                ui.add_space(12.0,);
                add(ui,);
            },);
    },);
}

unsafe fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui,),) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new(label,).size(13.0,).color(Theme::TEXT_DIM,),).selectable(false,),);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center,), add,);
    },);
    ui.add_space(10.0,);
}

unsafe fn slider_row(ui: &mut Ui, label: &str, value: &mut f32,) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new(label,).size(13.0,).color(Theme::TEXT_DIM,),).selectable(false,),);
        ui.add_space(10.0,);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center,), |ui| {
            ui.add(
                egui::Label::new(RichText::new(format!("{:.0}%", *value),).size(12.5,).color(Theme::TEXT,),)
                    .selectable(false,),
            );
            ui.add_space(8.0,);
            let w = ui.available_width().max(40.0,);
            ui.spacing_mut().slider_width = w;
            ui.add(egui::Slider::new(value, 0.0..=100.0,).show_value(false,).trailing_fill(true,),);
        },);
    },);
    ui.add_space(10.0,);
}

const PRESETS: [Color32; 24] = [
    Color32::from_rgb(91, 140, 255,),
    Color32::from_rgb(86, 204, 242,),
    Color32::from_rgb(72, 219, 166,),
    Color32::from_rgb(163, 230, 106,),
    Color32::from_rgb(255, 214, 102,),
    Color32::from_rgb(255, 159, 90,),
    Color32::from_rgb(255, 107, 107,),
    Color32::from_rgb(255, 130, 192,),
    Color32::from_rgb(186, 138, 255,),
    Color32::from_rgb(230, 236, 245,),
    Color32::from_rgb(124, 135, 152,),
    Color32::from_rgb(76, 58, 120,),
    Color32::from_rgb(255, 176, 102,),
    Color32::from_rgb(255, 122, 69,),
    Color32::from_rgb(233, 69, 96,),
    Color32::from_rgb(181, 23, 158,),
    Color32::from_rgb(114, 9, 183,),
    Color32::from_rgb(58, 12, 163,),
    Color32::from_rgb(67, 97, 238,),
    Color32::from_rgb(76, 201, 240,),
    Color32::from_rgb(0, 187, 249,),
    Color32::from_rgb(0, 245, 212,),
    Color32::from_rgb(6, 214, 160,),
    Color32::from_rgb(128, 237, 153,),
];

unsafe fn color_swatch(ui: &mut Ui, color: &mut Color32, open: &mut bool, screen: (u32, u32,),) {
    let (screen_w, screen_h,) = (screen.0 as f32, screen.1 as f32,);
    let (rect, resp,) = ui.allocate_exact_size(Vec2::new(120.0, 24.0,), Sense::click(),);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(6,), *color,);
    p.rect_stroke(rect, CornerRadius::same(6,), Stroke::new(1.0, Theme::LINE,), StrokeKind::Inside,);

    p.text(
        egui::pos2(rect.right() - 8.0, rect.center().y,),
        egui::Align2::RIGHT_CENTER,
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b()),
        FontId::proportional(10.5,),
        Color32::from_rgb(20, 24, 30,),
    );

    let area_id = ui.make_persistent_id("menu_kit.picker",);
    if resp.clicked()
    {
        *open = !*open;
    }
    if !*open
    {
        return;
    }

    let rows = PRESETS.len().div_ceil(COLS,);
    let est_w = COLS as f32 * CHIP + (COLS as f32 - 1.0) * CHIP_GAP + 20.0;
    let est_h = rows as f32 * (CHIP + CHIP_GAP) + 3.0 * (18.0 + CHIP_GAP) + 24.0;
    let screen = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(screen_w, screen_h,),);

    let mut ax = rect.left();
    let mut ay = rect.bottom() + 6.0;
    if ay + est_h > screen.bottom() - 8.0
    {
        ay = (rect.top() - est_h - 6.0).max(screen.top() + 8.0,);
    }
    if ax + est_w > screen.right() - 8.0
    {
        ax = (screen.right() - est_w - 8.0).max(screen.left() + 8.0,);
    }

    let anchor = egui::pos2(ax, ay,);
    let area = egui::Area::new(area_id,).order(egui::Order::Foreground,).fixed_pos(anchor,).show(ui.ctx(), |ui| {
        egui::Frame::new()
            .fill(Theme::CARD,)
            .corner_radius(CornerRadius::same(10,),)
            .stroke(Stroke::new(1.0, Theme::LINE,),)
            .inner_margin(Margin::same(10,),)
            .shadow(egui::epaint::Shadow {
                offset: [0, 10,],
                blur:   24,
                spread: 0,
                color:  Color32::from_black_alpha(150,),
            },)
            .show(ui, |ui| {
                let grid_w = COLS as f32 * CHIP + (COLS as f32 - 1.0) * CHIP_GAP;

                ui.set_width(grid_w,);
                ui.spacing_mut().item_spacing = Vec2::splat(CHIP_GAP,);

                let rows = PRESETS.len().div_ceil(COLS,);
                for row in 0..rows
                {
                    ui.horizontal(|ui| {
                        for col in 0..COLS
                        {
                            let c = PRESETS[row * COLS + col];
                            let (r, chip,) = ui.allocate_exact_size(Vec2::splat(CHIP,), Sense::click(),);
                            let p = ui.painter();
                            p.rect_filled(r, CornerRadius::same(5,), c,);

                            p.rect_stroke(
                                r,
                                CornerRadius::same(5,),
                                Stroke::new(1.0, Color32::from_white_alpha(30,),),
                                StrokeKind::Inside,
                            );
                            if *color == c
                            {
                                p.rect_stroke(
                                    r,
                                    CornerRadius::same(5,),
                                    Stroke::new(1.8, Theme::TEXT,),
                                    StrokeKind::Inside,
                                );
                            }
                            else if chip.hovered()
                            {
                                p.rect_stroke(
                                    r,
                                    CornerRadius::same(5,),
                                    Stroke::new(1.4, Theme::ACCENT,),
                                    StrokeKind::Inside,
                                );
                            }
                            if chip.clicked()
                            {
                                *color = c;
                            }
                        }
                    },);
                }

                ui.add_space(2.0,);

                const LBL_W: f32 = 10.0;
                const VAL_W: f32 = 24.0;
                let mut rgb = [color.r(), color.g(), color.b(),];
                for (i, name,) in ["R", "G", "B",].iter().enumerate()
                {
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            Vec2::new(LBL_W, 18.0,),
                            egui::Layout::left_to_right(egui::Align::Center,),
                            |ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(*name,).size(11.5,).color(Theme::MUTED,),)
                                        .selectable(false,),
                                );
                            },
                        );
                        ui.spacing_mut().slider_width = grid_w - LBL_W - CHIP_GAP - CHIP_GAP - VAL_W;
                        let mut v = rgb[i] as f32;
                        if ui.add(egui::Slider::new(&mut v, 0.0..=255.0,).show_value(false,),).changed()
                        {
                            rgb[i] = v.round() as u8;
                        }
                        ui.allocate_ui_with_layout(
                            Vec2::new(VAL_W, 18.0,),
                            egui::Layout::right_to_left(egui::Align::Center,),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("{:.0}", rgb[i]),).size(11.5,).color(Theme::TEXT,),
                                    )
                                    .selectable(false,),
                                );
                            },
                        );
                    },);
                }
                let next = Color32::from_rgb(rgb[0], rgb[1], rgb[2],);
                if next != *color
                {
                    *color = next;
                }
            },);
    },);

    let pointer = ui.ctx().pointer_latest_pos();
    let inside = pointer.is_some_and(|p| rect.contains(p,) || area.response.rect.contains(p,),);
    if ui.input(|i| i.pointer.any_click(),) && !inside
    {
        *open = false;
    }
}

unsafe fn numbered(ui: &mut Ui, n: usize, on: &mut bool,) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new(n.to_string(),).size(13.0,).color(Theme::MUTED,),).selectable(false,),);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center,), |ui| {
            switch(ui, on,);
        },);
    },);
    ui.add_space(4.0,);
}

pub unsafe fn draw(ctx: &egui::Context, st: &mut State, info: &Info,) {
    if !st.open || !fits(info.screen,)
    {
        return;
    }

    let frame = egui::Frame::new()
        .fill(Theme::BG,)
        .corner_radius(CornerRadius::same(16,),)
        .stroke(Stroke::new(1.0, Theme::LINE,),)
        .inner_margin(Margin::same(0,),)
        .shadow(egui::epaint::Shadow {
            offset: [0, 16,],
            blur:   40,
            spread: 0,
            color:  Color32::from_black_alpha(160,),
        },);

    egui::Window::new("menu_kit",)
        .title_bar(false,)
        .resizable(false,)
        .collapsible(false,)
        .fixed_pos(st.pos,)
        .fixed_size(Vec2::new(W, H,),)
        .frame(frame,)
        .show(ctx, |w| {
            let full = w.available_rect_before_wrap();

            w.painter().rect_filled(
                Rect::from_min_size(full.min, Vec2::new(SIDEBAR_W, full.height(),),),
                CornerRadius { nw: 16, sw: 16, ne: 0, se: 0, },
                Theme::SIDE,
            );

            w.painter().rect_filled(
                Rect::from_min_size(
                    egui::pos2(full.left() + SIDEBAR_W, full.top() + 16.0,),
                    Vec2::new(1.0, full.height() - 32.0,),
                ),
                CornerRadius::same(0,),
                Theme::LINE,
            );

            w.horizontal_top(|w| {
                w.allocate_ui_with_layout(
                    Vec2::new(SIDEBAR_W, full.height(),),
                    egui::Layout::top_down(egui::Align::LEFT,),
                    |w| {
                        w.add_space(20.0,);
                        w.horizontal(|ui| {
                            ui.add_space(20.0,);
                            ui.add(
                                egui::Label::new(RichText::new("NOVA",).size(22.0,).strong().color(Theme::TEXT,),)
                                    .selectable(false,),
                            );
                        },);

                        #[allow(deprecated)]
                        let drag = w.interact(
                            Rect::from_min_size(egui::pos2(full.left(), full.top(),), Vec2::new(SIDEBAR_W, 60.0,),),
                            egui::Id::new("menu_kit.drag",),
                            Sense::drag(),
                        );
                        if drag.dragged()
                        {
                            st.pos += drag.drag_delta();
                        }

                        w.add_space(10.0,);
                        let mut idx = 0u8;
                        for (gi, (_title, items,),) in GROUPS.iter().enumerate()
                        {
                            if gi > 0
                            {
                                w.add_space(10.0,);
                            }
                            w.horizontal(|ui| {
                                ui.add_space(10.0,);
                                ui.vertical(|ui| {
                                    for (name, icon,) in items.iter()
                                    {
                                        if nav_item(ui, *icon, name, st.nav == idx,).clicked()
                                        {
                                            st.nav = idx;
                                        }
                                        ui.add_space(2.0,);
                                        idx += 1;
                                    }
                                },);
                            },);
                        }
                    },
                );

                w.allocate_ui_with_layout(
                    Vec2::new(full.width() - SIDEBAR_W, full.height(),),
                    egui::Layout::top_down(egui::Align::LEFT,),
                    |w| {
                        w.add_space(20.0,);
                        w.horizontal_top(|ui| {
                            ui.add_space(20.0,);
                            card(ui, Icon::Pen, "Feather", |ui| feather_card(ui, st, info.screen,),);
                            ui.add_space(16.0,);
                            card(ui, Icon::Crown, "Crown", |ui| crown_card(ui, st,),);
                        },);
                    },
                );
            },);
        },);
}

unsafe fn feather_card(ui: &mut Ui, st: &mut State, screen: (u32, u32,),) {
    row(ui, "Skoro t'ma ostypit", |ui| {
        switch(ui, &mut st.feather,);
    },);

    slider_row(ui, "Slider", &mut st.slider,);

    row(ui, "Combo", |ui| {
        egui::ComboBox::from_id_salt("menu_sample.combo",)
            .selected_text(["Option", "Other", "Third",][st.combo.min(2,) as usize],)
            .width(120.0,)
            .show_ui(ui, |ui| {
                for (i, t,) in ["Option", "Other", "Third",].iter().enumerate()
                {
                    ui.selectable_value(&mut st.combo, i as u8, *t,);
                }
            },);
    },);

    row(ui, "Input", |ui| {
        ui.add(egui::TextEdit::singleline(&mut st.input,).desired_width(120.0,).hint_text("type…",),);
    },);

    row(ui, "Color", |ui| {
        color_swatch(ui, &mut st.color, &mut st.color_open, screen,);
    },);

    ui.add_space(12.0,);
    let _ = pill_button(ui, "Button", ui.available_width(), false, 0.0,);
}

unsafe fn crown_card(ui: &mut Ui, st: &mut State,) {
    for n in 1..=11usize
    {
        let slot = &mut st.crown[n - 1];
        numbered(ui, n, slot,);
    }
}

pub unsafe fn draw_cursor(ctx: &egui::Context, pos: egui::Pos2,) {
    #[allow(deprecated)]
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("menu_sample.cursor",),),);
    let pts = vec![
        pos,
        pos + egui::vec2(0.0, 18.0,),
        pos + egui::vec2(5.0, 13.5,),
        pos + egui::vec2(9.0, 20.0,),
        pos + egui::vec2(12.5, 18.4,),
        pos + egui::vec2(8.6, 12.0,),
        pos + egui::vec2(15.0, 11.5,),
    ];
    p.add(egui::Shape::convex_polygon(pts.clone(), Color32::from_rgb(245, 248, 255,), Stroke::NONE,),);
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.0, Color32::from_rgb(20, 24, 30,),),),);
}
