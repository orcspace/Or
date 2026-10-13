# Аудит фазы 0 — промежуточный

Фаза **не завершена**. Windows-first авторизован владельцем. Это промежуточный отчёт, не приёмка.

| Часть | Статус | Проверка / ограничение |
|---|---|---|
| Спецификация/реестр/ADR/facts/планы |подготовлено|442source points/350features, registry consistency локально passed|
| Rust workspace1.98.1/edition2024 |проверено локально и в первом Windows CI|core без внешних deps; format/clippy/tests прошли, GUI отсутствует|
| Theme/platform core |13 unit tests локально проходят|полные semantic tokens, system/mode/high contrast, AA contrast, reduced motion, version/build policy; no GUI/native probing|
| Spec registry tooling |локально checked|4negative tests прошли; detects missing IDs/drift/no verification/fake done|
| Qt/CXX-Qt/CMake |не реализовано|Qt age/QML restriction unresolved, build compatibility unverified|
| QML Theme.qml/оболочка |не реализовано|ADR-0003 owner interpretation needed|
| Mica/Snap/title/native APIs |не реализовано|minimum22000 vs documented22621 ADR-0007|
| Шрифты/Lucide assets |8 TTF и 36 SVG включены|версии/источники/SHA256/OFL/ISC; asset checker passed; Qt registration не реализована|
| Window themes/ru-en/a11y/IME |не проверено|real Windows11 GUI absent; Mac deferred|
| Signed installer |нет|signing identity/model pending; no fake signature|
| Performance |не измерено|no native OS bench, no1.5s/16ms/fps claims|
| Mandatory spikes |CRDT частично выполнен|4 tests и Linux development measurements; Windows CI готов; остальные 6 plans; phase1 blocked layout/target CRDT acceptance|

## Красная команда первого прохода

Windows CI первого asset/CRDT прогона выявил CRLF checkout: SVG SHA256 и frozen Cargo.lock SHA256 расходились с audited bytes. Сборки остановлены корректно (runs 38338911562/38338911510). Исправление: .gitattributes с LF для текста и binary для TTF; проверки не ослаблены. Требуется успешный повторный Windows CI.

Spec checker подвергнут purposeful corruption: hard platform text change, untracked new clause, done without evidence, empty test route. Все четыре детектируются. Это не red team runtime импорт/гонки/MCP: этих runtime components пока нет.

Новые риски: stale MCP2025spec corrected to2026-07-28, SDK/client compatibility unverified; history compaction vs offline/time machine; atomic apply_ops vs irreversible side-effects; origin/role/cursor/Unicode semantics missing.

## Следующий шаг

Проверить обновлённый core CI и CRDT Windows workflow. Предыдущий core Windows CI успешен: https://github.com/orcspace/Or/actions/runs/38331138940 . Для окна нужны owner ADR-0003 и Mica decision; Qt age соблюдается по ADR-0002. Затем Qt/CXX-Qt package/license pin и clean Windows shell spike. Mac отложен. Поздние фазы разрешены владельцем; фаза 1 не стартует до результатов layout/CRDT и phase 0 acceptance.

