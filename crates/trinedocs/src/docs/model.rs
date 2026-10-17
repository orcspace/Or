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
    pub indent: f32,
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
            indent: 0.0,
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
    pub bytes: crate::blob::Blob,
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
    if nested.replace_range(a, b, &inserted, None).is_ok() {
        cell.paragraphs = nested.paragraphs;
        *next_id = nested.next_id;
    }
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
                - f64::from(p.indent.max(0.0) + p.first_indent.max(0.0))
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
            let columns = table.grid_columns().max(1);
            let widths: Vec<f64> = (0..columns)
                .map(|i| {
                    f64::from(
                        table
                            .column_widths
                            .get(i)
                            .copied()
                            .unwrap_or(body_width as f32 / columns as f32)
                            .max(1.0),
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
                        if line_height + f64::from(p.spacing_before + p.spacing_after) + 10.0
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
                        cell_lines += ((p.len() as f64 * f64::from(size) / width).ceil().max(1.0)
                            + hard_breaks as f64)
                            * line_height
                            / 15.0
                            + f64::from(p.spacing_before + p.spacing_after) / 15.0;
                    }
                    row_lines = row_lines.max(cell_lines);
                }
                estimated_height += row_lines * 15.0 + 10.0;
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
        self.plain_text().split_whitespace().count()
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
            .cloned()
            .ok_or("Начальный абзац не найден")?;
        let last = self
            .paragraphs
            .get(b.paragraph)
            .cloned()
            .ok_or("Конечный абзац не найден")?;
        if a.offset > first.len() || b.offset > last.len() {
            return Err("Недопустимый диапазон текста".into());
        }
        let fmt = format.unwrap_or_else(|| {
            // Insertion follows the character before the caret; replacement follows
            // the first deleted character, including matches at a run boundary.
            first.format_at(a.offset + usize::from(a != b))
        });
        let prefix = first.slice(0, a.offset);
        let suffix = last.slice(b.offset, last.len());
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();
        let mut replacement = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let mut p = first.clone();
            if i > 0 {
                p.id = self.alloc_id();
                p.page_break_before = false;
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
    pub fn replace_all(&mut self, needle: &str, replacement: &str, case_sensitive: bool) -> usize {
        let mut matches = self.find(needle, case_sensitive);
        // Only non-overlapping matches are replaced, matching Word's replace-all behaviour.
        let mut previous: Option<Position> = None;
        matches.retain(|(a, b)| {
            let keep = previous.is_none_or(|end| *a >= end);
            if keep {
                previous = Some(*b);
            }
            keep
        });
        let mut count = 0;
        for (a, b) in matches.into_iter().rev() {
            if self.replace_range(a, b, replacement, None).is_ok() {
                count += 1;
            }
        }
        count
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
                    table.rows.remove(at);
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
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                spans.sort_unstable();
                spans.dedup();
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
        edit_cell_text(cell, text, &mut next);
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
            let (before, after) = paragraph_spacing(p, prev, next);
            if p.page_break_before && (y > setup.margin_top || pi > 0) {
                next_page(&mut layout, setup, &mut y);
            }
            let marker = list_marker(p, &mut list_numbers);
            let indent = p.indent + list_indent(p);
            let base_x = setup.margin_left + indent;
            let available = (setup.width - setup.margin_right - base_x).max(12.0);
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
                if p.keep_next
                    && breaks.is_empty()
                    && let Some(n) = next
                {
                    let first = n.format_at(0);
                    let need = before
                        + total
                        + after
                        + paragraph_spacing(n, Some(p), doc.paragraphs.get(pi + 2)).0
                        + paragraph_line_height(n, &first, first.size.max(4.0));
                    if y + need > bottom && need <= body {
                        breaks = vec![0];
                    }
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
                let scale = (max_w / image.width.max(1.0))
                    .min(max_h / image.height.max(1.0))
                    .min(1.0);
                let width = image.width.max(1.0) * scale;
                let height = image.height.max(1.0) * scale;
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
        || [
            p.indent,
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
        || p.first_indent.abs() > 10000.0
        || !(0.0..=1000.0).contains(&p.spacing_before)
        || !(0.0..=1000.0).contains(&p.spacing_after)
        || !(0.5..=5.0).contains(&p.line_spacing)
    {
        return Err("Недопустимое оформление абзаца".into());
    }
    for run in &p.runs {
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
            x: row.x - 18.0,
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
                text: seg.text.clone(),
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
    (
        if same(prev) {
            0.0
        } else {
            p.spacing_before.max(0.0)
        },
        if same(next) {
            0.0
        } else {
            p.spacing_after.max(0.0)
        },
    )
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
        let format = p.format_at(start);
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
) -> Vec<Segment> {
    let line = chars.get(start..end).unwrap_or_default();
    let content = line.len() - line.iter().rev().take_while(|(ch, _)| *ch == ' ').count();
    let mut segments: Vec<Segment> = Vec::new();
    let mut cursor = x;
    let mut pos = origin;
    let mut alone = true;
    for (i, (ch, format)) in line.iter().enumerate() {
        let mut width = advance(*ch, format, pos, hang);
        pos += width;
        let wide = justify > 0.0 && *ch == ' ' && i < content;
        if wide {
            width += justify;
        }
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
fn list_marker(p: &Paragraph, numbers: &mut [usize; 9]) -> String {
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
            numbers[level] += 1;
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
        let available = (width - margin[1] - margin[3] - indent).max(1.0);
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
                .max(1.0)
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
                .max(1.0)
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
    let cell_lines: Vec<Vec<Row>> = cells
        .iter()
        .enumerate()
        .map(|(ci, cell)| table_lines(cell, cell_widths.get(ci).copied().unwrap_or(12.0), margin))
        .collect();
    let line_total = cell_lines.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let heights: Vec<f32> = (0..line_total)
        .map(|i| {
            cell_lines
                .iter()
                .filter_map(|lines| lines.get(i))
                .map(|row| row.height)
                .fold(8.0_f32, f32::max)
        })
        .collect();
    let body_height = setup.height - setup.margin_top - setup.margin_bottom;
    let min_height = table
        .row_heights
        .get(row_index)
        .copied()
        .filter(|h| h.is_finite())
        .unwrap_or_default()
        .clamp(0.0, body_height);
    let full_height = (heights.iter().sum::<f32>() + pad).max(min_height);
    if full_height <= body_height && *y + full_height > setup.height - setup.margin_bottom {
        table_page(layout, setup, table, geometry, y, repeat);
    }
    let mut line_offset = 0;
    // Header rows that leave no room for the row are not repeated again (no endless pages).
    let mut stalled = false;
    while line_offset < line_total {
        let available = setup.height - setup.margin_bottom - *y;
        let first_height = heights.get(line_offset).copied().unwrap_or(8.0);
        if available < first_height + pad && *y > setup.margin_top {
            table_page(layout, setup, table, geometry, y, repeat && !stalled);
            stalled = true;
            continue;
        }
        stalled = false;
        let mut count = 0;
        let mut row_height = pad;
        for height in heights.iter().skip(line_offset) {
            if count > 0 && row_height + height > available {
                break;
            }
            row_height += height;
            count += 1;
        }
        if line_offset == 0 && count == line_total {
            row_height = row_height.max(min_height);
        }
        let page_index = layout.pages.len().saturating_sub(1);
        if let Some(page) = layout.pages.last_mut() {
            for (ci, cell) in cells.iter().enumerate() {
                let x = setup.margin_left
                    + widths
                        .iter()
                        .take(starts.get(ci).copied().unwrap_or_default())
                        .sum::<f32>();
                let width = cell_widths.get(ci).copied().unwrap_or(12.0);
                layout.cells.push(CellBox {
                    table: table.id,
                    row: row_index,
                    column: ci,
                    page: page_index,
                    x,
                    y: *y,
                    w: width,
                    h: row_height,
                });
                page.items.push(Primitive::Rect {
                    x,
                    y: *y,
                    w: width,
                    h: row_height,
                    color: cell.fill,
                });
                let edges = [
                    (!cell.merged, (x, *y, x + width, *y)),
                    (
                        !continued.get(ci).copied().unwrap_or_default(),
                        (x, *y + row_height, x + width, *y + row_height),
                    ),
                    (true, (x, *y, x, *y + row_height)),
                    (true, (x + width, *y, x + width, *y + row_height)),
                ];
                for (visible, (a, b, c, d)) in edges {
                    if !visible || table.borderless {
                        continue;
                    }
                    page.items.push(Primitive::Line {
                        x1: a,
                        y1: b,
                        x2: c,
                        y2: d,
                        width: 0.7,
                        color: table.border_color,
                    });
                }
                let content: f32 = (line_offset..line_offset + count)
                    .filter(|li| cell_lines.get(ci).is_some_and(|lines| *li < lines.len()))
                    .map(|li| heights.get(li).copied().unwrap_or(8.0))
                    .sum();
                let free = (row_height - pad - content).max(0.0);
                let mut slot_y = *y
                    + margin[0]
                    + match cell.v_align {
                        VAlign::Top => 0.0,
                        VAlign::Center => free / 2.0,
                        VAlign::Bottom => free,
                    };
                for li in line_offset..line_offset + count {
                    if let Some(row) = cell_lines.get(ci).and_then(|lines| lines.get(li)) {
                        let mut positioned = row.clone();
                        positioned.y = slot_y + row.y;
                        positioned.x += x;
                        for segment in &mut positioned.segments {
                            segment.x += x;
                        }
                        render_row(page, &positioned);
                    }
                    slot_y += heights.get(li).copied().unwrap_or(8.0);
                }
            }
        }
        *y += row_height;
        line_offset += count;
        if line_offset < line_total {
            table_page(layout, setup, table, geometry, y, repeat);
        }
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
