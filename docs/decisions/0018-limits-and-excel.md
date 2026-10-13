# ADR-0018: Лимиты и Excel semantics

Статус: **предложено; engine spike pending**. Дата:2026-10-12UTC.

## Контекст

Official Excel limits FACT-015; performance target100k не значит весь sheet allocated.1900/1904 semantics,15digits/cycles/locale/volatile ещё сверить primary docs.

## Варианты и рекомендация

A: sparse stable-ID model+virtual grid, canonical formula syntax with locale display; B: dense full sheet (риски memory). Рекомендация A; exact max document pages/slides/files и resource-limit policy требуют data, не снижать Office features без владельца.

## Критерии, данные и последствия

Boundary cases1048576×16384,32767cell text; date leap bug/month/year conversions; 15digits; cyclic errors/iteration; argument localization; dependency order; volatile seeds/now; import cached values vs authoritative recalc. Unexpected input reject with readable error, never allocate huge dense grid.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
