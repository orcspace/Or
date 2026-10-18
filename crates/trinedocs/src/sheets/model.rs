use crate::render::{Page, Primitive};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type PageCells<'a> = BTreeMap<(u32,u32),Vec<PrintCell<'a>>>;
struct PrintCell<'a> {row:u32,col:u32,cell:Option<&'a Cell>,merge:Option<Range>}
struct PrintAxis {
    base:f64, maximum:u32, hidden:Vec<u32>, adjustments:Vec<(u32,f64)>,
}
impl PrintAxis {
    fn new(base:f32,sizes:&BTreeMap<u32,f32>,hidden:&BTreeSet<u32>,maximum:u32)->Self {
        let mut sum=0.0;
        let adjustments=sizes.iter().filter(|(index,_)|!hidden.contains(*index)).map(|(index,size)| {
            sum+=f64::from(*size)-f64::from(base);(*index,sum)
        }).collect();
        Self{base:f64::from(base),maximum,hidden:hidden.iter().copied().collect(),adjustments}
    }
    fn visible(&self,index:u32)->u32 {index-self.hidden.partition_point(|hidden|*hidden<index) as u32}
    fn position(&self,index:u32)->f64 {
        let offset=self.adjustments.partition_point(|(position,_)|*position<index);
        f64::from(self.visible(index))*self.base+offset.checked_sub(1).map(|index|self.adjustments[index].1).unwrap_or(0.0)
    }
    fn original(&self,visible:u32)->u32 {
        let (mut low,mut high)=(0,self.maximum);
        while low<high {
            let middle=low+(high-low)/2;
            if self.visible(middle+1)<=visible {low=middle+1;}else{high=middle;}
        }
        low
    }
}
fn print_blocks(sheet:&Sheet)->PageCells<'_> {
    let rows=PrintAxis::new(sheet.default_row_height,&sheet.row_heights,&sheet.hidden_rows,MAX_ROWS);
    let cols=PrintAxis::new(sheet.default_col_width,&sheet.widths,&sheet.hidden_cols,MAX_COLS);
    let mut positions:BTreeMap<_,_>=sheet.cells.iter().filter_map(|(key,cell)|parse_address(key).map(|position|(position,Some(cell)))).collect();
    for merge in &sheet.merges {positions.entry((merge.r0,merge.c0)).or_insert(None);}
    let positions:Vec<_>=positions.into_iter().collect();
    let mut events=Vec::new();
    for (index,merge) in sheet.merges.iter().enumerate() {
        events.push((merge.r0,1u8,merge.c0,index));events.push((merge.r1+1,0u8,merge.c0,index));
    }
    for (index,((row,col),_)) in positions.iter().enumerate() {events.push((*row,2u8,*col,index));}
    events.sort_unstable();
    let mut active:BTreeMap<u32,Range>=BTreeMap::new();
    let mut blocks:PageCells<'_>=BTreeMap::new();
    for (row,kind,col,index) in events {
        match kind {
            0=>{active.remove(&col);}
            1=>{active.insert(col,sheet.merges[index]);}
            _=>{
                let merge=active.range(..=col).next_back().map(|(_,range)|*range).filter(|range|range.c1>=col);
                if let Some(merge)=merge {
                    if (row,col)!=(merge.r0,merge.c0)
                        || rows.visible(merge.r1+1)==rows.visible(merge.r0)
                        || cols.visible(merge.c1+1)==cols.visible(merge.c0) {continue;}
                } else if sheet.hidden_rows.contains(&row)||sheet.hidden_cols.contains(&col) {continue;}
                blocks.entry((rows.visible(row)/38,cols.visible(col)/8)).or_default()
                    .push(PrintCell{row,col,cell:positions[index].1,merge});
            }
        }
    }
    for image in &sheet.images {blocks.entry((rows.visible(image.row)/38,cols.visible(image.col)/8)).or_default();}
    if blocks.is_empty(){blocks.entry((0,0)).or_default();}
    blocks
}

pub const MAX_ROWS: u32 = 1_048_576;
pub const MAX_COLS: u32 = 16_384;
pub const MAX_RANGE_CELLS: usize = 100_000;
const MAX_BOOK_TEXT_BYTES: usize = 64*1024*1024;

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
            // As text a number keeps 15 significant digits, so 0.1+0.2 joins as "0.3".
            Self::Number(n) => super::format::general_number(*n, 15),
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
impl CellFormat {
    pub fn validate(&self) -> Result<(), String> {
        if self.font.len() > 256
            || !self.size.is_finite()
            || !(1.0..=409.0).contains(&self.size)
            || self.number.len() > 1024
            || !(-90..=90).contains(&self.rotation)
        {
            return Err("Недопустимый формат ячейки".into());
        }
        Ok(())
    }
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
    #[serde(default="true_value")]
    pub has_headers: bool,
    #[serde(default="true_value")]
    pub has_categories: bool,
    pub title: String,
    pub kind: ChartKind,
    pub range: Range,
    pub row: u32,
    pub col: u32,
    pub width: f32,
    pub height: f32,
}
fn true_value()->bool {true}
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
    pub default_col_width: f32,
    pub default_row_height: f32,
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
            default_col_width: 96.0,
            default_row_height: 26.0,
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
    /// Excel Ctrl+Arrow target: the end of the current data block, the start of the next
    /// block, or the sheet edge when nothing else lies in that direction.
    pub fn data_edge(&self, row: u32, col: u32, d_row: i8, d_col: i8) -> (u32, u32) {
        let vertical = d_row != 0;
        let forward = if vertical { d_row > 0 } else { d_col > 0 };
        let (pos, limit) = if vertical {
            (row, MAX_ROWS - 1)
        } else {
            (col, MAX_COLS - 1)
        };
        let hidden = if vertical {
            &self.hidden_rows
        } else {
            &self.hidden_cols
        };
        let filled: BTreeSet<u32> = self
            .cells
            .iter()
            .filter(|(_, cell)| !cell.input.is_empty())
            .filter_map(|(key, _)| parse_address(key))
            .filter(|&(r, c)| if vertical { c == col } else { r == row })
            .map(|(r, c)| if vertical { r } else { c })
            .filter(|position| !hidden.contains(position))
            .collect();
        let step = |mut p: u32| -> Option<u32> {
            loop {
                p = if forward {
                    p.checked_add(1).filter(|p| *p <= limit)?
                } else {
                    p.checked_sub(1)?
                };
                if !hidden.contains(&p) {
                    return Some(p);
                }
            }
        };
        let target = match step(pos) {
            None => pos,
            Some(next) if filled.contains(&pos) && filled.contains(&next) => {
                let mut end = next;
                while let Some(after) = step(end).filter(|p| filled.contains(p)) {
                    end = after;
                }
                end
            }
            Some(next) => {
                let found = if forward {
                    filled.range(next..).next().copied()
                } else {
                    filled.range(..=next).next_back().copied()
                };
                found.unwrap_or(if forward { limit } else { 0 })
            }
        };
        if vertical {
            (target, col)
        } else {
            (row, target)
        }
    }
    pub fn conditional_fill(&self,row:u32,col:u32,value:&Value)->Option<[u8;3]> {
        let value=value.number().ok()?;
        self.conditional.iter().find(|rule|rule.range.contains(row,col)&&match rule.operator.as_str() {
            ">"=>value>rule.threshold,"<"=>value<rule.threshold,"="=>value==rule.threshold,
            ">="=>value>=rule.threshold,"<="=>value<=rule.threshold,_=>false,
        }).map(|rule|rule.fill)
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
    fn atomic_edit<R>(
        &mut self,
        edit: impl FnOnce(&mut Self) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut candidate = self.clone();
        let result = edit(&mut candidate)?;
        candidate.validate()?;
        *self = candidate;
        Ok(result)
    }
    fn text_bytes(&self) -> usize {
        let names = self.names.iter().fold(0usize, |total,(name,value)| total.saturating_add(name.len()).saturating_add(value.len()));
        self.sheets.iter().fold(names, |total,sheet| {
            let metadata = sheet.charts.iter().fold(0usize, |bytes, chart| bytes.saturating_add(chart.title.len()))
                .saturating_add(sheet.images.iter().fold(0usize, |bytes, image| bytes.saturating_add(image.alt.len())));
            sheet.cells.values().fold(total.saturating_add(sheet.name.len()).saturating_add(metadata), |total,cell| {
                total.saturating_add(cell.input.len()).saturating_add(cell.fmt.font.len()).saturating_add(cell.fmt.number.len())
            })
        })
    }
    fn check_cell_growth(&self, removed: usize, added: usize, new_cells: usize, limit: usize) -> Result<(), String> {
        let count=self.sheets.iter().map(|sheet|sheet.cells.len()).sum::<usize>();
        if count.checked_add(new_cells).is_none_or(|count|count>1_000_000)
            || self.text_bytes().saturating_sub(removed).checked_add(added).is_none_or(|bytes|bytes>limit) {
            return Err("Правка превышает лимит ячеек или исходного текста книги".into());
        }
        Ok(())
    }
    fn validate_text_budget(&self, limit: usize) -> Result<(), String> {
        if self.text_bytes() > limit {
            Err("Текст книги и метаданных превышает общий лимит".into())
        } else { Ok(()) }
    }
    pub fn validate(&self) -> Result<(), String> {
        self.validate_text_budget(MAX_BOOK_TEXT_BYTES)?;
        if self.sheets.is_empty()
            || self.sheets.len() > 1024
            || self.sheets.iter().all(|sheet| sheet.hidden)
            || self.active >= self.sheets.len()
            || self
                .sheets
                .get(self.active)
                .is_some_and(|sheet| sheet.hidden)
            || self.selection.row >= MAX_ROWS
            || self.selection.col >= MAX_COLS
            || self.selection.anchor_row >= MAX_ROWS
            || self.selection.anchor_col >= MAX_COLS
        {
            return Err("Недопустимая книга или выделение".into());
        }
        let mut ids = BTreeSet::new();
        let mut count = 0usize;
        let mut image_bytes = 0usize;
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
                c.fmt.validate()?;
                if parse_address(key).is_none_or(|(r, col)| address(r, col) != *key)
                    || c.input.len() > 32767
                    || matches!(&c.value,Value::Number(n)if !n.is_finite())
                    || matches!(&c.value,Value::Text(t)|Value::Error(t)if t.len()>32767)
                {
                    return Err("Недопустимая ячейка или формат".into());
                }
            }
            if !s.default_col_width.is_finite() || !(1.0..=1800.0).contains(&s.default_col_width)
                || !s.default_row_height.is_finite() || !(1.0..=600.0).contains(&s.default_row_height)
                || s.widths
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
            validate_merge_inputs(s)?;
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
                image_bytes = image_bytes.saturating_add(image.bytes.len());
                if image_bytes > 64*1024*1024 {
                    return Err("Изображения книги превышают 64 МиБ".into());
                }
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
                let reader=image::ImageReader::new(std::io::Cursor::new(&image.bytes))
                    .with_guessed_format().map_err(|e| e.to_string())?;
                let (width,height)=reader.into_dimensions().map_err(|e| e.to_string())?;
                if width == 0 || height == 0 || u64::from(width)*u64::from(height)>32_000_000 {
                    return Err("Изображение превышает 32 млн пикселей".into());
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
        let mut defined_names = BTreeSet::new();
        if self.names.len() > 10000
            || self.names.iter().any(|(name, value)| {
                // Office stores these built-in metadata names; callers still cannot create
                // them through the user-defined-name MCP command.
                let office_metadata=matches!(name.to_ascii_uppercase().as_str(),
                    "_XLNM.PRINT_AREA"|"_XLNM.PRINT_TITLES"|"_XLNM._FILTERDATABASE");
                (!valid_defined_name(name) && !office_metadata)
                    || !defined_names.insert(name.to_lowercase()) || value.len() > 32767
            })
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
            || name
                .chars()
                .any(|c| c.is_control() || "[]:*?/\\".contains(c))
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
        self.atomic_edit(|book| book.rename_sheet_inner(id, name))
    }
    fn rename_sheet_inner(&mut self, id: u64, name: &str) -> Result<(), String> {
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
        if !self.sheets[pos].hidden && self.sheets.iter().filter(|sheet| !sheet.hidden).count() == 1
        {
            return Err("Нельзя удалить последний видимый лист".into());
        }
        let removed = self.sheets.remove(pos);
        if self.active > pos {
            self.active -= 1;
        }
        self.active = self.active.min(self.sheets.len().saturating_sub(1));
        if self.sheets[self.active].hidden
            && let Some(visible) = self.sheets.iter().position(|sheet| !sheet.hidden)
        {
            self.active = visible;
        }
        for s in &mut self.sheets {
            for cell in s.cells.values_mut() {
                if cell.input.starts_with('=') {
                    cell.input = super::formula::invalidate_sheet(&cell.input, &removed.name);
                }
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
        let active = self.active_id();
        let s = self.sheets.remove(pos);
        self.sheets.insert(index, s);
        self.active = self
            .sheets
            .iter()
            .position(|sheet| sheet.id == active)
            .unwrap_or(index);
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
        let normalized = super::formula::canonicalize(input);
        if normalized.len() > 32767 {
            return Err("Нормализованная формула превышает предел длины ячейки".into());
        }
        let source = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if !normalized.is_empty() && source.merges.iter().any(|merge| {
            merge.contains(row, col) && (row, col) != (merge.r0, merge.c0)
        }) {
            return Err("Нельзя записывать содержимое в скрытую часть объединённой ячейки".into());
        }
        let key = address(row, col);
        let existing = self
            .sheet_by_id(sheet)
            .ok_or("Лист не найден")?
            .cells
            .get(&key)
            .map(|c| c.id);
        let id = existing.unwrap_or_else(|| self.allocate_id());
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
        self.set_range_with_limit(sheet, row, col, values, MAX_BOOK_TEXT_BYTES)
    }
    fn set_range_with_limit(&mut self, sheet: u64, row: u32, col: u32, values: &[Vec<String>], limit: usize) -> Result<(), String> {
        if self.sheet_by_id(sheet).is_none() {
            return Err("Лист не найден".into());
        }
        let count = values.iter().map(Vec::len).sum::<usize>();
        if count > MAX_RANGE_CELLS
            || u64::from(row) + values.len() as u64 > u64::from(MAX_ROWS)
            || values.iter().any(|v| {
                u64::from(col) + v.len() as u64 > u64::from(MAX_COLS)
                    || v.iter()
                        .any(|x| x.len() > 32767 || super::formula::canonicalize(x).len() > 32767)
            })
        {
            return Err("Недопустимый или слишком большой диапазон".into());
        }
        let source = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if source.merges.iter().any(|merge| {
            values.iter().enumerate().any(|(r, line)| {
                line.iter().enumerate().any(|(c, _)| {
                    let target = (row + r as u32, col + c as u32);
                    merge.contains(target.0, target.1)
                        && (count > 1 || target != (merge.r0, merge.c0))
                })
            })
        }) {
            return Err("Вставка затрагивает объединённые ячейки: сначала разъедините их".into());
        }
        let mut removed=0usize;
        let mut added=0usize;
        let mut new_cells=0usize;
        for (r,line) in values.iter().enumerate() {
            for (c,input) in line.iter().enumerate() {
                let previous=source.cell(row+r as u32,col+c as u32);
                if let Some(cell)=previous { removed=removed.saturating_add(cell.input.len()); }
                else { new_cells+=1; }
                added=added.saturating_add(super::formula::canonicalize(input).len());
                if previous.is_none() { added=added.saturating_add(CellFormat::default().font.len()+CellFormat::default().number.len()); }
            }
        }
        self.check_cell_growth(removed,added,new_cells,limit)?;
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
        self.set_format_with_limit(sheet,range,fmt,MAX_BOOK_TEXT_BYTES)
    }
    fn set_format_with_limit(&mut self, sheet: u64, range: Range, fmt: CellFormat, limit: usize) -> Result<(), String> {
        range.validate()?;
        fmt.validate()?;
        let source=self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        let mut removed=0usize;
        let mut added=0usize;
        let mut new_cells=0usize;
        for r in range.r0..=range.r1 {
            for c in range.c0..=range.c1 {
                if let Some(cell)=source.cell(r,c) {
                    removed=removed.saturating_add(cell.fmt.font.len()+cell.fmt.number.len());
                } else { new_cells+=1; }
                added=added.saturating_add(fmt.font.len()+fmt.number.len());
            }
        }
        self.check_cell_growth(removed,added,new_cells,limit)?;
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
        if range.r0 == range.r1 && range.c0 == range.c1 {
            return Ok(());
        }
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
        if limit==0 {
            self.sheet_by_id(sheet).ok_or("Лист не найден")?;
            return Ok(Vec::new());
        }
        let found = self.find_positions(sheet, query, case_sensitive)?;
        let sheet = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        Ok(found.into_iter().take(limit).filter_map(|(row,col)| {
            sheet.cell(row,col).map(|cell|
                (row,col,cell.input.clone(),sheet.display(row,col,self.locale_ru)))
        }).collect())
    }
    /// Search positions without cloning the text of every matching cell into UI state.
    pub fn find_positions(
        &self,
        sheet: u64,
        query: &str,
        case_sensitive: bool,
    ) -> Result<Vec<(u32,u32)>,String> {
        let sheet = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if query.is_empty() { return Ok(Vec::new()); }
        let mut found = Vec::new();
        for (key,cell) in &sheet.cells {
            let Some((row,col)) = parse_address(key) else { continue; };
            if !match_ranges(&cell.input,query,case_sensitive).is_empty()
                || !match_ranges(&sheet.display(row,col,self.locale_ru),query,case_sensitive).is_empty() {
                found.push((row,col));
            }
        }
        found.sort_unstable();
        Ok(found)
    }
    pub fn replace_all(&mut self,sheet:u64,query:&str,replacement:&str)->Result<usize,String> {
        self.replace_all_with_case(sheet,query,replacement,true)
    }
    pub fn replace_all_with_case(&mut self,sheet:u64,query:&str,replacement:&str,case_sensitive:bool)->Result<usize,String> {
        self.replace_all_with_limit(sheet,query,replacement,case_sensitive,MAX_BOOK_TEXT_BYTES)
    }
    fn replace_all_with_limit(&mut self,sheet:u64,query:&str,replacement:&str,case_sensitive:bool,limit:usize)->Result<usize,String> {
        let s=self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if query.is_empty(){return Ok(0);}
        let removed_total=s.cells.values().filter(|cell| !match_ranges(&cell.input,query,case_sensitive).is_empty())
            .map(|cell|cell.input.len()).sum::<usize>();
        let mut bytes=self.text_bytes().saturating_sub(removed_total);
        let mut changes=Vec::new();
        for (key,cell) in &s.cells {
            let ranges=match_ranges(&cell.input,query,case_sensitive);
            if ranges.is_empty(){continue;}
            let removed:usize=ranges.iter().map(|(start,end)|end-start).sum();
            let length=ranges.len().checked_mul(replacement.len())
                .and_then(|added|(cell.input.len()-removed).checked_add(added));
            let Some(length)=length.filter(|length|*length<=32767) else {return Err("Замена превышает предел длины ячейки".into());};
            let mut input=String::with_capacity(length);
            let mut offset=0;
            for (start,end) in ranges {
                input.push_str(&cell.input[offset..start]);input.push_str(replacement);offset=end;
            }
            input.push_str(&cell.input[offset..]);
            let input=super::formula::canonicalize(&input);
            if input.len()>32767{return Err("Нормализованная формула превышает предел длины ячейки".into());}
            bytes=bytes.checked_add(input.len()).filter(|bytes|*bytes<=limit)
                .ok_or("Замена превышает предел текста книги")?;
            if input!=cell.input {changes.push((key.clone(),input));}
        }
        let count=changes.len();
        let s=self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        for (key,input) in changes {
            if let Some(cell)=s.cells.get_mut(&key) {cell.input=input;}
        }
        if count>0 {self.recalculate();}
        Ok(count)
    }
    pub fn recalculate(&mut self) {
        super::formula::recalculate(self);
    }
    pub fn fill(&mut self, sheet: u64, source: Range, target: Range) -> Result<(), String> {
        self.atomic_edit(|book| book.fill_inner(sheet, source, target))
    }
    fn fill_inner(&mut self, sheet: u64, source: Range, target: Range) -> Result<(), String> {
        self.fill_with_limit(sheet, source, target, MAX_BOOK_TEXT_BYTES)
    }
    fn fill_with_limit(&mut self, sheet: u64, source: Range, target: Range, limit: usize) -> Result<(), String> {
        source.validate()?;
        target.validate()?;
        let s = self.sheet_by_id(sheet).ok_or("Лист не найден")?;
        if s.merges.iter().any(|merge| {
            merge.r0 <= target.r1
                && target.r0 <= merge.r1
                && merge.c0 <= target.c1
                && target.c0 <= merge.c1
                && (target.r0 != target.r1
                    || target.c0 != target.c1
                    || (target.r0, target.c0) != (merge.r0, merge.c0))
        }) {
            return Err(
                "Заполнение затрагивает объединённые ячейки: сначала разъедините их".into(),
            );
        }
        // Predict input/format replication before cloning a large source cell repeatedly.
        let mut text_bytes = self.text_bytes();
        for r in target.r0..=target.r1 {
            for c in target.c0..=target.c1 {
                if let Some(old) = s.cell(r,c) {
                    text_bytes = text_bytes.saturating_sub(old.input.len()+old.fmt.font.len()+old.fmt.number.len());
                }
            }
        }
        for r in target.r0..=target.r1 {
            for c in target.c0..=target.c1 {
                let sr = source.r0+(r-target.r0)%(source.r1-source.r0+1);
                let sc = source.c0+(c-target.c0)%(source.c1-source.c0+1);
                if let Some(original) = s.cell(sr,sc) {
                    let input = super::formula::shift_references(&original.input,i64::from(r)-i64::from(sr),i64::from(c)-i64::from(sc));
                    text_bytes = text_bytes.saturating_add(input.len()+original.fmt.font.len()+original.fmt.number.len());
                }
                if text_bytes > limit {
                    return Err("Заполнение превысит лимит исходного текста книги 64 МиБ".into());
                }
            }
        }
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
                if input.is_empty() && original.fmt == CellFormat::default() {
                    if let Some(sheet) = self.sheet_by_id_mut(sheet) {
                        sheet.cells.remove(&address(r, c));
                    }
                    continue;
                }
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
        self.atomic_edit(|book| book.sort_inner(sheet, range, keys, header))
    }
    fn sort_inner(
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
        let positions: Vec<_> = (start..=range.r1)
            .filter(|row| !s.hidden_rows.contains(row))
            .collect();
        let mut rows: Vec<_> = positions
            .iter()
            .copied()
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
                let av = av.unwrap_or(&Value::Empty);
                let bv = bv.unwrap_or(&Value::Empty);
                let a_blank = matches!(av, Value::Empty);
                let b_blank = matches!(bv, Value::Empty);
                if a_blank != b_blank {
                    // Empty cells stay after data in either sort direction.
                    return a_blank.cmp(&b_blank);
                }
                let kind = |value: &Value| match value {
                    Value::Number(_) => 0,
                    Value::Text(_) => 1,
                    Value::Bool(_) => 2,
                    Value::Error(_) => 3,
                    Value::Empty => 4,
                };
                let order = match (av, bv) {
                    (Value::Number(a), Value::Number(b)) => {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
                    (Value::Text(a), Value::Text(b)) | (Value::Error(a), Value::Error(b)) => {
                        a.to_lowercase().cmp(&b.to_lowercase())
                    }
                    _ => kind(av).cmp(&kind(bv)),
                };
                if !order.is_eq() {
                    return if *desc { order.reverse() } else { order };
                }
            }
            std::cmp::Ordering::Equal
        });
        let s = self.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        for (r, (old, line)) in positions.into_iter().zip(rows) {
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
        self.atomic_edit(|book| book.edit_dimension_inner(sheet, index, count, rows, delete))
    }
    fn edit_dimension_inner(
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
            let image_overflow = s
                .images
                .iter()
                .any(|image| shift_index(if rows { image.row } else { image.col }).is_none());
            if cells_overflow || metadata_overflow || chart_overflow || image_overflow || !ranges_ok
            {
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
        s.merges
            .retain_mut(|range| adjust(range) && (range.r0 != range.r1 || range.c0 != range.c1));
        s.charts.retain_mut(|chart| {
            if !adjust(&mut chart.range) {
                return false;
            }
            let anchor = if rows { &mut chart.row } else { &mut chart.col };
            *anchor = shift_index(*anchor).unwrap_or(index.min(max - 1));
            true
        });
        for image in &mut s.images {
            let anchor = if rows { &mut image.row } else { &mut image.col };
            *anchor = shift_index(*anchor).unwrap_or(index.min(max - 1));
        }
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
            id,has_headers:true,has_categories:true,
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
        self.sheets.iter().filter(|sheet|!sheet.hidden)
            .map(|sheet|print_blocks(sheet).len()).sum()
    }
    pub fn pages(&self) -> Vec<Page> {
        let mut pages=Vec::new();
        for sheet in self.sheets.iter().filter(|sheet|!sheet.hidden) {
            let rows=PrintAxis::new(sheet.default_row_height,&sheet.row_heights,&sheet.hidden_rows,MAX_ROWS);
            let cols=PrintAxis::new(sheet.default_col_width,&sheet.widths,&sheet.hidden_cols,MAX_COLS);
            for ((row_block,col_block),cells) in print_blocks(sheet) {
                let top=rows.original(row_block*38);
                let left=cols.original(col_block*8);
                let origin_x=cols.position(left);
                let origin_y=rows.position(top);
                let cell_rect=|cell:&PrintCell<'_>| {
                    let range=cell.merge.unwrap_or(Range::new(cell.row,cell.col,cell.row,cell.col));
                    let x=(cols.position(range.c0)-origin_x)*0.75;
                    let y=(rows.position(range.r0)-origin_y)*0.75;
                    let width=(cols.position(range.c1+1)-cols.position(range.c0))*0.75;
                    let height=(rows.position(range.r1+1)-rows.position(range.r0))*0.75;
                    (x,y,width,height)
                };
                let images:Vec<_>=sheet.images.iter().filter(|image|
                    (rows.visible(image.row)/38,cols.visible(image.col)/8)==(row_block,col_block)).collect();
                let mut width=(cols.position(cols.original((col_block+1)*8))-origin_x)*0.75;
                let mut height=(rows.position(rows.original((row_block+1)*38))-origin_y)*0.75;
                for cell in &cells {let (x,y,w,h)=cell_rect(cell);width=width.max(x+w);height=height.max(y+h);}
                for image in &images {
                    width=width.max((cols.position(image.col)-origin_x+f64::from(image.x_offset+image.width))*0.75);
                    height=height.max((rows.position(image.row)-origin_y+f64::from(image.y_offset+image.height))*0.75);
                }
                let scale=(523.0/width.max(1.0)).min(752.0/height.max(1.0)).min(1.0) as f32;
                let mut items=vec![Primitive::Text{x:36.0,y:26.0,
                    text:format!("{} · {}:{}",sheet.name,address(top,left),
                        address(rows.original((row_block+1)*38).saturating_sub(1).min(MAX_ROWS-1),
                            cols.original((col_block+1)*8).saturating_sub(1).min(MAX_COLS-1))),
                    size:12.0,color:[28,32,38]}];
                for printed in cells {
                    let (x,y,w,h)=cell_rect(&printed);
                    let (x,y,w,h)=(36.0+x as f32*scale,54.0+y as f32*scale,w as f32*scale,h as f32*scale);
                    let fmt=printed.cell.map(|cell|cell.fmt.clone()).unwrap_or_default();
                    let value=printed.cell.map(|cell|&cell.value).unwrap_or(&Value::Empty);
                    if let Some(fill)=sheet.conditional_fill(printed.row,printed.col,value).or(fmt.fill) {
                        items.push(Primitive::Rect{x,y,w,h,color:fill});
                    }
                    let text=super::format::display(value,&fmt.number,self.locale_ru);
                    let text_color=super::format::color(value,&fmt.number).unwrap_or(fmt.color);
                    let size=fmt.size*scale;
                    items.push(Primitive::TextStyled{x:x+3.0*scale,y:y+3.0*scale,text,
                        size,color:text_color,bold:fmt.bold,italic:fmt.italic,font:fmt.font.clone()});
                    for offset in [fmt.underline.then_some(size+2.0*scale),fmt.strike.then_some(size*0.55)]
                        .into_iter().flatten() {
                        items.push(Primitive::Line{x1:x+3.0*scale,y1:y+offset,x2:x+w-3.0*scale,y2:y+offset,
                            width:0.5,color:text_color});
                    }
                    let color=if fmt.border {[70,70,70]}else{[210,214,220]};
                    let stroke=if fmt.border {0.75}else{0.4}*scale;
                    for (x1,y1,x2,y2) in [(x,y+h,x+w,y+h),(x+w,y,x+w,y+h)] {
                        items.push(Primitive::Line{x1,y1,x2,y2,width:stroke,color});
                    }
                    if fmt.border {
                        for (x1,y1,x2,y2) in [(x,y,x+w,y),(x,y,x,y+h)] {
                            items.push(Primitive::Line{x1,y1,x2,y2,width:stroke,color});
                        }
                    }
                }
                for image in images {
                    items.push(Primitive::Image{
                        x:36.0+((cols.position(image.col)-origin_x+f64::from(image.x_offset))*0.75) as f32*scale,
                        y:54.0+((rows.position(image.row)-origin_y+f64::from(image.y_offset))*0.75) as f32*scale,
                        w:image.width*0.75*scale,h:image.height*0.75*scale,bytes:image.bytes.clone(),
                    });
                }
                pages.push(Page{width:595.0,height:842.0,items});
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
    let s = input.trim();
    let s = s.strip_prefix('$').unwrap_or(s);
    let split = s.find(|c: char| !c.is_ascii_alphabetic())?;
    let (letters, tail) = s.split_at(split);
    let digits = tail.strip_prefix('$').unwrap_or(tail);
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

fn validate_merge_inputs(sheet: &Sheet) -> Result<(), String> {
    // Sweep rows, so native validation does not compare every cell with every merge.
    let mut events=Vec::new();
    for merge in &sheet.merges {
        events.push((merge.r0,1u8,merge.c0,merge.c1));
        events.push((merge.r1+1,0u8,merge.c0,merge.c1));
    }
    for (key,cell) in &sheet.cells {
        if !cell.input.is_empty() && let Some((row,col))=parse_address(key) {
            events.push((row,2u8,col,col));
        }
    }
    events.sort_unstable();
    let mut active=BTreeMap::new();
    for (row,kind,col,end) in events {
        match kind {
            0 => { active.remove(&col); }
            1 => { active.insert(col,(end,row)); }
            _ => {
                if let Some((&start,&(last,anchor_row)))=active.range(..=col).next_back()
                    && col<=last && (row,col)!=(anchor_row,start) {
                    return Err("Содержимое в скрытой части объединённой ячейки".into());
                }
            }
        }
    }
    Ok(())
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
pub fn valid_defined_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if rust_xlsxwriter::utility::check_name(name).is_err()
        || name.chars().count() > 255
        || !(first.is_alphabetic() || matches!(first, '_' | '\\'))
        || !chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '.'))
        || matches!(name, "R" | "r" | "C" | "c")
        || parse_address(name).is_some()
    {
        return false;
    }
    let upper = name.to_ascii_uppercase();
    let r1c1 = upper
        .strip_prefix('R')
        .and_then(|tail| tail.split_once('C'))
        .is_some_and(|(r, c)| {
            !r.is_empty()
                && !c.is_empty()
                && r.bytes().all(|b| b.is_ascii_digit())
                && c.bytes().all(|b| b.is_ascii_digit())
        });
    !r1c1
}


#[cfg(test)]
mod model_defect_tests {
    use super::*;
    #[test]
    fn addresses_reject_misplaced_absolute_markers() {
        for bad in ["A$$1","A1$","$$A1","A$B1"] { assert_eq!(parse_address(bad),None,"{bad}"); }
        for valid in ["A1","$A1","A$1","$A$1"] { assert_eq!(parse_address(valid),Some((0,0))); }
    }
    #[test]
    fn native_defined_names_follow_the_same_rules_as_mcp() {
        for name in ["A1","bad name","R1C1","R"] {
            let mut book=Book::default();
            book.names.insert(name.into(),"=1".into());
            assert!(book.validate().is_err(),"{name}");
        }
        let mut book=Book::default();
        book.names.insert("Tax".into(),"=1".into());
        book.names.insert("tax".into(),"=2".into());
        assert!(book.validate().is_err());
        let mut imported=Book::default();
        for name in ["_xlnm.Print_Area","_xlnm.Print_Titles","_xlnm._FilterDatabase"] {
            imported.names.insert(name.into(),"=A1".into());
            assert!(!valid_defined_name(name));
        }
        assert!(imported.validate().is_ok());
    }
    #[test]
    fn fill_rejects_excessive_text_before_replication() -> Result<(),String> {
        let mut book=Book::default();
        let id=book.active_id();
        book.set_input(id,0,0,&"x".repeat(32000))?;
        let before=book.clone();
        assert!(book.fill(id,Range::new(0,0,0,0),Range::new(0,0,99999,0)).is_err());
        assert_eq!(book,before);
        Ok(())
    }
    fn picture(id: u64,bytes: Vec<u8>) -> SheetImage {
        SheetImage { id,row:0,col:0,x_offset:0.0,y_offset:0.0,width:10.0,height:10.0,bytes,alt:String::new() }
    }
    #[test]
    fn native_sheet_images_require_a_decodable_header() {
        let mut book=Book::default();
        book.sheets[0].images.push(picture(2,vec![1,2,3]));
        book.next_id=3;
        assert!(book.validate().is_err());
    }
    #[test]
    fn workbook_image_budget_is_aggregate() -> Result<(),String> {
        let mut png=std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1,1).write_to(&mut png,image::ImageFormat::Png).map_err(|e| e.to_string())?;
        let mut bytes=png.into_inner();
        bytes.resize(22*1024*1024,0);
        let mut book=Book::default();
        for id in 2..=4 { book.sheets[0].images.push(picture(id,bytes.clone())); }
        book.next_id=5;
        assert!(book.validate().err().is_some_and(|error| error.contains("64 МиБ")));
        Ok(())
    }
}

fn match_ranges(text:&str,query:&str,case_sensitive:bool)->Vec<(usize,usize)> {
    if query.is_empty(){return Vec::new();}
    if case_sensitive{return text.match_indices(query).map(|(start,value)|(start,start+value.len())).collect();}
    let mut folded=String::new();
    let mut boundaries=Vec::new();
    for (offset,ch) in text.char_indices() {
        boundaries.push((folded.len(),offset));
        folded.extend(ch.to_lowercase());
    }
    boundaries.push((folded.len(),text.len()));
    let needle:String=query.chars().flat_map(char::to_lowercase).collect();
    folded.match_indices(&needle).filter_map(|(start,value)| {
        let a=boundaries.binary_search_by_key(&start,|pair|pair.0).ok()?;
        let end=start+value.len();
        let z=boundaries.binary_search_by_key(&end,|pair|pair.0).ok()?;
        Some((boundaries[a].1,boundaries[z].1))
    }).collect()
}
#[cfg(test)]
mod additional_replace_defect_tests {
    use super::*;
    #[test]
    fn replacement_uses_the_same_case_mode_as_search_without_splitting_unicode()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"КОТ кот İ")?;
        assert_eq!(book.find(id,"Кот",false,10)?.len(),1);
        assert_eq!(book.replace_all_with_case(id,"Кот","пёс",false)?,1);
        assert_eq!(book.active_sheet().and_then(|s|s.cell(0,0)).map(|c|c.input.as_str()),Some("пёс пёс İ"));
        assert_eq!(book.replace_all_with_case(id,"i\u{0307}","X",false)?,1);
        assert_eq!(book.active_sheet().and_then(|s|s.cell(0,0)).map(|c|c.input.as_str()),Some("пёс пёс X"));
        Ok(())
    }
    #[test]
    fn replacement_checks_the_workbook_text_budget_before_any_mutation()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"a")?;book.set_cell(id,0,1,"a")?;
        let before=book.clone();let limit=book.text_bytes()+3;
        assert!(book.replace_all_with_limit(id,"a","1234",true,limit).is_err());
        assert_eq!(book,before);Ok(())
    }
}

#[cfg(test)]
mod hundred_book_budget_tests {
    use super::*;
    #[test]
    fn chart_titles_and_picture_descriptions_share_the_workbook_text_budget() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        let before = book.text_bytes();
        book.add_chart(id, ChartKind::Column, Range::new(0, 0, 0, 1), &"x".repeat(128))?;
        assert!(book.validate_text_budget(before + 127).is_err());
        assert!(book.validate_text_budget(before + 128).is_ok());
        Ok(())
    }
}

#[cfg(test)]
mod phase200_model_defects {
    use super::*;
    #[test]
    fn covered_merge_inputs_are_rejected_by_mutation_and_native_validation() -> Result<(), String> {
        let mut book=Book::default();let id=book.active_id();
        book.merge(id,Range::new(0,0,0,1))?;
        let before=book.clone();
        assert!(book.set_cell(id,0,1,"hidden").is_err());
        assert_eq!(book,before);
        book.set_input(id,0,1,"")?;
        book.sheet_by_id_mut(id).and_then(|sheet|sheet.cell_mut(0,1)).ok_or("cell")?.input="hidden".into();
        assert!(book.validate().is_err());
        Ok(())
    }
    #[test]
    fn range_and_format_replication_check_final_budgets_before_mutation() -> Result<(), String> {
        let mut book=Book::default();let id=book.active_id();let before=book.clone();
        assert!(book.set_range_with_limit(id,0,0,&[vec!["x".repeat(50),"y".repeat(50)]],100).is_err());
        assert_eq!(book,before);
        let fmt=CellFormat{font:"f".repeat(50),number:"0".repeat(50),..CellFormat::default()};
        assert!(book.set_format_with_limit(id,Range::new(0,0,0,1),fmt,100).is_err());
        assert_eq!(book,before);
        Ok(())
    }
    #[test]
    fn budget_neutral_replacements_and_fills_do_not_fail_on_prefix_growth() -> Result<(), String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_range(id,0,0,&[vec!["k".into(),"K".into()]])?;
        let limit=book.text_bytes();
        assert_eq!(book.replace_all_with_limit(id,"k","é",false,limit)?,2);
        assert_eq!(book.text_bytes(),limit);
        book.set_range(id,0,0,&[vec!["x".into(),"long-value".into(),"long-value".into(),"x".into()]])?;
        let limit=book.text_bytes();
        book.fill_with_limit(id,Range::new(0,2,0,3),Range::new(0,0,0,1),limit)?;
        assert_eq!(book.text_bytes(),limit);
        Ok(())
    }
}

#[cfg(test)]
mod phase200_print_defects {
    use super::*;
    fn fill(book:&mut Book,row:u32,col:u32,color:[u8;3])->Result<(),String> {
        let id=book.active_id();book.set_format(id,Range::new(row,col,row,col),CellFormat{fill:Some(color),..CellFormat::default()})
    }
    fn rect_width(pages:&[Page],color:[u8;3])->Result<f32,String> {
        pages.iter().flat_map(|page|&page.items).find_map(|item|match item {
            Primitive::Rect{w,color:actual,..} if *actual==color=>Some(*w),_=>None,
        }).ok_or("rectangle".into())
    }
    #[test]
    fn print_geometry_preserves_custom_width_and_height_ratios() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"first")?;book.set_cell(id,0,1,"second")?;
        fill(&mut book,0,0,[1,2,3])?;fill(&mut book,0,1,[4,5,6])?;
        book.sheets[0].widths.insert(0,200.0);book.sheets[0].widths.insert(1,100.0);
        let pages=book.pages();
        assert!((rect_width(&pages,[1,2,3])?/rect_width(&pages,[4,5,6])?-2.0).abs()<1e-5);
        Ok(())
    }
    #[test]
    fn print_compacts_hidden_axes_before_assigning_pages() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"first")?;book.set_cell(id,39,0,"last")?;
        book.sheets[0].hidden_rows.extend(1..39);
        assert_eq!(book.print_page_count(),1);
        assert_eq!(book.pages().len(),1);
        let axis=PrintAxis::new(26.0,&BTreeMap::new(),&book.sheets[0].hidden_rows,MAX_ROWS);
        assert_eq!(axis.original(1),39);
        assert_eq!(axis.position(39),26.0);
        Ok(())
    }
    #[test]
    fn print_merges_use_one_rectangle_and_retain_hidden_anchor_text() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"merged")?;book.merge(id,Range::new(0,0,1,1))?;
        fill(&mut book,0,0,[1,2,3])?;fill(&mut book,0,1,[7,8,9])?;fill(&mut book,2,0,[4,5,6])?;
        let pages=book.pages();
        assert!((rect_width(&pages,[1,2,3])?/rect_width(&pages,[4,5,6])?-2.0).abs()<1e-5);
        assert!(rect_width(&pages,[7,8,9]).is_err());
        book.sheets[0].hidden_cols.insert(0);
        assert!(book.pages().iter().flat_map(|page|&page.items).any(|item|matches!(item,Primitive::TextStyled{text,..} if text=="merged")));
        Ok(())
    }
    #[test]
    fn print_keeps_font_sizes_above_fourteen_points() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,0,0,"Large")?;
        book.sheets[0].cell_mut(0,0).ok_or("cell")?.fmt.size=36.0;
        let size=book.pages().iter().flat_map(|page|&page.items).find_map(|item|match item {
            Primitive::TextStyled{text,size,..} if text=="Large"=>Some(*size),_=>None,
        }).ok_or("text")?;
        assert!(size>30.0);
        Ok(())
    }
    #[test]
    fn print_cell_borders_draw_all_four_sides() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,0,0,"Border")?;
        book.sheets[0].cell_mut(0,0).ok_or("cell")?.fmt.border=true;
        let count=book.pages().iter().flat_map(|page|&page.items).filter(|item|matches!(item,
            Primitive::Line{color:[70,70,70],..})).count();
        assert_eq!(count,4);
        Ok(())
    }
    #[test]
    fn print_applies_the_first_matching_conditional_fill() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,0,0,"20")?;
        book.sheets[0].conditional.push(ConditionalRule{range:Range::new(0,0,0,0),operator:">".into(),threshold:10.0,fill:[9,8,7]});
        assert!(rect_width(&book.pages(),[9,8,7]).is_ok());
        Ok(())
    }
    #[test]
    fn print_includes_pictures_and_counts_their_anchor_pages() -> Result<(),String> {
        let mut png=std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2,2).write_to(&mut png,image::ImageFormat::Png).map_err(|error|error.to_string())?;
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,0,0,"first")?;
        book.add_image(id,100,20,png.into_inner())?;
        let pages=book.pages();
        assert_eq!(book.print_page_count(),pages.len());
        assert_eq!(pages.len(),2);
        assert_eq!(pages.iter().flat_map(|page|&page.items).filter(|item|matches!(item,Primitive::Image{..})).count(),1);
        Ok(())
    }
}
