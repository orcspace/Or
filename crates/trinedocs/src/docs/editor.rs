use super::model::*;
use egui::{Align2, Color32, Event, FontId, ImeEvent, Key, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Clone)]
struct Clipboard {
    text: String,
    paragraphs: Vec<Paragraph>,
}
struct ImageAsset {
    bytes: Vec<u8>,
    width: f32,
    height: f32,
}
type ImageResult = std::sync::Arc<std::sync::Mutex<Option<Result<Option<ImageAsset>, String>>>>;
#[derive(Clone)]
struct ImageTask {
    anchor: u64,
    result: ImageResult,
}
#[derive(Clone)]
struct EditorState {
    cursor: Position,
    anchor: Position,
    typing: TextFormat,
    zoom: f32,
    show_find: bool,
    search: String,
    replacement: String,
    case_sensitive: bool,
    match_index: usize,
    preedit: String,
    clipboard: Option<Clipboard>,
    format_painter: Option<TextFormat>,
    link: String,
    table_rows: usize,
    table_columns: usize,
    scroll_to_cursor: bool,
    previous: Option<std::sync::Arc<Doc>>,
    cached_layout: Option<std::sync::Arc<Layout>>,
    pending_image: Option<ImageTask>,
    message: Option<String>,
}
impl Default for EditorState {
    fn default() -> Self {
        Self {
            cursor: Position::default(),
            anchor: Position::default(),
            typing: TextFormat::default(),
            zoom: 1.25,
            show_find: false,
            search: String::new(),
            replacement: String::new(),
            case_sensitive: false,
            match_index: 0,
            preedit: String::new(),
            clipboard: None,
            format_painter: None,
            link: String::new(),
            table_rows: 3,
            table_columns: 3,
            scroll_to_cursor: false,
            previous: None,
            cached_layout: None,
            pending_image: None,
            message: None,
        }
    }
}
impl EditorState {
    fn range(&self) -> (Position, Position) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }
    fn selected(&self) -> bool {
        self.cursor != self.anchor
    }
    fn move_to(&mut self, doc: &Doc, pos: Position, extend: bool) {
        self.cursor = doc.clamp_position(pos);
        if !extend {
            self.anchor = self.cursor;
        }
        self.typing = doc
            .paragraphs
            .get(self.cursor.paragraph)
            .map(|p| p.format_at(self.cursor.offset))
            .unwrap_or_default();
        self.scroll_to_cursor = true;
        self.preedit.clear();
    }
    fn insert(&mut self, doc: &mut Doc, text: &str) -> bool {
        let (a, b) = self.range();
        match doc.replace_range(a, b, text, Some(self.typing.clone())) {
            Ok(pos) => {
                self.cursor = pos;
                self.anchor = pos;
                self.scroll_to_cursor = true;
                true
            }
            Err(_) => false,
        }
    }
    fn apply_format(&mut self, doc: &mut Doc, apply: impl Fn(&mut TextFormat)) {
        apply(&mut self.typing);
        if self.selected() {
            let (a, b) = self.range();
            doc.format_range(a, b, apply);
        }
    }
}

pub fn show(ui: &mut egui::Ui, doc: &mut Doc) -> bool {
    doc.normalize();
    let state_id = ui.id().with("docs-state");
    let canvas_id = ui.id().with("docs-canvas");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<EditorState>(state_id))
        .unwrap_or_default();
    let before = doc.clone();
    poll_image(doc, &mut state);
    if let Some(previous) = &state.previous
        && previous.as_ref() != doc
    {
        state.cursor = rebase_position(previous, doc, state.cursor);
        state.anchor = rebase_position(previous, doc, state.anchor);
        state.cached_layout = None;
    }
    state.cursor = doc.clamp_position(state.cursor);
    state.anchor = doc.clamp_position(state.anchor);
    if !doc.losses.is_empty() {
        egui::CollapsingHeader::new(format!("Ограничения импорта: {}", doc.losses.len())).show(
            ui,
            |ui| {
                for loss in &doc.losses {
                    ui.label(loss);
                }
                ui.label(
                    "Сохранение в Office может удалить неподдерживаемые части исходного файла.",
                );
            },
        );
    }
    toolbar(ui, doc, &mut state, canvas_id);
    if state.show_find {
        find_bar(ui, doc, &mut state);
    }
    if state.pending_image.is_some() {
        ui.label("Загрузка изображения…");
    }
    if let Some(message) = &state.message {
        ui.colored_label(Color32::from_rgb(160, 40, 40), message);
    }
    table_panels(ui, doc);
    ui.separator();
    if ui.memory(|m| m.has_focus(canvas_id)) {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                canvas_id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            )
        });
        let event_layout = state
            .cached_layout
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(doc.layout()));
        let events = ui.input(|i| i.events.clone());
        for event in events {
            handle_event(ui, doc, &event_layout, &mut state, event);
        }
    }
    let layout = if state.previous.as_deref() == Some(doc) {
        state
            .cached_layout
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(doc.layout()))
    } else {
        std::sync::Arc::new(doc.layout())
    };
    let scale = state.zoom;
    let page_width = doc.page.width * scale;
    let page_height = doc.page.height * scale;
    let gap = 24.0;
    let total = Vec2::new(
        page_width + 48.0,
        (page_height + gap) * layout.pages.len() as f32 + 24.0,
    );
    let mut cursor_rect = None;
    egui::ScrollArea::both()
        .id_salt("docs-scroll")
        .show_viewport(ui, |ui, viewport| {
            let (canvas, _) = ui.allocate_exact_size(total, Sense::hover());
            let response = ui.interact(canvas, canvas_id, Sense::click_and_drag());
            if response.clicked() || response.drag_started() {
                response.request_focus();
                if let Some(pointer) = response.interact_pointer_pos() {
                    let pos = hit_test(&layout, pointer, canvas.min, scale, page_height, gap);
                    let shift = ui.input(|i| i.modifiers.shift);
                    if response.triple_clicked() {
                        state.anchor = Position {
                            paragraph: pos.paragraph,
                            offset: 0,
                        };
                        state.cursor = Position {
                            paragraph: pos.paragraph,
                            offset: doc
                                .paragraphs
                                .get(pos.paragraph)
                                .map(Paragraph::len)
                                .unwrap_or_default(),
                        };
                    } else if response.double_clicked() {
                        let (a, b) = word_range(doc, pos);
                        state.anchor = a;
                        state.cursor = b;
                    } else {
                        state.move_to(doc, pos, shift);
                    }
                    if let Some(fmt) = state.format_painter.take() {
                        state.typing = fmt.clone();
                        let (a, b) = state.range();
                        doc.format_range(a, b, |f| *f = fmt.clone());
                    }
                }
            }
            if response.dragged()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                state.cursor = hit_test(&layout, pointer, canvas.min, scale, page_height, gap);
                state.scroll_to_cursor = false;
            }
            let focused = response.has_focus();
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
            // Compute the caret even outside the viewport so keyboard/outline navigation can scroll to it.
            cursor_rect = layout
                .rows
                .iter()
                .rev()
                .find(|r| {
                    r.paragraph == state.cursor.paragraph
                        && state.cursor.offset >= r.start
                        && state.cursor.offset <= r.end
                })
                .map(|row| {
                    let origin =
                        canvas.min + egui::vec2(24.0, 12.0 + row.page as f32 * (page_height + gap));
                    Rect::from_min_size(
                        origin + egui::vec2(row_x(row, state.cursor.offset) * scale, row.y * scale),
                        egui::vec2(1.5, row.height * scale),
                    )
                });
            let visible = Rect::from_min_size(canvas.min + viewport.min.to_vec2(), viewport.size())
                .expand(80.0);
            for (page_index, page) in layout.pages.iter().enumerate() {
                let origin =
                    canvas.min + egui::vec2(24.0, 12.0 + page_index as f32 * (page_height + gap));
                let rect = Rect::from_min_size(origin, egui::vec2(page_width, page_height));
                if !rect.intersects(visible) {
                    continue;
                }
                ui.painter().rect_filled(rect, 2.0, Color32::WHITE);
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    Stroke::new(1.0, Color32::from_gray(190)),
                    egui::StrokeKind::Outside,
                );
                crate::render::paint(ui, page, origin, scale);
                let (a, b) = state.range();
                for row in layout.rows.iter().filter(|r| r.page == page_index) {
                    if state.selected()
                        && row.paragraph >= a.paragraph
                        && row.paragraph <= b.paragraph
                    {
                        let from = if row.paragraph == a.paragraph {
                            a.offset.max(row.start)
                        } else {
                            row.start
                        };
                        let to = if row.paragraph == b.paragraph {
                            b.offset.min(row.end)
                        } else {
                            row.end
                        };
                        if to > from {
                            let x1 = row_x(row, from);
                            let x2 = row_x(row, to);
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    origin + egui::vec2(x1 * scale, row.y * scale),
                                    egui::vec2((x2 - x1) * scale, row.height * scale),
                                ),
                                0.0,
                                Color32::from_rgba_unmultiplied(50, 115, 220, 65),
                            );
                        }
                    }
                    if row.paragraph == state.cursor.paragraph
                        && state.cursor.offset >= row.start
                        && state.cursor.offset <= row.end
                    {
                        let x = row_x(row, state.cursor.offset);
                        let rect = Rect::from_min_size(
                            origin + egui::vec2(x * scale, row.y * scale),
                            egui::vec2(1.5, row.height * scale),
                        );
                        cursor_rect = Some(rect);
                        if focused && !state.selected() {
                            ui.painter().line_segment(
                                [rect.left_top(), rect.left_bottom()],
                                Stroke::new(1.4, Color32::from_rgb(25, 80, 180)),
                            );
                            if !state.preedit.is_empty() {
                                ui.painter().text(
                                    rect.left_top(),
                                    Align2::LEFT_TOP,
                                    &state.preedit,
                                    FontId::proportional(state.typing.size * scale),
                                    Color32::from_rgb(30, 80, 180),
                                );
                            }
                        }
                    }
                }
            }
            if state.scroll_to_cursor
                && let Some(rect) = cursor_rect
            {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
            if focused && let Some(rect) = cursor_rect {
                ui.ctx().output_mut(|o| {
                    o.ime = Some(egui::output::IMEOutput {
                        purpose: egui::IMEPurpose::Normal,
                        rect: canvas,
                        cursor_rect: rect,
                        should_interrupt_composition: false,
                    })
                });
            }
        });
    state.scroll_to_cursor = false;
    ui.horizontal(|ui| {
        ui.label(format!(
            "{} стр.  ·  {} слов  ·  {} знаков",
            layout.pages.len(),
            doc.word_count(),
            doc.plain_text().chars().count()
        ));
        ui.separator();
        ui.label(format!(
            "Абзац {} : {}",
            state.cursor.paragraph + 1,
            state.cursor.offset + 1
        ));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Slider::new(&mut state.zoom, 0.5..=2.5).text("Масштаб"));
        });
    });
    let changed = before != *doc;
    if changed {
        state.cached_layout = None;
    } else {
        state.cached_layout = Some(layout);
    }
    if state.previous.as_deref() != Some(doc) {
        state.previous = Some(std::sync::Arc::new(doc.clone()));
    }
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
    changed
}
fn toolbar(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    ui.horizontal_wrapped(|ui| {
        let current_style = doc
            .paragraphs
            .get(state.cursor.paragraph)
            .map(|p| p.style.as_str())
            .unwrap_or("Normal")
            .to_string();
        egui::ComboBox::from_id_salt("docs-style")
            .selected_text(style_label(&current_style))
            .show_ui(ui, |ui| {
                for style in [
                    "Normal", "Title", "Heading1", "Heading2", "Heading3", "Quote",
                ] {
                    if ui
                        .selectable_label(current_style == style, style_label(style))
                        .clicked()
                    {
                        let (a, b) = state.range();
                        for p in doc
                            .paragraphs
                            .iter_mut()
                            .skip(a.paragraph)
                            .take(b.paragraph.saturating_sub(a.paragraph) + 1)
                        {
                            p.set_style(style);
                        }
                        state.typing = doc
                            .paragraphs
                            .get(state.cursor.paragraph)
                            .map(|p| p.format_at(state.cursor.offset))
                            .unwrap_or_default();
                        ui.memory_mut(|m| m.request_focus(canvas_id));
                    }
                }
            });
        let mut font = state.typing.font.clone();
        egui::ComboBox::from_id_salt("docs-font")
            .selected_text(&font)
            .show_ui(ui, |ui| {
                for name in [
                    "Calibri",
                    "Arial",
                    "Times New Roman",
                    "Cambria",
                    "Consolas",
                    "Segoe UI",
                ] {
                    ui.selectable_value(&mut font, name.into(), name);
                }
            });
        if font != state.typing.font {
            state.apply_format(doc, |f| f.font = font.clone());
        }
        let mut size = state.typing.size;
        if ui
            .add(
                egui::DragValue::new(&mut size)
                    .range(4.0..=144.0)
                    .speed(0.5)
                    .suffix(" pt"),
            )
            .changed()
        {
            state.apply_format(doc, |f| f.size = size);
        }
        for (label, on, which) in [
            ("B", state.typing.bold, 0),
            ("I", state.typing.italic, 1),
            ("U", state.typing.underline, 2),
            ("S", state.typing.strike, 3),
        ] {
            if ui.selectable_label(on, label).clicked() {
                state.apply_format(doc, |f| match which {
                    0 => f.bold = !on,
                    1 => f.italic = !on,
                    2 => f.underline = !on,
                    _ => f.strike = !on,
                });
                ui.memory_mut(|m| m.request_focus(canvas_id));
            }
        }
        if ui.button("x²").clicked() {
            let on = state.typing.script == 1;
            state.apply_format(doc, |f| f.script = if on { 0 } else { 1 });
        }
        if ui.button("x₂").clicked() {
            let on = state.typing.script == -1;
            state.apply_format(doc, |f| f.script = if on { 0 } else { -1 });
        }
        let mut color = state.typing.color;
        if ui.color_edit_button_srgb(&mut color).changed() {
            state.apply_format(doc, |f| f.color = color);
        }
        ui.menu_button("Выделение", |ui| {
            for (label, color) in [
                ("Жёлтый", Some([255, 245, 80])),
                ("Зелёный", Some([150, 240, 160])),
                ("Голубой", Some([145, 220, 250])),
                ("Без цвета", None),
            ] {
                if ui.button(label).clicked() {
                    state.apply_format(doc, |f| f.highlight = color);
                    ui.close();
                }
            }
        });
        ui.menu_button("Формат", |ui| {
            if ui.button("Очистить").clicked() {
                state.apply_format(doc, |f| *f = TextFormat::default());
                ui.close();
            }
            if ui.button("Копировать образец").clicked() {
                state.format_painter = Some(state.typing.clone());
                ui.close();
            }
            if ui
                .add_enabled(
                    state.format_painter.is_some(),
                    egui::Button::new("Применить образец"),
                )
                .clicked()
            {
                if let Some(format) = state.format_painter.take() {
                    state.apply_format(doc, |f| *f = format.clone());
                }
                ui.close();
            }
        });
        for (label, align) in [
            ("≡", Align::Left),
            ("☰", Align::Center),
            ("⇥", Align::Right),
        ] {
            if ui
                .button(label)
                .on_hover_text(match align {
                    Align::Left => "По левому краю",
                    Align::Center => "По центру",
                    _ => "По правому краю",
                })
                .clicked()
            {
                let (a, b) = state.range();
                for p in doc
                    .paragraphs
                    .iter_mut()
                    .skip(a.paragraph)
                    .take(b.paragraph - a.paragraph + 1)
                {
                    p.align = align;
                }
            }
        }
        for (label, list) in [("• Список", List::Bullet), ("1. Список", List::Number)]
        {
            if ui.button(label).clicked() {
                let (a, b) = state.range();
                for p in doc
                    .paragraphs
                    .iter_mut()
                    .skip(a.paragraph)
                    .take(b.paragraph - a.paragraph + 1)
                {
                    p.list = if p.list == list { List::None } else { list };
                }
            }
        }
        ui.menu_button("Абзац", |ui| {
            if let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph) {
                ui.label("Отступ, pt");
                ui.add(egui::DragValue::new(&mut p.indent).range(0.0..=250.0));
                ui.label("Первая строка, pt");
                ui.add(egui::DragValue::new(&mut p.first_indent).range(-100.0..=100.0));
                ui.label("Интервал");
                ui.add(
                    egui::DragValue::new(&mut p.line_spacing)
                        .range(0.5..=5.0)
                        .speed(0.05),
                );
                ui.label("До / после, pt");
                ui.add(egui::DragValue::new(&mut p.spacing_before).range(0.0..=100.0));
                ui.add(egui::DragValue::new(&mut p.spacing_after).range(0.0..=100.0));
                ui.checkbox(&mut p.page_break_before, "С новой страницы");
            }
        });
        ui.menu_button("Вставка", |ui| {
            ui.label("Таблица: строки × столбцы");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut state.table_rows).range(1..=100));
                ui.add(egui::DragValue::new(&mut state.table_columns).range(1..=32));
            });
            if ui.button("Вставить таблицу").clicked() {
                if let Some(id) = doc.paragraphs.get(state.cursor.paragraph).map(|p| p.id) {
                    let _ = doc.add_table(id, state.table_rows, state.table_columns);
                }
                ui.close();
            }
            if ui
                .add_enabled(
                    state.pending_image.is_none(),
                    egui::Button::new("Изображение…"),
                )
                .clicked()
            {
                start_image_load(ui, doc, state);
                ui.close();
            }
            if ui.button("Разрыв страницы").clicked() {
                if state.insert(doc, "\n")
                    && let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph)
                {
                    p.page_break_before = true;
                }
                ui.close();
            }
            ui.separator();
            ui.label("Ссылка:");
            ui.text_edit_singleline(&mut state.link);
            if ui.button("Применить ссылку").clicked() {
                let link = state.link.clone();
                if !state.selected() {
                    let start = state.cursor;
                    if state.insert(doc, &link) {
                        state.anchor = start;
                    }
                }
                state.apply_format(doc, |f| {
                    f.link = Some(link.clone());
                    f.underline = true;
                });
                ui.close();
            }
            if ui.button("Дата YYYY-MM-DD").clicked() {
                state.insert(doc, &today());
                ui.close();
            }
            ui.horizontal_wrapped(|ui| {
                for ch in [
                    "©", "®", "™", "€", "₽", "°", "±", "×", "÷", "—", "…", "§", "✓",
                ] {
                    if ui.button(ch).clicked() {
                        state.insert(doc, ch);
                        ui.close();
                    }
                }
            });
        });
        ui.menu_button("Страница", |ui| {
            for (name, width, height) in [
                ("A4", 595.28, 841.89),
                ("Letter", 612.0, 792.0),
                ("A3", 841.89, 1190.55),
                ("Legal", 612.0, 1008.0),
            ] {
                if ui.button(name).clicked() {
                    doc.page.width = width;
                    doc.page.height = height;
                }
            }
            if ui.button("Повернуть страницу").clicked() {
                std::mem::swap(&mut doc.page.width, &mut doc.page.height);
            }
            for (label, value) in [
                ("Слева", &mut doc.page.margin_left),
                ("Справа", &mut doc.page.margin_right),
                ("Сверху", &mut doc.page.margin_top),
                ("Снизу", &mut doc.page.margin_bottom),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.add(egui::DragValue::new(value).range(0.0..=200.0).suffix(" pt"));
                });
            }
            ui.label("Верхний колонтитул");
            ui.text_edit_singleline(&mut doc.page.header);
            ui.label("Нижний колонтитул");
            ui.text_edit_singleline(&mut doc.page.footer);
            ui.checkbox(&mut doc.page.page_numbers, "Номера страниц");
        });
        if ui.button("Найти / заменить").clicked() {
            state.show_find = !state.show_find;
        }
        ui.menu_button("Заголовки", |ui| {
            for (id, _, text) in doc.outline() {
                if ui.button(&text).clicked() {
                    if let Some(paragraph) = doc.paragraphs.iter().position(|p| p.id == id) {
                        state.move_to(
                            doc,
                            Position {
                                paragraph,
                                offset: 0,
                            },
                            false,
                        );
                    }
                    ui.close();
                }
            }
        });
    });
}
fn start_image_load(ui: &egui::Ui, doc: &Doc, state: &mut EditorState) {
    let Some(anchor) = doc.paragraphs.get(state.cursor.paragraph).map(|p| p.id) else {
        return;
    };
    let result = std::sync::Arc::new(std::sync::Mutex::new(None));
    let worker_result = result.clone();
    let ctx = ui.ctx().clone();
    let task = std::thread::Builder::new()
        .name("docs-image-reader".into())
        .spawn(move || {
            let loaded = (|| {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("Изображения", &["png", "jpg", "jpeg"])
                    .pick_file()
                else {
                    return Ok(None);
                };
                let bytes = crate::util::read_file(&path)?;
                if bytes.len() > 32 * 1024 * 1024 {
                    return Err("Изображение превышает 32 МБ".into());
                }
                let image = crate::util::decode_image(&bytes)?;
                let width = (image.width() as f32).min(450.0);
                let height = width * image.height() as f32 / image.width().max(1) as f32;
                Ok(Some(ImageAsset {
                    bytes,
                    width,
                    height,
                }))
            })();
            if let Ok(mut slot) = worker_result.lock() {
                *slot = Some(loaded);
            }
            ctx.request_repaint();
        });
    match task {
        Ok(_) => {
            state.pending_image = Some(ImageTask { anchor, result });
            state.message = None;
        }
        Err(error) => state.message = Some(format!("Не удалось запустить загрузку: {error}")),
    }
}
fn poll_image(doc: &mut Doc, state: &mut EditorState) {
    let Some(task) = state.pending_image.as_ref() else {
        return;
    };
    let result = task.result.lock().ok().and_then(|mut slot| slot.take());
    let anchor = task.anchor;
    if let Some(result) = result {
        state.pending_image = None;
        match result {
            Ok(Some(asset)) => {
                if doc.paragraphs.iter().any(|p| p.id == anchor) {
                    let id = doc.alloc_id();
                    doc.images.push(AnchoredImage {
                        id,
                        after_paragraph_id: anchor,
                        bytes: asset.bytes,
                        width: asset.width,
                        height: asset.height,
                        alt: String::new(),
                    });
                } else {
                    state.message =
                        Some("Место вставки изображения удалено; выберите его заново.".into());
                }
            }
            Ok(None) => {}
            Err(error) => state.message = Some(error),
        }
    }
}
fn style_label(style: &str) -> &str {
    match style {
        "Title" => "Название",
        "Heading1" => "Заголовок 1",
        "Heading2" => "Заголовок 2",
        "Heading3" => "Заголовок 3",
        "Quote" => "Цитата",
        _ => "Обычный",
    }
}
fn find_bar(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Найти:");
        ui.add(egui::TextEdit::singleline(&mut state.search).desired_width(170.0));
        ui.label("Заменить:");
        ui.add(egui::TextEdit::singleline(&mut state.replacement).desired_width(170.0));
        ui.checkbox(&mut state.case_sensitive, "Регистр");
        let matches = doc.find(&state.search, state.case_sensitive);
        ui.label(format!("{} совпадений", matches.len()));
        if ui
            .add_enabled(!matches.is_empty(), egui::Button::new("Следующее"))
            .clicked()
        {
            let index = state.match_index % matches.len();
            if let Some((a, b)) = matches.get(index) {
                state.anchor = *a;
                state.cursor = *b;
                state.scroll_to_cursor = true;
            }
            state.match_index = index + 1;
        }
        if ui
            .add_enabled(state.selected(), egui::Button::new("Заменить"))
            .clicked()
        {
            let replacement = state.replacement.clone();
            state.insert(doc, &replacement);
        }
        if ui
            .add_enabled(!matches.is_empty(), egui::Button::new("Заменить все"))
            .clicked()
        {
            doc.replace_all(&state.search, &state.replacement, state.case_sensitive);
            state.cursor = doc.clamp_position(state.cursor);
            state.anchor = state.cursor;
        }
        if ui.button("×").clicked() {
            state.show_find = false;
        }
    });
}
fn table_panels(ui: &mut egui::Ui, doc: &mut Doc) {
    let mut remove = None;
    let mut next = doc.next_id;
    for (ti, table) in doc.tables.iter_mut().enumerate() {
        egui::CollapsingHeader::new(format!("Таблица {} — редактировать ячейки", ti + 1))
            .id_salt(table.id)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("+ Строка").clicked() {
                        let columns = table.rows.first().map(Vec::len).unwrap_or(1);
                        let mut row = Vec::new();
                        for _ in 0..columns {
                            row.push(new_cell(&mut next));
                        }
                        table.rows.push(row);
                    }
                    if ui.button("+ Столбец").clicked() {
                        for row in &mut table.rows {
                            row.push(new_cell(&mut next));
                        }
                        table.column_widths.push(80.0);
                    }
                    if ui
                        .add_enabled(table.rows.len() > 1, egui::Button::new("− Строка"))
                        .clicked()
                    {
                        table.rows.pop();
                    }
                    if ui
                        .add_enabled(
                            table.rows.first().is_some_and(|r| r.len() > 1),
                            egui::Button::new("− Столбец"),
                        )
                        .clicked()
                    {
                        for row in &mut table.rows {
                            row.pop();
                        }
                        table.column_widths.pop();
                    }
                    if ui.button("Удалить таблицу").clicked() {
                        remove = Some(ti);
                    }
                    ui.label("Границы");
                    ui.color_edit_button_srgb(&mut table.border_color);
                });
                egui::ScrollArea::horizontal()
                    .id_salt(table.id)
                    .show(ui, |ui| {
                        egui::Grid::new(("docs-table", table.id))
                            .striped(true)
                            .show(ui, |ui| {
                                for row in &mut table.rows {
                                    for cell in row {
                                        ui.vertical(|ui| {
                                            let mut text = cell
                                                .paragraphs
                                                .iter()
                                                .map(Paragraph::text)
                                                .collect::<Vec<_>>()
                                                .join("\n");
                                            if ui
                                                .add(
                                                    egui::TextEdit::multiline(&mut text)
                                                        .desired_width(120.0)
                                                        .desired_rows(2),
                                                )
                                                .changed()
                                            {
                                                edit_cell_text(cell, &text, &mut next);
                                            }
                                            ui.color_edit_button_srgb(&mut cell.fill);
                                        });
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });
    }
    if let Some(index) = remove {
        doc.tables.remove(index);
    }
    doc.next_id = next;
    for image in &mut doc.images {
        egui::CollapsingHeader::new(format!("Изображение {}", image.id))
            .id_salt(image.id)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Ширина / высота, pt");
                    ui.add(egui::DragValue::new(&mut image.width).range(1.0..=2000.0));
                    ui.add(egui::DragValue::new(&mut image.height).range(1.0..=2000.0));
                    ui.label("Описание");
                    ui.text_edit_singleline(&mut image.alt);
                });
            });
    }
}
fn edit_cell_text(cell: &mut Cell, text: &str, next_id: &mut u64) {
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
fn take_id(next: &mut u64) -> u64 {
    let id = *next;
    *next = next.saturating_add(1);
    id
}
fn new_cell(next: &mut u64) -> Cell {
    Cell {
        id: take_id(next),
        paragraphs: vec![Paragraph {
            id: take_id(next),
            ..Paragraph::default()
        }],
        fill: [255, 255, 255],
    }
}
fn handle_event(
    ui: &mut egui::Ui,
    doc: &mut Doc,
    layout: &Layout,
    state: &mut EditorState,
    event: Event,
) {
    match event {
        Event::Text(text) => {
            if text == " " && !state.selected() && markdown_input(doc, state) {
                return;
            }
            if !text.is_empty() {
                state.insert(doc, &text);
            }
        }
        Event::Paste(text) => {
            let copied = state.clipboard.as_ref().filter(|c| c.text == text).cloned();
            let (a, _) = state.range();
            state.insert(doc, &text);
            if let Some(copy) = copied {
                for (i, p) in copy.paragraphs.iter().enumerate() {
                    let pi = a.paragraph + i;
                    let mut offset = if i == 0 { a.offset } else { 0 };
                    for run in &p.runs {
                        let end = offset + run.text.chars().count();
                        let fmt = run.format.clone();
                        doc.format_range(
                            Position {
                                paragraph: pi,
                                offset,
                            },
                            Position {
                                paragraph: pi,
                                offset: end,
                            },
                            |f| *f = fmt.clone(),
                        );
                        offset = end;
                    }
                }
            }
        }
        Event::Copy | Event::Cut => {
            if !state.selected() {
                return;
            }
            let cut = matches!(event, Event::Cut);
            let (a, b) = state.range();
            let text = doc.selected_text(a, b);
            ui.ctx().copy_text(text.clone());
            let mut paragraphs = Vec::new();
            for pi in a.paragraph..=b.paragraph {
                if let Some(p) = doc.paragraphs.get(pi) {
                    let from = if pi == a.paragraph { a.offset } else { 0 };
                    let to = if pi == b.paragraph { b.offset } else { p.len() };
                    let mut selected = p.clone();
                    selected.runs = selected_runs(p, from, to);
                    paragraphs.push(selected);
                }
            }
            state.clipboard = Some(Clipboard { text, paragraphs });
            if cut && state.selected() {
                state.insert(doc, "");
            }
        }
        Event::Ime(ImeEvent::Preedit { text, .. }) => state.preedit = text,
        Event::Ime(ImeEvent::Commit(text)) => commit_ime(doc, state, &text),
        Event::Ime(ImeEvent::DeleteSurrounding {
            before_chars,
            after_chars,
        }) => {
            let mut a = state.cursor;
            let mut b = state.cursor;
            for _ in 0..before_chars {
                a = previous(doc, a, false);
            }
            for _ in 0..after_chars {
                b = next(doc, b, false);
            }
            state.anchor = a;
            state.cursor = b;
            state.insert(doc, "");
        }
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let ctrl = modifiers.command || modifiers.ctrl;
            if ctrl {
                match key {
                    Key::A => {
                        state.anchor = Position::default();
                        let paragraph = doc.paragraphs.len().saturating_sub(1);
                        state.cursor = Position {
                            paragraph,
                            offset: doc
                                .paragraphs
                                .get(paragraph)
                                .map(Paragraph::len)
                                .unwrap_or_default(),
                        };
                        return;
                    }
                    Key::B => {
                        let on = state.typing.bold;
                        state.apply_format(doc, |f| f.bold = !on);
                        return;
                    }
                    Key::I => {
                        let on = state.typing.italic;
                        state.apply_format(doc, |f| f.italic = !on);
                        return;
                    }
                    Key::U => {
                        let on = state.typing.underline;
                        state.apply_format(doc, |f| f.underline = !on);
                        return;
                    }
                    Key::F | Key::H => {
                        state.show_find = true;
                        return;
                    }
                    Key::Z | Key::Y | Key::S | Key::W | Key::K | Key::O | Key::N => return,
                    _ => {}
                }
            }
            match key {
                Key::ArrowLeft => {
                    let pos = if state.selected() && !modifiers.shift && !ctrl {
                        state.range().0
                    } else {
                        previous(doc, state.cursor, ctrl)
                    };
                    state.move_to(doc, pos, modifiers.shift);
                }
                Key::ArrowRight => {
                    let pos = if state.selected() && !modifiers.shift && !ctrl {
                        state.range().1
                    } else {
                        next(doc, state.cursor, ctrl)
                    };
                    state.move_to(doc, pos, modifiers.shift);
                }
                Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown => {
                    let delta = match key {
                        Key::ArrowUp => -1,
                        Key::ArrowDown => 1,
                        Key::PageUp => -25,
                        _ => 25,
                    };
                    let pos = vertical(layout, state.cursor, delta);
                    state.move_to(doc, pos, modifiers.shift);
                }
                Key::Home | Key::End => {
                    let pos = if ctrl {
                        if key == Key::Home {
                            Position::default()
                        } else {
                            let paragraph = doc.paragraphs.len().saturating_sub(1);
                            Position {
                                paragraph,
                                offset: doc
                                    .paragraphs
                                    .get(paragraph)
                                    .map(Paragraph::len)
                                    .unwrap_or_default(),
                            }
                        }
                    } else {
                        let row = layout.rows.iter().find(|r| {
                            r.paragraph == state.cursor.paragraph
                                && state.cursor.offset >= r.start
                                && state.cursor.offset <= r.end
                        });
                        Position {
                            paragraph: state.cursor.paragraph,
                            offset: row
                                .map(|r| if key == Key::Home { r.start } else { r.end })
                                .unwrap_or_default(),
                        }
                    };
                    state.move_to(doc, pos, modifiers.shift);
                }
                Key::Backspace | Key::Delete => {
                    if !state.selected() {
                        if key == Key::Backspace {
                            state.anchor = previous(doc, state.cursor, ctrl);
                        } else {
                            state.cursor = next(doc, state.cursor, ctrl);
                        }
                    }
                    if state.selected() {
                        state.insert(doc, "");
                    }
                }
                Key::Enter => {
                    if state.insert(doc, if modifiers.shift { "\u{000b}" } else { "\n" })
                        && ctrl
                        && let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph)
                    {
                        p.page_break_before = true;
                    }
                }
                Key::Tab => {
                    if let Some(p) = doc
                        .paragraphs
                        .get_mut(state.cursor.paragraph)
                        .filter(|p| p.list != List::None)
                    {
                        p.list_level = if modifiers.shift {
                            p.list_level.saturating_sub(1)
                        } else {
                            p.list_level.saturating_add(1).min(8)
                        };
                    } else if !modifiers.shift {
                        state.insert(doc, "\t");
                    }
                }
                Key::Escape => {
                    state.preedit.clear();
                    state.show_find = false;
                    state.anchor = state.cursor;
                }
                _ => {}
            }
        }
        _ => {}
    }
}
fn commit_ime(doc: &mut Doc, state: &mut EditorState, text: &str) {
    state.preedit.clear();
    if !text.is_empty() && text != "\n" && text != "\r" {
        state.insert(doc, text);
    }
}
fn selected_runs(p: &Paragraph, start: usize, end: usize) -> Vec<Run> {
    let mut offset = 0;
    let mut runs = Vec::new();
    for run in &p.runs {
        let len = run.text.chars().count();
        let a = start.saturating_sub(offset).min(len);
        let b = end.saturating_sub(offset).min(len);
        if b > a {
            runs.push(Run {
                text: run.text.chars().skip(a).take(b - a).collect(),
                format: run.format.clone(),
            });
        }
        offset += len;
    }
    runs
}
fn previous(doc: &Doc, pos: Position, word: bool) -> Position {
    if pos.offset == 0 {
        let paragraph = pos.paragraph.saturating_sub(1);
        return Position {
            paragraph,
            offset: if pos.paragraph == 0 {
                0
            } else {
                doc.paragraphs
                    .get(paragraph)
                    .map(Paragraph::len)
                    .unwrap_or_default()
            },
        };
    }
    let mut offset = pos.offset - 1;
    if word && let Some(p) = doc.paragraphs.get(pos.paragraph) {
        let chars: Vec<char> = p.text().chars().collect();
        while offset > 0 && chars.get(offset).is_some_and(|c| c.is_whitespace()) {
            offset -= 1;
        }
        while offset > 0
            && chars
                .get(offset - 1)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            offset -= 1;
        }
    }
    Position {
        paragraph: pos.paragraph,
        offset,
    }
}
fn next(doc: &Doc, pos: Position, word: bool) -> Position {
    let len = doc
        .paragraphs
        .get(pos.paragraph)
        .map(Paragraph::len)
        .unwrap_or_default();
    if pos.offset >= len {
        if pos.paragraph + 1 >= doc.paragraphs.len() {
            return pos;
        }
        return Position {
            paragraph: pos.paragraph + 1,
            offset: 0,
        };
    }
    let mut offset = pos.offset + 1;
    if word && let Some(p) = doc.paragraphs.get(pos.paragraph) {
        let chars: Vec<char> = p.text().chars().collect();
        while offset < len
            && chars
                .get(offset)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            offset += 1;
        }
        while offset < len && chars.get(offset).is_some_and(|c| c.is_whitespace()) {
            offset += 1;
        }
    }
    Position {
        paragraph: pos.paragraph,
        offset,
    }
}
fn word_range(doc: &Doc, pos: Position) -> (Position, Position) {
    let chars: Vec<char> = doc
        .paragraphs
        .get(pos.paragraph)
        .map(Paragraph::text)
        .unwrap_or_default()
        .chars()
        .collect();
    let mut a = pos.offset.min(chars.len());
    let mut b = a;
    while a > 0
        && chars
            .get(a - 1)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_')
    {
        a -= 1;
    }
    while b < chars.len()
        && chars
            .get(b)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_')
    {
        b += 1;
    }
    (
        Position {
            paragraph: pos.paragraph,
            offset: a,
        },
        Position {
            paragraph: pos.paragraph,
            offset: b,
        },
    )
}
fn row_x(row: &Row, offset: usize) -> f32 {
    let mut remaining = offset.saturating_sub(row.start);
    let mut x = row.x;
    for segment in &row.segments {
        for ch in segment.text.chars() {
            if remaining == 0 {
                return x;
            }
            x += char_width(ch, &segment.format);
            remaining -= 1;
        }
    }
    x
}
fn row_offset(row: &Row, x: f32) -> usize {
    let mut offset = row.start;
    let mut cursor = row.x;
    for segment in &row.segments {
        for ch in segment.text.chars() {
            let width = char_width(ch, &segment.format);
            if x < cursor + width / 2.0 {
                return offset;
            }
            cursor += width;
            offset += 1;
        }
    }
    offset.min(row.end)
}
fn hit_test(
    layout: &Layout,
    pointer: Pos2,
    origin: Pos2,
    scale: f32,
    page_height: f32,
    gap: f32,
) -> Position {
    let relative = pointer - origin;
    let page = ((relative.y - 12.0) / (page_height + gap)).floor().max(0.0) as usize;
    let y = (relative.y - 12.0 - page as f32 * (page_height + gap)) / scale;
    let x = (relative.x - 24.0) / scale;
    let row = layout
        .rows
        .iter()
        .filter(|r| r.page == page)
        .min_by(|a, b| {
            let da = (a.y + a.height / 2.0 - y).abs();
            let db = (b.y + b.height / 2.0 - y).abs();
            da.total_cmp(&db)
        })
        .or_else(|| layout.rows.last());
    row.map(|r| Position {
        paragraph: r.paragraph,
        offset: row_offset(r, x),
    })
    .unwrap_or_default()
}
fn vertical(layout: &Layout, pos: Position, delta: isize) -> Position {
    let index = layout
        .rows
        .iter()
        .position(|r| r.paragraph == pos.paragraph && pos.offset >= r.start && pos.offset <= r.end)
        .unwrap_or_default();
    let x = layout
        .rows
        .get(index)
        .map(|r| row_x(r, pos.offset))
        .unwrap_or_default();
    let target =
        (index as isize + delta).clamp(0, layout.rows.len().saturating_sub(1) as isize) as usize;
    layout
        .rows
        .get(target)
        .map(|r| Position {
            paragraph: r.paragraph,
            offset: row_offset(r, x),
        })
        .unwrap_or(pos)
}
fn markdown_input(doc: &mut Doc, state: &mut EditorState) -> bool {
    let Some(p) = doc.paragraphs.get(state.cursor.paragraph) else {
        return false;
    };
    if state.cursor.offset != p.len() {
        return false;
    }
    let text = p.text();
    let (style, list) = match text.as_str() {
        "#" => ("Heading1", List::None),
        "##" => ("Heading2", List::None),
        "###" => ("Heading3", List::None),
        "-" | "*" => ("Normal", List::Bullet),
        "1." => ("Normal", List::Number),
        ">" => ("Quote", List::None),
        _ => return false,
    };
    state.anchor = Position {
        paragraph: state.cursor.paragraph,
        offset: 0,
    };
    state.insert(doc, "");
    if let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph) {
        p.set_style(style);
        p.list = list;
        state.typing = p.format_at(0);
    }
    true
}
fn rebase_position(old: &Doc, new: &Doc, pos: Position) -> Position {
    let pos = old.clamp_position(pos);
    if old.paragraphs == new.paragraphs {
        return new.clamp_position(pos);
    }
    let flatten = |doc: &Doc| {
        doc.paragraphs
            .iter()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .collect::<Vec<_>>()
    };
    let a = flatten(old);
    let b = flatten(new);
    let old_offset = old
        .paragraphs
        .iter()
        .take(pos.paragraph)
        .map(|p| p.len() + 1)
        .sum::<usize>()
        + pos.offset;
    let prefix = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(a.len().min(b.len()).saturating_sub(prefix))
        .take_while(|(a, b)| a == b)
        .count();
    let mut offset = if old_offset <= prefix {
        old_offset
    } else if old_offset >= a.len().saturating_sub(suffix) {
        b.len().saturating_sub(a.len().saturating_sub(old_offset))
    } else {
        prefix
    };
    for (paragraph, p) in new.paragraphs.iter().enumerate() {
        let len = p.len();
        if offset <= len {
            return Position { paragraph, offset };
        }
        offset = offset.saturating_sub(len + 1);
    }
    new.clamp_position(Position {
        paragraph: new.paragraphs.len().saturating_sub(1),
        offset: usize::MAX,
    })
}
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_ime_commit_preserves_user_selection() {
        let mut doc = Doc::from_text("selected text");
        let mut state = EditorState {
            anchor: Position::default(),
            cursor: Position {
                paragraph: 0,
                offset: 8,
            },
            preedit: "candidate".into(),
            ..EditorState::default()
        };
        commit_ime(&mut doc, &mut state, "");
        assert_eq!(doc.plain_text(), "selected text");
        assert!(state.preedit.is_empty());
        commit_ime(&mut doc, &mut state, "\n");
        assert_eq!(doc.plain_text(), "selected text");
        commit_ime(&mut doc, &mut state, "確定");
        assert_eq!(doc.plain_text(), "確定 text");
    }
    #[test]
    fn cell_text_edit_keeps_surrounding_formats_and_ids() {
        let mut p = Paragraph {
            id: 7,
            ..Paragraph::default()
        };
        p.runs = vec![
            Run {
                text: "bold".into(),
                format: TextFormat {
                    bold: true,
                    ..TextFormat::default()
                },
            },
            Run {
                text: " italic".into(),
                format: TextFormat {
                    italic: true,
                    ..TextFormat::default()
                },
            },
        ];
        let mut cell = Cell {
            id: 6,
            paragraphs: vec![p],
            fill: [255; 3],
        };
        let mut next = 8;
        edit_cell_text(&mut cell, "boldX italic", &mut next);
        assert_eq!(cell.paragraphs.first().map(|p| p.id), Some(7));
        assert!(cell.paragraphs.first().is_some_and(|p| {
            p.runs
                .iter()
                .any(|r| r.text.contains("italic") && r.format.italic)
        }));
        assert!(
            cell.paragraphs
                .first()
                .is_some_and(|p| p.runs.iter().any(|r| r.text == "boldX" && r.format.bold))
        );
        edit_cell_text(&mut cell, "boldX\n italic", &mut next);
        assert_eq!(cell.paragraphs.len(), 2);
        assert_eq!(cell.paragraphs.first().map(|p| p.id), Some(7));
        assert!(
            cell.paragraphs
                .get(1)
                .is_some_and(|p| p.id == 8 && p.runs.iter().any(|r| r.format.italic))
        );
        assert_eq!(next, 9);
    }
    #[test]
    fn rebases_caret_after_agent_insert() {
        let old = Doc::from_text("abcdef");
        let mut new = old.clone();
        let id = new.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(new.insert_text(id, 2, "XYZ").is_ok());
        assert_eq!(
            rebase_position(
                &old,
                &new,
                Position {
                    paragraph: 0,
                    offset: 5
                }
            )
            .offset,
            8
        );
    }
    #[test]
    fn rebases_caret_when_agent_splits_and_merges_paragraphs() {
        let old = Doc::from_text("abcdef\nsecond");
        let mut split = old.clone();
        let id = split.paragraphs.first().map(|p| p.id).unwrap_or_default();
        assert!(split.insert_text(id, 2, "XY\nZ").is_ok());
        assert_eq!(
            rebase_position(
                &old,
                &split,
                Position {
                    paragraph: 0,
                    offset: 5
                }
            ),
            Position {
                paragraph: 1,
                offset: 4
            }
        );
        assert_eq!(
            rebase_position(
                &old,
                &split,
                Position {
                    paragraph: 1,
                    offset: 3
                }
            ),
            Position {
                paragraph: 2,
                offset: 3
            }
        );
        let old = Doc::from_text("first\nsecond");
        let mut merged = old.clone();
        assert!(
            merged
                .replace_range(
                    Position {
                        paragraph: 0,
                        offset: 5
                    },
                    Position {
                        paragraph: 1,
                        offset: 0
                    },
                    "",
                    None
                )
                .is_ok()
        );
        assert_eq!(
            rebase_position(
                &old,
                &merged,
                Position {
                    paragraph: 1,
                    offset: 3
                }
            ),
            Position {
                paragraph: 0,
                offset: 8
            }
        );
    }
    #[test]
    fn word_navigation_unicode() {
        let doc = Doc::from_text("Привет мир");
        assert_eq!(next(&doc, Position::default(), true).offset, 7);
        assert_eq!(
            previous(
                &doc,
                Position {
                    paragraph: 0,
                    offset: 10
                },
                true
            )
            .offset,
            7
        );
    }
    #[test]
    fn markdown_typing_changes_style() {
        let mut doc = Doc::from_text("##");
        let mut state = EditorState {
            cursor: Position {
                paragraph: 0,
                offset: 2,
            },
            anchor: Position {
                paragraph: 0,
                offset: 2,
            },
            ..EditorState::default()
        };
        assert!(markdown_input(&mut doc, &mut state));
        assert_eq!(doc.plain_text(), "");
        assert_eq!(
            doc.paragraphs.first().map(|p| p.style.as_str()),
            Some("Heading2")
        );
    }
}
