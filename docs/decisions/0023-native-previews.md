# ADR-0023: Превью ОС

Статус: **предложено; P1 сохранён**. Дата:2026-10-12UTC.

## Контекст

SPEC-AUDIT добавляет Windows preview handler/Finder QuickLook. Это отдельный boundary reading potentially hostile documents, не только Home cached thumbnail.

## Варианты и рекомендация

A: isolated thumbnail renderer + native shell provider; B: reuse full desktop process (coupling/hostile input risk); C: cached only preview с explicit cache miss. Рекомендация исследовать A.

## Критерии, данные и последствия

Microsoft/Apple exact APIs/platform versions/signing/security ещё проверить. No macros/network; file-size limits; missing app/cache fallback; Explorer host crash containment. P1 parity, Mac implementation deferred.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
