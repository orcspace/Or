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
    chart_kind: ChartKind,
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
    state.editing = false;
    let value = book
        .set_cell(state.last.0, state.last.1, state.last.2, &state.buffer)
        .map(|_| ());
    result(state, value)
}
pub fn show(ui: &mut egui::Ui, book: &mut Book) -> bool {
    let state_id = ui.id().with("sheets-editor");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id))
        .unwrap_or_default();
    let mut changed = false;
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
        state.last = (id, row, col);
        state.buffer = book
            .active_sheet()
            .and_then(|s| s.cell(row, col))
            .map(|c| c.input.clone())
            .unwrap_or_default();
    }
    let selected = book
        .active_sheet()
        .and_then(|s| s.cell(row, col))
        .map(|c| c.fmt.clone())
        .unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        let mut size = selected.size;
        if ui
            .add(
                egui::DragValue::new(&mut size)
                    .range(6.0..=72.0)
                    .speed(0.5)
                    .suffix(" pt"),
            )
            .changed()
        {
            changed |= result(&mut state, apply_format(book, |f| f.size = size));
        }
        let mut family = selected.font.clone();
        egui::ComboBox::from_id_salt("sheet-font")
            .width(100.0)
            .selected_text(&family)
            .show_ui(ui, |ui| {
                for name in [
                    "Calibri",
                    "Arial",
                    "Times New Roman",
                    "Courier New",
                    "Segoe UI",
                ] {
                    ui.selectable_value(&mut family, name.into(), name);
                }
            });
        if family != selected.font {
            changed |= result(&mut state, apply_format(book, |f| f.font = family.clone()));
        }
        for (label, flag) in [
            ("B", selected.bold),
            ("I", selected.italic),
            ("U", selected.underline),
            ("S", selected.strike),
        ] {
            if ui.selectable_label(flag, label).clicked() {
                changed |= result(
                    &mut state,
                    apply_format(book, |f| match label {
                        "B" => f.bold = !flag,
                        "I" => f.italic = !flag,
                        "U" => f.underline = !flag,
                        _ => f.strike = !flag,
                    }),
                );
            }
        }
        let mut number = selected.number.clone();
        egui::ComboBox::from_id_salt("sheet-format")
            .width(112.0)
            .selected_text(
                super::format::NUMBER_FORMATS
                    .iter()
                    .find(|(_, c)| *c == number)
                    .map(|(label, _)| *label)
                    .unwrap_or("Свой формат"),
            )
            .show_ui(ui, |ui| {
                for (label, code) in super::format::NUMBER_FORMATS {
                    ui.selectable_value(&mut number, code.to_string(), *label);
                }
            });
        if number != selected.number {
            changed |= result(
                &mut state,
                apply_format(book, |f| f.number = number.clone()),
            );
        }
        let mut fg = selected.color;
        if ui
            .color_edit_button_srgb(&mut fg)
            .on_hover_text("Цвет текста")
            .changed()
        {
            changed |= result(&mut state, apply_format(book, |f| f.color = fg));
        }
        let mut fill = selected.fill.unwrap_or([255, 255, 255]);
        if ui
            .color_edit_button_srgb(&mut fill)
            .on_hover_text("Заливка")
            .changed()
        {
            changed |= result(&mut state, apply_format(book, |f| f.fill = Some(fill)));
        }
        for (label, align) in [
            ("≡", Align::Left),
            ("↔", Align::Center),
            ("☰", Align::Right),
        ] {
            if ui
                .selectable_label(selected.align == align, label)
                .clicked()
            {
                changed |= result(&mut state, apply_format(book, |f| f.align = align));
            }
        }
        if ui.selectable_label(selected.wrap, "Перенос").clicked() {
            changed |= result(&mut state, apply_format(book, |f| f.wrap = !selected.wrap));
        }
        if ui.selectable_label(selected.border, "Границы").clicked() {
            changed |= result(
                &mut state,
                apply_format(book, |f| f.border = !selected.border),
            );
        }
        ui.menu_button("Данные", |ui| {
            if ui.button("Сортировать А → Я").clicked() {
                let range = book.selection.range();
                changed |= result(
                    &mut state,
                    book.sort(id, range, &[(range.c0, false)], false),
                );
                ui.close();
            }
            if ui.button("Сортировать Я → А").clicked() {
                let range = book.selection.range();
                changed |= result(&mut state, book.sort(id, range, &[(range.c0, true)], false));
                ui.close();
            }
            if ui.button("Заполнить вниз · Ctrl+D").clicked() {
                let target = book.selection.range();
                let source = Range::new(target.r0, target.c0, target.r0, target.c1);
                changed |= result(&mut state, book.fill(id, source, target));
                ui.close();
            }
            if ui.button("Заполнить вправо · Ctrl+R").clicked() {
                let target = book.selection.range();
                let source = Range::new(target.r0, target.c0, target.r1, target.c0);
                changed |= result(&mut state, book.fill(id, source, target));
                ui.close();
            }
            if ui.button("Объединить").clicked() {
                changed |= result(&mut state, book.merge(id, book.selection.range()));
                ui.close();
            }
            if ui.button("Разъединить").clicked() {
                let range = book.selection.range();
                if let Some(s) = book.active_sheet_mut() {
                    s.merges.retain(|m| {
                        !(m.r0 <= range.r1
                            && range.r0 <= m.r1
                            && m.c0 <= range.c1
                            && range.c0 <= m.c1)
                    });
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Закрепить до выбранной ячейки").clicked() {
                if let Some(s) = book.active_sheet_mut() {
                    s.frozen_rows = row;
                    s.frozen_cols = col;
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Снять закрепление").clicked() {
                if let Some(s) = book.active_sheet_mut() {
                    s.frozen_rows = 0;
                    s.frozen_cols = 0;
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Автоподбор ширины столбца").clicked() {
                if let Some(s) = book.active_sheet_mut() {
                    let width = s
                        .cells
                        .iter()
                        .filter(|(a, _)| {
                            super::model::parse_address(a).is_some_and(|(_, c)| c == col)
                        })
                        .map(|(_, c)| c.value.display().chars().count() as f32 * 7.5 + 18.0)
                        .fold(96.0, f32::max)
                        .min(600.0);
                    s.widths.insert(col, width);
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Пересчитать").clicked() {
                book.recalculate();
                changed = true;
                ui.close();
            }
            if ui.button("Найти и заменить · Ctrl+H").clicked() {
                state.find_open = true;
                ui.close();
            }
            if ui.button("Очистить формат").clicked() {
                changed |= result(
                    &mut state,
                    apply_format(book, |f| *f = CellFormat::default()),
                );
                ui.close();
            }
        });
        ui.menu_button("Строки / столбцы", |ui| {
            for (label, rows, delete) in [
                ("Вставить строку", true, false),
                ("Удалить строку", true, true),
                ("Вставить столбец", false, false),
                ("Удалить столбец", false, true),
            ] {
                if ui.button(label).clicked() {
                    changed |= result(
                        &mut state,
                        book.edit_dimension(id, if rows { row } else { col }, 1, rows, delete),
                    );
                    ui.close();
                }
            }
            if ui.button("Скрыть строку").clicked() {
                if let Some(s) = book.active_sheet_mut() {
                    s.hidden_rows.insert(row);
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Скрыть столбец").clicked() {
                if let Some(s) = book.active_sheet_mut() {
                    s.hidden_cols.insert(col);
                    changed = true;
                }
                ui.close();
            }
            if ui.button("Показать скрытые").clicked() {
                if let Some(s) = book.active_sheet_mut() {
                    s.hidden_rows.clear();
                    s.hidden_cols.clear();
                    changed = true;
                }
                ui.close();
            }
        });
        ui.menu_button("Диаграмма", |ui| {
            for (label, kind) in [
                ("Столбцы", ChartKind::Column),
                ("Линейчатая", ChartKind::Bar),
                ("Линии", ChartKind::Line),
                ("Область", ChartKind::Area),
                ("Круговая", ChartKind::Pie),
                ("Кольцевая", ChartKind::Doughnut),
                ("Точечная", ChartKind::Scatter),
            ] {
                if ui.button(label).clicked() {
                    state.chart_kind = kind;
                    changed |= result(
                        &mut state,
                        book.add_chart(id, kind, book.selection.range(), "Диаграмма")
                            .map(|_| ()),
                    );
                    ui.close();
                }
            }
        });
    });
    ui.horizontal(|ui| {
        ui.monospace(address(row, col));
        ui.label("ƒx");
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.buffer)
                .desired_width(ui.available_width())
                .id(state_id.with("formula")),
        );
        if response.changed() {
            state.editing = true;
        }
        if response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            changed |= commit(book, &mut state);
            response.surrender_focus();
        }
        if response.lost_focus() {
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
    if !state.error.is_empty() {
        ui.colored_label(Color32::from_rgb(200, 60, 50), &state.error);
    }
    if state.find_open {
        ui.horizontal(|ui| {
            ui.label("Найти");
            ui.add(egui::TextEdit::singleline(&mut state.find).desired_width(140.0));
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
                    book.selection = Selection {
                        row: r,
                        col: c,
                        anchor_row: r,
                        anchor_col: c,
                    };
                }
            }
            ui.label("Заменить");
            ui.add(egui::TextEdit::singleline(&mut state.replace).desired_width(140.0));
            if ui.button("Все").clicked() && !state.find.is_empty() {
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
                            ui.label(format!("{:?}", chart.kind));
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
            let rows = (max_row.saturating_add(100)).clamp(1000, MAX_ROWS);
            let cols = (max_col.saturating_add(20)).clamp(26, MAX_COLS);
            let width = axis_pos(cols, 96.0, &sheet.widths, &sheet.hidden_cols) + 48.0;
            let height = axis_pos(rows, 26.0, &sheet.row_heights, &sheet.hidden_rows) + 28.0;
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
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
                        cell_rect.max.x = origin.x
                            + 48.0
                            + axis_pos(merge.c1 + 1, 96.0, &sheet.widths, &sheet.hidden_cols);
                        cell_rect.max.y = origin.y
                            + 28.0
                            + axis_pos(merge.r1 + 1, 26.0, &sheet.row_heights, &sheet.hidden_rows);
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
                            Color32::from_rgba_unmultiplied(40, 150, 105, 35),
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
                    let family = match (fmt.bold, fmt.italic) {
                        (true, true) => "bold-italic",
                        (true, false) => "bold",
                        (false, true) => "italic",
                        _ => "system",
                    };
                    let font = FontId::new(fmt.size * 1.333, egui::FontFamily::Name(family.into()));
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
                    clipped.text(
                        point,
                        align,
                        text,
                        font,
                        if ui.visuals().dark_mode && fmt.color == [28, 32, 38] {
                            ui.visuals().text_color()
                        } else {
                            color(fmt.color)
                        },
                    );
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
                            Stroke::new(2.0, Color32::from_rgb(35, 150, 100)),
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
                painter.rect_filled(header, 0.0, ui.visuals().panel_fill);
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
                painter.rect_filled(header, 0.0, ui.visuals().panel_fill);
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
                    let extended = ui.input(|i| i.modifiers.shift) || response.dragged();
                    changed |= commit(book, &mut state);
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
            if !state.editing && !ui.ctx().egui_wants_keyboard_input() {
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
                            state.buffer = text;
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
                            if modifiers.ctrl && (key == Key::F || key == Key::H) {
                                state.find_open = true;
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
                            let step = if modifiers.ctrl { 20 } else { 1 };
                            let selection = &mut book.selection;
                            match key {
                                Key::ArrowUp => selection.row = selection.row.saturating_sub(step),
                                Key::ArrowDown | Key::Enter => {
                                    selection.row = (selection.row + step).min(MAX_ROWS - 1)
                                }
                                Key::ArrowLeft => {
                                    selection.col = selection.col.saturating_sub(step)
                                }
                                Key::ArrowRight | Key::Tab => {
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
                            let target = Rect::from_min_size(
                                origin
                                    + Vec2::new(
                                        48.0 + selection.col as f32 * 96.0,
                                        28.0 + selection.row as f32 * 26.0,
                                    ),
                                Vec2::new(96.0, 26.0),
                            );
                            ui.scroll_to_rect(target, None);
                        }
                        _ => (),
                    }
                }
            }
        });
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
            if response.clicked() {
                changed |= commit(book, &mut state);
                book.active = index;
                book.selection = Selection::default();
            }
            response.context_menu(|ui| {
                ui.label("Имя листа");
                if state.rename.is_empty() {
                    state.rename = name.clone();
                }
                ui.text_edit_singleline(&mut state.rename);
                if ui.button("Переименовать").clicked() {
                    let name = state.rename.clone();
                    changed |= result(&mut state, book.rename_sheet(sid, &name));
                    state.rename.clear();
                    ui.close();
                }
                if ui.button("Переместить в начало").clicked() {
                    changed |= result(&mut state, book.move_sheet(sid, 0));
                    ui.close();
                }
                if ui.button("Удалить лист").clicked() {
                    changed |= result(&mut state, book.delete_sheet(sid));
                    ui.close();
                }
            });
        }
        if ui.button("+").clicked() {
            let mut index = book.sheets.len() + 1;
            loop {
                let name = format!("Лист{index}");
                if !book.sheets.iter().any(|s| s.name == name) {
                    changed |= result(&mut state, book.add_sheet(&name).map(|_| ()));
                    book.active = book.sheets.len() - 1;
                    break;
                }
                index += 1;
            }
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
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
    changed
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
