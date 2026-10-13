# ADR-0017: Буфер обмена и drag/drop

Статус: **предложено**. Дата:2026-10-12UTC.

## Контекст

HTML/RTF/text/images differ across Windows/macOS and Office clients; lossy plain fallback не P0 parity.

## Варианты и рекомендация

A: internal+HTML+RTF+plain+images with priority negotiation; B: plain only (не отвечает заданию). Рекомендация A, native code thin adapter, all parsed content via import worker/ops.

## Критерии, данные и последствия

Windows actual registered Office formats, CF_HTML offset validation, bounded image/RTF parse, multi-range/transpose/values-only; mac pasteboard позже. Roundtrip Word/Excel/PPT/browser; no external URI fetch without permission; malformed paste cannot crash UI.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
