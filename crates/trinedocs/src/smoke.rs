use crate::native::{self, Document};
use std::path::Path;
pub fn run(root: &Path) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut docs = crate::docs::model::Doc::from_text("TrineDocs smoke test\nПривет, мир.");
    if let Some(p) = docs.paragraphs.first_mut() {
        p.set_style("Heading1");
    }
    let mut book = crate::sheets::model::Book::default();
    let sid = book.sheets.first().ok_or("Нет листа")?.id;
    book.set_cell(sid, 0, 0, "12")?;
    book.set_cell(sid, 1, 0, "30")?;
    book.set_cell(sid, 2, 0, "=SUM(A1:A2)")?;
    book.recalculate();
    if book
        .sheet_by_id(sid)
        .and_then(|s| s.cell(2, 0))
        .map(|c| c.value.display())
        != Some("42".into())
    {
        return Err("Ошибка вычисления формулы".into());
    }
    let mut deck = crate::slides::model::Deck::default();
    let slide = deck.slides.first().ok_or("Нет слайда")?.id;
    let eid = deck.add_element(slide, crate::slides::model::ElementKind::Text)?;
    if let Some(e) = deck
        .slide_mut(slide)
        .and_then(|s| s.elements.iter_mut().find(|e| e.id == eid))
    {
        e.text = "TrineDocs".into();
    }
    let png = native::export(&Document::Slides(deck.clone()), "png")?;
    let decoded = crate::util::decode_image(&png)?;
    if decoded.width() == 0 || decoded.height() == 0 {
        return Err("PNG empty".into());
    }
    crate::util::atomic_write(&root.join("slides.png"), &png, false)?;
    for (name, document) in [
        ("docs", Document::Docs(docs)),
        ("sheets", Document::Sheets(book)),
        ("slides", Document::Slides(deck)),
    ] {
        let p = root.join(format!("{name}.{}", document.native_extension()));
        crate::util::atomic_write(&p, &native::encode(&document)?, false)?;
        if native::open(&p)? != document {
            return Err(format!("native roundtrip {name}"));
        }
        let p = root.join(format!("{name}.{}", document.office_extension()));
        crate::util::atomic_write(
            &p,
            &native::export(&document, document.office_extension())?,
            false,
        )?;
        native::open(&p)?.validate()?;
        let bytes = native::export(&document, "pdf")?;
        if !bytes.starts_with(b"%PDF-") || !bytes.ends_with(b"%%EOF\n") {
            return Err("PDF повреждён".into());
        }
        crate::util::atomic_write(&root.join(format!("{name}.pdf")), &bytes, false)?;
    }
    crate::util::atomic_write(
        &root.join("result.json"),
        br#"{"status":"passed","formats":["native","docx","xlsx","pptx","pdf","png"]}"#,
        false,
    )?;
    Ok(())
}
