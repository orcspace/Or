# ADR-0021: Трёхстороннее слияние локального файла

Статус: **предложено как реализация обязательного дополнения**. Дата:2026-10-12UTC.

## Контекст

Watcher reload/keep без common base не обеспечивает требуемое merge local/disk/base. Office sidecar должен идентифицировать base даже после rename/copy.

## Варианты и рекомендация

A: preserve base hash/snapshot, parse disk in worker, semantic compare, user resolves conflicting spans; B: two-way reload only (отвергнуто SPEC-AUDIT). Рекомендация A, не обещать автоматический merge любых unsupported OOXML parts.

## Критерии, данные и последствия

Race save/watch/reparse, partial writes, conflicting styles/assets/opaque parts, content hash identity, cannot overwrite newest disk без confirmation. Tests T-02-FILES and CRDT convergence; exact semantic-conflict UI after model spike.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
