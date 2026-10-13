# Изменения спецификации

| ID | Дата UTC | Что/почему | Источник/решение |
|---|---|---|---|
| CHANGE-001 |2026-10-12|Сохранены AGENTS/MCP/SPEC-AUDIT; редакционная нормализация длинных перечислений в компактные пункты без удаления целевого объёма. Источником требований остаются эти файлы; это не заявление о реализации.|Полный текст заданий владельца в текущей сессии|
| CHANGE-002 |2026-10-12|Windows-first; Mac работа/замеры позже, конечная платформа/общий DoD сохранены.|Явное указание владельца, ADR-0001|
| CHANGE-003 |2026-10-12|macOS minimum заполнен14.4, не взят из памяти.|FACT-002, ADR-0004|
| CHANGE-004 |2026-10-12|MCP target2026-07-28 вместо устаревшего «сейчас2025-11-25»; Tasks opt-in extension; SDK/client support не утверждается.|[latest](https://modelcontextprotocol.io/specification/latest), FACT-006|
| CHANGE-005 |2026-10-12|Конфликты release-age, JavaScript/QML, Mica22000 помечены; требования не ослаблены.|ADR-0002/0003/0007|
| CHANGE-006 |2026-10-12|Внесены прямые дополнения: separate import worker; external three-way merge; BYOK default; layout!=paragraph; Docs/P1/Google/Slides start gates.|SPEC-AUDIT §3.C, ADR-0005/0006|
| CHANGE-007 |2026-10-12|REQ IDs: AGENTS REQ-XX-NNN, MCP REQ-MXX-NNN, аудит REQ-AXX-NNN; namespaces предотвращают collisions. ID аннотирует каждый пункт/строку требований, вложенные feature catalog entries имеют отдельные PAR IDs.|SPEC-AUDIT §1 уточнение формата (low-risk)|
| CHANGE-008 |2026-10-12|Различены initial audits/spike plans и реальные результаты; тестовые методы запланированы, не выданы за passed tests.|SPEC-AUDIT honesty/DoD|
| CHANGE-009 |2026-10-12|В parity добавлены Explorer/Finder preview, uninstall/migrations/fonts/date semantics и обнаруженные Office alt-text/side-by-side gaps; финальный exhaustive ribbon audit ещё открыт.|SPEC-AUDIT; FACT-017|
| CHANGE-010 |2026-10-12|Rust1.98.1 pinned; latest1.99.0 младше14дней. CXX-Qt0.10.0 — проверенный latest candidate, совместимость pending.|FACT-003/011|

Высокорисковые предложения в ADR-0010 (transactions/Unicode/rights) ещё не приняты. Фазы не перенумерованы, mandatory функции/производительность не снижены. Qt/GUI stack не заменён.


## Дополнения2026-10-13

| ID | Дата UTC | Что/почему | Источник/решение |
|---|---|---|---|
| CHANGE-011 |2026-10-13|Владелец разрешил продолжать все фазы до9; late-phase scope-gate снят, критерии приёмки и жёсткие ограничения сохранены. Исходная нумерация остаётся.|Текущая сессия, ADR-0005|
| CHANGE-012 |2026-10-13|Полные Rust theme tokens,8cursor colors,mode accents,system highcontrast+selection/button text,motion;4REQ частично реализованы, GUI остаётся pending.|crates/shell-core/src/theme.rs,13local Rust tests|
| CHANGE-013 |2026-10-13|Inter4.1/JBM2.304 OFL-1.1 иLucide1.48.0 ISC pinned assets; hashes/notices, passive SVG validation.|assets manifests, official release URLs|
| CHANGE-014 |2026-10-13|Isolated CRDT experiment Loro1.16.2;156exact pins audited via crates.io;17younger dependencies constrained to aged versions. SemVer build metadata (+wasi-...) не prerelease: validator исправлен.|docs/dependencies/crdt-spike.json,spikes/crdt/Cargo.lock|
| CHANGE-015 |2026-10-13|Bench import-only latency отделена от open+first text/style materialization; original-state comparison не включается в measured open. Development Linux results не target acceptance.|spikes/crdt/src/main.rs|
