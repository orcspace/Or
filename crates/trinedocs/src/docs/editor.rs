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
    find_focus: Option<bool>,
    ribbon_tab: usize,
    show_tables: bool,
    show_objects: bool,
    search: String,
    replacement: String,
    case_sensitive: bool,
    match_index: usize,
    preedit: String,
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
            find_focus: None,
            ribbon_tab: 0,
            show_tables: false,
            show_objects: false,
            search: String::new(),
            replacement: String::new(),
            case_sensitive: false,
            match_index: 0,
            preedit: String::new(),
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
    show_with_edits(ui, doc, None)
}
pub fn show_with_edits(ui: &mut egui::Ui, doc: &mut Doc, edits: Option<&[TextEditDelta]>) -> bool {
    doc.normalize();
    let state_id = ui.id().with("docs-state");
    let canvas_id = ui.id().with("docs-canvas");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<EditorState>(state_id))
        .unwrap_or_default();
    let before = doc.clone();
    poll_image(doc, &mut state);
    if let Some(edits) = edits {
        for edit in edits {
            state.cursor = edit.transform(state.cursor);
            state.anchor = edit.transform(state.anchor);
        }
        if !edits.is_empty() || state.previous.as_deref() != Some(doc) {
            state.cached_layout = None;
        }
    } else if let Some(previous) = &state.previous
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
    let canvas_was_focused = ui.is_enabled() && ui.memory(|m| m.has_focus(canvas_id));
    let find_was_focused = ui.is_enabled()
        && ui.memory(|m| {
            m.has_focus(find_field_id(canvas_id, false))
                || m.has_focus(find_field_id(canvas_id, true))
        });
    if (canvas_was_focused || find_was_focused)
        && let Some((index, replacement)) = ui.input(|i| {
            i.events
                .iter()
                .enumerate()
                .find_map(|(index, event)| find_shortcut(event).map(|target| (index, target)))
        })
    {
        if canvas_was_focused {
            // Preserve input that preceded the shortcut; later text belongs to the search field.
            let earlier = ui.input(|i| i.events.iter().take(index).cloned().collect::<Vec<_>>());
            let event_layout = doc.layout();
            for event in earlier {
                handle_event(ui, doc, &event_layout, &mut state, canvas_id, event);
            }
            ui.input_mut(|i| i.events.drain(..=index).for_each(drop));
        }
        open_find(ui, &mut state, canvas_id, replacement);
        ui.input_mut(|i| i.events.retain(|event| find_shortcut(event).is_none()));
    }
    toolbar(ui, doc, &mut state, canvas_id);
    if state.show_find {
        find_bar(ui, doc, &mut state, canvas_id);
    }
    if state.pending_image.is_some() {
        ui.label("Загрузка изображения…");
    }
    if let Some(message) = &state.message {
        ui.colored_label(Color32::from_rgb(160, 40, 40), message);
    }
    object_panels(ui, doc, &mut state);
    ui.add_space(4.0);
    if canvas_was_focused && ui.memory(|m| m.has_focus(canvas_id)) {
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
            if !ui.memory(|m| m.has_focus(canvas_id)) {
                break;
            }
            handle_event(ui, doc, &event_layout, &mut state, canvas_id, event);
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
        .max_height((ui.available_height() - 32.0).max(80.0))
        .auto_shrink([false, false])
        .id_salt("docs-scroll")
        .show_viewport(ui, |ui, viewport| {
            let size = Vec2::new(total.x.max(viewport.width()), total.y);
            let (canvas, _) = ui.allocate_exact_size(size, Sense::hover());
            let page_left = ((canvas.width() - page_width) / 2.0).max(24.0);
            let page_origin = canvas.min + egui::vec2(page_left, 12.0);
            ui.painter()
                .rect_filled(canvas, 0.0, crate::chrome::workspace(ui));
            let response = ui.interact(canvas, canvas_id, Sense::click_and_drag());
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("docs-test-canvas"),
                    (canvas.intersect(ui.clip_rect()), canvas_id),
                )
            });
            if response.clicked() || response.drag_started() {
                response.request_focus();
                if let Some(pointer) = response.interact_pointer_pos() {
                    let pos = hit_test(&layout, pointer, page_origin, scale, page_height, gap);
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
                state.cursor = hit_test(&layout, pointer, page_origin, scale, page_height, gap);
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
                        page_origin + egui::vec2(0.0, row.page as f32 * (page_height + gap));
                    Rect::from_min_size(
                        origin + egui::vec2(row_x(row, state.cursor.offset) * scale, row.y * scale),
                        egui::vec2(1.5, row.height * scale),
                    )
                });
            let visible = Rect::from_min_size(canvas.min + viewport.min.to_vec2(), viewport.size())
                .expand(80.0);
            for (page_index, page) in layout.pages.iter().enumerate() {
                let origin = page_origin + egui::vec2(0.0, page_index as f32 * (page_height + gap));
                let rect = Rect::from_min_size(origin, egui::vec2(page_width, page_height));
                if !rect.intersects(visible) {
                    continue;
                }
                ui.painter().rect_filled(
                    rect.translate(egui::vec2(0.0, 3.0)),
                    3.0,
                    Color32::from_black_alpha(14),
                );
                ui.painter().rect_filled(rect, 2.0, Color32::WHITE);
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
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
    crate::chrome::tabs(
        ui,
        &mut state.ribbon_tab,
        &["Главная", "Вставка", "Макет", "Вид"],
    );
    crate::chrome::ribbon(ui, |ui| match state.ribbon_tab {
        1 => {
            crate::chrome::group(ui, "Таблицы", 100.0, |ui| {
                ui.menu_button("Таблица", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Строки");
                        ui.add(egui::DragValue::new(&mut state.table_rows).range(1..=100));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Столбцы");
                        ui.add(egui::DragValue::new(&mut state.table_columns).range(1..=32));
                    });
                    if ui.button("Вставить таблицу").clicked() {
                        insert_table(doc, state);
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !doc.tables.is_empty(),
                            egui::Button::new("Редактор таблиц…"),
                        )
                        .clicked()
                    {
                        state.show_tables = true;
                        ui.close();
                    }
                });
            });
            crate::chrome::group(ui, "Изображения", 140.0, |ui| {
                ui.menu_button("Изображение", |ui| {
                    if ui
                        .add_enabled(
                            state.pending_image.is_none(),
                            egui::Button::new("Вставить из файла…"),
                        )
                        .clicked()
                    {
                        start_image_load(ui, doc, state);
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !doc.images.is_empty(),
                            egui::Button::new("Размер и описание…"),
                        )
                        .clicked()
                    {
                        state.show_objects = true;
                        ui.close();
                    }
                });
            });
            crate::chrome::group(ui, "Текст", 255.0, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Ссылка…", |ui| {
                        ui.label("Адрес ссылки");
                        ui.text_edit_singleline(&mut state.link);
                        if ui
                            .add_enabled(
                                !state.link.trim().is_empty(),
                                egui::Button::new("Применить"),
                            )
                            .clicked()
                        {
                            let link = state.link.trim().to_string();
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
                            ui.memory_mut(|m| m.request_focus(canvas_id));
                            ui.close();
                        }
                    });
                    ui.menu_button("Символ", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for ch in [
                                "©", "®", "™", "€", "₽", "°", "±", "×", "÷", "—", "…", "§", "✓",
                            ] {
                                if ui.button(ch).clicked() {
                                    state.insert(doc, ch);
                                    ui.memory_mut(|m| m.request_focus(canvas_id));
                                    ui.close();
                                }
                            }
                        });
                    });
                    if ui
                        .button("Дата")
                        .on_hover_text("Вставить дату YYYY-MM-DD")
                        .clicked()
                    {
                        state.insert(doc, &today());
                        ui.memory_mut(|m| m.request_focus(canvas_id));
                    }
                });
            });
            crate::chrome::group(ui, "Страницы", 270.0, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Разрыв страницы").clicked() {
                        if state.insert(doc, "\n")
                            && let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph)
                        {
                            p.page_break_before = true;
                        }
                        ui.memory_mut(|m| m.request_focus(canvas_id));
                    }
                    ui.menu_button("Колонтитулы…", |ui| page_headers(ui, doc));
                });
            });
        }
        2 => {
            crate::chrome::group(ui, "Страница", 280.0, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Размер", |ui| {
                        for (name, width, height) in [
                            ("A4", 595.28, 841.89),
                            ("Letter", 612.0, 792.0),
                            ("A3", 841.89, 1190.55),
                            ("Legal", 612.0, 1008.0),
                        ] {
                            if ui.button(name).clicked() {
                                let landscape = doc.page.width > doc.page.height;
                                (doc.page.width, doc.page.height) = if landscape {
                                    (height, width)
                                } else {
                                    (width, height)
                                };
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text(format!("{:.0} × {:.0} pt", doc.page.width, doc.page.height));
                    ui.menu_button("Ориентация", |ui| {
                        for (name, landscape) in [("Книжная", false), ("Альбомная", true)]
                        {
                            if ui.button(name).clicked() {
                                if (doc.page.width > doc.page.height) != landscape {
                                    std::mem::swap(&mut doc.page.width, &mut doc.page.height);
                                }
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Поля", |ui| {
                        for (label, value) in [
                            ("Слева", &mut doc.page.margin_left),
                            ("Справа", &mut doc.page.margin_right),
                            ("Сверху", &mut doc.page.margin_top),
                            ("Снизу", &mut doc.page.margin_bottom),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(label);
                                ui.add(
                                    egui::DragValue::new(value).range(0.0..=200.0).suffix(" pt"),
                                );
                            });
                        }
                    });
                });
            });
            crate::chrome::group(ui, "Абзац", 125.0, |ui| paragraph_menu(ui, doc, state));
            crate::chrome::group(ui, "Колонтитулы", 285.0, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Текст…", |ui| page_headers(ui, doc));
                    ui.checkbox(&mut doc.page.page_numbers, "Номера страниц");
                });
            });
        }
        3 => {
            crate::chrome::group(ui, "Масштаб", 355.0, |ui| {
                ui.horizontal(|ui| {
                    for (label, zoom) in [("75%", 0.75), ("100%", 1.0), ("125%", 1.25)] {
                        if ui
                            .selectable_label((state.zoom - zoom).abs() < 0.01, label)
                            .clicked()
                        {
                            state.zoom = zoom;
                        }
                    }
                    if ui
                        .button("По ширине")
                        .on_hover_text("По ширине страницы")
                        .clicked()
                    {
                        state.zoom = ((ui.ctx().content_rect().width() - 100.0) / doc.page.width)
                            .clamp(0.5, 2.5);
                    }
                });
            });
            crate::chrome::group(ui, "Навигация", 245.0, |ui| {
                ui.horizontal(|ui| {
                    outline_menu(ui, doc, state);
                    if ui
                        .button("Поиск")
                        .on_hover_text("Найти / заменить · Ctrl+F / Ctrl+H")
                        .clicked()
                    {
                        toggle_find(ui, state, canvas_id);
                    }
                });
            });
            crate::chrome::group(ui, "Содержимое", 135.0, |ui| {
                ui.menu_button("Редакторы", |ui| {
                    if ui
                        .add_enabled(!doc.tables.is_empty(), egui::Button::new("Таблицы…"))
                        .clicked()
                    {
                        state.show_tables = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(!doc.images.is_empty(), egui::Button::new("Изображения…"))
                        .clicked()
                    {
                        state.show_objects = true;
                        ui.close();
                    }
                });
            });
        }
        _ => {
            crate::chrome::group(ui, "Стили", 145.0, |ui| {
                let current = doc
                    .paragraphs
                    .get(state.cursor.paragraph)
                    .map(|p| p.style.as_str())
                    .unwrap_or("Normal")
                    .to_string();
                egui::ComboBox::from_id_salt("docs-style")
                    .width(135.0)
                    .selected_text(style_label(&current))
                    .show_ui(ui, |ui| {
                        for style in [
                            "Normal", "Title", "Heading1", "Heading2", "Heading3", "Quote",
                        ] {
                            if ui
                                .selectable_label(current == style, style_label(style))
                                .clicked()
                            {
                                apply_style(doc, state, style);
                                ui.memory_mut(|m| m.request_focus(canvas_id));
                            }
                        }
                    });
            });
            crate::chrome::group(ui, "Шрифт", 215.0, |ui| {
                ui.horizontal(|ui| {
                    let mut font = state.typing.font.clone();
                    egui::ComboBox::from_id_salt("docs-font")
                        .width(130.0)
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
                        ui.memory_mut(|m| m.request_focus(canvas_id));
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
                });
            });
            crate::chrome::group(ui, "Начертание", 170.0, |ui| {
                ui.horizontal(|ui| {
                    for (label, tip, on, which) in [
                        ("Ж", "Полужирный · Ctrl+B", state.typing.bold, 0),
                        ("К", "Курсив · Ctrl+I", state.typing.italic, 1),
                        ("Ч", "Подчёркнутый · Ctrl+U", state.typing.underline, 2),
                    ] {
                        if ui.selectable_label(on, label).on_hover_text(tip).clicked() {
                            state.apply_format(doc, |f| match which {
                                0 => f.bold = !on,
                                1 => f.italic = !on,
                                _ => f.underline = !on,
                            });
                            ui.memory_mut(|m| m.request_focus(canvas_id));
                        }
                    }
                    ui.menu_button("Формат", |ui| {
                        text_format_menu(ui, doc, state, canvas_id)
                    });
                });
            });
            crate::chrome::group(ui, "Абзац", 235.0, |ui| {
                ui.horizontal(|ui| {
                    let current = doc
                        .paragraphs
                        .get(state.cursor.paragraph)
                        .map(|p| p.align)
                        .unwrap_or_default();
                    for (index, (tip, align)) in [
                        ("По левому краю", Align::Left),
                        ("По центру", Align::Center),
                        ("По правому краю", Align::Right),
                        ("По ширине", Align::Justify),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if crate::chrome::alignment_button(ui, index, tip, current == align)
                            .clicked()
                        {
                            selected_paragraphs(doc, state, |p| p.align = align);
                            ui.memory_mut(|m| m.request_focus(canvas_id));
                        }
                    }
                    ui.menu_button("Ещё", |ui| {
                        for (label, list) in [
                            ("Маркированный список", List::Bullet),
                            ("Нумерованный список", List::Number),
                        ] {
                            if ui.button(label).clicked() {
                                selected_paragraphs(doc, state, |p| {
                                    p.list = if p.list == list { List::None } else { list }
                                });
                                ui.memory_mut(|m| m.request_focus(canvas_id));
                                ui.close();
                            }
                        }
                        ui.separator();
                        for (label, delta) in
                            [("Уменьшить отступ", -18.0), ("Увеличить отступ", 18.0)]
                        {
                            if ui.button(label).clicked() {
                                selected_paragraphs(doc, state, |p| {
                                    p.indent = (p.indent + delta).clamp(0.0, 250.0)
                                });
                                ui.memory_mut(|m| m.request_focus(canvas_id));
                                ui.close();
                            }
                        }
                        paragraph_menu(ui, doc, state);
                    });
                });
            });
            crate::chrome::group(ui, "Редактирование", 100.0, |ui| {
                ui.menu_button("Правка", |ui| {
                    for (label, command, enabled) in [
                        (
                            "Вставить · Ctrl+V",
                            egui::ViewportCommand::RequestPaste,
                            true,
                        ),
                        (
                            "Вырезать · Ctrl+X",
                            egui::ViewportCommand::RequestCut,
                            state.selected(),
                        ),
                        (
                            "Копировать · Ctrl+C",
                            egui::ViewportCommand::RequestCopy,
                            state.selected(),
                        ),
                    ] {
                        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                            ui.memory_mut(|m| m.request_focus(canvas_id));
                            ui.ctx().send_viewport_cmd(command);
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("Найти / заменить · Ctrl+F / Ctrl+H").clicked() {
                        toggle_find(ui, state, canvas_id);
                        ui.close();
                    }
                    outline_menu(ui, doc, state);
                    if ui
                        .add_enabled(
                            !doc.tables.is_empty(),
                            egui::Button::new("Редактор таблиц…"),
                        )
                        .clicked()
                    {
                        state.show_tables = true;
                        ui.close();
                    }
                });
            });
        }
    });
}
fn text_format_menu(
    ui: &mut egui::Ui,
    doc: &mut Doc,
    state: &mut EditorState,
    canvas_id: egui::Id,
) {
    for (label, on, which) in [
        ("Зачёркнутый", state.typing.strike, 0),
        ("Надстрочный", state.typing.script == 1, 1),
        ("Подстрочный", state.typing.script == -1, 2),
    ] {
        if ui.selectable_label(on, label).clicked() {
            state.apply_format(doc, |f| match which {
                0 => f.strike = !on,
                1 => f.script = if on { 0 } else { 1 },
                _ => f.script = if on { 0 } else { -1 },
            });
            ui.memory_mut(|m| m.request_focus(canvas_id));
            ui.close();
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Цвет текста");
        let mut color = state.typing.color;
        if ui.color_edit_button_srgb(&mut color).changed() {
            state.apply_format(doc, |f| f.color = color);
        }
    });
    ui.menu_button("Маркер", |ui| {
        for (label, color) in [
            ("Жёлтый", Some([255, 245, 80])),
            ("Зелёный", Some([150, 240, 160])),
            ("Голубой", Some([145, 220, 250])),
            ("Без цвета", None),
        ] {
            if ui.button(label).clicked() {
                state.apply_format(doc, |f| f.highlight = color);
                ui.memory_mut(|m| m.request_focus(canvas_id));
                ui.close();
            }
        }
    });
    ui.separator();
    if ui.button("Очистить форматирование").clicked() {
        state.apply_format(doc, |f| *f = TextFormat::default());
        ui.memory_mut(|m| m.request_focus(canvas_id));
        ui.close();
    }
    if ui.button("Копировать образец").clicked() {
        state.format_painter = Some(state.typing.clone());
        ui.memory_mut(|m| m.request_focus(canvas_id));
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
        ui.memory_mut(|m| m.request_focus(canvas_id));
        ui.close();
    }
}
fn selected_paragraphs(doc: &mut Doc, state: &EditorState, apply: impl Fn(&mut Paragraph)) {
    let (a, b) = state.range();
    let end = if b.paragraph > a.paragraph && b.offset == 0 {
        b.paragraph - 1
    } else {
        b.paragraph
    };
    for p in doc
        .paragraphs
        .iter_mut()
        .skip(a.paragraph)
        .take(end.saturating_sub(a.paragraph) + 1)
    {
        apply(p);
    }
}
fn apply_style(doc: &mut Doc, state: &mut EditorState, style: &str) {
    selected_paragraphs(doc, state, |p| p.set_style(style));
    state.typing = doc
        .paragraphs
        .get(state.cursor.paragraph)
        .map(|p| p.format_at(state.cursor.offset))
        .unwrap_or_default();
}
fn insert_table(doc: &mut Doc, state: &mut EditorState) {
    if let Some(id) = doc.paragraphs.get(state.cursor.paragraph).map(|p| p.id) {
        match doc.add_table(id, state.table_rows, state.table_columns) {
            Ok(_) => state.show_tables = true,
            Err(error) => state.message = Some(error),
        }
    }
}
fn paragraph_menu(ui: &mut egui::Ui, doc: &mut Doc, state: &EditorState) {
    ui.menu_button("Интервалы…", |ui| {
        if let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph) {
            ui.label("Отступ, pt");
            ui.add(egui::DragValue::new(&mut p.indent).range(0.0..=250.0));
            ui.label("Первая строка, pt");
            ui.add(egui::DragValue::new(&mut p.first_indent).range(-100.0..=100.0));
            ui.label("Межстрочный интервал");
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
}
fn page_headers(ui: &mut egui::Ui, doc: &mut Doc) {
    ui.label("Верхний колонтитул");
    ui.text_edit_singleline(&mut doc.page.header);
    ui.label("Нижний колонтитул");
    ui.text_edit_singleline(&mut doc.page.footer);
    ui.checkbox(&mut doc.page.page_numbers, "Номера страниц");
}
fn outline_menu(ui: &mut egui::Ui, doc: &Doc, state: &mut EditorState) {
    ui.menu_button("Заголовки", |ui| {
        let outline = doc.outline();
        if outline.is_empty() {
            ui.weak("Добавьте стиль заголовка, чтобы перейти к нему.");
        }
        for (id, _, text) in outline {
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
fn clipboard_id() -> egui::Id {
    egui::Id::new("trinedocs-docs-rich-clipboard")
}
fn find_field_id(canvas_id: egui::Id, replacement: bool) -> egui::Id {
    canvas_id.with(if replacement {
        "find-replacement"
    } else {
        "find-query"
    })
}
fn find_shortcut(event: &Event) -> Option<bool> {
    if let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
        && (modifiers.command || modifiers.ctrl)
        && matches!(key, Key::F | Key::H)
    {
        Some(*key == Key::H)
    } else {
        None
    }
}
fn open_find(ui: &mut egui::Ui, state: &mut EditorState, canvas_id: egui::Id, replacement: bool) {
    state.show_find = true;
    state.find_focus = Some(replacement);
    state.preedit.clear();
    ui.memory_mut(|m| m.request_focus(find_field_id(canvas_id, replacement)));
}
fn toggle_find(ui: &mut egui::Ui, state: &mut EditorState, canvas_id: egui::Id) {
    if state.show_find {
        state.show_find = false;
        state.find_focus = None;
        ui.memory_mut(|m| m.request_focus(canvas_id));
    } else {
        open_find(ui, state, canvas_id, false);
    }
}
fn find_bar(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    let query_id = find_field_id(canvas_id, false);
    let replacement_id = find_field_id(canvas_id, true);
    if let Some(replacement) = state.find_focus.take() {
        ui.memory_mut(|m| m.request_focus(find_field_id(canvas_id, replacement)));
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Найти:");
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .id(query_id)
                .desired_width(170.0),
        );
        ui.label("Заменить:");
        ui.add(
            egui::TextEdit::singleline(&mut state.replacement)
                .id(replacement_id)
                .desired_width(170.0),
        );
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
            .add_enabled(
                matches.contains(&state.range()),
                egui::Button::new("Заменить"),
            )
            .clicked()
        {
            replace_current_match(doc, state);
        }
        if ui
            .add_enabled(!matches.is_empty(), egui::Button::new("Заменить все"))
            .clicked()
        {
            doc.replace_all(&state.search, &state.replacement, state.case_sensitive);
            state.cursor = doc.clamp_position(state.cursor);
            state.anchor = state.cursor;
        }
        if ui.button("×").on_hover_text("Закрыть поиск").clicked() {
            state.show_find = false;
            state.find_focus = None;
            ui.memory_mut(|m| m.request_focus(canvas_id));
        }
    });
}
fn replace_current_match(doc: &mut Doc, state: &mut EditorState) -> bool {
    if !state.selected()
        || !doc
            .find(&state.search, state.case_sensitive)
            .contains(&state.range())
    {
        return false;
    }
    let replacement = state.replacement.clone();
    state.insert(doc, &replacement)
}
fn object_panels(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState) {
    let enabled = ui.is_enabled();
    if state.show_tables {
        let window_id = ui.id().with("docs-tables-window");
        egui::Window::new("Редактор таблиц")
            .id(window_id)
            .open(&mut state.show_tables)
            .default_size(egui::vec2(800.0, 470.0))
            .vscroll(true)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.weak("Текст, формат и размеры меняются сразу. Ctrl+Z отменяет правку.");
                    table_panels(ui, doc);
                });
            });
    }
    if state.show_objects {
        egui::Window::new("Изображения")
            .id(ui.id().with("docs-images-window"))
            .open(&mut state.show_objects)
            .default_width(540.0)
            .vscroll(true)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    for image in &mut doc.images {
                        ui.push_id(image.id, |ui| {
                            ui.label(format!("Изображение {}", image.alt));
                            ui.horizontal(|ui| {
                                ui.label("Ширина / высота");
                                ui.add(
                                    egui::DragValue::new(&mut image.width)
                                        .range(1.0..=2000.0)
                                        .suffix(" pt"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut image.height)
                                        .range(1.0..=2000.0)
                                        .suffix(" pt"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Описание");
                                ui.text_edit_singleline(&mut image.alt);
                            });
                            ui.separator();
                        });
                    }
                });
            });
    }
}
fn table_panels(ui: &mut egui::Ui, doc: &mut Doc) {
    let mut remove = None;
    let mut next = doc.next_id;
    if doc.tables.is_empty() {
        ui.label("Таблиц пока нет. Вставка → Таблица → Вставить таблицу.");
    }
    let single_table = doc.tables.len() == 1;
    for (ti, table) in doc.tables.iter_mut().enumerate() {
        let columns = table.rows.iter().map(Vec::len).max().unwrap_or(1);
        egui::CollapsingHeader::new(format!(
            "Таблица {} · {} × {}",
            ti + 1,
            table.rows.len(),
            columns
        ))
        .id_salt(table.id)
        .default_open(single_table)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(table.rows.len() < 1000, egui::Button::new("+ Строка"))
                    .clicked()
                {
                    table
                        .rows
                        .push((0..columns).map(|_| new_cell(&mut next)).collect());
                }
                if ui
                    .add_enabled(columns < 64, egui::Button::new("+ Столбец"))
                    .clicked()
                {
                    add_table_column(table, &mut next);
                }
                if ui
                    .add_enabled(
                        table.rows.len() > 1,
                        egui::Button::new("− Последняя строка"),
                    )
                    .clicked()
                {
                    table.rows.pop();
                }
                if ui
                    .add_enabled(columns > 1, egui::Button::new("− Последний столбец"))
                    .clicked()
                {
                    remove_table_column(table);
                }
                if ui.button("Удалить таблицу").clicked() {
                    remove = Some(ti);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Цвет границ");
                ui.color_edit_button_srgb(&mut table.border_color);
                ui.menu_button("Ширины столбцов…", |ui| {
                    let columns = table.rows.iter().map(Vec::len).max().unwrap_or(1);
                    table.column_widths.resize(columns, 80.0);
                    for (column, width) in table.column_widths.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("Столбец {}", column + 1));
                            ui.add(
                                egui::DragValue::new(width)
                                    .range(12.0..=2000.0)
                                    .suffix(" pt"),
                            );
                        });
                    }
                });
            });
            egui::ScrollArea::horizontal()
                .id_salt(("table-cells", table.id))
                .show(ui, |ui| {
                    egui::Grid::new(("docs-table", table.id))
                        .spacing(egui::vec2(10.0, 10.0))
                        .show(ui, |ui| {
                            for (ri, row) in table.rows.iter_mut().enumerate() {
                                for (ci, cell) in row.iter_mut().enumerate() {
                                    ui.push_id(cell.id, |ui| {
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.weak(format!("{}:{}", ri + 1, ci + 1));
                                                ui.menu_button("Формат", |ui| {
                                                    cell_format(ui, cell)
                                                });
                                                ui.color_edit_button_srgb(&mut cell.fill)
                                                    .on_hover_text("Заливка ячейки");
                                            });
                                            let mut text = cell
                                                .paragraphs
                                                .iter()
                                                .map(Paragraph::text)
                                                .collect::<Vec<_>>()
                                                .join("\n");
                                            if ui
                                                .add(
                                                    egui::TextEdit::multiline(&mut text)
                                                        .desired_width(150.0)
                                                        .desired_rows(3),
                                                )
                                                .changed()
                                            {
                                                edit_cell_text(cell, &text, &mut next);
                                            }
                                        });
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
}
fn add_table_column(table: &mut Table, next: &mut u64) {
    let columns = table.rows.iter().map(Vec::len).max().unwrap_or_default();
    for row in &mut table.rows {
        while row.len() <= columns {
            row.push(new_cell(next));
        }
    }
    table.column_widths.resize(columns, 80.0);
    table.column_widths.push(80.0);
}
fn remove_table_column(table: &mut Table) {
    let columns = table.rows.iter().map(Vec::len).max().unwrap_or_default();
    if columns <= 1 {
        return;
    }
    for row in &mut table.rows {
        row.truncate(columns - 1);
    }
    table.column_widths.truncate(columns - 1);
}
fn format_cell(cell: &mut Cell, apply: impl Fn(&mut TextFormat)) {
    for paragraph in &mut cell.paragraphs {
        for run in &mut paragraph.runs {
            apply(&mut run.format);
        }
    }
}
fn cell_format(ui: &mut egui::Ui, cell: &mut Cell) {
    ui.weak("Формат всей ячейки");
    let current = cell
        .paragraphs
        .first()
        .map(|p| p.format_at(0))
        .unwrap_or_default();
    ui.horizontal(|ui| {
        for (label, on, which) in [
            ("Ж", current.bold, 0),
            ("К", current.italic, 1),
            ("Ч", current.underline, 2),
        ] {
            if ui.selectable_label(on, label).clicked() {
                format_cell(cell, |f| match which {
                    0 => f.bold = !on,
                    1 => f.italic = !on,
                    _ => f.underline = !on,
                });
            }
        }
        let mut size = current.size;
        if ui
            .add(
                egui::DragValue::new(&mut size)
                    .range(4.0..=144.0)
                    .suffix(" pt"),
            )
            .changed()
        {
            format_cell(cell, |f| f.size = size);
        }
    });
    let mut color = current.color;
    ui.horizontal(|ui| {
        ui.label("Цвет текста");
        if ui.color_edit_button_srgb(&mut color).changed() {
            format_cell(cell, |f| f.color = color);
        }
    });
    ui.horizontal(|ui| {
        for (label, align) in [
            ("Слева", Align::Left),
            ("Центр", Align::Center),
            ("Справа", Align::Right),
        ] {
            if ui.button(label).clicked() {
                for p in &mut cell.paragraphs {
                    p.align = align;
                }
            }
        }
    });
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
    canvas_id: egui::Id,
    event: Event,
) {
    if !ui.is_enabled() {
        return;
    }
    if let Some(replacement) = find_shortcut(&event) {
        open_find(ui, state, canvas_id, replacement);
        return;
    }
    if !ui.memory(|m| m.has_focus(canvas_id)) {
        return;
    }
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
            let copied = ui
                .ctx()
                .data_mut(|d| d.get_temp::<Clipboard>(clipboard_id()))
                .filter(|c| c.text == text);
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
            ui.ctx()
                .data_mut(|d| d.insert_temp(clipboard_id(), Clipboard { text, paragraphs }));
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
                    state.find_focus = None;
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
    let page = (relative.y / (page_height + gap)).floor().max(0.0) as usize;
    let y = (relative.y - page as f32 * (page_height + gap)) / scale;
    let x = relative.x / scale;
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
    if let Some(paragraph) = old.paragraphs.get(pos.paragraph)
        && let Some((index, unchanged)) = new
            .paragraphs
            .iter()
            .enumerate()
            .find(|(_, p)| p.id == paragraph.id)
        && paragraph.text() == unchanged.text()
    {
        return new.clamp_position(Position {
            paragraph: index,
            offset: pos.offset,
        });
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
    fn disabled_editor_preserves_document_against_canvas_events() -> Result<(), String> {
        for event in [
            Event::Text("new".into()),
            Event::Paste("pasted".into()),
            Event::Key {
                key: Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx)?;
            let mut doc = Doc::from_text("original");
            let before = doc.clone();
            let seed = EditorState {
                anchor: Position {
                    paragraph: 0,
                    offset: 1,
                },
                cursor: Position {
                    paragraph: 0,
                    offset: 4,
                },
                ..Default::default()
            };
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1100.0, 900.0))),
                events: vec![event],
                ..Default::default()
            };
            let mut changed = false;
            let mut output = ctx.run_ui(input, |ui| {
                ui.add_enabled_ui(false, |ui| {
                    let canvas_id = ui.id().with("docs-canvas");
                    let state_id = ui.id().with("docs-state");
                    ui.ctx().data_mut(|d| d.insert_temp(state_id, seed.clone()));
                    ui.memory_mut(|m| m.request_focus(canvas_id));
                    changed = show(ui, &mut doc);
                });
            });
            output.textures_delta.clear();
            assert!(!changed);
            assert_eq!(doc, before);
        }
        Ok(())
    }

    #[test]
    fn replace_does_not_delete_a_previous_unrelated_selection() {
        let mut doc = Doc::from_text("keep target");
        let mut state = EditorState {
            anchor: Position::default(),
            cursor: Position {
                paragraph: 0,
                offset: 4,
            },
            search: "target".into(),
            replacement: "new".into(),
            ..Default::default()
        };
        assert!(!replace_current_match(&mut doc, &mut state));
        assert_eq!(doc.plain_text(), "keep target");
        state.anchor = Position {
            paragraph: 0,
            offset: 5,
        };
        state.cursor = Position {
            paragraph: 0,
            offset: 11,
        };
        assert!(replace_current_match(&mut doc, &mut state));
        assert_eq!(doc.plain_text(), "keep new");
    }

    #[test]
    fn rich_clipboard_crosses_document_tabs_and_does_not_reuse_ids() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut source = Doc::from_text("bold\nitalic");
        source.format_range(
            Position::default(),
            Position {
                paragraph: 0,
                offset: 4,
            },
            |f| f.bold = true,
        );
        source.format_range(
            Position {
                paragraph: 1,
                offset: 0,
            },
            Position {
                paragraph: 1,
                offset: 6,
            },
            |f| f.italic = true,
        );
        let mut from = EditorState {
            anchor: Position::default(),
            cursor: Position {
                paragraph: 1,
                offset: 6,
            },
            ..Default::default()
        };
        let source_layout = source.layout();
        let mut target = Doc::from_text("start");
        let existing_id = target.paragraphs.first().ok_or("paragraph")?.id;
        let old_next = target.next_id;
        let mut to = EditorState {
            cursor: Position {
                paragraph: 0,
                offset: 5,
            },
            anchor: Position {
                paragraph: 0,
                offset: 5,
            },
            ..Default::default()
        };
        let target_layout = target.layout();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let source_canvas = ui.id().with("source");
            ui.memory_mut(|m| m.request_focus(source_canvas));
            handle_event(
                ui,
                &mut source,
                &source_layout,
                &mut from,
                source_canvas,
                Event::Copy,
            );
            let target_canvas = ui.id().with("target");
            ui.memory_mut(|m| m.request_focus(target_canvas));
            handle_event(
                ui,
                &mut target,
                &target_layout,
                &mut to,
                target_canvas,
                Event::Paste("bold\nitalic".into()),
            );
        });
        output.textures_delta.clear();
        assert_eq!(target.plain_text(), "startbold\nitalic");
        assert_eq!(target.paragraphs.first().map(|p| p.id), Some(existing_id));
        assert!(
            target
                .paragraphs
                .first()
                .is_some_and(|p| p.runs.iter().any(|r| r.text == "bold" && r.format.bold))
        );
        assert!(
            target.paragraphs.get(1).is_some_and(|p| p.id >= old_next
                && p.runs.iter().any(|r| r.text == "italic" && r.format.italic))
        );
        target.validate()?;
        // A later clipboard change with other text must not inherit rich formatting.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let canvas = ui.id().with("target");
            ui.memory_mut(|m| m.request_focus(canvas));
            to.move_to(
                &target,
                Position {
                    paragraph: 1,
                    offset: 6,
                },
                false,
            );
            to.typing = TextFormat::default();
            let layout = target.layout();
            handle_event(
                ui,
                &mut target,
                &layout,
                &mut to,
                canvas,
                Event::Paste("plain".into()),
            );
        });
        output.textures_delta.clear();
        assert!(target.paragraphs.get(1).is_some_and(|p| {
            p.runs
                .last()
                .is_some_and(|r| r.text == "plain" && !r.format.bold && !r.format.italic)
        }));
        Ok(())
    }

    #[test]
    fn text_before_find_shortcut_stays_in_document_and_later_text_enters_query()
    -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut doc = Doc::from_text("keep");
        let raw = |events| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1250.0, 820.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw(Vec::new()), |ui| {
            ui.memory_mut(|m| m.request_focus(ui.id().with("docs-canvas")));
            show_with_edits(ui, &mut doc, Some(&[]));
        });
        output.textures_delta.clear();
        let mut observed = None;
        let mut output = ctx.run_ui(
            raw(vec![
                Event::Text("x".into()),
                find_hotkey(Key::F),
                Event::Text("needle".into()),
            ]),
            |ui| {
                show_with_edits(ui, &mut doc, Some(&[]));
                observed = ui
                    .ctx()
                    .data_mut(|d| d.get_temp::<EditorState>(ui.id().with("docs-state")));
            },
        );
        output.textures_delta.clear();
        assert_eq!(doc.plain_text(), "xkeep");
        assert_eq!(observed.ok_or("editor")?.search, "needle");
        Ok(())
    }

    fn find_hotkey(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                ctrl: true,
                command: true,
                ..Default::default()
            },
        }
    }
    #[test]
    fn find_and_replace_hotkeys_focus_fields_without_editing_document() -> Result<(), String> {
        for (key, text, replacement) in [(Key::F, "needle", false), (Key::H, "замена", true)]
        {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx)?;
            let mut doc = Doc::from_text("keep document");
            let original = doc.clone();
            let raw = |events| egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1250.0, 820.0))),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw(Vec::new()), |ui| {
                let state = EditorState {
                    search: if replacement {
                        "keep".into()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                };
                ui.ctx()
                    .data_mut(|d| d.insert_temp(ui.id().with("docs-state"), state));
                ui.memory_mut(|m| m.request_focus(ui.id().with("docs-canvas")));
                show_with_edits(ui, &mut doc, Some(&[]));
            });
            output.textures_delta.clear();
            let mut observed = None;
            let mut output = ctx.run_ui(
                raw(vec![find_hotkey(key), Event::Text(text.into())]),
                |ui| {
                    show_with_edits(ui, &mut doc, Some(&[]));
                    assert!(ui.memory(|m| {
                        m.has_focus(find_field_id(ui.id().with("docs-canvas"), replacement))
                    }));
                    observed = ui
                        .ctx()
                        .data_mut(|d| d.get_temp::<EditorState>(ui.id().with("docs-state")));
                },
            );
            output.textures_delta.clear();
            let state = observed.ok_or("editor state")?;
            assert!(state.show_find);
            assert_eq!(
                if replacement {
                    state.replacement.as_str()
                } else {
                    state.search.as_str()
                },
                text
            );
            if replacement {
                assert_eq!(state.search, "keep");
            }
            assert_eq!(doc, original);
        }
        Ok(())
    }
    #[test]
    fn opening_find_during_event_dispatch_stops_canvas_text_and_paste() {
        let ctx = egui::Context::default();
        let mut doc = Doc::from_text("unchanged");
        let original = doc.clone();
        let layout = doc.layout();
        let mut state = EditorState::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let canvas_id = ui.id().with("docs-canvas");
            ui.memory_mut(|m| m.request_focus(canvas_id));
            handle_event(
                ui,
                &mut doc,
                &layout,
                &mut state,
                canvas_id,
                find_hotkey(Key::F),
            );
            assert!(ui.memory(|m| m.has_focus(find_field_id(canvas_id, false))));
            handle_event(
                ui,
                &mut doc,
                &layout,
                &mut state,
                canvas_id,
                Event::Text("wrong".into()),
            );
            handle_event(
                ui,
                &mut doc,
                &layout,
                &mut state,
                canvas_id,
                Event::Paste("wrong".into()),
            );
        });
        output.textures_delta.clear();
        assert_eq!(doc, original);
        assert_eq!(state.find_focus, Some(false));
    }
    #[test]
    fn canvas_copy_paste_cut_keep_formatted_text() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut doc = Doc::from_text("bold plain");
        doc.format_range(
            Position::default(),
            Position {
                paragraph: 0,
                offset: 4,
            },
            |f| f.bold = true,
        );
        let mut state = EditorState {
            anchor: Position::default(),
            cursor: Position {
                paragraph: 0,
                offset: 4,
            },
            ..Default::default()
        };
        let layout = doc.layout();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let canvas_id = ui.id().with("docs-canvas");
            ui.memory_mut(|m| m.request_focus(canvas_id));
            handle_event(ui, &mut doc, &layout, &mut state, canvas_id, Event::Copy);
            state.move_to(
                &doc,
                Position {
                    paragraph: 0,
                    offset: 10,
                },
                false,
            );
            handle_event(
                ui,
                &mut doc,
                &layout,
                &mut state,
                canvas_id,
                Event::Paste("bold".into()),
            );
            state.anchor = Position::default();
            state.cursor = Position {
                paragraph: 0,
                offset: 4,
            };
            handle_event(ui, &mut doc, &layout, &mut state, canvas_id, Event::Cut);
        });
        output.textures_delta.clear();
        assert_eq!(doc.plain_text(), " plainbold");
        assert!(doc.paragraphs.first().is_some_and(|p| {
            p.runs
                .last()
                .is_some_and(|r| r.text == "bold" && r.format.bold)
        }));
        assert_eq!(
            ctx.data_mut(|d| d.get_temp::<Clipboard>(clipboard_id()))
                .map(|c| c.text),
            Some("bold".into())
        );
        doc.validate()?;
        Ok(())
    }

    #[test]
    fn centered_page_hit_test_uses_the_same_origin_as_paint() -> Result<(), String> {
        let doc = Doc::from_text("Привет мир\nsecond");
        let layout = doc.layout();
        let row = layout.rows.first().ok_or("row")?;
        let origin = Pos2::new(300.0, 120.0);
        let scale = 1.25;
        let pointer =
            origin + egui::vec2(row_x(row, 3) * scale, (row.y + row.height / 2.0) * scale);
        assert_eq!(
            hit_test(
                &layout,
                pointer,
                origin,
                scale,
                doc.page.height * scale,
                24.0
            ),
            Position {
                paragraph: 0,
                offset: 3
            }
        );
        Ok(())
    }
    #[test]
    fn paragraph_format_does_not_include_unselected_next_paragraph() {
        let mut doc = Doc::from_text("first\nsecond");
        let state = EditorState {
            anchor: Position::default(),
            cursor: Position {
                paragraph: 1,
                offset: 0,
            },
            ..Default::default()
        };
        selected_paragraphs(&mut doc, &state, |p| p.align = Align::Center);
        assert_eq!(doc.paragraphs.first().map(|p| p.align), Some(Align::Center));
        assert_eq!(doc.paragraphs.get(1).map(|p| p.align), Some(Align::Left));
    }
    #[test]
    fn table_column_changes_preserve_ids_and_rectangularize_ragged_import() -> Result<(), String> {
        let mut doc = Doc::from_text("table");
        let anchor = doc.paragraphs.first().ok_or("paragraph")?.id;
        doc.add_table(anchor, 2, 2)?;
        let mut next = doc.next_id;
        let table = doc.tables.first_mut().ok_or("table")?;
        let keep = table.rows.first().and_then(|r| r.first()).ok_or("cell")?.id;
        table.rows.get_mut(1).ok_or("row")?.pop();
        add_table_column(table, &mut next);
        assert!(table.rows.iter().all(|r| r.len() == 3));
        assert_eq!(table.column_widths.len(), 3);
        remove_table_column(table);
        assert!(table.rows.iter().all(|r| r.len() == 2));
        assert_eq!(table.column_widths.len(), 2);
        assert_eq!(
            table.rows.first().and_then(|r| r.first()).map(|c| c.id),
            Some(keep)
        );
        doc.next_id = next;
        doc.validate()?;
        Ok(())
    }

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
    fn unchanged_identical_paragraph_follows_stable_id() -> Result<(), String> {
        let old = Doc::from_text("same\nsame");
        let mut new = old.clone();
        let first = old.paragraphs.first().ok_or("paragraph")?.id;
        new.insert_text(first, 0, "same\n")?;
        assert_eq!(
            rebase_position(
                &old,
                &new,
                Position {
                    paragraph: 1,
                    offset: 2
                }
            ),
            Position {
                paragraph: 2,
                offset: 2
            }
        );
        Ok(())
    }
    #[test]
    fn explicit_identical_insert_start_and_end_have_different_cursor_transforms()
    -> Result<(), String> {
        let old = Doc::from_text("same");
        let mut at_start = old.clone();
        let mut at_end = old.clone();
        let start = Position {
            paragraph: 0,
            offset: 0,
        };
        let end = Position {
            paragraph: 0,
            offset: 4,
        };
        let start_delta = TextEditDelta::capture(&at_start, start, start)?;
        let after = at_start.replace_range(start, start, "same\n", None)?;
        let start_delta = start_delta.finish(&at_start, after)?;
        let end_delta = TextEditDelta::capture(&at_end, end, end)?;
        let after = at_end.replace_range(end, end, "\nsame", None)?;
        let end_delta = end_delta.finish(&at_end, after)?;
        assert_eq!(at_start.plain_text(), at_end.plain_text());
        assert_eq!(
            start_delta.transform(Position {
                paragraph: 0,
                offset: 2
            }),
            Position {
                paragraph: 1,
                offset: 2
            }
        );
        assert_eq!(
            end_delta.transform(Position {
                paragraph: 0,
                offset: 2
            }),
            Position {
                paragraph: 0,
                offset: 2
            }
        );
        Ok(())
    }
    #[test]
    fn sequential_exact_deltas_transform_both_selection_ends_and_inverse() -> Result<(), String> {
        let mut doc = Doc::from_text("abcdef");
        let mut cursor = Position {
            paragraph: 0,
            offset: 5,
        };
        let mut anchor = Position {
            paragraph: 0,
            offset: 2,
        };
        let before = (cursor, anchor);
        let a = Position {
            paragraph: 0,
            offset: 0,
        };
        let delta = TextEditDelta::capture(&doc, a, a)?;
        let after = doc.replace_range(a, a, "xy\n", None)?;
        let first = delta.finish(&doc, after)?;
        cursor = first.transform(cursor);
        anchor = first.transform(anchor);
        let a = Position {
            paragraph: 1,
            offset: 1,
        };
        let b = Position {
            paragraph: 1,
            offset: 2,
        };
        let delta = TextEditDelta::capture(&doc, a, b)?;
        let after = doc.replace_range(a, b, "LONG", None)?;
        let second = delta.finish(&doc, after)?;
        cursor = second.transform(cursor);
        anchor = second.transform(anchor);
        assert_eq!(
            (cursor, anchor),
            (
                Position {
                    paragraph: 1,
                    offset: 8
                },
                Position {
                    paragraph: 1,
                    offset: 5
                }
            )
        );
        cursor = first
            .inverse()
            .transform(second.inverse().transform(cursor));
        anchor = first
            .inverse()
            .transform(second.inverse().transform(anchor));
        assert_eq!((cursor, anchor), before);
        Ok(())
    }
    #[test]
    fn mcp_multicall_editor_selection_rebases_once() -> Result<(), String> {
        use crate::{native::Document, ops::Tab};
        use serde_json::json;
        let original = Doc::from_text("same\nsame");
        let first_id = original.paragraphs.first().ok_or("first")?.id;
        let second_id = original.paragraphs.get(1).ok_or("second")?.id;
        let mut tabs = vec![Tab::new(
            1,
            Document::Docs(original.clone()),
            "test".into(),
            None,
        )];
        let mut next_id = 2;
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let seed = EditorState {
            cursor: Position {
                paragraph: 1,
                offset: 3,
            },
            anchor: Position {
                paragraph: 1,
                offset: 1,
            },
            previous: Some(std::sync::Arc::new(original.clone())),
            ..EditorState::default()
        };
        let mut doc = original;
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1100.0, 900.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input(), |ui| {
            ui.ctx()
                .data_mut(|d| d.insert_temp(ui.id().with("docs-state"), seed.clone()));
            show_with_edits(ui, &mut doc, Some(&[]));
        });
        output.textures_delta.clear();
        for (name, args) in [
            (
                "docs_insert_text",
                json!({"document_id":1,"paragraph_id":first_id,"offset":0,"text":"same\n"}),
            ),
            (
                "docs_insert_text",
                json!({"document_id":1,"paragraph_id":second_id,"offset":0,"text":"x"}),
            ),
            (
                "docs_delete_range",
                json!({"document_id":1,"start_paragraph_id":second_id,"start_offset":0,"end_offset":1}),
            ),
        ] {
            crate::mcp::tools::dispatch(&mut tabs, &mut next_id, name, &args, "agent")?;
        }
        let edits = tabs
            .first_mut()
            .and_then(|t| t.pending_docs_edits.take())
            .ok_or("missing deltas")?;
        doc = match &tabs.first().ok_or("tab")?.document {
            Document::Docs(d) => d.clone(),
            _ => return Err("kind".into()),
        };
        let mut observed = None;
        let mut output = ctx.run_ui(input(), |ui| {
            show_with_edits(ui, &mut doc, Some(&edits));
            observed = ui
                .ctx()
                .data_mut(|d| d.get_temp::<EditorState>(ui.id().with("docs-state")));
        });
        output.textures_delta.clear();
        let state = observed.take().ok_or("editor")?;
        assert_eq!(
            state.cursor,
            Position {
                paragraph: 2,
                offset: 3
            }
        );
        assert_eq!(
            state.anchor,
            Position {
                paragraph: 2,
                offset: 1
            }
        );
        let mut output = ctx.run_ui(input(), |ui| {
            show_with_edits(ui, &mut doc, Some(&[]));
            observed = ui
                .ctx()
                .data_mut(|d| d.get_temp::<EditorState>(ui.id().with("docs-state")));
        });
        output.textures_delta.clear();
        let state = observed.take().ok_or("editor")?;
        assert_eq!(
            state.cursor,
            Position {
                paragraph: 2,
                offset: 3
            }
        );
        assert_eq!(
            state.anchor,
            Position {
                paragraph: 2,
                offset: 1
            }
        );
        Ok(())
    }

    #[test]
    fn mcp_replace_all_undo_redo_emit_ordered_exact_deltas() -> Result<(), String> {
        use crate::{native::Document, ops::Tab};
        use serde_json::json;
        let doc = Doc::from_text("aa aa aa");
        let mut tabs = vec![Tab::new(1, Document::Docs(doc), "test".into(), None)];
        let mut next_id = 2;
        crate::mcp::tools::dispatch(
            &mut tabs,
            &mut next_id,
            "docs_replace_all",
            &json!({
                "document_id":1,"query":"aa","replacement":"aa\n","case_sensitive":true
            }),
            "agent",
        )?;
        let original = Position {
            paragraph: 0,
            offset: 7,
        };
        let mut cursor = original;
        let edits = tabs
            .first_mut()
            .and_then(|t| t.pending_docs_edits.take())
            .ok_or("deltas")?;
        assert_eq!(edits.len(), 3);
        for edit in &edits {
            cursor = edit.transform(cursor);
        }
        assert_eq!(
            cursor,
            Position {
                paragraph: 3,
                offset: 0
            }
        );
        if let Some(t) = tabs.first_mut() {
            t.pending_docs_edits = Some(Vec::new());
        }
        crate::mcp::tools::dispatch(
            &mut tabs,
            &mut next_id,
            "undo",
            &json!({"document_id":1}),
            "agent",
        )?;
        let edits = tabs
            .first_mut()
            .and_then(|t| t.pending_docs_edits.take())
            .ok_or("inverse")?;
        for edit in &edits {
            cursor = edit.transform(cursor);
        }
        // The caret was inside replaced text; Undo restores it to the range boundary.
        assert_eq!(
            cursor,
            Position {
                paragraph: 0,
                offset: 8
            }
        );
        if let Some(t) = tabs.first_mut() {
            t.pending_docs_edits = Some(Vec::new());
        }
        crate::mcp::tools::dispatch(
            &mut tabs,
            &mut next_id,
            "redo",
            &json!({"document_id":1}),
            "agent",
        )?;
        let edits = tabs
            .first_mut()
            .and_then(|t| t.pending_docs_edits.take())
            .ok_or("redo")?;
        for edit in &edits {
            cursor = edit.transform(cursor);
        }
        assert_eq!(
            cursor,
            Position {
                paragraph: 3,
                offset: 0
            }
        );
        Ok(())
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
