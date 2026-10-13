#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--mcp-smoke-test") {
        let root = std::env::args()
            .skip_while(|a| a != "--mcp-smoke-test")
            .nth(1)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("TrineDocs-MCP-smoke"));
        if let Err(e) = trinedocs::mcp::integration::run(&root) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--smoke-test") {
        let root = std::env::args()
            .skip_while(|a| a != "--smoke-test")
            .nth(1)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("TrineDocs-smoke"));
        if let Err(e) = trinedocs::smoke::run(&root) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let ui_root = if std::env::args().any(|a| a == "--ui-smoke-test") {
        let Some(path) = std::env::args()
            .skip_while(|a| a != "--ui-smoke-test")
            .nth(1)
            .filter(|s| !s.starts_with('-'))
        else {
            eprintln!("--ui-smoke-test requires an output directory");
            std::process::exit(1);
        };
        let root = std::path::PathBuf::from(path);
        if let Err(e) = trinedocs::ui_smoke::prepare(&root) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        Some(root)
    } else {
        None
    };
    let icon = image::load_from_memory(include_bytes!("../assets/trinedocs.png"))
        .ok()
        .map(|i| {
            let image = i.to_rgba8();
            egui::IconData {
                width: image.width(),
                height: image.height(),
                rgba: image.into_raw(),
            }
        });
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1250.0, 820.0])
        .with_min_inner_size([780.0, 550.0])
        .with_title("TrineDocs");
    if ui_root.is_some() {
        if std::env::args().any(|a| a == "--ui-smoke-compact") {
            viewport = viewport.with_inner_size([780.0, 550.0]);
        } else if std::env::args().any(|a| a == "--ui-smoke-wide") {
            viewport = viewport.with_inner_size([1680.0, 940.0]);
        }
    }
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let result = eframe::run_native(
        "TrineDocs",
        options,
        Box::new(|cc| Ok(Box::new(trinedocs::app::App::new(cc)))),
    );
    if let Some(root) = ui_root
        && let Err(e) = trinedocs::ui_smoke::verify(&root)
    {
        eprintln!("UI smoke failed: {e}");
        std::process::exit(1);
    }
    result
}
