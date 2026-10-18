# docs/MCP.md — MCP-сервер TrineDocs

MCP предоставляет семантическое чтение и правку моделей через ops → UI-поток. CRDT/сервер совместной работы отложены владельцем ADR-0025. Реализованные инструменты и пробелы в docs/audit/release-review.md. <!-- REQ-M00-001 -->

Crate `mcp` на **rmcp**, официальном Rust SDK. Версия протокола — актуальная стабильная при реализации; на 2026-10-12 официальная `/specification/latest` указывает **2026-07-28** (FACT-006). Утверждение «сейчас 2025-11-25» исправлено (CHANGE-004). Поддержка 2026-07-28 конкретным rmcp и клиентами ещё не подтверждена; breaking changes требуют спайка 4.6 и ADR-0010. Предыдущие версии поддерживаются в пределах возможностей закреплённого rmcp. Непроверенные возможности SDK/клиентов не объявляются рабочими. <!-- REQ-M00-002 -->

## 1. Запуск и транспорты

### Автозапуск

- Сервер стартует с приложением, по умолчанию вкл.; фоновая tokio-задача не задерживает окно. <!-- REQ-M01-001 -->
- «Работать в фоне»: Windows tray / macOS menu bar; последнее окно закрывается, MCP остаётся. <!-- REQ-M01-002 -->
- «При входе»: Windows MSIX Startup task / current-user Run; macOS `SMAppService`; без окна. <!-- REQ-M01-003 -->
- Если приложения нет, шим запускает `--background`, ждёт готовности до 10 с. <!-- REQ-M01-004 -->

### stdio через `trinedocs-mcp`

- Поставляется маленький Rust-бинарник; проксирует JSON-RPC в приложение, сам документы не редактирует. <!-- REQ-M01-005 -->
Windows stdio-шим проксирует запросы на loopback Streamable HTTP, порт/токен в APPDATA/TrineDocs/mcp.json (ADR-0025). <!-- REQ-M01-006 -->
- macOS unix socket в `~/Library/Application Support/TrineDocs/`, 0600. <!-- REQ-M01-007 -->
При отсутствии приложения шим запускает TrineDocs --background и ждёт loopback до 10 секунд. <!-- REQ-M01-008 -->
- IPC: same OS user SID/uid + одноразовый токен в owner-only файле. Конкретная handshake/replay-защита — threat model. <!-- REQ-M01-009 -->

### Streamable HTTP

- Только `127.0.0.1` / `::1`; настройка порта, следующий свободный при занятости, фактический порт виден UI. <!-- REQ-M01-010 -->
Bearer-токен CSPRNG при первом запуске, файл конфигурации в пользовательском APPDATA. Keychain/ротация требуют отдельной реализации; не объявлять их выполненными. <!-- REQ-M01-011 -->
- Проверка Origin **каждого** запроса; чужой browser Origin отклоняется. Отдельно определить разрешённые Origin, отсутствие Origin у native clients и Host validation; ADR-0010. <!-- REQ-M01-012 -->
Streamable HTTP запускается по умолчанию в отдельном потоке, переключатель MCP отзывает клиентский доступ (план владельца/ADR-0025). <!-- REQ-M01-013 -->

### In-process

Встроенный чат — тот же сервер, in-memory transport, одна реализация инструментов. <!-- REQ-M01-014 -->

## 2. Подключение внешних клиентов в один клик

- MCP → Подключения: обнаруженные Claude Desktop, Codex CLI, Cursor, VS Code, Windsurf и др., «Подключить». <!-- REQ-M02-001 -->
- Согласие → дифф → бэкап → дописать только запись `trinedocs` с stdio-командой абсолютного пути к шиму. «Отключить» удаляет только принадлежащую приложению запись. <!-- REQ-M02-002 -->
- **Кандидатные пути из исходного задания; не использовать до сверки официальной документации:** <!-- REQ-M02-003 -->
  - Claude Desktop: `~/Library/Application Support/Claude/claude_desktop_config.json` / `%APPDATA%\Claude\claude_desktop_config.json`, `mcpServers`. <!-- REQ-M02-004 -->
  - Codex CLI: `~/.codex/config.toml`, `[mcp_servers.trinedocs]`. <!-- REQ-M02-005 -->
  - Cursor: `~/.cursor/mcp.json`. <!-- REQ-M02-006 -->
  - VS Code/Windsurf: найти актуальные пути/форматы, FACT-REGISTRY. <!-- REQ-M02-007 -->
- Копировать JSON-конфиг / команду для иных клиентов. <!-- REQ-M02-008 -->
- Какие клиенты подключены, последний вызов, «Отключить сессию». <!-- REQ-M02-009 -->

## 3. Права и подтверждения

- Клиент: read-only / edit / full (edit + экспорт, PDF, create/delete, Google). Новый внешний и встроенный чат по умолчанию edit. <!-- REQ-M03-001 -->
- Первый новый клиент: имя, запрошенный доступ, Разрешить / Один раз / Запретить. <!-- REQ-M03-002 -->
- Elicitation либо локальный диалог: delete document/sheet/slide, >N абзацев/ячеек (настройка), экспорт за пределы приложения, overwrite disk, Google upload. <!-- REQ-M03-003 -->
- По умолчанию только открытые и Недавние. Arbitrary path только явный opt-in и allowlisted folders, canonical path/reparse checks. <!-- REQ-M03-004 -->
- Аудит каждый вызов: клиент/tool/doc/time/result, переход/undo в «Активность MCP». <!-- REQ-M03-005 -->
- Rate limit, response size, pagination, timeout. <!-- REQ-M03-006 -->
- Документ — недоверенные данные; text в явно обозначенных полях, не команды. Это явно сказано в tool descriptions. <!-- REQ-M03-007 -->
- Наследование доступа по токену, сессии и текущей роли; отзыв должен прекращать уже начатую незафиксированную правку (ADR-0010). <!-- REQ-M03-008 -->

## 4. Модель адресации

- Stable IDs: `doc_`, `blk_`, `tbl_`, `img_`, `sht_`, `sld_`, `shp_`, `cmt_`, `ver_`. <!-- REQ-M04-001 -->
Текстовые offset считают Unicode scalar values, не байты UTF-8. Grapheme-safe навигация остаётся отдельным требованием и до реализации отмечается gap. <!-- REQ-M04-002 -->
- Sheets A1 (`Лист1!B2:D10`, Cyrillic names) или `{ sheet_id, row_id, col_id }`. <!-- REQ-M04-003 -->
- Slides/shapes по ID; порядок ответов 0..N. <!-- REQ-M04-004 -->
- Read содержит компактный CRDT version vector. Optional `base_version`: если затронутые объекты менялись → `conflict` + текущий фрагмент; без него правка текущего состояния со слиянием CRDT. Границы «затронутого» должны быть формализованы. <!-- REQ-M04-005 -->
- pt; colors `#RRGGBB` / theme name `accent1`; font family name. <!-- REQ-M04-006 -->

## 5. Общие свойства инструментов

- `inputSchema`, `outputSchema`, честные `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`. <!-- REQ-M05-001 -->
- Изменение: обязательный `document_id`, optional `base_version`; `mode` apply(default)/suggest/dry_run; `label` undo/AI. <!-- REQ-M05-002 -->
- Ответ `{ applied, version, changed_ids[], diff_summary, undo_group_id }`. <!-- REQ-M05-003 -->
- Для create/open (ещё нет открытого document_id) и app/asset операций схема адресации уточняется ADR-0010, не придумывать фиктивный doc ID. <!-- REQ-M05-004 -->
- Длительные export/LO/Google/bulk: tasks при поддержке клиента, иначе progress; cancellation. В 2026-07-28 Tasks — opt-in extension, явно согласуется поддержка обеих сторон; наличие инструмента в rmcp не доказывает поддержку клиентом. <!-- REQ-M05-005 -->
- Structured errors `not_found`, `conflict`, `permission_denied`, `needs_confirmation`, `invalid_range`, `unsupported_feature`, `too_large`, `timeout`, `internal`, сообщение/дальнейшие действия. <!-- REQ-M05-006 -->

## 6. Инструменты

### 6.1 Приложение и документы

| Инструмент | Поведение | RO |
|---|---|---|
|<!-- REQ-M06-001 --> `app_status` | версия, открытые/активный документ, selection, права клиента | ✓ |
|<!-- REQ-M06-002 --> `list_documents` | open/recent/pinned/cloud, тип/query/date, pagination | ✓ |
|<!-- REQ-M06-003 --> `search_documents` | FTS библиотеки | ✓ |
|<!-- REQ-M06-004 --> `open_document` | library ID / разрешённый path / link, показать пользователю | — |
|<!-- REQ-M06-005 --> `create_document` | Docs/Sheets/Slides, template, Markdown/table/deck seed | — |
|<!-- REQ-M06-006 --> `close_document` | с save/без, confirmation | — |
|<!-- REQ-M06-007 --> `save_document` | текущий файл | — |
|<!-- REQ-M06-008 --> `save_document_as` | path/format/loss report, confirmation | — |
|<!-- REQ-M06-009 --> `get_document_info` | properties/stats/format/path/author/presence/sync | ✓ |
|<!-- REQ-M06-010 --> `set_document_properties` | title/author/tags | — |
|<!-- REQ-M06-011 --> `focus` | scroll/object/selection показать пользователю | — |
|<!-- REQ-M06-012 --> `get_selection` | текущая selection пользователя | ✓ |
|<!-- REQ-M06-013 --> `set_selection` | установить selection | — |

### 6.2 Чтение

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-014 --> `get_outline` | Docs headings/blocks IDs; Sheets ranges/tables/charts/pivots; Slides titles/shapes |
|<!-- REQ-M06-015 --> `read_content` | range/objects: markdown ID anchors / full formatted json / text; block pagination |
|<!-- REQ-M06-016 --> `find` | text/regex/format/value/formula, positions/context |
|<!-- REQ-M06-017 --> `render_preview` | PNG page/range/slide, ограниченное разрешение |

### 6.3 Docs

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-018 --> `insert_content` | position + Markdown/JSON paragraphs/lists/tables/asset images/links, style mapping |
|<!-- REQ-M06-019 --> `replace_range` | replacement, preserve surrounding format/explicit format |
|<!-- REQ-M06-020 --> `delete_range` | range/blocks |
|<!-- REQ-M06-021 --> `move_blocks` | reorder blocks |
|<!-- REQ-M06-022 --> `format_text` | bold/italic/underline/strike/font/size/color/highlight/super/sub/case/clear |
|<!-- REQ-M06-023 --> `set_paragraph_format` | style/align/indents/spacing/tabs/keep/breaks |
|<!-- REQ-M06-024 --> `apply_style` / `define_style` | named style apply/create/update |
|<!-- REQ-M06-025 --> `set_list` | type/level/restart |
|<!-- REQ-M06-026 --> `insert_table` / `edit_table` | rows/cols/merge/borders/fill/width/header |
|<!-- REQ-M06-027 --> `insert_image` | asset/allowed file/generated; size/wrap/caption |
|<!-- REQ-M06-028 --> `insert_break` | page/section/column |
|<!-- REQ-M06-029 --> `set_page_setup` | paper/orientation/margins/columns/headers/page numbering |
|<!-- REQ-M06-030 --> `insert_field` | TOC/page/date/crossref/footnote |
|<!-- REQ-M06-031 --> `update_fields` | TOC/fields |
|<!-- REQ-M06-032 --> `replace_all` | whole-document replacement, count preview |

### 6.4 Sheets

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-033 --> `read_cells` | values/formulas/formats/display text; values-only |
|<!-- REQ-M06-034 --> `write_cells` | atomic 2D array into range, explicit/inferred types |
|<!-- REQ-M06-035 --> `set_formula` | cell/range relative fill, syntax validation before apply |
|<!-- REQ-M06-036 --> `explain_formula` | dependencies/intermediate values, RO |
|<!-- REQ-M06-037 --> `format_cells` | number/font/color/fill/borders/align/wrap/rotation |
|<!-- REQ-M06-038 --> `merge_cells` / `unmerge_cells` | merge |
|<!-- REQ-M06-039 --> `insert_rows` / `insert_cols` / `delete_rows` / `delete_cols` / `resize` / `hide` | structure |
|<!-- REQ-M06-040 --> `sort_range` / `filter_range` | multi-key sort/autofilter |
|<!-- REQ-M06-041 --> `freeze_panes` | freeze |
|<!-- REQ-M06-042 --> `add_sheet` / `rename_sheet` / `move_sheet` / `delete_sheet` | sheet operations |
|<!-- REQ-M06-043 --> `create_chart` / `update_chart` | type/data/axes/title/legend/position |
|<!-- REQ-M06-044 --> `create_pivot` / `update_pivot` | source/rows/columns/values/filters |
|<!-- REQ-M06-045 --> `conditional_format` | rules |
|<!-- REQ-M06-046 --> `data_validation` | validation |
|<!-- REQ-M06-047 --> `define_name` | named ranges |
|<!-- REQ-M06-048 --> `recalculate` | sheet/book |
|<!-- REQ-M06-049 --> `import_csv_to_sheet` | string/asset CSV, parse parameters |

### 6.5 Slides

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-050 --> `list_slides` | ID/index/layout/title/hidden/preview, RO |
|<!-- REQ-M06-051 --> `add_slide` / `duplicate_slide` / `delete_slide` / `reorder_slides` / `hide_slide` | slide operations |
|<!-- REQ-M06-052 --> `set_layout` / `apply_theme` | layout/theme |
|<!-- REQ-M06-053 --> `add_shape` / `update_shape` / `delete_shape` | text/shape/image/table/chart; pos/size/rotation/fill/stroke/formatted text |
|<!-- REQ-M06-054 --> `align_shapes` / `group_shapes` / `set_z_order` | composition |
|<!-- REQ-M06-055 --> `set_notes` | speaker notes |
|<!-- REQ-M06-056 --> `add_animation` / `update_animation` / `remove_animation` | effect/trigger/delay/duration/order |
|<!-- REQ-M06-057 --> `set_transition` | type/duration/auto advance |
|<!-- REQ-M06-058 --> `generate_deck` | structure/theme/titles/bullets/notes, batch task |

### 6.6 Комментарии, предложения, версии

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-059 --> `list_comments` / `add_comment` / `reply_comment` / `resolve_comment` | comments/threads |
|<!-- REQ-M06-060 --> `list_suggestions` | track changes/AI suggest, RO |
|<!-- REQ-M06-061 --> `accept_suggestions` / `reject_suggestions` | by IDs/all; edit + user confirmation для чужих |
|<!-- REQ-M06-062 --> `list_versions` / `create_version` | history/named snapshot |
|<!-- REQ-M06-063 --> `diff_versions` | diff, RO |
|<!-- REQ-M06-064 --> `restore_version` | restore, confirmation |

### 6.7 Экспорт, печать, облако

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-065 --> `export_document` | все AGENTS §8 formats/parameters; asset ID/allowed path, losses, task |
|<!-- REQ-M06-066 --> `print_to_pdf` | print range/handouts/area/scale |
|<!-- REQ-M06-067 --> `print_document` | только через user confirmation dialog |
|<!-- REQ-M06-068 --> `google_export` | connected account + confirmation; Google/Office, returns URL |
|<!-- REQ-M06-069 --> `google_import` | accessible Drive file |

### 6.8 Ассеты

| Инструмент | Поведение |
|---|---|
|<!-- REQ-M06-070 --> `upload_asset` | base64 limited image/file → asset ID |
|<!-- REQ-M06-071 --> `get_asset` | preview/export asset |

### 6.9 Пакетные операции

`apply_ops`: по исходному требованию атомарный пакет операций **любых типов выше** одного документа, всё/ничего, одна undo group. ⚠ Это конфликтует с необратимыми side effects (печать, сеть). ADR-0010 предлагает ограничить пакет транзакционными модельными ops, а side effects выделить отдельно; предложение пока не принято и не заменяет требование. <!-- REQ-M06-072 -->

Все UI-команды должны иметь ops/MCP-путь и наоборот. Недостающие инструменты находятся в `docs/mcp-gaps.md`; `apply_ops` не служит маскировкой отсутствующей схемы конкретной операции. <!-- REQ-M06-073 -->

## 7. Ресурсы

| URI | Ресурс |
|---|---|
|<!-- REQ-M07-001 --> `trinedocs://documents` | open/recent |
|<!-- REQ-M07-002 --> `trinedocs://doc/{id}/outline` | outline |
|<!-- REQ-M07-003 --> `trinedocs://doc/{id}/content` | Markdown Docs / Sheets summary / Slides text |
|<!-- REQ-M07-004 --> `trinedocs://doc/{id}/sheet/{name}/{range}` | CSV/JSON cells |
|<!-- REQ-M07-005 --> `trinedocs://doc/{id}/slide/{sid}/preview.png` | slide preview |
|<!-- REQ-M07-006 --> `trinedocs://doc/{id}/selection` | user selection |
|<!-- REQ-M07-007 --> `trinedocs://doc/{id}/comments` | comments |

Subscriptions: document edit → resource-updated; resource set change → list_changed. Проверяются revocation/privacy/pagination, percent-encoding name/range. <!-- REQ-M07-008 -->

## 8. Промпты

`summarize`; `proofread` suggest; `rewrite_selection` formal/simple/shorter/longer; `translate`; `make_table_from_text`; `analyze_sheet` anomalies/formulas; `build_chart`; `deck_from_document`; `speaker_notes`; `fix_formulas` #REF!/#DIV/0! proposals. <!-- REQ-M08-001 -->

## 9. Видимость агента

- MCP edit session — CRDT participant, cursor, встроенный AI / внешнее имя Claude/Codex, ai palette. <!-- REQ-M09-001 -->
- Smooth cursor → next edit, fading highlights. <!-- REQ-M09-002 -->
- «Агент редактирует», Stop → cancel session/rollback unfinished step. <!-- REQ-M09-003 -->
- Concurrent user/remote edits converge. <!-- REQ-M09-004 -->
- Agent undo groups, AI review panel, suggest=track changes. <!-- REQ-M09-005 -->

## 10. Логирование и диагностика

- «Активность MCP»: live calls, masked content parameters by settings, duration/errors/jump to edits. <!-- REQ-M10-001 -->
- Log level settings, MCP logging notifications. <!-- REQ-M10-002 -->
- Inspector-compatible debug endpoint только dev build. <!-- REQ-M10-003 -->

## 11. Тестирование

- Каждому tool: schema/errors/rights/dry_run/suggest/base_version conflicts. <!-- REQ-M11-001 -->
- rmcp e2e client: stdio→shim→IPC, bearer HTTP/Origin, in-process. <!-- REQ-M11-002 -->
- Live concurrent agent/user/participant: same state, correct cursors. <!-- REQ-M11-003 -->
- MCP Inspector/Claude Desktop/Codex CLI ручная проверка каждого релиза. <!-- REQ-M11-004 -->
- Reject missing bearer/foreign Origin/other-user IPC; injection fixture не вызывает действий вне запроса. <!-- REQ-M11-005 -->
- Tasks/elicitation capability fallback реально тестируется, не предполагается по наличию фичи в протоколе. <!-- REQ-M11-006 -->

## Дополнение 2026-10-18: паспорт формулы и линза оформления

Код реализован, runtime-приёмка не проводилась. Спецификация: [office-capabilities](specifications/office-capabilities-2026-10-18.md).

`sheets_explain_formula` — read-only. Обязательны document_id/sheet_id (integer ≥1), row (0…1048575), col (0…16383); необязательный верхнеуровневый client_access проходит обычную проверку клиента. Применяются права текущего документа, никакие права edit не выдаются. Ошибки: отсутствующие/неверные координаты, лист/документ не найден, документ другого типа. Возвращает version/document_id, address/input (до4096 знаков), value_type/display (до512), syntax_error, dependencies (до64, глубина32, четыре образца), omitted/truncated/scope. Результат содержит недоверенные данные документа. Значения берутся из кэша; пересчёт, flush draft, selection и undo не изменяются.

`inspect_document(kind="formatting")` добавляет ограниченные замечания об отличиях size/bold/italic от встроенных стилей Docs. Пользовательские стили пропускаются; прямое оформление может быть намеренным. Пагинация, версия снимка и устаревание — общие правила инспектора. Команда сама ничего не исправляет.

Для новых UI-команд специальной вставки и повестки нет отдельных MCP-контрактов одного вызова; это явно остаётся в [реестре пробелов](mcp-gaps.md). Наличие базовых ops не считается закрытием этого пробела.

## Дополнение2026-10-18: noFill фигуры и проверенная по исходникам навигация

Код, без runtime-приёмки: `slides_read` сериализует `fill_visible`; `slides_update_element` принимает существующий nested patch `element: {"fill_visible": false}` вместе с обязательными document_id/slide_id/element_id. Права, version/mode, кандидатная модель и undo проходят существующий dispatch; новый bypass не добавлен. Поле применяется к Rect/Ellipse, хранится отдельно от fill RGB и stroke, экспортируется в PPTX как a:noFill. Старые модели без поля получают true. Подробности и ограничения: [BUG-150](audit/bugs-101-150-2026-10-18.md).

Панель поиска Docs и фильтр навигатора Slides изменяют view state. Новых MCP tools управления этими панелями нет. Инструмент `docs_replace_all` уже существует, как и Sheets replace_all; UI использует отдельный checked bulk-replace API модели. Наличие существующего tool не означает одинаковых алгоритмов/лимитов или полного MCP-паритета. Предыдущая формулировка могла ошибочно создавать впечатление отсутствия Docs replace-all и здесь уточнена.


## Дополнение2026-10-18: BUG-151…250, bounded object reads

Статус — реализация по исходникам, без компиляции, запуска тестов или MCP/GUI-приёмки. Полный реестр: [BUG-151…250](audit/bugs-151-250-2026-10-18.md).

`docs_read_objects` принимает обязательный `document_id`; `index` (по умолчанию0) и `count` (по умолчанию20, clamp1…50) листают независимо списки таблиц и изображений. Необязательный `table_id` фильтрует таблицы **до** применения index/count; для конкретной таблицы используйте index=0, отсутствующий ID возвращает not_found. Изображения этим фильтром не ограничиваются.

В каждой возвращаемой таблице `row`/`rows` задают начало/число строк (0/20, rows clamp1…100), `col`/`columns` — начало/число физических ячеек каждой строки (0/64, columns clamp1…256). Индексы нулевые. Это физические элементы массива ячеек, не логические grid column coordinates; `span` и `merged` нужны для восстановления сетки. Ответ содержит `row_offset`, `column_offset`, `total_rows`, `grid_columns`, размеры колонок, border_color/borderless, cell_margin, header_rows и страницу row_heights; ячейки добавляют span/merged/v_align к text/fill/align/format. Корень содержит total_tables/total_images, offset/page_size, page и version.

Это изменение поведения: по умолчанию table rows теперь ограничены20, cells —64; клиенты, которым нужна вся таблица, должны листать обе оси. `text` остаётся плоским текстом ячейки, `format` — её первым оформлением, не полной rich-run моделью. Общий лимит ответа1MiB сохраняется; одна огромная ячейка/поле может по-прежнему дать too_large. Потоковое чтение содержимого одной ячейки не добавлено.

`sheets_read_objects(document_id,sheet_id,index?,count?)`: index=0, count=100, clamp1…100. Один index/count применяется **отдельно** к каждой коллекции widths, row_heights, hidden_rows, hidden_cols, merges, conditional, charts, images и workbook names. Пары BTreeMap возвращаются JSON-объектами с прежними ключами; последовательности — массивами. `total_widths`, `total_row_heights`, `total_hidden_rows`, `total_hidden_cols`, `total_merges`, `total_conditional`, `total_charts`, `total_images`, `total_names` относятся к полным коллекциям, `offset/page_size/version` описывают страницу. sheet name/extent/freeze/filter/hidden/tab_color остаются скалярными полями. Bytes изображений не возвращаются.

Оба read tool сохраняют обычные document/client permissions и не меняют draft/selection/undo. Между страницами возможна правка документа: клиент сравнивает version и начинает чтение заново при изменении; закреплённого snapshot token нет.

`slides_read` теперь фактически включает `fill_visible` в element metadata (предыдущая документация опережала сериализатор). `slides_format_text` допускает `format: {"fill_visible":false}` для массива element_ids; `slides_update_element` по-прежнему принимает nested element patch. Существующие typed validation/candidate/ops/undo и права edit сохраняются. Это noFill для поддержанных фигур, не полная модель DrawingML fills.

Файловый dry_run выполняет общий preflight: ID документа, поддержанный формат, путь сохранения, read-only/save-lock/busy для соответствующей команды. Он не пишет файл и не запускает экспорт; это проверка базовых предпосылок, не гарантия успешного рендера/дисковой записи. Open по-прежнему проходит пользовательский UI-путь. Возврат фонового media job повторно проверяет MCP enabled и актуальное разрешение/поколение встроенного чата; отзыв не отменяет уже выполненный дисковый export.
