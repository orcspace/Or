# Отчёт: ещё100 исправлений и UI —2026-10-18

Запрос владельца: ещё100 и4 агента; ранее заданные запреты на сборку и исполнение сохранены. План: [phase-100-more](phase-100-more-2026-10-18.md). Основа — draft [PR #13](https://github.com/orcspace/Or/pull/13), commit `bca943875d6855b745cbc2f85d5c2be486adf69f`. Новая ветка `codex/100-more-fixes-ui-20261018`; BUG-001…150 не пересчитываются.

## Что реализовано

Ровно100 новых дефектов BUG-151…250 и100 новых `#[test]` относительно основы: Docs25 (22IO/3model), Sheets25 (8formula/3format/1model/5editor/8IO), Slides25 (21IO/1model/3editor), общие службы25. [Общий индекс100](../audit/bugs-151-250-2026-10-18.md) связывает каждый ID с before/after, исходником и именем регрессии.

- Docs: структурный HTML table import, row groups/rowspan0, whitespace/CSS/list inheritance, image dimensions/notice и metadata; DOCX hidden/style/table/history subset; Markdown edge cases; preflight multiline replace и word count.
- Sheets: устойчивые числовые/текстовые формулы и AST/search budgets; native text rotation/decoration; Ctrl+A current region и sparse selection statistics; merged typed clipboard/transpose; XLSX relationships/selection/style/geometry; bounded CSV/MD output и видимый merged anchor в HTML.
- Slides: PPTX rotation/placeholder/layout/theme/text/chart/sections/group/image subset; HTML/MD embedded images с общим output budget; canvas picking по повёрнутой фигуре/тонкой линии, Esc inline edit и multiline table cells.
- Общие службы: bounded MCP object pages и table/noFill metadata; transparent PNG media budget, Windows file stems, contrast inspector и aggregate slide picture budget; chat bold/code/escaping/clear; точные shell hotkeys, status probe; file dry-run preflight, media reply permissions и shape-only PDF без системного шрифта.

Пользовательские улучшения интерфейса относятся к существующим native egui flows: оформление текста таблиц, корректное выделение, редактирование ячеек с несколькими строками, выбор фигур и клавиатура. Новые зависимости и отдельный веб-интерфейс не добавлялись.

## Работа четырёх агентов и review

Root — общие службы; три дополнительных агента — Docs, Sheets и Slides. Итоговые агентские heads:

| Набор | Commit |
|---|---|
| Docs | `d4efaf1bc0694fed128acb54701ce25643667a31` |
| Sheets | `1c3dcefc67e86e37829bcba9216fdc81a77d4114` |
| Slides | `e7379d9ac6653144b7104d25211fdba2205c54ca` |

Merge commit сохраняет три агентских parent commits; дерево собиралось из объявленных путей каждого набора поверх общей ветки, с expected-head lease. После публикации содержимое перечитано. Все изменения выполнены через GitHub; удалённые локальные исходники/.git не восстанавливались. Remote mutations координировались Orell claims, освобождёнными после записи.

Peer review нашёл и закрыл потерю контекста nested HTML lists, row-group semantics rowspan0, chat escapes после обычного текста, пустой final file stem, раннее разрешение шрифта для empty PDF text и бюджет data-URI export. Sheets final review сохранил отказ plain paste поверх merge и атомарный отказ несовпадающих merged geometries. Уточнения входят в исходные BUG IDs, дополнительный счёт не создают.

## Проверено без исполнения приложения

- Опубликованные diffs относительно PR #13 и итоговые исходники просмотрены; уникальные BUG IDs151…250 непрерывны.
- Каждое из100 имён регрессий найдено как функция в изменённых исходниках; net additions составляют100 `#[test]`.
- Лексический баланс строк/комментариев/скобок проверен как текст. Это не Rust parser, typecheck, borrow check или компиляция.
- Матрица Markdown и JSON синхронизирована:366 строк, включая ранее пропущенный PAR-TRI-045 и новый PAR-TRI-046. Новые/уточнённые статусы — partial/implemented_unverified.
- MCP documentation уточняет новые defaults страниц и существование `docs_replace_all`; исходная формулировка о noFill read теперь обеспечена сериализатором.

**Не запускались** build/check/test/fmt/lint, GitHub Actions, приложение, Office, установщик и release. Тесты написаны, но не выполнены; компиляция, производительность, GUI/AccessKit/клавиатура, undo integration, внешние MCP transports и Office round-trip остаются непроверенными. В commit messages указан `[skip ci]`; PR создаётся draft, main не обновляется, merge не выполняется.

## Материальные пределы

HTML/CSS/CommonMark/OOXML реализованы поднаборами. Hidden DOCX runs отбрасываются с notice; track-changes history не сохраняется. Nested HTML tables/images in cells, rich-cell streaming, pinned read snapshots, полный master/group/mixed text handling, cross-sheet chart sources и cut-reference movement не закрыты. Огромный single cell может превышать MCP response limit; export может явно отказать по aggregate budget. Canvas text rotation не заменяет PDF/print renderer. NoFill не является всеми DrawingML fill effects; shape-only PDF не решает embedding/subsetting текстовых PDF. RANDBETWEEN на огромных границах проверяется инвариантом движка, не подтверждённым Excel parity.

Следующий шаг при разрешённом исполнении — Rust typecheck и100 регрессий, затем targeted GUI/MCP и Office corpus acceptance. Эта фаза не является подтверждением полного Office/MCP parity или release DoD.
