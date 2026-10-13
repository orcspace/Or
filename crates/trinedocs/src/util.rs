use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static COUNTER: AtomicU64 = AtomicU64::new(1);
pub fn app_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("TrineDocs")
}
pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        encoding_rs::UTF_16LE.decode(&bytes[2..]).0.into_owned()
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        encoding_rs::UTF_16BE.decode(&bytes[2..]).0.into_owned()
    } else if let Ok(s) =
        std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
    {
        s.into()
    } else {
        encoding_rs::WINDOWS_1251.decode(bytes).0.into_owned()
    }
}
pub fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    read_limited(path, crate::package::MAX_FILE)
}
pub fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > limit as u64 {
        return Err(format!("Файл превышает лимит {limit} байт"));
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err(format!("Файл превышает лимит {limit} байт"));
    }
    Ok(bytes)
}
pub fn atomic_write(path: &Path, bytes: &[u8], backup: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("Нет имени файла")?.to_string_lossy();
    let tmp = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        if backup && path.exists() {
            let bak = path.with_file_name(format!("{name}.bak"));
            fs::copy(path, &bak).map_err(|e| format!("Не удалось создать резервную копию: {e}"))?;
        }
        fs::rename(&tmp, path)
            .map_err(|e| format!("Не удалось атомарно сохранить {}: {e}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn decode_image(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 32_000_000 {
        return Err("Изображение превышает 32 млн пикселей".into());
    }
    image::load_from_memory(bytes).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_decoding() {
        assert_eq!(decode_text(&[0xff, 0xfe, 0x41, 0]), "A");
        assert_eq!(decode_text(&[0xef, 0xbb, 0xbf, 65]), "A");
    }
    #[test]
    fn atomic_replace() -> Result<(), String> {
        let p = std::env::temp_dir().join(format!("trinedocs-atomic-{}.txt", std::process::id()));
        atomic_write(&p, b"one", false)?;
        atomic_write(&p, b"two", true)?;
        assert_eq!(fs::read(&p).map_err(|e| e.to_string())?, b"two");
        assert_eq!(
            fs::read(p.with_extension("txt.bak")).map_err(|e| e.to_string())?,
            b"one"
        );
        fs::remove_file(&p).map_err(|e| e.to_string())?;
        fs::remove_file(p.with_extension("txt.bak")).map_err(|e| e.to_string())?;
        Ok(())
    }
}
