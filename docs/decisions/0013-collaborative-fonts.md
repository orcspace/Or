# ADR-0013: Шрифты соавторов

Статус: **предложено**. Дата:2026-10-12UTC.

## Контекст

Одинаковый текст с разными font metrics не гарантирует одинаковые страницы. Embedded fonts не всегда имеют разрешённое распространение.

## Варианты и рекомендация

A: licensed common font set; B: embedded per-font rights check; C: fallback с видимым баннером и предупреждением layout divergence. Рекомендация A для defaults и B только по проверенным правам; specific fallback claims ещё читать.

## Критерии, данные и последствия

Hash/metrics font identity в document metadata; no unauthorized font upload; missing font golden on both OS; PDF subsets rights. Legal вопросы владельцу/юристу.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
