use crate::render::{Page, Primitive};
use pdf_writer::{Content, Name, Pdf, Rect, Ref, Str};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
struct Font {
    name: Vec<u8>,
    id: Ref,
    map: BTreeMap<char, u16>,
    ascent: f32,
}
/// Font family (empty = default) and style index (bold + 2 * italic).
type FontKey = (String, usize);
fn requested(item: &Primitive) -> FontKey {
    match item {
        Primitive::TextStyled {
            bold, italic, font, ..
        } => (
            if crate::fonts::is_installed(font) {
                font.clone()
            } else {
                String::new()
            },
            usize::from(*bold) + 2 * usize::from(*italic),
        ),
        _ => (String::new(), 0),
    }
}
fn load_face(key: &FontKey) -> Result<(Arc<Vec<u8>>, u32), String> {
    let family = if key.0.is_empty() {
        crate::fonts::DEFAULT_FAMILY
    } else {
        &key.0
    };
    crate::fonts::face(family, key.1 & 1 != 0, key.1 & 2 != 0)
}
fn text(item: &Primitive) -> Option<&str> {
    match item {
        Primitive::Text { text, .. } | Primitive::TextStyled { text, .. } => Some(text),
        _ => None,
    }
}
pub fn write(pages: &[Page]) -> Result<Vec<u8>, String> {
    if pages.is_empty() || pages.len() > 10000 {
        return Err("Недопустимое количество страниц PDF".into());
    }
    validate_pages(pages)?;
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let tree = Ref::new(2);
    let mut next = 3;
    let mut alloc = || {
        let r = Ref::new(next);
        next += 1;
        r
    };
    // Text whose font lacks a glyph falls back to the default font of the same style.
    let mut faces: BTreeMap<FontKey, (Arc<Vec<u8>>, u32)> = BTreeMap::new();
    let mut keys = Vec::new();
    for item in pages.iter().flat_map(|p| p.items.iter()) {
        let Some(t) = text(item) else {
            keys.push((String::new(), 0));
            continue;
        };
        let mut key = requested(item);
        if !key.0.is_empty() {
            if !faces.contains_key(&key) {
                faces.insert(key.clone(), load_face(&key)?);
            }
            let covered = faces.get(&key).is_some_and(|(bytes, index)| {
                ttf_parser::Face::parse(bytes, *index).is_ok_and(|face| {
                    t.chars()
                        .filter(|c| !c.is_control())
                        .all(|c| face.glyph_index(c).is_some())
                })
            });
            if !covered {
                key.0.clear();
            }
        }
        keys.push(key);
    }
    let mut slots: BTreeMap<FontKey, usize> = BTreeMap::new();
    let mut fonts = Vec::new();
    let used_keys: BTreeSet<FontKey> = keys.iter().cloned().collect();
    for (index, key) in used_keys.into_iter().enumerate() {
        let used: BTreeSet<char> = pages
            .iter()
            .flat_map(|p| p.items.iter())
            .zip(&keys)
            .filter(|(_, k)| **k == key)
            .filter_map(|(i, _)| text(i))
            .flat_map(str::chars)
            .filter(|c| !c.is_control())
            .collect();
        let (bytes, face_index) = match faces.get(&key) {
            Some(face) => face.clone(),
            None => load_face(&key)?,
        };
        let face = ttf_parser::Face::parse(&bytes, face_index)
            .map_err(|e| format!("Повреждённый шрифт: {e}"))?;
        let units = f32::from(face.units_per_em());
        let mut mapper = subsetter::GlyphRemapper::new();
        let mut map = BTreeMap::new();
        let mut widths = BTreeMap::new();
        let mut unicode = BTreeMap::new();
        for ch in used {
            let glyph = face.glyph_index(ch).ok_or_else(|| {
                format!(
                    "Системный шрифт не содержит U+{:04X}; PDF отменён",
                    ch as u32
                )
            })?;
            let cid = mapper.remap(glyph.0);
            map.insert(ch, cid);
            widths.insert(
                cid,
                f32::from(face.glyph_hor_advance(glyph).unwrap_or_default()) * 1000.0 / units,
            );
            unicode.entry(cid).or_insert(ch);
        }
        let subset = subsetter::subset(&bytes, face_index, &mapper)
            .map_err(|e| format!("Не удалось встроить шрифт: {e}"))?;
        let id = alloc();
        let descendant = alloc();
        let descriptor = alloc();
        let data = alloc();
        let cmap = alloc();
        let name = format!("F{}", index + 1).into_bytes();
        let family: String = if key.0.is_empty() {
            crate::fonts::DEFAULT_FAMILY.into()
        } else {
            key.0.chars().filter(char::is_ascii_alphanumeric).collect()
        };
        let base = format!(
            "TRINE{}+{family}{}",
            (b'A' + (index % 26) as u8) as char,
            key.1
        )
        .into_bytes();
        let bbox = face.global_bounding_box();
        let mut desc = pdf.font_descriptor(descriptor);
        desc.name(Name(&base))
            .flags(pdf_writer::types::FontFlags::SYMBOLIC)
            .bbox(Rect::new(
                f32::from(bbox.x_min) * 1000.0 / units,
                f32::from(bbox.y_min) * 1000.0 / units,
                f32::from(bbox.x_max) * 1000.0 / units,
                f32::from(bbox.y_max) * 1000.0 / units,
            ))
            .italic_angle(if key.1 & 2 != 0 { -12.0 } else { 0.0 })
            .ascent(f32::from(face.ascender()) * 1000.0 / units)
            .descent(f32::from(face.descender()) * 1000.0 / units)
            .cap_height(700.0)
            .stem_v(80.0)
            .font_file2(data);
        drop(desc);
        pdf.stream(data, &subset)
            .pair(Name(b"Length1"), subset.len() as i32);
        let mut cid = pdf.cid_font(descendant);
        cid.subtype(pdf_writer::types::CidFontType::Type2)
            .base_font(Name(&base))
            .font_descriptor(descriptor)
            .system_info(pdf_writer::types::SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            })
            .cid_to_gid_map_predefined(Name(b"Identity"))
            .default_width(500.0);
        {
            let mut all_widths = cid.widths();
            for (g, w) in widths {
                all_widths.same(g, g, w);
            }
        }
        drop(cid);
        pdf.type0_font(id)
            .base_font(Name(&base))
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(descendant)
            .to_unicode(cmap);
        let mut mapping = String::from(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /TrineUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
        );
        let pairs: Vec<_> = unicode.into_iter().collect();
        for chunk in pairs.chunks(100) {
            mapping.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (gid, ch) in chunk {
                let mut buf = [0; 2];
                let code = ch
                    .encode_utf16(&mut buf)
                    .iter()
                    .map(|c| format!("{c:04X}"))
                    .collect::<String>();
                mapping.push_str(&format!("<{gid:04X}> <{code}>\n"));
            }
            mapping.push_str("endbfchar\n");
        }
        mapping.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
        pdf.stream(cmap, mapping.as_bytes());
        slots.insert(key, fonts.len());
        fonts.push(Font {
            name,
            id,
            map,
            ascent: f32::from(face.ascender()) / units,
        });
    }
    let mut kids = Vec::new();
    let mut page_keys = keys.iter();
    for page in pages {
        if !page.width.is_finite()
            || !page.height.is_finite()
            || page.width <= 0.0
            || page.height <= 0.0
        {
            return Err("Недопустимый размер PDF".into());
        }
        let page_id = alloc();
        let content_id = alloc();
        kids.push(page_id);
        let mut content = Content::new();
        let mut images = Vec::new();
        for (item, key) in page.items.iter().zip(page_keys.by_ref()) {
            let rgb = |c: [u8; 3]| {
                [
                    f32::from(c[0]) / 255.0,
                    f32::from(c[1]) / 255.0,
                    f32::from(c[2]) / 255.0,
                ]
            };
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
                    let f = slots
                        .get(key)
                        .and_then(|slot| fonts.get(*slot))
                        .ok_or("Шрифт PDF отсутствует")?;
                    let c = rgb(*color);
                    content.set_fill_rgb(c[0], c[1], c[2]);
                    for (line, t) in text.lines().enumerate() {
                        let encoded: Vec<u8> = t
                            .chars()
                            .filter(|c| !c.is_control())
                            .filter_map(|c| f.map.get(&c))
                            .flat_map(|id| id.to_be_bytes())
                            .collect();
                        content.begin_text();
                        content.set_font(Name(&f.name), *size);
                        content.next_line(
                            *x,
                            page.height - y - size * f.ascent - (line as f32) * size * 1.2,
                        );
                        content.show(Str(&encoded));
                        content.end_text();
                    }
                }
                Primitive::Rect { x, y, w, h, color } => {
                    let c = rgb(*color);
                    content.set_fill_rgb(c[0], c[1], c[2]);
                    content.rect(*x, page.height - y - h, *w, *h);
                    content.fill_nonzero();
                }
                Primitive::Ellipse { x, y, w, h, color } => {
                    let c = rgb(*color);
                    content.set_fill_rgb(c[0], c[1], c[2]);
                    let cx = x + w / 2.0;
                    let cy = page.height - y - h / 2.0;
                    let rx = w / 2.0;
                    let ry = h / 2.0;
                    let k = 0.5522848;
                    content.move_to(cx + rx, cy);
                    content.cubic_to(cx + rx, cy + k * ry, cx + k * rx, cy + ry, cx, cy + ry);
                    content.cubic_to(cx - k * rx, cy + ry, cx - rx, cy + k * ry, cx - rx, cy);
                    content.cubic_to(cx - rx, cy - k * ry, cx - k * rx, cy - ry, cx, cy - ry);
                    content.cubic_to(cx + k * rx, cy - ry, cx + rx, cy - k * ry, cx + rx, cy);
                    content.fill_nonzero();
                }
                Primitive::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    width,
                    color,
                } => {
                    let c = rgb(*color);
                    content.set_stroke_rgb(c[0], c[1], c[2]);
                    content.set_line_width(*width);
                    content.move_to(*x1, page.height - y1);
                    content.line_to(*x2, page.height - y2);
                    content.stroke();
                }
                Primitive::Image { x, y, w, h, bytes } => {
                    let image = crate::util::decode_image(bytes)?.to_rgba8();
                    if u64::from(image.width()) * u64::from(image.height()) > 32_000_000 {
                        return Err("Изображение слишком велико".into());
                    }
                    let id = alloc();
                    let mask = alloc();
                    let name = format!("I{}", images.len() + 1).into_bytes();
                    let mut rgb = Vec::new();
                    let mut alpha = Vec::new();
                    for p in image.pixels() {
                        rgb.extend_from_slice(&p.0[..3]);
                        alpha.push(p.0[3]);
                    }
                    pdf.image_xobject(mask, &alpha)
                        .width(image.width() as i32)
                        .height(image.height() as i32)
                        .bits_per_component(8)
                        .color_space()
                        .device_gray();
                    {
                        let mut img = pdf.image_xobject(id, &rgb);
                        img.width(image.width() as i32)
                            .height(image.height() as i32)
                            .bits_per_component(8)
                            .s_mask(mask);
                        img.color_space().device_rgb();
                    }
                    content.save_state();
                    content.transform([*w, 0.0, 0.0, *h, *x, page.height - y - h]);
                    content.x_object(Name(&name));
                    content.restore_state();
                    images.push((name, id));
                }
            }
        }
        pdf.stream(content_id, &content.finish());
        let mut p = pdf.page(page_id);
        p.parent(tree)
            .media_box(Rect::new(0.0, 0.0, page.width, page.height))
            .contents(content_id);
        let mut resources = p.resources();
        {
            let mut dictionary = resources.fonts();
            for f in &fonts {
                dictionary.pair(Name(&f.name), f.id);
            }
        }
        {
            let mut dictionary = resources.x_objects();
            for (name, id) in &images {
                dictionary.pair(Name(name), *id);
            }
        }
    }
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids(kids.iter().copied())
        .count(kids.len() as i32);
    let mut bytes = pdf.finish();
    bytes.push(b'\n');
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_pdf() -> Result<(), String> {
        let b = write(&[Page {
            width: 595.0,
            height: 842.0,
            items: vec![Primitive::Text {
                x: 50.0,
                y: 50.0,
                text: "Привет, World".into(),
                size: 12.0,
                color: [0, 0, 0],
            }],
        }])?;
        assert!(b.starts_with(b"%PDF-"));
        assert!(b.windows(10).any(|b| b == b"/ToUnicode"));
        assert!(b.ends_with(b"%%EOF\n"));
        Ok(())
    }
}

fn validate_pages(pages: &[Page]) -> Result<(), String> {
    let mut objects = 0usize;
    let mut text_bytes = 0usize;
    let mut image_pixels = 0u64;
    for page in pages {
        if !page.width.is_finite()
            || !page.height.is_finite()
            || page.width <= 0.0
            || page.height <= 0.0
            || page.width > 20000.0
            || page.height > 20000.0
        {
            return Err("Недопустимый размер PDF".into());
        }
        objects = objects.saturating_add(page.items.len());
        if objects > 1_000_000 {
            return Err("PDF превышает 1 млн объектов".into());
        }
        for item in &page.items {
            let positive = |v: f32| v.is_finite() && v > 0.0 && v <= 20000.0;
            let nonnegative = |v: f32| v.is_finite() && (0.0..=20000.0).contains(&v);
            let coord = |v: f32| v.is_finite() && v.abs() <= 1_000_000.0;
            let valid = match item {
                Primitive::Text {
                    x, y, size, text, ..
                }
                | Primitive::TextStyled {
                    x, y, size, text, ..
                } => {
                    text_bytes = text_bytes.saturating_add(text.len());
                    coord(*x) && coord(*y) && positive(*size)
                }
                Primitive::Rect { x, y, w, h, .. } | Primitive::Ellipse { x, y, w, h, .. } => {
                    coord(*x) && coord(*y) && nonnegative(*w) && nonnegative(*h)
                }
                Primitive::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    width,
                    ..
                } => coord(*x1) && coord(*x2) && coord(*y1) && coord(*y2) && nonnegative(*width),
                Primitive::Image { x, y, w, h, bytes } => {
                    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                        .with_guessed_format()
                        .map_err(|e| e.to_string())?;
                    let (iw, ih) = reader.into_dimensions().map_err(|e| e.to_string())?;
                    image_pixels = image_pixels.saturating_add(u64::from(iw) * u64::from(ih));
                    coord(*x) && coord(*y) && positive(*w) && positive(*h) && iw > 0 && ih > 0
                }
            };
            if !valid {
                return Err("Недопустимая геометрия объекта PDF".into());
            }
            if text_bytes > 16 * 1024 * 1024 || image_pixels > 32_000_000 {
                return Err("PDF превышает общий лимит текста/изображений".into());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod limit_tests {
    use super::*;
    #[test]
    fn invalid_geometry_before_font_or_stream() {
        let p = Page {
            width: 595.0,
            height: 842.0,
            items: vec![Primitive::Line {
                x1: f32::NAN,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0,
                width: 1.0,
                color: [0, 0, 0],
            }],
        };
        assert!(write(&[p]).is_err());
    }
}
