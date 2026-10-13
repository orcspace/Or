use super::{editor::paint_slide, model::Deck};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Default)]
pub struct Presentation {
    pub active: bool,
    pub index: usize,
    previous: Option<u64>,
    slide_id: Option<u64>,
    slide_started: f64,
    started: f64,
    clicks: usize,
    click_times: Vec<f32>,
    ink: bool,
    pointer: bool,
    notes: bool,
    blank: u8,
    /// The black "end of slide show" screen after the last slide.
    ended: bool,
    strokes: Vec<Vec<Pos2>>,
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
            } else {
                return;
            }
        }
        self.slide_id = deck.slides.get(self.index).map(|slide| slide.id);
        self.active = true;
        self.started = ctx.input(|i| i.time);
        self.slide_started = self.started;
        self.previous = None;
        self.clicks = 0;
        self.click_times.clear();
        self.strokes.clear();
        self.blank = 0;
        self.ended = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        ctx.request_repaint();
    }
    fn move_to(&mut self, deck: &Deck, index: usize, time: f64) {
        self.previous = deck.slides.get(self.index).map(|slide| slide.id);
        self.index = index;
        self.slide_id = deck.slides.get(index).map(|slide| slide.id);
        self.slide_started = time;
        self.clicks = 0;
        self.click_times.clear();
        self.strokes.clear();
        self.blank = 0;
        self.ended = false;
    }
    fn click_steps(deck: &Deck, index: usize) -> usize {
        deck.slides
            .get(index)
            .map(|s| {
                s.elements
                    .iter()
                    .filter(|e| e.animation.effect != "none" && e.animation.trigger == "on_click")
                    .count()
            })
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
            self.blank = 0;
            return;
        }
        let count = Self::click_steps(deck, self.index);
        if self.clicks < count {
            self.clicks += 1;
            self.click_times.push((time - self.slide_started) as f32);
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
            self.ended = false;
            self.blank = 0;
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
            self.click_times = vec![-1.0e6; self.clicks];
        }
    }
    fn advance_timed(&mut self, deck: &Deck, time: f64) -> bool {
        if self.ended
            || self.blank != 0
            || !deck.slides.get(self.index).is_some_and(|slide| {
                slide
                    .transition
                    .advance_after
                    .is_some_and(|seconds| time - self.slide_started >= f64::from(seconds))
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
}
pub fn show(ui: &mut egui::Ui, deck: &Deck, state: &mut Presentation) {
    let ctx = ui.ctx().clone();
    let time = ctx.input(|i| i.time);
    if let Some(id) = state.slide_id {
        if let Some(index) = deck.slides.iter().position(|slide| slide.id == id) {
            state.index = index;
        } else {
            state.active = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            return;
        }
    }
    if deck.slides.get(state.index).is_none() {
        state.active = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        return;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.active = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        return;
    }
    if ctx.input(|i| {
        i.key_pressed(egui::Key::ArrowRight)
            || i.key_pressed(egui::Key::ArrowDown)
            || i.key_pressed(egui::Key::PageDown)
            || i.key_pressed(egui::Key::Space)
    }) {
        state.next(deck, time);
    }
    if ctx.input(|i| {
        i.key_pressed(egui::Key::ArrowLeft)
            || i.key_pressed(egui::Key::ArrowUp)
            || i.key_pressed(egui::Key::PageUp)
    }) {
        state.previous(deck, time);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Home))
        && let Some(index) = deck.slides.iter().position(|s| !s.hidden)
    {
        state.move_to(deck, index, time);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::End))
        && let Some(index) = deck.slides.iter().rposition(|s| !s.hidden)
    {
        state.move_to(deck, index, time);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::B)) {
        state.blank = if state.blank == 1 { 0 } else { 1 };
    }
    if ctx.input(|i| i.key_pressed(egui::Key::W)) {
        state.blank = if state.blank == 2 { 0 } else { 2 };
    }
    if ctx.input(|i| i.key_pressed(egui::Key::P)) {
        state.notes = !state.notes;
    }
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
            let mut advanced_automatically = false;
            if let Some(slide) = deck.slides.get(state.index) {
                let elapsed = (time - state.slide_started) as f32;
                let progress = (elapsed / slide.transition.duration.max(0.05)).clamp(0.0, 1.0);
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
                        paint_slide(&ctx, &painter, rect, deck, previous, None);
                    }
                    let animation_time = elapsed;
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
                    advanced_automatically = state.advance_timed(deck, time);
                    for points in &state.strokes {
                        if points.len() > 1 {
                            painter.add(egui::Shape::line(
                                points.clone(),
                                Stroke::new(2.5, Color32::from_rgb(235, 50, 50)),
                            ));
                        }
                    }
                    if state.pointer
                        && let Some(pos) = ctx.pointer_hover_pos()
                    {
                        painter.circle_filled(pos, 6.0, Color32::from_rgb(240, 50, 45));
                    }
                }
            }
            if response.clicked() && !state.ink && !advanced_automatically {
                state.next(deck, time);
            }
            if state.ink && response.drag_started() {
                state.strokes.push(Vec::new());
            }
            if state.ink
                && response.dragged()
                && let Some(pos) = ctx.pointer_interact_pos()
                && let Some(points) = state.strokes.last_mut()
            {
                points.push(pos);
            }
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
                        if ui
                            .button("←")
                            .on_hover_text("Предыдущий слайд · ←")
                            .clicked()
                        {
                            state.previous(deck, time);
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
                        if ui
                            .button("→")
                            .on_hover_text("Следующий слайд · →")
                            .clicked()
                            && !advanced_automatically
                        {
                            state.next(deck, time);
                        }
                        ui.checkbox(&mut state.pointer, "Указка");
                        ui.checkbox(&mut state.ink, "Перо");
                        if ui.button("Очистить").clicked() {
                            state.strokes.clear();
                        }
                        ui.checkbox(&mut state.notes, "Заметки (P)");
                        if ui.button("Завершить Esc").clicked() {
                            state.active = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        }
                    });
                });
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
    use super::*;
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
