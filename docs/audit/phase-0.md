# Аудит фазы 0 — промежуточный

Фаза **не завершена**. Windows-first авторизован владельцем. Это промежуточный отчёт, не приёмка.

| Часть | Статус | Проверка / ограничение |
|---|---|---|
| Спецификация/реестр/ADR/facts/планы |подготовлено|442source points/350features, registry consistency локально passed|
| Rust workspace1.98.1/edition2024 |source готов для проверки|no third-party deps; native build/format/clippy/tests ещё предстоит CI|
| Theme/platform core |source с9unit tests|theme live resolution, highcontrast override, AA contrast, motion, version/build policy; no GUI/native probing|
| Spec registry tooling |локально checked|4negative tests прошли; detects missing IDs/drift/no verification/fake done|
| Qt/CXX-Qt/CMake |не реализовано|Qt age/QML restriction unresolved, build compatibility unverified|
| QML Theme.qml/оболочка |не реализовано|ADR-0003 owner interpretation needed|
| Mica/Snap/title/native APIs |не реализовано|minimum22000 vs documented22621 ADR-0007|
| Шрифты/Lucide assets |не включены|exact license audit pending|
| Window themes/ru-en/a11y/IME |не проверено|real Windows11 GUI absent; Mac deferred|
| Signed installer |нет|signing identity/model pending; no fake signature|
| Performance |не измерено|no native OS bench, no1.5s/16ms/fps claims|
| Mandatory spikes |не запущены|семь plans, ноль results; phase1 blocked layout/CRDT|

## Красная команда первого прохода

Spec checker подвергнут purposeful corruption: hard platform text change, untracked new clause, done without evidence, empty test route. Все четыре детектируются. Это не red team runtime импорт/гонки/MCP: этих runtime components пока нет.

Новые риски: stale MCP2025spec corrected to2026-07-28, SDK/client compatibility unverified; history compaction vs offline/time machine; atomic apply_ops vs irreversible side-effects; origin/role/cursor/Unicode semantics missing.

## Следующий шаг

Проверить core CI. Для окна нужны owner ADR-0003 и Mica decision; Qt age соблюдается по ADR-0002. После этого точный Qt/CXX-Qt package/license pin и clean Windows shell spike. Mac work по отдельному переходу владельца. Фаза1 не стартует до результатов layout/CRDT и phase0 acceptance.

