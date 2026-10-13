use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// Default document font when a family is missing on this computer.
pub const DEFAULT_FAMILY: &str = "Arial";
/// Font sizes offered by the size pickers, as in Office.
pub const SIZES: [f32; 22] = [
    8.0, 9.0, 10.0, 10.5, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 32.0, 36.0,
    40.0, 44.0, 48.0, 54.0, 60.0, 72.0,
];
const MAX_FONT_FILE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Source {
    path: PathBuf,
    index: u32,
}
/// Regular, bold, italic and bold-italic faces of one legacy (RIBBI) family.
type Faces = [Option<Source>; 4];
fn slot(bold: bool, italic: bool) -> usize {
    usize::from(bold) + 2 * usize::from(italic)
}
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| "C:/Windows".into())
            .join("Fonts"),
    ];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
    }
    dirs.push("/usr/share/fonts/truetype".into());
    dirs
}
fn read_at(file: &mut std::fs::File, offset: u64, len: usize) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    if len > 4 * 1024 * 1024 {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0; len];
    file.read_exact(&mut buf).ok()?;
    Some(buf)
}
fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}
fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}
/// Reads only the table directory, `name` and `OS/2` of one face, so scanning
/// the Fonts folder does not load whole font files.
fn describe_face(file: &mut std::fs::File, offset: u64) -> Option<(String, bool, bool)> {
    let header = read_at(file, offset, 12)?;
    let count = usize::from(be16(&header, 4)?).min(128);
    let records = read_at(file, offset + 12, count * 16)?;
    let mut name = None;
    let mut os2 = None;
    for record in records.chunks_exact(16) {
        let location = (u64::from(be32(record, 8)?), be32(record, 12)? as usize);
        match record.get(..4)? {
            b"name" => name = Some(location),
            b"OS/2" => os2 = Some(location),
            _ => {}
        }
    }
    let (at, len) = name?;
    let data = read_at(file, at, len)?;
    let table = ttf_parser::name::Table::parse(&data)?;
    let mut family = None;
    for record in table.names {
        if record.name_id == ttf_parser::name_id::FAMILY && record.is_unicode() {
            let english = record.language_id == 0x409 || record.language_id == 0;
            if let Some(text) = record.to_string()
                && (family.is_none() || english)
            {
                family = Some(text);
                if english {
                    break;
                }
            }
        }
    }
    let family = family?.trim().to_string();
    if family.is_empty() || family.starts_with('.') || family.starts_with('@') {
        return None;
    }
    let selection = os2
        .and_then(|(at, len)| read_at(file, at, len.min(64)))
        .and_then(|data| be16(&data, 62))
        .unwrap_or_default();
    Some((family, selection & 0x20 != 0, selection & 0x01 != 0))
}
fn scan() -> BTreeMap<String, Faces> {
    let mut families: BTreeMap<String, Faces> = BTreeMap::new();
    let mut files = Vec::new();
    for dir in font_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if matches!(extension.as_str(), "ttf" | "otf" | "ttc")
                && entry
                    .metadata()
                    .is_ok_and(|m| m.is_file() && m.len() <= MAX_FONT_FILE)
            {
                files.push(path);
            }
        }
    }
    files.sort();
    for path in files.into_iter().take(4000) {
        let Ok(mut file) = std::fs::File::open(&path) else {
            continue;
        };
        let Some(header) = read_at(&mut file, 0, 12) else {
            continue;
        };
        let offsets: Vec<u64> = if header.get(..4) == Some(b"ttcf") {
            let count = be32(&header, 8).unwrap_or_default().min(64) as usize;
            read_at(&mut file, 12, count * 4)
                .map(|b| {
                    b.chunks_exact(4)
                        .filter_map(|c| be32(c, 0))
                        .map(u64::from)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![0]
        };
        for (index, offset) in offsets.into_iter().enumerate() {
            if let Some((family, bold, italic)) = describe_face(&mut file, offset) {
                let faces = families.entry(family).or_default();
                if let Some(face) = faces.get_mut(slot(bold, italic))
                    && face.is_none()
                {
                    *face = Some(Source {
                        path: path.clone(),
                        index: index as u32,
                    });
                }
            }
        }
    }
    families.retain(|_, faces| faces.iter().any(Option::is_some));
    families
}
fn catalog() -> &'static BTreeMap<String, Faces> {
    static CATALOG: OnceLock<BTreeMap<String, Faces>> = OnceLock::new();
    CATALOG.get_or_init(scan)
}
/// Starts the Fonts folder scan off the UI thread.
pub fn warm_up() {
    let _ = std::thread::Builder::new()
        .name("font-scan".into())
        .spawn(|| {
            catalog();
        });
}
fn canonical(family: &str) -> Option<&'static String> {
    let wanted = family.trim();
    let catalog = catalog();
    catalog
        .get_key_value(wanted)
        .map(|(k, _)| k)
        .or_else(|| catalog.keys().find(|k| k.eq_ignore_ascii_case(wanted)))
}
/// Installed font families, alphabetically, as the font list in Office shows them.
pub fn families() -> Vec<String> {
    catalog().keys().cloned().collect()
}
pub fn is_installed(family: &str) -> bool {
    canonical(family).is_some()
}
fn source(family: &str, bold: bool, italic: bool) -> Option<Source> {
    let faces = catalog().get(canonical(family)?)?;
    let pick = |b: bool, i: bool| faces.get(slot(b, i)).cloned().flatten();
    pick(bold, italic)
        .or_else(|| pick(bold, false))
        .or_else(|| pick(false, italic))
        .or_else(|| pick(false, false))
        .or_else(|| faces.iter().flatten().next().cloned())
}
type Loaded = Option<(Arc<Vec<u8>>, u32)>;
fn load(source: &Source) -> Loaded {
    static FILES: OnceLock<Mutex<HashMap<PathBuf, Arc<Vec<u8>>>>> = OnceLock::new();
    let files = FILES.get_or_init(Default::default);
    if let Some(bytes) = files
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&source.path)
    {
        return Some((bytes.clone(), source.index));
    }
    let bytes = crate::util::read_limited(&source.path, MAX_FONT_FILE as usize).ok()?;
    ttf_parser::Face::parse(&bytes, source.index).ok()?;
    let bytes = Arc::new(bytes);
    files
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(source.path.clone(), bytes.clone());
    Some((bytes, source.index))
}
/// Bytes and face index for a family/style, falling back to the default font.
pub fn face(family: &str, bold: bool, italic: bool) -> Result<(Arc<Vec<u8>>, u32), String> {
    source(family, bold, italic)
        .and_then(|s| load(&s))
        .or_else(|| source(DEFAULT_FAMILY, bold, italic).and_then(|s| load(&s)))
        .or_else(|| source("Segoe UI", bold, italic).and_then(|s| load(&s)))
        .or_else(|| {
            font_bytes(bold, italic)
                .ok()
                .map(|bytes| (Arc::new(bytes), 0))
        })
        .ok_or_else(|| format!("Не найден шрифт {family}"))
}
pub fn font_bytes(bold: bool, italic: bool) -> Result<Vec<u8>, String> {
    let root = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:/Windows".into())
        .join("Fonts");
    let names = match (bold, italic) {
        (true, true) => ["arialbi.ttf", "segoeuiz.ttf"],
        (true, false) => ["arialbd.ttf", "segoeuib.ttf"],
        (false, true) => ["ariali.ttf", "segoeuii.ttf"],
        _ => ["arial.ttf", "segoeui.ttf"],
    };
    for name in names {
        if let Ok(bytes) = std::fs::read(root.join(name)) {
            return Ok(bytes);
        }
    }
    for name in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
    ] {
        if let Ok(b) = std::fs::read(name) {
            return Ok(b);
        }
    }
    Err("Не найден системный шрифт Arial / Segoe UI".into())
}
/// Horizontal advances (em fractions) of a family/style; cached per face.
pub fn advances(family: &str, bold: bool, italic: bool) -> Arc<HashMap<char, f32>> {
    static TABLES: OnceLock<Mutex<HashMap<(String, usize), Arc<HashMap<char, f32>>>>> =
        OnceLock::new();
    let tables = TABLES.get_or_init(Default::default);
    let key = (
        canonical(family)
            .cloned()
            .unwrap_or_else(|| DEFAULT_FAMILY.into()),
        slot(bold, italic),
    );
    if let Some(table) = tables.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return table.clone();
    }
    let mut widths = HashMap::new();
    if let Ok((bytes, index)) = face(&key.0, bold, italic)
        && let Ok(face) = ttf_parser::Face::parse(&bytes, index)
    {
        let units = f32::from(face.units_per_em()).max(1.0);
        if let Some(cmap) = face.tables().cmap {
            for subtable in cmap.subtables {
                if subtable.is_unicode() {
                    subtable.codepoints(|code| {
                        if let Some(ch) = char::from_u32(code)
                            && let Some(advance) =
                                face.glyph_index(ch).and_then(|g| face.glyph_hor_advance(g))
                        {
                            widths.entry(ch).or_insert(f32::from(advance) / units);
                        }
                    });
                }
            }
        }
    }
    let table = Arc::new(widths);
    tables
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, table.clone());
    table
}

fn base_family(bold: bool, italic: bool) -> egui::FontFamily {
    match (bold, italic) {
        (true, true) => egui::FontFamily::Name("bold-italic".into()),
        (true, false) => egui::FontFamily::Name("bold".into()),
        (false, true) => egui::FontFamily::Name("italic".into()),
        _ => egui::FontFamily::Proportional,
    }
}
fn family_key(family: &str, bold: bool, italic: bool) -> Option<String> {
    let name = canonical(family)?;
    Some(format!("doc:{name}:{}", slot(bold, italic)))
}
#[derive(Default)]
struct Registry {
    definitions: Option<egui::FontDefinitions>,
    requested: std::collections::HashSet<String>,
}
type SharedRegistry = Arc<Mutex<Registry>>;
/// Font state is kept per egui context, so offscreen PNG rendering does not
/// disturb the fonts of the application window.
fn registry(ctx: &egui::Context) -> SharedRegistry {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<SharedRegistry>(egui::Id::new("trinedocs-fonts"))
            .clone()
    })
}
/// Adds a family/style to egui; it becomes usable from the next pass.
fn register(
    ctx: &egui::Context,
    registry: &SharedRegistry,
    key: &str,
    family: &str,
    bold: bool,
    italic: bool,
) {
    let Some((bytes, index)) = source(family, bold, italic).and_then(|s| load(&s)) else {
        return;
    };
    let mut registry = registry.lock().unwrap_or_else(|e| e.into_inner());
    let Some(definitions) = registry.definitions.as_mut() else {
        return;
    };
    let mut data = egui::FontData::from_owned(bytes.as_ref().clone());
    data.index = index;
    definitions.font_data.insert(key.into(), data.into());
    let fallback = match (bold, italic) {
        (true, true) => "bold-italic",
        (true, false) => "bold",
        (false, true) => "italic",
        _ => "system",
    };
    definitions.families.insert(
        egui::FontFamily::Name(key.into()),
        vec![key.into(), fallback.into(), "system".into()],
    );
    ctx.set_fonts(definitions.clone());
    ctx.request_repaint();
}
/// The egui family that draws `family` in the requested style. Until the
/// face is loaded (in the background) the default system font is used.
pub fn egui_family(
    ctx: &egui::Context,
    family: &str,
    bold: bool,
    italic: bool,
) -> egui::FontFamily {
    let Some(key) = family_key(family, bold, italic) else {
        return base_family(bold, italic);
    };
    let name = egui::FontFamily::Name(key.as_str().into());
    if ctx.fonts(|f| f.definitions().families.contains_key(&name)) {
        return name;
    }
    let registry = registry(ctx);
    let first_request = registry
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .requested
        .insert(key.clone());
    if first_request {
        let ctx = ctx.clone();
        let family = family.to_string();
        let _ = std::thread::Builder::new()
            .name("font-load".into())
            .spawn(move || register(&ctx, &registry, &key, &family, bold, italic));
    }
    base_family(bold, italic)
}
/// Loads the given families synchronously (offscreen rendering such as PNG).
pub fn preload(ctx: &egui::Context, faces: &[(String, bool, bool)]) {
    let registry = registry(ctx);
    for (family, bold, italic) in faces {
        if let Some(key) = family_key(family, *bold, *italic) {
            registry
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .requested
                .insert(key.clone());
            register(ctx, &registry, &key, family, *bold, *italic);
        }
    }
}
pub fn install(ctx: &egui::Context) -> Result<(), String> {
    let mut fonts = egui::FontDefinitions::empty();
    for (name, bold, italic) in [
        ("system", false, false),
        ("bold", true, false),
        ("italic", false, true),
        ("bold-italic", true, true),
    ] {
        fonts.font_data.insert(
            name.into(),
            egui::FontData::from_owned(font_bytes(bold, italic)?).into(),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), vec![name.into()]);
    }
    fonts
        .families
        .insert(egui::FontFamily::Proportional, vec!["system".into()]);
    fonts
        .families
        .insert(egui::FontFamily::Monospace, vec!["system".into()]);
    {
        let registry = registry(ctx);
        let mut registry = registry.lock().unwrap_or_else(|e| e.into_inner());
        registry.definitions = Some(fonts.clone());
        registry.requested.clear();
    }
    ctx.set_fonts(fonts);
    warm_up();
    Ok(())
}
