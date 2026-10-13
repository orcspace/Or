use super::model::{Deck, Element, ElementKind, Slide};
use crate::package::{read_zip, write_zip, xml_escape};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const GROUP: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";

pub fn read(bytes: &[u8], ext: &str) -> Result<Deck, String> {
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
        "txt" | "md" => Ok(deck
            .slides
            .iter()
            .map(|s| {
                format!(
                    "# {}\n{}\n\n{}",
                    s.title,
                    s.elements
                        .iter()
                        .map(|e| e.text.as_str())
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
                        s.elements
                            .iter()
                            .map(|e| format!("<p>{}</p>", xml_escape(&e.text)))
                            .collect::<String>(),
                        xml_escape(&s.notes)
                    )
                })
                .collect::<String>();
            Ok(format!("<!doctype html><html lang=\"ru\"><meta charset=\"utf-8\"><title>TrineDocs Slides</title><style>section{{min-height:80vh;padding:4rem;break-after:page}}aside{{color:#666}}</style><body>{body}</body></html>").into_bytes())
        }
        _ => Err(format!("Экспорт .{ext} ещё не поддерживается")),
    }
}
#[derive(Default, Debug)]
struct Node {
    name: String,
    attrs: BTreeMap<String, String>,
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
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let key = attr.key.as_ref().to_string();
                    let value = quick_xml::escape::unescape(attr.value.as_ref())
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    node.attrs.insert(key.clone(), value.clone());
                    node.attrs.entry(local(&key).into()).or_insert(value);
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
    stack
        .pop()
        .and_then(|n| n.children.into_iter().next())
        .ok_or_else(|| "Пустой XML".into())
}
fn relationships(bytes: Option<&Vec<u8>>, base: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rels = BTreeMap::new();
    if let Some(bytes) = bytes {
        for rel in &parse(bytes)?.children {
            if rel.name != "Relationship" || rel.attr("TargetMode") == "External" {
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
fn color(node: Option<&Node>, fallback: [u8; 3]) -> [u8; 3] {
    let Some(value) = node
        .and_then(|n| n.descend("srgbClr"))
        .map(|n| n.attr("val"))
    else {
        return fallback;
    };
    if value.len() != 6 || !value.is_ascii() {
        return fallback;
    }
    let Ok(number) = u32::from_str_radix(value, 16) else {
        return fallback;
    };
    [(number >> 16) as u8, (number >> 8) as u8, number as u8]
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
    let presentation = parse(
        parts
            .get("ppt/presentation.xml")
            .ok_or("Нет ppt/presentation.xml")?,
    )?;
    let rels = relationships(parts.get("ppt/_rels/presentation.xml.rels"), "ppt")?;
    let mut deck = Deck::empty();
    if let Some(size) = presentation.child("sldSz") {
        deck.width = size.number("cx", 12192000.0) / 12700.0;
        deck.height = size.number("cy", 6858000.0) / 12700.0;
    }
    let Some(slide_list) = presentation.child("sldIdLst") else {
        deck.validate()?;
        return Ok(deck);
    };
    for (index, slide_ref) in slide_list.children.iter().enumerate() {
        let target = rels
            .get(slide_ref.attr("r:id"))
            .ok_or("Не найдена ссылка слайда")?;
        let xml = parse(parts.get(target).ok_or("Нет XML слайда")?)?;
        let id = deck.allocate_id();
        let mut slide = Slide {
            id,
            title: format!("Слайд {}", index + 1),
            hidden: xml.attr("show") == "0",
            ..Slide::default()
        };
        let named_title = xml
            .child("cSld")
            .map(|n| n.attr("name"))
            .filter(|name| !name.is_empty());
        if let Some(title) = named_title {
            slide.title = title.into();
        }
        slide.background = color(xml.descend("bg"), [255, 255, 255]);
        let (base, name) = target.rsplit_once('/').ok_or("Некорректный путь слайда")?;
        let slide_rels = relationships(parts.get(&format!("{base}/_rels/{name}.rels")), base)?;
        if let Some(tree) = xml.descend("spTree") {
            for shape in &tree.children {
                if !matches!(shape.name.as_str(), "sp" | "pic" | "cxnSp" | "graphicFrame") {
                    if shape.name == "grpSp" {
                        deck.warnings
                            .push("Вложенные группы PowerPoint пока не импортируются".into());
                    }
                    continue;
                }
                let mut element = Element {
                    id: deck.allocate_id(),
                    ..Element::default()
                };
                if let Some(transform) = shape.descend("xfrm") {
                    if let Some(offset) = transform.child("off") {
                        element.x = offset.number("x", 0.0) / 12700.0;
                        element.y = offset.number("y", 0.0) / 12700.0;
                    }
                    if let Some(size) = transform.child("ext") {
                        element.w = (size.number("cx", 1270000.0) / 12700.0).max(1.0);
                        element.h = (size.number("cy", 1270000.0) / 12700.0).max(1.0);
                    }
                    element.rotation = transform.number("rot", 0.0) / 60000.0;
                }
                let props = shape.child("spPr");
                element.fill = color(props.and_then(|n| n.child("solidFill")), element.fill);
                element.stroke = color(props.and_then(|n| n.child("ln")), element.stroke);
                if let Some(line) = props.and_then(|n| n.child("ln")) {
                    element.stroke_width = line.number("w", 12700.0) / 12700.0;
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
                    element.kind = ElementKind::Image {
                        bytes: bytes.clone(),
                    };
                } else if let Some(table) = shape.descend("tbl") {
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
                        let mut values = Vec::new();
                        let mut labels = Vec::new();
                        if let Some(val) = chart.descend("val") {
                            let mut points = Vec::new();
                            val.all("v", &mut points);
                            values = points
                                .iter()
                                .filter_map(|p| p.text.parse::<f32>().ok())
                                .collect();
                        }
                        if let Some(cat) = chart.descend("cat") {
                            let mut points = Vec::new();
                            cat.all("v", &mut points);
                            labels = points.iter().map(|p| p.text.clone()).collect();
                        }
                        element.kind = ElementKind::Chart { values, labels };
                    }
                } else {
                    element.kind = match shape
                        .descend("prstGeom")
                        .map(|n| n.attr("prst"))
                        .unwrap_or("")
                    {
                        "ellipse" => ElementKind::Ellipse,
                        "line" => ElementKind::Line,
                        "rightArrow" => ElementKind::Arrow,
                        "rect" | "roundRect" => ElementKind::Rect,
                        _ => ElementKind::Text,
                    };
                    if matches!(element.kind, ElementKind::Line)
                        && props
                            .and_then(|n| n.child("ln"))
                            .and_then(|n| n.child("tailEnd"))
                            .is_some_and(|n| n.attr("type") != "none" && !n.attr("type").is_empty())
                    {
                        element.kind = ElementKind::Arrow;
                    }
                    if !matches!(element.kind, ElementKind::Line | ElementKind::Arrow)
                        && props.and_then(|n| n.child("noFill")).is_some()
                        && shape.child("txBody").is_some()
                    {
                        element.kind = ElementKind::Text;
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
                    if let Some(props) = body.descend("rPr").or_else(|| body.descend("defRPr")) {
                        element.font_size = (props.number("sz", 2800.0) / 100.0).max(1.0);
                        element.bold = props.attr("b") == "1";
                        element.italic = props.attr("i") == "1";
                        element.color = color(Some(props), element.color);
                        if let Some(latin) = props.descend("latin") {
                            let face = latin.attr("typeface").trim();
                            if !face.is_empty() && !face.starts_with('+') && face.len() <= 64 {
                                element.font = face.into();
                            }
                        }
                    }
                    if named_title.is_none()
                        && slide.elements.is_empty()
                        && !element.text.is_empty()
                    {
                        slide.title = element.text.lines().next().unwrap_or("").into();
                    }
                }
                slide.elements.push(element);
            }
        }
        if let Some(transition) = xml.child("transition") {
            slide.transition.effect = transition
                .children
                .first()
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "none".into());
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
            slide.transition.duration = match transition.attr("spd") {
                "slow" => 1.0,
                "fast" => 0.25,
                _ => 0.5,
            };
            slide.transition.advance_after = transition
                .attr("advTm")
                .parse::<f32>()
                .ok()
                .map(|v| v / 1000.0);
        }
        for path in slide_rels.values().filter(|p| p.contains("/notesSlides/")) {
            if let Some(bytes) = parts.get(path) {
                slide.notes = parse(bytes)?.paragraph_text();
            }
        }
        if xml.child("timing").is_some() {
            deck.warnings
                .push("Внешние анимации PowerPoint не воспроизводятся; timing не сохранён".into());
        }
        deck.slides.push(slide);
    }
    deck.warnings.push("Импорт внешнего PPTX: часть стилей, мастеров, групп и эффектов может быть потеряна. Сохраняйте копию.".into());
    deck.validate()?;
    Ok(deck)
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
    let rotation = if prefix == "a" {
        format!(" rot=\"{}\"", (e.rotation * 60000.0) as i64)
    } else {
        String::new()
    };
    format!(
        "<{prefix}:xfrm{rotation}><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></{prefix}:xfrm>",
        emu(e.x),
        emu(e.y),
        emu(e.w),
        emu(e.h)
    )
}
fn text_body(e: &Element) -> String {
    let paragraphs = e.text.split('\n').map(|line| format!("<a:p><a:r><a:rPr lang=\"ru-RU\" sz=\"{}\" b=\"{}\" i=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:latin typeface=\"{}\"/></a:rPr><a:t xml:space=\"preserve\">{}</a:t></a:r><a:endParaRPr lang=\"ru-RU\"/></a:p>", (e.font_size * 100.0) as u32, u8::from(e.bold), u8::from(e.italic), hex(e.color), xml_escape(&e.font), xml_escape(line))).collect::<String>();
    format!(
        "<p:txBody><a:bodyPr wrap=\"square\"><a:spAutoFit/></a:bodyPr><a:lstStyle/>{paragraphs}</p:txBody>"
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
    ) {
        "<a:noFill/>".into()
    } else {
        format!(
            "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            hex(e.fill)
        )
    };
    let line = if matches!(e.kind, ElementKind::Text) {
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
fn table_xml(e: &Element, id: usize, cells: &[Vec<String>]) -> String {
    let cols = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let grid = (0..cols)
        .map(|_| format!("<a:gridCol w=\"{}\"/>", emu(e.w / cols as f32)))
        .collect::<String>();
    let rows = cells.iter().map(|row| { let values = (0..cols).map(|col| format!("<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz=\"{}\"/><a:t>{}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>", (e.font_size * 100.0) as u32, xml_escape(row.get(col).map(String::as_str).unwrap_or("")))).collect::<String>(); format!("<a:tr h=\"{}\">{values}</a:tr>", emu(e.h / cells.len().max(1) as f32)) }).collect::<String>();
    format!(
        "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"Table {id}\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr firstRow=\"1\" bandRow=\"1\"/><a:tblGrid>{grid}</a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>",
        xfrm(e, "p")
    )
}
fn chart_xml(values: &[f32], labels: &[String], fill: [u8; 3]) -> String {
    let cats = values
        .iter()
        .enumerate()
        .map(|(i, _)| {
            format!(
                "<c:pt idx=\"{i}\"><c:v>{}</c:v></c:pt>",
                xml_escape(labels.get(i).map(String::as_str).unwrap_or(""))
            )
        })
        .collect::<String>();
    let vals = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("<c:pt idx=\"{i}\"><c:v>{v}</c:v></c:pt>"))
        .collect::<String>();
    format!(
        "{XML}<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"{A}\"><c:chart><c:plotArea><c:layout/><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:spPr><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></c:spPr><c:cat><c:strLit><c:ptCount val=\"{}\"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val=\"{}\"/>{vals}</c:numLit></c:val></c:ser><c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"b\"/><c:crossAx val=\"2\"/><c:crosses val=\"autoZero\"/><c:auto val=\"1\"/><c:lblAlgn val=\"ctr\"/><c:lblOffset val=\"100\"/></c:catAx><c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:axPos val=\"l\"/><c:numFmt formatCode=\"General\" sourceLinked=\"1\"/><c:crossAx val=\"1\"/><c:crosses val=\"autoZero\"/><c:crossBetween val=\"between\"/></c:valAx></c:plotArea><c:plotVisOnly val=\"1\"/></c:chart></c:chartSpace>",
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
                ElementKind::Table { cells } => shapes.push_str(&table_xml(e, id, cells)),
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
                        chart_xml(values, labels, e.fill).into_bytes(),
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
            format!(
                "<p:transition spd=\"{}\"{advance}>{effect}</p:transition>",
                if slide.transition.duration > 0.75 {
                    "slow"
                } else if slide.transition.duration < 0.3 {
                    "fast"
                } else {
                    "med"
                }
            )
        } else {
            String::new()
        };
        let xml = wrap(
            "sld",
            &format!("show=\"{}\"", u8::from(!slide.hidden)),
            &format!(
                "<p:cSld name=\"{}\"><p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree>{GROUP}{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>{transition}",
                xml_escape(&slide.title),
                hex(slide.background)
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
            "<p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rIdMaster\"/></p:sldMasterIdLst><p:notesMasterIdLst><p:notesMasterId r:id=\"rIdNotesMaster\"/></p:notesMasterIdLst><p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/><p:defaultTextStyle/>",
            emu(deck.width),
            emu(deck.height)
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
    use super::*;
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
}
