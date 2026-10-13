# ADR-0019: Word compatibility modes

Статус: **предложено; standard audit pending**. Дата:2026-10-12UTC.

## Контекст

Internal model!=OOXML. docDefaults/latentStyles/abstractNum/num/w:compat могут влиять на inherited formatting/pagination; одинаковые run properties не гарантируют fidelity.

## Варианты и рекомендация

A: normalized style inheritance+versioned compatibility traits preserving opaque parts; B: flatten formatting (losses); C: renderer by Office compat family. Рекомендация исследовать A; no silent flatten claim lossless.

## Критерии, данные и последствия

ECMA-376/ISO29500+MS-OI29500 per-file fixtures, effective-style golden, numbering continuation/restart, unknown relationships/parts/macros byte-preserve, package signature behavior. Full complex layer data from spike4.1 before phase1.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
