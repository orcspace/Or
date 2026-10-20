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
    if let Some(decoded) = decode_utf32_bom(bytes) {
        return decoded.unwrap_or_else(|_| "\u{fffd}".into());
    }
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
        fs::rename(tmp, path)
            .map_err(|e| format!("Не удалось атомарно сохранить {}: {e}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

pub fn decode_image(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    use image::ImageDecoder;
    let mut decoder = reader.into_decoder().map_err(|error| error.to_string())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 32_000_000 {
        return Err("Изображение превышает 32 млн пикселей".into());
    }
    // Optional malformed EXIF must not make otherwise decodable pixels unreadable.
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut decoded =
        image::DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
    decoded.apply_orientation(orientation);
    Ok(decoded)
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

/// Applies an affine luminance transform while keeping hue and saturation.
pub fn adjust_rgb_luminance(rgb: [u8; 3], multiplier: f64, offset: f64) -> [u8; 3] {
    let [r, g, b] = rgb.map(|channel| f64::from(channel) / 255.0);
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let light = (high + low) / 2.0;
    let next = (light * multiplier + offset).clamp(0.0, 1.0);
    let delta = high - low;
    if delta <= f64::EPSILON {
        return [(next * 255.0).round() as u8; 3];
    }
    let saturation = delta / (1.0 - (2.0 * light - 1.0).abs());
    let chroma = (1.0 - (2.0 * next - 1.0).abs()) * saturation;
    let shift = next - chroma / 2.0;
    [r, g, b].map(|channel| {
        (((channel - low) / delta * chroma + shift) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

/// A UTF-32 BOM must be checked before the UTF-16 LE prefix that it shares.
pub fn decode_utf32_bom(bytes: &[u8]) -> Option<Result<String, String>> {
    let little = bytes.starts_with(&[0xff, 0xfe, 0, 0]);
    if !little && !bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        return None;
    }
    Some((|| {
        if !(bytes.len() - 4).is_multiple_of(4) {
            return Err("Неполный UTF-32 символ".into());
        }
        let mut out = String::new();
        for unit in bytes[4..].as_chunks::<4>().0 {
            let value = if little {
                u32::from_le_bytes(*unit)
            } else {
                u32::from_be_bytes(*unit)
            };
            out.push(char::from_u32(value).ok_or("Недопустимый UTF-32 scalar")?);
        }
        Ok(out)
    })())
}
#[cfg(test)]
mod next_text_encoding_tests {
    #[test]
    fn utf32_bom_is_decoded_before_utf16_and_rejects_invalid_scalars() -> Result<(), String> {
        for little in [true, false] {
            let mut bytes = if little {
                vec![0xff, 0xfe, 0, 0]
            } else {
                vec![0, 0, 0xfe, 0xff]
            };
            for ch in "A😀".chars() {
                bytes.extend_from_slice(&if little {
                    (ch as u32).to_le_bytes()
                } else {
                    (ch as u32).to_be_bytes()
                });
            }
            assert_eq!(super::decode_utf32_bom(&bytes).ok_or("BOM")??, "A😀");
            assert_eq!(crate::docs::io::read(&bytes, "txt")?.plain_text(), "A😀");
            assert_eq!(crate::sheets::io::decode_text(&bytes)?, "A😀");
        }
        assert!(
            super::decode_utf32_bom(&[0xff, 0xfe, 0, 0, 0, 0xd8, 0, 0])
                .ok_or("BOM")?
                .is_err()
        );
        Ok(())
    }
}

#[cfg(test)]
mod two_hundred_image_regressions {
    #[test]
    fn camera_exif_orientation_is_applied_when_decoding_images() -> Result<(), String> {
        let pixels = image::RgbImage::from_fn(2, 1, |x, _| {
            image::Rgb(if x == 0 { [255, 0, 0] } else { [0, 0, 255] })
        });
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
            .encode_image(&pixels)
            .map_err(|error| error.to_string())?;
        let plain = super::decode_image(&jpeg)?;
        assert_eq!((plain.width(), plain.height()), (2, 1));
        // A little-endian TIFF IFD containing only Orientation (SHORT, one value).
        let mut exif = b"Exif\0\0II".to_vec();
        exif.extend_from_slice(&42u16.to_le_bytes());
        exif.extend_from_slice(&8u32.to_le_bytes());
        exif.extend_from_slice(&1u16.to_le_bytes());
        exif.extend_from_slice(&0x0112u16.to_le_bytes());
        exif.extend_from_slice(&3u16.to_le_bytes());
        exif.extend_from_slice(&1u32.to_le_bytes());
        let orientation_at = exif.len();
        exif.extend_from_slice(&6u16.to_le_bytes());
        exif.extend_from_slice(&0u16.to_le_bytes());
        exif.extend_from_slice(&0u32.to_le_bytes());
        let with_exif = |metadata: &[u8]| {
            let mut bytes = jpeg[..2].to_vec();
            bytes.extend_from_slice(&[0xff, 0xe1]);
            bytes.extend_from_slice(&((metadata.len() + 2) as u16).to_be_bytes());
            bytes.extend_from_slice(metadata);
            bytes.extend_from_slice(&jpeg[2..]);
            bytes
        };
        let rotated = super::decode_image(&with_exif(&exif))?;
        assert_eq!((rotated.width(), rotated.height()), (1, 2));
        let expected = plain.rotate90().to_rgba8();
        assert_eq!(rotated.to_rgba8(), expected);
        exif[orientation_at..orientation_at + 2].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            super::decode_image(&with_exif(&exif))?.to_rgba8(),
            plain.fliph().to_rgba8()
        );
        Ok(())
    }
}
