# ADR-0014: Обновление, миграции и удаление

Статус: **предложено**. Дата:2026-10-12UTC.

## Контекст

Sidecars/CRDT/native container/client MCP configs переживают versions; partial update/delete может испортить данные.

## Варианты и рекомендация

A: versioned schema+copy migration+rollback; B: in-place irreversible migration. Рекомендация A; updater mechanism WinSparkle/MSIX/Sparkle ещё требует release/license review и владельца.

## Критерии, данные и последствия

Подписанные manifests/packages, rollback after launch failure, migration atomicity/future-version read-only or rejection, backup. Uninstall opt-in data deletion, remove only owned MCP/startup entries, never чужие files/config entries; keychain/cache choices visible. Clean-machine tests before release.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
