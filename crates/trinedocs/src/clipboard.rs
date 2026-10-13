//! Pictures from the system clipboard (ADR-0026). egui delivers only text pastes, so the
//! shell notices Ctrl+V without a text paste and offers the clipboard bitmap to the editor.
use std::sync::Arc;

type Offer = (f64, Arc<Vec<u8>>);

fn image_id() -> egui::Id {
    egui::Id::new("trinedocs-clipboard-image")
}
fn text_paste_id() -> egui::Id {
    egui::Id::new("trinedocs-clipboard-text-paste")
}
/// Reads the clipboard bitmap and encodes it as PNG.
pub fn read_image() -> Option<Vec<u8>> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let image = clipboard.get_image().ok()?;
    let width = u32::try_from(image.width).ok()?;
    let height = u32::try_from(image.height).ok()?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return None;
    }
    let rgba = image::RgbaImage::from_raw(width, height, image.bytes.into_owned())?;
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut png, image::ImageFormat::Png)
        .ok()?;
    Some(png.into_inner())
}
fn has_text() -> bool {
    arboard::Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_text().ok())
        .is_some_and(|text| !text.is_empty())
}
/// Makes `png` available to the active editor for a short time.
pub fn offer(ctx: &egui::Context, png: Vec<u8>) {
    let now = ctx.input(|i| i.time);
    ctx.data_mut(|d| d.insert_temp::<Offer>(image_id(), (now, Arc::new(png))));
    ctx.request_repaint();
}
/// Takes a picture pasted during the last two seconds, if any.
pub fn take_image(ctx: &egui::Context) -> Option<Arc<Vec<u8>>> {
    let now = ctx.input(|i| i.time);
    let offer = ctx.data_mut(|d| {
        let offer = d.get_temp::<Offer>(image_id());
        if offer.is_some() {
            d.remove::<Offer>(image_id());
        }
        offer
    })?;
    (now - offer.0 <= 2.0).then_some(offer.1)
}
/// Called once per frame by the shell: Ctrl+V (or Shift+Insert) without a text paste
/// pastes the clipboard picture.
pub fn observe(ctx: &egui::Context) {
    let (now, events) = ctx.input(|i| (i.time, i.events.clone()));
    let mut last_text = ctx
        .data(|d| d.get_temp::<f64>(text_paste_id()))
        .unwrap_or(f64::NEG_INFINITY);
    let mut wanted = false;
    for event in &events {
        match event {
            egui::Event::Paste(_) => last_text = now,
            egui::Event::Key {
                key,
                pressed: false,
                modifiers,
                ..
            } if (*key == egui::Key::V && modifiers.command)
                || (*key == egui::Key::Insert && modifiers.shift) =>
            {
                wanted = now - last_text > 1.0;
            }
            _ => {}
        }
    }
    ctx.data_mut(|d| d.insert_temp(text_paste_id(), last_text));
    if wanted && let Some(png) = read_image() {
        offer(ctx, png);
    }
}
/// The ribbon "Paste" command: text goes through egui, otherwise the clipboard picture.
pub fn paste(ctx: &egui::Context) {
    if has_text() {
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    } else if let Some(png) = read_image() {
        offer(ctx, png);
    }
}
