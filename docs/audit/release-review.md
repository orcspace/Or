# TrineDocs 0.1.0 preview: release review
Дата: 2026-10-13. Это неподписанная предварительная сборка, не завершённая замена Microsoft Office. Полное условие готовности владельца пока не выполнено.

## Два больших ревью до сборки установщика
Волна 1: [Docs](review-1-docs.md), [Sheets](review-1-sheets.md), [Slides](review-1-slides.md).
Волна 2: [Docs](review-2-docs.md), [Sheets](review-2-sheets.md), [Slides](review-2-slides.md).
Три независимых агента проверили редакторы, форматы, формулы, границы ресурсов, сохранение, MCP и общую оболочку. Root применил общие исправления. После этого отдельно проверены NSIS, release workflow и smoke harness. Анализ исходников не считается ручной приёмкой Office.

## Подтверждённые результаты
- Rust 1.98.1 / Windows MSVC: cargo fmt, cargo clippy --workspace --all-targets --locked -- -D warnings прошли.
- cargo test --workspace --locked: 112 app + 14 shell = 126 passed, 0 failed. Формульный тест содержит >150 числовых cases; это не 150 отдельных test functions.
- Debug executable: файлы создаются/сохраняются/повторно открываются в native, DOCX, XLSX, PPTX; PDF создаётся.
- Реальный rmcp HTTP-клиент + отдельный stdio shim: create/read/insert/undo/redo, стабильная session identity; no-token, invalid Host/Origin, >1 MiB body отклоняются.
- Native eframe/glow: Home, Docs, Sheets, Slides отрисованы в реальном окне; снимки просмотрены. Это render smoke, не автоматизация клавиатуры/IME/диалогов.
- Исправлены ZIP/XML/allocation bounds, PDF resources/Unicode/geometry, recovery sequence races, undo version overflow, stale save guards, same-file reopen, cancel/exit save continuations, MCP author labels/types/bounds, zero-width PDF outlines.
- Native v2 хранит binary assets отдельно в ZIP; recovery использует тот же формат. Операции модели принадлежат UI-потоку.
- NSIS 3.13 archive SHA-256 проверен по official release-data. Макросы/Office defaults не исполняются/не изменяются.

- После закрепления aged lock все 123 теста и clippy прошли повторно. 444 exact registry версии проверены по crates.io; все ≥14 дней, SHA-256 совпадают. Windows graph: 259 normal/build/proc-macro crates, полные notices воспроизводимы offline. Mac-only dispatch не имеет исходного текста лицензии и явно исключён из Windows payload, а не признан проверенным.
- Внешний MuPDF parser открыл DOCS/SHEETS/SLIDES PDF без xref repair; русский/числовой/латинский текст извлекается, первые страницы отрендерены и просмотрены. Poppler не найден на данном стенде, использован доступный MuPDF.

Статическая policy допускает Windows 10/11 x64 build10240+; совместимость реальной Win10 ещё не проверена. Стенд текущего GUI smoke: Windows 11 Pro 10.0.26300 x64, Intel Core i5-13400F (16 logical cores), NVIDIA RTX 3070 driver32.0.16.1088. Итог installer/CI замеров будет дописан после выполнения, а не предсказан.

## Что есть
Docs: собственный paginated canvas, rich runs, заголовки, списки, paragraph/page controls, таблицы/картинки, поиск, DOCX/text/HTML, PDF.
Sheets: sparse multi-sheet модель, virtual grid, ~140 formula names, reference transforms, форматирование, CSV/XLSX/ODS import, XLSX/CSV/HTML/PDF export, базовые charts.
Slides: stable IDs, холст, resize/rotate/groups/layers, текст/фигуры/таблицы/charts/images/notes, basic fullscreen transitions/animations, PPTX/PDF/PNG.
Оболочка: Home/recent/pins/tabs, save/copy/export/backup, file watcher/locks/recovery, undo/AI history, palette/settings, localhost token-auth MCP и registration preview для Codex/Claude/Cursor/VS Code.

## Открытые требования и ограничения
- Все функции Word/Excel/PowerPoint не реализованы. Подробно: [Docs](../parity-docs.md), [Sheets](../parity-sheets.md), [Slides](../parity-slides.md), [общая matrix](../parity.md). Будущие пункты остаются в спецификации.
- Advanced Word layout, track changes/comments/mail merge, секции/поля/Math/RTL/CJK/system-font selection, rich clipboard и полный EN UI неполны.
- Excel: полный function catalog/dynamic arrays/pivots/Power Query/VBA/data validation/conditional formats/advanced filters отсутствуют или частичны. XLS/XLSB и date1904 явно отклоняются. Incremental graph нет; 100k formulas≤50ms и grid60fps не подтверждены.
- PowerPoint masters/layout fidelity/SmartArt/media/equations и полный p:timing, second-monitor presenter, rich text engine неполны.
- UI клонирует модель на кадр. Large-document latency/cold start/60fps/DPI/IME/Narrator не измерены.
- MCP работает с существующей моделью через UI queue; полноценные live agent cursors/30ms streaming, resources notifications/Stop unfinished rollback/logs ещё не реализованы. MCP open/export пока требуют File UI; откат AI после последующей правки пользователя возвращает conflict.
- PDF общий display list, но полное совпадение с экраном для kerning/font fallback/rotation/wrap не подтверждено. Unsupported CJK glyphs — ошибка, не silent corrupt text. «Печать» открывает PDF системным просмотрщиком.
- Unknown OOXML пропускается с предупреждениями, не opaque roundtrip. Полный корпус ≥50 реальных файлов каждого Office типа, cargo-fuzz и внешняя Microsoft Office/LibreOffice приёмка не выполнены.
- Stale lock после аварии может открыть файл read-only; native file picker может задерживать выход до закрытия диалога.
- Нет Authenticode и выбранной лицензии проекта. Системные Windows fonts не включены в payload. Notices зависимостей включаются отдельно.
- Цели ≤10 MiB exe/≤5 MiB installer являются целями, не подтверждённым результатом.

Нельзя помечать v0.1.0 стабильным или удалять ограничения только потому, что установщик успешно собран.

## Финальные dist проверки
- cargo build --locked --profile dist -p trinedocs --bins: PASS; исходники после последних PNG/chart/UI исправлений.
- Final dist --smoke-test: native/DOCX/XLSX/PPTX/PDF/PNG PASS; реальный HTTP+stdio MCP smoke и native Home/Docs/Sheets/Slides screenshots PASS. Новые output directories, прежний success не переиспользовался.
- Внешние проверки: ZIP CRC и все XML parts smoke DOCX/XLSX/PPTX корректны; MuPDF все 3 PDF открыл без repair, Unicode/42/TrineDocs извлекаются. PNG 1920×1080 прочитан независимым Pillow, текст и фон просмотрены. Это не external Office fidelity acceptance.
- Визуальная проверка final UI: Sheets chart теперь полностью виден в отдельной прокручиваемой панели; Slides UI sample без пересечения шаблонных placeholders. PNG использует ту же Page, первый слайд 2×; поворот явно обозначен как потеря.
- PE imports обоих exe — только системные DLL Windows; поставляемых DLL нет. TrineDocs.exe 10 459 136 bytes (9.975 MiB), shim 2 423 808 bytes. Decimal цель 10 МБ превышена на459136 bytes; binary 10 MiB не превышена. Installer размер пока не измерен.
- PNG preflight проверяет отсутствие глифов, целостность/aggregate32MP изображений и geometry budget до context/tessellation; обе high находки второй волны закрыты повторным независимым чтением.

## Установщик: первая фактическая проверка
- NSIS LZMA solid unsigned preview собран:4 114 181 bytes, decimal5МБ gate выполнен.
- Install per-user /S в пути с пробелами, установленные files+HTTP/stdio MCP+native GUI smoke, uninstall: PASS. Office defaults сохранены; native defaults восстановлены; payload/uninstall registration удалены. APPDATA QA-профиль сохранён.
- Найден и исправлен выбор codepage NSIS: script компилируется явно /INPUTCHARSET UTF8, compiler warnings становятся errors (/WX). Следующая сборка и повторная установка проверят этот payload; эти строки не подменяют результаты второй проверки.

## Итог локальной приёмки preview installer
Повторный setup с явным UTF8 и /WX:4 112 593 bytes, SHA-256 `573a81bf5179cf745d70954b1237617e2c776c4b950e9953e9c86fd21b4e203b`.
Повторная установка в новый путь с пробелами: per-user install, files/MCP/UI, uninstall и restoration всех проверенных defaults PASS. Дополнительно duplicate install отклонён; при эксклюзивной блокировке main exe uninstall отклонён до удаления обоих exe/регистрации. Это подтверждает sharing-violation guard, не все возможные OS/crash/registry failure paths.
TrineDocs.exe SHA-256 `281d8da4a6eacfae56c5a2dc6854552e124b48c4833c034ee8b6429a40647391`; shim `b9d99663c196362f888595c12c4cf18da5e3bbd6e3336d9b265228ac308ec495`.
Local preview version0.1.0-preview.1; binary VERSIONINFO0.1.0.0. CI использует свой unique preview installer/tag version; binary resource version остаётся0.1.0.0. Local hashes не выдаются за hashes будущего CI артефакта. Remote CI/Release статус определяется GitHub workflow, до его выполнения это pending.
All-users/elevation, чистая Windows10, внешние Office corpus, full functional/performance acceptance не выполнены. Все эти ограничения сохраняются после успешного install smoke.
