# CRDT-спайк (эксперимент, не модель продукта)

Exact Loro1.16.2, source checksum/license проверяются перед сборкой. Отдельный workspace: production фаза1 не начинается и dependency-free shell-core не меняется.

Синтетические workloads:100explicit page groups/200formatted blocks;1000rows×100cols стабильных IDs/100kcells;100slides×5shapes;1mcommitted map edits. Page groups не доказывают фактическую пагинацию. Ячейки строковые: formulas/number formats не исследуются здесь. Метрики CRDT не key-to-pixel/GUI fps.

```sh
cargo test --locked --manifest-path spikes/crdt/Cargo.toml
cargo run --locked --release --manifest-path spikes/crdt/Cargo.toml -- docs
cargo run --locked --release --manifest-path spikes/crdt/Cargo.toml -- sheets
cargo run --locked --release --manifest-path spikes/crdt/Cargo.toml -- slides
cargo run --locked --release --manifest-path spikes/crdt/Cargo.toml -- history
cargo run --locked --release --manifest-path spikes/crdt/Cargo.toml -- merge
```

Проверки convergence с разным порядком/duplicate updates, rich-text marks, stable-cell values, local undo при concurrent remote edits, Unicode scalar/UTF16/UTF8 units, snapshot restore. Полная grapheme-safe адресация MCP, случайные property tests, offline/shallow rebootstrap и50000author patterns ещё не выполнены.

JSON results являются реальными измерениями конкретного процесса/стенда. Перед запуском требуются frozen Cargo.lock, license/age audit всех transitive dependencies. WindowsServerCI — инфраструктурный результат, не приёмка на Windows11x64; Mac позже по ADR-0001. Заносить failed/no-go и ограничения в docs/spikes/02-crdt.md.
