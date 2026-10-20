use egui::{Color32, FontId, RichText, Stroke, TextStyle, Ui};

pub fn accent(ui: &Ui) -> Color32 {
    ui.visuals().selection.stroke.color
}
pub fn muted(ui: &Ui) -> Color32 {
    ui.visuals().weak_text_color()
}
/// Neutral Office-like palette: black / grey / white in dark mode, white in light mode.
struct Palette {
    text: Color32,
    weak: Color32,
    border: Color32,
    /// Title bar and ribbon.
    chrome: Color32,
    /// Behind the page / grid.
    workspace: Color32,
    /// Page, inputs, popups.
    surface: Color32,
    hover: Color32,
    pressed: Color32,
}
fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            text: Color32::from_rgb(235, 237, 241),
            weak: Color32::from_rgb(169, 175, 185),
            border: Color32::from_rgb(65, 70, 79),
            chrome: Color32::from_rgb(32, 35, 41),
            workspace: Color32::from_rgb(24, 27, 32),
            surface: Color32::from_rgb(37, 41, 48),
            hover: Color32::from_rgb(48, 53, 62),
            pressed: Color32::from_rgb(60, 67, 78),
        }
    } else {
        Palette {
            text: Color32::from_rgb(34, 39, 47),
            weak: Color32::from_rgb(98, 106, 117),
            border: Color32::from_rgb(224, 228, 234),
            chrome: Color32::from_rgb(250, 251, 252),
            workspace: Color32::from_rgb(241, 243, 246),
            surface: Color32::WHITE,
            hover: Color32::from_rgb(237, 240, 244),
            pressed: Color32::from_rgb(224, 230, 238),
        }
    }
}
pub fn workspace(ui: &Ui) -> Color32 {
    palette(ui.visuals().dark_mode).workspace
}
pub fn hover_fill(ui: &Ui) -> Color32 {
    palette(ui.visuals().dark_mode).hover
}
pub fn border(ui: &Ui) -> Color32 {
    palette(ui.visuals().dark_mode).border
}
pub fn surface(ui: &Ui) -> Color32 {
    palette(ui.visuals().dark_mode).surface
}
pub fn accent_for(kind: &str, dark: bool) -> Color32 {
    match (kind, dark) {
        ("sheets", false) => Color32::from_rgb(22, 112, 68),
        ("sheets", true) => Color32::from_rgb(108, 202, 151),
        ("slides", false) => Color32::from_rgb(176, 71, 35),
        ("slides", true) => Color32::from_rgb(245, 161, 127),
        (_, false) => Color32::from_rgb(39, 94, 175),
        (_, true) => Color32::from_rgb(139, 180, 240),
    }
}
pub fn separator(ui: &mut Ui) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 18.0), egui::Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0, border(ui)),
    );
    ui.add_space(4.0);
}
pub fn sidebar_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(surface(ui))
        .inner_margin(egui::Margin::symmetric(12, 12))
}

/// Applies the shell style; cheap to call every frame (re-applied only on change).
pub fn style(ctx: &egui::Context, kind: &str, reduced_motion: bool) {
    let key = egui::Id::new("trinedocs-style");
    let wanted = (kind.to_string(), ctx.theme(), reduced_motion);
    if ctx
        .data(|d| d.get_temp::<(String, egui::Theme, bool)>(key))
        .as_ref()
        == Some(&wanted)
    {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(key, wanted));
    ctx.all_styles_mut(|style| {
        let dark = style.visuals.dark_mode;
        let accent = accent_for(kind, dark);
        let p = palette(dark);
        let v = &mut style.visuals;
        v.override_text_color = Some(p.text);
        v.weak_text_color = Some(p.weak);
        v.window_fill = p.surface;
        v.panel_fill = p.chrome;
        v.extreme_bg_color = p.surface;
        v.text_edit_bg_color = Some(p.workspace);
        v.faint_bg_color = p.workspace;
        v.code_bg_color = p.hover;
        v.window_stroke = Stroke::new(1.0, p.border);
        v.window_corner_radius = 10.into();
        v.menu_corner_radius = 10.into();
        v.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 80 } else { 20 }),
        };
        v.popup_shadow = egui::Shadow {
            offset: [0, 3],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 70 } else { 24 }),
        };
        v.selection.bg_fill = accent.gamma_multiply(if dark { 0.45 } else { 0.22 });
        v.selection.stroke = Stroke::new(1.0, accent);
        v.hyperlink_color = accent;
        v.text_cursor.stroke = Stroke::new(1.5, p.text);
        v.striped = false;
        for widget in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            widget.corner_radius = 6.into();
            widget.bg_stroke = Stroke::NONE;
            widget.fg_stroke = Stroke::new(1.0, p.text);
            widget.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_fill = p.chrome;
        v.widgets.noninteractive.weak_bg_fill = p.chrome;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
        v.widgets.inactive.bg_fill = p.hover;
        v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.hovered.bg_fill = p.hover;
        v.widgets.hovered.weak_bg_fill = p.hover;
        v.widgets.hovered.bg_stroke = Stroke::NONE;
        v.widgets.active.bg_fill = p.pressed;
        v.widgets.active.weak_bg_fill = p.pressed;
        v.widgets.active.bg_stroke = Stroke::new(1.0, accent);
        v.widgets.open.bg_fill = p.hover;
        v.widgets.open.weak_bg_fill = p.hover;
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 5.0);
        style.spacing.interact_size = egui::vec2(28.0, 28.0);
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.combo_height = 360.0;
        style.spacing.tooltip_width = 360.0;
        style.animation_time = if reduced_motion { 0.0 } else { 0.12 };
        style.interaction.tooltip_delay = 0.45;
        style.interaction.show_tooltips_only_when_still = true;
        v.slider_trailing_fill = true;
        v.text_cursor.blink = true;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(13.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(11.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(24.0));
    });
}
/// Ribbon tabs: plain labels, the active one underlined with the app accent.
pub fn tabs(ui: &mut Ui, active: &mut usize, labels: &[&str]) {
    if labels.is_empty() {
        return;
    }
    *active = (*active).min(labels.len() - 1);
    egui::ScrollArea::horizontal()
        .id_salt("ribbon-tabs")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (index, label) in labels.iter().enumerate() {
                    let selected = *active == index;
                    let text = RichText::new(*label).size(13.0);
                    let text = if selected {
                        text.strong().color(accent(ui))
                    } else {
                        text
                    };
                    let response = ui
                        .add(
                            egui::Button::new(text)
                                .selected(selected)
                                .frame_when_inactive(false)
                                .min_size(egui::vec2(52.0, 32.0)),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.clicked() {
                        *active = index;
                    }
                    if response.has_focus() {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if selected {
                        ui.painter().line_segment(
                            [
                                response.rect.left_bottom() + egui::vec2(12.0, -1.0),
                                response.rect.right_bottom() - egui::vec2(12.0, 1.0),
                            ],
                            Stroke::new(2.0, accent(ui)),
                        );
                    }
                }
            });
        });
}
/// Height of the two-row content area of a ribbon group.
pub const GROUP_HEIGHT: f32 = 52.0;
pub fn ribbon<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    let fill = ui.visuals().panel_fill;
    let line = border(ui);
    let response = egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::horizontal()
                .id_salt("ribbon")
                .max_height(GROUP_HEIGHT + 22.0)
                .auto_shrink([false, true])
                .show(ui, |ui| ui.horizontal_top(content).inner)
                .inner
        });
    let rect = response.response.rect;
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, line));
    response.inner
}
/// A ribbon group: up to two rows of commands with a caption underneath and a
/// thin divider on the right, as in Word, Excel and PowerPoint.
pub fn group<R>(ui: &mut Ui, title: &str, width: f32, content: impl FnOnce(&mut Ui) -> R) -> R {
    let response = ui.vertical(|ui| {
        ui.set_width(width);
        let value = ui
            .allocate_ui_with_layout(
                egui::vec2(width, GROUP_HEIGHT),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_min_height(GROUP_HEIGHT);
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 4.0);
                    content(ui)
                },
            )
            .inner;
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(title).size(10.5).color(muted(ui)));
        });
        value
    });
    let rect = response.response.rect;
    let x = rect.right() + 5.0;
    ui.painter().vline(
        x,
        rect.top() + 4.0..=rect.bottom() - 4.0,
        Stroke::new(1.0, border(ui)),
    );
    ui.add_space(10.0);
    response.inner
}
/// Small frameless ribbon command (icon text or glyph), highlighted when on.
pub fn tool(
    ui: &mut Ui,
    label: impl Into<egui::WidgetText>,
    tip: &str,
    on: bool,
) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .selected(on)
            .frame_when_inactive(false)
            .min_size(egui::vec2(24.0, 24.0)),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
    .on_hover_text(tip)
}
fn font_filter_id(ui: &Ui, id: &str) -> egui::Id {
    ui.make_persistent_id((id, "font-filter"))
}
fn color_swatch(ui: &mut Ui, color: [u8; 3], selected: bool) -> egui::Response {
    let fill = Color32::from_rgb(color[0], color[1], color[2]);
    let label = format!("Цвет #{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
    let response = ui.add_sized(
        [22.0, 22.0],
        egui::Button::new("").fill(fill).selected(selected),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    ui.painter()
        .rect_filled(response.rect.shrink(2.0), 2.0, fill);
    response.on_hover_text(label)
}
/// Font family picker with a search box; each family is previewed in itself.
/// Returns true when the family changed.
pub fn font_picker(ui: &mut Ui, id: &str, family: &mut String, width: f32) -> bool {
    let mut changed = false;
    let filter_id = font_filter_id(ui, id);
    egui::ComboBox::from_id_salt(id)
        .width(width)
        .height(420.0)
        .selected_text(family.as_str())
        .show_ui(ui, |ui| {
            let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
            let search = ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Поиск шрифта")
                    .desired_width(240.0),
            );
            if search.changed() {
                ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
            }
            if !search.has_focus() && !ui.memory(|m| m.focused().is_some()) {
                search.request_focus();
            }
            let needle = filter.to_lowercase();
            let names: Vec<String> = crate::fonts::families()
                .into_iter()
                .filter(|name| needle.is_empty() || name.to_lowercase().contains(&needle))
                .collect();
            if names.is_empty() {
                ui.weak("Шрифты не найдены");
            }
            let row = 24.0;
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, true])
                .show_rows(ui, row, names.len(), |ui, range| {
                    for name in names.get(range).unwrap_or_default() {
                        let font = crate::fonts::egui_family(ui.ctx(), name, false, false);
                        let label = RichText::new(name.as_str()).font(FontId::new(15.0, font));
                        let response = ui.add_sized(
                            [ui.available_width(), row],
                            egui::Button::new(label)
                                .selected(name == family)
                                .frame_when_inactive(false),
                        );
                        if response.clicked() && name != family {
                            *family = name.clone();
                            changed = true;
                            ui.close();
                        }
                    }
                });
        });
    changed
}
/// Office font-size box: standard sizes in a list plus free input.
pub fn size_picker(ui: &mut Ui, id: &str, size: &mut f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        changed |= ui
            .add_sized(
                [38.0, 24.0],
                egui::DragValue::new(size).range(1.0..=400.0).speed(0.25),
            )
            .on_hover_text("Размер шрифта")
            .changed();
        egui::ComboBox::from_id_salt(id)
            .width(14.0)
            .selected_text("")
            .show_ui(ui, |ui| {
                for value in crate::fonts::SIZES {
                    if ui
                        .selectable_label((*size - value).abs() < 0.01, format!("{value}"))
                        .clicked()
                    {
                        *size = value;
                        changed = true;
                    }
                }
            });
    });
    changed
}
/// Next / previous standard size, as "Увеличить/Уменьшить размер шрифта".
pub fn step_size(size: f32, grow: bool) -> f32 {
    if grow {
        crate::fonts::SIZES
            .iter()
            .copied()
            .find(|v| *v > size + 0.01)
            .unwrap_or((size + 12.0).min(400.0))
    } else {
        crate::fonts::SIZES
            .iter()
            .rev()
            .copied()
            .find(|v| *v < size - 0.01)
            .unwrap_or((size - 1.0).max(1.0))
    }
}
/// Colour command in the Office style: a glyph with a colour bar under it.
pub fn color_tool(ui: &mut Ui, glyph: &str, tip: &str, color: Color32) -> egui::Response {
    let response = tool(ui, RichText::new(glyph).size(13.0), tip, false);
    let r = response.rect;
    let bar = egui::Rect::from_min_max(
        egui::pos2(r.left() + 5.0, r.bottom() - 5.0),
        egui::pos2(r.right() - 5.0, r.bottom() - 2.0),
    );
    ui.painter().rect_filled(bar, 0.0, color);
    if color == Color32::TRANSPARENT {
        ui.painter().line_segment(
            [bar.left_bottom(), bar.right_top()],
            Stroke::new(1.0, muted(ui)),
        );
    }
    response
}
/// Theme colours, greys and standard colours, as in the Office colour menu.
pub const COLORS: [[u8; 3]; 30] = [
    [255, 255, 255],
    [0, 0, 0],
    [231, 230, 230],
    [68, 84, 106],
    [68, 114, 196],
    [237, 125, 49],
    [165, 165, 165],
    [255, 192, 0],
    [91, 155, 213],
    [112, 173, 71],
    [242, 242, 242],
    [217, 217, 217],
    [191, 191, 191],
    [166, 166, 166],
    [128, 128, 128],
    [89, 89, 89],
    [64, 64, 64],
    [38, 38, 38],
    [13, 13, 13],
    [214, 220, 228],
    [192, 0, 0],
    [255, 0, 0],
    [255, 192, 0],
    [255, 255, 0],
    [146, 208, 80],
    [0, 176, 80],
    [0, 176, 240],
    [0, 112, 192],
    [0, 32, 96],
    [112, 48, 160],
];
/// Colour command with a drop-down palette. Returns `Some(None)` when the
/// "no colour" entry (if `none_label` is given) was chosen.
pub fn color_menu(
    ui: &mut Ui,
    glyph: &str,
    tip: &str,
    current: Option<[u8; 3]>,
    none_label: Option<&str>,
) -> Option<Option<[u8; 3]>> {
    let shown = current.map_or(Color32::TRANSPARENT, |c| {
        Color32::from_rgb(c[0], c[1], c[2])
    });
    let response = color_tool(ui, glyph, tip, shown);
    let mut picked = None;
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(266.0);
        if let Some(label) = none_label
            && ui.button(label).clicked()
        {
            picked = Some(None);
            ui.close();
        }
        egui::Grid::new((glyph, tip))
            .spacing([3.0, 3.0])
            .show(ui, |ui| {
                for (index, color) in COLORS.iter().enumerate() {
                    let swatch = color_swatch(ui, *color, current == Some(*color));
                    let rect = swatch.rect;
                    let edge = if swatch.hovered() || swatch.has_focus() || current == Some(*color)
                    {
                        Stroke::new(2.0, accent(ui))
                    } else {
                        Stroke::new(1.0, border(ui))
                    };
                    ui.painter()
                        .rect_stroke(rect, 2.0, edge, egui::StrokeKind::Inside);
                    if swatch.clicked() {
                        picked = Some(Some(*color));
                        ui.close();
                    }
                    if (index + 1) % 10 == 0 {
                        ui.end_row();
                    }
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            let mut custom = current.unwrap_or([0, 0, 0]);
            if ui.color_edit_button_srgb(&mut custom).changed() {
                picked = Some(Some(custom));
            }
            ui.label("Другие цвета…");
        });
    });
    picked
}
pub fn badge(ui: &mut Ui, label: &str, color: Color32, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6, surface(ui));
    ui.painter().rect_stroke(
        rect,
        6,
        Stroke::new(1.0, border(ui)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(size * 0.52),
        color,
    );
    response
}

#[derive(Clone, Copy)]
pub enum Icon {
    Open,
    Undo,
    Redo,
    Search,
    Save,
    Close,
}
pub fn icon_button(ui: &mut Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let enabled = enabled && ui.is_enabled();
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .frame_when_inactive(false)
            .min_size(egui::vec2(28.0, 28.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    let center = response.rect.center();
    let point = |x: f32, y: f32| center + egui::vec2(x, y);
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        muted(ui).gamma_multiply(0.45)
    };
    let stroke = Stroke::new(1.75, color);
    let painter = ui.painter();
    match icon {
        Icon::Open => {
            painter.line(
                vec![
                    point(-8.0, 5.0),
                    point(-8.0, -6.0),
                    point(-2.0, -6.0),
                    point(0.0, -3.0),
                    point(8.0, -3.0),
                ],
                stroke,
            );
            painter.line(
                vec![
                    point(-8.0, 5.0),
                    point(-5.0, -1.0),
                    point(9.0, -1.0),
                    point(6.0, 5.0),
                    point(-8.0, 5.0),
                ],
                stroke,
            );
        }
        Icon::Undo | Icon::Redo => {
            let direction = if matches!(icon, Icon::Undo) {
                1.0
            } else {
                -1.0
            };
            let p = |x: f32, y: f32| point(x * direction, y);
            painter.line(
                vec![
                    p(-7.0, -3.0),
                    p(0.0, -3.0),
                    p(5.0, -1.0),
                    p(7.0, 3.0),
                    p(5.0, 7.0),
                ],
                stroke,
            );
            painter.line(vec![p(-3.0, -7.0), p(-7.0, -3.0), p(-3.0, 1.0)], stroke);
        }
        Icon::Save => {
            painter.rect_stroke(
                egui::Rect::from_center_size(center, egui::vec2(14.0, 16.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line(
                vec![
                    point(-3.0, -7.0),
                    point(-3.0, -2.0),
                    point(4.0, -2.0),
                    point(4.0, -7.0),
                ],
                stroke,
            );
            painter.line(
                vec![
                    point(-3.0, 7.0),
                    point(-3.0, 2.0),
                    point(3.0, 2.0),
                    point(3.0, 7.0),
                ],
                stroke,
            );
        }
        Icon::Close => {
            painter.line_segment([point(-4.0, -4.0), point(4.0, 4.0)], stroke);
            painter.line_segment([point(4.0, -4.0), point(-4.0, 4.0)], stroke);
        }
        Icon::Search => {
            painter.circle_stroke(point(-2.0, -2.0), 5.5, stroke);
            painter.line_segment([point(2.0, 2.0), point(7.0, 7.0)], stroke);
        }
    }
    response.on_hover_text(label)
}

pub fn alignment_button(
    ui: &mut Ui,
    alignment: usize,
    label: &str,
    selected: bool,
) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .selected(selected)
            .frame_when_inactive(false)
            .min_size(egui::vec2(26.0, 26.0)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let center = response.rect.center();
    let color = if selected {
        accent(ui)
    } else {
        ui.visuals().text_color()
    };
    let color = if ui.is_enabled() {
        color
    } else {
        color.gamma_multiply(0.45)
    };
    for row in 0..4 {
        let length = if alignment == 3 || row % 2 == 0 {
            14.0
        } else {
            9.0
        };
        let left = match alignment {
            1 => -length / 2.0,
            2 => 7.0 - length,
            _ => -7.0,
        };
        let y = -5.0 + row as f32 * 3.5;
        ui.painter().line_segment(
            [
                center + egui::vec2(left, y),
                center + egui::vec2(left + length, y),
            ],
            Stroke::new(1.4, color),
        );
    }
    response.on_hover_text(label)
}

#[cfg(test)]
mod tests {
    #[test]
    fn motion_setting_updates_style_without_restart() {
        let ctx = egui::Context::default();
        super::style(&ctx, "docs", false);
        assert_eq!(ctx.global_style().animation_time, 0.12);
        super::style(&ctx, "docs", true);
        assert_eq!(ctx.global_style().animation_time, 0.0);
        super::style(&ctx, "docs", false);
        assert_eq!(ctx.global_style().animation_time, 0.12);
    }
    #[test]
    fn ribbon_leaves_space_for_content_at_both_window_widths() -> Result<(), String> {
        for width in [780.0, 1250.0] {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx)?;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 820.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let top = ui.cursor().top();
                    super::ribbon(ui, |ui| {
                        for label in ["Шрифт", "Абзац", "Правка"] {
                            super::group(ui, label, 200.0, |ui| {
                                ui.button(label).clicked();
                            });
                        }
                    });
                    assert!(
                        ui.cursor().top() - top < 90.0,
                        "ribbon consumed canvas height: {}",
                        ui.cursor().top() - top
                    );
                },
            );
            output.textures_delta.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod ui_review_tests {
    use super::*;
    #[test]
    fn font_filters_are_scoped_to_their_parent_ui() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let a = ui.push_id("doc-a", |ui| font_filter_id(ui, "font")).inner;
            let b = ui.push_id("doc-b", |ui| font_filter_id(ui, "font")).inner;
            assert_ne!(a, b);
            ui.data_mut(|data| data.insert_temp(a, "Arial".to_string()));
            assert!(ui.data(|data| data.get_temp::<String>(b)).is_none());
        }).textures_delta.clear();
    }
    #[test]
    fn palette_swatches_are_focusable_named_controls() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        ctx.enable_accesskit();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let swatch = color_swatch(ui, [12, 34, 56], true);
            assert!(swatch.sense.is_focusable());
        });
        out.textures_delta.clear();
        let tree = format!("{:?}", out.platform_output.accesskit_update);
        assert!(tree.contains("Цвет #0C2238"));
        Ok(())
    }
    #[test]
    fn ribbon_tabs_keep_a_horizontal_scroll_extent() -> Result<(), String> {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx)?;
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(260.0, 240.0),
                )),
                ..Default::default()
            },
            |ui| {
                let right = ui.max_rect().right();
                let mut active = usize::MAX;
                tabs(
                    ui,
                    &mut active,
                    &["Главная", "Вставка", "Макет", "Рецензирование", "Вид"],
                );
                assert_eq!(active, 4);
                assert!(ui.min_rect().right() <= right + 1.0);
            },
        ).textures_delta.clear();
        Ok(())
    }
}
