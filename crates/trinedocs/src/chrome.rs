use egui::{Color32, FontId, RichText, Stroke, TextStyle, Ui};

pub fn accent(ui: &Ui) -> Color32 {
    ui.visuals().selection.stroke.color
}
pub fn muted(ui: &Ui) -> Color32 {
    ui.visuals().weak_text_color()
}
pub fn workspace(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(24, 29, 37)
    } else {
        Color32::from_rgb(237, 240, 244)
    }
}
pub fn style(ctx: &egui::Context, kind: &str) {
    let accent = match kind {
        "sheets" => Color32::from_rgb(30, 132, 88),
        "slides" => Color32::from_rgb(207, 100, 43),
        _ => Color32::from_rgb(47, 102, 208),
    };
    ctx.all_styles_mut(|style| {
        let dark = style.visuals.dark_mode;
        let text = if dark {
            Color32::from_rgb(225, 232, 243)
        } else {
            Color32::from_rgb(30, 43, 62)
        };
        let border = if dark {
            Color32::from_rgb(60, 74, 93)
        } else {
            Color32::from_rgb(225, 230, 237)
        };
        let panel = if dark {
            Color32::from_rgb(31, 42, 56)
        } else {
            Color32::from_rgb(250, 251, 253)
        };
        let surface = if dark {
            Color32::from_rgb(37, 49, 65)
        } else {
            Color32::WHITE
        };
        let soft = if dark {
            Color32::from_rgb(49, 65, 86)
        } else {
            Color32::from_rgb(235, 241, 250)
        };
        let v = &mut style.visuals;
        v.override_text_color = Some(text);
        v.weak_text_color = Some(if dark {
            Color32::from_rgb(161, 178, 200)
        } else {
            Color32::from_rgb(103, 118, 140)
        });
        v.window_fill = surface;
        v.panel_fill = panel;
        v.extreme_bg_color = surface;
        v.text_edit_bg_color = Some(surface);
        v.faint_bg_color = panel;
        v.window_stroke = Stroke::new(1.0, border);
        v.window_corner_radius = 10.into();
        v.menu_corner_radius = 8.into();
        v.selection.bg_fill = soft;
        v.selection.stroke = Stroke::new(1.0, accent);
        v.hyperlink_color = accent;
        v.text_cursor.stroke = Stroke::new(1.5, accent);
        for widget in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            widget.corner_radius = 5.into();
            widget.bg_stroke = Stroke::new(1.0, border);
            widget.fg_stroke = Stroke::new(1.0, text);
            widget.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_fill = panel;
        v.widgets.noninteractive.weak_bg_fill = panel;
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.inactive.bg_fill = surface;
        v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        v.widgets.hovered.bg_fill = soft;
        v.widgets.hovered.weak_bg_fill = soft;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.55));
        v.widgets.active.bg_fill = soft;
        v.widgets.active.weak_bg_fill = soft;
        v.widgets.active.bg_stroke = Stroke::new(1.0, accent);
        v.widgets.open.bg_fill = soft;
        v.widgets.open.weak_bg_fill = soft;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.interact_size = egui::vec2(30.0, 26.0);
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
            .insert(TextStyle::Heading, FontId::proportional(23.0));
    });
}
pub fn tabs(ui: &mut Ui, active: &mut usize, labels: &[&str]) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (index, label) in labels.iter().enumerate() {
            let selected = *active == index;
            let text = RichText::new(*label).size(13.0);
            let text = if selected {
                text.strong().color(accent(ui))
            } else {
                text
            };
            let response = ui.add(
                egui::Button::new(text)
                    .selected(selected)
                    .frame_when_inactive(false)
                    .min_size(egui::vec2(70.0, 29.0)),
            );
            if response.clicked() {
                *active = index;
            }
            if selected {
                ui.painter().line_segment(
                    [
                        response.rect.left_bottom() + egui::vec2(9.0, 0.0),
                        response.rect.right_bottom() - egui::vec2(9.0, 0.0),
                    ],
                    Stroke::new(2.0, accent(ui)),
                );
            }
        }
    });
    ui.add_space(3.0);
}
pub fn ribbon<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    let fill = ui.visuals().window_fill;
    egui::Frame::new()
        .fill(fill)
        .corner_radius(6)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::horizontal()
                .id_salt("ribbon")
                .max_height(52.0)
                .auto_shrink([false, true])
                .show(ui, |ui| ui.horizontal_top(content).inner)
                .inner
        })
        .inner
}
pub fn group<R>(ui: &mut Ui, title: &str, width: f32, content: impl FnOnce(&mut Ui) -> R) -> R {
    let value = ui
        .vertical(|ui| {
            ui.set_width(width);
            let value = ui
                .allocate_ui_with_layout(
                    egui::vec2(width, 28.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    content,
                )
                .inner;
            ui.add_space(2.0);
            ui.label(RichText::new(title).size(10.0).color(muted(ui)));
            value
        })
        .inner;
    ui.separator();
    value
}
pub fn badge(ui: &mut Ui, label: &str, color: Color32, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6, color);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(size * 0.62),
        Color32::WHITE,
    );
    response
}

#[derive(Clone, Copy)]
pub enum Icon {
    Open,
    Undo,
    Redo,
    Search,
}
pub fn icon_button(ui: &mut Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let enabled = enabled && ui.is_enabled();
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(28.0, 28.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let center = response.rect.center();
    let point = |x: f32, y: f32| center + egui::vec2(x, y);
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        muted(ui).gamma_multiply(0.45)
    };
    let stroke = Stroke::new(1.5, color);
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
