use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    pub background: [u8; 3],
    pub foreground: [u8; 3],
    pub accent: [u8; 3],
    pub font: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Trine".into(),
            background: [255, 255, 255],
            foreground: [31, 41, 55],
            accent: [230, 106, 45],
            font: "Arial".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    #[default]
    Text,
    Rect,
    Ellipse,
    Line,
    Arrow,
    Image {
        bytes: Vec<u8>,
    },
    Table {
        cells: Vec<Vec<String>>,
    },
    Chart {
        values: Vec<f32>,
        labels: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Animation {
    pub effect: String,
    pub trigger: String,
    pub delay: f32,
    pub duration: f32,
}
impl Default for Animation {
    fn default() -> Self {
        Self {
            effect: "none".into(),
            trigger: "with_previous".into(),
            delay: 0.0,
            duration: 0.5,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transition {
    pub effect: String,
    pub duration: f32,
    pub advance_after: Option<f32>,
}
impl Default for Transition {
    fn default() -> Self {
        Self {
            effect: "none".into(),
            duration: 0.5,
            advance_after: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Element {
    pub id: u64,
    pub kind: ElementKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rotation: f32,
    pub fill: [u8; 3],
    pub stroke: [u8; 3],
    pub stroke_width: f32,
    pub text: String,
    pub font_size: f32,
    pub color: [u8; 3],
    pub bold: bool,
    pub italic: bool,
    pub font: String,
    pub group: Option<u64>,
    pub animation: Animation,
}
impl Default for Element {
    fn default() -> Self {
        Self {
            id: 0,
            kind: ElementKind::Text,
            x: 60.0,
            y: 80.0,
            w: 600.0,
            h: 80.0,
            rotation: 0.0,
            fill: [237, 242, 247],
            stroke: [74, 85, 104],
            stroke_width: 1.0,
            text: String::new(),
            font_size: 28.0,
            color: [31, 41, 55],
            bold: false,
            italic: false,
            font: "Calibri".into(),
            group: None,
            animation: Animation::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slide {
    pub id: u64,
    pub title: String,
    pub elements: Vec<Element>,
    pub background: [u8; 3],
    pub notes: String,
    pub hidden: bool,
    pub layout: String,
    pub section: String,
    pub transition: Transition,
}
impl Default for Slide {
    fn default() -> Self {
        Self {
            id: 0,
            title: "Слайд".into(),
            elements: Vec::new(),
            background: [255, 255, 255],
            notes: String::new(),
            hidden: false,
            layout: "blank".into(),
            section: String::new(),
            transition: Transition::default(),
        }
    }
}
impl Slide {
    pub fn element(&self, id: u64) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }
    pub fn element_mut(&mut self, id: u64) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id == id)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Deck {
    pub width: f32,
    pub height: f32,
    pub theme: Theme,
    pub slides: Vec<Slide>,
    pub next_id: u64,
    pub warnings: Vec<String>,
}
impl Default for Deck {
    fn default() -> Self {
        let mut deck = Self {
            width: 960.0,
            height: 540.0,
            theme: Theme::default(),
            slides: Vec::new(),
            next_id: 1,
            warnings: Vec::new(),
        };
        deck.add_slide("title");
        deck
    }
}
impl Deck {
    pub fn empty() -> Self {
        Self {
            width: 960.0,
            height: 540.0,
            theme: Theme::default(),
            slides: Vec::new(),
            next_id: 1,
            warnings: Vec::new(),
        }
    }
    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }
    pub fn slide(&self, id: u64) -> Option<&Slide> {
        self.slides.iter().find(|s| s.id == id)
    }
    pub fn slide_mut(&mut self, id: u64) -> Option<&mut Slide> {
        self.slides.iter_mut().find(|s| s.id == id)
    }
    pub fn add_slide(&mut self, layout: &str) -> u64 {
        let id = self.allocate_id();
        let mut slide = Slide {
            id,
            title: format!("Слайд {}", self.slides.len() + 1),
            background: self.theme.background,
            layout: layout.into(),
            ..Slide::default()
        };
        if layout != "blank" {
            let title_id = self.allocate_id();
            slide.elements.push(Element {
                id: title_id,
                x: 60.0,
                y: 50.0,
                w: self.width - 120.0,
                h: 80.0,
                font_size: 40.0,
                bold: true,
                text: "Заголовок".into(),
                color: self.theme.foreground,
                font: self.theme.font.clone(),
                ..Element::default()
            });
            let body_id = self.allocate_id();
            slide.elements.push(Element {
                id: body_id,
                x: 60.0,
                y: 160.0,
                w: self.width - 120.0,
                h: self.height - 200.0,
                text: if layout == "title" {
                    "Подзаголовок".into()
                } else {
                    "Текст слайда".into()
                },
                color: self.theme.foreground,
                font: self.theme.font.clone(),
                ..Element::default()
            });
        }
        self.slides.push(slide);
        id
    }
    pub fn add_element(&mut self, slide_id: u64, kind: ElementKind) -> Result<u64, String> {
        if self.slide(slide_id).is_none() {
            return Err("Слайд не найден".into());
        }
        let id = self.allocate_id();
        let mut element = Element {
            id,
            kind,
            color: self.theme.foreground,
            fill: self.theme.accent,
            font: self.theme.font.clone(),
            ..Element::default()
        };
        match &element.kind {
            ElementKind::Text => element.text = "Текст".into(),
            ElementKind::Table { .. } | ElementKind::Chart { .. } | ElementKind::Image { .. } => {
                element.w = 480.0;
                element.h = 280.0;
            }
            ElementKind::Line | ElementKind::Arrow => {
                element.w = 220.0;
                element.h = 80.0;
            }
            _ => {
                element.w = 180.0;
                element.h = 120.0;
            }
        }
        if let Some(slide) = self.slide_mut(slide_id) {
            slide.elements.push(element);
        }
        Ok(id)
    }
    pub fn duplicate_slide(&mut self, id: u64) -> Result<u64, String> {
        let Some(index) = self.slides.iter().position(|s| s.id == id) else {
            return Err("Слайд не найден".into());
        };
        let Some(mut copy) = self.slides.get(index).cloned() else {
            return Err("Слайд не найден".into());
        };
        copy.id = self.allocate_id();
        copy.title.push_str(" (копия)");
        let mut groups = std::collections::BTreeMap::new();
        for element in &mut copy.elements {
            element.id = self.allocate_id();
            element.group = element
                .group
                .map(|group| *groups.entry(group).or_insert_with(|| self.allocate_id()));
        }
        let new_id = copy.id;
        self.slides.insert(index + 1, copy);
        Ok(new_id)
    }
    pub fn delete_slide(&mut self, id: u64) -> Result<(), String> {
        let Some(index) = self.slides.iter().position(|s| s.id == id) else {
            return Err("Слайд не найден".into());
        };
        self.slides.remove(index);
        Ok(())
    }
    pub fn move_slide(&mut self, id: u64, index: usize) -> Result<(), String> {
        if index >= self.slides.len() {
            return Err("Некорректный индекс слайда".into());
        }
        let Some(old_index) = self.slides.iter().position(|s| s.id == id) else {
            return Err("Слайд не найден".into());
        };
        let slide = self.slides.remove(old_index);
        self.slides.insert(index, slide);
        Ok(())
    }
    pub fn set_layout(&mut self, slide_id: u64, layout: &str) -> Result<(), String> {
        if !matches!(layout, "title" | "content" | "blank" | "two_content") {
            return Err("Неизвестный макет".into());
        }
        let width = self.width;
        let height = self.height;
        let Some(slide) = self.slide_mut(slide_id) else {
            return Err("Слайд не найден".into());
        };
        slide.layout = layout.into();
        if layout == "blank" {
            return Ok(());
        }
        let text_ids = slide
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Text))
            .map(|e| e.id)
            .collect::<Vec<_>>();
        for (index, id) in text_ids.into_iter().enumerate() {
            if let Some(e) = slide.element_mut(id) {
                if index == 0 {
                    e.x = 60.0;
                    e.y = 50.0;
                    e.w = (width - 120.0).max(1.0);
                    e.h = 80.0;
                    e.font_size = 40.0;
                    e.bold = true;
                } else if layout == "two_content" {
                    e.x = 60.0 + ((index - 1) % 2) as f32 * (width / 2.0);
                    e.y = 160.0;
                    e.w = (width / 2.0 - 90.0).max(1.0);
                    e.h = (height - 200.0).max(1.0);
                } else {
                    e.x = 60.0;
                    e.y = 160.0;
                    e.w = (width - 120.0).max(1.0);
                    e.h = (height - 200.0).max(1.0);
                }
            }
        }
        Ok(())
    }
    pub fn export_warnings(&self) -> Vec<String> {
        let mut warnings = self.warnings.clone();
        if self
            .slides
            .iter()
            .any(|s| s.elements.iter().any(|e| e.animation.effect != "none"))
        {
            warnings.push(
                "Анимации воспроизводятся в TrineDocs; PPTX timing пока не экспортируется".into(),
            );
        }
        if self
            .slides
            .iter()
            .any(|s| s.elements.iter().any(|e| e.group.is_some()))
        {
            warnings.push("Группы экспортируются в PPTX как отдельные объекты".into());
        }
        if self
            .slides
            .iter()
            .any(|s| s.elements.iter().any(|e| e.rotation != 0.0))
        {
            warnings.push("PDF и PNG пока не сохраняют поворот объектов".into());
        }
        warnings
    }
    pub fn duplicate_element(&mut self, slide_id: u64, element_id: u64) -> Result<u64, String> {
        let Some(mut copy) = self
            .slide(slide_id)
            .and_then(|s| s.element(element_id))
            .cloned()
        else {
            return Err("Объект не найден".into());
        };
        copy.id = self.allocate_id();
        copy.x += 20.0;
        copy.y += 20.0;
        copy.group = None;
        let id = copy.id;
        if let Some(slide) = self.slide_mut(slide_id) {
            slide.elements.push(copy);
        }
        Ok(id)
    }
    pub fn delete_element(&mut self, slide_id: u64, element_id: u64) -> Result<(), String> {
        let Some(slide) = self.slide_mut(slide_id) else {
            return Err("Слайд не найден".into());
        };
        let count = slide.elements.len();
        slide.elements.retain(|e| e.id != element_id);
        if count == slide.elements.len() {
            return Err("Объект не найден".into());
        }
        Ok(())
    }
    pub fn set_z_order(
        &mut self,
        slide_id: u64,
        element_id: u64,
        front: bool,
    ) -> Result<(), String> {
        let Some(slide) = self.slide_mut(slide_id) else {
            return Err("Слайд не найден".into());
        };
        let Some(index) = slide.elements.iter().position(|e| e.id == element_id) else {
            return Err("Объект не найден".into());
        };
        let element = slide.elements.remove(index);
        if front {
            slide.elements.push(element);
        } else {
            slide.elements.insert(0, element);
        }
        Ok(())
    }
    pub fn set_z_order_many(
        &mut self,
        slide_id: u64,
        element_ids: &[u64],
        front: bool,
    ) -> Result<(), String> {
        let slide = self.slide_mut(slide_id).ok_or("Слайд не найден")?;
        let selected: std::collections::HashSet<_> = element_ids.iter().copied().collect();
        if selected.iter().any(|id| slide.element(*id).is_none()) {
            return Err("Объект не найден".into());
        }
        let (mut moved, mut remaining): (Vec<_>, Vec<_>) = std::mem::take(&mut slide.elements)
            .into_iter()
            .partition(|element| selected.contains(&element.id));
        if front {
            remaining.append(&mut moved);
            slide.elements = remaining;
        } else {
            moved.append(&mut remaining);
            slide.elements = moved;
        }
        Ok(())
    }
    fn table_cells_mut(
        &mut self,
        slide_id: u64,
        element_id: u64,
    ) -> Result<&mut Vec<Vec<String>>, String> {
        match self
            .slide_mut(slide_id)
            .ok_or("Слайд не найден")?
            .element_mut(element_id)
            .map(|e| &mut e.kind)
        {
            Some(ElementKind::Table { cells }) => Ok(cells),
            Some(_) => Err("invalid_args: объект не является таблицей".into()),
            None => Err("Объект не найден".into()),
        }
    }
    /// Writes a block of values into a slide table starting at (row, col), growing it as needed.
    pub fn write_table(
        &mut self,
        slide_id: u64,
        element_id: u64,
        row: usize,
        col: usize,
        values: &[Vec<String>],
    ) -> Result<(), String> {
        let cells = self.table_cells_mut(slide_id, element_id)?;
        let width = values.iter().map(Vec::len).max().unwrap_or(0);
        let rows = cells.len().max(row.saturating_add(values.len()));
        let columns = cells
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(col.saturating_add(width))
            .max(1);
        if rows > 1000 || columns > 1000 {
            return Err("invalid_range: таблица слайда ограничена 1000×1000".into());
        }
        cells.resize_with(rows, Vec::new);
        for line in cells.iter_mut() {
            line.resize_with(columns, String::new);
        }
        for (r, line) in values.iter().enumerate() {
            for (c, value) in line.iter().enumerate() {
                if let Some(cell) = cells.get_mut(row + r).and_then(|l| l.get_mut(col + c)) {
                    cell.clone_from(value);
                }
            }
        }
        Ok(())
    }
    /// Inserts or deletes a table row (`rows`) or column at `index`; the last row or column cannot be deleted.
    pub fn edit_table(
        &mut self,
        slide_id: u64,
        element_id: u64,
        rows: bool,
        insert: bool,
        index: usize,
    ) -> Result<(), String> {
        let cells = self.table_cells_mut(slide_id, element_id)?;
        let columns = cells.iter().map(Vec::len).max().unwrap_or(0).max(1);
        for line in cells.iter_mut() {
            line.resize_with(columns, String::new);
        }
        let count = if rows { cells.len() } else { columns };
        if insert {
            if index > count || count >= 1000 {
                return Err("invalid_range: index".into());
            }
            if rows {
                cells.insert(index, vec![String::new(); columns]);
            } else {
                for line in cells.iter_mut() {
                    line.insert(index, String::new());
                }
            }
        } else {
            if index >= count || count <= 1 {
                return Err("invalid_range: нельзя удалить эту строку или столбец".into());
            }
            if rows {
                cells.remove(index);
            } else {
                for line in cells.iter_mut() {
                    line.remove(index);
                }
            }
        }
        Ok(())
    }
    pub fn apply_theme(&mut self, theme: Theme) {
        let old = self.theme.clone();
        for slide in &mut self.slides {
            if slide.background == old.background {
                slide.background = theme.background;
            }
            for element in &mut slide.elements {
                if element.color == old.foreground {
                    element.color = theme.foreground;
                }
                if element.fill == old.accent {
                    element.fill = theme.accent;
                }
                if element.font == old.font {
                    element.font.clone_from(&theme.font);
                }
            }
        }
        self.theme = theme;
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.width.is_finite()
            || !self.height.is_finite()
            || !(1.0..=10000.0).contains(&self.width)
            || !(1.0..=10000.0).contains(&self.height)
        {
            return Err("Некорректный размер слайда".into());
        }
        if self.slides.len() > 10000 || self.warnings.len() > 1000 {
            return Err("Слишком много слайдов или сообщений".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut groups = std::collections::HashMap::new();
        let mut text_bytes = self.theme.name.len()
            + self.theme.font.len()
            + self.warnings.iter().map(String::len).sum::<usize>();
        let mut image_bytes = 0_usize;
        let mut table_cells = 0_usize;
        let mut chart_points = 0_usize;
        let mut elements = 0_usize;
        let mut layout_lines = 0_usize;
        for slide in &self.slides {
            if slide.id == 0 || !ids.insert(slide.id) {
                return Err("Повторяющийся ID слайда".into());
            }
            text_bytes = text_bytes.saturating_add(
                slide.title.len() + slide.notes.len() + slide.section.len() + slide.layout.len(),
            );
            elements = elements.saturating_add(slide.elements.len());
            if slide.elements.len() > 10000 || elements > 100000 {
                return Err("Слишком много объектов".into());
            }
            if !matches!(
                slide.transition.effect.as_str(),
                "none" | "fade" | "push" | "wipe"
            ) || !slide.transition.duration.is_finite()
                || !(0.001..=3600.0).contains(&slide.transition.duration)
                || slide
                    .transition
                    .advance_after
                    .is_some_and(|v| !v.is_finite() || !(0.001..=86400.0).contains(&v))
            {
                return Err("Некорректный переход".into());
            }
            for element in &slide.elements {
                if element.id == 0 || !ids.insert(element.id) {
                    return Err("Повторяющийся ID объекта".into());
                }
                if [
                    element.x,
                    element.y,
                    element.w,
                    element.h,
                    element.rotation,
                    element.font_size,
                    element.stroke_width,
                ]
                .iter()
                .any(|v| !v.is_finite())
                    || !(0.001..=10000.0).contains(&element.w)
                    || !(0.001..=10000.0).contains(&element.h)
                    || !(1.0..=300.0).contains(&element.font_size)
                    || element.font.len() > 64
                    || !(0.0..=300.0).contains(&element.stroke_width)
                    || element.x.abs() > 1000000.0
                    || element.y.abs() > 1000000.0
                    || element.rotation.abs() > 360000.0
                {
                    return Err("Некорректная геометрия объекта".into());
                }
                if !matches!(
                    element.animation.effect.as_str(),
                    "none" | "fade" | "fly_in" | "zoom"
                ) || !matches!(
                    element.animation.trigger.as_str(),
                    "with_previous" | "after_previous" | "on_click"
                ) || !element.animation.duration.is_finite()
                    || !(0.001..=3600.0).contains(&element.animation.duration)
                    || !element.animation.delay.is_finite()
                    || !(0.0..=86400.0).contains(&element.animation.delay)
                {
                    return Err("Некорректная анимация".into());
                }
                if let Some(group) = element.group
                    && (group == 0
                        || groups
                            .insert(group, slide.id)
                            .is_some_and(|previous| previous != slide.id))
                {
                    return Err("Некорректный ID группы".into());
                }
                text_bytes = text_bytes.saturating_add(element.text.len());
                if text_bytes > 8 * 1024 * 1024 {
                    return Err("Текст презентации превышает 8 МБ".into());
                }
                if !element.text.is_empty() {
                    let lines = layout_line_budget(&element.text, element.w, element.font_size);
                    if lines > 10000 {
                        return Err("Текст объекта превышает безопасный предел строк".into());
                    }
                    layout_lines = layout_lines.saturating_add(lines);
                }
                match &element.kind {
                    ElementKind::Image { bytes } => {
                        image_bytes = image_bytes.saturating_add(bytes.len());
                        if bytes.len() > 32 * 1024 * 1024 || image_bytes > 64 * 1024 * 1024 {
                            return Err("Изображения превышают допустимый размер".into());
                        }
                        image_dimensions(bytes)?;
                    }
                    ElementKind::Table { cells } => {
                        table_cells =
                            table_cells.saturating_add(cells.iter().map(Vec::len).sum::<usize>());
                        if cells.len() > 1000
                            || cells.iter().any(|row| row.len() > 1000)
                            || table_cells > 100000
                        {
                            return Err("Таблица слишком велика".into());
                        }
                        text_bytes = text_bytes
                            .saturating_add(cells.iter().flatten().map(String::len).sum::<usize>());
                        if text_bytes > 8 * 1024 * 1024 {
                            return Err("Текст презентации превышает 8 МБ".into());
                        }
                        let columns = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
                        let width = element.w / columns as f32;
                        let size = element
                            .font_size
                            .min(element.h / cells.len().max(1) as f32 * 0.7);
                        for text in cells.iter().flatten() {
                            let lines = layout_line_budget(text, width, size);
                            if lines > 10000 {
                                return Err("Текст ячейки превышает безопасный предел строк".into());
                            }
                            layout_lines = layout_lines.saturating_add(lines);
                            if layout_lines > 100000 {
                                return Err("Презентация превышает безопасный предел строк".into());
                            }
                        }
                    }
                    ElementKind::Chart { values, labels } => {
                        chart_points = chart_points.saturating_add(values.len().max(labels.len()));
                        if values.len() > 10000
                            || labels.len() > 10000
                            || chart_points > 100000
                            || values.iter().any(|v| !v.is_finite() || v.abs() > 1.0e20)
                        {
                            return Err("Некорректные данные диаграммы".into());
                        }
                        text_bytes = text_bytes
                            .saturating_add(labels.iter().map(String::len).sum::<usize>());
                        for label in labels {
                            layout_lines = layout_lines.saturating_add(
                                label
                                    .chars()
                                    .filter(|ch| matches!(ch, '\n' | '\r'))
                                    .count()
                                    .saturating_add(1),
                            );
                        }
                    }
                    _ => {}
                }
                if text_bytes > 8 * 1024 * 1024 {
                    return Err("Текст презентации превышает 8 МБ".into());
                }
                if layout_lines > 100000 {
                    return Err("Презентация превышает безопасный предел строк".into());
                }
            }
        }
        if text_bytes > 8 * 1024 * 1024 {
            return Err("Текст презентации превышает 8 МБ".into());
        }
        if groups.keys().any(|id| ids.contains(id)) {
            return Err("ID группы совпадает с ID объекта".into());
        }
        let max_id = ids.iter().chain(groups.keys()).copied().max().unwrap_or(0);
        if self.next_id <= max_id || self.next_id == u64::MAX {
            return Err("Некорректный счётчик ID".into());
        }
        Ok(())
    }
    pub fn pages(&self) -> Vec<crate::render::Page> {
        self.slides.iter().map(|slide| self.page(slide)).collect()
    }
    pub fn page(&self, slide: &Slide) -> crate::render::Page {
        use crate::render::Primitive;
        let mut items = vec![Primitive::Rect {
            x: 0.0,
            y: 0.0,
            w: self.width,
            h: self.height,
            color: slide.background,
        }];
        for element in &slide.elements {
            match &element.kind {
                ElementKind::Text => {}
                ElementKind::Line | ElementKind::Arrow if element.stroke_width > 0.0 => {
                    let [x1, y1, x2, y2] = line_points(element);
                    items.push(Primitive::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        width: element.stroke_width,
                        color: element.stroke,
                    })
                }
                ElementKind::Line | ElementKind::Arrow => {}
                ElementKind::Ellipse => {
                    items.push(Primitive::Ellipse {
                        x: element.x,
                        y: element.y,
                        w: element.w,
                        h: element.h,
                        color: element.fill,
                    });
                    if element.stroke_width > 0.0 {
                        for index in 0..48 {
                            let a = index as f32 * std::f32::consts::TAU / 48.0;
                            let b = (index + 1) as f32 * std::f32::consts::TAU / 48.0;
                            items.push(Primitive::Line {
                                x1: element.x + element.w * (1.0 + a.cos()) * 0.5,
                                y1: element.y + element.h * (1.0 + a.sin()) * 0.5,
                                x2: element.x + element.w * (1.0 + b.cos()) * 0.5,
                                y2: element.y + element.h * (1.0 + b.sin()) * 0.5,
                                width: element.stroke_width,
                                color: element.stroke,
                            });
                        }
                    }
                }
                ElementKind::Table { cells } => {
                    let columns = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
                    let rows = cells.len().max(1);
                    let cw = element.w / columns as f32;
                    let ch = element.h / rows as f32;
                    for (row, values) in cells.iter().enumerate() {
                        for (col, text) in values.iter().enumerate() {
                            let x = element.x + col as f32 * cw;
                            let y = element.y + row as f32 * ch;
                            items.push(Primitive::Rect {
                                x,
                                y,
                                w: cw,
                                h: ch,
                                color: if row == 0 {
                                    element.fill
                                } else {
                                    [245, 247, 250]
                                },
                            });
                            let cell = Element {
                                x,
                                y,
                                w: cw,
                                h: ch,
                                text: text.clone(),
                                font_size: element.font_size.min(ch * 0.7),
                                bold: element.bold,
                                italic: element.italic,
                                font: element.font.clone(),
                                ..Element::default()
                            };
                            for (index, line) in text_lines(&cell).into_iter().enumerate() {
                                items.push(Primitive::TextStyled {
                                    x: x + 5.0,
                                    y: y + 5.0 + index as f32 * cell.font_size * 1.2,
                                    text: line,
                                    size: cell.font_size,
                                    color: element.color,
                                    bold: element.bold,
                                    italic: element.italic,
                                    font: element.font.clone(),
                                });
                            }
                        }
                    }
                    if element.stroke_width > 0.0 {
                        for col in 0..=columns {
                            let x = element.x + col as f32 * cw;
                            items.push(Primitive::Line {
                                x1: x,
                                y1: element.y,
                                x2: x,
                                y2: element.y + element.h,
                                width: element.stroke_width,
                                color: element.stroke,
                            });
                        }
                        for row in 0..=rows {
                            let y = element.y + row as f32 * ch;
                            items.push(Primitive::Line {
                                x1: element.x,
                                y1: y,
                                x2: element.x + element.w,
                                y2: y,
                                width: element.stroke_width,
                                color: element.stroke,
                            });
                        }
                    }
                }
                ElementKind::Chart { values, labels } => {
                    let (min, max) = chart_bounds(values);
                    let range = (max - min).max(f32::EPSILON);
                    let plot_height = (element.h - 30.0).max(1.0);
                    let baseline = element.y + plot_height * max / range;
                    let cw = element.w / values.len().max(1) as f32;
                    if element.stroke_width > 0.0 {
                        items.push(Primitive::Line {
                            x1: element.x,
                            y1: baseline,
                            x2: element.x + element.w,
                            y2: baseline,
                            width: element.stroke_width,
                            color: element.stroke,
                        });
                    }
                    for (index, value) in values.iter().enumerate() {
                        let height = value.abs() / range * plot_height;
                        items.push(Primitive::Rect {
                            x: element.x + index as f32 * cw + 4.0,
                            y: if *value >= 0.0 {
                                baseline - height
                            } else {
                                baseline
                            },
                            w: (cw - 8.0).max(1.0),
                            h: height,
                            color: element.fill,
                        });
                        let label = labels
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| (index + 1).to_string());
                        let center =
                            element.x + index as f32 * cw + 4.0 + (cw - 8.0).max(1.0) / 2.0;
                        let label = label
                            .replace("\r\n", "\n")
                            .replace('\r', "\n")
                            .replace('\t', "    ");
                        for (line_index, line) in label.split('\n').enumerate() {
                            items.push(Primitive::Text {
                                x: center - text_width(line, 14.0, false, false) / 2.0,
                                y: element.y + element.h - 24.0 + line_index as f32 * 14.0 * 1.2,
                                text: line.into(),
                                size: 14.0,
                                color: element.color,
                            });
                        }
                    }
                }
                ElementKind::Image { bytes } => items.push(Primitive::Image {
                    x: element.x,
                    y: element.y,
                    w: element.w,
                    h: element.h,
                    bytes: bytes.clone(),
                }),
                ElementKind::Rect => items.push(Primitive::Rect {
                    x: element.x,
                    y: element.y,
                    w: element.w,
                    h: element.h,
                    color: element.fill,
                }),
            }
            if !element.text.is_empty() {
                for (index, line) in text_lines(element).iter().enumerate() {
                    items.push(Primitive::TextStyled {
                        x: element.x + 5.0,
                        y: element.y + 5.0 + index as f32 * element.font_size * 1.2,
                        text: line.clone(),
                        size: element.font_size,
                        color: element.color,
                        bold: element.bold,
                        italic: element.italic,
                        font: element.font.clone(),
                    });
                }
            }
            if matches!(element.kind, ElementKind::Rect) && element.stroke_width > 0.0 {
                for (x1, y1, x2, y2) in [
                    (element.x, element.y, element.x + element.w, element.y),
                    (
                        element.x + element.w,
                        element.y,
                        element.x + element.w,
                        element.y + element.h,
                    ),
                    (
                        element.x + element.w,
                        element.y + element.h,
                        element.x,
                        element.y + element.h,
                    ),
                    (element.x, element.y + element.h, element.x, element.y),
                ] {
                    items.push(Primitive::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        width: element.stroke_width,
                        color: element.stroke,
                    });
                }
            }
            if matches!(element.kind, ElementKind::Arrow) && element.stroke_width > 0.0 {
                let angle = std::f32::consts::TAU / 10.0;
                let [x1, y1, tip_x, tip_y] = line_points(element);
                let (w, h) = (tip_x - x1, tip_y - y1);
                for a in [angle, -angle] {
                    let dx = (w * a.cos() - h * a.sin()) / 4.0;
                    let dy = (w * a.sin() + h * a.cos()) / 4.0;
                    items.push(Primitive::Line {
                        x1: tip_x,
                        y1: tip_y,
                        x2: tip_x - dx,
                        y2: tip_y - dy,
                        width: element.stroke_width,
                        color: element.stroke,
                    });
                }
            }
        }
        crate::render::Page {
            width: self.width,
            height: self.height,
            items,
        }
    }
}

/// End points `[x1, y1, x2, y2]` of a line or arrow: the frame diagonal from the top-left to
/// the bottom-right corner, turned by the element rotation around the frame centre.
pub fn line_points(e: &Element) -> [f32; 4] {
    let (cx, cy) = (e.x + e.w / 2.0, e.y + e.h / 2.0);
    let (sin, cos) = e.rotation.to_radians().sin_cos();
    let (hx, hy) = (e.w / 2.0, e.h / 2.0);
    let (rx, ry) = (hx * cos - hy * sin, hx * sin + hy * cos);
    [cx - rx, cy - ry, cx + rx, cy + ry]
}

fn layout_line_budget(text: &str, width: f32, size: f32) -> usize {
    let capacity = (((width - 10.0).max(1.0) / (size * 3.0).max(0.001)).floor() as usize).max(1);
    let mut lines = 0_usize;
    for paragraph in text.split(['\n', '\r']) {
        let characters = paragraph
            .chars()
            .map(|ch| if ch == '\t' { 4_usize } else { 1 })
            .fold(0_usize, usize::saturating_add);
        let wrapped = characters.saturating_add(capacity - 1) / capacity;
        lines = lines.saturating_add(wrapped.saturating_mul(2).saturating_add(1));
        if lines > 10000 {
            return lines;
        }
    }
    lines
}
pub fn image_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err("Поддерживаются изображения PNG и JPEG".into());
    }
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 32_000_000
    {
        return Err("Изображение превышает допустимое число пикселей".into());
    }
    Ok((width, height))
}
pub fn chart_bounds(values: &[f32]) -> (f32, f32) {
    let min = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0_f32, f32::min);
    let max = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0_f32, f32::max);
    if min == max { (0.0, 1.0) } else { (min, max) }
}
pub fn text_width(text: &str, size: f32, bold: bool, italic: bool) -> f32 {
    let advances = crate::fonts::advances(crate::fonts::DEFAULT_FAMILY, bold, italic);
    text.chars()
        .map(|ch| advances.get(&ch).map_or(size * 0.55, |w| w * size))
        .sum()
}
pub fn text_lines(element: &Element) -> Vec<String> {
    let advances = crate::fonts::advances(&element.font, element.bold, element.italic);
    let advance = |ch: char| {
        advances
            .get(&ch)
            .map_or(element.font_size * 0.55, |w| w * element.font_size)
    };
    let limit = (element.w - 10.0).max(1.0);
    let mut lines = Vec::new();
    let normalized = element
        .text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ");
    for paragraph in normalized.split('\n') {
        let mut line = String::new();
        let mut width = 0.0;
        for word in paragraph.split_inclusive(char::is_whitespace) {
            let word_width = word.chars().map(advance).sum::<f32>();
            if !line.is_empty() && width + word_width > limit {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
            }
            for ch in word.chars() {
                let w = advance(ch);
                if !line.is_empty() && width + w > limit {
                    lines.push(std::mem::take(&mut line));
                    width = 0.0;
                }
                line.push(ch);
                width += w;
            }
        }
        lines.push(line);
    }
    lines
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_survive_reorder() {
        let mut d = Deck::default();
        let first = d.slides.first().map(|s| s.id).unwrap_or(0);
        let second = d.add_slide("blank");
        assert!(d.move_slide(second, 0).is_ok());
        assert_eq!(d.slides.first().map(|s| s.id), Some(second));
        assert!(d.slide(first).is_some());
        assert!(d.validate().is_ok());
    }
    #[test]
    fn duplication_uses_unique_ids() {
        let mut d = Deck::default();
        let first = d.slides.first().map(|s| s.id).unwrap_or(0);
        assert!(d.duplicate_slide(first).is_ok());
        assert!(d.validate().is_ok());
    }
    #[test]
    fn json_roundtrip() {
        let d = Deck::default();
        let json = serde_json::to_vec(&d);
        assert!(json.is_ok());
        if let Ok(bytes) = json {
            assert_eq!(serde_json::from_slice::<Deck>(&bytes).ok(), Some(d));
        }
    }
    #[test]
    fn invalid_geometry_is_rejected() {
        let d = Deck {
            width: f32::NAN,
            ..Deck::default()
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn signed_chart_bounds() {
        assert_eq!(chart_bounds(&[-10.0, 30.0]), (-10.0, 30.0));
        assert_eq!(chart_bounds(&[-10.0, -1.0]), (-10.0, 0.0));
    }
    #[test]
    fn text_wrap_preserves_content() {
        let e = Element {
            text: "Длинный текст 日本語 многословный текст".into(),
            w: 90.0,
            font_size: 20.0,
            ..Element::default()
        };
        let lines = text_lines(&e);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(""), e.text);
    }
    #[test]
    fn grouped_slide_duplicate_remaps_groups() -> Result<(), String> {
        let mut deck = Deck::default();
        let slide_id = deck.slides.first().map(|s| s.id).ok_or("slide")?;
        let group = deck.allocate_id();
        for element in &mut deck.slide_mut(slide_id).ok_or("slide")?.elements {
            element.group = Some(group);
        }
        let copy_id = deck.duplicate_slide(slide_id)?;
        let copy = deck.slide(copy_id).ok_or("copy")?;
        let copied_group = copy.elements.first().and_then(|e| e.group).ok_or("group")?;
        assert_ne!(group, copied_group);
        assert!(copy.elements.iter().all(|e| e.group == Some(copied_group)));
        deck.validate()
    }
    #[test]
    fn rejects_bad_group_counter_and_images() -> Result<(), String> {
        let mut deck = Deck::default();
        let first = deck.slides.first_mut().ok_or("slide")?;
        first.elements.first_mut().ok_or("element")?.group = Some(100);
        assert!(deck.validate().is_err());
        let mut deck = Deck::default();
        let id = deck.slides.first().map(|s| s.id).ok_or("slide")?;
        deck.add_element(id, ElementKind::Image { bytes: vec![0; 10] })?;
        assert!(deck.validate().is_err());
        let deck = Deck {
            next_id: u64::MAX,
            ..Deck::default()
        };
        assert!(deck.validate().is_err());
        Ok(())
    }
    #[test]
    fn rejects_aggregate_table_and_chart_overflow() -> Result<(), String> {
        let mut deck = Deck::default();
        let id = deck.slides.first().map(|s| s.id).ok_or("slide")?;
        deck.add_element(
            id,
            ElementKind::Table {
                cells: vec![vec![String::new(); 1000]; 101],
            },
        )?;
        assert!(deck.validate().is_err());
        let mut deck = Deck::default();
        let id = deck.slides.first().map(|s| s.id).ok_or("slide")?;
        deck.add_element(
            id,
            ElementKind::Chart {
                values: vec![f32::MAX],
                labels: vec![],
            },
        )?;
        assert!(deck.validate().is_err());
        Ok(())
    }
    #[test]
    fn multiline_table_pdf_has_lines_and_grid() -> Result<(), String> {
        let mut deck = Deck::default();
        let id = deck.add_slide("blank");
        deck.add_element(
            id,
            ElementKind::Table {
                cells: vec![vec!["A\nB".into(), "C".into()]],
            },
        )?;
        let page = deck.page(deck.slide(id).ok_or("slide")?);
        assert!(
            page.items
                .iter()
                .filter(|item| matches!(item, crate::render::Primitive::Line { .. }))
                .count()
                >= 5
        );
        let texts = page
            .items
            .iter()
            .filter_map(|item| match item {
                crate::render::Primitive::TextStyled { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["A", "B", "C"]);
        Ok(())
    }
    #[test]
    fn zero_chart_value_has_no_visible_bar() -> Result<(), String> {
        let mut deck = Deck::default();
        let id = deck.add_slide("blank");
        deck.add_element(
            id,
            ElementKind::Chart {
                values: vec![0.0],
                labels: vec!["zero".into()],
            },
        )?;
        let page = deck.page(deck.slide(id).ok_or("slide")?);
        assert!(
            page.items
                .iter()
                .any(|item| matches!(item, crate::render::Primitive::Rect { h, .. } if *h == 0.0))
        );
        Ok(())
    }

    #[test]
    fn multiple_object_layer_changes_keep_relative_order() -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        let a = deck.add_element(slide, ElementKind::Text)?;
        let b = deck.add_element(slide, ElementKind::Rect)?;
        let c = deck.add_element(slide, ElementKind::Ellipse)?;
        let d = deck.add_element(slide, ElementKind::Arrow)?;
        deck.set_z_order_many(slide, &[c, a], true)?;
        assert_eq!(
            deck.slide(slide)
                .ok_or("slide")?
                .elements
                .iter()
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            vec![b, d, a, c]
        );
        deck.set_z_order_many(slide, &[c, a], false)?;
        assert_eq!(
            deck.slide(slide)
                .ok_or("slide")?
                .elements
                .iter()
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            vec![a, c, b, d]
        );
        Ok(())
    }
    #[test]
    fn text_layout_normalizes_crlf_and_tabs() {
        let element = Element {
            text: "A\r\nB\tC".into(),
            w: 900.0,
            ..Element::default()
        };
        assert_eq!(text_lines(&element), vec!["A", "B    C"]);
    }
    #[test]
    fn excessive_layout_lines_are_rejected_without_rendering() -> Result<(), String> {
        let mut deck = Deck::empty();
        let id = deck.add_slide("blank");
        let element = deck.add_element(id, ElementKind::Text)?;
        let element = deck
            .slide_mut(id)
            .and_then(|slide| slide.element_mut(element))
            .ok_or("element")?;
        element.text = "\n".repeat(10001);
        assert!(deck.validate().is_err());
        let mut deck = Deck::empty();
        let id = deck.add_slide("blank");
        let element = deck.add_element(id, ElementKind::Text)?;
        let element = deck
            .slide_mut(id)
            .and_then(|slide| slide.element_mut(element))
            .ok_or("element")?;
        element.w = 1.0;
        element.text = "X".repeat(10001);
        assert!(deck.validate().is_err());
        Ok(())
    }
    #[test]
    fn zero_outline_never_emits_pdf_hairlines() -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        for kind in [
            ElementKind::Rect,
            ElementKind::Ellipse,
            ElementKind::Line,
            ElementKind::Arrow,
            ElementKind::Chart {
                values: vec![1.0],
                labels: vec!["A".into()],
            },
            ElementKind::Table {
                cells: vec![vec!["cell".into()]],
            },
        ] {
            let id = deck.add_element(slide, kind)?;
            deck.slide_mut(slide)
                .and_then(|slide| slide.element_mut(id))
                .ok_or("element")?
                .stroke_width = 0.0;
        }
        assert!(
            !deck
                .page(deck.slide(slide).ok_or("slide")?)
                .items
                .iter()
                .any(|item| matches!(item, crate::render::Primitive::Line { .. }))
        );
        Ok(())
    }
}
