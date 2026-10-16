# Большое ревью 2 — Slides и независимый cross-review оболочки

Дата: 2026-10-13. Автор: агент Slides второй волны. Ревью выполняется после исправлений первой волны и до NSIS. Повторно проверены slides/model.rs, editor.rs, present.rs, io.rs; независимо прочитаны app.rs, file_io.rs, settings.rs, native/render/PDF/MCP. Чужие исходники агент не редактировал.

## Статическое ревью и исправления Slides

| ID | Уровень | Сценарий и дефект | Исправление / регрессия |
|---|---|---|---|
| S-R2-01 | high | Shift resize верхней/нижней средней ручки перезаписывал изменённую высоту из неизменившейся ширины. Верхние углы меняли противоположный anchor. | resize_geometry учитывает ведущую ось, отношение сторон, противоположную сторону/центр. Tests side/corner + 8 ручек с большими deltas. |
| S-R2-02 | high | Вставка картинки вызывала picker и fs::read на UI; размер проверялся после неограниченного чтения. | Picker/read/decode validation вынесены в read-only worker. util::read_file bounded, bytes 32 МБ, dimensions 8192/32 млн до decode. Результат адресуется по исходному slide_id, вставка сохраняет aspect ratio. |
| S-R2-03 | high | Deck::validate не ограничивал общий text/image/table budget, размеры растров, IDs групп, next_id saturation и огромные chart floats. | Aggregate budgets, image header preflight, finite/bounded geometry, animation/transition checks, group IDs. Negative tests group/counter/image, aggregate table, f32::MAX chart. |
| S-R2-04 | medium | Дубликат слайда терял группы. | Сохраняются группы с новыми stable IDs; grouped_slide_duplicate_remaps_groups. |
| S-R2-05 | medium | Bring back обращал порядок нескольких объектов; Bring front использовал ID order вместо исходного слоя. | set_z_order_many сохраняет относительный порядок; regression front/back с выбранными IDs в другом порядке. |
| S-R2-06 | medium | TextShape::with_angle вращает вокруг начала текста, rect/PPTX xfrm — вокруг объекта. | Начало текста трансформируется относительно rect.center. API epaint 0.36.2 прочитан. Pixel comparison ещё не выполнен. |
| S-R2-07 | medium | Поворот image/table/chart/line менялся в свойствах без результата на canvas. | Неработающий control отключён и подписан. Rotation PDF остаётся gap. |
| S-R2-08 | high | slides::io native формат писал raw Deck, app ожидал общий Envelope. | Оба API делегируют native::encode/decode; тест совместимости обоих направлений. |
| S-R2-09 | high | customXml snapshot скрывал OOXML дефекты: notes строки сливались, title менялся на first text, a:br терялся. | Notes/cells paragraph_text, soft break, cSld.name priority. External test удаляет customXml и проверяет реальные XML parts. |
| S-R2-10 | medium | Автопереход без эффекта не попадал в PPTX. Arrow экспортировался как filled rightArrow вместо canvas line-arrow. | p:transition advTm без effect, arrow line geometry+a:tailEnd, importer end recognition. external_roundtrip_preserves_notes_title_line_arrow_auto. |
| S-R2-11 | medium | Отсутствующие/внешние картинки становились Image с пустыми bytes; unsupported media могли прервать import при deep validation. | Отсутствующие/неподдерживаемые картинки пропускаются с warning. PNG/JPEG dimensions limit возвращает safe error. Missing_picture regression. |
| S-R2-12 | high | PDF multiline table передавал newline как glyph и не рисовал grid; нулевой chart value получал высоту 1, labels/baseline расходились. | Shared text_lines для cell, отдельные строки, grid/fills, zero-height bar, baseline/center labels. Tests multiline_table_pdf_has_lines_and_grid, zero_chart_value_has_no_visible_bar. |
| S-R2-13 | medium | CRLF/tab от MCP/пасты могли отменить PDF export при glyph lookup. | Layout нормализует line endings и expands tabs; chart labels разбиты на строки. text_layout_normalizes_crlf_and_tabs. Root уведомлён об общей проблеме PDF. |
| S-R2-14 | medium | Reorder/deletion через MCP менял слайд показа по индексу. | Stable current/previous slide IDs; reorder сохраняет слайд, удаление safely завершает fullscreen. Empty/all-hidden deck не стартует. Tests. |
| S-R2-15 | medium | Selected/edit IDs оставались после external deletion. Arrow/Delete работали лишь при hovered canvas. | Stale IDs очищаются, клики фокусируют canvas, navigation работает по focus. |
| S-R2-16 | performance | Создание text Element через ..element.clone для каждой ячейки клонировало всю таблицу O(n²). | Text layout получает только нужные размер/формат/текст; nested table не копируется per-cell. |
| S-R2-17 | high | Текст 8 МБ из переносов или very narrow box создавал миллионы layout lines/Primitive. | Добавлен preflight budget до rendering: conservative per-object/cell 10k, aggregate 100k lines; CR/LF/tabs учтены, safe error без усечения. Regression excessive_layout_lines_are_rejected_without_rendering. |
| S-R2-18 | medium | stroke_width=0 трактуется PDF как hairline, egui как отсутствие контура; ellipse/line/arrow/chart baseline всё ещё emitted Line width0. | Эти Primitive::Line не создаются при нулевом stroke. Regression zero_outline_never_emits_pdf_hairlines проверяет все 6 типов shape/table/chart. |

## Независимый cross-review root

| ID | Уровень | Сценарий / найденное состояние | Передано root |
|---|---|---|---|
| S-R2-X01 | high | Exit → Save при нескольких dirty tabs сохранял только первую и терял window exit continuation. | Подтверждено повторным чтением: exit_after_save последовательно сохраняет dirty tabs; новые правки во время Save вызывают повторный Close prompt. |
| S-R2-X02 | high | SaveFormat cancel/error или enqueue failure оставляли close_after_save; поздний Save закрывал вкладку. | Подтверждено повторным чтением: cancel/error/scheduling rejection сбрасывают continuation и saving state. |
| S-R2-X03 | high | Save As из readonly получал собственный lock, но read_only оставался true. | Подтверждено повторным чтением Event::Saved: собственный новый lock сбрасывает read_only. |
| S-R2-X04 | medium | FileLock create_new + write_all disk-full оставлял незавершённый lock. Crash тоже сохраняет lock. | Cleanup write/sync failure подтверждён повторным чтением. Stale-lock после crash остаётся открытым. |
| S-R2-X05 | high | Ctrl+Tab/Ctrl+W или MCP focus change во время показа скрывают/закрывают Slides без Fullscreen(false). | Shell integration подтверждена: shortcuts guard, tick/ui stop при active tab change. Сам показ хранит stable slide ID. |
| S-R2-X06 | medium | MCP save обходил base_version/read_only preflight; rejected enqueue мог оставить reply до timeout. | Подтверждено повторным чтением: version/read_only и saving guard до enqueue; закрытые replies удаляются в tick. |
| S-R2-X07 | medium | Settings read без size/count bounds; recent/pinned persist лишь при exit/settings; exit save error ignored. | Повторное чтение: load limited 1 МБ, normalized scale/language/theme/counts; persist error on_exit ещё ignored. |
| S-R2-X08 | medium | disk_changed не очищался после подтверждённого reload; modal оставался. | Подтверждено повторным чтением: disk_changed очищается после reload. |
| S-R2-X09 | medium | Worker::Drop join ждёт открытый blocking native picker. | Открытый shutdown сценарий из первой волны; требуется проверка root. |

## Дополнительная проверка нового MCP

Повторно прочитаны register.rs/server.rs/shim.rs/integration.rs. Найден и передан root возможный TOML indexing panic на scalar/array mcp_servers; root добавил is_table guard до IndexMut. JSON root/servers проверяются через as_object. Preview/apply сравнивает исходные bytes перед atomic_write и создаёт .bak; чужие серверы сохраняются, existing trinedocs не перезаписывается. Чтение конфигов ограничено; регистрация находится в file worker и требует согласия в окне приложения.

Shim теперь читает mcp.json с пределом 8192 и поддерживает --client name; регистрация передаёт Codex/Claude Desktop/Cursor/VS Code labels. reqwest.no_proxy гарантирует, что loopback HTTP не уйдёт на configured proxy. Server требует Bearer, SDK Host/Origin validation и request cap 1 МБ; UI approval отдельно.

MCP integration smoke использует настоящий HTTP и stdio SDK, но pump напрямую dispatch без пользовательской approval UI. Поэтому успешный smoke доказывает transport/auth/session boundaries, а не end-to-end permission диалог/GUI cursor/16 мс frame target. Notifications/resources subscribe и потоковый видимый cursor не считаются реализованными.

## Динамические результаты

По сообщению root, cargo test после основной второй волны: 107 тестов app + 13 shell, всего 120 PASS. Агент самостоятельно не запускал Cargo/общий target. Этот запуск предшествовал последнему zero-outline regression и новому UI smoke; их свежая компиляция/запуск на момент записи ещё требуется. Результат 120 PASS не переименовывается в GUI/Office совместимость.

CustomXml roundtrip не принимается как доказательство OOXML. Новые тесты удаляют customXml и проверяют реальные parts, manifest-invalidation regression сохранён. Программные проверки model/egui не заменяют ручную GUI приёмку.

Microsoft PowerPoint/Open XML SDK validator, 50 внешних PPTX, pixel comparison экран/PDF, Windows DPI/IME, monitor 2 и performance FPS этим агентом не запускались.

## Критерии выпуска, ещё требующие внешней проверки

1. Финальный locked build/test/clippy -D warnings/fmt после последних zero-outline/UI smoke изменений.
2. Форматный smoke через общий Envelope и Office XML без customXml; независимый Office/Open XML validator и визуальный PDF renderer. Удаление customXml тестируется программно, но внешняя Office приёмка ещё не доказана.
3. Windows smoke launch, silent per-user installer/uninstaller, registry association audit без изменения default для Office, чистые Windows 10/11 и сохранность пользовательских документов/настроек.
4. Фактические SHA-256, размер exe/installer, явная пометка unsigned; отсутствие Qt/WebView/non-system DLL доказать PE dependencies.
5. Пока полный Office parity/real file corpus/FPS/IME/DPI/monitor 2 не доказаны, сборку обозначать предварительной. Stable v0.1.0 и «все функции Office» не принимать по этим результатам.

## Ограничения

Дополнительный форматный gap: table_xml сейчас не сохраняет uniform color/bold/italic/fill/stroke и импорт table не восстанавливает cell font size; customXml нативный snapshot маскирует эти потери. Это явная потеря при внешнем PPTX roundtrip, а не подтверждённый Office formatting parity.

Полный PowerPoint паритет не достигнут: mixed rich text, master placeholders, external nested groups, transparency/gradients/SmartArt, crop/flip, все charts, PPTX timing, мультимедиа, PNG/handouts/notes-print, monitor 2, accessibility/CJK/font fallback и 60 fps остаются неполными. Точный список в docs/parity-slides.md. Нативный формат сохраняет реализованную модель, OOXML требует честного предупреждения оболочки.

[Microsoft TailEnd](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.tailend?view=openxml-3.0.1) прочитан 2026-10-13. API egui/epaint проверены по pinned registry 0.36.2. Mapping line endpoint следует из schema и regression XML; PowerPoint COM не запускался.

## Дополнение: release tooling / NSIS до сборки установщика

Проверены read-only tools/build_release.ps1, tools/test_installer.ps1, .github/workflows/release.yml, installer/trinedocs.nsi, rust-toolchain.toml, .cargo/config.toml, crates/trinedocs/build.rs и CLI tools/spec_registry.py / audit_dependencies.py. Исходники release tooling агент не менял; находки направлены root.

По новому сообщению root финальный cargo test: 123 PASS, fmt/clippy green. Это включает поздние source fixes, но не означает install/UI/Office внешнюю приёмку. Root в этот момент собирает dist binaries.

| ID | Уровень | Finding / проверка | Состояние на момент чтения |
|---|---|---|---|
| S-R2-W01 | high | GitHub ref_name подставлялся прямо в single-quoted shell run. Git tag может содержать apostrophe/$/semicolon, поэтому возможно shell injection с contents:write token. | Передано root: только env values + quoted shell expansion, без raw expression в скрипте. |
| S-R2-W02 | high | Branch build повторно публиковал fixed v0.1.0-preview.1 --clobber; существующий tag продолжал указывать старый commit, assets могли быть новым кодом. | Требуется unique preview tag или отказ при несовпадении tag SHA с build SHA. |
| S-R2-W03 | medium | Version build script hardcoded preview.1, EXE resources также preview.1, но workflow принимал произвольные v* tags. | Version/tag/source metadata consistency требует проверки и строгой допустимой версии при передаче в NSIS macro. |
| S-R2-W04 | high | Get-Command makensis выбирал arbitrary PATH NSIS и обходил portable ZIP hash pin. | Default должен использовать verified portable, explicit override отдельно. |
| S-R2-W05 | medium | Download SourceForge обрабатывал только HTML meta-refresh. Если Invoke-WebRequest после redirect сразу возвращает ZIP, script ошибочно abort. | Рекомендован direct official artifact URL + обязательный SHA check; HTML fallback только при необходимости. |
| S-R2-W06 | medium | CI actions использовали mutable @v4/@v5/@stable refs. Последний dtolnay stable commit опубликован 2026-10-09, моложе 14 дней. | Переданы primary GitHub SHA pins; dtolnay stable commit 2026-09-12. Rust toolchain itself =1.98.1 закреплён правильно. |
| S-R2-W07 | medium | Update/uninstall могут частично изменить файлы, если main unlocked, а shim locked: main уже заменён/удалён перед ошибкой следующего файла. | Нужен preflight/two-file rollback или явно documented repair/retry limitation. |
| S-R2-W08 | medium | Silent error MessageBox может ждать ввод бесконечно; registry WriteReg и WriteUninstaller errors не проверяются. | Требуется silent-safe failure/exitcode и registration/uninstaller error handling. |
| S-R2-W09 | medium | Start-Process app smoke args unquoted $path ломаются при spaces в checkout/output. Wait не имеет timeout. | Передано root: quoted app argument, bounded waits/cleanup. NSIS /D и _?= — особый последний rest-of-command argument. |
| S-R2-W10 | medium | Test preflight запрещал existing UNKEY, но не existing TrineDocs.* ProgIDs / Applications key / OpenWith values. Ручная регистрация без UNKEY могла быть удалена тестом. | Fail-closed collision checks или полноценное сохранение/восстановление registry. |
| S-R2-W11 | verification | tools/spec_registry.py CLI действительно принимает check. | Фактически выполнен python -B tools/spec_registry.py check: 442 REQ, 350 PAR, exit 0. Acceptance explicitly incomplete. |
| S-R2-W12 | verification | Artifact LCA для dist/ + target/installer-smoke/ — checkout root; download ожидает artifact/dist. | Пути upload/download согласованы при успешном install smoke. Publish зависит от успешного windows job; PR исключены. |
| S-R2-W13 | verification | NSIS 3.13 metadata/hash. | Primary catalogue confirmed stable, releasedAt 2026-09-27, ZIP SHA256 ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8 совпадает со скриптом. Не моложе 14 дней. |
| S-R2-W14 | limitation | Размер только печатается, не является gate. | Только реальные измерения могут подтвердить <=10 МБ main / <=5 МБ setup; превышение честно фиксируется. |

NSIS static positives: RequestExecutionLevel user позволяет per-user без UAC; Windows x64/Win10 checks и registry view64; all-users только при административных правах; Office default values не перехватываются, OpenWith добавляется отдельно; native defaults сохраняются/восстанавливаются при uninstall. Удаляются конкретные payload filenames, нет RMDir /r и нет удаления APPDATA/docs. Full machine scope и locked-process recovery этим агентом не исполнялись.

Primary references: [NSIS release catalogue](https://nsis-dev.github.io/release-data/versions.json), [NSIS execution-level reference](https://nsis.sourceforge.io/Docs/Chapter4.html), [dtolnay stable commit 2026-09-12](https://github.com/dtolnay/rust-toolchain/commit/02cb101ec7c40f2c49e1d9714d64511d8e1b74de). Catalogue и exact action refs проверены через official HTTPS JSON API; installer ещё не запускался данным агентом. Findings, сообщённые root, требуют повторного чтения после его fixes, прежде чем считать закрытыми.

## Повторная проверка release fixes и реализация PNG

Повторное чтение 2026-10-13 подтверждает закрытие S-R2-W01/W02/W04/W05/W06/W09/W10: GitHub ref metadata передаётся через env, shell использует quoted expansions; unique run/attempt preview tags и запрет overwrite; default NSIS только из проверенного portable ZIP; ZIP response поддерживается напрямую, HTML refresh — fallback; actions pinned exact SHA, Rust1.98.1 явно указан; тест использует ProcessStartInfo.ArgumentList для app, NSIS last-argument parsing отдельно, timeout60s, новый Output, запрет existing own registry/shortcut collisions.

Проверен read-only Git ref сценарий: git check-ref-format для имени с apostrophe/semicolon/# вернул exit0. Payload не исполнялся. Это подтверждает, что прежняя raw interpolation действительно была опасна; после env fix ref не становится shell-кодом.

S-R2-W07: preview upgrades теперь явно требуют uninstall и отказ на existing UNKEY/payload files. Uninstall проверяет exclusive writable opens обоих binaries перед удалением. Это снижает риск обычного locked-process partial deletion; race между preflight и delete полностью не исключён. S-R2-W08: error MessageBox имеют /SD IDOK, WriteUninstaller failure guarded; failures всех registry writes и transactional rollback ещё не доказаны. Native association backup теперь отдельно хранит presence default value, поэтому существующее пустое значение не смешивается с отсутствующим.

Installer Version берётся из CI tag/run и validated до NSIS. EXE VERSIONINFO пока имеет строку preview.1: installer/tag version совпадают, но EXE build metadata consistency не считается закрытой автоматически. Цели размера и Windows10/all-users/locked-process scenarios ещё требуют фактического выполнения.

Найдена и исправлена активная PNG команда без implementation. Новый png.rs использует fonts::install, Context::run_ui, shared Page paint, egui tessellation, font/image textures, bilinear UV sampling, premultiplied alpha и top-left triangle coverage. Geometry/clipping/indexes/callbacks проверяются; arbitrary shader callbacks явно отклоняются. Native PNG экспортирует первый слайд при масштабе2, меню root подписал «PNG · первый слайд». Нет active slide в Deck, и это не выдаётся за экспорт текущего слайда.

До tessellation действуют bounds: dimension8192/32MP, aggregate decoded image pixels32M, byte/texture/geometry/glyph budgets, полноценная image decode validation. Raster work ограничен200M candidate pixels. Три meaningful regressions проверяют encoded dimensions, реальные glyph/shape pixels, white background, embedded PNG alpha composition и invalid dimensions.

По сообщению root после PNG/API/preflight fixes: 112 app + 14 shell = 126 PASS, fmt/clippy PASS. Агент не запускал общий Cargo target самостоятельно. Final dist повторно компилируется; release binary smoke/NSIS ещё ожидаются.

PNG соответствует общему Page display list, но этот список пока не представляет rotation. Поэтому rotation/crop/group/style fidelity и pixel-perfect соответствие editor canvas не объявлены достигнутыми. Handouts/notes-print/second monitor/Office full corpus остаются открытыми. Старое ограничение «PNG отсутствует» заменено конкретным first-slide CPU export; внешнее visual QA PNG ещё нужно.

## Независимый повторный review: стабильные ID и история операций

После успешного первого preview CI root обнаружил high дефект: Undo/Redo восстанавливал next_id из старого snapshot, а новая ветка правок могла повторно выдать удалённый ID. Ранее опубликованный preview1.1 не принимается как итоговая ссылка для пользователя; root планирует новую preview2 после исправления и повторных проверок.

Read-only повторно проверены ops.rs, Slides model/editor и MCP tools. Fix подготовлен другим агентом, этот агент исходники и target не менял.

| ID | Уровень | Результат независимого чтения | Статус |
|---|---|---|---|
| S-R2-O01 | high / fixed source | Replace, Undo, Redo и AI rollback сохраняют max(next_id incoming, next_id current). Успешная новая ветка не снижает floor, удалённый ID не выдаётся typed UI/MCP allocator повторно. | Source verified; новый финальный Cargo/CI ещё ожидается. |
| S-R2-O02 | verification | Slides slide/element/group/paste/duplicate используют общий Deck.allocate_id. Deck.validate учитывает group IDs в max_id, запрещает collisions и next_id == u64::MAX. Поэтому floor включает также ID групп. | Static verified; отдельная group+Undo end-to-end регрессия этим агентом не запускалась. |
| S-R2-O03 | verification | Stale-version, kind mismatch, model validation и version overflow проверяются до commit. Undo/Redo сначала готовят и валидируют restored, затем pop history. Новый успешный Replace очищает redo branch; Undo/Redo разрывают typing group через last_edit. | Static verified; при этих обычных error paths модель/история не меняются. |
| S-R2-O04 | verification | reject_ai нормализует только expected_after.next_id, затем сравнивает полную модель. Поздние content edits продолжают конфликтовать; возвращённый snapshot получает сохранённый floor. | Static verified; нормализация не игнорирует текст, фигуры, таблицы, формат или листы. |
| S-R2-O05 | high / found outside Op | app Event::Opened reload напрямую заменял модель при том же TabID. Повторный external import назначает IDs заново, поэтому одного max(next_id) для reload недостаточно. AI/history и pending old-ID state должны сбрасываться, namespace документа меняться. | Передано root: новый TabID при explicit reload. На момент первого чтения не исправлено. |
| S-R2-O06 | high / found outside Op | Любой повторный Open same path при disk_changed интерпретировался как согласие reload. Во время фонового чтения UI/MCP могли внести новые правки, которые completion перезаписывал без проверки версии. | Передано root: отдельный explicit reload intent с tabID/base version и отказ при новых правках. |
| S-R2-O07 | contract gap | replace_document_model может вручную принести newly-appearing retired ID ниже allocator floor; без base_version stale whole model может перетереть свежие правки. High-water fix typed allocation сам по себе этого не исключает. | Передано root: требовать base_version и проверять новые IDs для whole-model tool либо явно обозначить ограничение preview. Undo/Redo восстановления истории следует сохранить. |

Прочитаны новые meaningful regression cases другого агента: Undo→новый объект во всех трёх моделях; Redo и AI rollback с увеличенным floor; настоящий последующий content conflict; старый model snapshot не снижает floor; readonly/version overflow оставляют модель и stacks; central kind mismatch атомарен. Тестовые определения соответствуют найденному багу, а не повторяют только helper implementation. Общий cargo target этот агент не запускал: результаты выполнения должен записать root после последнего source изменения.

Модели, allocator counters и история остаются источником стабильности в пределах одной открытой document namespace. Повторное открытие с новой TabID и explicit reload generation boundary нужны отдельно от Undo/Redo. Полный Office parity и прежние форматные/UI/performance ограничения этим fix не закрываются.

## Source closures перед повторной сборкой preview

S-R2-O05/O06 закрыты независимым чтением последних app.rs/file_io.rs: Job::Reload несёт tab_id/base_version/path; reloading map проверяется на completion; обычный Open активирует существующую вкладку. prepare_reload проверяет существование вкладки, версию, путь, новый checked TabID и валидность модели до commit. Успешный reload заменяет namespace и сбрасывает history/AI; retire_state очищает old snapshots/recovery/replies/pending/agent state. Собственный lock переносится лишь после успешного preflight. Поздняя правка User или Agent отклоняет completion и сохраняет текущую модель. Save/Reload не перекрываются; Close/Remove не снимают lock, пока Save writer работает.

S-R2-O07 закрыт source fix этого агента: replace_document_model требует document_id/base_version/model в schema, missing/stale version отклоняется до parsing/commit. get_document_info возвращает next_id, описание tool объясняет контракт. После полного model.validate lifecycle guard сравнивает (ID, structural role) с текущими живыми объектами: отсутствующая роль/ID ниже current.next_id отклоняется; сохранённые живые ID и новые ID >= floor допустимы. Это также запрещает переприсвоить live paragraph ID новому table/image/group. Проверены Docs main/nested paragraphs, tables/cells/images; Sheets sheets/cells/charts; Slides slides/elements/groups.

Дополнительно найден S-R2-O08 (medium): slides_update_element принимал произвольный новый group ID, включая retired ID ниже floor. Lifecycle guard теперь применяется ко всем validated MCP candidate replacements; typed allocators естественно используют fresh IDs. Undo/Redo возвращаются отдельным путём до guard; AI rollback использует ops/history и остаётся законным восстановлением. slides_add_shape/text/image не парсят arbitrary group/element patch и создают group=None, а update защищён общим guard.

Пять новых regression functions: required schema/missing-version atomicity; 11 structural roles с retired/fresh ID и apply/dry_run/stale version; фактический Undo удалённого Slides element с raw injection rejection и разрешённым typed Redo; repurpose live paragraph ID как table; удаление живой Slides group через два typed calls, atomic rejection retired group injection в apply/dry_run, legitimate Undo restore и присоединение к живой группе. Другой агент независимо прочитал первый raw contract fix и подтвердил согласованность; финальное выполнение tests/fmt/clippy root ещё требуется.

S-R2-O09 (medium) найден при независимом reload review: первый SaveAs сохранял raw dialog path, обычный Open/Reload canonicalized path; Windows extended prefix ломал exact equality и открывал duplicate tab либо отклонял корректный reload. Sheets агент нормализовал Event::Saved.path после успешной atomic_write; повторное чтение это подтвердило. Open/save теперь хранят одну canonical namespace. При этом Explorer должен получать обычный shell path; canonical path нельзя считать автоматически поддержанным оболочкой. Предупреждение о newly introduced canonical-before-print передано Sheets для исправления до freeze. Это вывод из [Microsoft Win32 namespaces](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file), а не выполненный GUI print test.

Source claims этого агента освобождены после hash-guarded edits. Общий Cargo/target/installer агент не запускал. Предыдущий успешный CI не считается выполнением этих новых regression tests; окончательные source SHA, dist smoke и installer/CI результаты требуется привязать ко второму preview.

Финальное повторное чтение file_io.rs подтверждает закрытие shell path риска S-R2-O09: Explorer spawn выполняется с исходным path, canonicalize выполняется после него и до Event::Saved. Storage identity остаётся canonical. Настоящий PDF viewer/print UI этим агентом не запускался. После общей fmt source повторно прочитан: lifecycle guard вызывается для всех MCP candidates, required base_version raw tool сохранён, regression tests присутствуют. Новых HIGH в последних ops/MCP/reload source после этих closures не найдено. Финальные Cargo/CI результаты и artifact source SHA сообщает root отдельно; старый preview1.1 не выдаётся как исправленная сборка.
