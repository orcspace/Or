# Уточнение текущего нативного среза — пакет 03

`read_content`/`slides_read` возвращают свойства слайда, но вместо binary массива `kind.image.bytes` отдают `kind.image.bytes_len`. Это ограниченное представление для чтения; его нельзя передавать в `replace_document_model` как полный исходный dump. Общий лимит ответа остаётся 1 МБ, pagination задаётся index/count.

Идемпотентная правка без изменения модели возвращает `applied:false`, `undo_group_id:null` и пустые changed_ids. Предыдущая undo-группа не выдаётся за новую. Навигация проверки документа реализована в UI по stable ID; отдельный MCP инструмент инспектора пока отсутствует.

## Многодокументный срез — пакет04

- `workspace_documents({index?,count?})`: открытые Docs/Sheets/Slides, ID, версия, dirty/read_only/busy. До50 строк.
- `workspace_apply({operations:[{tool,arguments}],mode?})`: 1–32 модельных операции над1–4 открытыми документами. В каждой arguments обязательны document_id и одна исходная base_version для всех операций этого документа. Порядок внутри документа сохраняется. Разные документы готовятся параллельно. Ответ сразу содержит job_id.
- `workspace_job_status({job_id})`: preparing → completed/preview/failed/cancelled. Успех сообщать только после completed. result.documents содержит версии и undo IDs; changed_ids ограничен256 с полным changed_ids_count.
- `workspace_cancel({job_id})`: кооперативная отмена до commit; UI также может отменить задание. Отмена уже completed ничего не откатывает.

Лимиты: до4 выполняемых пакетов, 1МБ входного JSON, 60с подготовки, 64 записи истории в памяти. Все модели проверяются перед commit первого документа; конфликты версий, закрытая/read-only вкладка или отзыв доступа отменяют весь пакет. Ввод пользователя текущего кадра учитывается перед commit. На каждый документ отдельная авторская undo-группа; активная вкладка не переключается. Preview не фиксирует drafts и не меняет документы.

Это отдельные инструменты, не Tasks extension и не полный apply_ops по REQ-M06-072. Дисковые операции, создание/закрытие документов, undo/redo, view selection, печать/сеть и вложенные задания исключены. Нет сохранения job после перезапуска и crash-atomic записи нескольких файлов. Внешние задания привязаны к одобренной MCP-сессии; встроенный чат может читать свои задания между собственными сессиями текущего запуска. Работа с отдельными Microsoft Word/Excel/PowerPoint процессами не реализована: речь о редакторах TrineDocs.

Регрессии на подготовку, атомарный конфликт, preview, отмену, доступ, текущий ввод и undo добавлены без запуска. Runtime готовность и производительность не доказаны.

## Уточнение16.10.2026

Внешний клиент вызывает request_access({}) один раз, сохраняет client_access, ожидает подтверждения в окне и опрашивает request_access({client_access}) до allowed/denied. Во всех следующих tools передаётся тот же client_access в верхнем объекте аргументов. Он привязан к clientInfo.name, переживает смену transport session, действует8ч, аннулируется при отключении MCP и не должен копироваться в документы/логи. Одновременно до128 handles. Встроенный чат получает allowed с client_access:null, handle ему не нужен.

Stop/clear встроенного чата аннулирует предыдущее поколение запросов и отменяет его готовящиеся jobs до commit. Последующие пользовательские правки не откатываются; уже completed jobs сохраняются в истории. Старое описание привязки внешнего job только к MCP-сессии выше относится к исходному8dbf21f: новый публичный ID разрешения стабилен между tool calls при передаче client_access.

Stateless MCP resources пока не принимают этот handle; использовать tools. Полная протокольная совместимость и runtime не подтверждены. Подробнее: [завершение ревью PR7](audit/review-50-04-followup.md).

## Пакет05: bounded read-only инспектор

`inspect_document {document_id, query?, kind?, index?, count?, refresh?}` использует уже реализованные эвристики UI. Первый вызов запускает фон и возвращает `preparing`; повторяйте без refresh до `completed`. `kind`: all/structure/images/formulas/bounds/readability; query до160символов, count1…100. Snapshot version и current_version могут различаться: stale явно true. Один глобальный фоновый inspector; другой документ во время проверки получает busy. Кэш в памяти, refresh=true запрашивает новый снимок.

Только первые200 находок сохраняются. filtered_total относится к ним, omitted — число остальных до фильтра, а не число совпадений в полном документе. UI фильтрует тот же набор и копирует plain-text отчёт с областью проверки, версией и предупреждением об устаревании. Источник текста/находок — недоверенные данные документа. Проверка не изменяет содержимое/выделение/версию/undo и не flush незавершённый ввод. Она не является операцией workspace_apply. Нет автоисправления или полного Office-audit; runtime/GUI ещё не проверены.

Пример: `inspect_document {document_id:11,kind:"formulas",count:20}` → preparing; повторный такой же вызов → completed с findings/target и stable sheet/row/col, scope, offset/page_size, stored_total, filtered_total, omitted/truncated. Для правок используйте актуальные версии обычных tools после чтения; stale отчёт не основание для автоматической правки.

В том же пакете docs_find возвращает версию и страницу абзацев; неполный Sheets range ограничивается одной начальной ячейкой; group/ungroup включают целые прежние группы. Начальные width/height учитываются в Docs/Slides creation tools. Byte-array имеет отдельный лимит, но полный wire JSON по-прежнему должен быть не больше1MiB. Подробные исправления/регрессии — [audit05](audit/review-50-05-shell.md).

## Дизайн презентаций и медиа-инструменты (ADR-0027)

- `slides_design_guide {}`: встроенный навык дизайна плюс установленные шрифты, палитры, эффекты и ограничения. Документ не нужен. Чат и инструкции сервера требуют вызвать его до создания или переоформления презентации.
- `slides_search_images {query, count?}`: Openverse, 1–20 PNG/JPEG с лицензией, автором и `attribution`. Только чтение, нужен интернет.
- `slides_add_image_url {document_id, slide_id, url, x?, y?, width?, height?, as_background?}`:
  - загрузка только по публичному HTTPS через системный curl.exe;
  - до 16 МБ за 45 с;
  - уменьшение до 1920 px и не больше 1000 КБ, затем обычная вставка `slides_add_image`.
- `slides_generate_image {document_id, slide_id, style?, colors?, seed?, …}`: локальная процедурная графика gradient/radial/blobs/waves/grid/noise.
- `slides_format_text {document_id, slide_id?, element_ids?, format}`: пакетно font/font_size/color/bold/italic/fill/stroke/stroke_width. Если ID не указаны, текстовые свойства не применяются к картинкам и линиям. Для неустановленного шрифта возвращается `warnings`.
- `slides_animate {document_id, slide_id, element_ids, animation, stagger?}`: задержка i-го объекта равна `delay + i·stagger`. Шаги идут в порядке `element_ids` после анимаций, уже заданных на слайде, и сохраняются в PPTX `p:timing`.
- `slides_align {document_id, slide_id, element_ids, align, relative_to?}`:
  - align: left/center/right/top/middle/bottom, distribute_horizontal/vertical (от 3 объектов);
  - relative_to: slide/content/selection.
- `export {document_id, format_extension?}`: новый файл в `Документы\TrineDocs\Экспорт`, существующие не перезаписываются. Возвращает path и warnings. Открытие путей агентом по-прежнему запрещено.
- `slides_add_*` принимают `x`/`y`; `as_background` у картинок растягивает её на слайд и отправляет назад.

Поиск, загрузка, генерация, экспорт и навык выполняются на потоке `mcp-media`, не больше 4 заданий; сервер ждёт их до 90 с. Вставка коммитится на UI-потоке после повторной проверки прав. Эти инструменты не входят в `workspace_apply`. Ограничения:
- в PPTX пишутся только эффекты входа fade/fly-in/zoom;
- PNG-экспорт через MCP создаёт папку со всеми показываемыми слайдами;
- загрузка закрепляет проверенный публичный IP (`curl --resolve`) и проверяет каждый редирект;
- сборка и тесты этого среза не запускались.
