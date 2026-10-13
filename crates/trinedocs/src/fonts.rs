pub fn font_bytes(bold: bool, italic: bool) -> Result<Vec<u8>, String> {
    let root = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:/Windows".into())
        .join("Fonts");
    let names = match (bold, italic) {
        (true, true) => ["arialbi.ttf", "segoeuiz.ttf"],
        (true, false) => ["arialbd.ttf", "segoeuib.ttf"],
        (false, true) => ["ariali.ttf", "segoeuii.ttf"],
        _ => ["arial.ttf", "segoeui.ttf"],
    };
    for name in names {
        if let Ok(bytes) = std::fs::read(root.join(name)) {
            return Ok(bytes);
        }
    }
    for name in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
    ] {
        if let Ok(b) = std::fs::read(name) {
            return Ok(b);
        }
    }
    Err("Не найден системный шрифт Arial / Segoe UI".into())
}
pub fn install(ctx: &egui::Context) -> Result<(), String> {
    let mut fonts = egui::FontDefinitions::empty();
    for (name, bold, italic) in [
        ("system", false, false),
        ("bold", true, false),
        ("italic", false, true),
        ("bold-italic", true, true),
    ] {
        fonts.font_data.insert(
            name.into(),
            egui::FontData::from_owned(font_bytes(bold, italic)?).into(),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), vec![name.into()]);
    }
    fonts
        .families
        .insert(egui::FontFamily::Proportional, vec!["system".into()]);
    fonts
        .families
        .insert(egui::FontFamily::Monospace, vec!["system".into()]);
    ctx.set_fonts(fonts);
    Ok(())
}
