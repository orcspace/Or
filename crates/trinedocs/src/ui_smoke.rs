use crate::native::Document;
use std::{path::PathBuf, time::Instant};
pub struct State {
    pub root: PathBuf,
    stage: usize,
    frames: usize,
    requested: bool,
    started: Instant,
}
impl State {
    pub fn from_args() -> Option<Self> {
        std::env::args()
            .skip_while(|a| a != "--ui-smoke-test")
            .nth(1)
            .map(|p| Self {
                root: PathBuf::from(p),
                stage: 0,
                frames: 0,
                requested: false,
                started: Instant::now(),
            })
    }
    pub fn tick(&mut self, ctx: &egui::Context) -> Result<Option<Document>, String> {
        if self.started.elapsed().as_secs() > 20 {
            return Err("UI screenshot timeout".into());
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let (w, h) = (image.size[0] as u32, image.size[1] as u32);
                let rgba = image
                    .pixels
                    .iter()
                    .flat_map(|p| p.to_array())
                    .collect::<Vec<_>>();
                let image = image::RgbaImage::from_raw(w, h, rgba).ok_or("Screenshot pixels")?;
                let mut png = std::io::Cursor::new(Vec::new());
                image
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                let name = ["home", "docs", "sheets", "slides"]
                    .get(self.stage)
                    .ok_or("Screenshot stage")?;
                crate::util::atomic_write(
                    &self.root.join(format!("{name}.png")),
                    &png.into_inner(),
                    false,
                )?;
                self.stage += 1;
                self.frames = 0;
                self.requested = false;
                if self.stage == 4 {
                    crate::util::atomic_write(
                        &self.root.join("result.json"),
                        br#"{"status":"passed","rendered":["home","docs","sheets","slides"]}"#,
                        false,
                    )?;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return Ok(None);
                }
                return sample(self.stage).map(Some);
            }
        }
        self.frames += 1;
        if self.frames >= 10 && !self.requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        ctx.request_repaint();
        Ok(None)
    }
}
fn sample(stage: usize) -> Result<Document, String> {
    match stage {
        1 => {
            let mut d = crate::docs::model::Doc::from_text(
                "TrineDocs · Документ\nФорматирование, страницы и таблицы\nАгент и пользователь редактируют одну модель.",
            );
            d.paragraphs[0].set_style("Heading1");
            d.paragraphs[1].set_style("Heading2");
            let id = d.paragraphs[2].id;
            d.add_table(id, 3, 3)?;
            Ok(Document::Docs(d))
        }
        2 => {
            let mut b = crate::sheets::model::Book::default();
            let sid = b.sheets[0].id;
            b.set_range(
                sid,
                0,
                0,
                &[
                    vec!["Месяц".into(), "Продажи".into()],
                    vec!["Январь".into(), "120".into()],
                    vec!["Февраль".into(), "180".into()],
                    vec!["Март".into(), "240".into()],
                    vec!["Итого".into(), "=SUM(B2:B4)".into()],
                ],
            )?;
            b.recalculate();
            b.add_chart(
                sid,
                crate::sheets::model::ChartKind::Column,
                crate::sheets::model::Range::new(0, 0, 3, 1),
                "Продажи",
            )?;
            Ok(Document::Sheets(b))
        }
        3 => {
            let mut d = crate::slides::model::Deck::default();
            d.slides[0].elements.clear();
            let sid = d.slides[0].id;
            let id = d.add_element(sid, crate::slides::model::ElementKind::Text)?;
            if let Some(e) = d
                .slide_mut(sid)
                .and_then(|s| s.elements.iter_mut().find(|e| e.id == id))
            {
                e.text = "TrineDocs · Презентация".into();
                e.x = 70.0;
                e.y = 70.0;
                e.w = 700.0;
                e.font_size = 42.0;
                e.bold = true;
            }
            let shape = d.add_element(sid, crate::slides::model::ElementKind::Rect)?;
            if let Some(e) = d
                .slide_mut(sid)
                .and_then(|s| s.elements.iter_mut().find(|e| e.id == shape))
            {
                e.x = 70.0;
                e.y = 220.0;
                e.w = 350.0;
                e.h = 200.0;
            }
            Ok(Document::Slides(d))
        }
        _ => Err("stage".into()),
    }
}

pub fn prepare(root: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    match std::fs::remove_file(root.join("result.json")) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    Ok(())
}
pub fn verify(root: &std::path::Path) -> Result<(), String> {
    let bytes = crate::util::read_limited(&root.join("result.json"), 1024)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["status"] != "passed"
        || value["rendered"] != serde_json::json!(["home", "docs", "sheets", "slides"])
    {
        return Err("Incomplete UI smoke result".into());
    }
    for name in ["home", "docs", "sheets", "slides"] {
        let bytes = crate::util::read_limited(&root.join(format!("{name}.png")), 16 * 1024 * 1024)?;
        let image = crate::util::decode_image(&bytes)?;
        if image.width() < 780 || image.height() < 550 {
            return Err("UI screenshot viewport too small".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn stale_ui_success_is_not_reused() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!("trinedocs-ui-smoke-{}", std::process::id()));
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        crate::util::atomic_write(
            &root.join("result.json"),
            br#"{"status":"passed","rendered":["home","docs","sheets","slides"]}"#,
            false,
        )?;
        super::prepare(&root)?;
        assert!(super::verify(&root).is_err());
        std::fs::remove_dir(&root).map_err(|e| e.to_string())?;
        Ok(())
    }
}
