# Пробелы UI ↔ ops ↔ MCP

Первый проход; перечисление tool names в MCP.md ещё не их реализация. PAR-матрица показывает связанный tool или явный GAP. Любая будущая UI-команда имеет typed op, точную schema и собственную проверку прав/undo/preview; generic apply_ops не заменяет missing op contract.

| Область | Не хватает контракта | Фаза/проверка |
|---|---|---|
| App/files | save_copy, pin/remove recent, rename, recovery, compare external/base, multiwindow/tab/split, file-lock policy |1/2, T-02-FILES/T-03-MCP|
| General | undo/redo/history scope, paste special/import clipboard, autocorrect/spell/OCR/TTS, view/zoom/rulers/guides |1/8, T-03-MCP|
| Docs | update/delete images, crop/rotate/anchors, shapes/textboxes, table sort/formulas, bookmarks/endnotes/citations/OMML, document compare/merge |1/8, T-01-LAYOUT/T-03-MCP|
| Sheets | autofill, autofit, sheet hide/tab color, notes/protection, named-range manager/delete, structured tables/slicers/sparklines/flash fill/what-if |5/8/9, T-05-CALC/T-03-MCP|
| Slides | edit master/sections, distribute/flip, media assets+trim/SmartArt/Zoom, show/presenter/rehearsal, pointer/ink |6/8, T-06-SLIDES/T-03-MCP|
| Collaboration | roles/invites/comments ownership/mention delivery/follow/presence operation contracts |4, T-04-SYNC/T-SECURITY|
| Settings/security | authorized subset settings updates; AIoff respected; never reveal keys via tools; client-registration consent/backup/diff |0/3, T-03-MCP/T-SECURITY|
| P1 own | live links/template fields, native-format passwords/history compaction/migrations, document inspector |8/9, T-08-PARITY/T-SECURITY|

Атомарность side-effects, Unicode offsets, IDs до create, уровни create/export/PDF, concurrent AI rollback/suggest — ADR-0010. Не запускать MCP с придуманными схемами, закрывающими эти пробелы только на бумаге.


## Срез 2026-10-18

- Реализован отдельный read-only `sheets_explain_formula` с координатами, пределами и версией снимка; инспектор принимает kind=formatting. Добавлен контрактный тест, запуск не проводился.
- Новые UI-команды «Специальная вставка» (режим/transpose/внутренний снимок) и «Повестка» (предпросмотр/разбиение) проходят обычный UI→ops→undo. Их отдельные MCP-схемы одного вызова ещё нужны; generic ops не выдаются за такие контракты.
- Паспорт формулы — карта прямых/именованных ссылок с кэшированными значениями, не полный пошаговый debugger вычисления Excel. Линза оформления пока охватывает body-абзацы и встроенные стили.


## Срез BUG-151…250

- `docs_read_objects` получил layout/merge metadata и bounded table pages, `sheets_read_objects` — paging каждой коллекции и total_*; точные defaults и изменение поведения описаны в [MCP.md](MCP.md). Эти чтения не закрывают full rich-cell content streaming и version-pinned snapshots.
- `slides_read` теперь возвращает fill_visible, `slides_format_text` принимает noFill. Остальные shape/master/flip gaps не считаются закрытыми.
- `docs_replace_all` существовал до этой фазы. Отсутствует отдельный контракт управления новыми UI-панелями; это не отсутствие инструмента замены текста. UI checked bulk replace и существующий MCP positioned-edit path не объявляются идентичными.
- Dry-run preflight и повторная проверка разрешений media jobs реализованы в коде; transport/permission race/undo/runtime acceptance не проводились.
- Dedicated tools для typed clipboard с merge/transpose, section management и новых view controls остаются отдельными задачами. Сборка, тесты, GUI и внешние MCP-клиенты не запускались.
