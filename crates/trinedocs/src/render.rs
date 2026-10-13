#[derive(Clone, Debug, Default)]
pub struct Page {
    pub width: f32,
    pub height: f32,
    pub items: Vec<Primitive>,
}
#[derive(Clone, Debug)]
pub enum Primitive {
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        color: [u8; 3],
    },
    TextStyled {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        color: [u8; 3],
        bold: bool,
        italic: bool,
        /// Font family name; empty means the default system font.
        font: String,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [u8; 3],
    },
    Ellipse {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [u8; 3],
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: [u8; 3],
    },
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        bytes: Vec<u8>,
    },
}
pub fn paint(ui: &mut egui::Ui, page: &Page, origin: egui::Pos2, scale: f32) {
    let p = ui.painter();
    let point = |x: f32, y: f32| origin + egui::vec2(x * scale, y * scale);
    let rgb = |c: [u8; 3]| egui::Color32::from_rgb(c[0], c[1], c[2]);
    for item in &page.items {
        match item {
            Primitive::Text {
                x,
                y,
                text,
                size,
                color,
            }
            | Primitive::TextStyled {
                x,
                y,
                text,
                size,
                color,
                ..
            } => {
                let family = match item {
                    Primitive::TextStyled {
                        bold, italic, font, ..
                    } if !font.is_empty() => {
                        crate::fonts::egui_family(ui.ctx(), font, *bold, *italic)
                    }
                    Primitive::TextStyled {
                        bold: true,
                        italic: true,
                        ..
                    } => egui::FontFamily::Name("bold-italic".into()),
                    Primitive::TextStyled { bold: true, .. } => {
                        egui::FontFamily::Name("bold".into())
                    }
                    Primitive::TextStyled { italic: true, .. } => {
                        egui::FontFamily::Name("italic".into())
                    }
                    _ => egui::FontFamily::Name("system".into()),
                };
                p.text(
                    point(*x, *y),
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::new(size * scale, family),
                    rgb(*color),
                );
            }
            Primitive::Rect { x, y, w, h, color } => {
                p.rect_filled(
                    egui::Rect::from_min_size(point(*x, *y), egui::vec2(w * scale, h * scale)),
                    0.0,
                    rgb(*color),
                );
            }
            Primitive::Ellipse { x, y, w, h, color } => {
                p.add(egui::Shape::ellipse_filled(
                    point(x + w / 2.0, y + h / 2.0),
                    egui::vec2(w * scale / 2.0, h * scale / 2.0),
                    rgb(*color),
                ));
            }
            Primitive::Line {
                x1,
                y1,
                x2,
                y2,
                width,
                color,
            } => {
                p.line_segment(
                    [point(*x1, *y1), point(*x2, *y2)],
                    egui::Stroke::new(width * scale, rgb(*color)),
                );
            }
            Primitive::Image { x, y, w, h, bytes } => {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut hasher);
                let key = egui::Id::new(("asset", hasher.finish()));
                let cached = ui
                    .ctx()
                    .data_mut(|d| d.get_temp::<egui::TextureHandle>(key));
                let tex = cached.or_else(|| {
                    crate::util::decode_image(bytes).ok().map(|img| {
                        let rgba = img.to_rgba8();
                        let tex = ui.ctx().load_texture(
                            format!("asset-{:?}", key),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [rgba.width() as usize, rgba.height() as usize],
                                rgba.as_raw(),
                            ),
                            egui::TextureOptions::LINEAR,
                        );
                        ui.ctx().data_mut(|d| d.insert_temp(key, tex.clone()));
                        tex
                    })
                });
                if let Some(tex) = tex {
                    p.image(
                        tex.id(),
                        egui::Rect::from_min_size(point(*x, *y), egui::vec2(w * scale, h * scale)),
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
    }
}
