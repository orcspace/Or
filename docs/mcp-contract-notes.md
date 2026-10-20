# Уточнения MCP-контрактов (2026-10-18)

Вынесено из `docs/MCP.md`: эти заметки описывают детали реализации и пробелы, а не новые требования, и поэтому не несут REQ ID. Статусы — по коду; runtime/MCP Inspector-приёмка не проводилась.

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

## Уточнения контракта нового прохода BUG-251…450 (2026-10-18)

`app_status` / `list_documents` и `sheets_list` / `slides_list` теперь соблюдают `index` (0) и `count` (100, clamp1…100), возвращая `offset/page_size/total`. Это изменение default: списки длиннее100 требуют продолжения.

`docs_read` и Docs-вариант `read_content` поддерживают `paragraph_id` (фильтр до index/count), `offset` (Unicode scalars, 0) и `characters` (4096, clamp1…16384). Наличие любого из этих аргументов включает окно rich runs; каждый paragraph содержит `text_offset/text_page_size/total_chars/text_truncated`. Без них прежнее чтение абзаца целиком сохраняется. Большой абзац можно обнаружить, задав characters и листая index/count. `docs_get_outline` отдаёт первые512 Unicode scalars заголовка с total_chars/text_truncated. Лимит ответа1MiB действует и для этих путей; это bounded response, а не полностью потоковая сериализация модели.

`slides_read` и Slides-вариант `read_content`: необязательный `slide_id` фильтрует слайды до index/count; `element_index` (0) и `element_count` (100, clamp1…100) листают объекты каждого возвращаемого слайда независимо. Metadata содержит `element_offset/element_page_size/total_elements`. Изображения возвращают metadata без bytes. Один огромный text/table/chart по-прежнему может превысить1MiB; окна внутри таких объектов не добавлены. Для стабильного продолжения клиент сверяет version; snapshot token между страницами не реализован.

Ответы model edit tools ограничивают `changed_ids` первыми256 уникальными IDs в исходном порядке. `changed_ids_count` сообщает полное число, `changed_ids_truncated` — усечение; no-op возвращает0 и пустой массив. Range writes/format включают IDs ячеек, sheet/slide правки — затронутый структурный объект. Workspace worker сохраняет полные IDs через непубличный model path для точных totals; внешнего обхода ограничения нет.

`dry_run` model edits/undo/redo проверяет тот же commit engine, включая version exhaustion, read-only и очередь позиционных Docs edits, на минимальной preview Tab. Это не side effect и не гарантия сетевой/файловой операции. Create preflight проверяет allocator и занятые document IDs до dry_run. Неизвестные tools и неверно типизированные outer args отклоняются до создания запроса доступа. Новых ожидающих approval entries максимум128; повтор существующего legacy клиента не требует нового места.

`sheets_add_image` сохраняет явно переданные dimensions и alt без молчаливого clamp/truncate: значения за пределами модели отклоняются атомарно. Typed schema `slides_format_text.format` теперь также объявляет ранее поддержанный handler-ом `fill_visible` (исправление остатка BUG-230, без нового BUG ID).

Эти контракты проверены чтением кода и опубликованного diff; регрессии написаны, не запускались. MCP Inspector/GUI/runtime/Office-приёмка не проводилась.
