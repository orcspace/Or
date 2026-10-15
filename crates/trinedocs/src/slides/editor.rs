use super::model::{Deck, Element, ElementKind, Slide, Theme};
use super::present::Presentation;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

#[derive(Clone)]
struct SlideDrag(u64);
type ImageResult = Result<Option<Vec<u8>>, String>;
type ImageReceiver = std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<ImageResult>>>;
#[derive(Clone)]
struct ImageJob {
    slide_id: u64,
    result: ImageReceiver,
}
#[derive(Clone, Default)]
struct State {
    slide_id: u64,
    ribbon_tab: usize,
    table_rows: usize,
    table_columns: usize,
    show_thumbnails: bool,
    hide_thumbnails: bool,
    show_properties: bool,
    canvas_rect: Option<Rect>,
    show_notes: bool,
    selected: Vec<u64>,
    clipboard: std::sync::Arc<Vec<Element>>,
    drag_original: Vec<(u64, f32, f32, f32, f32)>,
    edit_text: Option<u64>,
    /// Table cell (row, column) edited in place when `edit_text` is a table.
    edit_cell: Option<(usize, usize)>,
    /// Focus the in-place editor on the next frame.
    edit_focus: bool,
    grid: bool,
    zoom: f32,
    error: String,
    image_job: Option<ImageJob>,
    presentation: Presentation,
}
const PRESENTATION_ID: &str = "trinedocs_active_presentation";
pub fn is_presenting(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<egui::Id>(egui::Id::new(PRESENTATION_ID)))
        .is_some()
}
pub fn stop_presentation(ctx: &egui::Context) {
    let id = ctx.data_mut(|data| {
        let marker = egui::Id::new(PRESENTATION_ID);
        let id = data.get_temp::<egui::Id>(marker);
        data.remove::<egui::Id>(marker);
        id
    });
    if let Some(id) = id {
        if let Some(mut state) = ctx.data(|data| data.get_temp::<State>(id)) {
            state.presentation.active = false;
            ctx.data_mut(|data| data.insert_temp(id, state));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
    }
}
fn record_presentation(ctx: &egui::Context, id: egui::Id, active: bool) {
    ctx.data_mut(|data| {
        let marker = egui::Id::new(PRESENTATION_ID);
        if active {
            data.insert_temp(marker, id);
        } else if data.get_temp::<egui::Id>(marker) == Some(id) {
            data.remove::<egui::Id>(marker);
        }
    });
}
pub fn show(ui: &mut egui::Ui, deck: &mut Deck) -> bool {
    let memory_id = ui.id().with("slides_editor");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(memory_id).unwrap_or_default());
    if state.zoom <= 0.0 {
        state.zoom = 1.0;
    }
    if state.table_rows == 0 {
        state.table_rows = 3;
    }
    if state.table_columns == 0 {
        state.table_columns = 3;
    }
    if let Some(clipboard) = ui.ctx().data(|data| {
        data.get_temp::<std::sync::Arc<Vec<Element>>>(egui::Id::new("slides-shared-clipboard"))
    }) {
        state.clipboard = clipboard;
    }
    let mut dropped_slide = None;
    if deck.slide(state.slide_id).is_none() {
        state.slide_id = deck.slides.first().map(|s| s.id).unwrap_or(0);
        state.selected.clear();
    }
    state.selected.retain(|id| {
        deck.slide(state.slide_id)
            .is_some_and(|slide| slide.element(*id).is_some())
    });
    if state.edit_text.is_some_and(|id| {
        deck.slide(state.slide_id)
            .is_none_or(|slide| slide.element(id).is_none())
    }) {
        state.edit_text = None;
    }
    if ui.is_enabled()
        && !egui::Popup::is_any_open(ui.ctx())
        && ui.input(|i| i.key_pressed(egui::Key::F5))
    {
        let index = if ui.input(|i| i.modifiers.shift) {
            deck.slides
                .iter()
                .position(|s| s.id == state.slide_id)
                .unwrap_or(0)
        } else {
            0
        };
        state.presentation.start(ui.ctx(), deck, index);
    }
    if state.presentation.active {
        super::present::show(ui, deck, &mut state.presentation);
        record_presentation(ui.ctx(), memory_id, state.presentation.active);
        ui.ctx().data_mut(|d| d.insert_temp(memory_id, state));
        return false;
    }
    let mut changed = false;
    let image_result = state
        .image_job
        .as_ref()
        .filter(|_| ui.is_enabled())
        .and_then(|job| {
            let result = match job.result.lock() {
                Ok(receiver) => match receiver.try_recv() {
                    Ok(result) => Some(result),
                    Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        Some(Err("Загрузка изображения прервана".into()))
                    }
                },
                Err(_) => Some(Err("Ошибка загрузки изображения".into())),
            };
            result.map(|result| (job.slide_id, result))
        });
    if let Some((slide_id, result)) = image_result {
        state.image_job = None;
        match result {
            Ok(Some(bytes)) => {
                changed |= insert_image(deck, &mut state, slide_id, bytes);
            }
            Ok(None) => {}
            Err(error) => state.error = error,
        }
    }
    if ui.is_enabled()
        && let Some(png) = crate::clipboard::take_image(ui.ctx())
    {
        let slide_id = state.slide_id;
        changed |= insert_image(deck, &mut state, slide_id, png.to_vec());
    }
    if state.image_job.is_some() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
    changed |= ribbon(ui, deck, &mut state);
    let (deck_width, deck_height) = (deck.width, deck.height);
    if !state.error.is_empty() {
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(190, 50, 50), &state.error);
            if ui.small_button("×").clicked() {
                state.error.clear();
            }
        });
    }
    if let Some(warning) = deck.warnings.first() {
        ui.small(warning);
    }
    let compact = ui.available_width() < 860.0;
    let compact_notes = ui.available_height() < 300.0;
    let notes_visible = state.show_notes && !compact_notes;
    let main_height =
        (ui.available_height() - 34.0 - if notes_visible { 94.0 } else { 0.0 }).max(60.0);
    let main_width = ui.available_width();
    let thumbnails_visible = !compact && !state.hide_thumbnails;
    let properties_visible = !compact && state.show_properties;
    let spacing = ui.spacing().item_spacing.x;
    let center_width = (main_width
        - if thumbnails_visible {
            172.0 + spacing
        } else {
            0.0
        }
        - if properties_visible {
            260.0 + spacing
        } else {
            0.0
        })
    .max(100.0);
    ui.horizontal_top(|ui| {
        if thumbnails_visible {
            ui.allocate_ui_with_layout(
                Vec2::new(172.0, main_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    changed |=
                        thumbnail_panel(ui, deck, &mut state, main_height, &mut dropped_slide);
                },
            );
        }
        ui.allocate_ui_with_layout(
            Vec2::new(center_width, main_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_width(center_width);
                egui::Frame::new()
                    .fill(crate::chrome::workspace(ui))
                    .inner_margin(12)
                    .corner_radius(6)
                    .show(ui, |ui| {
                        let available_width = (center_width - 24.0).max(76.0);
                        let available_height = (main_height - 24.0).max(24.0);
                        if let Some(slide) = deck.slide_mut(state.slide_id) {
                            let scale = (available_width / deck_width)
                                .min(available_height / deck_height)
                                * state.zoom;
                            egui::ScrollArea::both()
                                .id_salt("slides_canvas")
                                .auto_shrink([false, false])
                                .max_height(available_height)
                                .show(ui, |ui| {
                                    ui.add_space(
                                        ((available_height - deck_height * scale) / 2.0).max(0.0),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.add_space(
                                            ((available_width - deck_width * scale) / 2.0).max(0.0),
                                        );
                                        changed |= canvas(
                                            ui,
                                            slide,
                                            deck_width,
                                            deck_height,
                                            scale,
                                            &mut state,
                                        );
                                    });
                                });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("Создайте первый слайд");
                            });
                        }
                    });
            },
        );
        if properties_visible {
            ui.allocate_ui_with_layout(
                Vec2::new(260.0, main_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    changed |= slide_panel(ui, deck, &mut state, main_height);
                },
            );
        }
    });
    if compact {
        let enabled = ui.is_enabled();
        if state.show_thumbnails {
            let mut open = true;
            egui::Window::new("Слайды")
                .id(memory_id.with("thumbnails-window"))
                .open(&mut open)
                .default_width(195.0)
                .default_height(420.0)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let height = ui.available_height().clamp(140.0, 600.0);
                        changed |=
                            thumbnail_panel(ui, deck, &mut state, height, &mut dropped_slide);
                    });
                });
            state.show_thumbnails = open;
        }
        if state.show_properties {
            let mut open = true;
            egui::Window::new("Свойства слайда и объекта")
                .id(memory_id.with("properties-window"))
                .open(&mut open)
                .default_width(280.0)
                .default_height(500.0)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let height = ui.available_height().clamp(140.0, 650.0);
                        changed |= slide_panel(ui, deck, &mut state, height);
                    });
                });
            state.show_properties = open;
        }
    }
    if let Some((id, index)) = dropped_slide {
        match deck.move_slide(id, index) {
            Ok(()) => changed = true,
            Err(e) => state.error = e,
        }
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let index = deck
            .slides
            .iter()
            .position(|slide| slide.id == state.slide_id);
        ui.label(
            egui::RichText::new(format!(
                "Слайд {} из {}",
                index.map(|index| index + 1).unwrap_or(0),
                deck.slides.len()
            ))
            .color(crate::chrome::muted(ui)),
        );
        ui.separator();
        ui.toggle_value(&mut state.show_notes, "Заметки");
        ui.toggle_value(&mut state.show_properties, "Свойства");
        if compact && ui.button("Слайды…").clicked() {
            state.show_thumbnails = true;
        }
    });
    if compact_notes {
        if state.show_notes {
            let mut open = true;
            let enabled = ui.is_enabled();
            egui::Window::new("Заметки докладчика")
                .id(memory_id.with("notes-window"))
                .open(&mut open)
                .default_size(Vec2::new(500.0, 250.0))
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        if let Some(slide) = deck.slide_mut(state.slide_id) {
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(&mut slide.notes)
                                        .id(memory_id.with("notes"))
                                        .desired_rows(8)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed();
                        }
                    });
                });
            state.show_notes = open;
        }
    } else if state.show_notes
        && let Some(slide) = deck.slide_mut(state.slide_id)
    {
        ui.label(egui::RichText::new("Заметки докладчика").color(crate::chrome::muted(ui)));
        changed |= ui
            .add_sized(
                Vec2::new(ui.available_width(), 70.0),
                egui::TextEdit::multiline(&mut slide.notes)
                    .id(memory_id.with("notes"))
                    .desired_rows(3),
            )
            .changed();
    }
    if ui.is_enabled()
        && !ui.ctx().text_edit_focused()
        && !egui::Popup::is_any_open(ui.ctx())
        && state.edit_text.is_none()
    {
        let key = |key| ui.input(|i| i.modifiers.ctrl && i.key_pressed(key));
        if key(egui::Key::A)
            && let Some(slide) = deck.slide(state.slide_id)
        {
            state.selected = slide.elements.iter().map(|e| e.id).collect();
        }
        if key(egui::Key::C)
            || ui.input(|i| {
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy))
            })
        {
            copy_selection(ui.ctx(), deck, &mut state);
        }
        if key(egui::Key::X)
            || ui.input(|i| {
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Cut))
            })
        {
            copy_selection(ui.ctx(), deck, &mut state);
            changed |= delete_selection(deck, &mut state);
        }
        let pasted_text = ui.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Paste(text) = event {
                    Some(text.clone())
                } else {
                    None
                }
            })
        });
        if let Some(text) = pasted_text {
            let internal_text = state
                .clipboard
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            if !state.clipboard.is_empty() && text == internal_text {
                changed |= paste_selection(deck, &mut state);
            } else if !text.is_empty() {
                match deck.add_element(state.slide_id, ElementKind::Text) {
                    Ok(id) => {
                        if let Some(element) = deck
                            .slide_mut(state.slide_id)
                            .and_then(|slide| slide.element_mut(id))
                        {
                            element.text = text;
                        }
                        state.selected = vec![id];
                        changed = true;
                    }
                    Err(error) => state.error = error,
                }
            }
        } else if key(egui::Key::V) && owns_system_clipboard(&state) {
            changed |= paste_selection(deck, &mut state);
        }
        if key(egui::Key::D) {
            changed |= duplicate_selection(deck, &mut state);
        }
    }
    record_presentation(ui.ctx(), memory_id, state.presentation.active);
    ui.ctx().data_mut(|d| d.insert_temp(memory_id, state));
    changed
}
fn thumbnail_panel(
    ui: &mut egui::Ui,
    deck: &mut Deck,
    state: &mut State,
    main_height: f32,
    dropped_slide: &mut Option<(u64, usize)>,
) -> bool {
    let mut changed = false;
    let left_width = 172.0;
    egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .inner_margin(9)
        .corner_radius(8)
        .show(ui, |ui| {
            ui.set_width(left_width - 18.0);
            ui.label(
                egui::RichText::new(format!("Слайды · {}", deck.slides.len()))
                    .strong()
                    .color(crate::chrome::muted(ui)),
            );
            egui::ScrollArea::vertical()
                .id_salt("slide_thumbnails")
                .max_height((main_height - 70.0).max(70.0))
                .show(ui, |ui| {
                    for (index, slide) in deck.slides.iter().enumerate() {
                        let selected = slide.id == state.slide_id;
                        if (index == 0
                            || deck
                                .slides
                                .get(index.wrapping_sub(1))
                                .is_some_and(|s| s.section != slide.section))
                            && !slide.section.is_empty()
                        {
                            ui.strong(&slide.section);
                        }
                        let preview_width = 140.0;
                        let preview_height = preview_width * deck.height / deck.width;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(150.0, preview_height + 35.0),
                            Sense::click_and_drag(),
                        );
                        response.dnd_set_drag_payload(SlideDrag(slide.id));
                        if response.dnd_hover_payload::<SlideDrag>().is_some() {
                            ui.painter().line_segment(
                                [rect.left_top(), rect.right_top()],
                                Stroke::new(3.0, crate::chrome::accent(ui)),
                            );
                        }
                        if let Some(payload) = response.dnd_release_payload::<SlideDrag>() {
                            *dropped_slide = Some((payload.0, index));
                        }
                        ui.painter().rect_filled(
                            rect,
                            6,
                            if selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().panel_fill
                            },
                        );
                        if selected {
                            ui.painter().rect_stroke(
                                rect,
                                6,
                                Stroke::new(1.5, crate::chrome::accent(ui)),
                                StrokeKind::Inside,
                            );
                        }
                        let canvas = Rect::from_min_size(
                            rect.min + Vec2::splat(5.0),
                            Vec2::new(preview_width, preview_height),
                        );
                        paint_slide(ui.ctx(), ui.painter(), canvas, deck, slide, None);
                        ui.painter().text(
                            rect.left_bottom() + Vec2::new(8.0, -7.0),
                            Align2::LEFT_BOTTOM,
                            format!(
                                "{}{}  {}",
                                index + 1,
                                if slide.hidden { " ◌" } else { "" },
                                slide.title.chars().take(15).collect::<String>()
                            ),
                            FontId::proportional(11.0),
                            ui.visuals().text_color(),
                        );
                        if response.clicked() {
                            state.slide_id = slide.id;
                            state.selected.clear();
                            state.edit_text = None;
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui
                    .button("↑")
                    .on_hover_text("Сдвинуть слайд вверх")
                    .clicked()
                    && let Some(index) = deck.slides.iter().position(|s| s.id == state.slide_id)
                    && index > 0
                {
                    match deck.move_slide(state.slide_id, index - 1) {
                        Ok(()) => changed = true,
                        Err(error) => state.error = error,
                    }
                }
                if ui
                    .button("↓")
                    .on_hover_text("Сдвинуть слайд вниз")
                    .clicked()
                    && let Some(index) = deck.slides.iter().position(|s| s.id == state.slide_id)
                    && index + 1 < deck.slides.len()
                {
                    match deck.move_slide(state.slide_id, index + 1) {
                        Ok(()) => changed = true,
                        Err(error) => state.error = error,
                    }
                }
                ui.small("Перетащите");
            });
        });

    changed
}
fn slide_panel(ui: &mut egui::Ui, deck: &mut Deck, state: &mut State, main_height: f32) -> bool {
    let mut changed = false;
    let right_width = 260.0;
    egui::Frame::new().fill(ui.visuals().window_fill).inner_margin(10).corner_radius(8).show(ui, |ui| {
                ui.set_width(right_width - 20.0);
                egui::ScrollArea::vertical().id_salt("slides_properties").max_height(main_height - 20.0).show(ui, |ui| {
                    if let Some(slide) = deck.slide_mut(state.slide_id) {
                        ui.strong("Свойства слайда");
                        changed |= ui.add(egui::TextEdit::singleline(&mut slide.title).desired_width(f32::INFINITY)).changed();
                        ui.horizontal(|ui| {
                            changed |= ui.checkbox(&mut slide.hidden, "Скрыть в показе").changed();
                            ui.label("Фон");
                            changed |= ui.color_edit_button_srgb(&mut slide.background).changed();
                        });
                        ui.label(egui::RichText::new("Раздел").color(crate::chrome::muted(ui)));
                        changed |= ui.add(egui::TextEdit::singleline(&mut slide.section).desired_width(f32::INFINITY).hint_text("Без раздела")).changed();
                        ui.separator();
                        egui::CollapsingHeader::new("Переход слайда").default_open(false).show(ui, |ui| {
                            egui::ComboBox::from_id_salt("transition").selected_text(effect_label(&slide.transition.effect)).show_ui(ui, |ui| {
                                for effect in ["none", "fade", "push", "wipe"] {
                                    changed |= ui.selectable_value(&mut slide.transition.effect, effect.into(), effect_label(effect)).changed();
                                }
                            });
                            changed |= ui.add(egui::Slider::new(&mut slide.transition.duration, 0.1..=3.0).text("Сек.")).changed();
                            let mut auto = slide.transition.advance_after.is_some();
                            if ui.checkbox(&mut auto, "Автоматический переход").changed() {
                                slide.transition.advance_after = if auto { Some(5.0) } else { None }; changed = true;
                            }
                            if let Some(seconds) = &mut slide.transition.advance_after {
                                changed |= ui.add(egui::DragValue::new(seconds).range(0.1..=3600.0).suffix(" сек.")).changed();
                            }
                        });
                        ui.separator();
                        if let Some(id) = state.selected.first().copied() {
                            if state.selected.len() > 1 { ui.small(format!("Выделено {} · свойства первого объекта", state.selected.len())); }
                            if let Some(element) = slide.element_mut(id) { changed |= properties(ui, element); }
                        } else {
                            ui.add_space(15.0);
                            ui.label(egui::RichText::new("Выберите объект").strong());
                            ui.label(egui::RichText::new("Клик — выделение\nДвойной клик — текст\nCtrl / Shift — несколько объектов").color(crate::chrome::muted(ui)));
                        }
                    }
                });
            });

    changed
}

/// Applies `change` to every selected element of the current slide.
fn edit_selected(deck: &mut Deck, state: &State, mut change: impl FnMut(&mut Element)) -> bool {
    let Some(slide) = deck.slide_mut(state.slide_id) else {
        return false;
    };
    let mut changed = false;
    for id in &state.selected {
        if let Some(element) = slide.element_mut(*id) {
            change(element);
            changed = true;
        }
    }
    changed
}
fn new_slide_menu(ui: &mut egui::Ui, deck: &mut Deck, state: &mut State) -> bool {
    let mut changed = false;
    ui.menu_button("＋ Новый слайд", |ui| {
        for (name, layout) in [
            ("Титульный слайд", "title"),
            ("Заголовок и объект", "content"),
            ("Два объекта", "two_content"),
            ("Пустой слайд", "blank"),
        ] {
            if ui.button(name).clicked() {
                state.slide_id = deck.add_slide(layout);
                state.selected.clear();
                state.edit_text = None;
                changed = true;
                ui.close();
            }
        }
    });
    changed
}
fn ribbon(ui: &mut egui::Ui, deck: &mut Deck, state: &mut State) -> bool {
    let mut changed = false;
    let has_slide = deck.slide(state.slide_id).is_some();
    let has_selection = has_slide && !state.selected.is_empty();
    let compact = ui.available_width() < 860.0;
    let current = deck
        .slide(state.slide_id)
        .and_then(|slide| slide.element(state.selected.first().copied()?))
        .map(|e| Element {
            font: e.font.clone(),
            font_size: e.font_size,
            bold: e.bold,
            italic: e.italic,
            color: e.color,
            fill: e.fill,
            stroke: e.stroke,
            stroke_width: e.stroke_width,
            ..Element::default()
        });
    crate::chrome::tabs(
        ui,
        &mut state.ribbon_tab,
        &["Главная", "Вставка", "Конструктор", "Показ слайдов", "Вид"],
    );
    crate::chrome::ribbon(ui, |ui| match state.ribbon_tab {
        0 => {
            crate::chrome::group(ui, "Буфер обмена", 150.0, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button("Вставить")
                        .on_hover_text("Вставить объекты, текст или рисунок · Ctrl+V")
                        .clicked()
                    {
                        if owns_system_clipboard(state) {
                            changed |= paste_selection(deck, state);
                        } else {
                            crate::clipboard::paste(ui.ctx());
                        }
                    }
                    ui.add_enabled_ui(has_selection, |ui| {
                        if crate::chrome::tool(ui, "⧉", "Копировать · Ctrl+C", false).clicked()
                        {
                            copy_selection(ui.ctx(), deck, state);
                        }
                        if crate::chrome::tool(ui, "✂", "Вырезать · Ctrl+X", false).clicked()
                        {
                            copy_selection(ui.ctx(), deck, state);
                            changed |= delete_selection(deck, state);
                        }
                    });
                });
                ui.add_enabled_ui(has_selection, |ui| {
                    if ui.small_button("Дублировать · Ctrl+D").clicked() {
                        changed |= duplicate_selection(deck, state);
                    }
                });
            });
            crate::chrome::group(ui, "Слайды", 170.0, |ui| {
                changed |= new_slide_menu(ui, deck, state);
                ui.add_enabled_ui(has_slide, |ui| {
                    ui.horizontal(|ui| {
                        changed |= layout_menu(ui, deck, state);
                        if crate::chrome::tool(ui, "⧉", "Дублировать слайд", false).clicked()
                        {
                            match deck.duplicate_slide(state.slide_id) {
                                Ok(id) => {
                                    state.slide_id = id;
                                    state.selected.clear();
                                    state.edit_text = None;
                                    changed = true;
                                }
                                Err(error) => state.error = error,
                            }
                        }
                        if crate::chrome::tool(ui, "🗑", "Удалить слайд", false).clicked()
                        {
                            let position = deck
                                .slides
                                .iter()
                                .position(|slide| slide.id == state.slide_id)
                                .unwrap_or(0);
                            match deck.delete_slide(state.slide_id) {
                                Ok(()) => {
                                    // Stay in place: the next slide, or the previous one at the end.
                                    state.slide_id = deck
                                        .slides
                                        .get(position)
                                        .or_else(|| deck.slides.last())
                                        .map(|slide| slide.id)
                                        .unwrap_or(0);
                                    state.selected.clear();
                                    state.edit_text = None;
                                    changed = true;
                                }
                                Err(error) => state.error = error,
                            }
                        }
                    });
                });
            });
            crate::chrome::group(ui, "Шрифт", 236.0, |ui| {
                ui.add_enabled_ui(has_selection, |ui| {
                    let format = current.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let mut family = format.font.clone();
                        if crate::chrome::font_picker(ui, "slides-font", &mut family, 120.0) {
                            changed |= edit_selected(deck, state, |e| e.font = family.clone());
                        }
                        let mut size = format.font_size;
                        if crate::chrome::size_picker(ui, "slides-size", &mut size) {
                            let size = size.clamp(1.0, 300.0);
                            changed |= edit_selected(deck, state, |e| e.font_size = size);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if crate::chrome::tool(
                            ui,
                            egui::RichText::new("B").strong(),
                            "Полужирный",
                            format.bold,
                        )
                        .clicked()
                        {
                            changed |= edit_selected(deck, state, |e| e.bold = !format.bold);
                        }
                        if crate::chrome::tool(
                            ui,
                            egui::RichText::new("I").italics(),
                            "Курсив",
                            format.italic,
                        )
                        .clicked()
                        {
                            changed |= edit_selected(deck, state, |e| e.italic = !format.italic);
                        }
                        for (glyph, grow, tip) in [
                            ("A+", true, "Увеличить размер шрифта"),
                            ("A−", false, "Уменьшить размер шрифта"),
                        ] {
                            if crate::chrome::tool(ui, glyph, tip, false).clicked() {
                                let size = crate::chrome::step_size(format.font_size, grow)
                                    .clamp(1.0, 300.0);
                                changed |= edit_selected(deck, state, |e| e.font_size = size);
                            }
                        }
                        if let Some(Some(color)) = crate::chrome::color_menu(
                            ui,
                            "A",
                            "Цвет текста",
                            Some(format.color),
                            None,
                        ) {
                            changed |= edit_selected(deck, state, |e| e.color = color);
                        }
                    });
                });
            });
            crate::chrome::group(ui, "Рисование", 196.0, |ui| {
                ui.add_enabled_ui(has_selection, |ui| {
                    let format = current.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if let Some(Some(fill)) = crate::chrome::color_menu(
                            ui,
                            "◧",
                            "Заливка фигуры",
                            Some(format.fill),
                            None,
                        ) {
                            changed |= edit_selected(deck, state, |e| e.fill = fill);
                        }
                        if let Some(Some(stroke)) = crate::chrome::color_menu(
                            ui,
                            "▢",
                            "Контур фигуры",
                            Some(format.stroke),
                            None,
                        ) {
                            changed |= edit_selected(deck, state, |e| e.stroke = stroke);
                        }
                        let mut width = format.stroke_width;
                        if ui
                            .add(
                                egui::DragValue::new(&mut width)
                                    .range(0.0..=40.0)
                                    .speed(0.25)
                                    .suffix(" pt"),
                            )
                            .on_hover_text("Толщина контура")
                            .changed()
                        {
                            changed |= edit_selected(deck, state, |e| e.stroke_width = width);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        ui.menu_button("Упорядочить", |ui| {
                            for (label, front) in
                                [("На передний план", true), ("На задний план", false)]
                            {
                                if ui.button(label).clicked() {
                                    match deck.set_z_order_many(
                                        state.slide_id,
                                        &state.selected,
                                        front,
                                    ) {
                                        Ok(()) => changed = true,
                                        Err(error) => state.error = error,
                                    }
                                    ui.close();
                                }
                            }
                            ui.separator();
                            if ui
                                .add_enabled(
                                    state.selected.len() > 1,
                                    egui::Button::new("Сгруппировать"),
                                )
                                .clicked()
                            {
                                let group = deck.allocate_id();
                                changed |= edit_selected(deck, state, |e| e.group = Some(group));
                                ui.close();
                            }
                            if ui.button("Разгруппировать").clicked() {
                                let mut any = false;
                                edit_selected(deck, state, |e| any |= e.group.take().is_some());
                                changed |= any;
                                ui.close();
                            }
                            ui.separator();
                            for (label, angle) in [
                                ("Повернуть вправо на 90°", 90.0),
                                ("Повернуть влево на 90°", -90.0),
                            ] {
                                let supported = deck.slide(state.slide_id).is_some_and(|slide| {
                                    state.selected.iter().all(|id| {
                                        slide.element(*id).is_some_and(|e| {
                                            !matches!(
                                                e.kind,
                                                ElementKind::Table { .. }
                                                    | ElementKind::Chart { .. }
                                            )
                                        })
                                    })
                                });
                                if ui
                                    .add_enabled(supported, egui::Button::new(label))
                                    .on_disabled_hover_text(
                                        "Поворот таблиц и диаграмм пока не поддерживается",
                                    )
                                    .clicked()
                                {
                                    changed |= edit_selected(deck, state, |e| {
                                        e.rotation = (e.rotation + angle).rem_euclid(360.0);
                                    });
                                    ui.close();
                                }
                            }
                        });
                        if crate::chrome::tool(ui, "✎", "Редактировать текст", false).clicked()
                            && let Some(id) = state.selected.first().copied()
                        {
                            begin_edit(state, id, Some((0, 0)));
                        }
                        if crate::chrome::tool(ui, "🗑", "Удалить объект · Delete", false).clicked()
                        {
                            changed |= delete_selection(deck, state);
                        }
                    });
                });
            });
            crate::chrome::group(ui, "Выравнивание", 176.0, |ui| {
                ui.add_enabled_ui(has_selection, |ui| {
                    for row in [
                        [
                            ("⇤", "left", "По левому краю"),
                            ("↔", "center", "По центру"),
                            ("⇥", "right", "По правому краю"),
                            ("⇹", "horizontal", "Распределить по горизонтали"),
                        ],
                        [
                            ("⤒", "top", "По верхнему краю"),
                            ("↕", "middle", "Посередине"),
                            ("⤓", "bottom", "По нижнему краю"),
                            ("⇳", "vertical", "Распределить по вертикали"),
                        ],
                    ] {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (glyph, axis, tip) in row {
                                let enabled = !matches!(axis, "horizontal" | "vertical")
                                    || state.selected.len() >= 3;
                                let response = ui
                                    .add_enabled_ui(enabled, |ui| {
                                        crate::chrome::tool(ui, glyph, tip, false)
                                    })
                                    .inner;
                                if response.clicked() {
                                    let (width, height) = (deck.width, deck.height);
                                    if let Some(slide) = deck.slide_mut(state.slide_id) {
                                        align(slide, &state.selected, axis, width, height);
                                        changed = true;
                                    }
                                }
                            }
                        });
                    }
                });
            });
        }
        1 => {
            crate::chrome::group(ui, "Слайды", 120.0, |ui| {
                changed |= new_slide_menu(ui, deck, state);
            });
            ui.add_enabled_ui(has_slide, |ui| {
                crate::chrome::group(ui, "Таблицы", 110.0, |ui| {
                    ui.menu_button("▦ Таблица", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Строки");
                            ui.add(egui::DragValue::new(&mut state.table_rows).range(1..=20));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Столбцы");
                            ui.add(egui::DragValue::new(&mut state.table_columns).range(1..=20));
                        });
                        let kind = ElementKind::Table {
                            cells: vec![
                                vec![String::new(); state.table_columns.max(1)];
                                state.table_rows.max(1)
                            ],
                        };
                        if insert_button(ui, deck, state, "Вставить таблицу", kind) {
                            changed = true;
                            ui.close();
                        }
                    });
                });
                crate::chrome::group(ui, "Изображения", 120.0, |ui| {
                    if ui
                        .add_enabled(state.image_job.is_none(), egui::Button::new("🖼 Рисунок…"))
                        .on_hover_text("PNG, JPEG · до 32 МБ")
                        .clicked()
                    {
                        start_image_job(ui.ctx(), state);
                    }
                });
                crate::chrome::group(ui, "Иллюстрации", 230.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (glyph, label, kind) in [
                            ("▭", "Прямоугольник", ElementKind::Rect),
                            ("◯", "Эллипс", ElementKind::Ellipse),
                            ("╱", "Линия", ElementKind::Line),
                            ("→", "Стрелка", ElementKind::Arrow),
                        ] {
                            if crate::chrome::tool(ui, glyph, label, false).clicked() {
                                match deck.add_element(state.slide_id, kind) {
                                    Ok(id) => {
                                        state.selected = vec![id];
                                        changed = true;
                                    }
                                    Err(error) => state.error = error,
                                }
                            }
                        }
                    });
                    if insert_button(
                        ui,
                        deck,
                        state,
                        "📊 Диаграмма",
                        ElementKind::Chart {
                            values: vec![30.0, 60.0, 45.0],
                            labels: vec!["A".into(), "B".into(), "C".into()],
                        },
                    ) {
                        changed = true;
                    }
                });
                crate::chrome::group(ui, "Текст", 120.0, |ui| {
                    if insert_button(ui, deck, state, "🅰 Надпись", ElementKind::Text) {
                        changed = true;
                    }
                });
            });
        }
        2 => {
            ui.add_enabled_ui(has_slide, |ui| {
                crate::chrome::group(ui, "Темы", 130.0, |ui| {
                    changed |= theme_menu(ui, deck);
                });
                crate::chrome::group(ui, "Настроить", 210.0, |ui| {
                    ui.menu_button("Размер слайда", |ui| {
                        for (label, width) in
                            [("Широкоэкранный 16:9", 960.0), ("Стандартный 4:3", 720.0)]
                        {
                            if ui
                                .selectable_label(
                                    deck.width == width && deck.height == 540.0,
                                    label,
                                )
                                .clicked()
                            {
                                changed |= deck.width != width || deck.height != 540.0;
                                deck.width = width;
                                deck.height = 540.0;
                                ui.close();
                            }
                        }
                    });
                    let background = deck.slide(state.slide_id).map(|slide| slide.background);
                    ui.horizontal(|ui| {
                        if let Some(Some(color)) =
                            crate::chrome::color_menu(ui, "◩", "Формат фона", background, None)
                            && let Some(slide) = deck.slide_mut(state.slide_id)
                        {
                            slide.background = color;
                            changed = true;
                        }
                        ui.label("Фон слайда");
                    });
                });
            });
        }
        3 => {
            ui.add_enabled_ui(has_slide, |ui| {
                crate::chrome::group(ui, "Начать показ", 230.0, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("▶ С начала").on_hover_text("F5").clicked() {
                            state.presentation.start(ui.ctx(), deck, 0);
                        }
                        if ui
                            .button("▷ С текущего")
                            .on_hover_text("Shift+F5")
                            .clicked()
                        {
                            let index = deck
                                .slides
                                .iter()
                                .position(|slide| slide.id == state.slide_id)
                                .unwrap_or(0);
                            state.presentation.start(ui.ctx(), deck, index);
                        }
                    });
                    ui.small("← → пробел · Esc — выход · указка и перо");
                });
                crate::chrome::group(ui, "Настройка", 150.0, |ui| {
                    let mut hidden = deck.slide(state.slide_id).is_some_and(|slide| slide.hidden);
                    if ui.checkbox(&mut hidden, "Скрыть слайд").changed()
                        && let Some(slide) = deck.slide_mut(state.slide_id)
                    {
                        slide.hidden = hidden;
                        changed = true;
                    }
                });
            });
        }
        _ => {
            crate::chrome::group(ui, "Показать", 300.0, |ui| {
                ui.horizontal(|ui| {
                    if compact {
                        if ui.button("Слайды…").clicked() {
                            state.show_thumbnails = true;
                        }
                    } else {
                        let mut thumbnails = !state.hide_thumbnails;
                        if ui.checkbox(&mut thumbnails, "Миниатюры").changed() {
                            state.hide_thumbnails = !thumbnails;
                        }
                    }
                    ui.checkbox(&mut state.show_notes, "Заметки");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.show_properties, "Свойства");
                    ui.checkbox(&mut state.grid, "Сетка и привязка");
                });
            });
            crate::chrome::group(ui, "Масштаб", 220.0, |ui| {
                ui.add(egui::Slider::new(&mut state.zoom, 0.25..=2.0).show_value(false));
                ui.horizontal(|ui| {
                    ui.label(format!("{:.0}%", state.zoom * 100.0));
                    if ui.button("Вписать").clicked() {
                        state.zoom = 1.0;
                    }
                });
            });
        }
    });
    ui.add_space(4.0);
    changed
}
fn insert_button(
    ui: &mut egui::Ui,
    deck: &mut Deck,
    state: &mut State,
    label: &str,
    kind: ElementKind,
) -> bool {
    if !ui.button(label).clicked() {
        return false;
    }
    match deck.add_element(state.slide_id, kind) {
        Ok(id) => {
            state.selected = vec![id];
            true
        }
        Err(error) => {
            state.error = error;
            false
        }
    }
}
fn layout_menu(ui: &mut egui::Ui, deck: &mut Deck, state: &mut State) -> bool {
    let mut changed = false;
    ui.menu_button("Макет", |ui| {
        for (label, layout) in [
            ("Титульный", "title"),
            ("Заголовок и текст", "content"),
            ("Две колонки", "two_content"),
            ("Пустой", "blank"),
        ] {
            if ui.button(label).clicked() {
                match deck.set_layout(state.slide_id, layout) {
                    Ok(()) => changed = true,
                    Err(error) => state.error = error,
                }
                state.selected.clear();
                state.edit_text = None;
                ui.close();
            }
        }
    });
    changed
}
fn theme_menu(ui: &mut egui::Ui, deck: &mut Deck) -> bool {
    let mut changed = false;
    ui.menu_button("Тема", |ui| {
        for theme in [
            Theme::default(),
            Theme {
                name: "Тёмная".into(),
                background: [24, 30, 42],
                foreground: [244, 246, 250],
                accent: [84, 156, 230],
                ..Theme::default()
            },
            Theme {
                name: "Зелёная".into(),
                background: [245, 250, 247],
                foreground: [26, 53, 38],
                accent: [62, 153, 101],
                ..Theme::default()
            },
        ] {
            if ui.button(&theme.name).clicked() {
                deck.apply_theme(theme);
                changed = true;
                ui.close();
            }
        }
    });
    changed
}
/// Adds a picture to a slide, fitted into 480×280 and centred, and selects it.
fn insert_image(deck: &mut Deck, state: &mut State, slide_id: u64, bytes: Vec<u8>) -> bool {
    let (width, height) = (deck.width, deck.height);
    match super::model::image_dimensions(&bytes).and_then(|size| {
        deck.add_element(slide_id, ElementKind::Image { bytes })
            .map(|id| (id, size))
    }) {
        Ok((id, (w, h))) => {
            if let Some(element) = deck
                .slide_mut(slide_id)
                .and_then(|slide| slide.element_mut(id))
            {
                let factor = (480.0 / w.max(1) as f32).min(280.0 / h.max(1) as f32);
                element.w = w as f32 * factor;
                element.h = h as f32 * factor;
                element.x = ((width - element.w) / 2.0).max(0.0);
                element.y = ((height - element.h) / 2.0).max(0.0);
            }
            if state.slide_id == slide_id {
                state.selected = vec![id];
            }
            true
        }
        Err(error) => {
            state.error = error;
            false
        }
    }
}
fn start_image_job(ctx: &egui::Context, state: &mut State) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    state.image_job = Some(ImageJob {
        slide_id: state.slide_id,
        result: std::sync::Arc::new(std::sync::Mutex::new(receiver)),
    });
    std::thread::spawn(move || {
        let result = (|| {
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
            super::model::image_dimensions(&bytes)?;
            crate::util::decode_image(&bytes)?;
            Ok(Some(bytes))
        })();
        let _ = sender.send(result);
        ctx.request_repaint();
    });
}
fn copy_selection(ctx: &egui::Context, deck: &Deck, state: &mut State) {
    if let Some(slide) = deck.slide(state.slide_id) {
        state.clipboard = slide
            .elements
            .iter()
            .filter(|e| state.selected.contains(&e.id))
            .cloned()
            .collect::<Vec<_>>()
            .into();
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("slides-shared-clipboard"),
                state.clipboard.clone(),
            )
        });
        ctx.copy_text(
            state
                .clipboard
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}
fn paste_selection(deck: &mut Deck, state: &mut State) -> bool {
    if state.clipboard.is_empty() || deck.slide(state.slide_id).is_none() {
        return false;
    }
    let mut groups = std::collections::HashMap::new();
    state.selected.clear();
    for mut element in state.clipboard.iter().cloned() {
        element.id = deck.allocate_id();
        element.x += 20.0;
        element.y += 20.0;
        element.group = element
            .group
            .map(|group| *groups.entry(group).or_insert_with(|| deck.allocate_id()));
        if let Some(slide) = deck.slide_mut(state.slide_id) {
            state.selected.push(element.id);
            slide.elements.push(element);
        }
    }
    true
}
fn clipboard_text(elements: &[Element]) -> String {
    elements
        .iter()
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn owns_system_clipboard(state: &State) -> bool {
    !state.clipboard.is_empty()
        && arboard::Clipboard::new()
            .ok()
            .and_then(|mut clipboard| clipboard.get_text().ok())
            .is_some_and(|text| text == clipboard_text(&state.clipboard))
}
fn delete_selection(deck: &mut Deck, state: &mut State) -> bool {
    let mut changed = false;
    for id in state.selected.clone() {
        match deck.delete_element(state.slide_id, id) {
            Ok(()) => changed = true,
            Err(error) => state.error = error,
        }
    }
    state.selected.clear();
    state.edit_text = None;
    changed
}
fn duplicate_selection(deck: &mut Deck, state: &mut State) -> bool {
    let Some(slide) = deck.slide(state.slide_id) else {
        return false;
    };
    let copies = slide
        .elements
        .iter()
        .filter(|element| state.selected.contains(&element.id))
        .cloned()
        .collect::<Vec<_>>();
    let clipboard = std::mem::replace(&mut state.clipboard, copies.into());
    let changed = paste_selection(deck, state);
    state.clipboard = clipboard;
    changed
}

fn table_columns(cells: &[Vec<String>]) -> usize {
    cells.iter().map(Vec::len).max().unwrap_or(0)
}
fn add_table_row(cells: &mut Vec<Vec<String>>) -> bool {
    if cells.len() >= 1000 || (cells.len() + 1).saturating_mul(table_columns(cells).max(1)) > 100000
    {
        return false;
    }
    cells.push(vec![String::new(); table_columns(cells).max(1)]);
    true
}
fn add_table_column(cells: &mut Vec<Vec<String>>) -> bool {
    let columns = table_columns(cells);
    if columns >= 1000 || cells.len().max(1).saturating_mul(columns + 1) > 100000 {
        return false;
    }
    if cells.is_empty() {
        cells.push(Vec::new());
    }
    for row in cells {
        row.resize_with(columns + 1, String::new);
    }
    true
}
fn delete_table_column(cells: &mut [Vec<String>]) -> bool {
    let columns = table_columns(cells);
    if columns <= 1 {
        return false;
    }
    for row in cells {
        if row.len() == columns {
            row.pop();
        }
    }
    true
}

fn effect_label(effect: &str) -> &str {
    match effect {
        "none" => "Без эффекта",
        "fade" => "Плавное появление",
        "push" => "Сдвиг",
        "wipe" => "Открывание",
        "fly_in" => "Вылет",
        "zoom" => "Масштаб",
        _ => effect,
    }
}
fn trigger_label(trigger: &str) -> &str {
    match trigger {
        "on_click" => "По щелчку",
        "after_previous" => "После предыдущего",
        "with_previous" => "С предыдущим",
        _ => trigger,
    }
}

fn properties(ui: &mut egui::Ui, e: &mut Element) -> bool {
    let mut changed = false;
    ui.strong(match e.kind {
        ElementKind::Text => "Текстовое поле",
        ElementKind::Rect => "Прямоугольник",
        ElementKind::Ellipse => "Эллипс",
        ElementKind::Line => "Линия",
        ElementKind::Arrow => "Стрелка",
        ElementKind::Image { .. } => "Изображение",
        ElementKind::Table { .. } => "Таблица",
        ElementKind::Chart { .. } => "Диаграмма",
    });
    ui.horizontal(|ui| {
        ui.label("X");
        changed |= ui.add(egui::DragValue::new(&mut e.x).speed(1.0)).changed();
        ui.label("Y");
        changed |= ui.add(egui::DragValue::new(&mut e.y).speed(1.0)).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Ш");
        changed |= ui
            .add(egui::DragValue::new(&mut e.w).range(1.0..=10000.0))
            .changed();
        ui.label("В");
        changed |= ui
            .add(egui::DragValue::new(&mut e.h).range(1.0..=10000.0))
            .changed();
    });
    let rotation_supported = matches!(
        e.kind,
        ElementKind::Text
            | ElementKind::Rect
            | ElementKind::Ellipse
            | ElementKind::Line
            | ElementKind::Arrow
            | ElementKind::Image { .. }
    );
    changed |= ui
        .add_enabled(
            rotation_supported,
            egui::Slider::new(&mut e.rotation, -180.0..=180.0).text("Поворот"),
        )
        .changed();
    if !rotation_supported {
        ui.small("Поворот этого типа объекта на холсте пока не поддерживается");
    }
    ui.horizontal(|ui| {
        ui.label("Заливка");
        changed |= ui.color_edit_button_srgb(&mut e.fill).changed();
        ui.label("Контур");
        changed |= ui.color_edit_button_srgb(&mut e.stroke).changed();
    });
    changed |= ui
        .add(
            egui::DragValue::new(&mut e.stroke_width)
                .range(0.0..=30.0)
                .prefix("Контур "),
        )
        .changed();
    changed |= ui
        .add(
            egui::DragValue::new(&mut e.font_size)
                .range(1.0..=300.0)
                .prefix("Шрифт "),
        )
        .changed();
    ui.horizontal(|ui| {
        changed |= ui.checkbox(&mut e.bold, "B").changed();
        changed |= ui.checkbox(&mut e.italic, "I").changed();
        changed |= ui.color_edit_button_srgb(&mut e.color).changed();
    });
    ui.label(egui::RichText::new("Текст объекта").color(crate::chrome::muted(ui)));
    changed |= ui
        .add(
            egui::TextEdit::multiline(&mut e.text)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        )
        .changed();
    match &mut e.kind {
        ElementKind::Table { cells } => {
            ui.label("Ячейки таблицы");
            egui::ScrollArea::both()
                .id_salt(("slide_table_cells", e.id))
                .max_height(210.0)
                .show_rows(ui, 26.0, cells.len(), |ui, rows| {
                    for ri in rows {
                        let Some(row) = cells.get_mut(ri) else {
                            continue;
                        };
                        ui.push_id(ri, |ui| {
                            ui.horizontal(|ui| {
                                for (ci, value) in row.iter_mut().enumerate() {
                                    ui.push_id(ci, |ui| {
                                        changed |= ui
                                            .add(
                                                egui::TextEdit::singleline(value)
                                                    .desired_width(65.0),
                                            )
                                            .on_hover_text(format!(
                                                "Строка {}, столбец {}",
                                                ri + 1,
                                                ci + 1
                                            ))
                                            .changed();
                                    });
                                }
                            });
                        });
                    }
                });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        cells.len() < 1000
                            && (cells.len() + 1).saturating_mul(table_columns(cells).max(1))
                                <= 100000,
                        egui::Button::new("+ строка"),
                    )
                    .clicked()
                {
                    changed |= add_table_row(cells);
                }
                if ui
                    .add_enabled(
                        table_columns(cells) < 1000
                            && cells.len().max(1).saturating_mul(table_columns(cells) + 1)
                                <= 100000,
                        egui::Button::new("+ столбец"),
                    )
                    .clicked()
                {
                    changed |= add_table_column(cells);
                }
                if ui
                    .add_enabled(cells.len() > 1, egui::Button::new("− строка"))
                    .on_hover_text("Удалить последнюю строку")
                    .clicked()
                {
                    cells.pop();
                    changed = true;
                }
                if ui
                    .add_enabled(table_columns(cells) > 1, egui::Button::new("− столбец"))
                    .on_hover_text("Удалить последний столбец")
                    .clicked()
                {
                    changed |= delete_table_column(cells);
                }
            });
        }
        ElementKind::Chart { values, labels } => {
            ui.label("Данные диаграммы");
            egui::ScrollArea::vertical()
                .id_salt(("slide_chart_values", e.id))
                .max_height(210.0)
                .show_rows(ui, 26.0, values.len(), |ui, rows| {
                    for index in rows {
                        let Some(value) = values.get_mut(index) else {
                            continue;
                        };
                        ui.push_id(index, |ui| {
                            ui.horizontal(|ui| {
                                let mut label = labels
                                    .get(index)
                                    .cloned()
                                    .unwrap_or_else(|| (index + 1).to_string());
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut label).desired_width(100.0),
                                    )
                                    .changed()
                                {
                                    if labels.len() <= index {
                                        labels.resize_with(index + 1, String::new);
                                    }
                                    if let Some(stored) = labels.get_mut(index) {
                                        *stored = label;
                                        changed = true;
                                    }
                                }
                                changed |= ui
                                    .add(egui::DragValue::new(value).range(-1.0e20..=1.0e20))
                                    .changed();
                            });
                        });
                    }
                });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(values.len() < 10000, egui::Button::new("+ категория"))
                    .clicked()
                {
                    let index = values.len();
                    values.push(0.0);
                    if labels.len() <= index {
                        labels.resize_with(index + 1, String::new);
                        if let Some(label) = labels.get_mut(index) {
                            *label = (index + 1).to_string();
                        }
                    }
                    changed = true;
                }
                if ui
                    .add_enabled(!values.is_empty(), egui::Button::new("− категория"))
                    .on_hover_text("Удалить последнюю категорию")
                    .clicked()
                {
                    let removed = values.len() - 1;
                    values.pop();
                    if removed < labels.len() {
                        labels.remove(removed);
                    }
                    changed = true;
                }
            });
        }
        _ => {}
    }
    ui.separator();
    ui.label("Анимация при показе");
    egui::ComboBox::from_id_salt("animation_effect")
        .selected_text(effect_label(&e.animation.effect))
        .show_ui(ui, |ui| {
            for effect in ["none", "fade", "fly_in", "zoom"] {
                changed |= ui
                    .selectable_value(&mut e.animation.effect, effect.into(), effect_label(effect))
                    .changed();
            }
        });
    egui::ComboBox::from_id_salt("animation_trigger")
        .selected_text(trigger_label(&e.animation.trigger))
        .show_ui(ui, |ui| {
            for trigger in ["with_previous", "after_previous", "on_click"] {
                changed |= ui
                    .selectable_value(
                        &mut e.animation.trigger,
                        trigger.into(),
                        trigger_label(trigger),
                    )
                    .changed();
            }
        });
    changed |= ui
        .add(
            egui::DragValue::new(&mut e.animation.duration)
                .range(0.05..=30.0)
                .prefix("Длительность "),
        )
        .changed();
    changed |= ui
        .add(
            egui::DragValue::new(&mut e.animation.delay)
                .range(0.0..=60.0)
                .prefix("Задержка "),
        )
        .changed();
    changed
}
fn begin_edit(state: &mut State, id: u64, cell: Option<(usize, usize)>) {
    state.edit_text = Some(id);
    state.edit_cell = cell;
    state.edit_focus = true;
    if !state.selected.contains(&id) {
        state.selected = vec![id];
    }
}
/// Table cell under `pos` for a table element drawn in `object`.
fn table_cell_at(element: &Element, object: Rect, pos: Pos2) -> Option<(usize, usize)> {
    let ElementKind::Table { cells } = &element.kind else {
        return None;
    };
    let columns = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let rows = cells.len().max(1);
    let x = ((pos.x - object.left()) / object.width() * columns as f32).floor();
    let y = ((pos.y - object.top()) / object.height() * rows as f32).floor();
    Some((
        (y.max(0.0) as usize).min(rows - 1),
        (x.max(0.0) as usize).min(columns - 1),
    ))
}
/// Edits the text of a slide object, or one table cell, directly on the canvas.
fn inline_editor(
    ui: &mut egui::Ui,
    origin: Pos2,
    scale: f32,
    e: &mut Element,
    state: &mut State,
) -> bool {
    let object = Rect::from_min_size(
        origin + Vec2::new(e.x, e.y) * scale,
        Vec2::new(e.w, e.h) * scale,
    );
    let family = crate::fonts::egui_family(ui.ctx(), &e.font, e.bold, e.italic);
    let color = rgb(e.color);
    let font_size = e.font_size;
    let editor_id = ui.id().with(("slide_inline", e.id));
    let mut changed = false;
    let focus_requested = std::mem::take(&mut state.edit_focus);
    let response = if let ElementKind::Table { cells } = &mut e.kind {
        if cells.is_empty() {
            cells.push(vec![String::new()]);
            changed = true;
        }
        let columns = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let rows = cells.len();
        let (r, c) = state.edit_cell.unwrap_or((0, 0));
        let (r, c) = (r.min(rows - 1), c.min(columns - 1));
        state.edit_cell = Some((r, c));
        let cw = object.width() / columns as f32;
        let ch = object.height() / rows as f32;
        let cell = Rect::from_min_size(
            object.min + Vec2::new(c as f32 * cw, r as f32 * ch),
            Vec2::new(cw, ch),
        );
        ui.painter().rect_stroke(
            cell,
            0.0,
            Stroke::new(2.0, crate::chrome::accent(ui)),
            StrokeKind::Inside,
        );
        let Some(row) = cells.get_mut(r) else {
            return changed;
        };
        if row.len() < columns {
            row.resize(columns, String::new());
            changed = true;
        }
        let Some(value) = row.get_mut(c) else {
            return changed;
        };
        let size = font_size.min(ch / scale * 0.7) * scale;
        let response = ui.put(
            Rect::from_min_size(
                cell.min + Vec2::splat(5.0 * scale),
                Vec2::new((cw - 10.0 * scale).max(8.0), size * 1.3),
            ),
            egui::TextEdit::singleline(value)
                .id(editor_id.with((r, c)))
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(FontId::new(size.max(4.0), family))
                .text_color(color)
                .desired_width(f32::INFINITY),
        );
        let (tab, shift, enter, up, down) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Tab),
                i.modifiers.shift,
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        let moving = response.has_focus() && (up || down);
        if moving || (response.lost_focus() && (tab || enter)) {
            let index = r * columns + c;
            let target = if tab && shift {
                index.checked_sub(1)
            } else if tab {
                if index + 1 == rows * columns && add_table_row(cells) {
                    changed = true;
                }
                Some(index + 1).filter(|i| *i < cells.len() * columns)
            } else if up {
                index.checked_sub(columns)
            } else {
                Some(index + columns).filter(|i| *i < rows * columns)
            };
            match target {
                Some(target) => {
                    state.edit_cell = Some((target / columns, target % columns));
                    state.edit_focus = true;
                }
                None if enter => state.edit_text = None,
                None => state.edit_focus = true,
            }
        } else if response.lost_focus() && !focus_requested {
            state.edit_text = None;
        }
        response
    } else {
        let response = ui.put(
            object.shrink(5.0 * scale),
            egui::TextEdit::multiline(&mut e.text)
                .id(editor_id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(FontId::new((font_size * scale).max(4.0), family))
                .text_color(color)
                .desired_width(f32::INFINITY),
        );
        if response.lost_focus() && !focus_requested {
            state.edit_text = None;
        }
        response
    };
    if focus_requested {
        response.request_focus();
    }
    changed | response.changed()
}
fn select_object(slide: &Slide, state: &mut State, id: u64, additive: bool, clicked: bool) {
    let Some(element) = slide.element(id) else {
        return;
    };
    let members = slide
        .elements
        .iter()
        .filter(|candidate| {
            candidate.id == id
                || element
                    .group
                    .is_some_and(|group| candidate.group == Some(group))
        })
        .map(|element| element.id)
        .collect::<Vec<_>>();
    if additive && clicked && state.selected.contains(&id) {
        state
            .selected
            .retain(|selected| !members.contains(selected));
        state.edit_text = None;
        return;
    }
    if !additive && !state.selected.contains(&id) {
        state.selected.clear();
    }
    state.selected.extend(members);
    state.selected.sort_unstable();
    state.selected.dedup();
}
fn snapped_drag_delta(original: &[(u64, f32, f32, f32, f32)], delta: Vec2, grid: bool) -> Vec2 {
    if grid && let Some((_, x, y, _, _)) = original.first() {
        Vec2::new(
            ((x + delta.x) / 24.0).round() * 24.0 - x,
            ((y + delta.y) / 24.0).round() * 24.0 - y,
        )
    } else {
        delta
    }
}
fn canvas(
    ui: &mut egui::Ui,
    slide: &mut Slide,
    width: f32,
    height: f32,
    scale: f32,
    state: &mut State,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width * scale, height * scale),
        Sense::click_and_drag(),
    );
    state.canvas_rect = Some(rect);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, rgb(slide.background));
    painter.rect_stroke(rect, 0, ui.visuals().window_stroke, StrokeKind::Inside);
    let mut changed = false;
    if state.grid {
        for x in (0..width as usize).step_by(24) {
            let x = rect.left() + x as f32 * scale;
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(0.5, Color32::from_gray(205)),
            );
        }
        for y in (0..height as usize).step_by(24) {
            let y = rect.top() + y as f32 * scale;
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(0.5, Color32::from_gray(205)),
            );
        }
    }
    for element in &slide.elements {
        if ui.is_enabled() && state.edit_text == Some(element.id) {
            // The in-place editor draws this text itself.
            let mut hidden = element.clone();
            match (&mut hidden.kind, state.edit_cell) {
                (ElementKind::Table { cells }, Some((r, c))) => {
                    if let Some(value) = cells.get_mut(r).and_then(|row| row.get_mut(c)) {
                        value.clear();
                    }
                }
                (ElementKind::Table { .. }, None) => {}
                _ => hidden.text.clear(),
            }
            paint_element(ui.ctx(), &painter, rect.min, scale, &hidden, 1.0);
        } else {
            paint_element(ui.ctx(), &painter, rect.min, scale, element, 1.0);
        }
    }
    if response.clicked() {
        state.selected.clear();
        state.edit_text = None;
        response.request_focus();
    }
    for index in 0..slide.elements.len() {
        let Some(element) = slide.elements.get(index) else {
            continue;
        };
        let object = Rect::from_min_size(
            rect.min + Vec2::new(element.x, element.y) * scale,
            Vec2::new(element.w, element.h) * scale,
        );
        let hit = ui.interact(
            object,
            ui.id().with(("slide_object", element.id)),
            Sense::click_and_drag(),
        );
        let was_selected = state.selected.contains(&element.id);
        if hit.clicked() || hit.drag_started() {
            response.request_focus();
            select_object(
                slide,
                state,
                element.id,
                ui.input(|i| i.modifiers.ctrl || i.modifiers.shift),
                hit.clicked(),
            );
        }
        if hit.clicked() && state.edit_text.is_some_and(|id| id != element.id) {
            state.edit_text = None;
        }
        // Like PowerPoint: a click on an already selected text object or table places the caret.
        let reclick = hit.clicked()
            && was_selected
            && !ui.input(|i| i.modifiers.ctrl || i.modifiers.shift)
            && matches!(
                element.kind,
                ElementKind::Text
                    | ElementKind::Rect
                    | ElementKind::Ellipse
                    | ElementKind::Table { .. }
            );
        if (hit.double_clicked() && !ui.input(|i| i.modifiers.ctrl || i.modifiers.shift)) || reclick
        {
            let cell = hit
                .interact_pointer_pos()
                .and_then(|pos| table_cell_at(element, object, pos));
            begin_edit(state, element.id, cell);
        }
        if hit.drag_started() {
            state.drag_original = slide
                .elements
                .iter()
                .filter(|e| state.selected.contains(&e.id))
                .map(|e| (e.id, e.x, e.y, e.w, e.h))
                .collect();
        }
        if hit.dragged() {
            let delta = snapped_drag_delta(
                &state.drag_original,
                hit.total_drag_delta().unwrap_or(Vec2::ZERO) / scale,
                state.grid,
            );
            for (id, x, y, _, _) in &state.drag_original {
                if let Some(e) = slide.element_mut(*id) {
                    e.x = x + delta.x;
                    e.y = y + delta.y;
                    changed = true;
                }
            }
            break;
        }
    }
    for id in state.selected.clone() {
        let Some(element) = slide.element_mut(id) else {
            continue;
        };
        let object = Rect::from_min_size(
            rect.min + Vec2::new(element.x, element.y) * scale,
            Vec2::new(element.w, element.h) * scale,
        );
        painter.rect_stroke(
            object,
            0.0,
            Stroke::new(1.5, crate::chrome::accent(ui)),
            StrokeKind::Outside,
        );
        for (handle, (hx, hy)) in [
            (0.0, 0.0),
            (0.5, 0.0),
            (1.0, 0.0),
            (1.0, 0.5),
            (1.0, 1.0),
            (0.5, 1.0),
            (0.0, 1.0),
            (0.0, 0.5),
        ]
        .iter()
        .enumerate()
        {
            let center = Pos2::new(
                object.left() + object.width() * hx,
                object.top() + object.height() * hy,
            );
            let handle_rect = Rect::from_center_size(center, Vec2::splat(8.0));
            painter.rect_filled(handle_rect, 1.0, Color32::WHITE);
            painter.rect_stroke(
                handle_rect,
                1.0,
                Stroke::new(1.0, crate::chrome::accent(ui)),
                StrokeKind::Inside,
            );
            let response = ui.interact(
                handle_rect,
                ui.id().with(("resize", id, handle)),
                Sense::drag(),
            );
            if response.drag_started() {
                state.drag_original = vec![(id, element.x, element.y, element.w, element.h)];
            }
            if response.dragged()
                && let Some((_, x, y, w, h)) = state.drag_original.first().copied()
            {
                let delta = response.total_drag_delta().unwrap_or(Vec2::ZERO) / scale;
                let (x, y, w, h) = resize_geometry(
                    (x, y, w, h),
                    (*hx, *hy),
                    delta,
                    ui.input(|i| i.modifiers.shift),
                );
                element.x = x;
                element.y = y;
                element.w = w;
                element.h = h;
                changed = true;
            }
        }
    }
    if ui.is_enabled()
        && let Some(id) = state.edit_text
        && let Some(element) = slide.element_mut(id)
    {
        changed |= inline_editor(ui, rect.min, scale, element, state);
    }
    if response.has_focus() {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
    }
    if ui.is_enabled()
        && response.has_focus()
        && !ui.ctx().text_edit_focused()
        && !egui::Popup::is_any_open(ui.ctx())
        && state.edit_text.is_none()
    {
        let delete = ui.input(|i| i.key_pressed(egui::Key::Delete));
        let step = if ui.input(|i| i.modifiers.shift) {
            10.0
        } else {
            1.0
        };
        let delta = ui.input(|i| {
            Vec2::new(
                if i.key_pressed(egui::Key::ArrowLeft) {
                    -step
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    step
                } else {
                    0.0
                },
                if i.key_pressed(egui::Key::ArrowUp) {
                    -step
                } else if i.key_pressed(egui::Key::ArrowDown) {
                    step
                } else {
                    0.0
                },
            )
        });
        if delete {
            slide.elements.retain(|e| !state.selected.contains(&e.id));
            state.selected.clear();
            changed = true;
        }
        if delta != Vec2::ZERO {
            for e in &mut slide.elements {
                if state.selected.contains(&e.id) {
                    e.x += delta.x;
                    e.y += delta.y;
                    changed = true;
                }
            }
        }
    }
    changed
}
fn resize_geometry(
    original: (f32, f32, f32, f32),
    handle: (f32, f32),
    delta: Vec2,
    preserve_aspect: bool,
) -> (f32, f32, f32, f32) {
    let (x, y, width, height) = original;
    let (hx, hy) = handle;
    let horizontal = hx != 0.5;
    let vertical = hy != 0.5;
    let mut new_width = if horizontal {
        (width + if hx == 0.0 { -delta.x } else { delta.x }).clamp(4.0, 10000.0)
    } else {
        width
    };
    let mut new_height = if vertical {
        (height + if hy == 0.0 { -delta.y } else { delta.y }).clamp(4.0, 10000.0)
    } else {
        height
    };
    if preserve_aspect {
        let ratio = width / height;
        if horizontal
            && (!vertical
                || (new_width - width).abs() / width >= (new_height - height).abs() / height)
        {
            new_height = new_width / ratio;
        } else {
            new_width = new_height * ratio;
        }
        let factor = (10000.0 / new_width).min(10000.0 / new_height).min(1.0);
        new_width *= factor;
        new_height *= factor;
    }
    let left = if hx == 0.0 {
        x + width - new_width
    } else if hx == 0.5 {
        x + (width - new_width) / 2.0
    } else {
        x
    };
    let top = if hy == 0.0 {
        y + height - new_height
    } else if hy == 0.5 {
        y + (height - new_height) / 2.0
    } else {
        y
    };
    (left, top, new_width, new_height)
}
fn align(slide: &mut Slide, ids: &[u64], axis: &str, width: f32, height: f32) {
    let mut selected = slide
        .elements
        .iter()
        .filter(|e| ids.contains(&e.id))
        .map(|e| (e.id, e.x, e.y, e.w, e.h))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return;
    }
    if axis == "horizontal" || axis == "vertical" {
        let horizontal = axis == "horizontal";
        selected.sort_by(|a, b| {
            if horizontal {
                a.1.total_cmp(&b.1)
            } else {
                a.2.total_cmp(&b.2)
            }
        });
        let first = selected.first().copied();
        let last = selected.last().copied();
        if let (Some(first), Some(last)) = (first, last) {
            let total = selected
                .iter()
                .map(|s| if horizontal { s.3 } else { s.4 })
                .sum::<f32>();
            let start = if horizontal { first.1 } else { first.2 };
            let end = if horizontal {
                last.1 + last.3
            } else {
                last.2 + last.4
            };
            let gap = (end - start - total) / (selected.len().saturating_sub(1).max(1)) as f32;
            let mut pos = start;
            for (id, _, _, w, h) in selected {
                if let Some(e) = slide.element_mut(id) {
                    if horizontal {
                        e.x = pos;
                        pos += w + gap;
                    } else {
                        e.y = pos;
                        pos += h + gap;
                    }
                }
            }
        }
    } else {
        for id in ids {
            if let Some(e) = slide.element_mut(*id) {
                match axis {
                    "left" => e.x = 0.0,
                    "center" => e.x = (width - e.w) / 2.0,
                    "right" => e.x = width - e.w,
                    "top" => e.y = 0.0,
                    "middle" => e.y = (height - e.h) / 2.0,
                    "bottom" => e.y = height - e.h,
                    _ => {}
                }
            }
        }
    }
}
pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
pub fn paint_slide(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    deck: &Deck,
    slide: &Slide,
    animation: Option<(f32, &[f32])>,
) {
    let painter = painter.with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, rgb(slide.background));
    let scale = (rect.width() / deck.width).min(rect.height() / deck.height);
    let mut previous_end = 0.0;
    let mut previous_start = 0.0;
    let mut click_count = 0;
    for element in &slide.elements {
        let mut animated;
        let mut rendered = element;
        let mut opacity = 1.0;
        if let Some((time, click_times)) = animation
            && element.animation.effect != "none"
        {
            let start = match element.animation.trigger.as_str() {
                "on_click" => {
                    let start = click_times
                        .get(click_count)
                        .copied()
                        .unwrap_or(f32::INFINITY)
                        + element.animation.delay;
                    click_count += 1;
                    start
                }
                "after_previous" => previous_end + element.animation.delay,
                _ => previous_start + element.animation.delay,
            };
            let progress = ((time - start) / element.animation.duration.max(0.05)).clamp(0.0, 1.0);
            previous_end = start + element.animation.duration;
            previous_start = start;
            let ease = 1.0 - (1.0 - progress).powi(3);
            match element.animation.effect.as_str() {
                "fade" => opacity = ease,
                "fly_in" | "zoom" => {
                    animated = element.clone();
                    if element.animation.effect == "fly_in" {
                        animated.x += (1.0 - ease) * deck.width;
                    } else {
                        let factor = 0.05 + 0.95 * ease;
                        animated.x += element.w * (1.0 - factor) / 2.0;
                        animated.y += element.h * (1.0 - factor) / 2.0;
                        animated.w *= factor;
                        animated.h *= factor;
                    }
                    rendered = &animated;
                    opacity = ease;
                }
                _ => {}
            }
        }
        paint_element(ctx, &painter, rect.min, scale, rendered, opacity);
    }
}
fn paint_element(
    ctx: &egui::Context,
    painter: &egui::Painter,
    origin: Pos2,
    scale: f32,
    e: &Element,
    opacity: f32,
) {
    if opacity <= 0.0 {
        return;
    }
    let mut painter = painter.clone();
    painter.multiply_opacity(opacity);
    let rect = Rect::from_min_size(
        origin + Vec2::new(e.x, e.y) * scale,
        Vec2::new(e.w, e.h) * scale,
    );
    let stroke = Stroke::new(e.stroke_width * scale, rgb(e.stroke));
    match &e.kind {
        ElementKind::Rect => {
            let mut shape =
                egui::epaint::RectShape::new(rect, 0.0, rgb(e.fill), stroke, StrokeKind::Inside);
            shape.angle = e.rotation.to_radians();
            painter.add(shape);
        }
        ElementKind::Ellipse => {
            let mut shape =
                egui::epaint::EllipseShape::filled(rect.center(), rect.size() / 2.0, rgb(e.fill));
            shape.stroke = stroke;
            shape.angle = e.rotation.to_radians();
            painter.add(shape);
        }
        ElementKind::Line | ElementKind::Arrow => {
            let [x1, y1, x2, y2] = super::model::line_points(e);
            let start = origin + Vec2::new(x1, y1) * scale;
            let end = origin + Vec2::new(x2, y2) * scale;
            if matches!(e.kind, ElementKind::Arrow) {
                painter.arrow(start, end - start, stroke);
            } else {
                painter.line_segment([start, end], stroke);
            }
        }
        ElementKind::Image { bytes } => {
            let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |h, b| {
                (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
            });
            let key = egui::Id::new(("slide_image", hash));
            let texture = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(key));
            let texture = texture.or_else(|| {
                let image = crate::util::decode_image(bytes).ok()?.to_rgba8();
                if image.width() > 8192 || image.height() > 8192 {
                    return None;
                }
                let size = [image.width() as usize, image.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                let handle = ctx.load_texture(
                    format!("slide_image_{}", e.id),
                    color,
                    egui::TextureOptions::LINEAR,
                );
                ctx.data_mut(|d| d.insert_temp(key, handle.clone()));
                Some(handle)
            });
            if let Some(texture) = texture {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                mesh.add_rect_with_uv(
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                if e.rotation != 0.0 {
                    let turn = egui::emath::Rot2::from_angle(e.rotation.to_radians());
                    let center = rect.center();
                    for vertex in &mut mesh.vertices {
                        vertex.pos = center + turn * (vertex.pos - center);
                    }
                }
                painter.add(mesh);
            } else {
                painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Ошибка изображения",
                    FontId::proportional(14.0),
                    Color32::RED,
                );
            }
        }
        ElementKind::Table { cells } => {
            let columns = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let rows = cells.len().max(1);
            let cw = rect.width() / columns as f32;
            let ch = rect.height() / rows as f32;
            for (ri, row) in cells.iter().enumerate() {
                for (ci, value) in row.iter().enumerate() {
                    let cell = Rect::from_min_size(
                        rect.min + Vec2::new(ci as f32 * cw, ri as f32 * ch),
                        Vec2::new(cw, ch),
                    );
                    painter.rect_filled(
                        cell,
                        0.0,
                        if ri == 0 {
                            rgb(e.fill)
                        } else {
                            Color32::from_rgb(245, 247, 250)
                        },
                    );
                    painter.rect_stroke(cell, 0.0, stroke, StrokeKind::Inside);
                    let cell_element = Element {
                        text: value.clone(),
                        w: cw / scale,
                        h: ch / scale,
                        font_size: e.font_size.min(ch / scale * 0.7),
                        bold: e.bold,
                        italic: e.italic,
                        font: e.font.clone(),
                        ..Element::default()
                    };
                    let font = FontId::new(
                        cell_element.font_size * scale,
                        crate::fonts::egui_family(painter.ctx(), &e.font, e.bold, e.italic),
                    );
                    let cell_painter = painter.with_clip_rect(cell.shrink(2.0 * scale));
                    for (index, line) in super::model::text_lines(&cell_element)
                        .into_iter()
                        .enumerate()
                    {
                        cell_painter.text(
                            cell.left_top()
                                + Vec2::new(5.0, 5.0 + index as f32 * cell_element.font_size * 1.2)
                                    * scale,
                            Align2::LEFT_TOP,
                            line,
                            font.clone(),
                            rgb(e.color),
                        );
                    }
                }
            }
        }
        ElementKind::Chart { values, labels } => {
            let (min, max) = super::model::chart_bounds(values);
            let range = (max - min).max(f32::EPSILON);
            let plot_height = (rect.height() - 30.0 * scale).max(1.0);
            let cw = rect.width() / values.len().max(1) as f32;
            let baseline = rect.top() + plot_height * max / range;
            painter.line_segment(
                [
                    Pos2::new(rect.left(), baseline),
                    Pos2::new(rect.right(), baseline),
                ],
                stroke,
            );
            for (i, v) in values.iter().enumerate() {
                let height = v.abs() / range * plot_height;
                let bar = Rect::from_min_size(
                    Pos2::new(
                        rect.left() + i as f32 * cw + 4.0 * scale,
                        if *v >= 0.0 {
                            baseline - height
                        } else {
                            baseline
                        },
                    ),
                    Vec2::new((cw - 8.0 * scale).max(1.0), height),
                );
                painter.rect_filled(bar, 0.0, rgb(e.fill));
                painter.text(
                    Pos2::new(bar.center().x, rect.bottom() - 24.0 * scale),
                    Align2::CENTER_TOP,
                    labels
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| (i + 1).to_string()),
                    FontId::proportional(14.0 * scale),
                    rgb(e.color),
                );
            }
        }
        ElementKind::Text => {}
    }
    if !e.text.is_empty() {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        job.append(
            &super::model::text_lines(e).join("\n"),
            0.0,
            egui::TextFormat {
                font_id: FontId::new(
                    e.font_size * scale,
                    crate::fonts::egui_family(painter.ctx(), &e.font, e.bold, e.italic),
                ),
                color: rgb(e.color),
                italics: false,
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        let angle = e.rotation.to_radians();
        let pos = rect.center()
            + egui::emath::Rot2::from_angle(angle)
                * (rect.min + Vec2::splat(5.0 * scale) - rect.center());
        let shape = egui::epaint::TextShape::new(pos, galley, rgb(e.color)).with_angle(angle);
        painter.add(shape);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn table_navigation_focuses_the_destination_cell() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut element = Element {
            id: 1,
            kind: ElementKind::Table {
                cells: vec![vec!["a".into()], vec!["b".into()]],
            },
            ..Element::default()
        };
        let mut state = State::default();
        begin_edit(&mut state, 1, Some((0, 0)));
        {
            let mut frame = |events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        inline_editor(ui, Pos2::ZERO, 1.0, &mut element, &mut state);
                    },
                );
                output.textures_delta.clear();
            };
            frame(vec![]);
            frame(vec![input_key(egui::Key::ArrowDown, egui::Modifiers::NONE)]);
        }
        assert_eq!(state.edit_cell, Some((1, 0)));
        assert!(
            state.edit_focus,
            "destination focus must survive the source field's frame"
        );
        Ok(())
    }
    #[test]
    fn table_row_growth_stops_before_the_model_cell_budget() {
        let mut cells = vec![vec![String::new(); 1000]; 100];
        assert!(!add_table_row(&mut cells));
        assert_eq!(cells.len(), 100);
    }

    use super::*;
    fn input_key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    fn editor_frame(
        ctx: &egui::Context,
        deck: &mut Deck,
        mut events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
        focus_notes: bool,
    ) -> Result<State, String> {
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let mut saved = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0))),
                events,
                ..Default::default()
            },
            |ui| {
                let memory_id = ui.id().with("slides_editor");
                if focus_notes {
                    let mut state = ui
                        .ctx()
                        .data(|data| data.get_temp::<State>(memory_id))
                        .unwrap_or_default();
                    state.show_notes = true;
                    ui.ctx().data_mut(|data| data.insert_temp(memory_id, state));
                    ui.memory_mut(|memory| memory.request_focus(memory_id.with("notes")));
                }
                show(ui, deck);
                saved = ui.ctx().data_mut(|data| data.get_temp::<State>(memory_id));
            },
        );
        output.textures_delta.clear();
        saved.ok_or_else(|| "Нет состояния редактора".into())
    }
    fn clicked_object(ctx: &egui::Context, deck: &mut Deck, id: u64) -> Result<State, String> {
        let state = editor_frame(ctx, deck, Vec::new(), egui::Modifiers::NONE, false)?;
        let element = deck
            .slide(state.slide_id)
            .and_then(|slide| slide.element(id))
            .ok_or("Нет объекта")?;
        let canvas_rect = state.canvas_rect.ok_or("Нет геометрии холста")?;
        assert!(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0)).contains_rect(canvas_rect),
            "Холст должен находиться внутри окна: {canvas_rect:?}"
        );
        let scale = canvas_rect.width() / deck.width;
        let position = canvas_rect.min
            + Vec2::new(element.x + element.w / 2.0, element.y + element.h / 2.0) * scale;
        editor_frame(
            ctx,
            deck,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            egui::Modifiers::NONE,
            false,
        )?;
        let state = editor_frame(
            ctx,
            deck,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            egui::Modifiers::NONE,
            false,
        )?;
        assert!(
            state.selected.contains(&id),
            "Click {:?}; old canvas {:?}, new canvas {:?}; selected {:?}; focused {:?}",
            position,
            canvas_rect,
            state.canvas_rect,
            state.selected,
            ctx.memory(|memory| memory.focused())
        );
        assert!(ctx.egui_wants_keyboard_input());
        assert!(!ctx.text_edit_focused());
        Ok(state)
    }
    #[test]
    fn clicked_canvas_moves_copies_and_deletes_objects() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut deck = Deck::empty();
        let slide_id = deck.add_slide("blank");
        let id = deck.add_element(slide_id, ElementKind::Rect)?;
        let x = deck
            .slide(slide_id)
            .and_then(|slide| slide.element(id))
            .ok_or("Нет объекта")?
            .x;
        clicked_object(&ctx, &mut deck, id)?;
        editor_frame(
            &ctx,
            &mut deck,
            vec![input_key(egui::Key::ArrowRight, egui::Modifiers::NONE)],
            egui::Modifiers::NONE,
            false,
        )?;
        assert_eq!(
            deck.slide(slide_id)
                .and_then(|slide| slide.element(id))
                .map(|e| e.x),
            Some(x + 1.0)
        );
        let state = editor_frame(
            &ctx,
            &mut deck,
            vec![egui::Event::Copy],
            egui::Modifiers::NONE,
            false,
        )?;
        assert_eq!(state.clipboard.len(), 1);
        assert_eq!(state.clipboard.first().map(|element| element.id), Some(id));
        editor_frame(
            &ctx,
            &mut deck,
            vec![input_key(egui::Key::Delete, egui::Modifiers::NONE)],
            egui::Modifiers::NONE,
            false,
        )?;
        assert!(
            deck.slide(slide_id)
                .is_some_and(|slide| slide.elements.is_empty())
        );
        assert!(deck.validate().is_ok());
        Ok(())
    }
    #[test]
    fn notes_focus_keeps_object_shortcuts_out_of_text_editor() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut deck = Deck::empty();
        let slide_id = deck.add_slide("blank");
        let id = deck.add_element(slide_id, ElementKind::Rect)?;
        clicked_object(&ctx, &mut deck, id)?;
        let before = deck
            .slide(slide_id)
            .and_then(|slide| slide.element(id))
            .cloned()
            .ok_or("Нет объекта")?;
        editor_frame(&ctx, &mut deck, Vec::new(), egui::Modifiers::NONE, true)?;
        assert!(ctx.text_edit_focused());
        editor_frame(
            &ctx,
            &mut deck,
            vec![input_key(egui::Key::ArrowRight, egui::Modifiers::NONE)],
            egui::Modifiers::NONE,
            true,
        )?;
        editor_frame(
            &ctx,
            &mut deck,
            vec![input_key(egui::Key::Delete, egui::Modifiers::NONE)],
            egui::Modifiers::NONE,
            true,
        )?;
        let state = editor_frame(
            &ctx,
            &mut deck,
            vec![egui::Event::Copy],
            egui::Modifiers::NONE,
            true,
        )?;
        assert!(state.clipboard.is_empty());
        assert_eq!(
            deck.slide(slide_id).and_then(|slide| slide.element(id)),
            Some(&before)
        );
        Ok(())
    }
    fn scoped_editor_frame(
        ctx: &egui::Context,
        deck: &mut Deck,
        scope: &str,
        events: Vec<egui::Event>,
    ) -> Result<(State, egui::Id), String> {
        let mut saved = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0))),
                events,
                ..Default::default()
            },
            |ui| {
                ui.push_id(scope, |ui| {
                    let memory_id = ui.id().with("slides_editor");
                    show(ui, deck);
                    saved = ui.ctx().data_mut(|data| {
                        data.get_temp::<State>(memory_id)
                            .map(|state| (state, memory_id))
                    });
                });
            },
        );
        output.textures_delta.clear();
        saved.ok_or_else(|| "Нет состояния вкладки".into())
    }
    #[test]
    fn object_clipboard_crosses_tabs_and_external_text_stays_text() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut source = Deck::empty();
        let source_slide = source.add_slide("blank");
        let first = source.add_element(source_slide, ElementKind::Rect)?;
        let second = source.add_element(source_slide, ElementKind::Ellipse)?;
        let source_group = source.allocate_id();
        for id in [first, second] {
            if let Some(element) = source
                .slide_mut(source_slide)
                .and_then(|slide| slide.element_mut(id))
            {
                element.group = Some(source_group);
                element.text = "clipboard".into();
            }
        }
        let (mut source_state, source_memory) =
            scoped_editor_frame(&ctx, &mut source, "tab-a", Vec::new())?;
        source_state.selected = vec![first, second];
        ctx.data_mut(|data| data.insert_temp(source_memory, source_state));
        scoped_editor_frame(&ctx, &mut source, "tab-a", vec![egui::Event::Copy])?;
        let mut destination = Deck::empty();
        let destination_slide = destination.add_slide("blank");
        destination.add_element(destination_slide, ElementKind::Rect)?;
        let floor = destination.next_id;
        let (destination_state, _) = scoped_editor_frame(
            &ctx,
            &mut destination,
            "tab-b",
            vec![egui::Event::Paste("clipboard\nclipboard".into())],
        )?;
        assert_eq!(destination_state.selected.len(), 2);
        let copied: Vec<_> = destination
            .slide(destination_slide)
            .ok_or("Нет слайда")?
            .elements
            .iter()
            .filter(|element| destination_state.selected.contains(&element.id))
            .collect();
        assert_eq!(copied.len(), 2);
        assert!(matches!(
            copied.first().map(|element| &element.kind),
            Some(ElementKind::Rect)
        ));
        assert!(matches!(
            copied.last().map(|element| &element.kind),
            Some(ElementKind::Ellipse)
        ));
        assert!(copied.iter().all(|element| element.id >= floor));
        let group = copied.first().and_then(|element| element.group);
        assert!(group.is_some() && copied.iter().all(|element| element.group == group));
        scoped_editor_frame(
            &ctx,
            &mut destination,
            "tab-b",
            vec![egui::Event::Paste("external plain text".into())],
        )?;
        let external = destination
            .slide(destination_slide)
            .and_then(|slide| slide.elements.last())
            .ok_or("Нет текста")?;
        assert!(matches!(external.kind, ElementKind::Text));
        assert_eq!(external.text, "external plain text");
        assert!(destination.validate().is_ok());
        Ok(())
    }
    #[test]
    fn empty_and_ragged_tables_remain_editable() {
        let mut cells = Vec::new();
        assert!(add_table_column(&mut cells));
        assert_eq!(cells, vec![vec![String::new()]]);
        cells = vec![vec!["A".into()], vec!["B".into(), "C".into()]];
        assert!(add_table_column(&mut cells));
        assert_eq!(
            cells,
            vec![
                vec!["A".into(), String::new(), String::new()],
                vec!["B".into(), "C".into(), String::new()]
            ]
        );
        assert!(delete_table_column(&mut cells));
        assert_eq!(
            cells,
            vec![
                vec!["A".into(), String::new()],
                vec!["B".into(), "C".into()]
            ]
        );
        assert!(add_table_row(&mut cells));
        assert_eq!(cells.last(), Some(&vec![String::new(), String::new()]));
        let mut ragged = vec![vec!["A".into()], vec!["B".into(), "C".into(), "D".into()]];
        assert!(delete_table_column(&mut ragged));
        assert_eq!(
            ragged,
            vec![
                vec![String::from("A")],
                vec![String::from("B"), String::from("C")]
            ]
        );
    }
    #[test]
    fn paste_preserves_groups_with_fresh_ids() {
        let mut deck = Deck::default();
        let slide_id = deck.slides.first().map(|s| s.id).unwrap_or(0);
        let group = deck.allocate_id();
        let mut copied = deck
            .slide(slide_id)
            .map(|slide| slide.elements.clone())
            .unwrap_or_default();
        for element in &mut copied {
            element.group = Some(group);
        }
        let mut state = State {
            slide_id,
            clipboard: copied.clone().into(),
            ..State::default()
        };
        assert!(paste_selection(&mut deck, &mut state));
        let pasted = deck
            .slide(slide_id)
            .map(|slide| {
                slide
                    .elements
                    .iter()
                    .filter(|e| state.selected.contains(&e.id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert_eq!(pasted.len(), copied.len());
        let pasted_group = pasted.first().and_then(|element| element.group);
        assert!(pasted_group.is_some() && pasted_group != Some(group));
        assert!(pasted.iter().all(|element| element.group == pasted_group
            && !copied.iter().any(|old| old.id == element.id)));
        assert!(deck.validate().is_ok());
    }
    #[test]
    fn additive_click_deselects_whole_group_without_deselecting_on_drag() -> Result<(), String> {
        let mut deck = Deck::empty();
        let slide_id = deck.add_slide("blank");
        let first = deck.add_element(slide_id, ElementKind::Rect)?;
        let second = deck.add_element(slide_id, ElementKind::Rect)?;
        let third = deck.add_element(slide_id, ElementKind::Rect)?;
        let group = deck.allocate_id();
        let slide = deck.slide_mut(slide_id).ok_or("slide")?;
        slide.element_mut(first).ok_or("first")?.group = Some(group);
        slide.element_mut(second).ok_or("second")?.group = Some(group);
        let mut state = State::default();
        select_object(slide, &mut state, first, false, true);
        assert_eq!(state.selected, vec![first, second]);
        select_object(slide, &mut state, third, true, true);
        select_object(slide, &mut state, first, true, false);
        assert_eq!(state.selected, vec![first, second, third]);
        select_object(slide, &mut state, first, true, true);
        assert_eq!(state.selected, vec![third]);
        Ok(())
    }
    #[test]
    fn grid_drag_preserves_fractional_spacing_between_group_members() {
        let original = [
            (1, 12.5, 13.25, 100.0, 100.0),
            (2, 43.75, 69.5, 100.0, 100.0),
        ];
        let delta = snapped_drag_delta(&original, Vec2::new(17.0, 20.0), true);
        assert_eq!(
            (original[0].1 + delta.x, original[0].2 + delta.y),
            (24.0, 24.0)
        );
        assert_eq!((original[1].1 + delta.x) - (original[0].1 + delta.x), 31.25);
        assert_eq!((original[1].2 + delta.y) - (original[0].2 + delta.y), 56.25);
        assert_eq!(
            snapped_drag_delta(&original, Vec2::new(0.25, 0.5), false),
            Vec2::new(0.25, 0.5)
        );
    }
    #[test]
    fn external_picture_paste_overrides_stale_copied_shapes() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut deck = Deck::empty();
        let slide = deck.add_slide("blank");
        let shape = deck.add_element(slide, ElementKind::Rect)?;
        clicked_object(&ctx, &mut deck, shape)?;
        editor_frame(
            &ctx,
            &mut deck,
            vec![egui::Event::Copy],
            egui::Modifiers::NONE,
            false,
        )?;
        let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        source
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        crate::clipboard::offer(&ctx, png.into_inner());
        editor_frame(&ctx, &mut deck, Vec::new(), egui::Modifiers::NONE, false)?;
        let elements = &deck.slide(slide).ok_or("slide")?.elements;
        assert_eq!(elements.len(), 2);
        assert!(matches!(elements[1].kind, ElementKind::Image { .. }));
        deck.validate()
    }
    #[test]
    fn duplication_preserves_groups_layer_order_and_clipboard() -> Result<(), String> {
        let mut deck = Deck::default();
        let slide_id = deck.slides.first().map(|slide| slide.id).ok_or("slide")?;
        let group = deck.allocate_id();
        let slide = deck.slide_mut(slide_id).ok_or("slide")?;
        for element in &mut slide.elements {
            element.group = Some(group);
        }
        slide.elements.reverse();
        let originals = slide.elements.clone();
        let clipboard = std::sync::Arc::new(vec![Element {
            text: "Буфер".into(),
            ..Element::default()
        }]);
        let mut state = State {
            slide_id,
            selected: originals.iter().rev().map(|element| element.id).collect(),
            clipboard: clipboard.clone(),
            ..State::default()
        };
        assert!(duplicate_selection(&mut deck, &mut state));
        assert!(std::sync::Arc::ptr_eq(&clipboard, &state.clipboard));
        let copies = &deck.slide(slide_id).ok_or("slide")?.elements[originals.len()..];
        let copy_group = copies
            .first()
            .and_then(|element| element.group)
            .ok_or("group")?;
        assert_ne!(copy_group, group);
        for (copy, original) in copies.iter().zip(&originals) {
            assert_eq!(copy.text, original.text);
            assert_eq!(copy.group, Some(copy_group));
            assert_eq!((copy.x, copy.y), (original.x + 20.0, original.y + 20.0));
        }
        deck.validate()?;
        Ok(())
    }
    #[test]
    fn proportional_side_resize_moves_both_dimensions() {
        let (x, y, w, h) = resize_geometry(
            (10.0, 20.0, 200.0, 100.0),
            (0.5, 1.0),
            Vec2::new(0.0, 50.0),
            true,
        );
        assert_eq!((x, y, w, h), (-40.0, 20.0, 300.0, 150.0));
        let (x, y, w, h) = resize_geometry(
            (10.0, 20.0, 200.0, 100.0),
            (0.5, 0.0),
            Vec2::new(0.0, -50.0),
            true,
        );
        assert_eq!((x, y, w, h), (-40.0, -30.0, 300.0, 150.0));
    }
    #[test]
    fn proportional_corner_resize_preserves_opposite_anchor() {
        let (x, y, w, h) = resize_geometry(
            (10.0, 20.0, 200.0, 100.0),
            (0.0, 0.0),
            Vec2::new(-100.0, 0.0),
            true,
        );
        assert_eq!((x, y, w, h), (-90.0, -30.0, 300.0, 150.0));
    }
    #[test]
    fn every_resize_handle_stays_positive() {
        for handle in [
            (0.0, 0.0),
            (0.5, 0.0),
            (1.0, 0.0),
            (1.0, 0.5),
            (1.0, 1.0),
            (0.5, 1.0),
            (0.0, 1.0),
            (0.0, 0.5),
        ] {
            for preserve in [false, true] {
                let (_, _, width, height) = resize_geometry(
                    (0.0, 0.0, 10.0, 20.0),
                    handle,
                    Vec2::new(50000.0, -50000.0),
                    preserve,
                );
                assert!(width > 0.0 && height > 0.0 && width <= 10000.0 && height <= 10000.0);
                if preserve {
                    assert!((width / height - 0.5).abs() < 1.0e-5);
                }
            }
        }
    }
}
