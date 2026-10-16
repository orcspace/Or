//! Clipboard decoding stays off the UI thread; offers belong to one active tab.
use std::sync::{Arc, Mutex, mpsc};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Target {
    tab: Option<u64>,
    generation: u64,
}
type Offer = (f64, Target, Arc<Vec<u8>>);
#[derive(Clone)]
struct Pending {
    target: Target,
    text_requested: bool,
    focus: Option<egui::Id>,
    result: Arc<Mutex<mpsc::Receiver<Content>>>,
}
enum Content {
    Text(String),
    Image(Vec<u8>),
    Empty,
}
fn image_id() -> egui::Id {
    egui::Id::new("trinedocs-clipboard-image")
}
fn target_id() -> egui::Id {
    egui::Id::new("trinedocs-clipboard-target")
}
fn pending_id() -> egui::Id {
    egui::Id::new("trinedocs-clipboard-pending")
}
fn target(ctx: &egui::Context) -> Target {
    ctx.data(|data| data.get_temp(target_id()))
        .unwrap_or_default()
}
pub fn set_target(ctx: &egui::Context, tab: Option<u64>) {
    let previous = target(ctx);
    if previous.tab != tab {
        let next = Target {
            tab,
            generation: previous.generation.saturating_add(1),
        };
        ctx.data_mut(|data| {
            data.insert_temp(target_id(), next);
            data.remove::<Offer>(image_id());
        });
    }
}
pub fn read_image() -> Option<Vec<u8>> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let image = clipboard.get_image().ok()?;
    let width = u32::try_from(image.width).ok()?;
    let height = u32::try_from(image.height).ok()?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 32_000_000
    {
        return None;
    }
    let rgba = image::RgbaImage::from_raw(width, height, image.bytes.into_owned())?;
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut png, image::ImageFormat::Png)
        .ok()?;
    Some(png.into_inner())
}
fn read_text() -> Option<String> {
    arboard::Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_text().ok())
        .filter(|text| !text.is_empty())
}
pub fn offer(ctx: &egui::Context, png: Vec<u8>) {
    let now = ctx.input(|input| input.time);
    let target = target(ctx);
    ctx.data_mut(|data| data.insert_temp::<Offer>(image_id(), (now, target, Arc::new(png))));
    ctx.request_repaint();
}
fn poll(ctx: &egui::Context) {
    let pending = ctx.data(|data| data.get_temp::<Pending>(pending_id()));
    let Some(pending) = pending else {
        return;
    };
    let result = pending
        .result
        .lock()
        .ok()
        .and_then(|receiver| match receiver.try_recv() {
            Ok(content) => Some(content),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Content::Empty),
        });
    if let Some(content) = result {
        ctx.data_mut(|data| data.remove::<Pending>(pending_id()));
        if pending.target != target(ctx) || pending.focus != ctx.memory(|memory| memory.focused()) {
            return;
        }
        match content {
            Content::Image(png) => offer(ctx, png),
            Content::Text(text) if pending.text_requested => {
                ctx.input_mut(|input| input.events.push(egui::Event::Paste(text)));
            }
            _ => {}
        }
    }
}
pub fn take_image(ctx: &egui::Context) -> Option<Arc<Vec<u8>>> {
    let now = ctx.input(|input| input.time);
    let offered = ctx.data_mut(|data| {
        let offered = data.get_temp::<Offer>(image_id());
        data.remove::<Offer>(image_id());
        offered
    })?;
    (offered.1 == target(ctx) && (0.0..=2.0).contains(&(now - offered.0))).then_some(offered.2)
}
fn request(ctx: &egui::Context, text_requested: bool) {
    if ctx.data(|data| data.get_temp::<Pending>(pending_id()).is_some()) {
        return;
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    let pending = Pending {
        target: target(ctx),
        text_requested,
        focus: ctx.memory(|memory| memory.focused()),
        result: Arc::new(Mutex::new(receiver)),
    };
    let repaint = ctx.clone();
    let task = std::thread::Builder::new()
        .name("clipboard-image".into())
        .spawn(move || {
            let content = if let Some(text) = read_text() {
                Content::Text(text)
            } else {
                read_image().map(Content::Image).unwrap_or(Content::Empty)
            };
            let _ = sender.send(content);
            repaint.request_repaint();
        });
    if task.is_ok() {
        ctx.data_mut(|data| data.insert_temp(pending_id(), pending));
    }
}
pub fn observe(ctx: &egui::Context) {
    let pending = ctx.data(|data| data.get_temp::<Pending>(pending_id()));
    if pending
        .as_ref()
        .is_some_and(|pending| pending.target == target(ctx))
        && ctx.input(|input| {
            input.events.iter().any(|event| match event {
                egui::Event::PointerButton { pressed: true, .. }
                | egui::Event::Text(_)
                | egui::Event::Paste(_) => true,
                egui::Event::Key { pressed: true, .. } => {
                    !image_requested(std::slice::from_ref(event))
                }
                _ => false,
            })
        })
    {
        let previous = target(ctx);
        ctx.data_mut(|data| {
            data.insert_temp(
                target_id(),
                Target {
                    generation: previous.generation.saturating_add(1),
                    ..previous
                },
            );
            data.remove::<Offer>(image_id());
        });
    }
    poll(ctx);
    if target(ctx).tab.is_none() || ctx.text_edit_focused() {
        return;
    }
    if ctx.input(|input| image_requested(&input.events)) {
        request(ctx, false);
    }
}
fn image_requested(events: &[egui::Event]) -> bool {
    !events.iter().any(|event| matches!(event, egui::Event::Paste(_)))
        && events.iter().any(|event| matches!(event,
            egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. }
            if (*key == egui::Key::V && (modifiers.command || modifiers.ctrl) && !modifiers.shift && !modifiers.alt)
                || (*key == egui::Key::Insert && modifiers.shift && !modifiers.ctrl && !modifiers.alt)))
}
pub fn paste(ctx: &egui::Context) {
    request(ctx, true);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key: egui::Key::V,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        }
    }
    #[test]
    fn text_paste_suppresses_picture_regardless_of_event_order() {
        let key = key(true, egui::Modifiers::CTRL);
        let text = egui::Event::Paste("text".into());
        assert!(image_requested(std::slice::from_ref(&key)));
        assert!(!image_requested(&[key.clone(), text.clone()]));
        assert!(!image_requested(&[text, key]));
    }
    #[test]
    fn clipboard_shortcut_uses_key_press_without_a_global_text_cooldown() {
        assert!(image_requested(&[key(true, egui::Modifiers::CTRL)]));
        assert!(!image_requested(&[key(false, egui::Modifiers::NONE)]));
        assert!(!image_requested(&[key(
            true,
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT
        )]));
    }
    #[test]
    fn an_offer_cannot_cross_tabs_or_survive_a_return_to_the_old_tab() {
        let ctx = egui::Context::default();
        set_target(&ctx, Some(1));
        offer(&ctx, vec![1, 2, 3]);
        set_target(&ctx, Some(2));
        set_target(&ctx, Some(1));
        assert!(take_image(&ctx).is_none());
    }
    #[test]
    fn asynchronous_result_keeps_its_original_tab_generation() -> Result<(), String> {
        let ctx = egui::Context::default();
        set_target(&ctx, Some(1));
        let (sender, receiver) = mpsc::sync_channel(1);
        let pending = Pending {
            target: target(&ctx),
            text_requested: false,
            focus: None,
            result: Arc::new(Mutex::new(receiver)),
        };
        ctx.data_mut(|data| data.insert_temp(pending_id(), pending));
        set_target(&ctx, Some(2));
        sender
            .send(Content::Image(vec![1, 2, 3]))
            .map_err(|error| error.to_string())?;
        poll(&ctx);
        assert!(take_image(&ctx).is_none());
        Ok(())
    }
    #[test]
    fn completed_paste_cannot_follow_a_new_text_focus() -> Result<(), String> {
        let ctx = egui::Context::default();
        set_target(&ctx, Some(1));
        let original = egui::Id::new("document");
        let other = egui::Id::new("chat");
        ctx.memory_mut(|memory| memory.request_focus(original));
        let (sender, receiver) = mpsc::sync_channel(1);
        let pending = Pending {
            target: target(&ctx),
            text_requested: true,
            focus: Some(original),
            result: Arc::new(Mutex::new(receiver)),
        };
        ctx.data_mut(|data| data.insert_temp(pending_id(), pending));
        ctx.memory_mut(|memory| memory.request_focus(other));
        sender
            .send(Content::Text("private paste".into()))
            .map_err(|error| error.to_string())?;
        poll(&ctx);
        assert!(!ctx.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Paste(_)))
        }));
        Ok(())
    }
}
