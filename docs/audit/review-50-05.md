# Пакет 05 — 50 исправленных причин и AI-проверка документа

Основа `8e6c20b`, ветка `codex/review-50-ui-05`. В код внесены **50 отдельных production исправлений**:20 Sheets,20 Docs/Slides,10 MCP/оболочка/сохранение. Число означает описанные причины с конкретными сценариями, а не результат выполнения50 тестов.

| Область | Реестр |
|---|---|
| Sheets: VALUE/ANSI, рабочие дни, финансовая точность, форматы, merged/hidden data, XLSX metadata/export | [20 причин C5-01…20](review-50-05-core.md) |
| Docs/Slides: стиль/списки DOCX, Markdown/Unicode, input focus, pagination, timing/blanking | [20 причин E5-01…20](review-50-05-editors.md) |
| MCP/файлы/UI: диапазон, группы, имена, версии поиска, пересчёт, backup, byte payload/geometry/selection, pins | [10 причин S5-01…10](review-50-05-shell.md) |

Дополнительно: поиск/категории/копирование отчёта проверки; AI tool `inspect_document` с фоновым снимком, polling, фильтрами и страницами. Отчёт ограничен первыми200 замечаниями с явным omitted/stale. Эвристики не расширены, автоисправления не добавлены. Runtime ещё не проверен.

Два независимых агента завершили перекрёстное чтение Sheets и Docs/Slides, затем оба перечитали root исправления и новую inspector API/UI. Подтверждённых статических блокеров после исправлений не осталось. Подробные sign-off и границы проверки — в индивидуальных отчётах.

Проверки прошли: cargo fmt --all -- --check, git diff --check, python -B tools/spec_registry.py check (442 REQ,361 PAR), python -B tools/check_assets.py. Добавленные Rust-регрессии не запускались. Нет сборки, cargo check/test/clippy, GUI/app, установщика, релиза, merge и компилирующего CI. Коммит с [skip ci].

Не входят в50: собственные промежуточные ошибки05, расширение прежних исправлений TextEdit reconciliation/empty paragraph formatting, PR7 follow-up stop/clear и stateless access, новые фичи. Полная Office/OOXML совместимость, производительность, visual acceptance и безопасность внешних importer процессов остаются неподтверждёнными либо GAP.
