# ADR-0028 — Перенос функций из WordCraft, GridCraft и DeckCraft
Дата: 2026-10-17. Статус: принято в рамках поручения владельца 2026-10-17.

## Контекст
Владелец указал три открытых проекта ArtCraft — [wordcraft](https://github.com/storytold/wordcraft), [gridcraft](https://github.com/storytold/gridcraft), [deckcraft](https://github.com/storytold/deckcraft) — и поручил перенести в TrineDocs все функции, которых у нас нет, и улучшить интерфейс. Все три проекта под `MIT OR Apache-2.0`, написаны на Rust.

Самый большой измеримый пробел — функции листа: в GridCraft 491 имя функции, в нашем движке формул (`sheets/formula.rs`) около 150. Остальное (редакторы документов и слайдов) зависит от другой модели данных и UI-стека и переносится по смыслу, а не копированием.

## Решение
1. **Волна 1 — библиотека функций листа.** Модули функций `gridcraft-functions` (criteria, database, date, dynamic, engineering, ets, financial, info, lift, lookup, math, special, stat, text, util) и нужная им часть `gridcraft-core` (value, date, parse) скопированы в `crates/trinedocs/src/sheets/xlfn/`:
   - в каждом файле — заголовок с источником, copyright и лицензией; полный текст MIT — `xlfn/LICENSE-MIT`;
   - пути `gridcraft_core::`/`crate::` переписаны на `crate::sheets::xlfn::…`; serde и wasm-ветка удалены;
   - вендорный код проходит наш clippy (`-D warnings`), допущены только `too_many_arguments` и `type_complexity`; тесты upstream сохранены и выполняются в `cargo test`.
2. **Мост к движку.** Встроенные функции движка выигрывают; неизвестное имя уходит в `xlfn::lookup` → `xlfn::call`. Аргументы-диапазоны передаются массивами с признаком ссылки (для ISREF/ROWS и т. п.), результат-ошибка остаётся ошибкой Excel, массив возвращается диапазоном. `NOW`/`RAND` библиотеки берут время и генератор движка, система дат — 1900.
3. **Регулярные выражения.** REGEXTEST/REGEXEXTRACT/REGEXREPLACE требуют `regex =1.13.1` (2026-07-15, MIT OR Apache-2.0) и транзитивно `regex-automata 0.4.18`, `regex-syntax 0.8.11`, `aho-corasick 1.1.5` (Unlicense OR MIT, выбран MIT), `memchr` уже есть. Все старше 14 дней, AGPL нет. Свой движок регулярных выражений не пишем: это больше кода и риск ReDoS; `regex` гарантирует линейное время.
4. **Дальнейшие волны** (план — `docs/plan/phase-craft-parity.md`): разлив динамических массивов и LET/LAMBDA, подсказки функций в строке формул, функции редакторов документов и слайдов, улучшение интерфейса.

## Последствия
- Число функций листа выросло примерно до 490. Пока нет разлива массивов: формула, возвращающая массив (SEQUENCE, FILTER, SORT, TEXTSPLIT …), показывает в ячейке верхний левый элемент, а внутри других формул (`SUM(SEQUENCE(3))`) массив используется целиком. Это записано в паритете, функции не заявлены как полностью готовые.
- Вендорный код обновляется вручную по upstream; источник и адаптации указаны в заголовках файлов.
- В установщик THIRD-PARTY-NOTICES добавляется текст MIT GridCraft (`tools/build_release.ps1`). Реестр crate-зависимостей `docs/licenses.md` генерируется `tools/audit_dependencies.py --refresh` и обновляется отдельным коммитом после загрузки новых crates.
