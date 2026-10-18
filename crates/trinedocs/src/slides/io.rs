use super::model::{Animation, Deck, Element, ElementKind, Slide};
use crate::package::{read_zip, write_zip, xml_escape};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const GROUP: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";

pub fn read(bytes: &[u8], ext: &str) -> Result<Deck, String> {
    read_with_limit(bytes,ext,crate::package::MAX_FILE)
}
fn read_with_limit(bytes:&[u8],ext:&str,limit:usize)->Result<Deck,String> {
    if bytes.len()>limit {return Err("Файл презентации превышает безопасный размер".into());}
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "pptx" | "ppsx" | "potx" | "pptm" => read_pptx(bytes),
        "trineslide" => match crate::native::decode(bytes)? {
            crate::native::Document::Slides(deck) => Ok(deck),
            _ => Err("Нативный файл содержит другой тип документа".into()),
        },
        "json" => {
            let deck: Deck = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            deck.validate()?;
            Ok(deck)
        }
        _ => Err(format!("Формат .{ext} не поддерживается")),
    }
}
pub fn write(deck: &Deck, ext: &str) -> Result<Vec<u8>, String> {
    deck.validate()?;
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "pptx" => write_pptx(deck),
        "trineslide" => crate::native::encode(&crate::native::Document::Slides(deck.clone())),
        "json" => serde_json::to_vec_pretty(deck).map_err(|e| e.to_string()),
        "md" => Ok(markdown_deck(deck).into_bytes()),
        "txt" => Ok(deck
            .slides
            .iter()
            .map(|s| {
                format!(
                    "# {}\n{}\n\n{}",
                    s.title,
                    s.elements
                        .iter()
                        .map(element_text)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    s.notes
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
            .into_bytes()),
        "html" => {
            let body = deck
                .slides
                .iter()
                .map(|s| {
                    format!(
                        "<section><h1>{}</h1>{}<aside>{}</aside></section>",
                        xml_escape(&s.title),
                        s.elements.iter().map(element_html).collect::<String>(),
                        xml_escape(&s.notes)
                    )
                })
                .collect::<String>();
            Ok(format!("<!doctype html><html lang=\"ru\"><meta charset=\"utf-8\"><title>TrineDocs Slides</title><style>section{{min-height:80vh;padding:4rem;break-after:page}}p,td,aside{{white-space:pre-wrap}}table{{border-collapse:collapse}}td{{border:1px solid #ccc;padding:.5rem}}aside{{color:#666}}</style><body>{body}</body></html>").into_bytes())
        }
        _ => Err(format!("Экспорт .{ext} ещё не поддерживается")),
    }
}
pub(crate) fn element_text(element: &Element) -> String {
    let content = match &element.kind {
        ElementKind::Table { cells } => cells
            .iter()
            .map(|row| row.join("\t"))
            .collect::<Vec<_>>()
            .join("\n"),
        ElementKind::Chart { values, labels } => (0..values.len().max(labels.len()))
            .map(|index| {
                format!(
                    "{}\t{}",
                    labels
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| (index + 1).to_string()),
                    values
                        .get(index)
                        .map(ToString::to_string)
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    if content.is_empty() {
        element.text.clone()
    } else if element.text.is_empty() {
        content
    } else {
        format!("{}\n{content}", element.text)
    }
}
fn image_data_uri(bytes:&[u8]) -> String {
    const ALPHABET:&[u8;64]=b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mime=if matches!(image::guess_format(bytes),Ok(image::ImageFormat::Jpeg)) {"image/jpeg"}else{"image/png"};
    let mut out=format!("data:{mime};base64,");
    for chunk in bytes.chunks(3) {
        let n=(u32::from(chunk[0])<<16)|(u32::from(*chunk.get(1).unwrap_or(&0))<<8)|u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(ALPHABET[((n>>18)&63)as usize]as char);out.push(ALPHABET[((n>>12)&63)as usize]as char);
        out.push(if chunk.len()>1 {ALPHABET[((n>>6)&63)as usize]as char}else{'='});
        out.push(if chunk.len()>2 {ALPHABET[(n&63)as usize]as char}else{'='});
    }
    out
}
fn markdown_literal(text:&str)->String {
    let mut out=String::new();
    for ch in text.chars() {
        if ch.is_ascii_punctuation() {out.push('\\');}out.push(ch);
    }
    out.replace("\r\n","\n").replace('\r',"\n").replace('\n',"  \n")
}
fn markdown_deck(deck:&Deck)->String {
    deck.slides.iter().map(|slide| {
        let title=markdown_literal(&slide.title.replace(['\r','\n']," "));
        let body=slide.elements.iter().map(|element| {
            if let ElementKind::Image{bytes}=&element.kind {
                format!("![{}]({})",markdown_literal(&element.text),image_data_uri(bytes))
            }else {markdown_literal(&element_text(element))}
        }).collect::<Vec<_>>().join("\n\n");
        format!("# {title}\n\n{body}\n\n{}",markdown_literal(&slide.notes))
    }).collect::<Vec<_>>().join("\n\n---\n\n")
}
fn element_html(element: &Element) -> String {
    let font=element.font.replace('\\',"\\\\").replace('\'',"\\'");
    let style=format!("font-family:'{font}';font-size:{}pt;font-weight:{};font-style:{};color:#{}",
        element.font_size,if element.bold {"bold"}else{"normal"},if element.italic {"italic"}else{"normal"},hex(element.color));
    format!("<div style=\"{}\">{}</div>",xml_escape(&style),element_html_content(element))
}
fn element_html_content(element: &Element) -> String {
    if let ElementKind::Image{bytes}=&element.kind {
        return format!("<figure><img src=\"{}\" alt=\"{}\" style=\"max-width:100%;width:{}pt;height:{}pt\"></figure>",
            image_data_uri(bytes),xml_escape(&element.text),element.w,element.h);
    }
    if let ElementKind::Table { cells } = &element.kind {
        let rows = cells
            .iter()
            .map(|row| {
                format!(
                    "<tr>{}</tr>",
                    row.iter()
                        .map(|cell| format!("<td>{}</td>", xml_escape(cell)))
                        .collect::<String>()
                )
            })
            .collect::<String>();
        format!("<p>{}</p><table>{rows}</table>", xml_escape(&element.text))
    } else {
        format!("<p>{}</p>", xml_escape(&element_text(element)))
    }
}
#[derive(Default, Debug)]
struct Node {
    name: String,
    attrs: BTreeMap<String, String>,
    namespaces: BTreeMap<String, String>,
    text: String,
    children: Vec<Node>,
}
impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|n| n.name == name)
    }
    fn descend(&self, name: &str) -> Option<&Node> {
        if self.name == name {
            Some(self)
        } else {
            self.children.iter().find_map(|n| n.descend(name))
        }
    }
    fn all<'a>(&'a self, name: &str, out: &mut Vec<&'a Node>) {
        if self.name == name {
            out.push(self);
        }
        for node in &self.children {
            node.all(name, out);
        }
    }
    fn attr(&self, key: &str) -> &str {
        self.attrs.get(key).map(String::as_str).unwrap_or("")
    }
    fn number(&self, key: &str, fallback: f32) -> f32 {
        self.attr(key)
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .unwrap_or(fallback)
    }
    fn text_content(&self) -> String {
        match self.name.as_str() {
            "t" => self.text.clone(),
            "br" => "\n".into(),
            _ => self.children.iter().map(Node::text_content).collect(),
        }
    }
    fn paragraph_text(&self) -> String {
        let mut paragraphs = Vec::new();
        self.all("p", &mut paragraphs);
        paragraphs
            .iter()
            .map(|node| node.text_content())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}
fn parse(bytes: &[u8]) -> Result<Node, String> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("XML превышает 16 МБ".into());
    }
    let xml = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_str(xml);
    let mut stack = vec![Node::default()];
    let mut count = 0_usize;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("Некорректный XML: {e}"))?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                count += 1;
                if count > 500000 || stack.len() > 64 {
                    return Err("XML слишком сложный".into());
                }
                let mut node = Node {
                    name: local(e.name().as_ref()).into(),
                    ..Node::default()
                };
                let attributes = e
                    .attributes()
                    .map(|attr| {
                        let attr = attr.map_err(|e| e.to_string())?;
                        let key = attr.key.as_ref().to_string();
                        let value = attr
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(|e| e.to_string())?
                            .into_owned();
                        Ok((key, value))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                for (key, value) in &attributes {
                    if let Some(prefix) = key.strip_prefix("xmlns:") {
                        node.namespaces.insert(prefix.into(), value.clone());
                    }
                }
                for (key, value) in &attributes {
                    node.attrs.insert(key.clone(), value.clone());
                    node.attrs
                        .entry(local(key).into())
                        .or_insert_with(|| value.clone());
                }
                for (key, value) in attributes {
                    if let Some((prefix, name)) = key.split_once(':')
                        && node.namespaces.get(prefix).or_else(|| {
                            stack.iter().rev().find_map(|parent| parent.namespaces.get(prefix))
                        }).is_some_and(|uri| matches!(uri.as_str(),
                            "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                            | "http://purl.oclc.org/ooxml/officeDocument/relationships"))
                    {
                        node.attrs.insert(format!("r:{name}"), value);
                    }
                }
                if empty {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err("Лишний закрывающий XML тег".into());
                }
                if let Some(node) = stack.pop()
                    && let Some(parent) = stack.last_mut()
                {
                    parent.children.push(node);
                }
            }
            Event::Text(e) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&e.xml10_content());
                }
            }
            Event::CData(e) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(e.as_ref());
                }
            }
            Event::GeneralRef(e) => {
                let escaped = format!("&{};", e.as_ref());
                let value = quick_xml::escape::unescape(&escaped).map_err(|e| e.to_string())?;
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&value);
                }
            }
            Event::DocType(_) => return Err("DTD и внешние XML сущности запрещены".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("XML не закрыт".into());
    }
    let root = stack.pop().ok_or("Пустой XML")?;
    if !root.text.trim().is_empty() {
        return Err("XML содержит текст вне корневого элемента".into());
    }
    let mut children = root.children.into_iter();
    let node = children.next().ok_or("Пустой XML")?;
    if children.next().is_some() {
        return Err("XML должен содержать ровно один корневой элемент".into());
    }
    Ok(node)
}
fn relationships(bytes: Option<&Vec<u8>>, base: &str) -> Result<BTreeMap<String, String>, String> {
    relationships_kind(bytes, base, None)
}
fn relationships_kind(
    bytes: Option<&Vec<u8>>, base: &str, kind: Option<&str>,
) -> Result<BTreeMap<String, String>, String> {
    let mut rels = BTreeMap::new();
    if let Some(bytes) = bytes {
        for rel in &parse(bytes)?.children {
            if rel.name != "Relationship" || rel.attr("TargetMode") == "External"
                || kind.is_some_and(|kind| !rel.attr("Type").ends_with(&format!("/{kind}")))
            {
                continue;
            }
            let mut components: Vec<&str> = if rel.attr("Target").starts_with('/') {
                Vec::new()
            } else {
                base.split('/').filter(|v| !v.is_empty()).collect()
            };
            for component in rel.attr("Target").trim_start_matches('/').split('/') {
                match component {
                    "." | "" => {}
                    ".." => {
                        if components.pop().is_none() {
                            return Err("Relationship выходит из архива".into());
                        }
                    }
                    value => components.push(value),
                }
            }
            rels.insert(rel.attr("Id").into(), components.join("/"));
        }
    }
    Ok(rels)
}
/// Colours and fonts of a PowerPoint theme. Scheme colours are reachable by their own names
/// and by the default colour map aliases (`tx1` = `dk1`, `bg1` = `lt1`, ...).
#[derive(Clone, Default)]
struct Look {
    name: String,
    colors: BTreeMap<String, [u8; 3]>,
    major: String,
    minor: String,
}
impl Look {
    fn read(theme: Option<&Node>) -> Self {
        let mut look = Self::default();
        let Some(theme) = theme else {
            return look;
        };
        look.name = theme.attr("name").to_string();
        if let Some(scheme) = theme.descend("clrScheme") {
            for entry in &scheme.children {
                let value = entry
                    .child("srgbClr")
                    .map(|n| n.attr("val"))
                    .or_else(|| entry.child("sysClr").map(|n| n.attr("lastClr")));
                if let Some(rgb) = value.and_then(parse_hex) {
                    look.colors.insert(entry.name.clone(), rgb);
                }
            }
        }
        for (alias, name) in [
            ("tx1", "dk1"),
            ("bg1", "lt1"),
            ("tx2", "dk2"),
            ("bg2", "lt2"),
        ] {
            if let Some(rgb) = look.colors.get(name).copied() {
                look.colors.insert(alias.into(), rgb);
            }
        }
        let face = |kind: &str| {
            theme
                .descend(kind)
                .and_then(|n| n.child("latin"))
                .map(|n| n.attr("typeface").trim())
                .filter(|face| !face.is_empty() && face.len() <= 64)
                .map(str::to_string)
                .unwrap_or_default()
        };
        look.major = face("majorFont");
        look.minor = face("minorFont");
        look
    }
    /// Mapping targets are theme slots, not aliases from a preceding map.
    fn mapped(&self, mapping: Option<&Node>) -> Self {
        let mut mapped = self.clone();
        if let Some(mapping) = mapping {
            for (alias, target) in &mapping.attrs {
                if let Some(rgb) = self.colors.get(target) {
                    mapped.colors.insert(alias.clone(), *rgb);
                }
            }
        }
        mapped
    }
    /// Resolves `+mj-lt` / `+mn-lt` theme font references.
    fn font(&self, face: &str) -> Option<String> {
        let face = face.trim();
        let resolved = match face {
            "+mj-lt" => self.major.as_str(),
            "+mn-lt" => self.minor.as_str(),
            _ => face,
        };
        (!resolved.is_empty() && !resolved.starts_with('+') && resolved.len() <= 64)
            .then(|| resolved.to_string())
    }
}
fn parse_hex(value: &str) -> Option<[u8; 3]> {
    if value.len() != 6 || !value.is_ascii() {
        return None;
    }
    let number = u32::from_str_radix(value, 16).ok()?;
    Some([(number >> 16) as u8, (number >> 8) as u8, number as u8])
}
fn color(node: Option<&Node>, look: &Look, fallback: [u8; 3]) -> [u8; 3] {
    let Some(node) = node else { return fallback; };
    let source = node.descend("srgbClr").or_else(|| node.descend("schemeClr"))
        .or_else(|| node.descend("sysClr"));
    let Some(source) = source else { return fallback; };
    let mut rgb = match source.name.as_str() {
        "srgbClr" => parse_hex(source.attr("val")),
        "schemeClr" => look.colors.get(source.attr("val")).copied(),
        "sysClr" => parse_hex(source.attr("lastClr")),
        _ => None,
    }.unwrap_or(fallback);
    for transform in &source.children {
        let Some(amount) = transform.attr("val").parse::<f64>().ok()
            .filter(|v| v.is_finite()) else { continue; };
        let amount = amount / 100_000.0;
        match transform.name.as_str() {
            "tint" => rgb = rgb.map(|c| (f64::from(c) * amount
                + 255.0 * (1.0 - amount)).round().clamp(0.0, 255.0) as u8),
            "shade" => rgb = rgb.map(|c| (f64::from(c) * amount)
                .round().clamp(0.0, 255.0) as u8),
            "lumMod" => rgb = drawing_luminance(rgb, amount, 0.0),
            "lumOff" => rgb = drawing_luminance(rgb, 1.0, amount),
            _ => {}
        }
    }
    rgb
}
fn drawing_luminance(rgb: [u8; 3], multiplier: f64, offset: f64) -> [u8; 3] {
    let [r, g, b] = rgb.map(|c| f64::from(c) / 255.0);
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let light = (high + low) / 2.0;
    let next = (light * multiplier + offset).clamp(0.0, 1.0);
    let delta = high - low;
    if delta <= f64::EPSILON {
        return [(next * 255.0).round() as u8; 3];
    }
    let saturation = delta / (1.0 - (2.0 * light - 1.0).abs());
    let chroma = (1.0 - (2.0 * next - 1.0).abs()) * saturation;
    let shift = next - chroma / 2.0;
    [r, g, b].map(|c| (((c - low) / delta * chroma + shift) * 255.0)
        .round().clamp(0.0, 255.0) as u8)
}
/// Placeholder type and index of a shape (`p:nvSpPr/p:nvPr/p:ph`).
fn placeholder(shape: &Node) -> Option<(&str, &str)> {
    let ph = shape
        .children
        .iter()
        .find(|n| n.name.starts_with("nv"))?
        .child("nvPr")?
        .child("ph")?;
    Some((ph.attr("type"), if ph.attr("idx").is_empty() { "0" } else { ph.attr("idx") }))
}
fn placeholder_class(kind: &str) -> &str {
    match kind {
        "title" | "ctrTitle" => "title",
        "dt" | "ftr" | "sldNum" | "hdr" | "pic" | "chart" | "tbl" => kind,
        _ => "body",
    }
}
/// Geometry a placeholder without its own `a:xfrm` inherits from the layout, then the master.
fn inherited_xfrm<'a>(shape: &Node, sources: &[&'a Node]) -> Option<&'a Node> {
    let (kind, idx) = placeholder(shape)?;
    let mut candidates = Vec::new();
    for (level, &source) in sources.iter().enumerate() {
        candidates.clear();
        source.all("sp", &mut candidates);
        let by_index = if level == 0 && !idx.is_empty() {
            candidates
                .iter()
                .find(|c| placeholder(c).is_some_and(|(_, i)| i == idx))
        } else {
            None
        };
        let found = by_index.or_else(|| {
            candidates.iter().find(|c| {
                placeholder(c).is_some_and(|(t, _)| placeholder_class(t) == placeholder_class(kind))
            })
        });
        let xfrm = found
            .copied()
            .and_then(|n| n.child("spPr"))
            .and_then(|n| n.child("xfrm"))
            .filter(|n| n.child("off").is_some() && n.child("ext").is_some());
        if xfrm.is_some() {
            return xfrm;
        }
    }
    None
}
/// Shapes of a shape tree with group transforms folded in; `[sx, sy, dx, dy]` maps child EMU
/// coordinates to slide EMU coordinates (`slide = s * child + d`).
fn flatten<'a>(
    tree: &'a Node,
    map: [f32; 4],
    depth: usize,
    out: &mut Vec<(&'a Node, [f32; 4])>,
    rotated_groups: &mut bool,
) {
    for shape in &tree.children {
        if shape.name == "AlternateContent" {
            if let Some(branch) = shape.child("Fallback").or_else(|| shape.child("Choice")) {
                flatten(branch, map, depth + 1, out, rotated_groups);
            }
            continue;
        }
        if shape.name != "grpSp" {
            out.push((shape, map));
            continue;
        }
        if depth >= 16 {
            continue;
        }
        let mut inner = map;
        if let Some(x) = shape.child("grpSpPr").and_then(|n| n.child("xfrm")) {
            let value = |child: &str, key: &str| x.child(child).map_or(0.0, |n| n.number(key, 0.0));
            let (cx, cy) = (value("ext", "cx"), value("ext", "cy"));
            let (chx, chy) = (value("chExt", "cx"), value("chExt", "cy"));
            let sx = if chx > 0.0 { cx / chx } else { 1.0 };
            let sy = if chy > 0.0 { cy / chy } else { 1.0 };
            let dx = value("off", "x") - value("chOff", "x") * sx;
            let dy = value("off", "y") - value("chOff", "y") * sy;
            inner = [
                map[0] * sx,
                map[1] * sy,
                map[0] * dx + map[2],
                map[1] * dy + map[3],
            ];
            *rotated_groups |= x.number("rot", 0.0) != 0.0;
        }
        flatten(shape, inner, depth + 1, out, rotated_groups);
    }
}
fn cached_points(node: &Node) -> Result<Vec<String>, String> {
    let cache = ["numCache", "strCache", "numLit", "strLit"]
        .iter()
        .find_map(|name| node.descend(name))
        .unwrap_or(node);
    let declared = cache
        .child("ptCount")
        .map(|n| n.attr("val").parse::<usize>())
        .transpose()
        .map_err(|_| "Некорректное число точек диаграммы")?
        .unwrap_or(0);
    if declared > 10000 {
        return Err("Слишком много точек диаграммы".into());
    }
    let mut points = vec![String::new(); declared];
    for point in cache.children.iter().filter(|n| n.name == "pt") {
        let index = point
            .attr("idx")
            .parse::<usize>()
            .map_err(|_| "Некорректный индекс точки диаграммы")?;
        if index >= 10000 {
            return Err("Слишком большой индекс точки диаграммы".into());
        }
        if points.len() <= index {
            points.resize_with(index + 1, String::new);
        }
        points[index] = point.child("v").map(|n| n.text.clone()).unwrap_or_default();
    }
    Ok(points)
}
fn first_text_paragraph(body: &Node) -> Option<&Node> {
    body.children.iter().filter(|node|node.name=="p")
        .find(|node|!node.text_content().is_empty())
        .or_else(||body.child("p"))
}
fn paragraph_level(paragraph: Option<&Node>) -> usize {
    paragraph.and_then(|p|p.child("pPr")).and_then(|p|p.attr("lvl").parse::<usize>().ok())
        .unwrap_or(0).min(8)
}
fn level_properties(style: &Node, level: usize) -> Option<&Node> {
    style.child(&format!("lvl{}pPr",level+1)).and_then(|p|p.child("defRPr"))
        .or_else(||style.child("defPPr").and_then(|p|p.child("defRPr")))
}
fn apply_run_properties(props: &Node, element: &mut Element, look: &Look) {
    if !props.attr("sz").is_empty() {
        element.font_size=(props.number("sz",element.font_size*100.0)/100.0).clamp(1.0,300.0);
    }
    if !props.attr("b").is_empty() { element.bold=matches!(props.attr("b"),"1"|"true"); }
    if !props.attr("i").is_empty() { element.italic=matches!(props.attr("i"),"1"|"true"); }
    element.color=color(props.child("solidFill"),look,element.color);
    if let Some(face)=props.child("latin").and_then(|n|look.font(n.attr("typeface"))) { element.font=face; }
}
fn selected_run_properties(body: &Node) -> (Vec<&Node>, bool) {
    let paragraph=first_text_paragraph(body);
    let mut layers=Vec::new();
    if let Some(props)=body.child("lstStyle").and_then(|style|level_properties(style,paragraph_level(paragraph))) {
        layers.push(props);
    }
    if let Some(props)=paragraph.and_then(|p|p.child("pPr")).and_then(|p|p.child("defRPr")) {layers.push(props);}
    let text_run=paragraph.and_then(|p|p.children.iter().find(|node|
        matches!(node.name.as_str(),"r"|"fld")&&!node.text_content().is_empty()));
    if let Some(props)=text_run.and_then(|run|run.child("rPr")) {layers.push(props);}
    let has_text=paragraph.is_some_and(|p|!p.text_content().is_empty());
    if !has_text && let Some(props)=paragraph.and_then(|p|p.child("endParaRPr")) {layers.push(props);}
    (layers,has_text)
}
fn apply_text_look(
    body:&Node,element:&mut Element,look:&Look,style:Option<&Node>,default_size:f32,title:bool,
) {
    let (layers,_)=selected_run_properties(body);
    element.font_size=default_size;
    let theme_face=if title {&look.major}else{&look.minor};
    if !theme_face.is_empty() {element.font.clone_from(theme_face);}
    element.color=color(style.and_then(|n|n.child("fontRef")),look,element.color);
    for props in layers {apply_run_properties(props,element,look);}
}
fn apply_inherited_text_look(
    shape:&Node,body:&Node,sources:&[&Node],element:&mut Element,look:&Look,
    style:Option<&Node>,default_size:f32,title:bool,
) {
    apply_text_look(body,element,look,style,default_size,title);
    let own_layers=selected_run_properties(body).0;
    let own_font=own_layers.iter().any(|props|props.child("latin").and_then(|n|look.font(n.attr("typeface"))).is_some());
    let own_color=own_layers.iter().any(|props|props.child("solidFill").is_some())
        ||style.and_then(|n|n.child("fontRef")).is_some();
    let level=paragraph_level(first_text_paragraph(body));
    let mut inherited=Element {font_size:default_size,color:element.color,font:element.font.clone(),..Element::default()};
    for (source_index,source) in sources.iter().enumerate().rev() {
        if let Some(styles)=source.child("txStyles").and_then(|styles|styles.child(if title {"titleStyle"}else if placeholder(shape).is_some() {"bodyStyle"}else{"otherStyle"}))
            && let Some(props)=level_properties(styles,level) {apply_run_properties(props,&mut inherited,look);}
        if let Some((kind,index))=placeholder(shape) {
            let mut candidates=Vec::new();source.all("sp",&mut candidates);
            let found=(source_index==0).then(||candidates.iter().find(|candidate|placeholder(candidate).is_some_and(|(_,i)|i==index))).flatten()
                .or_else(||candidates.iter().find(|candidate|placeholder(candidate).is_some_and(|(k,_)|placeholder_class(k)==placeholder_class(kind))));
            if let Some(inherited_body)=found.and_then(|node|node.child("txBody")) {
                if let Some(props)=inherited_body.child("lstStyle").and_then(|style|level_properties(style,level)) {
                    apply_run_properties(props,&mut inherited,look);
                }
                for props in selected_run_properties(inherited_body).0 {apply_run_properties(props,&mut inherited,look);}
            }
        }
    }
    let explicit=|name:&str|own_layers.iter().any(|props|!props.attr(name).is_empty());
    if !explicit("sz") {element.font_size=inherited.font_size;}
    if !explicit("b") {element.bold=inherited.bold;}
    if !explicit("i") {element.italic=inherited.italic;}
    if !own_font {element.font=inherited.font;}
    if !own_color {element.color=inherited.color;}
}

fn flipped_image(bytes:&[u8],flip:(bool,bool))->Result<Vec<u8>,String> {
    let mut image=crate::util::decode_image(bytes)?;
    if flip.0 {image=image.fliph();}
    if flip.1 {image=image.flipv();}
    let mut output=std::io::Cursor::new(Vec::new());
    image.write_to(&mut output,image::ImageFormat::Png).map_err(|error|error.to_string())?;
    Ok(output.into_inner())
}
fn scale_group_appearance(element:&mut Element,map:[f32;4]) {
    // A uniform group scale applies to line widths and text sizes too. Nonuniform text
    // deformation cannot be represented by the current single font-size field.
    if map[0]>0.0 && (map[0]-map[1]).abs()<=map[0].abs()*0.00001 {
        element.font_size=(element.font_size*map[0]).clamp(1.0,300.0);
        element.stroke_width=(element.stroke_width*map[0]).clamp(0.0,300.0);
    }
}
fn read_pptx(bytes: &[u8]) -> Result<Deck, String> {
    let parts = read_zip(bytes)?;
    if let Some(json) = parts
        .get("customXml/trinedocs-deck.json")
        .filter(|_| native_manifest_matches(&parts))
    {
        let deck: Deck = serde_json::from_slice(json)
            .map_err(|e| format!("Некорректная модель TrineDocs: {e}"))?;
        deck.validate()?;
        return Ok(deck);
    }
    let root_rels = relationships_kind(parts.get("_rels/.rels"), "", Some("officeDocument"))?;
    let presentation_path = root_rels.values().next().map(String::as_str)
        .unwrap_or("ppt/presentation.xml");
    let presentation = parse(parts.get(presentation_path).ok_or("Нет XML презентации")?)?;
    let rels_of_kind = |path: &str, kind: Option<&str>| {
        let (base, name) = path.rsplit_once('/').unwrap_or(("", path));
        let rel_path = if base.is_empty() { format!("_rels/{name}.rels") }
            else { format!("{base}/_rels/{name}.rels") };
        if kind.is_none() { relationships(parts.get(&rel_path), base) }
        else { relationships_kind(parts.get(&rel_path), base, kind) }
    };
    let rels = rels_of_kind(presentation_path, Some("slide"))?;
    let mut deck = Deck::empty();
    let mut parsed: BTreeMap<String, Node> = BTreeMap::new();
    let mut looks: BTreeMap<String, Look> = BTreeMap::new();
    let no_look = Look::default();
    let mut themed = false;
    let mut approximated = 0_usize;
    let rels_of = |path: &str| rels_of_kind(path, None);
    if let Some(size) = presentation.child("sldSz") {
        deck.width = size.number("cx", 12192000.0) / 12700.0;
        deck.height = size.number("cy", 6858000.0) / 12700.0;
    }
    let Some(slide_list) = presentation.child("sldIdLst") else {
        deck.validate()?;
        return Ok(deck);
    };
    let mut section_names=BTreeMap::new();
    if let Some(sections)=presentation.descend("sectionLst") {
        for section in sections.children.iter().filter(|node|node.name=="section") {
            if let Some(ids)=section.child("sldIdLst") {
                for id in &ids.children {section_names.insert(id.attr("id").to_string(),section.attr("name").to_string());}
            }
        }
    }
    for (index, slide_ref) in slide_list.children.iter().filter(|node|node.name=="sldId").enumerate() {
        let target = rels
            .get(slide_ref.attr("r:id"))
            .ok_or("Не найдена ссылка слайда")?;
        let xml = parse(parts.get(target).ok_or("Нет XML слайда")?)?;
        let id = deck.allocate_id();
        let mut slide = Slide {
            id,
            title: format!("Слайд {}", index + 1),
            hidden: matches!(xml.attr("show"), "0" | "false"),
            section:section_names.get(slide_ref.attr("id")).cloned().unwrap_or_default(),
            ..Slide::default()
        };
        let named_title = xml
            .child("cSld")
            .map(|n| n.attr("name"))
            .filter(|name| !name.is_empty());
        if let Some(title) = named_title {
            slide.title = title.into();
        }
        let slide_rels = rels_of(target.as_str())?;
        // The layout and its master supply placeholder geometry, the background and the theme.
        let mut chain = Vec::new();
        if let Some(layout) = rels_of_kind(target, Some("slideLayout"))?.values().next() {
            chain.push(layout.clone());
            if let Some(master) = rels_of_kind(layout, Some("slideMaster"))?.values().next()
            {
                chain.push(master.clone());
            }
        }
        for path in &chain {
            if !parsed.contains_key(path)
                && let Some(bytes) = parts.get(path)
            {
                parsed.insert(path.clone(), parse(bytes)?);
            }
        }
        let master = chain.get(1).cloned().unwrap_or_default();
        if !master.is_empty() && !looks.contains_key(&master) {
            let theme = rels_of_kind(&master, Some("theme"))?
                .values().next()
                .and_then(|p| parts.get(p))
                .map(|bytes| parse(bytes))
                .transpose()?;
            looks.insert(master.clone(), Look::read(theme.as_ref()));
        }
        let raw_look = looks.get(&master).unwrap_or(&no_look);
        let sources: Vec<&Node> = chain.iter().filter_map(|p| parsed.get(p)).collect();
        if let Some(layout) = sources.first() {
            slide.layout = match layout.attr("type") {
                "title" | "ctrTx" => "title",
                "obj" | "tx" | "titleOnly" => "content",
                "twoObj" | "twoTxTwoObj" => "two_content",
                _ => "blank",
            }.into();
        }
        let master_look = raw_look.mapped(sources.get(1).and_then(|n| n.child("clrMap")));
        let mut resolved_look = master_look.clone();
        for node in sources.iter().take(1).copied().chain(std::iter::once(&xml)) {
            if let Some(override_map) = node.child("clrMapOvr") {
                if override_map.child("masterClrMapping").is_some() {
                    resolved_look = master_look.clone();
                } else if let Some(mapping) = override_map.child("overrideClrMapping") {
                    resolved_look = raw_look.mapped(Some(mapping));
                }
            }
        }
        let look = &resolved_look;
        if !themed && !look.colors.is_empty() {
            themed = true;
            if !look.name.is_empty() { deck.theme.name.clone_from(&look.name); }
            for (name, slot) in [
                ("tx1", &mut deck.theme.foreground),
                ("bg1", &mut deck.theme.background),
                ("accent1", &mut deck.theme.accent),
            ] {
                if let Some(rgb) = look.colors.get(name) {
                    *slot = *rgb;
                }
            }
            if !look.minor.is_empty() {
                deck.theme.font.clone_from(&look.minor);
            }
        }
        let background = std::iter::once(&xml)
            .chain(sources.iter().copied())
            .find_map(|n| n.child("cSld").and_then(|c| c.child("bg")));
        slide.background = color(background, look, [255, 255, 255]);
        let text_color = look.colors.get("tx1").copied().unwrap_or([31, 41, 55]);
        let mut titled = false;
        // PowerPoint shape ID (cNvPr id) → element index, for p:timing targets.
        let mut shape_ids: BTreeMap<String, usize> = BTreeMap::new();
        if let Some(tree) = xml.descend("spTree") {
            let mut shapes = Vec::new();
            let mut rotated_groups = false;
            flatten(
                tree,
                [1.0, 1.0, 0.0, 0.0],
                0,
                &mut shapes,
                &mut rotated_groups,
            );
            if rotated_groups {
                deck.warnings.push(
                    "Поворот групп PowerPoint не импортируется; объекты групп размещены без поворота"
                        .into(),
                );
            }
            for (shape, map) in shapes {
                if !matches!(shape.name.as_str(), "sp" | "pic" | "cxnSp" | "graphicFrame") {
                    continue;
                }
                let mut element = Element {
                    id: deck.allocate_id(),
                    color: text_color,
                    ..Element::default()
                };
                let own = shape
                    .descend("xfrm")
                    .filter(|n| n.child("off").is_some() || n.child("ext").is_some());
                let mut flip = (false, false);
                let inherited = inherited_xfrm(shape, &sources);
                if let Some(transform) = own.or(inherited) {
                    if let Some(offset) = own.and_then(|n|n.child("off")).or_else(||inherited.and_then(|n|n.child("off"))) {
                        element.x = (map[0] * offset.number("x", 0.0) + map[2]) / 12700.0;
                        element.y = (map[1] * offset.number("y", 0.0) + map[3]) / 12700.0;
                    }
                    if let Some(size) = own.and_then(|n|n.child("ext")).or_else(||inherited.and_then(|n|n.child("ext"))) {
                        element.w = (map[0] * size.number("cx", 1270000.0) / 12700.0).max(0.001);
                        element.h = (map[1] * size.number("cy", 1270000.0) / 12700.0).max(0.001);
                    }
                    element.rotation = transform.number("rot", 0.0) / 60000.0;
                    flip = (
                        matches!(transform.attr("flipH"), "1" | "true"),
                        matches!(transform.attr("flipV"), "1" | "true"),
                    );
                }
                let props = shape.child("spPr");
                let style = shape.child("style");
                if matches!(shape.name.as_str(), "sp" | "cxnSp") {
                    // Explicit shape properties win over the theme style references (p:style).
                    let explicit = props.and_then(|n| {
                        n.children.iter().find(|c| {
                            matches!(
                                c.name.as_str(),
                                "solidFill" | "gradFill" | "pattFill" | "noFill"
                            )
                        })
                    });
                    let fill = match explicit {
                        Some(n) if n.name == "noFill" => None,
                        Some(n) => Some(n),
                        None => style.and_then(|n| n.child("fillRef")),
                    };
                    element.fill = color(fill, look, element.fill);
                    let line = props.and_then(|n| n.child("ln"));
                    let line_ref = style
                        .and_then(|n| n.child("lnRef"))
                        .filter(|n| n.attr("idx") != "0");
                    element.stroke = color(line.or(line_ref), look, element.stroke);
                    element.stroke_width = match line {
                        Some(line) if line.child("noFill").is_some() => 0.0,
                        Some(line)
                            if line.child("solidFill").is_none()
                                && line.child("gradFill").is_none()
                                && line_ref.is_none() =>
                        {
                            0.0
                        }
                        Some(line) => line.number("w", 12700.0) / 12700.0,
                        None if line_ref.is_some() => 1.0,
                        None => 0.0,
                    }
                    .clamp(0.0, 300.0);
                } else {
                    element.fill =
                        color(props.and_then(|n| n.child("solidFill")), look, element.fill);
                    element.stroke = color(props.and_then(|n| n.child("ln")), look, element.stroke);
                    if let Some(line) = props.and_then(|n| n.child("ln")) {
                        element.stroke_width =
                            (line.number("w", 12700.0) / 12700.0).clamp(0.0, 300.0);
                    }
                }
                if shape.name == "pic" {
                    let target = shape
                        .descend("blip")
                        .and_then(|n| slide_rels.get(n.attr("r:embed")));
                    let Some(bytes) = target.and_then(|p| parts.get(p)) else {
                        deck.warnings
                            .push("Пропущено отсутствующее или внешнее изображение".into());
                        continue;
                    };
                    if !matches!(
                        image::guess_format(bytes),
                        Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg)
                    ) {
                        deck.warnings.push(
                            "Пропущено изображение неподдерживаемого формата; доступны PNG/JPEG"
                                .into(),
                        );
                        continue;
                    }
                    super::model::image_dimensions(bytes)?;
                    let image_bytes=if flip.0||flip.1 {flipped_image(bytes,flip)?}else{bytes.clone()};
                    element.kind = ElementKind::Image { bytes:image_bytes };
                } else if let Some(table) = shape.descend("tbl") {
                    if let Some(cell) = table.child("tr").and_then(|row| row.child("tc")) {
                        if let Some(body) = cell.child("txBody") {
                            apply_text_look(body, &mut element, look, None, 18.0, false);
                        }
                        if let Some(props) = cell.child("tcPr") {
                            element.fill = color(props.child("solidFill"), look, element.fill);
                            if let Some(line) = props.child("lnL") {
                                element.stroke_width = if line.child("noFill").is_some() {
                                    0.0
                                } else {
                                    (line.number("w", 12700.0) / 12700.0).clamp(0.0, 300.0)
                                };
                                element.stroke =
                                    color(line.child("solidFill"), look, element.stroke);
                            }
                        }
                    }
                    element.kind = ElementKind::Table {
                        cells: table
                            .children
                            .iter()
                            .filter(|n| n.name == "tr")
                            .map(|row| {
                                row.children
                                    .iter()
                                    .filter(|n| n.name == "tc")
                                    .map(Node::paragraph_text)
                                    .collect()
                            })
                            .collect(),
                    };
                } else if let Some(chart_ref) = shape.descend("chart") {
                    if let Some(chart_xml) = slide_rels
                        .get(chart_ref.attr("r:id"))
                        .and_then(|p| parts.get(p))
                    {
                        let chart = parse(chart_xml)?;
                        let values = chart
                            .descend("val")
                            .map(cached_points)
                            .transpose()?
                            .unwrap_or_default()
                            .iter()
                            .map(|value| {
                                if value.is_empty() {
                                    Ok(0.0)
                                } else {
                                    value
                                        .parse::<f32>()
                                        .map_err(|_| "Некорректное значение диаграммы".to_string())
                                }
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        let labels = chart
                            .descend("cat")
                            .map(cached_points)
                            .transpose()?
                            .unwrap_or_default();
                        element.text = chart.descend("title").map(Node::text_content).unwrap_or_default();
                        element.fill = color(chart.descend("ser").and_then(|series|series.child("spPr")), look, element.fill);
                        element.kind = ElementKind::Chart { values, labels };
                    } else {
                        deck.warnings.push("Пропущена диаграмма: отсутствует внутренняя часть PPTX".into());
                        continue;
                    }
                } else {
                    let geometry = shape.descend("prstGeom").map(|n| n.attr("prst"));
                    element.kind = match geometry {
                        _ if shape.name == "cxnSp" => ElementKind::Line,
                        Some("ellipse" | "donut" | "flowChartConnector") => ElementKind::Ellipse,
                        Some("line" | "straightConnector1") => ElementKind::Line,
                        Some("rightArrow" | "leftArrow" | "upArrow" | "downArrow") => {
                            ElementKind::Arrow
                        }
                        Some(
                            "rect" | "roundRect" | "flowChartProcess" | "flowChartAlternateProcess",
                        ) => ElementKind::Rect,
                        Some(other) if !other.is_empty() => {
                            approximated += 1;
                            ElementKind::Rect
                        }
                        None if shape.descend("custGeom").is_some() => {
                            approximated += 1;
                            ElementKind::Rect
                        }
                        _ => ElementKind::Text,
                    };
                    // Block arrows become a line arrow through the middle of their frame.
                    match geometry {
                        Some("rightArrow" | "leftArrow") => {
                            element.y += element.h / 2.0;
                            element.h = 1.0;
                            if geometry == Some("leftArrow") {
                                element.rotation += 180.0;
                            }
                        }
                        Some("upArrow" | "downArrow") => {
                            element.x += element.w / 2.0;
                            element.w = 1.0;
                            if geometry == Some("upArrow") {
                                element.rotation += 180.0;
                            }
                        }
                        _ => {}
                    }
                    let arrow_end = |end: &str| {
                        props
                            .and_then(|n| n.child("ln"))
                            .and_then(|n| n.child(end))
                            .is_some_and(|n| n.attr("type") != "none" && !n.attr("type").is_empty())
                    };
                    if matches!(element.kind, ElementKind::Line)
                        && (arrow_end("tailEnd") || arrow_end("headEnd"))
                    {
                        element.kind = ElementKind::Arrow;
                        if !arrow_end("tailEnd") {
                            // Only the start has an arrowhead: the arrow points backwards.
                            flip = (!flip.0, !flip.1);
                        }
                    }
                    // A flipped line runs along the other diagonal; the model expresses that as a
                    // rotation of the top-left → bottom-right diagonal around the frame centre.
                    if matches!(element.kind, ElementKind::Line | ElementKind::Arrow)
                        && (flip.0 || flip.1)
                    {
                        let sx = if flip.0 { -1.0 } else { 1.0 };
                        let sy = if flip.1 { -1.0 } else { 1.0 };
                        let turn =
                            (element.h * sy).atan2(element.w * sx) - element.h.atan2(element.w);
                        element.rotation += turn.to_degrees();
                    }
                    element.rotation = element.rotation.rem_euclid(360.0);
                    if shape.descend("cNvSpPr").is_some_and(|node|matches!(node.attr("txBox"),"1"|"true")) {
                        element.kind=ElementKind::Text;
                    }
                    if matches!(element.kind,ElementKind::Rect|ElementKind::Ellipse) {
                        element.fill_visible=props.and_then(|node|node.child("noFill")).is_none();
                    }
                }
                if let Some(body) = shape.child("txBody") {
                    element.text = body
                        .children
                        .iter()
                        .filter(|n| n.name == "p")
                        .map(Node::text_content)
                        .collect::<Vec<_>>()
                        .join("\n");
                    let class = placeholder(shape).map(|(t, _)| placeholder_class(t));
                    let is_title = class == Some("title");
                    // PowerPoint defaults: title placeholder 44 pt, other placeholders 28 pt,
                    // ordinary text boxes 18 pt.
                    let default_size = match class {
                        Some("title") => 44.0,
                        Some(_) => 28.0,
                        None => 18.0,
                    };
                    apply_inherited_text_look(shape,body,&sources,&mut element,look,style,default_size,is_title);
                    if named_title.is_none()
                        && !element.text.is_empty()
                        && (is_title || (!titled && slide.elements.is_empty()))
                    {
                        slide.title = element.text.lines().next().unwrap_or("").into();
                        titled |= is_title;
                    }
                }
                scale_group_appearance(&mut element,map);
                if let Some(spid) = shape
                    .descend("cNvPr")
                    .map(|n| n.attr("id"))
                    .filter(|id| !id.is_empty())
                {
                    shape_ids.insert(spid.to_string(), slide.elements.len());
                }
                slide.elements.push(element);
            }
        }
        // PowerPoint 2010+ wraps transitions in mc:AlternateContent; the fallback holds the
        // classic effect, the p14 choice the exact duration.
        let alternate = xml
            .children
            .iter()
            .find(|n| n.name == "AlternateContent" && n.descend("transition").is_some());
        let choice = alternate
            .and_then(|n| n.child("Choice"))
            .and_then(|n| n.child("transition"));
        let fallback = alternate
            .and_then(|n| n.child("Fallback"))
            .and_then(|n| n.child("transition"));
        if let Some(transition) = xml.child("transition").or(fallback).or(choice) {
            slide.transition.effect = transition
                .children
                .iter()
                .map(|n| n.name.as_str())
                .find(|name| !matches!(*name, "sndAc" | "extLst"))
                .map(|name| if name == "cut" { "none" } else { name })
                .unwrap_or("none")
                .to_string();
            if !matches!(
                slide.transition.effect.as_str(),
                "none" | "fade" | "push" | "wipe"
            ) {
                deck.warnings.push(format!(
                    "Переход {} не поддерживается",
                    slide.transition.effect
                ));
                slide.transition.effect = "none".into();
            }
            let exact = [Some(transition), choice]
                .into_iter()
                .flatten()
                .find_map(|n| n.attr("dur").parse::<f32>().ok())
                .filter(|ms| ms.is_finite() && *ms > 0.0);
            slide.transition.duration = match (exact, transition.attr("spd")) {
                (Some(ms), _) => (ms / 1000.0).clamp(0.001, 3600.0),
                (None, "slow") => 1.0,
                (None, "fast") => 0.25,
                _ => 0.5,
            };
            slide.transition.advance_after = transition
                .attr("advTm")
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite() && *v > 0.0)
                .map(|v| (v / 1000.0).clamp(0.001, 86400.0));
        }
        for path in rels_of_kind(target, Some("notesSlide"))?.values() {
            if let Some(bytes) = parts.get(path) {
                // Only the notes body: the slide image and the slide number placeholders are
                // not part of the speaker notes.
                let notes = parse(bytes)?;
                let mut shapes = Vec::new();
                notes.all("sp", &mut shapes);
                slide.notes = shapes
                    .iter()
                    .filter(|s| placeholder(s).is_some_and(|(t, _)| t == "body"))
                    .filter_map(|s| s.child("txBody"))
                    .map(|body| body.paragraph_text())
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        if let Some(timing) = xml.child("timing") {
            let skipped = read_timing(timing, &shape_ids, &mut slide, (deck.width, deck.height));
            if skipped > 0 {
                deck.warnings.push(format!(
                    "Слайд {}: анимации PowerPoint без точного аналога (прочие эффекты, кривые пути, триггеры объектов) заменены ближайшими или пропущены: {skipped}",
                    index + 1
                ));
            }
        }
        deck.slides.push(slide);
    }
    if approximated > 0 {
        deck.warnings.push(format!(
            "Фигуры PowerPoint без аналога заменены прямоугольниками: {approximated}"
        ));
    }
    deck.warnings.push("Импорт внешнего PPTX: часть стилей, мастеров, групп и эффектов может быть потеряна. Сохраняйте копию.".into());
    deck.validate()?;
    Ok(deck)
}
/// Start delay of a time node in ms (`indefinite` and missing = 0).
fn node_delay(node: &Node) -> f32 {
    node.child("stCondLst")
        .and_then(|n| n.child("cond"))
        .map_or(0.0, |n| n.number("delay", 0.0).max(0.0))
}
/// Effect nodes under a time node with their start in ms from the start of the click group.
fn timed_effects<'a>(node: &'a Node, offset: f32, out: &mut Vec<(f32, &'a Node)>) {
    let Some(children) = node.child("childTnLst") else {
        return;
    };
    for child in &children.children {
        let Some(ctn) = child.child("cTn") else {
            continue;
        };
        let start = offset + node_delay(ctn);
        if ctn.attr("presetClass").is_empty() {
            timed_effects(ctn, start, out);
        } else {
            out.push((start, ctn));
        }
    }
}
/// TrineDocs effect for a PowerPoint preset; `None` = no equivalent. The flag marks presets
/// replaced by the closest TrineDocs effect.
fn preset_effect(effect: &Node, size: (f32, f32)) -> Option<(&'static str, f32, f32, bool)> {
    let id = effect.attr("presetID");
    Some(match effect.attr("presetClass") {
        "entr" => match id {
            "10" => ("fade", 0.0, 0.0, false),
            "2" => ("fly_in", 0.0, 0.0, false),
            "23" | "53" => ("zoom", 0.0, 0.0, false),
            _ => ("fade", 0.0, 0.0, true),
        },
        "exit" => match id {
            "10" => ("fade_out", 0.0, 0.0, false),
            "2" => ("fly_out", 0.0, 0.0, false),
            "23" | "53" => ("zoom_out", 0.0, 0.0, false),
            _ => ("fade_out", 0.0, 0.0, true),
        },
        "emph" => match id {
            "26" => ("pulse", 0.0, 0.0, false),
            "8" => ("spin", 0.0, 0.0, false),
            _ => ("pulse", 0.0, 0.0, true),
        },
        "path" => {
            let path = effect.descend("animMotion")?.attr("path");
            let numbers: Vec<f32> = path
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter_map(|t| t.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .collect();
            let [.., x, y] = numbers.as_slice() else {
                return None;
            };
            // Only straight paths are exact; curves keep their end point.
            let straight = !path.contains('C') && numbers.len() <= 4;
            (
                "move",
                (x * size.0).clamp(-100000.0, 100000.0),
                (y * size.1).clamp(-100000.0, 100000.0),
                !straight,
            )
        }
        _ => return None,
    })
}
/// Object animations of a slide's `p:timing` main sequence mapped onto its elements. Start
/// times are rebuilt from the group/sub-group/effect delays, so with/after-previous chains keep
/// their timing. Returns the number of effects that were approximated or skipped.
fn read_timing(
    timing: &Node,
    shape_ids: &BTreeMap<String, usize>,
    slide: &mut Slide,
    size: (f32, f32),
) -> usize {
    let mut all = Vec::new();
    timing.all("cTn", &mut all);
    let total = all
        .iter()
        .filter(|n| !n.attr("presetClass").is_empty())
        .count();
    let Some(main) = all.into_iter().find(|n| n.attr("nodeType") == "mainSeq") else {
        return total;
    };
    let mut inexact = 0_usize;
    let mut used = 0_usize;
    let mut order = 0_u32;
    let (mut previous_start, mut previous_end) = (0.0_f32, 0.0_f32);
    let groups = main
        .child("childTnLst")
        .map_or(&[][..], |n| &n.children[..]);
    for (group_index, group) in groups.iter().enumerate() {
        let Some(group) = group.child("cTn") else {
            continue;
        };
        let auto = group_index == 0
            && group
                .child("stCondLst")
                .is_some_and(|n| n.children.iter().any(|c| c.attr("evt") == "onBegin"));
        let mut effects = Vec::new();
        timed_effects(group, 0.0, &mut effects);
        // Times are in seconds from the click (or slide start) that began this group.
        for (position, (start_ms, effect)) in effects.into_iter().enumerate() {
            used += 1;
            let start = start_ms / 1000.0;
            let index = effect
                .descend("spTgt")
                .and_then(|n| shape_ids.get(n.attr("spid")))
                .copied();
            let Some(element) = index.and_then(|i| slide.elements.get_mut(i)) else {
                inexact += 1;
                continue;
            };
            let Some((kind, dx, dy, approximate)) = preset_effect(effect, size) else {
                inexact += 1;
                continue;
            };
            if element.more_animations.len() >= 32 {
                inexact += 1;
                continue;
            }
            inexact += usize::from(approximate);
            let mut behaviours = Vec::new();
            if let Some(children) = effect.child("childTnLst") {
                children.all("cTn", &mut behaviours);
            }
            let longest = behaviours
                .iter()
                .map(|n| {
                    let duration = n.number("dur", 0.0);
                    // Auto-reversed behaviours (pulse) run twice.
                    if n.attr("autoRev") == "1" {
                        duration * 2.0
                    } else {
                        duration
                    }
                })
                .filter(|ms| *ms > 1.0)
                .fold(0.0_f32, f32::max);
            // "Appear" and other instant effects.
            let duration = if longest > 0.0 {
                (longest / 1000.0).clamp(0.01, 3600.0)
            } else {
                0.01
            };
            let (trigger, delay) = if position == 0 && !auto {
                ("on_click", start)
            } else if position == 0 {
                ("with_previous", start)
            } else if effect.attr("nodeType") == "afterEffect" && start + 0.0005 >= previous_end {
                ("after_previous", start - previous_end)
            } else {
                if start + 0.0005 < previous_start {
                    // Starts before the previous step: TrineDocs steps cannot run backwards.
                    inexact += 1;
                }
                ("with_previous", start - previous_start)
            };
            let delay = delay.clamp(0.0, 86400.0);
            let start = match trigger {
                "after_previous" => previous_end + delay,
                _ if position == 0 => delay,
                _ => previous_start + delay,
            };
            previous_start = start;
            previous_end = start + duration;
            order += 1;
            let animation = super::model::Animation {
                effect: kind.into(),
                trigger: trigger.into(),
                delay,
                duration,
                order,
                dx,
                dy,
            };
            if element.is_animated() {
                element.more_animations.push(animation);
            } else {
                element.animation = animation;
            }
        }
    }
    // Interactive sequences (object triggers) and other effects outside the main sequence.
    inexact + total.saturating_sub(used)
}
fn part_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, b| {
        (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn native_manifest_matches(parts: &BTreeMap<String, Vec<u8>>) -> bool {
    let Some(bytes) = parts.get("customXml/trinedocs-manifest.json") else {
        return false;
    };
    let Ok(expected) = serde_json::from_slice::<BTreeMap<String, u64>>(bytes) else {
        return false;
    };
    let actual: BTreeMap<String, u64> = parts
        .iter()
        .filter(|(path, _)| !path.starts_with("customXml/"))
        .map(|(path, bytes)| (path.clone(), part_hash(bytes)))
        .collect();
    expected == actual
}
fn hex(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}
fn emu(value: f32) -> i64 {
    (value * 12700.0).round() as i64
}
fn rel(id: &str, kind: &str, target: &str) -> String {
    format!(
        "<Relationship Id=\"{id}\" Type=\"{R}/{kind}\" Target=\"{}\"/>",
        xml_escape(target)
    )
}
fn rels(body: &str) -> String {
    format!(
        "{XML}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>"
    )
}
fn wrap(root: &str, attributes: &str, body: &str) -> String {
    format!(
        "{XML}<p:{root} xmlns:a=\"{A}\" xmlns:r=\"{R}\" xmlns:p=\"{P}\" {attributes}>{body}</p:{root}>"
    )
}
fn xfrm(e: &Element, prefix: &str) -> String {
    let rotation = format!(" rot=\"{}\"", (e.rotation * 60000.0) as i64);
    format!(
        "<{prefix}:xfrm{rotation}><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></{prefix}:xfrm>",
        emu(e.x),
        emu(e.y),
        emu(e.w),
        emu(e.h)
    )
}
fn text_body(e: &Element) -> String {
    let normalized = e.text.replace("\r\n", "\n").replace('\r', "\n");
    let paragraphs = normalized.split('\n').map(|line| format!("<a:p><a:r><a:rPr lang=\"ru-RU\" sz=\"{}\" b=\"{}\" i=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t xml:space=\"preserve\">{}</a:t></a:r><a:endParaRPr lang=\"ru-RU\"/></a:p>", (e.font_size * 100.0) as u32, u8::from(e.bold), u8::from(e.italic), hex(e.color), xml_escape(&e.font), xml_escape(line))).collect::<String>();
    // Only text boxes grow with their text; shapes keep the frame the user drew.
    let fit = if matches!(e.kind, ElementKind::Text) {
        "<a:spAutoFit/>"
    } else {
        ""
    };
    format!(
        "<p:txBody><a:bodyPr wrap=\"square\">{fit}</a:bodyPr><a:lstStyle/>{paragraphs}</p:txBody>"
    )
}
fn shape_xml(e: &Element, id: usize) -> String {
    let geometry = match e.kind {
        ElementKind::Ellipse => "ellipse",
        ElementKind::Line => "line",
        ElementKind::Arrow => "line",
        _ => "rect",
    };
    let fill = if matches!(
        e.kind,
        ElementKind::Text | ElementKind::Line | ElementKind::Arrow
    ) || !e.fill_visible {
        "<a:noFill/>".into()
    } else {
        format!(
            "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            hex(e.fill)
        )
    };
    // A zero width outline is "no line": PowerPoint draws w="0" as a hairline.
    let line = if matches!(e.kind, ElementKind::Text) || e.stroke_width <= 0.0 {
        "<a:ln><a:noFill/></a:ln>".into()
    } else {
        format!(
            "<a:ln w=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></a:ln>",
            emu(e.stroke_width),
            hex(e.stroke)
        )
    };
    let line = if matches!(e.kind, ElementKind::Arrow) {
        line.replace(
            "</a:ln>",
            "<a:tailEnd type=\"arrow\" w=\"lg\" len=\"lg\"/></a:ln>",
        )
    } else {
        line
    };
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Object {id}\"/><p:cNvSpPr txBox=\"{}\"/><p:nvPr/></p:nvSpPr><p:spPr>{}<a:prstGeom prst=\"{geometry}\"><a:avLst/></a:prstGeom>{fill}{line}</p:spPr>{}</p:sp>",
        u8::from(matches!(e.kind, ElementKind::Text)),
        xfrm(e, "a"),
        text_body(e)
    )
}
fn table_xml(e: &Element, id: usize, cells: &[Vec<String>]) -> Result<String,String> {
    let cols = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
    if cells.len().max(1).saturating_mul(cols)>100000 {
        return Err("Плотная таблица PPTX превышает безопасный предел ячеек".into());
    }
    let grid = (0..cols)
        .map(|_| format!("<a:gridCol w=\"{}\"/>", emu(e.w / cols as f32)))
        .collect::<String>();
    let paragraphs = |text: &str| {
        text.replace("\r\n", "\n").replace('\r', "\n").split('\n')
            .map(|line| format!(
                "<a:p><a:r><a:rPr lang=\"ru-RU\" sz=\"{}\" b=\"{}\" i=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t xml:space=\"preserve\">{}</a:t></a:r></a:p>",
                (e.font_size * 100.0) as u32, u8::from(e.bold), u8::from(e.italic), hex(e.color), xml_escape(&e.font), xml_escape(line)
            )).collect::<String>()
    };
    let line = if e.stroke_width <= 0.0 {
        "<a:noFill/>".to_string()
    } else {
        format!(
            "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            hex(e.stroke)
        )
    };
    let borders = ["lnL", "lnR", "lnT", "lnB"]
        .iter()
        .map(|edge| format!("<a:{edge} w=\"{}\">{line}</a:{edge}>", emu(e.stroke_width)))
        .collect::<String>();
    let rows = (0..cells.len().max(1))
        .map(|row_index| {
            let row = cells.get(row_index).map(Vec::as_slice).unwrap_or(&[]);
            let fill = hex(if row_index == 0 { e.fill } else { [245, 247, 250] });
            let values = (0..cols)
                .map(|col| {
                    format!(
                        "<a:tc><a:txBody><a:bodyPr/><a:lstStyle/>{}</a:txBody><a:tcPr>{borders}<a:solidFill><a:srgbClr val=\"{fill}\"/></a:solidFill></a:tcPr></a:tc>",
                        paragraphs(row.get(col).map(String::as_str).unwrap_or(""))
                    )
                })
                .collect::<String>();
            format!(
                "<a:tr h=\"{}\">{values}</a:tr>",
                emu(e.h / cells.len().max(1) as f32)
            )
        })
        .collect::<String>();
    Ok(format!(
        "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Table {id}\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr firstRow=\"1\" bandRow=\"1\"/><a:tblGrid>{grid}</a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>",
        xfrm(e, "p")
    ))
}
fn chart_xml(values: &[f32], labels: &[String], fill: [u8; 3],title:&str) -> String {
    let cats = values
        .iter()
        .enumerate()
        .map(|(i, _)| {
            format!(
                "<c:pt idx=\"{i}\"><c:v>{}</c:v></c:pt>",
                xml_escape(
                    &labels
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| (i + 1).to_string())
                )
            )
        })
        .collect::<String>();
    let vals = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("<c:pt idx=\"{i}\"><c:v>{v}</c:v></c:pt>"))
        .collect::<String>();
    let title=if title.is_empty() {String::new()}else {
        format!("<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val=\"0\"/></c:title>",xml_escape(title))
    };
    format!(
        "{XML}<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"{A}\"><c:chart>{title}<c:plotArea><c:layout/><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:spPr><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></c:spPr><c:cat><c:strLit><c:ptCount val=\"{}\"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val=\"{}\"/>{vals}</c:numLit></c:val></c:ser><c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"b\"/><c:crossAx val=\"2\"/><c:crosses val=\"autoZero\"/><c:auto val=\"1\"/><c:lblAlgn val=\"ctr\"/><c:lblOffset val=\"100\"/></c:catAx><c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"l\"/><c:numFmt formatCode=\"General\" sourceLinked=\"1\"/><c:crossAx val=\"1\"/><c:crosses val=\"autoZero\"/><c:crossBetween val=\"between\"/></c:valAx></c:plotArea><c:plotVisOnly val=\"1\"/></c:chart></c:chartSpace>",
        hex(fill),
        values.len(),
        values.len()
    )
}
fn theme_xml(deck: &Deck) -> String {
    let colors = [
        ("dk1", deck.theme.foreground),
        ("lt1", deck.theme.background),
        ("dk2", [31, 41, 55]),
        ("lt2", [242, 244, 247]),
        ("accent1", deck.theme.accent),
        ("accent2", [65, 125, 180]),
        ("accent3", [90, 160, 95]),
        ("accent4", [165, 95, 190]),
        ("accent5", [230, 180, 60]),
        ("accent6", [45, 180, 180]),
        ("hlink", [0, 90, 180]),
        ("folHlink", [100, 55, 150]),
    ]
    .iter()
    .map(|(n, c)| format!("<a:{n}><a:srgbClr val=\"{}\"/></a:{n}>", hex(*c)))
    .collect::<String>();
    let font = xml_escape(&deck.theme.font);
    let fonts = format!(
        "<a:fontScheme name=\"Trine\"><a:majorFont><a:latin typeface=\"{font}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"{font}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme>"
    );
    let fills = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>".repeat(3);
    let lines = "<a:ln w=\"12700\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/><a:miter lim=\"800000\"/></a:ln>".repeat(3);
    format!(
        "{XML}<a:theme xmlns:a=\"{A}\" name=\"{}\"><a:themeElements><a:clrScheme name=\"Trine\">{colors}</a:clrScheme>{fonts}<a:fmtScheme name=\"Trine\"><a:fillStyleLst>{fills}</a:fillStyleLst><a:lnStyleLst>{lines}</a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst>{fills}</a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>",
        xml_escape(&deck.theme.name)
    )
}
fn milliseconds(seconds: f32) -> u64 {
    (f64::from(seconds.max(0.0)) * 1000.0).round() as u64
}
/// PowerPoint effect for one animation step of an object (shape ID `spid`); `size` is the
/// slide size in points for motion paths.
fn effect_xml(
    next: &mut u32,
    a: &Animation,
    spid: usize,
    node_type: &str,
    delay: u64,
    size: (f32, f32),
) -> String {
    let mut id = || {
        *next += 1;
        *next
    };
    let outer = id();
    let duration = milliseconds(a.duration).max(1);
    let target = format!("<p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>");
    let visibility = |id: u32, at: u64, value: &str| {
        format!(
            "<p:set><p:cBhvr><p:cTn id=\"{id}\" dur=\"1\" fill=\"hold\"><p:stCondLst><p:cond delay=\"{at}\"/></p:stCondLst></p:cTn>{target}<p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val=\"{value}\"/></p:to></p:set>"
        )
    };
    let fade = |id: u32, direction: &str| {
        format!(
            "<p:animEffect transition=\"{direction}\" filter=\"fade\"><p:cBhvr><p:cTn id=\"{id}\" dur=\"{duration}\"/>{target}</p:cBhvr></p:animEffect>"
        )
    };
    let anim = |id: u32, attribute: &str, from: &str, to: &str| {
        format!(
            "<p:anim calcmode=\"lin\" valueType=\"num\"><p:cBhvr additive=\"base\"><p:cTn id=\"{id}\" dur=\"{duration}\" fill=\"hold\"/>{target}<p:attrNameLst><p:attrName>{attribute}</p:attrName></p:attrNameLst></p:cBhvr><p:tavLst><p:tav tm=\"0\"><p:val><p:strVal val=\"{from}\"/></p:val></p:tav><p:tav tm=\"100000\"><p:val><p:strVal val=\"{to}\"/></p:val></p:tav></p:tavLst></p:anim>"
        )
    };
    let hide_at = duration.saturating_sub(1);
    let (class, preset, subtype, behaviours) = match a.effect.as_str() {
        "fly_in" => (
            "entr",
            2,
            2,
            visibility(id(), 0, "visible")
                + &anim(id(), "ppt_x", "1+#ppt_w/2", "#ppt_x")
                + &anim(id(), "ppt_y", "#ppt_y", "#ppt_y"),
        ),
        "zoom" => (
            "entr",
            53,
            16,
            visibility(id(), 0, "visible")
                + &anim(id(), "ppt_w", "0", "#ppt_w")
                + &anim(id(), "ppt_h", "0", "#ppt_h")
                + &fade(id(), "in"),
        ),
        "fade_out" => (
            "exit",
            10,
            0,
            fade(id(), "out") + &visibility(id(), hide_at, "hidden"),
        ),
        "fly_out" => (
            "exit",
            2,
            2,
            anim(id(), "ppt_x", "#ppt_x", "1+#ppt_w/2")
                + &anim(id(), "ppt_y", "#ppt_y", "#ppt_y")
                + &visibility(id(), hide_at, "hidden"),
        ),
        "zoom_out" => (
            "exit",
            53,
            16,
            anim(id(), "ppt_w", "#ppt_w", "0")
                + &anim(id(), "ppt_h", "#ppt_h", "0")
                + &fade(id(), "out")
                + &visibility(id(), hide_at, "hidden"),
        ),
        "pulse" => {
            let half = (duration / 2).max(1);
            let scale = id();
            (
                "emph",
                26,
                0,
                format!(
                    "<p:animScale><p:cBhvr><p:cTn id=\"{scale}\" dur=\"{half}\" autoRev=\"1\" fill=\"hold\"/>{target}</p:cBhvr><p:by x=\"115000\" y=\"115000\"/></p:animScale>"
                ),
            )
        }
        "spin" => {
            let rotate = id();
            (
                "emph",
                8,
                0,
                format!(
                    "<p:animRot by=\"21600000\"><p:cBhvr><p:cTn id=\"{rotate}\" dur=\"{duration}\" fill=\"hold\"/>{target}<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot>"
                ),
            )
        }
        "move" => {
            let motion = id();
            let x = a.dx / size.0.max(1.0);
            let y = a.dy / size.1.max(1.0);
            (
                "path",
                0,
                0,
                format!(
                    "<p:animMotion origin=\"layout\" path=\"M 0 0 L {x:.5} {y:.5} E\" pathEditMode=\"relative\" ptsTypes=\"\"><p:cBhvr><p:cTn id=\"{motion}\" dur=\"{duration}\" fill=\"hold\"/>{target}<p:attrNameLst><p:attrName>ppt_x</p:attrName><p:attrName>ppt_y</p:attrName></p:attrNameLst></p:cBhvr></p:animMotion>"
                ),
            )
        }
        _ => (
            "entr",
            10,
            0,
            visibility(id(), 0, "visible") + &fade(id(), "in"),
        ),
    };
    format!(
        "<p:par><p:cTn id=\"{outer}\" presetID=\"{preset}\" presetClass=\"{class}\" presetSubtype=\"{subtype}\" fill=\"hold\" grpId=\"0\" nodeType=\"{node_type}\"><p:stCondLst><p:cond delay=\"{delay}\"/></p:stCondLst><p:childTnLst>{behaviours}</p:childTnLst></p:cTn></p:par>"
    )
}
/// `p:timing` main sequence for the slide's object animations, or an empty string. Shape IDs
/// follow `write_pptx` (element index + 2); timing mirrors the TrineDocs show: a click group per
/// on-click step, an after-previous step opens a sub-group at the previous step's end.
fn timing_xml(slide: &Slide, size: (f32, f32)) -> String {
    struct Step<'a> {
        animation: &'a Animation,
        spid: usize,
        node_type: &'static str,
        delay: u64,
    }
    // Click groups → sub-groups (start offset in ms) → effects.
    type SubGroup<'a> = (u64, Vec<Step<'a>>);
    type ClickGroup<'a> = (bool, Vec<SubGroup<'a>>);
    let mut groups: Vec<ClickGroup<'_>> = Vec::new();
    let (mut previous_start, mut previous_end) = (0.0_f32, 0.0_f32);
    for (index, slot) in slide.animation_steps() {
        let Some(animation) = slide
            .elements
            .get(index)
            .and_then(|e| e.animation_slot(slot))
        else {
            continue;
        };
        let delay = animation.delay.max(0.0);
        let spid = index + 2;
        let trigger = animation.trigger.as_str();
        // Times are relative to the start of the current click group, as in the show.
        let start = if trigger == "on_click" || groups.is_empty() {
            let node_type = match trigger {
                "on_click" => "clickEffect",
                "after_previous" => "afterEffect",
                _ => "withEffect",
            };
            let step = Step {
                animation,
                spid,
                node_type,
                delay: milliseconds(delay),
            };
            groups.push((trigger == "on_click", vec![(0, vec![step])]));
            delay
        } else if trigger == "after_previous" {
            let step = Step {
                animation,
                spid,
                node_type: "afterEffect",
                delay: milliseconds(delay),
            };
            if let Some((_, subs)) = groups.last_mut() {
                subs.push((milliseconds(previous_end), vec![step]));
            }
            previous_end + delay
        } else {
            let start = previous_start + delay;
            if let Some((_, subs)) = groups.last_mut()
                && let Some((offset, steps)) = subs.last_mut()
            {
                steps.push(Step {
                    animation,
                    spid,
                    node_type: "withEffect",
                    delay: milliseconds(start).saturating_sub(*offset),
                });
            }
            start
        };
        previous_start = start;
        previous_end = start + animation.duration;
    }
    if groups.is_empty() {
        return String::new();
    }
    let mut next = 2_u32;
    let mut body = String::new();
    for (index, (clicked, subs)) in groups.iter().enumerate() {
        next += 1;
        let group_id = next;
        let start = if index == 0 && !clicked {
            "<p:cond delay=\"indefinite\"/><p:cond evt=\"onBegin\" delay=\"0\"><p:tn val=\"2\"/></p:cond>"
        } else {
            "<p:cond delay=\"indefinite\"/>"
        };
        let mut inner = String::new();
        for (offset, steps) in subs {
            next += 1;
            let sub_id = next;
            let effects: String = steps
                .iter()
                .map(|s| effect_xml(&mut next, s.animation, s.spid, s.node_type, s.delay, size))
                .collect();
            inner.push_str(&format!("<p:par><p:cTn id=\"{sub_id}\" fill=\"hold\"><p:stCondLst><p:cond delay=\"{offset}\"/></p:stCondLst><p:childTnLst>{effects}</p:childTnLst></p:cTn></p:par>"));
        }
        body.push_str(&format!("<p:par><p:cTn id=\"{group_id}\" fill=\"hold\"><p:stCondLst>{start}</p:stCondLst><p:childTnLst>{inner}</p:childTnLst></p:cTn></p:par>"));
    }
    format!(
        "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst><p:seq concurrent=\"1\" nextAc=\"seek\"><p:cTn id=\"2\" dur=\"indefinite\" nodeType=\"mainSeq\"><p:childTnLst>{body}</p:childTnLst></p:cTn><p:prevCondLst><p:cond evt=\"onPrev\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst><p:nextCondLst><p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"
    )
}
fn sections_xml(deck:&Deck)->String {
    if !deck.slides.iter().any(|slide|!slide.section.is_empty()) {return String::new();}
    let mut body=String::new();let mut start=0;
    while start<deck.slides.len() {
        let name=&deck.slides[start].section;
        let end=(start+1..deck.slides.len()).find(|&index|&deck.slides[index].section!=name).unwrap_or(deck.slides.len());
        let ids=(start..end).map(|index|format!("<p14:sldId id=\"{}\"/>",256+index)).collect::<String>();
        body.push_str(&format!("<p14:section name=\"{}\" id=\"{{00000000-0000-4000-8000-{:012X}}}\"><p14:sldIdLst>{ids}</p14:sldIdLst></p14:section>",xml_escape(name),start+1));
        start=end;
    }
    format!("<p:extLst><p:ext uri=\"{{521415D9-36F7-43E2-AB2F-B90AF26B5E84}}\"><p14:sectionLst xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\">{body}</p14:sectionLst></p:ext></p:extLst>")
}
fn write_pptx(deck: &Deck) -> Result<Vec<u8>, String> {
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    let mut overrides = String::new();
    let mut slide_ids = String::new();
    let mut chart_count = 0;
    let mut image_count = 0;
    let mut presentation_rels = rel("rIdMaster", "slideMaster", "slideMasters/slideMaster1.xml")
        + &rel(
            "rIdNotesMaster",
            "notesMaster",
            "notesMasters/notesMaster1.xml",
        );
    for (index, slide) in deck.slides.iter().enumerate() {
        let number = index + 1;
        slide_ids.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"rIdSlide{number}\"/>",
            256 + index
        ));
        presentation_rels.push_str(&rel(
            &format!("rIdSlide{number}"),
            "slide",
            &format!("slides/slide{number}.xml"),
        ));
        overrides.push_str(&format!("<Override PartName=\"/ppt/slides/slide{number}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/><Override PartName=\"/ppt/notesSlides/notesSlide{number}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>"));
        let mut slide_rels = rel(
            "rIdLayout",
            "slideLayout",
            "../slideLayouts/slideLayout1.xml",
        ) + &rel(
            "rIdNotes",
            "notesSlide",
            &format!("../notesSlides/notesSlide{number}.xml"),
        );
        let mut shapes = String::new();
        for (ei, e) in slide.elements.iter().enumerate() {
            let id = ei + 2;
            match &e.kind {
                ElementKind::Image { bytes } => {
                    let extension = match image::guess_format(bytes)
                        .map_err(|err| format!("Изображение: {err}"))?
                    {
                        image::ImageFormat::Png => "png",
                        image::ImageFormat::Jpeg => "jpg",
                        _ => return Err("Для PPTX используйте изображения PNG или JPEG".into()),
                    };
                    image_count += 1;
                    let relationship = format!("rIdImage{image_count}");
                    slide_rels.push_str(&rel(
                        &relationship,
                        "image",
                        &format!("../media/image{image_count}.{extension}"),
                    ));
                    parts.push((
                        format!("ppt/media/image{image_count}.{extension}"),
                        bytes.clone(),
                    ));
                    shapes.push_str(&format!("<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"Image {id}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{relationship}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",xfrm(e,"a")));
                }
                ElementKind::Table { cells } => shapes.push_str(&table_xml(e, id, cells)?),
                ElementKind::Chart { values, labels } => {
                    chart_count += 1;
                    let relationship = format!("rIdChart{chart_count}");
                    slide_rels.push_str(&rel(
                        &relationship,
                        "chart",
                        &format!("../charts/chart{chart_count}.xml"),
                    ));
                    overrides.push_str(&format!("<Override PartName=\"/ppt/charts/chart{chart_count}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>"));
                    parts.push((
                        format!("ppt/charts/chart{chart_count}.xml"),
                        chart_xml(values, labels, e.fill,&e.text).into_bytes(),
                    ));
                    shapes.push_str(&format!("<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Chart {id}\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"><c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" r:id=\"{relationship}\"/></a:graphicData></a:graphic></p:graphicFrame>",xfrm(e,"p")));
                }
                _ => shapes.push_str(&shape_xml(e, id)),
            }
        }
        let advance = slide
            .transition
            .advance_after
            .map(|s| format!(" advTm=\"{}\"", (s * 1000.0) as u32))
            .unwrap_or_default();
        let effect = match slide.transition.effect.as_str() {
            "fade" => "<p:fade/>".to_string(),
            "push" | "wipe" => format!("<p:{} dir=\"l\"/>", slide.transition.effect),
            _ => String::new(),
        };
        let transition = if !effect.is_empty() || slide.transition.advance_after.is_some() {
            let speed = if slide.transition.duration > 0.75 {
                "slow"
            } else if slide.transition.duration < 0.3 {
                "fast"
            } else {
                "med"
            };
            let duration = (slide.transition.duration * 1000.0).round() as u32;
            format!(
                "<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\"><mc:Choice Requires=\"p14\"><p:transition spd=\"{speed}\" p14:dur=\"{duration}\"{advance}>{effect}</p:transition></mc:Choice><mc:Fallback><p:transition spd=\"{speed}\"{advance}>{effect}</p:transition></mc:Fallback></mc:AlternateContent>"
            )
        } else {
            String::new()
        };
        let xml = wrap(
            "sld",
            &format!("show=\"{}\"", u8::from(!slide.hidden)),
            &format!(
                "<p:cSld name=\"{}\"><p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree>{GROUP}{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>{transition}{}",
                xml_escape(&slide.title),
                hex(slide.background),
                timing_xml(slide, (deck.width, deck.height))
            ),
        );
        parts.push((format!("ppt/slides/slide{number}.xml"), xml.into_bytes()));
        parts.push((
            format!("ppt/slides/_rels/slide{number}.xml.rels"),
            rels(&slide_rels).into_bytes(),
        ));
        let notes = wrap(
            "notes",
            "",
            &format!(
                "<p:cSld><p:spTree>{GROUP}<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr/>{}</p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>",
                text_body(&Element {
                    text: slide.notes.clone(),
                    ..Element::default()
                })
            ),
        );
        parts.push((
            format!("ppt/notesSlides/notesSlide{number}.xml"),
            notes.into_bytes(),
        ));
        parts.push((
            format!("ppt/notesSlides/_rels/notesSlide{number}.xml.rels"),
            rels(
                &(rel(
                    "rIdMaster",
                    "notesMaster",
                    "../notesMasters/notesMaster1.xml",
                ) + &rel("rIdSlide", "slide", &format!("../slides/slide{number}.xml"))),
            )
            .into_bytes(),
        ));
    }
    let color_map = "<p:clrMap accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" bg1=\"lt1\" bg2=\"lt2\" folHlink=\"folHlink\" hlink=\"hlink\" tx1=\"dk1\" tx2=\"dk2\"/>";
    let master = wrap(
        "sldMaster",
        "",
        &format!(
            "<p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld>{color_map}<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rIdLayout\"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles>"
        ),
    );
    let layout = wrap(
        "sldLayout",
        "type=\"blank\" preserve=\"1\"",
        &format!(
            "<p:cSld name=\"Blank\"><p:spTree>{GROUP}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>"
        ),
    );
    let presentation = wrap(
        "presentation",
        "",
        &format!(
            "<p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rIdMaster\"/></p:sldMasterIdLst><p:notesMasterIdLst><p:notesMasterId r:id=\"rIdNotesMaster\"/></p:notesMasterIdLst><p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/><p:defaultTextStyle/>{}",
            emu(deck.width),
            emu(deck.height),
            sections_xml(deck)
        ),
    );
    for (path, kind) in [
        ("presentation.xml", "presentation.main"),
        ("slideMasters/slideMaster1.xml", "slideMaster"),
        ("slideLayouts/slideLayout1.xml", "slideLayout"),
        ("notesMasters/notesMaster1.xml", "notesMaster"),
    ] {
        overrides.push_str(&format!("<Override PartName=\"/ppt/{path}\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.{kind}+xml\"/>"));
    }
    overrides.push_str("<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>");
    parts.extend([
        ("[Content_Types].xml".into(),format!("{XML}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"json\" ContentType=\"application/json\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpg\" ContentType=\"image/jpeg\"/>{overrides}</Types>").into_bytes()),
        ("_rels/.rels".into(),rels(&(rel("rId1","officeDocument","ppt/presentation.xml")+"<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>")).into_bytes()),
        ("ppt/presentation.xml".into(),presentation.into_bytes()),("ppt/_rels/presentation.xml.rels".into(),rels(&presentation_rels).into_bytes()),
        ("ppt/slideMasters/slideMaster1.xml".into(),master.into_bytes()),("ppt/slideMasters/_rels/slideMaster1.xml.rels".into(),rels(&(rel("rIdLayout","slideLayout","../slideLayouts/slideLayout1.xml")+&rel("rIdTheme","theme","../theme/theme1.xml"))).into_bytes()),
        ("ppt/slideLayouts/slideLayout1.xml".into(),layout.into_bytes()),("ppt/slideLayouts/_rels/slideLayout1.xml.rels".into(),rels(&rel("rIdMaster","slideMaster","../slideMasters/slideMaster1.xml")).into_bytes()),
        ("ppt/theme/theme1.xml".into(),theme_xml(deck).into_bytes()),("ppt/notesMasters/notesMaster1.xml".into(),wrap("notesMaster","",&format!("<p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld>{color_map}<p:notesStyle/>")).into_bytes()),("ppt/notesMasters/_rels/notesMaster1.xml.rels".into(),rels(&rel("rIdTheme","theme","../theme/theme1.xml")).into_bytes()),
        ("docProps/core.xml".into(),format!("{XML}<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:creator>TrineDocs</dc:creator><dc:title>{}</dc:title></cp:coreProperties>",xml_escape(deck.slides.first().map(|s| s.title.as_str()).unwrap_or("Презентация"))).into_bytes()),
        ("customXml/trinedocs-deck.json".into(),serde_json::to_vec(deck).map_err(|e| e.to_string())?),
    ]);
    let manifest: BTreeMap<String, u64> = parts
        .iter()
        .filter(|(path, _)| !path.starts_with("customXml/"))
        .map(|(path, bytes)| (path.clone(), part_hash(bytes)))
        .collect();
    parts.push((
        "customXml/trinedocs-manifest.json".into(),
        serde_json::to_vec(&manifest).map_err(|e| e.to_string())?,
    ));
    write_zip(parts)
}
#[cfg(test)]
mod tests {
    #[test]
    fn review5_external_pptx_keeps_exact_transition_duration() -> Result<(), String> {
        let mut deck = Deck::default();
        deck.slides[0].transition.effect = "fade".into();
        deck.slides[0].transition.duration = 1.375;
        let mut parts = read_zip(&write(&deck, "pptx")?)?;
        parts.remove("customXml/trinedocs-deck.json");
        let imported = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        assert_eq!(imported.slides[0].transition.effect, "fade");
        assert!((imported.slides[0].transition.duration - 1.375).abs() < 0.001);
        Ok(())
    }
    #[test]
    fn fallback_shapes_and_partial_run_properties_import_without_losing_text_style()
    -> Result<(), String> {
        let slide = format!(
            r#"<p:sld {NS} xmlns:mc="urn:mc"><p:cSld><p:spTree><mc:AlternateContent><mc:Choice Requires="unsupported"/><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="2" name="Text"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:p><a:pPr><a:defRPr sz="3200" i="1"><a:solidFill><a:srgbClr val="123456"/></a:solidFill><a:latin typeface="Arial"/></a:defRPr></a:pPr><a:r><a:rPr b="1"/><a:t>Fallback</a:t></a:r></a:p></p:txBody></p:sp></mc:Fallback></mc:AlternateContent></p:spTree></p:cSld></p:sld>"#
        );
        let deck = external_deck(&slide)?;
        let e = deck
            .slides
            .first()
            .and_then(|s| s.elements.first())
            .ok_or("fallback")?;
        assert_eq!(e.text, "Fallback");
        assert_eq!(e.font_size, 32.0);
        assert!(e.bold && e.italic);
        assert_eq!(e.font, "Arial");
        assert_eq!(e.color, [0x12, 0x34, 0x56]);
        Ok(())
    }
    fn without_native(deck: &Deck) -> Result<Deck, String> {
        let parts = read_zip(&write(deck, "pptx")?)?
            .into_iter()
            .filter(|(path, _)| !path.starts_with("customXml/"))
            .collect();
        read(&write_zip(parts)?, "pptx")
    }
    #[test]
    fn external_table_roundtrip_preserves_the_shared_supported_style() -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        let id = deck.add_element(
            slide,
            ElementKind::Table {
                cells: vec![vec!["A".into(), "B".into()]],
            },
        )?;
        let e = deck
            .slide_mut(slide)
            .and_then(|s| s.element_mut(id))
            .ok_or("table")?;
        e.font_size = 24.0;
        e.bold = true;
        e.italic = true;
        e.color = [11, 22, 33];
        e.fill = [44, 55, 66];
        e.stroke = [77, 88, 99];
        e.stroke_width = 2.0;
        e.font = "Arial".into();
        let again = without_native(&deck)?;
        let e = &again.slides[0].elements[0];
        assert_eq!(e.font_size, 24.0);
        assert!(e.bold && e.italic);
        assert_eq!(e.color, [11, 22, 33]);
        assert_eq!(e.fill, [44, 55, 66]);
        assert_eq!(e.stroke, [77, 88, 99]);
        assert_eq!(e.stroke_width, 2.0);
        assert_eq!(e.font, "Arial");
        Ok(())
    }
    #[test]
    fn object_animations_roundtrip_through_powerpoint_timing() -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        let mut ids = Vec::new();
        for _ in 0..3 {
            ids.push(deck.add_element(slide, ElementKind::Rect)?);
        }
        // Sequence order (3, 1, 2) differs from the layer order.
        for (id, effect, trigger, order) in [
            (ids[0], "fly_in", "on_click", 2),
            (ids[1], "zoom", "after_previous", 3),
            (ids[2], "fade", "with_previous", 1),
        ] {
            let e = deck
                .slide_mut(slide)
                .and_then(|s| s.element_mut(id))
                .ok_or("element")?;
            e.animation = Animation {
                effect: effect.into(),
                trigger: trigger.into(),
                delay: 0.25,
                duration: 0.75,
                order,
                ..Animation::default()
            };
        }
        // Emphasis, motion and exit steps after the entrances, on the same objects.
        let step = |effect: &str, trigger: &str, order: u32| Animation {
            effect: effect.into(),
            trigger: trigger.into(),
            delay: 0.25,
            duration: 0.75,
            order,
            ..Animation::default()
        };
        let first = deck
            .slide_mut(slide)
            .and_then(|s| s.element_mut(ids[0]))
            .ok_or("element")?;
        first.more_animations = vec![
            step("spin", "after_previous", 4),
            Animation {
                dx: 100.0,
                dy: -50.0,
                ..step("move", "with_previous", 5)
            },
            step("fade_out", "on_click", 6),
        ];
        deck.slide_mut(slide)
            .and_then(|s| s.element_mut(ids[1]))
            .ok_or("element")?
            .more_animations = vec![step("pulse", "after_previous", 7)];
        let parts = read_zip(&write(&deck, "pptx")?)?;
        let xml = String::from_utf8(parts.get("ppt/slides/slide1.xml").cloned().ok_or("slide")?)
            .map_err(|e| e.to_string())?;
        assert!(xml.contains("<p:timing>") && xml.contains("nodeType=\"mainSeq\""));
        assert!(xml.contains("presetID=\"2\"") && xml.contains("presetID=\"53\""));
        for class in ["entr", "emph", "path", "exit"] {
            assert!(xml.contains(&format!("presetClass=\"{class}\"")), "{class}");
        }
        assert!(xml.contains("<p:animRot by=\"21600000\">") && xml.contains("<p:animScale>"));
        assert!(xml.find("<p:spTgt spid=\"4\"/>") < xml.find("<p:spTgt spid=\"2\"/>"));
        parse(xml.as_bytes())?;
        let again = without_native(&deck)?;
        let slide = &again.slides[0];
        assert_eq!(slide.animation_sequence(), vec![2, 0, 1, 0, 0, 0, 1]);
        let steps: Vec<(&str, &str)> = slide
            .animation_steps()
            .into_iter()
            .filter_map(|(index, step)| slide.elements.get(index)?.animation_slot(step))
            .map(|a| (a.effect.as_str(), a.trigger.as_str()))
            .collect();
        assert_eq!(
            steps,
            vec![
                ("fade", "with_previous"),
                ("fly_in", "on_click"),
                ("zoom", "after_previous"),
                ("spin", "after_previous"),
                ("move", "with_previous"),
                ("fade_out", "on_click"),
                ("pulse", "after_previous"),
            ]
        );
        assert!(
            slide
                .elements
                .iter()
                .flat_map(Element::animations)
                .all(|(_, a)| (a.duration - 0.75).abs() < 0.001 && (a.delay - 0.25).abs() < 0.001)
        );
        let motion = slide
            .elements
            .first()
            .and_then(|e| e.animation_slot(2))
            .ok_or("move")?;
        assert!((motion.dx - 100.0).abs() < 0.05 && (motion.dy + 50.0).abs() < 0.05);
        assert_eq!(slide.click_steps(), 2);
        assert!(!again.warnings.iter().any(|w| w.contains("анимации")));
        Ok(())
    }
    #[test]
    fn external_timing_rebuilds_delays_and_counts_unsupported_effects() -> Result<(), String> {
        // A PowerPoint-style main sequence: one auto group with an effect inside a nested
        // sub-group at 500 ms, then a click group with an unsupported "Wheel" entrance (21) and
        // a curved motion path, plus an object-triggered interactive sequence.
        let effect = |id: u32, class: &str, preset: u32, node: &str, delay: u32, inner: &str| {
            format!(
                "<p:par><p:cTn id=\"{id}\" presetID=\"{preset}\" presetClass=\"{class}\" nodeType=\"{node}\"><p:stCondLst><p:cond delay=\"{delay}\"/></p:stCondLst><p:childTnLst>{inner}</p:childTnLst></p:cTn></p:par>"
            )
        };
        let target = |spid: u32| format!("<p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>");
        let fade = format!(
            "<p:animEffect transition=\"in\" filter=\"fade\"><p:cBhvr><p:cTn id=\"90\" dur=\"400\"/>{}</p:cBhvr></p:animEffect>",
            target(2)
        );
        let wheel = format!(
            "<p:animEffect transition=\"in\" filter=\"wheel(1)\"><p:cBhvr><p:cTn id=\"91\" dur=\"1000\"/>{}</p:cBhvr></p:animEffect>",
            target(3)
        );
        let curve = format!(
            "<p:animMotion path=\"M 0 0 C 0.1 0.1 0.2 0.1 0.25 0.5 E\"><p:cBhvr><p:cTn id=\"92\" dur=\"2000\"/>{}</p:cBhvr></p:animMotion>",
            target(2)
        );
        let sub = |delay: u32, inner: String| {
            format!(
                "<p:par><p:cTn id=\"80\"><p:stCondLst><p:cond delay=\"{delay}\"/></p:stCondLst><p:childTnLst>{inner}</p:childTnLst></p:cTn></p:par>"
            )
        };
        let auto = format!(
            "<p:par><p:cTn id=\"3\"><p:stCondLst><p:cond delay=\"indefinite\"/><p:cond evt=\"onBegin\" delay=\"0\"><p:tn val=\"2\"/></p:cond></p:stCondLst><p:childTnLst>{}</p:childTnLst></p:cTn></p:par>",
            sub(500, effect(4, "entr", 10, "afterEffect", 100, &fade))
        );
        let click = format!(
            "<p:par><p:cTn id=\"5\"><p:stCondLst><p:cond delay=\"indefinite\"/></p:stCondLst><p:childTnLst>{}</p:childTnLst></p:cTn></p:par>",
            sub(
                0,
                effect(6, "entr", 21, "clickEffect", 0, &wheel)
                    + &effect(7, "path", 0, "withEffect", 300, &curve)
            )
        );
        let interactive = format!(
            "<p:seq><p:cTn id=\"8\" nodeType=\"interactiveSeq\"><p:childTnLst>{}</p:childTnLst></p:cTn></p:seq>",
            effect(9, "emph", 26, "clickEffect", 0, "")
        );
        let xml = format!(
            "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" nodeType=\"tmRoot\"><p:childTnLst><p:seq><p:cTn id=\"2\" nodeType=\"mainSeq\"><p:childTnLst>{auto}{click}</p:childTnLst></p:cTn></p:seq>{interactive}</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"
        );
        let timing = parse(xml.as_bytes())?;
        let mut slide = Slide {
            elements: vec![Element::default(), Element::default()],
            ..Slide::default()
        };
        let shape_ids = BTreeMap::from([("2".to_string(), 0), ("3".to_string(), 1)]);
        let inexact = read_timing(&timing, &shape_ids, &mut slide, (960.0, 540.0));
        // Wheel → fade, curved path → straight move, interactive pulse skipped.
        assert_eq!(inexact, 3);
        let fade = &slide.elements[0].animation;
        assert_eq!(
            (fade.effect.as_str(), fade.trigger.as_str()),
            ("fade", "with_previous")
        );
        assert!((fade.delay - 0.6).abs() < 0.001 && (fade.duration - 0.4).abs() < 0.001);
        let wheel = &slide.elements[1].animation;
        assert_eq!(
            (wheel.effect.as_str(), wheel.trigger.as_str()),
            ("fade", "on_click")
        );
        let motion = slide.elements[0].more_animations.first().ok_or("move")?;
        assert_eq!(
            (motion.effect.as_str(), motion.trigger.as_str()),
            ("move", "with_previous")
        );
        assert!((motion.delay - 0.3).abs() < 0.001 && (motion.duration - 2.0).abs() < 0.001);
        assert!((motion.dx - 240.0).abs() < 0.01 && (motion.dy - 270.0).abs() < 0.01);
        assert_eq!(slide.animation_sequence(), vec![0, 1, 0]);
        Ok(())
    }
    #[test]
    fn chart_sparse_caches_keep_point_positions_and_missing_labels_get_numbers()
    -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        deck.add_element(
            slide,
            ElementKind::Chart {
                values: vec![10.0, 0.0, 30.0],
                labels: vec![],
            },
        )?;
        let again = without_native(&deck)?;
        assert!(
            matches!(&again.slides[0].elements[0].kind, ElementKind::Chart { labels, .. } if labels == &["1", "2", "3"])
        );
        let mut parts = read_zip(&write(&deck, "pptx")?)?;
        let chart_path = parts
            .keys()
            .find(|path| path.starts_with("ppt/charts/") && path.ends_with(".xml"))
            .cloned()
            .ok_or("chart")?;
        let xml = String::from_utf8(parts[&chart_path].clone())
            .map_err(|e| e.to_string())?
            .replace("<c:pt idx=\"1\"><c:v>0</c:v></c:pt>", "");
        parts.insert(chart_path, xml.into_bytes());
        parts.retain(|path, _| !path.starts_with("customXml/"));
        let again = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        assert!(
            matches!(&again.slides[0].elements[0].kind, ElementKind::Chart { values, .. } if values == &[10.0, 0.0, 30.0])
        );
        Ok(())
    }

    use super::*;
    #[test]
    fn text_and_html_export_preserve_tables_and_charts() -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        deck.add_element(
            slide,
            ElementKind::Table {
                cells: vec![vec!["A&B".into(), "<2>\n次".into()]],
            },
        )?;
        deck.add_element(
            slide,
            ElementKind::Chart {
                values: vec![-12.5, 0.0],
                labels: vec!["Продажи".into(), "Расходы".into(), "Без значения".into()],
            },
        )?;
        for extension in ["txt", "md"] {
            let text =
                String::from_utf8(write(&deck, extension)?).map_err(|error| error.to_string())?;
            assert!(text.contains("A&B\t<2>\n次"));
            assert!(text.contains("Продажи\t-12.5"));
            assert!(text.contains("Расходы\t0"));
            assert!(text.contains("Без значения\t"));
        }
        let html = String::from_utf8(write(&deck, "html")?).map_err(|error| error.to_string())?;
        assert!(html.contains("<td>A&amp;B</td><td>&lt;2&gt;\n次</td>"));
        assert!(html.contains("Продажи\t-12.5"));
        assert!(html.contains("white-space:pre-wrap"));
        Ok(())
    }
    #[test]
    fn pptx_roundtrip_preserves_model() {
        let mut deck = Deck::default();
        let id = deck.add_slide("blank");
        assert!(
            deck.add_element(
                id,
                ElementKind::Table {
                    cells: vec![vec!["A&B".into(), "2".into()]]
                }
            )
            .is_ok()
        );
        assert!(
            deck.add_element(
                id,
                ElementKind::Chart {
                    values: vec![1.0, 3.0],
                    labels: vec!["A".into(), "Б".into()]
                }
            )
            .is_ok()
        );
        if let Some(s) = deck.slide_mut(id) {
            s.notes = "Заметки\n日本語".into();
            s.transition.effect = "fade".into();
        }
        let bytes = write(&deck, "pptx");
        assert!(bytes.is_ok());
        if let Ok(bytes) = bytes {
            assert_eq!(read(&bytes, "pptx").ok(), Some(deck));
        }
    }
    #[test]
    fn native_roundtrip() {
        let d = Deck::default();
        let b = write(&d, "trineslide");
        assert!(b.is_ok());
        if let Ok(b) = b {
            assert_eq!(read(&b, "trineslide").ok(), Some(d));
        }
    }
    #[test]
    fn external_pptx_reads_shapes() {
        let bytes = write(&Deck::default(), "pptx");
        assert!(bytes.is_ok());
        if let Ok(bytes) = bytes
            && let Ok(mut parts) = read_zip(&bytes)
        {
            parts.remove("customXml/trinedocs-deck.json");
            let bytes = write_zip(parts.into_iter().collect());
            assert!(bytes.is_ok());
            if let Ok(bytes) = bytes {
                let d = read(&bytes, "pptx");
                assert!(d.is_ok());
                if let Ok(d) = d {
                    assert_eq!(d.slides.len(), 1);
                    assert_eq!(
                        d.slides
                            .first()
                            .and_then(|s| s.elements.first())
                            .map(|e| e.text.as_str()),
                        Some("Заголовок")
                    );
                }
            }
        }
    }
    #[test]
    fn rejects_doctype() {
        assert!(parse(b"<!DOCTYPE x [<!ENTITY e SYSTEM 'file:///secret'>]><x>&e;</x>").is_err());
    }
    #[test]
    fn rejects_invalid_xml() {
        assert!(parse(b"<x><y></x>").is_err());
    }
    #[test]
    fn entities_roundtrip() {
        assert_eq!(
            parse("<x><t>A&amp;Б&lt;日本語</t></x>".as_bytes())
                .ok()
                .map(|n| n.text_content()),
            Some("A&Б<日本語".into())
        );
    }

    #[test]
    fn ooxml_chart_and_table_namespaces() -> Result<(), String> {
        let mut deck = Deck::default();
        let id = deck.add_slide("blank");
        deck.add_element(
            id,
            ElementKind::Table {
                cells: vec![vec!["cell".into()]],
            },
        )?;
        deck.add_element(
            id,
            ElementKind::Chart {
                values: vec![42.0],
                labels: vec!["label".into()],
            },
        )?;
        let parts = read_zip(&write(&deck, "pptx")?)?;
        let chart = parse(parts.get("ppt/charts/chart1.xml").ok_or("chart part")?)?;
        assert_eq!(
            chart.attr("xmlns:c"),
            "http://schemas.openxmlformats.org/drawingml/2006/chart"
        );
        let slide = parse(parts.get("ppt/slides/slide2.xml").ok_or("slide part")?)?;
        let mut data = Vec::new();
        slide.all("graphicData", &mut data);
        assert!(
            data.iter()
                .any(|n| n.attr("uri") == "http://schemas.openxmlformats.org/drawingml/2006/table")
        );
        assert!(
            data.iter()
                .any(|n| n.attr("uri") == "http://schemas.openxmlformats.org/drawingml/2006/chart")
        );
        Ok(())
    }
    #[test]
    fn modified_ooxml_does_not_load_stale_native_model() -> Result<(), String> {
        let deck = Deck::default();
        let mut parts = read_zip(&write(&deck, "pptx")?)?;
        let source = parts.get("ppt/slides/slide1.xml").ok_or("slide part")?;
        let edited = String::from_utf8(source.clone())
            .map_err(|e| e.to_string())?
            .replace("Заголовок", "Изменено в PowerPoint");
        parts.insert("ppt/slides/slide1.xml".into(), edited.into_bytes());
        let imported = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        assert_eq!(
            imported
                .slides
                .first()
                .and_then(|s| s.elements.first())
                .map(|e| e.text.as_str()),
            Some("Изменено в PowerPoint")
        );
        Ok(())
    }
    #[test]
    fn native_api_uses_shared_envelope() -> Result<(), String> {
        let deck = Deck::default();
        let bytes = write(&deck, "trineslide")?;
        assert_eq!(
            crate::native::decode(&bytes)?,
            crate::native::Document::Slides(deck.clone())
        );
        assert_eq!(
            read(
                &crate::native::encode(&crate::native::Document::Slides(deck.clone()))?,
                "trineslide"
            )?,
            deck
        );
        Ok(())
    }
    #[test]
    fn external_roundtrip_preserves_notes_title_line_arrow_auto() -> Result<(), String> {
        let mut deck = Deck::default();
        let id = deck.slides.first().map(|s| s.id).ok_or("slide")?;
        deck.add_element(id, ElementKind::Arrow)?;
        let slide = deck.slide_mut(id).ok_or("slide")?;
        slide.title = "Имя в миниатюре".into();
        slide.notes = "Первая строка\nВторая строка".into();
        slide.transition.advance_after = Some(5.0);
        let mut parts = read_zip(&write(&deck, "pptx")?)?;
        parts.remove("customXml/trinedocs-deck.json");
        let imported = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        let slide = imported.slides.first().ok_or("slide")?;
        assert_eq!(slide.title, "Имя в миниатюре");
        assert_eq!(slide.notes, "Первая строка\nВторая строка");
        assert_eq!(slide.transition.effect, "none");
        assert_eq!(slide.transition.advance_after, Some(5.0));
        assert!(
            slide
                .elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Arrow))
        );
        Ok(())
    }
    #[test]
    fn drawingml_soft_break_and_missing_picture_are_safe() -> Result<(), String> {
        assert_eq!(parse(b"<a:txBody><a:p><a:r><a:t>A</a:t></a:r><a:br/><a:r><a:t>B</a:t></a:r></a:p><a:p><a:r><a:t>C</a:t></a:r></a:p></a:txBody>")?.paragraph_text(), "A\nB\nC");
        let mut parts = read_zip(&write(&Deck::default(), "pptx")?)?;
        parts.remove("customXml/trinedocs-deck.json");
        let source = String::from_utf8(parts.get("ppt/slides/slide1.xml").ok_or("slide")?.clone())
            .map_err(|e| e.to_string())?;
        let source = source.replace("</p:spTree>", "<p:pic><p:nvPicPr><p:cNvPr id=\"100\" name=\"missing\"/></p:nvPicPr><p:blipFill><a:blip r:embed=\"missing\"/></p:blipFill></p:pic></p:spTree>");
        parts.insert("ppt/slides/slide1.xml".into(), source.into_bytes());
        let imported = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        assert!(imported.warnings.iter().any(|s| s.contains("изображение")));
        assert!(imported.validate().is_ok());
        Ok(())
    }
    const NS: &str = "xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"";
    fn rels(entries: &[(&str, &str)]) -> Vec<u8> {
        let body = entries
            .iter()
            .map(|(id, target)| {
                let kind = if target.contains("slideLayouts") { "slideLayout" }
                    else if target.contains("slideMasters") { "slideMaster" }
                    else if target.contains("notesSlides") { "notesSlide" }
                    else if target.contains("theme") { "theme" } else { "slide" };
                format!("<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/>")
            })
            .collect::<String>();
        format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>").into_bytes()
    }
    /// A PowerPoint-like package: slide → layout → master → theme, plus a notes page.
    fn external_parts(slide: &str) -> Vec<(String, Vec<u8>)> {
        let part = |path: &str, xml: String| (path.to_string(), xml.into_bytes());
        let parts = vec![
            part("ppt/presentation.xml", format!("<p:presentation {NS}><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId1\"/></p:sldIdLst><p:sldSz cx=\"12192000\" cy=\"6858000\"/></p:presentation>")),
            ("ppt/_rels/presentation.xml.rels".into(), rels(&[("rId1", "slides/slide1.xml")])),
            part("ppt/slides/slide1.xml", slide.to_string()),
            ("ppt/slides/_rels/slide1.xml.rels".into(), rels(&[("rId1", "../slideLayouts/slideLayout1.xml"), ("rId2", "../notesSlides/notesSlide1.xml")])),
            part("ppt/slideLayouts/slideLayout1.xml", format!("<p:sldLayout {NS}><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"T\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"1270000\" y=\"635000\"/><a:ext cx=\"10160000\" cy=\"1270000\"/></a:xfrm></p:spPr></p:sp></p:spTree></p:cSld></p:sldLayout>")),
            ("ppt/slideLayouts/_rels/slideLayout1.xml.rels".into(), rels(&[("rId1", "../slideMasters/slideMaster1.xml")])),
            part("ppt/slideMasters/slideMaster1.xml", format!("<p:sldMaster {NS}><p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"bg2\"/></a:solidFill></p:bgPr></p:bg><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"B\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"1270000\" y=\"2540000\"/><a:ext cx=\"10160000\" cy=\"3810000\"/></a:xfrm></p:spPr></p:sp></p:spTree></p:cSld></p:sldMaster>")),
            ("ppt/slideMasters/_rels/slideMaster1.xml.rels".into(), rels(&[("rId1", "../theme/theme1.xml")])),
            part("ppt/theme/theme1.xml", "<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:themeElements><a:clrScheme name=\"T\"><a:dk1><a:sysClr val=\"windowText\" lastClr=\"000000\"/></a:dk1><a:lt1><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"1F497D\"/></a:dk2><a:lt2><a:srgbClr val=\"EEECE1\"/></a:lt2><a:accent1><a:srgbClr val=\"4F81BD\"/></a:accent1></a:clrScheme><a:fontScheme name=\"T\"><a:majorFont><a:latin typeface=\"Georgia\"/></a:majorFont><a:minorFont><a:latin typeface=\"Verdana\"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>".into()),
            part("ppt/notesSlides/notesSlide1.xml", format!("<p:notes {NS}><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"N\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"sldNum\" idx=\"5\"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:fld type=\"slidenum\"><a:t>1</a:t></a:fld></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Body\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:r><a:t>Говорить медленно</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>")),
        ];
        parts
    }
    fn external_deck(slide: &str) -> Result<Deck, String> {
        read(&write_zip(external_parts(slide))?, "pptx")
    }
    fn typed_rels(entries: &[(&str, &str, &str)]) -> Vec<u8> {
        let body = entries.iter().map(|(id, kind, target)| format!(
            "<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/>"
        )).collect::<String>();
        format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>").into_bytes()
    }
    #[test]
    fn pptx_relationship_roles_do_not_depend_on_part_names() -> Result<(), String> {
        let slide = format!("<p:sld {NS}><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>Custom paths</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>");
        let mut parts = external_parts(&slide).into_iter().collect::<BTreeMap<_,_>>();
        for (from, to) in [
            ("ppt/presentation.xml","deck.xml"),
            ("ppt/slides/slide1.xml","content/s1.xml"),
            ("ppt/slideLayouts/slideLayout1.xml","design/l.xml"),
            ("ppt/slideMasters/slideMaster1.xml","design/m.xml"),
            ("ppt/theme/theme1.xml","colors/t.xml"),
            ("ppt/notesSlides/notesSlide1.xml","notes/n.xml"),
        ] {
            let bytes = parts.remove(from).ok_or("fixture part")?;
            parts.insert(to.into(), bytes);
        }
        parts.insert("_rels/.rels".into(), typed_rels(&[("r1","officeDocument","deck.xml")]));
        parts.insert("_rels/deck.xml.rels".into(), typed_rels(&[("rId1","slide","content/s1.xml")]));
        parts.insert("content/_rels/s1.xml.rels".into(), typed_rels(&[
            ("r1","slideLayout","../design/l.xml"),("r2","notesSlide","../notes/n.xml")]));
        parts.insert("design/_rels/l.xml.rels".into(), typed_rels(&[("r1","slideMaster","m.xml")]));
        parts.insert("design/_rels/m.xml.rels".into(), typed_rels(&[("r1","theme","../colors/t.xml")]));
        let deck = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        assert_eq!(deck.theme.font, "Verdana");
        assert_eq!(deck.slides[0].notes, "Говорить медленно");
        assert_eq!(deck.slides[0].background, [238,236,225]);
        assert_eq!(deck.slides[0].elements[0].x, 100.0);
        Ok(())
    }
    #[test]
    fn pptx_color_maps_resolve_theme_slots() -> Result<(), String> {
        let template = format!("<p:sld {NS}><p:cSld><p:spTree/></p:cSld>{{override}}</p:sld>");
        let load = |mapping: &str| -> Result<Deck,String> {
            let mut parts = external_parts(&template.replace("{override}",mapping))
                .into_iter().collect::<BTreeMap<_,_>>();
            let master = String::from_utf8(parts.remove("ppt/slideMasters/slideMaster1.xml").ok_or("master")?)
                .map_err(|e|e.to_string())?;
            parts.insert("ppt/slideMasters/slideMaster1.xml".into(),
                master.replace("</p:sldMaster>","<p:clrMap bg2=\"accent1\" tx1=\"accent1\"/></p:sldMaster>").into_bytes());
            read(&write_zip(parts.into_iter().collect())?, "pptx")
        };
        let master = load("")?;
        assert_eq!(master.slides[0].background,[79,129,189]);
        assert_eq!(master.theme.foreground,[79,129,189]);
        let overridden = load("<p:clrMapOvr><a:overrideClrMapping bg2=\"dk2\" tx1=\"dk1\"/></p:clrMapOvr>")?;
        assert_eq!(overridden.slides[0].background,[31,73,125]);
        assert_eq!(overridden.theme.foreground,[0,0,0]);
        assert_eq!(load("<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>")?.slides[0].background,
            master.slides[0].background);
        Ok(())
    }
    #[test]
    fn drawingml_color_transforms_preserve_order() -> Result<(), String> {
        let transformed = |body: &str| -> Result<[u8;3],String> {
            Ok(color(Some(&parse(format!("<a:srgbClr val=\"FF0000\">{body}</a:srgbClr>").as_bytes())?),
                &Look::default(),[0,0,0]))
        };
        assert_eq!(transformed("<a:tint val=\"10000\"/>")?,[255,230,230]);
        assert_eq!(transformed("<a:shade val=\"10000\"/>")?,[26,0,0]);
        assert_eq!(transformed("<a:lumMod val=\"50000\"/>")?,[128,0,0]);
        // Values are rounded only when stored as RGB; allow one channel unit.
        let rgb = transformed("<a:lumMod val=\"50000\"/><a:lumOff val=\"50000\"/>")?;
        assert_eq!(rgb[0],255);
        assert!((i16::from(rgb[1])-128).abs()<=1);
        assert!((i16::from(rgb[2])-128).abs()<=1);
        Ok(())
    }
    #[test]
    fn external_pptx_inherits_layout_master_and_theme() -> Result<(), String> {
        let slide = format!(
            "<p:sld {NS} show=\"false\"><p:cSld><p:spTree><p:nvGrpSpPr/><p:grpSpPr/>\
             <p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"TB\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"1270000\" cy=\"254000\"/></a:xfrm><a:prstGeom prst=\"rect\"/><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>Подпись</a:t></a:r></a:p></p:txBody></p:sp>\
             <p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>Квартальный отчёт</a:t></a:r></a:p></p:txBody></p:sp>\
             </p:spTree></p:cSld></p:sld>"
        );
        let deck = external_deck(&slide)?;
        let slide = deck.slides.first().ok_or("slide")?;
        assert!(slide.hidden);
        assert_eq!(slide.title, "Квартальный отчёт");
        assert_eq!(slide.background, [238, 236, 225]);
        assert_eq!(slide.notes, "Говорить медленно");
        assert_eq!(deck.theme.font, "Verdana");
        assert_eq!(deck.theme.accent, [79, 129, 189]);
        let caption = slide.elements.first().ok_or("caption")?;
        assert!(matches!(caption.kind, ElementKind::Text));
        assert_eq!(
            (caption.font.as_str(), caption.font_size),
            ("Verdana", 18.0)
        );
        let title = slide.elements.get(1).ok_or("title")?;
        assert_eq!(
            (title.x, title.y, title.w, title.h),
            (100.0, 50.0, 800.0, 100.0)
        );
        assert_eq!((title.font.as_str(), title.font_size), ("Georgia", 44.0));
        assert_eq!(title.color, [0, 0, 0]);
        Ok(())
    }
    #[test]
    fn external_pptx_groups_flips_outline_and_transition() -> Result<(), String> {
        let slide = format!(
            "<p:sld {NS}><p:cSld><p:spTree><p:nvGrpSpPr/><p:grpSpPr/>\
             <p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"4\" name=\"G\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"1270000\" y=\"1270000\"/><a:ext cx=\"2540000\" cy=\"2540000\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"1270000\" cy=\"1270000\"/></a:xfrm></p:grpSpPr>\
             <p:sp><p:nvSpPr><p:cNvPr id=\"5\" name=\"R\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"635000\" y=\"0\"/><a:ext cx=\"635000\" cy=\"635000\"/></a:xfrm><a:prstGeom prst=\"rect\"/><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill><a:ln w=\"25400\"><a:noFill/></a:ln></p:spPr></p:sp></p:grpSp>\
             <p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"6\" name=\"L\"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm flipH=\"1\"><a:off x=\"0\" y=\"0\"/><a:ext cx=\"1270000\" cy=\"1270000\"/></a:xfrm><a:prstGeom prst=\"line\"/><a:ln w=\"12700\"><a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill></a:ln></p:spPr></p:cxnSp>\
             </p:spTree></p:cSld>\
             <mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\"><mc:Choice Requires=\"p14\"><p:transition spd=\"slow\" p14:dur=\"1200\"><p14:prism/></p:transition></mc:Choice><mc:Fallback><p:transition spd=\"slow\"><p:fade/></p:transition></mc:Fallback></mc:AlternateContent></p:sld>"
        );
        let deck = external_deck(&slide)?;
        assert!(!deck.warnings.iter().any(|w| w.contains("Поворот групп")));
        let slide = deck.slides.first().ok_or("slide")?;
        let rect = slide.elements.first().ok_or("rect")?;
        assert!(matches!(rect.kind, ElementKind::Rect));
        assert_eq!(
            (rect.x, rect.y, rect.w, rect.h),
            (200.0, 100.0, 100.0, 100.0)
        );
        assert_eq!(rect.fill, [79, 129, 189]);
        assert_eq!(rect.stroke_width, 0.0);
        let line = slide.elements.get(1).ok_or("line")?;
        assert!(matches!(line.kind, ElementKind::Line));
        let [x1, y1, x2, y2] = super::super::model::line_points(line);
        assert!((x1 - 100.0).abs() < 0.01 && y1.abs() < 0.01, "{x1} {y1}");
        assert!(x2.abs() < 0.01 && (y2 - 100.0).abs() < 0.01, "{x2} {y2}");
        assert_eq!(line.stroke, [255, 0, 0]);
        assert_eq!(slide.transition.effect, "fade");
        assert!((slide.transition.duration - 1.2).abs() < 0.001);
        Ok(())
    }
    #[test]
    fn pptx_export_no_outline_and_multiline_cells_roundtrip() -> Result<(), String> {
        let mut deck = Deck::default();
        let id = deck.add_slide("blank");
        let rect = deck.add_element(id, ElementKind::Rect)?;
        deck.add_element(
            id,
            ElementKind::Table {
                cells: vec![vec!["Строка 1\nСтрока 2".into(), "B".into()]],
            },
        )?;
        if let Some(e) = deck
            .slide_mut(id)
            .and_then(|s| s.elements.iter_mut().find(|e| e.id == rect))
        {
            e.stroke_width = 0.0;
        }
        let mut parts = read_zip(&write(&deck, "pptx")?)?;
        let slide_xml =
            String::from_utf8(parts.get("ppt/slides/slide2.xml").ok_or("slide")?.clone())
                .map_err(|e| e.to_string())?;
        assert!(!slide_xml.contains("<a:ln w=\"0\""));
        assert!(slide_xml.contains("<a:t xml:space=\"preserve\">Строка 2</a:t>"));
        parts.remove("customXml/trinedocs-deck.json");
        let imported = read(&write_zip(parts.into_iter().collect())?, "pptx")?;
        let slide = imported.slides.get(1).ok_or("slide")?;
        let rect = slide
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Rect))
            .ok_or("rect")?;
        assert_eq!(rect.stroke_width, 0.0);
        assert!(slide.elements.iter().any(|e| matches!(
            &e.kind,
            ElementKind::Table { cells } if cells.first().and_then(|r| r.first()).map(String::as_str) == Some("Строка 1\nСтрока 2")
        )));
        Ok(())
    }
}

#[cfg(test)]
mod additional_text_property_tests {
    use super::*;
    #[test]
    fn future_run_properties_do_not_restyle_existing_slide_text()->Result<(),String> {
        let body=parse(br#"<txBody><p><pPr><defRPr sz="1600"><solidFill><srgbClr val="123456"/></solidFill><latin typeface="Georgia"/></defRPr></pPr><r><t>text</t></r><endParaRPr sz="4000" b="1"><solidFill><srgbClr val="FF0000"/></solidFill><latin typeface="Verdana"/></endParaRPr></p></txBody>"#)?;
        let mut element=Element::default();
        apply_text_look(&body,&mut element,&Look::default(),None,12.0,false);
        assert_eq!(element.font_size,16.0);assert!(!element.bold);
        assert_eq!(element.color,[18,52,86]);assert_eq!(element.font,"Georgia");Ok(())
    }
}

#[cfg(test)]
mod next_read_limit_tests {
    #[test]
    fn slide_json_limits_apply_before_deserializing_the_document() {
        let json=br#"{"slides":[],"padding":"long long long"}"#;
        let error=super::read_with_limit(json,"json",8).err().unwrap_or_default();
        assert!(error.contains("размер"));
    }
}

#[cfg(test)]
mod next_chart_title_tests {
    use super::*;
    #[test]
    fn pptx_chart_exports_the_object_title_as_rich_chart_text()->Result<(),String> {
        let xml=chart_xml(&[1.0,2.0],&["a".into(),"b".into()],[1,2,3],"Revenue & costs");
        let node=parse(xml.as_bytes())?;
        assert_eq!(node.descend("title").map(Node::text_content).as_deref(),Some("Revenue & costs"));Ok(())
    }
}

#[cfg(test)]
mod next_no_fill_tests {
    use super::*;
    #[test]
    fn transparent_auto_shapes_keep_their_geometry_outline_and_text_in_pptx()->Result<(),String> {
        let mut deck=Deck::empty();let slide=deck.add_slide("blank");
        let object=deck.add_element(slide,ElementKind::Rect)?;
        let element=deck.slide_mut(slide).and_then(|slide|slide.element_mut(object)).ok_or("shape")?;
        element.fill_visible=false;element.stroke_width=2.0;element.text="outline".into();
        let parts=read_zip(&write(&deck,"pptx")?)?.into_iter().filter(|(path,_)|!path.starts_with("customXml/")).collect();
        let imported=read(&write_zip(parts)?,"pptx")?;
        let element=imported.slides[0].elements.iter().find(|element|element.text=="outline").ok_or("imported")?;
        assert!(matches!(element.kind,ElementKind::Rect));assert!(!element.fill_visible);assert_eq!(element.stroke_width,2.0);
        let xml=shape_xml(element,2);assert!(xml.contains("<a:noFill/>"));
        assert!(!imported.page(&imported.slides[0]).items.iter().any(|primitive|
            matches!(primitive,crate::render::Primitive::Rect{x,y,w,h,..}
                if *x==element.x && *y==element.y && *w==element.w && *h==element.h)));
        Ok(())
    }
}


#[cfg(test)]
mod hundred_slide_io_tests {
    use super::*;
    fn parts_without_native(deck:&Deck)->Result<BTreeMap<String,Vec<u8>>,String> {
        Ok(read_zip(&write(deck,"pptx")?)?.into_iter().filter(|(path,_)|!path.starts_with("customXml/")).collect())
    }
    fn roundtrip_external(deck:&Deck)->Result<Deck,String> {
        read(&write_zip(parts_without_native(deck)?.into_iter().collect())?,"pptx")
    }
    fn blank_deck()->Deck {let mut deck=Deck::empty();deck.add_slide("blank");deck}
    fn shape_parts(shape:&str)->Result<BTreeMap<String,Vec<u8>>,String> {
        let mut parts=parts_without_native(&blank_deck())?;
        parts.insert("ppt/slides/slide1.xml".into(),wrap("sld","",&format!("<p:cSld><p:spTree>{GROUP}{shape}</p:spTree></p:cSld>")).into_bytes());
        Ok(parts)
    }
    fn import_parts(parts:BTreeMap<String,Vec<u8>>)->Result<Deck,String> {read(&write_zip(parts.into_iter().collect())?,"pptx")}
    fn placeholder_shape(properties:&str,body:&str)->String {
        format!("<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Text\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\" idx=\"0\"/></p:nvPr></p:nvSpPr><p:spPr>{properties}</p:spPr><p:txBody>{body}</p:txBody></p:sp>")
    }
    fn chart_deck()->Result<Deck,String> {
        let mut deck=blank_deck();let slide=deck.slides[0].id;
        let id=deck.add_element(slide,ElementKind::Chart{values:vec![1.0,2.0],labels:vec!["A".into(),"B".into()]})?;
        let element=deck.slide_mut(slide).and_then(|s|s.element_mut(id)).ok_or("chart")?;
        element.text="Profit & costs".into();element.fill=[11,22,33];Ok(deck)
    }
    fn png()->Result<Vec<u8>,String> {
        let image=image::RgbaImage::from_fn(2,1,|x,_|if x==0 {image::Rgba([255,0,0,255])}else{image::Rgba([0,0,255,255])});
        let mut out=std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image).write_to(&mut out,image::ImageFormat::Png).map_err(|e|e.to_string())?;
        Ok(out.into_inner())
    }
    #[test]
    fn pptx_graphic_frames_export_rotation()->Result<(),String> {
        for kind in [ElementKind::Table{cells:vec![vec!["a".into()]]},ElementKind::Chart{values:vec![1.0],labels:vec![]}] {
            let mut deck=blank_deck();let slide=deck.slides[0].id;let id=deck.add_element(slide,kind)?;
            deck.slide_mut(slide).and_then(|s|s.element_mut(id)).ok_or("element")?.rotation=37.5;
            let imported=roundtrip_external(&deck)?;assert_eq!(imported.slides[0].elements[0].rotation,37.5);
        }Ok(())
    }
    #[test]
    fn empty_pptx_tables_have_one_valid_row()->Result<(),String> {
        let node=parse(table_xml(&Element::default(),2,&[])?.as_bytes())?;
        let table=node.descend("tbl").ok_or("table")?;let row=table.child("tr").ok_or("empty row")?;
        assert_eq!(row.children.iter().filter(|n|n.name=="tc").count(),1);Ok(())
    }
    #[test]
    fn pptx_chart_imports_rich_title()->Result<(),String> {
        let imported=roundtrip_external(&chart_deck()?)?;assert_eq!(imported.slides[0].elements[0].text,"Profit & costs");Ok(())
    }
    #[test]
    fn pptx_chart_imports_series_fill()->Result<(),String> {
        let imported=roundtrip_external(&chart_deck()?)?;assert_eq!(imported.slides[0].elements[0].fill,[11,22,33]);Ok(())
    }
    #[test]
    fn missing_chart_part_is_reported_and_not_a_textbox()->Result<(),String> {
        let mut parts=parts_without_native(&chart_deck()?)?;parts.remove("ppt/charts/chart1.xml");
        let imported=import_parts(parts)?;assert!(imported.slides[0].elements.is_empty());
        assert!(imported.warnings.iter().any(|warning|warning.contains("отсутствует внутренняя часть")));Ok(())
    }
    #[test]
    fn placeholder_default_index_matches_explicit_zero()->Result<(),String> {
        let shape=parse(br#"<sp><nvSpPr><nvPr><ph type="body"/></nvPr></nvSpPr></sp>"#)?;
        let layout=parse(br#"<sldLayout><sp><nvSpPr><nvPr><ph type="title" idx="0"/></nvPr></nvSpPr><spPr><xfrm><off x="7" y="9"/><ext cx="11" cy="13"/></xfrm></spPr></sp></sldLayout>"#)?;
        let transform=inherited_xfrm(&shape,&[&layout]).ok_or("transform")?;
        assert_eq!(transform.child("off").map(|n|n.attr("x")),Some("7"));Ok(())
    }
    #[test]
    fn partial_placeholder_transform_keeps_inherited_extent()->Result<(),String> {
        let shape=placeholder_shape("<a:xfrm><a:off x=\"254000\" y=\"381000\"/></a:xfrm>","<a:p><a:r><a:t>Title</a:t></a:r></a:p>");
        let mut parts=shape_parts(&shape)?;
        parts.insert("ppt/slideLayouts/slideLayout1.xml".into(),wrap("sldLayout","type=\"title\"",&format!("<p:cSld><p:spTree>{}</p:spTree></p:cSld>",placeholder_shape("<a:xfrm><a:off x=\"12700\" y=\"12700\"/><a:ext cx=\"1270000\" cy=\"635000\"/></a:xfrm>",""))).into_bytes());
        let imported=import_parts(parts)?;let e=&imported.slides[0].elements[0];
        assert_eq!((e.x,e.y,e.w,e.h),(20.0,30.0,100.0,50.0));Ok(())
    }
    #[test]
    fn pptx_theme_name_is_preserved()->Result<(),String> {
        let mut deck=blank_deck();deck.theme.name="Custom & theme".into();
        assert_eq!(roundtrip_external(&deck)?.theme.name,deck.theme.name);Ok(())
    }
    #[test]
    fn pptx_layout_metadata_matches_the_linked_layout()->Result<(),String> {
        let mut parts=shape_parts("")?;
        parts.insert("ppt/slideLayouts/slideLayout1.xml".into(),wrap("sldLayout","type=\"twoObj\"","<p:cSld><p:spTree/></p:cSld>").into_bytes());
        assert_eq!(import_parts(parts)?.slides[0].layout,"two_content");Ok(())
    }
    #[test]
    fn leading_empty_paragraph_does_not_restyle_visible_text()->Result<(),String> {
        let body=parse(br#"<txBody><p><r><rPr sz="6600" b="1"/><t/></r></p><p><r><rPr sz="1600" i="1"/><t>real text</t></r></p></txBody>"#)?;
        let mut e=Element::default();apply_text_look(&body,&mut e,&Look::default(),None,18.0,false);
        assert_eq!(e.font_size,16.0);assert!(e.italic);assert!(!e.bold);Ok(())
    }
    #[test]
    fn drawingml_text_uses_its_list_level_defaults()->Result<(),String> {
        let body=parse(br#"<txBody><lstStyle><lvl1pPr><defRPr sz="1000"/></lvl1pPr><lvl3pPr><defRPr sz="3300" b="1"/></lvl3pPr></lstStyle><p><pPr lvl="2"/><r><t>level three</t></r></p></txBody>"#)?;
        let mut e=Element::default();apply_text_look(&body,&mut e,&Look::default(),None,18.0,false);
        assert_eq!(e.font_size,33.0);assert!(e.bold);Ok(())
    }
    #[test]
    fn placeholder_text_inherits_master_and_layout_properties()->Result<(),String> {
        let shape=parse(br#"<sp><nvSpPr><nvPr><ph type="title" idx="0"/></nvPr></nvSpPr></sp>"#)?;
        let body=parse(br#"<txBody><p><r><rPr i="0"/><t>text</t></r></p></txBody>"#)?;
        let master=parse(br#"<sldMaster><txStyles><titleStyle><lvl1pPr><defRPr sz="4000" b="1" i="1"><solidFill><srgbClr val="123456"/></solidFill></defRPr></lvl1pPr></titleStyle></txStyles></sldMaster>"#)?;
        let layout=parse(br#"<sldLayout><sp><nvSpPr><nvPr><ph type="title" idx="0"/></nvPr></nvSpPr><txBody><p><pPr><defRPr sz="3200"/></pPr></p></txBody></sp></sldLayout>"#)?;
        let mut e=Element::default();apply_inherited_text_look(&shape,&body,&[&layout,&master],&mut e,&Look::default(),None,44.0,true);
        assert_eq!(e.font_size,32.0);assert!(e.bold);assert!(!e.italic);assert_eq!(e.color,[18,52,86]);Ok(())
    }
    #[test]
    fn unresolved_theme_face_keeps_the_fallback_font()->Result<(),String> {
        let body=parse(br#"<txBody><p><r><rPr><latin typeface="+unknown"/></rPr><t>Text</t></r></p></txBody>"#)?;
        let look=Look{minor:"Verdana".into(),..Look::default()};let mut e=Element::default();
        apply_text_look(&body,&mut e,&look,None,18.0,false);assert_eq!(e.font,"Verdana");Ok(())
    }
    #[test]
    fn html_slides_include_image_payloads()->Result<(),String> {
        let mut deck=blank_deck();deck.add_element(deck.slides[0].id,ElementKind::Image{bytes:png()?})?;
        let html=String::from_utf8(write(&deck,"html")?).map_err(|e|e.to_string())?;
        assert!(html.contains("<img src=\"data:image/png;base64,iVBOR"));Ok(())
    }
    #[test]
    fn html_slides_keep_supported_text_style() {
        let e=Element{text:"<literal>".into(),font:"A'B\"C".into(),bold:true,italic:true,font_size:19.0,color:[1,2,3],..Element::default()};
        let html=element_html(&e);assert!(html.contains("font-size:19pt"));assert!(html.contains("font-weight:bold"));
        assert!(html.contains("font-style:italic"));assert!(html.contains("color:#010203"));assert!(html.contains("&lt;literal&gt;"));
        assert!(html.contains("&quot;"));assert!(!html.contains("font-family:'A'B"));
    }
    #[test]
    fn markdown_slides_escape_literal_content()->Result<(),String> {
        let mut deck=blank_deck();deck.slides[0].title="# title\nsecond".into();
        let id=deck.add_element(deck.slides[0].id,ElementKind::Text)?;
        deck.slides[0].element_mut(id).ok_or("text")?.text="<tag> *literal*\n[next](url)".into();
        let md=String::from_utf8(write(&deck,"md")?).map_err(|e|e.to_string())?;
        assert!(md.contains("# \\# title second"));assert!(md.contains("\\<tag\\> \\*literal\\*  \n\\[next\\]\\(url\\)"));Ok(())
    }
    #[test]
    fn pptx_sections_survive_without_native_metadata()->Result<(),String> {
        let mut deck=blank_deck();deck.slides[0].section="Intro & overview".into();
        let id=deck.add_slide("blank");deck.slide_mut(id).ok_or("slide")?.section="Intro & overview".into();
        let id=deck.add_slide("blank");deck.slide_mut(id).ok_or("slide")?.section="Details".into();
        let imported=roundtrip_external(&deck)?;
        assert_eq!(imported.slides.iter().map(|s|s.section.as_str()).collect::<Vec<_>>(),vec!["Intro & overview","Intro & overview","Details"]);Ok(())
    }
    #[test]
    fn pptx_short_transitions_keep_their_declared_duration()->Result<(),String> {
        let mut deck=blank_deck();deck.slides[0].transition.effect="fade".into();deck.slides[0].transition.duration=0.004;
        assert!((roundtrip_external(&deck)?.slides[0].transition.duration-0.004).abs()<0.000001);Ok(())
    }
    #[test]
    fn ragged_pptx_table_expansion_is_bounded()->Result<(),String> {
        let mut cells=vec![vec![];1000];cells[0]=vec!["x".into();1000];
        let e=Element{kind:ElementKind::Table{cells:cells.clone()},..Element::default()};
        let mut deck=blank_deck();deck.add_element(deck.slides[0].id,e.kind.clone())?;deck.validate()?;
        assert!(table_xml(&e,2,&cells).is_err());assert!(write(&deck,"pptx").is_err());Ok(())
    }
    #[test]
    fn pptx_picture_flips_transform_the_image_pixels()->Result<(),String> {
        let mut deck=blank_deck();deck.add_element(deck.slides[0].id,ElementKind::Image{bytes:png()?})?;
        let mut parts=parts_without_native(&deck)?;
        let source=String::from_utf8(parts.remove("ppt/slides/slide1.xml").ok_or("slide")?).map_err(|e|e.to_string())?;
        parts.insert("ppt/slides/slide1.xml".into(),source.replace("<a:xfrm rot=\"0\">","<a:xfrm rot=\"0\" flipH=\"1\">").into_bytes());
        let imported=import_parts(parts)?;
        let ElementKind::Image{bytes}=&imported.slides[0].elements[0].kind else{return Err("image".into());};
        let flipped=crate::util::decode_image(bytes)?.to_rgba8();
        assert_eq!(flipped.get_pixel(0,0).0,[0,0,255,255]);assert_eq!(flipped.get_pixel(1,0).0,[255,0,0,255]);Ok(())
    }
    #[test]
    fn uniform_group_scale_updates_text_and_outline()->Result<(),String> {
        let shape="<p:grpSp><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"2540000\" cy=\"2540000\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"1270000\" cy=\"1270000\"/></a:xfrm></p:grpSpPr><p:sp><p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"1270000\" cy=\"1270000\"/></a:xfrm><a:prstGeom prst=\"rect\"/><a:ln w=\"19050\"><a:solidFill><a:srgbClr val=\"000000\"/></a:solidFill></a:ln></p:spPr><p:txBody><a:p><a:r><a:rPr sz=\"1200\"/><a:t>Scaled</a:t></a:r></a:p></p:txBody></p:sp></p:grpSp>";
        let imported=import_parts(shape_parts(shape)?)?;let e=&imported.slides[0].elements[0];
        assert_eq!((e.font_size,e.stroke_width),(24.0,3.0));Ok(())
    }
}
