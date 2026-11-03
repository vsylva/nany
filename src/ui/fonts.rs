use std::sync::Arc;

use crate::{log_info, log_warn};

pub unsafe fn install_font(ctx: &egui::Context, path: &str,) -> bool {
    let Ok(bytes,) = std::fs::read(path,)
    else
    {
        log_warn!("ui", format!("font not found path={path}"));
        return false;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("user".to_owned(), Arc::new(egui::FontData::from_owned(bytes,),),);
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace,]
    {
        fonts.families.entry(fam,).or_default().insert(0, "user".to_owned(),);
    }
    ctx.set_fonts(fonts,);
    log_info!("ui", format!("font loaded path={path}"));
    true
}

pub unsafe fn install_cjk(ctx: &egui::Context,) -> bool {
    for path in [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ]
    {
        if install_font(ctx, path,)
        {
            return true;
        }
    }
    false
}
