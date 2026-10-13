# ADR-0025 — Нативный Rust/egui и Windows-first
Дата: 2026-10-13. Статус: принято по прямому плану владельца в чате.
Qt/QML/CXX-Qt заменены на Rust 1.98.1, egui/eframe 0.36.2 glow. Windows 10/11 x64; macOS, сервер совместной работы, Google Drive, DOC/PPT отложены. CRDT не является текущим источником истины: модель принадлежит UI, MCP передаёт операции bounded mpsc/oneshot. Эти изменения разрешены владельцем; они не являются отменой остальных функций Office.
Системные TTF, без WebView/JS и встроенных шрифтов. DOCX/PPTX — свой OOXML, XLSX — calamine/rust_xlsxwriter, PDF — общие страницы + pdf-writer/ttf-parser/subsetter. Нативный формат zip(JSON). Lints forbid unsafe, deny unwrap/expect остаются.
Нативная сборка MSVC доступна на данной Windows машине. Статический CRT используется при сборке dist; GNU/MinGW путь поддерживается в CI при подтверждённом результате. Не требуется Visual C++ redistributable. Фактические DLL/размер и совместимость проверяются при упаковке.
NSIS LZMA solid; без UPX. Сборки без Authenticode помечаются неподписанными. Два независимых ревью обязательны перед запуском makensis. Цели размера ≤10/≤5 МБ не считаются достигнутыми до измерений.
Предыдущие ADR о Qt/Skia/Loro описывают первоначальный подход и замещены этим решением в затронутой части. Требования полного паритета остаются в matrix, нельзя выпускать preview как завершённый аналог Office.
Источники: [eframe 0.36.2](https://docs.rs/eframe/0.36.2/eframe/), [rmcp 3.5.0](https://docs.rs/rmcp/3.5.0/rmcp/), [NSIS](https://nsis.sourceforge.io/Download).
