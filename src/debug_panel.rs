use crate::log;

pub unsafe fn set_enabled(on: bool,) {
    ENABLED.store(on, std::sync::atomic::Ordering::Relaxed,);
}

pub unsafe fn enabled() -> bool {
    ENABLED.load(std::sync::atomic::Ordering::Relaxed,)
}

static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(cfg!(debug_assertions),);
pub struct Info {
    pub backend: &'static str,
    pub width:   u32,
    pub height:  u32,

    pub frame_ms:   f32,
    pub frames:     u64,
    pub capture:    bool,
    pub swapchains: usize,
    pub hooks_ok:   bool,

    pub node_mask: Option<u32,>,
}

unsafe fn hex_opt(v: Option<u32,>,) -> String {
    match v
    {
        Some(x,) => format!("0x{x:X}"),
        None => "—".to_string(),
    }
}

pub unsafe fn draw(ctx: &egui::Context, info: &Info,) {
    egui::Window::new("nany · debug",)
        .title_bar(false,)
        .resizable(false,)
        .collapsible(false,)
        .fixed_pos(egui::pos2(info.width as f32 - 232.0, info.height as f32 - 188.0,),)
        .fixed_size(egui::vec2(220.0, 176.0,),)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("{}  {}x{}", info.backend, info.width, info.height),).size(12.0,).strong(),
            );
            ui.separator();
            let mut kv = |k: &str, v: String| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(k,).size(11.0,).weak(),);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center,), |ui| {
                        ui.label(egui::RichText::new(v,).size(11.0,),);
                    },);
                },);
            };
            kv("frame", format!("{:.2} ms", info.frame_ms),);
            kv("frames", info.frames.to_string(),);
            kv("swapchains", info.swapchains.to_string(),);
            kv("capture", (info.capture as u8).to_string(),);
            kv("hooks", if info.hooks_ok { "ok".into() } else { "down".into() },);
            kv("nodemask", hex_opt(info.node_mask,),);
            ui.separator();
            if ui.small_button("copy log path",).clicked()
            {
                ui.ctx().copy_text(log::path(),);
            }
        },);
}
