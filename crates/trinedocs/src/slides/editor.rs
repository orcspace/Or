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
    selected: Vec<u64>,
    clipboard: Vec<Element>,
    drag_original: Vec<(u64, f32, f32, f32, f32)>,
    edit_text: Option<u64>,
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
    if ui.input(|i| i.key_pressed(egui::Key::F5)) {
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
    let image_result = state.image_job.as_ref().and_then(|job| {
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
                match super::model::image_dimensions(&bytes).and_then(|size| {
                    deck.add_element(slide_id, ElementKind::Image { bytes })
                        .map(|id| (id, size))
                }) {
                    Ok((id, (width, height))) => {
                        if let Some(element) = deck
                            .slide_mut(slide_id)
                            .and_then(|slide| slide.element_mut(id))
                        {
                            let factor = (480.0 / width as f32).min(280.0 / height as f32);
                            element.w = width as f32 * factor;
                            element.h = height as f32 * factor;
                        }
                        if state.slide_id == slide_id {
                            state.selected = vec![id];
                        }
                        changed = true;
                    }
                    Err(error) => state.error = error,
                }
            }
            Ok(None) => {}
            Err(error) => state.error = error,
        }
    }
    if state.image_job.is_some() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
    let (deck_width, deck_height) = (deck.width, deck.height);
    ui.horizontal_wrapped(|ui| {
        ui.menu_button("Новый слайд", |ui| {
            for (name, layout) in [
                ("Титульный", "title"),
                ("Заголовок и текст", "content"),
                ("Пустой", "blank"),
            ] {
                if ui.button(name).clicked() {
                    state.slide_id = deck.add_slide(layout);
                    state.selected.clear();
                    changed = true;
                    ui.close();
                }
            }
        });
        if ui.button("Дубликат").clicked() {
            match deck.duplicate_slide(state.slide_id) {
                Ok(id) => {
                    state.slide_id = id;
                    changed = true;
                }
                Err(e) => state.error = e,
            }
        }
        if ui
            .add_enabled(!deck.slides.is_empty(), egui::Button::new("Удалить слайд"))
            .clicked()
        {
            match deck.delete_slide(state.slide_id) {
                Ok(()) => {
                    state.slide_id = deck.slides.first().map(|s| s.id).unwrap_or(0);
                    state.selected.clear();
                    changed = true;
                }
                Err(e) => state.error = e,
            }
        }
        ui.separator();
        ui.menu_button("Вставить", |ui| {
            for (label, kind) in [
                ("Текст", ElementKind::Text),
                ("Прямоугольник", ElementKind::Rect),
                ("Эллипс", ElementKind::Ellipse),
                ("Линия", ElementKind::Line),
                ("Стрелка", ElementKind::Arrow),
                (
                    "Таблица 3×3",
                    ElementKind::Table {
                        cells: vec![vec!["".into(); 3]; 3],
                    },
                ),
                (
                    "Столбчатая диаграмма",
                    ElementKind::Chart {
                        values: vec![30.0, 60.0, 45.0],
                        labels: vec!["A".into(), "B".into(), "C".into()],
                    },
                ),
            ] {
                if ui.button(label).clicked() {
                    match deck.add_element(state.slide_id, kind) {
                        Ok(id) => {
                            state.selected = vec![id];
                            changed = true;
                        }
                        Err(e) => state.error = e,
                    };
                    ui.close();
                }
            }
            if ui
                .add_enabled(
                    state.image_job.is_none(),
                    egui::Button::new("Изображение PNG / JPEG…"),
                )
                .clicked()
            {
                let (sender, receiver) = std::sync::mpsc::channel();
                let ctx = ui.ctx().clone();
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
                ui.close();
            }
        });
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
                        Err(e) => state.error = e,
                    }
                    ui.close();
                }
            }
        });
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
        if ui.button("16:9").clicked() {
            deck.width = 960.0;
            deck.height = 540.0;
            changed = true;
        }
        if ui.button("4:3").clicked() {
            deck.width = 720.0;
            deck.height = 540.0;
            changed = true;
        }
        ui.checkbox(&mut state.grid, "Сетка");
        ui.add(egui::Slider::new(&mut state.zoom, 0.25..=2.0).text("Масштаб"));
        if ui.button("Показ F5").clicked() {
            state.presentation.start(ui.ctx(), deck, 0);
        }
        if ui.button("С текущего").clicked() {
            let index = deck
                .slides
                .iter()
                .position(|s| s.id == state.slide_id)
                .unwrap_or(0);
            state.presentation.start(ui.ctx(), deck, index);
        }
    });
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
    ui.separator();
    let main_height = (ui.available_height() - 135.0).max(180.0);
    let main_width = ui.available_width();
    let center_width = (main_width - 420.0).max(180.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui(Vec2::new(155.0, main_height), |ui| {
            ui.set_width(155.0);
            egui::ScrollArea::vertical()
                .id_salt("slide_thumbnails")
                .max_height(main_height)
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
                        let (rect, response) = ui
                            .allocate_exact_size(Vec2::new(145.0, 100.0), Sense::click_and_drag());
                        response.dnd_set_drag_payload(SlideDrag(slide.id));
                        if response.dnd_hover_payload::<SlideDrag>().is_some() {
                            ui.painter().line_segment(
                                [rect.left_top(), rect.right_top()],
                                Stroke::new(3.0, Color32::from_rgb(230, 106, 45)),
                            );
                        }
                        if let Some(payload) = response.dnd_release_payload::<SlideDrag>() {
                            dropped_slide = Some((payload.0, index));
                        }
                        ui.painter().rect_filled(
                            rect,
                            4.0,
                            if selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().faint_bg_color
                            },
                        );
                        let canvas = Rect::from_min_size(
                            rect.min + Vec2::new(5.0, 5.0),
                            Vec2::new(135.0, 135.0 * deck.height / deck.width),
                        );
                        paint_slide(ui.ctx(), ui.painter(), canvas, deck, slide, None);
                        ui.painter().text(
                            rect.left_bottom() + Vec2::new(5.0, -5.0),
                            Align2::LEFT_BOTTOM,
                            format!(
                                "{}{} {}",
                                index + 1,
                                if slide.hidden { " ◌" } else { "" },
                                slide.title.chars().take(14).collect::<String>()
                            ),
                            FontId::proportional(12.0),
                            ui.visuals().text_color(),
                        );
                        if response.clicked() {
                            state.slide_id = slide.id;
                            state.selected.clear();
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("↑").clicked()
                    && let Some(index) = deck.slides.iter().position(|s| s.id == state.slide_id)
                    && index > 0
                {
                    match deck.move_slide(state.slide_id, index - 1) {
                        Ok(()) => changed = true,
                        Err(e) => state.error = e,
                    }
                }
                if ui.button("↓").clicked()
                    && let Some(index) = deck.slides.iter().position(|s| s.id == state.slide_id)
                    && index + 1 < deck.slides.len()
                {
                    match deck.move_slide(state.slide_id, index + 1) {
                        Ok(()) => changed = true,
                        Err(e) => state.error = e,
                    }
                }
            });
        });
        ui.allocate_ui(Vec2::new(center_width, main_height), |ui| {
            ui.set_width(center_width);
            if let Some(slide) = deck.slide_mut(state.slide_id) {
                let scale = (center_width / deck_width).min((main_height - 20.0) / deck_height)
                    * state.zoom;
                egui::ScrollArea::both()
                    .id_salt("slides_canvas")
                    .max_height(main_height)
                    .show(ui, |ui| {
                        changed |= canvas(ui, slide, deck_width, deck_height, scale, &mut state);
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Добавьте слайд");
                });
            }
        });
        ui.allocate_ui(Vec2::new(245.0, main_height), |ui| {
            ui.set_width(245.0);
            egui::ScrollArea::vertical()
                .id_salt("slides_properties")
                .max_height(main_height)
                .show(ui, |ui| {
                    if let Some(slide) = deck.slide_mut(state.slide_id) {
                        ui.strong("Слайд");
                        changed |= ui.text_edit_singleline(&mut slide.title).changed();
                        changed |= ui.checkbox(&mut slide.hidden, "Скрыть в показе").changed();
                        ui.horizontal(|ui| {
                            ui.label("Фон");
                            changed |= ui.color_edit_button_srgb(&mut slide.background).changed();
                        });
                        ui.label("Раздел");
                        changed |= ui.text_edit_singleline(&mut slide.section).changed();
                        ui.separator();
                        ui.label("Переход");
                        egui::ComboBox::from_id_salt("transition")
                            .selected_text(&slide.transition.effect)
                            .show_ui(ui, |ui| {
                                for effect in ["none", "fade", "push", "wipe"] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut slide.transition.effect,
                                            effect.into(),
                                            effect,
                                        )
                                        .changed();
                                }
                            });
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut slide.transition.duration, 0.1..=3.0)
                                    .text("Сек."),
                            )
                            .changed();
                        let mut auto = slide.transition.advance_after.is_some();
                        if ui.checkbox(&mut auto, "Автопереход").changed() {
                            slide.transition.advance_after = if auto { Some(5.0) } else { None };
                            changed = true;
                        }
                        if let Some(seconds) = &mut slide.transition.advance_after {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(seconds)
                                        .range(0.1..=3600.0)
                                        .suffix(" сек."),
                                )
                                .changed();
                        }
                        ui.separator();
                        if let Some(id) = state.selected.first().copied() {
                            if let Some(element) = slide.element_mut(id) {
                                changed |= properties(ui, element);
                            }
                        } else {
                            ui.label("Выберите объект на слайде");
                        }
                    }
                    if !state.selected.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Копия").clicked() {
                                let selected = state.selected.clone();
                                state.selected.clear();
                                for id in selected {
                                    match deck.duplicate_element(state.slide_id, id) {
                                        Ok(id) => {
                                            state.selected.push(id);
                                            changed = true;
                                        }
                                        Err(e) => state.error = e,
                                    }
                                }
                            }
                            if ui.button("Удалить").clicked() {
                                for id in state.selected.clone() {
                                    match deck.delete_element(state.slide_id, id) {
                                        Ok(()) => changed = true,
                                        Err(e) => state.error = e,
                                    }
                                }
                                state.selected.clear();
                            }
                            if ui.button("Вперёд").clicked() {
                                match deck.set_z_order_many(state.slide_id, &state.selected, true) {
                                    Ok(()) => changed = true,
                                    Err(error) => state.error = error,
                                }
                            }
                            if ui.button("Назад").clicked() {
                                match deck.set_z_order_many(state.slide_id, &state.selected, false)
                                {
                                    Ok(()) => changed = true,
                                    Err(error) => state.error = error,
                                }
                            }
                        });
                        ui.menu_button("Выровнять", |ui| {
                            for (label, axis) in [
                                ("Слева", "left"),
                                ("По центру", "center"),
                                ("Справа", "right"),
                                ("Сверху", "top"),
                                ("Посередине", "middle"),
                                ("Снизу", "bottom"),
                                ("Распределить по горизонтали", "horizontal"),
                                ("Распределить по вертикали", "vertical"),
                            ] {
                                if ui.button(label).clicked() {
                                    if let Some(slide) = deck.slide_mut(state.slide_id) {
                                        align(
                                            slide,
                                            &state.selected,
                                            axis,
                                            deck_width,
                                            deck_height,
                                        );
                                        changed = true;
                                    }
                                    ui.close();
                                }
                            }
                        });
                        if state.selected.len() > 1 && ui.button("Сгруппировать").clicked()
                        {
                            let group = deck.allocate_id();
                            if let Some(slide) = deck.slide_mut(state.slide_id) {
                                for id in &state.selected {
                                    if let Some(e) = slide.element_mut(*id) {
                                        e.group = Some(group);
                                    }
                                }
                                changed = true;
                            }
                        }
                        if ui.button("Разгруппировать").clicked()
                            && let Some(slide) = deck.slide_mut(state.slide_id)
                        {
                            for id in &state.selected {
                                if let Some(e) = slide.element_mut(*id) {
                                    e.group = None;
                                }
                            }
                            changed = true;
                        }
                    }
                });
        });
    });
    if let Some((id, index)) = dropped_slide {
        match deck.move_slide(id, index) {
            Ok(()) => changed = true,
            Err(e) => state.error = e,
        }
    }
    if !ui.ctx().egui_wants_keyboard_input() {
        let key = |key| ui.input(|i| i.modifiers.ctrl && i.key_pressed(key));
        if key(egui::Key::A)
            && let Some(slide) = deck.slide(state.slide_id)
        {
            state.selected = slide.elements.iter().map(|e| e.id).collect();
        }
        if key(egui::Key::C)
            && let Some(slide) = deck.slide(state.slide_id)
        {
            state.clipboard = slide
                .elements
                .iter()
                .filter(|e| state.selected.contains(&e.id))
                .cloned()
                .collect();
            ui.ctx().copy_text(
                state
                    .clipboard
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        if key(egui::Key::V) && !state.clipboard.is_empty() {
            state.selected.clear();
            for mut e in state.clipboard.clone() {
                e.id = deck.allocate_id();
                e.x += 20.0;
                e.y += 20.0;
                e.group = None;
                if let Some(slide) = deck.slide_mut(state.slide_id) {
                    state.selected.push(e.id);
                    slide.elements.push(e);
                    changed = true;
                }
            }
        }
        if key(egui::Key::D) {
            let selected = state.selected.clone();
            state.selected.clear();
            for id in selected {
                match deck.duplicate_element(state.slide_id, id) {
                    Ok(id) => {
                        state.selected.push(id);
                        changed = true;
                    }
                    Err(e) => state.error = e,
                }
            }
        }
    }
    ui.separator();
    if let Some(slide) = deck.slide_mut(state.slide_id) {
        ui.label("Заметки докладчика");
        changed |= ui
            .add_sized(
                Vec2::new(ui.available_width(), 85.0),
                egui::TextEdit::multiline(&mut slide.notes).desired_rows(3),
            )
            .changed();
    }
    if let Some(id) = state.edit_text {
        let mut open = true;
        egui::Window::new("Текст объекта")
            .id(memory_id.with("text"))
            .open(&mut open)
            .default_width(500.0)
            .show(ui.ctx(), |ui| {
                if let Some(element) = deck
                    .slide_mut(state.slide_id)
                    .and_then(|s| s.element_mut(id))
                {
                    changed |= ui
                        .add(
                            egui::TextEdit::multiline(&mut element.text)
                                .desired_width(480.0)
                                .desired_rows(8),
                        )
                        .changed();
                }
            });
        if !open {
            state.edit_text = None;
        }
    }
    record_presentation(ui.ctx(), memory_id, state.presentation.active);
    ui.ctx().data_mut(|d| d.insert_temp(memory_id, state));
    changed
}
fn properties(ui: &mut egui::Ui, e: &mut Element) -> bool {
    let mut changed = false;
    ui.strong(format!("Объект {}", e.id));
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
        ElementKind::Text | ElementKind::Rect | ElementKind::Ellipse
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
    changed |= ui.text_edit_multiline(&mut e.text).changed();
    match &mut e.kind {
        ElementKind::Table { cells } => {
            ui.label("Ячейки таблицы");
            for (ri, row) in cells.iter_mut().enumerate() {
                ui.push_id(ri, |ui| {
                    ui.horizontal(|ui| {
                        for (ci, value) in row.iter_mut().enumerate() {
                            ui.push_id(ci, |ui| {
                                changed |= ui
                                    .add(egui::TextEdit::singleline(value).desired_width(55.0))
                                    .changed();
                            });
                        }
                    });
                });
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("+ строка").clicked() {
                    cells.push(vec![
                        String::new();
                        cells.first().map(Vec::len).unwrap_or(1)
                    ]);
                    changed = true;
                }
                if ui.button("+ столбец").clicked() {
                    for row in cells.iter_mut() {
                        row.push(String::new());
                    }
                    changed = true;
                }
                if ui
                    .add_enabled(cells.len() > 1, egui::Button::new("− строка"))
                    .clicked()
                {
                    cells.pop();
                    changed = true;
                }
                if ui
                    .add_enabled(
                        cells.first().map(Vec::len).unwrap_or(0) > 1,
                        egui::Button::new("− столбец"),
                    )
                    .clicked()
                {
                    for row in cells.iter_mut() {
                        row.pop();
                    }
                    changed = true;
                }
            });
        }
        ElementKind::Chart { values, labels } => {
            ui.label("Данные диаграммы");
            for (index, value) in values.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(label) = labels.get_mut(index) {
                            changed |= ui
                                .add(egui::TextEdit::singleline(label).desired_width(90.0))
                                .changed();
                        }
                        changed |= ui.add(egui::DragValue::new(value)).changed();
                    });
                });
            }
            if ui.button("+ категория").clicked() {
                values.push(0.0);
                labels.push((values.len()).to_string());
                changed = true;
            }
        }
        _ => {}
    }
    ui.separator();
    ui.label("Анимация при показе");
    egui::ComboBox::from_id_salt("animation_effect")
        .selected_text(&e.animation.effect)
        .show_ui(ui, |ui| {
            for effect in ["none", "fade", "fly_in", "zoom"] {
                changed |= ui
                    .selectable_value(&mut e.animation.effect, effect.into(), effect)
                    .changed();
            }
        });
    egui::ComboBox::from_id_salt("animation_trigger")
        .selected_text(&e.animation.trigger)
        .show_ui(ui, |ui| {
            for trigger in ["with_previous", "after_previous", "on_click"] {
                changed |= ui
                    .selectable_value(&mut e.animation.trigger, trigger.into(), trigger)
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
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, rgb(slide.background));
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
        paint_element(ui.ctx(), &painter, rect.min, scale, element, 1.0);
    }
    if response.clicked() {
        state.selected.clear();
        response.request_focus();
    }
    let interactions = slide.elements.clone();
    for element in &interactions {
        let object = Rect::from_min_size(
            rect.min + Vec2::new(element.x, element.y) * scale,
            Vec2::new(element.w, element.h) * scale,
        );
        let hit = ui.interact(
            object,
            ui.id().with(("slide_object", element.id)),
            Sense::click_and_drag(),
        );
        if hit.clicked() || hit.drag_started() {
            response.request_focus();
            if !state.selected.contains(&element.id) {
                if !ui.input(|i| i.modifiers.ctrl || i.modifiers.shift) {
                    state.selected.clear();
                }
                state.selected.push(element.id);
            }
            if let Some(group) = element.group {
                state.selected.extend(
                    slide
                        .elements
                        .iter()
                        .filter(|e| e.group == Some(group))
                        .map(|e| e.id),
                );
                state.selected.sort_unstable();
                state.selected.dedup();
            }
        }
        if hit.double_clicked() {
            state.edit_text = Some(element.id);
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
            let delta = hit.total_drag_delta().unwrap_or(Vec2::ZERO) / scale;
            for (id, x, y, _, _) in &state.drag_original {
                if let Some(e) = slide.element_mut(*id) {
                    let step = if state.grid { 24.0 } else { 1.0 };
                    e.x = ((x + delta.x) / step).round() * step;
                    e.y = ((y + delta.y) / step).round() * step;
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
            Stroke::new(1.5, Color32::from_rgb(40, 120, 230)),
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
                Stroke::new(1.0, Color32::from_rgb(40, 120, 230)),
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
    if (response.has_focus() || response.hovered()) && !ui.ctx().egui_wants_keyboard_input() {
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
        let mut element = element.clone();
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
                "fly_in" => {
                    element.x += (1.0 - ease) * deck.width;
                    opacity = ease;
                }
                "zoom" => {
                    let factor = 0.05 + 0.95 * ease;
                    element.x += element.w * (1.0 - factor) / 2.0;
                    element.y += element.h * (1.0 - factor) / 2.0;
                    element.w *= factor;
                    element.h *= factor;
                    opacity = ease;
                }
                _ => {}
            }
        }
        paint_element(ctx, &painter, rect.min, scale, &element, opacity);
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
            if matches!(e.kind, ElementKind::Arrow) {
                painter.arrow(rect.min, rect.size(), stroke);
            } else {
                painter.line_segment([rect.min, rect.max], stroke);
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
                painter.image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
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
                        ..Element::default()
                    };
                    let font = FontId::new(
                        cell_element.font_size * scale,
                        egui::FontFamily::Name(
                            if e.bold && e.italic {
                                "bold-italic"
                            } else if e.bold {
                                "bold"
                            } else if e.italic {
                                "italic"
                            } else {
                                "system"
                            }
                            .into(),
                        ),
                    );
                    for (index, line) in super::model::text_lines(&cell_element)
                        .into_iter()
                        .enumerate()
                    {
                        painter.text(
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
                    egui::FontFamily::Name(
                        if e.bold && e.italic {
                            "bold-italic"
                        } else if e.bold {
                            "bold"
                        } else if e.italic {
                            "italic"
                        } else {
                            "system"
                        }
                        .into(),
                    ),
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
    use super::*;
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
