# Финальное независимое ревью MCP, Slides и выпуска

Дата: 2026-10-13. Read-only review исходного diff; исправления только ops.rs и mcp/tools.rs. Cargo/target проверяет root централизованно.

## Две найденные и исправленные MEDIUM ошибки

- AI-правка Sheets → пользователь лишь выбирает другую ячейку или лист → «Откат AI» возвращал ложный conflict. Book equality включает active/selection, хотя UI меняет их без содержательной операции.
- MCP set_selection проходил через Replace: чистая сохранённая книга становилась dirty, появлялась AI undo-группа и уничтожалась redo branch.

AI rollback сравнивает содержимое с сохранённым после AI snapshot, нормализуя только allocator floor и Sheets view state. При восстановлении выбранный лист сопоставляется по stable sheet ID, а не индексу; при удалении созданного AI листа используется допустимый fallback. Текущее выделение сохраняется с ограничением координат границами модели. Поздние правки текста, форматов, ячеек, структуры и других данных продолжают конфликтовать.

MCP set_selection имеет отдельный атомарный validated путь: тип sheet_id, существование листа, координаты, mode и optional base_version проверяются до присваивания. dry_run не меняет модель. Успешная навигация сохраняет version/saved_version/dirty, allocator, undo/redo/AI history, last_author и typing timestamp. Неверные аргументы не меняют даже active sheet.

## Регрессии

Добавлены 7 осмысленных tests:

- откат AI после выделения диапазона и переключения листа сохраняет навигацию и удаляет только AI cell;
- восстановленный порядок листов требует сопоставления текущего stable sheet ID;
- откат созданного выбранного листа даёт допустимый fallback;
- реальная последующая User cell правка по-прежнему атомарно конфликтует;
- MCP навигация в сохранённой книге не делает её dirty и не меняет историю/автора/версию;
- MCP навигация сохраняет настоящую redo branch и разрешает повтор AI cell;
- dry_run, неверный sheet ID/тип, отрицательные/переполненные/вышедшие за границы координаты и stale version оставляют модель и историю неизменными.

Scoped rustfmt и git diff --check выполнены. Исполнение тестов, clippy, новый dist и installer QA добавляет root после последнего изменения; здесь PASS для них не заявлен.

## Остальное независимое чтение

Новых HIGH в проверенных MCP tools/ops, Slides focus/modal, smoke scripts, NSIS и release workflow не обнаружено. MCP raw-model required base_version/lifecycle guard и allocator high-water сохранены. Уникальная session identity связывает право отмены с текущей сессией; rmcp 3.5.0 defaults валидируют localhost Host, enforce_origin_validation блокирует чужой Origin, request body ограничен 1 MiB. Slides canvas/clipboard и окна учитывают disabled UI и text focus. Прежние release defects с shell ref interpolation, неподтверждённым NSIS download, existing install/shortcut collisions и silent failure guards не вернулись.

Это статическое ревью и новые source fixes. Предыдущие preview/Office parity ограничения сохраняются; чистая Windows VM и внешняя Office corpus приёмка этим агентом не выполнялись.
