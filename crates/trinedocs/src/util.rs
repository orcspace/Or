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
    replace_file(
        path,
        |file| file.write_all(bytes),
        || {
            if backup && path.exists() {
                let name = path.file_name().ok_or("Нет имени файла")?.to_string_lossy();
                let bak = path.with_file_name(format!("{name}.bak"));
                replace_file(
                    &bak,
                    |file| {
                        let mut source = fs::File::open(path)?;
                        std::io::copy(&mut source, file).map(|_| ())
                    },
                    || Ok(()),
                )
                .map_err(|error| format!("Не удалось создать резервную копию: {error}"))?;
            }
            Ok(())
        },
    )
}

/// Publishes complete bytes at a new name. A hard link atomically refuses an existing target;
/// both names are on the same volume. Filesystems without hard links return an explicit error.
pub fn atomic_write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".trinedocs-export-{}-{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::hard_link(&tmp, path)
    })();
    let _ = fs::remove_file(&tmp);
    result
}

fn replace_file(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
    before_replace: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
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
    replace_file_at(path, &tmp, write, before_replace)
}

fn replace_file_at(
    path: &Path,
    tmp: &Path,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
    before_replace: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        write(&mut file)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        before_replace()?;
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
    fn new_exports_cannot_overwrite_an_existing_file() -> Result<(), String> {
        let path = std::env::temp_dir().join(format!(
            "trinedocs-new-export-{}-{}.png",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        atomic_write_new(&path, b"original").map_err(|e| e.to_string())?;
        let error = atomic_write_new(&path, b"replacement")
            .err()
            .ok_or("overwritten")?;
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).map_err(|e| e.to_string())?, b"original");
        fs::remove_file(path).map_err(|e| e.to_string())?;
        Ok(())
    }
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

    #[test]
    fn failed_partial_backup_preserves_the_previous_backup_and_source() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "TrineDocs-backup-failure-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let source = root.join("file.docx");
        let backup = root.join("file.docx.bak");
        fs::write(&source, b"current source").map_err(|e| e.to_string())?;
        fs::write(&backup, b"previous backup").map_err(|e| e.to_string())?;
        let failed = replace_file(
            &source,
            |f| f.write_all(b"new document"),
            || {
                replace_file(
                    &backup,
                    |f| {
                        f.write_all(b"partial")?;
                        Err(std::io::Error::other("injected disk failure"))
                    },
                    || Ok(()),
                )
            },
        );
        assert!(failed.is_err());
        assert_eq!(
            fs::read(&source).map_err(|e| e.to_string())?,
            b"current source"
        );
        assert_eq!(
            fs::read(&backup).map_err(|e| e.to_string())?,
            b"previous backup"
        );
        assert_eq!(fs::read_dir(&root).map_err(|e| e.to_string())?.count(), 2);
        fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn temporary_file_collision_does_not_remove_another_writers_file() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "TrineDocs-temp-collision-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let path = root.join("file.docx");
        let tmp = root.join("occupied.tmp");
        fs::write(&path, b"source").map_err(|e| e.to_string())?;
        fs::write(&tmp, b"another writer").map_err(|e| e.to_string())?;
        assert!(replace_file_at(&path, &tmp, |f| f.write_all(b"replacement"), || Ok(())).is_err());
        assert_eq!(
            fs::read(&tmp).map_err(|e| e.to_string())?,
            b"another writer"
        );
        assert_eq!(fs::read(&path).map_err(|e| e.to_string())?, b"source");
        fs::remove_dir_all(root).map_err(|e| e.to_string())?;
        Ok(())
    }
}
