use super::model::*;
use egui::{Align2, Color32, Event, FontId, ImeEvent, Key, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Clone)]
struct Clipboard {
    text: String,
    paragraphs: Vec<Paragraph>,
    complete: Vec<bool>,
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
/// The table cell being edited in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TableCursor {
    table: u64,
    row: usize,
    column: usize,
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
    table_cell: Option<TableCursor>,
    table_page: Option<usize>,
    preferred_x: Option<f32>,
    focus_cell: bool,
    stats: Option<(usize, usize)>,
    show_objects: bool,
    search: String,
    replacement: String,
    case_sensitive: bool,
    match_index: usize,
    find_notice: String,
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
            table_cell: None,
            table_page: None,
            preferred_x: None,
            focus_cell: false,
            stats: None,
            show_objects: false,
            search: String::new(),
            replacement: String::new(),
            case_sensitive: false,
            match_index: 0,
            find_notice: String::new(),
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
        self.preferred_x = None;
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
        if self.table_cell.is_some() {
            self.message = Some("Эта команда недоступна внутри ячейки таблицы".into());
            return false;
        }
        let (a, b) = self.range();
        match doc.replace_range(a, b, text, Some(self.typing.clone())) {
            Ok(pos) => {
                self.preferred_x = None;
                self.cursor = pos;
                self.anchor = pos;
                self.scroll_to_cursor = true;
                true
            }
            Err(error) => {
                self.message = Some(error);
                false
            }
        }
    }
    fn apply_format(&mut self, doc: &mut Doc, apply: impl Fn(&mut TextFormat)) {
        apply(&mut self.typing);
        if let Some(cell) = active_cell(doc, self.table_cell) {
            format_cell(cell, &apply);
        } else if self.selected() {
            let (a, b) = self.range();
            doc.format_range(a, b, apply);
        } else if let Some(p) = doc
            .paragraphs
            .get_mut(self.cursor.paragraph)
            .filter(|p| p.is_empty())
        {
            for run in &mut p.runs {
                apply(&mut run.format);
            }
        }
    }
}

pub(crate) fn reconcile_text_edit(ctx: &egui::Context, id: egui::Id, text: &str) {
    let previous =
        ctx.data(|data| data.get_temp::<std::sync::Arc<str>>(id.with("model-text-baseline")));
    if let Some(previous) = previous.filter(|previous| previous.as_ref() != text)
        && let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id)
    {
        if let Some(mut range) = state.cursor.char_range() {
            range.primary.index = rebase_text_offset(&previous, text, range.primary.index.0).into();
            range.secondary.index =
                rebase_text_offset(&previous, text, range.secondary.index.0).into();
            range.h_pos = None;
            state.cursor.set_char_range(Some(range));
        }
        state.clear_undoer();
        state.store(ctx, id);
    }
}
pub(crate) fn remember_text_edit(ctx: &egui::Context, id: egui::Id, text: &str) {
    ctx.data_mut(|data| {
        let key = id.with("model-text-baseline");
        if data
            .get_temp::<std::sync::Arc<str>>(key)
            .is_none_or(|previous| previous.as_ref() != text)
        {
            data.insert_temp(key, std::sync::Arc::<str>::from(text));
        }
    });
}
fn rebase_text_offset(old: &str, new: &str, offset: usize) -> usize {
    let old: Vec<_> = old.chars().collect();
    let new: Vec<_> = new.chars().collect();
    let offset = offset.min(old.len());
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(old.len().min(new.len()).saturating_sub(prefix))
        .take_while(|(a, b)| a == b)
        .count();
    if offset <= prefix {
        offset
    } else if offset >= old.len().saturating_sub(suffix) {
        new.len().saturating_sub(old.len().saturating_sub(offset))
    } else {
        prefix
    }
}
fn cell_ref(doc: &Doc, cursor: TableCursor) -> Option<&Cell> {
    doc.tables
        .iter()
        .find(|table| table.id == cursor.table)?
        .rows
        .get(cursor.row)?
        .get(cursor.column)
}
fn canonical_cell(doc: &Doc, mut cursor: TableCursor) -> Option<TableCursor> {
    let table = doc.tables.iter().find(|table| table.id == cursor.table)?;
    let start = *table.grid_starts(cursor.row).get(cursor.column)?;
    let span = cell_ref(doc, cursor)?.span();
    while cell_ref(doc, cursor)?.merged {
        cursor.row = cursor.row.checked_sub(1)?;
        cursor.column = table
            .grid_starts(cursor.row)
            .iter()
            .position(|grid| *grid == start)?;
        if cell_ref(doc, cursor)?.span() != span {
            return None;
        }
    }
    Some(cursor)
}
fn rebase_active_cell(old: &Doc, new: &Doc, cursor: TableCursor) -> Option<TableCursor> {
    let id = cell_ref(old, cursor)?.id;
    for table in &new.tables {
        for (row, cells) in table.rows.iter().enumerate() {
            if let Some(column) = cells.iter().position(|cell| cell.id == id) {
                return canonical_cell(
                    new,
                    TableCursor {
                        table: table.id,
                        row,
                        column,
                    },
                );
            }
        }
    }
    None
}
fn current_paragraph<'a>(doc: &'a Doc, state: &EditorState) -> Option<&'a Paragraph> {
    if let Some(cursor) = state.table_cell {
        cell_ref(doc, cursor)?.paragraphs.first()
    } else {
        doc.paragraphs.get(state.cursor.paragraph)
    }
}
fn insert_ribbon_text(
    ui: &egui::Ui,
    doc: &mut Doc,
    state: &mut EditorState,
    text: &str,
    canvas_id: egui::Id,
) {
    if state.table_cell.is_some() {
        ui.ctx()
            .input_mut(|input| input.events.push(Event::Text(text.into())));
    } else {
        state.insert(doc, text);
    }
    refocus(ui, doc, state, canvas_id);
}

pub fn request_focus(ctx: &egui::Context, tab_id: u64, paragraph_id: u64) {
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("docs-focus-request"), (tab_id, paragraph_id))
    });
    ctx.request_repaint();
}
pub fn show(ui: &mut egui::Ui, doc: &mut Doc) -> bool {
    show_with_edits(ui, doc, None)
}
/// Where an AI agent just edited: drawn as a black caret with a fading trail.
#[derive(Clone, Copy, Debug)]
pub struct AiCaret {
    pub tab: u64,
    pub paragraph_id: u64,
    pub from: usize,
    pub to: usize,
    /// `ctx.input(|i| i.time)` when the edit landed.
    pub time: f64,
}
const AI_CARET_SECONDS: f64 = 2.6;
pub fn ai_caret_id() -> egui::Id {
    egui::Id::new("trinedocs-ai-caret")
}
pub fn active_tab_id() -> egui::Id {
    egui::Id::new("trinedocs-active-tab")
}
/// The selected text of the document editor and its tab, for the AI chat.
pub fn selection_id() -> egui::Id {
    egui::Id::new("trinedocs-doc-selection")
}
fn paint_ai_caret(
    ui: &egui::Ui,
    doc: &Doc,
    layout: &Layout,
    page_origin: egui::Pos2,
    page_height: f32,
    gap: f32,
    scale: f32,
) {
    let ctx = ui.ctx();
    let Some(caret) = ctx.data(|d| d.get_temp::<AiCaret>(ai_caret_id())) else {
        return;
    };
    let tab = ctx.data(|d| d.get_temp::<u64>(active_tab_id()));
    let age = ctx.input(|i| i.time) - caret.time;
    if tab != Some(caret.tab) || !(0.0..AI_CARET_SECONDS).contains(&age) {
        return;
    }
    let Some(paragraph) = doc
        .paragraphs
        .iter()
        .position(|p| p.id == caret.paragraph_id)
    else {
        return;
    };
    // Fade out over the last second; the caret blinks gently meanwhile.
    let fade = ((AI_CARET_SECONDS - age) as f32).clamp(0.0, 1.0);
    let blink = 0.65 + 0.35 * ((age * 7.0).cos() as f32).abs();
    let len = doc.paragraphs.get(paragraph).map_or(0, Paragraph::len);
    let caret_to = caret.to.min(len);
    let (from, to) = (caret.from.min(caret_to), caret_to.max(caret.from).min(len));
    let painter = ui.painter();
    for row in layout.rows.iter().filter(|r| r.paragraph == paragraph) {
        let origin = page_origin + egui::vec2(0.0, row.page as f32 * (page_height + gap));
        let a = from.max(row.start);
        let b = to.min(row.end);
        if b > a {
            let x1 = row_x(row, a);
            let x2 = row_x(row, b);
            painter.rect_filled(
                Rect::from_min_size(
                    origin + egui::vec2(x1 * scale, row.y * scale),
                    egui::vec2((x2 - x1) * scale, row.height * scale),
                ),
                2.0,
                Color32::from_black_alpha((34.0 * fade) as u8),
            );
        }
        if caret_to >= row.start && caret_to <= row.end {
            let x = origin.x + row_x(row, caret_to) * scale;
            let top = origin.y + row.y * scale;
            let bottom = top + caret_height(row, caret_to, None) * scale;
            let ink = Color32::from_black_alpha((235.0 * fade * blink) as u8);
            painter.line_segment(
                [egui::pos2(x, top), egui::pos2(x, bottom)],
                Stroke::new(2.0, ink),
            );
            let tag = Rect::from_min_size(egui::pos2(x, top - 13.0), egui::vec2(20.0, 12.0));
            painter.rect_filled(tag, 3.0, Color32::from_black_alpha((235.0 * fade) as u8));
            painter.text(
                tag.center(),
                Align2::CENTER_CENTER,
                "AI",
                FontId::proportional(9.0),
                Color32::from_white_alpha((255.0 * fade) as u8),
            );
            break;
        }
    }
    ctx.request_repaint();
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
    let follow_typing = state.previous.as_deref().is_some_and(|previous| {
        current_paragraph(previous, &state).is_some_and(|p| {
            p.format_at(if state.table_cell.is_some() {
                0
            } else {
                state.cursor.offset
            }) == state.typing
        })
    });
    if let Some(previous) = state.previous.as_deref()
        && previous != doc
        && let Some(cursor) = state.table_cell
    {
        state.table_cell = rebase_active_cell(previous, doc, cursor);
        state.table_page = None;
    }
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
    if follow_typing && let Some(paragraph) = current_paragraph(doc, &state) {
        state.typing = paragraph.format_at(if state.table_cell.is_some() {
            0
        } else {
            state.cursor.offset
        });
    }
    if ui.is_enabled() {
        poll_image(doc, &mut state);
    }
    if ui.is_enabled()
        && let Some(png) = crate::clipboard::take_image(ui.ctx())
    {
        paste_image(doc, &mut state, &png);
    }
    if let Some((tab, paragraph_id)) = ui
        .ctx()
        .data(|data| data.get_temp::<(u64, u64)>(egui::Id::new("docs-focus-request")))
        && ui.ctx().data(|data| data.get_temp::<u64>(active_tab_id())) == Some(tab)
    {
        ui.ctx()
            .data_mut(|data| data.remove::<(u64, u64)>(egui::Id::new("docs-focus-request")));
        if let Some(paragraph) = doc.paragraphs.iter().position(|p| p.id == paragraph_id) {
            state.table_cell = None;
            state.table_page = None;
            state.move_to(
                doc,
                Position {
                    paragraph,
                    offset: 0,
                },
                false,
            );
            state.cursor.offset = doc.paragraphs[paragraph].len();
            ui.memory_mut(|memory| memory.request_focus(canvas_id));
        }
    }
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
                state.preferred_x = None;
                let cell = response
                    .interact_pointer_pos()
                    .and_then(|p| cell_at(doc, &layout, p, page_origin, scale, page_height, gap));
                if let Some((cell, page)) = cell {
                    state.table_cell = Some(cell);
                    state.table_page = Some(page);
                    state.focus_cell = true;
                    state.format_painter = None;
                    sync_cell_typing(doc, &mut state);
                } else {
                    state.table_cell = None;
                    response.request_focus();
                }
                if cell.is_none()
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    let pointer = if response.drag_started() {
                        ui.input(|i| i.pointer.press_origin()).unwrap_or(pointer)
                    } else {
                        pointer
                    };
                    let pos = hit_test(&layout, pointer, page_origin, scale, page_height, gap);
                    let shift = ui.input(|i| i.modifiers.shift);
                    if response.triple_clicked() {
                        state.anchor = Position {
                            paragraph: pos.paragraph,
                            offset: 0,
                        };
                        let end = Position {
                            paragraph: pos.paragraph,
                            offset: doc
                                .paragraphs
                                .get(pos.paragraph)
                                .map(Paragraph::len)
                                .unwrap_or_default(),
                        };
                        let anchor = state.anchor;
                        mouse_selection(doc, &mut state, anchor, end);
                    } else if response.double_clicked() {
                        let (a, b) = word_range(doc, pos);
                        mouse_selection(doc, &mut state, a, b);
                    } else {
                        state.move_to(doc, pos, shift);
                    }
                }
            }
            if response.dragged()
                && state.table_cell.is_none()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let cursor = hit_test(&layout, pointer, page_origin, scale, page_height, gap);
                let anchor = state.anchor;
                mouse_selection(doc, &mut state, anchor, cursor);
                state.scroll_to_cursor = false;
                state.preferred_x = None;
            }
            if (response.drag_stopped() || response.clicked())
                && state.selected()
                && let Some(fmt) = state.format_painter.take()
            {
                state.typing = fmt.clone();
                let (a, b) = state.range();
                doc.format_range(a, b, |f| *f = fmt.clone());
            }
            let focused = response.has_focus();
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
            let active_row = layout.rows.iter().rev().find(|r| {
                r.paragraph == state.cursor.paragraph
                    && state.cursor.offset >= r.start
                    && state.cursor.offset <= r.end
            });
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
                        egui::vec2(
                            1.5,
                            caret_height(row, state.cursor.offset, Some(&state.typing)) * scale,
                        ),
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
                        && active_row.is_some_and(|active| std::ptr::eq(active, row))
                    {
                        let x = row_x(row, state.cursor.offset);
                        let rect = Rect::from_min_size(
                            origin + egui::vec2(x * scale, row.y * scale),
                            egui::vec2(
                                1.5,
                                caret_height(row, state.cursor.offset, Some(&state.typing)) * scale,
                            ),
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
            paint_ai_caret(ui, doc, &layout, page_origin, page_height, gap, scale);
            cell_editor(
                ui,
                doc,
                &mut state,
                &layout,
                page_origin,
                scale,
                page_height,
                gap,
                canvas_id,
            );
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
    if state.stats.is_none() || state.previous.as_deref() != Some(doc) {
        state.stats = Some((doc.word_count(), doc.plain_text().chars().count()));
    }
    let (words, chars) = state.stats.unwrap_or_default();
    ui.horizontal(|ui| {
        ui.weak(format!(
            "Страница {} из {}  ·  {} слов  ·  {} знаков",
            current_page(&layout, &state),
            layout.pages.len(),
            words,
            chars
        ));
        ui.weak(match state.table_cell {
            Some(c) => format!("Таблица · ячейка {}:{}", c.row + 1, c.column + 1),
            None => format!(
                "Абзац {} : {}",
                state.cursor.paragraph + 1,
                state.cursor.offset + 1
            ),
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::chrome::tool(ui, "+", "Увеличить", false).clicked() {
                state.zoom = (state.zoom + 0.1).min(2.5);
            }
            ui.spacing_mut().slider_width = 90.0;
            ui.add(egui::Slider::new(&mut state.zoom, 0.5..=2.5).show_value(false));
            if crate::chrome::tool(ui, "−", "Уменьшить", false).clicked() {
                state.zoom = (state.zoom - 0.1).max(0.5);
            }
            if ui
                .add(
                    egui::Button::new(format!("{:.0}%", state.zoom * 100.0))
                        .frame_when_inactive(false),
                )
                .on_hover_text("Масштаб 100%")
                .clicked()
            {
                state.zoom = 1.0;
            }
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
    let selection = editor_selection(ui.ctx(), doc, &state, canvas_id);
    let tab = ui.ctx().data(|d| d.get_temp::<u64>(active_tab_id()));
    ui.ctx().data_mut(|d| {
        d.insert_temp(selection_id(), (tab, selection));
        d.insert_temp(state_id, state);
    });
    changed
}
fn editor_selection(
    ctx: &egui::Context,
    doc: &Doc,
    state: &EditorState,
    canvas_id: egui::Id,
) -> String {
    if let Some(cell) = state.table_cell.and_then(|cursor| cell_ref(doc, cursor)) {
        return egui::text_edit::TextEditState::load(ctx, cell_editor_id(canvas_id, doc, state))
            .and_then(|widget| widget.cursor.char_range())
            .map(|range| {
                let [start, end] = range.sorted_cursors();
                cell_text(cell)
                    .chars()
                    .skip(start.index.0)
                    .take(end.index.0.saturating_sub(start.index.0))
                    .collect()
            })
            .unwrap_or_default();
    }
    if state.selected() {
        let (start, end) = state.range();
        doc.selected_text(start, end)
    } else {
        String::new()
    }
}
fn current_page(layout: &Layout, state: &EditorState) -> usize {
    if let Some(cursor) = state.table_cell {
        return layout
            .cells
            .iter()
            .filter(|cell| {
                cell.table == cursor.table && cell.row == cursor.row && cell.column == cursor.column
            })
            .min_by_key(|cell| cell.page.abs_diff(state.table_page.unwrap_or(cell.page)))
            .map_or(1, |cell| cell.page + 1);
    }
    layout
        .rows
        .iter()
        .rev()
        .find(|row| {
            row.paragraph == state.cursor.paragraph
                && state.cursor.offset >= row.start
                && state.cursor.offset <= row.end
        })
        .map_or(1, |row| row.page + 1)
}
fn cell_editor_id(canvas_id: egui::Id, doc: &Doc, state: &EditorState) -> egui::Id {
    let cell = state
        .table_cell
        .and_then(|cursor| cell_ref(doc, cursor))
        .map(|cell| cell.id);
    canvas_id.with(("table-cell-editor", cell))
}
/// Returns keyboard focus to where the user was typing: the active table cell or the page.
fn refocus(ui: &egui::Ui, doc: &Doc, state: &EditorState, canvas_id: egui::Id) {
    egui::Popup::close_all(ui.ctx());
    let target = if state.table_cell.is_some() {
        cell_editor_id(canvas_id, doc, state)
    } else {
        canvas_id
    };
    ui.memory_mut(|m| m.request_focus(target));
}
fn table_mut(doc: &mut Doc, id: u64) -> Option<&mut Table> {
    doc.tables.iter_mut().find(|t| t.id == id)
}
fn active_cell(doc: &mut Doc, cursor: Option<TableCursor>) -> Option<&mut Cell> {
    let cursor = cursor?;
    table_mut(doc, cursor.table)?
        .rows
        .get_mut(cursor.row)?
        .get_mut(cursor.column)
}
fn table_columns(table: &Table) -> usize {
    table.grid_columns()
}
fn default_column_width(doc: &Doc, columns: usize) -> f32 {
    (doc.page.width - doc.page.margin_left - doc.page.margin_right) / columns.max(1) as f32
}
/// Row/column commands of the contextual "Макет таблицы" tab.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TableCommand {
    RowAbove,
    RowBelow,
    ColumnLeft,
    ColumnRight,
    DeleteRow,
    DeleteColumn,
    DeleteTable,
}
/// Leaves a removed table: the caret goes to the paragraph the table followed.
fn leave_table(doc: &Doc, state: &mut EditorState, anchor: Option<u64>) {
    state.table_cell = None;
    if let Some(paragraph) = anchor.and_then(|id| doc.paragraphs.iter().position(|p| p.id == id)) {
        let offset = doc
            .paragraphs
            .get(paragraph)
            .map(Paragraph::len)
            .unwrap_or_default();
        state.move_to(doc, Position { paragraph, offset }, false);
    }
}
fn table_command(doc: &mut Doc, state: &mut EditorState, command: TableCommand) {
    let Some(cursor) = state.table_cell else {
        return;
    };
    let anchor = doc
        .tables
        .iter()
        .find(|t| t.id == cursor.table)
        .map(|t| t.after_paragraph_id);
    // Column commands address grid columns; merged cells cover several of them.
    let (grid, span) = doc
        .tables
        .iter()
        .find(|t| t.id == cursor.table)
        .map(|t| {
            (
                t.grid_starts(cursor.row)
                    .get(cursor.column)
                    .copied()
                    .unwrap_or(cursor.column),
                t.rows
                    .get(cursor.row)
                    .and_then(|r| r.get(cursor.column))
                    .map(Cell::span)
                    .unwrap_or(1),
            )
        })
        .unwrap_or((cursor.column, 1));
    let (edit, target) = match command {
        TableCommand::RowAbove => (TableEdit::InsertRow(cursor.row), cursor),
        TableCommand::RowBelow => (
            TableEdit::InsertRow(cursor.row + 1),
            TableCursor {
                row: cursor.row + 1,
                ..cursor
            },
        ),
        TableCommand::ColumnLeft => (TableEdit::InsertColumn(grid), cursor),
        TableCommand::ColumnRight => (
            TableEdit::InsertColumn(grid + span),
            TableCursor {
                column: cursor.column + 1,
                ..cursor
            },
        ),
        TableCommand::DeleteRow => (TableEdit::DeleteRow(cursor.row), cursor),
        TableCommand::DeleteColumn => (TableEdit::DeleteColumn(grid), cursor),
        TableCommand::DeleteTable => {
            if let Err(error) = doc.delete_table(cursor.table) {
                state.message = Some(error);
            }
            leave_table(doc, state, anchor);
            return;
        }
    };
    match doc.edit_table(cursor.table, edit) {
        Ok(true) => {
            let table = doc.tables.iter().find(|t| t.id == cursor.table);
            let rows = table.map(|t| t.rows.len()).unwrap_or(1);
            let row = target.row.min(rows.saturating_sub(1));
            let columns = table
                .and_then(|t| t.rows.get(row))
                .map(Vec::len)
                .unwrap_or(1);
            state.table_cell = canonical_cell(
                doc,
                TableCursor {
                    row,
                    column: target.column.min(columns.saturating_sub(1)),
                    ..cursor
                },
            );
        }
        Ok(false) => leave_table(doc, state, anchor),
        Err(error) => state.message = Some(error),
    }
}
/// Moves to the next/previous cell; Tab in the last cell adds a row, as in Word.
fn step_cell(doc: &mut Doc, state: &mut EditorState, forward: bool) {
    let Some(cursor) = state.table_cell else {
        return;
    };
    let Some(table) = doc.tables.iter().find(|t| t.id == cursor.table) else {
        state.table_cell = None;
        return;
    };
    let visible = table
        .rows
        .iter()
        .enumerate()
        .flat_map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .filter(|(_, cell)| !cell.merged)
                .map(move |(column, _)| TableCursor {
                    row,
                    column,
                    ..cursor
                })
        })
        .collect::<Vec<_>>();
    if let Some(index) = visible.iter().position(|candidate| *candidate == cursor) {
        let target = if forward {
            visible.get(index + 1)
        } else {
            index.checked_sub(1).and_then(|index| visible.get(index))
        };
        if let Some(target) = target {
            state.table_cell = Some(*target);
        } else if forward {
            let at = table.rows.len();
            match doc.edit_table(cursor.table, TableEdit::InsertRow(at)) {
                Ok(true) => {
                    state.table_cell = canonical_cell(
                        doc,
                        TableCursor {
                            row: at,
                            column: 0,
                            ..cursor
                        },
                    )
                }
                Ok(false) => state.table_cell = None,
                Err(error) => state.message = Some(error),
            }
        }
    }
    state.table_page = None;
    state.focus_cell = true;
}
fn table_grid_picker(
    ui: &mut egui::Ui,
    doc: &mut Doc,
    state: &mut EditorState,
    canvas_id: egui::Id,
) {
    if state.table_cell.is_some() {
        ui.weak("Вложенные таблицы пока не поддерживаются");
        return;
    }

    const COLUMNS: usize = 10;
    const ROWS: usize = 8;
    const STEP: f32 = 19.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(COLUMNS as f32 * STEP, ROWS as f32 * STEP),
        Sense::click(),
    );
    let hovered = response.hover_pos().map(|p| {
        let local = p - rect.min;
        (
            ((local.x / STEP) as usize).min(COLUMNS - 1),
            ((local.y / STEP) as usize).min(ROWS - 1),
        )
    });
    let accent = crate::chrome::accent(ui);
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let square = Rect::from_min_size(
                rect.min + egui::vec2(column as f32 * STEP, row as f32 * STEP),
                egui::vec2(STEP - 3.0, STEP - 3.0),
            );
            let on = hovered.is_some_and(|(c, r)| column <= c && row <= r);
            ui.painter().rect_filled(
                square,
                1.0,
                if on {
                    accent.gamma_multiply(0.25)
                } else {
                    ui.visuals().window_fill
                },
            );
            ui.painter().rect_stroke(
                square,
                1.0,
                Stroke::new(
                    1.0,
                    if on {
                        accent
                    } else {
                        crate::chrome::border(ui)
                    },
                ),
                egui::StrokeKind::Inside,
            );
        }
    }
    ui.label(match hovered {
        Some((c, r)) => format!("Таблица {} × {}", c + 1, r + 1),
        None => "Вставка таблицы".into(),
    });
    if response.clicked()
        && let Some((c, r)) = hovered
    {
        state.table_columns = c + 1;
        state.table_rows = r + 1;
        insert_table(doc, state);
        refocus(ui, doc, state, canvas_id);
        ui.close();
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Строк");
        ui.add(egui::DragValue::new(&mut state.table_rows).range(1..=1000));
        ui.label("Столбцов");
        ui.add(egui::DragValue::new(&mut state.table_columns).range(1..=64));
    });
    if ui.button("Вставить таблицу").clicked() {
        insert_table(doc, state);
        refocus(ui, doc, state, canvas_id);
        ui.close();
    }
}
fn style_card(ui: &mut egui::Ui, style: &str, selected: bool) -> egui::Response {
    let (size, bold, italic) = match style {
        "Title" => (17.0, false, false),
        "Heading1" => (16.0, true, false),
        "Heading2" => (15.0, true, false),
        "Heading3" => (14.0, true, false),
        "Quote" => (13.0, false, true),
        _ => (13.0, false, false),
    };
    let family = crate::fonts::egui_family(ui.ctx(), crate::fonts::DEFAULT_FAMILY, bold, italic);
    ui.add_sized(
        [220.0, 30.0],
        egui::Button::new(egui::RichText::new(style_label(style)).font(FontId::new(size, family)))
            .selected(selected)
            .frame_when_inactive(false),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
fn command_menu(ui: &mut egui::Ui, title: &str, width: f32, content: impl FnOnce(&mut egui::Ui)) {
    ui.menu_button(title, |ui| {
        ui.set_min_width(width.clamp(180.0, 320.0));
        ui.spacing_mut().item_spacing.y = 8.0;
        content(ui);
    });
}
fn toolbar(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    if state
        .table_cell
        .is_some_and(|c| active_cell(doc, Some(c)).is_none())
    {
        state.table_cell = None;
    }
    let mut labels = vec!["Главная", "Вставка", "Макет", "Вид"];
    if state.table_cell.is_some() {
        labels.push("Макет таблицы");
    } else if state.ribbon_tab > 3 {
        state.ribbon_tab = 0;
    }
    crate::chrome::tabs(ui, &mut state.ribbon_tab, &labels);
    crate::chrome::ribbon(ui, |ui| match state.ribbon_tab {
        1 => insert_tab(ui, doc, state, canvas_id),
        2 => layout_tab(ui, doc, state, canvas_id),
        3 => view_tab(ui, doc, state, canvas_id),
        4 => table_tab(ui, doc, state, canvas_id),
        _ => home_tab(ui, doc, state, canvas_id),
    });
}
fn home_tab(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    use crate::chrome::tool;
    ui.push_id("docs-quick-commands", |ui| {
        if tool(ui, "Вставить", "Текст или рисунок · Ctrl+V", false).clicked()
        {
            refocus(ui, doc, state, canvas_id);
            crate::clipboard::paste(ui.ctx());
        }
        crate::chrome::separator(ui);
        let mut font = state.typing.font.clone();
        if crate::chrome::font_picker(ui, "docs-font-quick", &mut font, 128.0) {
            state.apply_format(doc, |format| format.font = font.clone());
            refocus(ui, doc, state, canvas_id);
        }
        let mut size = state.typing.size;
        if crate::chrome::size_picker(ui, "docs-size-quick", &mut size) {
            state.apply_format(doc, |format| format.size = size.clamp(4.0, 144.0));
        }
        let typing = state.typing.clone();
        for (label, tip, on, index) in [
            (
                egui::RichText::new("Ж").strong(),
                "Полужирный · Ctrl+B",
                typing.bold,
                0,
            ),
            (
                egui::RichText::new("К").italics(),
                "Курсив · Ctrl+I",
                typing.italic,
                1,
            ),
            (
                egui::RichText::new("Ч").underline(),
                "Подчёркнутый · Ctrl+U",
                typing.underline,
                2,
            ),
        ] {
            if tool(ui, label, tip, on).clicked() {
                state.apply_format(doc, |format| match index {
                    0 => format.bold = !on,
                    1 => format.italic = !on,
                    _ => format.underline = !on,
                });
                refocus(ui, doc, state, canvas_id);
            }
        }
        if let Some(Some(color)) =
            crate::chrome::color_menu(ui, "A", "Цвет шрифта", Some(typing.color), None)
        {
            state.apply_format(doc, |format| format.color = color);
            refocus(ui, doc, state, canvas_id);
        }
    });
    command_menu(ui, "Стили", 240.0, |ui| {
        let current = doc
            .paragraphs
            .get(state.cursor.paragraph)
            .map(|p| p.style.clone())
            .unwrap_or_else(|| "Normal".into());
        ui.vertical(|ui| {
            ui.add_enabled_ui(state.table_cell.is_none(), |ui| {
                for &style in DOCS_STYLE_CHOICES {
                    if style_card(ui, style, current == style).clicked() {
                        apply_style(doc, state, style);
                        refocus(ui, doc, state, canvas_id);
                    }
                }
            });
        });
    });
    command_menu(ui, "Абзац", 168.0, |ui| {
        let (list, current) = current_paragraph(doc, state)
            .map(|p| (p.list, p.align))
            .unwrap_or_default();
        ui.horizontal(|ui| {
            for (glyph, tip, kind) in [
                ("•≡", "Маркеры", List::Bullet),
                ("1≡", "Нумерация", List::Number),
            ] {
                if tool(ui, glyph, tip, list == kind).clicked() {
                    selected_paragraphs(doc, state, |p| {
                        p.list = if p.list == kind { List::None } else { kind }
                    });
                    refocus(ui, doc, state, canvas_id);
                }
            }
            for (glyph, tip, delta) in [
                ("⇤", "Уменьшить отступ", -18.0),
                ("⇥", "Увеличить отступ", 18.0),
            ] {
                if tool(ui, glyph, tip, false).clicked() {
                    selected_paragraphs(doc, state, |p| {
                        p.indent = (p.indent + delta).clamp(0.0, 250.0)
                    });
                    refocus(ui, doc, state, canvas_id);
                }
            }
            ui.menu_button("↕", |ui| paragraph_spacing(ui, doc, state))
                .response
                .on_hover_text("Интервал между строками и абзацами");
        });
        ui.horizontal(|ui| {
            let current = if state.table_cell.is_some() {
                active_cell(doc, state.table_cell)
                    .and_then(|c| c.paragraphs.first())
                    .map(|p| p.align)
                    .unwrap_or_default()
            } else {
                current
            };
            for (index, (tip, align)) in [
                ("Выровнять по левому краю · Ctrl+L", Align::Left),
                ("По центру · Ctrl+E", Align::Center),
                ("Выровнять по правому краю · Ctrl+R", Align::Right),
                ("По ширине · Ctrl+J", Align::Justify),
            ]
            .into_iter()
            .enumerate()
            {
                if crate::chrome::alignment_button(ui, index, tip, current == align).clicked() {
                    selected_paragraphs(doc, state, |p| p.align = align);
                    refocus(ui, doc, state, canvas_id);
                }
            }
        });
    });
    if tool(ui, "Найти", "Найти · Ctrl+F", state.show_find).clicked() {
        toggle_find(ui, state, canvas_id);
    }
    ui.menu_button("Ещё", |ui| {
        command_menu(ui, "Буфер обмена", 96.0, |ui| {
            if tool(
                ui,
                "📋  Вставить",
                "Вставить текст или рисунок · Ctrl+V",
                false,
            )
            .clicked()
            {
                refocus(ui, doc, state, canvas_id);
                crate::clipboard::paste(ui.ctx());
            }
            ui.horizontal(|ui| {
                let can_copy = state.selected() || state.table_cell.is_some();
                for (glyph, tip, command) in [
                    ("✂", "Вырезать · Ctrl+X", egui::ViewportCommand::RequestCut),
                    (
                        "⧉",
                        "Копировать · Ctrl+C",
                        egui::ViewportCommand::RequestCopy,
                    ),
                ] {
                    if ui
                        .add_enabled_ui(can_copy, |ui| tool(ui, glyph, tip, false))
                        .inner
                        .clicked()
                    {
                        refocus(ui, doc, state, canvas_id);
                        ui.ctx().send_viewport_cmd(command);
                    }
                }
                let painter_on = state.format_painter.is_some();
                if tool(
                    ui,
                    "🖌",
                    "Формат по образцу: выделите текст-образец, нажмите и выделите целевой текст",
                    painter_on,
                )
                .clicked()
                {
                    state.format_painter = if painter_on {
                        None
                    } else {
                        Some(state.typing.clone())
                    };
                    refocus(ui, doc, state, canvas_id);
                }
            });
        });
        command_menu(ui, "Шрифт", 262.0, |ui| {
            ui.horizontal(|ui| {
                let mut font = state.typing.font.clone();
                if crate::chrome::font_picker(ui, "docs-font", &mut font, 130.0) {
                    state.apply_format(doc, |f| f.font = font.clone());
                    refocus(ui, doc, state, canvas_id);
                }
                let mut size = state.typing.size;
                if crate::chrome::size_picker(ui, "docs-size", &mut size) {
                    let size = size.clamp(4.0, 144.0);
                    state.apply_format(doc, |f| f.size = size);
                }
                for (glyph, tip, grow) in [
                    ("A⁺", "Увеличить размер шрифта · Ctrl+]", true),
                    ("A⁻", "Уменьшить размер шрифта · Ctrl+[", false),
                ] {
                    if tool(ui, glyph, tip, false).clicked() {
                        let size =
                            crate::chrome::step_size(state.typing.size, grow).clamp(4.0, 144.0);
                        state.apply_format(doc, |f| f.size = size);
                        refocus(ui, doc, state, canvas_id);
                    }
                }
            });
            ui.horizontal(|ui| {
                let t = state.typing.clone();
                for (label, tip, on, which) in [
                    (
                        egui::RichText::new("Ж").strong(),
                        "Полужирный · Ctrl+B",
                        t.bold,
                        0,
                    ),
                    (
                        egui::RichText::new("К").italics(),
                        "Курсив · Ctrl+I",
                        t.italic,
                        1,
                    ),
                    (
                        egui::RichText::new("Ч").underline(),
                        "Подчёркнутый · Ctrl+U",
                        t.underline,
                        2,
                    ),
                    (
                        egui::RichText::new("abc").strikethrough(),
                        "Зачёркнутый",
                        t.strike,
                        3,
                    ),
                    (
                        egui::RichText::new("x₂"),
                        "Подстрочный знак",
                        t.script == -1,
                        4,
                    ),
                    (
                        egui::RichText::new("x²"),
                        "Надстрочный знак",
                        t.script == 1,
                        5,
                    ),
                ] {
                    if tool(ui, label, tip, on).clicked() {
                        state.apply_format(doc, |f| match which {
                            0 => f.bold = !on,
                            1 => f.italic = !on,
                            2 => f.underline = !on,
                            3 => f.strike = !on,
                            4 => f.script = if on { 0 } else { -1 },
                            _ => f.script = if on { 0 } else { 1 },
                        });
                        refocus(ui, doc, state, canvas_id);
                    }
                }
                if let Some(color) = crate::chrome::color_menu(
                    ui,
                    "ab",
                    "Цвет выделения текста",
                    t.highlight.or(Some([255, 255, 0])),
                    Some("Нет цвета"),
                ) {
                    state.apply_format(doc, |f| f.highlight = color);
                    refocus(ui, doc, state, canvas_id);
                }
                if let Some(Some(color)) =
                    crate::chrome::color_menu(ui, "A", "Цвет шрифта", Some(t.color), None)
                {
                    state.apply_format(doc, |f| f.color = color);
                    refocus(ui, doc, state, canvas_id);
                }
                if tool(ui, "A✕", "Очистить всё форматирование", false).clicked()
                {
                    state.apply_format(doc, |f| *f = TextFormat::default());
                    refocus(ui, doc, state, canvas_id);
                }
            });
        });
        command_menu(ui, "Редактирование", 104.0, |ui| {
            if tool(ui, "🔍  Найти", "Найти · Ctrl+F", false).clicked() {
                toggle_find(ui, state, canvas_id);
            }
            ui.horizontal(|ui| {
                if tool(ui, "Заменить", "Заменить · Ctrl+H", false).clicked() {
                    open_find(ui, state, canvas_id, true);
                }
                if tool(ui, "⬚", "Выделить всё · Ctrl+A", false).clicked() {
                    state.table_cell = None;
                    let paragraph = doc.paragraphs.len().saturating_sub(1);
                    let offset = doc
                        .paragraphs
                        .last()
                        .map(Paragraph::len)
                        .unwrap_or_default();
                    state.anchor = Position::default();
                    state.cursor = Position { paragraph, offset };
                    refocus(ui, doc, state, canvas_id);
                }
            });
        });
    });
}
fn insert_tab(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    use crate::chrome::tool;
    let big = |text: &str| {
        egui::Button::new(egui::RichText::new(text).size(12.5))
            .frame_when_inactive(false)
            .min_size(egui::vec2(60.0, 28.0))
    };
    let response = ui.add(big("Таблица")).on_hover_text("Вставить таблицу");
    egui::Popup::menu(&response).show(|ui| table_grid_picker(ui, doc, state, canvas_id));
    let response = ui
        .add_enabled(state.pending_image.is_none(), big("Рисунок…"))
        .on_hover_text("Вставить рисунок из файла (PNG, JPEG)");
    if response.clicked() {
        start_image_load(ui, doc, state);
    }
    if !doc.images.is_empty() {
        response.context_menu(|ui| {
            if ui.button("Размер и описание…").clicked() {
                state.show_objects = true;
                ui.close();
            }
        });
    }
    let response = ui
        .add(big("Ссылка"))
        .on_hover_text("Вставить гиперссылку · Ctrl+K");
    egui::Popup::menu(&response).show(|ui| {
        ui.label("Адрес");
        ui.add(
            egui::TextEdit::singleline(&mut state.link)
                .hint_text("https://")
                .desired_width(260.0),
        );
        if ui
            .add_enabled(!state.link.trim().is_empty(), egui::Button::new("Вставить"))
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
                f.color = [5, 99, 193];
            });
            egui::Popup::close_all(ui.ctx());
            ui.memory_mut(|m| m.request_focus(canvas_id));
            ui.close();
        }
    });
    command_menu(ui, "Колонтитулы", 150.0, |ui| {
        ui.menu_button(
            "Верхний и нижний колонтитулы…",
            |ui| page_headers(ui, doc, canvas_id),
        );
        ui.checkbox(&mut doc.page.page_numbers, "Номера страниц");
    });
    command_menu(ui, "Текст и символы", 150.0, |ui| {
        ui.horizontal(|ui| {
            let symbols = tool(ui, "Ω  Символ", "Вставить символ", false);
            egui::Popup::menu(&symbols).show(|ui| {
                ui.set_max_width(230.0);
                ui.horizontal_wrapped(|ui| {
                    for ch in [
                        "©", "®", "™", "€", "₽", "$", "£", "°", "±", "×", "÷", "≠", "≤", "≥", "—",
                        "–", "…", "§", "¶", "•", "→", "←", "✓", "∞", "½", "¼", "α", "β", "π", "Σ",
                    ] {
                        if ui
                            .add(egui::Button::new(ch).min_size(egui::vec2(26.0, 26.0)))
                            .clicked()
                        {
                            insert_ribbon_text(ui, doc, state, ch, canvas_id);
                            ui.close();
                        }
                    }
                });
            });
            if tool(ui, "📅  Дата", "Вставить текущую дату", false).clicked()
            {
                insert_ribbon_text(ui, doc, state, &today(), canvas_id);
            }
        });
        if ui
            .add_enabled_ui(state.table_cell.is_none(), |ui| {
                tool(
                    ui,
                    "⤓  Разрыв страницы",
                    "Начать новую страницу · Ctrl+Enter",
                    false,
                )
            })
            .inner
            .clicked()
        {
            if state.insert(doc, "\n")
                && let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph)
            {
                p.page_break_before = true;
            }
            egui::Popup::close_all(ui.ctx());
            ui.memory_mut(|m| m.request_focus(canvas_id));
        }
        if tool(
            ui,
            "🗑  Удалить пустую страницу",
            "Удалить пустую страницу с курсором, иначе все пустые страницы документа",
            false,
        )
        .clicked()
        {
            delete_empty_pages(doc, state);
            egui::Popup::close_all(ui.ctx());
            ui.memory_mut(|m| m.request_focus(canvas_id));
        }
    });
}
/// Deletes the empty page under the caret, or every empty page when it has content.
fn delete_empty_pages(doc: &mut Doc, state: &mut EditorState) {
    let layout = doc.layout();
    let current = layout
        .rows
        .iter()
        .rev()
        .find(|r| {
            r.paragraph == state.cursor.paragraph
                && state.cursor.offset >= r.start
                && state.cursor.offset <= r.end
        })
        .map(|r| r.page);
    let mut removed = current.is_some_and(|page| doc.delete_empty_page(page)) as usize;
    if removed == 0 {
        // Walk from the end so earlier page numbers stay valid; bounded by the page count.
        let mut page = doc.layout().pages.len();
        while page > 0 {
            page -= 1;
            if doc.delete_empty_page(page) {
                removed += 1;
                page = page.min(doc.layout().pages.len());
            }
        }
    }
    state.cursor = doc.clamp_position(state.cursor);
    state.anchor = state.cursor;
    state.message = Some(if removed == 0 {
        "Пустых страниц нет".into()
    } else {
        format!("Удалено пустых страниц: {removed}")
    });
}
fn layout_tab(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    let previous_page = doc.page.clone();
    command_menu(ui, "Параметры страницы", 250.0, |ui| {
        ui.horizontal(|ui| {
            ui.menu_button("Поля", |ui| {
                for (name, value) in [("Обычные", 72.0), ("Узкие", 36.0), ("Широкие", 144.0)]
                {
                    if ui
                        .button(format!("{name} · {:.1} см", value / 28.35))
                        .clicked()
                    {
                        doc.page.margin_left = value;
                        doc.page.margin_right = value;
                        doc.page.margin_top = value;
                        doc.page.margin_bottom = value;
                        ui.close();
                    }
                }
                ui.separator();
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
            });
            ui.menu_button("Ориентация", |ui| {
                for (name, landscape) in [("Книжная", false), ("Альбомная", true)] {
                    if ui
                        .selectable_label((doc.page.width > doc.page.height) == landscape, name)
                        .clicked()
                    {
                        if (doc.page.width > doc.page.height) != landscape {
                            std::mem::swap(&mut doc.page.width, &mut doc.page.height);
                        }
                        ui.close();
                    }
                }
            });
            ui.menu_button("Размер", |ui| {
                for (name, width, height) in [
                    ("A4 · 21 × 29,7 см", 595.28, 841.89),
                    ("Letter · 21,6 × 27,9 см", 612.0, 792.0),
                    ("A3 · 29,7 × 42 см", 841.89, 1190.55),
                    ("A5 · 14,8 × 21 см", 419.53, 595.28),
                    ("Legal · 21,6 × 35,6 см", 612.0, 1008.0),
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
            });
        });
        if ui
            .add_enabled(
                state.table_cell.is_none(),
                egui::Button::new("Разрывы страниц").frame_when_inactive(false),
            )
            .on_hover_text("Вставить разрыв страницы перед текущим абзацем")
            .clicked()
            && let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph)
        {
            p.page_break_before = !p.page_break_before;
            egui::Popup::close_all(ui.ctx());
            ui.memory_mut(|m| m.request_focus(canvas_id));
        }
    });
    command_menu(ui, "Абзац", 300.0, |ui| {
        paragraph_controls(ui, doc, state, true);
    });
    if doc.page != previous_page {
        doc.page.fit_margins();
    }
}
fn view_tab(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    use crate::chrome::tool;
    command_menu(ui, "Масштаб", 230.0, |ui| {
        ui.horizontal(|ui| {
            for (label, zoom) in [("75%", 0.75), ("100%", 1.0), ("125%", 1.25), ("150%", 1.5)] {
                if tool(ui, label, "Масштаб", (state.zoom - zoom).abs() < 0.01).clicked() {
                    state.zoom = zoom;
                }
            }
        });
        if tool(ui, "↔  По ширине страницы", "По ширине страницы", false).clicked()
        {
            state.zoom =
                ((ui.ctx().content_rect().width() - 100.0) / doc.page.width).clamp(0.5, 2.5);
        }
    });
    command_menu(ui, "Навигация", 150.0, |ui| {
        outline_menu(ui, doc, state);
        if tool(ui, "🔍  Поиск", "Найти / заменить · Ctrl+F / Ctrl+H", false).clicked()
        {
            toggle_find(ui, state, canvas_id);
        }
    });
    command_menu(ui, "Объекты", 120.0, |ui| {
        if ui
            .add_enabled(
                !doc.images.is_empty(),
                egui::Button::new("Рисунки…").frame_when_inactive(false),
            )
            .clicked()
        {
            state.show_objects = true;
        }
    });
}
fn table_tab(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState, canvas_id: egui::Id) {
    use crate::chrome::tool;
    let Some(cursor) = state.table_cell else {
        return;
    };
    let mut command = None;
    command_menu(ui, "Строки и столбцы", 196.0, |ui| {
        ui.horizontal(|ui| {
            if tool(ui, "⬆  Вставить сверху", "Вставить строку сверху", false).clicked()
            {
                command = Some(TableCommand::RowAbove);
            }
            if tool(ui, "⬇  Снизу", "Вставить строку снизу", false).clicked()
            {
                command = Some(TableCommand::RowBelow);
            }
        });
        ui.horizontal(|ui| {
            if tool(ui, "⬅  Вставить слева", "Вставить столбец слева", false).clicked()
            {
                command = Some(TableCommand::ColumnLeft);
            }
            if tool(ui, "➡  Справа", "Вставить столбец справа", false).clicked()
            {
                command = Some(TableCommand::ColumnRight);
            }
        });
    });
    command_menu(ui, "Удаление", 176.0, |ui| {
        ui.horizontal(|ui| {
            if tool(ui, "Удалить строку", "Удалить текущую строку", false).clicked()
            {
                command = Some(TableCommand::DeleteRow);
            }
            if tool(ui, "Столбец", "Удалить текущий столбец", false).clicked()
            {
                command = Some(TableCommand::DeleteColumn);
            }
        });
        if tool(ui, "✕  Удалить таблицу", "Удалить всю таблицу", false).clicked()
        {
            command = Some(TableCommand::DeleteTable);
        }
    });
    command_menu(ui, "Оформление", 170.0, |ui| {
        ui.horizontal(|ui| {
            let fill = active_cell(doc, Some(cursor)).map(|c| c.fill);
            if let Some(color) =
                crate::chrome::color_menu(ui, "◩", "Заливка ячейки", fill, Some("Нет заливки"))
                && let Some(cell) = active_cell(doc, Some(cursor))
            {
                cell.fill = color.unwrap_or([255, 255, 255]);
            }
            let border = doc
                .tables
                .iter()
                .find(|t| t.id == cursor.table)
                .map(|t| t.border_color);
            if let Some(Some(color)) =
                crate::chrome::color_menu(ui, "▦", "Цвет границ таблицы", border, None)
                && let Some(table) = table_mut(doc, cursor.table)
            {
                table.border_color = color;
            }
            if let Some(table) = table_mut(doc, cursor.table) {
                let mut borders = !table.borderless;
                if ui
                    .checkbox(&mut borders, "Границы")
                    .on_hover_text("Показывать границы таблицы")
                    .changed()
                {
                    table.borderless = !borders;
                }
            }
        });
        if let Some(cell) = active_cell(doc, Some(cursor)) {
            ui.horizontal(|ui| {
                for (value, glyph, tip) in [
                    (VAlign::Top, "⤒", "По верхнему краю"),
                    (VAlign::Center, "↕", "По центру по вертикали"),
                    (VAlign::Bottom, "⤓", "По нижнему краю"),
                ] {
                    if ui
                        .selectable_label(cell.v_align == value, glyph)
                        .on_hover_text(tip)
                        .clicked()
                    {
                        cell.v_align = value;
                    }
                }
            });
        }
        ui.weak(format!("Ячейка {}:{}", cursor.row + 1, cursor.column + 1));
    });
    command_menu(ui, "Размер ячейки", 150.0, |ui| {
        let fallback = {
            let columns = doc
                .tables
                .iter()
                .find(|t| t.id == cursor.table)
                .map(table_columns)
                .unwrap_or(1);
            default_column_width(doc, columns)
        };
        if let Some(table) = table_mut(doc, cursor.table) {
            let columns = table_columns(table);
            let mut widths = table.column_widths.clone();
            widths.resize(columns.max(widths.len()), fallback);
            let grid = table
                .grid_starts(cursor.row)
                .get(cursor.column)
                .copied()
                .unwrap_or(cursor.column);
            let mut widths_changed = false;
            if let Some(width) = widths.get_mut(grid) {
                ui.horizontal(|ui| {
                    ui.label("Ширина");
                    widths_changed |= ui
                        .add(
                            egui::DragValue::new(width)
                                .range(12.0..=2000.0)
                                .suffix(" pt"),
                        )
                        .changed();
                });
            }
            if ui
                .add(egui::Button::new("Выровнять ширину столбцов").frame_when_inactive(false))
                .clicked()
            {
                let total: f32 = widths.iter().take(columns).sum();
                let each = total / columns.max(1) as f32;
                for width in widths.iter_mut().take(columns) {
                    *width = each;
                }
                widths_changed = true;
            }
            if widths_changed {
                table.column_widths = widths;
            }
        }
    });
    if let Some(command) = command {
        egui::Popup::close_all(ui.ctx());
        table_command(doc, state, command);
        state.focus_cell = state.table_cell.is_some();
        if state.table_cell.is_none() {
            egui::Popup::close_all(ui.ctx());
            ui.memory_mut(|m| m.request_focus(canvas_id));
        }
    }
}
fn selected_paragraphs(doc: &mut Doc, state: &EditorState, apply: impl Fn(&mut Paragraph)) {
    if let Some(cell) = active_cell(doc, state.table_cell) {
        cell.paragraphs.iter_mut().for_each(apply);
        return;
    }
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
    state.typing = current_paragraph(doc, state)
        .map(|p| {
            p.format_at(if state.table_cell.is_some() {
                0
            } else {
                state.cursor.offset
            })
        })
        .unwrap_or_default();
}
fn insert_table(doc: &mut Doc, state: &mut EditorState) {
    if state.table_cell.is_some() {
        state.message = Some("Вложенные таблицы пока не поддерживаются".into());
        return;
    }
    if let Some(id) = doc.paragraphs.get(state.cursor.paragraph).map(|p| p.id) {
        match doc.add_table(id, state.table_rows, state.table_columns) {
            Ok(table) => {
                state.table_cell = Some(TableCursor {
                    table,
                    row: 0,
                    column: 0,
                });
                state.focus_cell = true;
                sync_cell_typing(doc, state);
            }
            Err(error) => state.message = Some(error),
        }
    }
}
fn paragraph_controls(ui: &mut egui::Ui, doc: &mut Doc, state: &EditorState, indent: bool) {
    let Some(p) = current_paragraph(doc, state).cloned() else {
        return;
    };
    if indent {
        ui.horizontal(|ui| {
            for (label, old, side) in [
                ("Отступ слева", p.indent, 0),
                ("Отступ справа", p.right_indent, 1),
                ("Первая строка", p.first_indent, 2),
            ] {
                let mut value = old;
                ui.label(label);
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .range(-10000.0..=10000.0)
                            .suffix(" pt"),
                    )
                    .changed()
                {
                    selected_paragraphs(doc, state, |p| match side {
                        0 => p.indent = value,
                        1 => p.right_indent = value,
                        _ => p.first_indent = value,
                    });
                }
            }
        });
    }
    ui.horizontal(|ui| {
        for (label, old, after) in [
            ("До", p.spacing_before, false),
            ("После", p.spacing_after, true),
        ] {
            let mut value = old;
            ui.label(label);
            if ui
                .add(
                    egui::DragValue::new(&mut value)
                        .range(0.0..=1000.0)
                        .suffix(" pt"),
                )
                .changed()
            {
                selected_paragraphs(doc, state, |p| {
                    if after {
                        p.spacing_after = value
                    } else {
                        p.spacing_before = value
                    }
                });
            }
        }
    });
}
fn paragraph_spacing(ui: &mut egui::Ui, doc: &mut Doc, state: &EditorState) {
    ui.weak("Междустрочный интервал");
    let current = current_paragraph(doc, state)
        .map(|p| p.line_spacing)
        .unwrap_or(1.0);
    for value in [1.0_f32, 1.15, 1.5, 2.0, 2.5, 3.0] {
        if ui
            .selectable_label((current - value).abs() < 0.01, format!("{value}"))
            .clicked()
        {
            selected_paragraphs(doc, state, |p| {
                p.line_spacing = value;
                p.line_points = 0.0;
            });
            ui.close();
        }
    }
    ui.separator();
    paragraph_controls(ui, doc, state, false);
    let mut page_break = current_paragraph(doc, state).is_some_and(|p| p.page_break_before);
    if ui
        .add_enabled(
            state.table_cell.is_none(),
            egui::Checkbox::new(&mut page_break, "С новой страницы"),
        )
        .changed()
    {
        selected_paragraphs(doc, state, |p| p.page_break_before = page_break);
    }
}
fn page_headers(ui: &mut egui::Ui, doc: &mut Doc, canvas_id: egui::Id) {
    for (label, salt, text) in [
        ("Верхний колонтитул", "header", &mut doc.page.header),
        ("Нижний колонтитул", "footer", &mut doc.page.footer),
    ] {
        let id = canvas_id.with(salt);
        ui.label(label);
        reconcile_text_edit(ui.ctx(), id, text);
        ui.add(egui::TextEdit::singleline(&mut *text).id(id));
        remember_text_edit(ui.ctx(), id, text);
    }
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
                    state.table_cell = None;
                    state.table_page = None;
                    state.focus_cell = false;
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
fn fitted_image_size(width: u32, height: u32) -> (f32, f32) {
    let width = width.max(1) as f32;
    let height = height.max(1) as f32;
    let scale = (450.0 / width).min(2000.0 / height).min(1.0);
    (width * scale, height * scale)
}
fn start_image_load(ui: &egui::Ui, doc: &Doc, state: &mut EditorState) {
    if state.table_cell.is_some() {
        state.message = Some("Вставка изображения в ячейку пока не поддерживается".into());
        return;
    }
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
                let (width, height) = fitted_image_size(image.width(), image.height());
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
/// Inserts a picture from the clipboard after the paragraph with the caret, like Word.
fn paste_image(doc: &mut Doc, state: &mut EditorState, png: &[u8]) {
    if state.table_cell.is_some() {
        state.message = Some("Вставка изображения в ячейку пока не поддерживается".into());
        return;
    }
    let Some(anchor) = doc.paragraphs.get(state.cursor.paragraph).map(|p| p.id) else {
        return;
    };
    match crate::util::decode_image(png) {
        Ok(image) => {
            let (width, height) = fitted_image_size(image.width(), image.height());
            let id = doc.alloc_id();
            doc.images.push(AnchoredImage {
                id,
                after_paragraph_id: anchor,
                bytes: png.to_vec(),
                width,
                height,
                alt: String::new(),
            });
            state.message = None;
        }
        Err(error) => state.message = Some(error),
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
    egui::Popup::close_all(ui.ctx());
    state.show_find = true;
    state.find_focus = Some(replacement);
    state.preedit.clear();
    ui.memory_mut(|m| m.request_focus(find_field_id(canvas_id, replacement)));
}
fn toggle_find(ui: &mut egui::Ui, state: &mut EditorState, canvas_id: egui::Id) {
    if state.show_find {
        egui::Popup::close_all(ui.ctx());
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
    if ui.is_enabled()
        && ui.memory(|m| m.has_focus(query_id) || m.has_focus(replacement_id))
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
    {
        state.show_find = false;
        state.find_focus = None;
        ui.memory_mut(|m| m.request_focus(canvas_id));
        return;
    }
    if let Some(replacement) = state.find_focus.take() {
        ui.memory_mut(|m| m.request_focus(find_field_id(canvas_id, replacement)));
    }
    let direction = if ui.is_enabled()
        && ui.memory(|memory| memory.has_focus(query_id))
        && ui.input_mut(|input| input.consume_key(input.modifiers, Key::Enter))
    {
        Some(ui.input(|input| input.modifiers.shift))
    } else {
        None
    };
    let mut navigation = direction;
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(8.0)
        .corner_radius(5.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Найти");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut state.search)
                            .char_limit(32767)
                            .id(query_id)
                            .desired_width(170.0)
                            .hint_text("Текст документа"),
                    )
                    .changed()
                {
                    state.match_index = 0;
                    state.find_notice.clear();
                }
                if ui
                    .checkbox(&mut state.case_sensitive, "Учитывать регистр")
                    .changed()
                {
                    state.find_notice.clear();
                }
                if ui.button("← Назад").on_hover_text("Shift+Enter").clicked() {
                    navigation = Some(true);
                }
                if ui.button("Далее →").on_hover_text("Enter").clicked() {
                    navigation = Some(false);
                }
                if ui.button("Закрыть").on_hover_text("Esc").clicked() {
                    state.show_find = false;
                    state.find_focus = None;
                    ui.memory_mut(|memory| memory.request_focus(canvas_id));
                }
            });
            let (matches, truncated) = find_panel_matches(doc, &state.search, state.case_sensitive);
            if let Some(backwards) = navigation {
                navigate_find_matches(doc, state, &matches, backwards);
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Заменить на");
                ui.add(
                    egui::TextEdit::singleline(&mut state.replacement)
                        .char_limit(32767)
                        .id(replacement_id)
                        .desired_width(170.0)
                        .hint_text("Новый текст"),
                );
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
                    match doc.try_replace_all(
                        &state.search,
                        &state.replacement,
                        state.case_sensitive,
                    ) {
                        Ok(count) => {
                            state.cursor = doc.clamp_position(state.cursor);
                            state.anchor = state.cursor;
                            state.message = None;
                            state.find_notice = format!("Заменено совпадений: {count}");
                        }
                        Err(error) => state.message = Some(error),
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if let Some(index) = matches.iter().position(|range| *range == state.range()) {
                    ui.small(format!("Совпадение {} из {}", index + 1, matches.len()));
                } else {
                    ui.small(format!("Совпадений: {}", matches.len()));
                }
                if !state.find_notice.is_empty() {
                    ui.separator();
                    ui.small(&state.find_notice);
                }
                if truncated {
                    ui.separator();
                    ui.small("Показаны первые100000 совпадений; уточните запрос.");
                }
                ui.separator();
                ui.small("Поиск в основном тексте документа.");
            });
        });
}
fn navigate_find_matches(
    doc: &Doc,
    state: &mut EditorState,
    matches: &[(Position, Position)],
    backwards: bool,
) {
    if matches.is_empty() {
        return;
    }
    let selected = matches.contains(&state.range());
    let index = if backwards {
        matches
            .iter()
            .rposition(|(start, end)| {
                if selected {
                    *start < state.range().0
                } else {
                    *end <= state.cursor
                }
            })
            .unwrap_or(matches.len() - 1)
    } else {
        matches
            .iter()
            .position(|(start, _)| {
                if selected {
                    *start > state.range().0
                } else {
                    *start >= state.cursor
                }
            })
            .unwrap_or(0)
    };
    let (start, end) = matches[index];
    select_match(doc, state, start, end);
    state.match_index = index + 1;
}

fn select_match(doc: &Doc, state: &mut EditorState, start: Position, end: Position) {
    state.table_cell = None;
    state.table_page = None;
    state.focus_cell = false;
    state.move_to(doc, end, false);
    state.anchor = start;
}
fn replace_current_match(doc: &mut Doc, state: &mut EditorState) -> bool {
    if !state.selected()
        || !doc
            .find_bounded(&state.search, state.case_sensitive, DOCS_UI_FIND_LIMIT + 1)
            .contains(&state.range())
    {
        return false;
    }
    let (start, end) = state.range();
    match doc.replace_range(start, end, &state.replacement, None) {
        Ok(position) => {
            state.move_to(doc, position, false);
            state.match_index = doc
                .find_bounded(&state.search, state.case_sensitive, DOCS_UI_FIND_LIMIT + 1)
                .partition_point(|(start, _)| *start < position);
            true
        }
        Err(error) => {
            state.message = Some(error);
            false
        }
    }
}
fn object_panels(ui: &mut egui::Ui, doc: &mut Doc, state: &mut EditorState) {
    let enabled = ui.is_enabled();
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
                                        .range(0.001..=10000.0)
                                        .clamp_existing_to_range(false)
                                        .suffix(" pt"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut image.height)
                                        .range(0.001..=10000.0)
                                        .clamp_existing_to_range(false)
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
fn format_cell(cell: &mut Cell, apply: impl Fn(&mut TextFormat)) {
    for paragraph in &mut cell.paragraphs {
        for run in &mut paragraph.runs {
            apply(&mut run.format);
        }
    }
}
/// Screen rectangle of a laid-out table cell (part).
fn cell_box_rect(
    cell: &CellBox,
    page_origin: Pos2,
    scale: f32,
    page_height: f32,
    gap: f32,
) -> Rect {
    let origin = page_origin + egui::vec2(0.0, cell.page as f32 * (page_height + gap));
    Rect::from_min_size(
        origin + egui::vec2(cell.x * scale, cell.y * scale),
        egui::vec2(cell.w * scale, cell.h * scale),
    )
}
fn cell_at(
    doc: &Doc,
    layout: &Layout,
    pointer: Pos2,
    page_origin: Pos2,
    scale: f32,
    page_height: f32,
    gap: f32,
) -> Option<(TableCursor, usize)> {
    layout
        .cells
        .iter()
        .find(|c| cell_box_rect(c, page_origin, scale, page_height, gap).contains(pointer))
        .and_then(|c| {
            canonical_cell(
                doc,
                TableCursor {
                    table: c.table,
                    row: c.row,
                    column: c.column,
                },
            )
            .map(|cursor| (cursor, c.page))
        })
}
/// Makes the ribbon show the format of the active cell.
fn sync_cell_typing(doc: &mut Doc, state: &mut EditorState) {
    if let Some(format) = active_cell(doc, state.table_cell)
        .and_then(|c| c.paragraphs.first())
        .map(|p| p.format_at(0))
    {
        state.typing = format;
    }
}
/// In-place cell editor drawn exactly over the cell on the page, as in Word.
#[allow(clippy::too_many_arguments)]
fn cell_editor(
    ui: &mut egui::Ui,
    doc: &mut Doc,
    state: &mut EditorState,
    layout: &Layout,
    page_origin: Pos2,
    scale: f32,
    page_height: f32,
    gap: f32,
    canvas_id: egui::Id,
) {
    let Some(cursor) = state.table_cell else {
        return;
    };
    let id = cell_editor_id(canvas_id, doc, state);
    if ui.is_enabled() && ui.memory(|m| m.has_focus(id)) {
        let (forward, back, escape) = ui.input_mut(|input| {
            let direction = cell_tab_direction(input);
            (
                direction == Some(true),
                direction == Some(false),
                input.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        for (key, which) in [(Key::B, 0), (Key::I, 1), (Key::U, 2)] {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                let t = state.typing.clone();
                state.apply_format(doc, |f| match which {
                    0 => f.bold = !t.bold,
                    1 => f.italic = !t.italic,
                    _ => f.underline = !t.underline,
                });
            }
        }
        for (key, align) in [
            (Key::L, Align::Left),
            (Key::E, Align::Center),
            (Key::R, Align::Right),
            (Key::J, Align::Justify),
        ] {
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                selected_paragraphs(doc, state, |p| p.align = align);
            }
        }
        for (key, grow) in [(Key::OpenBracket, false), (Key::CloseBracket, true)] {
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                let size = crate::chrome::step_size(state.typing.size, grow).clamp(4.0, 144.0);
                state.apply_format(doc, |format| format.size = size);
            }
        }
        if escape {
            state.table_cell = None;
            ui.memory_mut(|m| m.request_focus(canvas_id));
            return;
        }
        if forward || back {
            step_cell(doc, state, forward);
            sync_cell_typing(doc, state);
            ui.ctx().request_repaint();
            return;
        }
    }
    let Some(rect) = layout
        .cells
        .iter()
        .filter(|c| c.table == cursor.table && c.row == cursor.row && c.column == cursor.column)
        .min_by_key(|c| c.page.abs_diff(state.table_page.unwrap_or(c.page)))
        .map(|c| cell_box_rect(c, page_origin, scale, page_height, gap))
    else {
        // The cell is not laid out yet (new row/column): show it next frame.
        ui.ctx().request_repaint();
        return;
    };
    let padding = doc
        .tables
        .iter()
        .find(|table| table.id == cursor.table)
        .map(|table| table.cell_margin)
        .unwrap_or([5.0; 4]);
    let Some(cell) = active_cell(doc, Some(cursor)) else {
        state.table_cell = None;
        return;
    };
    let format = cell
        .paragraphs
        .first()
        .map(|p| p.format_at(0))
        .unwrap_or_default();
    let align = cell.paragraphs.first().map(|p| p.align).unwrap_or_default();
    let mut text = cell_text(cell);
    reconcile_text_edit(ui.ctx(), id, &text);
    ui.painter()
        .rect_filled(rect.shrink(1.0), 0.0, rgb(cell.fill));
    let family = if format.font.is_empty() {
        egui::FontFamily::Name("system".into())
    } else {
        crate::fonts::egui_family(ui.ctx(), &format.font, format.bold, format.italic)
    };
    let content_rect = cell_content_rect(rect, padding, scale);
    let editor = egui::TextEdit::multiline(&mut text)
        .id(id)
        .frame(egui::Frame::NONE)
        .margin(egui::Margin::ZERO)
        .font(FontId::new((format.size * scale).max(4.0), family))
        .text_color(rgb(format.color))
        .lock_focus(true)
        .desired_width(content_rect.width())
        .desired_rows(1)
        .horizontal_align(match align {
            Align::Center => egui::Align::Center,
            Align::Right => egui::Align::Max,
            _ => egui::Align::Min,
        });
    // The caret takes the text colour so it stays visible on white paper in dark theme too.
    let caret = rgb(format.color);
    let response = ui
        .scope(|ui| {
            let visuals = ui.visuals_mut();
            visuals.text_cursor.stroke = Stroke::new((1.5 * scale).clamp(1.0, 3.0), caret);
            visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(70, 130, 230, 90);
            visuals.selection.stroke = Stroke::new(1.0, caret);
            ui.put(content_rect, editor)
        })
        .inner;
    if state.focus_cell {
        state.focus_cell = false;
        response.request_focus();
    }
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(2.0, crate::chrome::accent(ui)),
        egui::StrokeKind::Inside,
    );
    remember_text_edit(ui.ctx(), id, &text);
    if response.changed() {
        let mut next = doc.next_id;
        if let Some(cell) = active_cell(doc, Some(cursor)) {
            if let Err(error) = try_edit_cell_text(cell, &text, &mut next) {
                state.message = Some(error);
            }
        }
        doc.next_id = next;
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
            if text.is_empty() {
                return;
            }
            let copied = ui
                .ctx()
                .data_mut(|d| d.get_temp::<Clipboard>(clipboard_id()))
                .filter(|c| c.text == text);
            let (a, b) = state.range();
            let keeps_suffix = doc
                .paragraphs
                .get(b.paragraph)
                .is_some_and(|paragraph| b.offset < paragraph.len());
            if !state.insert(doc, &text) {
                return;
            }
            if let Some(copy) = copied {
                for (i, p) in copy.paragraphs.iter().enumerate() {
                    let pi = a.paragraph + i;
                    if copy.complete.get(i) == Some(&true)
                        && (i > 0 || a.offset == 0)
                        && !(keeps_suffix && i + 1 == copy.paragraphs.len())
                        && let Some(target) = doc.paragraphs.get_mut(pi)
                    {
                        let id = target.id;
                        let runs = std::mem::take(&mut target.runs);
                        *target = p.clone();
                        target.id = id;
                        target.runs = runs;
                    }
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
                state.typing = doc
                    .paragraphs
                    .get(state.cursor.paragraph)
                    .map(|p| p.format_at(state.cursor.offset))
                    .unwrap_or_default();
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
            let mut complete = Vec::new();
            for pi in a.paragraph..=b.paragraph {
                if let Some(p) = doc.paragraphs.get(pi) {
                    let from = if pi == a.paragraph { a.offset } else { 0 };
                    let to = if pi == b.paragraph { b.offset } else { p.len() };
                    let mut selected = p.clone();
                    selected.runs = selected_runs(p, from, to);
                    complete.push(from == 0 && to == p.len());
                    paragraphs.push(selected);
                }
            }
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    clipboard_id(),
                    Clipboard {
                        text,
                        paragraphs,
                        complete,
                    },
                )
            });
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
            let a = surrounding_position(doc, state.cursor, before_chars, false);
            let b = surrounding_position(doc, state.cursor, after_chars, true);
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
            let navigation_layout;
            let layout = if matches!(
                key,
                Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown | Key::Home | Key::End
            ) {
                navigation_layout = doc.layout();
                &navigation_layout
            } else {
                layout
            };
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
                    Key::L | Key::E | Key::R | Key::J => {
                        let align = match key {
                            Key::L => Align::Left,
                            Key::E => Align::Center,
                            Key::R => Align::Right,
                            _ => Align::Justify,
                        };
                        selected_paragraphs(doc, state, |p| p.align = align);
                        return;
                    }
                    Key::OpenBracket | Key::CloseBracket => {
                        let size =
                            crate::chrome::step_size(state.typing.size, key == Key::CloseBracket)
                                .clamp(4.0, 144.0);
                        state.apply_format(doc, |format| format.size = size);
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
                    let x = state.preferred_x.unwrap_or_else(|| {
                        layout
                            .rows
                            .iter()
                            .rev()
                            .find(|r| {
                                r.paragraph == state.cursor.paragraph
                                    && state.cursor.offset >= r.start
                                    && state.cursor.offset <= r.end
                            })
                            .map_or(0.0, |r| row_x(r, state.cursor.offset))
                    });
                    let pos = vertical(layout, state.cursor, delta, x);
                    state.move_to(doc, pos, modifiers.shift);
                    state.preferred_x = Some(x);
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
                        let row = layout.rows.iter().rev().find(|r| {
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
                Key::Enter => insert_break(doc, state, modifiers.shift, ctrl),
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
fn insert_break(doc: &mut Doc, state: &mut EditorState, soft: bool, page: bool) {
    let paragraph = doc.paragraphs.get(state.cursor.paragraph);
    if !soft
        && !page
        && !state.selected()
        && paragraph.is_some_and(|p| p.is_empty() && p.list != List::None)
    {
        if let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph) {
            p.list = List::None;
            p.list_level = 0;
        }
        return;
    }
    let body_after_heading = !soft
        && !page
        && !state.selected()
        && paragraph.is_some_and(|p| {
            state.cursor.offset == p.len()
                && (heading_level(&p.style).is_some() || p.style == "Title")
        });
    if state.insert(doc, if soft { "\u{000b}" } else { "\n" })
        && let Some(p) = doc.paragraphs.get_mut(state.cursor.paragraph)
    {
        if body_after_heading {
            p.set_style("Normal");
            state.typing = p.format_at(0);
        }
        if page {
            p.page_break_before = true;
        }
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
fn word_class(ch: char) -> u8 {
    if ch.is_whitespace() {
        0
    } else if ch.is_alphanumeric() || ch == '_' {
        1
    } else {
        2
    }
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
        while offset > 0 && chars.get(offset).is_some_and(|c| word_class(*c) == 0) {
            offset -= 1;
        }
        let class = chars.get(offset).copied().map(word_class);
        while offset > 0 && chars.get(offset - 1).copied().map(word_class) == class {
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
        let class = chars.get(pos.offset).copied().map(word_class);
        while offset < len && chars.get(offset).copied().map(word_class) == class {
            offset += 1;
        }
        if class != Some(0) {
            while offset < len && chars.get(offset).is_some_and(|c| word_class(*c) == 0) {
                offset += 1;
            }
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
/// Widths of a segment's characters as laid out: tabs and justified spaces are wider than
/// their glyphs, so the segment's own width is shared out in proportion.
fn segment_advances(segment: &Segment) -> Vec<f32> {
    let count = segment.text.chars().count();
    if count == 1 {
        return vec![segment.width];
    }
    let natural: Vec<f32> = segment
        .text
        .chars()
        .map(|ch| char_width(ch, &segment.format))
        .collect();
    let total: f32 = natural.iter().sum();
    if total > 0.01 && (total - segment.width).abs() > 0.01 {
        let k = segment.width / total;
        natural.iter().map(|w| w * k).collect()
    } else {
        natural
    }
}
fn row_x(row: &Row, offset: usize) -> f32 {
    let mut remaining = offset.saturating_sub(row.start);
    let mut x = row.segments.first().map_or(row.x, |s| s.x);
    for segment in &row.segments {
        x = segment.x;
        for width in segment_advances(segment) {
            if remaining == 0 {
                return x;
            }
            x += width;
            remaining -= 1;
        }
    }
    x
}
fn row_offset(row: &Row, x: f32) -> usize {
    let mut offset = row.start;
    for segment in &row.segments {
        let mut cursor = segment.x;
        for width in segment_advances(segment) {
            if x < cursor + width / 2.0 {
                return offset;
            }
            cursor += width;
            offset += 1;
        }
    }
    offset.min(row.end)
}
/// Height of the text caret: the font of the character before it (as in Word), not the
/// whole line with its line spacing.
fn caret_height(row: &Row, offset: usize, typing: Option<&TextFormat>) -> f32 {
    let mut index = row.start;
    let mut format = row.segments.first().map(|s| &s.format);
    for segment in &row.segments {
        if index >= offset.max(row.start + 1) {
            break;
        }
        format = Some(&segment.format);
        index += segment.text.chars().count();
    }
    let format = if row.segments.is_empty() {
        typing
    } else {
        format
    };
    format
        .map(|f| line_height(f, f.size.max(4.0), 1.0))
        .unwrap_or(row.height)
        .min(row.height)
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
fn vertical(layout: &Layout, pos: Position, delta: isize, x: f32) -> Position {
    let index = layout
        .rows
        .iter()
        .rposition(|r| r.paragraph == pos.paragraph && pos.offset >= r.start && pos.offset <= r.end)
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
    #[test]
    fn review5_cell_widget_identity_follows_stable_ids_and_selection_uses_that_cell()
    -> Result<(), String> {
        let ctx = egui::Context::default();
        let mut doc = Doc::from_text("unrelated body selection");
        let table = doc.add_table(doc.paragraphs[0].id, 2, 1)?;
        doc.set_cell_text(table, 0, 0, "a🌍bc")?;
        let canvas = egui::Id::new("cell-selection");
        let mut state = EditorState {
            table_cell: Some(TableCursor {
                table,
                row: 0,
                column: 0,
            }),
            ..Default::default()
        };
        let first = cell_editor_id(canvas, &doc, &state);
        let mut widget = egui::text_edit::TextEditState::default();
        widget
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(1),
                egui::text::CCursor::new(3),
            )));
        widget.store(&ctx, first);
        assert_eq!(editor_selection(&ctx, &doc, &state, canvas), "🌍b");
        state.table_cell = Some(TableCursor {
            table,
            row: 1,
            column: 0,
        });
        assert_ne!(cell_editor_id(canvas, &doc, &state), first);
        state.table_cell = Some(TableCursor {
            table,
            row: 0,
            column: 0,
        });
        let before = doc.clone();
        doc.edit_table(table, TableEdit::InsertRow(0))?;
        state.table_cell = rebase_active_cell(&before, &doc, state.table_cell.ok_or("cursor")?);
        assert_eq!(cell_editor_id(canvas, &doc, &state), first);
        Ok(())
    }
    #[test]
    fn review5_status_page_follows_the_selected_table_fragment() {
        let cursor = TableCursor {
            table: 12,
            row: 1,
            column: 2,
        };
        let mut layout = Layout::default();
        for page in [0, 3] {
            layout.cells.push(CellBox {
                table: 12,
                row: 1,
                column: 2,
                page,
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            });
        }
        let state = EditorState {
            table_cell: Some(cursor),
            table_page: Some(3),
            ..Default::default()
        };
        assert_eq!(current_page(&layout, &state), 4);
    }
    #[test]
    fn review4_text_baseline_reuses_storage_and_rebases_external_unicode_edit() -> Result<(), String>
    {
        let ctx = egui::Context::default();
        let id = egui::Id::new("external-text-baseline");
        remember_text_edit(&ctx, id, "a🌍bc");
        let first = ctx
            .data(|d| d.get_temp::<std::sync::Arc<str>>(id.with("model-text-baseline")))
            .ok_or("baseline")?;
        remember_text_edit(&ctx, id, "a🌍bc");
        let second = ctx
            .data(|d| d.get_temp::<std::sync::Arc<str>>(id.with("model-text-baseline")))
            .ok_or("baseline")?;
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert_eq!(rebase_text_offset("a🌍bc", "prefix a🌍bc", 3), 10);
        Ok(())
    }

    #[test]
    fn tall_image_fit_preserves_ratio_and_passes_document_geometry_validation() -> Result<(), String>
    {
        let (w, h) = fitted_image_size(1, 32768);
        assert!(w > 0.0 && h <= 2000.0);
        assert!((w / h - 1.0 / 32768.0).abs() < 0.000001);
        let mut doc = Doc::from_text("image");
        let id = doc.alloc_id();
        let asset = image::DynamicImage::new_rgb8(1, 32768);
        let mut bytes = std::io::Cursor::new(Vec::new());
        asset
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        doc.images.push(AnchoredImage {
            id,
            after_paragraph_id: doc.paragraphs[0].id,
            bytes: bytes.into_inner(),
            width: w,
            height: h,
            alt: String::new(),
        });
        doc.validate()
    }
    #[test]
    fn canvas_alignment_and_size_shortcuts_apply_to_the_selection() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let mut doc = Doc::from_text("selected");
        let layout = doc.layout();
        let mut state = EditorState {
            cursor: Position {
                paragraph: 0,
                offset: 8,
            },
            ..EditorState::default()
        };
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let canvas_id = ui.id().with("shortcut-test");
            ui.memory_mut(|memory| memory.request_focus(canvas_id));
            for key in [Key::E, Key::CloseBracket] {
                handle_event(
                    ui,
                    &mut doc,
                    &layout,
                    &mut state,
                    canvas_id,
                    Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::CTRL,
                    },
                );
            }
        });
        output.textures_delta.clear();
        assert_eq!(doc.paragraphs[0].align, Align::Center);
        assert!(doc.paragraphs[0].runs[0].format.size > 11.0);
        Ok(())
    }

    #[test]
    fn find_selection_uses_the_matched_text_format_for_typing() {
        let mut doc = Doc::from_text("match");
        doc.paragraphs[0].runs[0].format.size = 24.0;
        doc.paragraphs[0].runs[0].format.bold = true;
        doc.paragraphs[0].runs.push(Run {
            text: " tail".into(),
            format: TextFormat::default(),
        });
        let mut state = EditorState::default();
        select_match(
            &doc,
            &mut state,
            Position::default(),
            Position {
                paragraph: 0,
                offset: 5,
            },
        );
        assert_eq!(state.typing.size, 24.0);
        assert!(state.insert(&mut doc, "replacement"));
        assert!(doc.paragraphs[0].runs[0].format.bold);
        assert_eq!(doc.paragraphs[0].runs[0].format.size, 24.0);
    }

    #[test]
    fn word_navigation_from_whitespace_does_not_skip_the_next_word() {
        let doc = Doc::from_text("one  two... three");
        let p = |offset| Position {
            paragraph: 0,
            offset,
        };
        assert_eq!(next(&doc, p(3), true), p(5));
        assert_eq!(next(&doc, p(0), true), p(5));
        assert_eq!(next(&doc, p(8), true), p(12));
        assert_eq!(previous(&doc, p(12), true), p(8));
        assert_eq!(previous(&doc, p(8), true), p(5));
    }
    #[test]
    fn replace_then_find_continues_at_the_next_surviving_match() {
        let mut doc = Doc::from_text("a a a");
        let mut state = EditorState {
            search: "a".into(),
            replacement: "long".into(),
            match_index: 1,
            cursor: Position {
                paragraph: 0,
                offset: 1,
            },
            ..EditorState::default()
        };
        assert!(replace_current_match(&mut doc, &mut state));
        let matches = doc.find(&state.search, false);
        assert_eq!(matches[state.match_index].0.offset, 5);
    }

    #[test]
    fn wrapped_boundary_navigation_uses_visible_row_and_remembers_x() {
        let mut doc = Doc::from_text(&"word ".repeat(50));
        doc.page.width = 240.0;
        doc.page.margin_left = 24.0;
        doc.page.margin_right = 24.0;
        let layout = doc.layout();
        assert!(layout.rows.len() > 2);
        let row = &layout.rows[1];
        let pos = Position {
            paragraph: 0,
            offset: row.start,
        };
        assert_eq!(
            vertical(&layout, pos, -1, row.x).offset,
            layout.rows[0].start
        );
        assert_eq!(
            vertical(&layout, pos, 1, row.x).offset,
            layout.rows[2].start
        );
    }

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
    fn empty_paste_preserves_selected_unicode_text() {
        let ctx = egui::Context::default();
        let mut doc = Doc::from_text("Привет 😀");
        let before = doc.clone();
        let mut state = EditorState {
            anchor: Position::default(),
            cursor: Position {
                paragraph: 0,
                offset: 8,
            },
            ..EditorState::default()
        };
        let range = state.range();
        let layout = doc.layout();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let canvas = ui.id().with("empty-paste");
            ui.memory_mut(|memory| memory.request_focus(canvas));
            handle_event(
                ui,
                &mut doc,
                &layout,
                &mut state,
                canvas,
                Event::Paste(String::new()),
            );
        });
        output.textures_delta.clear();
        assert_eq!(doc, before);
        assert_eq!(state.range(), range);
    }
    #[test]
    fn search_replace_uses_match_format_instead_of_stale_typing() {
        let mut doc = Doc::from_text("");
        doc.paragraphs[0].runs = vec![
            Run {
                text: "plain ".into(),
                format: TextFormat::default(),
            },
            Run {
                text: "needle".into(),
                format: TextFormat {
                    italic: true,
                    ..TextFormat::default()
                },
            },
        ];
        let mut state = EditorState {
            search: "needle".into(),
            replacement: "замена".into(),
            anchor: Position {
                paragraph: 0,
                offset: 6,
            },
            cursor: Position {
                paragraph: 0,
                offset: 12,
            },
            ..EditorState::default()
        };
        assert!(replace_current_match(&mut doc, &mut state));
        assert_eq!(doc.plain_text(), "plain замена");
        assert!(
            doc.paragraphs[0]
                .runs
                .last()
                .is_some_and(|run| run.format.italic)
        );
        assert!(state.typing.italic);
        assert!(state.insert(&mut doc, "!"));
        assert!(
            doc.paragraphs[0]
                .runs
                .last()
                .is_some_and(|run| run.text == "замена!" && run.format.italic)
        );
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
        source.paragraphs[1].align = Align::Right;
        source.paragraphs[1].list = List::Number;
        source.paragraphs[1].indent = 24.0;
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
        assert_eq!(target.paragraphs[1].align, Align::Right);
        assert_eq!(target.paragraphs[1].list, List::Number);
        assert_eq!(target.paragraphs[1].indent, 24.0);
        assert_eq!(target.paragraphs[0].align, Align::Left);
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
        assert!(
            to.typing.italic,
            "typing after rich paste must follow its final run"
        );
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
        let table = doc.tables.first_mut().ok_or("table")?;
        let id = table.id;
        let keep = table.rows.first().and_then(|r| r.first()).ok_or("cell")?.id;
        table.rows.get_mut(1).ok_or("row")?.pop();
        let mut state = EditorState {
            table_cell: Some(TableCursor {
                table: id,
                row: 0,
                column: 1,
            }),
            ..EditorState::default()
        };
        table_command(&mut doc, &mut state, TableCommand::ColumnRight);
        let table = doc.tables.first().ok_or("table")?;
        assert!(table.rows.iter().all(|r| r.len() == 3));
        assert_eq!(table.column_widths.len(), 3);
        assert_eq!(state.table_cell.map(|c| c.column), Some(2));
        doc.validate()?;
        table_command(&mut doc, &mut state, TableCommand::DeleteColumn);
        table_command(&mut doc, &mut state, TableCommand::RowBelow);
        let table = doc.tables.first().ok_or("table")?;
        assert!(table.rows.iter().all(|r| r.len() == 2));
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.column_widths.len(), 2);
        assert_eq!(
            table.rows.first().and_then(|r| r.first()).map(|c| c.id),
            Some(keep)
        );
        doc.validate()?;
        for _ in 0..3 {
            step_cell(&mut doc, &mut state, true);
        }
        assert_eq!(doc.tables.first().map(|t| t.rows.len()), Some(4));
        table_command(&mut doc, &mut state, TableCommand::DeleteTable);
        assert!(doc.tables.is_empty());
        assert!(state.table_cell.is_none());
        doc.validate()?;
        Ok(())
    }

    #[test]
    fn enter_finishes_empty_lists_and_starts_body_after_headings() {
        let mut doc = Doc::from_text("");
        doc.paragraphs[0].list = List::Bullet;
        doc.paragraphs[0].list_level = 3;
        let mut state = EditorState::default();
        insert_break(&mut doc, &mut state, false, false);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].list, List::None);
        assert_eq!(doc.paragraphs[0].list_level, 0);

        doc = Doc::from_text("Heading");
        doc.paragraphs[0].set_style("Heading1");
        state.move_to(
            &doc,
            Position {
                paragraph: 0,
                offset: 7,
            },
            false,
        );
        insert_break(&mut doc, &mut state, false, false);
        assert_eq!(doc.paragraphs[0].style, "Heading1");
        assert_eq!(doc.paragraphs[1].style, "Normal");
        assert!(!doc.paragraphs[1].keep_next);
        assert!(!state.typing.bold);
        assert_eq!(state.typing.size, 11.0);
        assert!(state.insert(&mut doc, "Body"));
        assert_eq!(doc.plain_text(), "Heading\nBody");
    }
    #[test]
    fn soft_breaks_keep_heading_style_and_page_breaks_keep_lists() {
        let mut doc = Doc::from_text("Heading");
        doc.paragraphs[0].set_style("Heading1");
        let mut state = EditorState::default();
        state.move_to(
            &doc,
            Position {
                paragraph: 0,
                offset: 7,
            },
            false,
        );
        insert_break(&mut doc, &mut state, true, false);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].text(), "Heading\u{000b}");
        assert_eq!(doc.paragraphs[0].style, "Heading1");

        doc = Doc::from_text("");
        doc.paragraphs[0].list = List::Number;
        state.move_to(&doc, Position::default(), false);
        insert_break(&mut doc, &mut state, false, true);
        assert_eq!(doc.paragraphs.len(), 2);
        assert_eq!(doc.paragraphs[1].list, List::Number);
        assert!(doc.paragraphs[1].page_break_before);
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
        let mut cell = Cell::new(6, vec![p], [255; 3]);
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

#[cfg(test)]
mod next_find_defects {
    use super::*;
    #[test]
    fn find_navigation_uses_the_current_caret_and_wraps_in_both_directions() {
        let doc = Doc::from_text("cat one cat two cat");
        let mut state = EditorState {
            cursor: Position {
                paragraph: 0,
                offset: 9,
            },
            anchor: Position {
                paragraph: 0,
                offset: 9,
            },
            ..Default::default()
        };
        let matches = doc.find("cat", true);
        navigate_find_matches(&doc, &mut state, &matches, false);
        assert_eq!(state.range(), matches[2]);
        navigate_find_matches(&doc, &mut state, &matches, false);
        assert_eq!(state.range(), matches[0]);
        navigate_find_matches(&doc, &mut state, &matches, true);
        assert_eq!(state.range(), matches[2]);
    }
    #[test]
    fn enter_in_the_query_routes_to_find_and_shift_enter_goes_back() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        let canvas = egui::Id::new("find-enter-canvas");
        let mut doc = Doc::from_text("a a");
        let mut state = EditorState {
            search: "a".into(),
            show_find: true,
            ..Default::default()
        };
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            find_bar(ui, &mut doc, &mut state, canvas)
        });
        ctx.memory_mut(|memory| memory.request_focus(find_field_id(canvas, false)));
        for (shift, offset) in [(false, 1), (true, 3)] {
            let modifiers = egui::Modifiers {
                shift,
                ..Default::default()
            };
            let _ = ctx.run_ui(
                egui::RawInput {
                    events: vec![
                        egui::Event::ModifiersChanged(modifiers),
                        egui::Event::Key {
                            key: Key::Enter,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers,
                        },
                    ],
                    ..Default::default()
                },
                |ui| find_bar(ui, &mut doc, &mut state, canvas),
            );
            assert_eq!(state.cursor.offset, offset);
            let _ = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: Key::Enter,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers,
                    }],
                    ..Default::default()
                },
                |ui| find_bar(ui, &mut doc, &mut state, canvas),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod next_body_navigation_tests {
    use super::*;
    #[test]
    fn finding_body_text_leaves_the_active_table_editor() -> Result<(), String> {
        let mut doc = Doc::from_text("body target");
        let table = doc.add_table(doc.paragraphs[0].id, 1, 1)?;
        let mut state = EditorState {
            table_cell: Some(TableCursor {
                table,
                row: 0,
                column: 0,
            }),
            table_page: Some(0),
            focus_cell: true,
            ..Default::default()
        };
        select_match(
            &doc,
            &mut state,
            Position {
                paragraph: 0,
                offset: 5,
            },
            Position {
                paragraph: 0,
                offset: 11,
            },
        );
        assert!(state.table_cell.is_none());
        assert!(state.table_page.is_none());
        assert!(!state.focus_cell);
        assert_eq!(state.cursor.offset, 11);
        Ok(())
    }
}

const DOCS_UI_FIND_LIMIT: usize = 100000;
fn mouse_selection(doc: &Doc, state: &mut EditorState, start: Position, end: Position) {
    state.move_to(doc, end, false);
    state.anchor = doc.clamp_position(start);
}
fn surrounding_position(
    doc: &Doc,
    start: Position,
    mut distance: usize,
    forward: bool,
) -> Position {
    let mut position = doc.clamp_position(start);
    loop {
        let length = doc
            .paragraphs
            .get(position.paragraph)
            .map(Paragraph::len)
            .unwrap_or_default();
        let available = if forward {
            length.saturating_sub(position.offset)
        } else {
            position.offset
        };
        if distance <= available {
            position.offset = if forward {
                position.offset + distance
            } else {
                position.offset - distance
            };
            return position;
        }
        distance -= available;
        if forward {
            if position.paragraph + 1 >= doc.paragraphs.len() {
                position.offset = length;
                return position;
            }
            position.paragraph += 1;
            position.offset = 0;
        } else {
            if position.paragraph == 0 {
                position.offset = 0;
                return position;
            }
            position.paragraph -= 1;
            position.offset = doc.paragraphs[position.paragraph].len();
        }
        distance = distance.saturating_sub(1);
    }
}

fn cell_content_rect(rect: Rect, padding: [f32; 4], scale: f32) -> Rect {
    let left = (padding[3] * scale)
        .max(0.0)
        .min((rect.width() - 1.0).max(0.0));
    let right = (padding[1] * scale)
        .max(0.0)
        .min((rect.width() - left - 1.0).max(0.0));
    let top = (padding[0] * scale)
        .max(0.0)
        .min((rect.height() - 1.0).max(0.0));
    let bottom = (padding[2] * scale)
        .max(0.0)
        .min((rect.height() - top - 1.0).max(0.0));
    Rect::from_min_max(
        rect.min + egui::vec2(left, top),
        rect.max - egui::vec2(right, bottom),
    )
}

fn cell_tab_direction(input: &mut egui::InputState) -> Option<bool> {
    if input.consume_key(egui::Modifiers::SHIFT, Key::Tab) {
        Some(false)
    } else if input.consume_key(egui::Modifiers::NONE, Key::Tab) {
        Some(true)
    } else {
        None
    }
}
const DOCS_STYLE_CHOICES: &[&str] = &[
    "Normal", "Title", "Heading1", "Heading2", "Heading3", "Heading4", "Heading5", "Heading6",
    "Quote", "Code",
];

fn find_panel_matches(
    doc: &Doc,
    query: &str,
    case_sensitive: bool,
) -> (Vec<(Position, Position)>, bool) {
    let mut matches = doc.find_bounded(query, case_sensitive, DOCS_UI_FIND_LIMIT + 1);
    let truncated = matches.len() > DOCS_UI_FIND_LIMIT;
    matches.truncate(DOCS_UI_FIND_LIMIT);
    (matches, truncated)
}

#[cfg(test)]
mod two_hundred_docs_editor_regressions {
    use super::*;
    #[test]
    fn shift_tab_from_a_table_cell_chooses_the_previous_visible_cell() -> Result<(), String> {
        let mut doc = Doc::from_text("anchor");
        let table = doc.add_table(doc.paragraphs[0].id, 1, 2)?;
        let mut state = EditorState {
            table_cell: Some(TableCursor {
                table,
                row: 0,
                column: 1,
            }),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let modifiers = egui::Modifiers::SHIFT;
        let _ = ctx.run_ui(
            egui::RawInput {
                events: vec![
                    Event::ModifiersChanged(modifiers),
                    Event::Key {
                        key: Key::Tab,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                let direction = ui.input_mut(cell_tab_direction);
                assert_eq!(direction, Some(false));
                if let Some(forward) = direction {
                    step_cell(&mut doc, &mut state, forward);
                }
            },
        );
        assert_eq!(
            state.table_cell,
            Some(TableCursor {
                table,
                row: 0,
                column: 0
            })
        );
        assert_eq!(doc.tables[0].rows.len(), 1);
        Ok(())
    }
    #[test]
    fn ime_surrounding_deletion_jumps_offsets_without_scanning_for_each_character() {
        let doc = Doc::from_text("αβ\n猫犬");
        assert_eq!(
            surrounding_position(
                &doc,
                Position {
                    paragraph: 1,
                    offset: 1
                },
                3,
                false
            ),
            Position {
                paragraph: 0,
                offset: 1
            }
        );
        assert_eq!(
            surrounding_position(
                &doc,
                Position {
                    paragraph: 0,
                    offset: 1
                },
                3,
                true
            ),
            Position {
                paragraph: 1,
                offset: 1
            }
        );
        assert_eq!(
            surrounding_position(
                &doc,
                Position {
                    paragraph: 0,
                    offset: 1
                },
                usize::MAX,
                true
            ),
            Position {
                paragraph: 1,
                offset: 2
            }
        );
        assert_eq!(
            surrounding_position(
                &doc,
                Position {
                    paragraph: 1,
                    offset: 1
                },
                usize::MAX,
                false
            ),
            Position::default()
        );
        let long = Doc::from_text(&"界".repeat(100000));
        assert_eq!(
            surrounding_position(&long, Position::default(), 100000, true).offset,
            100000
        );
    }
    #[test]
    fn mouse_selection_updates_the_typing_format_before_replacing_text() {
        let mut doc = Doc::from_text("old\nnew");
        doc.paragraphs[1].runs[0].format.bold = true;
        let mut state = EditorState {
            preedit: "composition".into(),
            typing: TextFormat {
                bold: false,
                ..Default::default()
            },
            ..Default::default()
        };
        mouse_selection(
            &doc,
            &mut state,
            Position {
                paragraph: 1,
                offset: 0,
            },
            Position {
                paragraph: 1,
                offset: 3,
            },
        );
        assert!(state.typing.bold);
        assert!(state.preedit.is_empty());
        assert!(state.insert(&mut doc, "replacement"));
        assert_eq!(doc.paragraphs[1].text(), "replacement");
        assert!(doc.paragraphs[1].runs.iter().all(|run| run.format.bold));
    }
    #[test]
    fn native_code_style_and_all_supported_headings_are_available_to_the_editor() {
        let mut doc = Doc::from_text("code");
        let mut state = EditorState::default();
        for style in ["Heading4", "Heading5", "Heading6", "Code"] {
            assert!(DOCS_STYLE_CHOICES.contains(&style));
        }
        apply_style(&mut doc, &mut state, "Code");
        assert_eq!(doc.paragraphs[0].style, "Code");
        assert_eq!(doc.paragraphs[0].runs[0].format.font, "Consolas");
        assert_eq!(state.typing.font, "Consolas");
        apply_style(&mut doc, &mut state, "Normal");
        assert_eq!(state.typing.font, "Calibri");
    }
    #[test]
    fn table_cell_editor_insets_follow_all_four_model_margins_at_zoom() {
        let rect = Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(210.0, 120.0));
        let inner = cell_content_rect(rect, [2.0, 4.0, 6.0, 8.0], 1.5);
        assert_eq!(inner.min, egui::pos2(22.0, 23.0));
        assert_eq!(inner.max, egui::pos2(204.0, 111.0));
        let tiny = cell_content_rect(
            Rect::from_min_size(Pos2::ZERO, egui::vec2(2.0, 2.0)),
            [100.0; 4],
            2.0,
        );
        assert!(tiny.width() >= 1.0 && tiny.height() >= 1.0);
    }
    #[test]
    fn find_panel_caps_dense_matches_and_reports_that_the_list_is_truncated() {
        let doc = Doc::from_text(&format!("{}\n{}", "a".repeat(60000), "a".repeat(60000)));
        let (matches, truncated) = find_panel_matches(&doc, "a", true);
        assert_eq!(matches.len(), DOCS_UI_FIND_LIMIT);
        assert!(truncated);
        assert_eq!(
            matches.last().map(|(_, end)| *end),
            Some(Position {
                paragraph: 1,
                offset: 40000
            })
        );
        let (matches, truncated) = find_panel_matches(&Doc::from_text("a a"), "a", true);
        assert_eq!(matches.len(), 2);
        assert!(!truncated);
    }
    #[test]
    fn tab_from_the_last_merged_cell_appends_after_the_whole_table() -> Result<(), String> {
        let mut doc = Doc::from_text("anchor");
        let table = doc.add_table(doc.paragraphs[0].id, 3, 1)?;
        doc.tables[0].rows[1][0].merged = true;
        doc.tables[0].rows[2][0].merged = true;
        let mut state = EditorState {
            table_cell: Some(TableCursor {
                table,
                row: 0,
                column: 0,
            }),
            ..Default::default()
        };
        step_cell(&mut doc, &mut state, true);
        assert_eq!(doc.tables[0].rows.len(), 4);
        assert_eq!(
            state.table_cell,
            Some(TableCursor {
                table,
                row: 3,
                column: 0
            })
        );
        assert!(doc.tables[0].rows[1][0].merged && doc.tables[0].rows[2][0].merged);
        assert!(!doc.tables[0].rows[3][0].merged);
        doc.validate()?;
        Ok(())
    }
    #[test]
    fn rich_text_paste_before_a_surviving_suffix_preserves_target_paragraph_properties() {
        let ctx = egui::Context::default();
        let mut source = Doc::from_text("copied");
        source.paragraphs[0].set_style("Heading1");
        ctx.data_mut(|data| {
            data.insert_temp(
                clipboard_id(),
                Clipboard {
                    text: "copied".into(),
                    paragraphs: source.paragraphs,
                    complete: vec![true],
                },
            )
        });
        let mut target = Doc::from_text("suffix");
        target.paragraphs[0].align = Align::Right;
        target.paragraphs[0].right_indent = 18.0;
        let mut state = EditorState::default();
        let layout = target.layout();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let canvas = ui.id().with("suffix-paste");
            ui.memory_mut(|memory| memory.request_focus(canvas));
            handle_event(
                ui,
                &mut target,
                &layout,
                &mut state,
                canvas,
                Event::Paste("copied".into()),
            );
        });
        assert_eq!(target.plain_text(), "copiedsuffix");
        assert_eq!(target.paragraphs[0].style, "Normal");
        assert_eq!(target.paragraphs[0].align, Align::Right);
        assert_eq!(target.paragraphs[0].right_indent, 18.0);
        assert!(target.paragraphs[0].runs[0].format.bold);
        assert!(
            !target.paragraphs[0]
                .runs
                .last()
                .is_some_and(|run| run.format.bold)
        );
    }
}
