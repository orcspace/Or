use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
pub const MAX_FILE: usize = 64 * 1024 * 1024;
pub const MAX_INFLATED: u64 = 256 * 1024 * 1024;
pub fn read_zip(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if bytes.len() > MAX_FILE {
        return Err("Файл превышает 64 МБ".into());
    }
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Повреждённый ZIP: {e}"))?;
    if zip.len() > 10000 {
        return Err("Слишком много частей в архиве".into());
    }
    let mut out = BTreeMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        if f.enclosed_name().is_none() || !safe_name(&name) {
            return Err("Небезопасное имя ZIP".into());
        }
        total = total.checked_add(f.size()).ok_or("Размер ZIP переполнен")?;
        if total > MAX_INFLATED
            || f.size() > MAX_FILE as u64
            || (f.size() > 1024 * 1024 && f.size() > f.compressed_size().saturating_mul(500))
        {
            return Err("Превышен безопасный лимит распаковки".into());
        }
        let mut content = Vec::new();
        f.by_ref()
            .take(MAX_FILE as u64 + 1)
            .read_to_end(&mut content)
            .map_err(|e| e.to_string())?;
        if content.len() as u64 != f.size()
            || content.len() > MAX_FILE
            || out.insert(name, content).is_some()
        {
            return Err("Недопустимый ZIP".into());
        }
    }
    Ok(out)
}
pub fn write_zip(parts: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, String> {
    if parts.len() > 10000 {
        return Err("Слишком много частей в архиве".into());
    }
    let mut total = 0u64;
    let mut names = std::collections::BTreeSet::new();
    for (name, data) in &parts {
        if !safe_name(name) || !names.insert(name) {
            return Err("Недопустимое имя/дубликат ZIP".into());
        }
        if (name.ends_with(".xml") || name.ends_with(".rels"))
            && let Ok(xml) = std::str::from_utf8(data)
            && let Some(character) = xml.chars().find(|ch| !matches!(*ch, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'))
        {
            return Err(format!("Недопустимый символ U+{:04X} в XML-части {name}; файл не изменён", character as u32));
        }
        total = total.saturating_add(data.len() as u64);
        if data.len() > MAX_FILE || total > MAX_INFLATED {
            return Err("Превышен лимит архива при сохранении".into());
        }
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in parts {
        zip.start_file(
            name,
            // OpenDocument requires an uncompressed `mimetype` as the first entry.
            if data.len() > 1024 * 1024 || name == "mimetype" {
                options.compression_method(zip::CompressionMethod::Stored)
            } else {
                options
            },
        )
        .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }
    let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
    if bytes.len() > MAX_FILE {
        return Err("Сохраняемый архив превышает 64 МБ".into());
    }
    Ok(bytes)
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['\\', '\0', ':'])
        && name.split('/').all(|part| !matches!(part, "" | "." | ".."))
}
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zip_roundtrip() -> Result<(), String> {
        let b = write_zip(vec![("a/b.xml".into(), b"hello".to_vec())])?;
        assert_eq!(read_zip(&b)?.get("a/b.xml"), Some(&b"hello".to_vec()));
        Ok(())
    }
    #[test]
    fn rejects_traversal() -> Result<(), String> {
        assert!(write_zip(vec![("../bad".into(), vec![0])]).is_err());
        Ok(())
    }
    #[test]
    fn reader_and_writer_reject_ambiguous_archive_names() -> Result<(), String> {
        for name in [
            "a/../b.xml",
            "a/./b.xml",
            "a//b.xml",
            "/absolute.xml",
            "C:/drive.xml",
            "a\\b.xml",
        ] {
            assert!(write_zip(vec![(name.into(), b"text".to_vec())]).is_err());
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .map_err(|e| e.to_string())?;
            zip.write_all(b"text").map_err(|e| e.to_string())?;
            let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
            assert!(read_zip(&bytes).is_err(), "{name}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod xml_character_tests {
    #[test]
    fn rejects_illegal_xml_scalars_without_modifying_binary_assets() -> Result<(), String> {
        let mut deck = crate::slides::model::Deck::default();
        deck.slides[0].notes = "Text with \u{1} control".into();
        assert!(crate::native::export(&crate::native::Document::Slides(deck), "pptx").is_err());

        for character in ['\u{0}', '\u{1}', '\u{b}', '\u{fffe}', '\u{ffff}'] {
            assert!(
                super::write_zip(vec![(
                    "ppt/slides/slide1.xml".into(),
                    format!("<t>{character}</t>").into_bytes()
                )])
                .is_err()
            );
        }
        let bytes = super::write_zip(vec![
            (
                "ppt/slides/slide1.xml".into(),
                "<t>\t\n\r Кириллица 😀</t>".as_bytes().to_vec(),
            ),
            ("assets/image.bin".into(), vec![0, 1, 11, 255]),
        ])?;
        assert_eq!(
            super::read_zip(&bytes)?.get("assets/image.bin"),
            Some(&vec![0, 1, 11, 255])
        );
        Ok(())
    }
}
