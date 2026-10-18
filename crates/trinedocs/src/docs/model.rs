use crate::render::{Page, Primitive};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextFormat {
    pub font: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: [u8; 3],
    pub highlight: Option<[u8; 3]>,
    pub script: i8,
    pub link: Option<String>,
}
impl Default for TextFormat {
    fn default() -> Self {
        Self {
            font: "Calibri".into(),
            size: 11.0,
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            color: [24, 28, 35],
            highlight: None,
            script: 0,
            link: None,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
    pub format: TextFormat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum List {
    #[default]
    None,
    Bullet,
    Number,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Paragraph {
    pub id: u64,
    pub runs: Vec<Run>,
    pub style: String,
    pub align: Align,
    pub list: List,
    pub list_level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_start: Option<u32>,
    pub indent: f32,
    #[serde(skip_serializing_if = "is_zero")]
    pub right_indent: f32,
    pub first_indent: f32,
    pub spacing_before: f32,
    pub spacing_after: f32,
    pub line_spacing: f32,
    /// Fixed line height in points (Word "Exactly"/"At least"); 0 = use `line_spacing`.
    #[serde(skip_serializing_if = "is_zero")]
    pub line_points: f32,
    /// With `line_points`: true = exactly, false = at least.
    #[serde(skip_serializing_if = "is_false")]
    pub line_exact: bool,
    pub page_break_before: bool,
    /// Keep on the same page as the next paragraph (`w:keepNext`).
    #[serde(skip_serializing_if = "is_false")]
    pub keep_next: bool,
    /// Don't split the paragraph across pages (`w:keepLines`).
    #[serde(skip_serializing_if = "is_false")]
    pub keep_lines: bool,
    /// No page starts or ends with a single line of the paragraph (`w:widowControl`).
    #[serde(skip_serializing_if = "is_true")]
    pub widow_control: bool,
    /// No spacing between paragraphs of the same style (`w:contextualSpacing`).
    #[serde(skip_serializing_if = "is_false")]
    pub contextual_spacing: bool,
}
fn is_true(value: &bool) -> bool {
    *value
}
fn is_zero(value: &f32) -> bool {
    *value == 0.0
}
impl Default for Paragraph {
    fn default() -> Self {
        Self {
            id: 1,
            runs: vec![Run::default()],
            style: "Normal".into(),
            align: Align::Left,
            list: List::None,
            list_level: 0,
            list_start: None,
            indent: 0.0,
            right_indent: 0.0,
            first_indent: 0.0,
            spacing_before: 0.0,
            spacing_after: 6.0,
            line_spacing: 1.15,
            line_points: 0.0,
            line_exact: false,
            page_break_before: false,
            keep_next: false,
            keep_lines: false,
            widow_control: true,
            contextual_spacing: false,
        }
    }
}
pub fn heading_level(style: &str) -> Option<u8> {
    match style {
        "Heading1" => Some(1),
        "Heading2" => Some(2),
        "Heading3" => Some(3),
        "Heading4" => Some(4),
        "Heading5" => Some(5),
        "Heading6" => Some(6),
        _ => None,
    }
}
impl Paragraph {
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
    pub fn len(&self) -> usize {
        self.runs.iter().map(|r| r.text.chars().count()).sum()
    }
    pub fn is_empty(&self) -> bool {
        self.runs.iter().all(|r| r.text.is_empty())
    }
    pub fn format_at(&self, offset: usize) -> TextFormat {
        let mut n = 0;
        for run in &self.runs {
            n += run.text.chars().count();
            if offset <= n {
                return run.format.clone();
            }
        }
        self.runs
            .last()
            .map(|r| r.format.clone())
            .unwrap_or_default()
    }
    fn slice(&self, start: usize, end: usize) -> Vec<Run> {
        let mut out = Vec::new();
        let mut n = 0;
        for run in &self.runs {
            let len = run.text.chars().count();
            let a = start.saturating_sub(n).min(len);
            let b = end.saturating_sub(n).min(len);
            if b > a {
                out.push(Run {
                    text: run.text.chars().skip(a).take(b - a).collect(),
                    format: run.format.clone(),
                });
            }
            n += len;
        }
        out
    }
    pub fn normalize(&mut self) {
        let mut compact: Vec<Run> = Vec::new();
        let mut empty_format = None;
        for run in self.runs.drain(..) {
            if run.text.is_empty() {
                empty_format.get_or_insert(run.format);
                continue;
            }
            if let Some(last) = compact.last_mut().filter(|last| last.format == run.format) {
                last.text.push_str(&run.text);
            } else {
                compact.push(run);
            }
        }
        if compact.is_empty() {
            compact.push(Run {
                text: String::new(),
                format: empty_format.unwrap_or_default(),
            });
        }
        self.runs = compact;
    }
    pub fn set_style(&mut self, style: &str) {
        let leaving_code=self.style=="Code" && style!="Code";
        self.style = style.into();
        let (size, bold) = match style {
            "Title" => (28.0, true),
            "Heading1" => (22.0, true),
            "Heading2" => (17.0, true),
            "Heading3" => (14.0, true),
            "Heading4" => (12.0, true),
            "Heading5" | "Heading6" => (11.0, true),
            "Quote" => (11.0, false),
            _ => (11.0, false),
        };
        for run in &mut self.runs {
            run.format.size = size;
            run.format.bold = bold;
            run.format.italic = style == "Quote";
            if style=="Code" {run.format.font="Consolas".into();}
            else if leaving_code && run.format.font=="Consolas" {run.format.font=TextFormat::default().font;}
        }
        self.spacing_before = if heading_level(style).is_some() {
            12.0
        } else {
            0.0
        };
        // Like Word's built-in headings, a heading stays with the text that follows it.
        self.keep_next = heading_level(style).is_some() || style == "Title";
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageSetup {
    pub width: f32,
    pub height: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub header: String,
    pub footer: String,
    pub page_numbers: bool,
}
impl Default for PageSetup {
    fn default() -> Self {
        Self {
            width: 595.28,
            height: 841.89,
            margin_left: 56.7,
            margin_right: 56.7,
            margin_top: 56.7,
            margin_bottom: 56.7,
            header: String::new(),
            footer: String::new(),
            page_numbers: true,
        }
    }
}
impl PageSetup {
    pub fn fit_margins(&mut self) {
        let horizontal = self.margin_left + self.margin_right;
        let vertical = self.margin_top + self.margin_bottom;
        let width = (self.width - 24.0).max(0.0);
        let height = (self.height - 30.0).max(0.0);
        if horizontal > width {
            self.margin_left *= width / horizontal;
            self.margin_right = (width - self.margin_left).max(0.0);
        }
        if vertical > height {
            self.margin_top *= height / vertical;
            self.margin_bottom = (height - self.margin_top).max(0.0);
        }
    }
}
fn one_span() -> u16 {
    1
}
fn is_one(span: &u16) -> bool {
    *span == 1
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn is_top(align: &VAlign) -> bool {
    *align == VAlign::Top
}
/// Vertical alignment of text inside a table cell (`w:vAlign`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign {
    #[default]
    Top,
    Center,
    Bottom,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub id: u64,
    pub paragraphs: Vec<Paragraph>,
    pub fill: [u8; 3],
    /// Number of grid columns the cell covers (`w:gridSpan`).
    #[serde(default = "one_span", skip_serializing_if = "is_one")]
    pub span: u16,
    /// Continuation of a vertically merged cell from the row above (`w:vMerge`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub merged: bool,
    #[serde(default, skip_serializing_if = "is_top")]
    pub v_align: VAlign,
}
impl Cell {
    pub fn new(id: u64, paragraphs: Vec<Paragraph>, fill: [u8; 3]) -> Self {
        Self {
            id,
            paragraphs,
            fill,
            span: 1,
            merged: false,
            v_align: VAlign::Top,
        }
    }
    pub fn span(&self) -> usize {
        usize::from(self.span.max(1))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: u64,
    pub after_paragraph_id: u64,
    pub rows: Vec<Vec<Cell>>,
    pub column_widths: Vec<f32>,
    pub border_color: [u8; 3],
    /// Word tables without borders (e.g. signature blocks) stay borderless.
    #[serde(default, skip_serializing_if = "is_false")]
    pub borderless: bool,
    /// Cell padding in points: top, right, bottom, left (`w:tblCellMar`).
    #[serde(default = "legacy_cell_margin")]
    pub cell_margin: [f32; 4],
    /// Minimum height of each row in points (`w:trHeight`); 0 or missing = automatic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_heights: Vec<f32>,
    /// Leading rows repeated at the top of every page the table continues on (`w:tblHeader`).
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub header_rows: usize,
}
fn is_zero_count(value: &usize) -> bool {
    *value == 0
}
/// Padding of tables saved before cell margins were stored.
fn legacy_cell_margin() -> [f32; 4] {
    [5.0; 4]
}
/// Word's default cell margins: 0 top/bottom, 0.19 cm left/right.
pub const WORD_CELL_MARGIN: [f32; 4] = [0.0, 5.4, 0.0, 5.4];
impl Table {
    /// Number of grid columns, counting merged cells by their span.
    pub fn grid_columns(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.iter().map(Cell::span).sum::<usize>())
            .max()
            .unwrap_or_default()
    }
    /// First grid column of each cell in `row`.
    pub fn grid_starts(&self, row: usize) -> Vec<usize> {
        let mut start = 0;
        self.rows
            .get(row)
            .map(|cells| {
                cells
                    .iter()
                    .map(|cell| {
                        let at = start;
                        start += cell.span();
                        at
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnchoredImage {
    pub id: u64,
    pub after_paragraph_id: u64,
    pub bytes: Vec<u8>,
    pub width: f32,
    pub height: f32,
    pub alt: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Doc {
    pub title: String,
    pub author: String,
    pub paragraphs: Vec<Paragraph>,
    pub tables: Vec<Table>,
    pub images: Vec<AnchoredImage>,
    pub page: PageSetup,
    pub next_id: u64,
    pub losses: Vec<String>,
}
impl Default for Doc {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
            paragraphs: vec![Paragraph::default()],
            tables: Vec::new(),
            images: Vec::new(),
            page: PageSetup::default(),
            next_id: 2,
            losses: Vec::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub paragraph: usize,
    pub offset: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditDelta {
    pub start_id: u64,
    pub end_id: u64,
    pub after_end_id: u64,
    pub start: Position,
    pub end: Position,
    pub after_end: Position,
}
impl TextEditDelta {
    pub fn capture(doc: &Doc, start: Position, end: Position) -> Result<Self, String> {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let a = doc
            .paragraphs
            .get(start.paragraph)
            .ok_or("Нет начального абзаца delta")?;
        let b = doc
            .paragraphs
            .get(end.paragraph)
            .ok_or("Нет конечного абзаца delta")?;
        if start.offset > a.len() || end.offset > b.len() {
            return Err("Delta за пределами текста".into());
        }
        Ok(Self {
            start_id: a.id,
            end_id: b.id,
            after_end_id: a.id,
            start,
            end,
            after_end: start,
        })
    }
    pub fn finish(mut self, doc: &Doc, after_end: Position) -> Result<Self, String> {
        let p = doc
            .paragraphs
            .get(after_end.paragraph)
            .ok_or("Нет конечного абзаца после delta")?;
        if after_end.offset > p.len() {
            return Err("Конечная delta за пределами текста".into());
        }
        self.after_end_id = p.id;
        self.after_end = after_end;
        Ok(self)
    }
    pub fn transform(&self, position: Position) -> Position {
        if position < self.start {
            return position;
        }
        if position <= self.end {
            return self.after_end;
        }
        if position.paragraph == self.end.paragraph {
            Position {
                paragraph: self.after_end.paragraph,
                offset: self
                    .after_end
                    .offset
                    .saturating_add(position.offset.saturating_sub(self.end.offset)),
            }
        } else {
            Position {
                paragraph: self
                    .after_end
                    .paragraph
                    .saturating_add(position.paragraph.saturating_sub(self.end.paragraph)),
                offset: position.offset,
            }
        }
    }
    pub fn inverse(&self) -> Self {
        Self {
            start_id: self.start_id,
            end_id: self.after_end_id,
            after_end_id: self.end_id,
            start: self.start,
            end: self.after_end,
            after_end: self.end,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub text: String,
    pub format: TextFormat,
    pub x: f32,
    pub width: f32,
}
#[derive(Clone, Debug)]
pub struct Row {
    pub paragraph: usize,
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub page: usize,
    pub segments: Vec<Segment>,
    pub marker: String,
}
/// Where a table cell (or its part on one page) was laid out, in page points.
#[derive(Clone, Debug, PartialEq)]
pub struct CellBox {
    pub table: u64,
    pub row: usize,
    pub column: usize,
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub pages: Vec<Page>,
    pub rows: Vec<Row>,
    pub cells: Vec<CellBox>,
}

/// Row/column edits of a Docs table, shared by the editor ribbon and MCP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableEdit {
    InsertRow(usize),
    InsertColumn(usize),
    DeleteRow(usize),
    DeleteColumn(usize),
}
fn empty_cell(next: &mut u64, fill: [u8; 3]) -> Cell {
    let id = *next;
    let paragraph = next.saturating_add(1);
    *next = next.saturating_add(2);
    Cell::new(
        id,
        vec![Paragraph {
            id: paragraph,
            ..Paragraph::default()
        }],
        fill,
    )
}
/// Plain text of a cell, paragraphs separated by line breaks.
pub fn cell_text(cell: &Cell) -> String {
    cell.paragraphs
        .iter()
        .map(Paragraph::text)
        .collect::<Vec<_>>()
        .join("\n")
}
/// Replaces the text of a cell, keeping formats and IDs of unchanged parts.
pub fn edit_cell_text(cell: &mut Cell, text: &str, next_id: &mut u64) {
    let _=try_edit_cell_text(cell,text,next_id);
}
pub fn try_edit_cell_text(cell: &mut Cell, text: &str, next_id: &mut u64)->Result<(),String> {
    let normalized=text.replace("\r\n","\n").replace('\r',"\n");
    let text=normalized.as_str();
    let mut nested = Doc {
        paragraphs: cell.paragraphs.clone(),
        next_id: *next_id,
        ..Doc::default()
    };
    let old = nested.plain_text();
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = text.chars().collect();
    let prefix = old_chars
        .iter()
        .zip(&new_chars)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = old_chars
        .iter()
        .rev()
        .zip(new_chars.iter().rev())
        .take(old_chars.len().min(new_chars.len()).saturating_sub(prefix))
        .take_while(|(a, b)| a == b)
        .count();
    fn position_at(paragraphs: &[Paragraph], mut offset: usize) -> Position {
        for (paragraph, p) in paragraphs.iter().enumerate() {
            let len = p.len();
            if offset <= len {
                return Position { paragraph, offset };
            }
            offset = offset.saturating_sub(len + 1);
        }
        let paragraph = paragraphs.len().saturating_sub(1);
        Position {
            paragraph,
            offset: paragraphs.last().map(Paragraph::len).unwrap_or_default(),
        }
    }
    let a = position_at(&nested.paragraphs, prefix);
    let b = position_at(&nested.paragraphs, old_chars.len().saturating_sub(suffix));
    let inserted: String = new_chars
        .iter()
        .skip(prefix)
        .take(new_chars.len().saturating_sub(prefix + suffix))
        .collect();
    nested.replace_range(a,b,&inserted,None)?;
    cell.paragraphs=nested.paragraphs;*next_id=nested.next_id;
    Ok(())
}
type FaceMetrics = (
    String,
    bool,
    bool,
    std::sync::Arc<std::collections::HashMap<char, f32>>,
);
/// Height of a text line: font line metrics × size × paragraph line spacing.
pub fn line_height(format: &TextFormat, size: f32, spacing: f32) -> f32 {
    (size * crate::fonts::line_factor(&format.font) * spacing).max(4.0)
}
/// Line height of a paragraph line, honouring Word's "Exactly" / "At least" rules.
pub fn paragraph_line_height(p: &Paragraph, format: &TextFormat, size: f32) -> f32 {
    if p.line_points > 0.0 {
        let fixed = p.line_points.clamp(1.0, 1584.0);
        if p.line_exact {
            fixed
        } else {
            line_height(format, size, 1.0).max(fixed)
        }
    } else {
        line_height(format, size, p.line_spacing)
    }
}
pub fn char_width(ch: char, format: &TextFormat) -> f32 {
    thread_local! {
        static LAST: std::cell::RefCell<Option<FaceMetrics>> =
            const { std::cell::RefCell::new(None) };
    }
    let metrics = LAST.with(|last| {
        let mut last = last.borrow_mut();
        match last.as_ref() {
            Some((font, bold, italic, table))
                if *font == format.font && *bold == format.bold && *italic == format.italic =>
            {
                table.clone()
            }
            _ => {
                let table = crate::fonts::advances(&format.font, format.bold, format.italic);
                *last = Some((
                    format.font.clone(),
                    format.bold,
                    format.italic,
                    table.clone(),
                ));
                table
            }
        }
    });
    let size = format.size.clamp(4.0, 144.0) * if format.script != 0 { 0.75 } else { 1.0 };
    if matches!(ch,'\u{00ad}'|'\u{200b}'|'\u{200c}'|'\u{200d}'|'\u{2060}'|'\u{feff}') {return 0.0;}
    if ch == '\t' {
        return metrics.get(&' ').copied().unwrap_or(0.3) * size * 4.0;
    }
    metrics.get(&ch).copied().unwrap_or_else(|| {
        if ch.is_whitespace() {
            0.3
        } else if "ilI.,:;!|'".contains(ch) {
            0.28
        } else if "MWЖШЩЮФ@".contains(ch) {
            0.88
        } else if ch as u32 >= 0x2e80 {
            1.0
        } else {
            0.56
        }
    }) * size
}
impl Doc {
    pub fn validate(&self) -> Result<(), String> {
        if self.paragraphs.is_empty()
            || self.paragraphs.len() > 100000
            || self.tables.len() > 1000
            || self.images.len() > 1000
        {
            return Err("Недопустимое число блоков Docs".into());
        }
        let page = &self.page;
        if [
            page.width,
            page.height,
            page.margin_left,
            page.margin_right,
            page.margin_top,
            page.margin_bottom,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || !(144.0..=2880.0).contains(&page.width)
            || !(144.0..=2880.0).contains(&page.height)
            || page.margin_left < 0.0
            || page.margin_right < 0.0
            || page.margin_top < 0.0
            || page.margin_bottom < 0.0
            || page.margin_left + page.margin_right > page.width - 24.0
            || page.margin_top + page.margin_bottom > page.height - 30.0
        {
            return Err("Недопустимые размеры/поля страницы".into());
        }
        if self.title.len() > 32767
            || self.author.len() > 32767
            || page.header.len() > 32767
            || page.footer.len() > 32767
        {
            return Err("Слишком длинные свойства документа".into());
        }
        if self.losses.len()>1000 || self.losses.iter().any(|loss|loss.len()>8192)
            || self.losses.iter().fold(0usize,|total,loss|total.saturating_add(loss.len()))>1024*1024 {
            return Err("Import notices Docs превышают безопасный лимит".into());
        }
        if [&self.title,&self.author,&page.header,&page.footer].iter().any(|text|!xml_model_text(text,false)) {
            return Err("Свойства Docs содержат недопустимые control characters".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut text_size = 0usize;
        let mut runs = 0usize;
        let mut cells = 0usize;
        let mut estimated_height = 0.0_f64;
        let body_width = f64::from(page.width - page.margin_left - page.margin_right);
        let body_height = f64::from(page.height - page.margin_top - page.margin_bottom);
        for p in &self.paragraphs {
            validate_paragraph(p, &mut ids, &mut text_size, &mut runs)?;
            let size = p
                .runs
                .iter()
                .map(|r| r.format.size)
                .fold(11.0_f32, f32::max);
            let height = f64::from(paragraph_line_height(p, &p.format_at(0), size));
            if height > body_height {
                return Err("Строка текста выше доступной области страницы".into());
            }
            let width = (body_width
                - f64::from(p.indent.max(0.0) + p.right_indent.max(0.0) + p.first_indent.max(0.0))
                - f64::from(p.list_level) * 18.0)
                .max(4.0);
            let line_count = (p.len() as f64 * f64::from(size) / width).ceil().max(1.0)
                + p.text()
                    .chars()
                    .filter(|c| matches!(*c, '\u{000b}' | '\u{000c}' | '\n'))
                    .count() as f64;
            estimated_height += line_count * height
                + f64::from(p.spacing_before + p.spacing_after)
                + p.runs
                    .iter()
                    .flat_map(|run| run.text.chars())
                    .filter(|ch| *ch == '\u{000c}')
                    .count() as f64
                    * body_height;
            if p.page_break_before {
                estimated_height += body_height;
            }
        }
        let mut image_bytes = 0usize;
        for table in &self.tables {
            unique_id(table.id, &mut ids)?;
            if !self
                .paragraphs
                .iter()
                .any(|p| p.id == table.after_paragraph_id)
                || table.rows.is_empty()
                || table.rows.len() > 10000
                || table
                    .rows
                    .iter()
                    .any(|row| row.is_empty() || row.len() > 256)
                || table.rows.iter().flatten().any(|c| c.span == 0)
                || table.grid_columns() > 256
                || table.column_widths.len() > 256
                || table
                    .column_widths
                    .iter()
                    .any(|w| !w.is_finite() || *w <= 0.0 || *w > 10000.0)
            {
                return Err("Недопустимая таблица Docs".into());
            }
            if table
                .cell_margin
                .iter()
                .any(|m| !m.is_finite() || !(0.0..=72.0).contains(m))
                || table
                    .row_heights
                    .iter()
                    .any(|h| !h.is_finite() || !(0.0..=1500.0).contains(h))
            {
                return Err("Недопустимые размеры/заголовки таблицы Docs".into());
            }
            if table.header_rows>table.rows.len() || table.row_heights.len()>table.rows.len() {
                return Err("Геометрия/заголовки таблицы выходят за её строки".into());
            }
            validate_table_merges(table)?;
            let columns = table.grid_columns().max(1);
            let widths: Vec<f64> = (0..columns)
                .map(|i| {
                    f64::from(
                        table
                            .column_widths
                            .get(i)
                            .copied()
                            .unwrap_or(body_width as f32 / columns as f32)
                            .max(0.05),
                    )
                })
                .collect();
            let raw_sum = widths.iter().sum::<f64>();
            let scale = (body_width / raw_sum).min(1.0);
            for row in &table.rows {
                let mut row_lines = 1.0_f64;
                let mut grid_start = 0;
                for cell in row {
                    let width = (widths
                        .iter()
                        .skip(grid_start)
                        .take(cell.span())
                        .sum::<f64>()
                        * scale
                        - f64::from(table.cell_margin[1] + table.cell_margin[3]))
                    .max(1.0);
                    grid_start += cell.span();
                    cells += 1;
                    if cells > 100000 {
                        return Err("Таблицы Docs превышают 100 000 ячеек".into());
                    }
                    unique_id(cell.id, &mut ids)?;
                    if cell.paragraphs.is_empty() || cell.paragraphs.len() > 10000 {
                        return Err("Недопустимое число абзацев ячейки".into());
                    }
                    let mut cell_lines = 0.0;
                    for p in &cell.paragraphs {
                        validate_paragraph(p, &mut ids, &mut text_size, &mut runs)?;
                        let size = p
                            .runs
                            .iter()
                            .map(|r| r.format.size)
                            .fold(11.0_f32, f32::max);
                        let line_height =
                            f64::from(paragraph_line_height(p, &p.format_at(0), size));
                        if line_height + f64::from(p.spacing_before + p.spacing_after + table.cell_margin[0] + table.cell_margin[2])
                            > body_height
                        {
                            return Err("Строка таблицы выше доступной области страницы".into());
                        }
                        let hard_breaks = p
                            .runs
                            .iter()
                            .flat_map(|r| r.text.chars())
                            .filter(|c| matches!(*c, '\u{000b}' | '\u{000c}' | '\n'))
                            .count();
                        let line_width=(width-f64::from(p.indent.max(0.0)+p.right_indent.max(0.0)+p.first_indent.max(0.0)+list_indent(p))).max(1.0);
                        cell_lines += ((p.len() as f64 * f64::from(size) / line_width).ceil().max(1.0)
                            + hard_breaks as f64)
                            * line_height
                            / 15.0
                            + f64::from(p.spacing_before + p.spacing_after) / 15.0;
                    }
                    row_lines = row_lines.max(cell_lines);
                }
                estimated_height += row_lines * 15.0 + f64::from(table.cell_margin[0]+table.cell_margin[2]);
            }
        }
        for image in &self.images {
            unique_id(image.id, &mut ids)?;
            if !self
                .paragraphs
                .iter()
                .any(|p| p.id == image.after_paragraph_id)
                || !image.width.is_finite()
                || !image.height.is_finite()
                || image.width <= 0.0
                || image.height <= 0.0
                || image.width > 10000.0
                || image.height > 10000.0
                || image.alt.len() > 32767
                || !xml_model_text(&image.alt,false)
                || image.bytes.len() > 32 * 1024 * 1024
            {
                return Err("Недопустимое изображение Docs".into());
            }
            image_bytes = image_bytes
                .checked_add(image.bytes.len())
                .ok_or("Переполнение размера изображений")?;
            if image_bytes > 64 * 1024 * 1024 {
                return Err("Изображения Docs превышают 64 МБ".into());
            }
            let reader = image::ImageReader::new(std::io::Cursor::new(&image.bytes))
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let (w, h) = reader
                .into_dimensions()
                .map_err(|e| format!("Повреждённое изображение Docs: {e}"))?;
            if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 32_000_000 {
                return Err("Изображение превышает 32 млн пикселей".into());
            }
            estimated_height += body_height.min(f64::from(image.height)) + 8.0;
        }
        if self.next_id == u64::MAX || self.next_id <= ids.iter().copied().max().unwrap_or_default()
        {
            return Err("Недопустимый счётчик ID Docs".into());
        }
        if !estimated_height.is_finite() || estimated_height / body_height > 9000.0 {
            return Err("Документ требует больше безопасного лимита 9000 страниц".into());
        }
        Ok(())
    }
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id.max(2);
        self.next_id = id.checked_add(1).unwrap_or(2);
        id
    }
    pub fn normalize(&mut self) {
        if self.paragraphs.is_empty() {
            self.paragraphs.push(Paragraph::default());
        }
        let mut seen = std::collections::HashSet::new();
        let mut next = self.next_id.max(2);
        for p in self.paragraphs.iter_mut().chain(
            self.tables
                .iter_mut()
                .flat_map(|t| t.rows.iter_mut().flatten())
                .flat_map(|c| c.paragraphs.iter_mut()),
        ) {
            if p.id == 0 || !seen.insert(p.id) {
                while seen.contains(&next) {
                    next = next.checked_add(1).unwrap_or(2);
                }
                p.id = next;
                seen.insert(next);
                next = next.checked_add(1).unwrap_or(2);
            }
            next = next.max(p.id.checked_add(1).unwrap_or(2));
            p.normalize();
            p.line_spacing = p.line_spacing.clamp(0.5, 5.0);
            if !p.line_points.is_finite() {
                p.line_points = 0.0;
            }
            p.line_points = p.line_points.clamp(0.0, 1584.0);
            for run in &mut p.runs {
                run.format.size = run.format.size.clamp(4.0, 144.0);
            }
        }
        self.next_id = next;
        for table in &mut self.tables {
            assign_unique_id(&mut table.id, &mut next, &mut seen);
            if !self
                .paragraphs
                .iter()
                .any(|p| p.id == table.after_paragraph_id)
            {
                table.after_paragraph_id = self.paragraphs.first().map(|p| p.id).unwrap_or(1);
            }
            for cell in table.rows.iter_mut().flatten() {
                assign_unique_id(&mut cell.id, &mut next, &mut seen);
            }
        }
        for image in &mut self.images {
            assign_unique_id(&mut image.id, &mut next, &mut seen);
            if !self
                .paragraphs
                .iter()
                .any(|p| p.id == image.after_paragraph_id)
            {
                image.after_paragraph_id = self.paragraphs.first().map(|p| p.id).unwrap_or(1);
            }
        }
        while seen.contains(&next) {
            next = next.checked_add(1).unwrap_or(2);
        }
        self.next_id = next;
        self.page.width = self.page.width.clamp(144.0, 2880.0);
        self.page.height = self.page.height.clamp(144.0, 2880.0);
        self.page.margin_left = self.page.margin_left.clamp(0.0, self.page.width - 24.0);
        self.page.margin_right = self.page.margin_right.clamp(0.0, self.page.width - 24.0);
        self.page.margin_top = self.page.margin_top.clamp(0.0, self.page.height - 30.0);
        self.page.margin_bottom = self.page.margin_bottom.clamp(0.0, self.page.height - 30.0);
        self.page.fit_margins();
    }
    pub fn from_text(text: &str) -> Self {
        let mut doc = Self::default();
        doc.paragraphs.clear();
        for line in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
            let id = doc.alloc_id();
            doc.paragraphs.push(Paragraph {
                id,
                runs: vec![Run {
                    text: line.into(),
                    format: TextFormat::default(),
                }],
                ..Paragraph::default()
            });
        }
        doc
    }
    pub fn plain_text(&self) -> String {
        let mut out = Vec::new();
        for paragraph in &self.paragraphs {
            out.push(paragraph.text());
            for table in self
                .tables
                .iter()
                .filter(|t| t.after_paragraph_id == paragraph.id)
            {
                for row in &table.rows {
                    out.push(
                        row.iter()
                            .map(|c| {
                                c.paragraphs
                                    .iter()
                                    .map(Paragraph::text)
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            })
                            .collect::<Vec<_>>()
                            .join("\t"),
                    );
                }
            }
        }
        out.join("\n")
    }
    pub fn word_count(&self) -> usize {
        self.paragraphs.iter().map(|paragraph|paragraph.text().split_whitespace().count()).sum::<usize>()
            + self.tables.iter().flat_map(|table|table.rows.iter().enumerate())
                .flat_map(|(row,cells)|cells.iter().filter(move |cell|row==0 || !cell.merged))
                .flat_map(|cell|&cell.paragraphs)
                .map(|paragraph|paragraph.text().split_whitespace().count()).sum::<usize>()
    }
    pub fn position(&self, paragraph_id: u64, offset: usize) -> Result<Position, String> {
        let paragraph = self
            .paragraphs
            .iter()
            .position(|p| p.id == paragraph_id)
            .ok_or_else(|| "Абзац не найден".to_string())?;
        if offset
            > self
                .paragraphs
                .get(paragraph)
                .map(Paragraph::len)
                .unwrap_or_default()
        {
            return Err("Позиция за пределами абзаца".into());
        }
        Ok(Position { paragraph, offset })
    }
    pub fn clamp_position(&self, pos: Position) -> Position {
        let paragraph = pos.paragraph.min(self.paragraphs.len().saturating_sub(1));
        Position {
            paragraph,
            offset: pos.offset.min(
                self.paragraphs
                    .get(paragraph)
                    .map(Paragraph::len)
                    .unwrap_or_default(),
            ),
        }
    }
    pub fn selected_text(&self, start: Position, end: Position) -> String {
        let (a, b) = if start <= end {
            (self.clamp_position(start), self.clamp_position(end))
        } else {
            (self.clamp_position(end), self.clamp_position(start))
        };
        let mut result = Vec::new();
        for i in a.paragraph..=b.paragraph {
            if let Some(p) = self.paragraphs.get(i) {
                let from = if i == a.paragraph { a.offset } else { 0 };
                let to = if i == b.paragraph { b.offset } else { p.len() };
                result.push(
                    p.text()
                        .chars()
                        .skip(from)
                        .take(to.saturating_sub(from))
                        .collect::<String>(),
                );
            }
        }
        result.join("\n")
    }
    pub fn insert_text(
        &mut self,
        paragraph_id: u64,
        offset: usize,
        text: &str,
    ) -> Result<Position, String> {
        let pos = self.position(paragraph_id, offset)?;
        self.replace_range(pos, pos, text, None)
    }
    pub fn replace_range(
        &mut self,
        start: Position,
        end: Position,
        text: &str,
        format: Option<TextFormat>,
    ) -> Result<Position, String> {
        let (a, b) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let first = self
            .paragraphs
            .get(a.paragraph)
            .ok_or("Начальный абзац не найден")?;
        let last = self
            .paragraphs
            .get(b.paragraph)
            .ok_or("Конечный абзац не найден")?;
        if a.offset > first.len() || b.offset > last.len() {
            return Err("Недопустимый диапазон текста".into());
        }
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();
        let remaining=self.paragraphs.len().saturating_sub(b.paragraph-a.paragraph+1);
        if remaining.checked_add(lines.len()).is_none_or(|count|count>100000) {
            return Err("Правка превышает100000 абзацев Docs".into());
        }
        let text_bytes=self.paragraphs.iter().chain(self.tables.iter()
            .flat_map(|table|table.rows.iter().flatten()).flat_map(|cell|&cell.paragraphs))
            .flat_map(|paragraph|&paragraph.runs).fold(0usize,|total,run|total.saturating_add(run.text.len()));
        let removed=self.selected_text(a,b).len().saturating_sub(b.paragraph-a.paragraph);
        let inserted=lines.iter().fold(0usize,|total,line|total.saturating_add(line.len()));
        if text_bytes.saturating_sub(removed).checked_add(inserted).is_none_or(|total|total>16*1024*1024)
            || self.next_id.max(2).checked_add(lines.len().saturating_sub(1) as u64).is_none_or(|next|next==u64::MAX) {
            return Err("Правка превышает безопасный лимит текста или ID Docs".into());
        }
        let first=first.clone();let last=last.clone();
        let fmt = format.unwrap_or_else(|| {
            // Insertion follows the character before the caret; replacement follows
            // the first deleted character, including matches at a run boundary.
            first.format_at(a.offset + usize::from(a != b))
        });
        let prefix = first.slice(0, a.offset);
        let suffix = last.slice(b.offset, last.len());
        let mut template=if normalized.is_empty() && a.offset==0 && a.paragraph<b.paragraph {last.clone()}else{first.clone()};
        template.id=first.id;template.runs.clear();
        let mut replacement = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let mut p = template.clone();
            if i > 0 {
                p.id = self.alloc_id();
                p.page_break_before = false;
                p.list_start=None;
            }
            p.runs = Vec::new();
            if i == 0 {
                p.runs.extend(prefix.clone());
            }
            p.runs.push(Run {
                text: (*line).into(),
                format: fmt.clone(),
            });
            if i + 1 == lines.len() {
                p.runs.extend(suffix.clone());
            }
            p.normalize();
            replacement.push(p);
        }
        let tail_id = replacement.last().map(|p| p.id).unwrap_or(first.id);
        let mut deleted_ids: Vec<u64> = self
            .paragraphs
            .iter()
            .skip(a.paragraph + 1)
            .take(b.paragraph.saturating_sub(a.paragraph))
            .map(|p| p.id)
            .collect();
        // Blocks after the edited paragraph follow its end (the suffix): when the
        // paragraph splits, they move to the last part instead of jumping up.
        if lines.len() > 1 && a.paragraph == b.paragraph {
            deleted_ids.push(first.id);
        }
        for table in &mut self.tables {
            if deleted_ids.contains(&table.after_paragraph_id) {
                table.after_paragraph_id = tail_id;
            }
        }
        for image in &mut self.images {
            if deleted_ids.contains(&image.after_paragraph_id) {
                image.after_paragraph_id = tail_id;
            }
        }
        let result = Position {
            paragraph: a.paragraph + lines.len().saturating_sub(1),
            offset: if lines.len() == 1 {
                a.offset + lines.first().map(|s| s.chars().count()).unwrap_or_default()
            } else {
                lines.last().map(|s| s.chars().count()).unwrap_or_default()
            },
        };
        self.paragraphs
            .splice(a.paragraph..=b.paragraph, replacement);
        Ok(result)
    }
    pub fn format_range(
        &mut self,
        start: Position,
        end: Position,
        mut apply: impl FnMut(&mut TextFormat),
    ) {
        let (a, b) = if start <= end {
            (self.clamp_position(start), self.clamp_position(end))
        } else {
            (self.clamp_position(end), self.clamp_position(start))
        };
        for i in a.paragraph..=b.paragraph {
            if let Some(p) = self.paragraphs.get_mut(i) {
                let from = if i == a.paragraph { a.offset } else { 0 };
                let to = if i == b.paragraph { b.offset } else { p.len() };
                if p.is_empty() {
                    if i < b.paragraph {
                        for run in &mut p.runs {
                            apply(&mut run.format);
                        }
                    }
                    continue;
                }
                let mut runs = p.slice(0, from);
                let mut middle = p.slice(from, to);
                for run in &mut middle {
                    apply(&mut run.format);
                }
                runs.extend(middle);
                runs.extend(p.slice(to, p.len()));
                p.runs = runs;
                p.normalize();
            }
        }
    }
    pub fn find(&self, needle: &str, case_sensitive: bool) -> Vec<(Position, Position)> {
        self.find_in(needle, case_sensitive, 0, self.paragraphs.len(), usize::MAX)
    }
    pub fn find_page(
        &self,
        needle: &str,
        case_sensitive: bool,
        start_paragraph: usize,
        count: usize,
        max_matches: usize,
    ) -> Vec<(Position, Position)> {
        self.find_in(
            needle,
            case_sensitive,
            start_paragraph,
            count.min(100),
            max_matches.min(1000),
        )
    }
    pub(crate) fn find_bounded(
        &self,
        needle: &str,
        case_sensitive: bool,
        max_matches: usize,
    ) -> Vec<(Position, Position)> {
        self.find_in(
            needle,
            case_sensitive,
            0,
            self.paragraphs.len(),
            max_matches,
        )
    }
    fn find_in(
        &self,
        needle: &str,
        case_sensitive: bool,
        start_paragraph: usize,
        count: usize,
        max_matches: usize,
    ) -> Vec<(Position, Position)> {
        if needle.is_empty() || count == 0 || max_matches == 0 {
            return Vec::new();
        }
        let pattern: Vec<char> = needle.chars().collect();
        let equal = |a: char, b: char| {
            if case_sensitive {
                a == b
            } else {
                a.to_lowercase().eq(b.to_lowercase())
            }
        };
        let mut prefix = vec![0; pattern.len()];
        let mut matched = 0;
        for (i, ch) in pattern.iter().copied().enumerate().skip(1) {
            while matched > 0 && !pattern.get(matched).is_some_and(|c| equal(ch, *c)) {
                matched = prefix.get(matched - 1).copied().unwrap_or_default();
            }
            if pattern.get(matched).is_some_and(|c| equal(ch, *c)) {
                matched += 1;
            }
            if let Some(slot) = prefix.get_mut(i) {
                *slot = matched;
            }
        }
        let mut matches = Vec::new();
        for (paragraph, p) in self
            .paragraphs
            .iter()
            .enumerate()
            .skip(start_paragraph)
            .take(count)
        {
            matched = 0;
            for (offset, ch) in p.runs.iter().flat_map(|r| r.text.chars()).enumerate() {
                while matched > 0 && !pattern.get(matched).is_some_and(|c| equal(ch, *c)) {
                    matched = prefix.get(matched - 1).copied().unwrap_or_default();
                }
                if pattern.get(matched).is_some_and(|c| equal(ch, *c)) {
                    matched += 1;
                }
                if matched == pattern.len() {
                    matches.push((
                        Position {
                            paragraph,
                            offset: offset + 1 - pattern.len(),
                        },
                        Position {
                            paragraph,
                            offset: offset + 1,
                        },
                    ));
                    if matches.len() >= max_matches {
                        return matches;
                    }
                    matched = prefix.get(matched - 1).copied().unwrap_or_default();
                }
            }
        }
        matches
    }
    pub fn replace_all(&mut self,needle:&str,replacement:&str,case_sensitive:bool)->usize {
        self.try_replace_all(needle,replacement,case_sensitive).unwrap_or(0)
    }
    /// Preflight the entire operation and build each affected paragraph only once.
    pub fn try_replace_all(&mut self,needle:&str,replacement:&str,case_sensitive:bool)->Result<usize,String> {
        self.replace_all_with_limits(needle,replacement,case_sensitive,16*1024*1024,100_000)
    }
    fn replace_all_with_limits(&mut self,needle:&str,replacement:&str,case_sensitive:bool,text_limit:usize,match_limit:usize)->Result<usize,String> {
        let mut matches=self.find_bounded(needle,case_sensitive,match_limit.saturating_add(1));
        if matches.len()>match_limit {return Err("Замена превышает 100 000 совпадений; уточните поиск".into());}
        let mut previous=None;
        matches.retain(|(start,end)| {
            if previous.is_none_or(|previous|*start>=previous) {previous=Some(*end);true}else{false}
        });
        if matches.is_empty(){return Ok(0);}
        let replacement=replacement.replace("\r\n","\n").replace('\r',"\n");
        let count=matches.len();
        let extra_lines=replacement.chars().filter(|ch|*ch=='\n').count().checked_mul(count)
            .and_then(|lines|lines.checked_add(self.paragraphs.len()));
        if extra_lines.is_none_or(|lines|lines>100000) {return Err("Замена превышает предел числа абзацев".into());}
        let mut grouped=std::collections::BTreeMap::<usize,Vec<(usize,usize)>>::new();
        for (start,end) in matches {grouped.entry(start.paragraph).or_default().push((start.offset,end.offset));}
        let bytes=self.paragraphs.iter().chain(self.tables.iter().flat_map(|table|table.rows.iter().flatten())
                .flat_map(|cell|cell.paragraphs.iter()))
            .flat_map(|paragraph|paragraph.runs.iter()).map(|run|run.text.len()).sum::<usize>();
        let mut removed=0usize;
        for (index,ranges) in &grouped {
            let mut range=0;
            for (offset,ch) in self.paragraphs[*index].runs.iter().flat_map(|run|run.text.chars()).enumerate() {
                while ranges.get(range).is_some_and(|(_,end)|offset>=*end) {range+=1;}
                if ranges.get(range).is_some_and(|(start,end)|offset>=*start && offset<*end) {removed+=ch.len_utf8();}
            }
        }
        let projected=replacement.len().checked_mul(count)
            .and_then(|added|bytes.saturating_sub(removed).checked_add(added));
        if projected.is_none_or(|bytes|bytes>text_limit) {return Err("Замена превышает лимит текста Docs".into());}
        let mut candidate=self.clone();
        let originals=std::mem::take(&mut candidate.paragraphs);
        for (index,original) in originals.into_iter().enumerate() {
            let Some(ranges)=grouped.get(&index) else {candidate.paragraphs.push(original);continue;};
            let mut template=original.clone();template.runs.clear();
            let mut current=template.clone();
            let mut source=original.runs.iter().flat_map(|run|
                run.text.chars().map(move|ch|(ch,&run.format))).peekable();
            let mut offset=0;
            for (start,end) in ranges {
                while offset<*start {
                    if let Some((ch,format))=source.next() {push_replacement_char(&mut current,ch,format);}
                    offset+=1;
                }
                let format=source.peek().map(|(_,format)|(**format).clone()).unwrap_or_default();
                for _ in *start..*end {source.next();offset+=1;}
                if current.runs.is_empty() {current.runs.push(Run{text:String::new(),format:format.clone()});}
                for ch in replacement.chars() {
                    if ch=='\n' {
                        current.normalize();candidate.paragraphs.push(current);
                        current=template.clone();current.id=candidate.alloc_id();current.page_break_before=false;
                        current.runs.push(Run{text:String::new(),format:format.clone()});
                    }else{push_replacement_char(&mut current,ch,&format);}
                }
            }
            for (ch,format) in source {push_replacement_char(&mut current,ch,format);}
            current.normalize();
            let last=current.id;
            candidate.paragraphs.push(current);
            if last!=original.id {
                for table in &mut candidate.tables {if table.after_paragraph_id==original.id {table.after_paragraph_id=last;}}
                for image in &mut candidate.images {if image.after_paragraph_id==original.id {image.after_paragraph_id=last;}}
            }
        }
        candidate.validate()?;
        *self=candidate;Ok(count)
    }
    pub fn add_table(
        &mut self,
        after_paragraph_id: u64,
        rows: usize,
        columns: usize,
    ) -> Result<u64, String> {
        if rows == 0 || columns == 0 || rows > 1000 || columns > 64 {
            return Err("Размер таблицы: 1–1000 строк, 1–64 столбца".into());
        }
        if !self.paragraphs.iter().any(|p| p.id == after_paragraph_id) {
            return Err("Абзац не найден".into());
        }
        let id = self.alloc_id();
        let mut cells = Vec::new();
        for _ in 0..rows {
            let mut row = Vec::new();
            for _ in 0..columns {
                let cell_id = self.alloc_id();
                let paragraph_id = self.alloc_id();
                row.push(Cell::new(
                    cell_id,
                    vec![Paragraph {
                        id: paragraph_id,
                        ..Paragraph::default()
                    }],
                    [255, 255, 255],
                ));
            }
            cells.push(row);
        }
        let width =
            (self.page.width - self.page.margin_left - self.page.margin_right) / columns as f32;
        self.tables.push(Table {
            id,
            after_paragraph_id,
            rows: cells,
            column_widths: vec![width; columns],
            border_color: [100, 110, 120],
            borderless: false,
            cell_margin: WORD_CELL_MARGIN,
            row_heights: Vec::new(),
            header_rows: 0,
        });
        Ok(id)
    }
    /// Applies a row/column edit. Returns `Ok(false)` when the last row or
    /// column was deleted and the whole table was therefore removed.
    pub fn edit_table(&mut self, table_id: u64, edit: TableEdit) -> Result<bool, String> {
        let mut candidate = self.clone();
        let retained = candidate.edit_table_inner(table_id, edit)?;
        candidate.validate()?;
        *self = candidate;
        Ok(retained)
    }
    fn edit_table_inner(&mut self, table_id: u64, edit: TableEdit) -> Result<bool, String> {
        let usable = self.page.width - self.page.margin_left - self.page.margin_right;
        let index = self
            .tables
            .iter()
            .position(|t| t.id == table_id)
            .ok_or("not_found: таблица не найдена")?;
        let mut next = self.next_id;
        let mut remove = false;
        let table = self
            .tables
            .get_mut(index)
            .ok_or("not_found: таблица не найдена")?;
        let columns = table.grid_columns().max(1);
        // Imported tables may be ragged; editing makes them rectangular.
        for row in &mut table.rows {
            while row.iter().map(Cell::span).sum::<usize>() < columns {
                row.push(empty_cell(&mut next, [255, 255, 255]));
            }
        }
        if table.column_widths.len() < columns {
            table.column_widths.resize(columns, usable / columns as f32);
        }
        table.column_widths.truncate(columns);
        match edit {
            TableEdit::InsertRow(at) => {
                if table.rows.len() >= 1000 {
                    return Err("too_large: не более 1000 строк".into());
                }
                let at = at.min(table.rows.len());
                let fills: Vec<[u8; 3]> = table
                    .rows
                    .get(at.saturating_sub(1))
                    .map(|row| row.iter().map(|c| c.fill).collect())
                    .unwrap_or_default();
                // The new row copies the merged-cell layout of the row above.
                let spans: Vec<u16> = table
                    .rows
                    .get(at.saturating_sub(1))
                    .map(|row| row.iter().map(|c| c.span.max(1)).collect())
                    .unwrap_or_else(|| vec![1; columns]);
                let following = table.rows.get(at).cloned();
                let mut grid = 0;
                let row = spans
                    .iter()
                    .enumerate()
                    .map(|(i, span)| {
                        let mut cell =
                            empty_cell(&mut next, fills.get(i).copied().unwrap_or([255; 3]));
                        cell.span = *span;
                        if at > 0
                            && let Some(following) = &following
                        {
                            let mut start = 0;
                            cell.merged = following
                                .iter()
                                .find_map(|below| {
                                    let matches = start == grid;
                                    start += below.span();
                                    matches.then_some(
                                        below.merged && below.span() == usize::from(*span),
                                    )
                                })
                                .unwrap_or(false);
                        }
                        grid += usize::from(*span);
                        cell
                    })
                    .collect();
                table.rows.insert(at, row);
                if at < table.header_rows {
                    table.header_rows += 1;
                }
                if at <= table.row_heights.len() {
                    table.row_heights.insert(at, 0.0);
                }
            }
            TableEdit::InsertColumn(at) => {
                if columns >= 64 {
                    return Err("too_large: не более 64 столбцов".into());
                }
                let at = at.min(columns);
                for row in &mut table.rows {
                    // `at` is a grid column: split points inside a merged cell widen it.
                    let mut start = 0;
                    let mut index = row.len();
                    let mut inside = None;
                    for (i, cell) in row.iter().enumerate() {
                        if start == at {
                            index = i;
                            break;
                        }
                        if at < start + cell.span() {
                            inside = Some(i);
                            break;
                        }
                        start += cell.span();
                    }
                    if let Some(cell) = inside.and_then(|i| row.get_mut(i)) {
                        cell.span = cell.span.saturating_add(1);
                        continue;
                    }
                    let fill = row
                        .get(index.saturating_sub(1))
                        .map(|c| c.fill)
                        .unwrap_or([255; 3]);
                    row.insert(index, empty_cell(&mut next, fill));
                }
                let width = table
                    .column_widths
                    .get(at.saturating_sub(1))
                    .copied()
                    .unwrap_or(usable / columns as f32);
                table.column_widths.insert(at, width);
            }
            TableEdit::DeleteRow(at) => {
                if at >= table.rows.len() {
                    return Err("invalid_range: строка не найдена".into());
                }
                if table.rows.len() == 1 {
                    remove = true;
                } else {
                    let deleted = table.rows.remove(at);
                    if let Some(next_row) = table.rows.get_mut(at) {
                        let mut heads = Vec::new();
                        let mut column = 0;
                        for cell in &deleted {
                            if !cell.merged { heads.push((column, cell.span())); }
                            column += cell.span();
                        }
                        column = 0;
                        for cell in next_row {
                            if cell.merged && heads.contains(&(column, cell.span())) {
                                cell.merged = false;
                            }
                            column += cell.span();
                        }
                    }
                    if at < table.header_rows {
                        table.header_rows -= 1;
                    }
                    if at < table.row_heights.len() {
                        table.row_heights.remove(at);
                    }
                }
            }
            TableEdit::DeleteColumn(at) => {
                if at >= columns {
                    return Err("invalid_range: столбец не найден".into());
                }
                if columns == 1 {
                    remove = true;
                } else {
                    for row in &mut table.rows {
                        let mut start = 0;
                        let mut hit = None;
                        for (i, cell) in row.iter().enumerate() {
                            if at < start + cell.span() {
                                hit = Some(i);
                                break;
                            }
                            start += cell.span();
                        }
                        if let Some(i) = hit {
                            if let Some(cell) = row.get_mut(i).filter(|c| c.span > 1) {
                                cell.span -= 1;
                            } else {
                                row.remove(i);
                            }
                        }
                    }
                    table.rows.retain(|row| !row.is_empty());
                    if at < table.column_widths.len() {
                        table.column_widths.remove(at);
                    }
                }
            }
        }
        if let Some(first) = table.rows.first_mut() {
            for cell in first {
                cell.merged = false;
            }
        }
        if remove {
            self.tables.remove(index);
        }
        self.next_id = next;
        Ok(!remove)
    }
    /// Removes a page that holds no text, tables or images: its empty paragraphs and
    /// the page breaks that create it. Returns `false` when the page has content.
    pub fn delete_empty_page(&mut self, page: usize) -> bool {
        let layout = self.layout();
        if layout.pages.len() < 2 || page >= layout.pages.len() {
            return false;
        }
        if layout.cells.iter().any(|c| c.page == page)
            || layout.pages.get(page).is_some_and(|p| {
                p.items
                    .iter()
                    .any(|item| matches!(item, Primitive::Image { .. }))
            })
        {
            return false;
        }
        let blank = |text: &str| text.chars().all(char::is_whitespace);
        let on_page: Vec<usize> = {
            let mut v: Vec<usize> = layout
                .rows
                .iter()
                .filter(|r| r.page == page)
                .map(|r| r.paragraph)
                .collect();
            v.dedup();
            v
        };
        // Text on the page means it is not empty.
        if layout
            .rows
            .iter()
            .filter(|r| r.page == page)
            .any(|r| r.segments.iter().any(|s| !blank(&s.text)))
        {
            return false;
        }
        let anchors = |id: u64, doc: &Doc| {
            doc.tables.iter().any(|t| t.after_paragraph_id == id)
                || doc.images.iter().any(|i| i.after_paragraph_id == id)
        };
        let mut changed = false;
        // Remove only the hard break belonging to this blank page, including internal ones.
        for &pi in &on_page {
            if let Some(p) = self.paragraphs.get_mut(pi) {
                let chars: Vec<char> = p.text().chars().collect();
                let mut spans = layout
                    .rows
                    .iter()
                    .filter(|row| row.page == page && row.paragraph == pi)
                    .filter_map(|row| {
                        if chars.get(row.end) == Some(&'\u{000c}') {
                            Some((row.start, row.end + 1))
                        } else if row.start > 0 && chars.get(row.start - 1) == Some(&'\u{000c}') {
                            Some((row.start - 1, row.end))
                        } else {
                            (row.end>row.start).then_some((row.start,row.end))
                        }
                    })
                    .collect::<Vec<_>>();
                spans.sort_unstable();
                let mut merged:Vec<(usize,usize)>=Vec::new();
                for (from,to) in spans {
                    if let Some(last)=merged.last_mut().filter(|last|from<=last.1) {last.1=last.1.max(to);}
                    else {merged.push((from,to));}
                }
                let spans=merged;
                for (from, to) in spans.into_iter().rev() {
                    let format = p.format_at(from);
                    let mut runs = p.slice(0, from);
                    runs.extend(p.slice(to, p.len()));
                    if runs.is_empty() {
                        runs.push(Run {
                            text: String::new(),
                            format,
                        });
                    }
                    p.runs = runs;
                    p.normalize();
                    changed = true;
                }
            }
        }
        // Paragraphs that start on the page and are blank are removed entirely.
        let first_rows: Vec<usize> = on_page
            .iter()
            .copied()
            .filter(|pi| {
                layout
                    .rows
                    .iter()
                    .find(|r| r.paragraph == *pi)
                    .is_some_and(|r| r.page == page)
            })
            .collect();
        let mut remove = Vec::new();
        for &pi in &first_rows {
            if let Some(p) = self.paragraphs.get(pi)
                && blank(&p.text())
                && !anchors(p.id, self)
            {
                remove.push(pi);
            }
        }
        // Keep one paragraph; tables before the page stay anchored to a live paragraph.
        if remove.len() >= self.paragraphs.len() {
            remove.pop();
        }
        for pi in remove.into_iter().rev() {
            self.paragraphs.remove(pi);
            changed = true;
        }
        if changed {
            self.normalize();
        }
        changed
    }
    pub fn delete_table(&mut self, table_id: u64) -> Result<(), String> {
        let before = self.tables.len();
        self.tables.retain(|t| t.id != table_id);
        if self.tables.len() == before {
            return Err("not_found: таблица не найдена".into());
        }
        Ok(())
    }
    pub fn table_cell_mut(
        &mut self,
        table_id: u64,
        row: usize,
        column: usize,
    ) -> Option<&mut Cell> {
        self.tables
            .iter_mut()
            .find(|t| t.id == table_id)?
            .rows
            .get_mut(row)?
            .get_mut(column)
    }
    /// Replaces the text of a cell, keeping formats and IDs of unchanged parts.
    pub fn set_cell_text(
        &mut self,
        table_id: u64,
        row: usize,
        column: usize,
        text: &str,
    ) -> Result<u64, String> {
        let mut next = self.next_id;
        let cell = self
            .table_cell_mut(table_id, row, column)
            .ok_or("invalid_range: ячейка не найдена")?;
        try_edit_cell_text(cell, text, &mut next)?;
        let id = cell.id;
        self.next_id = next;
        Ok(id)
    }
    pub fn outline(&self) -> Vec<(u64, String, String)> {
        self.paragraphs
            .iter()
            .filter(|p| heading_level(&p.style).is_some() || p.style == "Title")
            .map(|p| (p.id, p.style.clone(), p.text()))
            .collect()
    }
    fn has_blocks_after(&self,id:u64)->bool {
        self.tables.iter().any(|table|table.after_paragraph_id==id) || self.images.iter().any(|image|image.after_paragraph_id==id)
    }
    pub fn pages(&self) -> Vec<Page> {
        self.layout().pages
    }
    pub fn layout(&self) -> Layout {
        let mut doc = self.clone();
        doc.normalize();
        let setup = &doc.page;
        let mut layout = Layout {
            pages: vec![blank_page(setup, 1)],
            rows: Vec::new(),
            cells: Vec::new(),
        };
        let bottom = setup.height - setup.margin_bottom;
        let body = bottom - setup.margin_top;
        let mut y = setup.margin_top;
        let mut list_numbers = [0_usize; 9];
        for (pi, p) in doc.paragraphs.iter().enumerate() {
            let prev = pi.checked_sub(1).and_then(|i| doc.paragraphs.get(i));
            let next = doc.paragraphs.get(pi + 1);
            let adjacent_prev=prev.filter(|previous|!doc.has_blocks_after(previous.id));
            let adjacent_next=next.filter(|_|!doc.has_blocks_after(p.id));
            let (before, after) = paragraph_spacing(p, adjacent_prev, adjacent_next);
            if p.page_break_before && (y > setup.margin_top || pi > 0) {
                next_page(&mut layout, setup, &mut y);
            }
            let marker = list_marker(p, &mut list_numbers);
            let indent = p.indent + list_indent(p);
            let base_x = setup.margin_left + indent;
            let available = (setup.width - setup.margin_right - base_x - p.right_indent).max(12.0);
            let chars: Vec<(char, TextFormat)> = p
                .runs
                .iter()
                .flat_map(|r| r.text.chars().map(move |ch| (ch, r.format.clone())))
                .collect();
            let lines = wrap_lines(p, &chars, available, indent, 12.0);
            let heights: Vec<f32> = lines.iter().map(|l| l.height).collect();
            // Manual page breaks inside the paragraph are placed as they come.
            let forced = chars.iter().any(|(ch, _)| *ch == '\u{000c}');
            let mut breaks = if forced {
                Vec::new()
            } else {
                line_breaks(
                    &heights,
                    y + before,
                    setup.margin_top,
                    bottom,
                    p.widow_control,
                )
            };
            if y > setup.margin_top + 0.5 && !forced {
                let total: f32 = heights.iter().sum();
                // Keep lines together: the whole paragraph moves to the next page.
                if p.keep_lines && !breaks.is_empty() && total <= body {
                    breaks = vec![0];
                }
                // Keep with next: this paragraph and the first line of the next share a page.
                if p.keep_next && breaks.is_empty()
                    && let Some(need)=keep_group_height(&doc,pi,before+total+after,body)
                    && y+need>bottom && need<=body {
                    breaks=vec![0];
                }
            }
            if breaks.first() == Some(&0) {
                // Word drops the space before a paragraph that moved to the top of a page.
                next_page(&mut layout, setup, &mut y);
            } else {
                y += before;
            }
            let count = lines.len();
            for (li, line) in lines.iter().enumerate() {
                let (start, end, height) = (line.start, line.end, line.height);
                if (li > 0 && breaks.contains(&li))
                    || (y + height > bottom && y > setup.margin_top + 0.5)
                {
                    next_page(&mut layout, setup, &mut y);
                }
                let x = base_x + line.indent;
                let max_width = (available - line.indent).max(12.0);
                let visible = line.width - trailing_spaces(&chars, start, end);
                let align_x = match p.align {
                    Align::Center => x + (max_width - visible).max(0.0) / 2.0,
                    Align::Right => x + (max_width - visible).max(0.0),
                    _ => x,
                };
                // Justified text stretches every line but the last one.
                let justify = if p.align == Align::Justify && li + 1 < count {
                    justify_gap(&chars, start, end, max_width - visible)
                } else {
                    0.0
                };
                let segments = line_segments(
                    &chars,
                    start,
                    end,
                    align_x,
                    indent + line.indent,
                    indent,
                    justify,
                    &p.format_at(start),
                );
                let row_marker = if li == 0 {
                    marker.clone()
                } else {
                    String::new()
                };
                let page = layout.pages.len().saturating_sub(1);
                let row = Row {
                    paragraph: pi,
                    start,
                    end,
                    x: align_x,
                    y,
                    height,
                    page,
                    segments,
                    marker: row_marker,
                };
                if let Some(page) = layout.pages.last_mut() {
                    render_row(page, &row);
                }
                layout.rows.push(row);
                y += height;
                if line.hard_break && chars.get(end).is_some_and(|(ch, _)| *ch == '\u{000c}') {
                    next_page(&mut layout, setup, &mut y);
                }
            }
            y += after;
            for table in doc.tables.iter().filter(|t| t.after_paragraph_id == p.id) {
                render_table(&mut layout, setup, table, &mut y);
            }
            for image in doc.images.iter().filter(|im| im.after_paragraph_id == p.id) {
                let max_w = setup.width - setup.margin_left - setup.margin_right;
                let max_h = setup.height - setup.margin_top - setup.margin_bottom;
                let scale = (max_w / image.width)
                    .min(max_h / image.height)
                    .min(1.0);
                let width = image.width * scale;
                let height = image.height * scale;
                if y + height > setup.height - setup.margin_bottom {
                    next_page(&mut layout, setup, &mut y);
                }
                if let Some(page) = layout.pages.last_mut() {
                    page.items.push(Primitive::Image {
                        x: setup.margin_left,
                        y,
                        w: width,
                        h: height,
                        bytes: image.bytes.clone(),
                    });
                }
                y += height + 8.0;
            }
        }
        layout
    }
}
fn unique_id(id: u64, ids: &mut std::collections::HashSet<u64>) -> Result<(), String> {
    if id == 0 || id == u64::MAX || !ids.insert(id) {
        return Err("Пустой, исчерпанный или повторяющийся ID Docs".into());
    }
    Ok(())
}
fn validate_paragraph(
    p: &Paragraph,
    ids: &mut std::collections::HashSet<u64>,
    text_size: &mut usize,
    runs: &mut usize,
) -> Result<(), String> {
    unique_id(p.id, ids)?;
    if p.runs.is_empty()
        || p.runs.len() > 100000
        || p.style.len() > 256
        || p.list_level > 8
        || p.list_start.is_some_and(|value|value>999_999_999)
        || [
            p.indent,
            p.right_indent,
            p.first_indent,
            p.spacing_before,
            p.spacing_after,
            p.line_spacing,
            p.line_points,
        ]
        .iter()
        .any(|v| !v.is_finite())
        || !(0.0..=1584.0).contains(&p.line_points)
        || p.indent.abs() > 10000.0
        || p.right_indent.abs() > 10000.0
        || p.first_indent.abs() > 10000.0
        || !(0.0..=1000.0).contains(&p.spacing_before)
        || !(0.0..=1000.0).contains(&p.spacing_after)
        || !(0.5..=5.0).contains(&p.line_spacing)
    {
        return Err("Недопустимое оформление абзаца".into());
    }
    if !xml_model_text(&p.style,false) {return Err("Недопустимый control character в style".into());}
    for run in &p.runs {
        if !xml_model_text(&run.text,true) || run.format.font.chars().any(char::is_control)
            || run.format.link.as_ref().is_some_and(|link|!xml_model_text(link,false)) {
            return Err("Docs text/font/link содержит недопустимые control characters".into());
        }
        *runs += 1;
        *text_size = text_size
            .checked_add(run.text.len())
            .ok_or("Переполнение текста")?;
        if *runs > 1_000_000 || *text_size > 16 * 1024 * 1024 {
            return Err("Текст Docs превышает безопасный лимит".into());
        }
        let f = &run.format;
        if f.font.len() > 256
            || !f.size.is_finite()
            || !(4.0..=144.0).contains(&f.size)
            || !(-1..=1).contains(&f.script)
            || f.link.as_ref().is_some_and(|s| s.len() > 8192)
        {
            return Err("Недопустимое оформление текста".into());
        }
    }
    Ok(())
}
fn assign_unique_id(id: &mut u64, next: &mut u64, seen: &mut std::collections::HashSet<u64>) {
    if *id == 0 || !seen.insert(*id) {
        while seen.contains(next) {
            *next = next.checked_add(1).unwrap_or(2);
        }
        *id = *next;
        seen.insert(*id);
        *next = next.checked_add(1).unwrap_or(2);
    }
    *next = (*next).max(id.checked_add(1).unwrap_or(2));
}
fn blank_page(setup: &PageSetup, number: usize) -> Page {
    let mut items = Vec::new();
    if !setup.header.is_empty() {
        items.push(Primitive::Text {
            x: setup.margin_left,
            y: 24.0,
            text: setup.header.clone(),
            size: 9.0,
            color: [80, 80, 80],
        });
    }
    if !setup.footer.is_empty() {
        items.push(Primitive::Text {
            x: setup.margin_left,
            y: setup.height - 30.0,
            text: setup.footer.clone(),
            size: 9.0,
            color: [80, 80, 80],
        });
    }
    if setup.page_numbers {
        items.push(Primitive::Text {
            x: setup.width / 2.0,
            y: setup.height - 30.0,
            text: number.to_string(),
            size: 9.0,
            color: [80, 80, 80],
        });
    }
    Page {
        width: setup.width,
        height: setup.height,
        items,
    }
}
fn next_page(layout: &mut Layout, setup: &PageSetup, y: &mut f32) {
    layout.pages.push(blank_page(setup, layout.pages.len() + 1));
    *y = setup.margin_top;
}
fn render_row(page: &mut Page, row: &Row) {
    if !row.marker.is_empty() {
        let format = row
            .segments
            .first()
            .map(|segment| segment.format.clone())
            .unwrap_or_default();
        page.items.push(Primitive::TextStyled {
            x: row.x - 4.0 - row.marker.chars().map(|ch|char_width(ch,&format)).sum::<f32>(),
            y: row.y,
            text: row.marker.clone(),
            size: format.size,
            color: format.color,
            font: format.font,
            bold: format.bold,
            italic: format.italic,
        });
    }
    for seg in &row.segments {
        let size = seg.format.size * if seg.format.script != 0 { 0.75 } else { 1.0 };
        let glyph_y = row.y
            + if seg.format.script > 0 {
                -3.0
            } else if seg.format.script < 0 {
                3.0
            } else {
                0.0
            };
        if let Some(color) = seg.format.highlight {
            page.items.push(Primitive::Rect {
                x: seg.x,
                y: row.y,
                w: seg.width,
                h: row.height,
                color,
            });
        }
        // A tab only moves the text after it; underline still covers it below.
        if seg.text != "\t" {
            page.items.push(Primitive::TextStyled {
                x: seg.x,
                y: glyph_y,
                text: seg.text.chars().filter(|ch|!matches!(*ch,'\u{00ad}'|'\u{200b}')).collect(),
                size,
                color: seg.format.color,
                bold: seg.format.bold,
                italic: seg.format.italic,
                font: seg.format.font.clone(),
            });
        }
        if seg.format.underline || seg.format.link.is_some() {
            page.items.push(Primitive::Line {
                x1: seg.x,
                y1: glyph_y + size * 1.13,
                x2: seg.x + seg.width,
                y2: glyph_y + size * 1.13,
                width: 0.7,
                color: seg.format.color,
            });
        }
        if seg.format.strike {
            page.items.push(Primitive::Line {
                x1: seg.x,
                y1: glyph_y + size * 0.6,
                x2: seg.x + seg.width,
                y2: glyph_y + size * 0.6,
                width: 0.7,
                color: seg.format.color,
            });
        }
    }
}
/// Width of the spaces ending `chars[start..end]`; they don't count for alignment.
fn trailing_spaces(chars: &[(char, TextFormat)], start: usize, end: usize) -> f32 {
    chars
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .rev()
        .take_while(|(ch, _)| *ch == ' ')
        .map(|(ch, f)| char_width(*ch, f))
        .sum()
}
/// Word's default tab stops: every 1.27 cm (`w:defaultTabStop` 720 twips).
pub const DEFAULT_TAB_STOP: f32 = 36.0;
/// Space before and after a paragraph; contextual spacing drops it between paragraphs of
/// the same style (list items, table text).
fn paragraph_spacing(
    p: &Paragraph,
    prev: Option<&Paragraph>,
    next: Option<&Paragraph>,
) -> (f32, f32) {
    let same = |other: Option<&Paragraph>| {
        p.contextual_spacing && other.is_some_and(|q| q.style == p.style)
    };
    let before=if same(prev) {0.0}else{p.spacing_before.max(0.0)};
    let previous_after=prev.map(|previous| {
        if previous.contextual_spacing && previous.style==p.style {0.0}else{previous.spacing_after.max(0.0)}
    }).unwrap_or(0.0);
    ((before-previous_after).max(0.0),if same(next) {0.0}else{p.spacing_after.max(0.0)})
}
/// Advance of a character `pos` points right of the text area's left edge. A tab jumps to
/// the hanging indent `hang` when that is ahead, otherwise to the next default tab stop.
fn advance(ch: char, format: &TextFormat, pos: f32, hang: f32) -> f32 {
    if ch != '\t' {
        return char_width(ch, format);
    }
    if pos + 0.5 < hang {
        return hang - pos;
    }
    let next = ((pos + 0.5) / DEFAULT_TAB_STOP).floor() * DEFAULT_TAB_STOP + DEFAULT_TAB_STOP;
    (next - pos).max(0.0)
}
fn line_width(
    chars: &[(char, TextFormat)],
    start: usize,
    end: usize,
    origin: f32,
    hang: f32,
) -> f32 {
    let mut pos = origin;
    for (ch, format) in chars.get(start..end).unwrap_or_default() {
        pos += advance(*ch, format, pos, hang);
    }
    pos - origin
}
/// A wrapped line of a paragraph before it is placed on a page.
struct Line {
    start: usize,
    end: usize,
    /// Width including trailing spaces.
    width: f32,
    height: f32,
    /// First-line indent applied to this line.
    indent: f32,
    hard_break: bool,
}
/// Breaks a paragraph into lines at most `available` wide. `origin` is the paragraph's left
/// indent relative to the text area, where tab stops are measured from.
fn wrap_lines(
    p: &Paragraph,
    chars: &[(char, TextFormat)],
    available: f32,
    origin: f32,
    floor: f32,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        let indent = if lines.is_empty() {
            p.first_indent
        } else {
            0.0
        };
        let max_width = (available - indent).max(floor);
        let line_origin = origin + indent;
        let mut end = start;
        let mut width = 0.0;
        let mut break_at = None;
        let mut hard_break = false;
        while let Some((ch, format)) = chars.get(end) {
            if matches!(*ch, '\u{000b}' | '\u{000c}' | '\n') {
                hard_break = true;
                break;
            }
            let w = advance(*ch, format, line_origin + width, origin);
            // Like Word, spaces may hang past the margin instead of wrapping a word that fits.
            if width + w > max_width && end > start && *ch != ' ' {
                break;
            }
            width += w;
            end += 1;
            if ch.is_whitespace() && !matches!(*ch, '\u{00a0}' | '\u{202f}') {
                break_at = Some(end);
            }
        }
        if end < chars.len()
            && !hard_break
            && let Some(word_end) = break_at.filter(|n| *n > start)
        {
            end = word_end;
            width = line_width(chars, start, end, line_origin, origin);
        }
        let format=chars.get(start).map(|(_,format)|format.clone()).unwrap_or_else(||p.format_at(start));
        let max_size = chars
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .map(|(_, f)| f.size)
            .fold(format.size.max(4.0), f32::max);
        lines.push(Line {
            start,
            end,
            width,
            height: paragraph_line_height(p, &format, max_size),
            indent,
            hard_break,
        });
        start = end + usize::from(hard_break);
        if start >= chars.len() && !hard_break {
            break;
        }
    }
    lines
}
/// Extra width for each inner space of a justified line.
fn justify_gap(chars: &[(char, TextFormat)], start: usize, end: usize, extra: f32) -> f32 {
    let line = chars.get(start..end).unwrap_or_default();
    let trailing = line.iter().rev().take_while(|(ch, _)| *ch == ' ').count();
    let spaces = line
        .iter()
        .take(line.len() - trailing)
        .filter(|(ch, _)| *ch == ' ')
        .count();
    if spaces == 0 || extra <= 0.0 {
        0.0
    } else {
        extra / spaces as f32
    }
}
/// Segments of one line starting at `x`; `origin` is the line start relative to the text
/// area (for tab stops). Tabs and widened justified spaces get their own segments.
fn line_segments(
    chars: &[(char, TextFormat)],
    start: usize,
    end: usize,
    x: f32,
    origin: f32,
    hang: f32,
    justify: f32,
    empty_format:&TextFormat,
) -> Vec<Segment> {
    let line = chars.get(start..end).unwrap_or_default();
    if line.is_empty() {return vec![Segment{text:String::new(),format:empty_format.clone(),x,width:0.0}];}
    let content = line.len() - line.iter().rev().take_while(|(ch, _)| *ch == ' ').count();
    let mut segments: Vec<Segment> = Vec::new();
    let mut cursor = x;
    let mut pos = origin;
    let mut alone = true;
    for (i, (ch, format)) in line.iter().enumerate() {
        let mut width = advance(*ch, format, pos, hang);
        let wide = justify > 0.0 && *ch == ' ' && i < content;
        if wide {
            width += justify;
        }
        pos += width;
        let solo = *ch == '\t' || wide;
        if !solo
            && !alone
            && let Some(segment) = segments.last_mut().filter(|s| s.format == *format)
        {
            segment.text.push(*ch);
            segment.width += width;
        } else {
            segments.push(Segment {
                text: ch.to_string(),
                format: format.clone(),
                x: cursor,
                width,
            });
        }
        alone = solo;
        cursor += width;
    }
    segments
}
/// Lines of a paragraph that start a new page when its first line starts at `start`
/// (`0` moves the whole paragraph). With widow control a page never ends with only the
/// first line or starts with only the last line of a paragraph.
fn line_breaks(heights: &[f32], start: f32, top: f32, bottom: f32, widow: bool) -> Vec<usize> {
    let count = heights.len();
    let mut breaks = Vec::new();
    let mut y = start;
    let mut page_first = 0;
    let mut i = 0;
    while let Some(&height) = heights.get(i) {
        if y + height > bottom && y > top + 0.5 {
            let mut at = i;
            if widow && count >= 2 {
                if at == 1 && page_first == 0 && start > top + 0.5 {
                    at = 0;
                } else if at + 1 == count && at >= 2 && at - 1 > page_first {
                    at -= 1;
                }
            }
            breaks.push(at);
            page_first = at;
            i = at;
            y = top;
            continue;
        }
        y += height;
        i += 1;
    }
    breaks
}
fn list_indent(p: &Paragraph) -> f32 {
    if p.list == List::None {
        0.0
    } else {
        (f32::from(p.list_level) + 1.0) * 18.0
    }
}
pub(super) fn list_marker(p: &Paragraph, numbers: &mut [usize; 9]) -> String {
    let level = usize::from(p.list_level).min(8);
    match p.list {
        List::None => {
            numbers.fill(0);
            String::new()
        }
        List::Bullet => {
            numbers[level..].fill(0);
            "•".into()
        }
        List::Number => {
            numbers[level] = p.list_start.map(|start|start as usize).unwrap_or_else(||numbers[level].saturating_add(1));
            numbers[level + 1..].fill(0);
            format!("{}.", numbers[level])
        }
    }
}
fn table_lines(cell: &Cell, width: f32, margin: [f32; 4]) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut list_numbers = [0_usize; 9];
    for (paragraph, p) in cell.paragraphs.iter().enumerate() {
        let marker = list_marker(p, &mut list_numbers);
        let indent = p.indent + list_indent(p);
        let available = (width - margin[1] - margin[3] - indent - p.right_indent).max(1.0);
        let chars: Vec<(char, TextFormat)> = p
            .runs
            .iter()
            .flat_map(|r| r.text.chars().map(move |ch| (ch, r.format.clone())))
            .collect();
        let (spacing_before, spacing_after) = paragraph_spacing(
            p,
            paragraph
                .checked_sub(1)
                .and_then(|i| cell.paragraphs.get(i)),
            cell.paragraphs.get(paragraph + 1),
        );
        let lines = wrap_lines(p, &chars, available, indent, 1.0);
        let count = lines.len();
        for (li, line) in lines.iter().enumerate() {
            let max_width = (available - line.indent).max(1.0);
            let visible = line.width - trailing_spaces(&chars, line.start, line.end);
            let before = if li == 0 { spacing_before } else { 0.0 };
            let after = if li + 1 == count { spacing_after } else { 0.0 };
            let x = margin[3]
                + indent
                + line.indent
                + match p.align {
                    Align::Center => (max_width - visible).max(0.0) / 2.0,
                    Align::Right => (max_width - visible).max(0.0),
                    _ => 0.0,
                };
            let justify = if p.align == Align::Justify && li + 1 < count {
                justify_gap(&chars, line.start, line.end, max_width - visible)
            } else {
                0.0
            };
            rows.push(Row {
                paragraph,
                start: line.start,
                end: line.end,
                x,
                y: before,
                height: before + line.height + after,
                page: 0,
                segments: line_segments(
                    &chars,
                    line.start,
                    line.end,
                    x,
                    indent + line.indent,
                    indent,
                    justify,
                    &p.format_at(line.start),
                ),
                marker: if li == 0 {
                    marker.clone()
                } else {
                    String::new()
                },
            });
        }
    }
    rows
}
/// Column widths and cell padding of a table on the page.
struct TableGeometry {
    widths: Vec<f32>,
    margin: [f32; 4],
}
fn render_table(layout: &mut Layout, setup: &PageSetup, table: &Table, y: &mut f32) {
    let usable_width = setup.width - setup.margin_left - setup.margin_right;
    let columns = table.grid_columns();
    if columns == 0 {
        return;
    }
    let mut widths: Vec<f32> = (0..columns)
        .map(|i| {
            table
                .column_widths
                .get(i)
                .copied()
                .unwrap_or(usable_width / columns as f32)
                .max(0.05)
        })
        .collect();
    let raw_sum: f32 = widths.iter().sum();
    // Word lets tables run into the margins a little; only clearly oversized ones shrink.
    let limit = usable_width + setup.margin_right.min(36.0);
    if raw_sum > limit {
        for width in &mut widths {
            *width *= limit / raw_sum;
        }
    }
    let geometry = TableGeometry {
        widths,
        margin: table.cell_margin.map(|m| m.clamp(0.0, 72.0)),
    };
    let headers = table.header_rows.min(table.rows.len());
    for row_index in 0..table.rows.len() {
        table_row(
            layout,
            setup,
            table,
            &geometry,
            row_index,
            y,
            row_index >= headers,
        );
    }
    *y += 8.0;
}
/// Starts a new page inside a table; `repeat` draws the table's header rows at its top.
fn table_page(
    layout: &mut Layout,
    setup: &PageSetup,
    table: &Table,
    geometry: &TableGeometry,
    y: &mut f32,
    repeat: bool,
) {
    next_page(layout, setup, y);
    if repeat {
        for row_index in 0..table.header_rows.min(table.rows.len()) {
            table_row(layout, setup, table, geometry, row_index, y, false);
        }
    }
}
/// Lays out one table row, splitting it across pages when it doesn't fit.
fn table_row(
    layout: &mut Layout,
    setup: &PageSetup,
    table: &Table,
    geometry: &TableGeometry,
    row_index: usize,
    y: &mut f32,
    repeat: bool,
) {
    let Some(cells) = table.rows.get(row_index) else {
        return;
    };
    let widths = &geometry.widths;
    let margin = geometry.margin;
    let pad = margin[0] + margin[2];
    let starts = table.grid_starts(row_index);
    let next_starts = table.grid_starts(row_index + 1);
    // Width of a cell = sum of the grid columns it spans.
    let cell_widths: Vec<f32> = cells
        .iter()
        .zip(&starts)
        .map(|(cell, start)| {
            widths
                .iter()
                .skip(*start)
                .take(cell.span())
                .sum::<f32>()
                .max(f32::MIN_POSITIVE)
        })
        .collect();
    // A merged cell below continues this one: no line between them.
    let continued: Vec<bool> = starts
        .iter()
        .map(|start| {
            table.rows.get(row_index + 1).is_some_and(|below| {
                next_starts
                    .iter()
                    .position(|s| s == start)
                    .and_then(|i| below.get(i))
                    .is_some_and(|c| c.merged)
            })
        })
        .collect();
    let cell_lines: Vec<Vec<Row>> = cells.iter().enumerate().map(|(ci,cell)| {
        if row_index>0 && cell.merged {Vec::new()}
        else {table_lines(cell,cell_widths.get(ci).copied().unwrap_or(12.0),margin)}
    }).collect();
    let body_height=setup.height-setup.margin_top-setup.margin_bottom;
    let min_height=table.row_heights.get(row_index).copied().filter(|height|height.is_finite())
        .unwrap_or_default().clamp(0.0,body_height);
    let full_height=(cell_lines.iter().map(|lines|lines.iter().map(|line|line.height).sum::<f32>())
        .fold(8.0_f32,f32::max)+pad).max(min_height);
    if full_height<=body_height && *y+full_height>setup.height-setup.margin_bottom {
        table_page(layout,setup,table,geometry,y,repeat);
    }
    let mut offsets=vec![0usize;cells.len()];let mut first=true;let mut stalled=false;
    while first || cell_lines.iter().zip(&offsets).any(|(lines,offset)|*offset<lines.len()) {
        let available=setup.height-setup.margin_bottom-*y;
        if first && min_height>available && *y>setup.margin_top+0.5 {
            table_page(layout,setup,table,geometry,y,repeat && !stalled);stalled=true;continue;
        }
        let content_available=(available-pad).max(0.0);
        let mut counts=Vec::with_capacity(cells.len());let mut used=Vec::with_capacity(cells.len());
        for (lines,offset) in cell_lines.iter().zip(&offsets) {
            let mut count=0;let mut height=0.0;
            for line in lines.iter().skip(*offset) {
                if height+line.height>content_available {break;}
                height+=line.height;count+=1;
            }
            counts.push(count);used.push(height);
        }
        let blocked=cell_lines.iter().zip(&offsets).zip(&counts)
            .any(|((lines,offset),count)|*offset<lines.len() && *count==0);
        if blocked && *y>setup.margin_top+0.5 {
            table_page(layout,setup,table,geometry,y,repeat && !stalled);stalled=true;continue;
        }
        // Public layout can receive an unvalidated model; always make bounded progress.
        if blocked {
            for (ci,lines) in cell_lines.iter().enumerate() {
                if counts[ci]==0 && offsets[ci]<lines.len() {counts[ci]=1;used[ci]=lines[offsets[ci]].height;}
            }
        }
        stalled=false;
        let complete=cell_lines.iter().zip(&offsets).zip(&counts)
            .all(|((lines,offset),count)|offset+count>=lines.len());
        let mut row_height=used.iter().copied().fold(8.0_f32,f32::max)+pad;
        if first && complete {row_height=row_height.max(min_height);}
        let page_index=layout.pages.len().saturating_sub(1);
        if let Some(page)=layout.pages.last_mut() {
            for (ci,cell) in cells.iter().enumerate() {
                let x=setup.margin_left+widths.iter().take(starts.get(ci).copied().unwrap_or_default()).sum::<f32>();
                let width=cell_widths.get(ci).copied().unwrap_or(12.0);
                layout.cells.push(CellBox{table:table.id,row:row_index,column:ci,page:page_index,x,y:*y,w:width,h:row_height});
                let fill=if cell.merged && row_index>0 {merged_head_fill(table,row_index,ci)}else{cell.fill};
                page.items.push(Primitive::Rect{x,y:*y,w:width,h:row_height,color:fill});
                let edges=[
                    (!cell.merged,(x,*y,x+width,*y)),
                    (!continued.get(ci).copied().unwrap_or_default(),(x,*y+row_height,x+width,*y+row_height)),
                    (true,(x,*y,x,*y+row_height)),(true,(x+width,*y,x+width,*y+row_height))
                ];
                for (visible,(a,b,c,d)) in edges {
                    if visible && !table.borderless {
                        page.items.push(Primitive::Line{x1:a,y1:b,x2:c,y2:d,width:0.7,color:table.border_color});
                    }
                }
                let free=(row_height-pad-used[ci]).max(0.0);
                let mut slot_y=*y+margin[0]+match cell.v_align {VAlign::Top=>0.0,VAlign::Center=>free/2.0,VAlign::Bottom=>free};
                for row in cell_lines[ci].iter().skip(offsets[ci]).take(counts[ci]) {
                    let mut positioned=row.clone();positioned.y=slot_y+row.y;positioned.x+=x;
                    for segment in &mut positioned.segments {segment.x+=x;}
                    render_row(page,&positioned);slot_y+=row.height;
                }
            }
        }
        *y+=row_height;
        for (offset,count) in offsets.iter_mut().zip(&counts) {*offset+=count;}
        first=false;
        if !complete {table_page(layout,setup,table,geometry,y,repeat);}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn review5_validation_accounts_for_explicit_line_height() -> Result<(), String> {
        let mut doc = Doc::from_text("small exact line");
        doc.page.height = 144.0;
        doc.page.margin_top = 50.0;
        doc.page.margin_bottom = 50.0;
        let paragraph = &mut doc.paragraphs[0];
        paragraph.runs[0].format.size = 80.0;
        paragraph.line_points = 12.0;
        paragraph.line_exact = true;
        doc.validate()?;
        doc.paragraphs[0].line_points = 100.0;
        assert!(doc.validate().is_err());
        Ok(())
    }
    #[test]
    fn review5_hard_page_breaks_are_charged_as_pages_before_layout() {
        let doc = Doc::from_text(&"\u{000c}".repeat(9001));
        assert!(doc.validate().is_err());
    }
    #[test]
    fn review5_selection_format_keeps_an_empty_paragraph_mark() {
        let mut doc = Doc::from_text("before\n\nafter");
        doc.paragraphs[1].runs[0].format.italic = true;
        doc.format_range(
            Position {
                paragraph: 0,
                offset: 0,
            },
            Position {
                paragraph: 2,
                offset: 5,
            },
            |format| format.bold = true,
        );
        assert!(doc.paragraphs[1].runs[0].format.bold);
        assert!(doc.paragraphs[1].runs[0].format.italic);
        let mut doc = Doc::from_text("selected\n");
        doc.format_range(
            Position {
                paragraph: 0,
                offset: 0,
            },
            Position {
                paragraph: 1,
                offset: 0,
            },
            |format| format.bold = true,
        );
        assert!(!doc.paragraphs[1].runs[0].format.bold);
    }

    #[test]
    fn normalizing_keeps_valid_asymmetric_page_margins() -> Result<(), String> {
        let mut doc = Doc::from_text("Narrow body");
        doc.page.margin_left = 450.0;
        doc.page.margin_right = 50.0;
        doc.page.margin_top = 500.0;
        doc.page.margin_bottom = 50.0;
        let expected = doc.page.clone();
        doc.normalize();
        assert_eq!(doc.page, expected);
        doc.validate()
    }
    #[test]
    fn negative_indent_and_disabled_list_level_match_page_and_cell_layout() {
        let mut doc = Doc::from_text("outdent");
        doc.paragraphs[0].indent = -24.0;
        doc.paragraphs[0].list_level = 3;
        let layout = doc.layout();
        assert!((layout.rows[0].x - (doc.page.margin_left - 24.0)).abs() < 0.01);
        let cell = Cell::new(10, vec![doc.paragraphs[0].clone()], [255; 3]);
        let rows = table_lines(&cell, 200.0, [5.0; 4]);
        assert!((rows[0].x - (5.0 - 24.0)).abs() < 0.01);
        doc.paragraphs[0].indent = 0.0;
        doc.paragraphs[0].list_level = 3;
        assert!((doc.layout().rows[0].x - doc.page.margin_left).abs() < 0.01);
        let cell = Cell::new(10, vec![doc.paragraphs[0].clone()], [255; 3]);
        assert!((table_lines(&cell, 200.0, [5.0; 4])[0].x - 5.0).abs() < 0.01);
    }
    #[test]
    fn inserted_row_preserves_the_vertical_merge_chain() -> Result<(), String> {
        let mut doc = Doc::from_text("table");
        let id = doc.add_table(doc.paragraphs[0].id, 3, 2)?;
        doc.tables[0].rows[1][0].merged = true;
        doc.tables[0].rows[2][0].merged = true;
        doc.edit_table(id, TableEdit::InsertRow(1))?;
        let rows = &doc.tables[0].rows;
        assert_eq!(rows.len(), 4);
        assert!(!rows[0][0].merged);
        assert!(rows.iter().skip(1).all(|row| row[0].merged));
        assert!(rows.iter().all(|row| !row[1].merged));
        doc.validate()
    }

    #[test]
    fn page_margin_changes_fit_a_smaller_paper_without_invalidating_it() -> Result<(), String> {
        let mut doc = Doc::from_text("A5");
        doc.page.width = 419.53;
        doc.page.height = 595.28;
        doc.page.margin_left = 200.0;
        doc.page.margin_right = 200.0;
        doc.page.fit_margins();
        doc.validate()?;
        assert!((doc.page.margin_left - doc.page.margin_right).abs() < 0.001);
        assert!(doc.page.width - doc.page.margin_left - doc.page.margin_right >= 23.99);
        Ok(())
    }
    #[test]
    fn wrapping_prefers_a_normal_space_over_a_nonbreaking_space() {
        let p = Paragraph::default();
        let text = "X A\u{00a0}B";
        let chars = text
            .chars()
            .map(|ch| (ch, TextFormat::default()))
            .collect::<Vec<_>>();
        let width = chars
            .iter()
            .take(4)
            .map(|(ch, f)| char_width(*ch, f))
            .sum::<f32>()
            + 0.01;
        let lines = wrap_lines(&p, &chars, width, 0.0, 1.0);
        assert_eq!(
            lines
                .first()
                .map(|line| text.chars().take(line.end).collect::<String>()),
            Some("X ".into())
        );
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn failed_table_commands_leave_ragged_content_and_ids_unchanged() -> Result<(), String> {
        let mut doc = Doc::from_text("table");
        let anchor = doc.paragraphs[0].id;
        doc.add_table(anchor, 2, 2)?;
        doc.tables[0].rows[1].pop();
        let id = doc.tables[0].id;
        let before = doc.clone();
        assert!(doc.edit_table(id, TableEdit::DeleteRow(99)).is_err());
        assert_eq!(doc, before);
        Ok(())
    }
    #[test]
    fn nested_numbering_resumes_parent_and_restarts_children() {
        let mut p = Paragraph {
            list: List::Number,
            ..Paragraph::default()
        };
        let mut numbers = [0; 9];
        assert_eq!(list_marker(&p, &mut numbers), "1.");
        p.list_level = 1;
        assert_eq!(list_marker(&p, &mut numbers), "1.");
        assert_eq!(list_marker(&p, &mut numbers), "2.");
        p.list_level = 0;
        assert_eq!(list_marker(&p, &mut numbers), "2.");
        p.list_level = 1;
        assert_eq!(list_marker(&p, &mut numbers), "1.");
    }
    #[test]
    fn table_lists_render_markers_and_invalid_table_geometry_is_rejected() -> Result<(), String> {
        let mut doc = Doc::from_text("table");
        let anchor = doc.paragraphs[0].id;
        doc.add_table(anchor, 1, 1)?;
        doc.tables[0].rows[0][0].paragraphs[0].list = List::Bullet;
        let lines = table_lines(&doc.tables[0].rows[0][0], 100.0, [5.0; 4]);
        assert_eq!(lines[0].marker, "•");
        doc.validate()?;
        doc.tables[0].cell_margin[0] = f32::NAN;
        assert!(doc.validate().is_err());
        Ok(())
    }

    use super::*;
    #[test]
    fn page_breaks_avoid_widows_and_orphans() {
        // Five 10 pt lines on a page whose text area is 0..100.
        let h = [10.0; 5];
        // Only the first line would fit: the whole paragraph moves.
        assert_eq!(line_breaks(&h, 85.0, 0.0, 100.0, true), vec![0]);
        assert_eq!(line_breaks(&h, 85.0, 0.0, 100.0, false), vec![1]);
        // Only the last line would move: it takes one more line with it.
        assert_eq!(line_breaks(&h, 55.0, 0.0, 100.0, true), vec![3]);
        assert_eq!(line_breaks(&h, 55.0, 0.0, 100.0, false), vec![4]);
        assert!(line_breaks(&h, 0.0, 0.0, 100.0, true).is_empty());
    }
    #[test]
    fn tabs_jump_to_default_stops_and_hanging_indent() {
        let f = TextFormat::default();
        assert_eq!(advance('\t', &f, 10.0, 0.0), 26.0);
        assert_eq!(advance('\t', &f, 0.0, 0.0), 36.0);
        assert_eq!(advance('\t', &f, 5.0, 18.0), 13.0);
        let doc = Doc::from_text("a\tb");
        let layout = doc.layout();
        let row = layout
            .rows
            .first()
            .map(|r| r.segments.clone())
            .unwrap_or_default();
        let b = row.iter().find(|s| s.text == "b").map(|s| s.x);
        let expected = doc.page.margin_left + 36.0;
        assert!(b.is_some_and(|x| (x - expected).abs() < 0.01), "{b:?}");
    }
    #[test]
    fn justified_lines_fill_the_width() -> Result<(), String> {
        let mut doc = Doc::from_text(&"слово ".repeat(80));
        if let Some(p) = doc.paragraphs.first_mut() {
            p.align = Align::Justify;
        }
        let layout = doc.layout();
        let right = doc.page.width - doc.page.margin_right;
        let row = layout.rows.first().ok_or("row")?;
        let end = row
            .segments
            .iter()
            .rev()
            .find(|s| !s.text.trim().is_empty())
            .map(|s| {
                let trailing = s.text.chars().rev().take_while(|c| *c == ' ').count();
                s.x + s.width - trailing as f32 * char_width(' ', &s.format)
            })
            .ok_or("segment")?;
        assert!((end - right).abs() < 0.5, "{end} vs {right}");
        // The last line keeps normal spacing.
        assert_eq!(layout.rows.last().map(|r| r.segments.len()), Some(1));
        Ok(())
    }
    #[test]
    fn contextual_spacing_and_keep_with_next() -> Result<(), String> {
        let mut doc = Doc::from_text("a\nb");
        for p in &mut doc.paragraphs {
            p.spacing_after = 10.0;
            p.contextual_spacing = true;
        }
        let layout = doc.layout();
        let (a, b) = (
            layout.rows.first().ok_or("a")?,
            layout.rows.get(1).ok_or("b")?,
        );
        assert!((b.y - (a.y + a.height)).abs() < 0.01);
        let mut doc = Doc::from_text(&vec!["строка"; 90].join("\n"));
        let layout = doc.layout();
        let last = layout
            .rows
            .iter()
            .filter(|r| r.page == 0)
            .map(|r| r.paragraph)
            .max()
            .ok_or("page")?;
        if let Some(p) = doc.paragraphs.get_mut(last) {
            p.keep_next = true;
        }
        let layout = doc.layout();
        assert_eq!(
            layout
                .rows
                .iter()
                .find(|r| r.paragraph == last)
                .map(|r| r.page),
            Some(1)
        );
        Ok(())
    }
    #[test]
    fn table_header_rows_repeat_on_new_pages() -> Result<(), String> {
        let mut doc = Doc::from_text("a");
        let anchor = doc.paragraphs.first().ok_or("p")?.id;
        doc.add_table(anchor, 120, 2)?;
        if let Some(t) = doc.tables.first_mut() {
            t.header_rows = 1;
        }
        let layout = doc.layout();
        assert!(layout.pages.len() > 1);
        for page in 1..layout.pages.len() {
            assert!(layout.cells.iter().any(|c| c.page == page && c.row == 0));
        }
        Ok(())
    }
    #[test]
    fn splitting_a_paragraph_keeps_its_table_after_the_text() -> Result<(), String> {
        let mut doc = Doc::from_text("ab");
        let anchor = doc.paragraphs.first().ok_or("p")?.id;
        doc.add_table(anchor, 1, 1)?;
        let at = Position {
            paragraph: 0,
            offset: 1,
        };
        doc.replace_range(
            at, at, "
", None,
        )?;
        let tail = doc.paragraphs.get(1).ok_or("tail")?.id;
        assert_eq!(doc.tables.first().map(|t| t.after_paragraph_id), Some(tail));
        Ok(())
    }
    #[test]
    fn empty_page_is_deleted_but_content_is_kept() {
        let mut doc = Doc::from_text(
            "Текст


Конец",
        );
        if let Some(p) = doc.paragraphs.get_mut(1) {
            p.page_break_before = true;
        }
        if let Some(p) = doc.paragraphs.get_mut(3) {
            p.page_break_before = true;
        }
        assert_eq!(doc.layout().pages.len(), 3);
        assert!(!doc.delete_empty_page(0), "page with text stays");
        assert!(doc.delete_empty_page(1));
        assert_eq!(doc.layout().pages.len(), 2);
        assert_eq!(
            doc.plain_text().trim(),
            "Текст
Конец"
        );
    }
    #[test]
    fn column_edits_respect_merged_cells() -> Result<(), String> {
        let mut doc = Doc::from_text("a");
        let anchor = doc.paragraphs.first().ok_or("p")?.id;
        let id = doc.add_table(anchor, 2, 3)?;
        if let Some(row) = doc.tables.first_mut().and_then(|t| t.rows.first_mut()) {
            row.truncate(2);
            if let Some(cell) = row.first_mut() {
                cell.span = 2;
            }
        }
        doc.edit_table(id, TableEdit::InsertColumn(1))?;
        let t = doc.tables.first().ok_or("t")?;
        assert_eq!(t.grid_columns(), 4);
        assert_eq!(
            t.rows.first().and_then(|r| r.first()).map(|c| c.span),
            Some(3)
        );
        assert_eq!(t.rows.get(1).map(Vec::len), Some(4));
        doc.edit_table(id, TableEdit::DeleteColumn(0))?;
        let t = doc.tables.first().ok_or("t")?;
        assert_eq!(t.grid_columns(), 3);
        assert_eq!(
            t.rows.first().and_then(|r| r.first()).map(|c| c.span),
            Some(2)
        );
        Ok(())
    }
    #[test]
    fn empty_table_cell_format_survives_frames_and_first_input() -> Result<(), String> {
        let mut doc = Doc::from_text("anchor");
        let anchor = doc.paragraphs.first().ok_or("anchor")?.id;
        doc.add_table(anchor, 1, 1)?;
        let expected = TextFormat {
            font: "Arial".into(),
            size: 18.0,
            bold: true,
            italic: true,
            color: [34, 86, 172],
            highlight: Some([240, 230, 160]),
            ..TextFormat::default()
        };
        let cell = doc
            .tables
            .first_mut()
            .and_then(|table| table.rows.first_mut())
            .and_then(|row| row.first_mut())
            .ok_or("cell")?;
        let cell_id = cell.id;
        let paragraph = cell.paragraphs.first_mut().ok_or("paragraph")?;
        let paragraph_id = paragraph.id;
        paragraph.runs.first_mut().ok_or("run")?.format = expected.clone();

        doc.normalize();
        doc.normalize();
        doc.validate()?;
        let cell = doc
            .tables
            .first()
            .and_then(|table| table.rows.first())
            .and_then(|row| row.first())
            .ok_or("cell")?;
        assert_eq!(cell.id, cell_id);
        assert_eq!(cell.paragraphs.first().ok_or("paragraph")?.id, paragraph_id);
        assert_eq!(
            cell.paragraphs.first().ok_or("paragraph")?.format_at(0),
            expected
        );

        let mut nested = Doc {
            paragraphs: cell.paragraphs.clone(),
            next_id: doc.next_id,
            ..Doc::default()
        };
        nested.replace_range(Position::default(), Position::default(), "Привет 🌍", None)?;
        nested.normalize();
        assert_eq!(nested.plain_text(), "Привет 🌍");
        let paragraph = nested.paragraphs.first().ok_or("paragraph")?;
        assert_eq!(paragraph.id, paragraph_id);
        assert_eq!(paragraph.runs.len(), 1);
        assert_eq!(paragraph.runs.first().ok_or("run")?.format, expected);
        Ok(())
    }
    #[test]
    fn unicode_insert_delete() {
        let mut d = Doc::from_text("Привет 🌍");
        let id = d.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(d.insert_text(id, 7, "мир ").is_ok());
        assert_eq!(d.plain_text(), "Привет мир 🌍");
        let result = d.replace_range(
            Position {
                paragraph: 0,
                offset: 7,
            },
            Position {
                paragraph: 0,
                offset: 11,
            },
            "",
            None,
        );
        assert!(result.is_ok());
        assert_eq!(d.plain_text(), "Привет 🌍");
    }
    #[test]
    fn multiline_preserves_surrounding_format_and_ids() {
        let mut d = Doc::from_text("abcXYZ");
        if let Some(p) = d.paragraphs.first_mut()
            && let Some(r) = p.runs.first_mut()
        {
            r.format.bold = true;
        }
        let id = d.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(d.insert_text(id, 3, "one\ntwo").is_ok());
        assert_eq!(d.plain_text(), "abcone\ntwoXYZ");
        assert_eq!(d.paragraphs.first().map(|p| p.id), Some(id));
        assert!(
            d.paragraphs
                .get(1)
                .is_some_and(|p| p.id != id && p.runs.iter().all(|r| r.format.bold))
        );
    }
    #[test]
    fn format_partial_range() {
        let mut d = Doc::from_text("abcdef");
        d.format_range(
            Position {
                paragraph: 0,
                offset: 2,
            },
            Position {
                paragraph: 0,
                offset: 4,
            },
            |f| f.bold = true,
        );
        let p = d.paragraphs.first();
        assert!(p.is_some_and(|p| {
            p.runs.len() == 3
                && p.runs
                    .get(1)
                    .is_some_and(|r| r.text == "cd" && r.format.bold)
        }));
    }
    #[test]
    fn bounded_find_keeps_unicode_offsets_and_overlap() {
        let doc = Doc::from_text("skip\naaa ПрИвЕт 🌍\nother Привет");
        let found = doc.find_page("aa", true, 1, 1, 1);
        assert_eq!(
            found,
            vec![(
                Position {
                    paragraph: 1,
                    offset: 0
                },
                Position {
                    paragraph: 1,
                    offset: 2
                }
            )]
        );
        assert_eq!(doc.find_page("привет", false, 1, 1, 1000).len(), 1);
        assert_eq!(doc.find("aa", true).len(), 2);
        assert!(doc.find_page("a", true, 1, 0, 1).is_empty());
        assert!(doc.find_page("a", true, 1, 1, 0).is_empty());
        assert_eq!(
            doc.find_page("🌍", true, 1, 1, 1000)
                .first()
                .map(|(a, _)| a.offset),
            Some(11)
        );
    }
    #[test]
    fn oversized_lines_cannot_escape_page_bounds() {
        let mut doc = Doc::from_text("large");
        doc.page.height = 144.0;
        doc.page.margin_top = 50.0;
        doc.page.margin_bottom = 50.0;
        if let Some(r) = doc.paragraphs.first_mut().and_then(|p| p.runs.first_mut()) {
            r.format.size = 144.0;
        }
        assert!(doc.validate().is_err());
    }
    #[test]
    fn replacement_at_run_boundaries_preserves_the_matched_format() -> Result<(), String> {
        let mut doc = Doc::from_text("");
        doc.paragraphs[0].runs = vec![
            Run {
                text: "prefix ".into(),
                format: TextFormat::default(),
            },
            Run {
                text: "needle".into(),
                format: TextFormat {
                    bold: true,
                    color: [200, 0, 0],
                    ..TextFormat::default()
                },
            },
            Run {
                text: " suffix".into(),
                format: TextFormat::default(),
            },
        ];
        assert_eq!(doc.replace_all("needle", "новый", true), 1);
        assert_eq!(doc.plain_text(), "prefix новый suffix");
        assert!(
            doc.paragraphs[0]
                .runs
                .iter()
                .any(|r| r.text == "новый" && r.format.bold && r.format.color == [200, 0, 0])
        );
        assert!(
            doc.paragraphs[0]
                .runs
                .first()
                .is_some_and(|r| !r.format.bold)
        );
        assert!(
            doc.paragraphs[0]
                .runs
                .last()
                .is_some_and(|r| !r.format.bold)
        );
        doc.validate()
    }
    #[test]
    fn replacement_is_non_overlapping() {
        let mut d = Doc::from_text("aaaa ПрИвЕт привет");
        assert_eq!(d.replace_all("aa", "b", true), 2);
        assert_eq!(d.replace_all("привет", "ok", false), 2);
        assert_eq!(d.plain_text(), "bb ok ok");
    }
    #[test]
    fn paginate_long_paragraph() {
        let d = Doc::from_text(&"Длинный документ ".repeat(3000));
        let layout = d.layout();
        assert!(layout.pages.len() > 1);
        assert!(layout.rows.windows(2).all(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_none_or(|(a, b)| a.end == b.start)
        }));
        assert!(
            layout
                .rows
                .iter()
                .all(|r| r.y + r.height <= d.page.height - d.page.margin_bottom + 0.1)
        );
    }
    #[test]
    fn table_ids_and_layout() {
        let mut d = Doc::default();
        let id = d.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(d.add_table(id, 3, 2).is_ok());
        d.normalize();
        let ids: Vec<u64> = d
            .tables
            .iter()
            .flat_map(|t| t.rows.iter().flatten())
            .map(|c| c.id)
            .collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
        assert!(!d.pages().is_empty());
    }
    #[test]
    fn rejects_duplicate_ids_and_bad_geometry() {
        let mut doc = Doc::default();
        assert!(doc.validate().is_ok());
        doc.paragraphs.push(Paragraph::default());
        assert!(doc.validate().is_err());
        doc.paragraphs.pop();
        doc.page.width = f32::NAN;
        assert!(doc.validate().is_err());
    }
    #[test]
    fn validates_nested_ids_and_counter() {
        let mut doc = Doc::default();
        let id = doc.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(doc.add_table(id, 1, 1).is_ok());
        assert!(doc.validate().is_ok());
        if let Some(c) = doc
            .tables
            .first_mut()
            .and_then(|t| t.rows.first_mut())
            .and_then(|r| r.first_mut())
        {
            c.id = id;
        }
        assert!(doc.validate().is_err());
        doc.normalize();
        assert!(doc.validate().is_ok());
        doc.next_id = 1;
        assert!(doc.validate().is_err());
    }
    #[test]
    fn table_render_preserves_rich_format() {
        let mut doc = Doc::default();
        doc.page.page_numbers = false;
        let id = doc.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(doc.add_table(id, 1, 1).is_ok());
        if let Some(p) = doc
            .tables
            .first_mut()
            .and_then(|t| t.rows.first_mut())
            .and_then(|r| r.first_mut())
            .and_then(|c| c.paragraphs.first_mut())
        {
            p.runs = vec![Run {
                text: "Rich table text".into(),
                format: TextFormat {
                    size: 24.0,
                    bold: true,
                    italic: true,
                    color: [120, 30, 80],
                    highlight: Some([250, 230, 100]),
                    ..TextFormat::default()
                },
            }];
        }
        let pages = doc.pages();
        assert!(pages.iter().flat_map(|p| &p.items).any(|i| matches!(i,
            Primitive::TextStyled{text,size,bold:true,italic:true,color,..}
            if text=="Rich table text"&&*size==24.0&&*color==[120,30,80])));
        assert!(pages.iter().flat_map(|p| &p.items).any(|i| matches!(i,
            Primitive::Rect{color,..} if *color==[250,230,100])));
    }
    #[test]
    fn allocator_stays_monotonic_for_many_paragraphs() {
        let doc = Doc::from_text(&"x\n".repeat(10000));
        assert_eq!(doc.paragraphs.len(), 10001);
        assert!(doc.paragraphs.windows(2).all(|w| {
            w.first()
                .zip(w.get(1))
                .is_none_or(|(a, b)| a.id + 1 == b.id)
        }));
        assert_eq!(doc.next_id, 10003);
    }
    #[test]
    fn long_table_row_has_no_text_loss() {
        let mut doc = Doc::default();
        doc.page.page_numbers = false;
        let id = doc.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(doc.add_table(id, 1, 1).is_ok());
        let text = "Длинная ячейка ".repeat(1000);
        if let Some(p) = doc
            .tables
            .first_mut()
            .and_then(|t| t.rows.first_mut())
            .and_then(|r| r.first_mut())
            .and_then(|c| c.paragraphs.first_mut())
        {
            p.runs = vec![Run {
                text: text.clone(),
                format: TextFormat::default(),
            }];
        }
        let pages = doc.pages();
        assert!(pages.len() > 1);
        let rendered = pages
            .iter()
            .flat_map(|p| p.items.iter())
            .filter_map(|item| match item {
                Primitive::Text { text, .. } | Primitive::TextStyled { text, .. } => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect::<String>();
        assert_eq!(rendered, text);
    }
}

#[cfg(test)]
mod merge_deletion_defect_tests {
    use super::*;
    #[test]
    fn delete_middle_vertical_merge_head_promotes_its_continuation() -> Result<(),String> {
        let mut doc = Doc::from_text("table");
        let id = doc.add_table(doc.paragraphs[0].id,3,1)?;
        doc.tables[0].rows[2][0].merged=true;
        doc.validate()?;
        doc.edit_table(id,TableEdit::DeleteRow(1))?;
        assert!(!doc.tables[0].rows[1][0].merged);
        let mut chain = Doc::from_text("chain");
        let id = chain.add_table(chain.paragraphs[0].id,3,1)?;
        chain.tables[0].rows[1][0].merged=true;
        chain.tables[0].rows[2][0].merged=true;
        chain.edit_table(id,TableEdit::DeleteRow(1))?;
        assert!(chain.tables[0].rows[1][0].merged);
        Ok(())
    }
}

fn push_replacement_char(paragraph:&mut Paragraph,ch:char,format:&TextFormat) {
    if let Some(run)=paragraph.runs.last_mut().filter(|run|run.format==*format) {run.text.push(ch);}
    else{paragraph.runs.push(Run{text:ch.to_string(),format:format.clone()});}
}
#[cfg(test)]
mod next_replace_all_defects {
    use super::*;
    #[test]
    fn bulk_replacement_builds_dense_matches_and_preserves_rich_suffixes()->Result<(),String> {
        let mut doc=Doc::from_text(&"a".repeat(20000));
        doc.paragraphs[0].runs.push(Run{text:"tail".into(),format:TextFormat{bold:true,..Default::default()}});
        assert_eq!(doc.try_replace_all("a","xy",true)?,20001);
        assert_eq!(doc.paragraphs[0].text(),format!("{}txyil","xy".repeat(20000)));
        assert!(doc.paragraphs[0].runs.last().is_some_and(|run|run.format.bold));
        Ok(())
    }
    #[test]
    fn bulk_replacement_limits_refuse_the_whole_operation_before_mutation()->Result<(),String> {
        let mut doc=Doc::from_text("a a");let before=doc.clone();
        assert!(doc.replace_all_with_limits("a","long",true,4,100).is_err());
        assert_eq!(doc,before);
        assert!(doc.replace_all_with_limits("a","x",true,100,1).is_err());
        assert_eq!(doc,before);Ok(())
    }
}

#[cfg(test)]
mod hundred_docs_model_regressions {
    use super::*;
    #[test]
    fn subpoint_images_keep_their_model_dimensions_when_laid_out()->Result<(),String> {
        let mut doc=Doc::from_text("anchor");let id=doc.alloc_id();
        let mut png=std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1,1).write_to(&mut png,image::ImageFormat::Png).map_err(|error|error.to_string())?;
        doc.images.push(AnchoredImage{id,after_paragraph_id:doc.paragraphs[0].id,bytes:png.into_inner(),width:0.25,height:2.0,alt:String::new()});
        doc.validate()?;
        let layout=doc.layout();
        assert!(layout.pages.iter().flat_map(|page|&page.items).any(|item|matches!(item,Primitive::Image{w,h,..} if *w==0.25 && *h==2.0)));Ok(())
    }
    #[test]
    fn multiline_replace_preflights_limits_and_keeps_rich_suffix_without_repeated_source_clones()->Result<(),String> {
        let mut doc=Doc::from_text(&"x".repeat(20000));
        doc.paragraphs[0].runs.push(Run{text:"tail".into(),format:TextFormat{bold:true,..Default::default()}});
        let pos=Position{paragraph:0,offset:0};
        doc.replace_range(pos,pos,&"line\n".repeat(2000),None)?;
        assert_eq!(doc.paragraphs.len(),2001);
        assert!(doc.paragraphs.last().and_then(|paragraph|paragraph.runs.last()).is_some_and(|run|run.text=="tail" && run.format.bold));
        doc.validate()?;let before=doc.clone();
        assert!(doc.replace_range(pos,pos,&"\n".repeat(100000),None).is_err());
        assert_eq!(doc,before);Ok(())
    }
    #[test]
    fn word_count_excludes_invisible_vertical_merge_continuations()->Result<(),String> {
        let mut doc=Doc::from_text("body");let anchor=doc.paragraphs[0].id;
        doc.add_table(anchor,2,1)?;
        let table=doc.tables[0].id;
        doc.set_cell_text(table,0,0,"visible")?;doc.set_cell_text(table,1,0,"hidden extra words")?;
        doc.tables[0].rows[1][0].merged=true;
        assert_eq!(doc.word_count(),2);Ok(())
    }
}

fn validate_table_merges(table:&Table)->Result<(),String> {
    let mut previous=std::collections::BTreeMap::new();
    for row in &table.rows {
        let mut current=std::collections::BTreeMap::new();let mut column=0;
        for cell in row {
            if cell.merged && previous.get(&column)!=Some(&cell.span()) {
                return Err("Continuation объединённой ячейки не совпадает с предыдущей строкой".into());
            }
            current.insert(column,cell.span());column+=cell.span();
        }
        previous=current;
    }
    Ok(())
}

fn merged_head_fill(table:&Table,row:usize,column:usize)->[u8;3] {
    let Some(cell)=table.rows.get(row).and_then(|row|row.get(column)) else{return [255;3];};
    let start=table.grid_starts(row).get(column).copied().unwrap_or_default();
    let span=cell.span();
    for index in (0..=row).rev() {
        if let Some(owner)=table.rows[index].iter().zip(table.grid_starts(index))
            .find_map(|(cell,column)|(column==start && cell.span()==span).then_some(cell)) {
            if !owner.merged {return owner.fill;}
        }else{break;}
    }
    cell.fill
}

fn paragraph_lines_for_layout(paragraph:&Paragraph,setup:&PageSetup)->Vec<Line> {
    let indent=paragraph.indent+list_indent(paragraph);
    let available=(setup.width-setup.margin_left-setup.margin_right-indent-paragraph.right_indent).max(12.0);
    let chars=paragraph.runs.iter().flat_map(|run|run.text.chars().map(move |ch|(ch,run.format.clone()))).collect::<Vec<_>>();
    wrap_lines(paragraph,&chars,available,indent,12.0)
}
fn next_anchored_height(doc:&Doc,paragraph:u64,body:f32)->Option<f32> {
    if let Some(table)=doc.tables.iter().find(|table|table.after_paragraph_id==paragraph)
        && let Some(row)=table.rows.first() {
        let usable=doc.page.width-doc.page.margin_left-doc.page.margin_right;
        let columns=table.grid_columns().max(1);
        let mut widths=(0..columns).map(|column|table.column_widths.get(column).copied().unwrap_or(usable/columns as f32).max(0.05)).collect::<Vec<_>>();
        let total=widths.iter().sum::<f32>();let limit=usable+doc.page.margin_right.min(36.0);
        if total>limit {for width in &mut widths {*width*=limit/total;}}
        let starts=table.grid_starts(0);let pad=table.cell_margin[0]+table.cell_margin[2];
        let heights=row.iter().zip(starts).map(|(cell,start)| {
            let width=widths.iter().skip(start).take(cell.span()).sum::<f32>();
            table_lines(cell,width,table.cell_margin).iter().map(|line|line.height).sum::<f32>()
        }).collect::<Vec<_>>();
        let full=(heights.into_iter().fold(8.0_f32,f32::max)+pad)
            .max(table.row_heights.first().copied().unwrap_or_default());
        return Some(full.min(body));
    }
    doc.images.iter().find(|image|image.after_paragraph_id==paragraph).map(|image| {
        let width=doc.page.width-doc.page.margin_left-doc.page.margin_right;
        let scale=(width/image.width).min(body/image.height).min(1.0);
        image.height*scale
    })
}
fn keep_group_height(doc:&Doc,start:usize,mut height:f32,body:f32)->Option<f32> {
    let mut previous=start;
    loop {
        let paragraph=doc.paragraphs.get(previous)?;
        if let Some(block)=next_anchored_height(doc,paragraph.id,body) {return Some(height+block);}
        let next=doc.paragraphs.get(previous+1)?;
        if next.page_break_before {return None;}
        let spacing=paragraph_spacing(next,Some(paragraph),doc.paragraphs.get(previous+2));
        let lines=paragraph_lines_for_layout(next,&doc.page);
        height+=spacing.0;
        if next.keep_next {
            height+=lines.iter().map(|line|line.height).sum::<f32>()+spacing.1;
            if height>body{return None;}
            previous+=1;
        }else{
            return Some(height+lines.first().map(|line|line.height).unwrap_or(4.0));
        }
    }
}

fn xml_model_text(text:&str,breaks:bool)->bool {
    text.chars().all(|ch| ch>=' ' || matches!(ch,'\t'|'\n'|'\r') || breaks && matches!(ch,'\u{000b}'|'\u{000c}'))
}

#[cfg(test)]
mod two_hundred_docs_model_regressions {
    use super::*;
    fn small_doc(text:&str)->Doc {
        let mut doc=Doc::from_text(text);
        doc.page=PageSetup{width:300.0,height:144.0,margin_top:22.0,margin_bottom:22.0,margin_left:20.0,margin_right:20.0,page_numbers:false,..Default::default()};
        for paragraph in &mut doc.paragraphs {paragraph.spacing_before=0.0;paragraph.spacing_after=0.0;paragraph.line_points=20.0;paragraph.line_exact=true;paragraph.widow_control=false;}
        doc
    }
    fn drawn_text(layout:&Layout)->String {
        layout.pages.iter().flat_map(|page|&page.items).filter_map(|item|match item {Primitive::TextStyled{text,..}=>Some(text.as_str()),_=>None}).collect()
    }
    #[test]
    fn cell_text_crlf_is_normalized_before_computing_the_edit_delta()->Result<(),String> {
        let nested=Doc::from_text("one\ntwo");let ids=nested.paragraphs.iter().map(|paragraph|paragraph.id).collect::<Vec<_>>();
        let mut cell=Cell::new(90,nested.paragraphs,[255;3]);let mut next=100;
        try_edit_cell_text(&mut cell,"one\r\ntwo",&mut next)?;
        assert_eq!(cell_text(&cell),"one\ntwo");assert_eq!(cell.paragraphs.iter().map(|paragraph|paragraph.id).collect::<Vec<_>>(),ids);
        try_edit_cell_text(&mut cell,"one\r\nchanged\rtwo",&mut next)?;
        assert_eq!(cell_text(&cell),"one\nchanged\ntwo");Ok(())
    }
    #[test]
    fn table_cells_flow_independently_instead_of_stretching_each_others_lines()->Result<(),String> {
        let mut doc=Doc::from_text("anchor");doc.add_table(doc.paragraphs[0].id,1,2)?;
        doc.tables[0].rows[0][0].paragraphs[0].runs[0].text="one\u{000b}two".into();
        doc.tables[0].rows[0][1].paragraphs[0].runs[0]=Run{text:"large".into(),format:TextFormat{size:60.0,..Default::default()}};
        let layout=doc.layout();
        let ys=layout.pages.iter().flat_map(|page|&page.items).filter_map(|item|match item {Primitive::TextStyled{text,y,..} if text=="one" || text=="two"=>Some(*y),_=>None}).collect::<Vec<_>>();
        assert_eq!(ys.len(),2);assert!(ys[1]>ys[0] && ys[1]-ys[0]<30.0,"{ys:?}");
        assert_eq!(layout.cells.len(),2);Ok(())
    }
    #[test]
    fn table_validation_accounts_for_the_configured_vertical_padding()->Result<(),String> {
        let mut doc=small_doc("anchor");doc.add_table(doc.paragraphs[0].id,1,1)?;
        doc.tables[0].rows[0][0].paragraphs[0].runs[0].format.size=30.0;
        doc.tables[0].cell_margin=[40.0,0.0,40.0,0.0];assert!(doc.validate().is_err());
        doc.tables[0].cell_margin=[0.0;4];doc.tables[0].rows[0][0].paragraphs[0].runs[0].format.size=80.0;
        doc.tables[0].rows[0][0].paragraphs[0].line_points=96.0;doc.tables[0].rows[0][0].paragraphs[0].line_exact=true;
        doc.tables[0].rows[0][0].paragraphs[0].spacing_after=0.0;doc.validate()?;Ok(())
    }
    #[test]
    fn wrapped_line_height_uses_the_first_visible_run_instead_of_the_previous_one() {
        let large=TextFormat{size:60.0,..Default::default()};let small=TextFormat{size:11.0,..Default::default()};
        let paragraph=Paragraph{runs:vec![Run{text:"A".into(),format:large.clone()},Run{text:"b".into(),format:small.clone()}],..Default::default()};
        let chars=vec![('A',large.clone()),('b',small)];
        let lines=wrap_lines(&paragraph,&chars,char_width('A',&large)+0.1,0.0,1.0);
        assert_eq!(lines.len(),2);assert!(lines[1].height<lines[0].height/2.0);
    }
    #[test]
    fn vertical_merge_continuations_do_not_render_duplicate_text_or_replace_the_head_fill()->Result<(),String> {
        let mut doc=Doc::from_text("anchor");doc.add_table(doc.paragraphs[0].id,2,1)?;
        doc.tables[0].rows[0][0].fill=[1,2,3];doc.tables[0].rows[0][0].paragraphs[0].runs[0].text="visible".into();
        doc.tables[0].rows[1][0].merged=true;doc.tables[0].rows[1][0].fill=[200,201,202];
        doc.tables[0].rows[1][0].paragraphs[0].runs[0].text="hidden continuation".into();
        let layout=doc.layout();assert!(!drawn_text(&layout).contains("hidden continuation"));
        assert_eq!(merged_head_fill(&doc.tables[0],1,0),[1,2,3]);
        let below=layout.cells.iter().find(|cell|cell.row==1).ok_or("continuation box")?;
        assert!(layout.pages[below.page].items.iter().any(|item|matches!(item,Primitive::Rect{y,color,..} if *y==below.y && *color==[1,2,3])));Ok(())
    }
    #[test]
    fn table_validation_rejects_orphaned_or_misaligned_vertical_merges()->Result<(),String> {
        let mut doc=Doc::from_text("anchor");doc.add_table(doc.paragraphs[0].id,2,2)?;
        doc.tables[0].rows[0][0].merged=true;assert!(doc.validate().is_err());
        doc.tables[0].rows[0][0].merged=false;doc.tables[0].rows[1][0].merged=true;doc.validate()?;
        doc.tables[0].rows[1][0].span=2;doc.tables[0].rows[1].remove(1);assert!(doc.validate().is_err());Ok(())
    }
    #[test]
    fn keep_next_chains_move_as_one_group_when_the_whole_chain_fits() {
        let mut doc=small_doc("prefix\nfirst\nsecond\nbody");doc.paragraphs[0].line_points=60.0;
        doc.paragraphs[1].keep_next=true;doc.paragraphs[2].keep_next=true;
        let layout=doc.layout();let page=|paragraph|layout.rows.iter().find(|row|row.paragraph==paragraph).map(|row|row.page);
        assert_eq!(page(0),Some(0));assert_eq!(page(1),Some(1));assert_eq!(page(1),page(2));assert_eq!(page(2),page(3));
    }
    #[test]
    fn keep_next_includes_the_next_anchored_table_or_image_block()->Result<(),String> {
        let mut doc=small_doc("prefix\nheading");doc.paragraphs[0].line_points=60.0;doc.paragraphs[1].keep_next=true;
        let anchor=doc.paragraphs[1].id;doc.add_table(anchor,1,1)?;doc.tables[0].row_heights=vec![60.0];
        let layout=doc.layout();assert_eq!(layout.rows.iter().find(|row|row.paragraph==1).map(|row|row.page),Some(1));assert_eq!(layout.cells[0].page,1);
        let mut png=std::io::Cursor::new(Vec::new());image::DynamicImage::new_rgba8(1,1).write_to(&mut png,image::ImageFormat::Png).map_err(|error|error.to_string())?;
        doc.tables.clear();let id=doc.alloc_id();doc.images.push(AnchoredImage{id,after_paragraph_id:anchor,bytes:png.into_inner(),width:60.0,height:60.0,alt:String::new()});
        let layout=doc.layout();assert_eq!(layout.rows.iter().find(|row|row.paragraph==1).map(|row|row.page),Some(1));
        assert!(layout.pages[1].items.iter().any(|item|matches!(item,Primitive::Image{..})));Ok(())
    }
    #[test]
    fn empty_list_paragraph_markers_keep_the_paragraph_mark_format() {
        let mut doc=Doc::from_text("");let paragraph=&mut doc.paragraphs[0];paragraph.list=List::Bullet;
        paragraph.runs[0].format=TextFormat{font:"Georgia".into(),size:25.0,color:[11,22,33],..Default::default()};
        let layout=doc.layout();
        assert!(layout.pages[0].items.iter().any(|item|matches!(item,Primitive::TextStyled{text,font,size,color,..} if text=="•" && font=="Georgia" && *size==25.0 && *color==[11,22,33])));
    }
    #[test]
    fn long_numbered_list_markers_end_before_the_body_text() {
        let format=TextFormat::default();let row=Row{paragraph:0,start:0,end:1,x:100.0,y:10.0,height:15.0,page:0,
            marker:"123456789.".into(),segments:vec![Segment{text:"x".into(),format:format.clone(),x:100.0,width:6.0}]};
        let mut page=blank_page(&PageSetup::default(),1);render_row(&mut page,&row);
        let x=page.items.iter().find_map(|item|match item {Primitive::TextStyled{text,x,..} if text==&row.marker=>Some(*x),_=>None}).unwrap_or(f32::INFINITY);
        assert!((x+row.marker.chars().map(|ch|char_width(ch,&format)).sum::<f32>()-(row.x-4.0)).abs()<0.01);
    }
    #[test]
    fn justified_spaces_update_the_position_used_for_later_tab_stops() {
        let format=TextFormat::default();let chars="a \tb".chars().map(|ch|(ch,format.clone())).collect::<Vec<_>>();
        let segments=line_segments(&chars,0,chars.len(),0.0,0.0,0.0,10.0,&format);
        let x=segments.iter().find(|segment|segment.text=="b").map(|segment|segment.x).unwrap_or(f32::INFINITY);
        assert!((x-36.0).abs()<0.01,"{segments:?}");
    }
    #[test]
    fn zero_width_format_characters_do_not_occupy_visible_line_width() {
        let format=TextFormat::default();for ch in ['\u{00ad}','\u{200b}','\u{200c}','\u{200d}','\u{2060}','\u{feff}'] {assert_eq!(char_width(ch,&format),0.0);}
        let doc=Doc::from_text("a\u{00ad}\u{200b}b");let layout=doc.layout();
        assert_eq!(drawn_text(&layout),"ab");
        let width=layout.rows[0].segments.iter().map(|segment|segment.width).sum::<f32>();
        assert!((width-char_width('a',&format)-char_width('b',&format)).abs()<0.01);
    }
    #[test]
    fn table_metadata_cannot_extend_beyond_the_existing_rows()->Result<(),String> {
        let mut doc=Doc::from_text("anchor");doc.add_table(doc.paragraphs[0].id,1,1)?;
        doc.tables[0].header_rows=2;assert!(doc.validate().is_err());doc.tables[0].header_rows=1;
        doc.tables[0].row_heights=vec![10.0,10.0];assert!(doc.validate().is_err());doc.tables[0].row_heights.truncate(1);doc.validate()?;Ok(())
    }
    #[test]
    fn adjacent_paragraph_spacing_uses_the_maximum_gap_in_body_and_cells() {
        let mut doc=small_doc("one\ntwo");doc.paragraphs[0].spacing_after=12.0;doc.paragraphs[1].spacing_before=4.0;
        let layout=doc.layout();assert!((layout.rows[1].y-layout.rows[0].y-32.0).abs()<0.01);
        let cell=Cell::new(99,doc.paragraphs.clone(),[255;3]);let rows=table_lines(&cell,200.0,[0.0;4]);
        assert!((rows[0].height+rows[1].y-32.0).abs()<0.01);
        doc.paragraphs[1].spacing_before=18.0;let layout=doc.layout();assert!((layout.rows[1].y-layout.rows[0].y-38.0).abs()<0.01);
    }
    #[test]
    fn document_import_notices_have_count_item_and_aggregate_budgets() {
        let mut doc=Doc::from_text("text");doc.losses=vec!["notice".into();1001];assert!(doc.validate().is_err());
        doc.losses=vec!["x".repeat(8193)];assert!(doc.validate().is_err());
        doc.losses=vec!["x".repeat(1100);1000];assert!(doc.validate().is_err());doc.losses=vec!["notice".into();2];assert!(doc.validate().is_ok());
    }
    #[test]
    fn document_validation_rejects_xml_controls_but_keeps_supported_text_breaks() {
        let mut doc=Doc::from_text("a\t\u{000b}b\u{000c}c");assert!(doc.validate().is_ok());
        doc.title="bad\0title".into();assert!(doc.validate().is_err());doc.title.clear();
        doc.paragraphs[0].runs[0].text="bad\u{0001}text".into();assert!(doc.validate().is_err());
        doc.paragraphs[0].runs[0].text="valid".into();doc.paragraphs[0].runs[0].format.font="bad\u{000c}font".into();assert!(doc.validate().is_err());
    }
    #[test]
    fn deleting_an_empty_automatic_page_removes_its_whitespace_without_touching_other_text()->Result<(),String> {
        let mut doc=small_doc(&format!("a\u{000b}{}\u{000b}z","\t".repeat(120)));
        doc.page.width=144.0;let before=doc.layout();let pages=before.pages.len();
        let blank=(1..pages.saturating_sub(1)).find(|page|before.rows.iter().any(|row|row.page==*page)
            && before.rows.iter().filter(|row|row.page==*page).all(|row|row.segments.iter().all(|segment|segment.text.chars().all(char::is_whitespace)))).ok_or("blank automatic page")?;
        assert!(doc.delete_empty_page(blank));assert!(doc.layout().pages.len()<pages);
        assert_eq!(doc.plain_text().chars().filter(|ch|!ch.is_whitespace()).collect::<String>(),"az");Ok(())
    }
    #[test]
    fn right_indent_wraps_and_roundtrips()->Result<(),String> {
        let mut doc=Doc::from_text(&"word ".repeat(30));let wide=doc.layout().rows.len();doc.paragraphs[0].right_indent=300.0;
        assert!(doc.layout().rows.len()>wide);
        for ext in ["html","docx"] {let imported=super::super::io::read(&super::super::io::write(&doc,ext)?,ext)?;assert_eq!(imported.paragraphs[0].right_indent,300.0);}
        let imported=super::super::io::read(br#"<p style="margin-right:36pt">text</p>"#,"html")?;assert_eq!(imported.paragraphs[0].right_indent,36.0);
        let native=serde_json::to_value(Paragraph::default()).map_err(|error|error.to_string())?;assert!(native.get("right_indent").is_none());
        let old:Paragraph=serde_json::from_value(serde_json::json!({"id":1})).map_err(|error|error.to_string())?;assert_eq!(old.right_indent,0.0);Ok(())
    }
    #[test]
    fn explicit_list_starts_survive_supported_formats()->Result<(),String> {
        let mut doc=Doc::from_text("first\nsecond");for paragraph in &mut doc.paragraphs {paragraph.list=List::Number;}doc.paragraphs[0].list_start=Some(42);
        let layout=doc.layout();assert_eq!(layout.rows[0].marker,"42.");assert_eq!(layout.rows[1].marker,"43.");
        for ext in ["md","html","docx"] {
            let imported=super::super::io::read(&super::super::io::write(&doc,ext)?,ext)?;let layout=imported.layout();
            assert_eq!(layout.rows[0].marker,"42.","{ext}");assert_eq!(layout.rows[1].marker,"43.","{ext}");
        }
        let mut zero=Doc::from_text("zero\none");for paragraph in &mut zero.paragraphs {paragraph.list=List::Number;}zero.paragraphs[0].list_start=Some(0);
        assert_eq!(zero.layout().rows[0].marker,"0.");assert_eq!(zero.layout().rows[1].marker,"1.");
        let id=doc.paragraphs[0].id;doc.insert_text(id,5,"\nnew")?;assert_eq!(doc.paragraphs[1].list_start,None);
        let html=super::super::io::read(br#"<ol start="7"><li>a</li><li value="12">b</li><li>c</li></ol>"#,"html")?;
        assert_eq!(html.layout().rows.iter().map(|row|row.marker.as_str()).collect::<Vec<_>>(),vec!["7.","12.","13."]);
        let mut ordinary=Doc::from_text(&(0..64).map(|index|format!("item {index}")).collect::<Vec<_>>().join("\n"));
        for paragraph in &mut ordinary.paragraphs {paragraph.list=List::Number;}
        let imported=super::super::io::read(&super::super::io::write(&ordinary,"docx")?,"docx")?;
        let parts=crate::package::read_zip(&super::super::io::write(&imported,"docx")?)?;
        let numbering=String::from_utf8_lossy(parts.get("word/numbering.xml").ok_or("numbering")?);
        assert_eq!(numbering.matches("<w:num w:numId=").count(),2);
        let output=String::from_utf8(super::super::io::write(&imported,"html")?).map_err(|error|error.to_string())?;
        assert_eq!(output.matches("<ol ").count(),1);Ok(())
    }
    #[test]
    fn deleting_a_whole_leading_paragraph_keeps_the_surviving_paragraph_properties()->Result<(),String> {
        let mut doc=Doc::from_text("heading\nbody");let id=doc.paragraphs[0].id;doc.paragraphs[0].set_style("Heading1");
        doc.paragraphs[1].align=Align::Right;doc.paragraphs[1].right_indent=18.0;
        doc.replace_range(Position::default(),Position{paragraph:1,offset:0},"",None)?;
        assert_eq!(doc.plain_text(),"body");assert_eq!(doc.paragraphs[0].style,"Normal");assert_eq!(doc.paragraphs[0].align,Align::Right);assert_eq!(doc.paragraphs[0].right_indent,18.0);assert_eq!(doc.paragraphs[0].id,id);Ok(())
    }
}
