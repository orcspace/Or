use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum Document {
    Docs(crate::docs::model::Doc),
    Sheets(crate::sheets::model::Book),
    Slides(crate::slides::model::Deck),
}
impl Document {
    pub fn create(kind: &str) -> Result<Self, String> {
        match kind {
            "docs" => Ok(Self::Docs(Default::default())),
            "sheets" => Ok(Self::Sheets(Default::default())),
            "slides" => Ok(Self::Slides(Default::default())),
            _ => Err("Неизвестный тип документа".into()),
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Docs(_) => "docs",
            Self::Sheets(_) => "sheets",
            Self::Slides(_) => "slides",
        }
    }
    /// Legacy TrineDocs package; still opened and used for recovery, no longer offered for saving.
    pub fn native_extension(&self) -> &'static str {
        match self {
            Self::Docs(_) => "trinedoc",
            Self::Sheets(_) => "trinesheet",
            Self::Slides(_) => "trineslide",
        }
    }
    /// Default save format: the matching Microsoft Office format.
    pub fn extension(&self) -> &'static str {
        self.office_extension()
    }
    pub fn office_extension(&self) -> &'static str {
        match self {
            Self::Docs(_) => "docx",
            Self::Sheets(_) => "xlsx",
            Self::Slides(_) => "pptx",
        }
    }
    /// Formats offered by "Save as", default first, with dialog labels.
    pub fn save_formats(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Docs(_) => &[
                ("docx", "Документ Word"),
                ("pdf", "PDF"),
                ("odt", "Текст OpenDocument"),
                ("fodt", "Плоский XML OpenDocument"),
                ("txt", "Обычный текст"),
                ("md", "Markdown"),
                ("html", "Веб-страница"),
            ],
            Self::Sheets(_) => &[
                ("xlsx", "Книга Excel"),
                ("pdf", "PDF"),
                ("csv", "CSV (разделитель — точка с запятой)"),
                ("tsv", "TSV (разделители — табуляция)"),
                ("html", "Веб-страница"),
            ],
            Self::Slides(_) => &[
                ("pptx", "Презентация PowerPoint"),
                ("pdf", "PDF"),
                ("png", "Рисунки PNG (все показываемые слайды)"),
            ],
        }
    }
    pub fn pages(&self) -> Vec<crate::render::Page> {
        match self {
            Self::Docs(d) => d.pages(),
            Self::Sheets(b) => b.pages(),
            Self::Slides(d) => d.pages(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Docs(d) => {
                d.validate()?;
            }
            Self::Sheets(b) => {
                b.validate()?;
            }
            Self::Slides(d) => {
                d.validate()?;
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    document: Document,
    #[serde(default)]
    assets: std::collections::BTreeMap<u64, String>,
}
pub fn encode(doc: &Document) -> Result<Vec<u8>, String> {
    doc.validate()?;
    let mut document = doc.clone();
    let mut assets = std::collections::BTreeMap::new();
    let mut parts = Vec::new();
    let mut add = |id: u64, bytes: &mut Vec<u8>| {
        let path = format!("assets/{id}.bin");
        assets.insert(id, path.clone());
        parts.push((path, std::mem::take(bytes)));
    };
    match &mut document {
        Document::Docs(d) => {
            for image in &mut d.images {
                add(image.id, &mut image.bytes);
            }
        }
        Document::Slides(d) => {
            for slide in &mut d.slides {
                for element in &mut slide.elements {
                    if let crate::slides::model::ElementKind::Image { bytes } = &mut element.kind {
                        add(element.id, bytes);
                    }
                }
            }
        }
        Document::Sheets(book) => {
            for sheet in &mut book.sheets {
                for image in &mut sheet.images {add(image.id,&mut image.bytes);}
            }
        }
    }
    parts.push((
        "document.json".into(),
        serde_json::to_vec(&Envelope {
            version: 2,
            document,
            assets,
        })
        .map_err(|e| e.to_string())?,
    ));
    crate::package::write_zip(parts)
}
pub fn decode(bytes: &[u8]) -> Result<Document, String> {
    let mut parts = crate::package::read_zip(bytes)?;
    let data = parts
        .remove("document.json")
        .ok_or("В архиве нет document.json")?;
    let mut env: Envelope =
        serde_json::from_slice(&data).map_err(|e| format!("Повреждённый документ: {e}"))?;
    if !matches!(env.version, 1 | 2) || env.version == 1 && !env.assets.is_empty() {
        return Err("Версия документа не поддерживается".into());
    }
    if env.version == 2 {
        let mut restore = |id: u64, bytes: &mut Vec<u8>, inline_legacy: bool| -> Result<(), String> {
            if inline_legacy && !env.assets.contains_key(&id) && !bytes.is_empty() {return Ok(());}
            let path = env
                .assets
                .remove(&id)
                .ok_or("Нет записи изображения в manifest")?;
            if path != format!("assets/{id}.bin") {
                return Err("Неверный путь asset".into());
            }
            *bytes = parts.remove(&path).ok_or("Нет части изображения")?;
            Ok(())
        };
        match &mut env.document {
            Document::Docs(d) => {
                for image in &mut d.images {
                    restore(image.id, &mut image.bytes, false)?;
                }
            }
            Document::Slides(d) => {
                for slide in &mut d.slides {
                    for element in &mut slide.elements {
                        if let crate::slides::model::ElementKind::Image { bytes } =
                            &mut element.kind
                        {
                            restore(element.id, bytes, false)?;
                        }
                    }
                }
            }
            Document::Sheets(book) => {
                for sheet in &mut book.sheets {
                    for image in &mut sheet.images {
                        // Existing v2 Sheets packages kept images inline; retain that compatibility.
                        restore(image.id,&mut image.bytes,true)?;
                    }
                }
            }
        }
        if !env.assets.is_empty() {
            return Err("Неизвестный ID в asset manifest".into());
        }
    }
    env.document.validate()?;
    if let Document::Sheets(book) = &mut env.document {
        // Spilled array results are not stored; rebuild them like an XLSX import does.
        book.recalculate();
    }
    Ok(env.document)
}
pub fn open(path: &Path) -> Result<Document, String> {
    let ext = extension(path);
    let bytes = crate::util::read_file(path)?;
    let doc = match ext.as_str() {
        "trinedoc" | "trinesheet" | "trineslide" => decode(&bytes)?,
        "lock" => Document::Docs(crate::docs::io::read(&bytes, "txt")?),
        "docx" | "dotx" | "docm" | "odt" | "ott" | "fodt" | "txt" | "md" | "html" | "htm" => {
            Document::Docs(crate::docs::io::read(&bytes, &ext)?)
        }
        "xlsx" | "xlsm" | "xltx" | "xls" | "xlsb" | "ods" | "csv" | "tsv" => {
            Document::Sheets(crate::sheets::io::read(&bytes, &ext)?)
        }
        "pptx" | "ppsx" | "potx" | "pptm" => {
            Document::Slides(crate::slides::io::read(&bytes, &ext)?)
        }
        _ => return Err(format!("Формат .{ext} пока не поддерживается")),
    };
    doc.validate()?;
    Ok(doc)
}
pub fn export(doc: &Document, ext: &str) -> Result<Vec<u8>, String> {
    doc.validate()?;
    let normalized = ext.trim_start_matches('.').to_ascii_lowercase();
    let ext = normalized.as_str();
    if ext == doc.native_extension() {
        return encode(doc);
    }
    match (doc, ext) {
        (_, "pdf") => {
            doc.validate()?;
            if let Document::Sheets(b) = doc
                && b.print_page_count() > 10000
            {
                return Err("PDF превышает 10 000 страниц".into());
            }
            crate::pdf::write(&doc.pages())
        }
        (Document::Slides(deck), "png") => {
            let slide = deck
                .slides
                .iter()
                .find(|slide| !slide.hidden)
                .ok_or("В презентации нет показываемых слайдов")?;
            crate::png::write(&deck.page(slide), 2.0)
        }
        (Document::Docs(d), "lock") => crate::docs::io::write(d, "txt"),
        (Document::Docs(d), _) => crate::docs::io::write(d, ext),
        (Document::Sheets(b), _) => crate::sheets::io::write(b, ext),
        (Document::Slides(d), _) => crate::slides::io::write(d, ext),
    }
}
/// Every slide that is not hidden as PNG, with its 1-based slide number.
pub fn slide_pictures(deck: &crate::slides::model::Deck) -> Result<Vec<(usize, Vec<u8>)>, String> {
    deck.validate()?;
    collect_slide_pictures(deck,256*1024*1024,|slide|crate::png::write(&deck.page(slide),2.0))
}
fn collect_slide_pictures(deck:&crate::slides::model::Deck,limit:usize,mut render:impl FnMut(&crate::slides::model::Slide)->Result<Vec<u8>,String>)->Result<Vec<(usize,Vec<u8>)>,String> {
    let mut pictures=Vec::new();let mut bytes=0usize;
    for (index,slide) in deck.slides.iter().enumerate().filter(|(_,slide)|!slide.hidden) {
        let picture=render(slide)?;
        bytes=bytes.checked_add(picture.len()).ok_or("Размер PNG экспорта переполнен")?;
        if bytes>limit {return Err("Совокупный PNG экспорт превышает безопасный лимит256 МиБ".into());}
        pictures.push((index+1,picture));
    }
    if pictures.is_empty(){return Err("В презентации нет показываемых слайдов".into());}
    Ok(pictures)
}
/// Sibling file for slide `number` of a multi-slide PNG export to `path` (`Имя (слайд 2).png`).
pub fn slide_picture_path(path: &Path, number: usize) -> std::path::PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!("{stem} (слайд {number}).png"))
}
/// Extra PNGs are new files; only the main file was confirmed in the Save dialog.
pub fn write_slide_picture(
    path: &Path,
    number: usize,
    bytes: &[u8],
) -> Result<std::path::PathBuf, String> {
    let preferred = slide_picture_path(path, number);
    let stem = preferred
        .file_stem()
        .ok_or("Нет имени PNG")?
        .to_string_lossy();
    for index in 1..1000 {
        let target = if index == 1 {
            preferred.clone()
        } else {
            preferred.with_file_name(format!("{stem} ({index}).png"))
        };
        match crate::util::atomic_write_new(&target, bytes) {
            Ok(()) => return Ok(target),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Не удалось сохранить {}: {error}",
                    target.display()
                ));
            }
        }
    }
    Err("Слишком много PNG с таким именем".into())
}
pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_native_roundtrips() -> Result<(), String> {
        for kind in ["docs", "sheets", "slides"] {
            let d = Document::create(kind)?;
            assert_eq!(decode(&encode(&d)?)?, d);
        }
        Ok(())
    }
    #[test]
    fn future_version_rejected() -> Result<(), String> {
        let b = crate::package::write_zip(vec![(
            "document.json".into(),
            br#"{"version":999,"document":{"kind":"docs","data":{}}}"#.to_vec(),
        )])?;
        assert!(decode(&b).is_err());
        Ok(())
    }
    #[test]
    fn export_normalizes_native_and_rendered_extensions() -> Result<(), String> {
        let doc = Document::create("slides")?;
        assert_eq!(decode(&export(&doc, ".TRINESLIDE")?)?, doc);
        assert!(export(&doc, ".PDF")?.starts_with(b"%PDF"));
        assert!(export(&doc, ".PNG")?.starts_with(b"\x89PNG"));
        Ok(())
    }
    #[test]
    fn slide_pictures_cover_every_shown_slide() -> Result<(), String> {
        let mut deck = crate::slides::model::Deck::default();
        for _ in 0..2 {
            deck.add_slide("title");
        }
        deck.slides[1].hidden = true;
        let numbers: Vec<usize> = slide_pictures(&deck)?.into_iter().map(|(n, _)| n).collect();
        assert_eq!(numbers, vec![1, 3]);
        assert_eq!(
            slide_picture_path(Path::new("dir/Отчёт.png"), 3),
            Path::new("dir/Отчёт (слайд 3).png")
        );
        Ok(())
    }
    #[test]
    fn macro_enabled_presentations_open_as_slides() -> Result<(), String> {
        let path = std::env::temp_dir().join(format!("trinedocs-pptm-{}.PPTM", std::process::id()));
        let doc = Document::create("slides")?;
        crate::util::atomic_write(&path, &export(&doc, "pptx")?, false)?;
        let result = open(&path);
        std::fs::remove_file(path).map_err(|error| error.to_string())?;
        assert_eq!(result?, doc);
        Ok(())
    }
    #[test]
    fn text_lock_files_can_be_opened_edited_and_saved() -> Result<(), String> {
        let path = std::env::temp_dir().join(format!("trinedocs-text-{}.lock", std::process::id()));
        crate::util::atomic_write(&path, b"version = 4\nname = example", false)?;
        let mut doc = open(&path)?;
        if let Document::Docs(d) = &mut doc {
            let pid = d.paragraphs.first().ok_or("Нет абзаца")?.id;
            d.insert_text(pid, 0, "# editable\n")?;
        }
        crate::util::atomic_write(&path, &export(&doc, "lock")?, false)?;
        let roundtrip = open(&path)?;
        assert!(
            matches!(roundtrip, Document::Docs(d) if d.plain_text().contains("# editable") && d.plain_text().contains("version = 4"))
        );
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    #[test]
    fn native_images_are_binary_assets() -> Result<(), String> {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let bytes = png.into_inner();
        let mut d = crate::docs::model::Doc::default();
        let id = d.alloc_id();
        d.images.push(crate::docs::model::AnchoredImage {
            id,
            after_paragraph_id: d.paragraphs[0].id,
            bytes,
            width: 10.0,
            height: 10.0,
            alt: "test".into(),
        });
        let doc = Document::Docs(d);
        let zip = encode(&doc)?;
        let p = crate::package::read_zip(&zip)?;
        assert!(p.contains_key(&format!("assets/{id}.bin")));
        assert_eq!(decode(&zip)?, doc);
        Ok(())
    }
}

#[cfg(test)]
mod next_native_image_tests {
    use super::*;
    #[test]
    fn native_sheets_store_image_bytes_as_assets_and_open_legacy_inline_images()->Result<(),String> {
        let mut png=std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2,2).write_to(&mut png,image::ImageFormat::Png).map_err(|e|e.to_string())?;
        let mut book=crate::sheets::model::Book::default();let id=book.active_id();
        let image_id=book.add_image(id,0,0,png.into_inner())?;
        let document=Document::Sheets(book);
        let bytes=encode(&document)?;let parts=crate::package::read_zip(&bytes)?;
        assert!(parts.contains_key(&format!("assets/{image_id}.bin")));
        let metadata:serde_json::Value=serde_json::from_slice(parts.get("document.json").ok_or("json")?).map_err(|e|e.to_string())?;
        assert_eq!(metadata["document"]["data"]["sheets"][0]["images"][0]["bytes"],serde_json::json!([]));
        assert_eq!(decode(&bytes)?,document);
        let legacy=crate::package::write_zip(vec![("document.json".into(),
            serde_json::to_vec(&Envelope{version:2,document:document.clone(),assets:Default::default()}).map_err(|e|e.to_string())?)])?;
        assert_eq!(decode(&legacy)?,document);Ok(())
    }
}

#[cfg(test)]
mod next_csv_label_tests {
    #[test]
    fn csv_save_description_matches_the_written_separator()->Result<(),String> {
        let mut book=crate::sheets::model::Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"left")?;book.set_cell(id,0,1,"right")?;
        let document=super::Document::Sheets(book);
        let label=document.save_formats().iter().find(|(ext,_)|*ext=="csv").map(|(_,label)|*label).ok_or("CSV label")?;
        assert!(label.contains("точка с запятой"));
        let output=String::from_utf8(super::export(&document,"csv")?).map_err(|e|e.to_string())?;
        assert!(output.contains("left;right"));Ok(())
    }
}

#[cfg(test)]
mod hundred_png_aggregate_tests {
    use super::*;
    #[test]
    fn multi_slide_pngs_have_one_aggregate_byte_budget() {
        let mut deck=crate::slides::model::Deck::empty();for _ in 0..3 {deck.add_slide("blank");}
        let mut calls=0;
        let result=collect_slide_pictures(&deck,10,|_|{calls+=1;Ok(vec![1;6])});
        assert!(result.is_err());assert_eq!(calls,2);
        deck.slides[1].hidden=true;
        let output=collect_slide_pictures(&deck,12,|_|Ok(vec![1;6]));
        assert_eq!(output.map(|items|items.into_iter().map(|(number,_)|number).collect::<Vec<_>>()),Ok(vec![1,3]));
    }
}
