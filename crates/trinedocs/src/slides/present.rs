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
    }
    fn next(&mut self, deck: &Deck, time: f64) {
        let count = deck
            .slides
            .get(self.index)
            .map(|s| {
                s.elements
                    .iter()
                    .filter(|e| e.animation.effect != "none" && e.animation.trigger == "on_click")
                    .count()
            })
            .unwrap_or(0);
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
            self.blank = 1;
        }
    }
    fn previous(&mut self, deck: &Deck, time: f64) {
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
        }
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
                    if let Some(seconds) = slide.transition.advance_after
                        && elapsed >= seconds
                    {
                        if let Some(index) = deck
                            .slides
                            .iter()
                            .enumerate()
                            .skip(state.index + 1)
                            .find(|(_, s)| !s.hidden)
                            .map(|(i, _)| i)
                        {
                            state.move_to(deck, index, time);
                        } else {
                            state.blank = 1;
                        }
                    }
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
            if response.clicked() && !state.ink {
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
            let controls =
                Rect::from_min_size(outer.min + Vec2::new(12.0, 10.0), Vec2::new(660.0, 36.0));
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
                    if ui.button("←").clicked() {
                        state.previous(deck, time);
                    }
                    ui.label(format!("{} / {}", state.index + 1, deck.slides.len()));
                    if ui.button("→").clicked() {
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
    ctx.request_repaint();
}
#[cfg(test)]
mod tests {
    use super::*;
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
