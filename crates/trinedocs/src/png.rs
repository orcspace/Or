use crate::render::{Page, Primitive};
use egui::{
    Pos2, Rect, TextureId,
    epaint::{ColorImage, ImageData, Vertex},
};
use std::{collections::HashMap, sync::Arc};

const MAX_PIXELS: u64 = 32_000_000;
const MAX_TEXTURE_PIXELS: usize = 64_000_000;
const MAX_RASTER_WORK: u64 = 200_000_000;

pub fn write(page: &Page, scale: f32) -> Result<Vec<u8>, String> {
    let image = raster(page, scale)?;
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(png.into_inner())
}

fn dimensions(page: &Page, scale: f32) -> Result<(u32, u32), String> {
    if !page.width.is_finite()
        || !page.height.is_finite()
        || !scale.is_finite()
        || page.width <= 0.0
        || page.height <= 0.0
        || !(0.1..=8.0).contains(&scale)
    {
        return Err("Некорректный размер PNG".into());
    }
    let width = (page.width * scale).ceil();
    let height = (page.height * scale).ceil();
    if !width.is_finite()
        || !height.is_finite()
        || width > 8192.0
        || height > 8192.0
        || width * height > MAX_PIXELS as f32
    {
        return Err("PNG превышает 8192 пикселя по стороне или 32 млн пикселей".into());
    }
    Ok((width as u32, height as u32))
}

fn preflight(page: &Page, scale: f32) -> Result<(), String> {
    if page.items.len() > 200000 {
        return Err("Слишком много объектов PNG".into());
    }
    let mut characters = 0_usize;
    let mut vertices = 0_u64;
    let mut image_bytes = 0_usize;
    let mut image_pixels = 0_u64;
    let mut glyphs =
        std::collections::BTreeMap::<(bool, bool), std::collections::BTreeSet<char>>::new();
    for item in &page.items {
        let estimate = match item {
            Primitive::Text { text, .. } | Primitive::TextStyled { text, .. } => {
                (text.chars().count() as u64).saturating_mul(16)
            }
            Primitive::Ellipse { w, h, .. } => ((w.max(*h) * scale).max(0.0).ceil() as u64)
                .saturating_mul(16)
                .max(32),
            _ => 32,
        };
        vertices = vertices.saturating_add(estimate);
        if vertices > 1_000_000 {
            return Err("PNG превышает безопасный предел геометрии".into());
        }
        let finite = match item {
            Primitive::Text {
                x, y, text, size, ..
            }
            | Primitive::TextStyled {
                x, y, text, size, ..
            } => {
                characters = characters.saturating_add(text.chars().count());
                let style = match item {
                    Primitive::TextStyled { bold, italic, .. } => (*bold, *italic),
                    _ => (false, false),
                };
                glyphs.entry(style).or_default().extend(
                    text.chars()
                        .filter(|c| !c.is_whitespace() && !c.is_control()),
                );
                [*x, *y, *size]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
                    && *size > 0.0
                    && *size <= 600.0
            }
            Primitive::Rect { x, y, w, h, .. } | Primitive::Ellipse { x, y, w, h, .. } => {
                [*x, *y, *w, *h]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
                    && (0.0..=10000.0).contains(w)
                    && (0.0..=10000.0).contains(h)
            }
            Primitive::Line {
                x1,
                y1,
                x2,
                y2,
                width,
                ..
            } => {
                [*x1, *y1, *x2, *y2, *width]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
                    && (0.0..=300.0).contains(width)
            }
            Primitive::Image { x, y, w, h, bytes } => {
                image_bytes = image_bytes.saturating_add(bytes.len());
                if image_bytes > 64 * 1024 * 1024 {
                    return Err("Изображения PNG превышают 64 МБ".into());
                }
                let (iw, ih) = crate::slides::model::image_dimensions(bytes)?;
                image_pixels = image_pixels.saturating_add(u64::from(iw) * u64::from(ih));
                if image_pixels > 32_000_000 {
                    return Err("Изображения PNG превышают 32 млн декодированных пикселей".into());
                }
                crate::util::decode_image(bytes)?;
                [*x, *y, *w, *h]
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
                    && (0.001..=10000.0).contains(w)
                    && (0.001..=10000.0).contains(h)
            }
        };
        if !finite {
            return Err("Некорректная геометрия PNG".into());
        }
        if characters > 500000 {
            return Err("Текст PNG превышает безопасный предел глифов".into());
        }
    }
    for ((bold, italic), characters) in glyphs {
        let bytes = crate::fonts::font_bytes(bold, italic)?;
        let face = ttf_parser::Face::parse(&bytes, 0).map_err(|e| e.to_string())?;
        for c in characters {
            if face.glyph_index(c).is_none() {
                return Err(format!(
                    "Системный шрифт не содержит символ U+{:04X}; PNG не сохранён",
                    u32::from(c)
                ));
            }
        }
    }
    Ok(())
}

pub fn raster(page: &Page, scale: f32) -> Result<image::RgbaImage, String> {
    let (width, height) = dimensions(page, scale)?;
    preflight(page, scale)?;
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx)?;
    let faces: std::collections::BTreeSet<(String, bool, bool)> = page
        .items
        .iter()
        .filter_map(|item| match item {
            Primitive::TextStyled {
                font, bold, italic, ..
            } if !font.is_empty() => Some((font.clone(), *bold, *italic)),
            _ => None,
        })
        .collect();
    crate::fonts::preload(&ctx, &faces.into_iter().collect::<Vec<_>>());
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(page.width, page.height),
        )),
        max_texture_side: Some(8192),
        ..Default::default()
    };
    if let Some(viewport) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
        viewport.native_pixels_per_point = Some(scale);
    }
    let mut output = ctx.run_ui(input, |ui| {
        ui.set_clip_rect(Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(page.width, page.height),
        ));
        crate::render::paint(ui, page, Pos2::ZERO, 1.0);
    });
    if (output.pixels_per_point - scale).abs() > 0.001 {
        return Err("Не совпал масштаб PNG".into());
    }
    let mut textures = HashMap::<TextureId, Arc<ColorImage>>::new();
    for (id, deltas) in std::mem::take(&mut output.textures_delta.set) {
        for delta in deltas {
            let ImageData::Color(image) = delta.image;
            let pixels = image.size[0]
                .checked_mul(image.size[1])
                .ok_or("Неверный размер texture")?;
            if pixels != image.pixels.len() || pixels > MAX_TEXTURE_PIXELS {
                return Err("Неверная texture PNG".into());
            }
            if let Some([x, y]) = delta.pos {
                let texture = textures.get_mut(&id).ok_or("Нет исходной texture PNG")?;
                let texture = Arc::make_mut(texture);
                if x.saturating_add(image.size[0]) > texture.size[0]
                    || y.saturating_add(image.size[1]) > texture.size[1]
                {
                    return Err("Texture patch выходит за границы".into());
                }
                for row in 0..image.size[1] {
                    for col in 0..image.size[0] {
                        let source = image
                            .pixels
                            .get(row * image.size[0] + col)
                            .ok_or("Неверный texture patch")?;
                        let target = texture
                            .pixels
                            .get_mut((y + row) * texture.size[0] + x + col)
                            .ok_or("Неверный texture patch")?;
                        *target = *source;
                    }
                }
            } else {
                textures.insert(id, image);
            }
            if textures
                .values()
                .map(|texture| texture.pixels.len())
                .sum::<usize>()
                > MAX_TEXTURE_PIXELS
            {
                return Err("Textures PNG превышают безопасный предел".into());
            }
        }
    }
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let mut image = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 255, 255, 255]));
    let mut work = 0_u64;
    for primitive in primitives {
        let mesh = match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh,
            egui::epaint::Primitive::Callback(_) => {
                return Err("Shader callbacks не поддерживаются в PNG".into());
            }
        };
        if !mesh.is_valid() || mesh.indices.len() % 3 != 0 {
            return Err("Некорректная геометрия mesh PNG".into());
        }
        let texture = textures
            .get(&mesh.texture_id)
            .ok_or("Не найдена texture PNG")?;
        let clip = Rect::from_min_max(
            primitive.clip_rect.min * scale,
            primitive.clip_rect.max * scale,
        );
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let mut vertices = [
                *mesh
                    .vertices
                    .get(triangle[0] as usize)
                    .ok_or("Некорректная вершина")?,
                *mesh
                    .vertices
                    .get(triangle[1] as usize)
                    .ok_or("Некорректная вершина")?,
                *mesh
                    .vertices
                    .get(triangle[2] as usize)
                    .ok_or("Некорректная вершина")?,
            ];
            for vertex in &mut vertices {
                vertex.pos *= scale;
            }
            draw_triangle(&mut image, texture, vertices, clip, &mut work)?;
        }
    }
    Ok(image)
}

fn edge(a: Pos2, b: Pos2, point: Pos2) -> f32 {
    (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x)
}
fn top_left(a: Pos2, b: Pos2) -> bool {
    b.y < a.y || (b.y == a.y && b.x > a.x)
}
fn draw_triangle(
    image: &mut image::RgbaImage,
    texture: &ColorImage,
    mut vertices: [Vertex; 3],
    clip: Rect,
    work: &mut u64,
) -> Result<(), String> {
    if vertices.iter().any(|vertex| {
        !vertex.pos.x.is_finite()
            || !vertex.pos.y.is_finite()
            || !vertex.uv.x.is_finite()
            || !vertex.uv.y.is_finite()
    }) || ![clip.min.x, clip.min.y, clip.max.x, clip.max.y]
        .iter()
        .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
    {
        return Err("Нефинитная mesh PNG".into());
    }
    let mut area = edge(vertices[0].pos, vertices[1].pos, vertices[2].pos);
    if area.abs() < 1.0e-6 {
        return Ok(());
    }
    if area < 0.0 {
        vertices.swap(1, 2);
        area = -area;
    }
    let min_x = vertices
        .iter()
        .map(|vertex| vertex.pos.x)
        .fold(f32::INFINITY, f32::min)
        .max(clip.min.x)
        .clamp(0.0, image.width() as f32)
        .floor() as u32;
    let min_y = vertices
        .iter()
        .map(|vertex| vertex.pos.y)
        .fold(f32::INFINITY, f32::min)
        .max(clip.min.y)
        .clamp(0.0, image.height() as f32)
        .floor() as u32;
    let max_x = vertices
        .iter()
        .map(|vertex| vertex.pos.x)
        .fold(f32::NEG_INFINITY, f32::max)
        .min(clip.max.x)
        .clamp(0.0, image.width() as f32)
        .ceil() as u32;
    let max_y = vertices
        .iter()
        .map(|vertex| vertex.pos.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .min(clip.max.y)
        .clamp(0.0, image.height() as f32)
        .ceil() as u32;
    let candidates =
        u64::from(max_x.saturating_sub(min_x)) * u64::from(max_y.saturating_sub(min_y));
    *work = work.saturating_add(candidates);
    if *work > MAX_RASTER_WORK {
        return Err("PNG превышает безопасный предел работы растеризатора".into());
    }
    let inclusive = [
        top_left(vertices[1].pos, vertices[2].pos),
        top_left(vertices[2].pos, vertices[0].pos),
        top_left(vertices[0].pos, vertices[1].pos),
    ];
    let colors = vertices.map(|vertex| {
        vertex
            .color
            .to_array()
            .map(|value| f32::from(value) / 255.0)
    });
    for y in min_y..max_y {
        for x in min_x..max_x {
            let point = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
            if point.x < clip.min.x
                || point.x >= clip.max.x
                || point.y < clip.min.y
                || point.y >= clip.max.y
            {
                continue;
            }
            let distances = [
                edge(vertices[1].pos, vertices[2].pos, point),
                edge(vertices[2].pos, vertices[0].pos, point),
                edge(vertices[0].pos, vertices[1].pos, point),
            ];
            if distances
                .iter()
                .enumerate()
                .any(|(index, distance)| *distance < 0.0 || (*distance == 0.0 && !inclusive[index]))
            {
                continue;
            }
            let weights = distances.map(|distance| distance / area);
            let uv = egui::pos2(
                vertices[0].uv.x * weights[0]
                    + vertices[1].uv.x * weights[1]
                    + vertices[2].uv.x * weights[2],
                vertices[0].uv.y * weights[0]
                    + vertices[1].uv.y * weights[1]
                    + vertices[2].uv.y * weights[2],
            );
            let texel = sample(texture, uv)?;
            let color: [f32; 4] = std::array::from_fn(|channel| {
                (colors[0][channel] * weights[0]
                    + colors[1][channel] * weights[1]
                    + colors[2][channel] * weights[2])
                    * texel[channel]
            });
            let pixel = image.get_pixel_mut(x, y);
            let inverse_alpha = 1.0 - color[3].clamp(0.0, 1.0);
            for (channel, source) in color.iter().enumerate().take(3) {
                pixel.0[channel] = ((*source + f32::from(pixel.0[channel]) / 255.0 * inverse_alpha)
                    .clamp(0.0, 1.0)
                    * 255.0)
                    .round() as u8;
            }
            pixel.0[3] = 255;
        }
    }
    Ok(())
}
fn sample(texture: &ColorImage, uv: Pos2) -> Result<[f32; 4], String> {
    let [width, height] = texture.size;
    if width == 0 || height == 0 {
        return Err("Пустая texture PNG".into());
    }
    let x = (uv.x * width as f32 - 0.5).clamp(0.0, (width - 1) as f32);
    let y = (uv.y * height as f32 - 0.5).clamp(0.0, (height - 1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let color = |x: usize, y: usize| -> Result<[f32; 4], String> {
        Ok(texture
            .pixels
            .get(y * width + x)
            .ok_or("Неверный texel PNG")?
            .to_array()
            .map(|value| f32::from(value) / 255.0))
    };
    let a = color(x0, y0)?;
    let b = color(x1, y0)?;
    let c = color(x0, y1)?;
    let d = color(x1, y1)?;
    let sx = x - x0 as f32;
    let sy = y - y0 as f32;
    Ok(std::array::from_fn(|channel| {
        (a[channel] * (1.0 - sx) + b[channel] * sx) * (1.0 - sy)
            + (c[channel] * (1.0 - sx) + d[channel] * sx) * sy
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_dimensions_shapes_and_glyphs_are_real() -> Result<(), String> {
        let page = Page {
            width: 160.0,
            height: 80.0,
            items: vec![
                Primitive::Rect {
                    x: 5.0,
                    y: 5.0,
                    w: 20.0,
                    h: 20.0,
                    color: [230, 20, 40],
                },
                Primitive::TextStyled {
                    x: 45.0,
                    y: 8.0,
                    text: "PNG".into(),
                    size: 24.0,
                    color: [0, 0, 0],
                    bold: true,
                    italic: false,
                },
            ],
        };
        let bytes = write(&page, 2.0)?;
        let image = image::load_from_memory(&bytes)
            .map_err(|error| error.to_string())?
            .to_rgba8();
        assert_eq!(image.dimensions(), (320, 160));
        assert_eq!(image.get_pixel(20, 20).0, [230, 20, 40, 255]);
        assert!(image.enumerate_pixels().any(|(x, y, pixel)| x > 90
            && x < 300
            && y < 100
            && pixel.0[0] < 100
            && pixel.0[1] < 100
            && pixel.0[2] < 100));
        assert_eq!(image.get_pixel(315, 155).0, [255, 255, 255, 255]);
        Ok(())
    }
    #[test]
    fn png_images_composite_alpha_on_background() -> Result<(), String> {
        let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 128]));
        let mut encoded = std::io::Cursor::new(Vec::new());
        source
            .write_to(&mut encoded, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let page = Page {
            width: 20.0,
            height: 20.0,
            items: vec![
                Primitive::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 20.0,
                    h: 20.0,
                    color: [0, 0, 255],
                },
                Primitive::Image {
                    x: 0.0,
                    y: 0.0,
                    w: 20.0,
                    h: 20.0,
                    bytes: encoded.into_inner(),
                },
            ],
        };
        let image = raster(&page, 1.0)?;
        let pixel = image.get_pixel(10, 10).0;
        assert!(pixel[0] >= 120 && pixel[0] <= 136);
        assert_eq!(pixel[1], 0);
        assert!(pixel[2] >= 120 && pixel[2] <= 136);
        assert_eq!(pixel[3], 255);
        Ok(())
    }
    #[test]
    fn png_rejects_unbounded_dimensions() {
        assert!(
            dimensions(
                &Page {
                    width: 10000.0,
                    height: 10000.0,
                    items: vec![]
                },
                2.0
            )
            .is_err()
        );
        assert!(
            dimensions(
                &Page {
                    width: f32::NAN,
                    height: 20.0,
                    items: vec![]
                },
                2.0
            )
            .is_err()
        );
    }
}
