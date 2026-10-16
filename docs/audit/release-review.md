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

Статическая policy допускает Windows 10/11 x64 build10240+; совместимость реальной Win10 ещё не проверена. Стенд текущего GUI smoke: Windows 11 Pro 10.0.26300 x64, Intel Core i5-13400F (16 logical cores), NVIDIA RTX 3070 driver32.0.16.1088. Ниже зафиксированы выполненные локальные проверки и ограничения стенда.

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
- Lock-маркер защищает сохранение оригинала, но не отключает правку и сохранение копии. Снятие оставшегося marker требует явного подтверждения; живой Windows handle блокирует takeover. Во время file picker закрытие окна отменяется до завершения диалога.
- Нет Authenticode и выбранной лицензии проекта. Системные Windows fonts не включены в payload. Notices зависимостей включаются отдельно.
- Размеры прежнего проверенного payload приведены ниже. Новые исправления требуют новой сборки и измерения.

Нельзя помечать v0.1.0 стабильным или удалять ограничения только потому, что установщик успешно собран.

## Финальные dist проверки
- cargo build --locked --profile dist -p trinedocs --bins: PASS; исходники после последних PNG/chart/UI исправлений.
- Final dist --smoke-test: native/DOCX/XLSX/PPTX/PDF/PNG PASS; реальный HTTP+stdio MCP smoke и native Home/Docs/Sheets/Slides screenshots PASS. Новые output directories, прежний success не переиспользовался.
- Внешние проверки: ZIP CRC и все XML parts smoke DOCX/XLSX/PPTX корректны; MuPDF все 3 PDF открыл без repair, Unicode/42/TrineDocs извлекаются. PNG 1920×1080 прочитан независимым Pillow, текст и фон просмотрены. Это не external Office fidelity acceptance.
- Визуальная проверка final UI: Sheets chart теперь полностью виден в отдельной прокручиваемой панели; Slides UI sample без пересечения шаблонных placeholders. PNG использует ту же Page, первый слайд 2×; поворот явно обозначен как потеря.
- PE imports обоих exe — только системные DLL Windows; поставляемых DLL нет. TrineDocs.exe 10 459 136 bytes (9.975 MiB), shim 2 423 808 bytes. Decimal цель 10 МБ превышена на459136 bytes; binary 10 MiB не превышена. Размеры двух последующих установщиков приведены ниже.
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

## Дополнительная проверка после первой публикации
CI исходного commit `8cd07c26c7d2050b1d2c5a79be7c3fb8c826730f` прошёл: core/spec run38351587457 и release run38351587504, включая Windows install/files/MCP/uninstall. Опубликован preview.1.1, но затем обнаружены дополнительные defects; он не считается итоговой сборкой этой проверки.

- Undo/Redo/AI rollback сохраняют монотонный next_id во всех трёх редакторах; тип документа нельзя менять через Replace. Новые regressions проверяют реальные IDs, branching, overflow и атомарность.
- Explicit Reload отделён от обычного Open, completion связан с исходной вкладкой/path/version. Новые User/AI правки отменяют reload; успешная перезагрузка создаёт новую document namespace и очищает старую историю/AI/recovery state. Close во время записи не освобождает FileLock раньше worker completion.
- replace_document_model требует base_version; newly appearing retired IDs и смена структурной роли старого ID отклоняются. get_document_info сообщает allocator next_id. Undo/Redo намеренно могут восстанавливать прежние живые объекты из истории.
- Docs текстовые MCP операции передают точные позиционные deltas для курсора/выделения; inverse deltas используются в Undo/AI rollback. Full-model/неизвестная правка по-прежнему имеет ограниченный fallback, не полноценный CRDT.

Эти строки описывают source fixes, не заменяют повторный запуск fmt/clippy/tests, dist smoke и install/uninstall. Итоговые результаты добавляются после выполнения. Обе предыдущие локальные установки выполнялись из обычной, не административной Windows учётной записи; они не доказывают приёмку чистой Windows10/11 VM.

## Повторная проверка исправлений идентичности и курсора
- cargo test --workspace --locked:131 app +14 shell =145 passed,0 failed. Первый запуск обнаружил только headless test harness: egui0.36 требует обработать или явно clear TexturesDelta; три тестовых outputs исправлены, production rendering не менялся. Повторный полный запуск прошёл.
- cargo clippy --workspace --all-targets --locked -- -D warnings и cargo fmt --all -- --check повторно PASS после последнего изменения source.
- Spec registry442 REQ/350 PAR,4 Python tests и historic assets checker PASS. Acceptance Office остаётся incomplete.
- Последние source fixes взаимно проверены тремя агентами; закрыты дополнительные HIGH findings ID reuse/reload/cursor/raw-model/typed-group. После SaveAs путь canonicalize для хранения; Explorer печати получает исходный путь до canonicalization.
- Final dist и новое install/uninstall выполняются после этих результатов. Ранее опубликованный preview.1.1 не выбран для итоговой ссылки.

## Новый dist после дополнительных исправлений
cargo build --locked --profile dist -p trinedocs --bins PASS. Новый target/qa-final3: files(native/DOCX/XLSX/PPTX/PDF/PNG), реальный HTTP+stdio MCP и native UI smoke PASS. Документы/таблицы/слайды визуально просмотрены. Независимые MuPDF/Pillow и XML/ZIP проверки повторены:3 PDF без xref repair, ожидаемый Unicode/42/TrineDocs извлекаются,4 Type0 font resources; Office parts9/9/16 корректны; PNG1920×1080.
TrineDocs.exe10 488 320 bytes: превышает decimal10МБ на488320 bytes и binary10MiB на2560 bytes. Цель размера main exe не выполнена; shim2 423 808 bytes. PE imports обоих exe только системные DLL Windows, DLL в payload не добавлены.
SHA-256 main `56ee11c8ebc4fa7328a2b6f4c70c8fb8f7540dbb64c493605a266a8693453557`, shim `91bf83388c71e8f89aac4ffbdfdfac970ea33bc4bd1bc59490a68155000f9757`. Новые install/CI hashes будут другими и не выводятся из этих локальных результатов.
