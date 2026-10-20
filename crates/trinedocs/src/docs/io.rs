use super::model::*;
use crate::package::{read_zip, write_zip, xml_escape as esc};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

mod odt;

const XML_LIMIT: usize = 32 * 1024 * 1024;
#[derive(Clone, Debug, Default)]
struct Node {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
    text: String,
    supported_choice: bool,
}
impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|n| n.name == name)
    }
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }
    fn value(&self) -> Option<&str> {
        self.attr("val")
    }
    fn descendant(&self, name: &str) -> Option<&Node> {
        if self.name == name {
            return Some(self);
        }
        self.children.iter().find_map(|n| n.descendant(name))
    }
    fn text_content(&self) -> String {
        let mut t = self.text.clone();
        for c in &self.children {
            t.push_str(&c.text_content());
        }
        t
    }
    fn descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Node>) {
        for c in &self.children {
            if c.name == name {
                out.push(c);
            } else {
                c.descendants(name, out);
            }
        }
    }
}
fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or_default().to_string()
}
fn xml(bytes: &[u8]) -> Result<Node, String> {
    if bytes.len() > XML_LIMIT {
        return Err("XML превышает лимит 32 МБ".into());
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);
    let mut stack = vec![Node::default()];
    let mut namespace_stack = vec![BTreeMap::<String, String>::new()];
    let mut count = 0;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("Некорректный XML: {e}"))?;
        let is_empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let mut node = Node {
                    name: local(e.name().as_ref()),
                    ..Node::default()
                };
                let mut namespaces = namespace_stack.last().cloned().unwrap_or_default();
                for attribute in e.attributes() {
                    let a = attribute.map_err(|e| e.to_string())?;
                    let key: &str = a.key.as_ref();
                    let value = a
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    if let Some(prefix) = key.strip_prefix("xmlns:") {
                        namespaces.insert(prefix.to_string(), value.clone());
                    } else if key == "xmlns" {
                        namespaces.insert(String::new(), value.clone());
                    }
                    node.attrs.insert(local(key), value);
                }
                if node.name == "Choice" {
                    node.supported_choice = node.attr("Requires").is_some_and(|requires| {
                        !requires.trim().is_empty()
                            && requires.split_whitespace().all(|prefix| {
                                namespaces
                                    .get(prefix)
                                    .is_some_and(|uri| supported_choice_namespace(uri))
                            })
                    });
                }
                count += 1;
                if count > 1_000_000 || stack.len() > 128 {
                    return Err("Слишком сложный XML".into());
                }
                if is_empty {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                } else {
                    stack.push(node);
                    namespace_stack.push(namespaces);
                }
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err("Несогласованная структура XML".into());
                }
                namespace_stack.pop();
                if let Some(node) = stack.pop()
                    && let Some(parent) = stack.last_mut()
                {
                    parent.children.push(node);
                }
            }
            Event::Text(e) => {
                count += 1;
                if count > 1_000_000 {
                    return Err("Слишком сложный XML".into());
                }
                let value = e.xml10_content();
                let value = quick_xml::escape::unescape(&value).map_err(|e| e.to_string())?;
                if let Some(node) = stack.last_mut() {
                    node.children.push(Node {
                        name: "#text".into(),
                        text: value.into_owned(),
                        ..Node::default()
                    });
                }
            }
            Event::CData(e) => {
                count += 1;
                if count > 1_000_000 {
                    return Err("Слишком сложный XML".into());
                }
                if let Some(node) = stack.last_mut() {
                    node.children.push(Node {
                        name: "#text".into(),
                        text: e.xml10_content().into_owned(),
                        ..Node::default()
                    });
                }
            }
            Event::GeneralRef(e) => {
                count += 1;
                if count > 1_000_000 {
                    return Err("Слишком сложный XML".into());
                }
                let name = e.xml10_content();
                let entity = format!("&{name};");
                let value = quick_xml::escape::unescape(&entity).map_err(|e| e.to_string())?;
                if let Some(node) = stack.last_mut() {
                    node.children.push(Node {
                        name: "#text".into(),
                        text: value.into_owned(),
                        ..Node::default()
                    });
                }
            }
            Event::DocType(_) => return Err("DOCTYPE и внешние сущности запрещены".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("Незавершённый XML".into());
    }
    let root = stack.pop().ok_or("Пустой XML")?;
    let mut elements = root
        .children
        .into_iter()
        .filter(|node| node.name != "#text" || !node.text.trim().is_empty());
    let mut node = elements.next().ok_or("Пустой XML")?;
    if node.name == "#text" || elements.next().is_some() {
        return Err("XML должен содержать ровно один корневой элемент".into());
    }
    normalize_alternate_content(&mut node);
    Ok(node)
}
// Only namespaces whose content is understood by this importer can select a Choice.
fn supported_choice_namespace(uri: &str) -> bool {
    matches!(
        uri,
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            | "http://purl.oclc.org/ooxml/wordprocessingml/main"
            | "http://schemas.openxmlformats.org/drawingml/2006/main"
            | "http://purl.oclc.org/ooxml/drawingml/main"
            | "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            | "http://purl.oclc.org/ooxml/drawingml/wordprocessingDrawing"
            | "http://schemas.openxmlformats.org/drawingml/2006/picture"
            | "http://purl.oclc.org/ooxml/drawingml/picture"
    )
}
fn normalize_alternate_content(node: &mut Node) {
    let mut children = Vec::new();
    for mut child in std::mem::take(&mut node.children) {
        if child.name == "AlternateContent" {
            let selected = child
                .children
                .iter()
                .position(|branch| branch.name == "Choice" && branch.supported_choice)
                .or_else(|| {
                    child
                        .children
                        .iter()
                        .position(|branch| branch.name == "Fallback")
                });
            if let Some(index) = selected {
                let mut branch = child.children.remove(index);
                normalize_alternate_content(&mut branch);
                children.extend(branch.children);
            }
        } else {
            normalize_alternate_content(&mut child);
            children.push(child);
        }
    }
    node.children = children;
}
pub fn read(bytes: &[u8], ext: &str) -> Result<Doc, String> {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    let mut doc = match ext.as_str() {
        "docx" | "dotx" | "docm" => read_docx(bytes)?,
        "txt" => Doc::from_text(&decode_text(bytes)?),
        "md" | "markdown" => read_markdown(&decode_text(bytes)?),
        "html" | "htm" => read_html(&decode_text(bytes)?)?,
        "odt" | "ott" => odt::read_odt(bytes)?,
        "fodt" => odt::read_fodt(&decode_text(bytes)?)?,
        _ => return Err(format!("Неподдерживаемый формат Docs: {ext}")),
    };
    if ext == "docm" {
        doc.losses
            .push("Макросы DOCM не исполняются и не сохраняются.".into());
    }
    doc.losses.sort();
    doc.losses.dedup();
    doc.normalize();
    doc.validate()?;
    Ok(doc)
}
pub fn write(doc: &Doc, ext: &str) -> Result<Vec<u8>, String> {
    doc.validate()?;
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    match ext.as_str() {
        "docx" | "dotx" | "docm" => write_docx(doc, &ext),
        "txt" => Ok(document_plain_export(doc).into_bytes()),
        "md" | "markdown" => {
            if doc.images.iter().any(|image| {
                !matches!(
                    image::guess_format(&image.bytes),
                    Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg)
                )
            }) {
                return Err("Markdown: поддерживаются изображения PNG и JPEG".into());
            }
            let limit = 64 * 1024 * 1024;
            let image_bytes = doc.images.iter().fold(0usize, |total, image| {
                total.saturating_add(image.bytes.len().div_ceil(3).saturating_mul(4))
            });
            if image_bytes > limit {
                return Err("Markdown превышает лимит 64 МиБ".into());
            }
            let markdown = write_markdown(doc);
            if markdown.len() > limit {
                return Err("Markdown превышает лимит 64 МиБ".into());
            }
            Ok(markdown.into_bytes())
        }
        "html" | "htm" => {
            if doc.images.iter().any(|image| {
                !matches!(
                    image::guess_format(&image.bytes),
                    Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg)
                )
            }) {
                return Err("HTML: поддерживаются изображения PNG и JPEG".into());
            }
            let encoded_bytes = doc.images.iter().fold(0usize, |total, image| {
                total.saturating_add(image.bytes.len().div_ceil(3).saturating_mul(4))
            });
            if encoded_bytes > XML_LIMIT {
                return Err("HTML превышает безопасный предел 32 МиБ".into());
            }
            let html = write_html_bounded(doc, XML_LIMIT)?;
            if html.len() > XML_LIMIT {
                return Err("HTML превышает безопасный предел 32 МиБ".into());
            }
            Ok(html.into_bytes())
        }
        "odt" | "ott" => odt::write_odt(doc, ext == "ott"),
        "fodt" => odt::write_fodt(doc),
        _ => Err(format!("Неподдерживаемый формат Docs: {ext}")),
    }
}
fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Текст превышает лимит 64 МБ".into());
    }
    if let Some(decoded) = crate::util::decode_utf32_bom(bytes) {
        return decoded;
    }
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let big = bytes.starts_with(&[0xfe, 0xff]);
        if !(bytes.len() - 2).is_multiple_of(2) {
            return Err("Неполный UTF-16 символ".into());
        }
        let units: Vec<u16> = bytes
            .get(2..)
            .unwrap_or_default()
            .as_chunks::<2>()
            .0
            .iter()
            .filter_map(|p| {
                p.first().zip(p.get(1)).map(|(a, b)| {
                    if big {
                        u16::from_be_bytes([*a, *b])
                    } else {
                        u16::from_le_bytes([*a, *b])
                    }
                })
            })
            .collect();
        return String::from_utf16(&units).map_err(|e| e.to_string());
    }
    if let Some(content) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return std::str::from_utf8(content)
            .map(str::to_owned)
            .map_err(|error| format!("Некорректный UTF-8 после BOM: {error}"));
    }
    let content = bytes;
    if let Ok(text) = std::str::from_utf8(content) {
        return Ok(text.into());
    }
    let (text, _, errors) = encoding_rs::WINDOWS_1251.decode(content);
    // WHATWG maps the byte 0x98, undefined in Microsoft's code page 1251, to a C1 control.
    if errors || content.contains(&0x98) {
        Err("Некорректный Windows-1251".into())
    } else {
        Ok(text.into_owned())
    }
}

fn bool_prop(node: &Node, name: &str) -> Option<bool> {
    node.child(name)
        .map(|n| !matches!(n.value(), Some("0" | "false" | "off")))
}
fn color(value: &str) -> Option<[u8; 3]> {
    if value.len() != 6 || !value.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(value.get(0..2)?, 16).ok()?,
        u8::from_str_radix(value.get(2..4)?, 16).ok()?,
        u8::from_str_radix(value.get(4..6)?, 16).ok()?,
    ])
}
fn hex(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}
fn number(value: Option<&str>, fallback: f32) -> f32 {
    value
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|n| n.is_finite())
        .unwrap_or(fallback)
}
fn apply_rpr(format: &mut TextFormat, node: &Node) {
    if let Some(v) = bool_prop(node, "b") {
        format.bold = v;
    }
    if let Some(v) = bool_prop(node, "i") {
        format.italic = v;
    }
    if let Some(v) = bool_prop(node, "strike").or_else(|| bool_prop(node, "dstrike")) {
        format.strike = v;
    }
    if let Some(n) = node.child("u") {
        format.underline = n.value() != Some("none");
    }
    if let Some(n) = node.child("sz") {
        format.size = number(n.value(), format.size * 2.0) / 2.0;
    }
    if let Some(value) = node.child("color").and_then(Node::value) {
        if value == "auto" {
            format.color = [0, 0, 0];
        } else if let Some(rgb) = color(value) {
            format.color = rgb;
        }
    }
    if let Some(n) = node.child("rFonts")
        && let Some(font) = n
            .attr("ascii")
            .or_else(|| n.attr("hAnsi"))
            .or_else(|| n.attr("eastAsia"))
    {
        format.font = font.into();
    }
    if let Some(n) = node.child("vertAlign") {
        format.script = match n.value() {
            Some("superscript") => 1,
            Some("subscript") => -1,
            _ => 0,
        };
    }
    if let Some(shading) = node.child("shd") {
        format.highlight = word_shading(shading);
    }
    if let Some(n) = node.child("highlight").and_then(Node::value) {
        format.highlight = match n {
            "none" => None,
            "yellow" => Some([255, 255, 0]),
            "green" => Some([0, 255, 0]),
            "cyan" => Some([0, 255, 255]),
            "magenta" => Some([255, 0, 255]),
            "blue" => Some([0, 0, 255]),
            "red" => Some([255, 0, 0]),
            "black" => Some([0, 0, 0]),
            "white" => Some([255, 255, 255]),
            "darkBlue" => Some([0, 0, 128]),
            "darkCyan" => Some([0, 128, 128]),
            "darkGreen" => Some([0, 128, 0]),
            "darkMagenta" => Some([128, 0, 128]),
            "darkRed" => Some([128, 0, 0]),
            "darkYellow" => Some([128, 128, 0]),
            "darkGray" => Some([128, 128, 128]),
            "lightGray" => Some([192, 192, 192]),
            _ => Some([230, 230, 230]),
        };
    }
}
fn apply_style_rpr(format: &mut TextFormat, node: &Node) {
    let previous = (format.bold, format.italic, format.strike);
    apply_rpr(format, node);
    for (tag, old, value) in [
        ("b", previous.0, &mut format.bold),
        ("i", previous.1, &mut format.italic),
        ("strike", previous.2, &mut format.strike),
    ] {
        *value = match bool_prop(node, tag).or_else(|| {
            if tag == "strike" {
                bool_prop(node, "dstrike")
            } else {
                None
            }
        }) {
            Some(true) => !old,
            Some(false) | None => old,
        };
    }
}
fn apply_ppr(p: &mut Paragraph, node: &Node, num_formats: &BTreeMap<String, List>) {
    if let Some(style) = node.child("pStyle").and_then(Node::value) {
        p.style = style.into();
    }
    if let Some(align) = node.child("jc").and_then(Node::value) {
        p.align = match align {
            "center" => Align::Center,
            "right" | "end" => Align::Right,
            "both" | "distribute" => Align::Justify,
            _ => Align::Left,
        };
    }
    if let Some(n) = node.child("numPr") {
        if let Some(level) = n.child("ilvl").and_then(Node::value) {
            p.list_level = number(Some(level), f32::from(p.list_level)).clamp(0.0, 8.0) as u8;
        }
        p.list = match n.child("numId").and_then(Node::value) {
            Some("0") => {
                p.list_level = 0;
                List::None
            }
            Some(id) => num_formats
                .get(&format!("{id}:{}", p.list_level))
                .or_else(|| num_formats.get(id))
                .copied()
                .unwrap_or(List::Bullet),
            None => p.list,
        };
    }
    if let Some(n) = node.child("spacing") {
        apply_word_spacing(p, n);
    }
    // Attributes missing from `w:ind` keep the value inherited from the style.
    if let Some(n) = node.child("ind") {
        p.indent = number(n.attr("left").or_else(|| n.attr("start")), p.indent * 20.0) / 20.0;
        p.right_indent = number(
            n.attr("right").or_else(|| n.attr("end")),
            p.right_indent * 20.0,
        ) / 20.0;
        if n.attr("hanging").is_some() {
            p.first_indent = -number(n.attr("hanging"), 0.0) / 20.0;
        } else if n.attr("firstLine").is_some() {
            p.first_indent = number(n.attr("firstLine"), 0.0) / 20.0;
        }
    }
    if let Some(v) = bool_prop(node, "pageBreakBefore") {
        p.page_break_before = v;
    }
    if let Some(v) = bool_prop(node, "keepNext") {
        p.keep_next = v;
    }
    if let Some(v) = bool_prop(node, "keepLines") {
        p.keep_lines = v;
    }
    if let Some(v) = bool_prop(node, "widowControl") {
        p.widow_control = v;
    }
    if let Some(v) = bool_prop(node, "contextualSpacing") {
        p.contextual_spacing = v;
    }
}
fn apply_word_spacing(p: &mut Paragraph, n: &Node) {
    p.spacing_before = number(n.attr("before"), p.spacing_before * 20.0) / 20.0;
    p.spacing_after = number(n.attr("after"), p.spacing_after * 20.0) / 20.0;
    // lineRule defaults to "auto" (a multiple of single spacing) when absent.
    if let Some(line) = n.attr("line") {
        match n.attr("lineRule").unwrap_or("auto") {
            "exact" | "atLeast" => {
                p.line_points = (number(Some(line), 240.0) / 20.0).clamp(0.0, 1584.0);
                p.line_exact = n.attr("lineRule") == Some("exact");
            }
            _ => {
                p.line_spacing = (number(Some(line), 240.0) / 240.0).clamp(0.5, 5.0);
                p.line_points = 0.0;
            }
        }
    }
}
fn merge_word_spacing(spacing: &mut Node, ppr: &Node) {
    if let Some(node) = ppr.child("spacing") {
        spacing.attrs.extend(node.attrs.clone());
    }
}
/// Body-level blocks with content controls (`w:sdt`) and custom XML unwrapped: their
/// paragraphs and tables are ordinary document content.
fn body_blocks<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>, depth: usize) {
    for node in nodes {
        match node.name.as_str() {
            "sdt" if depth < 128 => {
                if let Some(content) = node.child("sdtContent") {
                    body_blocks(&content.children, out, depth + 1);
                }
            }
            "customXml" | "ins" | "moveTo" if depth < 128 => {
                body_blocks(&node.children, out, depth + 1)
            }
            "del" | "moveFrom" => {}
            _ => out.push(node),
        }
    }
}
fn style_chain<'a>(
    styles: &'a BTreeMap<String, Node>,
    id: &str,
    out: &mut Vec<&'a Node>,
) -> Result<(), String> {
    let mut current = Some(id);
    let mut chain = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id.to_string()) {
            return Err("Циклическое наследование стилей DOCX".into());
        }
        let Some(style) = styles.get(id) else { break };
        if chain.len() >= 4096 {
            return Err("Цепочка стилей DOCX превышает безопасный лимит".into());
        }
        chain.push(style);
        current = style.child("basedOn").and_then(Node::value);
    }
    out.extend(chain.into_iter().rev());
    Ok(())
}
/// Document-wide defaults of a DOCX: run format and paragraph layout (`w:docDefaults`).
struct Defaults<'a> {
    format: &'a TextFormat,
    hidden: bool,
    paragraph: &'a Paragraph,
    spacing: &'a Node,
    number_starts: &'a BTreeMap<String, u32>,
    number_counters: &'a std::cell::RefCell<BTreeMap<String, [Option<u32>; 9]>>,
    /// `styleId` of the default paragraph style (`a` in Russian Word, `Normal` in English).
    normal: &'a str,
}
/// Our style id for an imported style, matched by its language-neutral `w:name`.
fn canonical_style(styles: &BTreeMap<String, Node>, id: &str) -> String {
    let name = styles
        .get(id)
        .and_then(|s| s.child("name"))
        .and_then(Node::value)
        .unwrap_or(id)
        .to_ascii_lowercase();
    match name.as_str() {
        "normal" => "Normal".into(),
        "title" => "Title".into(),
        "heading 1" => "Heading1".into(),
        "heading 2" => "Heading2".into(),
        "heading 3" => "Heading3".into(),
        "heading 4" => "Heading4".into(),
        "heading 5" => "Heading5".into(),
        "heading 6" => "Heading6".into(),
        "quote" => "Quote".into(),
        _ => id.into(),
    }
}
#[allow(clippy::too_many_arguments)]
fn paragraph(
    node: &Node,
    doc: &mut Doc,
    styles: &BTreeMap<String, Node>,
    defaults: &Defaults,
    table_style: &[&Node],
    nums: &BTreeMap<String, List>,
    rels: &BTreeMap<String, String>,
) -> Result<Paragraph, String> {
    let id = doc.alloc_id();
    let default_format = defaults.format;
    let mut p = Paragraph {
        id,
        runs: Vec::new(),
        ..defaults.paragraph.clone()
    };
    let mut format = default_format.clone();
    let mut hidden = defaults.hidden;
    let mut spacing = defaults.spacing.clone();
    let style = node
        .child("pPr")
        .and_then(|n| n.child("pStyle"))
        .and_then(Node::value)
        .unwrap_or(defaults.normal);
    // Word layers a table style between the document defaults and the paragraph style,
    // which is why "Table Grid" cells have no spacing after paragraphs.
    let mut chain = table_style.to_vec();
    let effective = if styles.contains_key(style) {
        style
    } else {
        defaults.normal
    };
    style_chain(styles, effective, &mut chain)?;
    for n in &chain {
        if let Some(rpr) = n.child("rPr") {
            if bool_prop(rpr, "vanish") == Some(true) {
                hidden = !hidden;
            }
            apply_style_rpr(&mut format, rpr);
        }
        if let Some(ppr) = n.child("pPr") {
            merge_word_spacing(&mut spacing, ppr);
            apply_ppr(&mut p, ppr, nums);
        }
    }
    if let Some(n) = node.child("pPr") {
        merge_word_spacing(&mut spacing, n);
        apply_ppr(&mut p, n, nums);
    }
    apply_word_spacing(&mut p, &spacing);
    p.style = canonical_style(styles, style);
    let outline = chain
        .iter()
        .filter_map(|style| style.child("pPr"))
        .chain(node.child("pPr"))
        .filter_map(|properties| properties.child("outlineLvl").and_then(Node::value))
        .last()
        .and_then(|value| value.parse::<u8>().ok());
    // A custom style that is a heading only through its outline level is demoted with it.
    let style_heading = chain
        .iter()
        .filter_map(|style| style.child("pPr"))
        .filter_map(|properties| properties.child("outlineLvl").and_then(Node::value))
        .any(|value| value.parse::<u8>().is_ok_and(|level| level <= 5));
    match outline {
        Some(level @ 0..=5) => p.style = format!("Heading{}", level + 1),
        Some(9) if style_heading || heading_level(&p.style).is_some() => {
            p.style = "Normal".into()
        }
        Some(6..=8) => doc
            .losses
            .push("Уровни структуры DOCX 7–9 пока не отображаются в оглавлении.".into()),
        _ => {}
    }
    if p.list == List::Number {
        let num_id = chain
            .iter()
            .filter_map(|style| style.child("pPr"))
            .chain(node.child("pPr"))
            .filter_map(|properties| {
                properties
                    .child("numPr")
                    .and_then(|node| node.child("numId"))
                    .and_then(Node::value)
            })
            .last();
        if let Some(id) = num_id {
            let level = usize::from(p.list_level.min(8));
            let first = defaults
                .number_starts
                .get(&format!("{id}:{level}"))
                .copied()
                .unwrap_or(1);
            let mut counters = defaults.number_counters.borrow_mut();
            let levels = counters.entry(id.into()).or_insert([None; 9]);
            let value = levels[level]
                .map(|value| value.saturating_add(1).min(999_999_999))
                .unwrap_or(first);
            p.list_start = Some(value);
            levels[level] = Some(value);
            levels[level + 1..].fill(None);
        }
    }
    let mut nodes = Vec::new();
    collect_runs(node, rels, None, &mut nodes);
    for (run, link) in nodes {
        let mut fmt = format.clone();
        let mut run_hidden = hidden;
        fmt.link = link;
        if let Some(n) = run.child("rPr") {
            if let Some(id) = n.child("rStyle").and_then(Node::value) {
                let mut character_styles = Vec::new();
                style_chain(styles, id, &mut character_styles)?;
                for style in character_styles {
                    if let Some(rpr) = style.child("rPr") {
                        if bool_prop(rpr, "vanish") == Some(true) {
                            run_hidden = !run_hidden;
                        }
                        apply_style_rpr(&mut fmt, rpr);
                    }
                }
            }
            apply_rpr(&mut fmt, n);
            if let Some(value) = bool_prop(n, "vanish") {
                run_hidden = value;
            }
        }
        if run_hidden {
            doc.losses
                .push("Скрытый текст DOCX не отображается и не сохраняется.".into());
            continue;
        }
        word_run_content(&run.children, &fmt, doc, &mut p.runs);
    }
    if p.runs.is_empty() {
        // An empty paragraph keeps the size of its paragraph mark, so blank lines keep their height.
        let mut mark = format.clone();
        if let Some(rpr) = node.child("pPr").and_then(|n| n.child("rPr")) {
            apply_rpr(&mut mark, rpr);
        }
        p.runs.push(Run {
            text: String::new(),
            format: mark,
        });
    }
    p.normalize();
    let format = p
        .runs
        .iter()
        .map(|run| &run.format)
        .max_by(|a, b| a.size.total_cmp(&b.size))
        .cloned()
        .unwrap_or(format);
    let height = paragraph_line_height(&p, &format, format.size);
    if let Some(lines) = spacing.attr("beforeLines") {
        p.spacing_before = (number(Some(lines), 0.0) / 100.0 * height).clamp(0.0, 1000.0);
    }
    if let Some(lines) = spacing.attr("afterLines") {
        p.spacing_after = (number(Some(lines), 0.0) / 100.0 * height).clamp(0.0, 1000.0);
    }
    Ok(p)
}

fn word_run_content(nodes: &[Node], format: &TextFormat, doc: &mut Doc, out: &mut Vec<Run>) {
    for node in nodes {
        let text = match node.name.as_str() {
            "t" => Some(node.text_content()),
            "tab" => Some("\t".into()),
            "noBreakHyphen" => Some("\u{2011}".into()),
            "softHyphen" => Some("\u{00ad}".into()),
            "br" | "cr" => {
                if node.attr("type") == Some("column") {
                    doc.losses.push("Разрыв колонки DOCX представлен разрывом страницы в одноколоночном макете.".into());
                }
                Some(
                    if matches!(node.attr("type"), Some("page" | "column")) {
                        "\u{000c}"
                    } else {
                        "\u{000b}"
                    }
                    .into(),
                )
            }
            "sym" => {
                if let Some(ch) = node
                    .attr("char")
                    .and_then(|value| u32::from_str_radix(value, 16).ok())
                    .and_then(char::from_u32)
                    .filter(|ch| *ch >= ' ')
                {
                    let mut symbol = format.clone();
                    if let Some(font) = node.attr("font").filter(|font| !font.is_empty()) {
                        symbol.font = font.into();
                    }
                    out.push(Run {
                        text: ch.to_string(),
                        format: symbol,
                    });
                } else {
                    doc.losses
                        .push("Некорректный символ w:sym DOCX пропущен.".into());
                }
                None
            }
            "ruby" => {
                if let Some(base) = node.child("rubyBase") {
                    word_run_content(&base.children, format, doc, out);
                }
                doc.losses
                    .push("Фонетические подсказки DOCX сведены к базовому тексту.".into());
                None
            }
            "r" => {
                let mut nested = format.clone();
                if let Some(properties) = node.child("rPr") {
                    if bool_prop(properties, "vanish") == Some(true) {
                        continue;
                    }
                    apply_rpr(&mut nested, properties);
                }
                word_run_content(&node.children, &nested, doc, out);
                None
            }
            _ => None,
        };
        if let Some(text) = text.filter(|text| !text.is_empty()) {
            out.push(Run {
                text,
                format: format.clone(),
            });
        }
    }
}

/// `Some(true)` when any edge of a `tblBorders`/`tcBorders` element is drawn.
fn borders_visible(node: Option<&Node>) -> Option<bool> {
    let node = node?;
    Some(node.children.iter().any(|edge| {
        !matches!(edge.value(), Some("nil" | "none") | None)
            && edge.attr("sz").is_none_or(|s| s != "0")
    }))
}
fn border_color(node: Option<&Node>) -> Option<[u8; 3]> {
    node?.children.iter().find_map(|edge| {
        if matches!(edge.value(), Some("nil" | "none") | None) {
            return None;
        }
        Some(edge.attr("color").and_then(color).unwrap_or([0, 0, 0]))
    })
}
fn visible_drawings<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    for child in &node.children {
        match child.name.as_str() {
            "del" | "moveFrom" => {}
            "drawing" => out.push(child),
            _ => visible_drawings(child, out),
        }
    }
}

fn word_cropped_image(
    bytes: &[u8],
    drawing: &Node,
    losses: &mut Vec<String>,
) -> Result<Vec<u8>, String> {
    let Some(crop) = drawing.descendant("srcRect") else {
        return Ok(bytes.to_vec());
    };
    let mut sides = [0.0f64; 4];
    for (side, name) in sides.iter_mut().zip(["l", "t", "r", "b"]) {
        let Some(raw) = crop.attr(name) else {
            continue;
        };
        *side = if let Some(percent) = raw.strip_suffix('%') {
            percent.parse::<f64>().ok().map(|value| value / 100.0)
        } else {
            raw.parse::<f64>().ok().map(|value| value / 100000.0)
        }
        .filter(|value| value.is_finite())
        .ok_or("Некорректная обрезка рисунка DOCX")?;
    }
    if sides.iter().all(|value| *value == 0.0) {
        return Ok(bytes.to_vec());
    }
    if sides.iter().any(|value| *value < 0.0) {
        losses.push(
            "Внешние отступы обрезки рисунка DOCX не поддерживаются; оставлен исходный рисунок."
                .into(),
        );
        return Ok(bytes.to_vec());
    }
    if sides[0] + sides[2] >= 1.0 || sides[1] + sides[3] >= 1.0 {
        return Err("Обрезка DOCX не оставляет видимой области рисунка".into());
    }
    let image =
        crate::util::decode_image(bytes).map_err(|error| format!("Обрезка DOCX: {error}"))?;
    let (width, height) = (image.width(), image.height());
    let x = (f64::from(width) * sides[0])
        .round()
        .clamp(0.0, f64::from(width.saturating_sub(1))) as u32;
    let y = (f64::from(height) * sides[1])
        .round()
        .clamp(0.0, f64::from(height.saturating_sub(1))) as u32;
    let right = (f64::from(width) * (1.0 - sides[2]))
        .round()
        .clamp(f64::from(x + 1), f64::from(width)) as u32;
    let bottom = (f64::from(height) * (1.0 - sides[3]))
        .round()
        .clamp(f64::from(y + 1), f64::from(height)) as u32;
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .crop_imm(x, y, right - x, bottom - y)
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    let bytes = png.into_inner();
    if bytes.len() > XML_LIMIT {
        return Err("Обрезанный рисунок DOCX превышает 32 МБ".into());
    }
    Ok(bytes)
}

fn collect_runs<'a>(
    node: &'a Node,
    rels: &BTreeMap<String, String>,
    link: Option<String>,
    out: &mut Vec<(&'a Node, Option<String>)>,
) {
    for child in &node.children {
        // Deleted and moved-away revisions are not part of the visible text.
        if child.name == "del" || child.name == "moveFrom" {
            continue;
        }
        if child.name == "r" {
            out.push((child, link.clone()));
        } else if child.name == "hyperlink" {
            let target = child
                .attr("id")
                .and_then(|id| rels.get(id))
                .cloned()
                .or_else(|| child.attr("anchor").map(|s| format!("#{s}")));
            collect_runs(child, rels, target, out);
        } else if child.name != "pPr" {
            collect_runs(child, rels, link.clone(), out);
        }
    }
}
fn relationships(node: Option<Node>) -> BTreeMap<String, String> {
    node.map(|n| {
        n.children
            .into_iter()
            .filter_map(|r| Some((r.attr("Id")?.to_string(), r.attr("Target")?.to_string())))
            .collect()
    })
    .unwrap_or_default()
}
fn resolve_part(target: &str) -> Option<String> {
    if target.contains(':') || target.contains('\\') || target.starts_with("//") {
        return None;
    }
    let raw = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else {
        format!("word/{target}")
    };
    let mut components = Vec::new();
    for c in raw.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            _ => components.push(c),
        }
    }
    Some(components.join("/").to_ascii_lowercase())
}
fn read_docx(bytes: &[u8]) -> Result<Doc, String> {
    let mut parts = normalize_docx_part_names(read_zip(bytes)?)?;
    normalize_docx_parts(&mut parts)?;
    let theme = parts
        .get("word/theme/theme1.xml")
        .map(|bytes| xml(bytes))
        .transpose()?
        .map(|node| WordTheme::read(&node))
        .unwrap_or_default();
    let mut root = xml(parts
        .get("word/document.xml")
        .ok_or("В DOCX отсутствует word/document.xml")?)?;
    theme.apply(&mut root);
    let body = root
        .child("body")
        .ok_or("В DOCX отсутствует тело документа")?;
    let rels = relationships(
        parts
            .get("word/_rels/document.xml.rels")
            .map(|b| xml(b))
            .transpose()?,
    );
    let mut styles: BTreeMap<String, Node> = BTreeMap::new();
    // Word's built-in defaults: automatic (black) text, 10 pt, single spacing, no gaps.
    let mut default_format = TextFormat {
        color: [0, 0, 0],
        size: 10.0,
        ..TextFormat::default()
    };
    let mut default_hidden = false;
    let mut default_spacing = Node::default();
    let mut default_paragraph = Paragraph {
        spacing_before: 0.0,
        spacing_after: 0.0,
        line_spacing: 1.0,
        ..Paragraph::default()
    };
    if let Some(bytes) = parts.get("word/styles.xml") {
        let mut node = xml(bytes)?;
        theme.apply(&mut node);
        if let Some(n) = node.descendant("rPrDefault").and_then(|n| n.child("rPr")) {
            apply_rpr(&mut default_format, n);
            default_hidden = bool_prop(n, "vanish").unwrap_or(false);
        }
        if let Some(n) = node.descendant("pPrDefault").and_then(|n| n.child("pPr")) {
            merge_word_spacing(&mut default_spacing, n);
            apply_ppr(&mut default_paragraph, n, &BTreeMap::new());
        }
        for n in &node.children {
            if n.name == "style"
                && let Some(id) = n.attr("styleId")
            {
                styles.insert(id.into(), n.clone());
            }
        }
    }
    let mut nums = BTreeMap::new();
    let mut number_starts = BTreeMap::new();
    let number_counters = std::cell::RefCell::new(BTreeMap::new());
    if let Some(bytes) = parts.get("word/numbering.xml") {
        let node = xml(bytes)?;
        let mut abstracts = BTreeMap::new();
        let mut abstract_starts = BTreeMap::new();
        for n in &node.children {
            if n.name == "abstractNum"
                && let Some(id) = n.attr("abstractNumId")
            {
                let levels = n
                    .children
                    .iter()
                    .filter(|level| level.name == "lvl")
                    .filter_map(|level| {
                        let index = level.attr("ilvl")?.parse::<u8>().ok()?;
                        let kind = word_number_kind(level.child("numFmt").and_then(Node::value));
                        Some((index, kind))
                    })
                    .collect::<BTreeMap<_, _>>();
                let starts = n
                    .children
                    .iter()
                    .filter(|node| node.name == "lvl")
                    .filter_map(|level| {
                        Some((
                            level.attr("ilvl")?.parse::<u8>().ok()?,
                            level
                                .child("start")
                                .and_then(Node::value)
                                .and_then(|value| value.parse::<u32>().ok())
                                .unwrap_or(1)
                                .min(999_999_999),
                        ))
                    })
                    .collect::<BTreeMap<_, _>>();
                abstract_starts.insert(id.to_string(), starts);
                abstracts.insert(id.to_string(), levels);
            }
        }
        for n in &node.children {
            if n.name == "num"
                && let Some(id) = n.attr("numId")
            {
                let mut levels = n
                    .child("abstractNumId")
                    .and_then(Node::value)
                    .and_then(|id| abstracts.get(id))
                    .cloned()
                    .unwrap_or_default();
                let mut starts = n
                    .child("abstractNumId")
                    .and_then(Node::value)
                    .and_then(|id| abstract_starts.get(id))
                    .cloned()
                    .unwrap_or_default();
                for override_node in n.children.iter().filter(|n| n.name == "lvlOverride") {
                    if let Some(level) = override_node
                        .attr("ilvl")
                        .and_then(|value| value.parse::<u8>().ok())
                        && let Some(start) = override_node
                            .child("startOverride")
                            .or_else(|| {
                                override_node
                                    .child("lvl")
                                    .and_then(|node| node.child("start"))
                            })
                            .and_then(Node::value)
                            .and_then(|value| value.parse::<u32>().ok())
                            .filter(|value| *value <= 999_999_999)
                    {
                        starts.insert(level, start);
                    }
                    if let Some(level) = override_node.child("lvl")
                        && let Some(index) = override_node
                            .attr("ilvl")
                            .and_then(|v| v.parse::<u8>().ok())
                        && let Some(format) = level.child("numFmt").and_then(Node::value)
                    {
                        levels.insert(index, word_number_kind(Some(format)));
                    }
                }
                for (level, start) in starts {
                    number_starts.insert(format!("{id}:{level}"), start);
                }
                nums.insert(
                    id.to_string(),
                    levels.get(&0).copied().unwrap_or(List::Bullet),
                );
                for (level, kind) in levels {
                    nums.insert(format!("{id}:{level}"), kind);
                }
            }
        }
    }
    let mut doc = Doc::default();
    doc.paragraphs.clear();
    doc.page.page_numbers = false;
    default_paragraph.style = "Normal".into();
    let normal = styles
        .iter()
        .find(|(_, s)| {
            s.attr("type") == Some("paragraph") && matches!(s.attr("default"), Some("1" | "true"))
        })
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| "Normal".into());
    let defaults = Defaults {
        format: &default_format,
        hidden: default_hidden,
        paragraph: &default_paragraph,
        spacing: &default_spacing,
        number_starts: &number_starts,
        number_counters: &number_counters,
        normal: &normal,
    };
    let mut blocks = Vec::new();
    body_blocks(&body.children, &mut blocks, 0);
    for node in blocks {
        match node.name.as_str() {
            "p" => {
                let p = paragraph(node, &mut doc, &styles, &defaults, &[], &nums, &rels)?;
                let mut drawings = Vec::new();
                visible_drawings(node, &mut drawings);
                // Our spacer between adjacent tables is not part of the document.
                if p.style == TABLE_GAP && p.is_empty() && drawings.is_empty() {
                    continue;
                }
                let image_block = p.style == "TrineDocsImageBlock"
                    && p.is_empty()
                    && !drawings.is_empty()
                    && !doc.paragraphs.is_empty();
                let id = if image_block {
                    doc.paragraphs.last().map(|p| p.id).unwrap_or(p.id)
                } else {
                    let id = p.id;
                    doc.paragraphs.push(p);
                    id
                };
                for drawing in drawings {
                    let embed = drawing.descendant("blip").and_then(|n| n.attr("embed"));
                    let source = embed
                        .and_then(|id| rels.get(id))
                        .and_then(|s| resolve_part(s))
                        .and_then(|key| parts.get(&key));
                    if let Some(bytes) = source {
                        let extent = drawing.descendant("extent");
                        let width =
                            number(extent.and_then(|n| n.attr("cx")), 2_540_000.0) / 12_700.0;
                        let height =
                            number(extent.and_then(|n| n.attr("cy")), 1_905_000.0) / 12_700.0;
                        let alt = drawing
                            .descendant("docPr")
                            .and_then(|n| {
                                n.attr("descr")
                                    .filter(|value| !value.is_empty())
                                    .or_else(|| n.attr("title"))
                            })
                            .unwrap_or_default()
                            .to_string();
                        let image_id = doc.alloc_id();
                        doc.images.push(AnchoredImage {
                            id: image_id,
                            after_paragraph_id: id,
                            bytes: word_cropped_image(bytes, drawing, &mut doc.losses)?,
                            width,
                            height,
                            alt,
                        });
                    } else {
                        doc.losses
                            .push("Рисунок без доступного встроенного файла пропущен.".into());
                    }
                }
            }
            "tbl" => {
                if doc.paragraphs.is_empty() {
                    let id = doc.alloc_id();
                    doc.paragraphs.push(Paragraph {
                        id,
                        ..Paragraph::default()
                    });
                }
                let after = doc.paragraphs.last().map(|p| p.id).unwrap_or_default();
                let id = doc.alloc_id();
                let tbl_pr = node.child("tblPr");
                let mut chain = Vec::new();
                let table_style = tbl_pr
                    .and_then(|node| node.child("tblStyle"))
                    .and_then(Node::value)
                    .or_else(|| {
                        styles
                            .iter()
                            .find(|(_, node)| {
                                node.attr("type") == Some("table")
                                    && matches!(node.attr("default"), Some("1" | "true"))
                            })
                            .map(|(id, _)| id.as_str())
                    });
                if let Some(style) = table_style {
                    style_chain(&styles, style, &mut chain)?;
                }
                let mut rows = Vec::new();
                let mut row_heights = Vec::new();
                let mut row_blocks = Vec::new();
                body_blocks(&node.children, &mut row_blocks, 0);
                let row_nodes = row_blocks
                    .into_iter()
                    .filter(|n| n.name == "tr")
                    .collect::<Vec<_>>();
                // Leading rows marked as headers repeat on every page.
                let header_rows = row_nodes
                    .iter()
                    .copied()
                    .take_while(|tr| {
                        tr.child("trPr")
                            .and_then(|n| bool_prop(n, "tblHeader"))
                            .unwrap_or(false)
                    })
                    .count();
                if row_nodes.len() > 10000 {
                    return Err("Таблица DOCX превышает 10 000 строк".into());
                }
                let mut cell_nodes_by_row = Vec::new();
                for tr in &row_nodes {
                    row_heights.push(
                        tr.child("trPr")
                            .and_then(|n| n.child("trHeight"))
                            .filter(|n| n.attr("hRule") != Some("auto"))
                            .map(|n| (number(n.attr("val"), 0.0) / 20.0).clamp(0.0, 1500.0))
                            .unwrap_or_default(),
                    );
                    let mut cells = Vec::new();
                    let grid_before = word_grid_before(tr)?;
                    if grid_before > 0 {
                        let paragraph = Paragraph {
                            id: doc.alloc_id(),
                            ..Default::default()
                        };
                        let mut cell = Cell::new(doc.alloc_id(), vec![paragraph], [255, 255, 255]);
                        cell.span = grid_before as u16;
                        cells.push(cell);
                    }
                    let mut cell_blocks = Vec::new();
                    body_blocks(&tr.children, &mut cell_blocks, 0);
                    let cell_nodes = cell_blocks
                        .into_iter()
                        .filter(|n| n.name == "tc")
                        .collect::<Vec<_>>();
                    if cell_nodes.len() > 256 {
                        return Err("Таблица DOCX превышает 256 столбцов".into());
                    }
                    for tc in &cell_nodes {
                        if tc.descendant("drawing").is_some() {
                            doc.losses
                                .push("Рисунки внутри ячеек таблицы не импортированы.".into());
                        }
                        if tc.child("tbl").is_some() {
                            doc.losses
                                .push("Вложенные таблицы не импортированы.".into());
                        }
                        let cell_id = doc.alloc_id();
                        let mut paragraphs = Vec::new();
                        let mut blocks = Vec::new();
                        body_blocks(&tc.children, &mut blocks, 0);
                        for p in blocks.into_iter().filter(|n| n.name == "p") {
                            paragraphs.push(paragraph(
                                p, &mut doc, &styles, &defaults, &chain, &nums, &rels,
                            )?);
                        }
                        if paragraphs.is_empty() {
                            let id = doc.alloc_id();
                            paragraphs.push(Paragraph {
                                id,
                                ..Paragraph::default()
                            });
                        }
                        let tc_pr = tc.child("tcPr");
                        let mut fill = [255, 255, 255];
                        for properties in chain
                            .iter()
                            .filter_map(|style| style.child("tcPr"))
                            .chain(tc_pr)
                        {
                            if let Some(shading) = properties.child("shd") {
                                fill = word_shading(shading).unwrap_or([255, 255, 255]);
                            }
                        }
                        let mut cell = Cell::new(cell_id, paragraphs, fill);
                        cell.span = number(
                            tc_pr
                                .and_then(|n| n.child("gridSpan"))
                                .and_then(Node::value),
                            1.0,
                        )
                        .clamp(1.0, 256.0) as u16;
                        cell.merged = tc_pr
                            .and_then(|n| n.child("vMerge"))
                            .is_some_and(|n| n.value() != Some("restart"));
                        for properties in chain
                            .iter()
                            .filter_map(|style| style.child("tcPr"))
                            .chain(tc_pr)
                        {
                            if let Some(alignment) =
                                properties.child("vAlign").and_then(Node::value)
                            {
                                cell.v_align = match alignment {
                                    "center" => VAlign::Center,
                                    "bottom" => VAlign::Bottom,
                                    _ => VAlign::Top,
                                };
                            }
                        }
                        cells.push(cell);
                    }
                    rows.push(cells);
                    cell_nodes_by_row.push(cell_nodes);
                }
                let mut widths: Vec<f32> = node
                    .child("tblGrid")
                    .map(|g| {
                        g.children
                            .iter()
                            .filter(|n| n.name == "gridCol")
                            .map(|n| (number(n.attr("w"), 1440.0) / 20.0).clamp(0.05, 10000.0))
                            .collect()
                    })
                    .unwrap_or_default();
                if widths.is_empty() {
                    let columns = rows
                        .iter()
                        .map(|row| row.iter().map(Cell::span).sum::<usize>())
                        .max()
                        .unwrap_or(0);
                    let mut inferred = vec![None; columns];
                    for ((cells, nodes), tr) in rows.iter().zip(&cell_nodes_by_row).zip(&row_nodes)
                    {
                        let before = word_grid_before(tr)?;
                        let mut column = before;
                        for (cell, node) in cells.iter().skip(usize::from(before > 0)).zip(nodes) {
                            if let Some(width) = node
                                .child("tcPr")
                                .and_then(|n| n.child("tcW"))
                                .filter(|n| matches!(n.attr("type"), None | Some("dxa")))
                                .and_then(|n| n.attr("w"))
                                .and_then(|v| v.parse::<f32>().ok())
                                .filter(|v| v.is_finite() && *v > 0.0)
                            {
                                for slot in inferred.iter_mut().skip(column).take(cell.span()) {
                                    if slot.is_none() {
                                        *slot = Some(
                                            (width / 20.0 / cell.span() as f32)
                                                .clamp(0.05, 10000.0),
                                        );
                                    }
                                }
                            }
                            column += cell.span();
                        }
                    }
                    if inferred.iter().any(Option::is_some) {
                        let fallback =
                            (doc.page.width - doc.page.margin_left - doc.page.margin_right)
                                / columns.max(1) as f32;
                        widths = inferred
                            .into_iter()
                            .map(|width| width.unwrap_or(fallback).clamp(0.05, 10000.0))
                            .collect();
                    }
                }
                for ((row, nodes), tr) in rows.iter_mut().zip(&cell_nodes_by_row).zip(&row_nodes) {
                    let placeholder = usize::from(word_grid_before(tr)? > 0);
                    let original = std::mem::take(row);
                    let mut merged: Vec<Cell> = Vec::with_capacity(original.len());
                    let mut head = None;
                    for (index, mut cell) in original.into_iter().enumerate() {
                        let marker = index
                            .checked_sub(placeholder)
                            .and_then(|index| nodes.get(index))
                            .and_then(|node| node.child("tcPr"))
                            .and_then(|properties| properties.child("hMerge"));
                        if marker.is_some_and(|node| node.value() != Some("restart")) {
                            if let Some(owner) = head.and_then(|index: usize| merged.get_mut(index))
                            {
                                let span = owner.span() + cell.span();
                                if span > 256 {
                                    return Err(
                                        "Горизонтальное объединение DOCX превышает 256 столбцов"
                                            .into(),
                                    );
                                }
                                owner.span = span as u16;
                                if cell
                                    .paragraphs
                                    .iter()
                                    .any(|paragraph| !paragraph.is_empty())
                                {
                                    owner.paragraphs.append(&mut cell.paragraphs);
                                    doc.losses.push("Текст продолжения hMerge DOCX сохранён в объединённой ячейке.".into());
                                }
                                continue;
                            }
                            doc.losses.push("Продолжение hMerge DOCX без начальной ячейки оставлено отдельной ячейкой.".into());
                        }
                        head = marker
                            .filter(|node| node.value() == Some("restart"))
                            .map(|_| merged.len());
                        merged.push(cell);
                    }
                    *row = merged;
                }
                // Borders come from the table itself, its style, or the cells; Word's plain
                // table has none, so a table without any of them stays borderless.
                let own = tbl_pr.and_then(|n| n.child("tblBorders"));
                let mut cell_margin = WORD_CELL_MARGIN;
                for mar in chain
                    .iter()
                    .filter_map(|s| s.child("tblPr"))
                    .chain(tbl_pr)
                    .filter_map(|n| n.child("tblCellMar"))
                {
                    for (slot, names) in [
                        (0, ["top", "top"]),
                        (1, ["right", "end"]),
                        (2, ["bottom", "bottom"]),
                        (3, ["left", "start"]),
                    ] {
                        if let Some(side) = names.iter().find_map(|name| mar.child(name))
                            && matches!(side.attr("type"), None | Some("dxa"))
                        {
                            cell_margin[slot] =
                                (number(side.attr("w"), 0.0) / 20.0).clamp(0.0, 72.0);
                        }
                    }
                }
                if row_heights.iter().all(|h| *h <= 0.0) {
                    row_heights.clear();
                }
                let styled = chain
                    .iter()
                    .rev()
                    .find_map(|s| s.child("tblPr").and_then(|n| n.child("tblBorders")));
                let mut cell_borders = Vec::new();
                node.descendants("tcBorders", &mut cell_borders);
                let visible = borders_visible(own)
                    .or_else(|| borders_visible(styled))
                    .unwrap_or(false)
                    || cell_borders
                        .iter()
                        .any(|b| borders_visible(Some(b)).unwrap_or(false));
                let border_color = border_color(own)
                    .or_else(|| border_color(styled))
                    .or_else(|| cell_borders.iter().find_map(|b| border_color(Some(b))))
                    .unwrap_or([0, 0, 0]);
                doc.tables.push(Table {
                    id,
                    after_paragraph_id: after,
                    rows,
                    column_widths: widths,
                    border_color,
                    borderless: !visible,
                    cell_margin,
                    row_heights,
                    header_rows,
                });
            }
            "sectPr" => read_section(node, &mut doc, &rels, &parts)?,
            _ => {}
        }
        if node.name == "p"
            && let Some(section) = node.child("pPr").and_then(|n| n.child("sectPr"))
        {
            read_section(section, &mut doc, &rels, &parts)?;
            doc.losses
                .push("Несколько разделов приведены к единым параметрам страницы.".into());
        }
    }
    if let Some(core) = parts.get("docprops/core.xml") {
        let node = xml(core)?;
        doc.title = node
            .child("title")
            .map(Node::text_content)
            .unwrap_or_default();
        doc.author = node
            .child("creator")
            .map(Node::text_content)
            .unwrap_or_default();
    }
    for (tag, message) in [
        ("footnoteReference", "Сноски не импортированы."),
        ("endnoteReference", "Концевые сноски не импортированы."),
        ("commentReference", "Комментарии не импортированы."),
        (
            "ins",
            "История исправлений импортирована как текущий текст.",
        ),
        ("del", "Удалённые исправления пропущены."),
        ("object", "OLE-объекты не исполняются и не импортируются."),
        ("chart", "Диаграммы Docs не импортированы."),
        (
            "fldSimple",
            "Поля импортированы как сохранённый текст; динамическое обновление отсутствует.",
        ),
        (
            "instrText",
            "Поля импортированы как сохранённый текст; динамическое обновление отсутствует.",
        ),
        ("anchor", "Плавающие рисунки переведены в отдельные блоки."),
        ("tblpPr", "Плавающие таблицы размещены в потоке текста."),
        ("dstrike", "Двойное зачёркивание приведено к одинарному."),
    ] {
        if root.descendant(tag).is_some() {
            doc.losses.push(message.into());
        }
    }
    doc.losses.sort();
    doc.losses.dedup();
    doc.normalize();
    Ok(doc)
}
fn read_section(
    node: &Node,
    doc: &mut Doc,
    rels: &BTreeMap<String, String>,
    parts: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    if let Some(n) = node.child("pgSz") {
        doc.page.width = number(n.attr("w"), 11905.0) / 20.0;
        doc.page.height = number(n.attr("h"), 16838.0) / 20.0;
    }
    if let Some(n) = node.child("pgMar") {
        doc.page.margin_left = number(n.attr("left"), 1134.0) / 20.0;
        doc.page.margin_right = number(n.attr("right"), 1134.0) / 20.0;
        doc.page.margin_top = number(n.attr("top"), 1134.0) / 20.0;
        doc.page.margin_bottom = number(n.attr("bottom"), 1134.0) / 20.0;
    }
    for (tag, header) in [("headerReference", true), ("footerReference", false)] {
        // The default header is shown on most pages; first/even ones are not modelled yet.
        let references = node.children.iter().filter(|n| n.name == tag);
        let reference = references
            .clone()
            .find(|n| n.attr("type").is_none_or(|t| t == "default"))
            .or_else(|| references.clone().next());
        if let Some(target) = reference
            .and_then(|n| n.attr("id"))
            .and_then(|id| rels.get(id))
            .and_then(|s| resolve_part(s))
            && let Some(bytes) = parts.get(&target)
        {
            let n = xml(bytes)?;
            let mut fields = Vec::new();
            let mut text = String::new();
            let mut has_page = false;
            fn page_field(instruction: &str) -> bool {
                instruction
                    .split_whitespace()
                    .next()
                    .is_some_and(|code| code.eq_ignore_ascii_case("PAGE"))
            }
            fn visible_header(
                n: &Node,
                fields: &mut Vec<(String, bool)>,
                gap: &mut &'static str,
                out: &mut String,
                has_page: &mut bool,
            ) {
                if matches!(n.name.as_str(), "del" | "moveFrom" | "pPr" | "rPr") {
                    return;
                }
                if n.name == "fldSimple" && n.attr("instr").is_some_and(page_field) {
                    *has_page = true;
                    return;
                }
                if (n.name == "p" || n.name == "tab") && !out.is_empty() && !out.ends_with(' ') {
                    *gap = if n.name == "tab" || *gap == "    " {
                        "    "
                    } else {
                        "  "
                    };
                }
                if n.name == "fldChar" {
                    match n.attr("fldCharType") {
                        Some("begin") => fields.push((String::new(), false)),
                        Some("separate") => {
                            if let Some((instruction, result)) = fields.last_mut() {
                                *result = true;
                                *has_page |= page_field(instruction);
                            }
                        }
                        Some("end") => {
                            fields.pop();
                        }
                        _ => {}
                    }
                } else if n.name == "instrText" {
                    if let Some((instruction, false)) = fields.last_mut() {
                        instruction.push_str(&n.text_content());
                    }
                } else if n.name == "t"
                    && fields
                        .iter()
                        .all(|(instruction, result)| *result && !page_field(instruction))
                {
                    let text = n.text_content();
                    if !text.is_empty() {
                        out.push_str(gap);
                        *gap = "";
                        out.push_str(&text);
                    }
                } else {
                    for child in &n.children {
                        visible_header(child, fields, gap, out, has_page);
                    }
                }
            }
            visible_header(&n, &mut fields, &mut "", &mut text, &mut has_page);
            if header {
                doc.page.header = text;
            } else {
                doc.page.footer = text;
            }
            doc.page.page_numbers |= has_page;
        }
    }
    if node
        .child("cols")
        .is_some_and(|n| number(n.attr("num"), 1.0) > 1.0)
    {
        doc.losses
            .push("Многоколоночная вёрстка преобразована в одну колонку.".into());
    }
    Ok(())
}
/// Run properties in the element order required by the OOXML schema (CT_RPr).
fn rpr_xml(f: &TextFormat) -> String {
    let font = esc(&f.font);
    let mut properties = format!(
        "<w:rFonts w:ascii=\"{font}\" w:hAnsi=\"{font}\" w:eastAsia=\"{font}\" w:cs=\"{font}\"/>"
    );
    for (on, tag) in [(f.bold, "b"), (f.italic, "i"), (f.strike, "strike")] {
        properties.push_str(&format!(
            "<w:{tag} w:val=\"{}\"/>",
            if on { "true" } else { "false" }
        ));
    }
    let size = (f.size * 2.0).round() as u32;
    properties.push_str(&format!(
        "<w:color w:val=\"{}\"/><w:sz w:val=\"{size}\"/><w:szCs w:val=\"{size}\"/><w:u w:val=\"{}\"/>",
        hex(f.color),
        if f.underline { "single" } else { "none" }
    ));
    if let Some(c) = f.highlight {
        properties.push_str(&format!(
            "<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"{}\"/>",
            hex(c)
        ));
    }
    if f.script != 0 {
        properties.push_str(&format!(
            "<w:vertAlign w:val=\"{}\"/>",
            if f.script > 0 {
                "superscript"
            } else {
                "subscript"
            }
        ));
    }
    properties
}
fn r_xml(run: &Run) -> String {
    let properties = rpr_xml(&run.format);
    let mut text = String::new();
    let mut current = String::new();
    for ch in run.text.replace("\r\n", "\n").replace('\r', "\n").chars() {
        if ch == '\t' || ch == '\u{000b}' || ch == '\n' || ch == '\u{000c}' {
            if !current.is_empty() {
                text.push_str(&format!(
                    "<w:t xml:space=\"preserve\">{}</w:t>",
                    esc(&current)
                ));
                current.clear();
            }
            text.push_str(if ch == '\t' {
                "<w:tab/>"
            } else if ch == '\u{000c}' {
                "<w:br w:type=\"page\"/>"
            } else {
                "<w:br/>"
            });
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        text.push_str(&format!(
            "<w:t xml:space=\"preserve\">{}</w:t>",
            esc(&current)
        ));
    }
    format!("<w:r><w:rPr>{properties}</w:rPr>{text}</w:r>")
}
#[derive(Default)]
struct ExportNumbering {
    paragraphs: BTreeMap<u64, usize>,
    starts: Vec<[usize; 9]>,
}
impl ExportNumbering {
    fn record(&mut self, paragraphs: &[Paragraph]) {
        let mut counters = [0_usize; 9];
        let mut current = None;
        let mut start_level = 0;
        for paragraph in paragraphs {
            let level = usize::from(paragraph.list_level).min(8);
            match paragraph.list {
                List::None => {
                    counters.fill(0);
                    current = None;
                }
                List::Bullet => {
                    counters[level..].fill(0);
                    current = None;
                }
                List::Number => {
                    let restart = paragraph
                        .list_start
                        .is_some_and(|start| start as usize != counters[level].saturating_add(1));
                    if current.is_none() || level < start_level || restart {
                        let mut starts = counters.map(|value| value + 1);
                        starts[level + 1..].fill(1);
                        if let Some(start) = paragraph.list_start {
                            starts[level] = start as usize;
                        }
                        let id = self.starts.len() + 2;
                        self.starts.push(starts);
                        current = Some(id);
                        start_level = level;
                    }
                    counters[level] = paragraph
                        .list_start
                        .map(|start| start as usize)
                        .unwrap_or_else(|| counters[level].saturating_add(1));
                    counters[level + 1..].fill(0);
                    if let Some(id) = current {
                        self.paragraphs.insert(paragraph.id, id);
                    }
                }
            }
        }
    }
    fn of(doc: &Doc) -> Self {
        let mut export = Self::default();
        export.record(&doc.paragraphs);
        for cell in doc
            .tables
            .iter()
            .flat_map(|table| table.rows.iter().flatten())
        {
            export.record(&cell.paragraphs);
        }
        export
    }
    fn xml(&self) -> String {
        self.starts.iter().enumerate().map(|(index, starts)| {
            let overrides = starts.iter().enumerate().map(|(level, start)| {
                format!("<w:lvlOverride w:ilvl=\"{level}\"><w:startOverride w:val=\"{start}\"/></w:lvlOverride>")
            }).collect::<String>();
            format!("<w:num w:numId=\"{}\"><w:abstractNumId w:val=\"2\"/>{overrides}</w:num>", index + 2)
        }).collect()
    }
}
fn p_xml(p: &Paragraph, links: &mut Vec<(String, String)>, numbering: &ExportNumbering) -> String {
    let first_indent = if p.first_indent < 0.0 {
        format!("w:hanging=\"{}\"", (-p.first_indent * 20.0).round() as u32)
    } else {
        format!("w:firstLine=\"{}\"", (p.first_indent * 20.0).round() as u32)
    };
    let line = if p.line_points > 0.0 {
        format!(
            "w:line=\"{}\" w:lineRule=\"{}\"",
            (p.line_points.min(1584.0) * 20.0).round() as u32,
            if p.line_exact { "exact" } else { "atLeast" }
        )
    } else {
        format!(
            "w:line=\"{}\" w:lineRule=\"auto\"",
            (p.line_spacing * 240.0).round() as u32
        )
    };
    // Paragraph properties in the element order required by the OOXML schema (CT_PPr).
    let mut properties = format!("<w:pStyle w:val=\"{}\"/>", esc(&p.style));
    if p.keep_next {
        properties.push_str("<w:keepNext/>");
    }
    if p.keep_lines {
        properties.push_str("<w:keepLines/>");
    }
    if p.page_break_before {
        properties.push_str("<w:pageBreakBefore/>");
    }
    properties.push_str(if p.widow_control {
        "<w:widowControl/>"
    } else {
        "<w:widowControl w:val=\"0\"/>"
    });
    if p.list != List::None {
        properties.push_str(&format!(
            "<w:numPr><w:ilvl w:val=\"{}\"/><w:numId w:val=\"{}\"/></w:numPr>",
            p.list_level,
            if p.list == List::Bullet {
                1
            } else {
                numbering.paragraphs.get(&p.id).copied().unwrap_or(2)
            }
        ));
    }
    properties.push_str(&format!(
        "<w:spacing w:before=\"{}\" w:after=\"{}\" {line}/><w:ind w:left=\"{}\" w:right=\"{}\" {first_indent}/>{}<w:jc w:val=\"{}\"/>",
        (p.spacing_before * 20.0).round() as u32,
        (p.spacing_after * 20.0).round() as u32,
        (p.indent * 20.0).round() as i32,
        (p.right_indent * 20.0).round() as i32,
        if p.contextual_spacing {
            "<w:contextualSpacing/>"
        } else {
            ""
        },
        match p.align {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
            Align::Justify => "both",
        },
    ));
    // The paragraph mark carries the format of an empty line (its height in Word).
    properties.push_str(&format!("<w:rPr>{}</w:rPr>", rpr_xml(&p.format_at(0))));
    let mut runs = String::new();
    for run in &p.runs {
        if let Some(target) = &run.format.link {
            let rid = format!("hyperlink{}", links.len() + 1);
            links.push((rid.clone(), target.clone()));
            runs.push_str(&format!(
                "<w:hyperlink r:id=\"{rid}\">{}</w:hyperlink>",
                r_xml(run)
            ));
        } else {
            runs.push_str(&r_xml(run));
        }
    }
    format!("<w:p><w:pPr>{properties}</w:pPr>{runs}</w:p>")
}
/// Style of the tiny paragraph written between adjacent tables (skipped on import).
const TABLE_GAP: &str = "TrineDocsTableGap";
fn table_xml(
    table: &Table,
    usable_width: f32,
    links: &mut Vec<(String, String)>,
    numbering: &ExportNumbering,
) -> String {
    let columns = table.grid_columns().max(1);
    let widths: Vec<u32> = (0..columns)
        .map(|i| {
            let pt = table
                .column_widths
                .get(i)
                .copied()
                .unwrap_or(usable_width / columns as f32)
                .max(0.05);
            (pt * 20.0).round() as u32
        })
        .collect();
    // Explicit widths + fixed layout: Word keeps the columns exactly as laid out here
    // instead of auto-fitting (shrinking) the table to its content.
    let mut out = format!(
        "<w:tbl><w:tblPr><w:tblW w:w=\"{}\" w:type=\"dxa\"/><w:tblBorders>",
        widths.iter().sum::<u32>()
    );
    for tag in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        if table.borderless {
            out.push_str(&format!("<w:{tag} w:val=\"nil\"/>"));
        } else {
            out.push_str(&format!(
                "<w:{tag} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"{}\"/>",
                hex(table.border_color)
            ));
        }
    }
    out.push_str("</w:tblBorders><w:tblLayout w:type=\"fixed\"/><w:tblCellMar>");
    let twips = table
        .cell_margin
        .map(|m| (m.clamp(0.0, 72.0) * 20.0).round() as u32);
    for (tag, value) in ["top", "left", "bottom", "right"]
        .into_iter()
        .zip([twips[0], twips[3], twips[2], twips[1]])
    {
        out.push_str(&format!("<w:{tag} w:w=\"{value}\" w:type=\"dxa\"/>"));
    }
    out.push_str("</w:tblCellMar></w:tblPr><w:tblGrid>");
    for width in &widths {
        out.push_str(&format!("<w:gridCol w:w=\"{width}\"/>"));
    }
    out.push_str("</w:tblGrid>");
    for (row_index, row) in table.rows.iter().enumerate() {
        out.push_str("<w:tr>");
        let mut row_properties = String::new();
        if let Some(height) = table.row_heights.get(row_index).filter(|h| **h > 0.0) {
            row_properties.push_str(&format!(
                "<w:trHeight w:val=\"{}\" w:hRule=\"atLeast\"/>",
                (height.min(1500.0) * 20.0).round() as u32
            ));
        }
        if row_index < table.header_rows {
            row_properties.push_str("<w:tblHeader/>");
        }
        if !row_properties.is_empty() {
            out.push_str(&format!("<w:trPr>{row_properties}</w:trPr>"));
        }
        let starts = table.grid_starts(row_index);
        let below = table.grid_starts(row_index + 1);
        for (ci, cell) in row.iter().enumerate() {
            let start = starts.get(ci).copied().unwrap_or_default();
            let width: u32 = widths.iter().skip(start).take(cell.span()).sum();
            let mut properties = format!("<w:tcW w:w=\"{width}\" w:type=\"dxa\"/>");
            if cell.span > 1 {
                properties.push_str(&format!("<w:gridSpan w:val=\"{}\"/>", cell.span));
            }
            let continues = table.rows.get(row_index + 1).is_some_and(|next| {
                below
                    .iter()
                    .position(|s| *s == start)
                    .and_then(|i| next.get(i))
                    .is_some_and(|c| c.merged)
            });
            if cell.merged {
                properties.push_str("<w:vMerge/>");
            } else if continues {
                properties.push_str("<w:vMerge w:val=\"restart\"/>");
            }
            properties.push_str(&format!(
                "<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"{}\"/>",
                hex(cell.fill)
            ));
            match cell.v_align {
                VAlign::Top => {}
                VAlign::Center => properties.push_str("<w:vAlign w:val=\"center\"/>"),
                VAlign::Bottom => properties.push_str("<w:vAlign w:val=\"bottom\"/>"),
            }
            out.push_str(&format!("<w:tc><w:tcPr>{properties}</w:tcPr>"));
            for p in &cell.paragraphs {
                out.push_str(&p_xml(p, links, numbering));
            }
            if cell.paragraphs.is_empty() {
                out.push_str("<w:p/>");
            }
            out.push_str("</w:tc>");
        }
        out.push_str("</w:tr>");
    }
    out.push_str("</w:tbl>");
    out
}
fn write_docx(doc: &Doc, ext: &str) -> Result<Vec<u8>, String> {
    let mut doc = doc.clone();
    doc.normalize();
    let mut links = Vec::new();
    let export_numbering = ExportNumbering::of(&doc);
    let mut body = String::new();
    let mut media = Vec::new();
    let usable = doc.page.width - doc.page.margin_left - doc.page.margin_right;
    let gap = format!(
        "<w:p><w:pPr><w:pStyle w:val=\"{TABLE_GAP}\"/><w:spacing w:before=\"0\" w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/><w:rPr><w:sz w:val=\"2\"/></w:rPr></w:pPr></w:p>"
    );
    let last_paragraph = doc.paragraphs.last().map(|p| p.id);
    for p in &doc.paragraphs {
        body.push_str(&p_xml(p, &mut links, &export_numbering));
        let tables: Vec<&Table> = doc
            .tables
            .iter()
            .filter(|t| t.after_paragraph_id == p.id)
            .collect();
        let has_images = doc.images.iter().any(|im| im.after_paragraph_id == p.id);
        for (i, table) in tables.iter().enumerate() {
            body.push_str(&table_xml(table, usable, &mut links, &export_numbering));
            // Word merges adjacent tables and needs a paragraph after a final table.
            let followed_by_table = i + 1 < tables.len();
            let ends_body = i + 1 == tables.len() && !has_images && last_paragraph == Some(p.id);
            if followed_by_table || ends_body {
                body.push_str(&gap);
            }
        }
        for image in doc.images.iter().filter(|im| im.after_paragraph_id == p.id) {
            let decoded =
                crate::util::decode_image(&image.bytes).map_err(|e| format!("Изображение: {e}"))?;
            let mut png = std::io::Cursor::new(Vec::new());
            decoded
                .write_to(&mut png, image::ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            let name = format!("image{}.png", image.id);
            media.push((format!("word/media/{name}"), png.into_inner()));
            let cx = (image.width * 12700.0).round() as u32;
            let cy = (image.height * 12700.0).round() as u32;
            body.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"TrineDocsImageBlock\"/></w:pPr><w:r><w:drawing><wp:inline><wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{}\" name=\"{}\" descr=\"{}\"/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"{}\" name=\"{}\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"image{}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>",image.id,esc(&name),esc(&image.alt),image.id,esc(&name),image.id));
        }
    }
    let setup = &doc.page;
    body.push_str(&format!("<w:sectPr><w:headerReference w:type=\"default\" r:id=\"header\"/><w:footerReference w:type=\"default\" r:id=\"footer\"/><w:pgSz w:w=\"{}\" w:h=\"{}\" w:orient=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"400\" w:footer=\"400\"/></w:sectPr>",
        (setup.width*20.0).round() as u32,(setup.height*20.0).round() as u32,if setup.width > setup.height {"landscape"} else {"portrait"},(setup.margin_top*20.0).round() as u32,
        (setup.margin_right*20.0).round() as u32,(setup.margin_bottom*20.0).round() as u32,(setup.margin_left*20.0).round() as u32));
    let ns = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"";
    let main = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document {ns}><w:body>{body}</w:body></w:document>"
    );
    let mut relationships = String::from(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    for (id, kind, target) in [
        ("styles", "styles", "styles.xml"),
        ("numbering", "numbering", "numbering.xml"),
        ("header", "header", "header1.xml"),
        ("footer", "footer", "footer1.xml"),
    ] {
        relationships.push_str(&format!("<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/>"));
    }
    for image in &doc.images {
        relationships.push_str(&format!("<Relationship Id=\"image{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{}.png\"/>",image.id,image.id));
    }
    for (id, target) in links {
        relationships.push_str(&format!("<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"{}\" TargetMode=\"External\"/>",esc(&target)));
    }
    relationships.push_str("</Relationships>");
    let content_type = if ext == "dotx" {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml"
    } else if ext == "docm" {
        "application/vnd.ms-word.document.macroEnabled.main+xml"
    } else {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
    };
    let mut types = format!(
        "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Override PartName=\"/word/document.xml\" ContentType=\"{content_type}\"/>"
    );
    for (path, kind) in [
        ("styles", "styles"),
        ("numbering", "numbering"),
        ("header1", "header"),
        ("footer1", "footer"),
    ] {
        types.push_str(&format!("<Override PartName=\"/word/{path}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.{kind}+xml\"/>"));
    }
    types.push_str("<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/></Types>");
    let root_rels = "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"document\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"core\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>";
    let core = format!(
        "<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>",
        esc(&doc.title),
        esc(&doc.author)
    );
    let mut styles = String::from(
        "<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:eastAsia=\"Calibri\" w:cs=\"Calibri\"/><w:sz w:val=\"22\"/><w:lang w:val=\"ru-RU\" w:eastAsia=\"en-US\" w:bidi=\"ar-SA\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>",
    );
    for (id, name, size, bold) in [
        ("Normal", "Normal", 22, false),
        ("Title", "Title", 56, true),
        ("Heading1", "heading 1", 44, true),
        ("Heading2", "heading 2", 34, true),
        ("Heading3", "heading 3", 28, true),
        ("Heading4", "heading 4", 24, true),
        ("Heading5", "heading 5", 22, true),
        ("Heading6", "heading 6", 22, true),
        ("Quote", "Quote", 22, false),
    ] {
        let default = if id == "Normal" {
            " w:default=\"1\""
        } else {
            ""
        };
        styles.push_str(&format!("<w:style w:type=\"paragraph\"{default} w:styleId=\"{id}\"><w:name w:val=\"{name}\"/><w:rPr><w:sz w:val=\"{size}\"/>{}</w:rPr></w:style>",if bold {"<w:b/>"}else{""}));
    }
    styles.push_str(&format!("<w:style w:type=\"paragraph\" w:styleId=\"TrineDocsImageBlock\"><w:name w:val=\"TrineDocs image block\"/></w:style><w:style w:type=\"paragraph\" w:styleId=\"{TABLE_GAP}\"><w:name w:val=\"TrineDocs table gap\"/><w:hidden/></w:style></w:styles>"));
    let mut numbering = String::from(
        "<w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">",
    );
    for (id, kind, marker) in [(1, "bullet", "•"), (2, "decimal", "%1.")] {
        numbering.push_str(&format!(
            "<w:abstractNum w:abstractNumId=\"{id}\"><w:multiLevelType w:val=\"multilevel\"/>"
        ));
        for level in 0..9 {
            let marker = if kind == "decimal" {
                format!("%{}.", level + 1)
            } else {
                marker.to_string()
            };
            numbering.push_str(&format!("<w:lvl w:ilvl=\"{level}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"{kind}\"/><w:lvlText w:val=\"{marker}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"{}\" w:hanging=\"360\"/></w:pPr></w:lvl>",360+level*360));
        }
        numbering.push_str("</w:abstractNum>");
    }
    numbering.push_str("<w:num w:numId=\"1\"><w:abstractNumId w:val=\"1\"/></w:num>");
    numbering.push_str(&export_numbering.xml());
    numbering.push_str("</w:numbering>");
    let header = format!(
        "<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:hdr>",
        esc(&setup.header)
    );
    let footer = format!(
        "<w:ftr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r>{}</w:p></w:ftr>",
        esc(&setup.footer),
        if setup.page_numbers {
            "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r>"
        } else {
            ""
        }
    );
    let mut parts = vec![
        ("[Content_Types].xml".into(), types.into_bytes()),
        ("_rels/.rels".into(), root_rels.as_bytes().to_vec()),
        ("word/document.xml".into(), main.into_bytes()),
        (
            "word/_rels/document.xml.rels".into(),
            relationships.into_bytes(),
        ),
        ("word/styles.xml".into(), styles.into_bytes()),
        ("word/numbering.xml".into(), numbering.into_bytes()),
        ("word/header1.xml".into(), header.into_bytes()),
        ("word/footer1.xml".into(), footer.into_bytes()),
        ("docProps/core.xml".into(), core.into_bytes()),
    ];
    parts.extend(media);
    write_zip(parts)
}
fn md_spend(budget: &mut usize, amount: usize) -> bool {
    if let Some(remaining) = budget.checked_sub(amount) {
        *budget = remaining;
        true
    } else {
        false
    }
}
fn md_unescaped(text: &str, needle: &str, budget: &mut usize) -> Result<Option<usize>, ()> {
    let bytes = text.as_bytes();
    let mut escaped = false;
    for (index, ch) in bytes.iter().copied().enumerate() {
        if !md_spend(budget, 1) {
            return Err(());
        }
        if ch == b'\\' {
            escaped = !escaped;
            continue;
        }
        if !escaped && bytes[index..].starts_with(needle.as_bytes()) {
            if !md_spend(budget, needle.len().saturating_sub(1)) {
                return Err(());
            }
            return Ok(Some(index));
        }
        escaped = false;
    }
    Ok(None)
}
fn inline_md(text: &str, inherited: &TextFormat) -> Vec<Run> {
    fn parse(
        text: &str,
        inherited: &TextFormat,
        depth: usize,
        budget: &mut usize,
        out: &mut Vec<Run>,
    ) {
        if depth >= 16 {
            out.push(Run {
                text: text.into(),
                format: inherited.clone(),
            });
            return;
        }
        let mut plain = String::new();
        let mut offset = 0;
        'inline: while offset < text.len() {
            let rest = &text[offset..];
            if !md_spend(budget, 1) {
                plain.push_str(rest);
                break;
            }
            let Some(ch) = rest.chars().next() else { break };
            if ch == '\\'
                && let Some(next) = rest.chars().nth(1).filter(char::is_ascii_punctuation)
            {
                plain.push(next);
                offset += 1 + next.len_utf8();
                continue;
            }
            if ch == '&'
                && let Some(end) = rest.bytes().take(33).position(|ch| ch == b';')
                && let Some(decoded) = markdown_entity(&rest[..=end])
                && decoded.as_str() != &rest[..=end]
            {
                plain.push_str(&decoded);
                offset += end + 1;
                continue;
            }
            if ch == '`' {
                let length = rest.bytes().take_while(|ch| *ch == b'`').count();
                let marker = "`".repeat(length);
                let closing = match markdown_code_end(rest, length, budget) {
                    Ok(end) => end,
                    Err(()) => {
                        plain.push_str(rest);
                        break;
                    }
                };
                if let Some(end) = closing {
                    if !plain.is_empty() {
                        out.push(Run {
                            text: std::mem::take(&mut plain),
                            format: inherited.clone(),
                        });
                    }
                    let mut content = rest[length..end].replace(['\n', '\u{000b}'], " ");
                    if content.starts_with(' ')
                        && content.ends_with(' ')
                        && !content.chars().all(|ch| ch == ' ')
                    {
                        content = content[1..content.len() - 1].into();
                    }
                    let mut format = inherited.clone();
                    format.font = "Consolas".into();
                    out.push(Run {
                        text: content,
                        format,
                    });
                    offset += end + length;
                    continue;
                }
                plain.push_str(&marker);
                offset += length;
                continue;
            }
            if ch == '<' {
                let end = match md_unescaped(rest, ">", budget) {
                    Ok(end) => end,
                    Err(()) => {
                        plain.push_str(rest);
                        break;
                    }
                };
                if let Some(end) = end {
                    let target = &rest[1..end];
                    if !target.chars().any(char::is_whitespace)
                        && (target.to_ascii_lowercase().starts_with("https://")
                            || target.to_ascii_lowercase().starts_with("http://")
                            || target.to_ascii_lowercase().starts_with("mailto:")
                            || markdown_email(target))
                    {
                        if !plain.is_empty() {
                            out.push(Run {
                                text: std::mem::take(&mut plain),
                                format: inherited.clone(),
                            });
                        }
                        let mut format = inherited.clone();
                        format.link = Some(if markdown_email(target) {
                            format!("mailto:{target}")
                        } else {
                            target.into()
                        });
                        let display = if target
                            .get(..7)
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("mailto:"))
                        {
                            &target[7..]
                        } else {
                            target
                        };
                        out.push(Run {
                            text: display.into(),
                            format,
                        });
                        offset += end + 1;
                        continue;
                    }
                }
            }
            if ch == '[' {
                let close = match md_unescaped(rest, "](", budget) {
                    Ok(close) => close,
                    Err(()) => {
                        plain.push_str(rest);
                        break;
                    }
                };
                if let Some(close) = close {
                    let url_start = close + 2;
                    let end = match markdown_link_end(rest, url_start, budget) {
                        Ok(end) => end,
                        Err(()) => {
                            plain.push_str(rest);
                            break 'inline;
                        }
                    };
                    if let Some(end) = end {
                        if !plain.is_empty() {
                            out.push(Run {
                                text: std::mem::take(&mut plain),
                                format: inherited.clone(),
                            });
                        }
                        let mut format = inherited.clone();
                        let Some(url) = markdown_destination(&rest[url_start..end]) else {
                            plain.push(ch);
                            offset += ch.len_utf8();
                            continue;
                        };
                        format.link = Some(url);
                        parse(&rest[1..close], &format, depth + 1, budget, out);
                        offset += end + 1;
                        continue;
                    }
                }
            }
            if matches!(ch, '*' | '_' | '~') {
                let length = rest.chars().take_while(|next| *next == ch).count();
                if rest[length..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace)
                {
                    plain.push_str(&rest[..length]);
                    offset += length;
                    continue;
                }
            }
            let mut matched = false;
            for marker in ["***", "___", "**", "__", "~~", "*", "_"] {
                if !rest.starts_with(marker)
                    || rest[marker.len()..]
                        .chars()
                        .next()
                        .is_none_or(char::is_whitespace)
                {
                    continue;
                }
                if marker.starts_with('_')
                    && text[..offset]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_alphanumeric)
                    && rest[marker.len()..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric)
                {
                    continue;
                }
                let end = match markdown_emphasis_end(&rest[marker.len()..], marker, budget) {
                    Ok(Some(end)) if end > 0 => end,
                    Ok(_) => continue,
                    Err(()) => {
                        plain.push_str(rest);
                        break 'inline;
                    }
                };
                if !plain.is_empty() {
                    out.push(Run {
                        text: std::mem::take(&mut plain),
                        format: inherited.clone(),
                    });
                }
                let mut format = inherited.clone();
                format.bold |= matches!(marker, "**" | "__" | "***" | "___");
                format.italic |= matches!(marker, "*" | "_" | "***" | "___");
                format.strike |= marker == "~~";
                parse(
                    &rest[marker.len()..marker.len() + end],
                    &format,
                    depth + 1,
                    budget,
                    out,
                );
                offset += marker.len() * 2 + end;
                matched = true;
                break;
            }
            if !matched {
                plain.push(ch);
                offset += ch.len_utf8();
            }
        }
        if !plain.is_empty() {
            out.push(Run {
                text: plain,
                format: inherited.clone(),
            });
        }
    }
    let mut runs = Vec::new();
    let mut budget = text.len().saturating_mul(16).clamp(4096, 16 * 1024 * 1024);
    parse(text, inherited, 0, &mut budget, &mut runs);
    if runs.is_empty() {
        runs.push(Run {
            text: String::new(),
            format: inherited.clone(),
        });
    }
    runs
}
fn escape_markdown(text: &str) -> String {
    let mut out = String::new();
    let trailing_spaces = text.trim_end_matches(' ').len();
    for (index, ch) in text.char_indices() {
        if ch == ' ' && index >= trailing_spaces {
            out.push_str("&#32;");
            continue;
        }
        if ch.is_ascii_punctuation() {
            out.push('\\');
        }
        if matches!(ch, '\u{000b}' | '\n') {
            out.push_str("  \n");
        } else {
            out.push(ch);
        }
    }
    out
}
fn markdown_list_prefix(text: &str) -> Option<(List, &str)> {
    if let Some(marker) = text
        .chars()
        .next()
        .filter(|ch| matches!(ch, '-' | '*' | '+'))
    {
        let tail = &text[marker.len_utf8()..];
        if tail.is_empty() || tail.starts_with([' ', '\t']) {
            return Some((List::Bullet, tail.trim_start_matches([' ', '\t'])));
        }
    }
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=9).contains(&digits) && matches!(text.as_bytes().get(digits), Some(b'.' | b')')) {
        let tail = &text[digits + 1..];
        if tail.is_empty() || tail.starts_with([' ', '\t']) {
            return Some((List::Number, tail.trim_start_matches([' ', '\t'])));
        }
    }
    None
}
fn md_fence(line: &str) -> Option<(char, usize)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let marker = rest.chars().next().filter(|ch| matches!(ch, '`' | '~'))?;
    let count = rest.chars().take_while(|ch| *ch == marker).count();
    (count >= 3 && (marker != '`' || !rest[count..].contains('`'))).then_some((marker, count))
}
fn md_setext(line: &str) -> Option<&'static str> {
    let text = line.trim();
    if !text.is_empty() && text.chars().all(|ch| ch == '=') {
        Some("Heading1")
    } else if !text.is_empty() && text.chars().all(|ch| ch == '-') {
        Some("Heading2")
    } else {
        None
    }
}
fn md_atx(line: &str) -> Option<(usize, &str)> {
    let text = line.trim_start_matches(' ');
    if line.len() - text.len() > 3 {
        return None;
    }
    let count = text.bytes().take_while(|ch| *ch == b'#').count();
    if !(1..=6).contains(&count) {
        return None;
    }
    let tail = &text[count..];
    if !tail.is_empty() && !tail.starts_with([' ', '\t']) {
        return None;
    }
    let tail = tail.trim();
    let without = tail.trim_end_matches('#');
    let tail =
        if without.len() < tail.len() && (without.is_empty() || without.ends_with([' ', '\t'])) {
            without.trim_end()
        } else {
            tail
        };
    Some((count, tail))
}
fn md_starts_block(line: &str) -> bool {
    let text = line.trim_start_matches([' ', '\t']);
    line.trim().is_empty()
        || md_atx(line).is_some()
        || md_fence(line).is_some()
        || markdown_list_prefix(text).is_some()
        || text.starts_with('>')
}
fn md_code_paragraph(doc: &mut Doc, lines: &[String]) {
    let id = doc.alloc_id();
    doc.paragraphs.push(Paragraph {
        id,
        style: "Code".into(),
        runs: vec![Run {
            text: lines.join("\u{000b}"),
            format: TextFormat {
                font: "Consolas".into(),
                ..Default::default()
            },
        }],
        ..Default::default()
    });
}
fn read_markdown(text: &str) -> Doc {
    read_markdown_depth(text, 0)
}
fn markdown_quote_line(line: &str) -> Option<&str> {
    let text = line.trim_start_matches(' ');
    if line.len() - text.len() > 3 {
        return None;
    }
    let text = text.strip_prefix('>')?;
    Some(text.strip_prefix([' ', '\t']).unwrap_or(text))
}
fn append_markdown_doc(target: &mut Doc, mut imported: Doc) {
    let mut ids = BTreeMap::new();
    for paragraph in &mut imported.paragraphs {
        let old = paragraph.id;
        paragraph.id = target.alloc_id();
        ids.insert(old, paragraph.id);
    }
    for table in &mut imported.tables {
        table.id = target.alloc_id();
        table.after_paragraph_id = ids
            .get(&table.after_paragraph_id)
            .copied()
            .unwrap_or_default();
        for cell in table.rows.iter_mut().flatten() {
            cell.id = target.alloc_id();
            for paragraph in &mut cell.paragraphs {
                paragraph.id = target.alloc_id();
            }
        }
    }
    for image in &mut imported.images {
        image.id = target.alloc_id();
        image.after_paragraph_id = ids
            .get(&image.after_paragraph_id)
            .copied()
            .unwrap_or_default();
    }
    target.paragraphs.extend(imported.paragraphs);
    target.tables.extend(imported.tables);
    target.images.extend(imported.images);
    target.losses.extend(imported.losses);
}
fn read_markdown_depth(text: &str, depth: usize) -> Doc {
    let mut doc = Doc::default();
    doc.paragraphs.clear();
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let versioned = normalized.starts_with(MD_PARAGRAPHS);
    let input = if versioned {
        normalized
            .strip_prefix(MD_PARAGRAPHS)
            .unwrap_or(&normalized)
            .trim_start_matches('\n')
    } else {
        normalized.as_str()
    };
    let lines = input.split_terminator('\n').collect::<Vec<_>>();
    let mut index = 0;
    let mut list_indents = Vec::new();
    while index < lines.len() {
        let line = lines[index];
        index += 1;
        if line.trim().is_empty() {
            continue;
        }
        if depth < 16
            && let Some(first) = markdown_quote_line(line)
        {
            let mut content = vec![first.to_string()];
            while index < lines.len() {
                if let Some(next) = markdown_quote_line(lines[index]) {
                    content.push(next.into());
                    index += 1;
                } else if !versioned
                    && !content.last().is_none_or(|line| line.trim().is_empty())
                    && !md_starts_block(lines[index])
                {
                    content.push(lines[index].into());
                    index += 1;
                } else {
                    break;
                }
            }
            let mut quoted = read_markdown_depth(&content.join("\n"), depth + 1);
            for paragraph in &mut quoted.paragraphs {
                if paragraph.style == "Normal" {
                    paragraph.style = "Quote".into();
                    for run in &mut paragraph.runs {
                        run.format.italic = true;
                    }
                }
            }
            if depth > 0 {
                quoted.losses.push(
                    "Вложенные блоки цитат Markdown сведены к доступному стилю цитаты.".into(),
                );
            }
            append_markdown_doc(&mut doc, quoted);
            list_indents.clear();
            continue;
        }
        if depth >= 16 && markdown_quote_line(line).is_some() {
            doc.losses
                .push("Markdown: превышена глубина вложенных цитат; оставлен текст.".into());
        }
        if let Some((marker, count)) = md_fence(line) {
            let indent = line.len() - line.trim_start_matches(' ').len();
            let mut content = Vec::new();
            while index < lines.len() {
                let value = lines[index];
                index += 1;
                if md_fence(value).is_some_and(|(next, length)| next == marker && length >= count)
                    && value.trim().chars().all(|ch| ch == marker)
                {
                    break;
                }
                let removed = value
                    .bytes()
                    .take_while(|ch| *ch == b' ')
                    .count()
                    .min(indent);
                content.push(value[removed..].to_string());
            }
            md_code_paragraph(&mut doc, &content);
            list_indents.clear();
            continue;
        }
        if (line.starts_with("    ") || line.starts_with('\t'))
            && !(markdown_list_prefix(line.trim_start()).is_some()
                && (versioned || !list_indents.is_empty()))
        {
            let mut content = vec![
                line.strip_prefix("    ")
                    .or_else(|| line.strip_prefix('\t'))
                    .unwrap_or(line)
                    .to_string(),
            ];
            while index < lines.len() {
                if let Some(value) = lines[index]
                    .strip_prefix("    ")
                    .or_else(|| lines[index].strip_prefix('\t'))
                {
                    content.push(value.to_string());
                    index += 1;
                } else if lines[index].trim().is_empty() {
                    let next = (index..lines.len()).find(|next| !lines[*next].trim().is_empty());
                    if let Some(next) = next.filter(|next| {
                        lines[*next].starts_with("    ") || lines[*next].starts_with('\t')
                    }) {
                        content.extend((index..next).map(|_| String::new()));
                        index = next;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            md_code_paragraph(&mut doc, &content);
            list_indents.clear();
            continue;
        }
        if index < lines.len()
            && let Some(header) = markdown_table_row(line)
            && let Some(alignments) = markdown_table_separator(lines[index])
            && header.len() == alignments.len()
            && !header.is_empty()
            && header.len() <= 256
        {
            index += 1;
            let mut source = vec![header];
            while index < lines.len()
                && source.len() < 10000
                && (source.len() + 1).saturating_mul(alignments.len()) <= 100000
            {
                let Some(row) = markdown_body_table_row(lines[index]) else {
                    break;
                };
                source.push(row);
                index += 1;
            }
            if index < lines.len()
                && markdown_body_table_row(lines[index]).is_some()
                && (source.len() >= 10000
                    || (source.len() + 1).saturating_mul(alignments.len()) > 100000)
            {
                doc.losses.push("Markdown: таблица превышает лимит строк/ячеек; остаток импортируется как текст.".into());
            }
            if doc.paragraphs.is_empty() {
                let id = doc.alloc_id();
                doc.paragraphs.push(Paragraph {
                    id,
                    ..Default::default()
                });
            }
            let anchor = doc
                .paragraphs
                .last()
                .map(|paragraph| paragraph.id)
                .unwrap_or_default();
            let id = doc.alloc_id();
            let mut rows = Vec::new();
            for row in source {
                let mut cells = Vec::new();
                for (column, align) in alignments.iter().enumerate() {
                    let text = row.get(column).map(String::as_str).unwrap_or("");
                    let text = text
                        .replace("<br>", "\u{000b}")
                        .replace("<br/>", "\u{000b}")
                        .replace("<br />", "\u{000b}");
                    let paragraph = Paragraph {
                        id: doc.alloc_id(),
                        align: *align,
                        runs: inline_md(&text, &TextFormat::default()),
                        ..Default::default()
                    };
                    cells.push(Cell::new(doc.alloc_id(), vec![paragraph], [255, 255, 255]));
                }
                rows.push(cells);
            }
            let width = (doc.page.width - doc.page.margin_left - doc.page.margin_right)
                / alignments.len() as f32;
            doc.tables.push(Table {
                id,
                after_paragraph_id: anchor,
                rows,
                column_widths: vec![width; alignments.len()],
                border_color: [100, 110, 120],
                borderless: false,
                cell_margin: WORD_CELL_MARGIN,
                row_heights: Vec::new(),
                header_rows: 1,
            });
            list_indents.clear();
            continue;
        }
        let mut logical = line.to_string();
        while index < lines.len()
            && !lines[index].trim().is_empty()
            && !md_starts_block(lines[index])
            && md_setext(lines[index]).is_none()
            && !(index + 1 < lines.len()
                && markdown_table_row(lines[index]).is_some()
                && markdown_table_separator(lines[index + 1]).is_some())
        {
            let slashes = logical.chars().rev().take_while(|ch| *ch == '\\').count();
            let hard = logical.ends_with("  ") || slashes % 2 == 1;
            if !hard && (versioned || md_atx(line).is_some()) {
                break;
            }
            if hard {
                logical.truncate(logical.len() - if slashes % 2 == 1 { 1 } else { 2 });
                logical.push('\u{000b}');
            } else {
                logical.truncate(logical.trim_end().len());
                logical.push(' ');
            }
            logical.push_str(lines[index].trim_start());
            index += 1;
        }
        let id = doc.alloc_id();
        let mut paragraph = Paragraph {
            id,
            ..Default::default()
        };
        let mut content = logical.as_str();
        let trimmed = content.trim_start_matches([' ', '\t']);
        if let Some((list, rest)) = markdown_list_prefix(trimmed) {
            let indent = content.len() - trimmed.len();
            paragraph.list = list;
            paragraph.list_level = if versioned {
                (indent / 4).min(8) as u8
            } else {
                while list_indents.last().is_some_and(|value| *value > indent) {
                    list_indents.pop();
                }
                if list_indents.last() != Some(&indent) {
                    list_indents.push(indent);
                }
                list_indents.len().saturating_sub(1).min(8) as u8
            };
            if list == List::Number {
                let continuing = doc
                    .paragraphs
                    .iter()
                    .rev()
                    .take_while(|previous| previous.list != List::None)
                    .find(|previous| previous.list_level <= paragraph.list_level)
                    .is_some_and(|previous| {
                        previous.list == List::Number && previous.list_level == paragraph.list_level
                    });
                if versioned || !continuing {
                    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
                    paragraph.list_start = trimmed[..digits].parse::<u32>().ok();
                }
            }
            content = rest;
        } else {
            list_indents.clear();
        }
        if let Some((level, rest)) = md_atx(content) {
            paragraph.set_style(&format!("Heading{level}"));
            content = rest;
        } else if let Some(style) = lines
            .get(index)
            .and_then(|line| md_setext(line))
            .filter(|_| paragraph.list == List::None)
        {
            paragraph.set_style(style);
            index += 1;
        } else if let Some(rest) = content.strip_prefix('>') {
            paragraph.set_style("Quote");
            content = rest.trim_start_matches(' ');
        }
        let (visible, images) = markdown_inline_images(content);
        let content = visible.as_str();
        paragraph.runs = if versioned && content == "<!--td-empty-->" {
            vec![Run {
                text: String::new(),
                format: paragraph.format_at(0),
            }]
        } else {
            inline_md(content, &paragraph.format_at(0))
        };
        let standalone = !images.is_empty() && content.trim().is_empty();
        if standalone && doc.paragraphs.is_empty() {
            let id = doc.alloc_id();
            doc.paragraphs.push(Paragraph {
                id,
                ..Default::default()
            });
        }
        let anchor = if standalone {
            doc.paragraphs
                .last()
                .map(|paragraph| paragraph.id)
                .unwrap_or_default()
        } else {
            paragraph.id
        };
        if !standalone {
            doc.paragraphs.push(paragraph);
        }
        for (alt, bytes) in images {
            if doc.images.len() >= 1000 {
                doc.losses
                    .push("Markdown: превышен лимит изображений".into());
                break;
            }
            if let Ok(image) = crate::util::decode_image(&bytes) {
                let id = doc.alloc_id();
                doc.images.push(AnchoredImage {
                    id,
                    after_paragraph_id: anchor,
                    bytes,
                    width: (image.width() as f32 * 0.75).clamp(1.0, 10000.0),
                    height: (image.height() as f32 * 0.75).clamp(1.0, 10000.0),
                    alt,
                });
            }
        }
    }
    doc
}
const MD_PARAGRAPHS: &str = "<!-- TrineDocs Markdown paragraphs v2 -->";
fn markdown_run(run: &Run) -> String {
    let marked = run.format.bold || run.format.italic || run.format.strike;
    let text = run.text.as_str();
    let core = if marked {
        text.trim_matches(char::is_whitespace)
    } else {
        text
    };
    if core.is_empty() {
        return escape_markdown(text);
    }
    let start = text.len() - text.trim_start_matches(char::is_whitespace).len();
    let start = if marked { start } else { 0 };
    let end = start + core.len();
    let mut out = escape_markdown(&text[..start]);
    let emphasis = match (run.format.bold, run.format.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        _ => "",
    };
    if run.format.strike {
        out.push_str("~~");
    }
    out.push_str(emphasis);
    if let Some(link) = run
        .format
        .link
        .as_deref()
        .filter(|link| safe_html_link(link))
    {
        let mut encoded = String::new();
        for ch in link.chars() {
            if ch.is_whitespace() || ch.is_control() || matches!(ch, '<' | '>') {
                let mut bytes = [0u8; 4];
                for byte in ch.encode_utf8(&mut bytes).as_bytes() {
                    encoded.push_str(&format!("%{byte:02X}"));
                }
            } else {
                encoded.push(ch);
            }
        }
        let target = encoded
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        out.push_str(&format!("[{}]({target})", markdown_core(core, &run.format)));
    } else {
        out.push_str(&markdown_core(core, &run.format));
    }
    out.push_str(emphasis);
    if run.format.strike {
        out.push_str("~~");
    }
    out.push_str(&escape_markdown(&text[end..]));
    out
}
fn markdown_cell(cell: &Cell) -> String {
    cell.paragraphs
        .iter()
        .map(Paragraph::text)
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split(['\n', '\u{000b}', '\u{000c}'])
        .map(escape_markdown)
        .collect::<Vec<_>>()
        .join("<br>")
}
fn write_markdown(doc: &Doc) -> String {
    if doc.paragraphs.len() == 1
        && doc.paragraphs[0].is_empty()
        && doc.paragraphs[0].style == "Normal"
        && doc.paragraphs[0].list == List::None
        && doc.tables.is_empty()
        && doc.images.is_empty()
    {
        return "\n".into();
    }
    let mut out = format!("{MD_PARAGRAPHS}\n");
    let mut numbers = [0usize; 9];
    for p in &doc.paragraphs {
        let marker = list_marker(p, &mut numbers);
        if p.style == "Code" {
            let text = p.text().replace('\u{000b}', "\n");
            let length = text
                .split(|ch| ch != '`')
                .map(str::len)
                .max()
                .unwrap_or(0)
                .max(2)
                + 1;
            let fence = "`".repeat(length);
            out.push_str(&format!("{fence}\n{text}\n{fence}\n\n"));
        } else {
            if p.list != List::None {
                out.push_str(&"    ".repeat(usize::from(p.list_level)));
            }
            match p.list {
                List::Bullet => out.push_str("- "),
                List::Number => out.push_str(&format!("{marker} ")),
                List::None => {}
            }
            if let Some(level) = heading_level(&p.style) {
                out.push_str(&"#".repeat(usize::from(level)));
                out.push(' ');
            } else if p.style == "Quote" {
                out.push_str("> ");
            }
            if p.is_empty() {
                out.push_str("<!--td-empty-->");
            } else {
                for run in &p.runs {
                    out.push_str(&markdown_run(run));
                }
            }
            // Blank lines preserve paragraph boundaries in CommonMark/GFM renderers.
            out.push_str("\n\n");
        }
        for image in doc
            .images
            .iter()
            .filter(|image| image.after_paragraph_id == p.id)
        {
            let mime = if matches!(
                image::guess_format(&image.bytes),
                Ok(image::ImageFormat::Jpeg)
            ) {
                "image/jpeg"
            } else {
                "image/png"
            };
            out.push_str(&format!(
                "![{}](data:{mime};base64,{})\n\n",
                escape_markdown(&image.alt),
                encode_image_base64(&image.bytes)
            ));
        }
        for table in doc.tables.iter().filter(|t| t.after_paragraph_id == p.id) {
            let columns = table.grid_columns();
            for (row_index, row) in table.rows.iter().enumerate() {
                out.push('|');
                let mut written = 0;
                for cell in row {
                    if row_index == 0 || !cell.merged {
                        out.push_str(&markdown_cell(cell));
                    }
                    out.push('|');
                    written += 1;
                    for _ in 1..cell.span() {
                        out.push('|');
                        written += 1;
                    }
                }
                for _ in written..columns {
                    out.push('|');
                }
                out.push('\n');
                if row_index == 0 {
                    out.push('|');
                    let mut aligned = 0;
                    for cell in row {
                        let align = cell
                            .paragraphs
                            .first()
                            .map(|paragraph| paragraph.align)
                            .unwrap_or_default();
                        let marker = match align {
                            Align::Center => ":---:",
                            Align::Right => "---:",
                            _ => "---",
                        };
                        for _ in 0..cell.span() {
                            out.push_str(marker);
                            out.push('|');
                            aligned += 1;
                        }
                    }
                    for _ in aligned..columns {
                        out.push_str("---|");
                    }
                    out.push('\n');
                }
            }
            out.push('\n');
        }
    }
    out
}

fn css_points(value: &str, fallback: f32) -> f32 {
    let value = value.trim().to_ascii_lowercase();
    let (digits, factor) = ["px", "pt", "in", "cm", "mm", "pc", "q"]
        .into_iter()
        .find_map(|unit| {
            value.strip_suffix(unit).map(|digits| {
                (
                    digits,
                    match unit {
                        "px" => 0.75,
                        "in" => 72.0,
                        "cm" => 72.0 / 2.54,
                        "mm" => 72.0 / 25.4,
                        "pc" => 12.0,
                        "q" => 72.0 / 101.6,
                        _ => 1.0,
                    },
                )
            })
        })
        .unwrap_or((&value, 1.0));
    number(Some(digits), fallback / factor) * factor
}
fn safe_html_link(value: &str) -> bool {
    let scheme = value.split_once(':').map(|(scheme, _)| {
        scheme
            .chars()
            .filter(|c| !c.is_ascii_whitespace() && !c.is_control())
            .collect::<String>()
            .to_ascii_lowercase()
    });
    scheme.is_none_or(|scheme| {
        matches!(
            scheme.as_str(),
            "http" | "https" | "mailto" | "tel" | "ftp" | "file"
        )
    })
}
fn html_escaped_text(out: &mut String, text: &str, limit: usize) -> Result<(), String> {
    let mut buffer = [0u8; 4];
    for ch in text.chars() {
        let escaped = match ch {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '\'' => "&apos;",
            '\n' | '\u{000b}' => "<br/>",
            _ => ch.encode_utf8(&mut buffer),
        };
        html_extend(out, escaped, limit)?;
    }
    Ok(())
}
fn html_extend(out: &mut String, text: &str, limit: usize) -> Result<(), String> {
    if text.len() > limit.saturating_sub(out.len()) {
        return Err("HTML превышает безопасный предел размера".into());
    }
    out.push_str(text);
    Ok(())
}
fn html_close_lists(
    out: &mut String,
    lists: &mut Vec<List>,
    depth: usize,
    limit: usize,
) -> Result<(), String> {
    while lists.len() > depth {
        if let Some(kind) = lists.pop() {
            html_extend(
                out,
                if kind == List::Number {
                    "</li></ol>"
                } else {
                    "</li></ul>"
                },
                limit,
            )?;
        }
    }
    Ok(())
}
#[cfg(test)]
fn write_html(doc: &Doc) -> String {
    match write_html_bounded(doc, XML_LIMIT) {
        Ok(html) => html,
        Err(error) => panic!("{error}"),
    }
}
fn write_html_bounded(doc: &Doc, limit: usize) -> Result<String, String> {
    let mut out = format!(
        "<!DOCTYPE html><html lang=\"ru\"><head><meta charset=\"utf-8\"/><meta name=\"author\" content=\"{}\"/><title>{}</title></head><body>",
        esc(&doc.author),
        esc(&doc.title)
    );
    if out.len() > limit {
        return Err("HTML превышает безопасный предел размера".into());
    }
    let mut lists = Vec::<List>::new();
    let mut numbers = [0usize; 9];
    for p in &doc.paragraphs {
        let depth = usize::from(p.list_level.min(8)) + 1;
        if p.list == List::None {
            html_close_lists(&mut out, &mut lists, 0, limit)?;
            numbers.fill(0);
        } else {
            let expected = numbers[depth - 1].saturating_add(1);
            let restart = p.list == List::Number
                && p.list_start.is_some_and(|start| start as usize != expected);
            let number = if p.list == List::Number {
                p.list_start.map(|start| start as usize).unwrap_or(expected)
            } else {
                0
            };
            numbers[depth - 1] = number;
            for value in &mut numbers[depth..] {
                *value = 0;
            }
            if lists.len() > depth {
                html_close_lists(&mut out, &mut lists, depth, limit)?;
            }
            if lists.len() == depth && (lists.last() != Some(&p.list) || restart) {
                html_close_lists(&mut out, &mut lists, depth - 1, limit)?;
            } else if lists.len() == depth {
                html_extend(&mut out, "</li>", limit)?;
            }
            while lists.len() < depth {
                let kind = if lists.len() + 1 == depth {
                    p.list
                } else {
                    List::Bullet
                };
                let open = if kind == List::Number {
                    format!("<ol start=\"{number}\">")
                } else {
                    "<ul>".into()
                };
                html_extend(&mut out, &open, limit)?;
                lists.push(kind);
                if lists.len() < depth {
                    html_extend(
                        &mut out,
                        "<li data-trinedocs-list-container=\"true\" style=\"list-style:none\">",
                        limit,
                    )?;
                }
            }
        }
        let tag = match (p.list, p.style.as_str()) {
            (List::Number | List::Bullet, _) => "li",
            (_, "Heading1") => "h1",
            (_, "Heading2") => "h2",
            (_, "Heading3") => "h3",
            (_, "Heading4") => "h4",
            (_, "Heading5") => "h5",
            (_, "Heading6") => "h6",
            (_, "Quote") => "blockquote",
            (_, "Code") => "pre",
            _ => "p",
        };
        let mut paragraph = html_paragraph_bounded(p, tag, limit)?;
        if p.list != List::None {
            paragraph.truncate(paragraph.len().saturating_sub(5));
        }
        html_extend(&mut out, &paragraph, limit)?;

        for table in doc.tables.iter().filter(|t| t.after_paragraph_id == p.id) {
            html_close_lists(&mut out, &mut lists, 0, limit)?;
            html_extend(
                &mut out,
                &format!(
                    "<table data-trinedocs-header-rows=\"{}\" style=\"border-collapse:collapse;table-layout:fixed\">",
                    table.header_rows
                ),
                limit,
            )?;
            if !table.column_widths.is_empty() {
                html_extend(&mut out, "<colgroup>", limit)?;
                for width in &table.column_widths {
                    html_extend(
                        &mut out,
                        &format!("<col style=\"width:{width}pt\"/>"),
                        limit,
                    )?;
                }
                html_extend(&mut out, "</colgroup>", limit)?;
            }
            let grouped = table.header_rows > 0
                && table.header_rows < table.rows.len()
                && !table.rows[table.header_rows].iter().any(|cell| cell.merged);
            if grouped {
                html_extend(&mut out, "<thead>", limit)?;
            }
            for (ri, row) in table.rows.iter().enumerate() {
                if grouped && ri == table.header_rows {
                    html_extend(&mut out, "</thead><tbody>", limit)?;
                }
                let cell_tag = if ri < table.header_rows { "th" } else { "td" };
                html_extend(&mut out, "<tr>", limit)?;
                let starts = table.grid_starts(ri);
                for (ci, cell) in row.iter().enumerate() {
                    if cell.merged && ri > 0 {
                        continue;
                    }
                    let col = starts[ci];
                    let mut rowspan = 1;
                    for next in ri + 1..table.rows.len() {
                        let positions = table.grid_starts(next);
                        let continuation =
                            table.rows[next]
                                .iter()
                                .zip(positions)
                                .any(|(below, start)| {
                                    start == col && below.span() == cell.span() && below.merged
                                });
                        if !continuation {
                            break;
                        }
                        rowspan += 1;
                    }
                    let border = if table.borderless {
                        "none".into()
                    } else {
                        format!("1px solid #{}", hex(table.border_color))
                    };
                    let padding = table.cell_margin.map(|v| format!("{v}pt")).join(" ");
                    let align = match cell.v_align {
                        VAlign::Top => "top",
                        VAlign::Center => "middle",
                        VAlign::Bottom => "bottom",
                    };
                    let height = table.row_heights.get(ri).copied().unwrap_or(0.0);
                    html_extend(
                        &mut out,
                        &format!(
                            "<{cell_tag} colspan=\"{}\" rowspan=\"{rowspan}\" style=\"vertical-align:{align};height:{height}pt;padding:{padding};border:{border};background:#{}\">",
                            cell.span(),
                            hex(cell.fill)
                        ),
                        limit,
                    )?;
                    for paragraph in &cell.paragraphs {
                        html_extend(
                            &mut out,
                            &html_paragraph_bounded(paragraph, "p", limit)?,
                            limit,
                        )?;
                    }
                    html_extend(&mut out, &format!("</{cell_tag}>"), limit)?;
                }
                html_extend(&mut out, "</tr>", limit)?;
            }
            if grouped {
                html_extend(&mut out, "</tbody>", limit)?;
            }
            html_extend(&mut out, "</table>", limit)?;
        }
        for image in doc
            .images
            .iter()
            .filter(|image| image.after_paragraph_id == p.id)
        {
            html_close_lists(&mut out, &mut lists, 0, limit)?;
            let encoded = image.bytes.len().div_ceil(3).saturating_mul(4);
            if encoded > limit.saturating_sub(out.len()) {
                return Err("HTML превышает безопасный предел размера".into());
            }
            let mime = if image.bytes.starts_with(b"\x89PNG") {
                "image/png"
            } else {
                "image/jpeg"
            };
            html_extend(
                &mut out,
                &format!(
                    "<img src=\"data:{mime};base64,{}\" alt=\"{}\" style=\"width:{}pt;height:{}pt\" data-trinedocs-width=\"{}\" data-trinedocs-height=\"{}\"/>",
                    encode_image_base64(&image.bytes),
                    esc(&image.alt),
                    image.width,
                    image.height,
                    image.width,
                    image.height
                ),
                limit,
            )?;
        }
    }
    html_close_lists(&mut out, &mut lists, 0, limit)?;
    html_extend(&mut out, "</body></html>", limit)?;
    Ok(out)
}
fn fallback_html_text(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut plain = String::new();
    let mut offset = 0;
    let mut hidden = Vec::<&str>::new();
    while offset < text.len() {
        let rest = &text[offset..];
        if rest.starts_with('<') {
            let Some(end) = rest.find('>') else { break };
            let tag = lower[offset + 1..offset + end].trim();
            let closing = tag.starts_with('/');
            let name = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_ascii_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            if closing && let Some(index) = hidden.iter().rposition(|open| *open == name) {
                hidden.truncate(index);
            } else if !closing && matches!(name, "script" | "style" | "head" | "template") {
                hidden.push(name);
            } else if hidden.is_empty() && matches!(name, "br" | "p" | "div" | "li" | "tr") {
                plain.push('\n');
            }
            offset += end + 1;
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            if hidden.is_empty() {
                plain.push_str(&rest[..end]);
            }
            offset += end;
        }
    }
    plain
}
fn html_hidden(node: &Node) -> bool {
    node.attrs.contains_key("hidden")
        || node.attr("style").is_some_and(|css| {
            css_declarations(css)
                .iter()
                .any(|(key, value)| key == "display" && value.trim().eq_ignore_ascii_case("none"))
        })
}
fn normalize_html_names(node: &mut Node) {
    node.name.make_ascii_lowercase();
    node.attrs = std::mem::take(&mut node.attrs)
        .into_iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value))
        .collect();
    for child in &mut node.children {
        normalize_html_names(child);
    }
}

fn read_html(text: &str) -> Result<Doc, String> {
    read_html_with_limit(text, XML_LIMIT)
}
fn read_html_with_limit(text: &str, limit: usize) -> Result<Doc, String> {
    if text.len() > limit {
        return Err("HTML превышает безопасный лимит размера".into());
    }
    // HTML is imported as data only; scripts, CSS imports and network images are never executed.
    let mut clean = normalize_html_optional_tags(&normalize_html_voids(
        &normalize_html_attributes(&normalize_html_entities(text)),
    ))?;
    if let Some(start) = clean.to_ascii_lowercase().find("<!doctype")
        && let Some(end) = clean.get(start..).and_then(|s| s.find('>'))
    {
        clean.replace_range(start..=start + end, "");
    }
    if clean.trim_start().starts_with("<?xml")
        && let Some(end) = clean.find("?>")
    {
        clean.replace_range(..end + 2, "");
    }
    let wrapped = format!("<trinedocs-html-root>{clean}</trinedocs-html-root>");
    let mut node = xml(wrapped.as_bytes()).or_else(|_| {
        let plain = fallback_html_text(text);
        let plain = quick_xml::escape::unescape(&plain)
            .map_err(|e| e.to_string())?
            .into_owned();
        Ok::<Node, String>(Node {
            name: "p".into(),
            text: plain,
            ..Node::default()
        })
    })?;
    let mut doc = Doc::default();
    doc.paragraphs.clear();
    fn walk(
        node: &Node,
        doc: &mut Doc,
        inherited_css: &str,
        list: Option<(List, u8)>,
    ) -> Result<(), String> {
        if html_hidden(node)
            || matches!(node.name.as_str(), "script" | "style" | "head" | "template")
        {
            return Ok(());
        }
        if node.name == "img" {
            if let Some(bytes) = node.attr("src").and_then(decode_html_image)
                && doc.images.len() < 1000
                && let Ok(image) = crate::util::decode_image(&bytes)
            {
                if doc.paragraphs.is_empty() {
                    let id = doc.alloc_id();
                    doc.paragraphs.push(Paragraph {
                        id,
                        ..Paragraph::default()
                    });
                }
                let anchor = doc.paragraphs.last().map(|p| p.id).unwrap_or_default();
                let (width, height) = html_image_geometry(
                    node,
                    image.width() as f32 * 0.75,
                    image.height() as f32 * 0.75,
                );
                let id = doc.alloc_id();
                doc.images.push(AnchoredImage {
                    id,
                    after_paragraph_id: anchor,
                    bytes,
                    width,
                    height,
                    alt: node.attr("alt").unwrap_or("").into(),
                });
            } else {
                doc.losses.push("HTML: изображение пропущено — неподдерживаемый источник, повреждённые данные или превышен лимит.".into());
            }
            return Ok(());
        }
        let inherited = inherited_html_css(inherited_css);
        let css = html_computed_css(&inherited, node.attr("style").unwrap_or_default());
        let list = match node.name.as_str() {
            "ol" | "ul" => Some((
                if node.name == "ol" {
                    List::Number
                } else {
                    List::Bullet
                },
                list.map(|(_, depth)| depth.saturating_add(1).min(8))
                    .unwrap_or(0),
            )),
            _ => list,
        };
        if node.name == "li" && node.attr("data-trinedocs-list-container") == Some("true") {
            for child in &node.children {
                walk(child, doc, &css, list)?;
            }
            return Ok(());
        }
        if node.name == "ol" {
            let reversed = node.attrs.contains_key("reversed");
            let mut ordinal = node
                .attr("start")
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|value| *value <= 999_999_999)
                .unwrap_or_else(|| {
                    if reversed {
                        node.children
                            .iter()
                            .filter(|child| child.name == "li")
                            .count() as u32
                    } else {
                        1
                    }
                });
            for child in &node.children {
                if child.name == "li" {
                    if let Some(value) = child
                        .attr("value")
                        .and_then(|value| value.parse::<u32>().ok())
                        .filter(|value| *value <= 999_999_999)
                    {
                        ordinal = value;
                    }
                    let start = doc.paragraphs.len();
                    walk(child, doc, &css, list)?;
                    if let Some(paragraph) =
                        doc.paragraphs.iter_mut().skip(start).find(|paragraph| {
                            paragraph.list == List::Number
                                && Some(paragraph.list_level) == list.map(|(_, level)| level)
                        })
                    {
                        paragraph.list_start = Some(ordinal);
                    }
                    ordinal = if reversed {
                        ordinal.saturating_sub(1)
                    } else {
                        ordinal.saturating_add(1).min(999_999_999)
                    };
                } else {
                    walk(child, doc, &css, list)?;
                }
            }
            return Ok(());
        }
        if node.name == "table" {
            html_import_table(node, doc, &css, walk)?;
            return Ok(());
        }
        if node.name == "li"
            && node.children.iter().any(|child| {
                matches!(
                    child.name.as_str(),
                    "p" | "div" | "pre" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote"
                )
            })
        {
            html_import_list_item(node, doc, &css, list, walk)?;
            return Ok(());
        }
        let block = matches!(
            node.name.as_str(),
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "li" | "pre"
        );
        if block {
            let id = doc.alloc_id();
            let mut p = Paragraph {
                id,
                runs: Vec::new(),
                ..Paragraph::default()
            };
            fn runs(
                n: &Node,
                f: &TextFormat,
                out: &mut Vec<Run>,
                preserve: bool,
                pending: &mut Option<TextFormat>,
            ) {
                if html_hidden(n)
                    || matches!(
                        n.name.as_str(),
                        "script" | "style" | "template" | "ul" | "ol"
                    )
                {
                    return;
                }
                let mut fmt = f.clone();
                let mut preserve = preserve || n.name == "pre";
                match n.name.as_str() {
                    "pre" | "code" | "kbd" | "samp" | "tt" => fmt.font = "Consolas".into(),
                    "mark" => fmt.highlight = Some([255, 255, 0]),
                    "small" => fmt.size = (fmt.size * 0.8333333).max(4.0),
                    "big" => fmt.size = (fmt.size * 1.2).min(144.0),
                    "font" => {
                        if let Some(color) = n.attr("color").and_then(css_color) {
                            fmt.color = color;
                        }
                        if let Some(font) = n.attr("face").and_then(css_font_family) {
                            fmt.font = font;
                        }
                        if let Some(size) = n
                            .attr("size")
                            .and_then(|size| size.parse::<usize>().ok())
                            .filter(|size| (1..=7).contains(size))
                        {
                            fmt.size = [8.0, 10.0, 12.0, 14.0, 18.0, 24.0, 36.0][size - 1];
                        }
                    }
                    "b" | "strong" => fmt.bold = true,
                    "i" | "em" => fmt.italic = true,
                    "u" | "ins" => fmt.underline = true,
                    "s" | "del" | "strike" => fmt.strike = true,
                    "sup" => fmt.script = 1,
                    "sub" => fmt.script = -1,
                    "a" => fmt.link = n.attr("href").map(str::to_string),
                    _ => {}
                }
                if let Some(css) = n.attr("style") {
                    for (key, value) in css_declarations(css) {
                        let lower = value.to_ascii_lowercase();
                        match key.as_str() {
                            "white-space" => {
                                preserve =
                                    matches!(lower.trim(), "pre" | "pre-wrap" | "break-spaces")
                            }
                            "font-size" => fmt.size = html_font_size(&value, f.size),
                            "font-family" => {
                                if let Some(family) = css_font_family(&value) {
                                    fmt.font = family;
                                }
                            }
                            "color" => {
                                if let Some(c) = css_color(&value) {
                                    fmt.color = c;
                                }
                            }
                            "background-color" | "background" => {
                                if lower.trim() == "transparent" || lower.trim() == "initial" {
                                    fmt.highlight = None;
                                } else if let Some(color) = css_color(&value) {
                                    fmt.highlight = Some(color);
                                }
                            }
                            "font-weight" => {
                                fmt.bold = match lower.trim() {
                                    "bold" | "bolder" => true,
                                    "normal" | "lighter" => false,
                                    value => value
                                        .parse::<u16>()
                                        .ok()
                                        .filter(|weight| (1..=1000).contains(weight))
                                        .map(|weight| weight >= 600)
                                        .unwrap_or(fmt.bold),
                                };
                            }
                            "font-style" => {
                                fmt.italic = matches!(lower.trim(), "italic" | "oblique")
                            }
                            "text-decoration" | "text-decoration-line" => {
                                fmt.underline |=
                                    lower.split_whitespace().any(|word| word == "underline");
                                fmt.strike |=
                                    lower.split_whitespace().any(|word| word == "line-through");
                            }
                            "vertical-align" => {
                                fmt.script = match lower.trim() {
                                    "super" => 1,
                                    "sub" => -1,
                                    "baseline" => 0,
                                    _ => fmt.script,
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if !n.text.is_empty() {
                    html_inline_text(&n.text, &fmt, preserve, out, pending);
                }
                for c in &n.children {
                    if html_hidden(c) {
                        continue;
                    }
                    if c.name == "br" {
                        *pending = None;
                        out.push(Run {
                            text: "\u{000b}".into(),
                            format: fmt.clone(),
                        });
                    } else {
                        runs(c, &fmt, out, preserve, pending);
                    }
                }
            }
            let style = match node.name.as_str() {
                "h1" => Some("Heading1"),
                "h2" => Some("Heading2"),
                "h3" => Some("Heading3"),
                "h4" => Some("Heading4"),
                "h5" => Some("Heading5"),
                "h6" => Some("Heading6"),
                "blockquote" => Some("Quote"),
                "pre" => Some("Code"),
                _ => None,
            };
            p.runs.push(Run::default());
            if let Some(style) = style {
                p.set_style(style);
            }
            let default_format = p.format_at(0);
            p.runs.clear();
            let mut styled = node.clone();
            styled.attrs.insert("style".into(), css.clone());
            runs(&styled, &default_format, &mut p.runs, false, &mut None);
            if node.name == "li" {
                p.list = list.map(|(kind, _)| kind).unwrap_or(List::Bullet);
                p.list_level = list.map(|(_, depth)| depth).unwrap_or(0);
            }
            for (key, value) in css_declarations(&css) {
                let lower = value.to_ascii_lowercase();
                match key.as_str() {
                    "text-align" => {
                        p.align = match lower.trim() {
                            "center" => Align::Center,
                            "right" => Align::Right,
                            "justify" => Align::Justify,
                            _ => Align::Left,
                        }
                    }
                    "margin-left" => p.indent = css_points(&value, 0.0),
                    "margin-top" => p.spacing_before = css_points(&value, 0.0),
                    "margin-bottom" => p.spacing_after = css_points(&value, 0.0),
                    "text-indent" => p.first_indent = css_points(&value, 0.0),
                    "margin-right" => p.right_indent = css_points(&value, 0.0),
                    "break-before" | "page-break-before" => {
                        p.page_break_before = lower.trim() == "page" || lower.trim() == "always"
                    }
                    "break-after" | "page-break-after" => p.keep_next = lower.trim() == "avoid",
                    "break-inside" | "page-break-inside" => p.keep_lines = lower.trim() == "avoid",
                    "widows" => p.widow_control = lower.trim() != "1",
                    "line-height" => {
                        if ["pt", "px", "in", "cm", "mm", "pc", "q"]
                            .iter()
                            .any(|unit| lower.trim().ends_with(unit))
                        {
                            p.line_points = css_points(&value, 0.0);
                            p.line_exact = true;
                        } else if let Some(percent) = lower
                            .trim()
                            .strip_suffix('%')
                            .and_then(|number| number.trim().parse::<f32>().ok())
                            .filter(|number| number.is_finite() && *number > 0.0)
                        {
                            let size = p
                                .runs
                                .iter()
                                .map(|run| run.format.size)
                                .fold(0.0_f32, f32::max)
                                .max(4.0);
                            p.line_points = (size * percent / 100.0).clamp(1.0, 1584.0);
                            p.line_exact = true;
                        } else if let Some(em) = lower
                            .trim()
                            .strip_suffix("em")
                            .and_then(|number| number.trim().parse::<f32>().ok())
                            .filter(|number| number.is_finite() && *number > 0.0)
                        {
                            let size = p
                                .runs
                                .iter()
                                .map(|run| run.format.size)
                                .fold(0.0_f32, f32::max)
                                .max(4.0);
                            p.line_points = (size * em).clamp(1.0, 1584.0);
                            p.line_exact = true;
                        } else {
                            p.line_spacing =
                                number(Some(value.trim()), p.line_spacing).clamp(0.5, 5.0);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(level) = node
                .attr("data-trinedocs-list-level")
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|v| *v <= 8)
            {
                p.list_level = level;
                if p.list != List::None {
                    p.indent -= f32::from(level) * 18.0;
                }
            }
            if let Some(spacing) = node
                .attr("data-trinedocs-line-spacing")
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
            {
                p.line_spacing = spacing;
            }
            if let Some(exact) = node.attr("data-trinedocs-line-exact") {
                p.line_exact = exact == "true";
            }
            p.normalize();
            doc.paragraphs.push(p);
            fn inline_images(
                node: &Node,
                doc: &mut Doc,
                css: &str,
                list: Option<(List, u8)>,
            ) -> Result<(), String> {
                if html_hidden(node)
                    || matches!(
                        node.name.as_str(),
                        "script" | "style" | "template" | "ul" | "ol"
                    )
                {
                    return Ok(());
                }
                if node.name == "img" {
                    walk(node, doc, css, list)?;
                } else {
                    for child in &node.children {
                        inline_images(child, doc, css, list)?;
                    }
                }
                Ok(())
            }
            for child in &node.children {
                inline_images(child, doc, &css, list)?;
            }
            for child in node
                .children
                .iter()
                .filter(|n| matches!(n.name.as_str(), "ol" | "ul"))
            {
                walk(child, doc, &css, list)?;
            }
        } else {
            fn boundary(node: &Node) -> bool {
                matches!(
                    node.name.as_str(),
                    "p" | "pre"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "blockquote"
                        | "li"
                        | "div"
                        | "section"
                        | "article"
                        | "html"
                        | "body"
                        | "img"
                        | "table"
                        | "thead"
                        | "tbody"
                        | "tfoot"
                        | "tr"
                        | "td"
                        | "th"
                        | "ul"
                        | "ol"
                        | "head"
                        | "script"
                        | "style"
                ) || node.children.iter().any(boundary)
            }
            fn flush(inline: &mut Vec<Node>, doc: &mut Doc, css: &str) -> Result<(), String> {
                let paragraph = Node {
                    name: "p".into(),
                    attrs: BTreeMap::from([("style".into(), css.into())]),
                    children: std::mem::take(inline),
                    ..Node::default()
                };
                // Formatting whitespace between blocks must not add blank paragraphs.
                if !paragraph.text_content().trim().is_empty()
                    || paragraph.children.iter().any(|n| n.name == "br")
                {
                    walk(&paragraph, doc, "", None)?;
                }
                Ok(())
            }
            let mut inline = Vec::new();
            for child in &node.children {
                if boundary(child) {
                    flush(&mut inline, doc, &css)?;
                    walk(child, doc, &css, list)?;
                } else {
                    inline.push(child.clone());
                }
            }
            flush(&mut inline, doc, &css)?;
        }
        Ok(())
    }
    normalize_html_names(&mut node);
    if let Some(title) = node.descendant("title") {
        doc.title = title.text_content();
    }
    let mut metadata = Vec::new();
    node.descendants("meta", &mut metadata);
    if let Some(author) = metadata
        .iter()
        .find(|meta| {
            meta.attr("name")
                .is_some_and(|name| name.eq_ignore_ascii_case("author"))
        })
        .and_then(|meta| meta.attr("content"))
    {
        doc.author = author.into();
    }
    walk(&node, &mut doc, "", None)?;
    doc.losses.push(
        "HTML: сложные CSS-макеты, внешние изображения и часть оформления/вложенности таблиц не поддерживаются.".into(),
    );
    doc.normalize();
    Ok(doc)
}
#[cfg(test)]
mod tests {
    #[test]
    fn review5_separate_docx_numbered_lists_get_independent_instances() -> Result<(), String> {
        let mut doc = Doc::from_text("first\nsecond\nseparator\nnew first");
        for index in [0, 1, 3] {
            doc.paragraphs[index].list = List::Number;
        }
        let parts = read_zip(&write(&doc, "docx")?)?;
        let root = xml(parts.get("word/document.xml").ok_or("document")?)?;
        let paragraphs = root
            .child("body")
            .ok_or("body")?
            .children
            .iter()
            .filter(|node| node.name == "p")
            .collect::<Vec<_>>();
        let num_id = |index: usize| paragraphs[index].descendant("numId").and_then(Node::value);
        assert_eq!(num_id(0), num_id(1));
        assert_ne!(num_id(0), num_id(3));
        let numbering = xml(parts.get("word/numbering.xml").ok_or("numbering")?)?;
        let new_list = numbering
            .children
            .iter()
            .find(|node| node.name == "num" && node.attr("numId") == num_id(3))
            .ok_or("new list")?;
        assert_eq!(
            new_list
                .child("lvlOverride")
                .and_then(|node| node.child("startOverride"))
                .and_then(Node::value),
            Some("1")
        );
        Ok(())
    }
    #[test]
    fn review5_nested_bullets_preserve_parent_counts_and_reset_child_instances() {
        let mut doc = Doc::from_text("parent\nchild\nbullet child\nparent two\nchild one");
        for (paragraph, (list, level)) in doc.paragraphs.iter_mut().zip([
            (List::Number, 0),
            (List::Number, 1),
            (List::Bullet, 1),
            (List::Number, 0),
            (List::Number, 1),
        ]) {
            paragraph.list = list;
            paragraph.list_level = level;
        }
        let export = ExportNumbering::of(&doc);
        let second = export.paragraphs[&doc.paragraphs[3].id] - 2;
        assert_eq!(export.starts[second][0], 2);
        assert_eq!(export.starts[second][1], 1);
        assert_eq!(
            export.paragraphs[&doc.paragraphs[3].id],
            export.paragraphs[&doc.paragraphs[4].id]
        );
    }

    #[test]
    fn review5_utf8_bom_refuses_broken_utf8_instead_of_guessing_legacy() {
        assert!(decode_text(&[0xEF, 0xBB, 0xBF, 0xD0]).is_err());
        assert_eq!(decode_text(&[0xD0]).ok().as_deref(), Some("Р"));
        assert_eq!(
            decode_text(&[0xEF, 0xBB, 0xBF, b'A']).ok().as_deref(),
            Some("A")
        );
    }
    #[test]
    fn review5_deep_content_controls_keep_visible_text() -> Result<(), String> {
        let nested = format!(
            "{}<w:p><w:r><w:t>Deep visible text</w:t></w:r></w:p>{}",
            "<w:sdt><w:sdtContent>".repeat(40),
            "</w:sdtContent></w:sdt>".repeat(40)
        );
        let document =
            format!("<w:document xmlns:w=\"urn:w\"><w:body>{nested}</w:body></w:document>");
        let doc = read(
            &write_zip(vec![("word/document.xml".into(), document.into_bytes())])?,
            "docx",
        )?;
        assert_eq!(doc.plain_text(), "Deep visible text");
        Ok(())
    }
    #[test]
    fn review5_style_chains_keep_deep_inheritance_and_refuse_cycles() -> Result<(), String> {
        let mut styles = BTreeMap::new();
        for index in 0..20 {
            let base = if index == 0 {
                String::new()
            } else {
                format!("<w:basedOn w:val=\"S{}\"/>", index - 1)
            };
            let props = if index == 0 {
                "<w:rPr><w:color w:val=\"123456\"/></w:rPr>"
            } else {
                ""
            };
            styles.insert(
                format!("S{index}"),
                xml(format!("<w:style xmlns:w=\"urn:w\">{base}{props}</w:style>").as_bytes())?,
            );
        }
        let mut chain = Vec::new();
        style_chain(&styles, "S19", &mut chain)?;
        assert_eq!(chain.len(), 20);
        let mut format = TextFormat::default();
        for style in chain {
            if let Some(props) = style.child("rPr") {
                apply_style_rpr(&mut format, props);
            }
        }
        assert_eq!(format.color, [0x12, 0x34, 0x56]);
        styles.insert(
            "S0".into(),
            xml(br#"<w:style xmlns:w="urn:w"><w:basedOn w:val="S19"/></w:style>"#)?,
        );
        assert!(style_chain(&styles, "S19", &mut Vec::new()).is_err());
        Ok(())
    }
    #[test]
    fn review5_markdown_keeps_a_heading_or_quote_combined_with_a_list() -> Result<(), String> {
        for style in ["Heading2", "Quote"] {
            for list in [List::Number, List::Bullet] {
                let mut doc = Doc::from_text("Combined");
                doc.paragraphs[0].set_style(style);
                doc.paragraphs[0].list = list;
                let again = read(&write(&doc, "md")?, "md")?;
                assert_eq!(again.paragraphs[0].style, style);
                assert_eq!(again.paragraphs[0].list, list);
                assert_eq!(again.plain_text(), "Combined");
            }
        }
        Ok(())
    }

    #[test]
    fn review5_docx_automatic_color_overrides_the_inherited_color() -> Result<(), String> {
        let mut format = TextFormat {
            color: [200, 0, 0],
            highlight: Some([255, 255, 0]),
            ..Default::default()
        };
        apply_rpr(
            &mut format,
            &xml(br#"<w:rPr xmlns:w="urn:w"><w:color w:val="auto"/></w:rPr>"#)?,
        );
        // Word's automatic ink is black, not the inherited red.
        assert_eq!(format.color, [0, 0, 0]);
        assert_eq!(format.highlight, Some([255, 255, 0]));
        Ok(())
    }
    #[test]
    fn review5_docx_style_toggles_are_relative_and_direct_format_is_absolute() -> Result<(), String>
    {
        let properties = xml(br#"<w:rPr xmlns:w="urn:w"><w:b/><w:i/><w:strike/></w:rPr>"#)?;
        let mut format = TextFormat::default();
        apply_style_rpr(&mut format, &properties);
        assert!(format.bold && format.italic && format.strike);
        apply_style_rpr(&mut format, &properties);
        assert!(!format.bold && !format.italic && !format.strike);
        apply_rpr(&mut format, &properties);
        apply_rpr(&mut format, &properties);
        assert!(format.bold && format.italic && format.strike);
        Ok(())
    }
    #[test]
    fn review5_docx_export_uses_each_numbering_level_and_explicit_orientation() -> Result<(), String>
    {
        let mut doc = Doc::from_text("landscape");
        (doc.page.width, doc.page.height) = (841.89, 595.28);
        let parts = read_zip(&write(&doc, "docx")?)?;
        let numbering = xml(parts.get("word/numbering.xml").ok_or("numbering")?)?;
        let decimal = numbering
            .children
            .iter()
            .find(|node| node.name == "abstractNum" && node.attr("abstractNumId") == Some("2"))
            .ok_or("decimal")?;
        for (index, level) in decimal
            .children
            .iter()
            .filter(|node| node.name == "lvl")
            .enumerate()
        {
            assert_eq!(
                level.child("lvlText").and_then(Node::value),
                Some(format!("%{}.", index + 1).as_str())
            );
        }
        let document = xml(parts.get("word/document.xml").ok_or("document")?)?;
        assert_eq!(
            document
                .descendant("pgSz")
                .and_then(|node| node.attr("orient")),
            Some("landscape")
        );
        Ok(())
    }
    #[test]
    fn review5_empty_formatted_markdown_paragraph_does_not_create_visible_markers()
    -> Result<(), String> {
        let mut doc = Doc::from_text("");
        let format = &mut doc.paragraphs[0].runs[0].format;
        format.bold = true;
        format.italic = true;
        format.strike = true;
        format.link = Some("https://example.org".into());
        assert_eq!(
            String::from_utf8(write(&doc, "md")?).map_err(|error| error.to_string())?,
            "\n"
        );
        assert_eq!(read(&write(&doc, "md")?, "md")?.plain_text(), "");
        Ok(())
    }
    #[test]
    fn review4_docx_normalizes_crlf_once_and_refuses_xml_controls() -> Result<(), String> {
        let mut mixed = Doc::from_text("a");
        mixed.paragraphs[0].runs[0].text = "a\r\nb\rc".into();
        let again = read(&write(&mixed, "docx")?, "docx")?;
        assert_eq!(again.paragraphs[0].text(), "a\u{000b}b\u{000b}c");
        mixed.paragraphs[0].runs[0].text = "a\u{001b}b".into();
        assert!(write(&mixed, "docx").is_err());
        assert_eq!(mixed.paragraphs[0].text(), "a\u{001b}b");
        Ok(())
    }

    #[test]
    fn many_short_valid_markdown_spans_and_links_keep_all_formatting() {
        let bold = inline_md(&"**x** ".repeat(1000), &TextFormat::default());
        assert_eq!(
            bold.iter().map(|run| run.text.as_str()).collect::<String>(),
            "x ".repeat(1000)
        );
        assert_eq!(
            bold.iter()
                .filter(|run| run.format.bold && run.text == "x")
                .count(),
            1000
        );
        assert!(
            bold.iter()
                .filter(|run| !run.text.trim().is_empty())
                .all(|run| run.format.bold)
        );

        let links = inline_md(
            &"[x](https://example.com) ".repeat(1000),
            &TextFormat::default(),
        );
        assert_eq!(
            links
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            "x ".repeat(1000)
        );
        assert_eq!(links.iter().filter(|run| run.text == "x").count(), 1000);
        assert!(
            links
                .iter()
                .filter(|run| !run.text.trim().is_empty())
                .all(|run| { run.format.link.as_deref() == Some("https://example.com") })
        );
    }

    #[test]
    fn malformed_markdown_links_have_a_shared_work_budget_and_keep_literal_text() {
        for text in [
            "[".repeat(100_000),
            format!("{}]({}", "[".repeat(100_000), "x".repeat(100_000)),
            "&".repeat(100_000),
        ] {
            let format = TextFormat {
                italic: true,
                ..TextFormat::default()
            };
            let runs = inline_md(&text, &format);
            assert_eq!(
                runs.iter().map(|run| run.text.as_str()).collect::<String>(),
                text
            );
            assert!(runs.iter().all(|run| run.format.italic));
        }
    }

    #[test]
    fn markdown_literal_trailing_spaces_do_not_become_a_paragraph_join() -> Result<(), String> {
        let doc = Doc::from_text("trailing  \nnext & literal &#32;");
        let again = read(&write(&doc, "md")?, "md")?;
        assert_eq!(again.plain_text(), doc.plain_text());
        assert_eq!(again.paragraphs.len(), 2);
        Ok(())
    }

    #[test]
    fn markdown_literal_syntax_and_combined_formats_survive_own_export() -> Result<(), String> {
        let literal = r"# []()*_! a_b_c ~ \ path";
        let mut doc = Doc::from_text(literal);
        doc.paragraphs[0].runs[0].format.bold = true;
        doc.paragraphs[0].runs[0].format.italic = true;
        doc.paragraphs[0].runs[0].format.strike = true;
        let again = read(&write(&doc, "md")?, "md")?;
        assert_eq!(again.plain_text(), literal);
        assert!(
            again.paragraphs[0]
                .runs
                .iter()
                .all(|run| { run.format.bold && run.format.italic && run.format.strike })
        );
        let plain = read(&write(&Doc::from_text(literal), "md")?, "md")?;
        assert_eq!(plain.plain_text(), literal);
        assert!(
            plain.paragraphs[0]
                .runs
                .iter()
                .all(|run| { !run.format.bold && !run.format.italic && !run.format.strike })
        );
        Ok(())
    }
    #[test]
    fn markdown_links_and_hard_breaks_survive_own_export() -> Result<(), String> {
        let mut doc = Doc::from_text("before\u{000b}after [label]");
        let target = "https://example.com/a_(b)";
        doc.paragraphs[0].runs[0].format.link = Some(target.into());
        let again = read(&write(&doc, "md")?, "md")?;
        assert_eq!(again.paragraphs.len(), 1);
        assert_eq!(again.plain_text(), doc.plain_text());
        assert!(
            again.paragraphs[0]
                .runs
                .iter()
                .all(|r| r.format.link.as_deref() == Some(target))
        );
        Ok(())
    }
    #[test]
    fn markdown_heading_inherits_the_style_without_overwriting_inline_formats() -> Result<(), String>
    {
        let doc = read(b"# heading *italic* ~~strike~~", "md")?;
        assert_eq!(doc.paragraphs[0].style, "Heading1");
        assert!(doc.paragraphs[0].runs.iter().all(|run| run.format.bold));
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "italic" && run.format.italic)
        );
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "strike" && run.format.strike)
        );
        Ok(())
    }
    #[test]
    fn docx_table_cell_content_controls_keep_visible_paragraphs() -> Result<(), String> {
        let document = r#"<w:document xmlns:w="urn:w"><w:body><w:p/><w:tbl><w:tr><w:tc><w:sdt><w:sdtContent><w:p><w:r><w:t>Cell content</w:t></w:r></w:p></w:sdtContent></w:sdt></w:tc></w:tr></w:tbl></w:body></w:document>"#;
        let bytes = write_zip(vec![(
            "word/document.xml".into(),
            document.as_bytes().to_vec(),
        )])?;
        let doc = read(&bytes, "docx")?;
        assert_eq!(
            doc.tables[0].rows[0][0].paragraphs[0].text(),
            "Cell content"
        );
        Ok(())
    }
    #[test]
    fn docx_drawing_collection_uses_only_the_visible_revision_and_alternate_branch()
    -> Result<(), String> {
        let node = xml(br#"<p xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><del><drawing><blip/></drawing></del><moveFrom><drawing/></moveFrom><ins><drawing id="kept"/></ins><AlternateContent><Choice Requires="w"><drawing id="choice"/></Choice><Fallback><drawing id="fallback"/></Fallback></AlternateContent></p>"#)?;
        let mut drawings = Vec::new();
        visible_drawings(&node, &mut drawings);
        assert_eq!(drawings.len(), 2);
        assert_eq!(drawings[0].attr("id"), Some("kept"));
        assert_eq!(drawings[1].attr("id"), Some("choice"));
        Ok(())
    }
    #[test]
    fn docx_header_keeps_non_page_field_results_and_finds_all_page_fields() -> Result<(), String> {
        let document = r#"<w:document xmlns:w="urn:w" xmlns:r="urn:r"><w:body><w:p/><w:sectPr><w:headerReference w:type="default" r:id="h"/></w:sectPr></w:body></w:document>"#;
        let relationships =
            r#"<Relationships><Relationship Id="h" Target="header1.xml"/></Relationships>"#;
        let header = r#"<w:hdr xmlns:w="urn:w"><w:p><w:fldSimple w:instr="AUTHOR"><w:r><w:t>Alice</w:t></w:r></w:fldSimple><w:r><w:fldChar w:fldCharType="begin"/><w:instrText>DATE</w:instrText><w:fldChar w:fldCharType="separate"/><w:t>2026</w:t><w:fldChar w:fldCharType="end"/></w:r><w:r><w:fldChar w:fldCharType="begin"/><w:instrText>PAGE \* MERGEFORMAT</w:instrText><w:fldChar w:fldCharType="separate"/><w:t>9</w:t><w:fldChar w:fldCharType="end"/></w:r></w:p></w:hdr>"#;
        let bytes = write_zip(vec![
            ("word/document.xml".into(), document.as_bytes().to_vec()),
            (
                "word/_rels/document.xml.rels".into(),
                relationships.as_bytes().to_vec(),
            ),
            ("word/header1.xml".into(), header.as_bytes().to_vec()),
        ])?;
        let doc = read(&bytes, "docx")?;
        assert_eq!(doc.page.header, "Alice2026");
        assert!(doc.page.page_numbers);
        Ok(())
    }
    #[test]
    fn html_case_and_hidden_nodes_do_not_change_visible_content() -> Result<(), String> {
        let doc = read(br#"<HTML><BODY><P>before <B>bold</B><SPAN HIDDEN="hidden">secret</SPAN><SPAN STYLE="DISPLAY: none !important">secret</SPAN> after</P><P HIDDEN="hidden">secret</P></BODY></HTML>"#, "html")?;
        assert_eq!(doc.plain_text(), "before bold after");
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "bold" && run.format.bold)
        );
        Ok(())
    }

    #[test]
    fn supported_heading_levels_survive_html_markdown_and_docx() -> Result<(), String> {
        for level in 1..=6 {
            let html = format!("<h{level}>heading</h{level}>");
            let doc = read(html.as_bytes(), "html")?;
            let p = doc.paragraphs.first().ok_or("heading")?;
            assert_eq!(p.style, format!("Heading{level}"));
            assert!(p.runs.first().is_some_and(|run| run.format.bold));
            if level == 1 {
                assert!(p.runs[0].format.size > 11.0);
            }
            for extension in ["docx", "md"] {
                let again = read(&write(&doc, extension)?, extension)?;
                assert_eq!(again.paragraphs[0].style, format!("Heading{level}"));
            }
        }
        let doc = read(
            b"<h1 style=\"font-size:18px;font-weight:normal\">custom</h1>",
            "html",
        )?;
        assert_eq!(doc.paragraphs[0].runs[0].format.size, 13.5);
        assert!(!doc.paragraphs[0].runs[0].format.bold);
        Ok(())
    }
    #[test]
    fn docx_character_style_inheritance_preserves_hyphen_characters() -> Result<(), String> {
        let document = r#"<w:document xmlns:w="urn:w"><w:body><w:p><w:r><w:rPr><w:rStyle w:val="Child"/></w:rPr><w:t>a</w:t><w:noBreakHyphen/><w:t>b</w:t><w:softHyphen/><w:t>c</w:t></w:r></w:p></w:body></w:document>"#;
        let styles = r#"<w:styles xmlns:w="urn:w"><w:style w:type="character" w:styleId="Base"><w:rPr><w:b/><w:color w:val="112233"/></w:rPr></w:style><w:style w:type="character" w:styleId="Child"><w:basedOn w:val="Base"/><w:rPr><w:i/></w:rPr></w:style></w:styles>"#;
        let bytes = write_zip(vec![
            ("word/document.xml".into(), document.as_bytes().to_vec()),
            ("word/styles.xml".into(), styles.as_bytes().to_vec()),
        ])?;
        let doc = read(&bytes, "docx")?;
        assert_eq!(doc.paragraphs[0].text(), "a\u{2011}b\u{00ad}c");
        let f = &doc.paragraphs[0].runs[0].format;
        assert!(f.bold && f.italic);
        assert_eq!(f.color, [0x11, 0x22, 0x33]);
        Ok(())
    }
    #[test]
    fn partial_numbering_properties_keep_the_inherited_level() -> Result<(), String> {
        let mut p = Paragraph {
            list: List::Number,
            list_level: 3,
            ..Paragraph::default()
        };
        let node =
            xml(br#"<w:pPr xmlns:w="urn:w"><w:numPr><w:numId w:val="2"/></w:numPr></w:pPr>"#)?;
        apply_ppr(&mut p, &node, &BTreeMap::from([("2".into(), List::Number)]));
        assert_eq!((p.list, p.list_level), (List::Number, 3));
        let cancel =
            xml(br#"<w:pPr xmlns:w="urn:w"><w:numPr><w:numId w:val="0"/></w:numPr></w:pPr>"#)?;
        apply_ppr(&mut p, &cancel, &BTreeMap::new());
        assert_eq!((p.list, p.list_level), (List::None, 0));
        Ok(())
    }
    #[test]
    fn repeated_markdown_roundtrips_do_not_accumulate_empty_paragraphs() -> Result<(), String> {
        let mut doc = Doc::from_text("one\n\ntwo\n");
        let expected = doc.plain_text();
        for _ in 0..3 {
            doc = read(&write(&doc, "md")?, "md")?;
            assert_eq!(doc.plain_text(), expected);
        }
        Ok(())
    }
    #[test]
    fn docx_header_footer_preserve_edge_spaces_for_office_readers() -> Result<(), String> {
        let mut doc = Doc::from_text("body");
        doc.page.header = "  header  ".into();
        doc.page.footer = " footer ".into();
        let bytes = write(&doc, "docx")?;
        let parts = read_zip(&bytes)?;
        for path in ["word/header1.xml", "word/footer1.xml"] {
            let root = xml(parts.get(path).ok_or("part")?)?;
            assert_eq!(
                root.descendant("t").and_then(|node| node.attr("space")),
                Some("preserve")
            );
        }
        let again = read(&bytes, "docx")?;
        assert_eq!(again.page.header, doc.page.header);
        assert_eq!(again.page.footer, doc.page.footer);
        Ok(())
    }

    #[test]
    fn html_lists_paragraph_geometry_and_scripts_round_trip() -> Result<(), String> {
        let mut doc = Doc::from_text("one\ntwo");
        for p in &mut doc.paragraphs {
            p.list = List::Number;
            p.align = Align::Right;
            p.indent = 24.0;
            p.line_spacing = 1.5;
            p.runs[0].format.script = 1;
        }
        let html = write_html(&doc);
        assert!(html.contains("<ol start=\"1\">"));
        assert!(html.contains("<sup>"));
        let loaded = read_html(&html)?;
        assert_eq!(loaded.plain_text(), "one\ntwo");
        assert!(loaded.paragraphs.iter().all(|p| p.list == List::Number
            && p.align == Align::Right
            && p.indent == 24.0
            && p.line_spacing == 1.5
            && p.runs[0].format.script == 1));
        Ok(())
    }
    #[test]
    fn html_tables_close_list_containers_and_resume_numbering() -> Result<(), String> {
        let mut doc = Doc::from_text("first\nsecond");
        for p in &mut doc.paragraphs {
            p.list = List::Number;
        }
        doc.add_table(doc.paragraphs[0].id, 1, 1)?;
        let html = write_html(&doc);
        assert!(html.contains("</li></ol><table"));
        assert!(html.contains("</table><ol start=\"2\">"));
        Ok(())
    }
    #[test]
    fn html_pixel_sizes_and_inert_links_are_preserved_safely() -> Result<(), String> {
        let doc = read_html(
            "<html><body><p><span style=\"font-size:16px\">text</span></p></body></html>",
        )?;
        assert_eq!(doc.paragraphs[0].runs[0].format.size, 12.0);
        let mut doc = Doc::from_text("click");
        doc.paragraphs[0].runs[0].format.link = Some("java\tscript:alert(1)".into());
        assert!(!write_html(&doc).contains("<a "));
        assert!(safe_html_link("https://example.com/a"));
        Ok(())
    }
    #[test]
    fn malformed_html_fallback_keeps_hidden_content_out() -> Result<(), String> {
        let doc = read_html(
            "<html><head><style>secret CSS</style></head><body><p>Visible<img src='x'></p><script>secret JS</script></body></html>",
        )?;
        assert!(doc.plain_text().contains("Visible"));
        assert!(!doc.plain_text().contains("secret"));
        Ok(())
    }
    #[test]
    fn docx_zero_numbering_cancels_inherited_lists() -> Result<(), String> {
        let mut p = Paragraph {
            list: List::Number,
            ..Paragraph::default()
        };
        let props = xml(b"<w:pPr xmlns:w=\"x\"><w:numPr><w:numId w:val=\"0\"/></w:numPr></w:pPr>")?;
        apply_ppr(&mut p, &props, &BTreeMap::new());
        assert_eq!(p.list, List::None);
        Ok(())
    }

    use super::*;
    #[test]
    fn docx_tables_keep_merges_widths_and_borders() -> Result<(), String> {
        let mut doc = Doc::from_text(
            "До таблиц
После",
        );
        let anchor = doc.paragraphs.first().map(|p| p.id).unwrap_or_default();
        doc.add_table(anchor, 3, 3)?;
        doc.add_table(anchor, 1, 2)?;
        if let Some(table) = doc.tables.first_mut() {
            table.column_widths = vec![40.0, 200.0, 120.0];
            // Row 0: a header spanning all three columns.
            if let Some(row) = table.rows.get_mut(0) {
                row.truncate(1);
                if let Some(cell) = row.first_mut() {
                    cell.span = 3;
                    cell.v_align = VAlign::Center;
                }
            }
            // Column 0 merged vertically over rows 1 and 2.
            if let Some(cell) = table.rows.get_mut(2).and_then(|r| r.first_mut()) {
                cell.merged = true;
            }
        }
        if let Some(table) = doc.tables.get_mut(1) {
            table.borderless = true;
        }
        let first = read(&write(&doc, "docx")?, "docx")?;
        let second = read(&write(&first, "docx")?, "docx")?;
        for d in [&first, &second] {
            assert_eq!(d.paragraphs.len(), 2, "no spacer paragraphs leak in");
            assert_eq!(d.tables.len(), 2);
            let t = d.tables.first().ok_or("table")?;
            assert_eq!(t.grid_columns(), 3);
            assert_eq!(t.column_widths, vec![40.0, 200.0, 120.0]);
            assert_eq!(t.rows.first().map(Vec::len), Some(1));
            assert!(
                t.rows
                    .first()
                    .and_then(|r| r.first())
                    .is_some_and(|c| c.span == 3 && c.v_align == VAlign::Center)
            );
            assert!(
                t.rows
                    .get(2)
                    .and_then(|r| r.first())
                    .is_some_and(|c| c.merged)
            );
            assert!(
                !t.rows
                    .get(1)
                    .and_then(|r| r.first())
                    .is_some_and(|c| c.merged)
            );
            assert!(!t.borderless);
            assert!(d.tables.get(1).is_some_and(|t| t.borderless));
        }
        let xml = String::from_utf8_lossy(
            &crate::package::read_zip(&write(&doc, "docx")?)?
                .get("word/document.xml")
                .cloned()
                .unwrap_or_default(),
        )
        .into_owned();
        assert!(xml.contains("<w:tblLayout w:type=\"fixed\"/>"));
        assert!(xml.contains("<w:tcW w:w=\"800\" w:type=\"dxa\"/>"));
        assert!(xml.contains("<w:gridSpan w:val=\"3\"/>"));
        assert!(xml.contains("<w:vMerge w:val=\"restart\"/>"));
        Ok(())
    }
    #[test]
    fn docx_without_table_borders_imports_borderless() -> Result<(), String> {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>a</w:t></w:r></w:p><w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr><w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p/></w:body></w:document>"#;
        let bytes =
            crate::package::write_zip(vec![("word/document.xml".into(), xml.as_bytes().to_vec())])?;
        let doc = read(&bytes, "docx")?;
        assert!(doc.tables.first().is_some_and(|t| t.borderless));
        // Without docDefaults Word uses single spacing and no paragraph gaps.
        assert!(
            doc.paragraphs
                .first()
                .is_some_and(|p| p.spacing_after == 0.0 && p.line_spacing == 1.0)
        );
        Ok(())
    }
    #[test]
    fn docx_page_flow_properties_content_controls_and_header_rows() -> Result<(), String> {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:keepNext/><w:keepLines/><w:widowControl w:val="0"/><w:contextualSpacing/></w:pPr><w:r><w:t>a</w:t></w:r></w:p><w:sdt><w:sdtPr/><w:sdtContent><w:p><w:r><w:t>inside</w:t></w:r></w:p></w:sdtContent></w:sdt><w:tbl><w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid><w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p><w:r><w:t>h</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p/></w:body></w:document>"#;
        let bytes =
            crate::package::write_zip(vec![("word/document.xml".into(), xml.as_bytes().to_vec())])?;
        let doc = read(&bytes, "docx")?;
        let check = |doc: &Doc| -> Result<(), String> {
            let p = doc.paragraphs.first().ok_or("p")?;
            assert!(p.keep_next && p.keep_lines && p.contextual_spacing && !p.widow_control);
            assert!(doc.paragraphs.iter().any(|p| p.text() == "inside"));
            assert_eq!(doc.tables.first().map(|t| t.header_rows), Some(1));
            Ok(())
        };
        check(&doc)?;
        assert!(!doc.losses.iter().any(|l| l.contains("управления")));
        check(&read(&write(&doc, "docx")?, "docx")?)
    }
    #[test]
    fn docx_partial_indent_highlight_revisions_and_default_header() -> Result<(), String> {
        let styles = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Body"><w:name w:val="Body"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr><w:rPr><w:highlight w:val="yellow"/></w:rPr></w:style></w:styles>"#;
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:pPr><w:pStyle w:val="Body"/><w:ind w:firstLine="200"/></w:pPr><w:r><w:rPr><w:highlight w:val="none"/></w:rPr><w:t>a</w:t></w:r><w:moveFrom><w:r><w:t>old</w:t></w:r></w:moveFrom></w:p><w:p><w:pPr><w:pStyle w:val="Body"/><w:ind w:left="1440"/></w:pPr><w:r><w:t>b</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="first" r:id="h1"/><w:headerReference w:type="default" r:id="h2"/></w:sectPr></w:body></w:document>"#;
        let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="h1" Target="header1.xml"/><Relationship Id="h2" Target="header2.xml"/></Relationships>"#;
        let header = |text: &str| {
            format!(
                r#"<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">{text}</w:hdr>"#
            )
        };
        let bytes = crate::package::write_zip(vec![
            ("word/document.xml".into(), xml.as_bytes().to_vec()),
            ("word/styles.xml".into(), styles.as_bytes().to_vec()),
            ("word/_rels/document.xml.rels".into(), rels.as_bytes().to_vec()),
            (
                "word/header1.xml".into(),
                header(r#"<w:p><w:r><w:t>Title page</w:t></w:r></w:p>"#).into_bytes(),
            ),
            (
                "word/header2.xml".into(),
                header(r#"<w:p><w:pPr><w:tabs><w:tab w:val="center" w:pos="4677"/></w:tabs></w:pPr><w:r><w:t>Acme</w:t></w:r><w:r><w:tab/><w:t>Report</w:t></w:r></w:p><w:p><w:r><w:t>Draft</w:t></w:r></w:p>"#).into_bytes(),
            ),
        ])?;
        let doc = read(&bytes, "docx")?;
        let first = doc.paragraphs.first().ok_or("p")?;
        // Only firstLine is overridden: the style's left indent stays.
        assert_eq!((first.indent, first.first_indent), (36.0, 10.0));
        assert_eq!(first.text(), "a");
        assert!(first.runs.iter().all(|r| r.format.highlight.is_none()));
        let second = doc.paragraphs.get(1).ok_or("p")?;
        // Only left is overridden: the style's hanging indent stays.
        assert_eq!((second.indent, second.first_indent), (72.0, -18.0));
        assert!(
            second
                .runs
                .iter()
                .all(|r| r.format.highlight == Some([255, 255, 0]))
        );
        assert_eq!(doc.page.header, "Acme    Report  Draft");
        Ok(())
    }
    #[test]
    fn docx_line_rules_round_trip() -> Result<(), String> {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:spacing w:line="300" w:lineRule="exact"/></w:pPr><w:r><w:t>a</w:t></w:r></w:p><w:p><w:pPr><w:spacing w:line="360"/></w:pPr><w:r><w:t>b</w:t></w:r></w:p><w:p><w:pPr><w:spacing w:line="400" w:lineRule="atLeast"/></w:pPr><w:r><w:t>c</w:t></w:r></w:p></w:body></w:document>"#;
        let bytes =
            crate::package::write_zip(vec![("word/document.xml".into(), xml.as_bytes().to_vec())])?;
        for _ in 0..2 {
            let doc = read(&bytes, "docx")?;
            let doc = read(&write(&doc, "docx")?, "docx")?;
            let p: Vec<_> = doc.paragraphs.iter().take(3).collect();
            assert_eq!((p[0].line_points, p[0].line_exact), (15.0, true));
            assert_eq!((p[1].line_points, p[1].line_spacing), (0.0, 1.5));
            assert_eq!((p[2].line_points, p[2].line_exact), (20.0, false));
            let layout = doc.layout();
            assert_eq!(layout.rows.first().map(|r| r.height), Some(15.0));
        }
        Ok(())
    }
    #[test]
    fn docx_table_style_margins_and_row_height() -> Result<(), String> {
        let styles = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="table" w:styleId="Grid"><w:name w:val="Table Grid"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:tblPr><w:tblCellMar><w:left w:w="108" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style></w:styles>"#;
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>a</w:t></w:r></w:p><w:tbl><w:tblPr><w:tblStyle w:val="Grid"/><w:tblCellMar><w:top w:w="40" w:type="dxa"/></w:tblCellMar></w:tblPr><w:tblGrid><w:gridCol w:w="2000"/></w:tblGrid><w:tr><w:trPr><w:trHeight w:val="600"/></w:trPr><w:tc><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p/></w:body></w:document>"#;
        let bytes = crate::package::write_zip(vec![
            ("word/document.xml".into(), xml.as_bytes().to_vec()),
            ("word/styles.xml".into(), styles.as_bytes().to_vec()),
        ])?;
        let doc = read(&bytes, "docx")?;
        let table = doc.tables.first().ok_or("table")?;
        assert_eq!(table.cell_margin, [2.0, 5.4, 0.0, 5.4]);
        assert_eq!(table.row_heights, vec![30.0]);
        let cell = table
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|c| c.paragraphs.first())
            .ok_or("cell")?;
        assert_eq!((cell.spacing_after, cell.line_spacing), (0.0, 1.0));
        // Body text keeps the document defaults.
        assert!(
            doc.paragraphs
                .first()
                .is_some_and(|p| p.spacing_after == 8.0)
        );
        let again = read(&write(&doc, "docx")?, "docx")?;
        let table = again.tables.first().ok_or("table")?;
        assert_eq!(table.cell_margin, [2.0, 5.4, 0.0, 5.4]);
        assert_eq!(table.row_heights, vec![30.0]);
        Ok(())
    }
    #[test]
    fn docx_round_trip_format_table_page() {
        let mut doc = Doc::from_text("Привет <мир> & коллеги\nВторой абзац");
        doc.title = "Title & test".into();
        doc.page.header = "Header".into();
        if let Some(p) = doc.paragraphs.first_mut() {
            p.list = List::Number;
            p.align = Align::Center;
            if let Some(r) = p.runs.first_mut() {
                r.format.bold = true;
                r.format.color = [12, 30, 80];
                r.format.highlight = Some([240, 230, 10]);
            }
        }
        let id = doc.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(doc.add_table(id, 2, 2).is_ok());
        if let Some(table) = doc.tables.first_mut() {
            table.border_color = [180, 20, 60];
        }
        if let Some(c) = doc
            .tables
            .first_mut()
            .and_then(|t| t.rows.first_mut())
            .and_then(|r| r.first_mut())
            && let Some(p) = c.paragraphs.first_mut()
        {
            p.runs = vec![Run {
                text: "Ячейка".into(),
                format: TextFormat::default(),
            }];
        }
        let bytes = write(&doc, "docx");
        assert!(bytes.is_ok());
        let reopened = bytes.and_then(|b| read(&b, "docx"));
        assert!(reopened.is_ok());
        if let Ok(d) = reopened {
            assert_eq!(d.plain_text(), doc.plain_text());
            assert_eq!(d.title, doc.title);
            assert_eq!(d.tables.len(), 1);
            assert_eq!(d.page.header, "Header");
            assert_eq!(
                d.tables.first().map(|t| t.border_color),
                Some([180, 20, 60])
            );
            assert!(d.paragraphs.first().is_some_and(|p| {
                p.align == Align::Center
                    && p.list == List::Number
                    && p.runs
                        .first()
                        .is_some_and(|r| r.format.bold && r.format.color == [12, 30, 80])
            }));
        }
    }
    #[test]
    fn docx_rich_formatting_and_pictures_survive_two_generations() -> Result<(), String> {
        let mut doc = Doc::from_text("Заголовок\nТекст абзаца\nПункт списка");
        doc.page.footer = "Подвал".into();
        doc.page.margin_left = 90.0;
        doc.page.page_numbers = true;
        if let Some(p) = doc.paragraphs.get_mut(0) {
            p.style = "Heading1".into();
        }
        if let Some(p) = doc.paragraphs.get_mut(1) {
            p.align = Align::Justify;
            p.spacing_after = 12.0;
            p.first_indent = 18.0;
            if let Some(r) = p.runs.first_mut() {
                r.format.italic = true;
                r.format.underline = true;
                r.format.strike = true;
                r.format.size = 14.0;
                r.format.font = "Arial".into();
            }
        }
        if let Some(p) = doc.paragraphs.get_mut(2) {
            p.list = List::Bullet;
        }
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(4, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let anchor = doc.paragraphs.get(1).map(|p| p.id).ok_or("paragraph")?;
        let id = doc.alloc_id();
        doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: anchor,
            bytes: png.into_inner(),
            width: 200.0,
            height: 100.0,
            alt: "Схема".into(),
        });
        let first = read(&write(&doc, "docx")?, "docx")?;
        let second = read(&write(&first, "docx")?, "docx")?;
        assert_eq!(first, second, "второе поколение docx отличается от первого");
        assert_eq!(first.plain_text(), doc.plain_text());
        assert_eq!(first.page.footer, "Подвал");
        assert_eq!(first.images.len(), 1);
        assert_eq!(
            first
                .images
                .first()
                .map(|i| (i.width.round(), i.height.round())),
            Some((200.0, 100.0))
        );
        assert_eq!(
            first.paragraphs.first().map(|p| p.style.as_str()),
            Some("Heading1")
        );
        assert_eq!(first.paragraphs.get(2).map(|p| p.list), Some(List::Bullet));
        let body = first.paragraphs.get(1).ok_or("body")?;
        assert_eq!(body.align, Align::Justify);
        assert!(body.runs.first().is_some_and(|r| r.format.italic
            && r.format.underline
            && r.format.strike
            && r.format.size == 14.0
            && r.format.font == "Arial"));
        Ok(())
    }
    #[test]
    fn xml_ignores_outer_whitespace_and_rejects_extra_roots() {
        let root = xml(b"<?xml version=\"1.0\"?>\n  <document><body/></document>\n");
        assert_eq!(root.ok().map(|n| n.name), Some("document".into()));
        assert!(xml(b"<document/> <extra/>").is_err());
        assert!(xml(b"outside<document/>").is_err());
        assert!(xml(b"<document/>outside").is_err());
    }
    #[test]
    fn docx_with_pretty_outer_xml_opens() -> Result<(), String> {
        let doc = Doc::from_text("Outer whitespace");
        let bytes = write(&doc, "docx")?;
        let mut parts = read_zip(&bytes)?;
        if let Some(main) = parts.get_mut("word/document.xml") {
            let declaration_end = main
                .windows(2)
                .position(|w| w == b"?>")
                .map(|i| i + 2)
                .unwrap_or_default();
            main.splice(declaration_end..declaration_end, b"\n ".iter().copied());
            main.extend_from_slice(b"\n ");
        }
        let bytes = write_zip(parts.into_iter().collect())?;
        assert_eq!(read(&bytes, "docx")?.plain_text(), doc.plain_text());
        Ok(())
    }
    #[test]
    fn explicit_unbold_overrides_heading_style() -> Result<(), String> {
        let mut doc = Doc::from_text("Unbold heading");
        if let Some(p) = doc.paragraphs.first_mut() {
            p.set_style("Heading1");
            p.first_indent = -18.0;
            for r in &mut p.runs {
                r.format.bold = false;
                r.format.italic = false;
            }
        }
        let bytes = write(&doc, "docx")?;
        let parts = read_zip(&bytes)?;
        let main = parts.get("word/document.xml").ok_or("part")?;
        assert!(String::from_utf8_lossy(main).contains("w:hanging=\"360\""));
        let reopened = read(&bytes, "docx")?;
        assert!(
            reopened
                .paragraphs
                .first()
                .is_some_and(|p| p.runs.iter().all(|r| !r.format.bold))
        );
        assert_eq!(
            reopened.paragraphs.first().map(|p| p.first_indent),
            Some(-18.0)
        );
        Ok(())
    }
    #[test]
    fn image_roundtrips_do_not_grow_empty_paragraphs() -> Result<(), String> {
        let mut doc = Doc::from_text("Before image\nAfter image");
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let anchor = doc.paragraphs.first().map(|p| p.id).ok_or("anchor")?;
        let id = doc.alloc_id();
        doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: anchor,
            bytes: png.into_inner(),
            width: 72.0,
            height: 72.0,
            alt: "sample".into(),
        });
        let expected = doc.plain_text();
        for _ in 0..3 {
            doc = read(&write(&doc, "docx")?, "docx")?;
            assert_eq!(doc.plain_text(), expected);
            assert_eq!(doc.paragraphs.len(), 2);
            assert_eq!(doc.images.len(), 1);
            assert_eq!(
                doc.images.first().map(|i| i.after_paragraph_id),
                doc.paragraphs.first().map(|p| p.id)
            );
        }
        Ok(())
    }
    #[test]
    fn rejects_xml_entities() {
        assert!(
            xml(b"<!DOCTYPE test [<!ENTITY x SYSTEM 'file:///c:/secret'>]><a>&x;</a>").is_err()
        );
    }
    #[test]
    fn text_decoding() {
        assert_eq!(
            decode_text(&[0xff, 0xfe, 0x1f, 0x04, 0x40, 0x04])
                .ok()
                .as_deref(),
            Some("Пр")
        );
        assert_eq!(
            decode_text(&[0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2])
                .ok()
                .as_deref(),
            Some("Привет")
        );
    }
    #[test]
    fn markdown_styles() {
        let d = read_markdown("# Заголовок\n- **Важное** и *курсив*");
        assert_eq!(
            d.paragraphs.first().map(|p| p.style.as_str()),
            Some("Heading1")
        );
        assert!(
            d.paragraphs
                .get(1)
                .is_some_and(|p| p.list == List::Bullet && p.runs.iter().any(|r| r.format.bold))
        );
    }
    #[test]
    fn malformed_and_unsupported_are_errors() {
        assert!(read(b"bad", "docx").is_err());
        assert!(read(b"abc", "doc").is_err());
        assert!(xml(b"<a><b></a>").is_err());
        assert!(resolve_part("../../secret").is_none());
    }
    #[test]
    fn preserves_link_run_identity() {
        let mut doc = Doc::from_text("");
        if let Some(p) = doc.paragraphs.first_mut() {
            p.runs = vec![
                Run {
                    text: "same".into(),
                    format: TextFormat::default(),
                },
                Run {
                    text: "same".into(),
                    format: TextFormat {
                        link: Some("https://example.com".into()),
                        ..TextFormat::default()
                    },
                },
            ];
        }
        let d = write(&doc, "docx").and_then(|b| read(&b, "docx"));
        assert!(d.is_ok());
        if let Ok(d) = d {
            assert!(
                d.paragraphs
                    .first()
                    .and_then(|p| p.runs.first())
                    .is_some_and(|r| r.format.link.is_none())
            );
            assert!(
                d.paragraphs
                    .first()
                    .and_then(|p| p.runs.get(1))
                    .is_some_and(|r| r.format.link.as_deref() == Some("https://example.com"))
            );
        }
    }
    #[test]
    fn html_mixed_blocks_and_tables_keep_all_visible_text() -> Result<(), String> {
        let doc = read(
            br#"<html><head><title>hidden</title></head><body>before <b>bold</b><p>middle</p>after<table><tr><th>Header</th><td><p>Cell one</p><p>Cell two</p></td></tr></table>end<script>hidden</script></body></html>"#,
            "html",
        )?;
        assert_eq!(
            doc.plain_text(),
            "before bold\nmiddle\nafter\nHeader\tCell one\nCell two\nend"
        );
        assert!(
            doc.paragraphs
                .first()
                .is_some_and(|p| p.runs.iter().any(|r| r.text == "bold" && r.format.bold))
        );
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(doc.tables[0].rows[0].len(), 2);
        Ok(())
    }
    #[test]
    fn html_container_formats_survive_loose_and_mixed_blocks() -> Result<(), String> {
        for html in [
            r#"<div style="font-size:24pt;color:#ff0000">Hello <b>bold</b></div>"#,
            r#"<div style="font-size:24pt;color:#ff0000">Hello <p>middle <b>bold</b></p>end</div>"#,
        ] {
            let doc = read(html.as_bytes(), "html")?;
            assert!(
                doc.paragraphs
                    .iter()
                    .flat_map(|p| &p.runs)
                    .all(|r| r.format.size == 24.0 && r.format.color == [255, 0, 0])
            );
            assert!(
                doc.paragraphs
                    .iter()
                    .flat_map(|p| &p.runs)
                    .any(|r| r.text == "bold" && r.format.bold)
            );
        }
        Ok(())
    }
    #[test]
    fn html_loose_body_preserves_heading_semantics() -> Result<(), String> {
        let doc = read(b"<html><body><h4>Heading</h4></body></html>", "html")?;
        assert_eq!(doc.plain_text(), "Heading");
        assert_eq!(doc.paragraphs[0].style, "Heading4");
        Ok(())
    }
    #[test]
    fn office_extension_case_keeps_the_correct_content_type() -> Result<(), String> {
        for (extension, expected) in [
            (".DOTX", "wordprocessingml.template.main+xml"),
            ("DOCM", "macroEnabled.main+xml"),
        ] {
            let parts = read_zip(&write(&Doc::from_text("text"), extension)?)?;
            let content = parts
                .get("[Content_Types].xml")
                .ok_or("Missing content types")?;
            assert!(String::from_utf8_lossy(content).contains(expected));
        }
        Ok(())
    }
    #[test]
    fn markdown_external_newlines_are_soft_breaks_after_normalization() -> Result<(), String> {
        let doc = read(b"first\rsecond\r\nthird", "md")?;
        assert_eq!(doc.plain_text(), "first second third");
        Ok(())
    }
    #[test]
    fn html_own_export_preserves_mixed_text_order() {
        let mut doc = Doc::from_text("");
        if let Some(p) = doc.paragraphs.first_mut() {
            p.runs = vec![
                Run {
                    text: "before & ".into(),
                    format: TextFormat::default(),
                },
                Run {
                    text: "bold".into(),
                    format: TextFormat {
                        bold: true,
                        ..TextFormat::default()
                    },
                },
                Run {
                    text: " after".into(),
                    format: TextFormat::default(),
                },
            ];
        }
        let d = write(&doc, "html").and_then(|b| read(&b, "html"));
        assert!(d.is_ok());
        if let Ok(d) = d {
            assert_eq!(d.plain_text(), doc.plain_text());
            assert!(
                d.paragraphs
                    .first()
                    .is_some_and(|p| p.runs.iter().any(|r| r.format.bold))
            );
        }
    }
    #[test]
    fn page_field_does_not_enter_footer_text() {
        let mut doc = Doc::from_text("body");
        doc.page.footer = "Company ".into();
        doc.page.page_numbers = true;
        let d = write(&doc, "docx").and_then(|b| read(&b, "docx"));
        assert!(d.is_ok());
        if let Ok(d) = d {
            assert_eq!(d.page.footer, "Company ");
            assert!(d.page.page_numbers);
        }
    }
    #[test]
    fn inline_page_break_stays_inside_paragraph() {
        let doc = Doc::from_text("before\u{000c}after");
        let d = write(&doc, "docx").and_then(|b| read(&b, "docx"));
        assert!(d.is_ok());
        if let Ok(d) = d {
            assert_eq!(d.plain_text(), doc.plain_text());
            assert_eq!(d.pages().len(), 2);
        }
    }
    #[test]
    fn generated_docx_round_trip_corpus() {
        for variant in 0..50 {
            let mut doc = Doc::from_text(&format!(
                "Файл {variant}: русский & Latin <text>\nSecond paragraph"
            ));
            if let Some(p) = doc.paragraphs.first_mut() {
                p.align = if variant % 2 == 0 {
                    Align::Center
                } else {
                    Align::Right
                };
                p.list = if variant % 3 == 0 {
                    List::Number
                } else {
                    List::Bullet
                };
                if let Some(r) = p.runs.first_mut() {
                    r.format.bold = variant % 2 == 0;
                    r.format.italic = variant % 3 == 0;
                    r.format.underline = variant % 5 == 0;
                    r.format.size = 8.0 + variant as f32;
                }
            }
            let bytes = write(&doc, "docx");
            assert!(bytes.is_ok(), "variant {variant}");
            let reopened = bytes.and_then(|b| read(&b, "docx"));
            assert!(reopened.is_ok(), "variant {variant}");
            if let Ok(d) = reopened {
                assert_eq!(d.plain_text(), doc.plain_text());
                assert_eq!(
                    d.paragraphs.first().map(|p| p.align),
                    doc.paragraphs.first().map(|p| p.align)
                );
            }
        }
    }
}
#[cfg(test)]
mod alternate_content_review_tests {
    use super::*;

    fn import_body(body: &str) -> Result<Doc, String> {
        let document = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:core="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:future="urn:unsupported" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><w:body>{body}</w:body></w:document>"#
        );
        read(
            &write_zip(vec![("word/document.xml".into(), document.into_bytes())])?,
            "docx",
        )
    }
    #[test]
    fn docx_alternate_content_selects_one_supported_branch_at_every_level() -> Result<(), String> {
        let doc = import_body(
            r#"<mc:AlternateContent><mc:Choice Requires="core"><w:p><w:r><w:t>body</w:t></w:r></w:p></mc:Choice><mc:Fallback><w:p><w:r><w:t>duplicate body</w:t></w:r></w:p></mc:Fallback></mc:AlternateContent><w:p><mc:AlternateContent><mc:Choice Requires="core"><w:r><w:t>choice</w:t></w:r></mc:Choice><mc:Fallback><w:r><w:t>duplicate run</w:t></w:r></mc:Fallback></mc:AlternateContent><w:r><mc:AlternateContent><mc:Choice Requires="future"><w:t>unsupported</w:t></mc:Choice><mc:Fallback><w:t> fallback</w:t></mc:Fallback></mc:AlternateContent></w:r></w:p>"#,
        )?;
        let texts = doc.paragraphs.iter().map(|p| p.text()).collect::<Vec<_>>();
        assert_eq!(texts, vec!["body", "choice fallback"]);
        Ok(())
    }
    #[test]
    fn docx_requires_uses_namespace_uris_and_all_required_prefixes() -> Result<(), String> {
        for choice in [
            r#"<mc:Choice Requires="core future">"#,
            r#"<mc:Choice Requires="undeclared">"#,
            r#"<mc:Choice xmlns:core="urn:unsupported" Requires="core">"#,
            r#"<mc:Choice>"#,
        ] {
            let doc = import_body(&format!(
                r#"<w:p><mc:AlternateContent>{choice}<w:r><w:t>wrong</w:t></w:r></mc:Choice><mc:Fallback><w:r><w:t>fallback</w:t></w:r></mc:Fallback></mc:AlternateContent></w:p>"#
            ))?;
            assert_eq!(doc.paragraphs[0].text(), "fallback");
        }
        Ok(())
    }
}

fn css_font(font: &str) -> String {
    let font: String = font
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let font = font.replace('\\', "\\\\").replace('\'', "\\'");
    esc(&format!("'{font}'"))
}
fn html_paragraph_bounded(p: &Paragraph, tag: &str, limit: usize) -> Result<String, String> {
    let align = match p.align {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
        Align::Justify => "justify",
    };
    let line = if p.line_points > 0.0 {
        format!("{}pt", p.line_points)
    } else {
        p.line_spacing.to_string()
    };
    let mut out = format!(
        "<{tag} data-trinedocs-list-level=\"{}\" data-trinedocs-line-spacing=\"{}\" data-trinedocs-line-exact=\"{}\" style=\"white-space:pre-wrap;text-align:{align};margin-left:{}pt;margin-right:{}pt;text-indent:{}pt;margin-top:{}pt;margin-bottom:{}pt;line-height:{line};break-before:{};break-after:{};break-inside:{};widows:{};orphans:{}\">",
        p.list_level,
        p.line_spacing,
        p.line_exact,
        p.indent
            + if p.list != List::None {
                f32::from(p.list_level) * 18.0
            } else {
                0.0
            },
        p.right_indent,
        p.first_indent,
        p.spacing_before,
        p.spacing_after,
        if p.page_break_before { "page" } else { "auto" },
        if p.keep_next { "avoid" } else { "auto" },
        if p.keep_lines { "avoid" } else { "auto" },
        if p.widow_control { 2 } else { 1 },
        if p.widow_control { 2 } else { 1 }
    );
    if out.len() > limit {
        return Err("HTML превышает безопасный предел размера".into());
    }
    for r in &p.runs {
        let f = &r.format;
        html_extend(
            &mut out,
            &format!(
                "<span style=\"font-family:{};font-size:{}pt;color:#{};font-weight:{};font-style:{};text-decoration:{}{};{}\">",
                css_font(&f.font),
                f.size,
                hex(f.color),
                if f.bold { "bold" } else { "normal" },
                if f.italic { "italic" } else { "normal" },
                if f.underline { "underline " } else { "" },
                if f.strike { "line-through" } else { "" },
                f.highlight
                    .map(|c| format!("background-color:#{}", hex(c)))
                    .unwrap_or_default()
            ),
            limit,
        )?;
        let link = f.link.as_deref().filter(|link| safe_html_link(link));
        if let Some(link) = link {
            html_extend(&mut out, &format!("<a href=\"{}\">", esc(link)), limit)?;
        }
        if f.script != 0 {
            html_extend(
                &mut out,
                if f.script > 0 { "<sup>" } else { "<sub>" },
                limit,
            )?;
        }
        html_escaped_text(&mut out, &r.text, limit)?;
        if f.script != 0 {
            html_extend(
                &mut out,
                if f.script > 0 { "</sup>" } else { "</sub>" },
                limit,
            )?;
        }
        if link.is_some() {
            html_extend(&mut out, "</a>", limit)?;
        }
        html_extend(&mut out, "</span>", limit)?;
    }
    html_extend(&mut out, &format!("</{tag}>"), limit)?;
    Ok(out)
}
fn encode_image_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for part in bytes.chunks(3) {
        let number = (u32::from(part[0]) << 16)
            | (u32::from(*part.get(1).unwrap_or(&0)) << 8)
            | u32::from(*part.get(2).unwrap_or(&0));
        out.push(char::from(ALPHABET[((number >> 18) & 63) as usize]));
        out.push(char::from(ALPHABET[((number >> 12) & 63) as usize]));
        out.push(if part.len() > 1 {
            char::from(ALPHABET[((number >> 6) & 63) as usize])
        } else {
            '='
        });
        out.push(if part.len() > 2 {
            char::from(ALPHABET[(number & 63) as usize])
        } else {
            '='
        });
    }
    out
}
fn decode_html_image(source: &str) -> Option<Vec<u8>> {
    let source = source.split('#').next()?;
    let (prefix, data) = source.split_once(',')?;
    let prefix = prefix.trim().to_ascii_lowercase();
    let media = prefix.strip_prefix("data:")?;
    let (mime, parameters) = media.split_once(';')?;
    if !matches!(mime.trim(), "image/png" | "image/jpeg")
        || !parameters.trim_end().ends_with("base64")
        || parameters
            .rsplit_once(';')
            .map(|(_, tail)| tail)
            .unwrap_or(parameters)
            .trim()
            != "base64"
        || data.len() > XML_LIMIT
    {
        return None;
    }
    let mut decoded = Vec::with_capacity(data.len());
    let mut input = data.as_bytes().iter().copied();
    while let Some(ch) = input.next() {
        let ch = if ch == b'%' {
            let high = char::from(input.next()?).to_digit(16)?;
            let low = char::from(input.next()?).to_digit(16)?;
            (high * 16 + low) as u8
        } else {
            ch
        };
        if !matches!(ch, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ') {
            decoded.push(ch);
        }
    }
    if decoded.len().is_multiple_of(4) {
        for _ in 0..2 {
            if decoded.last() == Some(&b'=') {
                decoded.pop();
            } else {
                break;
            }
        }
    }
    if decoded.len() % 4 == 1 {
        return None;
    }
    let mut bytes = Vec::with_capacity(decoded.len() * 3 / 4);
    let (mut bits, mut buffer) = (0u32, 0u32);
    for ch in decoded {
        let digit = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(digit);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(bytes)
}

#[cfg(test)]
mod text_export_defect_tests {
    use super::*;
    #[test]
    fn html_paragraph_geometry_roundtrips() -> Result<(), String> {
        let mut doc = Doc::from_text("one\ntwo");
        let p = &mut doc.paragraphs[0];
        p.list = List::Number;
        p.list_level = 2;
        p.indent = 18.0;
        p.first_indent = -9.0;
        p.spacing_before = 7.0;
        p.spacing_after = 11.0;
        p.line_spacing = 1.5;
        p.line_points = 21.0;
        p.line_exact = false;
        p.page_break_before = true;
        p.keep_next = true;
        p.keep_lines = true;
        p.widow_control = false;
        let imported = read(&write(&doc, "html")?, "html")?;
        let p = &imported.paragraphs[0];
        assert_eq!(
            (p.indent, p.first_indent, p.spacing_before, p.spacing_after),
            (18.0, -9.0, 7.0, 11.0)
        );
        assert_eq!(
            (p.line_spacing, p.line_points, p.line_exact),
            (1.5, 21.0, false)
        );
        assert_eq!((p.list, p.list_level), (List::Number, 2));
        assert!(p.page_break_before && p.keep_next && p.keep_lines);
        assert!(!p.widow_control);
        Ok(())
    }
    #[test]
    fn html_table_merges_and_rich_cells() -> Result<(), String> {
        let mut doc = Doc::from_text("table");
        doc.add_table(doc.paragraphs[0].id, 3, 2)?;
        let table = &mut doc.tables[0];
        for row in table.rows.iter_mut().take(2) {
            row[0].span = 2;
            row.remove(1);
        }
        table.rows[1][0].merged = true;
        table.borderless = true;
        table.cell_margin = [1.0, 2.0, 3.0, 4.0];
        table.row_heights = vec![24.0, 24.0, 24.0];
        table.rows[0][0].v_align = VAlign::Bottom;
        let p = &mut table.rows[0][0].paragraphs[0];
        p.runs[0].text = "bold & cell".into();
        p.runs[0].format.bold = true;
        let html = String::from_utf8(write(&doc, "html")?).map_err(|e| e.to_string())?;
        assert!(html.contains("colspan=\"2\" rowspan=\"2\""));
        assert_eq!(
            html.matches("<td ").count() + html.matches("<th ").count(),
            3
        );
        assert!(
            html.contains("vertical-align:bottom;height:24pt;padding:1pt 2pt 3pt 4pt;border:none")
        );
        assert!(html.contains("font-weight:bold"));
        assert!(html.contains("bold &amp; cell"));
        assert!(html.contains("<col style=\"width:"));
        Ok(())
    }
    #[test]
    fn html_images_roundtrip_without_network() -> Result<(), String> {
        let mut doc = Doc::from_text("before\nafter");
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let bytes = png.into_inner();
        let id = doc.alloc_id();
        doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: doc.paragraphs[0].id,
            bytes: bytes.clone(),
            width: 72.0,
            height: 36.0,
            alt: "a & b".into(),
        });
        let exported = write(&doc, "html")?;
        assert!(String::from_utf8_lossy(&exported).contains("data:image/png;base64,"));
        let imported = read(&exported, "html")?;
        assert_eq!(imported.plain_text(), doc.plain_text());
        assert_eq!(imported.images.len(), 1);
        assert_eq!(imported.images[0].bytes, bytes);
        assert_eq!(
            (imported.images[0].width, imported.images[0].height),
            (72.0, 36.0)
        );
        assert_eq!(
            imported.images[0].after_paragraph_id,
            imported.paragraphs[0].id
        );
        assert_eq!(imported.images[0].alt, "a & b");
        assert!(decode_html_image("https://example.org/a.png").is_none());
        assert!(decode_html_image("data:image/png;base64,AA=A").is_none());
        Ok(())
    }
    #[test]
    fn html_font_names_cannot_inject_css() -> Result<(), String> {
        let mut doc = Doc::from_text("font");
        doc.paragraphs[0].runs[0].format.font = "A'; color:red; '\u{000c}B".into();
        let html = write_html(&doc);
        let root = xml(html.trim_start_matches("<!DOCTYPE html>").as_bytes())?;
        let span = root.descendant("span").ok_or("span")?;
        let style = span.attr("style").ok_or("style")?;
        assert!(style.starts_with("font-family:'A\\'; color:red; \\' B';font-size:"));
        assert!(!style.contains('\u{000c}'));
        Ok(())
    }
    #[test]
    fn markdown_paragraph_boundaries_and_empty_paragraphs() -> Result<(), String> {
        let doc = Doc::from_text("one\n\ntwo\n");
        let md = write_markdown(&doc);
        assert!(md.starts_with(MD_PARAGRAPHS));
        assert!(md.contains("one\n\n<!--td-empty-->\n\ntwo\n\n"));
        assert_eq!(read(md.as_bytes(), "md")?.plain_text(), doc.plain_text());
        assert_eq!(
            read(b"external\nlines", "md")?.plain_text(),
            "external lines"
        );
        let versioned = format!("{MD_PARAGRAPHS}\nlegacy\nlines");
        assert_eq!(
            read(versioned.as_bytes(), "md")?.plain_text(),
            "legacy\nlines"
        );
        let mut nested = Doc::from_text("item");
        nested.paragraphs[0].list = List::Bullet;
        nested.paragraphs[0].list_level = 2;
        let roundtrip = read(&write(&nested, "md")?, "md")?;
        assert_eq!(
            (
                roundtrip.paragraphs[0].list,
                roundtrip.paragraphs[0].list_level
            ),
            (List::Bullet, 2)
        );
        Ok(())
    }
    #[test]
    fn markdown_emphasis_does_not_enclose_edge_whitespace() -> Result<(), String> {
        let mut doc = Doc::from_text(" bold ");
        doc.paragraphs[0].runs[0].format.bold = true;
        let md = write_markdown(&doc);
        assert!(md.contains("&#32;**bold**&#32;"));
        let imported = read(md.as_bytes(), "md")?;
        assert_eq!(imported.plain_text(), " bold ");
        assert!(
            imported.paragraphs[0]
                .runs
                .iter()
                .any(|r| r.text == "bold" && r.format.bold)
        );
        Ok(())
    }
    #[test]
    fn markdown_tables_escape_content() -> Result<(), String> {
        let mut doc = Doc::from_text("table");
        doc.add_table(doc.paragraphs[0].id, 1, 1)?;
        doc.tables[0].rows[0][0].paragraphs[0].runs[0].text = "<img>|a\nb".into();
        let md = write_markdown(&doc);
        assert!(md.contains(r"\<img\>\|a<br>b"));
        assert!(!md.contains("<img>"));
        Ok(())
    }
    #[test]
    fn markdown_link_destinations_encode_whitespace_and_controls() -> Result<(), String> {
        let mut doc = Doc::from_text("link");
        doc.paragraphs[0].runs[0].format.link = Some("https://example.org/a b\nc".into());
        let md = write_markdown(&doc);
        assert!(md.contains("[link](https://example.org/a%20b%0Ac)"));
        let imported = read(md.as_bytes(), "md")?;
        assert_eq!(imported.plain_text(), "link");
        assert_eq!(
            imported.paragraphs[0].runs[0].format.link.as_deref(),
            Some("https://example.org/a%20b%0Ac")
        );
        Ok(())
    }
    #[test]
    fn markdown_unsafe_links_become_plain_text() -> Result<(), String> {
        let mut doc = Doc::from_text("visible");
        doc.paragraphs[0].runs[0].format.link = Some("javascript:alert(1)".into());
        let md = write_markdown(&doc);
        assert!(!md.contains("javascript:"));
        assert_eq!(read(md.as_bytes(), "md")?.plain_text(), "visible");
        doc.paragraphs[0].runs[0].format.link = Some("https://example.org/a".into());
        assert!(write_markdown(&doc).contains("[visible](https://example.org/a)"));
        Ok(())
    }
}

fn css_font_shorthand(value: &str) -> Option<Vec<(String, String)>> {
    let mut tokens = Vec::new();
    let (mut start, mut quote, mut escaped) = (None, None, false);
    for (index, ch) in value.char_indices() {
        if start.is_none() && !ch.is_ascii_whitespace() {
            start = Some(index);
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch.is_ascii_whitespace()
            && let Some(begin) = start.take()
        {
            tokens.push((begin, index));
        }
    }
    if let Some(begin) = start {
        tokens.push((begin, value.len()));
    }
    let mut style = "normal".to_string();
    let mut weight = "normal".to_string();
    for (index, (start, end)) in tokens.iter().copied().enumerate() {
        let token = &value[start..end];
        let lower = token.to_ascii_lowercase();
        let size = lower.split('/').next()?;
        let size_value = size.trim_end_matches(|ch: char| ch.is_ascii_alphabetic() || ch == '%');
        let is_size = matches!(
            size,
            "xx-small"
                | "x-small"
                | "small"
                | "medium"
                | "large"
                | "x-large"
                | "xx-large"
                | "smaller"
                | "larger"
        ) || size_value
            .parse::<f32>()
            .ok()
            .is_some_and(|value| value.is_finite() && value > 0.0)
            && size_value.len() < size.len();
        if is_size {
            let mut family_start = end;
            let mut line = "normal".to_string();
            if let Some((_, height)) = token.split_once('/') {
                if height.is_empty() {
                    let (begin, end) = tokens.get(index + 1).copied()?;
                    line = value[begin..end].into();
                    family_start = end;
                } else {
                    line = height.into();
                }
            } else if let Some((next_start, next_end)) = tokens.get(index + 1).copied() {
                let next = &value[next_start..next_end];
                if let Some(height) = next.strip_prefix('/') {
                    if height.is_empty() {
                        let (_, height_end) = tokens.get(index + 2).copied()?;
                        line = value[next_end..height_end].trim().into();
                        family_start = height_end;
                    } else {
                        line = height.into();
                        family_start = next_end;
                    }
                }
            }
            let family = value[family_start..].trim();
            if family.is_empty() {
                return None;
            }
            return Some(vec![
                ("font-style".into(), style),
                ("font-weight".into(), weight),
                ("font-size".into(), size.into()),
                ("line-height".into(), line),
                ("font-family".into(), family.into()),
            ]);
        }
        match lower.as_str() {
            "italic" | "oblique" => style = lower,
            "bold" | "bolder" | "lighter" => weight = lower,
            "normal" | "small-caps" => {}
            value
                if value
                    .parse::<u16>()
                    .ok()
                    .is_some_and(|value| (1..=1000).contains(&value)) =>
            {
                weight = value.into()
            }
            _ => return None,
        }
    }
    None
}
fn css_declarations(css: &str) -> Vec<(String, String)> {
    let mut chunks = Vec::new();
    let (mut start, mut quote, mut escape) = (0, None, false);
    for (index, ch) in css.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == ';' {
            chunks.push(&css[start..index]);
            start = index + 1;
        }
    }
    chunks.push(&css[start..]);
    let mut result: Vec<(String, String, bool)> = Vec::new();
    for chunk in chunks {
        let Some((key, value)) = chunk.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let (value, important) = match value.rsplit_once('!') {
            Some((value, tail)) if tail.trim().eq_ignore_ascii_case("important") => {
                (value.trim(), true)
            }
            _ => (value.trim(), false),
        };
        let declarations = if key == "font" {
            css_font_shorthand(value).unwrap_or_default()
        } else {
            vec![(key, value.into())]
        };
        for (key, value) in declarations {
            if let Some((_, old, priority)) = result.iter_mut().find(|(name, _, _)| *name == key) {
                if !*priority || important {
                    *old = value;
                    *priority = important;
                }
            } else {
                result.push((key, value, important));
            }
        }
    }
    result
        .into_iter()
        .map(|(key, value, _)| (key, value))
        .collect()
}
fn merge_html_css(parent: &str, child: &str) -> String {
    let mut values = css_declarations(parent);
    for (key, value) in css_declarations(child) {
        if let Some((_, old)) = values.iter_mut().find(|(name, _)| *name == key) {
            *old = value;
        } else {
            values.push((key, value));
        }
    }
    values
        .into_iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(";")
}
fn css_font_family(value: &str) -> Option<String> {
    let (mut quote, mut escaped, mut end) = (None, false, value.len());
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(open) = quote {
            if ch == open {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == ',' {
            end = index;
            break;
        }
    }
    let first = value[..end].trim();
    let first = if first.starts_with('"') && first.ends_with('"')
        || first.starts_with('\'') && first.ends_with('\'')
    {
        first.get(1..first.len().saturating_sub(1))?
    } else {
        first
    };
    let mut result = String::new();
    let mut chars = first.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            result.push(chars.next()?);
        } else {
            result.push(ch);
        }
    }
    (!result.is_empty()).then_some(result)
}
fn css_color(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 3 && hex.is_ascii() {
            return color(&hex.chars().flat_map(|ch| [ch, ch]).collect::<String>());
        }
        return color(hex);
    }
    if let Some(body) = value
        .strip_prefix("rgb(")
        .and_then(|body| body.strip_suffix(')'))
    {
        let parts = body.split(',').map(str::trim).collect::<Vec<_>>();
        if parts.len() != 3 {
            return None;
        }
        let mut rgb = [0u8; 3];
        for (slot, part) in rgb.iter_mut().zip(parts) {
            let number = if let Some(percent) = part.strip_suffix('%') {
                percent.parse::<f64>().ok()? * 255.0 / 100.0
            } else {
                part.parse::<f64>().ok()?
            };
            if !number.is_finite() {
                return None;
            }
            *slot = number.round().clamp(0.0, 255.0) as u8;
        }
        return Some(rgb);
    }
    Some(match value.as_str() {
        "black" => [0, 0, 0],
        "silver" => [192, 192, 192],
        "gray" | "grey" => [128, 128, 128],
        "white" => [255, 255, 255],
        "maroon" => [128, 0, 0],
        "red" => [255, 0, 0],
        "purple" => [128, 0, 128],
        "fuchsia" | "magenta" => [255, 0, 255],
        "green" => [0, 128, 0],
        "lime" => [0, 255, 0],
        "olive" => [128, 128, 0],
        "yellow" => [255, 255, 0],
        "navy" => [0, 0, 128],
        "blue" => [0, 0, 255],
        "teal" => [0, 128, 128],
        "aqua" | "cyan" => [0, 255, 255],
        "orange" => [255, 165, 0],
        "rebeccapurple" => [102, 51, 153],
        _ => return None,
    })
}
fn normalize_html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        let special = [("<!--", "-->"), ("<![CDATA[", "]]>")]
            .into_iter()
            .filter_map(|(open, close)| rest.find(open).map(|start| (start, close)))
            .filter(|(start, _)| *start < index)
            .min_by_key(|(start, _)| *start);
        if let Some((start, close)) = special {
            let end = rest[start..]
                .find(close)
                .map(|end| start + end + close.len())
                .unwrap_or(rest.len());
            out.push_str(&rest[..end]);
            rest = &rest[end..];
            continue;
        }
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let end = rest.find(';').filter(|end| *end <= 32);
        let Some(end) = end else {
            out.push_str("&amp;");
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        if name.is_empty()
            || !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '#')
        {
            out.push_str("&amp;");
            rest = &rest[1..];
            continue;
        }
        let decoded = match name {
            "nbsp" => Some(160),
            "copy" => Some(169),
            "reg" => Some(174),
            "trade" => Some(8482),
            "euro" => Some(8364),
            "pound" => Some(163),
            "yen" => Some(165),
            "cent" => Some(162),
            "ndash" => Some(8211),
            "mdash" => Some(8212),
            "hellip" => Some(8230),
            "laquo" => Some(171),
            "raquo" => Some(187),
            "lsquo" => Some(8216),
            "rsquo" => Some(8217),
            "ldquo" => Some(8220),
            "rdquo" => Some(8221),
            "bull" => Some(8226),
            "middot" => Some(183),
            "times" => Some(215),
            "divide" => Some(247),
            "minus" => Some(8722),
            "deg" => Some(176),
            "plusmn" => Some(177),
            "sup2" => Some(178),
            "sup3" => Some(179),
            _ => None,
        };
        if let Some(code) = decoded {
            out.push_str(&format!("&#{code};"));
        } else if matches!(name, "amp" | "lt" | "gt" | "quot" | "apos") || name.starts_with('#') {
            out.push_str(&rest[..=end]);
        } else {
            out.push_str("&amp;");
            out.push_str(&rest[1..=end]);
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}
fn normalize_html_voids(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        if !rest.starts_with('<') {
            let len = rest.find('<').unwrap_or(rest.len());
            out.push_str(&rest[..len]);
            offset += len;
            continue;
        }
        if rest.starts_with("<!--") || rest.starts_with("<![CDATA[") {
            let close = if rest.starts_with("<!--") {
                "-->"
            } else {
                "]]>"
            };
            let len = rest
                .find(close)
                .map(|end| end + close.len())
                .unwrap_or(rest.len());
            out.push_str(&rest[..len]);
            offset += len;
            continue;
        }
        let (mut quote, mut end) = (None, None);
        for (index, ch) in rest.char_indices().skip(1) {
            if let Some(open) = quote {
                if ch == open {
                    quote = None;
                }
                continue;
            }
            if matches!(ch, '\'' | '"') {
                quote = Some(ch);
            } else if ch == '>' {
                end = Some(index);
                break;
            }
        }
        let Some(end) = end else {
            out.push_str(rest);
            break;
        };
        let tag = &rest[..=end];
        let name = tag[1..]
            .trim_start()
            .trim_start_matches('/')
            .split(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '/' | '>'))
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let void = matches!(
            name.as_str(),
            "area"
                | "base"
                | "br"
                | "col"
                | "embed"
                | "hr"
                | "img"
                | "input"
                | "link"
                | "meta"
                | "param"
                | "source"
                | "track"
                | "wbr"
        );
        if void && !tag.starts_with("</") && !tag[..tag.len() - 1].trim_end().ends_with('/') {
            out.push_str(&tag[..tag.len() - 1]);
            out.push_str("/>");
        } else if !void || !tag.starts_with("</") {
            out.push_str(tag);
        }
        offset += end + 1;
    }
    out
}

#[cfg(test)]
mod additional_html_defect_tests {
    use super::*;
    fn html(source: &str) -> Result<Doc, String> {
        read(source.as_bytes(), "html")
    }
    fn first(doc: &Doc) -> Result<&TextFormat, String> {
        doc.paragraphs
            .first()
            .and_then(|p| p.runs.iter().find(|r| !r.text.is_empty()))
            .map(|r| &r.format)
            .ok_or_else(|| "run".into())
    }
    #[test]
    fn quoted_css_separators_cannot_create_other_properties() -> Result<(), String> {
        let doc = html(r#"<p style="font-family:'A; color:red; B';color:blue">text</p>"#)?;
        let format = first(&doc)?;
        assert_eq!(format.font, "A; color:red; B");
        assert_eq!(format.color, [0, 0, 255]);
        Ok(())
    }
    #[test]
    fn html_css_properties_and_keywords_are_case_insensitive() -> Result<(), String> {
        let doc = html(
            r#"<p style="TEXT-ALIGN:RIGHT"><span style="FONT-STYLE:ITALIC;COLOR:RED">text</span></p>"#,
        )?;
        assert_eq!(doc.paragraphs[0].align, Align::Right);
        assert!(first(&doc)?.italic);
        assert_eq!(first(&doc)?.color, [255, 0, 0]);
        Ok(())
    }
    #[test]
    fn important_css_wins_locally_but_does_not_block_child_overrides() -> Result<(), String> {
        let doc = html(
            r#"<div style="font-weight:bold!important"><p style="font-weight:normal;color:red!important;color:blue">text</p></div>"#,
        )?;
        assert!(!first(&doc)?.bold);
        assert_eq!(first(&doc)?.color, [255, 0, 0]);
        Ok(())
    }
    #[test]
    fn html_colors_support_short_hex_rgb_percentages_and_names() -> Result<(), String> {
        for (source, color) in [
            ("#abc", [170, 187, 204]),
            ("rgb(12,34,56)", [12, 34, 56]),
            ("rgb(100%,0%,50%)", [255, 0, 128]),
            ("navy", [0, 0, 128]),
        ] {
            let doc = html(&format!("<p style=\"color:{source}\">text</p>"))?;
            assert_eq!(first(&doc)?.color, color);
        }
        Ok(())
    }
    #[test]
    fn html_absolute_length_units_are_converted_to_points() -> Result<(), String> {
        for (value, expected) in [
            ("1in", 72.0),
            ("2.54cm", 72.0),
            ("25.4mm", 72.0),
            ("2pc", 24.0),
        ] {
            let doc = html(&format!("<p style=\"font-size:{value}\">text</p>"))?;
            assert!((first(&doc)?.size - expected).abs() < 0.01);
        }
        Ok(())
    }
    #[test]
    fn html_numeric_weights_map_to_the_supported_bold_face() -> Result<(), String> {
        for weight in [600, 700, 800, 900] {
            let doc = html(&format!("<p style=\"font-weight:{weight}\">text</p>"))?;
            assert!(first(&doc)?.bold, "{weight}");
        }
        let doc = html("<p style=\"font-weight:400\">text</p>")?;
        assert!(!first(&doc)?.bold);
        Ok(())
    }
    #[test]
    fn html_font_families_select_the_first_family_and_unescape_it() -> Result<(), String> {
        let doc = html(r#"<p style="font-family:'A\'s, Family', sans-serif">text</p>"#)?;
        assert_eq!(first(&doc)?.font, "A's, Family");
        Ok(())
    }
    #[test]
    fn html_inline_data_images_keep_their_paragraph_anchor() -> Result<(), String> {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let doc = html(&format!(
            "<p>before<img src=\"data:image/png;base64,{}\"/>after</p>",
            encode_image_base64(&png.into_inner())
        ))?;
        assert_eq!(doc.plain_text(), "beforeafter");
        assert_eq!(doc.images.len(), 1);
        assert_eq!(doc.images[0].after_paragraph_id, doc.paragraphs[0].id);
        Ok(())
    }
    #[test]
    fn html_nested_lists_restore_model_levels() -> Result<(), String> {
        let doc = html("<ul><li>outer<ol><li>inner</li></ol></li></ul>")?;
        assert_eq!(doc.plain_text(), "outer\ninner");
        assert_eq!(
            (doc.paragraphs[0].list, doc.paragraphs[0].list_level),
            (List::Bullet, 0)
        );
        assert_eq!(
            (doc.paragraphs[1].list, doc.paragraphs[1].list_level),
            (List::Number, 1)
        );
        Ok(())
    }
    #[test]
    fn html_void_tags_with_attributes_keep_rich_content() -> Result<(), String> {
        let doc = html(r#"<p><b>one<br class="soft">two</b></p>"#)?;
        assert_eq!(doc.plain_text(), "one\u{000b}two");
        assert!(first(&doc)?.bold);
        Ok(())
    }
    #[test]
    fn html_fragments_can_contain_multiple_rich_blocks() -> Result<(), String> {
        let doc = html("<p><b>one</b></p><p><i>two</i></p>")?;
        assert_eq!(doc.plain_text(), "one\ntwo");
        assert!(first(&doc)?.bold);
        assert!(doc.paragraphs[1].runs[0].format.italic);
        Ok(())
    }
    #[test]
    fn html_named_entities_and_literal_ampersands_do_not_abort_import() -> Result<(), String> {
        let doc = html("<p><b>&copy; &euro; &mdash; AT&T</b></p>")?;
        assert_eq!(doc.plain_text(), "© € — AT&T");
        assert!(first(&doc)?.bold);
        Ok(())
    }
    #[test]
    fn mixed_case_html_doctype_does_not_discard_formatting() -> Result<(), String> {
        let doc = html("<!DoCtYpE html><html><body><p><b>text</b></p></body></html>")?;
        assert_eq!(doc.plain_text(), "text");
        assert!(first(&doc)?.bold);
        Ok(())
    }
}

fn word_shading(node: &Node) -> Option<[u8; 3]> {
    if node.value() == Some("nil") {
        return None;
    }
    node.attr(if node.value() == Some("solid") {
        "color"
    } else {
        "fill"
    })
    .and_then(color)
}
#[derive(Default)]
struct WordTheme {
    colors: BTreeMap<String, [u8; 3]>,
    major: String,
    minor: String,
}
impl WordTheme {
    fn read(node: &Node) -> Self {
        let mut theme = Self::default();
        if let Some(scheme) = node.descendant("clrScheme") {
            for entry in &scheme.children {
                let rgb = entry
                    .child("srgbClr")
                    .and_then(|node| node.attr("val"))
                    .or_else(|| entry.child("sysClr").and_then(|node| node.attr("lastClr")))
                    .and_then(color);
                if let Some(rgb) = rgb {
                    theme.colors.insert(entry.name.clone(), rgb);
                }
            }
        }
        for (alias, slot) in [
            ("dark1", "dk1"),
            ("light1", "lt1"),
            ("dark2", "dk2"),
            ("light2", "lt2"),
            ("text1", "dk1"),
            ("background1", "lt1"),
            ("text2", "dk2"),
            ("background2", "lt2"),
            ("hyperlink", "hlink"),
            ("followedHyperlink", "folHlink"),
        ] {
            if let Some(rgb) = theme.colors.get(slot).copied() {
                theme.colors.insert(alias.into(), rgb);
            }
        }
        let face = |tag| {
            node.descendant(tag)
                .and_then(|node| node.child("latin"))
                .and_then(|node| node.attr("typeface"))
                .unwrap_or("")
                .to_string()
        };
        theme.major = face("majorFont");
        theme.minor = face("minorFont");
        theme
    }
    fn apply(&self, node: &mut Node) {
        for (destination, slot, tint, shade) in [
            ("val", "themeColor", "themeTint", "themeShade"),
            ("fill", "themeFill", "themeFillTint", "themeFillShade"),
            ("color", "themeColor", "themeTint", "themeShade"),
        ] {
            if (node.name == "color" && destination == "val")
                || (node.name == "shd" && destination != "val")
            {
                let base = node
                    .attr(slot)
                    .and_then(|slot| self.colors.get(slot))
                    .copied();
                if let Some(mut rgb) = base {
                    let amount = |key| {
                        node.attr(key)
                            .and_then(|value| u8::from_str_radix(value, 16).ok())
                            .map(|value| f64::from(value) / 255.0)
                    };
                    if let Some(tint) = amount(tint) {
                        rgb = crate::util::adjust_rgb_luminance(rgb, tint, 1.0 - tint);
                    } else if let Some(shade) = amount(shade) {
                        rgb = crate::util::adjust_rgb_luminance(rgb, shade, 0.0);
                    }
                    node.attrs.insert(destination.into(), hex(rgb));
                }
            }
        }
        if node.name == "rFonts" {
            if let Some(face) = ["asciiTheme", "hAnsiTheme", "eastAsiaTheme"]
                .iter()
                .find_map(|key| node.attr(key))
                .and_then(|kind| {
                    if kind.starts_with("major") {
                        Some(&self.major)
                    } else if kind.starts_with("minor") {
                        Some(&self.minor)
                    } else {
                        None
                    }
                })
                .filter(|face| !face.is_empty())
            {
                node.attrs.insert("ascii".into(), face.clone());
                node.attrs.insert("hAnsi".into(), face.clone());
            }
        }
        for child in &mut node.children {
            self.apply(child);
        }
    }
}
fn docx_part(source: &str, target: &str) -> Option<String> {
    if target.contains(':') || target.contains('\\') || target.starts_with("//") {
        return None;
    }
    let base = source.rsplit_once('/').map(|(base, _)| base).unwrap_or("");
    let raw = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else if base.is_empty() {
        target.to_string()
    } else {
        format!("{base}/{target}")
    };
    let mut parts = Vec::new();
    for component in raw.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(component),
        }
    }
    Some(parts.join("/").to_ascii_lowercase())
}
fn docx_rels_path(source: &str) -> String {
    let (base, name) = source.rsplit_once('/').unwrap_or(("", source));
    if base.is_empty() {
        format!("_rels/{name}.rels")
    } else {
        format!("{base}/_rels/{name}.rels")
    }
}
fn normalize_docx_parts(parts: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let root_rels = parts
        .get("_rels/.rels")
        .map(|bytes| xml(bytes))
        .transpose()?;
    let main = root_rels
        .as_ref()
        .and_then(|node| {
            node.children.iter().find(|node| {
                node.attr("Type")
                    .is_some_and(|kind| kind.ends_with("/officeDocument"))
                    && node.attr("TargetMode") != Some("External")
            })
        })
        .and_then(|node| node.attr("Target"))
        .and_then(|target| docx_part("", target))
        .unwrap_or_else(|| "word/document.xml".into());
    let rels = parts
        .get(&docx_rels_path(&main))
        .map(|bytes| xml(bytes))
        .transpose()?;
    let mut aliases = Vec::new();
    if let Some(core) = root_rels
        .as_ref()
        .and_then(|node| {
            node.children.iter().find(|node| {
                node.attr("Type")
                    .is_some_and(|kind| kind.ends_with("/metadata/core-properties"))
                    && node.attr("TargetMode") != Some("External")
            })
        })
        .and_then(|node| node.attr("Target"))
        .and_then(|target| docx_part("", target))
        .and_then(|path| parts.get(&path))
    {
        aliases.push(("docprops/core.xml".into(), core.clone()));
    }
    if main != "word/document.xml"
        && let Some(bytes) = parts.get(&main)
    {
        aliases.push(("word/document.xml".into(), bytes.clone()));
    }
    let mut normalized = String::from(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    if let Some(rels) = rels {
        for rel in rels
            .children
            .iter()
            .filter(|node| node.name == "Relationship")
        {
            let Some(id) = rel.attr("Id") else {
                continue;
            };
            let kind = rel.attr("Type").unwrap_or("");
            let external = rel.attr("TargetMode") == Some("External");
            let target = rel.attr("Target").unwrap_or("");
            let absolute = if external {
                target.into()
            } else {
                let Some(path) = docx_part(&main, target) else {
                    continue;
                };
                format!("/{path}")
            };
            normalized.push_str(&format!(
                "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"{} />",
                esc(id),
                esc(kind),
                esc(&absolute),
                if external {
                    " TargetMode=\"External\""
                } else {
                    ""
                }
            ));
            if !external {
                let alias = if kind.ends_with("/styles") {
                    Some("word/styles.xml")
                } else if kind.ends_with("/numbering") {
                    Some("word/numbering.xml")
                } else if kind.ends_with("/theme") {
                    Some("word/theme/theme1.xml")
                } else {
                    None
                };
                if let Some(alias) = alias
                    && let Some(bytes) = parts.get(absolute.trim_start_matches('/'))
                {
                    aliases.push((alias.into(), bytes.clone()));
                }
            }
        }
        normalized.push_str("</Relationships>");
        aliases.push((
            "word/_rels/document.xml.rels".into(),
            normalized.into_bytes(),
        ));
    }
    for (path, bytes) in aliases {
        parts.insert(path, bytes);
    }
    Ok(())
}

#[cfg(test)]
mod additional_word_defect_tests {
    use super::*;
    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
    fn document(body: &str, styles: &str, theme: &str) -> Result<Doc, String> {
        let mut parts = vec![(
            "word/document.xml".into(),
            format!("<w:document {NS}><w:body>{body}</w:body></w:document>").into_bytes(),
        )];
        if !styles.is_empty() {
            parts.push((
                "word/styles.xml".into(),
                format!("<w:styles {NS}>{styles}</w:styles>").into_bytes(),
            ));
        }
        if !theme.is_empty() {
            parts.push(("word/theme/theme1.xml".into(), theme.as_bytes().to_vec()));
        }
        read(&write_zip(parts)?, "docx")
    }
    fn run(body: &str, theme: &str) -> Result<TextFormat, String> {
        let doc = document(
            &format!("<w:p><w:r><w:rPr>{body}</w:rPr><w:t>text</w:t></w:r></w:p>"),
            "",
            theme,
        )?;
        Ok(doc.paragraphs[0].runs[0].format.clone())
    }
    fn theme() -> &'static str {
        r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme><a:dk1><a:srgbClr val="112233"/></a:dk1><a:accent1><a:srgbClr val="FF0000"/></a:accent1></a:clrScheme><a:fontScheme><a:majorFont><a:latin typeface="Georgia"/></a:majorFont><a:minorFont><a:latin typeface="Verdana"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#
    }
    #[test]
    fn word_auto_color_uses_the_automatic_black_ink() -> Result<(), String> {
        assert_eq!(run(r#"<w:color w:val="auto"/>"#, "")?.color, [0, 0, 0]);
        Ok(())
    }
    #[test]
    fn word_shading_resets_and_solid_foreground_are_respected() -> Result<(), String> {
        let styles = r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:rPr><w:shd w:val="clear" w:fill="FFFF00"/></w:rPr></w:style>"#;
        let doc = document(
            r#"<w:p><w:r><w:rPr><w:shd w:val="nil"/></w:rPr><w:t>plain</w:t></w:r></w:p>"#,
            styles,
            "",
        )?;
        assert_eq!(doc.paragraphs[0].runs[0].format.highlight, None);
        assert_eq!(
            run(
                r#"<w:shd w:val="solid" w:color="FF0000" w:fill="00FF00"/>"#,
                ""
            )?
            .highlight,
            Some([255, 0, 0])
        );
        Ok(())
    }
    #[test]
    fn word_theme_slots_override_fallback_rgb() -> Result<(), String> {
        assert_eq!(
            run(
                r#"<w:color w:val="0000FF" w:themeColor="accent1"/>"#,
                theme()
            )?
            .color,
            [255, 0, 0]
        );
        assert_eq!(
            run(r#"<w:color w:val="000000" w:themeColor="text1"/>"#, theme())?.color,
            [17, 34, 51]
        );
        Ok(())
    }
    #[test]
    fn word_theme_tint_and_shade_use_luminance_and_tint_precedence() -> Result<(), String> {
        assert_eq!(
            run(
                r#"<w:color w:themeColor="accent1" w:themeShade="BF"/>"#,
                theme()
            )?
            .color,
            [191, 0, 0]
        );
        assert_eq!(
            run(
                r#"<w:color w:themeColor="accent1" w:themeTint="80" w:themeShade="00"/>"#,
                theme()
            )?
            .color,
            [255, 127, 127]
        );
        Ok(())
    }
    #[test]
    fn word_theme_fonts_resolve_major_and_minor_faces() -> Result<(), String> {
        assert_eq!(
            run(r#"<w:rFonts w:asciiTheme="majorHAnsi"/>"#, theme())?.font,
            "Georgia"
        );
        assert_eq!(
            run(r#"<w:rFonts w:asciiTheme="minorHAnsi"/>"#, theme())?.font,
            "Verdana"
        );
        Ok(())
    }
    #[test]
    fn missing_word_style_keeps_the_default_paragraph_format() -> Result<(), String> {
        let doc = document(
            r#"<w:p><w:pPr><w:pStyle w:val="Missing"/></w:pPr><w:r><w:t>text</w:t></w:r></w:p>"#,
            r#"<w:style w:type="paragraph" w:default="1" w:styleId="Base"><w:rPr><w:b/></w:rPr></w:style>"#,
            "",
        )?;
        assert!(doc.paragraphs[0].runs[0].format.bold);
        Ok(())
    }
    #[test]
    fn word_table_content_controls_keep_rows_and_cells() -> Result<(), String> {
        let doc = document(
            "<w:tbl><w:sdt><w:sdtContent><w:tr><w:sdt><w:sdtContent><w:tc><w:p><w:r><w:t>kept</w:t></w:r></w:p></w:tc></w:sdtContent></w:sdt></w:tr></w:sdtContent></w:sdt></w:tbl>",
            "",
            "",
        )?;
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(
            doc.tables[0].rows[0][0]
                .paragraphs
                .iter()
                .map(Paragraph::text)
                .collect::<Vec<_>>()
                .join("\n"),
            "kept"
        );
        Ok(())
    }
    #[test]
    fn word_table_cell_fill_inherits_the_table_style() -> Result<(), String> {
        let doc = document(
            r#"<w:tbl><w:tblPr><w:tblStyle w:val="Tinted"/></w:tblPr><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl>"#,
            r#"<w:style w:type="table" w:styleId="Tinted"><w:tcPr><w:shd w:val="clear" w:fill="AABBCC"/></w:tcPr></w:style>"#,
            "",
        )?;
        assert_eq!(doc.tables[0].rows[0][0].fill, [170, 187, 204]);
        Ok(())
    }
    #[test]
    fn word_cell_widths_supply_a_missing_table_grid() -> Result<(), String> {
        let doc = document(
            r#"<w:tbl><w:tr><w:tc><w:tcPr><w:tcW w:type="dxa" w:w="720"/></w:tcPr><w:p/></w:tc><w:tc><w:tcPr><w:tcW w:type="dxa" w:w="1440"/></w:tcPr><w:p/></w:tc></w:tr></w:tbl>"#,
            "",
            "",
        )?;
        assert_eq!(doc.tables[0].column_widths, vec![36.0, 72.0]);
        Ok(())
    }
    #[test]
    fn docx_part_roles_are_resolved_from_relationships() -> Result<(), String> {
        let rels = |body: &str| {
            format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>").into_bytes()
        };
        let parts=vec![
            ("_rels/.rels".into(),rels(r#"<Relationship Id="doc" Type="http://x/officeDocument" Target="content/main.xml"/>"#)),
            ("content/main.xml".into(),format!("<w:document {NS}><w:body><w:p><w:pPr><w:pStyle w:val=\"Base\"/></w:pPr><w:r><w:t>text</w:t></w:r></w:p></w:body></w:document>").into_bytes()),
            ("content/_rels/main.xml.rels".into(),rels(r#"<Relationship Id="styles" Type="http://x/styles" Target="../looks/s.xml"/><Relationship Id="theme" Type="http://x/theme" Target="../looks/t.xml"/>"#)),
            ("looks/s.xml".into(),format!("<w:styles {NS}><w:style w:type=\"paragraph\" w:styleId=\"Base\"><w:rPr><w:color w:themeColor=\"accent1\"/><w:b/></w:rPr></w:style></w:styles>").into_bytes()),
            ("looks/t.xml".into(),theme().as_bytes().to_vec()),
        ];
        let doc = read(&write_zip(parts)?, "docx")?;
        assert_eq!(doc.plain_text(), "text");
        assert!(doc.paragraphs[0].runs[0].format.bold);
        assert_eq!(doc.paragraphs[0].runs[0].format.color, [255, 0, 0]);
        Ok(())
    }
    #[test]
    fn default_word_table_style_applies_without_an_explicit_reference() -> Result<(), String> {
        let doc = document(
            "<w:tbl><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl>",
            r#"<w:style w:type="table" w:default="1" w:styleId="DefaultTable"><w:tcPr><w:shd w:val="clear" w:fill="010203"/></w:tcPr></w:style>"#,
            "",
        )?;
        assert_eq!(doc.tables[0].rows[0][0].fill, [1, 2, 3]);
        Ok(())
    }
}

fn normalize_docx_part_names(
    parts: BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut normalized = BTreeMap::new();
    for (path, bytes) in parts {
        if normalized
            .insert(path.to_ascii_lowercase(), bytes)
            .is_some()
        {
            return Err("Повторяющиеся без учёта регистра имена частей DOCX".into());
        }
    }
    Ok(normalized)
}
#[cfg(test)]
mod additional_part_name_tests {
    use super::*;
    #[test]
    fn docx_part_identity_is_ascii_case_insensitive_and_unambiguous() -> Result<(), String> {
        let parts=vec![
            ("WORD/DOCUMENT.XML".into(),br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>kept</w:t></w:r></w:p></w:body></w:document>"#.to_vec()),
            ("WORD/_RELS/DOCUMENT.XML.RELS".into(),br#"<Relationships><Relationship Id="s" Type="http://x/styles" Target="STYLES.XML"/></Relationships>"#.to_vec()),
            ("word/styles.xml".into(),br#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Base"><w:rPr><w:b/></w:rPr></w:style></w:styles>"#.to_vec()),
        ];
        let doc = read(&write_zip(parts)?, "docx")?;
        assert_eq!(doc.plain_text(), "kept");
        assert!(doc.paragraphs[0].runs[0].format.bold);
        assert!(
            normalize_docx_part_names(BTreeMap::from([("A".into(), vec![]), ("a".into(), vec![])]))
                .is_err()
        );
        Ok(())
    }
}

fn markdown_destination(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let (target, tail) = if let Some(rest) = raw.strip_prefix('<') {
        let end = rest.find('>')?;
        (&rest[..end], rest[end + 1..].trim())
    } else {
        let mut escaped = false;
        let mut split = raw.len();
        for (index, ch) in raw.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch.is_whitespace() {
                split = index;
                break;
            }
        }
        (&raw[..split], raw[split..].trim())
    };
    if !tail.is_empty() {
        let quote = tail.chars().next()?;
        let close = if quote == '(' { ')' } else { quote };
        if !matches!(quote, '"' | '\'' | '(') || !tail.ends_with(close) || tail.len() < 2 {
            return None;
        }
    }
    let mut out = String::new();
    let mut chars = target.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek().is_some_and(char::is_ascii_punctuation) {
            out.push(chars.next()?);
        } else {
            out.push(ch);
        }
    }
    Some(
        quick_xml::escape::unescape(&out)
            .map(|value| value.into_owned())
            .unwrap_or(out),
    )
}
#[cfg(test)]
mod next_markdown_defect_tests {
    use super::*;
    fn md(source: &str) -> Result<Doc, String> {
        read(source.as_bytes(), "md")
    }
    #[test]
    fn markdown_code_spans_keep_literal_markers_and_monospace_text() -> Result<(), String> {
        let doc = md("before `` a`*b* `` after")?;
        assert_eq!(doc.plain_text(), "before a`*b* after");
        let run = doc.paragraphs[0]
            .runs
            .iter()
            .find(|run| run.text == "a`*b*")
            .ok_or("code")?;
        assert_eq!(run.format.font, "Consolas");
        assert!(!run.format.italic);
        Ok(())
    }
    #[test]
    fn fenced_and_indented_code_blocks_do_not_parse_inline_formatting() -> Result<(), String> {
        let doc = md("```rust\n**literal**\n# still code\n```\n\n    *literal*\n")?;
        assert_eq!(doc.paragraphs.len(), 2);
        assert_eq!(doc.paragraphs[0].text(), "**literal**\u{000b}# still code");
        assert_eq!(doc.paragraphs[1].text(), "*literal*");
        assert!(
            doc.paragraphs
                .iter()
                .all(|paragraph| paragraph.style == "Code" && !paragraph.runs[0].format.bold)
        );
        Ok(())
    }
    #[test]
    fn setext_underlines_create_headings_instead_of_text_paragraphs() -> Result<(), String> {
        let doc = md("Title\n=====\n\nSection\n---\n")?;
        assert_eq!(doc.paragraphs.len(), 2);
        assert_eq!(doc.paragraphs[0].style, "Heading1");
        assert_eq!(doc.paragraphs[1].style, "Heading2");
        Ok(())
    }
    #[test]
    fn markdown_paragraphs_join_soft_lines_and_ignore_separator_blank_lines() -> Result<(), String>
    {
        let doc = md("one\nwrapped line\n\nnext\n")?;
        assert_eq!(doc.plain_text(), "one wrapped line\nnext");
        assert_eq!(doc.paragraphs.len(), 2);
        Ok(())
    }
    #[test]
    fn backslash_hard_breaks_remain_inside_the_paragraph() -> Result<(), String> {
        let doc = md("one\\\ntwo")?;
        assert_eq!(doc.plain_text(), "one\u{000b}two");
        assert_eq!(doc.paragraphs.len(), 1);
        Ok(())
    }
    #[test]
    fn atx_headings_accept_tabs_and_strip_only_valid_closing_hashes() -> Result<(), String> {
        let doc = md("#\tTitle ###\n\n## text#\n")?;
        assert_eq!(doc.paragraphs[0].style, "Heading1");
        assert_eq!(doc.paragraphs[0].text(), "Title");
        assert_eq!(doc.paragraphs[1].text(), "text#");
        Ok(())
    }
    #[test]
    fn external_list_markers_and_nested_indentation_are_recognized() -> Result<(), String> {
        let doc = md("+ outer\n  - inner\n2) next")?;
        assert_eq!(doc.paragraphs.len(), 3);
        assert_eq!(doc.paragraphs[0].list, List::Bullet);
        assert_eq!(doc.paragraphs[1].list_level, 1);
        assert_eq!(doc.paragraphs[2].list, List::Number);
        assert_eq!(doc.paragraphs[2].list_level, 0);
        Ok(())
    }
    #[test]
    fn markdown_link_titles_and_angle_destinations_are_not_part_of_the_url() -> Result<(), String> {
        let doc = md("[label](<https://example.com/a> \"title\")")?;
        assert_eq!(doc.plain_text(), "label");
        assert_eq!(
            doc.paragraphs[0].runs[0].format.link.as_deref(),
            Some("https://example.com/a")
        );
        Ok(())
    }
    #[test]
    fn explicit_uri_autolinks_become_clickable_without_angle_brackets() -> Result<(), String> {
        let doc = md("<https://example.com/a>")?;
        assert_eq!(doc.plain_text(), "https://example.com/a");
        assert_eq!(
            doc.paragraphs[0].runs[0].format.link.as_deref(),
            Some("https://example.com/a")
        );
        Ok(())
    }
}

fn markdown_code_end(text: &str, length: usize, budget: &mut usize) -> Result<Option<usize>, ()> {
    let mut offset = length;
    while offset < text.len() {
        let tail = &text[offset..];
        let found = tail.find('`');
        let distance = found.unwrap_or(tail.len());
        if !md_spend(budget, distance + 1) {
            return Err(());
        }
        let Some(found) = found else {
            return Ok(None);
        };
        offset += found;
        let run = text[offset..].bytes().take_while(|ch| *ch == b'`').count();
        if !md_spend(budget, run) {
            return Err(());
        }
        if run == length {
            return Ok(Some(offset));
        }
        offset += run;
    }
    Ok(None)
}

fn markdown_table_row(line: &str) -> Option<Vec<String>> {
    if !line.contains('|') {
        return None;
    }
    let mut result = Vec::new();
    let mut value = String::new();
    let mut escaped = false;
    for ch in line.trim().chars() {
        if ch == '|' && !escaped {
            result.push(std::mem::take(&mut value));
        } else {
            value.push(ch);
        }
        if ch == '\\' {
            escaped = !escaped;
        } else {
            escaped = false;
        }
    }
    result.push(value);
    if result.first().is_some_and(String::is_empty) {
        result.remove(0);
    }
    if result.last().is_some_and(String::is_empty) {
        result.pop();
    }
    Some(
        result
            .into_iter()
            .map(|value| value.trim().to_string())
            .collect(),
    )
}
fn markdown_table_separator(line: &str) -> Option<Vec<Align>> {
    markdown_table_row(line)?
        .into_iter()
        .map(|value| {
            let value = value.trim();
            let hyphens = value.trim_start_matches(':').trim_end_matches(':');
            if hyphens.len() < 3 || !hyphens.chars().all(|ch| ch == '-') {
                return None;
            }
            Some(if value.starts_with(':') && value.ends_with(':') {
                Align::Center
            } else if value.ends_with(':') {
                Align::Right
            } else {
                Align::Left
            })
        })
        .collect()
}
fn markdown_inline_images(text: &str) -> (String, Vec<(String, Vec<u8>)>) {
    let mut visible = String::new();
    let mut images = Vec::new();
    let mut rest = text;
    let mut budget = text.len().saturating_mul(16).clamp(4096, 16 * 1024 * 1024);
    while !rest.is_empty() {
        if !md_spend(&mut budget, 1) {
            visible.push_str(rest);
            break;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if ch == '\\'
            && let Some(next) = rest.chars().nth(1)
        {
            let length = 1 + next.len_utf8();
            visible.push_str(&rest[..length]);
            rest = &rest[length..];
            continue;
        }
        if ch == '`' {
            let length = rest.bytes().take_while(|ch| *ch == b'`').count();
            match markdown_code_end(rest, length, &mut budget) {
                Ok(Some(end)) => {
                    visible.push_str(&rest[..end + length]);
                    rest = &rest[end + length..];
                    continue;
                }
                Err(()) => {
                    visible.push_str(rest);
                    break;
                }
                _ => {
                    visible.push_str(&rest[..length]);
                    rest = &rest[length..];
                    continue;
                }
            }
        }
        if rest.starts_with("![") {
            let close = match md_unescaped(rest, "](", &mut budget) {
                Ok(close) => close,
                Err(()) => {
                    visible.push_str(rest);
                    break;
                }
            };
            if let Some(close) = close {
                let destination = &rest[close + 2..];
                let end = destination.find(')');
                if !md_spend(&mut budget, end.unwrap_or(destination.len())) {
                    visible.push_str(rest);
                    break;
                }
                if let Some(end) = end
                    && let Some(bytes) = markdown_destination(&destination[..end])
                        .as_deref()
                        .and_then(decode_html_image)
                {
                    let alt = inline_md(&rest[2..close], &TextFormat::default())
                        .into_iter()
                        .map(|run| run.text)
                        .collect::<String>();
                    images.push((alt, bytes));
                    rest = &rest[close + 2 + end + 1..];
                    continue;
                }
            }
        }
        visible.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    (visible, images)
}
fn document_plain_export(doc: &Doc) -> String {
    let mut blocks = Vec::new();
    for paragraph in &doc.paragraphs {
        blocks.push(paragraph.text());
        for table in doc
            .tables
            .iter()
            .filter(|table| table.after_paragraph_id == paragraph.id)
        {
            for row in &table.rows {
                let mut cells = Vec::new();
                for cell in row {
                    cells.push(if cell.merged {
                        String::new()
                    } else {
                        cell_text(cell)
                    });
                    for _ in 1..cell.span() {
                        cells.push(String::new());
                    }
                }
                blocks.push(cells.join("\t"));
            }
        }
    }
    blocks.join("\n")
}
#[cfg(test)]
mod next_markdown_table_image_tests {
    use super::*;
    #[test]
    fn gfm_tables_import_structure_escaped_pipes_breaks_and_alignment() -> Result<(), String> {
        let doc = read(b"|left|right|\n|:---|---:|\n|a\\|b|x<br>y|\n", "md")?;
        let table = doc.tables.first().ok_or("table")?;
        assert_eq!(table.rows.len(), 2);
        assert_eq!(cell_text(&table.rows[1][0]), "a|b");
        assert_eq!(cell_text(&table.rows[1][1]), "x\u{000b}y");
        assert_eq!(table.rows[1][1].paragraphs[0].align, Align::Right);
        Ok(())
    }
    #[test]
    fn markdown_export_keeps_data_images_and_import_restores_their_anchor() -> Result<(), String> {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let mut doc = Doc::from_text("anchor");
        let id = doc.alloc_id();
        doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: doc.paragraphs[0].id,
            bytes: png.into_inner(),
            width: 1.5,
            height: 1.5,
            alt: "icon".into(),
        });
        let imported = read(&write(&doc, "md")?, "md")?;
        assert_eq!(imported.images.len(), 1);
        assert_eq!(imported.images[0].bytes, doc.images[0].bytes);
        assert_eq!(imported.images[0].alt, "icon");
        assert_eq!(imported.paragraphs.len(), 1);
        assert_eq!(
            imported.images[0].after_paragraph_id,
            imported.paragraphs[0].id
        );
        Ok(())
    }
    #[test]
    fn txt_export_keeps_grid_columns_after_spanning_cells() -> Result<(), String> {
        let mut doc = Doc::from_text("before\nafter");
        let id = doc.add_table(doc.paragraphs[0].id, 1, 3)?;
        doc.set_cell_text(id, 0, 0, "one")?;
        doc.set_cell_text(id, 0, 2, "two")?;
        doc.tables[0].rows[0].remove(1);
        doc.tables[0].rows[0][0].span = 2;
        let output = String::from_utf8(write(&doc, "txt")?).map_err(|e| e.to_string())?;
        assert_eq!(output, "before\none\t\ttwo\nafter");
        Ok(())
    }
}

fn inherited_html_css(css: &str) -> String {
    css_declarations(css)
        .into_iter()
        .filter(|(key, _)| {
            matches!(
                key.as_str(),
                "font-size"
                    | "font-family"
                    | "font-weight"
                    | "font-style"
                    | "color"
                    | "text-align"
                    | "text-indent"
                    | "line-height"
                    | "widows"
                    | "orphans"
                    | "white-space"
                    | "text-decoration"
                    | "text-decoration-line"
            )
        })
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(";")
}
#[cfg(test)]
mod next_html_semantic_tests {
    use super::*;
    #[test]
    fn html_container_margins_do_not_become_each_child_paragraphs_margins() -> Result<(), String> {
        let doc=read(br#"<div style="margin-left:30pt;margin-bottom:40pt;color:red"><p>one</p><p>two</p></div>"#,"html")?;
        assert_eq!(doc.paragraphs.len(), 2);
        assert!(
            doc.paragraphs
                .iter()
                .all(|paragraph| paragraph.indent == 0.0 && paragraph.spacing_after != 40.0)
        );
        assert_eq!(doc.paragraphs[0].runs[0].format.color, [255, 0, 0]);
        Ok(())
    }
    #[test]
    fn html_pre_and_code_elements_keep_monospace_format_and_line_boundaries() -> Result<(), String>
    {
        let doc = read(b"<pre>one\n  two</pre><p><code>a*b</code></p>", "html")?;
        assert_eq!(doc.paragraphs[0].style, "Code");
        assert_eq!(doc.paragraphs[0].text(), "one\n  two");
        assert_eq!(doc.paragraphs[1].runs[0].format.font, "Consolas");
        Ok(())
    }
    #[test]
    fn html_import_does_not_claim_that_preserved_native_tables_were_dropped() -> Result<(), String>
    {
        let mut doc = Doc::from_text("anchor");
        doc.add_table(doc.paragraphs[0].id, 1, 1)?;
        let imported = read(&write(&doc, "html")?, "html")?;
        assert_eq!(imported.tables.len(), 1);
        assert!(
            !imported
                .losses
                .iter()
                .any(|message| message.contains("таблицы преобразуются в абзацы"))
        );
        Ok(())
    }
}

#[cfg(test)]
mod next_decode_limit_tests {
    #[test]
    fn docs_legacy_decoder_rejects_the_undefined_windows1251_byte() {
        assert!(super::read(&[0x98], "txt").is_err());
    }
    #[test]
    fn oversized_html_is_rejected_before_plain_text_fallback() {
        let html = format!("<p><b>kept</b></p><!--{}-->", " ".repeat(256));
        assert!(super::read_html_with_limit(&html, 64).is_err());
    }
}

fn html_font_size(value: &str, parent: f32) -> f32 {
    let value = value.trim().to_ascii_lowercase();
    let relative = value
        .strip_suffix('%')
        .map(|number| (number, 0.01))
        .or_else(|| value.strip_suffix("em").map(|number| (number, 1.0)));
    if let Some((number, factor)) = relative {
        return number
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite() && *number > 0.0)
            .map(|number| (parent * number * factor).clamp(4.0, 144.0))
            .unwrap_or(parent);
    }
    css_points(&value, parent).clamp(4.0, 144.0)
}
fn html_append_run(out: &mut Vec<Run>, text: String, format: &TextFormat) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut().filter(|last| last.format == *format) {
        last.text.push_str(&text);
    } else {
        out.push(Run {
            text,
            format: format.clone(),
        });
    }
}
fn html_inline_text(
    text: &str,
    format: &TextFormat,
    preserve: bool,
    out: &mut Vec<Run>,
    pending: &mut Option<TextFormat>,
) {
    let flush = |out: &mut Vec<Run>, pending: &mut Option<TextFormat>| {
        if let Some(format) = pending.take()
            && out
                .last()
                .is_some_and(|run| !run.text.ends_with('\u{000b}'))
        {
            html_append_run(out, " ".into(), &format);
        }
    };
    if preserve {
        flush(out, pending);
        html_append_run(out, text.into(), format);
        return;
    }
    let mut chunk = String::new();
    for ch in text.chars() {
        if matches!(ch, ' ' | '\t' | '\r' | '\n' | '\u{000c}') {
            html_append_run(out, std::mem::take(&mut chunk), format);
            pending.get_or_insert_with(|| format.clone());
        } else {
            flush(out, pending);
            chunk.push(ch);
        }
    }
    html_append_run(out, chunk, format);
}

fn html_computed_css(parent: &str, child: &str) -> String {
    let parent_size = css_declarations(parent)
        .iter()
        .find(|(key, _)| key == "font-size")
        .map(|(_, value)| html_font_size(value, 11.0))
        .unwrap_or(11.0);
    let merged = merge_html_css(parent, child);
    let size = css_declarations(&merged)
        .into_iter()
        .find(|(key, _)| key == "font-size")
        .map(|(_, value)| html_font_size(&value, parent_size))
        .unwrap_or(parent_size);
    // Store computed relative lengths so inheritance does not apply the percentage twice.
    let mut computed = if css_declarations(&merged)
        .iter()
        .any(|(key, _)| key == "font-size")
    {
        merge_html_css(&merged, &format!("font-size:{size}pt"))
    } else {
        merged.clone()
    };
    if let Some(value) = css_declarations(&merged)
        .into_iter()
        .find(|(key, _)| key == "line-height")
        .map(|(_, value)| value.to_ascii_lowercase())
    {
        let relative = value
            .trim()
            .strip_suffix('%')
            .map(|number| (number, 0.01))
            .or_else(|| value.trim().strip_suffix("em").map(|number| (number, 1.0)));
        if let Some((number, factor)) = relative
            && let Some(number) = number
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|number| number.is_finite() && *number > 0.0)
        {
            computed = merge_html_css(
                &computed,
                &format!(
                    "line-height:{}pt",
                    (size * number * factor).clamp(1.0, 1584.0)
                ),
            );
        }
    }
    computed
}
fn html_image_geometry(node: &Node, intrinsic_width: f32, intrinsic_height: f32) -> (f32, f32) {
    let attribute = |name: &str| {
        node.attr(name)
            .and_then(|value| value.trim().parse::<f32>().ok())
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| value * 0.75)
    };
    let mut width = attribute("width");
    let mut height = attribute("height");
    for (key, value) in css_declarations(node.attr("style").unwrap_or("")) {
        if matches!(key.as_str(), "width" | "height") {
            let size = css_points(&value, f32::NAN);
            if size.is_finite() && size > 0.0 {
                if key == "width" {
                    width = Some(size);
                } else {
                    height = Some(size);
                }
            }
        }
    }
    if let Some(value) = node
        .attr("data-trinedocs-width")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        width = Some(value);
    }
    if let Some(value) = node
        .attr("data-trinedocs-height")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        height = Some(value);
    }
    let intrinsic_width = f64::from(intrinsic_width).max(f64::from(f32::MIN_POSITIVE));
    let intrinsic_height = f64::from(intrinsic_height).max(f64::from(f32::MIN_POSITIVE));
    let (width, height) = match (width.map(f64::from), height.map(f64::from)) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => (width, width * intrinsic_height / intrinsic_width),
        (None, Some(height)) => (height * intrinsic_width / intrinsic_height, height),
        (None, None) => (intrinsic_width, intrinsic_height),
    };
    let fit = (10000.0 / width).min(10000.0 / height).min(1.0);
    (
        (width * fit).max(0.001) as f32,
        (height * fit).max(0.001) as f32,
    )
}

fn word_grid_before(row: &Node) -> Result<usize, String> {
    let Some(value) = row
        .child("trPr")
        .and_then(|properties| properties.child("gridBefore"))
        .and_then(Node::value)
    else {
        return Ok(0);
    };
    let count = value
        .parse::<usize>()
        .map_err(|_| "Некорректный gridBefore DOCX")?;
    if count > 256 {
        return Err("gridBefore DOCX превышает256 столбцов".into());
    }
    Ok(count)
}
fn word_number_kind(format: Option<&str>) -> List {
    match format {
        Some("none") => List::None,
        Some("bullet") => List::Bullet,
        _ => List::Number,
    }
}

fn markdown_link_end(text: &str, start: usize, budget: &mut usize) -> Result<Option<usize>, ()> {
    let mut nesting = 0;
    let mut escaped = false;
    let mut angle = false;
    let mut quote = None;
    let mut first = true;
    let mut space = false;
    for (index, ch) in text[start..].char_indices() {
        if !md_spend(budget, ch.len_utf8()) {
            return Err(());
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if angle {
            if ch == '>' {
                angle = false;
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            }
            continue;
        }
        if first && ch.is_whitespace() {
            continue;
        }
        if first && ch == '<' {
            first = false;
            angle = true;
            continue;
        }
        first = false;
        if space && nesting == 0 && matches!(ch, '\'' | '"') {
            quote = Some(ch);
            space = false;
            continue;
        }
        match ch {
            '(' => nesting += 1,
            ')' if nesting == 0 => return Ok(Some(start + index)),
            ')' => nesting -= 1,
            _ => {}
        }
        space = ch.is_whitespace();
    }
    Ok(None)
}
fn markdown_emphasis_end(
    text: &str,
    marker: &str,
    budget: &mut usize,
) -> Result<Option<usize>, ()> {
    let mut offset = 0;
    while let Some(relative) = md_unescaped(&text[offset..], marker, budget)? {
        let end = offset + relative;
        let before = text[..end].chars().next_back();
        let after = text[end + marker.len()..].chars().next();
        if before.is_some_and(|ch| !ch.is_whitespace())
            && !(marker.starts_with('_')
                && before.is_some_and(char::is_alphanumeric)
                && after.is_some_and(char::is_alphanumeric))
        {
            return Ok(Some(end));
        }
        offset = end + marker.len();
    }
    Ok(None)
}

type HtmlWalker = fn(&Node, &mut Doc, &str, Option<(List, u8)>) -> Result<(), String>;
fn html_table_rows<'a>(node: &'a Node, out: &mut Vec<(&'a Node, bool, usize)>, header: bool) {
    let mut direct_start = out.len();
    for child in &node.children {
        if child.name == "tr" {
            out.push((child, header, 0));
        } else if matches!(child.name.as_str(), "thead" | "tbody" | "tfoot") {
            let end = out.len();
            for (_, _, group_end) in &mut out[direct_start..end] {
                *group_end = end;
            }
            html_table_rows(child, out, child.name == "thead");
            direct_start = out.len();
        }
    }
    let end = out.len();
    for (_, _, group_end) in &mut out[direct_start..end] {
        *group_end = end;
    }
}
fn html_span(node: &Node, name: &str, maximum: usize) -> Result<usize, String> {
    let Some(value) = node.attr(name) else {
        return Ok(1);
    };
    let count = value
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("HTML: некорректный {name}"))?;
    if count == 0 || count > maximum {
        return Err(format!("HTML: {name} превышает безопасный диапазон"));
    }
    Ok(count)
}
fn html_row_span(node: &Node, remaining: usize) -> Result<usize, String> {
    let Some(value) = node.attr("rowspan") else {
        return Ok(1);
    };
    let count = value
        .trim()
        .parse::<usize>()
        .map_err(|_| "HTML: некорректный rowspan")?;
    // rowspan=0 covers the remaining rows of this row group, never a later tbody.
    Ok(if count == 0 {
        remaining
    } else {
        count.min(remaining)
    })
}
fn html_import_table(
    node: &Node,
    doc: &mut Doc,
    css: &str,
    walk: HtmlWalker,
) -> Result<(), String> {
    if doc.tables.len() >= 1000 {
        return Err("HTML: превышен лимит1000 таблиц".into());
    }
    let mut source = Vec::new();
    html_table_rows(node, &mut source, false);
    if source.is_empty() {
        return Ok(());
    }
    if source.len() > 10000 {
        return Err("HTML: таблица превышает10000 строк".into());
    }
    let mut budget = doc
        .tables
        .iter()
        .flat_map(|table| &table.rows)
        .map(Vec::len)
        .sum::<usize>();
    let mut active = vec![None::<(usize, usize)>; 256];
    let mut rows = Vec::new();
    let mut row_heights = Vec::new();
    let mut header_rows = 0;
    let mut borderless = true;
    let mut border_color = [0, 0, 0];
    let mut margin = WORD_CELL_MARGIN;
    let mut first_cell = true;
    for (ri, (row_node, in_header, group_end)) in source.iter().enumerate() {
        let nodes = row_node
            .children
            .iter()
            .filter(|child| matches!(child.name.as_str(), "td" | "th"))
            .collect::<Vec<_>>();
        if nodes.len() > 256 {
            return Err("HTML: таблица превышает256 столбцов".into());
        }
        if ri == header_rows
            && (*in_header || !nodes.is_empty() && nodes.iter().all(|cell| cell.name == "th"))
        {
            header_rows += 1;
        }
        let mut row = Vec::new();
        let mut column = 0;
        let mut cells = nodes.into_iter().peekable();
        let mut height = 0.0_f32;
        loop {
            let continued = active
                .get(column)
                .copied()
                .flatten()
                .filter(|(until, _)| *until > ri);
            if let Some((_, span)) = continued {
                if span == 0 {
                    return Err("HTML: пересечение объединений таблицы".into());
                }
                let paragraph = Paragraph {
                    id: doc.alloc_id(),
                    ..Default::default()
                };
                let mut cell = Cell::new(doc.alloc_id(), vec![paragraph], [255, 255, 255]);
                cell.span = span as u16;
                cell.merged = true;
                row.push(cell);
                column += span;
            } else if let Some(cell_node) = cells.next() {
                let span = html_span(cell_node, "colspan", 256)?;
                if column + span > 256
                    || active[column..column + span]
                        .iter()
                        .any(|slot| slot.is_some_and(|(until, _)| until > ri))
                {
                    return Err("HTML: пересечение объединений или больше256 столбцов".into());
                }
                let row_span = html_row_span(cell_node, group_end.saturating_sub(ri).max(1))?;
                let cell_css = html_computed_css(
                    &inherited_html_css(css),
                    cell_node.attr("style").unwrap_or(""),
                );
                let mut nested = Doc {
                    paragraphs: Vec::new(),
                    next_id: doc.next_id,
                    page: doc.page.clone(),
                    ..Default::default()
                };
                walk(cell_node, &mut nested, css, None)?;
                doc.next_id = nested.next_id;
                doc.losses.extend(nested.losses);
                if !nested.tables.is_empty() || !nested.images.is_empty() {
                    doc.losses.push("HTML: вложенные таблицы и изображения ячеек не представлены в модели ячейки.".into());
                }
                if nested.paragraphs.is_empty() {
                    nested.paragraphs.push(Paragraph {
                        id: doc.alloc_id(),
                        ..Default::default()
                    });
                }
                let mut cell = Cell::new(doc.alloc_id(), nested.paragraphs, [255, 255, 255]);
                cell.span = span as u16;
                let mut cell_padding = None;
                for (key, value) in css_declarations(&cell_css) {
                    match key.as_str() {
                        "background" | "background-color" => {
                            if let Some(color) = css_color(&value) {
                                cell.fill = color;
                            }
                        }
                        "vertical-align" => {
                            cell.v_align = match value.trim().to_ascii_lowercase().as_str() {
                                "bottom" => VAlign::Bottom,
                                "middle" => VAlign::Center,
                                _ => VAlign::Top,
                            }
                        }
                        "height" => height = height.max(css_points(&value, 0.0).clamp(0.0, 1500.0)),
                        "padding" => {
                            let values = value
                                .split_whitespace()
                                .map(|part| css_points(part, 0.0).clamp(0.0, 72.0))
                                .collect::<Vec<_>>();
                            cell_padding = match values.as_slice() {
                                [a] => Some([*a; 4]),
                                [a, b] => Some([*a, *b, *a, *b]),
                                [a, b, c] => Some([*a, *b, *c, *b]),
                                [a, b, c, d] => Some([*a, *b, *c, *d]),
                                _ => None,
                            };
                        }
                        "border" => {
                            if !value.split_whitespace().any(|word| {
                                word.eq_ignore_ascii_case("none")
                                    || word.eq_ignore_ascii_case("hidden")
                            }) && !value.trim().starts_with('0')
                            {
                                borderless = false;
                                if let Some(color) = value.split_whitespace().find_map(css_color) {
                                    border_color = color;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if first_cell {
                    if let Some(padding) = cell_padding {
                        margin = padding;
                    }
                    first_cell = false;
                }
                for (offset, slot) in active.iter_mut().skip(column).take(span).enumerate() {
                    *slot = (row_span > 1)
                        .then_some((ri + row_span, if offset == 0 { span } else { 0 }));
                }
                row.push(cell);
                column += span;
            } else if let Some(next) = active
                .iter()
                .enumerate()
                .skip(column)
                .find_map(|(index, slot)| slot.filter(|(until, _)| *until > ri).map(|_| index))
            {
                if next > column {
                    let paragraph = Paragraph {
                        id: doc.alloc_id(),
                        ..Default::default()
                    };
                    let mut cell = Cell::new(doc.alloc_id(), vec![paragraph], [255, 255, 255]);
                    cell.span = (next - column) as u16;
                    row.push(cell);
                    column = next;
                }
            } else {
                break;
            }
            budget = budget.saturating_add(1);
            if budget > 100000 {
                return Err("HTML: таблицы превышают100000 ячеек".into());
            }
        }
        if row.is_empty() {
            let paragraph = Paragraph {
                id: doc.alloc_id(),
                ..Default::default()
            };
            row.push(Cell::new(doc.alloc_id(), vec![paragraph], [255, 255, 255]));
            budget += 1;
            if budget > 100000 {
                return Err("HTML: таблицы превышают100000 ячеек".into());
            }
        }
        for (key, value) in css_declarations(row_node.attr("style").unwrap_or("")) {
            if key == "height" {
                height = height.max(css_points(&value, 0.0).clamp(0.0, 1500.0));
            }
        }
        rows.push(row);
        row_heights.push(height);
    }
    if doc.paragraphs.is_empty() {
        let id = doc.alloc_id();
        doc.paragraphs.push(Paragraph {
            id,
            ..Default::default()
        });
    }
    let anchor = doc
        .paragraphs
        .last()
        .map(|paragraph| paragraph.id)
        .unwrap_or_default();
    let columns = rows
        .iter()
        .map(|row| row.iter().map(Cell::span).sum::<usize>())
        .max()
        .unwrap_or(1);
    let fallback = (doc.page.width - doc.page.margin_left - doc.page.margin_right) / columns as f32;
    let mut widths = Vec::new();
    for group in node
        .children
        .iter()
        .filter(|child| child.name == "colgroup")
    {
        for column in group.children.iter().filter(|child| child.name == "col") {
            let mut width = column
                .attr("width")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| value * 0.75)
                .unwrap_or(fallback);
            for (key, value) in css_declarations(column.attr("style").unwrap_or("")) {
                if key == "width" {
                    width = css_points(&value, width);
                }
            }
            let span = html_span(column, "span", 256)?;
            if widths.len() + span > 256 {
                return Err("HTML: colgroup превышает256 столбцов".into());
            }
            widths.extend(std::iter::repeat_n(width.clamp(0.001, 10000.0), span));
        }
    }
    widths.resize(columns, fallback);
    widths.truncate(columns);
    if row_heights.iter().all(|height| *height == 0.0) {
        row_heights.clear();
    }
    if let Some(count) = node
        .attr("data-trinedocs-header-rows")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| *count <= rows.len())
    {
        header_rows = count;
    }
    let id = doc.alloc_id();
    doc.tables.push(Table {
        id,
        after_paragraph_id: anchor,
        rows,
        column_widths: widths,
        border_color,
        borderless,
        cell_margin: margin,
        row_heights,
        header_rows,
    });
    Ok(())
}

fn html_import_list_item(
    node: &Node,
    doc: &mut Doc,
    css: &str,
    list: Option<(List, u8)>,
    walk: HtmlWalker,
) -> Result<(), String> {
    fn emit(
        node: &Node,
        doc: &mut Doc,
        css: &str,
        list: (List, u8),
        first: &mut bool,
        walk: HtmlWalker,
    ) -> Result<(), String> {
        let start = doc.paragraphs.len();
        walk(node, doc, css, Some(list))?;
        for paragraph in doc
            .paragraphs
            .iter_mut()
            .skip(start)
            .filter(|paragraph| paragraph.list == List::None)
        {
            if !*first {
                paragraph.list = list.0;
                paragraph.list_level = list.1;
                *first = true;
            } else {
                paragraph.indent += (f32::from(list.1) + 1.0) * 18.0;
            }
        }
        Ok(())
    }
    fn flush(
        inline: &mut Vec<Node>,
        doc: &mut Doc,
        css: &str,
        list: (List, u8),
        first: &mut bool,
        walk: HtmlWalker,
    ) -> Result<(), String> {
        if inline.is_empty() {
            return Ok(());
        }
        let paragraph = Node {
            name: "p".into(),
            children: std::mem::take(inline),
            ..Default::default()
        };
        if !paragraph.text_content().trim().is_empty()
            || paragraph
                .children
                .iter()
                .any(|child| child.name == "br" || child.name == "img")
        {
            emit(&paragraph, doc, css, list, first, walk)?;
        }
        Ok(())
    }
    let list = list.unwrap_or((List::Bullet, 0));
    let mut first = false;
    let mut inline = Vec::new();
    for child in &node.children {
        if matches!(child.name.as_str(), "ol" | "ul") {
            flush(&mut inline, doc, css, list, &mut first, walk)?;
            walk(child, doc, css, Some(list))?;
        } else if matches!(
            child.name.as_str(),
            "p" | "div" | "pre" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "table"
        ) {
            flush(&mut inline, doc, css, list, &mut first, walk)?;
            emit(child, doc, css, list, &mut first, walk)?;
        } else {
            inline.push(child.clone());
        }
    }
    flush(&mut inline, doc, css, list, &mut first, walk)?;
    Ok(())
}

#[cfg(test)]
mod hundred_docs_io_regressions {
    use super::*;
    fn html(source: &str) -> Result<Doc, String> {
        read(source.as_bytes(), "html")
    }
    fn md(source: &str) -> Result<Doc, String> {
        read(source.as_bytes(), "md")
    }
    fn word(body: &str, styles: &str, numbering: &str) -> Result<Doc, String> {
        let namespace = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
        let mut parts = vec![(
            "word/document.xml".into(),
            format!("<w:document {namespace}><w:body>{body}</w:body></w:document>").into_bytes(),
        )];
        if !styles.is_empty() {
            parts.push((
                "word/styles.xml".into(),
                format!("<w:styles {namespace}>{styles}</w:styles>").into_bytes(),
            ));
        }
        if !numbering.is_empty() {
            parts.push((
                "word/numbering.xml".into(),
                format!("<w:numbering {namespace}>{numbering}</w:numbering>").into_bytes(),
            ));
        }
        read(&write_zip(parts)?, "docx")
    }
    #[test]
    fn html_normal_whitespace_collapses_across_inline_nodes_but_pre_stays_literal()
    -> Result<(), String> {
        let doc = html(
            "<p> \n  one <b> \t two </b>  three \n <br/> four </p><pre>  a\n  b  </pre><p style=\"white-space:pre-wrap\">  c  </p>",
        )?;
        assert_eq!(doc.paragraphs[0].text(), "one two three\u{000b}four");
        assert_eq!(doc.paragraphs[1].text(), "  a\n  b  ");
        assert_eq!(doc.paragraphs[2].text(), "  c  ");
        Ok(())
    }
    #[test]
    fn html_code_style_survives_its_own_export() -> Result<(), String> {
        let mut doc = Doc::from_text("a  b\nnext");
        doc.paragraphs[0].style = "Code".into();
        let exported = write(&doc, "html")?;
        assert!(String::from_utf8_lossy(&exported).contains("<pre "));
        let imported = read(&exported, "html")?;
        assert_eq!(imported.paragraphs[0].style, "Code");
        assert_eq!(imported.paragraphs[0].text(), "a  b");
        let heading = html("<h1>Heading</h1>")?;
        assert_eq!(heading.paragraphs[0].runs[0].format.size, 22.0);
        Ok(())
    }
    #[test]
    fn html_relative_font_sizes_use_the_parent_size_once() -> Result<(), String> {
        let doc = html(
            r#"<div style="font-size:20pt"><p style="font-size:150%">big <span style="font-size:.5em">small</span></p></div>"#,
        )?;
        assert_eq!(doc.paragraphs[0].runs[0].format.size, 30.0);
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "small" && run.format.size == 15.0)
        );
        Ok(())
    }
    #[test]
    fn html_percentage_line_height_uses_the_computed_font_size() -> Result<(), String> {
        let doc = html(
            r#"<p style="font-size:20pt;line-height:150%">percent<span style="font-size:70pt">large</span></p><p style="font-size:12pt;line-height:2em">em</p>"#,
        )?;
        assert_eq!(doc.paragraphs[0].line_points, 30.0);
        assert_eq!(doc.paragraphs[1].line_points, 24.0);
        assert!(doc.paragraphs.iter().all(|paragraph| paragraph.line_exact));
        Ok(())
    }
    #[test]
    fn html_css_vertical_alignment_keeps_super_and_subscripts() -> Result<(), String> {
        let doc = html(
            r#"<p>x<span style="vertical-align:super">2</span><span style="vertical-align:sub">i</span></p>"#,
        )?;
        assert_eq!(doc.paragraphs[0].runs[1].format.script, 1);
        assert_eq!(doc.paragraphs[0].runs[2].format.script, -1);
        Ok(())
    }
    #[test]
    fn html_list_items_keep_multiple_block_paragraphs_and_nested_list_levels() -> Result<(), String>
    {
        let doc = html(
            "<ul><li><p>first</p><p>second</p><ol><li>nested</li></ol></li><li>last</li></ul>",
        )?;
        assert_eq!(
            doc.paragraphs
                .iter()
                .map(Paragraph::text)
                .collect::<Vec<_>>(),
            vec!["first", "second", "nested", "last"]
        );
        assert_eq!(
            (doc.paragraphs[0].list, doc.paragraphs[0].list_level),
            (List::Bullet, 0)
        );
        assert_eq!(doc.paragraphs[1].list, List::None);
        assert_eq!(doc.paragraphs[1].indent, 18.0);
        assert_eq!(
            (doc.paragraphs[2].list, doc.paragraphs[2].list_level),
            (List::Number, 1)
        );
        let wrapped = html("<ul><li><div><p>A</p><ol><li>B</li></ol></div></li></ul>")?;
        assert_eq!(
            wrapped
                .paragraphs
                .iter()
                .map(Paragraph::text)
                .collect::<Vec<_>>(),
            vec!["A", "B"]
        );
        assert_eq!(
            (wrapped.paragraphs[0].list, wrapped.paragraphs[0].list_level),
            (List::Bullet, 0)
        );
        assert_eq!(
            (wrapped.paragraphs[1].list, wrapped.paragraphs[1].list_level),
            (List::Number, 1)
        );
        Ok(())
    }
    #[test]
    fn html_tables_import_bounded_structure_merges_and_native_export_geometry() -> Result<(), String>
    {
        let doc = html(
            r##"<p>anchor</p><table><colgroup><col style="width:70pt"/><col style="width:90pt"/></colgroup><thead><tr><th colspan="2" rowspan="2" style="padding:1pt 2pt 3pt 4pt;vertical-align:bottom;border:1px solid #123456;background-color:#abcdef;height:24pt"><p><b>merged</b></p></th></tr><tr></tr></thead><tbody><tr><td>a</td><td>b</td></tr></tbody></table>"##,
        )?;
        let table = &doc.tables[0];
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.grid_columns(), 2);
        assert_eq!(table.rows[0][0].span, 2);
        assert!(table.rows[1][0].merged);
        assert_eq!(table.rows[0][0].fill, [171, 205, 239]);
        assert_eq!(table.rows[0][0].v_align, VAlign::Bottom);
        assert!(table.rows[0][0].paragraphs[0].runs[0].format.bold);
        assert_eq!(table.column_widths, vec![70.0, 90.0]);
        assert_eq!(table.cell_margin, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(table.border_color, [18, 52, 86]);
        assert!(!table.borderless);
        assert_eq!(table.header_rows, 2);
        assert_eq!(table.row_heights[0], 24.0);
        let imported = read(&write(&doc, "html")?, "html")?;
        assert_eq!(imported.tables[0].rows[0][0].span, 2);
        assert!(imported.tables[0].rows[1][0].merged);
        assert_eq!(imported.tables[0].column_widths, table.column_widths);
        assert!(html("<table><tr><td colspan=\"257\">bad</td></tr></table>").is_err());
        for span in ["0", "20"] {
            let grouped = html(&format!(
                "<table><thead><tr><th>H1</th><th>H2</th></tr></thead><tbody><tr><td rowspan=\"{span}\">A</td><td>B</td></tr><tr><td>C</td></tr></tbody><tbody><tr><td>D</td><td>E</td></tr></tbody></table>"
            ))?;
            let rows = &grouped.tables[0].rows;
            assert_eq!(rows.len(), 4);
            assert!(rows[2][0].merged);
            assert!(!rows[3][0].merged);
            assert_eq!(cell_text(&rows[3][0]), "D");
            assert_eq!(cell_text(&rows[2][1]), "C");
        }
        Ok(())
    }
    #[test]
    fn html_images_use_standard_attributes_and_css_with_intrinsic_ratio() -> Result<(), String> {
        let mut image = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(4, 2)
            .write_to(&mut image, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let uri = format!(
            "data:image/png;base64,{}",
            encode_image_base64(&image.into_inner())
        );
        let doc = html(&format!(
            "<img src=\"{uri}\" width=\"96\"/><img src=\"{uri}\" width=\"96\" style=\"width:2in;height:1in\"/>"
        ))?;
        assert_eq!((doc.images[0].width, doc.images[0].height), (72.0, 36.0));
        assert_eq!((doc.images[1].width, doc.images[1].height), (144.0, 72.0));
        Ok(())
    }
    #[test]
    fn html_rejected_embedded_images_report_the_actual_loss() -> Result<(), String> {
        let doc = html("<p>kept<img src=\"data:image/png;base64,AA=A\"/></p>")?;
        assert!(doc.images.is_empty());
        assert_eq!(doc.paragraphs[0].text(), "kept");
        assert!(
            doc.losses
                .iter()
                .any(|loss| loss.contains("изображение пропущено"))
        );
        Ok(())
    }
    #[test]
    fn html_document_metadata_roundtrips_without_entering_body_text() -> Result<(), String> {
        let doc = html(
            "<html><head><title>A &amp; B</title><meta name=\"author\" content=\"C &amp; D\"/></head><body><p>body</p></body></html>",
        )?;
        assert_eq!(doc.title, "A & B");
        assert_eq!(doc.author, "C & D");
        assert_eq!(doc.plain_text(), "body");
        let again = read(&write(&doc, "html")?, "html")?;
        assert_eq!(again.title, doc.title);
        assert_eq!(again.author, doc.author);
        Ok(())
    }
    #[test]
    fn html_descendants_cannot_erase_an_ancestor_text_decoration() -> Result<(), String> {
        let doc = html(
            r#"<div style="text-decoration:underline"><p>one <span style="text-decoration:none">two</span></p></div>"#,
        )?;
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .all(|run| run.format.underline)
        );
        Ok(())
    }
    #[test]
    fn docx_hidden_runs_respect_style_inheritance_and_explicit_visibility() -> Result<(), String> {
        let styles = r#"<w:style w:type="paragraph" w:styleId="Secret"><w:rPr><w:vanish/></w:rPr></w:style><w:style w:type="character" w:styleId="Visible"><w:rPr><w:vanish/></w:rPr></w:style>"#;
        let doc = word(
            r#"<w:p><w:pPr><w:pStyle w:val="Secret"/></w:pPr><w:r><w:t>hidden</w:t></w:r><w:r><w:rPr><w:rStyle w:val="Visible"/></w:rPr><w:t>shown</w:t></w:r><w:r><w:rPr><w:vanish w:val="0"/></w:rPr><w:t>override</w:t></w:r></w:p>"#,
            styles,
            "",
        )?;
        assert_eq!(doc.paragraphs[0].text(), "shownoverride");
        assert!(doc.losses.iter().any(|loss| loss.contains("Скрытый текст")));
        Ok(())
    }
    #[test]
    fn docx_double_strike_keeps_a_visible_strike_and_reports_the_approximation()
    -> Result<(), String> {
        let doc = word(
            "<w:p><w:r><w:rPr><w:dstrike/></w:rPr><w:t>strike</w:t></w:r></w:p>",
            "",
            "",
        )?;
        assert!(doc.paragraphs[0].runs[0].format.strike);
        assert!(
            doc.losses
                .iter()
                .any(|loss| loss.contains("Двойное зачёркивание"))
        );
        Ok(())
    }
    #[test]
    fn docx_table_vertical_alignment_inherits_then_allows_a_direct_override() -> Result<(), String>
    {
        let styles = r#"<w:style w:type="table" w:styleId="Aligned"><w:tcPr><w:vAlign w:val="bottom"/></w:tcPr></w:style>"#;
        let doc = word(
            r#"<w:tbl><w:tblPr><w:tblStyle w:val="Aligned"/></w:tblPr><w:tr><w:tc><w:p/></w:tc><w:tc><w:tcPr><w:vAlign w:val="center"/></w:tcPr><w:p/></w:tc></w:tr></w:tbl>"#,
            styles,
            "",
        )?;
        assert_eq!(doc.tables[0].rows[0][0].v_align, VAlign::Bottom);
        assert_eq!(doc.tables[0].rows[0][1].v_align, VAlign::Center);
        Ok(())
    }
    #[test]
    fn docx_grid_before_keeps_the_first_real_cell_in_its_grid_column() -> Result<(), String> {
        let doc = word(
            r#"<w:tbl><w:tblGrid><w:gridCol w:w="400"/><w:gridCol w:w="600"/><w:gridCol w:w="800"/></w:tblGrid><w:tr><w:trPr><w:gridBefore w:val="2"/></w:trPr><w:tc><w:p><w:r><w:t>third</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#,
            "",
            "",
        )?;
        let table = &doc.tables[0];
        assert_eq!(table.grid_starts(0), vec![0, 2]);
        assert_eq!(table.rows[0][0].span, 2);
        assert_eq!(cell_text(&table.rows[0][1]), "third");
        Ok(())
    }
    #[test]
    fn docx_numbering_none_does_not_create_a_visible_list_marker() -> Result<(), String> {
        let numbering = r#"<w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="none"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>"#;
        let doc = word(
            r#"<w:p><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>plain</w:t></w:r></w:p>"#,
            "",
            numbering,
        )?;
        assert_eq!(doc.paragraphs[0].list, List::None);
        Ok(())
    }
    #[test]
    fn docx_block_revisions_keep_inserted_content_and_skip_deleted_blocks() -> Result<(), String> {
        let doc = word(
            "<w:ins><w:p><w:r><w:t>inserted</w:t></w:r></w:p></w:ins><w:moveTo><w:p><w:r><w:t>moved</w:t></w:r></w:p></w:moveTo><w:del><w:p><w:r><w:t>deleted</w:t></w:r></w:p></w:del>",
            "",
            "",
        )?;
        assert_eq!(doc.plain_text(), "inserted\nmoved");
        Ok(())
    }
    #[test]
    fn markdown_fenced_code_checks_info_string_and_removes_opening_indentation()
    -> Result<(), String> {
        let doc = md("  ~~~lang\n  one\n    two\n ~~~\n")?;
        assert_eq!(doc.paragraphs[0].text(), "one\u{000b}  two");
        let invalid = md("```lang`bad\ntext")?;
        assert!(
            invalid
                .paragraphs
                .iter()
                .all(|paragraph| paragraph.style != "Code")
        );
        Ok(())
    }
    #[test]
    fn markdown_indented_code_keeps_internal_blanks_and_cannot_interrupt_a_paragraph()
    -> Result<(), String> {
        let doc = md("    one\n\n    two\n\ntext\n    continuation")?;
        assert_eq!(doc.paragraphs.len(), 2);
        assert_eq!(doc.paragraphs[0].text(), "one\u{000b}\u{000b}two");
        assert_eq!(doc.paragraphs[1].text(), "text continuation");
        Ok(())
    }
    #[test]
    fn markdown_link_closing_parentheses_ignore_angle_urls_and_quoted_titles() -> Result<(), String>
    {
        let doc = md(
            "[one](<https://example.org/a(b> \"title ) here\") [two](https://example.org/c \"another ) title\")",
        )?;
        assert_eq!(doc.paragraphs[0].text(), "one two");
        assert!(doc.paragraphs[0].runs.iter().any(|run| run.text == "one"
            && run.format.link.as_deref() == Some("https://example.org/a(b")));
        assert!(
            doc.paragraphs[0].runs.iter().any(|run| run.text == "two"
                && run.format.link.as_deref() == Some("https://example.org/c"))
        );
        Ok(())
    }
    #[test]
    fn markdown_emphasis_requires_non_whitespace_delimiter_edges() -> Result<(), String> {
        // A leading "* " would open a list item, so the stray delimiters start mid-line.
        let doc = md("Text * word * and __ word__ and *valid*")?;
        assert_eq!(doc.paragraphs[0].text(), "Text * word * and __ word__ and valid");
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "valid" && run.format.italic)
        );
        Ok(())
    }
    #[test]
    fn markdown_run_newlines_export_as_hard_breaks_inside_one_paragraph() -> Result<(), String> {
        let mut doc = Doc::from_text("placeholder");
        doc.paragraphs[0].runs[0].text = "one\ntwo".into();
        let imported = read(&write(&doc, "md")?, "md")?;
        assert_eq!(imported.paragraphs.len(), 1);
        assert_eq!(imported.paragraphs[0].text(), "one\u{000b}two");
        Ok(())
    }
}

fn markdown_body_table_row(line: &str) -> Option<Vec<String>> {
    markdown_table_row(line).or_else(|| {
        (!md_starts_block(line) && !line.trim().is_empty()).then(|| vec![line.trim().into()])
    })
}
fn markdown_core(text: &str, format: &TextFormat) -> String {
    let monospace = matches!(
        format.font.to_ascii_lowercase().as_str(),
        "consolas" | "courier new" | "lucida console" | "cascadia code" | "monospace"
    );
    if monospace && !text.contains(['\n', '\r', '\u{000b}', '\u{000c}']) {
        let length = text.split(|ch| ch != '`').map(str::len).max().unwrap_or(0) + 1;
        let fence = "`".repeat(length);
        let pad = text.starts_with('`')
            || text.ends_with('`')
            || text.starts_with(' ') && text.ends_with(' ') && !text.chars().all(|ch| ch == ' ');
        if pad {
            format!("{fence} {text} {fence}")
        } else {
            format!("{fence}{text}{fence}")
        }
    } else {
        escape_markdown(text)
    }
}
fn markdown_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(ch))
        && !domain.is_empty()
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.starts_with(|ch: char| ch.is_ascii_alphanumeric())
                && label.ends_with(|ch: char| ch.is_ascii_alphanumeric())
                && label
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        })
}
fn markdown_entity(text: &str) -> Option<String> {
    let normalized = normalize_html_entities(text);
    quick_xml::escape::unescape(&normalized)
        .ok()
        .map(|value| value.into_owned())
}

fn normalize_html_attributes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        if !rest.starts_with('<') {
            let length = rest.find('<').unwrap_or(rest.len());
            out.push_str(&rest[..length]);
            offset += length;
            continue;
        }
        if rest.starts_with("<!--") || rest.starts_with("<![CDATA[") {
            let close = if rest.starts_with("<!--") {
                "-->"
            } else {
                "]]>"
            };
            let length = rest
                .find(close)
                .map(|end| end + close.len())
                .unwrap_or(rest.len());
            out.push_str(&rest[..length]);
            offset += length;
            continue;
        }
        let mut quote = None;
        let mut end = None;
        for (index, ch) in rest.char_indices().skip(1) {
            if let Some(open) = quote {
                if ch == open {
                    quote = None;
                }
                continue;
            }
            if matches!(ch, '\'' | '"') {
                quote = Some(ch);
            } else if ch == '>' {
                end = Some(index);
                break;
            }
        }
        let Some(end) = end else {
            out.push_str(rest);
            break;
        };
        let tag = &rest[..=end];
        if tag.starts_with("</") || tag.starts_with("<!") || tag.starts_with("<?") {
            out.push_str(tag);
            offset += tag.len();
            continue;
        }
        let body = &tag[1..tag.len() - 1];
        let name_end = body
            .find(|ch: char| ch.is_ascii_whitespace() || ch == '/')
            .unwrap_or(body.len());
        out.push('<');
        out.push_str(&body[..name_end]);
        let mut position = name_end;
        while position < body.len() {
            while body
                .as_bytes()
                .get(position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                position += 1;
            }
            if position >= body.len() {
                break;
            }
            if body[position..].trim() == "/" {
                out.push('/');
                break;
            }
            let start = position;
            while position < body.len()
                && !body.as_bytes()[position].is_ascii_whitespace()
                && !matches!(body.as_bytes()[position], b'=' | b'/')
            {
                position += 1;
            }
            let name = &body[start..position];
            if name.is_empty() {
                out.push_str(&body[position..]);
                break;
            }
            out.push(' ');
            out.push_str(name);
            while body
                .as_bytes()
                .get(position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                position += 1;
            }
            if body.as_bytes().get(position) != Some(&b'=') {
                out.push_str("=\"\"");
                continue;
            }
            position += 1;
            while body
                .as_bytes()
                .get(position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                position += 1;
            }
            out.push('=');
            if body
                .as_bytes()
                .get(position)
                .is_some_and(|ch| matches!(*ch, b'\'' | b'"'))
            {
                let delimiter = body.as_bytes()[position];
                let start = position;
                position += 1;
                while position < body.len() && body.as_bytes()[position] != delimiter {
                    position += 1;
                }
                if position < body.len() {
                    position += 1;
                }
                out.push_str(&body[start..position]);
            } else {
                let start = position;
                while position < body.len() && !body.as_bytes()[position].is_ascii_whitespace() {
                    position += 1;
                }
                out.push('"');
                out.push_str(&body[start..position]);
                out.push('"');
            }
        }
        out.push('>');
        offset += tag.len();
    }
    out
}

fn normalize_html_optional_tags(text: &str) -> Result<String, String> {
    if text.len() > XML_LIMIT {
        return Err("HTML превышает безопасный предел размера".into());
    }
    fn template_end(text: &str, opening: usize) -> usize {
        let mut depth = 1usize;
        let mut position = opening;
        while position < text.len() {
            let Some(relative) = text[position..].find('<') else {
                return text.len();
            };
            position += relative;
            let rest = &text[position..];
            if rest.starts_with("<!--") || rest.starts_with("<![CDATA[") {
                let marker = if rest.starts_with("<!--") {
                    "-->"
                } else {
                    "]]>"
                };
                let length = rest
                    .find(marker)
                    .map(|end| end + marker.len())
                    .unwrap_or(rest.len());
                position += length;
                continue;
            }
            let (mut quote, mut end) = (None, None);
            for (index, ch) in rest.char_indices().skip(1) {
                if let Some(open) = quote {
                    if ch == open {
                        quote = None;
                    }
                    continue;
                }
                if matches!(ch, '\'' | '"') {
                    quote = Some(ch);
                } else if ch == '>' {
                    end = Some(index);
                    break;
                }
            }
            let Some(end) = end else {
                return text.len();
            };
            let tag = &rest[..=end];
            let closing = tag.starts_with("</");
            let begin = if closing { 2 } else { 1 };
            let name = tag[begin..]
                .split(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '/' | '>'))
                .next()
                .unwrap_or("");
            if name.eq_ignore_ascii_case("template") {
                if closing {
                    depth -= 1;
                    if depth == 0 {
                        return position + tag.len();
                    }
                } else if !tag.trim_end().ends_with("/>") {
                    depth += 1;
                }
            }
            position += tag.len();
        }
        text.len()
    }
    fn close(out: &mut String, stack: &mut Vec<String>, at: usize) -> Result<(), String> {
        while stack.len() > at {
            if let Some(name) = stack.pop() {
                html_extend(out, "</", XML_LIMIT)?;
                html_extend(out, &name, XML_LIMIT)?;
                html_extend(out, ">", XML_LIMIT)?;
            }
        }
        Ok(())
    }
    let mut out = String::with_capacity(text.len());
    let mut stack = Vec::<String>::new();
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        if !rest.starts_with('<') {
            let length = rest.find('<').unwrap_or(rest.len());
            html_extend(&mut out, &rest[..length], XML_LIMIT)?;
            offset += length;
            continue;
        }
        if rest.starts_with("<!--") || rest.starts_with("<![CDATA[") {
            let marker = if rest.starts_with("<!--") {
                "-->"
            } else {
                "]]>"
            };
            let length = rest
                .find(marker)
                .map(|end| end + marker.len())
                .unwrap_or(rest.len());
            html_extend(&mut out, &rest[..length], XML_LIMIT)?;
            offset += length;
            continue;
        }
        let (mut quote, mut end) = (None, None);
        for (index, ch) in rest.char_indices().skip(1) {
            if let Some(open) = quote {
                if ch == open {
                    quote = None;
                }
                continue;
            }
            if matches!(ch, '\'' | '"') {
                quote = Some(ch);
            } else if ch == '>' {
                end = Some(index);
                break;
            }
        }
        let Some(end) = end else {
            html_extend(&mut out, rest, XML_LIMIT)?;
            break;
        };
        let tag = &rest[..=end];
        if tag.starts_with("<!") || tag.starts_with("<?") {
            html_extend(&mut out, tag, XML_LIMIT)?;
            offset += tag.len();
            continue;
        }
        let closing = tag.starts_with("</");
        let begin = if closing { 2 } else { 1 };
        let name_end = tag[begin..]
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != ':')
            .map(|end| begin + end)
            .unwrap_or(tag.len() - 1);
        if begin == name_end {
            html_extend(&mut out, "&lt;", XML_LIMIT)?;
            offset += 1;
            continue;
        }
        let name = tag[begin..name_end].to_ascii_lowercase();
        if !closing && matches!(name.as_str(), "script" | "style") {
            let marker = format!("</{name}");
            let mut from = tag.len();
            let mut length = rest.len();
            while let Some(found) = rest[from..].find('<') {
                let at = from + found;
                let tail = at + marker.len();
                if rest[at..]
                    .get(..marker.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&marker))
                    && rest
                        .as_bytes()
                        .get(tail)
                        .is_some_and(|ch| ch.is_ascii_whitespace() || matches!(*ch, b'>' | b'/'))
                {
                    length = rest[tail..]
                        .find('>')
                        .map(|end| tail + end + 1)
                        .unwrap_or(rest.len());
                    break;
                }
                from = at + 1;
            }
            html_extend(&mut out, "<", XML_LIMIT)?;
            html_extend(&mut out, &name, XML_LIMIT)?;
            html_extend(&mut out, "/>", XML_LIMIT)?;
            offset += length;
            continue;
        }
        if !closing && name == "template" {
            let length = if tag.trim_end().ends_with("/>") {
                tag.len()
            } else {
                template_end(rest, tag.len())
            };
            html_extend(&mut out, "<template/>", XML_LIMIT)?;
            offset += length;
            continue;
        }
        if closing {
            if let Some(at) = stack.iter().rposition(|open| *open == name) {
                close(&mut out, &mut stack, at)?;
            }
            offset += tag.len();
            continue;
        }
        if name == "body"
            && let Some(at) = stack.iter().rposition(|open| open == "head")
        {
            close(&mut out, &mut stack, at)?;
        }
        if matches!(
            name.as_str(),
            "p" | "div"
                | "section"
                | "article"
                | "pre"
                | "blockquote"
                | "ul"
                | "ol"
                | "table"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
        ) && let Some(at) = stack.iter().rposition(|open| open == "p")
        {
            close(&mut out, &mut stack, at)?;
        }
        let boundary = match name.as_str() {
            "li" => Some(("li", &["ul", "ol"][..])),
            "dt" | "dd" => Some(("dt", &["dl"][..])),
            "tr" => Some(("tr", &["table"][..])),
            "td" | "th" => Some(("td", &["tr"][..])),
            "thead" | "tbody" | "tfoot" => Some(("tbody", &["table"][..])),
            _ => None,
        };
        if let Some((kind, parents)) = boundary {
            let floor = stack
                .iter()
                .rposition(|open| parents.contains(&open.as_str()))
                .map(|at| at + 1)
                .unwrap_or(0);
            if let Some(at) = stack
                .iter()
                .enumerate()
                .skip(floor)
                .rfind(|(_, open)| match kind {
                    "dt" => matches!(open.as_str(), "dt" | "dd"),
                    "td" => matches!(open.as_str(), "td" | "th"),
                    "tbody" => matches!(open.as_str(), "thead" | "tbody" | "tfoot"),
                    _ => open.as_str() == kind,
                })
                .map(|(at, _)| at)
            {
                close(&mut out, &mut stack, at)?;
            }
        }
        html_extend(&mut out, "<", XML_LIMIT)?;
        html_extend(&mut out, &name, XML_LIMIT)?;
        html_extend(&mut out, &tag[name_end..], XML_LIMIT)?;
        offset += tag.len();
        if !tag.trim_end().ends_with("/>")
            && !matches!(
                name.as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            )
        {
            // The synthetic HTML root consumes one of the XML parser's128 levels.
            if stack.len() >= 127 {
                return Err("HTML превышает безопасную глубину вложенности".into());
            }
            stack.push(name);
        }
    }
    close(&mut out, &mut stack, 0)?;
    Ok(out)
}

#[cfg(test)]
mod two_hundred_docs_io_regressions {
    use super::*;
    fn html(source: &str) -> Result<Doc, String> {
        read(source.as_bytes(), "html")
    }
    fn md(source: &str) -> Result<Doc, String> {
        read(source.as_bytes(), "md")
    }
    fn word(body: &str, styles: &str, numbering: &str) -> Result<Doc, String> {
        let ns = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
        let mut parts = vec![(
            "word/document.xml".into(),
            format!("<w:document {ns}><w:body>{body}</w:body></w:document>").into_bytes(),
        )];
        if !styles.is_empty() {
            parts.push((
                "word/styles.xml".into(),
                format!("<w:styles {ns}>{styles}</w:styles>").into_bytes(),
            ));
        }
        if !numbering.is_empty() {
            parts.push((
                "word/numbering.xml".into(),
                format!("<w:numbering {ns}>{numbering}</w:numbering>").into_bytes(),
            ));
        }
        read(&write_zip(parts)?, "docx")
    }
    fn image_bytes() -> Result<Vec<u8>, String> {
        let mut image = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, image::Rgba([0, 0, 255, 255]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        Ok(bytes.into_inner())
    }
    fn image_word(properties: &str, crop: &str) -> Result<Doc, String> {
        let document = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:r><w:drawing><wp:inline><wp:extent cx="914400" cy="457200"/><wp:docPr id="1" name="image" {properties}/><a:graphic><a:graphicData><a:blipFill><a:blip r:embed="image1"/>{crop}</a:blipFill></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p></w:body></w:document>"#
        );
        read(&write_zip(vec![("word/document.xml".into(),document.into_bytes()),
            ("word/_rels/document.xml.rels".into(),br#"<Relationships><Relationship Id="image1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image.png"/></Relationships>"#.to_vec()),
            ("word/media/image.png".into(),image_bytes()?)])?,"docx")
    }
    #[test]
    fn html_unquoted_and_boolean_attributes_keep_formatting_and_hidden_content_out()
    -> Result<(), String> {
        let doc = html(
            "<p class=plain style=color:red>red<b hidden>secret</b><input disabled/><br hidden/>ok</p>",
        )?;
        assert_eq!(doc.plain_text(), "redok");
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .all(|run| run.format.color == [255, 0, 0])
        );
        let doc = html("<p hidden/><p style=font-weight:bold>kept</p>")?;
        assert_eq!(doc.plain_text(), "kept");
        assert!(doc.paragraphs[0].runs[0].format.bold);
        Ok(())
    }
    #[test]
    fn gfm_table_body_rows_without_a_pipe_keep_their_first_column_and_padding() -> Result<(), String>
    {
        let doc = md("|a|b|\n|---|---|\nsolo\nx|y\n\nend")?;
        let table = &doc.tables[0];
        assert_eq!(table.rows.len(), 3);
        assert_eq!(cell_text(&table.rows[1][0]), "solo");
        assert!(cell_text(&table.rows[1][1]).is_empty());
        assert_eq!(cell_text(&table.rows[2][0]), "x");
        assert_eq!(cell_text(&table.rows[2][1]), "y");
        assert!(doc.plain_text().ends_with("end"));
        Ok(())
    }
    #[test]
    fn markdown_table_column_alignment_survives_export_and_reimport() -> Result<(), String> {
        let mut doc = Doc::from_text("anchor");
        doc.add_table(doc.paragraphs[0].id, 2, 3)?;
        for (cell, align) in
            doc.tables[0].rows[0]
                .iter_mut()
                .zip([Align::Center, Align::Right, Align::Left])
        {
            cell.paragraphs[0].align = align;
        }
        let text = write_markdown(&doc);
        assert!(text.contains("|:---:|---:|---|"));
        let imported = md(&text)?;
        assert_eq!(
            imported.tables[0].rows[0]
                .iter()
                .map(|cell| cell.paragraphs[0].align)
                .collect::<Vec<_>>(),
            vec![Align::Center, Align::Right, Align::Left]
        );
        Ok(())
    }
    #[test]
    fn markdown_table_export_does_not_leak_vertical_merge_continuation_text() -> Result<(), String>
    {
        let mut doc = Doc::from_text("anchor");
        doc.add_table(doc.paragraphs[0].id, 2, 1)?;
        doc.tables[0].rows[0][0].paragraphs[0].runs[0].text = "visible".into();
        doc.tables[0].rows[1][0].merged = true;
        doc.tables[0].rows[1][0].paragraphs[0].runs[0].text = "hidden continuation".into();
        let text = write_markdown(&doc);
        assert!(text.contains("visible"));
        assert!(!text.contains("hidden continuation"));
        let imported = md(&text)?;
        assert_eq!(imported.tables[0].rows.len(), 2);
        assert!(cell_text(&imported.tables[0].rows[1][0]).is_empty());
        Ok(())
    }
    #[test]
    fn markdown_export_keeps_single_line_inline_code_and_literal_backticks() -> Result<(), String> {
        let mut doc = Doc::from_text("`x` & *y*");
        doc.paragraphs[0].runs[0].format.font = "Consolas".into();
        let text = write_markdown(&doc);
        assert!(text.contains("`` `x` & *y* ``"));
        let imported = md(&text)?;
        assert_eq!(imported.plain_text(), doc.plain_text());
        assert_eq!(imported.paragraphs[0].runs[0].format.font, "Consolas");
        Ok(())
    }
    #[test]
    fn markdown_email_autolinks_use_mailto_without_visible_angle_brackets() -> Result<(), String> {
        let doc = md("<user.name+tag@example.org>")?;
        assert_eq!(doc.plain_text(), "user.name+tag@example.org");
        assert_eq!(
            doc.paragraphs[0].runs[0].format.link.as_deref(),
            Some("mailto:user.name+tag@example.org")
        );
        Ok(())
    }
    #[test]
    fn markdown_autolink_schemes_are_ascii_case_insensitive() -> Result<(), String> {
        let doc = md("<HTTPS://example.org/A> <MAILTO:User@example.org>")?;
        assert_eq!(doc.plain_text(), "HTTPS://example.org/A User@example.org");
        let links = doc.paragraphs[0]
            .runs
            .iter()
            .filter_map(|run| run.format.link.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(
            links,
            vec!["HTTPS://example.org/A", "MAILTO:User@example.org"]
        );
        Ok(())
    }
    #[test]
    fn markdown_supported_named_entities_decode_without_interpreting_markup() -> Result<(), String>
    {
        let doc = md("&copy; &nbsp; &mdash; &lt;b&gt; &unknown;")?;
        assert_eq!(doc.plain_text(), "© \u{00a0} — <b> &unknown;");
        assert!(!doc.paragraphs[0].runs[0].format.bold);
        Ok(())
    }
    #[test]
    fn markdown_quote_blocks_keep_paragraphs_and_lists() -> Result<(), String> {
        let doc = md("> first\n> second\n>\n> - **item**\n>\n> ```\n> code\n> ```\n\noutside")?;
        assert_eq!(doc.paragraphs[0].text(), "first second");
        assert_eq!(doc.paragraphs[0].style, "Quote");
        assert_eq!(doc.paragraphs[1].list, List::Bullet);
        assert_eq!(doc.paragraphs[1].text(), "item");
        assert!(doc.paragraphs[1].runs[0].format.bold);
        assert_eq!(doc.paragraphs[2].style, "Code");
        assert_eq!(doc.paragraphs[2].text(), "code");
        assert_eq!(doc.paragraphs[3].text(), "outside");
        let ids = doc
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), doc.paragraphs.len());
        doc.validate()?;
        Ok(())
    }
    #[test]
    fn html_optional_end_tags_preserve_structure() -> Result<(), String> {
        let doc = html(
            "<P style=color:red>one<p>two<ul><li><b>three</b><li>four</ul><table><tr><td>A<td>B<tr><td>C<td>D</table>",
        )?;
        assert_eq!(
            doc.paragraphs
                .iter()
                .map(Paragraph::text)
                .collect::<Vec<_>>(),
            vec!["one", "two", "three", "four"]
        );
        assert_eq!(doc.paragraphs[0].runs[0].format.color, [255, 0, 0]);
        assert!(doc.paragraphs[2].runs[0].format.bold);
        assert_eq!(doc.tables[0].rows.len(), 2);
        assert_eq!(cell_text(&doc.tables[0].rows[1][1]), "D");
        let head = html("<html><head><title>Title</title><body><p>Body")?;
        assert_eq!(head.title, "Title");
        assert_eq!(head.plain_text(), "Body");
        assert!(normalize_html_optional_tags(&"<a>".repeat(129)).is_err());
        assert!(html(&format!("{}text", "<a>".repeat(129))).is_err());
        Ok(())
    }
    #[test]
    fn html_hidden_detection_uses_real_css_declarations_instead_of_quoted_fragments()
    -> Result<(), String> {
        let doc = html(
            r#"<p style="font-family:'Keep;display:none'">visible</p><p style="display: none !important">secret</p>"#,
        )?;
        assert_eq!(doc.plain_text(), "visible");
        assert_eq!(doc.paragraphs[0].runs[0].format.font, "Keep;display:none");
        Ok(())
    }
    #[test]
    fn html_hidden_break_elements_do_not_insert_visible_line_breaks() -> Result<(), String> {
        let doc = html(r#"<p>a<br hidden/>b<br style="display:none!important"/>c<br/>d</p>"#)?;
        assert_eq!(doc.plain_text(), "abc\u{000b}d");
        Ok(())
    }
    #[test]
    fn html_template_contents_remain_inert_and_do_not_enter_document_text() -> Result<(), String> {
        let doc = html(
            "<p>before<template><p>secret</p><template>inner</template>outer secret</template>after</p>",
        )?;
        assert!(!doc.plain_text().contains("secret"));
        assert!(!doc.plain_text().contains("inner"));
        assert!(doc.plain_text().contains("before") && doc.plain_text().contains("after"));
        let fallback = fallback_html_text(
            "before<template>secret<template>inner</template>outer secret</template>after",
        );
        assert_eq!(fallback, "beforeafter");
        let script = html(
            "<p>before<script><p>secret</p></script><style><p>hidden css</p></style>after</p>",
        )?;
        assert_eq!(script.plain_text(), "beforeafter");
        Ok(())
    }
    #[test]
    fn html_semantic_and_legacy_text_tags_map_to_existing_text_format_fields() -> Result<(), String>
    {
        let doc = html(
            r#"<p><font color="red" face="'Georgia',serif" size="5"><ins>u</ins><strike>s</strike></font><mark>m</mark><small>x</small><big>y</big><kbd>k</kbd><samp>sample</samp><tt>t</tt></p>"#,
        )?;
        let runs = &doc.paragraphs[0].runs;
        let u = runs.iter().find(|run| run.text == "u").ok_or("u")?;
        assert!(u.format.underline);
        assert_eq!(
            (u.format.font.as_str(), u.format.size, u.format.color),
            ("Georgia", 18.0, [255, 0, 0])
        );
        assert!(
            runs.iter()
                .find(|run| run.text == "s")
                .is_some_and(|run| run.format.strike)
        );
        assert!(
            runs.iter()
                .find(|run| run.text == "m")
                .is_some_and(|run| run.format.highlight == Some([255, 255, 0]))
        );
        assert!(
            runs.iter()
                .find(|run| run.text == "x")
                .is_some_and(|run| run.format.size < 11.0)
        );
        assert!(
            runs.iter()
                .find(|run| run.text == "y")
                .is_some_and(|run| run.format.size > 11.0)
        );
        assert!(
            runs.iter()
                .filter(|run| run.text.contains('k')
                    || run.text.contains("sample")
                    || run.text == "t")
                .all(|run| run.format.font == "Consolas")
        );
        Ok(())
    }
    #[test]
    fn html_single_color_background_shorthand_keeps_text_highlighting() -> Result<(), String> {
        let doc = html(
            r#"<p><span style="background:#123">hex</span><span style="background: rgb(255,0,0)">rgb</span></p>"#,
        )?;
        assert_eq!(
            doc.paragraphs[0].runs[0].format.highlight,
            Some([17, 34, 51])
        );
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "rgb" && run.format.highlight == Some([255, 0, 0]))
        );
        Ok(())
    }
    #[test]
    fn html_font_shorthand_respects_cascade() -> Result<(), String> {
        let doc = html(
            r#"<p style="font:italic bold 16px/1.5 'Georgia'">first</p><p style="font-weight:normal!important;font:bold 18pt 'Arial'">second</p>"#,
        )?;
        let first = &doc.paragraphs[0];
        assert_eq!(first.runs[0].format.font, "Georgia");
        assert_eq!(first.runs[0].format.size, 12.0);
        assert!(first.runs[0].format.bold && first.runs[0].format.italic);
        assert_eq!(first.line_spacing, 1.5);
        let second = &doc.paragraphs[1].runs[0].format;
        assert_eq!(second.font, "Arial");
        assert_eq!(second.size, 18.0);
        assert!(!second.bold && !second.italic);
        Ok(())
    }
    #[test]
    fn html_data_url_normalization() -> Result<(), String> {
        assert_eq!(
            decode_html_image("DATA:IMAGE/PNG; charset=utf-8; BASE64,QQ%3D%3D"),
            Some(vec![65])
        );
        assert_eq!(
            decode_html_image("data:image/jpeg;base64, Q Q \n"),
            Some(vec![65])
        );
        assert!(decode_html_image("data:image/png;base64,AA=A").is_none());
        assert!(decode_html_image("data:image/png;base64,Q").is_none());
        let bytes = image_bytes()?;
        let encoded = encode_image_base64(&bytes).replace('=', "%3D");
        let doc = html(&format!(
            "<p>image</p><img src=\"DATA:IMAGE/PNG;BASE64,{encoded}\"/>"
        ))?;
        assert_eq!(doc.images.len(), 1);
        assert_eq!(doc.images[0].bytes, bytes);
        Ok(())
    }
    #[test]
    fn html_export_stops_at_output_budget() -> Result<(), String> {
        let mut doc = Doc::from_text("text");
        let exact = write_html_bounded(&doc, XML_LIMIT)?;
        assert_eq!(write_html_bounded(&doc, exact.len())?, exact);
        assert!(write_html_bounded(&doc, exact.len() - 1).is_err());
        doc.paragraphs[0].runs = (0..100)
            .map(|index| Run {
                text: "x".into(),
                format: TextFormat {
                    bold: index % 2 == 0,
                    link: Some(format!("https://example.org/{}", "a".repeat(1000))),
                    ..Default::default()
                },
            })
            .collect();
        assert!(write_html_bounded(&doc, 1024).is_err());
        assert!(html_paragraph_bounded(&doc.paragraphs[0], "p", 1024).is_err());
        doc.paragraphs[0].runs = vec![Run {
            text: "&".repeat(64 * 1024 * 1024),
            format: TextFormat::default(),
        }];
        assert!(html_paragraph_bounded(&doc.paragraphs[0], "p", 1024).is_err());
        Ok(())
    }
    #[test]
    fn word_symbols_keep_order_and_font() -> Result<(), String> {
        let doc = word(
            r#"<w:p><w:r><w:t>A</w:t><w:sym w:font="Segoe UI Symbol" w:char="263A"/><w:t>B</w:t></w:r></w:p>"#,
            "",
            "",
        )?;
        assert_eq!(doc.plain_text(), "A☺B");
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "☺" && run.format.font == "Segoe UI Symbol")
        );
        Ok(())
    }
    #[test]
    fn word_ruby_keeps_base_without_annotation() -> Result<(), String> {
        let doc = word(
            r#"<w:p><w:r><w:t>A</w:t><w:ruby><w:rt><w:r><w:t>reading</w:t></w:r></w:rt><w:rubyBase><w:r><w:rPr><w:b/></w:rPr><w:t>漢</w:t></w:r></w:rubyBase></w:ruby><w:t>B</w:t></w:r></w:p>"#,
            "",
            "",
        )?;
        assert_eq!(doc.plain_text(), "A漢B");
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|run| run.text == "漢" && run.format.bold)
        );
        assert!(doc.losses.iter().any(|loss| loss.contains("Фонетические")));
        Ok(())
    }
    #[test]
    fn word_column_break_advances_page() -> Result<(), String> {
        let doc = word(
            r#"<w:p><w:r><w:t>before</w:t><w:br w:type="column"/><w:t>after</w:t></w:r></w:p>"#,
            "",
            "",
        )?;
        assert_eq!(doc.plain_text(), "before\u{000c}after");
        assert_eq!(doc.layout().pages.len(), 2);
        Ok(())
    }
    #[test]
    fn word_relocated_core_properties() -> Result<(), String> {
        let parts=vec![("word/document.xml".into(),br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p/></w:body></w:document>"#.to_vec()),
            ("_rels/.rels".into(),br#"<Relationships><Relationship Id="core" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="custom/metadata.xml"/></Relationships>"#.to_vec()),
            ("custom/metadata.xml".into(),br#"<cp:coreProperties xmlns:cp="urn:core" xmlns:dc="urn:dc"><dc:title>Selected title</dc:title><dc:creator>Selected author</dc:creator></cp:coreProperties>"#.to_vec())];
        let doc = read(&write_zip(parts)?, "docx")?;
        assert_eq!(doc.title, "Selected title");
        assert_eq!(doc.author, "Selected author");
        let reopened = read(&write(&doc, "docx")?, "docx")?;
        assert_eq!(reopened.title, doc.title);
        assert_eq!(reopened.author, doc.author);
        Ok(())
    }
    #[test]
    fn word_logical_end_alignment() -> Result<(), String> {
        let doc = word(
            r#"<w:p><w:pPr><w:jc w:val="end"/></w:pPr><w:r><w:t>right edge</w:t></w:r></w:p>"#,
            "",
            "",
        )?;
        assert_eq!(doc.paragraphs[0].align, Align::Right);
        let row = &doc.layout().rows[0];
        assert!(row.x > doc.page.margin_left + 100.0);
        Ok(())
    }
    #[test]
    fn word_spacing_attributes_inherit_independently() -> Result<(), String> {
        let styles = r#"<w:style w:type="paragraph" w:styleId="Base"><w:pPr><w:spacing w:line="480" w:lineRule="exact"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Child"><w:basedOn w:val="Base"/><w:pPr><w:spacing w:line="600"/></w:pPr></w:style>"#;
        let doc = word(
            r#"<w:p><w:pPr><w:pStyle w:val="Child"/><w:spacing w:lineRule="atLeast"/></w:pPr><w:r><w:t>first</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Base"/><w:spacing w:lineRule="auto"/></w:pPr><w:r><w:t>second</w:t></w:r></w:p>"#,
            styles,
            "",
        )?;
        assert_eq!(doc.paragraphs[0].line_points, 30.0);
        assert!(!doc.paragraphs[0].line_exact);
        assert_eq!(doc.paragraphs[1].line_points, 0.0);
        assert_eq!(doc.paragraphs[1].line_spacing, 2.0);
        Ok(())
    }
    #[test]
    fn word_line_unit_paragraph_spacing() -> Result<(), String> {
        let doc = word(
            r#"<w:p><w:pPr><w:spacing w:before="999" w:after="999" w:beforeLines="150" w:afterLines="50" w:line="400" w:lineRule="exact"/></w:pPr><w:r><w:t>text</w:t></w:r></w:p>"#,
            "",
            "",
        )?;
        assert_eq!(doc.paragraphs[0].spacing_before, 30.0);
        assert_eq!(doc.paragraphs[0].spacing_after, 10.0);
        Ok(())
    }
    #[test]
    fn word_legacy_horizontal_merges() -> Result<(), String> {
        let doc = word(
            r#"<w:tbl><w:tblGrid><w:gridCol w:w="200"/><w:gridCol w:w="400"/><w:gridCol w:w="600"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:hMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:hMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p><w:r><w:t>C</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:tcPr><w:hMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>D</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:hMerge/></w:tcPr><w:p><w:r><w:t>E</w:t></w:r></w:p></w:tc><w:tc><w:p/></w:tc></w:tr></w:tbl>"#,
            "",
            "",
        )?;
        let table = &doc.tables[0];
        assert_eq!(table.rows[0].len(), 2);
        assert_eq!(table.rows[0][0].span(), 2);
        assert_eq!(table.column_widths, vec![10.0, 20.0, 30.0]);
        assert_eq!(cell_text(&table.rows[0][0]), "A");
        assert_eq!(cell_text(&table.rows[1][0]), "D\nE");
        assert!(doc.losses.iter().any(|loss| loss.contains("hMerge")));
        Ok(())
    }
    #[test]
    fn word_image_crop_keeps_visible_pixels() -> Result<(), String> {
        let doc = image_word("", r#"<a:srcRect r="50000"/>"#)?;
        let image = &doc.images[0];
        assert_eq!((image.width, image.height), (72.0, 36.0));
        let decoded = crate::util::decode_image(&image.bytes)?;
        assert_eq!((decoded.width(), decoded.height()), (1, 1));
        assert_eq!(decoded.to_rgba8().get_pixel(0, 0).0, [255, 0, 0, 255]);
        Ok(())
    }
    #[test]
    fn word_image_title_is_alt_fallback() -> Result<(), String> {
        assert_eq!(
            image_word(r#"title="Accessible title""#, "")?.images[0].alt,
            "Accessible title"
        );
        assert_eq!(
            image_word(r#"title="Title" descr="Description""#, "")?.images[0].alt,
            "Description"
        );
        assert_eq!(
            image_word(r#"title="Title" descr="""#, "")?.images[0].alt,
            "Title"
        );
        Ok(())
    }
    #[test]
    fn word_custom_outline_levels() -> Result<(), String> {
        let styles = r#"<w:style w:type="paragraph" w:styleId="Custom"><w:name w:val="Section"/><w:pPr><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:sz w:val="48"/></w:rPr></w:style>"#;
        let doc = word(
            r#"<w:p><w:pPr><w:pStyle w:val="Custom"/></w:pPr><w:r><w:t>heading</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Custom"/><w:outlineLvl w:val="9"/></w:pPr><w:r><w:t>body</w:t></w:r></w:p>"#,
            styles,
            "",
        )?;
        assert_eq!(doc.paragraphs[0].style, "Heading3");
        assert_eq!(doc.paragraphs[0].runs[0].format.size, 24.0);
        assert_eq!(doc.paragraphs[1].style, "Normal");
        Ok(())
    }
    #[test]
    fn word_table_column_width_precision() -> Result<(), String> {
        let mut doc = Doc::from_text("anchor");
        doc.add_table(doc.paragraphs[0].id, 1, 2)?;
        doc.tables[0].column_widths = vec![2500.0, 0.25];
        let reopened = read(&write(&doc, "docx")?, "docx")?;
        assert_eq!(reopened.tables[0].column_widths, vec![2500.0, 0.25]);
        let layout = reopened.layout();
        let ratio = layout.cells[0].w / layout.cells[1].w;
        assert!((ratio - 10000.0).abs() < 2.0, "{ratio}");
        Ok(())
    }
    #[test]
    fn html_nested_lists_are_structural() -> Result<(), String> {
        let mut doc = Doc::from_text("parent\nchild\nother\nparent two\ndeep");
        for (paragraph, (kind, level)) in doc.paragraphs.iter_mut().zip([
            (List::Number, 0),
            (List::Bullet, 1),
            (List::Number, 1),
            (List::Number, 0),
            (List::Number, 3),
        ]) {
            paragraph.list = kind;
            paragraph.list_level = level;
        }
        let exported = write(&doc, "html")?;
        let text = std::str::from_utf8(&exported).map_err(|error| error.to_string())?;
        let root = xml(text.trim_start_matches("<!DOCTYPE html>").as_bytes())?;
        let list = root.descendant("ol").ok_or("ol")?;
        assert!(list.child("li").and_then(|node| node.child("ul")).is_some());
        let imported = read(&exported, "html")?;
        assert_eq!(imported.plain_text(), doc.plain_text());
        assert_eq!(
            imported
                .paragraphs
                .iter()
                .map(|paragraph| (paragraph.list, paragraph.list_level))
                .collect::<Vec<_>>(),
            doc.paragraphs
                .iter()
                .map(|paragraph| (paragraph.list, paragraph.list_level))
                .collect::<Vec<_>>()
        );
        Ok(())
    }
    #[test]
    fn html_table_headers_roundtrip() -> Result<(), String> {
        let mut doc = Doc::from_text("anchor");
        doc.add_table(doc.paragraphs[0].id, 3, 1)?;
        doc.tables[0].header_rows = 2;
        let exported = write(&doc, "html")?;
        let text = String::from_utf8_lossy(&exported);
        assert!(text.contains("<thead>") && text.contains("<tbody>"));
        assert_eq!(text.matches("<th ").count(), 2);
        assert_eq!(read(&exported, "html")?.tables[0].header_rows, 2);
        doc.tables[0].header_rows = 1;
        doc.tables[0].rows[1][0].merged = true;
        let imported = read(&write(&doc, "html")?, "html")?;
        assert_eq!(imported.tables[0].header_rows, 1);
        assert!(imported.tables[0].rows[1][0].merged);
        Ok(())
    }
}
