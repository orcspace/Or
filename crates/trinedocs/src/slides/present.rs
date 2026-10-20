use super::{editor::paint_slide, model::Deck};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Default)]
pub struct Presentation {
    pub active: bool,
    pub index: usize,
    previous: Option<u64>,
    slide_id: Option<u64>,
    slide_started: f64,
    animation_bias: f32,
    animation_signature: Option<u64>,
    previous_time: f32,
    previous_click_times: Vec<f32>,
    started: f64,
    clicks: usize,
    click_times: Vec<f32>,
    ink: bool,
    pointer: bool,
    notes: bool,
    blank: u8,
    blank_since: Option<f64>,
    /// The black "end of slide show" screen after the last slide.
    ended: bool,
    /// Ink positions in slide coordinates (0..1), independent of the viewport.
    strokes: Vec<Vec<Pos2>>,
    control_ids: Vec<egui::Id>,
    inking: bool,
    ink_points: usize,
    ink_full: bool,
}
impl Presentation {
    pub fn start(&mut self, ctx: &egui::Context, deck: &Deck, index: usize) {
        if deck.slides.is_empty() {
            return;
        }
        self.index = index.min(deck.slides.len() - 1);
        if deck.slides.get(self.index).is_some_and(|s| s.hidden) {
            if let Some(index) = deck
                .slides
                .iter()
                .enumerate()
                .skip(self.index)
                .find(|(_, s)| !s.hidden)
                .map(|(i, _)| i)
            {
                self.index = index;
            } else if let Some(index) = deck.slides.iter().rposition(|slide| !slide.hidden) {
                self.index = index;
            } else {
                return;
            }
        }
        self.slide_id = deck.slides.get(self.index).map(|slide| slide.id);
        self.animation_signature = deck
            .slides
            .get(self.index)
            .map(super::editor::animation_signature);
        self.active = true;
        self.started = ctx.input(|i| i.time);
        self.slide_started = self.started;
        self.previous = None;
        self.animation_bias = 0.0;
        self.previous_time = 0.0;
        self.previous_click_times.clear();
        self.clicks = 0;
        self.click_times.clear();
        self.clear_ink();
        self.blank = 0;
        self.blank_since = None;
        self.ended = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        ctx.request_repaint();
    }
    fn clear_ink(&mut self) {
        self.strokes.clear();
        self.inking = false;
        self.ink_points = 0;
        self.ink_full = false;
    }
    fn move_to(&mut self, deck: &Deck, index: usize, time: f64) {
        self.previous = deck.slides.get(self.index).map(|slide| slide.id);
        self.previous_time = (time - self.slide_started).max(0.0) as f32 + self.animation_bias;
        self.previous_click_times.clone_from(&self.click_times);
        self.animation_bias = 0.0;
        self.index = index;
        self.slide_id = deck.slides.get(index).map(|slide| slide.id);
        self.animation_signature = deck
            .slides
            .get(index)
            .map(super::editor::animation_signature);
        self.slide_started = time;
        self.clicks = 0;
        self.click_times.clear();
        self.clear_ink();
        self.blank = 0;
        self.blank_since = None;
        self.ended = false;
    }
    fn set_blank(&mut self, blank: u8, time: f64) {
        if blank == 0 {
            if let Some(started) = self.blank_since.take() {
                self.slide_started += (time - started).max(0.0);
            }
        } else if self.blank == 0 {
            self.blank_since = Some(time);
        }
        self.blank = blank;
    }
    fn click_steps(deck: &Deck, index: usize) -> usize {
        deck.slides
            .get(index)
            .map(super::model::Slide::click_steps)
            .unwrap_or(0)
    }
    fn finish(&mut self) {
        self.ended = true;
        self.blank = 1;
    }
    fn next(&mut self, deck: &Deck, time: f64) {
        if self.ended {
            self.active = false;
            return;
        }
        if self.blank != 0 {
            // B / W blanking: the first advance only brings the slide back.
            self.set_blank(0, time);
            return;
        }
        let count = Self::click_steps(deck, self.index);
        if self.clicks < count {
            self.clicks += 1;
            self.click_times
                .push((time - self.slide_started) as f32 + self.animation_bias);
            return;
        }
        if let Some(index) = deck
            .slides
            .iter()
            .enumerate()
            .skip(self.index + 1)
            .find(|(_, s)| !s.hidden)
            .map(|(i, _)| i)
        {
            self.move_to(deck, index, time);
        } else {
            self.finish();
        }
    }
    fn previous(&mut self, deck: &Deck, time: f64) {
        if self.ended || self.blank != 0 {
            // Back from the end screen (or a blanked screen) shows the slide again.
            if self.ended {
                self.slide_started = time;
                self.reveal_completed(deck);
            }
            self.ended = false;
            self.set_blank(0, time);
            return;
        }
        if self.clicks > 0 {
            // Like PowerPoint, "back" first undoes the last on-click animation step.
            self.clicks -= 1;
            self.click_times.pop();
            return;
        }
        if let Some(index) = deck
            .slides
            .iter()
            .enumerate()
            .take(self.index)
            .rev()
            .find(|(_, s)| !s.hidden)
            .map(|(i, _)| i)
        {
            self.move_to(deck, index, time);
            // The previous slide comes back fully built, with its animations already shown.
            self.clicks = Self::click_steps(deck, index);
            self.reveal_completed(deck);
        }
    }
    fn reveal_completed(&mut self, deck: &Deck) {
        self.clicks = Self::click_steps(deck, self.index);
        self.click_times = vec![0.0; self.clicks];
        self.animation_bias = deck
            .slides
            .get(self.index)
            .map(|slide| super::editor::animation_end(slide, &self.click_times))
            .unwrap_or(0.0);
    }
    fn advance_timed(&mut self, deck: &Deck, time: f64) -> bool {
        if self.ended
            || self.blank != 0
            || !deck.slides.get(self.index).is_some_and(|slide| {
                slide.transition.advance_after.is_some_and(|seconds| {
                    let transition = if slide.transition.effect == "none" {
                        0.0
                    } else {
                        slide.transition.duration
                    };
                    let effects = (super::editor::animation_end(slide, &self.click_times)
                        - self.animation_bias)
                        .max(transition);
                    time - self.slide_started >= f64::from(effects + seconds)
                })
            })
        {
            return false;
        }
        if let Some(index) = deck
            .slides
            .iter()
            .enumerate()
            .skip(self.index + 1)
            .find(|(_, slide)| !slide.hidden)
            .map(|(index, _)| index)
        {
            self.move_to(deck, index, time);
        } else {
            self.finish();
        }
        true
    }
    fn sync_slide(&mut self, deck: &Deck, time: f64) -> bool {
        if let Some(id) = self.slide_id {
            let Some(index) = deck.slides.iter().position(|slide| slide.id == id) else {
                self.active = false;
                return false;
            };
            self.index = index;
        }
        let Some(slide) = deck.slides.get(self.index) else {
            self.active = false;
            return false;
        };
        if slide.hidden {
            let visible = deck
                .slides
                .iter()
                .enumerate()
                .skip(self.index + 1)
                .find(|(_, slide)| !slide.hidden)
                .map(|(index, _)| index)
                .or_else(|| {
                    deck.slides
                        .iter()
                        .enumerate()
                        .take(self.index)
                        .rev()
                        .find(|(_, slide)| !slide.hidden)
                        .map(|(index, _)| index)
                });
            let Some(index) = visible else {
                self.active = false;
                return false;
            };
            self.move_to(deck, index, time);
            self.previous = None;
        }
        let signature = deck
            .slides
            .get(self.index)
            .map(super::editor::animation_signature);
        if self.animation_signature.is_some() && self.animation_signature != signature {
            self.clicks = 0;
            self.click_times.clear();
            self.animation_bias = 0.0;
            self.slide_started = time;
            self.previous = None;
            if self.blank_since.is_some() {
                self.blank_since = Some(time);
            }
        }
        self.animation_signature = signature;
        true
    }
}
pub fn show(ui: &mut egui::Ui, deck: &Deck, state: &mut Presentation) {
    let ctx = ui.ctx().clone();
    let time = ctx.input(|i| i.time);
    if !state.sync_slide(deck, time) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        return;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.active = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        return;
    }
    let pressed = |key| {
        presentation_key_allowed(&ctx, state, key) && ctx.input(|input| input.key_pressed(key))
    };
    let forward = [
        egui::Key::ArrowRight,
        egui::Key::ArrowDown,
        egui::Key::PageDown,
        egui::Key::Space,
    ]
    .into_iter()
    .any(pressed);
    let backward = [egui::Key::ArrowLeft, egui::Key::ArrowUp, egui::Key::PageUp]
        .into_iter()
        .any(pressed);
    let home = pressed(egui::Key::Home);
    let end = pressed(egui::Key::End);
    let black = pressed(egui::Key::B);
    let white = pressed(egui::Key::W);
    let notes = pressed(egui::Key::P);
    let mut manual_navigation = forward || backward || home || end;
    if forward {
        state.next(deck, time);
    }
    if backward {
        state.previous(deck, time);
    }
    if home && let Some(index) = deck.slides.iter().position(|slide| !slide.hidden) {
        state.move_to(deck, index, time);
    }
    if end && let Some(index) = deck.slides.iter().rposition(|slide| !slide.hidden) {
        state.move_to(deck, index, time);
    }
    if black {
        state.set_blank(if state.blank == 1 { 0 } else { 1 }, time);
    }
    if white {
        state.set_blank(if state.blank == 2 { 0 } else { 2 }, time);
    }
    if notes {
        state.notes = !state.notes;
    }
    state.control_ids.clear();
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("slides_presentation_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(&ctx, |ui| {
            ui.set_min_size(screen.size());
            let (response, painter) =
                ui.allocate_painter(screen.size(), egui::Sense::click_and_drag());
            let outer = response.rect;
            painter.rect_filled(outer, 0.0, Color32::BLACK);
            let ratio = (outer.width() / deck.width).min(outer.height() / deck.height);
            let size = Vec2::new(deck.width * ratio, deck.height * ratio);
            let rect = Rect::from_center_size(outer.center(), size);
            if let Some(slide) = deck.slides.get(state.index) {
                let elapsed = (time - state.slide_started) as f32;
                let progress =
                    super::editor::motion_progress(elapsed, 0.0, slide.transition.duration);
                if state.blank != 0 {
                    painter.rect_filled(
                        outer,
                        0.0,
                        if state.blank == 2 {
                            Color32::WHITE
                        } else {
                            Color32::BLACK
                        },
                    );
                    if state.ended {
                        painter.text(
                            outer.center(),
                            egui::Align2::CENTER_CENTER,
                            "Конец показа слайдов. Щёлкните, чтобы выйти.",
                            egui::FontId::proportional(22.0),
                            Color32::from_gray(200),
                        );
                    }
                } else {
                    if progress < 1.0
                        && let Some(previous) = state.previous.and_then(|id| deck.slide(id))
                    {
                        let previous_rect =
                            outgoing_transition_rect(rect, &slide.transition.effect, progress);
                        paint_slide(
                            &ctx,
                            &painter.with_clip_rect(rect),
                            previous_rect,
                            deck,
                            previous,
                            Some((state.previous_time, &state.previous_click_times)),
                        );
                    }
                    let animation_time = elapsed + state.animation_bias;
                    match slide.transition.effect.as_str() {
                        "fade" if progress < 1.0 => {
                            let mut p = painter.clone();
                            p.multiply_opacity(progress);
                            paint_slide(
                                &ctx,
                                &p,
                                rect,
                                deck,
                                slide,
                                Some((animation_time, &state.click_times)),
                            );
                        }
                        "push" if progress < 1.0 => {
                            let shifted =
                                rect.translate(Vec2::new((1.0 - progress) * rect.width(), 0.0));
                            paint_slide(
                                &ctx,
                                &painter.with_clip_rect(rect),
                                shifted,
                                deck,
                                slide,
                                Some((animation_time, &state.click_times)),
                            );
                        }
                        "wipe" if progress < 1.0 => {
                            let clip = Rect::from_min_max(
                                rect.min,
                                Pos2::new(rect.left() + rect.width() * progress, rect.bottom()),
                            );
                            paint_slide(
                                &ctx,
                                &painter.with_clip_rect(clip),
                                rect,
                                deck,
                                slide,
                                Some((animation_time, &state.click_times)),
                            );
                        }
                        _ => paint_slide(
                            &ctx,
                            &painter,
                            rect,
                            deck,
                            slide,
                            Some((animation_time, &state.click_times)),
                        ),
                    }
                    for points in &state.strokes {
                        if let [point] = points.as_slice() {
                            painter.with_clip_rect(rect).circle_filled(
                                ink_to_screen(rect, *point),
                                1.25,
                                Color32::from_rgb(235, 50, 50),
                            );
                        } else if points.len() > 1 {
                            painter.with_clip_rect(rect).add(egui::Shape::line(
                                points
                                    .iter()
                                    .map(|point| ink_to_screen(rect, *point))
                                    .collect(),
                                Stroke::new(2.5, Color32::from_rgb(235, 50, 50)),
                            ));
                        }
                    }
                    if state.pointer
                        && let Some(pos) = ctx.pointer_hover_pos().filter(|pos| rect.contains(*pos))
                    {
                        painter.circle_filled(pos, 6.0, Color32::from_rgb(240, 50, 45));
                    }
                }
            }
            if response.clicked() && presentation_click_allowed(state) {
                state.next(deck, time);
                manual_navigation = true;
            }
            let point = ctx
                .pointer_interact_pos()
                .and_then(|pos| ink_from_screen(rect, pos));
            update_ink(
                state,
                point,
                response.clicked(),
                response.drag_started(),
                response.dragged(),
                response.drag_stopped(),
            );
            let controls = Rect::from_min_size(
                outer.min + Vec2::new(12.0, 10.0),
                Vec2::new((outer.width() - 24.0).clamp(1.0, 660.0), 36.0),
            );
            let mut controls_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(controls)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            egui::Frame::new()
                .fill(Color32::from_black_alpha(210))
                .inner_margin(6.0)
                .corner_radius(5.0)
                .show(&mut controls_ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if present_button(ui, state, "←")
                            .on_hover_text("Предыдущий слайд · ←")
                            .clicked()
                        {
                            state.previous(deck, time);
                            manual_navigation = true;
                        }
                        let visible_count =
                            deck.slides.iter().filter(|slide| !slide.hidden).count();
                        let visible_index = deck
                            .slides
                            .iter()
                            .take(state.index + 1)
                            .filter(|slide| !slide.hidden)
                            .count();
                        ui.label(format!("{visible_index} / {visible_count}"));
                        if present_button(ui, state, "→")
                            .on_hover_text("Следующий слайд · →")
                            .clicked()
                        {
                            state.next(deck, time);
                            manual_navigation = true;
                        }
                        let response = ui.checkbox(&mut state.pointer, "Указка");
                        state.control_ids.push(response.id);
                        let response = ui.checkbox(&mut state.ink, "Перо");
                        state.control_ids.push(response.id);
                        if present_button(ui, state, "Очистить").clicked() {
                            state.clear_ink();
                        }
                        let response = ui.checkbox(&mut state.notes, "Заметки (P)");
                        state.control_ids.push(response.id);
                        if state.ink_full {
                            ui.colored_label(
                                Color32::YELLOW,
                                "Перо: достигнут лимит. Очистите рисунок для продолжения.",
                            );
                        }
                        if present_button(ui, state, "Завершить Esc").clicked() {
                            state.active = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        }
                    });
                });
            advance_after_controls(state, deck, time, manual_navigation);
        });
    if state.notes {
        egui::Window::new("Докладчик · текущий экран")
            .order(egui::Order::Foreground)
            .default_pos(screen.min + Vec2::new(20.0, 80.0))
            .default_size(Vec2::new(400.0, 360.0))
            .show(&ctx, |ui| {
                ui.label(format!(
                    "Таймер {:02}:{:02}",
                    ((time - state.started) as u64) / 60,
                    ((time - state.started) as u64) % 60
                ));
                if let Some(slide) = deck.slides.get(state.index) {
                    ui.heading(&slide.title);
                    ui.label(&slide.notes);
                }
                if let Some(next) = deck.slides.iter().skip(state.index + 1).find(|s| !s.hidden) {
                    ui.separator();
                    ui.label("Следующий");
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(350.0, 350.0 * deck.height / deck.width),
                        egui::Sense::hover(),
                    );
                    paint_slide(&ctx, ui.painter(), rect, deck, next, None);
                }
                ui.small("Второй монитор: пока не поддерживается");
            });
    }
    if !state.active {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        return;
    }
    ctx.request_repaint();
}
#[cfg(test)]
mod tests {
    #[test]
    fn review5_automatic_advance_waits_for_the_last_effect_and_untriggered_clicks()
    -> Result<(), String> {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        let slide = &mut deck.slides[0];
        slide.transition.advance_after = Some(2.0);
        slide.elements[0].animation.effect = "fade".into();
        slide.elements[0].animation.trigger = "with_previous".into();
        slide.elements[0].animation.duration = 5.0;
        slide.elements[1].animation.effect = "fade".into();
        slide.elements[1].animation.trigger = "with_previous".into();
        slide.elements[1].animation.duration = 1.0;
        let mut state = Presentation {
            active: true,
            ..Default::default()
        };
        assert!(!state.advance_timed(&deck, 6.0));
        assert!(state.advance_timed(&deck, 7.0));
        state.move_to(&deck, 0, 10.0);
        deck.slides[0].elements[0].animation.trigger = "on_click".into();
        assert!(!state.advance_timed(&deck, 100.0));
        state.next(&deck, 100.0);
        assert!(!state.advance_timed(&deck, 106.0));
        assert!(state.advance_timed(&deck, 107.0));
        Ok(())
    }
    #[test]
    fn review5_blanking_pauses_the_slide_clock_and_returning_from_the_end_restarts_it() {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        deck.slides[0].transition.advance_after = Some(2.0);
        deck.slides[1].transition.advance_after = Some(2.0);
        let mut state = Presentation {
            active: true,
            ..Default::default()
        };
        state.set_blank(1, 1.0);
        assert!(!state.advance_timed(&deck, 10.0));
        state.next(&deck, 11.0);
        assert_eq!(state.index, 0);
        assert!(!state.advance_timed(&deck, 11.0));
        assert!(state.advance_timed(&deck, 12.0));
        assert!(state.advance_timed(&deck, 14.0));
        assert!(state.ended);
        state.previous(&deck, 20.0);
        assert!(!state.ended);
        assert!(!state.advance_timed(&deck, 21.0));
        assert!(state.advance_timed(&deck, 22.0));
    }

    #[test]
    fn starting_on_a_trailing_hidden_slide_uses_the_last_visible_slide() -> Result<(), String> {
        let mut deck = Deck::empty();
        let visible = deck.add_slide("blank");
        let hidden = deck.add_slide("blank");
        deck.slide_mut(hidden).ok_or("slide")?.hidden = true;
        let ctx = egui::Context::default();
        let mut state = Presentation::default();
        state.start(&ctx, &deck, 1);
        assert!(state.active);
        assert_eq!(state.slide_id, Some(visible));
        Ok(())
    }

    use super::*;
    #[test]
    fn live_hide_reorder_and_delete_keep_presentation_identity_safe() -> Result<(), String> {
        let mut deck = Deck::empty();
        let first = deck.add_slide("blank");
        let second = deck.add_slide("blank");
        let third = deck.add_slide("blank");
        let mut state = Presentation {
            active: true,
            ..Presentation::default()
        };
        state.move_to(&deck, 1, 0.0);
        deck.move_slide(second, 0)?;
        assert!(state.sync_slide(&deck, 1.0));
        assert_eq!((state.index, state.slide_id), (0, Some(second)));
        deck.slide_mut(second).ok_or("second")?.hidden = true;
        assert!(state.sync_slide(&deck, 2.0));
        assert_eq!(
            (state.slide_id, state.slide_started, state.previous),
            (Some(first), 2.0, None)
        );
        deck.slide_mut(first).ok_or("first")?.hidden = true;
        assert!(state.sync_slide(&deck, 3.0));
        assert_eq!(state.slide_id, Some(third));
        deck.delete_slide(third)?;
        assert!(!state.sync_slide(&deck, 4.0));
        assert!(!state.active);
        Ok(())
    }
    #[test]
    fn hiding_last_slide_falls_back_to_visible_previous_and_all_hidden_exits() -> Result<(), String>
    {
        let mut deck = Deck::empty();
        let first = deck.add_slide("blank");
        let last = deck.add_slide("blank");
        let mut state = Presentation {
            active: true,
            ..Presentation::default()
        };
        state.move_to(&deck, 1, 0.0);
        deck.slide_mut(last).ok_or("last")?.hidden = true;
        assert!(state.sync_slide(&deck, 1.0));
        assert_eq!(state.slide_id, Some(first));
        deck.slide_mut(first).ok_or("first")?.hidden = true;
        assert!(!state.sync_slide(&deck, 2.0));
        assert!(!state.active);
        Ok(())
    }
    #[test]
    fn automatic_advance_skips_hidden_and_resets_slide_clock() -> Result<(), String> {
        let mut deck = Deck::default();
        deck.slides
            .first_mut()
            .ok_or("slide")?
            .transition
            .advance_after = Some(2.0);
        let hidden = deck.add_slide("blank");
        deck.slide_mut(hidden).ok_or("hidden")?.hidden = true;
        deck.add_slide("blank");
        deck.slides
            .get_mut(2)
            .ok_or("last")?
            .transition
            .advance_after = Some(2.0);
        let mut state = Presentation {
            active: true,
            ..Presentation::default()
        };
        assert!(!state.advance_timed(&deck, 1.0));
        assert!(state.advance_timed(&deck, 2.0));
        assert_eq!((state.index, state.slide_started), (2, 2.0));
        assert!(!state.advance_timed(&deck, 2.0));
        state.blank = 1;
        assert!(!state.advance_timed(&deck, 4.0));
        state.blank = 0;
        assert!(state.advance_timed(&deck, 4.0));
        assert!(state.ended && state.active);
        assert!(!state.advance_timed(&deck, 5.0));
        Ok(())
    }
    #[test]
    fn navigation_skips_hidden() {
        let mut deck = Deck::default();
        let hidden = deck.add_slide("blank");
        let third = deck.add_slide("blank");
        if let Some(slide) = deck.slide_mut(hidden) {
            slide.hidden = true;
        }
        let mut state = Presentation {
            active: true,
            ..Presentation::default()
        };
        state.next(&deck, 1.0);
        assert_eq!(deck.slides.get(state.index).map(|s| s.id), Some(third));
        state.previous(&deck, 2.0);
        assert_eq!(state.index, 0);
    }
    #[test]
    fn click_animation_before_advance() {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        if let Some(e) = deck.slides.first_mut().and_then(|s| s.elements.first_mut()) {
            e.animation.effect = "fade".into();
            e.animation.trigger = "on_click".into();
        }
        let mut state = Presentation::default();
        state.next(&deck, 1.0);
        assert_eq!(state.index, 0);
        assert_eq!(state.clicks, 1);
        state.next(&deck, 2.0);
        assert_eq!(state.index, 1);
        // Back returns to the first slide fully built; a second back undoes the click step.
        state.previous(&deck, 3.0);
        assert_eq!((state.index, state.clicks), (0, 1));
        state.previous(&deck, 4.0);
        assert_eq!((state.index, state.clicks), (0, 0));
    }
    #[test]
    fn end_screen_then_exit_and_back() {
        let deck = Deck::default();
        let mut state = Presentation {
            active: true,
            ..Presentation::default()
        };
        state.next(&deck, 1.0);
        assert!(state.ended && state.active && state.blank == 1);
        state.previous(&deck, 2.0);
        assert!(!state.ended && state.blank == 0 && state.index == 0);
        state.next(&deck, 3.0);
        state.next(&deck, 4.0);
        assert!(!state.active);
    }
    #[test]
    fn blanked_screen_advance_only_unblanks() {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        let mut state = Presentation {
            active: true,
            blank: 2,
            ..Presentation::default()
        };
        state.next(&deck, 1.0);
        assert_eq!((state.index, state.blank), (0, 0));
    }
    #[test]
    fn empty_hidden_presentations_do_not_start() {
        let ctx = egui::Context::default();
        let mut state = Presentation::default();
        state.start(&ctx, &Deck::empty(), 0);
        assert!(!state.active);
        let mut deck = Deck::default();
        for slide in &mut deck.slides {
            slide.hidden = true;
        }
        state.start(&ctx, &deck, 0);
        assert!(!state.active);
    }
    #[test]
    fn move_to_keeps_stable_slide_identity() {
        let mut deck = Deck::default();
        let last = deck.add_slide("blank");
        let mut state = Presentation::default();
        state.move_to(&deck, 1, 1.0);
        assert_eq!(state.slide_id, Some(last));
        assert!(deck.move_slide(last, 0).is_ok());
        assert_eq!(
            deck.slides
                .iter()
                .position(|slide| Some(slide.id) == state.slide_id),
            Some(0)
        );
    }
}

fn ink_from_screen(rect: Rect, position: Pos2) -> Option<Pos2> {
    if !rect.contains(position) || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    Some(Pos2::new(
        (position.x - rect.left()) / rect.width(),
        (position.y - rect.top()) / rect.height(),
    ))
}
fn ink_to_screen(rect: Rect, position: Pos2) -> Pos2 {
    Pos2::new(
        rect.left() + position.x * rect.width(),
        rect.top() + position.y * rect.height(),
    )
}
fn presentation_key_allowed(ctx: &egui::Context, state: &Presentation, key: egui::Key) -> bool {
    key != egui::Key::Space
        || !ctx.memory(|memory| {
            memory
                .focused()
                .is_some_and(|id| state.control_ids.contains(&id))
        })
}
fn present_button(ui: &mut egui::Ui, state: &mut Presentation, label: &str) -> egui::Response {
    let response = ui.button(label);
    state.control_ids.push(response.id);
    response
}
#[cfg(test)]
mod additional_presenter_tests {
    use super::*;
    #[test]
    fn ink_tracks_slide_geometry_across_resize_and_ignores_letterbox() {
        let before = Rect::from_min_size(Pos2::new(100.0, 200.0), Vec2::new(800.0, 450.0));
        let after = Rect::from_min_size(Pos2::new(20.0, 30.0), Vec2::new(400.0, 225.0));
        let point = ink_from_screen(before, Pos2::new(500.0, 425.0));
        assert_eq!(point, Some(Pos2::new(0.5, 0.5)));
        assert_eq!(
            point.map(|point| ink_to_screen(after, point)),
            Some(after.center())
        );
        assert_eq!(ink_from_screen(before, Pos2::ZERO), None);
    }
    #[test]
    fn focused_presenter_controls_do_not_also_trigger_slide_shortcuts() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("presenter-control");
        let state = Presentation {
            control_ids: vec![id],
            ..Default::default()
        };
        ctx.memory_mut(|memory| memory.request_focus(id));
        assert!(!presentation_key_allowed(&ctx, &state, egui::Key::Space));
        ctx.memory_mut(|memory| memory.surrender_focus(id));
        assert!(presentation_key_allowed(&ctx, &state, egui::Key::Space));
    }
}

fn presentation_click_allowed(state: &Presentation) -> bool {
    !state.ink || state.blank != 0 || state.ended
}
fn advance_after_controls(state: &mut Presentation, deck: &Deck, time: f64, manual: bool) -> bool {
    state.active && !manual && state.advance_timed(deck, time)
}
fn append_ink(
    state: &mut Presentation,
    point: Pos2,
    start: bool,
    point_limit: usize,
    stroke_limit: usize,
) -> bool {
    if !start
        && state
            .strokes
            .last()
            .and_then(|points| points.last())
            .is_some_and(|last| *last == point)
    {
        return true;
    }
    if state.ink_points >= point_limit || start && state.strokes.len() >= stroke_limit {
        state.ink_full = true;
        return false;
    }
    if start {
        state.strokes.push(Vec::new());
    }
    let Some(points) = state.strokes.last_mut() else {
        return false;
    };
    points.push(point);
    state.ink_points += 1;
    true
}
#[cfg(test)]
mod next_presenter_defects {
    use super::*;
    #[test]
    fn manual_previous_has_priority_over_a_timer_expiring_in_the_same_frame() {
        let mut deck = Deck::empty();
        for _ in 0..3 {
            deck.add_slide("blank");
        }
        for slide in &mut deck.slides {
            slide.transition.advance_after = Some(1.0);
        }
        let mut state = Presentation {
            active: true,
            ..Default::default()
        };
        state.move_to(&deck, 1, 0.0);
        state.previous(&deck, 2.0);
        assert!(!advance_after_controls(&mut state, &deck, 2.0, true));
        assert_eq!(state.index, 0);
        assert!(advance_after_controls(&mut state, &deck, 4.0, false));
        assert_eq!(state.index, 1);
    }
    #[test]
    fn blank_and_end_screens_accept_clicks_even_when_the_pen_is_enabled() {
        let mut state = Presentation {
            ink: true,
            ..Default::default()
        };
        assert!(!presentation_click_allowed(&state));
        state.blank = 1;
        assert!(presentation_click_allowed(&state));
        state.ended = true;
        assert!(presentation_click_allowed(&state));
    }
    #[test]
    fn presenter_ink_has_shared_point_and_stroke_budgets_with_an_explicit_reset() {
        let mut state = Presentation::default();
        assert!(append_ink(&mut state, Pos2::ZERO, true, 2, 2));
        assert!(append_ink(&mut state, Pos2::new(1.0, 1.0), false, 2, 2));
        assert!(!append_ink(&mut state, Pos2::ZERO, true, 2, 2));
        assert_eq!(state.ink_points, 2);
        assert_eq!(state.strokes.len(), 1);
        assert!(state.ink_full);
        state.clear_ink();
        assert_eq!(state.ink_points, 0);
        assert!(!state.ink_full);
        assert!(append_ink(&mut state, Pos2::ZERO, true, 10, 1));
        assert!(!append_ink(&mut state, Pos2::ZERO, true, 10, 1));
    }
}

fn outgoing_transition_rect(rect: Rect, effect: &str, progress: f32) -> Rect {
    if effect == "push" {
        rect.translate(Vec2::new(-progress * rect.width(), 0.0))
    } else {
        rect
    }
}
fn update_ink(
    state: &mut Presentation,
    point: Option<Pos2>,
    clicked: bool,
    started: bool,
    dragged: bool,
    stopped: bool,
) {
    if !state.ink || state.blank != 0 || state.ended {
        state.inking = false;
        return;
    }
    if clicked && let Some(point) = point {
        append_ink(state, point, true, 100000, 1000);
    }
    if started {
        state.inking = false;
    }
    if started || dragged || stopped {
        if let Some(point) = point {
            if started || dragged || state.inking {
                let new_stroke = !state.inking;
                state.inking = append_ink(state, point, new_stroke, 100000, 1000);
            }
        } else {
            state.inking = false;
        }
    }
    if stopped {
        state.inking = false;
    }
}
#[cfg(test)]
mod phase200_presenter_tests {
    use super::*;
    #[test]
    fn focused_controls_keep_navigation_blanking_and_notes_shortcuts() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("focused-presenter");
        let state = Presentation {
            control_ids: vec![id],
            ..Default::default()
        };
        ctx.memory_mut(|memory| memory.request_focus(id));
        assert!(!presentation_key_allowed(&ctx, &state, egui::Key::Space));
        for key in [
            egui::Key::ArrowRight,
            egui::Key::B,
            egui::Key::W,
            egui::Key::P,
        ] {
            assert!(presentation_key_allowed(&ctx, &state, key));
        }
    }
    #[test]
    fn pen_taps_are_saved_as_visible_single_point_strokes() {
        let mut state = Presentation {
            ink: true,
            ..Default::default()
        };
        update_ink(
            &mut state,
            Some(Pos2::new(0.5, 0.5)),
            true,
            false,
            false,
            false,
        );
        assert_eq!(state.strokes, vec![vec![Pos2::new(0.5, 0.5)]]);
        assert_eq!(state.ink_points, 1);
    }
    #[test]
    fn leaving_the_slide_breaks_ink_instead_of_drawing_a_reentry_bridge() {
        let mut state = Presentation {
            ink: true,
            ..Default::default()
        };
        update_ink(
            &mut state,
            Some(Pos2::new(0.1, 0.1)),
            false,
            true,
            true,
            false,
        );
        update_ink(&mut state, None, false, false, true, false);
        update_ink(
            &mut state,
            Some(Pos2::new(0.9, 0.9)),
            false,
            false,
            true,
            false,
        );
        assert_eq!(state.strokes.len(), 2);
        assert_eq!(state.strokes[0].len(), 1);
        assert_eq!(state.strokes[1].len(), 1);
    }
    #[test]
    fn a_stationary_pen_does_not_consume_the_point_budget() {
        let mut state = Presentation::default();
        assert!(append_ink(&mut state, Pos2::ZERO, true, 2, 2));
        for _ in 0..100 {
            assert!(append_ink(&mut state, Pos2::ZERO, false, 2, 2));
        }
        assert_eq!(state.ink_points, 1);
        assert!(!state.ink_full);
        assert!(append_ink(&mut state, Pos2::new(0.1, 0.1), false, 2, 2));
        assert_eq!(state.ink_points, 2);
    }
    #[test]
    fn pen_release_keeps_the_final_sample_before_ending_the_stroke() {
        let mut state = Presentation {
            ink: true,
            ..Default::default()
        };
        update_ink(&mut state, Some(Pos2::ZERO), false, true, true, false);
        update_ink(
            &mut state,
            Some(Pos2::new(0.8, 0.7)),
            false,
            false,
            false,
            true,
        );
        assert_eq!(state.strokes[0], vec![Pos2::ZERO, Pos2::new(0.8, 0.7)]);
        assert!(!state.inking);
    }
    #[test]
    fn returning_to_a_slide_completes_automatic_builds_without_skipping_its_timer()
    -> Result<(), String> {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        let slide = deck.slides.first_mut().ok_or("slide")?;
        slide.transition.advance_after = Some(2.0);
        slide.elements[0].animation = super::super::model::Animation {
            effect: "fade".into(),
            trigger: "with_previous".into(),
            duration: 300.0,
            ..Default::default()
        };
        let mut state = Presentation {
            active: true,
            ..Default::default()
        };
        state.move_to(&deck, 1, 400.0);
        state.previous(&deck, 500.0);
        assert_eq!(state.index, 0);
        assert_eq!(state.animation_bias, 300.0);
        assert!(!state.advance_timed(&deck, 501.0));
        assert!(state.advance_timed(&deck, 502.0));
        Ok(())
    }
    #[test]
    fn push_transition_moves_both_frames_in_the_same_direction() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 450.0));
        let previous = outgoing_transition_rect(rect, "push", 0.25);
        let next = rect.translate(Vec2::new((1.0 - 0.25) * rect.width(), 0.0));
        assert_eq!(previous.left(), -200.0);
        assert_eq!(previous.right(), next.left());
        assert_eq!(outgoing_transition_rect(rect, "fade", 0.25), rect);
    }
    #[test]
    fn outgoing_transition_captures_the_departing_animation_clock_and_clicks() {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        let mut state = Presentation {
            active: true,
            ..Default::default()
        };
        state.move_to(&deck, 0, 10.0);
        state.click_times = vec![2.0];
        state.animation_bias = 5.0;
        state.move_to(&deck, 1, 14.0);
        assert_eq!(state.previous_time, 9.0);
        assert_eq!(state.previous_click_times, vec![2.0]);
        assert_eq!(state.animation_bias, 0.0);
    }
    #[test]
    fn live_animation_changes_reset_click_mapping_but_content_edits_keep_playback()
    -> Result<(), String> {
        let mut deck = Deck::default();
        deck.add_slide("blank");
        deck.slides[0].elements[0].animation = super::super::model::Animation {
            effect: "fade".into(),
            trigger: "on_click".into(),
            ..Default::default()
        };
        let mut state = Presentation {
            active: true,
            ..Default::default()
        };
        state.move_to(&deck, 0, 0.0);
        state.next(&deck, 1.0);
        assert_eq!(state.clicks, 1);
        deck.slides[0].elements[0].text = "Edited caption".into();
        assert!(state.sync_slide(&deck, 2.0));
        assert_eq!(state.clicks, 1);
        let slide_id = deck.slides[0].id;
        deck.add_element(slide_id, super::super::model::ElementKind::Rect)?;
        assert!(state.sync_slide(&deck, 2.5));
        assert_eq!(state.clicks, 1);
        deck.slides[0].elements[0].animation.effect = "fade_out".into();
        assert!(state.sync_slide(&deck, 3.0));
        assert_eq!(state.clicks, 0);
        assert!(state.click_times.is_empty());
        assert_eq!(state.slide_started, 3.0);
        state.next(&deck, 4.0);
        assert_eq!(state.index, 0);
        assert_eq!(state.clicks, 1);
        Ok(())
    }
}
