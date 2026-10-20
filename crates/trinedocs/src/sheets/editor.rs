use super::model::{
    Align, Book, Cell, CellFormat, ChartKind, MAX_COLS, MAX_ROWS, Range, Selection, Sheet, Value,
    address,
};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};

type ImageJob =
    std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<Result<Option<Vec<u8>>, String>>>>;
#[derive(Clone, Default)]
struct State {
    editing: bool,
    buffer: String,
    last: (u64, u32, u32),
    error: String,
    find: String,
    replace: String,
    find_open: bool,
    find_case: bool,
    find_message: String,
    passport_open: bool,
    paste_transpose: bool,
    rename: String,
    ribbon_tab: usize,
    location: String,
    scroll_to_selection: bool,
    rename_sheet_id: Option<u64>,
    selected_image: Option<u64>,
    selected_chart: Option<u64>,
    chart_drag: Option<(u64, Vec2)>,
    image_job: Option<ImageJob>,
    image_target: Option<(u64, u32, u32)>,
    hint_selected: usize,
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
    let input = entry_text(
        book,
        state.last.0,
        state.last.1,
        state.last.2,
        &state.buffer,
    );
    let value = book
        .set_cell(state.last.0, state.last.1, state.last.2, &input)
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
fn compact_commands(ui: &mut egui::Ui, state_id: egui::Id, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .id_salt(state_id.with("commands"))
                .max_height(36.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        content(ui);
                    });
                });
        });
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
    compact_commands(ui, state_id, |ui| match state.ribbon_tab {
        0 => {
            if command_action(
                ui.button("Вставить").on_hover_text("Ctrl+V"),
                book,
                state,
                &mut changed,
            ) {
                crate::clipboard::paste(ui.ctx());
            }
            ui.menu_button("Буфер", |ui| {
                if command_action(ui.button("Копировать · Ctrl+C"), book, state, &mut changed)
                {
                    copy_range(ui, book, state, false);
                    ui.close();
                }
                if command_action(ui.button("Вырезать · Ctrl+X"), book, state, &mut changed)
                {
                    if copy_range(ui, book, state, true) {
                        changed |= clear(book, state);
                    }
                    ui.close();
                }
                ui.separator();
                ui.label("Специальная вставка из внутреннего буфера");
                ui.checkbox(&mut state.paste_transpose, "Транспонировать");
                for (label, mode) in [
                    ("Всё", PasteMode::All),
                    ("Только значения", PasteMode::Values),
                    ("Формулы и исходный ввод", PasteMode::Inputs),
                    ("Только форматы", PasteMode::Formats),
                ] {
                    let available = ui
                        .ctx()
                        .data(|data| data.get_temp::<SheetClipboard>(clipboard_id()).is_some());
                    if command_action(
                        ui.add_enabled(available, egui::Button::new(label)),
                        book,
                        state,
                        &mut changed,
                    ) {
                        let transpose = state.paste_transpose;
                        changed |= result(state, paste_special(ui.ctx(), book, mode, transpose));
                        ui.close();
                    }
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

            ui.horizontal(|ui| {
                let mut family = selected.font.clone();
                if crate::chrome::font_picker(ui, "sheets-font", &mut family, 116.0) {
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
                        changed |= result(state, apply_format(book, |format| format.size = size));
                    }
                }

                ui.push_id(state_id.with("quick-emphasis"), |ui| {
                    for (index, label, on, tip) in [
                        (
                            0,
                            egui::RichText::new("B").strong(),
                            selected.bold,
                            "Полужирный · Ctrl+B",
                        ),
                        (
                            1,
                            egui::RichText::new("I").italics(),
                            selected.italic,
                            "Курсив · Ctrl+I",
                        ),
                        (
                            2,
                            egui::RichText::new("U").underline(),
                            selected.underline,
                            "Подчёркнутый · Ctrl+U",
                        ),
                    ] {
                        if command_action(
                            crate::chrome::tool(ui, label, tip, on),
                            book,
                            state,
                            &mut changed,
                        ) {
                            changed |= result(
                                state,
                                apply_format(book, |format| match index {
                                    0 => format.bold = !on,
                                    1 => format.italic = !on,
                                    _ => format.underline = !on,
                                }),
                            );
                        }
                    }
                });
            });
            ui.menu_button("Формат", |ui| {
                ui.horizontal(|ui| {
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

                    if command_action(
                        crate::chrome::tool(
                            ui,
                            egui::RichText::new("S").strikethrough(),
                            "Зачёркнутый · Ctrl+5",
                            selected.strike,
                        ),
                        book,
                        state,
                        &mut changed,
                    ) {
                        changed |= result(
                            state,
                            apply_format(book, |format| format.strike = !selected.strike),
                        );
                    }
                });
                ui.horizontal(|ui| {
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
            ui.menu_button("Выравнивание", |ui| {
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
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
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
            ui.menu_button("Число", |ui| {
                let mut number = selected.number.clone();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
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
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
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
        }
        1 => {
            if command_action(
                ui.button("Оформить диапазон")
                    .on_hover_text("Заголовок, чередование строк и границы выделенных ячеек"),
                book,
                state,
                &mut changed,
            ) {
                changed |= result(state, table_format(book));
            }
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
                    if command_action(ui.button(chart_kind_name(kind)), book, state, &mut changed) {
                        changed |= result(
                            state,
                            book.add_chart(id, kind, book.selection.range(), "Диаграмма")
                                .map(|_| ()),
                        );
                        ui.close();
                    }
                }
            });
            if ui
                .add_enabled(state.image_job.is_none(), egui::Button::new("🖼 Рисунок…"))
                .on_hover_text("Вставить рисунок из файла поверх ячеек · или Ctrl+V")
                .clicked()
            {
                start_image_job(ui.ctx(), state, book);
            }
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
        }
        2 => {
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
                if ui.button("Паспорт формулы").clicked() {
                    state.passport_open = true;
                }
            });
        }
        _ => {
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
            ui.horizontal(|ui| {
                ui.menu_button("Размеры", |ui| {
                    let row = book.selection.row;
                    let col = book.selection.col;
                    let mut width = book
                        .active_sheet()
                        .map(|sheet| {
                            sheet
                                .widths
                                .get(&col)
                                .copied()
                                .unwrap_or(sheet.default_col_width)
                        })
                        .unwrap_or(96.0);
                    let mut height = book
                        .active_sheet()
                        .map(|sheet| {
                            sheet
                                .row_heights
                                .get(&row)
                                .copied()
                                .unwrap_or(sheet.default_row_height)
                        })
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
                        let locale_ru = book.locale_ru;
                        if let Some(sheet) = book.active_sheet_mut() {
                            let width = autofit_width(ui, sheet, col, locale_ru);
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
fn hide_axis(book: &mut Book, row: u32, col: u32, rows: bool) -> bool {
    let previous = (row, col);
    let Some(sheet) = book.active_sheet_mut() else {
        return false;
    };
    let (index, maximum, hidden) = if rows {
        (row, MAX_ROWS, &mut sheet.hidden_rows)
    } else {
        (col, MAX_COLS, &mut sheet.hidden_cols)
    };
    if !hidden.insert(index) {
        return false;
    }
    let next = visible_index(index, maximum, 1, hidden)
        .or_else(|| visible_index(index, maximum, -1, hidden));
    let Some(next) = next else {
        hidden.remove(&index);
        return false;
    };
    let requested = if rows { (next, col) } else { (row, next) };
    let (row, col) = jump_target(sheet, previous, requested);
    book.selection = Selection {
        row,
        col,
        anchor_row: row,
        anchor_col: col,
    };
    true
}
fn visibility_menu(ui: &mut egui::Ui, book: &mut Book, state: &mut State, changed: &mut bool) {
    for (label, rows) in [("Скрыть строку", true), ("Скрыть столбец", false)]
    {
        if command_action(ui.button(label), book, state, changed) {
            let row = book.selection.row;
            let col = book.selection.col;
            *changed |= hide_axis(book, row, col, rows);
            state.scroll_to_selection = true;
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
pub fn request_focus(ctx: &egui::Context, tab_id: u64, sheet_id: u64, row: u32, col: u32) {
    let id = egui::Id::new(("sheets-editor", tab_id)).with("focus-request");
    ctx.data_mut(|data| data.insert_temp(id, (sheet_id, row, col)));
    ctx.request_repaint();
}
fn apply_focus_request(book: &mut Book, state: &mut State, request: (u64, u32, u32)) -> bool {
    if state.editing {
        return false;
    }
    let (sheet_id, row, col) = request;
    let Some(index) = book.sheets.iter().position(|sheet| {
        sheet.id == sheet_id
            && !sheet.hidden
            && !sheet.hidden_rows.contains(&row)
            && !sheet.hidden_cols.contains(&col)
            && !sheet
                .merges
                .iter()
                .any(|range| range.contains(row, col) && (range.r0, range.c0) != (row, col))
            && sheet.cell(row, col).is_some()
    }) else {
        return false;
    };
    book.active = index;
    book.selection = Selection {
        row,
        col,
        anchor_row: row,
        anchor_col: col,
    };
    state.last = (sheet_id, row, col);
    state.buffer = book.sheets[index]
        .cell(row, col)
        .map(|cell| cell.input.clone())
        .unwrap_or_default();
    state.selected_image = None;
    state.selected_chart = None;
    state.scroll_to_selection = true;
    state.error.clear();
    true
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
/// Hint for the formula bar text at its current caret.
fn formula_hint(ctx: &egui::Context, id: egui::Id, text: &str) -> super::hints::Hint {
    let caret = egui::text_edit::TextEditState::load(ctx, id)
        .and_then(|edit| edit.cursor.char_range())
        .map_or_else(|| text.chars().count(), |range| range.primary.index.0);
    super::hints::hint(text, caret, 8)
}
/// Signature of the call around the caret (current argument in bold) and name completions.
fn show_formula_popup(
    ui: &egui::Ui,
    state_id: egui::Id,
    anchor: Rect,
    hint: &super::hints::Hint,
    selected: usize,
) {
    let accent = crate::chrome::accent(ui);
    let muted = crate::chrome::muted(ui);
    egui::Area::new(state_id.with("formula-hint"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor.left_bottom() + Vec2::new(0.0, 4.0))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(520.0);
                if let Some((info, arg)) = hint.call {
                    let (name, params) = super::hints::parameters(info.signature);
                    let current = super::hints::parameter_index(&params, arg);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(egui::RichText::new(format!("{name}(")).monospace());
                        for (i, param) in params.iter().enumerate() {
                            if i > 0 {
                                ui.label(egui::RichText::new(", ").monospace());
                            }
                            let text = egui::RichText::new(*param).monospace();
                            ui.label(if Some(i) == current {
                                text.strong().color(accent)
                            } else {
                                text
                            });
                        }
                        ui.label(egui::RichText::new(")").monospace());
                    });
                    if hint.completions.is_empty() {
                        ui.label(egui::RichText::new(info.description).small().color(muted));
                    }
                }
                if !hint.completions.is_empty() {
                    if hint.call.is_some() {
                        ui.separator();
                    }
                    for (i, completion) in hint.completions.iter().enumerate() {
                        let chosen = i == selected;
                        let row = egui::RichText::new(completion.insert).monospace();
                        ui.horizontal(|ui| {
                            ui.label(if chosen {
                                row.strong().color(accent)
                            } else {
                                row
                            });
                            if completion.insert != completion.info.name {
                                ui.label(
                                    egui::RichText::new(completion.info.name)
                                        .small()
                                        .color(muted),
                                );
                            }
                            ui.label(
                                egui::RichText::new(completion.info.category)
                                    .small()
                                    .color(muted),
                            );
                        });
                        if chosen {
                            ui.label(
                                egui::RichText::new(completion.info.signature)
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(completion.info.description)
                                    .small()
                                    .color(muted),
                            );
                        }
                    }
                    ui.label(
                        egui::RichText::new("↑↓ — выбор · Tab — вставить")
                            .small()
                            .color(muted),
                    );
                }
            });
        });
}
fn show_impl(ui: &mut egui::Ui, book: &mut Book, state_id: egui::Id) -> bool {
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id))
        .unwrap_or_default();
    let mut changed = false;
    if let Some(request) = ui
        .ctx()
        .data_mut(|data| data.remove_temp::<(u64, u32, u32)>(state_id.with("focus-request")))
        && apply_focus_request(book, &mut state, request)
        && ui.is_enabled()
    {
        ui.memory_mut(|memory| memory.request_focus(state_id.with("canvas")));
    }
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
        if state.last.0 != id {
            state.selected_image = None;
            state.selected_chart = None;
        }
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
        if state.last.0 != id {
            state.selected_image = None;
            state.selected_chart = None;
        }
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
    ui.add_space(4.0);
    let mut formula_popup: Option<(Rect, super::hints::Hint)> = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
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
                    .desired_width(96.0)
                    .frame(egui::Frame::NONE)
                    .id(location_id),
            )
            .on_hover_text("Адрес ячейки или диапазона · например A1:D8");
        ui.painter().hline(
            location.rect.x_range(),
            location.rect.bottom(),
            Stroke::new(
                if location.has_focus() { 1.5 } else { 0.5 },
                if location.has_focus() {
                    crate::chrome::accent(ui)
                } else {
                    crate::chrome::border(ui)
                },
            ),
        );
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
        // Function completion: ↑/↓ choose, Tab inserts the name and "(".
        let before = if focused && ui.is_enabled() {
            formula_hint(ui.ctx(), formula_id, &state.buffer)
        } else {
            super::hints::Hint::default()
        };
        let count = before.completions.len();
        state.hint_selected = state.hint_selected.min(count.saturating_sub(1));
        if count > 0 {
            let mut accept = false;
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) {
                    state.hint_selected = (state.hint_selected + 1) % count;
                }
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) {
                    state.hint_selected = (state.hint_selected + count - 1) % count;
                }
                accept = i.consume_key(egui::Modifiers::NONE, Key::Tab);
            });
            if accept
                && let (Some(word), Some(completion)) =
                    (before.word, before.completions.get(state.hint_selected))
            {
                let (text, caret) = super::hints::accept(&state.buffer, word, completion);
                state.buffer = text;
                state.editing = true;
                state.hint_selected = 0;
                let mut edit =
                    egui::text_edit::TextEditState::load(ui.ctx(), formula_id).unwrap_or_default();
                edit.cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(caret),
                    )));
                edit.store(ui.ctx(), formula_id);
            }
        }
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
                .frame(egui::Frame::NONE)
                .id(formula_id)
                .event_filter(egui::EventFilter {
                    tab: true,
                    escape: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                })
                .hint_text("Введите значение или формулу =SUM(A1:A10)"),
        );
        ui.painter().hline(
            response.rect.x_range(),
            response.rect.bottom(),
            Stroke::new(
                if response.has_focus() { 1.5 } else { 0.5 },
                if response.has_focus() {
                    crate::chrome::accent(ui)
                } else {
                    crate::chrome::border(ui)
                },
            ),
        );
        #[cfg(test)]
        {
            state.formula_rect = Some(response.rect);
        }
        if response.changed() {
            state.editing = true;
            state.hint_selected = 0;
        }
        if response.has_focus() && action.is_none() {
            let hint = formula_hint(ui.ctx(), formula_id, &state.buffer);
            if !hint.completions.is_empty() || hint.call.is_some() {
                formula_popup = Some((response.rect, hint));
            }
        }
        if let Some(key) = action {
            if key == Key::Escape {
                cancel_edit(book, &mut state);
            } else {
                changed |= commit(book, &mut state);
                if !state.editing {
                    advance(book, key, modifiers.shift);
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
    if let Some((anchor, hint)) = formula_popup {
        show_formula_popup(ui, state_id, anchor, &hint, state.hint_selected);
    }
    if book
        .active_sheet()
        .is_some_and(|sheet| sheet.frozen_rows > 20 || sheet.frozen_cols > 10)
    {
        ui.small("Холст отображает закрепление до 20 строк и 10 столбцов. Полная область сохраняется в файле.");
    }
    if !state.error.is_empty() {
        ui.colored_label(Color32::from_rgb(200, 60, 50), &state.error);
    }
    if close_find_on_escape(ui, &mut state) {
        ui.memory_mut(|memory| memory.request_focus(canvas_id));
    }
    if state.find_open {
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(8.0)
            .corner_radius(5.0)
            .show(ui, |ui| {
                let find_id = state_id.with("find");
                let search_key = ui.is_enabled()
                    && ui.memory(|m| m.has_focus(find_id))
                    && ui.input_mut(|input| input.consume_key(input.modifiers, Key::Enter));
                let backwards = ui.input(|input| input.modifiers.shift);
                let mut direction = None;
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Найти");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut state.find)
                                .desired_width(150.0)
                                .id(find_id)
                                .hint_text("Текст или формула"),
                        )
                        .changed()
                    {
                        state.find_message.clear();
                    }
                    if ui.button("← Назад").on_hover_text("Shift+Enter").clicked() {
                        direction = Some(true);
                    }
                    if ui.button("Далее →").on_hover_text("Enter").clicked() {
                        direction = Some(false);
                    }
                    if ui
                        .checkbox(&mut state.find_case, "Учитывать регистр")
                        .changed()
                    {
                        state.find_message.clear();
                    }
                    if ui.button("Закрыть").on_hover_text("Esc").clicked() {
                        state.find_open = false;
                        ui.memory_mut(|memory| memory.request_focus(canvas_id));
                    }
                });
                if search_key {
                    direction = Some(backwards);
                }
                if let Some(backwards) = direction {
                    changed |= commit(book, &mut state);
                    if !state.editing {
                        find_step(book, &mut state, backwards);
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Заменить на");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.replace)
                            .desired_width(150.0)
                            .id(state_id.with("replace"))
                            .hint_text("Новый текст"),
                    );
                    if command_action(
                        ui.add_enabled(!state.find.is_empty(), egui::Button::new("Заменить все")),
                        book,
                        &mut state,
                        &mut changed,
                    ) {
                        match book.replace_all_with_case(
                            id,
                            &state.find,
                            &state.replace,
                            state.find_case,
                        ) {
                            Ok(count) => {
                                changed |= count > 0;
                                state.error.clear();
                                state.find_message = format!("Изменено ячеек: {count}");
                            }
                            Err(error) => state.error = error,
                        }
                    }
                    ui.small("Замена изменяет исходный текст ячеек.");
                });
                if !state.find_message.is_empty() {
                    ui.small(&state.find_message);
                }
            });
    }
    let loaded = ui
        .is_enabled()
        .then_some(())
        .and(state.image_job.as_ref())
        .and_then(|job| match job.lock() {
            Ok(receiver) => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Загрузка изображения прервана".into()))
                }
            },
            Err(_) => Some(Err("Ошибка загрузки изображения".into())),
        });
    if state.image_job.is_some() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
    let pasted = if ui.is_enabled() {
        crate::clipboard::take_image(ui.ctx()).map(|png| png.to_vec())
    } else {
        None
    };
    let image_target = if loaded.is_some() {
        state.image_target.take()
    } else {
        None
    };
    let picture = match loaded {
        Some(result) => {
            state.image_job = None;
            match result {
                Ok(bytes) => bytes,
                Err(error) => {
                    state.error = error;
                    None
                }
            }
        }
        None => pasted,
    };
    if let Some(bytes) = picture {
        let (sheet, row, col) =
            image_target.unwrap_or((book.active_id(), book.selection.row, book.selection.col));
        match book.add_image(sheet, row, col, bytes) {
            Ok(image) => {
                state.selected_image = (book.active_id() == sheet).then_some(image);
                changed = true;
            }
            Err(error) => state.error = error,
        }
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
            let (width, height) = grid_size(sheet, rows, cols);
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
            let grid = crate::chrome::border(ui).gamma_multiply(0.7);
            painter.rect_filled(rect, 0.0, bg);
            let c0 = axis_index(
                (viewport.min.x - 48.0).max(0.0),
                cols,
                sheet.default_col_width,
                &sheet.widths,
                &sheet.hidden_cols,
            );
            let r0 = axis_index(
                (viewport.min.y - 28.0).max(0.0),
                rows,
                sheet.default_row_height,
                &sheet.row_heights,
                &sheet.hidden_rows,
            );
            let c1 = axis_index(
                viewport.max.x,
                cols,
                sheet.default_col_width,
                &sheet.widths,
                &sheet.hidden_cols,
            )
            .saturating_add(2)
            .min(cols);
            let r1 = axis_index(
                viewport.max.y,
                rows,
                sheet.default_row_height,
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
            let cells = visible_cells(
                sheet,
                origin,
                viewport,
                r0..r1,
                c0..c1,
                (frozen_r, frozen_c),
                ui.clip_rect(),
            );
            for (r, c) in cells {
                if sheet
                    .merges
                    .iter()
                    .any(|merge| merge.contains(r, c) && (merge.r0, merge.c0) != (r, c))
                {
                    continue;
                }
                let cell_rect = cell_area(sheet, origin, viewport, r, c, frozen_r, frozen_c);
                if !ui.clip_rect().intersects(cell_rect) {
                    continue;
                }
                let cell = sheet.cell(r, c);
                let fmt = cell.map(|c| c.fmt.clone()).unwrap_or_default();
                let conditional = sheet
                    .conditional
                    .iter()
                    .find(|rule| {
                        rule.range.contains(r, c)
                            && sheet
                                .value(r, c)
                                .and_then(|v| v.number().ok())
                                .is_some_and(|n| match rule.operator.as_str() {
                                    ">" => n > rule.threshold,
                                    "<" => n < rule.threshold,
                                    "=" => n == rule.threshold,
                                    ">=" => n >= rule.threshold,
                                    "<=" => n <= rule.threshold,
                                    _ => false,
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
                let number_color =
                    cell.and_then(|cell| super::format::color(&cell.value, &fmt.number));
                let text_color = if let Some(rgb) = number_color {
                    color(rgb)
                } else if ui.visuals().dark_mode && fmt.color == [28, 32, 38] {
                    ui.visuals().text_color()
                } else {
                    color(fmt.color)
                };
                let clipped = painter.with_clip_rect(cell_rect.shrink(2.0));
                clipped.add(cell_text_shape(
                    ui, cell_rect, &fmt, text, text_color, right,
                ));
                if (r, c) == (row, col) {
                    painter.rect_stroke(
                        cell_rect,
                        0.0,
                        Stroke::new(2.0, crate::chrome::accent(ui)),
                        StrokeKind::Inside,
                    );
                }
            }
            if response.clicked() {
                state.selected_image = None;
                state.selected_chart = None;
            }
            let mut chart_edit = None;
            let mut chart_clicked = false;
            for chart in &sheet.charts {
                let chart_rect = floating_chart_rect(sheet, origin, chart);
                if !ui.clip_rect().intersects(chart_rect) {
                    continue;
                }
                paint_chart(ui, sheet, chart, chart_rect);
                #[cfg(test)]
                state
                    .test_controls
                    .insert(format!("chart-{}", chart.id), chart_rect);
                let response = ui.interact(
                    chart_rect,
                    state_id.with(("chart", chart.id)),
                    Sense::click_and_drag(),
                );
                if response.clicked() || response.drag_started() {
                    state.selected_chart = Some(chart.id);
                    state.selected_image = None;
                    chart_clicked = true;
                    ui.memory_mut(|memory| memory.request_focus(canvas_id));
                }
                if response.drag_started()
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    state.chart_drag = Some((chart.id, pointer - chart_rect.min));
                }
                if response.dragged()
                    && let Some((id, offset)) = state.chart_drag
                    && id == chart.id
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    chart_edit = Some((
                        chart.id,
                        Rect::from_min_size(pointer - offset, chart_rect.size()),
                    ));
                }
                if response.drag_stopped() {
                    state.chart_drag = None;
                }
                if state.selected_chart == Some(chart.id) {
                    let accent = crate::chrome::accent(ui);
                    painter.rect_stroke(
                        chart_rect,
                        0.0,
                        Stroke::new(1.5, accent),
                        StrokeKind::Outside,
                    );
                    let handle = Rect::from_center_size(chart_rect.max, Vec2::splat(9.0));
                    painter.rect_filled(handle, 1.0, accent);
                    let handle_response = ui
                        .interact(
                            handle,
                            state_id.with(("chart-size", chart.id)),
                            Sense::drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
                    if handle_response.dragged() {
                        let size = (chart_rect.size() + handle_response.drag_delta())
                            .clamp(Vec2::splat(1.0), Vec2::splat(4000.0));
                        chart_edit = Some((chart.id, Rect::from_min_size(chart_rect.min, size)));
                    }
                }
            }
            if chart_clicked || chart_edit.is_some() {
                changed |= commit(book, &mut state);
            }
            if !state.editing
                && let Some((chart, target)) = chart_edit
                && let Some(sheet) = book.active_sheet_mut()
            {
                changed |= place_chart(sheet, chart, target.min - origin, target.size());
            }
            let Some(sheet) = book.active_sheet() else {
                return;
            };
            let mut image_edit = None;
            for image in &sheet.images {
                let image_rect = Rect::from_min_size(
                    origin
                        + Vec2::new(
                            48.0 + axis_pos(
                                image.col,
                                sheet.default_col_width,
                                &sheet.widths,
                                &sheet.hidden_cols,
                            ) + image.x_offset,
                            28.0 + axis_pos(
                                image.row,
                                sheet.default_row_height,
                                &sheet.row_heights,
                                &sheet.hidden_rows,
                            ) + image.y_offset,
                        ),
                    Vec2::new(image.width, image.height),
                );
                if !ui.clip_rect().intersects(image_rect) {
                    continue;
                }
                paint_image(ui, &painter, sheet.id, image, image_rect);
                let image_response = ui.interact(
                    image_rect,
                    state_id.with(("image", image.id)),
                    Sense::click_and_drag(),
                );
                if image_response.clicked() || image_response.drag_started() {
                    state.selected_image = Some(image.id);
                    state.selected_chart = None;
                    response.request_focus();
                }
                if image_response.dragged() {
                    image_edit =
                        Some((image.id, image_rect.translate(image_response.drag_delta())));
                }
                if state.selected_image == Some(image.id) {
                    let accent = crate::chrome::accent(ui);
                    painter.rect_stroke(
                        image_rect,
                        0.0,
                        Stroke::new(1.5, accent),
                        StrokeKind::Outside,
                    );
                    let handle = Rect::from_center_size(image_rect.max, Vec2::splat(10.0));
                    painter.rect_filled(handle, 2.0, accent);
                    let handle_response = ui
                        .interact(
                            handle,
                            state_id.with(("image-size", image.id)),
                            Sense::drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
                    if handle_response.dragged() {
                        let size = constrained_image_size(
                            image_rect.size(),
                            image_rect.size() + handle_response.drag_delta(),
                            ui.input(|input| input.modifiers.shift),
                        );
                        image_edit = Some((image.id, Rect::from_min_size(image_rect.min, size)));
                    }
                }
            }
            if let Some((image, target)) = image_edit
                && let Some(s) = book.active_sheet_mut()
            {
                place_image(s, image, target.min - origin, target.size());
                changed = true;
            }
            let Some(sheet) = book.active_sheet() else {
                return;
            };
            let header_y = origin.y + viewport.min.y;
            let headers: Vec<_> = visible_axis(c0..c1, frozen_c)
                .into_iter()
                .filter(|c| !sheet.hidden_cols.contains(c))
                .map(|c| {
                    (
                        c,
                        cell_rect(sheet, origin, viewport, 0, c, frozen_r, frozen_c).left(),
                        sheet
                            .widths
                            .get(&c)
                            .copied()
                            .unwrap_or(sheet.default_col_width),
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
                    let old = s.widths.get(&c).copied().unwrap_or(s.default_col_width);
                    s.widths
                        .insert(c, (old + res.drag_delta().x).clamp(24.0, 1000.0));
                    changed = true;
                }
            }
            let Some(sheet) = book.active_sheet() else {
                return;
            };
            for r in visible_axis(r0..r1, frozen_r) {
                if sheet.hidden_rows.contains(&r) {
                    continue;
                }
                let y = cell_rect(sheet, origin, viewport, r, 0, frozen_r, frozen_c).top();
                let h = sheet
                    .row_heights
                    .get(&r)
                    .copied()
                    .unwrap_or(sheet.default_row_height);
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
                        sheet.default_row_height,
                        &sheet.row_heights,
                        &sheet.hidden_rows,
                    )
                    .min(rows - 1);
                    let c = axis_index(
                        local.x - 48.0,
                        cols,
                        sheet.default_col_width,
                        &sheet.widths,
                        &sheet.hidden_cols,
                    )
                    .min(cols - 1);
                    let r = if local.y - viewport.min.y
                        < 28.0
                            + axis_pos(
                                frozen_r,
                                sheet.default_row_height,
                                &sheet.row_heights,
                                &sheet.hidden_rows,
                            ) {
                        axis_index(
                            local.y - viewport.min.y - 28.0,
                            rows,
                            sheet.default_row_height,
                            &sheet.row_heights,
                            &sheet.hidden_rows,
                        )
                    } else {
                        r
                    };
                    let c = if local.x - viewport.min.x
                        < 48.0
                            + axis_pos(
                                frozen_c,
                                sheet.default_col_width,
                                &sheet.widths,
                                &sheet.hidden_cols,
                            ) {
                        axis_index(
                            local.x - viewport.min.x - 48.0,
                            cols,
                            sheet.default_col_width,
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
                            let cut = matches!(event, egui::Event::Cut);
                            if copy_range(ui, book, &mut state, cut) && cut {
                                changed |= clear(book, &mut state);
                            }
                        }
                        egui::Event::Paste(text) if state.editing => {
                            state.buffer.push_str(&text);
                        }
                        egui::Event::Paste(text) => {
                            let (row, col) = (book.selection.row, book.selection.col);
                            changed |= result(
                                &mut state,
                                paste_clipboard(ui.ctx(), book, id, row, col, &text),
                            );
                        }
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
                                        advance(book, key, modifiers.shift);
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
                                if let Some(image) = state.selected_image.take() {
                                    changed |= result(&mut state, book.delete_image(id, image));
                                } else if let Some(chart) = state.selected_chart.take() {
                                    if let Some(sheet) = book.active_sheet_mut() {
                                        let previous = sheet.charts.len();
                                        sheet.charts.retain(|item| item.id != chart);
                                        changed |= previous != sheet.charts.len();
                                    }
                                } else {
                                    changed |= clear(book, &mut state);
                                }
                                continue;
                            }
                            if key == Key::F2 {
                                state.editing = true;
                                ui.memory_mut(|m| m.request_focus(state_id.with("formula")));
                                continue;
                            }
                            if key == Key::Enter || key == Key::Tab {
                                advance(book, key, modifiers.shift);
                                state.scroll_to_selection = true;
                                continue;
                            }
                            if modifiers.ctrl && key == Key::A {
                                select_current_region(book);
                                continue;
                            }
                            if key == Key::Space && (modifiers.ctrl || modifiers.shift) {
                                // Ctrl+Space selects whole columns, Shift+Space whole rows.
                                let range = book.selection.range();
                                let selection = &mut book.selection;
                                if modifiers.ctrl {
                                    selection.anchor_row = 0;
                                    selection.row = MAX_ROWS - 1;
                                    selection.anchor_col = range.c0;
                                    selection.col = range.c1;
                                } else {
                                    selection.anchor_col = 0;
                                    selection.col = MAX_COLS - 1;
                                    selection.anchor_row = range.r0;
                                    selection.row = range.r1;
                                }
                                continue;
                            }
                            let edge = |book: &Book, d_row: i8, d_col: i8| {
                                book.active_sheet()
                                    .map(|sheet| {
                                        sheet.data_edge(
                                            book.selection.row,
                                            book.selection.col,
                                            d_row,
                                            d_col,
                                        )
                                    })
                                    .unwrap_or((book.selection.row, book.selection.col))
                            };
                            let jump = if modifiers.ctrl {
                                match key {
                                    Key::ArrowUp => Some(edge(book, -1, 0)),
                                    Key::ArrowDown => Some(edge(book, 1, 0)),
                                    Key::ArrowLeft => Some(edge(book, 0, -1)),
                                    Key::ArrowRight => Some(edge(book, 0, 1)),
                                    _ => None,
                                }
                            } else {
                                None
                            };
                            let step = if !modifiers.ctrl {
                                let delta = match key {
                                    Key::ArrowUp => Some((-1, 0)),
                                    Key::ArrowDown => Some((1, 0)),
                                    Key::ArrowLeft => Some((0, -1)),
                                    Key::ArrowRight => Some((0, 1)),
                                    _ => None,
                                };
                                delta.and_then(|(dr, dc)| {
                                    book.active_sheet().map(|sheet| {
                                        step_cell(
                                            sheet,
                                            book.selection.row,
                                            book.selection.col,
                                            dr,
                                            dc,
                                        )
                                    })
                                })
                            } else {
                                None
                            };
                            let previous = (book.selection.row, book.selection.col);
                            let selection = &mut book.selection;
                            match key {
                                _ if jump.is_some() => {
                                    if let Some((r, c)) = jump {
                                        selection.row = r;
                                        selection.col = c;
                                    }
                                }
                                _ if step.is_some() => {
                                    if let Some((r, c)) = step {
                                        selection.row = r;
                                        selection.col = c;
                                    }
                                }
                                Key::ArrowUp => selection.row = selection.row.saturating_sub(1),
                                Key::ArrowDown => {
                                    selection.row = (selection.row + 1).min(MAX_ROWS - 1)
                                }
                                Key::ArrowLeft => selection.col = selection.col.saturating_sub(1),
                                Key::ArrowRight => {
                                    selection.col = (selection.col + 1).min(MAX_COLS - 1)
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
                            let requested = (selection.row, selection.col);
                            let target = book
                                .active_sheet()
                                .map_or(previous, |sheet| jump_target(sheet, previous, requested));
                            book.selection.row = target.0;
                            book.selection.col = target.1;
                            if !modifiers.shift {
                                book.selection.anchor_row = target.0;
                                book.selection.anchor_col = target.1;
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
                    let active = book.active == index;
                    let response = ui.add(
                        egui::Button::new(if active {
                            egui::RichText::new(&name).strong()
                        } else {
                            egui::RichText::new(&name)
                        })
                        .selected(active)
                        .frame_when_inactive(false)
                        .min_size(egui::vec2(48.0, 28.0)),
                    );
                    if active {
                        ui.painter().hline(
                            response.rect.x_range(),
                            response.rect.bottom(),
                            Stroke::new(2.0, crate::chrome::accent(ui)),
                        );
                    }
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
                let response = ui
                    .add(
                        egui::Button::new("+")
                            .frame_when_inactive(false)
                            .min_size(egui::vec2(28.0, 28.0)),
                    )
                    .on_hover_text("Добавить лист");
                #[cfg(test)]
                state
                    .test_controls
                    .insert("new-sheet".into(), response.rect);
                if response.clicked() {
                    changed |= add_sheet(book, &mut state);
                }
            });
        });
    let range = book.selection.range();
    if statistics_range_valid(range)
        && let Some(sheet) = book.active_sheet()
    {
        let (sum, mean, nonempty) = selection_statistics(sheet, range);
        ui.horizontal_wrapped(|ui| {
            ui.small(format!(
                "Выделено: {}:{}  ·  Заполнено: {nonempty}",
                address(range.r0, range.c0),
                address(range.r1, range.c1)
            ));
            ui.separator();
            ui.small(format!(
                "Сумма: {}  ·  Среднее: {}",
                statistic_text(sum, book.locale_ru),
                statistic_text(mean, book.locale_ru)
            ));
        });
    }
    if state.passport_open {
        let mut open = true;
        egui::Window::new("Паспорт формулы")
            .id(state_id.with("formula-passport"))
            .open(&mut open)
            .default_width(560.0)
            .show(ui.ctx(), |ui| {
                ui.label("Сохранённые значения; пересчёт и изменение книги не выполняются.");
                match super::formula::explain(
                    book,
                    book.active_id(),
                    book.selection.row,
                    book.selection.col,
                ) {
                    Ok(passport) => {
                        let report = serde_json::to_string_pretty(&passport).unwrap_or_default();
                        if ui.button("Копировать отчёт").clicked() {
                            ui.ctx().copy_text(report.clone());
                        }
                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(egui::RichText::new(report).monospace())
                                        .selectable(true),
                                );
                            });
                    }
                    Err(error) => {
                        ui.label(error);
                    }
                }
            });
        state.passport_open = open;
    }
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
fn entry_text(book: &Book, sheet: u64, row: u32, col: u32, input: &str) -> String {
    if !input.is_empty()
        && !input.starts_with('\'')
        && book
            .sheet_by_id(sheet)
            .and_then(|sheet| sheet.cell(row, col))
            .is_some_and(|cell| cell.fmt.number == "@")
    {
        format!("'{input}")
    } else {
        input.into()
    }
}
fn paste_values(
    book: &mut Book,
    sheet: u64,
    row: u32,
    col: u32,
    rows: &[Vec<String>],
) -> Result<(), String> {
    let values: Vec<Vec<String>> = rows
        .iter()
        .enumerate()
        .map(|(r, line)| {
            line.iter()
                .enumerate()
                .map(|(c, input)| {
                    entry_text(
                        book,
                        sheet,
                        row.saturating_add(r as u32),
                        col.saturating_add(c as u32),
                        input,
                    )
                })
                .collect()
        })
        .collect();
    book.set_range(sheet, row, col, &values)
}
fn visible_index(mut index: u32, count: u32, direction: i8, hidden: &BTreeSet<u32>) -> Option<u32> {
    while hidden.contains(&index) {
        index = if direction < 0 {
            index.checked_sub(1)?
        } else {
            index.checked_add(1).filter(|value| *value < count)?
        };
    }
    Some(index)
}
fn jump_target(sheet: &Sheet, previous: (u32, u32), requested: (u32, u32)) -> (u32, u32) {
    let axis = |value, old, max, hidden: &BTreeSet<u32>| {
        let direction = if value < old { -1 } else { 1 };
        visible_index(value, max, direction, hidden)
            .or_else(|| visible_index(value, max, -direction, hidden))
    };
    let Some(row) = axis(requested.0, previous.0, MAX_ROWS, &sheet.hidden_rows) else {
        return previous;
    };
    let Some(col) = axis(requested.1, previous.1, MAX_COLS, &sheet.hidden_cols) else {
        return previous;
    };
    let target = sheet
        .merges
        .iter()
        .find(|range| range.contains(row, col))
        .map_or((row, col), |range| (range.r0, range.c0));
    // A merged anchor remains the logical edit target when another part of its area is visible.
    target
}
fn step_cell(sheet: &Sheet, row: u32, col: u32, dr: i8, dc: i8) -> (u32, u32) {
    let merge = sheet.merges.iter().find(|range| range.contains(row, col));
    let base_row = merge.map_or(row, |range| if dr > 0 { range.r1 } else { range.r0 });
    let base_col = merge.map_or(col, |range| if dc > 0 { range.c1 } else { range.c0 });
    let r = if dr < 0 {
        base_row.saturating_sub(1)
    } else if dr > 0 {
        base_row.saturating_add(1).min(MAX_ROWS - 1)
    } else {
        row
    };
    let c = if dc < 0 {
        base_col.saturating_sub(1)
    } else if dc > 0 {
        base_col.saturating_add(1).min(MAX_COLS - 1)
    } else {
        col
    };
    let Some(r) = visible_index(r, MAX_ROWS, dr, &sheet.hidden_rows) else {
        return (row, col);
    };
    let Some(c) = visible_index(c, MAX_COLS, dc, &sheet.hidden_cols) else {
        return (row, col);
    };
    sheet
        .merges
        .iter()
        .find(|range| range.contains(r, c))
        .map_or((r, c), |range| (range.r0, range.c0))
}
fn advance(book: &mut Book, key: Key, backwards: bool) {
    let direction = if backwards { -1 } else { 1 };
    let delta = if key == Key::Enter {
        (direction, 0)
    } else {
        (0, direction)
    };
    let current = (book.selection.row, book.selection.col);
    let (row, col) = book
        .active_sheet()
        .map(|sheet| step_cell(sheet, current.0, current.1, delta.0, delta.1))
        .unwrap_or(current);
    book.selection = Selection {
        row,
        col,
        anchor_row: row,
        anchor_col: col,
    };
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
#[derive(Clone, Default)]
struct SheetClipboard {
    text: String,
    cells: Vec<Vec<Option<Cell>>>,
    row: u32,
    col: u32,
    cut: bool,
    merges: Vec<Range>,
}
fn clipboard_id() -> egui::Id {
    egui::Id::new("sheets-typed-clipboard")
}
fn capture_clipboard(book: &Book, cut: bool) -> Result<SheetClipboard, String> {
    let mut range = book.selection.range();
    let selected_range = range;
    let id = book.active_id();
    let sheet = book.sheet_by_id(id).ok_or("Лист не найден")?;
    loop {
        let old = range;
        for merge in &sheet.merges {
            if merge.r0 <= range.r1
                && range.r0 <= merge.r1
                && merge.c0 <= range.c1
                && range.c0 <= merge.c1
            {
                range = Range::new(
                    range.r0.min(merge.r0),
                    range.c0.min(merge.c0),
                    range.r1.max(merge.r1),
                    range.c1.max(merge.c1),
                );
            }
        }
        range.validate()?;
        if range == old {
            break;
        }
    }
    if cut && range != selected_range {
        return Err("Для вырезания выделите объединённые области полностью".into());
    }
    let rows = book.read_range(id, range.r0, range.c0, range.r1, range.c1)?;
    let merges = sheet
        .merges
        .iter()
        .filter(|merge| range.contains(merge.r0, merge.c0) && range.contains(merge.r1, merge.c1))
        .map(|merge| {
            Range::new(
                merge.r0 - range.r0,
                merge.c0 - range.c0,
                merge.r1 - range.r0,
                merge.c1 - range.c0,
            )
        })
        .collect();
    Ok(SheetClipboard {
        text: super::io::to_csv(&rows, '\t'),
        cells: (range.r0..=range.r1)
            .map(|row| {
                (range.c0..=range.c1)
                    .map(|col| sheet.cell(row, col).cloned())
                    .collect()
            })
            .collect(),
        row: range.r0,
        col: range.c0,
        cut,
        merges,
    })
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PasteMode {
    All,
    Values,
    Inputs,
    Formats,
}
fn value_input(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => number.to_string(),
        Value::Text(text) => format!("'{text}"),
        Value::Bool(_) | Value::Error(_) => value.display(),
    }
}
fn paste_cells(
    book: &mut Book,
    sheet: u64,
    row: u32,
    col: u32,
    clipboard: &SheetClipboard,
) -> Result<(), String> {
    paste_cells_with(book, sheet, row, col, clipboard, PasteMode::All, false)
}
fn paste_cells_with(
    book: &mut Book,
    sheet: u64,
    row: u32,
    col: u32,
    clipboard: &SheetClipboard,
    mode: PasteMode,
    transpose: bool,
) -> Result<(), String> {
    let source_rows = clipboard.cells.len();
    let source_cols = clipboard.cells.first().map(Vec::len).unwrap_or(0);
    if source_rows == 0 || source_cols == 0 {
        return Ok(());
    }
    if clipboard.cells.iter().any(|line| line.len() != source_cols)
        || clipboard.merges.iter().any(|merge| {
            merge.r0 > merge.r1
                || merge.c0 > merge.c1
                || merge.r1 >= source_rows as u32
                || merge.c1 >= source_cols as u32
        })
    {
        return Err("Недопустимый внутренний буфер".into());
    }
    let (rows, cols) = if transpose {
        (source_cols, source_rows)
    } else {
        (source_rows, source_cols)
    };
    if u64::from(row) + rows as u64 > u64::from(MAX_ROWS)
        || u64::from(col) + cols as u64 > u64::from(MAX_COLS)
        || rows.saturating_mul(cols) > super::model::MAX_RANGE_CELLS
    {
        return Err("Вставка выходит за границы листа".into());
    }
    let target = book.sheet_by_id(sheet).ok_or("Лист не найден")?;
    let mut values = Vec::new();
    for r in 0..rows {
        let mut line = Vec::new();
        for c in 0..cols {
            let (sr, sc) = if transpose { (c, r) } else { (r, c) };
            let original = clipboard.cells[sr][sc].as_ref();
            let input = if mode == PasteMode::Formats {
                target
                    .cell(row + r as u32, col + c as u32)
                    .map(|cell| cell.input.clone())
                    .unwrap_or_default()
            } else if mode == PasteMode::Values {
                original
                    .map(|cell| value_input(&cell.value))
                    .unwrap_or_default()
            } else {
                original
                    .map(|cell| {
                        if clipboard.cut {
                            cell.input.clone()
                        } else {
                            super::formula::shift_references(
                                &cell.input,
                                i64::from(row) + r as i64 - i64::from(clipboard.row) - sr as i64,
                                i64::from(col) + c as i64 - i64::from(clipboard.col) - sc as i64,
                            )
                        }
                    })
                    .unwrap_or_default()
            };
            line.push(input);
        }
        values.push(line);
    }
    let mut candidate = book.clone();
    if mode == PasteMode::All && !clipboard.merges.is_empty() {
        let destination = Range::new(row, col, row + rows as u32 - 1, col + cols as u32 - 1);
        let incoming: Vec<Range> = clipboard
            .merges
            .iter()
            .map(|merge| {
                let local = if transpose {
                    Range::new(merge.c0, merge.r0, merge.c1, merge.r1)
                } else {
                    *merge
                };
                Range::new(
                    row + local.r0,
                    col + local.c0,
                    row + local.r1,
                    col + local.c1,
                )
            })
            .collect();
        if target.merges.iter().any(|merge| {
            merge.r0 <= destination.r1
                && destination.r0 <= merge.r1
                && merge.c0 <= destination.c1
                && destination.c0 <= merge.c1
                && !incoming.contains(merge)
        }) {
            return Err("Вставка пересекает несовпадающую объединённую область".into());
        }
        candidate.unmerge(sheet, destination)?;
    }
    if mode == PasteMode::Formats {
        let range = Range::new(row, col, row + rows as u32 - 1, col + cols as u32 - 1);
        if target.merges.iter().any(|merge| {
            merge.r0 <= range.r1
                && range.r0 <= merge.r1
                && merge.c0 <= range.c1
                && range.c0 <= merge.c1
                && (rows * cols > 1 || (row, col) != (merge.r0, merge.c0))
        }) {
            return Err("Вставка затрагивает объединённые ячейки".into());
        }
        for (r, line) in values.iter().enumerate() {
            for (c, input) in line.iter().enumerate() {
                candidate.set_input(sheet, row + r as u32, col + c as u32, input)?;
            }
        }
    } else {
        candidate.set_range(sheet, row, col, &values)?;
    }
    if matches!(mode, PasteMode::All | PasteMode::Formats) {
        let target = candidate.sheet_by_id_mut(sheet).ok_or("Лист не найден")?;
        for r in 0..rows {
            for c in 0..cols {
                let (sr, sc) = if transpose { (c, r) } else { (r, c) };
                let original = clipboard.cells[sr][sc].as_ref();
                let key = address(row + r as u32, col + c as u32);
                if mode == PasteMode::All && original.is_none() {
                    target.cells.remove(&key);
                } else if let Some(cell) = target.cells.get_mut(&key) {
                    cell.fmt = original.map(|cell| cell.fmt.clone()).unwrap_or_default();
                }
            }
        }
    }
    if mode == PasteMode::All {
        for merge in &clipboard.merges {
            let local = if transpose {
                Range::new(merge.c0, merge.r0, merge.c1, merge.r1)
            } else {
                *merge
            };
            if local.r1 >= rows as u32 || local.c1 >= cols as u32 {
                return Err("Повреждённый буфер объединений".into());
            }
            candidate.merge(
                sheet,
                Range::new(
                    row + local.r0,
                    col + local.c0,
                    row + local.r1,
                    col + local.c1,
                ),
            )?;
        }
    }
    candidate.validate()?;
    *book = candidate;
    Ok(())
}
fn paste_special(
    ctx: &egui::Context,
    book: &mut Book,
    mode: PasteMode,
    transpose: bool,
) -> Result<(), String> {
    let mut clipboard = ctx
        .data_mut(|data| data.get_temp::<SheetClipboard>(clipboard_id()))
        .ok_or("Сначала скопируйте ячейки в TrineDocs")?;
    let (sheet, row, col) = (book.active_id(), book.selection.row, book.selection.col);
    paste_cells_with(book, sheet, row, col, &clipboard, mode, transpose)?;
    if clipboard.cut && mode != PasteMode::Formats {
        clipboard.cut = false;
        clipboard.row = row;
        clipboard.col = col;
        if transpose {
            clipboard.merges = clipboard
                .merges
                .iter()
                .map(|range| Range::new(range.c0, range.r0, range.c1, range.r1))
                .collect();
            let rows = clipboard.cells.len();
            let cols = clipboard.cells.first().map(Vec::len).unwrap_or(0);
            clipboard.cells = (0..cols)
                .map(|c| (0..rows).map(|r| clipboard.cells[r][c].clone()).collect())
                .collect();
        }
        ctx.data_mut(|data| data.insert_temp(clipboard_id(), clipboard));
    }
    Ok(())
}
fn paste_clipboard(
    ctx: &egui::Context,
    book: &mut Book,
    sheet: u64,
    row: u32,
    col: u32,
    text: &str,
) -> Result<(), String> {
    let clipboard = ctx.data_mut(|data| data.get_temp::<SheetClipboard>(clipboard_id()));
    if let Some(mut clipboard) = clipboard.filter(|clipboard| clipboard.text == text) {
        paste_cells(book, sheet, row, col, &clipboard)?;
        if clipboard.cut {
            clipboard.cut = false;
            clipboard.row = row;
            clipboard.col = col;
            ctx.data_mut(|data| data.insert_temp(clipboard_id(), clipboard));
        }
        return Ok(());
    }
    ctx.data_mut(|data| data.remove_temp::<SheetClipboard>(clipboard_id()));
    paste_values(book, sheet, row, col, &super::io::parse_csv(text, '\t')?)
}
fn copy_range(ui: &egui::Ui, book: &Book, state: &mut State, cut: bool) -> bool {
    match capture_clipboard(book, cut) {
        Ok(clipboard) => {
            ui.ctx().copy_text(clipboard.text.clone());
            ui.ctx()
                .data_mut(|data| data.insert_temp(clipboard_id(), clipboard));
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
    result(
        state,
        book.clear_range(book.active_id(), book.selection.range(), true, false),
    )
}
fn autofit_width(ui: &egui::Ui, sheet: &Sheet, col: u32, locale_ru: bool) -> f32 {
    sheet
        .cells
        .iter()
        .filter_map(|(address, cell)| {
            let (row, cell_col) = super::model::parse_address(address)?;
            (cell_col == col && !sheet.merges.iter().any(|merge| merge.contains(row, col)))
                .then_some((row, cell))
        })
        .map(|(row, cell)| {
            let font = FontId::new(
                cell.fmt.size * 1.333,
                crate::fonts::egui_family(ui.ctx(), &cell.fmt.font, cell.fmt.bold, cell.fmt.italic),
            );
            ui.painter()
                .layout_no_wrap(
                    sheet.display(row, col, locale_ru),
                    font,
                    color(cell.fmt.color),
                )
                .size()
                .x
                + 10.0
        })
        .fold(24.0, f32::max)
        .min(1800.0)
}
fn grid_size(sheet: &Sheet, rows: u32, cols: u32) -> (f32, f32) {
    let row_end = sheet
        .merges
        .iter()
        .map(|merge| merge.r1.saturating_add(1))
        .max()
        .unwrap_or(0);
    let col_end = sheet
        .merges
        .iter()
        .map(|merge| merge.c1.saturating_add(1))
        .max()
        .unwrap_or(0);
    let mut width = axis_pos(
        cols.max(col_end),
        sheet.default_col_width,
        &sheet.widths,
        &sheet.hidden_cols,
    ) + 48.0;
    let mut height = axis_pos(
        rows.max(row_end),
        sheet.default_row_height,
        &sheet.row_heights,
        &sheet.hidden_rows,
    ) + 28.0;
    for image in &sheet.images {
        width = width.max(
            48.0 + axis_pos(
                image.col,
                sheet.default_col_width,
                &sheet.widths,
                &sheet.hidden_cols,
            ) + image.x_offset
                + image.width
                + 16.0,
        );
        height = height.max(
            28.0 + axis_pos(
                image.row,
                sheet.default_row_height,
                &sheet.row_heights,
                &sheet.hidden_rows,
            ) + image.y_offset
                + image.height
                + 16.0,
        );
    }
    for chart in &sheet.charts {
        let rect = floating_chart_rect(sheet, Pos2::ZERO, chart);
        width = width.max(rect.max.x + 16.0);
        height = height.max(rect.max.y + 16.0);
    }
    (width, height)
}
fn floating_chart_rect(sheet: &Sheet, origin: Pos2, chart: &super::model::Chart) -> Rect {
    Rect::from_min_size(
        origin
            + Vec2::new(
                48.0 + axis_pos(
                    chart.col,
                    sheet.default_col_width,
                    &sheet.widths,
                    &sheet.hidden_cols,
                ),
                28.0 + axis_pos(
                    chart.row,
                    sheet.default_row_height,
                    &sheet.row_heights,
                    &sheet.hidden_rows,
                ),
            ),
        Vec2::new(chart.width, chart.height),
    )
}
fn place_chart(sheet: &mut Sheet, id: u64, local: Vec2, size: Vec2) -> bool {
    let col = axis_index(
        (local.x - 48.0).max(0.0),
        MAX_COLS,
        sheet.default_col_width,
        &sheet.widths,
        &sheet.hidden_cols,
    );
    let row = axis_index(
        (local.y - 28.0).max(0.0),
        MAX_ROWS,
        sheet.default_row_height,
        &sheet.row_heights,
        &sheet.hidden_rows,
    );
    let Some(chart) = sheet.charts.iter_mut().find(|chart| chart.id == id) else {
        return false;
    };
    let (width, height) = (size.x.clamp(1.0, 4000.0), size.y.clamp(1.0, 4000.0));
    let changed = (chart.row, chart.col, chart.width, chart.height) != (row, col, width, height);
    chart.row = row;
    chart.col = col;
    chart.width = width;
    chart.height = height;
    changed
}
fn constrained_image_size(original: Vec2, requested: Vec2, keep_ratio: bool) -> Vec2 {
    if !keep_ratio {
        return requested.clamp(Vec2::splat(16.0), Vec2::splat(8000.0));
    }
    let minimum = (1.0 / original.x).max(1.0 / original.y);
    let maximum = (8000.0 / original.x).min(8000.0 / original.y);
    original * (requested.x / original.x).clamp(minimum, maximum)
}
/// Moves/resizes a picture; `local` is its top-left corner relative to the grid origin.
fn place_image(sheet: &mut Sheet, image: u64, local: Vec2, size: Vec2) {
    let x = (local.x - 48.0).max(0.0);
    let y = (local.y - 28.0).max(0.0);
    let col = axis_index(
        x,
        MAX_COLS,
        sheet.default_col_width,
        &sheet.widths,
        &sheet.hidden_cols,
    );
    let row = axis_index(
        y,
        MAX_ROWS,
        sheet.default_row_height,
        &sheet.row_heights,
        &sheet.hidden_rows,
    );
    let x_offset = (x - axis_pos(
        col,
        sheet.default_col_width,
        &sheet.widths,
        &sheet.hidden_cols,
    ))
    .clamp(0.0, 4000.0);
    let y_offset = (y - axis_pos(
        row,
        sheet.default_row_height,
        &sheet.row_heights,
        &sheet.hidden_rows,
    ))
    .clamp(0.0, 4000.0);
    if let Some(picture) = sheet.images.iter_mut().find(|i| i.id == image) {
        picture.row = row;
        picture.col = col;
        picture.x_offset = x_offset;
        picture.y_offset = y_offset;
        picture.width = size.x.clamp(1.0, 8000.0);
        picture.height = size.y.clamp(1.0, 8000.0);
    }
}
fn image_texture_id(sheet: u64, image: &super::model::SheetImage) -> egui::Id {
    egui::Id::new(("sheet_image", sheet, image.id, image.bytes.as_slice()))
}
fn paint_image(
    ui: &egui::Ui,
    painter: &egui::Painter,
    sheet: u64,
    image: &super::model::SheetImage,
    rect: Rect,
) {
    let key = image_texture_id(sheet, image);
    let ctx = ui.ctx();
    let texture = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(key));
    let texture = texture.or_else(|| {
        let decoded = crate::util::decode_image(&image.bytes).ok()?.to_rgba8();
        if decoded.width() > 8192 || decoded.height() > 8192 {
            return None;
        }
        let size = [decoded.width() as usize, decoded.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw());
        let handle = ctx.load_texture(
            format!("sheet_image_{}", image.id),
            color,
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| d.insert_temp(key, handle.clone()));
        Some(handle)
    });
    match texture {
        Some(texture) => {
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, Color32::RED),
                StrokeKind::Inside,
            );
        }
    }
}
fn start_image_job(ctx: &egui::Context, state: &mut State, book: &Book) {
    state.image_target = Some((book.active_id(), book.selection.row, book.selection.col));
    let (sender, receiver) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    state.image_job = Some(std::sync::Arc::new(std::sync::Mutex::new(receiver)));
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
            crate::util::decode_image(&bytes)?;
            Ok(Some(bytes))
        })();
        let _ = sender.send(result);
        ctx.request_repaint();
    });
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
    let x =
        48.0 + axis_pos(
            col,
            sheet.default_col_width,
            &sheet.widths,
            &sheet.hidden_cols,
        ) + if col < frozen_cols {
            viewport.min.x
        } else {
            0.0
        };
    let y =
        28.0 + axis_pos(
            row,
            sheet.default_row_height,
            &sheet.row_heights,
            &sheet.hidden_rows,
        ) + if row < frozen_rows {
            viewport.min.y
        } else {
            0.0
        };
    Rect::from_min_size(
        origin + Vec2::new(x, y),
        Vec2::new(
            sheet
                .widths
                .get(&col)
                .copied()
                .unwrap_or(sheet.default_col_width),
            sheet
                .row_heights
                .get(&row)
                .copied()
                .unwrap_or(sheet.default_row_height),
        ),
    )
}
fn visible_axis(range: std::ops::Range<u32>, frozen: u32) -> Vec<u32> {
    range
        .filter(|index| *index >= frozen)
        .chain(0..frozen)
        .collect()
}
fn cell_area(
    sheet: &Sheet,
    origin: Pos2,
    viewport: Rect,
    row: u32,
    col: u32,
    frozen_rows: u32,
    frozen_cols: u32,
) -> Rect {
    let mut rect = cell_rect(sheet, origin, viewport, row, col, frozen_rows, frozen_cols);
    if let Some(merge) = sheet
        .merges
        .iter()
        .find(|merge| merge.r0 == row && merge.c0 == col)
    {
        rect.max.x = rect.min.x
            + axis_pos(
                merge.c1 + 1,
                sheet.default_col_width,
                &sheet.widths,
                &sheet.hidden_cols,
            )
            - axis_pos(
                merge.c0,
                sheet.default_col_width,
                &sheet.widths,
                &sheet.hidden_cols,
            );
        rect.max.y = rect.min.y
            + axis_pos(
                merge.r1 + 1,
                sheet.default_row_height,
                &sheet.row_heights,
                &sheet.hidden_rows,
            )
            - axis_pos(
                merge.r0,
                sheet.default_row_height,
                &sheet.row_heights,
                &sheet.hidden_rows,
            );
    }
    rect
}
fn visible_cells(
    sheet: &Sheet,
    origin: Pos2,
    viewport: Rect,
    rows: std::ops::Range<u32>,
    cols: std::ops::Range<u32>,
    frozen: (u32, u32),
    clip: Rect,
) -> Vec<(u32, u32)> {
    let mut cells = BTreeSet::new();
    for row in visible_axis(rows, frozen.0) {
        if sheet.hidden_rows.contains(&row) {
            continue;
        }
        for col in visible_axis(cols.clone(), frozen.1) {
            if !sheet.hidden_cols.contains(&col) {
                cells.insert((row, col));
            }
        }
    }
    for merge in &sheet.merges {
        if (merge.r0..=merge.r1).any(|row| !sheet.hidden_rows.contains(&row))
            && (merge.c0..=merge.c1).any(|col| !sheet.hidden_cols.contains(&col))
            && cell_area(
                sheet, origin, viewport, merge.r0, merge.c0, frozen.0, frozen.1,
            )
            .intersects(clip)
        {
            cells.insert((merge.r0, merge.c0));
        }
    }
    let mut cells: Vec<_> = cells.into_iter().collect();
    cells.sort_by_key(|&(row, col)| (row < frozen.0, col < frozen.1, row, col));
    cells
}
fn chart_bounds(values: impl Iterator<Item = f64>, zero: bool) -> (f64, f64) {
    let mut low = if zero { 0.0 } else { f64::INFINITY };
    let mut high = if zero { 0.0 } else { f64::NEG_INFINITY };
    for value in values.filter(|value| value.is_finite()) {
        low = low.min(value);
        high = high.max(value);
    }
    if !low.is_finite() || !high.is_finite() {
        (0.0, 1.0)
    } else {
        (low, high)
    }
}
fn chart_fraction(value: f64, low: f64, high: f64) -> f32 {
    if low == high {
        return 0.5;
    }
    let span = high - low;
    let result = if span.is_finite() {
        (value - low) / span
    } else {
        (value / 2.0 - low / 2.0) / (high / 2.0 - low / 2.0)
    };
    result as f32
}

fn pie_fractions(values: &[f64]) -> Vec<f32> {
    let scale = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(0.0, f64::max);
    if scale <= 0.0 {
        return vec![0.0; values.len()];
    }
    let scaled: Vec<_> = values
        .iter()
        .map(|value| {
            if value.is_finite() {
                value.max(0.0) / scale
            } else {
                0.0
            }
        })
        .collect();
    let total = scaled.iter().sum::<f64>();
    scaled
        .into_iter()
        .map(|value| (value / total) as f32)
        .collect()
}
fn paint_chart(ui: &egui::Ui, sheet: &Sheet, chart: &super::model::Chart, rect: Rect) {
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let r = chart.range;
    let start = if chart.has_headers && r.r1 > r.r0 {
        r.r0 + 1
    } else {
        r.r0
    };
    let first_col = if chart.has_categories && r.c1 > r.c0 {
        r.c0 + 1
    } else {
        r.c0
    };
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
                        .value(row, c)
                        .and_then(|v| v.number().ok())
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
    if rect.width() > 32.0 && rect.height() > 36.0 {
        painter.text(
            rect.min + Vec2::new(8.0, 8.0),
            Align2::LEFT_TOP,
            &chart.title,
            FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
    }
    let mut plot = rect.shrink2(Vec2::new(30.0, 18.0));
    plot.min.y += 18.0;
    if plot.width() <= 0.0 || plot.height() <= 0.0 {
        return;
    }
    let (lo, hi) = chart_bounds(series.iter().flatten().copied(), true);
    let y = |v: f64| plot.bottom() - chart_fraction(v, lo, hi) * plot.height();
    if matches!(chart.kind, ChartKind::Pie | ChartKind::Doughnut) {
        let fractions = pie_fractions(first);
        if fractions.iter().all(|value| *value == 0.0) {
            return;
        }
        let radius = plot.height().min(plot.width()) / 2.0;
        let center = plot.center();
        let mut angle = -std::f32::consts::FRAC_PI_2;
        for (i, fraction) in fractions.iter().enumerate() {
            let end = angle + fraction * std::f32::consts::TAU;
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
                    let zero = plot.left() + chart_fraction(0.0, lo, hi) * plot.width();
                    let xx = plot.left() + chart_fraction(*v, lo, hi) * plot.width();
                    painter.rect_filled(
                        Rect::from_two_pos(Pos2::new(zero, yy), Pos2::new(xx, yy + h - 1.0)),
                        0.0,
                        c,
                    );
                }
                ChartKind::Scatter => {
                    let xv = sheet
                        .value(start + i as u32, r.c0)
                        .and_then(|v| v.number().ok())
                        .unwrap_or(i as f64);
                    let (xmin, xmax) = chart_bounds(
                        (start..=r.r1.min(start + 199))
                            .filter_map(|rr| sheet.value(rr, r.c0).and_then(|v| v.number().ok())),
                        false,
                    );
                    painter.circle_filled(
                        Pos2::new(
                            plot.left() + chart_fraction(xv, xmin, xmax) * plot.width(),
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
    #[test]
    fn review4_chart_coordinates_use_the_data_scale_without_a_unit_floor() {
        let (low, high) = chart_bounds([-1E-200, 1E-200].into_iter(), true);
        assert_eq!(chart_fraction(-1E-200, low, high), 0.0);
        assert_eq!(chart_fraction(1E-200, low, high), 1.0);
        assert_eq!(chart_fraction(0.0, low, high), 0.5);
        assert_eq!(chart_fraction(0.0, -1E308, 1E308), 0.5);
        assert_eq!(chart_fraction(1E308, -1E308, 1E308), 1.0);
        let (low, high) = chart_bounds([0.01, 0.02].into_iter(), false);
        assert_eq!(chart_fraction(0.02, low, high), 1.0);
        assert_eq!(chart_fraction(5.0, 5.0, 5.0), 0.5);
    }
    #[test]
    fn review4_canvas_renders_underline_and_strike_as_two_distinct_lines() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "both")?;
        book.set_format(
            id,
            Range::new(0, 0, 0, 0),
            CellFormat {
                underline: true,
                strike: true,
                ..Default::default()
            },
        )?;
        let mut target = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1250.0, 820.0))),
                ..Default::default()
            },
            |ui| {
                show(ui, &mut book);
                target = ui
                    .ctx()
                    .data_mut(|data| data.get_temp::<State>(ui.id().with("sheets-editor")))
                    .and_then(|state| state.test_controls.get("grid-b1").copied())
                    .map(|rect| rect.translate(Vec2::new(-96.0, 0.0)));
            },
        );
        let rect = target.ok_or("grid geometry")?;
        let mut lines = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::LineSegment { points, stroke } = &shape.shape {
                    (stroke.width == 1.0
                        && stroke.color == color([28, 32, 38])
                        && rect.contains(points[0])
                        && rect.contains(points[1])
                        && (points[0].y - points[1].y).abs() < 0.01)
                        .then_some(points[0].y)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        lines.sort_by(f32::total_cmp);
        output.textures_delta.clear();
        assert_eq!(lines.len(), 2);
        assert!((lines[1] - lines[0] - 7.0).abs() < 0.01);
        Ok(())
    }

    use super::*;
    #[test]
    fn review5_jumps_skip_hidden_axes_and_covered_merge_children() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_range(
            id,
            0,
            0,
            &[vec!["1".into()], vec!["2".into()], vec!["3".into()]],
        )?;
        let sheet = book.active_sheet_mut().ok_or("sheet")?;
        sheet.hidden_rows.insert(2);
        sheet.hidden_cols.insert(0);
        assert_eq!(sheet.data_edge(0, 0, 1, 0), (1, 0));
        assert_eq!(jump_target(sheet, (5, 3), (0, 0)), (0, 1));
        sheet.hidden_cols.clear();
        sheet.merges.push(Range::new(10, 1, 12, 3));
        assert_eq!(jump_target(sheet, (20, 5), (11, 2)), (10, 1));
        Ok(())
    }
    #[test]
    fn review5_changing_sheets_clears_the_old_selected_image() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.add_sheet("Second")?;
        let state = frame(&ctx, &mut book, Vec::new(), false)?;
        let mut state = state;
        state.selected_image = Some(999);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.ctx()
                .data_mut(|data| data.insert_temp(ui.id().with("sheets-editor"), state.clone()));
        });
        output.textures_delta.clear();
        book.active = 1;
        let state = frame(&ctx, &mut book, Vec::new(), false)?;
        assert_eq!(state.selected_image, None);
        Ok(())
    }
    #[test]
    fn same_frame_text_and_paste_preserve_the_pending_draft() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.set_cell(book.active_id(), 0, 0, "original")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        let state = frame(
            &ctx,
            &mut book,
            vec![
                egui::Event::Text("new".into()),
                egui::Event::Paste(" value".into()),
            ],
            false,
        )?;
        assert!(state.editing);
        assert_eq!(state.buffer, "new value");
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("original")
        );
        frame(&ctx, &mut book, vec![key(Key::Enter, false)], false)?;
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.input.as_str()),
            Some("new value")
        );
        Ok(())
    }
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
    fn tab_accepts_function_completion_in_formula_bar() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        book.set_cell(book.active_id(), 0, 1, "5")?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text("=SU".into())],
            false,
        )?;
        frame(&ctx, &mut book, Vec::new(), false)?;
        // ↓ moves to the second suggestion and ↑ back to SUM before Tab inserts it.
        frame(
            &ctx,
            &mut book,
            vec![key(Key::ArrowDown, false), key(Key::ArrowUp, false)],
            false,
        )?;
        let state = frame(&ctx, &mut book, vec![key(Key::Tab, false)], false)?;
        assert_eq!(state.buffer, "=SUM(");
        assert!(state.editing);
        assert_eq!((book.selection.row, book.selection.col), (0, 0));
        frame(
            &ctx,
            &mut book,
            vec![egui::Event::Text("B1)".into())],
            false,
        )?;
        frame(&ctx, &mut book, vec![key(Key::Enter, false)], false)?;
        let cell = book.active_sheet().and_then(|sheet| sheet.cell(0, 0));
        assert_eq!(cell.map(|cell| cell.input.as_str()), Some("=SUM(B1)"));
        assert_eq!(
            cell.map(|cell| cell.value.clone()),
            Some(Value::Number(5.0))
        );
        Ok(())
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
            // One character over the 32 767-character cell limit.
            let draft = "я".repeat(32768);
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
    #[test]
    fn review3_keyboard_skips_hidden_axes_and_crosses_entire_merge() {
        let mut sheet = Sheet::default();
        sheet.hidden_rows.insert(1);
        sheet.hidden_cols.insert(1);
        sheet.merges.push(Range::new(3, 2, 5, 4));
        assert_eq!(step_cell(&sheet, 0, 0, 1, 0), (2, 0));
        assert_eq!(step_cell(&sheet, 0, 0, 0, 1), (0, 2));
        assert_eq!(step_cell(&sheet, 3, 2, 1, 0), (6, 2));
        assert_eq!(step_cell(&sheet, 3, 2, 0, 1), (3, 5));
        assert_eq!(step_cell(&sheet, 3, 5, 0, -1), (3, 2));
        assert_eq!(step_cell(&sheet, 6, 2, -1, 0), (3, 2));
        sheet.hidden_rows.insert(MAX_ROWS - 1);
        assert_eq!(step_cell(&sheet, MAX_ROWS - 2, 0, 1, 0), (MAX_ROWS - 2, 0));
    }
    #[test]
    fn review3_frozen_cells_are_unique_and_headers_keep_viewport_coordinates() {
        let sheet = Sheet::default();
        let origin = Pos2::new(10.0, 20.0);
        let viewport = Rect::from_min_max(Pos2::new(400.0, 260.0), Pos2::new(800.0, 520.0));
        let columns = visible_axis(0..5, 2);
        assert_eq!(columns, vec![2, 3, 4, 0, 1]);
        let cells = visible_cells(
            &sheet,
            origin,
            viewport,
            0..4,
            0..5,
            (2, 2),
            Rect::EVERYTHING,
        );
        assert_eq!(cells.iter().filter(|&&(r, c)| (r, c) == (0, 0)).count(), 1);
        assert_eq!(cells.last().copied(), Some((1, 1)));
        let frozen = cell_rect(&sheet, origin, viewport, 0, 0, 2, 2);
        assert_eq!(frozen.min, Pos2::new(458.0, 308.0));
        let moving = cell_rect(&sheet, origin, viewport, 2, 2, 2, 2);
        assert_eq!(moving.min, Pos2::new(250.0, 100.0));
    }
    #[test]
    fn review3_merge_anchor_remains_visible_when_its_original_cell_is_offscreen() {
        let mut sheet = Sheet::default();
        sheet.merges.push(Range::new(0, 0, 14, 3));
        let viewport = Rect::from_min_max(Pos2::new(0.0, 260.0), Pos2::new(400.0, 520.0));
        let cells = visible_cells(&sheet, Pos2::ZERO, viewport, 10..20, 0..4, (0, 0), viewport);
        assert!(cells.contains(&(0, 0)));
        assert_eq!(
            cell_area(&sheet, Pos2::ZERO, viewport, 0, 0, 0, 0).max.y,
            418.0
        );
    }
    #[test]
    fn review3_text_formatted_cells_keep_typed_and_pasted_input_literal() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        let fmt = CellFormat {
            number: "@".into(),
            ..Default::default()
        };
        book.set_format(id, Range::new(0, 0, 1, 0), fmt)?;
        let mut state = State {
            editing: true,
            last: (id, 0, 0),
            buffer: "001".into(),
            ..Default::default()
        };
        assert!(commit(&mut book, &mut state));
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| &cell.value),
            Some(&Value::Text("001".into()))
        );
        paste_values(&mut book, id, 1, 0, &[vec!["=1+1".into(), "002".into()]])?;
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(1, 0))
                .map(|cell| &cell.value),
            Some(&Value::Text("=1+1".into()))
        );
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(1, 1))
                .map(|cell| &cell.value),
            Some(&Value::Number(2.0))
        );
        assert_eq!(entry_text(&book, id, 0, 0, ""), "");
        Ok(())
    }
    #[test]
    fn review3_delete_keeps_sparse_storage_and_existing_formats() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "value")?;
        book.set_format(
            id,
            Range::new(1, 1, 1, 1),
            CellFormat {
                bold: true,
                ..Default::default()
            },
        )?;
        let format_id = book
            .active_sheet()
            .and_then(|sheet| sheet.cell(1, 1))
            .ok_or("cell")?
            .id;
        book.selection = Selection {
            row: 99,
            col: 99,
            anchor_row: 0,
            anchor_col: 0,
        };
        assert!(clear(&mut book, &mut State::default()));
        let sheet = book.active_sheet().ok_or("sheet")?;
        assert_eq!(sheet.cells.len(), 1);
        assert_eq!(
            sheet
                .cell(1, 1)
                .map(|cell| (cell.id, cell.fmt.bold, cell.input.as_str())),
            Some((format_id, true, ""))
        );
        assert!(sheet.cell(0, 0).is_none());
        assert!(sheet.cell(99, 99).is_none());
        Ok(())
    }
    #[test]
    fn review3_grid_bounds_include_merge_extent_and_floating_image_edges() {
        let mut sheet = Sheet::default();
        sheet.merges.push(Range::new(0, 0, 1499, 49));
        let (width, height) = grid_size(&sheet, 1000, 26);
        assert!(width >= 48.0 + 50.0 * 96.0);
        assert!(height >= 28.0 + 1500.0 * 26.0);
        sheet.merges.clear();
        sheet.images.push(super::super::model::SheetImage {
            id: 2,
            row: 2000,
            col: 80,
            x_offset: 400.0,
            y_offset: 200.0,
            width: 8000.0,
            height: 8000.0,
            bytes: vec![1],
            alt: String::new(),
        });
        let (width, height) = grid_size(&sheet, 1000, 26);
        assert!(width > 48.0 + 80.0 * 96.0 + 400.0 + 8000.0);
        assert!(height > 28.0 + 2000.0 * 26.0 + 200.0 + 8000.0);
    }
    #[test]
    fn review3_autofit_measures_number_format_and_font_and_can_shrink() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "1")?;
        let mut widths = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            if let Some(sheet) = book.active_sheet_mut() {
                widths.push(autofit_width(ui, sheet, 0, false));
                if let Some(cell) = sheet.cell_mut(0, 0) {
                    cell.fmt.number = "0.000000000000".into();
                }
                widths.push(autofit_width(ui, sheet, 0, false));
                if let Some(cell) = sheet.cell_mut(0, 0) {
                    cell.fmt.size = 36.0;
                }
                widths.push(autofit_width(ui, sheet, 0, false));
            }
        });
        output.textures_delta.clear();
        assert_eq!(widths.len(), 3);
        assert!(widths[0] < 96.0);
        assert!(widths[1] > widths[0] * 2.0);
        assert!(widths[2] > widths[1] * 2.0);
        Ok(())
    }
    #[test]
    fn review3_async_image_retains_sheet_and_cell_while_active_sheet_changes() -> Result<(), String>
    {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        let first = book.active_id();
        let other = book.add_sheet("Other")?;
        book.active = 1;
        book.selection = Selection {
            row: 9,
            col: 9,
            anchor_row: 9,
            anchor_col: 9,
        };
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let (sender, receiver) = std::sync::mpsc::channel();
        sender
            .send(Ok(Some(png.into_inner())))
            .map_err(|error| error.to_string())?;
        let state_id = egui::Id::new(("sheets-editor", 73u64));
        ctx.data_mut(|data| {
            data.insert_temp(
                state_id,
                State {
                    last: (other, 9, 9),
                    image_job: Some(std::sync::Arc::new(std::sync::Mutex::new(receiver))),
                    image_target: Some((first, 3, 4)),
                    ..Default::default()
                },
            )
        });
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            show_for_tab(ui, &mut book, 73);
        });
        output.textures_delta.clear();
        let image = book
            .sheet_by_id(first)
            .and_then(|sheet| sheet.images.first())
            .ok_or("image")?;
        assert_eq!((image.row, image.col), (3, 4));
        assert!(book.sheet_by_id(other).ok_or("other")?.images.is_empty());
        assert_eq!(book.active_id(), other);
        assert!(
            ctx.data_mut(|data| data.get_temp::<State>(state_id))
                .ok_or("state")?
                .selected_image
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn review3_inspection_navigation_is_consumed_readonly_and_rejects_hidden_or_missing_cells()
    -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 2, 3, "error")?;
        request_focus(&ctx, 74, id, 2, 3);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.disable();
            show_for_tab(ui, &mut book, 74);
        });
        output.textures_delta.clear();
        assert_eq!((book.selection.row, book.selection.col), (2, 3));
        let request_id = egui::Id::new(("sheets-editor", 74u64)).with("focus-request");
        assert!(
            ctx.data_mut(|data| data.get_temp::<(u64, u32, u32)>(request_id))
                .is_none()
        );
        let mut state = State::default();
        let before = book.clone();
        assert!(!apply_focus_request(&mut book, &mut state, (id, 9, 9)));
        assert_eq!(book, before);
        book.active_sheet_mut()
            .ok_or("sheet")?
            .merges
            .push(Range::new(2, 2, 2, 3));
        assert!(!apply_focus_request(&mut book, &mut state, (id, 2, 3)));
        book.active_sheet_mut()
            .ok_or("sheet")?
            .hidden_rows
            .insert(2);
        assert!(!apply_focus_request(&mut book, &mut state, (id, 2, 3)));
        Ok(())
    }
}
#[cfg(test)]
mod clipboard_review_tests {
    use super::*;

    #[test]
    fn typed_clipboard_preserves_inputs_values_formats_and_blanks() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        for (col, input) in ["=1/3", "TRUE", "=1/0", "'0012", "1.2345678901234567"]
            .iter()
            .enumerate()
        {
            book.set_input(id, 0, col as u32, input)?;
        }
        book.set_format(
            id,
            Range::new(0, 0, 0, 0),
            CellFormat {
                number: "0.00".into(),
                bold: true,
                ..CellFormat::default()
            },
        )?;
        book.set_input(id, 2, 5, "replace this with a blank")?;
        book.recalculate();
        book.selection = Selection {
            anchor_col: 5,
            ..Selection::default()
        };
        let clipboard = capture_clipboard(&book, false)?;
        let text = clipboard.text.clone();
        let ctx = egui::Context::default();
        ctx.data_mut(|data| data.insert_temp(clipboard_id(), clipboard));
        paste_clipboard(&ctx, &mut book, id, 2, 0, &text)?;
        let sheet = book.sheet_by_id(id).ok_or("sheet")?;
        for col in 0..5 {
            let source = sheet.cell(0, col).ok_or("source")?;
            let pasted = sheet.cell(2, col).ok_or("pasted")?;
            assert_eq!(pasted.input, source.input);
            assert_eq!(pasted.value, source.value);
            assert_eq!(pasted.fmt, source.fmt);
            assert_ne!(pasted.id, source.id);
        }
        assert!(sheet.cell(2, 5).is_none());
        Ok(())
    }
    #[test]
    fn copied_formulas_shift_but_cut_formulas_keep_references() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_input(id, 0, 0, "7")?;
        book.set_input(id, 0, 1, "=A1+$A$1")?;
        book.recalculate();
        book.selection = Selection {
            row: 0,
            col: 1,
            anchor_row: 0,
            anchor_col: 1,
        };
        let copy = capture_clipboard(&book, false)?;
        paste_cells(&mut book, id, 2, 1, &copy)?;
        assert_eq!(
            book.sheet_by_id(id)
                .and_then(|s| s.cell(2, 1))
                .map(|c| c.input.as_str()),
            Some("=A3+$A$1")
        );
        let cut = capture_clipboard(&book, true)?;
        book.clear_range(id, Range::new(0, 1, 0, 1), true, false)?;
        paste_cells(&mut book, id, 4, 1, &cut)?;
        assert_eq!(
            book.sheet_by_id(id)
                .and_then(|s| s.cell(4, 1))
                .map(|c| c.input.as_str()),
            Some("=A1+$A$1")
        );
        assert_eq!(
            book.sheet_by_id(id)
                .and_then(|s| s.cell(4, 1))
                .map(|c| &c.value),
            Some(&Value::Number(14.0))
        );
        Ok(())
    }
    #[test]
    fn rejected_paste_is_atomic_and_external_text_uses_tsv() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_input(id, 0, 0, "=1/3")?;
        book.set_input(id, 0, 1, "other")?;
        book.recalculate();
        book.selection = Selection {
            anchor_col: 1,
            ..Selection::default()
        };
        let clipboard = capture_clipboard(&book, false)?;
        let before = book.clone();
        assert!(paste_cells(&mut book, id, 0, MAX_COLS - 1, &clipboard).is_err());
        assert_eq!(book, before);
        book.merge(id, Range::new(2, 0, 2, 1))?;
        let before = book.clone();
        assert!(paste_cells(&mut book, id, 2, 0, &clipboard).is_err());
        assert_eq!(book, before);
        let ctx = egui::Context::default();
        ctx.data_mut(|data| data.insert_temp(clipboard_id(), clipboard));
        paste_clipboard(&ctx, &mut book, id, 4, 0, "external")?;
        assert_eq!(
            book.sheet_by_id(id)
                .and_then(|s| s.cell(4, 0))
                .map(|c| c.input.as_str()),
            Some("external")
        );
        assert!(
            ctx.data_mut(|data| data.get_temp::<SheetClipboard>(clipboard_id()))
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn equal_length_picture_replacement_gets_a_different_texture_key() {
        let image = super::super::model::SheetImage {
            id: 2,
            row: 0,
            col: 0,
            x_offset: 0.0,
            y_offset: 0.0,
            width: 10.0,
            height: 10.0,
            bytes: vec![1, 2, 3],
            alt: String::new(),
        };
        let replacement = super::super::model::SheetImage {
            bytes: vec![4, 5, 6],
            ..image.clone()
        };
        assert_ne!(
            image_texture_id(1, &image),
            image_texture_id(1, &replacement)
        );
        assert_eq!(
            image_texture_id(1, &image),
            image_texture_id(1, &image.clone())
        );
    }
}
#[cfg(test)]
mod paste_special_tests {
    use super::*;
    #[test]
    fn values_paste_preserves_exact_types_without_copying_formats() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        for (col, input) in ["=1/3", "'001", "TRUE", "=1/0"].iter().enumerate() {
            book.set_input(id, 0, col as u32, input)?;
        }
        book.set_format(
            id,
            Range::new(0, 0, 0, 0),
            CellFormat {
                number: "0.00".into(),
                ..CellFormat::default()
            },
        )?;
        book.recalculate();
        book.selection = Selection {
            anchor_col: 3,
            ..Selection::default()
        };
        let clipboard = capture_clipboard(&book, false)?;
        paste_cells_with(&mut book, id, 2, 0, &clipboard, PasteMode::Values, false)?;
        let sheet = book.active_sheet().ok_or("sheet")?;
        for col in 0..4 {
            assert_eq!(
                sheet.cell(0, col).map(|c| &c.value),
                sheet.cell(2, col).map(|c| &c.value)
            );
        }
        assert_eq!(
            sheet.cell(2, 0).ok_or("pasted")?.fmt.number,
            CellFormat::default().number
        );
        assert!(!sheet.cell(2, 0).ok_or("pasted")?.input.starts_with('='));
        Ok(())
    }
    #[test]
    fn transposed_inputs_use_each_source_coordinate_and_formats_keep_values() -> Result<(), String>
    {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_input(id, 0, 0, "=C1")?;
        book.set_input(id, 0, 1, "=D1")?;
        book.set_format(
            id,
            Range::new(0, 0, 0, 0),
            CellFormat {
                bold: true,
                ..CellFormat::default()
            },
        )?;
        book.selection = Selection {
            anchor_col: 1,
            ..Selection::default()
        };
        let clipboard = capture_clipboard(&book, false)?;
        paste_cells_with(&mut book, id, 2, 0, &clipboard, PasteMode::Inputs, true)?;
        assert_eq!(
            book.active_sheet()
                .and_then(|s| s.cell(2, 0))
                .map(|c| c.input.as_str()),
            Some("=C3")
        );
        assert_eq!(
            book.active_sheet()
                .and_then(|s| s.cell(3, 0))
                .map(|c| c.input.as_str()),
            Some("=C4")
        );
        book.set_input(id, 5, 0, "keep")?;
        paste_cells_with(&mut book, id, 5, 0, &clipboard, PasteMode::Formats, false)?;
        let cell = book
            .active_sheet()
            .and_then(|s| s.cell(5, 0))
            .ok_or("cell")?;
        assert_eq!(cell.input, "keep");
        assert!(cell.fmt.bold);
        let before = book.clone();
        assert!(
            paste_cells_with(
                &mut book,
                id,
                MAX_ROWS - 1,
                0,
                &clipboard,
                PasteMode::Values,
                true
            )
            .is_err()
        );
        assert_eq!(book, before);
        Ok(())
    }
}

fn close_find_on_escape(ui: &mut egui::Ui, state: &mut State) -> bool {
    if state.find_open
        && ui.is_enabled()
        && ui.input_mut(|input| input.consume_key(input.modifiers, Key::Escape))
    {
        state.find_open = false;
        true
    } else {
        false
    }
}
fn find_step(book: &mut Book, state: &mut State, backwards: bool) {
    match book.find_positions(book.active_id(), &state.find, state.find_case) {
        Ok(found) => {
            if found.is_empty() {
                state.find_message = "Совпадений нет".into();
                return;
            }
            let current = (book.selection.row, book.selection.col);
            let index = if backwards {
                found
                    .iter()
                    .rposition(|position| *position < current)
                    .unwrap_or(found.len() - 1)
            } else {
                found
                    .iter()
                    .position(|position| *position > current)
                    .unwrap_or(0)
            };
            let (row, col) = &found[index];
            book.selection = Selection {
                row: *row,
                col: *col,
                anchor_row: *row,
                anchor_col: *col,
            };
            state.scroll_to_selection = true;
            state.error.clear();
            state.find_message = format!("Совпадение {} из {}", index + 1, found.len());
        }
        Err(error) => state.error = error,
    }
}
fn statistics_range_valid(range: Range) -> bool {
    range.r0 <= range.r1 && range.c0 <= range.c1 && range.r1 < MAX_ROWS && range.c1 < MAX_COLS
}
fn selection_statistics(sheet: &Sheet, range: Range) -> (Option<f64>, Option<f64>, usize) {
    let mut numbers = Vec::new();
    let mut nonempty = 0;
    let mut take = |cell: &Cell| {
        if cell.value != Value::Empty {
            nonempty += 1;
        }
        if let Value::Number(value) = cell.value {
            numbers.push(value);
        }
    };
    if range.validate().is_ok() {
        for r in range.r0..=range.r1 {
            for c in range.c0..=range.c1 {
                if let Some(cell) = sheet.cell(r, c) {
                    take(cell);
                }
            }
        }
    } else {
        // Whole rows/columns may contain billions of coordinates but at most the
        // workbook's bounded stored cells. Never walk the rectangular area here.
        for (key, cell) in &sheet.cells {
            if super::model::parse_address(key).is_some_and(|(r, c)| range.contains(r, c)) {
                take(cell);
            }
        }
    }
    if numbers.is_empty() {
        return (Some(0.0), None, nonempty);
    }
    let scale = numbers.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        return (Some(0.0), Some(0.0), nonempty);
    }
    let (mut normalized, mut compensation) = (0.0, 0.0);
    for value in &numbers {
        let next = value / scale - compensation;
        let total = normalized + next;
        compensation = (total - normalized) - next;
        normalized = total;
    }
    let sum = normalized * scale;
    let mean = (normalized / numbers.len() as f64) * scale;
    (sum.is_finite().then_some(sum), Some(mean), nonempty)
}
fn statistic_text(value: Option<f64>, ru: bool) -> String {
    value
        .map(|value| super::format::display(&Value::Number(value), "General", ru))
        .unwrap_or_else(|| "—".into())
}
#[cfg(test)]
mod additional_search_status_tests {
    use super::*;
    #[test]
    fn selection_mean_stays_finite_when_the_sum_overflows() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "1e308")?;
        book.set_cell(id, 0, 1, "1e308")?;
        let sheet = book.active_sheet().ok_or("sheet")?;
        let (sum, mean, count) = selection_statistics(sheet, Range::new(0, 0, 0, 1));
        assert_eq!(sum, None);
        assert_eq!(mean, Some(1e308));
        assert_eq!(count, 2);
        Ok(())
    }
    #[test]
    fn status_numbers_keep_small_values_and_the_workbook_locale() {
        assert_eq!(statistic_text(Some(0.00001), true), "0,00001");
        assert_eq!(statistic_text(Some(1.25), true), "1,25");
        assert_eq!(statistic_text(Some(1.25), false), "1.25");
    }
    #[test]
    fn escape_closes_the_find_bar_and_consumes_the_key() {
        let ctx = egui::Context::default();
        let mut state = State {
            find_open: true,
            ..State::default()
        };
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| {
            assert!(close_find_on_escape(ui, &mut state));
            assert!(!ui.input(|input| input.key_pressed(Key::Escape)));
        });
        assert!(!state.find_open);
    }
}

#[cfg(test)]
mod hundred_statistics_tests {
    use super::*;
    #[test]
    fn full_column_and_sheet_statistics_use_sparse_cells() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "10")?;
        book.set_cell(id, MAX_ROWS - 1, 0, "20")?;
        book.set_cell(id, 0, 1, "100")?;
        let column = Range::new(0, 0, MAX_ROWS - 1, 0);
        assert!(statistics_range_valid(column));
        assert_eq!(
            selection_statistics(book.active_sheet().ok_or("sheet")?, column),
            (Some(30.0), Some(15.0), 2)
        );
        assert!(statistics_range_valid(Range::new(
            0,
            0,
            MAX_ROWS - 1,
            MAX_COLS - 1
        )));
        Ok(())
    }
}

fn cell_text_shape(
    ui: &egui::Ui,
    rect: Rect,
    fmt: &CellFormat,
    text: String,
    text_color: Color32,
    right: bool,
) -> egui::epaint::TextShape {
    let family = crate::fonts::egui_family(ui.ctx(), &fmt.font, fmt.bold, fmt.italic);
    let font = FontId::new(fmt.size * 1.333, family);
    let width = if fmt.wrap {
        (rect.width() - 10.0).max(1.0)
    } else {
        f32::INFINITY
    };
    let mut job = egui::text::LayoutJob::simple(text, font, text_color, width);
    for section in &mut job.sections {
        if fmt.underline {
            section.format.underline = Stroke::new(1.0, text_color);
        }
        if fmt.strike {
            section.format.strikethrough = Stroke::new(1.0, text_color);
        }
    }
    let galley = ui.painter().layout_job(job);
    let x = if fmt.align == Align::Center || fmt.rotation != 0 {
        rect.center().x - galley.size().x / 2.0
    } else if right {
        rect.right() - 5.0 - galley.size().x
    } else {
        rect.left() + 5.0
    };
    let y = if fmt.wrap && fmt.rotation == 0 {
        rect.top() + 3.0
    } else {
        rect.center().y - galley.size().y / 2.0
    };
    egui::epaint::TextShape::new(Pos2::new(x, y), galley, text_color)
        .with_angle_and_anchor(-f32::from(fmt.rotation).to_radians(), Align2::CENTER_CENTER)
}
fn current_region(sheet: &Sheet, row: u32, col: u32) -> Option<Range> {
    let mut by_row: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut by_col: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (key, cell) in &sheet.cells {
        if cell.input.is_empty() {
            continue;
        }
        if let Some((r, c)) = super::model::parse_address(key) {
            by_row.entry(r).or_default().insert(c);
            by_col.entry(c).or_default().insert(r);
        }
    }
    let mut range = Range::new(row, col, row, col);
    loop {
        let old = range;
        let row_has = |r, range: &Range| {
            by_row.get(&r).is_some_and(|cols| {
                cols.range(range.c0.saturating_sub(1)..=(range.c1 + 1).min(MAX_COLS - 1))
                    .next()
                    .is_some()
            })
        };
        let col_has = |c, range: &Range| {
            by_col.get(&c).is_some_and(|rows| {
                rows.range(range.r0.saturating_sub(1)..=(range.r1 + 1).min(MAX_ROWS - 1))
                    .next()
                    .is_some()
            })
        };
        if range.r0 > 0 && row_has(range.r0 - 1, &range) {
            range.r0 -= 1;
        }
        if range.r1 + 1 < MAX_ROWS && row_has(range.r1 + 1, &range) {
            range.r1 += 1;
        }
        if range.c0 > 0 && col_has(range.c0 - 1, &range) {
            range.c0 -= 1;
        }
        if range.c1 + 1 < MAX_COLS && col_has(range.c1 + 1, &range) {
            range.c1 += 1;
        }
        if range == old {
            break;
        }
    }
    by_row
        .range(range.r0..=range.r1)
        .any(|(_, cols)| cols.range(range.c0..=range.c1).next().is_some())
        .then_some(range)
}
fn select_current_region(book: &mut Book) {
    let region = book
        .active_sheet()
        .and_then(|sheet| current_region(sheet, book.selection.row, book.selection.col));
    let range = region
        .filter(|range| *range != book.selection.range())
        .unwrap_or_else(|| Range::new(0, 0, MAX_ROWS - 1, MAX_COLS - 1));
    book.selection = Selection {
        row: range.r1,
        col: range.c1,
        anchor_row: range.r0,
        anchor_col: range.c0,
    };
}
#[cfg(test)]
mod hundred_cell_ui_tests {
    use super::*;
    #[test]
    fn cell_rotation_reaches_the_native_text_shape() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let fmt = CellFormat {
            rotation: 45,
            ..CellFormat::default()
        };
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let shape = cell_text_shape(
                ui,
                Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 30.0)),
                &fmt,
                "Angle".into(),
                Color32::BLACK,
                false,
            );
            assert!((shape.angle + std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        });
        Ok(())
    }
    #[test]
    fn decorations_follow_the_text_galley_instead_of_the_cell_edges() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let fmt = CellFormat {
            underline: true,
            strike: true,
            wrap: true,
            ..CellFormat::default()
        };
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let shape = cell_text_shape(
                ui,
                Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 60.0)),
                &fmt,
                "Short".into(),
                Color32::RED,
                false,
            );
            assert!(shape.galley.size().x < 100.0);
            for section in &shape.galley.job.sections {
                assert_eq!(section.format.underline, Stroke::new(1.0, Color32::RED));
                assert_eq!(section.format.strikethrough, Stroke::new(1.0, Color32::RED));
            }
        });
        Ok(())
    }
    #[test]
    fn control_a_starts_from_the_current_data_region() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        for (r, c) in [(1, 1), (1, 2), (2, 1), (2, 2), (7, 4)] {
            book.set_input(id, r, c, "1")?;
        }
        book.selection = Selection {
            row: 1,
            col: 1,
            anchor_row: 1,
            anchor_col: 1,
        };
        select_current_region(&mut book);
        assert_eq!(book.selection.range(), Range::new(1, 1, 2, 2));
        select_current_region(&mut book);
        assert_eq!(
            book.selection.range(),
            Range::new(0, 0, MAX_ROWS - 1, MAX_COLS - 1)
        );
        let mut diagonal = Book::default();
        let id = diagonal.active_id();
        diagonal.set_input(id, 10, 10, "a")?;
        diagonal.set_input(id, 11, 11, "b")?;
        assert_eq!(
            current_region(diagonal.active_sheet().ok_or("sheet")?, 10, 10),
            Some(Range::new(10, 10, 11, 11))
        );
        Ok(())
    }
}

#[cfg(test)]
mod hundred_merge_clipboard_tests {
    use super::*;
    #[test]
    fn typed_clipboard_keeps_merged_areas_and_transposes_their_geometry() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "Title")?;
        book.merge(id, Range::new(0, 0, 0, 1))?;
        let clipboard = capture_clipboard(&book, false)?;
        assert_eq!(clipboard.merges, vec![Range::new(0, 0, 0, 1)]);
        assert!(capture_clipboard(&book, true).is_err());
        paste_cells_with(&mut book, id, 3, 3, &clipboard, PasteMode::All, true)?;
        assert!(
            book.active_sheet()
                .ok_or("sheet")?
                .merges
                .contains(&Range::new(3, 3, 4, 3))
        );
        paste_cells_with(&mut book, id, 3, 3, &clipboard, PasteMode::All, true)?;
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(3, 3))
                .map(|cell| cell.value.clone()),
            Some(Value::Text("Title".into()))
        );
        let before = book.clone();
        assert!(paste_cells_with(&mut book, id, 0, 1, &clipboard, PasteMode::All, false).is_err());
        assert_eq!(book, before);
        Ok(())
    }
}

#[cfg(test)]
mod phase200_editor_defects {
    use super::*;
    #[test]
    fn visible_merged_content_keeps_a_hidden_anchor_in_render_and_navigation() {
        let mut sheet = Sheet::default();
        sheet.merges.push(Range::new(0, 0, 1, 1));
        sheet.hidden_rows.insert(0);
        sheet.hidden_cols.insert(0);
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));
        let cells = visible_cells(&sheet, Pos2::ZERO, viewport, 0..5, 0..5, (0, 0), viewport);
        assert!(cells.contains(&(0, 0)));
        assert_eq!(jump_target(&sheet, (5, 5), (1, 1)), (0, 0));
        sheet.hidden_rows.insert(1);
        assert!(
            !visible_cells(&sheet, Pos2::ZERO, viewport, 0..5, 0..5, (0, 0), viewport)
                .contains(&(0, 0))
        );
    }
    #[test]
    fn hiding_an_active_axis_moves_focus_to_a_visible_cell() {
        let mut book = Book::default();
        assert!(hide_axis(&mut book, 0, 0, true));
        assert_eq!((book.selection.row, book.selection.col), (1, 0));
        assert!(hide_axis(&mut book, 1, 0, false));
        assert_eq!((book.selection.row, book.selection.col), (1, 1));
        assert_eq!(book.selection.range(), Range::new(1, 1, 1, 1));
    }
    #[test]
    fn pie_fractions_remain_finite_for_large_and_tiny_values() {
        for values in [[1e308, 1e308], [1e39, 3e39], [1e-300, 3e-300]] {
            let fractions = pie_fractions(&values);
            assert!(fractions.iter().all(|value| value.is_finite()));
            assert!((fractions.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        }
        assert_eq!(pie_fractions(&[0.0, -1.0]), vec![0.0, 0.0]);
    }
    #[test]
    fn proportional_image_resize_obeys_both_dimension_limits() {
        assert_eq!(
            constrained_image_size(Vec2::new(4000.0, 2000.0), Vec2::new(12000.0, 6000.0), true),
            Vec2::new(8000.0, 4000.0)
        );
        let tiny = constrained_image_size(Vec2::new(1.0, 8000.0), Vec2::ZERO, true);
        assert_eq!(tiny, Vec2::new(1.0, 8000.0));
        assert_eq!(
            constrained_image_size(Vec2::new(100.0, 100.0), Vec2::new(9000.0, 20.0), false),
            Vec2::new(8000.0, 20.0)
        );
    }
}

#[cfg(test)]
mod phase200_floating_chart_defect {
    use super::*;
    #[test]
    fn native_charts_use_their_model_anchor_and_remain_editable() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        let chart = book.add_chart(id, ChartKind::Column, Range::new(0, 0, 1, 1), "Floating")?;
        let sheet = book.active_sheet_mut().ok_or("sheet")?;
        assert!(place_chart(
            sheet,
            chart,
            Vec2::new(48.0 + 80.0 * 96.0, 28.0 + 2000.0 * 26.0),
            Vec2::new(500.0, 300.0)
        ));
        let model = sheet
            .charts
            .iter()
            .find(|item| item.id == chart)
            .ok_or("chart")?;
        assert_eq!(
            (model.row, model.col, model.width, model.height),
            (2000, 80, 500.0, 300.0)
        );
        let rect = floating_chart_rect(sheet, Pos2::ZERO, model);
        let (width, height) = grid_size(sheet, 50, 20);
        assert!(width >= rect.max.x + 16.0 && height >= rect.max.y + 16.0);
        assert!(!place_chart(sheet, chart, rect.min.to_vec2(), rect.size()));
        Ok(())
    }
}
