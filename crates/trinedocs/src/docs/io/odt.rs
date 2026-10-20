//! OpenDocument Text: `.odt`/`.ott` packages and flat `.fodt` (ADR-0028, after WordCraft's
//! `formats::odt`, written against the TrineDocs model). Styles are resolved through their
//! parent chains; lists, tables (spans, vertical merges, header rows), pictures, page layout,
//! header and footer and document properties are kept. What has no place in the model is
//! reported in `Doc::losses`.

use super::{Node, color, hex, xml};
use crate::docs::model::*;
use crate::package::{read_zip, write_zip, xml_escape as esc};
use std::collections::{BTreeMap, BTreeSet};

const MIME_TEXT: &str = "application/vnd.oasis.opendocument.text";
const MIME_TEMPLATE: &str = "application/vnd.oasis.opendocument.text-template";
/// Our marker for a paragraph that only carries pictures anchored to the paragraph before it.
const IMAGE_BLOCK: &str = "TrineDocs_20_Image";
const NAMESPACES: &str = "xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\" xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" office:version=\"1.3\"";

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

/// `.odt`/`.ott`: a ZIP package with `content.xml`, `styles.xml` and `meta.xml`.
pub(super) fn read_odt(bytes: &[u8]) -> Result<Doc, String> {
    let parts = read_zip(bytes)?;
    if let Some(mime) = parts.get("mimetype") {
        let mime = String::from_utf8_lossy(mime);
        if !mime.trim().starts_with(MIME_TEXT) {
            return Err(format!(
                "Это не текстовый документ OpenDocument: {}",
                mime.trim()
            ));
        }
    }
    let content = xml(parts.get("content.xml").ok_or("В ODT нет content.xml")?)?;
    let styles = parts.get("styles.xml").map(|b| xml(b)).transpose()?;
    let meta = parts.get("meta.xml").map(|b| xml(b)).transpose()?;
    let mut roots = Vec::new();
    roots.extend(styles.as_ref());
    roots.push(&content);
    read_roots(&roots, meta.as_ref(), &parts)
}

/// `.fodt`: the same document as one XML file, pictures inline as base64.
pub(super) fn read_fodt(text: &str) -> Result<Doc, String> {
    let root = xml(text.as_bytes())?;
    if root.name != "document" {
        return Err("FODT должен начинаться с элемента office:document".into());
    }
    read_roots(&[&root], Some(&root), &BTreeMap::new())
}

fn read_roots(
    roots: &[&Node],
    meta: Option<&Node>,
    parts: &BTreeMap<String, Vec<u8>>,
) -> Result<Doc, String> {
    let mut styles = Styles::default();
    for root in roots {
        styles.collect(root);
    }
    let body = roots
        .iter()
        .rev()
        .find_map(|root| root.child("body"))
        .and_then(|body| body.child("text"))
        .ok_or("В документе OpenDocument нет текста (office:body/office:text)")?;
    let mut reader = Reader {
        styles: &styles,
        parts,
        doc: Doc::default(),
        losses: BTreeSet::new(),
        pending_break: false,
    };
    reader.doc.paragraphs.clear();
    reader.doc.page.page_numbers = false;
    let mut target = Target::Body;
    reader.blocks(&body.children, &mut target, None)?;
    reader.page_setup();
    if let Some(info) = meta.and_then(|m| m.child("meta")) {
        let text = |name: &str| info.child(name).map(Node::text_content);
        reader.doc.title = text("title").unwrap_or_default();
        reader.doc.author = text("creator")
            .or_else(|| text("initial-creator"))
            .unwrap_or_default();
    }
    let Reader {
        mut doc, losses, ..
    } = reader;
    doc.losses.extend(losses.into_iter().map(str::to_string));
    doc.normalize();
    Ok(doc)
}

#[derive(Default)]
struct Styles<'a> {
    named: BTreeMap<(String, String), &'a Node>,
    defaults: BTreeMap<String, &'a Node>,
    lists: BTreeMap<String, &'a Node>,
    layouts: BTreeMap<String, &'a Node>,
    masters: Vec<&'a Node>,
    fonts: BTreeMap<String, String>,
}

impl<'a> Styles<'a> {
    fn collect(&mut self, root: &'a Node) {
        for section in &root.children {
            match section.name.as_str() {
                "font-face-decls" => {
                    for face in section.children.iter().filter(|n| n.name == "font-face") {
                        if let (Some(name), Some(family)) =
                            (face.attr("name"), face.attr("font-family"))
                        {
                            self.fonts.insert(name.into(), unquote(family).into());
                        }
                    }
                }
                "styles" | "automatic-styles" => {
                    for style in &section.children {
                        let name = style.attr("name").unwrap_or_default().to_string();
                        match style.name.as_str() {
                            "style" => {
                                let family = style.attr("family").unwrap_or_default().to_string();
                                self.named.insert((family, name), style);
                            }
                            "default-style" => {
                                let family = style.attr("family").unwrap_or_default().to_string();
                                self.defaults.insert(family, style);
                            }
                            "list-style" => {
                                self.lists.insert(name, style);
                            }
                            "page-layout" => {
                                self.layouts.insert(name, style);
                            }
                            _ => {}
                        }
                    }
                }
                "master-styles" => {
                    self.masters
                        .extend(section.children.iter().filter(|n| n.name == "master-page"));
                }
                _ => {}
            }
        }
    }

    /// The style, its parents and the family default, outermost first.
    fn chain(&self, family: &str, name: Option<&str>) -> Vec<&'a Node> {
        let mut out = Vec::new();
        let mut next = name.map(str::to_string);
        while let Some(name) = next.take() {
            if out.len() >= 16 {
                break;
            }
            let Some(style) = self.named.get(&(family.to_string(), name)) else {
                break;
            };
            out.push(*style);
            next = style.attr("parent-style-name").map(str::to_string);
        }
        if let Some(default) = self.defaults.get(family) {
            out.push(*default);
        }
        out.reverse();
        out
    }

    fn props(&self, family: &str, name: Option<&str>, kind: &str) -> Option<&'a Node> {
        name.and_then(|name| self.named.get(&(family.to_string(), name.to_string())))
            .copied()
            .and_then(|style| style.child(kind))
    }

    fn text_format(&self, name: Option<&str>, base: &TextFormat) -> TextFormat {
        let mut format = base.clone();
        let mut chain = self.chain("text", name);
        // The paragraph default belongs to paragraphs, not to spans inside them.
        chain.retain(|style| style.name == "style");
        for style in chain {
            if let Some(props) = style.child("text-properties") {
                apply_text(&mut format, props, &self.fonts);
            }
        }
        format
    }

    fn list_kind(&self, style: Option<&str>, level: u8) -> List {
        let wanted = (u32::from(level) + 1).to_string();
        let definition = style
            .and_then(|name| self.lists.get(name))
            .and_then(|list| {
                list.children
                    .iter()
                    .find(|n| n.attr("level") == Some(wanted.as_str()))
            });
        match definition {
            Some(n)
                if n.name == "list-level-style-number"
                    && !n.attr("num-format").unwrap_or("1").is_empty() =>
            {
                List::Number
            }
            _ => List::Bullet,
        }
    }
}

fn unquote(value: &str) -> &str {
    value.trim().trim_matches(['\'', '"'])
}

/// An ODF length (`2cm`, `11pt`, `0.5in`, …) in points.
fn length(value: &str) -> Option<f32> {
    let value = value.trim();
    let split = value
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split);
    let n: f32 = number.trim().parse().ok()?;
    let points = match unit.trim() {
        "pt" | "" => n,
        "cm" => n * 72.0 / 2.54,
        "mm" => n * 72.0 / 25.4,
        "in" | "inch" => n * 72.0,
        "pc" => n * 12.0,
        "px" => n * 0.75,
        _ => return None,
    };
    points.is_finite().then_some(points)
}

fn percent(value: &str) -> Option<f32> {
    let n: f32 = value.trim().strip_suffix('%')?.trim().parse().ok()?;
    n.is_finite().then_some(n / 100.0)
}

fn rgb(value: &str) -> Option<[u8; 3]> {
    color(value.trim().strip_prefix('#')?)
}

fn apply_text(format: &mut TextFormat, props: &Node, fonts: &BTreeMap<String, String>) {
    let font = props
        .attr("font-name")
        .map(|name| fonts.get(name).map_or(name, String::as_str))
        .or_else(|| props.attr("font-family"))
        .map(unquote)
        .filter(|font| !font.is_empty());
    if let Some(font) = font {
        format.font = font.into();
    }
    if let Some(size) = props.attr("font-size") {
        if let Some(scale) = percent(size) {
            format.size *= scale;
        } else if let Some(points) = length(size) {
            format.size = points;
        }
    }
    if let Some(weight) = props.attr("font-weight") {
        format.bold = weight == "bold" || weight.parse::<u16>().is_ok_and(|w| w >= 600);
    }
    if let Some(style) = props.attr("font-style") {
        format.italic = matches!(style, "italic" | "oblique");
    }
    if let Some(style) = props.attr("text-underline-style") {
        format.underline = style != "none";
    }
    if let Some(style) = props.attr("text-line-through-style") {
        format.strike = style != "none";
    }
    if let Some(c) = props.attr("color").and_then(rgb) {
        format.color = c;
    }
    if let Some(background) = props.attr("background-color") {
        format.highlight = rgb(background);
    }
    if let Some(position) = props.attr("text-position") {
        let first = position.split_whitespace().next().unwrap_or_default();
        format.script = match first {
            "super" => 1,
            "sub" => -1,
            other => match percent(other) {
                Some(p) if p > 0.0 => 1,
                Some(p) if p < 0.0 => -1,
                _ => 0,
            },
        };
    }
}

fn apply_paragraph(p: &mut Paragraph, props: &Node) {
    if let Some(align) = props.attr("text-align") {
        p.align = match align {
            "center" => Align::Center,
            "end" | "right" => Align::Right,
            "justify" => Align::Justify,
            _ => Align::Left,
        };
    }
    let points = |name: &str| props.attr(name).and_then(length);
    if let Some(v) = points("margin-left") {
        p.indent = v.clamp(-10000.0, 10000.0);
    }
    if let Some(v) = points("text-indent") {
        p.first_indent = v.clamp(-10000.0, 10000.0);
    }
    if let Some(v) = points("margin-top") {
        p.spacing_before = v.clamp(0.0, 1000.0);
    }
    if let Some(v) = points("margin-bottom") {
        p.spacing_after = v.clamp(0.0, 1000.0);
    }
    if let Some(height) = props.attr("line-height") {
        if height == "normal" {
            p.line_spacing = 1.0;
            p.line_points = 0.0;
        } else if let Some(scale) = percent(height) {
            p.line_spacing = scale.clamp(0.5, 5.0);
            p.line_points = 0.0;
        } else if let Some(v) = length(height) {
            p.line_points = v.clamp(0.0, 1584.0);
            p.line_exact = true;
        }
    }
    if let Some(v) = points("line-height-at-least") {
        p.line_points = v.clamp(0.0, 1584.0);
        p.line_exact = false;
    }
    if props.attr("break-before") == Some("page") {
        p.page_break_before = true;
    }
    if let Some(keep) = props.attr("keep-with-next") {
        p.keep_next = keep == "always";
    }
    if let Some(keep) = props.attr("keep-together") {
        p.keep_lines = keep == "always";
    }
    if let Some(widows) = props.attr("widows") {
        p.widow_control = widows.trim() != "0";
    }
    if let Some(contextual) = props.attr("contextual-spacing") {
        p.contextual_spacing = contextual == "true";
    }
}

/// Our paragraph style for an ODF style name (`Heading_20_1`, `Title`, `Quotations`).
fn known_style(style: &Node) -> Option<String> {
    let name = style
        .attr("display-name")
        .map(str::to_string)
        .or_else(|| style.attr("name").map(|n| n.replace("_20_", " ")))?;
    let lower = name.to_lowercase();
    if let Some(level) = lower.strip_prefix("heading ")
        && let Ok(level) = level.trim().parse::<u8>()
        && (1..=6).contains(&level)
    {
        return Some(format!("Heading{level}"));
    }
    match lower.as_str() {
        "title" => Some("Title".into()),
        "quotations" | "quote" => Some("Quote".into()),
        _ => None,
    }
}

enum Target {
    Body,
    Cell(Vec<Paragraph>),
}

#[derive(Clone)]
struct ListContext {
    style: Option<String>,
    level: u8,
    numbered: bool,
}

struct Reader<'s, 'a> {
    styles: &'s Styles<'a>,
    parts: &'s BTreeMap<String, Vec<u8>>,
    doc: Doc,
    losses: BTreeSet<&'static str>,
    pending_break: bool,
}

struct Inline<'n> {
    runs: Vec<Run>,
    space: bool,
    frames: Vec<&'n Node>,
    losses: Vec<&'static str>,
}

impl Reader<'_, '_> {
    fn blocks(
        &mut self,
        nodes: &[Node],
        target: &mut Target,
        list: Option<&ListContext>,
    ) -> Result<(), String> {
        for node in nodes {
            match node.name.as_str() {
                "p" | "h" => self.paragraph_block(node, target, list)?,
                "list" => {
                    let style = node
                        .attr("style-name")
                        .map(str::to_string)
                        .or_else(|| list.and_then(|l| l.style.clone()));
                    let level = list.map_or(0, |l| (l.level + 1).min(8));
                    let inner = ListContext {
                        style,
                        level,
                        numbered: true,
                    };
                    for item in &node.children {
                        match item.name.as_str() {
                            "list-item" => self.blocks(&item.children, target, Some(&inner))?,
                            "list-header" => {
                                let header = ListContext {
                                    numbered: false,
                                    ..inner.clone()
                                };
                                self.blocks(&item.children, target, Some(&header))?
                            }
                            _ => {}
                        }
                    }
                }
                "table" => match target {
                    Target::Body => self.table(node)?,
                    Target::Cell(paragraphs) => {
                        self.losses
                            .insert("Вложенные таблицы ODT преобразованы в текст.");
                        let text = node.text_content();
                        let id = self.doc.alloc_id();
                        paragraphs.push(plain_paragraph(id, text.trim()));
                    }
                },
                "section" => {
                    if self
                        .styles
                        .props("section", node.attr("style-name"), "section-properties")
                        .and_then(|p| p.child("columns"))
                        .is_some_and(|c| c.attr("column-count").is_some_and(|n| n != "1"))
                    {
                        self.losses
                            .insert("Многоколоночные разделы ODT приведены к одной колонке.");
                    }
                    self.blocks(&node.children, target, list)?;
                }
                "table-of-content" | "alphabetical-index" | "bibliography"
                | "illustration-index" | "table-index" | "object-index" | "user-index" => {
                    self.losses
                        .insert("Оглавления и указатели ODT импортированы как обычный текст.");
                    if let Some(body) = node.child("index-body") {
                        self.blocks(&body.children, target, list)?;
                    }
                }
                "index-body" | "index-title" => self.blocks(&node.children, target, list)?,
                "tracked-changes" => {
                    self.losses
                        .insert("История исправлений ODT: импортирован текущий текст.");
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn paragraph_block(
        &mut self,
        node: &Node,
        target: &mut Target,
        list: Option<&ListContext>,
    ) -> Result<(), String> {
        let style_name = node.attr("style-name");
        let chain = self.styles.chain("paragraph", style_name);
        let id = self.doc.alloc_id();
        let mut p = Paragraph {
            id,
            ..Paragraph::default()
        };
        let style = if node.name == "h" {
            let level = node
                .attr("outline-level")
                .and_then(|l| l.parse::<u8>().ok())
                .unwrap_or(1)
                .clamp(1, 6);
            Some(format!("Heading{level}"))
        } else {
            chain.iter().rev().find_map(|s| known_style(s))
        };
        if let Some(style) = &style {
            p.set_style(style);
        }
        let mut base = p.format_at(0);
        for style in &chain {
            if let Some(props) = style.child("text-properties") {
                apply_text(&mut base, props, &self.styles.fonts);
            }
            if let Some(props) = style.child("paragraph-properties") {
                apply_paragraph(&mut p, props);
            }
        }
        let image_block = chain
            .iter()
            .any(|s| s.attr("name") == Some(IMAGE_BLOCK) && s.name == "style");
        if std::mem::take(&mut self.pending_break) {
            p.page_break_before = true;
        }
        if chain
            .iter()
            .filter_map(|s| s.child("paragraph-properties"))
            .any(|props| props.attr("break-after") == Some("page"))
        {
            self.pending_break = true;
        }
        if let Some(list) = list {
            p.list = if list.numbered {
                self.styles.list_kind(list.style.as_deref(), list.level)
            } else {
                List::None
            };
            p.list_level = list.level;
        }
        let mut inline = Inline {
            runs: Vec::new(),
            space: true,
            frames: Vec::new(),
            losses: Vec::new(),
        };
        self.inline(node, &base, &mut inline);
        self.losses.extend(inline.losses);
        p.runs = inline.runs;
        if p.runs.is_empty() {
            p.runs.push(Run {
                text: String::new(),
                format: base,
            });
        }
        p.normalize();
        match target {
            Target::Cell(paragraphs) => {
                if !inline.frames.is_empty() {
                    self.losses
                        .insert("Рисунки и надписи внутри ячеек ODT не импортированы.");
                }
                paragraphs.push(p);
            }
            Target::Body => {
                let anchor = if image_block && p.is_empty() && !self.doc.paragraphs.is_empty() {
                    self.doc.paragraphs.last().map_or(p.id, |last| last.id)
                } else {
                    let id = p.id;
                    self.doc.paragraphs.push(p);
                    id
                };
                for frame in inline.frames {
                    self.frame(frame, anchor, target)?;
                }
            }
        }
        Ok(())
    }

    fn inline<'n>(&self, node: &'n Node, format: &TextFormat, out: &mut Inline<'n>) {
        for child in &node.children {
            match child.name.as_str() {
                "#text" => push_text(out, &child.text, format),
                "s" => {
                    let count = child
                        .attr("c")
                        .and_then(|c| c.parse::<usize>().ok())
                        .unwrap_or(1)
                        .clamp(1, 1000);
                    out.runs.push(Run {
                        text: " ".repeat(count),
                        format: format.clone(),
                    });
                    out.space = true;
                }
                "tab" => {
                    out.runs.push(Run {
                        text: "\t".into(),
                        format: format.clone(),
                    });
                    out.space = false;
                }
                "line-break" => {
                    out.runs.push(Run {
                        text: "\u{000b}".into(),
                        format: format.clone(),
                    });
                    out.space = false;
                }
                "span" => {
                    let inner = self.styles.text_format(child.attr("style-name"), format);
                    self.inline(child, &inner, out);
                }
                "a" => {
                    let mut inner = self.styles.text_format(child.attr("style-name"), format);
                    if let Some(href) = child.attr("href") {
                        inner.link = Some(href.to_string());
                    }
                    self.inline(child, &inner, out);
                }
                "frame" => out.frames.push(child),
                "note" => out.losses.push("Сноски ODT не импортированы."),
                "annotation" => out.losses.push("Комментарии ODT не импортированы."),
                "annotation-end"
                | "ruby-text"
                | "bookmark"
                | "bookmark-start"
                | "bookmark-end"
                | "soft-page-break"
                | "change"
                | "change-start"
                | "change-end"
                | "reference-mark-start"
                | "reference-mark-end"
                | "toc-mark"
                | "alphabetical-index-mark" => {}
                _ => self.inline(child, format, out),
            }
        }
    }

    fn frame(&mut self, frame: &Node, anchor: u64, target: &mut Target) -> Result<(), String> {
        if let Some(text_box) = frame.child("text-box") {
            self.losses
                .insert("Надписи ODT преобразованы в обычные абзацы.");
            return self.blocks(&text_box.children, target, None);
        }
        let Some(image) = frame.child("image") else {
            self.losses
                .insert("Фигуры и объекты ODT, кроме рисунков, не импортированы.");
            return Ok(());
        };
        let bytes = if let Some(data) = image.child("binary-data") {
            base64_decode(&data.text_content())
        } else {
            image
                .attr("href")
                .map(|href| href.trim_start_matches("./"))
                .and_then(|href| self.parts.get(href))
                .cloned()
        };
        let Some(bytes) = bytes.filter(|b| {
            matches!(
                image::guess_format(b),
                Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg)
            )
        }) else {
            self.losses
                .insert("Рисунки ODT не в формате PNG/JPEG или без файла пропущены.");
            return Ok(());
        };
        let width = frame
            .attr("width")
            .and_then(length)
            .filter(|w| *w > 0.0)
            .unwrap_or(150.0)
            .min(10000.0);
        let height = frame
            .attr("height")
            .and_then(length)
            .filter(|h| *h > 0.0)
            .unwrap_or(112.5)
            .min(10000.0);
        let alt = frame
            .child("desc")
            .or_else(|| frame.child("title"))
            .map(Node::text_content)
            .unwrap_or_default();
        let id = self.doc.alloc_id();
        self.doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: anchor,
            bytes,
            width,
            height,
            alt,
        });
        Ok(())
    }

    fn table(&mut self, node: &Node) -> Result<(), String> {
        if self.doc.paragraphs.is_empty() {
            let id = self.doc.alloc_id();
            self.doc.paragraphs.push(Paragraph {
                id,
                ..Paragraph::default()
            });
        }
        let after = self.doc.paragraphs.last().map_or(0, |p| p.id);
        let table_name = node.attr("style-name");
        let mut columns = Vec::new();
        self.columns(node, &mut columns);
        let mut rows_xml = Vec::new();
        collect_rows(node, false, &mut rows_xml);
        if rows_xml.len() > 10000 {
            return Err("Таблица ODT превышает 10 000 строк".into());
        }
        let header_rows = rows_xml.iter().take_while(|(_, header)| *header).count();
        let mut rows = Vec::new();
        let mut row_heights = Vec::new();
        let mut vertical: Vec<(usize, u16)> = Vec::new();
        let mut borders = Vec::new();
        let mut padding = None;
        for (tr, _) in &rows_xml {
            let mut cells = Vec::new();
            let mut column = 0usize;
            let mut skip = 0usize;
            for tc in &tr.children {
                match tc.name.as_str() {
                    "table-cell" => {
                        if cells.len() >= 256 {
                            return Err("Таблица ODT превышает 256 столбцов".into());
                        }
                        let span = count(tc.attr("number-columns-spanned"), 256);
                        let rows_span = count(tc.attr("number-rows-spanned"), 10000);
                        let mut target = Target::Cell(Vec::new());
                        self.blocks(&tc.children, &mut target, None)?;
                        let mut paragraphs = match target {
                            Target::Cell(p) => p,
                            Target::Body => Vec::new(),
                        };
                        if paragraphs.is_empty() {
                            let id = self.doc.alloc_id();
                            paragraphs.push(Paragraph {
                                id,
                                ..Paragraph::default()
                            });
                        }
                        let props = self.styles.props(
                            "table-cell",
                            tc.attr("style-name"),
                            "table-cell-properties",
                        );
                        let fill = props
                            .and_then(|p| p.attr("background-color"))
                            .and_then(rgb)
                            .unwrap_or([255, 255, 255]);
                        let id = self.doc.alloc_id();
                        let mut cell = Cell::new(id, paragraphs, fill);
                        cell.span = u16::try_from(span).unwrap_or(256);
                        cell.v_align = match props.and_then(|p| p.attr("vertical-align")) {
                            Some("middle") => VAlign::Center,
                            Some("bottom") => VAlign::Bottom,
                            _ => VAlign::Top,
                        };
                        if let Some(props) = props {
                            for side in ["border", "border-top", "border-left"] {
                                if let Some(border) = props.attr(side) {
                                    borders.push(border.to_string());
                                }
                            }
                            if padding.is_none() {
                                padding = cell_padding(props);
                            }
                        }
                        if rows_span > 1 {
                            if vertical.len() <= column {
                                vertical.resize(column + 1, (0, 1));
                            }
                            if let Some(slot) = vertical.get_mut(column) {
                                *slot = (rows_span - 1, cell.span);
                            }
                        }
                        cells.push(cell);
                        // The covered cells that follow advance the column for the rest of the span.
                        column += 1;
                        skip = span - 1;
                    }
                    "covered-table-cell" => {
                        if skip > 0 {
                            skip -= 1;
                            column += 1;
                            continue;
                        }
                        match vertical.get_mut(column) {
                            Some((remaining, span)) if *remaining > 0 => {
                                *remaining -= 1;
                                let span = *span;
                                let id = self.doc.alloc_id();
                                let paragraph_id = self.doc.alloc_id();
                                let mut cell = Cell::new(
                                    id,
                                    vec![Paragraph {
                                        id: paragraph_id,
                                        ..Paragraph::default()
                                    }],
                                    [255, 255, 255],
                                );
                                cell.span = span;
                                cell.merged = true;
                                cells.push(cell);
                                skip = usize::from(span).saturating_sub(1);
                            }
                            _ => {}
                        }
                        column += 1;
                    }
                    _ => {}
                }
            }
            if cells.is_empty() {
                continue;
            }
            row_heights.push(
                self.styles
                    .props("table-row", tr.attr("style-name"), "table-row-properties")
                    .and_then(|p| p.attr("min-row-height").or_else(|| p.attr("row-height")))
                    .and_then(length)
                    .map_or(0.0, |h| h.clamp(0.0, 1500.0)),
            );
            rows.push(cells);
        }
        if rows.is_empty() {
            self.losses.insert("Пустые таблицы ODT пропущены.");
            return Ok(());
        }
        if row_heights.iter().all(|h| *h <= 0.0) {
            row_heights.clear();
        }
        let grid = rows
            .iter()
            .map(|row| row.iter().map(Cell::span).sum::<usize>())
            .max()
            .unwrap_or_default();
        let column_widths = if columns.len() == grid && columns.iter().all(Option::is_some) {
            columns
                .into_iter()
                .flatten()
                .map(|w| w.clamp(1.0, 2000.0))
                .collect()
        } else {
            Vec::new()
        };
        let visible = borders.iter().any(|b| b.trim() != "none");
        let border_color = borders
            .iter()
            .filter(|b| b.trim() != "none")
            .find_map(|b| b.split_whitespace().find_map(rgb))
            .unwrap_or([0, 0, 0]);
        if self
            .styles
            .props("table", table_name, "table-properties")
            .and_then(|p| p.attr("align"))
            .is_some_and(|a| a == "center" || a == "right")
        {
            self.losses
                .insert("Выравнивание таблиц ODT по центру/справа не сохраняется.");
        }
        let id = self.doc.alloc_id();
        self.doc.tables.push(Table {
            id,
            after_paragraph_id: after,
            rows,
            column_widths,
            border_color,
            borderless: !visible,
            cell_margin: padding.unwrap_or(WORD_CELL_MARGIN),
            row_heights,
            header_rows,
        });
        Ok(())
    }

    fn columns(&self, node: &Node, out: &mut Vec<Option<f32>>) {
        for child in &node.children {
            match child.name.as_str() {
                "table-column" => {
                    let width = self
                        .styles
                        .props(
                            "table-column",
                            child.attr("style-name"),
                            "table-column-properties",
                        )
                        .and_then(|p| p.attr("column-width"))
                        .and_then(length);
                    let repeat = count(child.attr("number-columns-repeated"), 256);
                    for _ in 0..repeat {
                        if out.len() < 256 {
                            out.push(width);
                        }
                    }
                }
                "table-columns" | "table-header-columns" | "table-column-group" => {
                    self.columns(child, out)
                }
                _ => {}
            }
        }
    }

    fn page_setup(&mut self) {
        let master = self
            .styles
            .masters
            .iter()
            .find(|m| m.attr("name") == Some("Standard"))
            .or_else(|| self.styles.masters.first())
            .copied();
        let Some(master) = master else { return };
        if let Some(props) = master
            .attr("page-layout-name")
            .and_then(|name| self.styles.layouts.get(name))
            .and_then(|layout| layout.child("page-layout-properties"))
        {
            let page = &mut self.doc.page;
            let get = |name: &str| props.attr(name).and_then(length);
            if let Some(v) = get("page-width") {
                page.width = v;
            }
            if let Some(v) = get("page-height") {
                page.height = v;
            }
            if let Some(v) = get("margin-left") {
                page.margin_left = v;
            }
            if let Some(v) = get("margin-right") {
                page.margin_right = v;
            }
            if let Some(v) = get("margin-top") {
                page.margin_top = v;
            }
            if let Some(v) = get("margin-bottom") {
                page.margin_bottom = v;
            }
            if props
                .attr("column-count")
                .or_else(|| props.child("columns").and_then(|c| c.attr("column-count")))
                .is_some_and(|n| n != "1")
            {
                self.losses
                    .insert("Многоколоночная вёрстка ODT преобразована в одну колонку.");
            }
        }
        for (name, header) in [("header", true), ("footer", false)] {
            let Some(part) = master.child(name) else {
                continue;
            };
            let mut text = String::new();
            let mut has_page = false;
            let mut first = true;
            for p in part
                .children
                .iter()
                .filter(|n| n.name == "p" || n.name == "h")
            {
                if !first {
                    text.push(' ');
                }
                first = false;
                let mut space = true;
                header_text(p, &mut text, &mut space, &mut has_page);
            }
            if header {
                self.doc.page.header = text;
            } else {
                self.doc.page.footer = text;
            }
            self.doc.page.page_numbers |= has_page;
        }
    }
}

fn collect_rows<'n>(node: &'n Node, header: bool, out: &mut Vec<(&'n Node, bool)>) {
    for child in &node.children {
        match child.name.as_str() {
            "table-row" => {
                let repeat = count(child.attr("number-rows-repeated"), 10000);
                for _ in 0..repeat.min(10001 - out.len().min(10001)) {
                    out.push((child, header));
                }
            }
            "table-header-rows" => collect_rows(child, true, out),
            "table-rows" | "table-row-group" => collect_rows(child, header, out),
            _ => {}
        }
    }
}

fn count(value: Option<&str>, max: usize) -> usize {
    value
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, max)
}

fn cell_padding(props: &Node) -> Option<[f32; 4]> {
    let all = props.attr("padding").and_then(length);
    let side = |name: &str| props.attr(name).and_then(length).or(all);
    let sides = [
        side("padding-top"),
        side("padding-right"),
        side("padding-bottom"),
        side("padding-left"),
    ];
    if sides.iter().all(Option::is_none) {
        return None;
    }
    Some(sides.map(|v| v.unwrap_or(0.0).clamp(0.0, 72.0)))
}

fn plain_paragraph(id: u64, text: &str) -> Paragraph {
    Paragraph {
        id,
        runs: vec![Run {
            text: text.into(),
            format: TextFormat::default(),
        }],
        ..Paragraph::default()
    }
}

/// ODF collapses runs of white space into one space; `text:s` and `text:tab` keep them.
fn push_text(out: &mut Inline, text: &str, format: &TextFormat) {
    let mut collapsed = String::new();
    for ch in text.chars() {
        let ch = if matches!(ch, '\n' | '\r' | '\t') {
            ' '
        } else {
            ch
        };
        if ch == ' ' {
            if out.space {
                continue;
            }
            out.space = true;
        } else {
            out.space = false;
        }
        collapsed.push(ch);
    }
    if !collapsed.is_empty() {
        out.runs.push(Run {
            text: collapsed,
            format: format.clone(),
        });
    }
}

fn header_text(node: &Node, out: &mut String, space: &mut bool, has_page: &mut bool) {
    for child in &node.children {
        match child.name.as_str() {
            "#text" => {
                for ch in child.text.chars() {
                    let ch = if matches!(ch, '\n' | '\r' | '\t') {
                        ' '
                    } else {
                        ch
                    };
                    if ch == ' ' {
                        if *space {
                            continue;
                        }
                        *space = true;
                    } else {
                        *space = false;
                    }
                    out.push(ch);
                }
            }
            "s" => {
                out.push_str(&" ".repeat(count(child.attr("c"), 1000)));
                *space = true;
            }
            "tab" => {
                out.push('\t');
                *space = false;
            }
            "page-number" => *has_page = true,
            _ => header_text(child, out, space, has_page),
        }
    }
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            break;
        }
        let value = BASE64.iter().position(|c| *c == byte)?;
        buffer = (buffer << 6) | u32::try_from(value).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).ok()?);
        }
        if out.len() > 32 * 1024 * 1024 {
            return None;
        }
    }
    Some(out)
}

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                let index = usize::try_from((n >> shift) & 63).unwrap_or(0);
                out.push(char::from(BASE64.get(index).copied().unwrap_or(b'A')));
            } else {
                out.push('=');
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------------------------

pub(super) fn write_odt(doc: &Doc, template: bool) -> Result<Vec<u8>, String> {
    let mut writer = Writer::new(doc, false);
    let body = writer.body();
    let content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content {NAMESPACES}>{}<office:automatic-styles>{}</office:automatic-styles><office:body><office:text>{body}</office:text></office:body></office:document-content>",
        writer.font_decls(),
        writer.automatic_styles()
    );
    let styles = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-styles {NAMESPACES}>{}<office:styles>{}</office:styles><office:automatic-styles>{}</office:automatic-styles><office:master-styles>{}</office:master-styles></office:document-styles>",
        writer.font_decls(),
        named_styles(),
        page_layout(&doc.page),
        writer.master_page()
    );
    let meta = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-meta {NAMESPACES}>{}</office:document-meta>",
        meta_xml(doc)
    );
    let mime = if template { MIME_TEMPLATE } else { MIME_TEXT };
    let mut manifest = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\"><manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.3\" manifest:media-type=\"{mime}\"/>"
    );
    for part in ["content.xml", "styles.xml", "meta.xml"] {
        manifest.push_str(&format!(
            "<manifest:file-entry manifest:full-path=\"{part}\" manifest:media-type=\"text/xml\"/>"
        ));
    }
    for (path, media, _) in &writer.pictures {
        manifest.push_str(&format!(
            "<manifest:file-entry manifest:full-path=\"{path}\" manifest:media-type=\"{media}\"/>"
        ));
    }
    manifest.push_str("</manifest:manifest>");
    let mut parts = vec![
        ("mimetype".to_string(), mime.as_bytes().to_vec()),
        ("META-INF/manifest.xml".into(), manifest.into_bytes()),
        ("content.xml".into(), content.into_bytes()),
        ("styles.xml".into(), styles.into_bytes()),
        ("meta.xml".into(), meta.into_bytes()),
    ];
    parts.extend(
        std::mem::take(&mut writer.pictures)
            .into_iter()
            .map(|(path, _, bytes)| (path, bytes.to_vec())),
    );
    write_zip(parts)
}

pub(super) fn write_fodt(doc: &Doc) -> Result<Vec<u8>, String> {
    let mut writer = Writer::new(doc, true);
    let body = writer.body();
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document {NAMESPACES} office:mimetype=\"{MIME_TEXT}\">{}{}<office:styles>{}</office:styles><office:automatic-styles>{}{}</office:automatic-styles><office:master-styles>{}</office:master-styles><office:body><office:text>{body}</office:text></office:body></office:document>",
        meta_xml(doc),
        writer.font_decls(),
        named_styles(),
        page_layout(&doc.page),
        writer.automatic_styles(),
        writer.master_page()
    );
    if let Some(ch) = xml.chars().find(|ch| {
        !matches!(*ch, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
    }) {
        return Err(format!(
            "Недопустимый символ U+{:04X} в FODT; файл не изменён",
            ch as u32
        ));
    }
    Ok(xml.into_bytes())
}

fn meta_xml(doc: &Doc) -> String {
    let mut out = String::from("<office:meta><meta:generator>TrineDocs</meta:generator>");
    if !doc.title.is_empty() {
        out.push_str(&format!("<dc:title>{}</dc:title>", esc(&doc.title)));
    }
    if !doc.author.is_empty() {
        out.push_str(&format!(
            "<meta:initial-creator>{0}</meta:initial-creator><dc:creator>{0}</dc:creator>",
            esc(&doc.author)
        ));
    }
    out.push_str("</office:meta>");
    out
}

fn named_styles() -> String {
    let base = TextFormat::default();
    let mut out = format!(
        "<style:default-style style:family=\"paragraph\"><style:paragraph-properties fo:margin-top=\"0pt\" fo:margin-bottom=\"6pt\" fo:line-height=\"115%\" fo:widows=\"2\" fo:orphans=\"2\"/><style:text-properties style:font-name=\"{0}\" fo:font-size=\"{1}pt\" style:font-size-asian=\"{1}pt\" style:font-size-complex=\"{1}pt\" fo:color=\"#{2}\" fo:language=\"ru\" fo:country=\"RU\"/></style:default-style><style:style style:name=\"Standard\" style:family=\"paragraph\" style:class=\"text\"/><style:style style:name=\"Heading\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"text\"><style:paragraph-properties fo:keep-with-next=\"always\"/></style:style>",
        esc(&base.font),
        base.size,
        hex(base.color)
    );
    for level in 1..=6u8 {
        let mut p = Paragraph::default();
        p.set_style(&format!("Heading{level}"));
        let size = p.format_at(0).size;
        out.push_str(&format!(
            "<style:style style:name=\"Heading_20_{level}\" style:display-name=\"Heading {level}\" style:family=\"paragraph\" style:parent-style-name=\"Heading\" style:default-outline-level=\"{level}\" style:class=\"text\"><style:paragraph-properties fo:margin-top=\"12pt\"/><style:text-properties fo:font-size=\"{size}pt\" fo:font-weight=\"bold\"/></style:style>"
        ));
    }
    out.push_str("<style:style style:name=\"Title\" style:family=\"paragraph\" style:parent-style-name=\"Heading\" style:class=\"chapter\"><style:text-properties fo:font-size=\"28pt\" fo:font-weight=\"bold\"/></style:style><style:style style:name=\"Quotations\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"html\"><style:text-properties fo:font-style=\"italic\"/></style:style>");
    out.push_str(&format!(
        "<style:style style:name=\"{IMAGE_BLOCK}\" style:display-name=\"TrineDocs Image\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"/>"
    ));
    out
}

fn page_layout(page: &PageSetup) -> String {
    let orientation = if page.width > page.height {
        "landscape"
    } else {
        "portrait"
    };
    format!(
        "<style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"{}pt\" fo:page-height=\"{}pt\" style:print-orientation=\"{orientation}\" fo:margin-top=\"{}pt\" fo:margin-bottom=\"{}pt\" fo:margin-left=\"{}pt\" fo:margin-right=\"{}pt\"/></style:page-layout>",
        page.width,
        page.height,
        page.margin_top,
        page.margin_bottom,
        page.margin_left,
        page.margin_right
    )
}

struct Writer<'d> {
    doc: &'d Doc,
    inline_pictures: bool,
    text_styles: Vec<String>,
    paragraph_styles: Vec<(String, String)>,
    table_styles: String,
    fonts: BTreeSet<String>,
    pictures: Vec<(String, &'static str, &'d [u8])>,
    tables: usize,
}

/// Open `text:list` elements, innermost last; `true` when a `text:list-item` is open in it.
struct ListState {
    open: Vec<bool>,
    kind: List,
}

impl ListState {
    fn new() -> Self {
        Self {
            open: Vec::new(),
            kind: List::None,
        }
    }

    fn close_to(&mut self, depth: usize, out: &mut String) {
        while self.open.len() > depth {
            if self.open.pop() == Some(true) {
                out.push_str("</text:list-item>");
            }
            out.push_str("</text:list>");
        }
        if self.open.is_empty() {
            self.kind = List::None;
        }
    }

    fn enter(&mut self, p: &Paragraph, out: &mut String) {
        if p.list == List::None {
            self.close_to(0, out);
            return;
        }
        if p.list != self.kind {
            self.close_to(0, out);
        }
        let depth = usize::from(p.list_level.min(8)) + 1;
        self.close_to(depth, out);
        self.kind = p.list;
        let style = if p.list == List::Number { "L2" } else { "L1" };
        while self.open.len() < depth {
            if self.open.is_empty() {
                out.push_str(&format!("<text:list text:style-name=\"{style}\">"));
            } else {
                if let Some(item) = self.open.last_mut()
                    && !*item
                {
                    out.push_str("<text:list-item>");
                    *item = true;
                }
                out.push_str("<text:list>");
            }
            self.open.push(false);
        }
        if let Some(item) = self.open.last_mut() {
            if *item {
                out.push_str("</text:list-item>");
            }
            out.push_str("<text:list-item>");
            *item = true;
        }
    }
}

impl<'d> Writer<'d> {
    fn new(doc: &'d Doc, inline_pictures: bool) -> Self {
        let mut fonts = BTreeSet::new();
        fonts.insert(TextFormat::default().font);
        Self {
            doc,
            inline_pictures,
            text_styles: Vec::new(),
            paragraph_styles: Vec::new(),
            table_styles: String::new(),
            fonts,
            pictures: Vec::new(),
            tables: 0,
        }
    }

    fn font_decls(&self) -> String {
        let mut out = String::from("<office:font-face-decls>");
        for font in &self.fonts {
            out.push_str(&format!(
                "<style:font-face style:name=\"{0}\" svg:font-family=\"{0}\"/>",
                esc(font)
            ));
        }
        out.push_str("</office:font-face-decls>");
        out
    }

    fn automatic_styles(&self) -> String {
        let mut out = String::new();
        for (i, props) in self.text_styles.iter().enumerate() {
            out.push_str(&format!(
                "<style:style style:name=\"T{}\" style:family=\"text\">{props}</style:style>",
                i + 1
            ));
        }
        for (i, (parent, props)) in self.paragraph_styles.iter().enumerate() {
            out.push_str(&format!(
                "<style:style style:name=\"P{}\" style:family=\"paragraph\" style:parent-style-name=\"{parent}\">{props}</style:style>",
                i + 1
            ));
        }
        out.push_str(&self.table_styles);
        for (name, numbered) in [("L1", false), ("L2", true)] {
            out.push_str(&format!("<text:list-style style:name=\"{name}\">"));
            for level in 1..=10u32 {
                let indent = 18 * level;
                let position = format!(
                    "<style:list-level-properties text:list-level-position-and-space-mode=\"label-alignment\"><style:list-level-label-alignment text:label-followed-by=\"listtab\" text:list-tab-stop-position=\"{indent}pt\" fo:text-indent=\"-18pt\" fo:margin-left=\"{indent}pt\"/></style:list-level-properties>"
                );
                if numbered {
                    out.push_str(&format!(
                        "<text:list-level-style-number text:level=\"{level}\" style:num-suffix=\".\" style:num-format=\"1\">{position}</text:list-level-style-number>"
                    ));
                } else {
                    let bullet = ["•", "◦", "▪"]
                        .get(usize::try_from((level - 1) % 3).unwrap_or(0))
                        .copied()
                        .unwrap_or("•");
                    out.push_str(&format!(
                        "<text:list-level-style-bullet text:level=\"{level}\" text:bullet-char=\"{bullet}\">{position}</text:list-level-style-bullet>"
                    ));
                }
            }
            out.push_str("</text:list-style>");
        }
        out
    }

    fn master_page(&self) -> String {
        let page = &self.doc.page;
        let mut out = String::from(
            "<style:master-page style:name=\"Standard\" style:page-layout-name=\"pm1\">",
        );
        if !page.header.is_empty() {
            out.push_str("<style:header><text:p text:style-name=\"Standard\">");
            let mut space = true;
            odf_text(&page.header, &mut out, &mut space);
            out.push_str("</text:p></style:header>");
        }
        if !page.footer.is_empty() || page.page_numbers {
            out.push_str("<style:footer><text:p text:style-name=\"Standard\">");
            let mut space = true;
            odf_text(&page.footer, &mut out, &mut space);
            if page.page_numbers {
                out.push_str("<text:page-number text:select-page=\"current\">1</text:page-number>");
            }
            out.push_str("</text:p></style:footer>");
        }
        out.push_str("</style:master-page>");
        out
    }

    fn text_style(&mut self, format: &TextFormat) -> String {
        self.fonts.insert(format.font.clone());
        let props = text_props(format);
        let index = match self.text_styles.iter().position(|s| *s == props) {
            Some(i) => i,
            None => {
                self.text_styles.push(props);
                self.text_styles.len() - 1
            }
        };
        format!("T{}", index + 1)
    }

    fn paragraph_style(&mut self, p: &Paragraph, page_break: bool) -> String {
        let parent = match p.style.as_str() {
            "Title" => "Title".to_string(),
            "Quote" => "Quotations".to_string(),
            style => heading_level(style)
                .map_or_else(|| "Standard".to_string(), |l| format!("Heading_20_{l}")),
        };
        let first = p.format_at(0);
        self.fonts.insert(first.font.clone());
        let props = format!("{}{}", paragraph_props(p, page_break), text_props(&first));
        let entry = (parent, props);
        let index = match self.paragraph_styles.iter().position(|s| *s == entry) {
            Some(i) => i,
            None => {
                self.paragraph_styles.push(entry);
                self.paragraph_styles.len() - 1
            }
        };
        format!("P{}", index + 1)
    }

    fn body(&mut self) -> String {
        let mut out = String::new();
        let mut lists = ListState::new();
        let doc = self.doc;
        for p in &doc.paragraphs {
            lists.enter(p, &mut out);
            self.paragraph(p, &mut out);
            let images: Vec<&AnchoredImage> = doc
                .images
                .iter()
                .filter(|i| i.after_paragraph_id == p.id)
                .collect();
            let tables: Vec<&Table> = doc
                .tables
                .iter()
                .filter(|t| t.after_paragraph_id == p.id)
                .collect();
            if images.is_empty() && tables.is_empty() {
                continue;
            }
            lists.close_to(0, &mut out);
            for image in images {
                self.picture(image, &mut out);
            }
            for table in tables {
                self.table(table, &mut out);
            }
        }
        lists.close_to(0, &mut out);
        out
    }

    fn paragraph(&mut self, p: &Paragraph, out: &mut String) {
        // A form feed inside a paragraph is a page break: the rest starts a new paragraph.
        let mut segments: Vec<Vec<Run>> = vec![Vec::new()];
        for run in &p.runs {
            for (i, piece) in run.text.split('\u{000c}').enumerate() {
                if i > 0 {
                    segments.push(Vec::new());
                }
                if let Some(segment) = segments.last_mut() {
                    segment.push(Run {
                        text: piece.into(),
                        format: run.format.clone(),
                    });
                }
            }
        }
        for (i, runs) in segments.iter().enumerate() {
            let style = self.paragraph_style(p, p.page_break_before || i > 0);
            let (open, close) = match heading_level(&p.style) {
                Some(level) => (
                    format!("<text:h text:style-name=\"{style}\" text:outline-level=\"{level}\">"),
                    "</text:h>",
                ),
                None => (format!("<text:p text:style-name=\"{style}\">"), "</text:p>"),
            };
            out.push_str(&open);
            let mut space = true;
            for run in runs.iter().filter(|r| !r.text.is_empty()) {
                let name = self.text_style(&run.format);
                let link = run
                    .format
                    .link
                    .as_deref()
                    .filter(|link| super::safe_html_link(link));
                if let Some(link) = link {
                    out.push_str(&format!(
                        "<text:a xlink:type=\"simple\" xlink:href=\"{}\">",
                        esc(link)
                    ));
                }
                out.push_str(&format!("<text:span text:style-name=\"{name}\">"));
                odf_text(&run.text, out, &mut space);
                out.push_str("</text:span>");
                if link.is_some() {
                    out.push_str("</text:a>");
                }
            }
            out.push_str(close);
        }
    }

    fn picture(&mut self, image: &'d AnchoredImage, out: &mut String) {
        let (extension, media) = match image::guess_format(&image.bytes) {
            Ok(image::ImageFormat::Jpeg) => ("jpg", "image/jpeg"),
            _ => ("png", "image/png"),
        };
        let source = if self.inline_pictures {
            format!(
                "<draw:image><office:binary-data>{}</office:binary-data></draw:image>",
                base64_encode(&image.bytes)
            )
        } else {
            let path = format!("Pictures/image{}.{extension}", image.id);
            let source = format!(
                "<draw:image xlink:href=\"{path}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/>"
            );
            self.pictures.push((path, media, &image.bytes));
            source
        };
        let alt = if image.alt.is_empty() {
            String::new()
        } else {
            format!("<svg:desc>{}</svg:desc>", esc(&image.alt))
        };
        out.push_str(&format!(
            "<text:p text:style-name=\"{IMAGE_BLOCK}\"><draw:frame draw:name=\"Image{}\" text:anchor-type=\"as-char\" svg:width=\"{}pt\" svg:height=\"{}pt\" draw:z-index=\"0\">{source}{alt}</draw:frame></text:p>",
            image.id, image.width, image.height
        ));
    }

    fn table(&mut self, table: &Table, out: &mut String) {
        self.tables += 1;
        let name = format!("Tbl{}", self.tables);
        let page = &self.doc.page;
        let body = (page.width - page.margin_left - page.margin_right).max(24.0);
        let columns = table.grid_columns().max(1);
        let widths: Vec<f32> = if table.column_widths.len() == columns {
            table.column_widths.clone()
        } else {
            vec![body / columns as f32; columns]
        };
        let border = if table.borderless {
            "none".to_string()
        } else {
            format!("0.5pt solid #{}", hex(table.border_color))
        };
        let [top, right, bottom, left] = table.cell_margin;
        self.table_styles.push_str(&format!(
            "<style:style style:name=\"{name}\" style:family=\"table\"><style:table-properties style:width=\"{}pt\" table:align=\"left\"/></style:style>",
            widths.iter().sum::<f32>()
        ));
        out.push_str(&format!(
            "<table:table table:name=\"Таблица{}\" table:style-name=\"{name}\">",
            self.tables
        ));
        for (i, width) in widths.iter().enumerate() {
            self.table_styles.push_str(&format!(
                "<style:style style:name=\"{name}.C{i}\" style:family=\"table-column\"><style:table-column-properties style:column-width=\"{width}pt\"/></style:style>"
            ));
            out.push_str(&format!(
                "<table:table-column table:style-name=\"{name}.C{i}\"/>"
            ));
        }
        let mut cell_styles: Vec<([u8; 3], VAlign)> = Vec::new();
        let header_rows = table.header_rows.min(table.rows.len());
        for (r, row) in table.rows.iter().enumerate() {
            if r == 0 && header_rows > 0 {
                out.push_str("<table:table-header-rows>");
            }
            let height = table.row_heights.get(r).copied().unwrap_or(0.0);
            if height > 0.0 {
                self.table_styles.push_str(&format!(
                    "<style:style style:name=\"{name}.R{r}\" style:family=\"table-row\"><style:table-row-properties style:min-row-height=\"{height}pt\"/></style:style>"
                ));
                out.push_str(&format!(
                    "<table:table-row table:style-name=\"{name}.R{r}\">"
                ));
            } else {
                out.push_str("<table:table-row>");
            }
            let starts = table.grid_starts(r);
            for (cell, start) in row.iter().zip(starts) {
                let covered = if cell.span() > 1 {
                    "<table:covered-table-cell/>".repeat(cell.span() - 1)
                } else {
                    String::new()
                };
                if cell.merged {
                    out.push_str("<table:covered-table-cell/>");
                    out.push_str(&covered);
                    continue;
                }
                let rows_spanned = 1 + table
                    .rows
                    .iter()
                    .enumerate()
                    .skip(r + 1)
                    .take_while(|(next, cells)| {
                        let next_starts = table.grid_starts(*next);
                        cells
                            .iter()
                            .zip(next_starts)
                            .any(|(c, s)| s == start && c.merged)
                    })
                    .count();
                let key = (cell.fill, cell.v_align);
                let index = match cell_styles.iter().position(|k| *k == key) {
                    Some(i) => i,
                    None => {
                        cell_styles.push(key);
                        let align = match cell.v_align {
                            VAlign::Top => "top",
                            VAlign::Center => "middle",
                            VAlign::Bottom => "bottom",
                        };
                        self.table_styles.push_str(&format!(
                            "<style:style style:name=\"{name}.X{}\" style:family=\"table-cell\"><style:table-cell-properties fo:background-color=\"#{}\" fo:padding-top=\"{top}pt\" fo:padding-right=\"{right}pt\" fo:padding-bottom=\"{bottom}pt\" fo:padding-left=\"{left}pt\" fo:border=\"{border}\" style:vertical-align=\"{align}\"/></style:style>",
                            cell_styles.len(),
                            hex(cell.fill)
                        ));
                        cell_styles.len() - 1
                    }
                };
                out.push_str(&format!(
                    "<table:table-cell table:style-name=\"{name}.X{}\" office:value-type=\"string\"",
                    index + 1
                ));
                if cell.span() > 1 {
                    out.push_str(&format!(
                        " table:number-columns-spanned=\"{}\"",
                        cell.span()
                    ));
                }
                if rows_spanned > 1 {
                    out.push_str(&format!(" table:number-rows-spanned=\"{rows_spanned}\""));
                }
                out.push('>');
                let mut lists = ListState::new();
                for p in &cell.paragraphs {
                    lists.enter(p, out);
                    self.paragraph(p, out);
                }
                lists.close_to(0, out);
                out.push_str("</table:table-cell>");
                out.push_str(&covered);
            }
            out.push_str("</table:table-row>");
            if header_rows > 0 && r + 1 == header_rows {
                out.push_str("</table:table-header-rows>");
            }
        }
        out.push_str("</table:table>");
    }
}

fn text_props(f: &TextFormat) -> String {
    let weight = if f.bold { "bold" } else { "normal" };
    let style = if f.italic { "italic" } else { "normal" };
    let mut out = format!(
        "<style:text-properties style:font-name=\"{}\" fo:font-size=\"{2}pt\" style:font-size-asian=\"{2}pt\" style:font-size-complex=\"{2}pt\" fo:font-weight=\"{3}\" style:font-weight-asian=\"{3}\" style:font-weight-complex=\"{3}\" fo:font-style=\"{4}\" style:font-style-asian=\"{4}\" style:font-style-complex=\"{4}\" fo:color=\"#{1}\"",
        esc(&f.font),
        hex(f.color),
        f.size,
        weight,
        style
    );
    out.push_str(if f.underline {
        " style:text-underline-style=\"solid\" style:text-underline-width=\"auto\" style:text-underline-color=\"font-color\""
    } else {
        " style:text-underline-style=\"none\""
    });
    out.push_str(if f.strike {
        " style:text-line-through-style=\"solid\""
    } else {
        " style:text-line-through-style=\"none\""
    });
    match f.highlight {
        Some(c) => out.push_str(&format!(" fo:background-color=\"#{}\"", hex(c))),
        None => out.push_str(" fo:background-color=\"transparent\""),
    }
    match f.script.signum() {
        1 => out.push_str(" style:text-position=\"super 58%\""),
        -1 => out.push_str(" style:text-position=\"sub 58%\""),
        _ => out.push_str(" style:text-position=\"0% 100%\""),
    }
    out.push_str("/>");
    out
}

fn paragraph_props(p: &Paragraph, page_break: bool) -> String {
    let align = match p.align {
        Align::Left => "start",
        Align::Center => "center",
        Align::Right => "end",
        Align::Justify => "justify",
    };
    let mut out = format!(
        "<style:paragraph-properties fo:text-align=\"{align}\" fo:margin-top=\"{}pt\" fo:margin-bottom=\"{}pt\"",
        p.spacing_before, p.spacing_after
    );
    // Zero indents are left out so list levels keep the list style's indentation.
    if p.indent != 0.0 || p.first_indent != 0.0 {
        out.push_str(&format!(
            " fo:margin-left=\"{}pt\" fo:text-indent=\"{}pt\"",
            p.indent, p.first_indent
        ));
    }
    if p.line_points > 0.0 {
        if p.line_exact {
            out.push_str(&format!(" fo:line-height=\"{}pt\"", p.line_points));
        } else {
            out.push_str(&format!(
                " style:line-height-at-least=\"{}pt\"",
                p.line_points
            ));
        }
    } else {
        out.push_str(&format!(
            " fo:line-height=\"{}%\"",
            (p.line_spacing * 100.0).round()
        ));
    }
    if page_break {
        out.push_str(" fo:break-before=\"page\"");
    }
    out.push_str(&format!(
        " fo:keep-with-next=\"{}\" fo:keep-together=\"{}\" fo:widows=\"{2}\" fo:orphans=\"{2}\"",
        if p.keep_next { "always" } else { "auto" },
        if p.keep_lines { "always" } else { "auto" },
        if p.widow_control { 2 } else { 0 }
    ));
    if p.contextual_spacing {
        out.push_str(" style:contextual-spacing=\"true\"");
    }
    out.push_str("/>");
    out
}

/// Text with ODF white space: repeated spaces as `text:s`, tabs and line breaks as elements.
fn odf_text(text: &str, out: &mut String, space: &mut bool) {
    for ch in text.chars() {
        match ch {
            ' ' => {
                if *space {
                    out.push_str("<text:s/>");
                } else {
                    out.push(' ');
                }
                *space = true;
            }
            '\t' => {
                out.push_str("<text:tab/>");
                *space = false;
            }
            '\u{000b}' | '\n' | '\r' => {
                out.push_str("<text:line-break/>");
                *space = false;
            }
            '&' => {
                out.push_str("&amp;");
                *space = false;
            }
            '<' => {
                out.push_str("&lt;");
                *space = false;
            }
            '>' => {
                out.push_str("&gt;");
                *space = false;
            }
            c if c.is_control() => {}
            c => {
                out.push(c);
                *space = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{read, write};
    use super::*;

    fn png() -> Result<Vec<u8>, String> {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(4, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(png.into_inner())
    }

    fn rich_doc() -> Result<Doc, String> {
        let mut doc = Doc::from_text(
            "Отчёт\nПервый  абзац\tс табуляцией\nПункт\nПодпункт\nШаг\nПосле таблицы",
        );
        doc.title = "Квартальный отчёт".into();
        doc.author = "Отдел".into();
        doc.page.width = 841.89;
        doc.page.height = 595.28;
        doc.page.margin_left = 72.0;
        doc.page.header = "  Шапка  ".into();
        doc.page.footer = "Стр. ".into();
        doc.page.page_numbers = true;
        let p = doc.paragraphs.get_mut(0).ok_or("p0")?;
        p.set_style("Heading1");
        p.align = Align::Center;
        let p = doc.paragraphs.get_mut(1).ok_or("p1")?;
        p.align = Align::Justify;
        p.first_indent = 18.0;
        p.spacing_after = 12.0;
        p.line_spacing = 1.5;
        p.keep_lines = true;
        let mut bold = p.runs.first().ok_or("run")?.format.clone();
        bold.bold = true;
        bold.italic = true;
        bold.underline = true;
        bold.color = [200, 10, 20];
        bold.size = 14.0;
        bold.font = "Arial".into();
        bold.highlight = Some([255, 255, 0]);
        let sup = TextFormat {
            script: 1,
            strike: true,
            link: Some("https://example.com/a?b=1&c=2".into()),
            ..TextFormat::default()
        };
        p.runs.push(Run {
            text: " жирный\u{000b}перенос".into(),
            format: bold,
        });
        p.runs.push(Run {
            text: "ссылка".into(),
            format: sup,
        });
        doc.paragraphs.get_mut(2).ok_or("p2")?.list = List::Bullet;
        let p = doc.paragraphs.get_mut(3).ok_or("p3")?;
        p.list = List::Bullet;
        p.list_level = 1;
        doc.paragraphs.get_mut(4).ok_or("p4")?.list = List::Number;
        doc.paragraphs.get_mut(5).ok_or("p5")?.page_break_before = true;
        let anchor = doc.paragraphs.get(1).map(|p| p.id).ok_or("anchor")?;
        let table = doc.paragraphs.get(4).map(|p| p.id).ok_or("table anchor")?;
        let id = doc.alloc_id();
        doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: anchor,
            bytes: png()?,
            width: 200.0,
            height: 100.0,
            alt: "Схема".into(),
        });
        doc.add_table(table, 3, 3)?;
        let t = doc.tables.last_mut().ok_or("table")?;
        t.header_rows = 1;
        t.column_widths = vec![100.0, 150.0, 200.0];
        t.borderless = false;
        t.border_color = [10, 20, 30];
        t.row_heights = vec![0.0, 30.0, 0.0];
        let mut next = doc.next_id;
        if let Some(cell) = doc
            .tables
            .last_mut()
            .and_then(|t| t.rows.get_mut(0))
            .and_then(|r| r.get_mut(0))
        {
            edit_cell_text(cell, "Заголовок", &mut next);
            cell.fill = [220, 230, 240];
        }
        doc.next_id = next;
        let t = doc.tables.last_mut().ok_or("table")?;
        // Row 1: the first two columns merged; row 2 continues the third column of row 1.
        if let Some(row) = t.rows.get_mut(1) {
            row.remove(1);
            if let Some(cell) = row.get_mut(0) {
                cell.span = 2;
                cell.v_align = VAlign::Center;
            }
        }
        if let Some(cell) = t.rows.get_mut(2).and_then(|r| r.get_mut(2)) {
            cell.merged = true;
        }
        doc.normalize();
        doc.validate()?;
        Ok(doc)
    }

    #[test]
    fn odt_round_trip_keeps_structure_and_formatting() -> Result<(), String> {
        let doc = rich_doc()?;
        let first = read(&write(&doc, "odt")?, "odt")?;
        let second = read(&write(&first, "odt")?, "odt")?;
        assert_eq!(first, second, "второе поколение ODT отличается от первого");
        assert_eq!(first.plain_text(), doc.plain_text());
        assert_eq!(first.title, "Квартальный отчёт");
        assert_eq!(first.author, "Отдел");
        assert_eq!(first.page.header, "  Шапка  ");
        assert_eq!(first.page.footer, "Стр. ");
        assert!(first.page.page_numbers);
        assert_eq!(first.page.width.round(), 842.0);
        assert_eq!(first.page.margin_left.round(), 72.0);
        let heading = first.paragraphs.first().ok_or("heading")?;
        assert_eq!(heading.style, "Heading1");
        assert_eq!(heading.align, Align::Center);
        let body = first.paragraphs.get(1).ok_or("body")?;
        assert_eq!(
            body.text(),
            "Первый  абзац\tс табуляцией жирный\u{000b}переносссылка"
        );
        assert_eq!(body.align, Align::Justify);
        assert_eq!(body.first_indent, 18.0);
        assert_eq!(body.spacing_after, 12.0);
        assert_eq!(body.line_spacing, 1.5);
        assert!(body.keep_lines);
        let bold = body.runs.get(1).ok_or("bold run")?;
        assert!(bold.format.bold && bold.format.italic && bold.format.underline);
        assert_eq!(bold.format.color, [200, 10, 20]);
        assert_eq!(bold.format.size, 14.0);
        assert_eq!(bold.format.font, "Arial");
        assert_eq!(bold.format.highlight, Some([255, 255, 0]));
        let link = body.runs.get(2).ok_or("link run")?;
        assert_eq!(link.format.script, 1);
        assert!(link.format.strike);
        assert_eq!(
            link.format.link.as_deref(),
            Some("https://example.com/a?b=1&c=2")
        );
        let lists: Vec<(List, u8)> = first
            .paragraphs
            .iter()
            .map(|p| (p.list, p.list_level))
            .collect();
        assert_eq!(
            lists,
            vec![
                (List::None, 0),
                (List::None, 0),
                (List::Bullet, 0),
                (List::Bullet, 1),
                (List::Number, 0),
                (List::None, 0)
            ]
        );
        assert!(first.paragraphs.get(5).is_some_and(|p| p.page_break_before));
        let image = first.images.first().ok_or("image")?;
        assert_eq!((image.width, image.height), (200.0, 100.0));
        assert_eq!(image.alt, "Схема");
        assert_eq!(
            Some(image.after_paragraph_id),
            first.paragraphs.get(1).map(|p| p.id)
        );
        let table = first.tables.first().ok_or("table")?;
        assert_eq!(
            Some(table.after_paragraph_id),
            first.paragraphs.get(4).map(|p| p.id)
        );
        assert_eq!(table.header_rows, 1);
        assert_eq!(table.column_widths, vec![100.0, 150.0, 200.0]);
        assert!(!table.borderless);
        assert_eq!(table.border_color, [10, 20, 30]);
        assert_eq!(table.row_heights, vec![0.0, 30.0, 0.0]);
        let cell = table.rows.first().and_then(|r| r.first()).ok_or("cell")?;
        assert_eq!(cell_text(cell), "Заголовок");
        assert_eq!(cell.fill, [220, 230, 240]);
        let spans: Vec<Vec<(u16, bool)>> = table
            .rows
            .iter()
            .map(|r| r.iter().map(|c| (c.span, c.merged)).collect())
            .collect();
        assert_eq!(
            spans,
            vec![
                vec![(1, false), (1, false), (1, false)],
                vec![(2, false), (1, false)],
                vec![(1, false), (1, false), (1, true)],
            ]
        );
        assert_eq!(
            table.rows.get(1).and_then(|r| r.first()).map(|c| c.v_align),
            Some(VAlign::Center)
        );
        Ok(())
    }

    #[test]
    fn odt_package_starts_with_stored_mimetype() -> Result<(), String> {
        let bytes = write(&rich_doc()?, "odt")?;
        let mut zip =
            zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let first = zip.by_index(0).map_err(|e| e.to_string())?;
        assert_eq!(first.name(), "mimetype");
        assert_eq!(first.compression(), zip::CompressionMethod::Stored);
        drop(first);
        let parts = read_zip(&write(&rich_doc()?, "ott")?)?;
        assert_eq!(
            parts.get("mimetype").map(Vec::as_slice),
            Some(MIME_TEMPLATE.as_bytes())
        );
        let manifest = String::from_utf8(
            parts
                .get("META-INF/manifest.xml")
                .cloned()
                .unwrap_or_default(),
        )
        .map_err(|e| e.to_string())?;
        assert!(manifest.contains("Pictures/image"), "{manifest}");
        Ok(())
    }

    #[test]
    fn flat_odt_round_trip_keeps_inline_pictures() -> Result<(), String> {
        let doc = rich_doc()?;
        let first = read(&write(&doc, "fodt")?, "fodt")?;
        let odt = read(&write(&doc, "odt")?, "odt")?;
        assert_eq!(first.plain_text(), odt.plain_text());
        assert_eq!(first.images.first().map(|i| i.bytes.clone()), Some(png()?));
        assert_eq!(first.tables, odt.tables);
        Ok(())
    }

    #[test]
    fn reads_libreoffice_styles_lists_and_merged_cells() -> Result<(), String> {
        let styles = format!(
            "<office:document-styles {NAMESPACES}><office:font-face-decls><style:font-face style:name=\"Liberation Serif1\" svg:font-family=\"'Liberation Serif'\"/></office:font-face-decls><office:styles><style:default-style style:family=\"paragraph\"><style:text-properties style:font-name=\"Liberation Serif1\" fo:font-size=\"12pt\"/></style:default-style><style:style style:name=\"Standard\" style:family=\"paragraph\"/><style:style style:name=\"Heading\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:text-properties fo:font-size=\"14pt\"/></style:style><style:style style:name=\"Heading_20_2\" style:display-name=\"Heading 2\" style:family=\"paragraph\" style:parent-style-name=\"Heading\"><style:text-properties fo:font-size=\"115%\" fo:font-weight=\"bold\"/></style:style><style:style style:name=\"Strong_20_Emphasis\" style:family=\"text\"><style:text-properties fo:font-weight=\"bold\"/></style:style></office:styles><office:automatic-styles><style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"21.001cm\" fo:page-height=\"29.7cm\" fo:margin-top=\"2cm\" fo:margin-bottom=\"2cm\" fo:margin-left=\"3cm\" fo:margin-right=\"1.5cm\"/></style:page-layout></office:automatic-styles><office:master-styles><style:master-page style:name=\"Standard\" style:page-layout-name=\"pm1\"><style:footer><text:p>Страница <text:page-number>1</text:page-number></text:p></style:footer></style:master-page></office:master-styles></office:document-styles>"
        );
        let content = format!(
            "<office:document-content {NAMESPACES}><office:automatic-styles><style:style style:name=\"P1\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"><style:paragraph-properties fo:text-align=\"end\" fo:margin-left=\"1cm\" fo:break-before=\"page\"/></style:style><style:style style:name=\"T1\" style:family=\"text\" style:parent-style-name=\"Strong_20_Emphasis\"><style:text-properties fo:font-style=\"italic\" style:text-position=\"-33% 58%\"/></style:style><style:style style:name=\"ce1\" style:family=\"table-cell\"><style:table-cell-properties fo:background-color=\"#ff0000\" fo:border=\"0.06pt solid #00ff00\" fo:padding=\"0.1cm\"/></style:style><text:list-style style:name=\"L1\"><text:list-level-style-number text:level=\"1\" style:num-format=\"1\"/><text:list-level-style-bullet text:level=\"2\" text:bullet-char=\"-\"/></text:list-style></office:automatic-styles><office:body><office:text><text:sequence-decls/><text:h text:style-name=\"Heading_20_2\" text:outline-level=\"2\">Раздел</text:h><text:p text:style-name=\"P1\">  Много   пробелов<text:s text:c=\"2\"/>и <text:span text:style-name=\"T1\">выделение</text:span><text:note><text:note-body><text:p>сноска</text:p></text:note-body></text:note></text:p><text:list text:style-name=\"L1\"><text:list-item><text:p>Один</text:p><text:list><text:list-item><text:p>Вложенный</text:p></text:list-item></text:list></text:list-item><text:list-header><text:p>Без номера</text:p></text:list-header></text:list><table:table><table:table-column table:number-columns-repeated=\"2\"/><table:table-row><table:table-cell table:style-name=\"ce1\" table:number-rows-spanned=\"2\"><text:p>A</text:p></table:table-cell><table:table-cell><text:p>B</text:p></table:table-cell></table:table-row><table:table-row><table:covered-table-cell/><table:table-cell><text:p>C</text:p></table:table-cell></table:table-row></table:table></office:text></office:body></office:document-content>"
        );
        let bytes = write_zip(vec![
            ("mimetype".into(), MIME_TEXT.as_bytes().to_vec()),
            ("styles.xml".into(), styles.into_bytes()),
            ("content.xml".into(), content.into_bytes()),
        ])?;
        let doc = read(&bytes, "odt")?;
        let heading = doc.paragraphs.first().ok_or("heading")?;
        assert_eq!(heading.style, "Heading2");
        let format = heading.format_at(0);
        assert_eq!(format.font, "Liberation Serif");
        assert!((format.size - 16.1).abs() < 0.01, "{}", format.size);
        assert!(format.bold);
        let p = doc.paragraphs.get(1).ok_or("paragraph")?;
        assert_eq!(p.text(), "Много пробелов  и выделение");
        assert_eq!(p.align, Align::Right);
        assert!((p.indent - 28.35).abs() < 0.01);
        assert!(p.page_break_before);
        let emphasis = p.runs.last().ok_or("emphasis")?;
        assert!(emphasis.format.bold && emphasis.format.italic);
        assert_eq!(emphasis.format.script, -1);
        let lists: Vec<(String, List, u8)> = doc
            .paragraphs
            .iter()
            .skip(2)
            .map(|p| (p.text(), p.list, p.list_level))
            .collect();
        assert_eq!(
            lists,
            vec![
                ("Один".into(), List::Number, 0),
                ("Вложенный".into(), List::Bullet, 1),
                ("Без номера".into(), List::None, 0),
            ]
        );
        let table = doc.tables.first().ok_or("table")?;
        assert_eq!(table.rows.len(), 2);
        assert_eq!(
            table
                .rows
                .get(1)
                .map(|r| r.iter().map(|c| c.merged).collect::<Vec<_>>()),
            Some(vec![true, false])
        );
        let a = table.rows.first().and_then(|r| r.first()).ok_or("A")?;
        assert_eq!(a.fill, [255, 0, 0]);
        assert_eq!(table.border_color, [0, 255, 0]);
        assert!((table.cell_margin[0] - 2.83).abs() < 0.01);
        assert!(!table.borderless);
        assert_eq!(doc.page.footer, "Страница ");
        assert!(doc.page.page_numbers);
        assert!((doc.page.margin_left - 85.04).abs() < 0.01);
        assert!(!doc.plain_text().contains("сноска"));
        assert!(
            doc.losses.iter().any(|l| l.contains("Сноски")),
            "{:?}",
            doc.losses
        );
        Ok(())
    }

    #[test]
    fn rejects_other_opendocument_types_and_broken_packages() -> Result<(), String> {
        let spreadsheet = write_zip(vec![(
            "mimetype".into(),
            b"application/vnd.oasis.opendocument.spreadsheet".to_vec(),
        )])?;
        assert!(read(&spreadsheet, "odt").is_err());
        assert!(read(b"not a zip", "odt").is_err());
        assert!(read(b"<root/>", "fodt").is_err());
        Ok(())
    }

    #[test]
    fn base64_round_trip() {
        for data in [&b""[..], b"a", b"ab", b"abc", b"abcd", &[0, 255, 128, 7, 9]] {
            assert_eq!(base64_decode(&base64_encode(data)).as_deref(), Some(data));
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_decode("TW\nE=").as_deref(), Some(&b"Ma"[..]));
    }
}
