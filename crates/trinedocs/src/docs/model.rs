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
    pub page_break_before: bool,
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
            page_break_before: false,
        }
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
            "Quote" => (11.0, false),
            _ => (11.0, false),
        };
        for run in &mut self.runs {
            run.format.size = size;
            run.format.bold = bold;
            run.format.italic = style == "Quote";
        }
        self.spacing_before = if style.starts_with("Heading") {
            12.0
        } else {
            0.0
        };
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub id: u64,
    pub paragraphs: Vec<Paragraph>,
    pub fill: [u8; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: u64,
    pub after_paragraph_id: u64,
    pub rows: Vec<Vec<Cell>>,
    pub column_widths: Vec<f32>,
    pub border_color: [u8; 3],
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
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub pages: Vec<Page>,
    pub rows: Vec<Row>,
}

pub fn char_width(ch: char, format: &TextFormat) -> f32 {
    static METRICS: std::sync::OnceLock<[std::collections::HashMap<char, f32>; 4]> =
        std::sync::OnceLock::new();
    let metrics = METRICS.get_or_init(|| {
        std::array::from_fn(|index| {
            let mut widths = std::collections::HashMap::new();
            if let Ok(bytes) = crate::fonts::font_bytes(index & 1 != 0, index & 2 != 0)
                && let Ok(face) = ttf_parser::Face::parse(&bytes, 0)
            {
                let units = f32::from(face.units_per_em()).max(1.0);
                if let Some(cmap) = face.tables().cmap {
                    for subtable in cmap.subtables {
                        if subtable.is_unicode() {
                            subtable.codepoints(|code| {
                                if let Some(ch) = char::from_u32(code)
                                    && let Some(advance) =
                                        face.glyph_index(ch).and_then(|g| face.glyph_hor_advance(g))
                                {
                                    widths.insert(ch, f32::from(advance) / units);
                                }
                            });
                        }
                    }
                }
            }
            widths
        })
    });
    let index = usize::from(format.bold) + usize::from(format.italic) * 2;
    let size = format.size.clamp(4.0, 144.0) * if format.script != 0 { 0.75 } else { 1.0 };
    if ch == '\t' {
        return metrics
            .get(index)
            .and_then(|m| m.get(&' '))
            .copied()
            .unwrap_or(0.3)
            * size
            * 4.0;
    }
    metrics
        .get(index)
        .and_then(|m| m.get(&ch))
        .copied()
        .unwrap_or_else(|| {
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
        })
        * size
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
            if f64::from(size * 1.28 * p.line_spacing) > body_height {
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
            estimated_height += line_count * f64::from(size * 1.28 * p.line_spacing)
                + f64::from(p.spacing_before + p.spacing_after);
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
                || table.column_widths.len() > 256
                || table
                    .column_widths
                    .iter()
                    .any(|w| !w.is_finite() || *w <= 0.0 || *w > 10000.0)
            {
                return Err("Недопустимая таблица Docs".into());
            }
            let columns = table.rows.iter().map(Vec::len).max().unwrap_or(1);
            let widths: Vec<f64> = (0..columns)
                .map(|i| {
                    f64::from(
                        table
                            .column_widths
                            .get(i)
                            .copied()
                            .unwrap_or(body_width as f32 / columns as f32)
                            .max(12.0),
                    )
                })
                .collect();
            let raw_sum = widths.iter().sum::<f64>();
            let scale = (body_width / raw_sum).min(1.0);
            for row in &table.rows {
                let mut row_lines = 1.0_f64;
                for (ci, cell) in row.iter().enumerate() {
                    let width = (widths
                        .get(ci)
                        .copied()
                        .unwrap_or(body_width / columns as f64)
                        * scale
                        - 10.0)
                        .max(1.0);
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
                        let line_height = f64::from(size * 1.28 * p.line_spacing);
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
        self.page.margin_left = self.page.margin_left.clamp(0.0, self.page.width * 0.4);
        self.page.margin_right = self.page.margin_right.clamp(0.0, self.page.width * 0.4);
        self.page.margin_top = self.page.margin_top.clamp(0.0, self.page.height * 0.4);
        self.page.margin_bottom = self.page.margin_bottom.clamp(0.0, self.page.height * 0.4);
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
        let fmt = format.unwrap_or_else(|| first.format_at(a.offset));
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
        let deleted_ids: Vec<u64> = self
            .paragraphs
            .iter()
            .skip(a.paragraph + 1)
            .take(b.paragraph.saturating_sub(a.paragraph))
            .map(|p| p.id)
            .collect();
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
                row.push(Cell {
                    id: cell_id,
                    paragraphs: vec![Paragraph {
                        id: paragraph_id,
                        ..Paragraph::default()
                    }],
                    fill: [255, 255, 255],
                });
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
        });
        Ok(id)
    }
    pub fn outline(&self) -> Vec<(u64, String, String)> {
        self.paragraphs
            .iter()
            .filter(|p| p.style.starts_with("Heading") || p.style == "Title")
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
        };
        let mut y = setup.margin_top;
        let mut list_number = 0;
        for (pi, p) in doc.paragraphs.iter().enumerate() {
            if p.page_break_before && (y > setup.margin_top || pi > 0) {
                next_page(&mut layout, setup, &mut y);
            }
            y += p.spacing_before.max(0.0);
            if p.list == List::Number {
                list_number += 1;
            } else if p.list == List::None {
                list_number = 0;
            }
            let marker = match p.list {
                List::Bullet => "•".into(),
                List::Number => format!("{list_number}."),
                List::None => String::new(),
            };
            let indent = p.indent.max(0.0)
                + f32::from(p.list_level) * 18.0
                + if p.list != List::None { 18.0 } else { 0.0 };
            let base_x = setup.margin_left + indent;
            let available = (setup.width - setup.margin_right - base_x).max(12.0);
            let chars: Vec<(char, TextFormat)> = p
                .runs
                .iter()
                .flat_map(|r| r.text.chars().map(move |ch| (ch, r.format.clone())))
                .collect();
            let mut offset = 0;
            let mut first = true;
            loop {
                let x = base_x + if first { p.first_indent } else { 0.0 };
                let max_width = (available - if first { p.first_indent } else { 0.0 }).max(12.0);
                let start = offset;
                let mut end = start;
                let mut width = 0.0;
                let mut break_at = None;
                let mut hard_break = false;
                while let Some((ch, fmt)) = chars.get(end) {
                    if *ch == '\u{000b}' || *ch == '\n' || *ch == '\u{000c}' {
                        hard_break = true;
                        break;
                    }
                    let w = char_width(*ch, fmt);
                    if width + w > max_width && end > start {
                        break;
                    }
                    width += w;
                    end += 1;
                    if ch.is_whitespace() {
                        break_at = Some(end);
                    }
                }
                if end < chars.len()
                    && !hard_break
                    && let Some(word_end) = break_at.filter(|n| *n > start)
                {
                    end = word_end;
                    width = chars
                        .iter()
                        .skip(start)
                        .take(end - start)
                        .map(|(ch, f)| char_width(*ch, f))
                        .sum();
                }
                let max_size = chars
                    .iter()
                    .skip(start)
                    .take(end.saturating_sub(start))
                    .map(|(_, f)| f.size)
                    .fold(p.format_at(start).size.max(4.0), f32::max);
                let height = (max_size * 1.28 * p.line_spacing).max(8.0);
                if y + height > setup.height - setup.margin_bottom {
                    next_page(&mut layout, setup, &mut y);
                }
                let align_x = match p.align {
                    Align::Center => x + (max_width - width).max(0.0) / 2.0,
                    Align::Right => x + (max_width - width).max(0.0),
                    _ => x,
                };
                let mut segments: Vec<Segment> = Vec::new();
                let mut cursor = align_x;
                for (ch, format) in chars.iter().skip(start).take(end.saturating_sub(start)) {
                    let cw = char_width(*ch, format);
                    if let Some(segment) = segments.last_mut().filter(|s| s.format == *format) {
                        segment.text.push(*ch);
                        segment.width += cw;
                    } else {
                        segments.push(Segment {
                            text: ch.to_string(),
                            format: format.clone(),
                            x: cursor,
                            width: cw,
                        });
                    }
                    cursor += cw;
                }
                let row_marker = if first { marker.clone() } else { String::new() };
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
                let page_break =
                    hard_break && chars.get(end).is_some_and(|(ch, _)| *ch == '\u{000c}');
                offset = end + usize::from(hard_break);
                if page_break {
                    next_page(&mut layout, setup, &mut y);
                }
                first = false;
                if offset >= chars.len() && !hard_break {
                    break;
                }
            }
            y += p.spacing_after.max(0.0);
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
        ]
        .iter()
        .any(|v| !v.is_finite())
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
        page.items.push(Primitive::Text {
            x: row.x - 18.0,
            y: row.y,
            text: row.marker.clone(),
            size: 11.0,
            color: [24, 28, 35],
        });
    }
    for seg in &row.segments {
        if let Some(color) = seg.format.highlight {
            page.items.push(Primitive::Rect {
                x: seg.x,
                y: row.y,
                w: seg.width,
                h: row.height,
                color,
            });
        }
        page.items.push(Primitive::TextStyled {
            x: seg.x,
            y: row.y
                + if seg.format.script > 0 {
                    -3.0
                } else if seg.format.script < 0 {
                    3.0
                } else {
                    0.0
                },
            text: seg.text.clone(),
            size: seg.format.size * if seg.format.script != 0 { 0.75 } else { 1.0 },
            color: seg.format.color,
            bold: seg.format.bold,
            italic: seg.format.italic,
        });
        if seg.format.underline || seg.format.link.is_some() {
            page.items.push(Primitive::Line {
                x1: seg.x,
                y1: row.y + seg.format.size * 1.13,
                x2: seg.x + seg.width,
                y2: row.y + seg.format.size * 1.13,
                width: 0.7,
                color: seg.format.color,
            });
        }
        if seg.format.strike {
            page.items.push(Primitive::Line {
                x1: seg.x,
                y1: row.y + seg.format.size * 0.6,
                x2: seg.x + seg.width,
                y2: row.y + seg.format.size * 0.6,
                width: 0.7,
                color: seg.format.color,
            });
        }
    }
}
fn table_lines(cell: &Cell, width: f32) -> Vec<Row> {
    let mut lines = Vec::new();
    for (paragraph, p) in cell.paragraphs.iter().enumerate() {
        let available =
            (width - 10.0 - p.indent.max(0.0) - f32::from(p.list_level) * 18.0).max(1.0);
        let chars: Vec<(char, TextFormat)> = p
            .runs
            .iter()
            .flat_map(|r| r.text.chars().map(move |ch| (ch, r.format.clone())))
            .collect();
        let mut start = 0;
        let mut first = true;
        loop {
            let indent = if first { p.first_indent } else { 0.0 };
            let max_width = (available - indent).max(1.0);
            let mut end = start;
            let mut used = 0.0;
            let mut break_at = None;
            let mut hard_break = false;
            while let Some((ch, format)) = chars.get(end) {
                if matches!(*ch, '\u{000b}' | '\u{000c}' | '\n') {
                    hard_break = true;
                    break;
                }
                let w = char_width(*ch, format);
                if used + w > max_width && end > start {
                    break;
                }
                used += w;
                end += 1;
                if ch.is_whitespace() {
                    break_at = Some(end);
                }
            }
            if end < chars.len()
                && !hard_break
                && let Some(word_end) = break_at.filter(|n| *n > start)
            {
                end = word_end;
                used = chars
                    .iter()
                    .skip(start)
                    .take(end - start)
                    .map(|(ch, f)| char_width(*ch, f))
                    .sum();
            }
            let max_size = chars
                .iter()
                .skip(start)
                .take(end - start)
                .map(|(_, f)| f.size)
                .fold(p.format_at(start).size, f32::max);
            let text_height = (max_size * 1.28 * p.line_spacing).max(8.0);
            let before = if first { p.spacing_before } else { 0.0 };
            let final_line = end + usize::from(hard_break) >= chars.len() && !hard_break;
            let after = if final_line { p.spacing_after } else { 0.0 };
            let x = 5.0
                + p.indent.max(0.0)
                + f32::from(p.list_level) * 18.0
                + indent
                + match p.align {
                    Align::Center => (max_width - used).max(0.0) / 2.0,
                    Align::Right => (max_width - used).max(0.0),
                    _ => 0.0,
                };
            let mut segments: Vec<Segment> = Vec::new();
            let mut cursor = x;
            for (ch, format) in chars.iter().skip(start).take(end - start) {
                let cw = char_width(*ch, format);
                if let Some(segment) = segments.last_mut().filter(|s| s.format == *format) {
                    segment.text.push(*ch);
                    segment.width += cw;
                } else {
                    segments.push(Segment {
                        text: ch.to_string(),
                        format: format.clone(),
                        x: cursor,
                        width: cw,
                    });
                }
                cursor += cw;
            }
            lines.push(Row {
                paragraph,
                start,
                end,
                x,
                y: before,
                height: before + text_height + after,
                page: 0,
                segments,
                marker: String::new(),
            });
            start = end + usize::from(hard_break);
            first = false;
            if start >= chars.len() && !hard_break {
                break;
            }
        }
    }
    lines
}
fn render_table(layout: &mut Layout, setup: &PageSetup, table: &Table, y: &mut f32) {
    let usable_width = setup.width - setup.margin_left - setup.margin_right;
    let columns = table.rows.iter().map(Vec::len).max().unwrap_or_default();
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
                .max(12.0)
        })
        .collect();
    let raw_sum: f32 = widths.iter().sum();
    if raw_sum > usable_width {
        for width in &mut widths {
            *width *= usable_width / raw_sum;
        }
    }
    for cells in &table.rows {
        let cell_lines: Vec<Vec<Row>> = cells
            .iter()
            .enumerate()
            .map(|(ci, cell)| {
                table_lines(
                    cell,
                    widths
                        .get(ci)
                        .copied()
                        .unwrap_or(usable_width / columns as f32),
                )
            })
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
        let full_height = heights.iter().sum::<f32>() + 10.0;
        if full_height <= body_height && *y + full_height > setup.height - setup.margin_bottom {
            next_page(layout, setup, y);
        }
        let mut line_offset = 0;
        while line_offset < line_total {
            let available = setup.height - setup.margin_bottom - *y;
            let first_height = heights.get(line_offset).copied().unwrap_or(8.0);
            if available < first_height + 10.0 && *y > setup.margin_top {
                next_page(layout, setup, y);
                continue;
            }
            let mut count = 0;
            let mut row_height = 10.0;
            for height in heights.iter().skip(line_offset) {
                if count > 0 && row_height + height > available {
                    break;
                }
                row_height += height;
                count += 1;
            }
            if let Some(page) = layout.pages.last_mut() {
                let mut x = setup.margin_left;
                for (ci, cell) in cells.iter().enumerate() {
                    let width = widths
                        .get(ci)
                        .copied()
                        .unwrap_or(usable_width / columns as f32);
                    page.items.push(Primitive::Rect {
                        x,
                        y: *y,
                        w: width,
                        h: row_height,
                        color: cell.fill,
                    });
                    for (a, b, c, d) in [
                        (x, *y, x + width, *y),
                        (x, *y + row_height, x + width, *y + row_height),
                        (x, *y, x, *y + row_height),
                        (x + width, *y, x + width, *y + row_height),
                    ] {
                        page.items.push(Primitive::Line {
                            x1: a,
                            y1: b,
                            x2: c,
                            y2: d,
                            width: 0.7,
                            color: table.border_color,
                        });
                    }
                    let mut slot_y = *y + 5.0;
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
                    x += width;
                }
            }
            *y += row_height;
            line_offset += count;
            if line_offset < line_total {
                next_page(layout, setup, y);
            }
        }
    }
    *y += 8.0;
}

#[cfg(test)]
mod tests {
    use super::*;
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
