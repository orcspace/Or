use super::model::{
    Align, Book, CellFormat, ChartKind, MAX_COLS, MAX_ROWS, Range, Selection, Sheet, Value, address,
};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
struct State {
    editing: bool,
    buffer: String,
    last: (u64, u32, u32),
    error: String,
    find: String,
    replace: String,
    find_open: bool,
    rename: String,
    ribbon_tab: usize,
    location: String,
    scroll_to_selection: bool,
    rename_sheet_id: Option<u64>,
    #[cfg(test)]
    formula_rect: Option<Rect>,
    #[cfg(test)]
    test_controls: BTreeMap<String, Rect>,
}
fn color(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
fn apply_format(book: &mut Book, mut change: impl FnMut(&mut CellFormat)) -> Result<(), String> {
    let id = book.active_id();
    let range = book.selection.range();
    range.validate()?;
    for r in range.r0..=range.r1 {
        for c in range.c0..=range.c1 {
            if book.sheet_by_id(id).and_then(|s| s.cell(r, c)).is_none() {
                book.set_input(id, r, c, "")?;
            }
            if let Some(cell) = book.sheet_by_id_mut(id).and_then(|s| s.cell_mut(r, c)) {
                change(&mut cell.fmt);
            }
        }
    }
    Ok(())
}
fn result(state: &mut State, value: Result<(), String>) -> bool {
    match value {
        Ok(()) => {
            state.error.clear();
            true
        }
        Err(e) => {
            state.error = e;
            false
        }
    }
}
fn commit(book: &mut Book, state: &mut State) -> bool {
    if !state.editing {
        return false;
    }
    let value = book
        .set_cell(state.last.0, state.last.1, state.last.2, &state.buffer)
        .map(|_| ());
    let changed = result(state, value);
    if changed {
        state.editing = false;
    }
    changed
}
fn cancel_edit(book: &Book, state: &mut State) {
    state.editing = false;
    state.error.clear();
    state.buffer = book
        .sheet_by_id(state.last.0)
        .and_then(|sheet| sheet.cell(state.last.1, state.last.2))
        .map(|cell| cell.input.clone())
        .unwrap_or_default();
}
fn command_action(
    response: egui::Response,
    book: &mut Book,
    state: &mut State,
    changed: &mut bool,
) -> bool {
    if response.clicked() || response.changed() {
        *changed |= commit(book, state);
        !state.editing
    } else {
        false
    }
}
fn toolbar(
    ui: &mut egui::Ui,
    book: &mut Book,
    state: &mut State,
    selected: &CellFormat,
    state_id: egui::Id,
) -> bool {
    let mut changed = false;
    let id = book.active_id();
    let find_id = state_id.with("find");
    crate::chrome::tabs(
        ui,
        &mut state.ribbon_tab,
        &["Главная", "Вставка", "Данные", "Вид"],
    );
    crate::chrome::ribbon(ui, |ui| match state.ribbon_tab {
        0 => {
            crate::chrome::group(ui, "Буфер", 126.0, |ui| {
                ui.horizontal(|ui| {
                    if command_action(
                        ui.button("Вставить").on_hover_text("Ctrl+V"),
                        book,
                        state,
                        &mut changed,
                    ) {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    }
                    ui.menu_button("Ещё", |ui| {
                        if command_action(
                            ui.button("Копировать · Ctrl+C"),
                            book,
                            state,
                            &mut changed,
                        ) {
                            copy_range(ui, book, state);
                            ui.close();
                        }
                        if command_action(ui.button("Вырезать · Ctrl+X"), book, state, &mut changed)
                        {
                            if copy_range(ui, book, state) {
                                changed |= clear(book, state);
                            }
                            ui.close();
                        }
                        ui.separator();
                        if command_action(
                            ui.button("Очистить значения · Delete"),
                            book,
                            state,
                            &mut changed,
                        ) {
                            changed |= clear(book, state);
                            ui.close();
                        }
                    });
                });
            });
            crate::chrome::group(ui, "Шрифт", 452.0, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let mut family = selected.font.clone();
                    if crate::chrome::font_picker(ui, "sheets-font", &mut family, 128.0) {
                        changed |= commit(book, state);
                        if !state.editing {
                            changed |= result(
                                state,
                                apply_format(book, |format| format.font = family.clone()),
                            );
                        }
                    }
                    let mut size = selected.size;
                    if crate::chrome::size_picker(ui, "sheets-size", &mut size) {
                        changed |= commit(book, state);
                        if !state.editing {
                            let size = size.clamp(1.0, 400.0);
                            changed |=
                                result(state, apply_format(book, |format| format.size = size));
                        }
                    }
                    for (glyph, grow, tip) in [
                        ("A+", true, "Увеличить размер шрифта"),
                        ("A−", false, "Уменьшить размер шрифта"),
                    ] {
                        if command_action(
                            crate::chrome::tool(ui, glyph, tip, false),
                            book,
                            state,
                            &mut changed,
                        ) {
                            let size = crate::chrome::step_size(selected.size, grow);
                            changed |=
                                result(state, apply_format(book, |format| format.size = size));
                        }
                    }
                    ui.separator();
                    for (index, (label, flag, tip)) in [
                        (
                            egui::RichText::new("B").strong(),
                            selected.bold,
                            "Полужирный · Ctrl+B",
                        ),
                        (
                            egui::RichText::new("I").italics(),
                            selected.italic,
                            "Курсив · Ctrl+I",
                        ),
                        (
                            egui::RichText::new("U").underline(),
                            selected.underline,
                            "Подчёркнутый · Ctrl+U",
                        ),
                        (
                            egui::RichText::new("S").strikethrough(),
                            selected.strike,
                            "Зачёркнутый · Ctrl+5",
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if command_action(
                            crate::chrome::tool(ui, label, tip, flag),
                            book,
                            state,
                            &mut changed,
                        ) {
                            changed |= result(
                                state,
                                apply_format(book, |format| match index {
                                    0 => format.bold = !flag,
                                    1 => format.italic = !flag,
                                    2 => format.underline = !flag,
                                    _ => format.strike = !flag,
                                }),
                            );
                        }
                    }
                    ui.separator();
                    if command_action(
                        crate::chrome::tool(ui, "▦", "Границы ячеек", selected.border),
                        book,
                        state,
                        &mut changed,
                    ) {
                        changed |= result(
                            state,
                            apply_format(book, |format| format.border = !selected.border),
                        );
                    }
                    if let Some(fill) = crate::chrome::color_menu(
                        ui,
                        "◧",
                        "Цвет заливки",
                        selected.fill,
                        Some("Нет заливки"),
                    ) {
                        changed |= commit(book, state);
                        if !state.editing {
                            changed |=
                                result(state, apply_format(book, |format| format.fill = fill));
                        }
                    }
                    if let Some(Some(color)) = crate::chrome::color_menu(
                        ui,
                        "A",
                        "Цвет текста",
                        Some(selected.color),
                        None,
                    ) {
                        changed |= commit(book, state);
                        if !state.editing {
                            changed |=
                                result(state, apply_format(book, |format| format.color = color));
                        }
                    }
                });
            });
            crate::chrome::group(ui, "Выравнивание", 236.0, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (index, label, align) in [
                        (0, "По левому краю", Align::Left),
                        (1, "По центру", Align::Center),
                        (2, "По правому краю", Align::Right),
                    ] {
                        let on = selected.align == align;
                        if command_action(
                            crate::chrome::alignment_button(ui, index, label, on),
                            book,
                            state,
                            &mut changed,
                        ) {
                            let align = if on { Align::General } else { align };
                            changed |=
                                result(state, apply_format(book, |format| format.align = align));
                        }
                    }
                    ui.separator();
                    if command_action(
                        crate::chrome::tool(ui, "↵", "Перенос текста", selected.wrap),
                        book,
                        state,
                        &mut changed,
                    ) {
                        changed |= result(
                            state,
                            apply_format(book, |format| format.wrap = !selected.wrap),
                        );
                    }
                    ui.menu_button("⟲", |ui| {
                        for (label, angle) in [
                            ("Без поворота", 0),
                            ("Против часовой (45°)", 45),
                            ("По часовой (−45°)", -45),
                            ("Вверх (90°)", 90),
                            ("Вниз (−90°)", -90),
                        ] {
                            if command_action(
                                ui.selectable_label(selected.rotation == angle, label),
                                book,
                                state,
                                &mut changed,
                            ) {
                                changed |= result(
                                    state,
                                    apply_format(book, |format| format.rotation = angle),
                                );
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Ориентация текста");
                    ui.menu_button("⊞", |ui| {
                        merge_menu(ui, book, state, &mut changed);
                    })
                    .response
                    .on_hover_text("Объединить ячейки");
                });
            });
            crate::chrome::group(ui, "Число", 214.0, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let mut number = selected.number.clone();
                    egui::ComboBox::from_id_salt(state_id.with("number-format"))
                        .width(118.0)
                        .selected_text(
                            super::format::NUMBER_FORMATS
                                .iter()
                                .find(|(_, code)| *code == number)
                                .map(|(label, _)| *label)
                                .unwrap_or("Свой формат"),
                        )
                        .show_ui(ui, |ui| {
                            for (label, code) in super::format::NUMBER_FORMATS {
                                ui.selectable_value(&mut number, code.to_string(), *label);
                            }
                        });
                    for (glyph, code, tip) in [
                        ("%", "0%", "Процентный формат"),
                        ("000", "#,##0", "Формат с разделителями"),
                    ] {
                        if crate::chrome::tool(ui, glyph, tip, selected.number == code).clicked() {
                            number = code.into();
                        }
                    }
                    if number != selected.number {
                        changed |= commit(book, state);
                        if !state.editing {
                            changed |= result(
                                state,
                                apply_format(book, |format| format.number = number.clone()),
                            );
                        }
                    }
                    if command_action(
                        crate::chrome::tool(ui, "⌫", "Очистить формат", false),
                        book,
                        state,
                        &mut changed,
                    ) {
                        changed |= result(
                            state,
                            apply_format(book, |format| *format = CellFormat::default()),
                        );
                    }
                });
            });
            crate::chrome::group(ui, "Правка", 92.0, |ui| {
                if command_action(
                    ui.button("Поиск")
                        .on_hover_text("Найти и заменить · Ctrl+F / Ctrl+H"),
                    book,
                    state,
                    &mut changed,
                ) {
                    state.find_open = true;
                    ui.memory_mut(|memory| memory.request_focus(find_id));
                }
            });
        }
        1 => {
            crate::chrome::group(ui, "Данные", 175.0, |ui| {
                if command_action(
                    ui.button("Оформить диапазон")
                        .on_hover_text("Заголовок, чередование строк и границы выделенных ячеек"),
                    book,
                    state,
                    &mut changed,
                ) {
                    changed |= result(state, table_format(book));
                }
            });
            crate::chrome::group(ui, "Диаграммы", 138.0, |ui| {
                ui.menu_button("Диаграмма", |ui| {
                    for kind in [
                        ChartKind::Column,
                        ChartKind::Bar,
                        ChartKind::Line,
                        ChartKind::Area,
                        ChartKind::Pie,
                        ChartKind::Doughnut,
                        ChartKind::Scatter,
                    ] {
                        if command_action(
                            ui.button(chart_kind_name(kind)),
                            book,
                            state,
                            &mut changed,
                        ) {
                            changed |= result(
                                state,
                                book.add_chart(id, kind, book.selection.range(), "Диаграмма")
                                    .map(|_| ()),
                            );
                            ui.close();
                        }
                    }
                });
            });
            crate::chrome::group(ui, "Лист", 244.0, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Строки и столбцы", |ui| {
                        dimension_menu(ui, book, state, &mut changed);
                    });
                    let response = ui.button("Новый лист");
                    #[cfg(test)]
                    state
                        .test_controls
                        .insert("toolbar-new-sheet".into(), response.rect);
                    if command_action(response, book, state, &mut changed) {
                        changed |= add_sheet(book, state);
                    }
                });
            });
        }
        2 => {
            crate::chrome::group(ui, "Порядок данных", 282.0, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Сортировка", |ui| {
                        for (label, descending) in [("А → Я", false), ("Я → А", true)] {
                            if command_action(ui.button(label), book, state, &mut changed) {
                                let range = book.selection.range();
                                changed |= result(
                                    state,
                                    book.sort(id, range, &[(range.c0, descending)], false),
                                );
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Заполнение", |ui| {
                        for (label, down) in [("Вниз · Ctrl+D", true), ("Вправо · Ctrl+R", false)]
                        {
                            if command_action(ui.button(label), book, state, &mut changed) {
                                let target = book.selection.range();
                                let source = if down {
                                    Range::new(target.r0, target.c0, target.r0, target.c1)
                                } else {
                                    Range::new(target.r0, target.c0, target.r1, target.c0)
                                };
                                changed |= result(state, book.fill(id, source, target));
                                ui.close();
                            }
                        }
                    });
                });
            });
            crate::chrome::group(ui, "Работа с данными", 216.0, |ui| {
                ui.horizontal(|ui| {
                    if command_action(
                        ui.button("Поиск")
                            .on_hover_text("Найти и заменить · Ctrl+F / Ctrl+H"),
                        book,
                        state,
                        &mut changed,
                    ) {
                        state.find_open = true;
                        ui.memory_mut(|memory| memory.request_focus(find_id));
                    }
                    if command_action(
                        ui.button("Пересчитать")
                            .on_hover_text("Обновить значения всех формул"),
                        book,
                        state,
                        &mut changed,
                    ) {
                        book.recalculate();
                        changed = true;
                    }
                });
            });
        }
        _ => {
            crate::chrome::group(ui, "Область просмотра", 170.0, |ui| {
                ui.menu_button("Закрепление", |ui| {
                    if command_action(ui.button("До выбранной ячейки"), book, state, &mut changed)
                    {
                        let row = book.selection.row;
                        let col = book.selection.col;
                        changed |= freeze_selection(book, state, row, col);
                        ui.close();
                    }
                    if command_action(ui.button("Снять закрепление"), book, state, &mut changed)
                    {
                        if let Some(sheet) = book.active_sheet_mut() {
                            sheet.frozen_rows = 0;
                            sheet.frozen_cols = 0;
                            changed = true;
                        }
                        ui.close();
                    }
                });
            });
            crate::chrome::group(ui, "Размеры и видимость", 248.0, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Размеры", |ui| {
                        let row = book.selection.row;
                        let col = book.selection.col;
                        let mut width = book
                            .active_sheet()
                            .and_then(|sheet| sheet.widths.get(&col))
                            .copied()
                            .unwrap_or(96.0);
                        let mut height = book
                            .active_sheet()
                            .and_then(|sheet| sheet.row_heights.get(&row))
                            .copied()
                            .unwrap_or(26.0);
                        ui.horizontal(|ui| {
                            ui.label("Ширина столбца");
                            if command_action(
                                ui.add(
                                    egui::DragValue::new(&mut width)
                                        .range(24.0..=1000.0)
                                        .speed(1.0),
                                ),
                                book,
                                state,
                                &mut changed,
                            ) && let Some(sheet) = book.active_sheet_mut()
                            {
                                sheet.widths.insert(col, width);
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Высота строки");
                            if command_action(
                                ui.add(
                                    egui::DragValue::new(&mut height)
                                        .range(14.0..=500.0)
                                        .speed(1.0),
                                ),
                                book,
                                state,
                                &mut changed,
                            ) && let Some(sheet) = book.active_sheet_mut()
                            {
                                sheet.row_heights.insert(row, height);
                                changed = true;
                            }
                        });
                        ui.separator();
                        if command_action(ui.button("Автоподбор ширины"), book, state, &mut changed)
                        {
                            if let Some(sheet) = book.active_sheet_mut() {
                                let width = sheet
                                    .cells
                                    .iter()
                                    .filter(|(address, _)| {
                                        super::model::parse_address(address)
                                            .is_some_and(|(_, c)| c == col)
                                    })
                                    .map(|(_, cell)| {
                                        cell.value.display().chars().count() as f32 * 7.5 + 18.0
                                    })
                                    .fold(96.0, f32::max)
                                    .min(600.0);
                                sheet.widths.insert(col, width);
                                changed = true;
                            }
                            ui.close();
                        }
                    });
                    ui.menu_button("Видимость", |ui| {
                        visibility_menu(ui, book, state, &mut changed);
                    });
                });
            });
        }
    });
    changed
}

fn merge_menu(ui: &mut egui::Ui, book: &mut Book, state: &mut State, changed: &mut bool) {
    if command_action(
        ui.button("Объединить")
            .on_hover_text("Непустые ячейки сохраняются"),
        book,
        state,
        changed,
    ) {
        *changed |= result(state, book.merge(book.active_id(), book.selection.range()));
        ui.close();
    }
    if command_action(ui.button("Разъединить"), book, state, changed) {
        let range = book.selection.range();
        if let Some(sheet) = book.active_sheet_mut() {
            sheet.merges.retain(|merge| {
                !(merge.r0 <= range.r1
                    && range.r0 <= merge.r1
                    && merge.c0 <= range.c1
                    && range.c0 <= merge.c1)
            });
            *changed = true;
        }
        ui.close();
    }
}
fn dimension_menu(ui: &mut egui::Ui, book: &mut Book, state: &mut State, changed: &mut bool) {
    for (label, rows, delete) in [
        ("Вставить строку", true, false),
        ("Удалить строку", true, true),
        ("Вставить столбец", false, false),
        ("Удалить столбец", false, true),
    ] {
        if command_action(ui.button(label), book, state, changed) {
            *changed |= result(
                state,
                book.edit_dimension(
                    book.active_id(),
                    if rows {
                        book.selection.row
                    } else {
                        book.selection.col
                    },
                    1,
                    rows,
                    delete,
                ),
            );
            ui.close();
        }
    }
    ui.separator();
    visibility_menu(ui, book, state, changed);
}
fn visibility_menu(ui: &mut egui::Ui, book: &mut Book, state: &mut State, changed: &mut bool) {
    for (label, rows) in [("Скрыть строку", true), ("Скрыть столбец", false)]
    {
        if command_action(ui.button(label), book, state, changed) {
            let row = book.selection.row;
            let col = book.selection.col;
            if let Some(sheet) = book.active_sheet_mut() {
                if rows {
                    sheet.hidden_rows.insert(row);
                } else {
                    sheet.hidden_cols.insert(col);
                }
                *changed = true;
            }
            ui.close();
        }
    }
    if command_action(ui.button("Показать скрытые"), book, state, changed) {
        if let Some(sheet) = book.active_sheet_mut() {
            sheet.hidden_rows.clear();
            sheet.hidden_cols.clear();
            *changed = true;
        }
        ui.close();
    }
}

pub fn show(ui: &mut egui::Ui, book: &mut Book) -> bool {
    show_impl(ui, book, ui.id().with("sheets-editor"))
}
pub fn show_for_tab(ui: &mut egui::Ui, book: &mut Book, tab_id: u64) -> bool {
    show_impl(ui, book, egui::Id::new(("sheets-editor", tab_id)))
}
pub fn commit_pending(ctx: &egui::Context, tab_id: u64, book: &mut Book) -> Result<bool, String> {
    let id = egui::Id::new(("sheets-editor", tab_id));
    let Some(mut state) = ctx.data_mut(|data| data.get_temp::<State>(id)) else {
        return Ok(false);
    };
    let changed = commit(book, &mut state);
    let error = if state.editing {
        Some(state.error.clone())
    } else {
        None
    };
    ctx.data_mut(|data| data.insert_temp(id, state));
    match error {
        Some(error) => Err(error),
        None => Ok(changed),
    }
}
fn show_impl(ui: &mut egui::Ui, book: &mut Book, state_id: egui::Id) -> bool {
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id))
        .unwrap_or_default();
    let mut changed = false;
    let formula_id = state_id.with("formula");
    let canvas_id = state_id.with("canvas");
    #[cfg(test)]
    state.test_controls.clear();
    if ui.is_enabled() && state.editing && !state.error.is_empty() {
        ui.memory_mut(|memory| memory.request_focus(formula_id));
    }
    let formula_active =
        ui.memory(|memory| memory.has_focus(formula_id) || memory.had_focus_last_frame(formula_id));
    let canvas_active = ui.is_enabled()
        && !state.editing
        && ui.memory(|memory| memory.focused().is_none() || memory.has_focus(canvas_id));
    if canvas_active {
        ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
    }
    if book.sheets.is_empty() {
        ui.label("Нет листов");
        return false;
    }
    book.active = book.active.min(book.sheets.len() - 1);
    let id = book.active_id();
    let row = book.selection.row;
    let col = book.selection.col;
    if state.last != (id, row, col) {
        changed |= commit(book, &mut state);
        if !state.editing {
            state.last = (id, row, col);
            state.buffer = book
                .active_sheet()
                .and_then(|s| s.cell(row, col))
                .map(|c| c.input.clone())
                .unwrap_or_default();
        }
    }
    let selected = book
        .active_sheet()
        .and_then(|s| s.cell(row, col))
        .map(|c| c.fmt.clone())
        .unwrap_or_default();
    changed |= toolbar(ui, book, &mut state, &selected, state_id);
    let id = book.active_id();
    let row = book.selection.row;
    let col = book.selection.col;
    if state.last != (id, row, col) {
        changed |= commit(book, &mut state);
        if !state.editing {
            state.last = (id, row, col);
            state.buffer = book
                .active_sheet()
                .and_then(|sheet| sheet.cell(row, col))
                .map(|cell| cell.input.clone())
                .unwrap_or_default();
        }
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let location_id = state_id.with("location");
        if !ui.memory(|m| m.has_focus(location_id)) {
            let range = book.selection.range();
            state.location = if (range.r0, range.c0) == (range.r1, range.c1) {
                address(row, col)
            } else {
                format!(
                    "{}:{}",
                    address(range.r0, range.c0),
                    address(range.r1, range.c1)
                )
            };
        }
        let location_focused = ui.memory(|memory| memory.has_focus(location_id));
        let location_enter = ui.is_enabled()
            && location_focused
            && ui.input_mut(|input| input.consume_key(input.modifiers, Key::Enter));
        let location = ui
            .add(
                egui::TextEdit::singleline(&mut state.location)
                    .desired_width(110.0)
                    .id(location_id),
            )
            .on_hover_text("Адрес ячейки или диапазона · например A1:D8");
        if location_enter {
            location.surrender_focus();
            match super::model::parse_range(&state.location) {
                Ok(range) => {
                    changed |= commit(book, &mut state);
                    if !state.editing {
                        book.selection = Selection {
                            row: range.r0,
                            col: range.c0,
                            anchor_row: range.r1,
                            anchor_col: range.c1,
                        };
                        state.scroll_to_selection = true;
                        state.error.clear();
                    }
                }
                Err(error) => state.error = error,
            }
        }
        ui.label(
            egui::RichText::new("ƒx")
                .size(18.0)
                .color(crate::chrome::accent(ui)),
        );
        let focused = ui.memory(|memory| memory.has_focus(formula_id));
        let modifiers = ui.input(|i| i.modifiers);
        let action = if ui.is_enabled() && (focused || (state.editing && formula_active)) {
            ui.input_mut(|i| {
                if i.consume_key(modifiers, Key::Escape) {
                    Some(Key::Escape)
                } else if i.consume_key(modifiers, Key::Enter) {
                    Some(Key::Enter)
                } else if i.consume_key(modifiers, Key::Tab) {
                    Some(Key::Tab)
                } else {
                    None
                }
            })
        } else {
            None
        };
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.buffer)
                .desired_width(ui.available_width())
                .id(formula_id)
                .event_filter(egui::EventFilter {
                    tab: true,
                    escape: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                })
                .hint_text("Введите значение или формулу =SUM(A1:A10)"),
        );
        #[cfg(test)]
        {
            state.formula_rect = Some(response.rect);
        }
        if response.changed() {
            state.editing = true;
        }
        if let Some(key) = action {
            if key == Key::Escape {
                cancel_edit(book, &mut state);
            } else {
                changed |= commit(book, &mut state);
                if !state.editing {
                    advance(&mut book.selection, key, modifiers.shift);
                    state.scroll_to_selection = true;
                }
            }
            if !state.editing {
                response.surrender_focus();
                ui.memory_mut(|memory| {
                    memory.request_focus(canvas_id);
                    memory.move_focus(egui::FocusDirection::None);
                });
            }
        } else if ui.is_enabled() && response.lost_focus() {
            changed |= commit(book, &mut state);
        }
        if !state.editing {
            state.buffer = book
                .active_sheet()
                .and_then(|s| s.cell(row, col))
                .map(|c| c.input.clone())
                .unwrap_or_default();
        }
    });
    if book
        .active_sheet()
        .is_some_and(|sheet| sheet.frozen_rows > 20 || sheet.frozen_cols > 10)
    {
        ui.small("Холст отображает закрепление до 20 строк и 10 столбцов. Полная область сохраняется в файле.");
    }
    if !state.error.is_empty() {
        ui.colored_label(Color32::from_rgb(200, 60, 50), &state.error);
    }
    if state.find_open {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Найти").strong());
            ui.add(
                egui::TextEdit::singleline(&mut state.find)
                    .desired_width(140.0)
                    .id(state_id.with("find")),
            );
            if ui.button("Следующее").clicked()
                && !state.find.is_empty()
                && let Some(s) = book.active_sheet()
            {
                let mut cells: Vec<_> = s
                    .cells
                    .iter()
                    .filter_map(|(a, c)| super::model::parse_address(a).map(|pos| (pos, c)))
                    .collect();
                cells.sort_by_key(|x| x.0);
                let needle = state.find.to_lowercase();
                let found = cells
                    .iter()
                    .filter(|((r, c), _)| (*r, *c) > (row, col))
                    .chain(cells.iter().filter(|((r, c), _)| (*r, *c) <= (row, col)))
                    .find(|(_, c)| {
                        c.input.to_lowercase().contains(&needle)
                            || c.value.display().to_lowercase().contains(&needle)
                    })
                    .map(|(pos, _)| *pos);
                if let Some((r, c)) = found {
                    changed |= commit(book, &mut state);
                    if !state.editing {
                        book.selection = Selection {
                            row: r,
                            col: c,
                            anchor_row: r,
                            anchor_col: c,
                        };
                        state.scroll_to_selection = true;
                    }
                }
            }
            ui.label("Заменить");
            ui.add(
                egui::TextEdit::singleline(&mut state.replace)
                    .desired_width(140.0)
                    .id(state_id.with("replace")),
            );
            if command_action(ui.button("Все"), book, &mut state, &mut changed)
                && !state.find.is_empty()
            {
                let inputs: Vec<_> = book
                    .active_sheet()
                    .map(|s| {
                        s.cells
                            .iter()
                            .filter_map(|(a, c)| {
                                super::model::parse_address(a)
                                    .map(|pos| (pos, c.input.replace(&state.find, &state.replace)))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                for ((r, c), input) in inputs {
                    changed |= result(&mut state, book.set_input(id, r, c, &input).map(|_| ()));
                }
                book.recalculate();
            }
            if ui.button("×").clicked() {
                state.find_open = false;
            }
        });
    }
    if let Some(sheet) = book.active_sheet()
        && !sheet.charts.is_empty()
    {
        let chart_width = (ui.available_width() * 0.32).clamp(220.0, 420.0);
        egui::Panel::right(state_id.with("charts_panel"))
            .default_size(chart_width)
            .min_size(180.0)
            .max_size((ui.available_width() * 0.5).max(180.0))
            .resizable(true)
            .show(ui, |ui| {
                ui.heading("Диаграммы");
                egui::ScrollArea::vertical()
                    .id_salt(state_id.with("charts_scroll"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for chart in &sheet.charts {
                            ui.strong(&chart.title);
                            ui.label(
                                egui::RichText::new(chart_kind_name(chart.kind))
                                    .small()
                                    .color(crate::chrome::muted(ui)),
                            );
                            let width = ui.available_width().min(chart.width);
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(width, chart.height.min(220.0)),
                                Sense::hover(),
                            );
                            paint_chart(ui, sheet, chart, rect);
                            ui.separator();
                        }
                    });
            });
    }

    let grid_height = (ui.available_height() - 62.0).max(100.0);
    egui::ScrollArea::both()
        .id_salt(state_id.with("grid"))
        .max_height(grid_height)
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let Some(sheet) = book.active_sheet() else {
                return;
            };
            let (max_row, max_col) = sheet.extent();
            let rows = (max_row
                .max(book.selection.row)
                .max(book.selection.anchor_row)
                .saturating_add(100))
            .clamp(1000, MAX_ROWS);
            let cols = (max_col
                .max(book.selection.col)
                .max(book.selection.anchor_col)
                .saturating_add(20))
            .clamp(26, MAX_COLS);
            let width = axis_pos(cols, 96.0, &sheet.widths, &sheet.hidden_cols) + 48.0;
            let height = axis_pos(rows, 26.0, &sheet.row_heights, &sheet.hidden_rows) + 28.0;
            let (_, rect) = ui.allocate_space(Vec2::new(width, height));
            let response = ui.interact(rect, canvas_id, Sense::click_and_drag());
            if ui.is_enabled()
                && !state.editing
                && (response.clicked() || ui.memory(|memory| memory.focused().is_none()))
            {
                response.request_focus();
            }
            if ui.is_enabled() {
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        canvas_id,
                        egui::EventFilter {
                            tab: true,
                            escape: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                        },
                    )
                });
            }
            let origin = rect.min;
            let painter = ui.painter().with_clip_rect(ui.clip_rect());
            let bg = ui.visuals().extreme_bg_color;
            let grid = ui.visuals().widgets.noninteractive.bg_stroke.color;
            painter.rect_filled(rect, 0.0, bg);
            let c0 = axis_index(
                (viewport.min.x - 48.0).max(0.0),
                cols,
                96.0,
                &sheet.widths,
                &sheet.hidden_cols,
            );
            let r0 = axis_index(
                (viewport.min.y - 28.0).max(0.0),
                rows,
                26.0,
                &sheet.row_heights,
                &sheet.hidden_rows,
            );
            let c1 = axis_index(
                viewport.max.x,
                cols,
                96.0,
                &sheet.widths,
                &sheet.hidden_cols,
            )
            .saturating_add(2)
            .min(cols);
            let r1 = axis_index(
                viewport.max.y,
                rows,
                26.0,
                &sheet.row_heights,
                &sheet.hidden_rows,
            )
            .saturating_add(2)
            .min(rows);
            let selection = book.selection.range();
            let frozen_r = sheet.frozen_rows.min(20);
            let frozen_c = sheet.frozen_cols.min(10);
            #[cfg(test)]
            state.test_controls.insert(
                "grid-b1".into(),
                cell_rect(sheet, origin, viewport, 0, 1, frozen_r, frozen_c),
            );
            for r in (r0..r1).chain(0..frozen_r) {
                if sheet.hidden_rows.contains(&r) {
                    continue;
                }
                for c in (c0..c1).chain(0..frozen_c) {
                    if sheet.hidden_cols.contains(&c) {
                        continue;
                    }
                    let cell_rect = cell_rect(sheet, origin, viewport, r, c, frozen_r, frozen_c);
                    if !ui.clip_rect().intersects(cell_rect) {
                        continue;
                    }
                    if let Some(merge) = sheet.merges.iter().find(|m| m.contains(r, c))
                        && (merge.r0, merge.c0) != (r, c)
                    {
                        continue;
                    }
                    let mut cell_rect = cell_rect;
                    if let Some(merge) = sheet.merges.iter().find(|m| m.r0 == r && m.c0 == c) {
                        cell_rect.max.x = cell_rect.min.x
                            + axis_pos(merge.c1 + 1, 96.0, &sheet.widths, &sheet.hidden_cols)
                            - axis_pos(merge.c0, 96.0, &sheet.widths, &sheet.hidden_cols);
                        cell_rect.max.y = cell_rect.min.y
                            + axis_pos(merge.r1 + 1, 26.0, &sheet.row_heights, &sheet.hidden_rows)
                            - axis_pos(merge.r0, 26.0, &sheet.row_heights, &sheet.hidden_rows);
                    }
                    let cell = sheet.cell(r, c);
                    let fmt = cell.map(|c| c.fmt.clone()).unwrap_or_default();
                    let conditional = sheet
                        .conditional
                        .iter()
                        .find(|rule| {
                            rule.range.contains(r, c)
                                && cell.and_then(|c| c.value.number().ok()).is_some_and(|n| {
                                    match rule.operator.as_str() {
                                        ">" => n > rule.threshold,
                                        "<" => n < rule.threshold,
                                        "=" => n == rule.threshold,
                                        ">=" => n >= rule.threshold,
                                        "<=" => n <= rule.threshold,
                                        _ => false,
                                    }
                                })
                        })
                        .map(|rule| rule.fill);
                    if r < frozen_r || c < frozen_c {
                        painter.rect_filled(cell_rect, 0.0, bg);
                    }
                    if let Some(fill) = conditional.or(fmt.fill) {
                        painter.rect_filled(cell_rect, 0.0, color(fill));
                    }
                    if selection.contains(r, c) {
                        painter.rect_filled(
                            cell_rect,
                            0.0,
                            crate::chrome::accent(ui).gamma_multiply(0.12),
                        );
                    }
                    painter.rect_stroke(
                        cell_rect,
                        0.0,
                        Stroke::new(
                            if fmt.border { 1.0 } else { 0.5 },
                            if fmt.border {
                                ui.visuals().text_color()
                            } else {
                                grid
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    let text = sheet.display(r, c, book.locale_ru);
                    let right = fmt.align == Align::Right
                        || (fmt.align == Align::General
                            && cell.is_some_and(|c| matches!(c.value, Value::Number(_))));
                    let family =
                        crate::fonts::egui_family(ui.ctx(), &fmt.font, fmt.bold, fmt.italic);
                    let font = FontId::new(fmt.size * 1.333, family);
                    let align = if fmt.align == Align::Center {
                        Align2::CENTER_CENTER
                    } else if right {
                        Align2::RIGHT_CENTER
                    } else {
                        Align2::LEFT_CENTER
                    };
                    let point = if fmt.align == Align::Center {
                        cell_rect.center()
                    } else if right {
                        Pos2::new(cell_rect.right() - 5.0, cell_rect.center().y)
                    } else {
                        Pos2::new(cell_rect.left() + 5.0, cell_rect.center().y)
                    };
                    let clipped = painter.with_clip_rect(cell_rect.shrink(2.0));
                    let text_color = if ui.visuals().dark_mode && fmt.color == [28, 32, 38] {
                        ui.visuals().text_color()
                    } else {
                        color(fmt.color)
                    };
                    if fmt.wrap {
                        let galley = clipped.layout(
                            text,
                            font,
                            text_color,
                            (cell_rect.width() - 10.0).max(1.0),
                        );
                        let x = if fmt.align == Align::Center {
                            cell_rect.center().x - galley.size().x / 2.0
                        } else if right {
                            cell_rect.right() - 5.0 - galley.size().x
                        } else {
                            cell_rect.left() + 5.0
                        };
                        clipped.galley(Pos2::new(x, cell_rect.top() + 3.0), galley, text_color);
                    } else {
                        clipped.text(point, align, text, font, text_color);
                    }
                    if fmt.underline || fmt.strike {
                        let y = cell_rect.center().y + if fmt.strike { 0.0 } else { 7.0 };
                        clipped.line_segment(
                            [
                                Pos2::new(cell_rect.left() + 5.0, y),
                                Pos2::new(cell_rect.right() - 5.0, y),
                            ],
                            Stroke::new(1.0, color(fmt.color)),
                        );
                    }
                    if (r, c) == (row, col) {
                        painter.rect_stroke(
                            cell_rect,
                            0.0,
                            Stroke::new(2.0, crate::chrome::accent(ui)),
                            StrokeKind::Inside,
                        );
                    }
                }
            }
            let header_y = origin.y + viewport.min.y;
            let headers: Vec<_> = (c0..c1)
                .filter(|c| !sheet.hidden_cols.contains(c))
                .map(|c| {
                    (
                        c,
                        origin.x + 48.0 + axis_pos(c, 96.0, &sheet.widths, &sheet.hidden_cols),
                        sheet.widths.get(&c).copied().unwrap_or(96.0),
                    )
                })
                .collect();
            for (c, x, w) in headers {
                let header = Rect::from_min_size(Pos2::new(x, header_y), Vec2::new(w, 28.0));
                painter.rect_filled(
                    header,
                    0.0,
                    if selection.c0 <= c && c <= selection.c1 {
                        crate::chrome::accent(ui).gamma_multiply(0.12)
                    } else {
                        crate::chrome::workspace(ui)
                    },
                );
                painter.rect_stroke(header, 0.0, Stroke::new(0.5, grid), StrokeKind::Inside);
                painter.text(
                    header.center(),
                    Align2::CENTER_CENTER,
                    address(0, c).trim_end_matches('1'),
                    FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );
                let resize = Rect::from_min_size(
                    Pos2::new(header.right() - 3.0, header.top()),
                    Vec2::new(6.0, 28.0),
                );
                let res = ui.interact(resize, state_id.with(("col-size", c)), Sense::drag());
                if res.dragged()
                    && let Some(s) = book.active_sheet_mut()
                {
                    let old = s.widths.get(&c).copied().unwrap_or(96.0);
                    s.widths
                        .insert(c, (old + res.drag_delta().x).clamp(24.0, 1000.0));
                    changed = true;
                }
            }
            let Some(sheet) = book.active_sheet() else {
                return;
            };
            for r in r0..r1 {
                if sheet.hidden_rows.contains(&r) {
                    continue;
                }
                let y = origin.y + 28.0 + axis_pos(r, 26.0, &sheet.row_heights, &sheet.hidden_rows);
                let h = sheet.row_heights.get(&r).copied().unwrap_or(26.0);
                let header = Rect::from_min_size(
                    Pos2::new(origin.x + viewport.min.x, y),
                    Vec2::new(48.0, h),
                );
                painter.rect_filled(
                    header,
                    0.0,
                    if selection.r0 <= r && r <= selection.r1 {
                        crate::chrome::accent(ui).gamma_multiply(0.12)
                    } else {
                        crate::chrome::workspace(ui)
                    },
                );
                painter.rect_stroke(header, 0.0, Stroke::new(0.5, grid), StrokeKind::Inside);
                painter.text(
                    header.center(),
                    Align2::CENTER_CENTER,
                    (r + 1).to_string(),
                    FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );
            }
            if (response.clicked() || response.dragged())
                && let Some(point) = response.interact_pointer_pos()
            {
                let local = point - origin;
                if local.x > 48.0 + viewport.min.x && local.y > 28.0 + viewport.min.y {
                    let r = axis_index(
                        local.y - 28.0,
                        rows,
                        26.0,
                        &sheet.row_heights,
                        &sheet.hidden_rows,
                    )
                    .min(rows - 1);
                    let c = axis_index(
                        local.x - 48.0,
                        cols,
                        96.0,
                        &sheet.widths,
                        &sheet.hidden_cols,
                    )
                    .min(cols - 1);
                    let r = if local.y - viewport.min.y
                        < 28.0 + axis_pos(frozen_r, 26.0, &sheet.row_heights, &sheet.hidden_rows)
                    {
                        axis_index(
                            local.y - viewport.min.y - 28.0,
                            rows,
                            26.0,
                            &sheet.row_heights,
                            &sheet.hidden_rows,
                        )
                    } else {
                        r
                    };
                    let c = if local.x - viewport.min.x
                        < 48.0 + axis_pos(frozen_c, 96.0, &sheet.widths, &sheet.hidden_cols)
                    {
                        axis_index(
                            local.x - viewport.min.x - 48.0,
                            cols,
                            96.0,
                            &sheet.widths,
                            &sheet.hidden_cols,
                        )
                    } else {
                        c
                    };
                    let (r, c) = sheet
                        .merges
                        .iter()
                        .find(|merge| merge.contains(r, c))
                        .map(|merge| (merge.r0, merge.c0))
                        .unwrap_or((r, c));
                    let extended = ui.input(|i| i.modifiers.shift) || response.dragged();
                    changed |= commit(book, &mut state);
                    if !state.editing {
                        book.selection.row = r;
                        book.selection.col = c;
                        if !extended {
                            book.selection.anchor_row = r;
                            book.selection.anchor_col = c;
                        }
                        if response.double_clicked() {
                            state.last = (id, r, c);
                            state.buffer = book
                                .active_sheet()
                                .and_then(|s| s.cell(r, c))
                                .map(|c| c.input.clone())
                                .unwrap_or_default();
                            state.editing = true;
                            ui.memory_mut(|m| m.request_focus(state_id.with("formula")));
                        }
                    }
                }
            }
            if ui.is_enabled()
                && !state.editing
                && (response.has_focus() || !ui.ctx().egui_wants_keyboard_input())
            {
                let events = ui.input(|i| i.events.clone());
                for event in events {
                    match event {
                        egui::Event::Copy | egui::Event::Cut => {
                            let range = book.selection.range();
                            if let Ok(rows) =
                                book.read_range(id, range.r0, range.c0, range.r1, range.c1)
                            {
                                ui.ctx().copy_text(super::io::to_csv(&rows, '\t'));
                            }
                            if matches!(event, egui::Event::Cut) {
                                changed |= clear(book, &mut state);
                            }
                        }
                        egui::Event::Paste(text) => match super::io::parse_csv(&text, '\t') {
                            Ok(rows) => {
                                changed |= result(
                                    &mut state,
                                    book.set_range(
                                        id,
                                        book.selection.row,
                                        book.selection.col,
                                        &rows,
                                    ),
                                )
                            }
                            Err(e) => state.error = e,
                        },
                        egui::Event::Text(text) => {
                            if state.editing {
                                state.buffer.push_str(&text);
                            } else {
                                state.buffer = text;
                            }
                            state.last = (id, book.selection.row, book.selection.col);
                            state.editing = true;
                            ui.memory_mut(|m| m.request_focus(state_id.with("formula")));
                        }
                        egui::Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } => {
                            if state.editing {
                                if key == Key::Escape {
                                    cancel_edit(book, &mut state);
                                } else if key == Key::Enter || key == Key::Tab {
                                    changed |= commit(book, &mut state);
                                    if !state.editing {
                                        advance(&mut book.selection, key, modifiers.shift);
                                        state.scroll_to_selection = true;
                                    }
                                }
                                if !state.editing {
                                    ui.memory_mut(|memory| {
                                        memory.request_focus(canvas_id);
                                        memory.move_focus(egui::FocusDirection::None);
                                    });
                                }
                                continue;
                            }
                            if modifiers.ctrl && matches!(key, Key::B | Key::I | Key::U | Key::Num5)
                            {
                                // Excel: Ctrl+B/I/U/5 toggle bold/italic/underline/strike.
                                let current = book
                                    .active_sheet()
                                    .and_then(|sheet| {
                                        sheet.cell(book.selection.row, book.selection.col)
                                    })
                                    .map(|cell| cell.fmt.clone())
                                    .unwrap_or_default();
                                changed |= result(
                                    &mut state,
                                    apply_format(book, |format| match key {
                                        Key::B => format.bold = !current.bold,
                                        Key::I => format.italic = !current.italic,
                                        Key::U => format.underline = !current.underline,
                                        _ => format.strike = !current.strike,
                                    }),
                                );
                                continue;
                            }
                            if modifiers.ctrl && (key == Key::F || key == Key::H) {
                                state.find_open = true;
                                ui.memory_mut(|memory| memory.request_focus(state_id.with("find")));
                                continue;
                            }
                            if modifiers.ctrl && (key == Key::D || key == Key::R) {
                                let range = book.selection.range();
                                let source = if key == Key::D {
                                    Range::new(range.r0, range.c0, range.r0, range.c1)
                                } else {
                                    Range::new(range.r0, range.c0, range.r1, range.c0)
                                };
                                changed |= result(&mut state, book.fill(id, source, range));
                                continue;
                            }
                            if key == Key::Delete || key == Key::Backspace {
                                changed |= clear(book, &mut state);
                                continue;
                            }
                            if key == Key::F2 {
                                state.editing = true;
                                ui.memory_mut(|m| m.request_focus(state_id.with("formula")));
                                continue;
                            }
                            if key == Key::Enter || key == Key::Tab {
                                advance(&mut book.selection, key, modifiers.shift);
                                state.scroll_to_selection = true;
                                continue;
                            }
                            let step = if modifiers.ctrl { 20 } else { 1 };
                            let selection = &mut book.selection;
                            match key {
                                Key::ArrowUp => selection.row = selection.row.saturating_sub(step),
                                Key::ArrowDown => {
                                    selection.row = (selection.row + step).min(MAX_ROWS - 1)
                                }
                                Key::ArrowLeft => {
                                    selection.col = selection.col.saturating_sub(step)
                                }
                                Key::ArrowRight => {
                                    selection.col = (selection.col + step).min(MAX_COLS - 1)
                                }
                                Key::Home => {
                                    selection.col = 0;
                                    if modifiers.ctrl {
                                        selection.row = 0;
                                    }
                                }
                                Key::End => {
                                    selection.col = max_col;
                                    if modifiers.ctrl {
                                        selection.row = max_row;
                                    }
                                }
                                Key::PageUp => selection.row = selection.row.saturating_sub(25),
                                Key::PageDown => {
                                    selection.row = (selection.row + 25).min(MAX_ROWS - 1)
                                }
                                _ => continue,
                            }
                            if !modifiers.shift {
                                selection.anchor_row = selection.row;
                                selection.anchor_col = selection.col;
                            }
                            state.scroll_to_selection = true;
                        }
                        _ => (),
                    }
                }
            }
            if state.scroll_to_selection
                && let Some(sheet) = book.active_sheet()
            {
                let target = cell_rect(
                    sheet,
                    origin,
                    viewport,
                    book.selection.row,
                    book.selection.col,
                    frozen_r,
                    frozen_c,
                );
                ui.scroll_to_rect(target, None);
                state.scroll_to_selection = false;
            }
        });
    ui.add_space(6.0);
    egui::ScrollArea::horizontal()
        .id_salt(state_id.with("sheet-tabs"))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let visible: Vec<_> = book
                    .sheets
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| !s.hidden)
                    .map(|(i, s)| (i, s.id, s.name.clone()))
                    .collect();
                for (index, sid, name) in visible {
                    let response = ui.selectable_label(book.active == index, &name);
                    #[cfg(test)]
                    state
                        .test_controls
                        .insert(format!("sheet-{sid}"), response.rect);
                    if command_action(response.clone(), book, &mut state, &mut changed) {
                        book.active = index;
                        book.selection = Selection::default();
                    }
                    response.context_menu(|ui| {
                        ui.label("Имя листа");
                        if state.rename_sheet_id != Some(sid) {
                            state.rename_sheet_id = Some(sid);
                            state.rename = name.clone();
                        }
                        ui.text_edit_singleline(&mut state.rename);
                        if command_action(
                            ui.button("Переименовать"),
                            book,
                            &mut state,
                            &mut changed,
                        ) {
                            let name = state.rename.clone();
                            changed |= result(&mut state, book.rename_sheet(sid, &name));
                            state.rename.clear();
                            state.rename_sheet_id = None;
                            ui.close();
                        }
                        if command_action(
                            ui.button("Переместить в начало"),
                            book,
                            &mut state,
                            &mut changed,
                        ) {
                            changed |= result(&mut state, book.move_sheet(sid, 0));
                            ui.close();
                        }
                        if command_action(ui.button("Удалить лист"), book, &mut state, &mut changed)
                        {
                            changed |= result(&mut state, book.delete_sheet(sid));
                            ui.close();
                        }
                    });
                }
                let response = ui.button("+").on_hover_text("Добавить лист");
                #[cfg(test)]
                state
                    .test_controls
                    .insert("new-sheet".into(), response.rect);
                if response.clicked() {
                    changed |= add_sheet(book, &mut state);
                }
                let range = book.selection.range();
                if range.validate().is_ok()
                    && let Some(s) = book.active_sheet()
                {
                    let mut sum = 0.0;
                    let mut count = 0;
                    let mut nonempty = 0;
                    for r in range.r0..=range.r1 {
                        for c in range.c0..=range.c1 {
                            if let Some(cell) = s.cell(r, c) {
                                if cell.value != Value::Empty {
                                    nonempty += 1;
                                }
                                if let Value::Number(v) = cell.value {
                                    sum += v;
                                    count += 1;
                                }
                            }
                        }
                    }
                    ui.separator();
                    ui.label(format!(
                        "Сумма: {sum:.4}  Среднее: {:.4}  Количество: {nonempty}",
                        if count == 0 {
                            0.0
                        } else {
                            sum / f64::from(count)
                        }
                    ));
                }
            });
        });
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
    changed
}
fn freeze_selection(book: &mut Book, state: &mut State, row: u32, col: u32) -> bool {
    if row > 20 || col > 10 {
        state.error = "Холст пока поддерживает закрепление не более 20 строк и 10 столбцов. Выберите ячейку в этой области.".into();
        return false;
    }
    if let Some(sheet) = book.active_sheet_mut() {
        let changed = sheet.frozen_rows != row || sheet.frozen_cols != col;
        sheet.frozen_rows = row;
        sheet.frozen_cols = col;
        state.error.clear();
        changed
    } else {
        false
    }
}
fn advance(selection: &mut Selection, key: Key, backwards: bool) {
    match (key, backwards) {
        (Key::Enter, true) => selection.row = selection.row.saturating_sub(1),
        (Key::Enter, false) => selection.row = selection.row.saturating_add(1).min(MAX_ROWS - 1),
        (Key::Tab, true) => selection.col = selection.col.saturating_sub(1),
        (Key::Tab, false) => selection.col = selection.col.saturating_add(1).min(MAX_COLS - 1),
        _ => (),
    }
    selection.anchor_row = selection.row;
    selection.anchor_col = selection.col;
}
fn add_sheet(book: &mut Book, state: &mut State) -> bool {
    let committed = commit(book, state);
    if state.editing {
        return committed;
    }
    let mut index = book.sheets.len() + 1;
    loop {
        let name = format!("Лист{index}");
        if !book
            .sheets
            .iter()
            .any(|sheet| sheet.name.eq_ignore_ascii_case(&name))
        {
            let changed = result(state, book.add_sheet(&name).map(|_| ()));
            if changed {
                book.active = book.sheets.len().saturating_sub(1);
                book.selection = Selection::default();
                state.scroll_to_selection = true;
            }
            return committed || changed;
        }
        index += 1;
    }
}
fn copy_range(ui: &egui::Ui, book: &Book, state: &mut State) -> bool {
    let range = book.selection.range();
    match book.read_range(book.active_id(), range.r0, range.c0, range.r1, range.c1) {
        Ok(rows) => {
            ui.ctx().copy_text(super::io::to_csv(&rows, '\t'));
            state.error.clear();
            true
        }
        Err(error) => {
            state.error = error;
            false
        }
    }
}
fn table_format(book: &mut Book) -> Result<(), String> {
    let id = book.active_id();
    let range = book.selection.range();
    range.validate()?;
    for row in range.r0..=range.r1 {
        for col in range.c0..=range.c1 {
            if book
                .sheet_by_id(id)
                .and_then(|sheet| sheet.cell(row, col))
                .is_none()
            {
                book.set_input(id, row, col, "")?;
            }
            if let Some(cell) = book
                .sheet_by_id_mut(id)
                .and_then(|sheet| sheet.cell_mut(row, col))
            {
                cell.fmt.border = true;
                if row == range.r0 {
                    cell.fmt.bold = true;
                    cell.fmt.color = [255, 255, 255];
                    cell.fmt.fill = Some([29, 116, 83]);
                } else {
                    cell.fmt.color = [28, 32, 38];
                    cell.fmt.fill = Some(if (row - range.r0).is_multiple_of(2) {
                        [255, 255, 255]
                    } else {
                        [236, 246, 240]
                    });
                }
            }
        }
    }
    Ok(())
}
fn chart_kind_name(kind: ChartKind) -> &'static str {
    match kind {
        ChartKind::Column => "Столбчатая",
        ChartKind::Bar => "Линейчатая",
        ChartKind::Line => "Линейная",
        ChartKind::Area => "С областями",
        ChartKind::Pie => "Круговая",
        ChartKind::Doughnut => "Кольцевая",
        ChartKind::Scatter => "Точечная",
    }
}

fn clear(book: &mut Book, state: &mut State) -> bool {
    let range = book.selection.range();
    if let Err(e) = range.validate() {
        state.error = e;
        return false;
    }
    let id = book.active_id();
    let mut changed = false;
    for r in range.r0..=range.r1 {
        for c in range.c0..=range.c1 {
            changed |= result(state, book.set_input(id, r, c, "").map(|_| ()));
        }
    }
    book.recalculate();
    changed
}
fn axis_pos(index: u32, base: f32, sizes: &BTreeMap<u32, f32>, hidden: &BTreeSet<u32>) -> f32 {
    (index as f32 * base
        + sizes
            .range(..index)
            .filter(|(n, _)| !hidden.contains(n))
            .map(|(_, v)| *v - base)
            .sum::<f32>()
        - hidden.range(..index).count() as f32 * base)
        .max(0.0)
}
fn axis_index(
    position: f32,
    count: u32,
    base: f32,
    sizes: &BTreeMap<u32, f32>,
    hidden: &BTreeSet<u32>,
) -> u32 {
    let mut lo = 0;
    let mut hi = count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if axis_pos(mid + 1, base, sizes, hidden) <= position {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo.min(count.saturating_sub(1))
}
fn cell_rect(
    sheet: &Sheet,
    origin: Pos2,
    viewport: Rect,
    row: u32,
    col: u32,
    frozen_rows: u32,
    frozen_cols: u32,
) -> Rect {
    let x = 48.0
        + axis_pos(col, 96.0, &sheet.widths, &sheet.hidden_cols)
        + if col < frozen_cols {
            viewport.min.x
        } else {
            0.0
        };
    let y = 28.0
        + axis_pos(row, 26.0, &sheet.row_heights, &sheet.hidden_rows)
        + if row < frozen_rows {
            viewport.min.y
        } else {
            0.0
        };
    Rect::from_min_size(
        origin + Vec2::new(x, y),
        Vec2::new(
            sheet.widths.get(&col).copied().unwrap_or(96.0),
            sheet.row_heights.get(&row).copied().unwrap_or(26.0),
        ),
    )
}
fn paint_chart(ui: &egui::Ui, sheet: &Sheet, chart: &super::model::Chart, rect: Rect) {
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let r = chart.range;
    let start = if r.r1 > r.r0 { r.r0 + 1 } else { r.r0 };
    let first_col = if r.c1 > r.c0 { r.c0 + 1 } else { r.c0 };
    let palette = [
        Color32::from_rgb(35, 150, 100),
        Color32::from_rgb(52, 110, 220),
        Color32::from_rgb(240, 155, 45),
        Color32::from_rgb(175, 75, 180),
    ];
    let series: Vec<Vec<f64>> = (first_col..=r.c1.min(first_col + 7))
        .map(|c| {
            (start..=r.r1.min(start + 199))
                .map(|row| {
                    sheet
                        .cell(row, c)
                        .and_then(|c| c.value.number().ok())
                        .unwrap_or(0.0)
                })
                .collect()
        })
        .collect();
    let Some(first) = series.first() else {
        return;
    };
    if first.is_empty() {
        return;
    }
    let plot = rect.shrink2(Vec2::new(30.0, 18.0));
    let lo = series.iter().flatten().copied().fold(0.0, f64::min);
    let hi = series
        .iter()
        .flatten()
        .copied()
        .fold(0.0, f64::max)
        .max(lo + 1.0);
    let y = |v: f64| plot.bottom() - ((v - lo) / (hi - lo)) as f32 * plot.height();
    if matches!(chart.kind, ChartKind::Pie | ChartKind::Doughnut) {
        let total = first.iter().map(|v| v.max(0.0)).sum::<f64>();
        if total <= 0.0 {
            return;
        }
        let radius = plot.height().min(plot.width()) / 2.0;
        let center = plot.center();
        let mut angle = -std::f32::consts::FRAC_PI_2;
        for (i, v) in first.iter().enumerate() {
            let end = angle + (*v).max(0.0) as f32 / total as f32 * std::f32::consts::TAU;
            let steps = ((end - angle) * 20.0).ceil().max(1.0) as usize;
            let mut points = vec![center];
            for n in 0..=steps {
                let a = angle + (end - angle) * n as f32 / steps as f32;
                points.push(center + Vec2::angled(a) * radius);
            }
            painter.add(egui::Shape::convex_polygon(
                points,
                palette[i % palette.len()],
                Stroke::NONE,
            ));
            angle = end;
        }
        if chart.kind == ChartKind::Doughnut {
            painter.circle_filled(center, radius * 0.55, ui.visuals().extreme_bg_color);
        }
        return;
    }
    painter.line_segment(
        [plot.left_bottom(), plot.right_bottom()],
        Stroke::new(1.0, ui.visuals().text_color()),
    );
    let band = plot.width() / first.len() as f32;
    for (si, values) in series.iter().enumerate() {
        let c = palette[si % palette.len()];
        let mut points = Vec::new();
        for (i, v) in values.iter().enumerate() {
            let x = plot.left() + band * (i as f32 + 0.5);
            let point = Pos2::new(x, y(*v));
            match chart.kind {
                ChartKind::Column => {
                    let bar_width = band * 0.8 / series.len() as f32;
                    let x = plot.left() + band * i as f32 + band * 0.1 + si as f32 * bar_width;
                    painter.rect_filled(
                        Rect::from_two_pos(
                            Pos2::new(x, y(0.0)),
                            Pos2::new(x + bar_width - 1.0, y(*v)),
                        ),
                        0.0,
                        c,
                    );
                }
                ChartKind::Bar => {
                    let h = plot.height() / values.len() as f32 / series.len() as f32;
                    let yy = plot.top() + i as f32 * h * series.len() as f32 + si as f32 * h;
                    let zero = plot.left() + ((-lo) / (hi - lo)) as f32 * plot.width();
                    let xx = plot.left() + ((*v - lo) / (hi - lo)) as f32 * plot.width();
                    painter.rect_filled(
                        Rect::from_two_pos(Pos2::new(zero, yy), Pos2::new(xx, yy + h - 1.0)),
                        0.0,
                        c,
                    );
                }
                ChartKind::Scatter => {
                    let xv = sheet
                        .cell(start + i as u32, r.c0)
                        .and_then(|c| c.value.number().ok())
                        .unwrap_or(i as f64);
                    let xmin = (start..=r.r1.min(start + 199))
                        .filter_map(|rr| sheet.cell(rr, r.c0).and_then(|c| c.value.number().ok()))
                        .fold(f64::INFINITY, f64::min);
                    let xmax = (start..=r.r1.min(start + 199))
                        .filter_map(|rr| sheet.cell(rr, r.c0).and_then(|c| c.value.number().ok()))
                        .fold(f64::NEG_INFINITY, f64::max);
                    let width = (xmax - xmin).max(1.0);
                    painter.circle_filled(
                        Pos2::new(
                            plot.left() + ((xv - xmin) / width) as f32 * plot.width(),
                            point.y,
                        ),
                        3.0,
                        c,
                    );
                }
                _ => points.push(point),
            }
        }
        if chart.kind == ChartKind::Area {
            for pair in points.windows(2) {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(pair[0].x, y(0.0)),
                        pair[0],
                        pair[1],
                        Pos2::new(pair[1].x, y(0.0)),
                    ],
                    c.gamma_multiply(0.35),
                    Stroke::NONE,
                ));
            }
        }
        if !points.is_empty() {
            painter.add(egui::Shape::line(points, Stroke::new(2.0, c)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(key: Key, shift: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                shift,
                ..Default::default()
            },
        }
    }
    fn frame(
        ctx: &egui::Context,
        book: &mut Book,
        events: Vec<egui::Event>,
        shift: bool,
    ) -> Result<State, String> {
        frame_enabled(ctx, book, events, shift, true)
    }
    fn frame_enabled(
        ctx: &egui::Context,
        book: &mut Book,
        events: Vec<egui::Event>,
        shift: bool,
        enabled: bool,
    ) -> Result<State, String> {
        frame_with_ribbon(ctx, book, events, shift, enabled, None)
    }
    fn frame_with_ribbon(
        ctx: &egui::Context,
        book: &mut Book,
        mut events: Vec<egui::Event>,
        shift: bool,
        enabled: bool,
        ribbon: Option<usize>,
    ) -> Result<State, String> {
        events.insert(
            0,
            egui::Event::ModifiersChanged(egui::Modifiers {
                shift,
                ..Default::default()
            }),
        );
        let mut saved = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0))),
                events,
                ..Default::default()
            },
            |ui| {
                if !enabled {
                    ui.disable();
                }
                if let Some(ribbon) = ribbon {
                    let id = ui.id().with("sheets-editor");
                    let mut state = ui
                        .ctx()
                        .data_mut(|data| data.get_temp::<State>(id))
                        .unwrap_or_default();
                    state.ribbon_tab = ribbon;
                    ui.ctx().data_mut(|data| data.insert_temp(id, state));
                }
                show(ui, book);
                saved = ui
                    .ctx()
                    .data_mut(|data| data.get_temp::<State>(ui.id().with("sheets-editor")));
            },
        );
        output.textures_delta.clear();
        saved.ok_or_else(|| "Состояние редактора не сохранено".into())
    }
    #[test]
    fn text_events_accumulate_and_escape_keeps_original_cell() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.set_cell(book.active_id(), 0, 0, "original")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        let state = frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text("a".into()), egui::Event::Text("b".into())],
            false,
        )?;
        assert_eq!(state.buffer, "ab");
        let state = frame(&ctx, &mut book, vec![key(Key::Escape, false)], false)?;
        assert!(!state.editing);
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original")
        );
        Ok(())
    }
    #[test]
    fn typed_value_commits_before_enter_and_shift_navigation() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book {
            selection: Selection {
                row: 1,
                col: 1,
                anchor_row: 1,
                anchor_col: 1,
            },
            ..Default::default()
        };
        frame(&ctx, &mut book, Vec::new(), false)?;
        frame(&ctx, &mut book, vec![egui::Event::Text("42".into())], false)?;
        frame(&ctx, &mut book, vec![key(Key::Enter, true)], true)?;
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(1, 1))
                .map(|cell| cell.input.as_str()),
            Some("42")
        );
        assert_eq!((book.selection.row, book.selection.col), (0, 1));
        frame(&ctx, &mut book, vec![key(Key::Tab, true)], true)?;
        assert_eq!(
            (
                book.selection.row,
                book.selection.col,
                book.selection.anchor_row,
                book.selection.anchor_col
            ),
            (0, 0, 0, 0)
        );
        Ok(())
    }
    #[test]
    fn clicking_inside_formula_does_not_commit_draft_before_escape() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.set_cell(book.active_id(), 0, 0, "original")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        frame(&ctx, &mut book, vec![key(Key::F2, false)], false)?;
        let state = frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text("draft".into())],
            false,
        )?;
        assert!(state.editing);
        assert_ne!(state.buffer, "original");
        let formula = state.formula_rect.ok_or("Нет геометрии строки формул")?;
        assert!(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0)).contains_rect(formula),
            "Formula outside viewport: {formula:?}"
        );
        let position = formula.center();
        let state = frame(
            &ctx,
            &mut book,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            false,
        )?;
        assert!(state.editing);
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original")
        );
        let state = frame(
            &ctx,
            &mut book,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            false,
        )?;
        assert!(state.editing);
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original")
        );
        let state = frame(&ctx, &mut book, vec![key(Key::Escape, false)], false)?;
        assert!(!state.editing);
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original")
        );
        Ok(())
    }
    #[test]
    fn text_and_navigation_in_one_frame_commit_before_moving() -> Result<(), String> {
        for (navigation, backwards, expected) in [
            (Key::Enter, false, (2, 1)),
            (Key::Enter, true, (0, 1)),
            (Key::Tab, false, (1, 2)),
            (Key::Tab, true, (1, 0)),
        ] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx)?;
            let mut book = Book {
                selection: Selection {
                    row: 1,
                    col: 1,
                    anchor_row: 1,
                    anchor_col: 1,
                },
                ..Default::default()
            };
            frame(&ctx, &mut book, Vec::new(), false)?;
            let state = frame(
                &ctx,
                &mut book,
                vec![egui::Event::Text("42".into()), key(navigation, backwards)],
                backwards,
            )?;
            assert!(!state.editing);
            assert_eq!(
                book.active_sheet()
                    .and_then(|sheet| sheet.cell(1, 1))
                    .map(|cell| cell.input.as_str()),
                Some("42"),
            );
            assert_eq!((book.selection.row, book.selection.col), expected);
            assert_eq!(
                (book.selection.anchor_row, book.selection.anchor_col),
                expected
            );
        }
        Ok(())
    }
    #[test]
    fn text_and_escape_in_one_frame_leave_cell_unchanged() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.set_cell(book.active_id(), 0, 0, "original")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        let state = frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text("draft".into()), key(Key::Escape, false)],
            false,
        )?;
        assert!(!state.editing);
        assert_eq!(state.buffer, "original");
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original"),
        );
        frame(&ctx, &mut book, vec![key(Key::ArrowRight, false)], false)?;
        assert_eq!((book.selection.row, book.selection.col), (0, 1));
        Ok(())
    }
    #[test]
    fn disabled_grid_ignores_delete_paste_cut_and_text() -> Result<(), String> {
        for previously_focused in [false, true] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx)?;
            let mut book = Book::default();
            book.set_cell(book.active_id(), 0, 0, "original")?;
            if previously_focused {
                frame(&ctx, &mut book, Vec::new(), false)?;
            }
            let original = book.clone();
            for events in [
                vec![key(Key::Delete, false)],
                vec![egui::Event::Paste("replacement".into())],
                vec![egui::Event::Cut],
                vec![egui::Event::Text("draft".into()), key(Key::Enter, false)],
                vec![key(Key::ArrowRight, false)],
            ] {
                let state = frame_enabled(&ctx, &mut book, events, false, false)?;
                assert_eq!(book, original);
                assert!(!state.editing);
                assert!(ctx.memory(|memory| memory.focused().is_none()));
            }
        }
        Ok(())
    }
    #[test]
    fn disabling_formula_does_not_commit_pending_draft() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.set_cell(book.active_id(), 0, 0, "original")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text("draft".into())],
            false,
        )?;
        let original = book.clone();
        let state = frame_enabled(&ctx, &mut book, vec![key(Key::Enter, false)], false, false)?;
        assert_eq!(book, original);
        assert!(state.editing);
        assert_eq!(state.buffer, "draft");
        let state = frame_enabled(&ctx, &mut book, Vec::new(), false, false)?;
        assert_eq!(book, original);
        assert!(state.editing);
        assert_eq!(state.buffer, "draft");
        Ok(())
    }
    #[test]
    fn pending_flush_is_scoped_to_tab_and_retains_invalid_draft() -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut book = Book::default();
        let state_id = egui::Id::new(("sheets-editor", 7_u64));
        ctx.data_mut(|data| {
            data.insert_temp(
                state_id,
                State {
                    editing: true,
                    buffer: "42".into(),
                    last: (book.active_id(), 0, 0),
                    ..Default::default()
                },
            )
        });
        assert!(!commit_pending(&ctx, 8, &mut book)?);
        assert!(commit_pending(&ctx, 7, &mut book)?);
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.value.clone()),
            Some(Value::Number(42.0))
        );
        assert!(!commit_pending(&ctx, 7, &mut book)?);
        ctx.data_mut(|data| {
            data.insert_temp(
                state_id,
                State {
                    editing: true,
                    buffer: "draft".into(),
                    last: (999, 0, 0),
                    ..Default::default()
                },
            )
        });
        assert!(commit_pending(&ctx, 7, &mut book).is_err());
        let state = ctx
            .data_mut(|data| data.get_temp::<State>(state_id))
            .ok_or("Нет состояния")?;
        assert!(state.editing);
        assert_eq!(state.buffer, "draft");
        Ok(())
    }

    fn click_control(ctx: &egui::Context, book: &mut Book, rect: Rect) -> Result<State, String> {
        assert!(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0)).contains_rect(rect),
            "Control outside viewport: {rect:?}"
        );
        let position = rect.center();
        frame(
            ctx,
            book,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            false,
        )?;
        frame(
            ctx,
            book,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            false,
        )?;
        frame(ctx, book, Vec::new(), false)
    }
    #[test]
    fn invalid_formula_draft_survives_grid_sheet_and_new_sheet_clicks() -> Result<(), String> {
        for action in ["grid-b1", "other-sheet", "new-sheet", "toolbar-new-sheet"] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx)?;
            let mut book = Book::default();
            let sheet_id = book.active_id();
            book.set_cell(sheet_id, 0, 0, "original")?;
            let other_sheet = book.add_sheet("Second")?;
            let control = if action == "other-sheet" {
                format!("sheet-{other_sheet}")
            } else {
                action.to_string()
            };
            frame_with_ribbon(
                &ctx,
                &mut book,
                Vec::new(),
                false,
                true,
                (action == "toolbar-new-sheet").then_some(1),
            )?;
            let original = book.clone();
            let draft = "я".repeat(16384);
            let state = frame(
                &ctx,
                &mut book,
                vec![egui::Event::Text(draft.clone())],
                false,
            )?;
            assert!(state.editing, "{action}");
            assert_eq!(state.buffer, draft, "{action}");
            let state = frame(&ctx, &mut book, Vec::new(), false)?;
            let rect = state
                .test_controls
                .get(&control)
                .copied()
                .ok_or_else(|| format!("No geometry for {control}"))?;
            let state = click_control(&ctx, &mut book, rect)?;
            assert_eq!(book, original, "{action}");
            assert!(state.editing, "{action}");
            assert_eq!(state.last, (sheet_id, 0, 0), "{action}");
            assert_eq!(state.buffer, draft, "{action}");
            assert!(!state.error.is_empty(), "{action}");
            let state = frame(&ctx, &mut book, vec![key(Key::Escape, false)], false)?;
            assert!(!state.editing, "{action}");
            assert_eq!(state.buffer, "original", "{action}");
            assert_eq!(book, original, "{action}");
            frame(
                &ctx,
                &mut book,
                vec![egui::Event::Text("42".into()), key(Key::Enter, false)],
                false,
            )?;
            assert_eq!(
                book.sheet_by_id(sheet_id)
                    .and_then(|sheet| sheet.cell(0, 0))
                    .map(|cell| cell.input.as_str()),
                Some("42"),
                "{action}"
            );
        }
        Ok(())
    }
    #[test]
    fn selection_sync_retains_draft_when_commit_fails() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        let sheet = book.active_id();
        book.set_cell(sheet, 0, 0, "original")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        let draft = "x".repeat(32768);
        frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text(draft.clone())],
            false,
        )?;
        book.selection = Selection {
            row: 0,
            col: 1,
            anchor_row: 0,
            anchor_col: 1,
        };
        let state = frame(&ctx, &mut book, Vec::new(), false)?;
        assert!(state.editing);
        assert_eq!(state.buffer, draft);
        assert_eq!(state.last, (sheet, 0, 0));
        assert!(!state.error.is_empty());
        assert_eq!(
            book.sheet_by_id(sheet)
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original"),
        );
        assert!(
            book.sheet_by_id(sheet)
                .and_then(|sheet| sheet.cell(0, 1))
                .is_none()
        );
        Ok(())
    }
}
