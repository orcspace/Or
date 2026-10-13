use crate::render::{Page, Primitive};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type PageCells<'a> = BTreeMap<(u32, u32), Vec<(u32, u32, &'a Cell)>>;

pub const MAX_ROWS: u32 = 1_048_576;
pub const MAX_COLS: u32 = 16_384;
pub const MAX_RANGE_CELLS: usize = 100_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum Value {
    #[default]
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    Error(String),
}
impl Value {
    pub fn display(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Number(n) => {
                if n.is_finite() {
                    format!("{n}")
                } else {
                    "#NUM!".into()
                }
            }
            Self::Text(s) | Self::Error(s) => s.clone(),
            Self::Bool(b) => if *b { "TRUE" } else { "FALSE" }.into(),
        }
    }
    pub fn number(&self) -> Result<f64, String> {
        match self {
            Self::Empty => Ok(0.0),
            Self::Number(n) => Ok(*n),
            Self::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Self::Text(s) => {
                let n: f64 = s
                    .trim()
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| "#VALUE!".to_string())?;
                if n.is_finite() {
                    Ok(n)
                } else {
                    Err("#NUM!".into())
                }
            }
            Self::Error(s) => Err(s.clone()),
        }
    }
    pub fn boolean(&self) -> Result<bool, String> {
        match self {
            Self::Bool(b) => Ok(*b),
            Self::Text(s) if s.eq_ignore_ascii_case("true") => Ok(true),
            Self::Text(s) if s.eq_ignore_ascii_case("false") => Ok(false),
            _ => self.number().map(|v| v != 0.0),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    General,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CellFormat {
    pub number: String,
    pub font: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: [u8; 3],
    pub fill: Option<[u8; 3]>,
    pub border: bool,
    pub align: Align,
    pub wrap: bool,
    pub rotation: i16,
}
impl Default for CellFormat {
    fn default() -> Self {
        Self {
            number: "General".into(),
            font: "Calibri".into(),
            size: 11.0,
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            color: [28, 32, 38],
            fill: None,
            border: false,
            align: Align::General,
            wrap: false,
            rotation: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Cell {
    pub id: u64,
    pub input: String,
    pub value: Value,
    pub fmt: CellFormat,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Range {
    pub r0: u32,
    pub c0: u32,
    pub r1: u32,
    pub c1: u32,
}
impl Range {
    pub fn new(r0: u32, c0: u32, r1: u32, c1: u32) -> Self {
        Self {
            r0: r0.min(r1),
            c0: c0.min(c1),
            r1: r0.max(r1),
            c1: c0.max(c1),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.r1 >= MAX_ROWS || self.c1 >= MAX_COLS || self.r0 > self.r1 || self.c0 > self.c1 {
            return Err("Недопустимый диапазон".into());
        }
        let count = u64::from(self.r1 - self.r0 + 1) * u64::from(self.c1 - self.c0 + 1);
        if count > MAX_RANGE_CELLS as u64 {
            return Err("Диапазон превышает 100 000 ячеек".into());
        }
        Ok(())
    }
    pub fn contains(&self, r: u32, c: u32) -> bool {
        r >= self.r0 && r <= self.r1 && c >= self.c0 && c <= self.c1
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Selection {
    pub row: u32,
    pub col: u32,
    pub anchor_row: u32,
    pub anchor_col: u32,
}
impl Selection {
    pub fn range(&self) -> Range {
        Range::new(self.row, self.col, self.anchor_row, self.anchor_col)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ChartKind {
    #[default]
    Column,
    Bar,
    Line,
    Area,
    Pie,
    Doughnut,
    Scatter,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Chart {
    pub id: u64,
    pub title: String,
    pub kind: ChartKind,
    pub range: Range,
    pub row: u32,
    pub col: u32,
    pub width: f32,
    pub height: f32,
}
/// A picture floating over the grid, anchored to a cell with a pixel offset (like Excel).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SheetImage {
    pub id: u64,
    pub row: u32,
    pub col: u32,
    #[serde(default)]
    pub x_offset: f32,
    #[serde(default)]
    pub y_offset: f32,
    pub width: f32,
    pub height: f32,
    pub bytes: Vec<u8>,
    #[serde(default)]
    pub alt: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConditionalRule {
    pub range: Range,
    pub operator: String,
    pub threshold: f64,
    pub fill: [u8; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sheet {
    pub id: u64,
    pub name: String,
    pub cells: BTreeMap<String, Cell>,
    pub widths: BTreeMap<u32, f32>,
    pub row_heights: BTreeMap<u32, f32>,
    pub hidden_rows: BTreeSet<u32>,
    pub hidden_cols: BTreeSet<u32>,
    pub merges: Vec<Range>,
    pub frozen_rows: u32,
    pub frozen_cols: u32,
    pub charts: Vec<Chart>,
    pub images: Vec<SheetImage>,
    pub conditional: Vec<ConditionalRule>,
    pub hidden: bool,
    pub tab_color: Option<[u8; 3]>,
    pub filter: Option<Range>,
}
impl Default for Sheet {
    fn default() -> Self {
        Self {
            id: 1,
            name: "Лист1".into(),
            cells: BTreeMap::new(),
            widths: BTreeMap::new(),
            row_heights: BTreeMap::new(),
            hidden_rows: BTreeSet::new(),
            hidden_cols: BTreeSet::new(),
            merges: Vec::new(),
            frozen_rows: 0,
            frozen_cols: 0,
            charts: Vec::new(),
            images: Vec::new(),
            conditional: Vec::new(),
            hidden: false,
            tab_color: None,
            filter: None,
        }
    }
}
impl Sheet {
    pub fn cell(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.get(&address(row, col))
    }
    pub fn cell_mut(&mut self, row: u32, col: u32) -> Option<&mut Cell> {
        self.cells.get_mut(&address(row, col))
    }
    pub fn extent(&self) -> (u32, u32) {
        self.cells
            .keys()
            .filter_map(|k| parse_address(k))
            .fold((0, 0), |(r, c), (rr, cc)| (r.max(rr), c.max(cc)))
    }
    pub fn display(&self, row: u32, col: u32, locale_ru: bool) -> String {
        self.cell(row, col)
            .map(|c| super::format::display(&c.value, &c.fmt.number, locale_ru))
            .unwrap_or_default()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Book {
    pub sheets: Vec<Sheet>,
    pub active: usize,
    pub next_id: u64,
    pub selection: Selection,
    pub names: BTreeMap<String, String>,
    pub locale_ru: bool,
}
impl Default for Book {
    fn default() -> Self {
        Self {
            sheets: vec![Sheet::default()],
            active: 0,
            next_id: 2,
            selection: Selection::default(),
            names: BTreeMap::new(),
            locale_ru: true,
        }
    }
}
impl Book {
    pub fn validate(&self) -> Result<(), String> {
        if self.sheets.is_empty()
            || self.sheets.len() > 1024
            || self.active >= self.sheets.len()
            || self.selection.row >= MAX_ROWS
            || self.selection.col >= MAX_COLS
            || self.selection.anchor_row >= MAX_ROWS
            || self.selection.anchor_col >= MAX_COLS
        {
            return Err("Недопустимая книга или выделение".into());
        }
        let mut ids = BTreeSet::new();
        let mut count = 0usize;
        let mut names = BTreeSet::new();
        let mut take = |id: u64| -> Result<(), String> {
            if id == 0 || id == u64::MAX || !ids.insert(id) {
                Err("Нулевой или повторяющийся ID объекта".into())
            } else {
                Ok(())
            }
        };
        for s in &self.sheets {
            take(s.id)?;
            self.validate_name(&s.name, Some(s.id))?;
            if !names.insert(s.name.to_lowercase()) {
                return Err("Повторяющиеся имена листов".into());
            }
            count = count
                .checked_add(s.cells.len())
                .ok_or("Слишком большая книга")?;
            if count > 1_000_000
                || s.charts.len() > 10000
                || s.conditional.len() > 10000
                || s.merges.len() > 100000
            {
                return Err("Слишком сложная книга".into());
            }
            for (key, c) in &s.cells {
                take(c.id)?;
                if parse_address(key).is_none_or(|(r, col)| address(r, col) != *key)
                    || c.input.len() > 32767
                    || c.fmt.font.len() > 256
                    || !c.fmt.size.is_finite()
                    || !(1.0..=409.0).contains(&c.fmt.size)
                    || c.fmt.number.len() > 1024
                    || !(-90..=90).contains(&c.fmt.rotation)
                    || matches!(&c.value,Value::Number(n)if !n.is_finite())
                    || matches!(&c.value,Value::Text(t)|Value::Error(t)if t.len()>32767)
                {
                    return Err("Недопустимая ячейка или формат".into());
                }
            }
            if s.widths
                .iter()
                .any(|(k, v)| *k >= MAX_COLS || !v.is_finite() || *v < 1.0 || *v > 1800.0)
                || s.row_heights
                    .iter()
                    .any(|(k, v)| *k >= MAX_ROWS || !v.is_finite() || *v < 1.0 || *v > 600.0)
                || s.hidden_rows.iter().any(|r| *r >= MAX_ROWS)
                || s.hidden_cols.iter().any(|c| *c >= MAX_COLS)
                || s.frozen_rows >= MAX_ROWS
                || s.frozen_cols >= MAX_COLS
            {
                return Err("Недопустимые размеры или закрепление листа".into());
            }
            validate_merges(&s.merges)?;
            if let Some(f) = s.filter {
                f.validate()?;
            }
            for chart in &s.charts {
                take(chart.id)?;
                chart.range.validate()?;
                if chart.row >= MAX_ROWS
                    || chart.col >= MAX_COLS
                    || !chart.width.is_finite()
                    || !chart.height.is_finite()
                    || !(1.0..=4000.0).contains(&chart.width)
                    || !(1.0..=4000.0).contains(&chart.height)
                    || chart.title.len() > 32767
                {
                    return Err("Недопустимая диаграмма".into());
                }
            }
            if s.images.len() > 1000 {
                return Err("Слишком много рисунков на листе".into());
            }
            for image in &s.images {
                take(image.id)?;
                if image.row >= MAX_ROWS
                    || image.col >= MAX_COLS
                    || ![image.x_offset, image.y_offset, image.width, image.height]
                        .iter()
                        .all(|v| v.is_finite())
                    || !(0.0..=4000.0).contains(&image.x_offset)
                    || !(0.0..=4000.0).contains(&image.y_offset)
                    || !(1.0..=8000.0).contains(&image.width)
                    || !(1.0..=8000.0).contains(&image.height)
                    || image.bytes.is_empty()
                    || image.bytes.len() > 32 * 1024 * 1024
                    || image.alt.len() > 32767
                {
                    return Err("Недопустимый рисунок".into());
                }
            }
            for rule in &s.conditional {
                rule.range.validate()?;
                if !rule.threshold.is_finite()
                    || !matches!(rule.operator.as_str(), ">" | "<" | "=" | ">=" | "<=")
                {
                    return Err("Недопустимое условное форматирование".into());
                }
            }
        }
        if self.next_id == 0
            || self.next_id == u64::MAX
            || ids.last().is_some_and(|id| *id >= self.next_id)
        {
            return Err("Недопустимый счётчик ID".into());
        }
        if self.names.len() > 10000
            || self
                .names
                .iter()
                .any(|(name, value)| name.is_empty() || name.len() > 255 || value.len() > 32767)
        {
            return Err("Недопустимое имя диапазона".into());
        }
        Ok(())
    }
    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id.saturating_add(1);
        id
    }
    pub fn sheet_by_id(&self, id: u64) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }
    pub fn sheet_by_id_mut(&mut self, id: u64) -> Option<&mut Sheet> {
        self.sheets.iter_mut().find(|s| s.id == id)
    }
    pub fn active_sheet(&self) -> Option<&Sheet> {
        self.sheets.get(self.active)
    }
    pub fn active_sheet_mut(&mut self) -> Option<&mut Sheet> {
        self.sheets.get_mut(self.active)
    }
    pub fn active_id(&self) -> u64 {
        self.active_sheet().map(|s| s.id).unwrap_or(0)
    }
    pub fn add_sheet(&mut self, name: &str) -> Result<u64, String> {
        self.validate_name(name, None)?;
        let id = self.allocate_id();
        self.sheets.push(Sheet {
            id,
            name: name.into(),
            ..Sheet::default()
        });
        Ok(id)
    }
    fn validate_name(&self, name: &str, except: Option<u64>) -> Result<(), String> {
        if name.is_empty()
            || name.chars().count() > 31
            || name.chars().any(|c| "[]:*?/\\".contains(c))
            || name.starts_with('\'')
            || name.ends_with('\'')
        {
            return Err("Недопустимое имя листа".into());
        }
        if self
            .sheets
            .iter()
            .any(|s| Some(s.id) != except && s.name.to_lowercase() == name.to_lowercase())
        {
            return Err("Лист с таким именем уже существует".into());
        }
        Ok(())
    }
    pub fn rename_sheet(&mut self, id: u64, name: &str) -> Result<(), String> {
        self.validate_name(name, Some(id))?;
        let old = self.sheet_by_id(id).ok_or("Лист не найден")?.name.clone();
        for s in &mut self.sheets {
            for c in s.cells.values_mut() {
                if c.input.starts_with('=') {
                    c.input = super::formula::rename_sheet(&c.input, &old, name);
                }
            }
        }
        for value in self.names.values_mut() {
            *value = super::formula::rename_sheet(value, &old, name);
        }
        self.sheet_by_id_mut(id).ok_or("Лист не найден")?.name = name.into();
        self.recalculate();
        Ok(())
    }
    pub fn delete_sheet(&mut self, id: u64) -> Result<(), String> {
        if self.sheets.len() <= 1 {
            return Err("Нельзя удалить последний лист".into());
        }
        let pos = self
            .sheets
            .iter()
            .position(|s| s.id == id)
            .ok_or("Лист не найден")?;
        let removed = self.sheets.remove(pos);
        if self.active > pos {
            self.active -= 1;
        }
        self.active = self.active.min(self.sheets.len().saturating_sub(1));
        for s in &mut self.sheets {
            for cell in s.cells.values_mut() {
                cell.input = super::formula::invalidate_sheet(&cell.input, &removed.name);
            }
        }
        for value in self.names.values_mut() {
            *value = super::formula::invalidate_sheet(value, &removed.name);
        }
        self.recalculate();
        Ok(())
    }
    pub fn move_sheet(&mut self, id: u64, index: usize) -> Result<(), String> {
        if index >= self.sheets.len() {
            return Err("Недопустимый индекс листа".into());
        }
        let pos = self
            .sheets
            .iter()
            .position(|s| s.id == id)
            .ok_or("Лист не найден")?;
        let s = self.sheets.remove(pos);
        self.sheets.insert(index, s);
        self.active = index;
        Ok(())
    }
    pub fn set_cell(&mut self, sheet: u64, row: u32, col: u32, input: &str) -> Result<u64, String> {
        let id = self.set_input(sheet, row, col, input)?;
        self.recalculate();
        Ok(id)
    }
    pub fn set_input(
        &mut self,
        sheet: u64,
        row: u32,
        col: u32,
        input: &str,
    ) -> Result<u64, String> {
        if row >= MAX_ROWS || col >= MAX_COLS || input.len() > 32767 {
            return Err("Ячейка вне пределов Excel или слишком длинный текст".into());
        }
        let key = address(row, col);
        let existing = self
            .sheet_by_id(sheet)
            .ok_or("Лист не найден")?
            .cells
            .get(&key)
            .map(|c| c.id);
        let id = existing.unwrap_or_else(|| self.allocate_id());
        let normalized = super::formula::canonicalize(input);
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        let c = s.cells.entry(key).or_insert_with(|| Cell {
            id,
            ..Cell::default()
        });
        c.input = normalized;
        Ok(id)
    }
    pub fn set_range(
        &mut self,
        sheet: u64,
        row: u32,
        col: u32,
        values: &[Vec<String>],
    ) -> Result<(), String> {
        if self.sheet_by_id(sheet).is_none() {
            return Err("Лист не найден".into());
        }
        let count = values.iter().map(Vec::len).sum::<usize>();
        if count > MAX_RANGE_CELLS
            || u64::from(row) + values.len() as u64 > u64::from(MAX_ROWS)
            || values.iter().any(|v| {
                u64::from(col) + v.len() as u64 > u64::from(MAX_COLS)
                    || v.iter().any(|x| x.len() > 32767)
            })
        {
            return Err("Недопустимый или слишком большой диапазон".into());
        }
        for (r, line) in values.iter().enumerate() {
            for (c, input) in line.iter().enumerate() {
                self.set_input(sheet, row + r as u32, col + c as u32, input)?;
            }
        }
        self.recalculate();
        Ok(())
    }
    pub fn read_range(
        &self,
        sheet: u64,
        r0: u32,
        c0: u32,
        r1: u32,
        c1: u32,
    ) -> Result<Vec<Vec<String>>, String> {
        let range = Range::new(r0, c0, r1, c1);
        range.validate()?;
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        Ok((range.r0..=range.r1)
            .map(|r| {
                (range.c0..=range.c1)
                    .map(|c| s.display(r, c, self.locale_ru))
                    .collect()
            })
            .collect())
    }
    pub fn set_format(&mut self, sheet: u64, range: Range, fmt: CellFormat) -> Result<(), String> {
        range.validate()?;
        for r in range.r0..=range.r1 {
            for c in range.c0..=range.c1 {
                if self.sheet_by_id(sheet).and_then(|s| s.cell(r, c)).is_none() {
                    self.set_input(sheet, r, c, "")?;
                }
                if let Some(cell) = self.sheet_by_id_mut(sheet).and_then(|s| s.cell_mut(r, c)) {
                    cell.fmt = fmt.clone();
                }
            }
        }
        Ok(())
    }
    pub fn merge(&mut self, sheet: u64, range: Range) -> Result<(), String> {
        range.validate()?;
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        if s.merges
            .iter()
            .any(|m| m.r0 <= range.r1 && range.r0 <= m.r1 && m.c0 <= range.c1 && range.c0 <= m.c1)
        {
            return Err("Объединённые диапазоны пересекаются".into());
        }
        if s.cells.iter().any(|(k, c)| {
            parse_address(k).is_some_and(|(r, col)| {
                range.contains(r, col) && (r != range.r0 || col != range.c0) && !c.input.is_empty()
            })
        }) {
            return Err("Объединение удалит данные: сначала очистите остальные ячейки".into());
        }
        s.merges.push(range);
        Ok(())
    }
    /// Removes every merged area that intersects `range`.
    pub fn unmerge(&mut self, sheet: u64, range: Range) -> Result<usize, String> {
        range.validate()?;
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        let before = s.merges.len();
        s.merges.retain(|m| {
            !(m.r0 <= range.r1 && range.r0 <= m.r1 && m.c0 <= range.c1 && range.c0 <= m.c1)
        });
        Ok(before - s.merges.len())
    }
    /// Clears contents (`contents`), formats (`formats`) or both in a range.
    pub fn clear_range(
        &mut self,
        sheet: u64,
        range: Range,
        contents: bool,
        formats: bool,
    ) -> Result<(), String> {
        range.validate()?;
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        s.cells.retain(|key, cell| {
            let Some((r, c)) = parse_address(key) else {
                return true;
            };
            if !range.contains(r, c) {
                return true;
            }
            if contents {
                cell.input.clear();
                cell.value = Value::Empty;
            }
            if formats {
                cell.fmt = CellFormat::default();
            }
            !(cell.input.is_empty() && cell.fmt == CellFormat::default())
        });
        self.recalculate();
        Ok(())
    }
    /// Cells whose input or shown value contains `query`: (row, col, input, display).
    pub fn find(
        &self,
        sheet: u64,
        query: &str,
        case_sensitive: bool,
        limit: usize,
    ) -> Result<Vec<(u32, u32, String, String)>, String> {
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let fold = |text: &str| {
            if case_sensitive {
                text.to_string()
            } else {
                text.to_lowercase()
            }
        };
        let needle = fold(query);
        let mut found = Vec::new();
        for (key, cell) in &s.cells {
            let Some((r, c)) = parse_address(key) else {
                continue;
            };
            let shown = s.display(r, c, self.locale_ru);
            if fold(&cell.input).contains(&needle) || fold(&shown).contains(&needle) {
                found.push((r, c, cell.input.clone(), shown));
                if found.len() >= limit {
                    break;
                }
            }
        }
        found.sort_unstable_by_key(|(r, c, _, _)| (*r, *c));
        Ok(found)
    }
    pub fn recalculate(&mut self) {
        super::formula::recalculate(self);
    }
    pub fn fill(&mut self, sheet: u64, source: Range, target: Range) -> Result<(), String> {
        source.validate()?;
        target.validate()?;
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        let snapshot = s.clone();
        for r in target.r0..=target.r1 {
            for c in target.c0..=target.c1 {
                let sr = source.r0 + (r - target.r0) % (source.r1 - source.r0 + 1);
                let sc = source.c0 + (c - target.c0) % (source.c1 - source.c0 + 1);
                let original = snapshot.cell(sr, sc).cloned().unwrap_or_default();
                let input = super::formula::shift_references(
                    &original.input,
                    r as i64 - sr as i64,
                    c as i64 - sc as i64,
                );
                self.set_input(sheet, r, c, &input)?;
                if let Some(dst) = self.sheet_by_id_mut(sheet).and_then(|s| s.cell_mut(r, c)) {
                    dst.fmt = original.fmt;
                }
            }
        }
        self.recalculate();
        Ok(())
    }
    pub fn sort(
        &mut self,
        sheet: u64,
        range: Range,
        keys: &[(u32, bool)],
        header: bool,
    ) -> Result<(), String> {
        range.validate()?;
        if keys.is_empty() || keys.iter().any(|(c, _)| *c < range.c0 || *c > range.c1) {
            return Err("Недопустимый ключ сортировки".into());
        }
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if s.merges
            .iter()
            .any(|m| m.r0 <= range.r1 && range.r0 <= m.r1 && m.c0 <= range.c1 && range.c0 <= m.c1)
        {
            return Err("Сортировка объединённых ячеек не поддерживается".into());
        }
        let start = range.r0 + u32::from(header);
        let mut rows: Vec<_> = (start..=range.r1)
            .map(|r| {
                (
                    r,
                    (range.c0..=range.c1)
                        .map(|c| s.cell(r, c).cloned())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        rows.sort_by(|a, b| {
            for (c, desc) in keys {
                let av =
                    a.1.get((c - range.c0) as usize)
                        .and_then(Option::as_ref)
                        .map(|x| &x.value);
                let bv =
                    b.1.get((c - range.c0) as usize)
                        .and_then(Option::as_ref)
                        .map(|x| &x.value);
                let order = match (av, bv) {
                    (Some(Value::Number(a)), Some(Value::Number(b))) => a.total_cmp(b),
                    _ => av
                        .map(Value::display)
                        .unwrap_or_default()
                        .to_lowercase()
                        .cmp(&bv.map(Value::display).unwrap_or_default().to_lowercase()),
                };
                if !order.is_eq() {
                    return if *desc { order.reverse() } else { order };
                }
            }
            std::cmp::Ordering::Equal
        });
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        for (i, (old, line)) in rows.into_iter().enumerate() {
            let r = start + i as u32;
            for (j, cell) in line.into_iter().enumerate() {
                let key = address(r, range.c0 + j as u32);
                s.cells.remove(&key);
                if let Some(mut cell) = cell {
                    cell.input =
                        super::formula::shift_references(&cell.input, r as i64 - old as i64, 0);
                    s.cells.insert(key, cell);
                }
            }
        }
        self.recalculate();
        Ok(())
    }
    pub fn edit_dimension(
        &mut self,
        sheet: u64,
        index: u32,
        count: u32,
        rows: bool,
        delete: bool,
    ) -> Result<(), String> {
        let max = if rows { MAX_ROWS } else { MAX_COLS };
        if count == 0 || index >= max || index.checked_add(count).is_none_or(|v| v > max) {
            return Err("Недопустимый диапазон".into());
        }
        let name = self
            .sheet_by_id(sheet)
            .ok_or("Лист не найден")?
            .name
            .clone();
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        let shift_index = |n: u32| -> Option<u32> {
            super::formula::shift_bounds(n, n, index, count, delete)
                .and_then(|(v, _)| (v < max).then_some(v))
        };
        let adjust = |range: &mut Range| -> bool {
            let (a, b) = if rows {
                (&mut range.r0, &mut range.r1)
            } else {
                (&mut range.c0, &mut range.c1)
            };
            if let Some((first, last)) = super::formula::shift_bounds(*a, *b, index, count, delete)
            {
                *a = first;
                *b = last;
                true
            } else {
                false
            }
        };
        if !delete {
            let cells_overflow = s
                .cells
                .keys()
                .filter_map(|a| parse_address(a))
                .any(|(r, c)| shift_index(if rows { r } else { c }).is_none());
            let metadata_overflow = if rows {
                s.row_heights
                    .keys()
                    .chain(s.hidden_rows.iter())
                    .any(|n| shift_index(*n).is_none())
            } else {
                s.widths
                    .keys()
                    .chain(s.hidden_cols.iter())
                    .any(|n| shift_index(*n).is_none())
            };
            let ranges_ok = s
                .merges
                .iter()
                .chain(s.charts.iter().map(|c| &c.range))
                .chain(s.conditional.iter().map(|r| &r.range))
                .chain(s.filter.iter())
                .all(|range| {
                    let mut value = *range;
                    adjust(&mut value) && value.validate().is_ok()
                });
            let chart_overflow = s
                .charts
                .iter()
                .any(|c| shift_index(if rows { c.row } else { c.col }).is_none());
            if cells_overflow || metadata_overflow || chart_overflow || !ranges_ok {
                return Err("Вставка вытеснит данные или форматирование за пределы листа".into());
            }
        }
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        let old = std::mem::take(&mut s.cells);
        for (k, cell) in old {
            if let Some((mut r, mut c)) = parse_address(&k)
                && let Some(n) = shift_index(if rows { r } else { c })
            {
                if rows {
                    r = n;
                } else {
                    c = n;
                }
                s.cells.insert(address(r, c), cell);
            }
        }
        if rows {
            s.row_heights = std::mem::take(&mut s.row_heights)
                .into_iter()
                .filter_map(|(k, v)| shift_index(k).map(|n| (n, v)))
                .collect();
            s.hidden_rows = std::mem::take(&mut s.hidden_rows)
                .into_iter()
                .filter_map(shift_index)
                .collect();
        } else {
            s.widths = std::mem::take(&mut s.widths)
                .into_iter()
                .filter_map(|(k, v)| shift_index(k).map(|n| (n, v)))
                .collect();
            s.hidden_cols = std::mem::take(&mut s.hidden_cols)
                .into_iter()
                .filter_map(shift_index)
                .collect();
        }
        s.merges.retain_mut(adjust);
        s.charts.retain_mut(|chart| {
            if !adjust(&mut chart.range) {
                return false;
            }
            let anchor = if rows { &mut chart.row } else { &mut chart.col };
            *anchor = shift_index(*anchor).unwrap_or(index.min(max - 1));
            true
        });
        s.conditional.retain_mut(|rule| adjust(&mut rule.range));
        if s.filter.as_mut().is_some_and(|filter| !adjust(filter)) {
            s.filter = None;
        }
        let frozen = if rows {
            &mut s.frozen_rows
        } else {
            &mut s.frozen_cols
        };
        if index < *frozen {
            *frozen = if delete {
                frozen.saturating_sub(count.min(*frozen - index))
            } else {
                frozen.saturating_add(count).min(max - 1)
            };
        }
        if self.active_id() == sheet {
            let selection = &mut self.selection;
            if rows {
                selection.row = shift_index(selection.row).unwrap_or(index.min(max - 1));
                selection.anchor_row =
                    shift_index(selection.anchor_row).unwrap_or(index.min(max - 1));
            } else {
                selection.col = shift_index(selection.col).unwrap_or(index.min(max - 1));
                selection.anchor_col =
                    shift_index(selection.anchor_col).unwrap_or(index.min(max - 1));
            }
        }
        for current in &mut self.sheets {
            for cell in current.cells.values_mut() {
                if cell.input.starts_with('=') {
                    cell.input = super::formula::shift_structural(
                        &cell.input,
                        &current.name,
                        &name,
                        index,
                        count,
                        rows,
                        delete,
                    );
                }
            }
        }
        let first_name = self
            .sheets
            .first()
            .map(|s| s.name.clone())
            .unwrap_or_default();
        for value in self.names.values_mut() {
            *value = super::formula::shift_structural(
                value,
                &first_name,
                &name,
                index,
                count,
                rows,
                delete,
            );
        }
        self.recalculate();
        Ok(())
    }
    pub fn add_chart(
        &mut self,
        sheet: u64,
        kind: ChartKind,
        range: Range,
        title: &str,
    ) -> Result<u64, String> {
        range.validate()?;
        if self.sheet_by_id(sheet).is_none() {
            return Err("Лист не найден".into());
        }
        let id = self.allocate_id();
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        s.charts.push(Chart {
            id,
            title: title.into(),
            kind,
            range,
            row: (range.r1 + 2).min(MAX_ROWS - 1),
            col: range.c0,
            width: 480.0,
            height: 280.0,
        });
        Ok(id)
    }
    /// Adds a PNG/JPEG picture at a cell, scaled down to fit 480×320 px.
    pub fn add_image(
        &mut self,
        sheet: u64,
        row: u32,
        col: u32,
        bytes: Vec<u8>,
    ) -> Result<u64, String> {
        if row >= MAX_ROWS || col >= MAX_COLS {
            return Err("Ячейка вне листа".into());
        }
        if bytes.len() > 32 * 1024 * 1024 {
            return Err("Изображение превышает 32 МБ".into());
        }
        let image = crate::util::decode_image(&bytes)?;
        let (w, h) = (image.width().max(1) as f32, image.height().max(1) as f32);
        let factor = (480.0 / w).min(320.0 / h).min(1.0);
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if s.images.len() >= 1000 {
            return Err("Слишком много рисунков на листе".into());
        }
        let id = self.allocate_id();
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        s.images.push(SheetImage {
            id,
            row,
            col,
            x_offset: 0.0,
            y_offset: 0.0,
            width: (w * factor).max(1.0),
            height: (h * factor).max(1.0),
            bytes,
            alt: String::new(),
        });
        Ok(id)
    }
    pub fn delete_image(&mut self, sheet: u64, image: u64) -> Result<(), String> {
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        let index = s
            .images
            .iter()
            .position(|i| i.id == image)
            .ok_or("Рисунок не найден")?;
        s.images.remove(index);
        Ok(())
    }
    pub fn print_page_count(&self) -> usize {
        let mut keys = BTreeSet::new();
        for sheet in self.sheets.iter().filter(|s| !s.hidden) {
            if sheet.cells.is_empty() {
                keys.insert((sheet.id, 0, 0));
            }
            for key in sheet.cells.keys() {
                if let Some((r, c)) = parse_address(key) {
                    keys.insert((sheet.id, r / 38, c / 8));
                }
            }
        }
        keys.len()
    }
    pub fn pages(&self) -> Vec<Page> {
        let mut pages = Vec::new();
        for s in self.sheets.iter().filter(|s| !s.hidden) {
            let mut blocks: PageCells<'_> = BTreeMap::new();
            for (key, cell) in &s.cells {
                if let Some((r, c)) = parse_address(key)
                    && !s.hidden_rows.contains(&r)
                    && !s.hidden_cols.contains(&c)
                {
                    blocks
                        .entry((r / 38, c / 8))
                        .or_default()
                        .push((r, c, cell));
                }
            }
            if blocks.is_empty() {
                blocks.entry((0, 0)).or_default();
            }
            for ((row_block, col_block), cells) in blocks {
                let top = row_block * 38;
                let left = col_block * 8;
                let mut items = vec![Primitive::Text {
                    x: 36.0,
                    y: 26.0,
                    text: format!(
                        "{} · {}:{}",
                        s.name,
                        address(top, left),
                        address((top + 37).min(MAX_ROWS - 1), (left + 7).min(MAX_COLS - 1))
                    ),
                    size: 12.0,
                    color: [28, 32, 38],
                }];
                for (row, col, cell) in cells {
                    let x = 36.0 + (col - left) as f32 * 65.0;
                    let y = 54.0 + (row - top) as f32 * 18.0;
                    if let Some(color) = cell.fmt.fill {
                        items.push(Primitive::Rect {
                            x,
                            y,
                            w: 65.0,
                            h: 18.0,
                            color,
                        });
                    }
                    let text =
                        super::format::display(&cell.value, &cell.fmt.number, self.locale_ru);
                    items.push(Primitive::TextStyled {
                        x: x + 3.0,
                        y: y + 3.0,
                        text,
                        size: cell.fmt.size.min(14.0),
                        color: cell.fmt.color,
                        bold: cell.fmt.bold,
                        italic: cell.fmt.italic,
                        font: cell.fmt.font.clone(),
                    });
                    if cell.fmt.underline || cell.fmt.strike {
                        let yy = y + if cell.fmt.strike { 8.0 } else { 15.0 };
                        items.push(Primitive::Line {
                            x1: x + 3.0,
                            y1: yy,
                            x2: x + 62.0,
                            y2: yy,
                            width: 0.5,
                            color: cell.fmt.color,
                        });
                    }
                    items.push(Primitive::Line {
                        x1: x,
                        y1: y + 18.0,
                        x2: x + 65.0,
                        y2: y + 18.0,
                        width: 0.4,
                        color: [210, 214, 220],
                    });
                    items.push(Primitive::Line {
                        x1: x + 65.0,
                        y1: y,
                        x2: x + 65.0,
                        y2: y + 18.0,
                        width: 0.4,
                        color: [210, 214, 220],
                    });
                }
                pages.push(Page {
                    width: 595.0,
                    height: 842.0,
                    items,
                });
            }
        }
        pages
    }
}
pub fn address(row: u32, col: u32) -> String {
    let mut n = col + 1;
    let mut chars = Vec::new();
    while n > 0 {
        n -= 1;
        chars.push(char::from(b'A' + (n % 26) as u8));
        n /= 26;
    }
    chars.reverse();
    format!("{}{}", chars.into_iter().collect::<String>(), row + 1)
}
pub fn parse_address(input: &str) -> Option<(u32, u32)> {
    let s = input.trim().replace('$', "");
    let split = s.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = s.split_at(split);
    if letters.is_empty()
        || !letters.chars().all(|c| c.is_ascii_alphabetic())
        || !digits.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let mut col = 0u32;
    for c in letters.bytes() {
        col = col
            .checked_mul(26)?
            .checked_add(u32::from(c.to_ascii_uppercase() - b'A' + 1))?;
    }
    let row: u32 = digits.parse().ok()?;
    if row == 0 || row > MAX_ROWS || col == 0 || col > MAX_COLS {
        return None;
    }
    Some((row - 1, col - 1))
}
pub fn parse_range(input: &str) -> Result<Range, String> {
    let mut parts = input.split(':');
    let a = parts
        .next()
        .and_then(parse_address)
        .ok_or("Недопустимый адрес")?;
    let b = match parts.next() {
        Some(b) => parse_address(b).ok_or("Недопустимый адрес")?,
        None => a,
    };
    if parts.next().is_some() {
        return Err("Недопустимый диапазон".into());
    }
    let range = Range::new(a.0, a.1, b.0, b.1);
    range.validate()?;
    Ok(range)
}

fn validate_merges(merges: &[Range]) -> Result<(), String> {
    let mut events = Vec::with_capacity(merges.len() * 2);
    for (id, m) in merges.iter().enumerate() {
        m.validate()?;
        events.push((m.r0, true, m.c0, m.c1, id));
        events.push((m.r1 + 1, false, m.c0, m.c1, id));
    }
    events.sort_unstable();
    let mut active = BTreeSet::new();
    for (_, start, c0, c1, id) in events {
        if !start {
            active.remove(&(c0, c1, id));
            continue;
        }
        let previous = active.range(..=(c0, u32::MAX, usize::MAX)).next_back();
        let next = active.range((c0, 0, 0)..).next();
        if previous.is_some_and(|(_, end, _)| *end >= c0)
            || next.is_some_and(|(start, _, _)| *start <= c1)
        {
            return Err("Пересекающиеся объединённые диапазоны".into());
        }
        active.insert((c0, c1, id));
    }
    Ok(())
}
