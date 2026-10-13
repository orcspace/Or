# ADR-0012: Вёрстка и запасной CalcEngine

Статус: **предложено; go/no-go не получен**. Дата:2026-10-12UTC.

## Контекст

SkParagraph не заменяет page layout. IronCalc0.8 release объявляет современные функции, но не гарантирует Excel equivalence (FACT-010).

## Варианты и рекомендация

Layout A: собственный Word-layer; B: audited OSS компоненты; C: LO layout отдельным процессом. Calc A: upstream/fork additions; B: own; C: alternate audited engine. Не менять Skia/IronCalc стек без владельца ADR; trait CalcEngine сохраняется.

## Критерии, данные и последствия

Layout4.1 сравнивает tables/wrap/columns/notes/headers и incremental≤16ms. Calc4.5 full official inventory, date/locale/errors100k≤50ms. Effort/license/maintainability измерить/оценить явно, числа пока неизвестны.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
