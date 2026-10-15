use super::model::*;
use crate::package::{read_zip, write_zip, xml_escape as esc};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

const XML_LIMIT: usize = 32 * 1024 * 1024;
#[derive(Clone, Debug, Default)]
struct Node {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
    text: String,
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
                for attribute in e.attributes() {
                    let a = attribute.map_err(|e| e.to_string())?;
                    node.attrs.insert(
                        local(a.key.as_ref()),
                        a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(|e| e.to_string())?
                            .into_owned(),
                    );
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
                }
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err("Несогласованная структура XML".into());
                }
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
    let node = elements.next().ok_or("Пустой XML")?;
    if node.name == "#text" || elements.next().is_some() {
        return Err("XML должен содержать ровно один корневой элемент".into());
    }
    Ok(node)
}
pub fn read(bytes: &[u8], ext: &str) -> Result<Doc, String> {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    let mut doc = match ext.as_str() {
        "docx" | "dotx" | "docm" => read_docx(bytes)?,
        "txt" => Doc::from_text(&decode_text(bytes)?),
        "md" | "markdown" => read_markdown(&decode_text(bytes)?),
        "html" | "htm" => read_html(&decode_text(bytes)?)?,
        _ => return Err(format!("Неподдерживаемый формат Docs: {ext}")),
    };
    if ext == "docm" {
        doc.losses
            .push("Макросы DOCM не исполняются и не сохраняются.".into());
    }
    doc.normalize();
    doc.validate()?;
    Ok(doc)
}
pub fn write(doc: &Doc, ext: &str) -> Result<Vec<u8>, String> {
    doc.validate()?;
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    match ext.as_str() {
        "docx" | "dotx" | "docm" => write_docx(doc, &ext),
        "txt" => Ok(doc.plain_text().into_bytes()),
        "md" | "markdown" => Ok(write_markdown(doc).into_bytes()),
        "html" | "htm" => Ok(write_html(doc).into_bytes()),
        _ => Err(format!("Неподдерживаемый формат Docs: {ext}")),
    }
}
fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Текст превышает лимит 64 МБ".into());
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
    let content = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if let Ok(text) = std::str::from_utf8(content) {
        return Ok(text.into());
    }
    // Windows-1251 fallback for legacy Russian text.
    let special: Vec<char> =
        "ЂЃ‚ѓ„…†‡€‰Љ‹ЊЌЋЏђ‘’“”•–—\u{0098}™љ›њќћџ\u{00a0}ЎўЈ¤Ґ¦§Ё©Є«¬\u{00ad}®Ї°±Ііґµ¶·ё№є»јЅѕї"
            .chars()
            .collect();
    Ok(content
        .iter()
        .map(|b| match *b {
            0..=127 => char::from(*b),
            128..=191 => special
                .get(usize::from(*b) - 128)
                .copied()
                .unwrap_or('\u{fffd}'),
            _ => char::from_u32(0x410 + u32::from(*b) - 192).unwrap_or('\u{fffd}'),
        })
        .collect())
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
    if let Some(v) = bool_prop(node, "strike") {
        format.strike = v;
    }
    if let Some(n) = node.child("u") {
        format.underline = n.value() != Some("none");
    }
    if let Some(n) = node.child("sz") {
        format.size = number(n.value(), format.size * 2.0) / 2.0;
    }
    if let Some(n) = node.child("color").and_then(Node::value).and_then(color) {
        format.color = n;
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
    if let Some(n) = node
        .child("shd")
        .and_then(|n| n.attr("fill"))
        .and_then(color)
    {
        format.highlight = Some(n);
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
fn apply_ppr(p: &mut Paragraph, node: &Node, num_formats: &BTreeMap<String, List>) {
    if let Some(style) = node.child("pStyle").and_then(Node::value) {
        p.style = style.into();
    }
    if let Some(align) = node.child("jc").and_then(Node::value) {
        p.align = match align {
            "center" => Align::Center,
            "right" => Align::Right,
            "both" | "distribute" => Align::Justify,
            _ => Align::Left,
        };
    }
    if let Some(n) = node.child("numPr") {
        p.list_level = number(n.child("ilvl").and_then(Node::value), 0.0).clamp(0.0, 8.0) as u8;
        p.list = match n.child("numId").and_then(Node::value) {
            Some("0") => List::None,
            Some(id) => num_formats.get(id).copied().unwrap_or(List::Bullet),
            None => p.list,
        };
    }
    if let Some(n) = node.child("spacing") {
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
    // Attributes missing from `w:ind` keep the value inherited from the style.
    if let Some(n) = node.child("ind") {
        p.indent = number(n.attr("left").or_else(|| n.attr("start")), p.indent * 20.0) / 20.0;
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
/// Body-level blocks with content controls (`w:sdt`) and custom XML unwrapped: their
/// paragraphs and tables are ordinary document content.
fn body_blocks<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>, depth: usize) {
    for node in nodes {
        match node.name.as_str() {
            "sdt" if depth < 32 => {
                if let Some(content) = node.child("sdtContent") {
                    body_blocks(&content.children, out, depth + 1);
                }
            }
            "customXml" if depth < 32 => body_blocks(&node.children, out, depth + 1),
            _ => out.push(node),
        }
    }
}
fn style_chain<'a>(
    styles: &'a BTreeMap<String, Node>,
    id: &str,
    out: &mut Vec<&'a Node>,
    depth: usize,
) {
    if depth > 12 {
        return;
    }
    if let Some(style) = styles.get(id) {
        if let Some(base) = style.child("basedOn").and_then(Node::value) {
            style_chain(styles, base, out, depth + 1);
        }
        out.push(style);
    }
}
/// Document-wide defaults of a DOCX: run format and paragraph layout (`w:docDefaults`).
struct Defaults<'a> {
    format: &'a TextFormat,
    paragraph: &'a Paragraph,
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
) -> Paragraph {
    let id = doc.alloc_id();
    let default_format = defaults.format;
    let mut p = Paragraph {
        id,
        runs: Vec::new(),
        ..defaults.paragraph.clone()
    };
    let mut format = default_format.clone();
    let style = node
        .child("pPr")
        .and_then(|n| n.child("pStyle"))
        .and_then(Node::value)
        .unwrap_or(defaults.normal);
    // Word layers a table style between the document defaults and the paragraph style,
    // which is why "Table Grid" cells have no spacing after paragraphs.
    let mut chain = table_style.to_vec();
    style_chain(styles, style, &mut chain, 0);
    for n in chain {
        if let Some(rpr) = n.child("rPr") {
            apply_rpr(&mut format, rpr);
        }
        if let Some(ppr) = n.child("pPr") {
            apply_ppr(&mut p, ppr, nums);
        }
    }
    if let Some(n) = node.child("pPr") {
        apply_ppr(&mut p, n, nums);
    }
    p.style = canonical_style(styles, style);
    let mut nodes = Vec::new();
    collect_runs(node, rels, None, &mut nodes);
    for (run, link) in nodes {
        let mut fmt = format.clone();
        fmt.link = link;
        if let Some(n) = run.child("rPr") {
            if let Some(s) = n
                .child("rStyle")
                .and_then(Node::value)
                .and_then(|id| styles.get(id))
                .and_then(|s| s.child("rPr"))
            {
                apply_rpr(&mut fmt, s);
            }
            apply_rpr(&mut fmt, n);
        }
        let mut text = String::new();
        for n in &run.children {
            match n.name.as_str() {
                "t" => text.push_str(&n.text_content()),
                "tab" => text.push('\t'),
                "br" | "cr" => {
                    if n.attr("type") == Some("page") {
                        text.push('\u{000c}');
                    } else {
                        text.push('\u{000b}');
                    }
                }
                _ => {}
            }
        }
        if !text.is_empty() {
            p.runs.push(Run { text, format: fmt });
        }
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
    p
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
    Some(components.join("/"))
}
fn read_docx(bytes: &[u8]) -> Result<Doc, String> {
    let parts = read_zip(bytes)?;
    let root = xml(parts
        .get("word/document.xml")
        .ok_or("В DOCX отсутствует word/document.xml")?)?;
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
    let mut default_paragraph = Paragraph {
        spacing_before: 0.0,
        spacing_after: 0.0,
        line_spacing: 1.0,
        ..Paragraph::default()
    };
    if let Some(bytes) = parts.get("word/styles.xml") {
        let node = xml(bytes)?;
        if let Some(n) = node.descendant("rPrDefault").and_then(|n| n.child("rPr")) {
            apply_rpr(&mut default_format, n);
        }
        if let Some(n) = node.descendant("pPrDefault").and_then(|n| n.child("pPr")) {
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
    if let Some(bytes) = parts.get("word/numbering.xml") {
        let node = xml(bytes)?;
        let mut abstracts = BTreeMap::new();
        for n in &node.children {
            if n.name == "abstractNum" {
                let kind = if n.descendant("numFmt").and_then(Node::value) == Some("bullet") {
                    List::Bullet
                } else {
                    List::Number
                };
                if let Some(id) = n.attr("abstractNumId") {
                    abstracts.insert(id.to_string(), kind);
                }
            }
        }
        for n in &node.children {
            if n.name == "num"
                && let Some(id) = n.attr("numId")
            {
                let kind = n
                    .child("abstractNumId")
                    .and_then(Node::value)
                    .and_then(|id| abstracts.get(id))
                    .copied()
                    .unwrap_or(List::Bullet);
                nums.insert(id.to_string(), kind);
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
        paragraph: &default_paragraph,
        normal: &normal,
    };
    let mut blocks = Vec::new();
    body_blocks(&body.children, &mut blocks, 0);
    for node in blocks {
        match node.name.as_str() {
            "p" => {
                let p = paragraph(node, &mut doc, &styles, &defaults, &[], &nums, &rels);
                let mut drawings = Vec::new();
                node.descendants("drawing", &mut drawings);
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
                            .and_then(|n| n.attr("descr"))
                            .unwrap_or_default()
                            .to_string();
                        let image_id = doc.alloc_id();
                        doc.images.push(AnchoredImage {
                            id: image_id,
                            after_paragraph_id: id,
                            bytes: bytes.clone(),
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
                if let Some(style) = tbl_pr
                    .and_then(|n| n.child("tblStyle"))
                    .and_then(Node::value)
                {
                    style_chain(&styles, style, &mut chain, 0);
                }
                let mut rows = Vec::new();
                let mut row_heights = Vec::new();
                // Leading rows marked as headers repeat on every page.
                let header_rows = node
                    .children
                    .iter()
                    .filter(|n| n.name == "tr")
                    .take_while(|tr| {
                        tr.child("trPr")
                            .and_then(|n| bool_prop(n, "tblHeader"))
                            .unwrap_or(false)
                    })
                    .count();
                if node.children.iter().filter(|n| n.name == "tr").count() > 10000 {
                    return Err("Таблица DOCX превышает 10 000 строк".into());
                }
                for tr in node.children.iter().filter(|n| n.name == "tr") {
                    row_heights.push(
                        tr.child("trPr")
                            .and_then(|n| n.child("trHeight"))
                            .filter(|n| n.attr("hRule") != Some("auto"))
                            .map(|n| (number(n.attr("val"), 0.0) / 20.0).clamp(0.0, 1500.0))
                            .unwrap_or_default(),
                    );
                    let mut cells = Vec::new();
                    if tr.children.iter().filter(|n| n.name == "tc").count() > 256 {
                        return Err("Таблица DOCX превышает 256 столбцов".into());
                    }
                    for tc in tr.children.iter().filter(|n| n.name == "tc") {
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
                        for p in tc.children.iter().filter(|n| n.name == "p") {
                            paragraphs.push(paragraph(
                                p, &mut doc, &styles, &defaults, &chain, &nums, &rels,
                            ));
                        }
                        if paragraphs.is_empty() {
                            let id = doc.alloc_id();
                            paragraphs.push(Paragraph {
                                id,
                                ..Paragraph::default()
                            });
                        }
                        let tc_pr = tc.child("tcPr");
                        let fill = tc_pr
                            .and_then(|n| n.child("shd"))
                            .and_then(|n| n.attr("fill"))
                            .and_then(color)
                            .unwrap_or([255, 255, 255]);
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
                        cell.v_align =
                            match tc_pr.and_then(|n| n.child("vAlign")).and_then(Node::value) {
                                Some("center") => VAlign::Center,
                                Some("bottom") => VAlign::Bottom,
                                _ => VAlign::Top,
                            };
                        cells.push(cell);
                    }
                    rows.push(cells);
                }
                let widths: Vec<f32> = node
                    .child("tblGrid")
                    .map(|g| {
                        g.children
                            .iter()
                            .filter(|n| n.name == "gridCol")
                            .map(|n| (number(n.attr("w"), 1440.0) / 20.0).clamp(1.0, 2000.0))
                            .collect()
                    })
                    .unwrap_or_default();
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
    if let Some(core) = parts.get("docProps/core.xml") {
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
            let mut depth = 0usize;
            let mut text = String::new();
            fn visible_header(
                n: &Node,
                depth: &mut usize,
                gap: &mut &'static str,
                out: &mut String,
            ) {
                if matches!(
                    n.name.as_str(),
                    "fldSimple" | "del" | "moveFrom" | "pPr" | "rPr"
                ) {
                    return;
                }
                // Paragraphs and tabs keep words apart on the single header line; the gap is
                // written only before following text, so author spaces survive a round trip.
                if (n.name == "p" || n.name == "tab") && !out.is_empty() && !out.ends_with(' ') {
                    *gap = if n.name == "tab" || *gap == "    " {
                        "    "
                    } else {
                        "  "
                    };
                }
                if n.name == "fldChar" {
                    match n.attr("fldCharType") {
                        Some("begin") => *depth += 1,
                        Some("end") => *depth = depth.saturating_sub(1),
                        _ => {}
                    }
                } else if n.name == "t" && *depth == 0 {
                    let text = n.text_content();
                    if !text.is_empty() {
                        out.push_str(gap);
                        *gap = "";
                        out.push_str(&text);
                    }
                } else {
                    for child in &n.children {
                        visible_header(child, depth, gap, out);
                    }
                }
            }
            visible_header(&n, &mut depth, &mut "", &mut text);
            if header {
                doc.page.header = text;
            } else {
                doc.page.footer = text;
            }
            if n.descendant("instrText")
                .is_some_and(|n| n.text_content().contains("PAGE"))
                || n.descendant("fldSimple")
                    .is_some_and(|n| n.attr("instr").is_some_and(|s| s.contains("PAGE")))
            {
                doc.page.page_numbers = true;
            }
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
    for ch in run.text.chars() {
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
        } else if ch >= ' ' {
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
fn p_xml(p: &Paragraph, links: &mut Vec<(String, String)>) -> String {
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
            if p.list == List::Bullet { 1 } else { 2 }
        ));
    }
    properties.push_str(&format!(
        "<w:spacing w:before=\"{}\" w:after=\"{}\" {line}/><w:ind w:left=\"{}\" {first_indent}/>{}<w:jc w:val=\"{}\"/>",
        (p.spacing_before * 20.0).round() as u32,
        (p.spacing_after * 20.0).round() as u32,
        (p.indent * 20.0).round() as i32,
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
fn table_xml(table: &Table, usable_width: f32, links: &mut Vec<(String, String)>) -> String {
    let columns = table.grid_columns().max(1);
    let widths: Vec<u32> = (0..columns)
        .map(|i| {
            let pt = table
                .column_widths
                .get(i)
                .copied()
                .unwrap_or(usable_width / columns as f32)
                .max(12.0);
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
                out.push_str(&p_xml(p, links));
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
    let mut body = String::new();
    let mut media = Vec::new();
    let usable = doc.page.width - doc.page.margin_left - doc.page.margin_right;
    let gap = format!(
        "<w:p><w:pPr><w:pStyle w:val=\"{TABLE_GAP}\"/><w:spacing w:before=\"0\" w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/><w:rPr><w:sz w:val=\"2\"/></w:rPr></w:pPr></w:p>"
    );
    let last_paragraph = doc.paragraphs.last().map(|p| p.id);
    for p in &doc.paragraphs {
        body.push_str(&p_xml(p, &mut links));
        let tables: Vec<&Table> = doc
            .tables
            .iter()
            .filter(|t| t.after_paragraph_id == p.id)
            .collect();
        let has_images = doc.images.iter().any(|im| im.after_paragraph_id == p.id);
        for (i, table) in tables.iter().enumerate() {
            body.push_str(&table_xml(table, usable, &mut links));
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
    body.push_str(&format!("<w:sectPr><w:headerReference w:type=\"default\" r:id=\"header\"/><w:footerReference w:type=\"default\" r:id=\"footer\"/><w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"400\" w:footer=\"400\"/></w:sectPr>",
        (setup.width*20.0).round() as u32,(setup.height*20.0).round() as u32,(setup.margin_top*20.0).round() as u32,
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
            numbering.push_str(&format!("<w:lvl w:ilvl=\"{level}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"{kind}\"/><w:lvlText w:val=\"{marker}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"{}\" w:hanging=\"360\"/></w:pPr></w:lvl>",360+level*360));
        }
        numbering.push_str(&format!(
            "</w:abstractNum><w:num w:numId=\"{id}\"><w:abstractNumId w:val=\"{id}\"/></w:num>"
        ));
    }
    numbering.push_str("</w:numbering>");
    let header = format!(
        "<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:hdr>",
        esc(&setup.header)
    );
    let footer = format!(
        "<w:ftr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:p><w:r><w:t>{}</w:t></w:r>{}</w:p></w:ftr>",
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
fn inline_md(text: &str) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let markers = [
            ("**", true, false, true),
            ("__", true, false, true),
            ("~~", false, false, true),
            ("*", false, true, false),
            ("_", false, true, false),
        ];
        let found = markers
            .iter()
            .filter_map(|m| remaining.find(m.0).map(|pos| (pos, *m)))
            .min_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.0.len().cmp(&a.1.0.len())));
        if let Some((pos, (marker, bold, italic, _))) = found {
            let prefix = remaining.get(..pos).unwrap_or_default();
            if !prefix.is_empty() {
                runs.push(Run {
                    text: prefix.into(),
                    format: TextFormat::default(),
                });
            }
            let rest = remaining.get(pos + marker.len()..).unwrap_or_default();
            if let Some(end) = rest.find(marker) {
                runs.push(Run {
                    text: rest.get(..end).unwrap_or_default().into(),
                    format: TextFormat {
                        bold,
                        italic,
                        strike: marker == "~~",
                        ..TextFormat::default()
                    },
                });
                remaining = rest.get(end + marker.len()..).unwrap_or_default();
            } else {
                runs.push(Run {
                    text: remaining.get(pos..).unwrap_or_default().into(),
                    format: TextFormat::default(),
                });
                break;
            }
        } else {
            runs.push(Run {
                text: remaining.into(),
                format: TextFormat::default(),
            });
            break;
        }
    }
    if runs.is_empty() {
        runs.push(Run::default());
    }
    runs
}
fn read_markdown(text: &str) -> Doc {
    let mut doc = Doc::default();
    doc.paragraphs.clear();
    for line in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let id = doc.alloc_id();
        let mut p = Paragraph {
            id,
            ..Paragraph::default()
        };
        let mut text = line;
        let headings = line.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&headings) && line.as_bytes().get(headings) == Some(&b' ') {
            p.set_style(&format!("Heading{}", headings.min(3)));
            text = line.get(headings + 1..).unwrap_or_default();
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            p.list = List::Bullet;
            text = rest;
        } else if let Some((prefix, rest)) = line
            .split_once(". ")
            .filter(|(prefix, _)| !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()))
        {
            let _ = prefix;
            p.list = List::Number;
            text = rest;
        } else if let Some(rest) = line.strip_prefix("> ") {
            p.set_style("Quote");
            text = rest;
        }
        p.runs = inline_md(text);
        if p.style != "Normal" {
            let style = p.style.clone();
            p.set_style(&style);
        }
        doc.paragraphs.push(p);
    }
    doc
}
fn write_markdown(doc: &Doc) -> String {
    let mut out = String::new();
    for p in &doc.paragraphs {
        if p.style.starts_with("Heading") {
            let level = p
                .style
                .trim_start_matches("Heading")
                .parse::<usize>()
                .unwrap_or(1)
                .clamp(1, 6);
            out.push_str(&"#".repeat(level));
            out.push(' ');
        } else if p.style == "Quote" {
            out.push_str("> ");
        }
        match p.list {
            List::Bullet => out.push_str("- "),
            List::Number => out.push_str("1. "),
            List::None => {}
        }
        for r in &p.runs {
            let marker = if r.format.bold {
                "**"
            } else if r.format.italic {
                "*"
            } else if r.format.strike {
                "~~"
            } else {
                ""
            };
            out.push_str(marker);
            if let Some(link) = &r.format.link {
                out.push_str(&format!("[{}]({})", r.text, link));
            } else {
                out.push_str(&r.text.replace('\u{000b}', "  \n"));
            }
            out.push_str(marker);
        }
        out.push('\n');
        for table in doc.tables.iter().filter(|t| t.after_paragraph_id == p.id) {
            for (ri, row) in table.rows.iter().enumerate() {
                out.push('|');
                for cell in row {
                    out.push_str(
                        &cell
                            .paragraphs
                            .iter()
                            .map(Paragraph::text)
                            .collect::<Vec<_>>()
                            .join(" ")
                            .replace('|', "\\|"),
                    );
                    out.push('|');
                }
                out.push('\n');
                if ri == 0 {
                    out.push('|');
                    for _ in row {
                        out.push_str("---|");
                    }
                    out.push('\n');
                }
            }
        }
    }
    out
}
fn css_points(value: &str, fallback: f32) -> f32 {
    let value = value.trim();
    let factor = if value.ends_with("px") { 0.75 } else { 1.0 };
    number(
        Some(value.trim_end_matches("pt").trim_end_matches("px")),
        fallback / factor,
    ) * factor
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
fn write_html(doc: &Doc) -> String {
    let mut out = format!(
        "<!DOCTYPE html><html lang=\"ru\"><head><meta charset=\"utf-8\"/><title>{}</title></head><body>",
        esc(&doc.title)
    );
    let mut list = List::None;
    let mut numbered = false;
    let mut ordinal = 0;
    for p in &doc.paragraphs {
        ordinal = if p.list == List::Number {
            if numbered { ordinal + 1 } else { 1 }
        } else {
            0
        };
        numbered = p.list == List::Number;
        if list != p.list {
            if list != List::None {
                out.push_str(if list == List::Number {
                    "</ol>"
                } else {
                    "</ul>"
                });
            }
            list = p.list;
            if list != List::None {
                if list == List::Number {
                    out.push_str(&format!("<ol start=\"{ordinal}\">"));
                } else {
                    out.push_str("<ul>");
                }
            }
        }
        let tag = match (p.list, p.style.as_str()) {
            (List::Number | List::Bullet, _) => "li",
            (_, "Heading1") => "h1",
            (_, "Heading2") => "h2",
            (_, "Heading3") => "h3",
            (_, "Quote") => "blockquote",
            _ => "p",
        };
        out.push_str(&format!(
            "<{tag} style=\"white-space:pre-wrap;text-align:{};margin-left:{}pt;line-height:{}\">",
            match p.align {
                Align::Left => "left",
                Align::Center => "center",
                Align::Right => "right",
                Align::Justify => "justify",
            },
            p.indent,
            p.line_spacing
        ));
        for r in &p.runs {
            let f = &r.format;
            out.push_str(&format!("<span style=\"font-family:{};font-size:{}pt;color:#{};font-weight:{};font-style:{};text-decoration:{}{};{}\">",esc(&f.font),f.size,hex(f.color),if f.bold{"bold"}else{"normal"},if f.italic{"italic"}else{"normal"},if f.underline{"underline "}else{""},if f.strike{"line-through"}else{""},f.highlight.map(|c|format!("background-color:#{}",hex(c))).unwrap_or_default()));
            let link = f.link.as_deref().filter(|link| safe_html_link(link));
            if let Some(link) = link {
                out.push_str(&format!("<a href=\"{}\">", esc(link)));
            }
            if f.script != 0 {
                out.push_str(if f.script > 0 { "<sup>" } else { "<sub>" });
            }
            out.push_str(&esc(&r.text).replace(['\u{000b}', '\n'], "<br/>"));
            if f.script != 0 {
                out.push_str(if f.script > 0 { "</sup>" } else { "</sub>" });
            }
            if link.is_some() {
                out.push_str("</a>");
            }
            out.push_str("</span>");
        }
        out.push_str(&format!("</{tag}>"));
        for table in doc.tables.iter().filter(|t| t.after_paragraph_id == p.id) {
            if list != List::None {
                out.push_str(if list == List::Number {
                    "</ol>"
                } else {
                    "</ul>"
                });
                list = List::None;
            }
            out.push_str("<table style=\"border-collapse:collapse\">");
            for row in &table.rows {
                out.push_str("<tr>");
                for cell in row {
                    out.push_str(&format!(
                        "<td style=\"white-space:pre-wrap;border:1px solid #{};background:#{}\">{}</td>",
                        hex(table.border_color),
                        hex(cell.fill),
                        esc(&cell
                            .paragraphs
                            .iter()
                            .map(Paragraph::text)
                            .collect::<Vec<_>>()
                            .join("\n"))
                    ));
                }
                out.push_str("</tr>");
            }
            out.push_str("</table>");
        }
    }
    if list != List::None {
        out.push_str(if list == List::Number {
            "</ol>"
        } else {
            "</ul>"
        });
    }
    out.push_str("</body></html>");
    out
}
fn fallback_html_text(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut plain = String::new();
    let mut offset = 0;
    let mut hidden: Option<&str> = None;
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
            if closing && hidden == Some(name) {
                hidden = None;
            } else if !closing && hidden.is_none() && matches!(name, "script" | "style" | "head") {
                hidden = Some(name);
            } else if hidden.is_none() && matches!(name, "br" | "p" | "div" | "li" | "tr") {
                plain.push('\n');
            }
            offset += end + 1;
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            if hidden.is_none() {
                plain.push_str(&rest[..end]);
            }
            offset += end;
        }
    }
    plain
}
fn read_html(text: &str) -> Result<Doc, String> {
    // HTML is imported as data only; scripts, CSS imports and network images are never executed.
    let mut clean = text
        .replace("&nbsp;", "&#160;")
        .replace("<br>", "<br/>")
        .replace("<BR>", "<br/>");
    if let Some(start) = clean.find("<!DOCTYPE").or_else(|| clean.find("<!doctype"))
        && let Some(end) = clean.get(start..).and_then(|s| s.find('>'))
    {
        clean.replace_range(start..=start + end, "");
    }
    let node = xml(clean.as_bytes()).or_else(|_| {
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
    fn walk(node: &Node, doc: &mut Doc, inherited_css: &str, list: Option<List>) {
        if matches!(node.name.as_str(), "script" | "style" | "head") {
            return;
        }
        let css = format!("{inherited_css};{}", node.attr("style").unwrap_or_default());
        let list = match node.name.as_str() {
            "ol" => Some(List::Number),
            "ul" => Some(List::Bullet),
            _ => list,
        };
        let block = matches!(
            node.name.as_str(),
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "li"
        );
        if block {
            let id = doc.alloc_id();
            let mut p = Paragraph {
                id,
                runs: Vec::new(),
                ..Paragraph::default()
            };
            fn runs(n: &Node, f: &TextFormat, out: &mut Vec<Run>) {
                if matches!(n.name.as_str(), "script" | "style" | "ul" | "ol") {
                    return;
                }
                let mut fmt = f.clone();
                match n.name.as_str() {
                    "b" | "strong" => fmt.bold = true,
                    "i" | "em" => fmt.italic = true,
                    "u" => fmt.underline = true,
                    "s" | "del" => fmt.strike = true,
                    "sup" => fmt.script = 1,
                    "sub" => fmt.script = -1,
                    "a" => fmt.link = n.attr("href").map(str::to_string),
                    _ => {}
                }
                if let Some(css) = n.attr("style") {
                    for declaration in css.split(';') {
                        if let Some((key, value)) = declaration.split_once(':') {
                            match key.trim() {
                                "font-size" => fmt.size = css_points(value, 11.0),
                                "font-family" => {
                                    fmt.font =
                                        value.trim().trim_matches('\'').trim_matches('"').into()
                                }
                                "color" => {
                                    if let Some(c) = color(value.trim().trim_start_matches('#')) {
                                        fmt.color = c;
                                    }
                                }
                                "background-color" => {
                                    fmt.highlight = color(value.trim().trim_start_matches('#'))
                                }
                                "font-weight" => {
                                    fmt.bold = value.trim() == "bold" || value.trim() == "700"
                                }
                                "font-style" => fmt.italic = value.trim() == "italic",
                                "text-decoration" => {
                                    fmt.underline = value.contains("underline");
                                    fmt.strike = value.contains("line-through");
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if !n.text.is_empty() {
                    out.push(Run {
                        text: n.text.clone(),
                        format: fmt.clone(),
                    });
                }
                for c in &n.children {
                    if c.name == "br" {
                        out.push(Run {
                            text: "\u{000b}".into(),
                            format: fmt.clone(),
                        });
                    } else {
                        runs(c, &fmt, out);
                    }
                }
            }
            let mut styled = node.clone();
            styled.attrs.insert("style".into(), css.clone());
            runs(&styled, &TextFormat::default(), &mut p.runs);
            match node.name.as_str() {
                "h1" => p.style = "Heading1".into(),
                "h2" => p.style = "Heading2".into(),
                "h3" | "h4" | "h5" | "h6" => p.style = "Heading3".into(),
                "blockquote" => p.style = "Quote".into(),
                "li" => p.list = list.unwrap_or(List::Bullet),
                _ => {}
            }
            for declaration in css.split(';') {
                if let Some((key, value)) = declaration.split_once(':') {
                    match key.trim() {
                        "text-align" => {
                            p.align = match value.trim() {
                                "center" => Align::Center,
                                "right" => Align::Right,
                                "justify" => Align::Justify,
                                _ => Align::Left,
                            }
                        }
                        "margin-left" => p.indent = css_points(value, 0.0),
                        "line-height" => {
                            p.line_spacing = number(Some(value.trim()), 1.28).clamp(0.5, 5.0)
                        }
                        _ => {}
                    }
                }
            }
            p.normalize();
            doc.paragraphs.push(p);
            for child in node
                .children
                .iter()
                .filter(|n| matches!(n.name.as_str(), "ol" | "ul"))
            {
                walk(child, doc, &css, list);
            }
        } else {
            fn boundary(node: &Node) -> bool {
                matches!(
                    node.name.as_str(),
                    "p" | "h1"
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
            fn flush(inline: &mut Vec<Node>, doc: &mut Doc, css: &str) {
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
                    walk(&paragraph, doc, "", None);
                }
            }
            let mut inline = Vec::new();
            for child in &node.children {
                if boundary(child) {
                    flush(&mut inline, doc, &css);
                    walk(child, doc, &css, list);
                } else {
                    inline.push(child.clone());
                }
            }
            flush(&mut inline, doc, &css);
        }
    }
    walk(&node, &mut doc, "", None);
    doc.losses.push(
        "HTML: сложные CSS-макеты и сетевые изображения не импортируются; таблицы преобразуются в абзацы.".into(),
    );
    doc.normalize();
    Ok(doc)
}
#[cfg(test)]
mod tests {
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
            "before bold\nmiddle\nafter\nHeader\nCell one\nCell two\nend"
        );
        assert!(
            doc.paragraphs
                .first()
                .is_some_and(|p| p.runs.iter().any(|r| r.text == "bold" && r.format.bold))
        );
        assert!(
            doc.losses
                .iter()
                .any(|loss| loss.contains("таблицы преобразуются"))
        );
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
        assert_eq!(doc.paragraphs[0].style, "Heading3");
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
    fn markdown_legacy_newlines_form_separate_paragraphs() -> Result<(), String> {
        let doc = read(b"first\rsecond\r\nthird", "md")?;
        assert_eq!(doc.plain_text(), "first\nsecond\nthird");
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
