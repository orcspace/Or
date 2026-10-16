# TrineDocs
Нативный Windows офисный пакет на Rust/egui: Docs, Sheets, Slides и локальный MCP. Windows 10/11 x64, системные шрифты, glow; без WebView и JavaScript.

**Разработка / preview. Полный паритет Word, Excel и PowerPoint ещё не достигнут.** Проверки и ограничения: [release-review](docs/audit/release-review.md), [Docs](docs/parity-docs.md), [Sheets](docs/parity-sheets.md), [Slides](docs/parity-slides.md). Исходная спецификация и все будущие функции сохранены в [AGENTS](AGENTS.md). Новый стек: [ADR-0025](docs/decisions/0025-native-egui.md).

Приложение: Главная/вкладки, paginated Docs canvas, формулы и виртуальная Sheets сетка, Slides canvas и показ. Нативные ZIP/JSON, DOCX/XLSX/PPTX, текстовые форматы, PDF и PNG первого слайда. Сохранение атомарно с .bak, recovery snapshots, авторские undo-группы. MCP через официальный rmcp и stdio-шим; новым клиентам требуется разрешение в UI.

Интерфейс: светлая компактная лента, основные действия в одной строке, расширенные команды в меню. Таблицы доступны в каждом редакторе; свойства объектов и заметки Slides открываются по запросу. Текстовые `.lock` можно править как TXT. Занятые документы доступны для правки в памяти и сохранения копии; перезапись занятого оригинала защищена.

```powershell
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p trinedocs --bin TrineDocs
cargo build --locked --profile dist -p trinedocs
target/dist/TrineDocs.exe --smoke-test target/smoke
target/dist/TrineDocs.exe --mcp-smoke-test target/mcp-smoke
tools/test_preview.ps1 -Executable target/dist/TrineDocs.exe -Output target/qa-preview
tools/build_release.ps1
tools/test_installer.ps1
```

Для MSVC нужен Windows SDK/Visual C++ Build Tools. NSIS script — installer/trinedocs.nsi. Шим trinedocs-mcp.exe лежит рядом с приложением; конфигурация APPDATA/TrineDocs/mcp.json, settings и recovery там же. Конфиг клиента копируется из настроек; регистрация показывает конкретные изменения и требует согласия. Макросы Office не исполняются; XLS/XLSB и date1904 пока явно отклоняются.

Системные шрифты не распространяются с приложением. Подписи Authenticode пока нет. Лицензия проекта владельцем ещё не выбрана; [зависимости и notices](docs/licenses.md) не заменяют лицензию исходников.
