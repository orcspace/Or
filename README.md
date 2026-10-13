# TrineDocs

Нативный офисный пакет: Docs, Sheets, Slides; общий Rust core, Qt Quick/QML, CXX-Qt, CRDT, MCP и совместная работа.

**Статус: фаза 0 начата, не принята. Рабочего GUI/EXE пока нет.** Сейчас работа и проверки ориентированы на Windows 11 x64 (build 22000+); macOS arm64 остаётся конечной платформой, реализация и проверки отложены владельцем.

## Что есть

- Обязательная спецификация [AGENTS.md](AGENTS.md), [MCP](docs/MCP.md), [SPEC-AUDIT](docs/SPEC-AUDIT.md).
- [План фазы 0](docs/plan/phase-0.md), [первичный аудит](docs/audit/00-spec-audit.md), [факты](docs/facts.md), [ADR](docs/decisions/README.md).
- [Реестр требований](docs/traceability.md), [паритет](docs/parity.md), [проверки приёмки](docs/testing/acceptance.md), [модель угроз](docs/security/threat-model.md).
- Dependency-free Rust workspace: политика версий Windows и выбора темы, тесты контраста/высокого контраста/уменьшения движения. Это логика для будущего GUI, не приложение.
- Проверка целостности спецификации: Python stdlib, без сторонних пакетов.
- Полный набор семантических токенов темы в Rust; 8 файлов Inter/JetBrains Mono и 36 Lucide SVG с закреплёнными источниками, лицензиями и проверкой SHA-256. Регистрация в Qt ещё не реализована.
- [Изолированный CRDT-спайк](spikes/crdt/README.md): Loro, три реплики, rich text, локальный undo, 100k ячеек, миллион записей истории. Это эксперимент модели, а не готовый редактор.
- CI для документации и Rust core на Windows runner. Успех этого CI **не** доказывает приёмку GUI, Windows 11 на реальной машине или macOS.

## Проверки

```sh
python tools/spec_registry.py check
python -m unittest discover -s tools/tests -v
python tools/check_assets.py
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

Toolchain закреплён в `rust-toolchain.toml`. Сейчас у core нет внешних Cargo-зависимостей. Кандидаты Qt/CXX-Qt не скачиваются и не линкуются до прохождения блокеров ADR-0002/0003. CMake/bridge/QML/native window интеграция ещё не реализованы.

Трассировка генерируется из [requirements.json](docs/requirements.json) и аннотаций в спецификации. После изменения текста сохранить ID, обновить соответствующую запись реестра и запустить `python tools/spec_registry.py render`; `check` выявляет рассинхронизацию.

## Что блокирует окно

1. Qt 6.12.0 опубликован 2026-09-30 и на дату начала ещё младше 14 суток: [ADR-0002](docs/decisions/0002-qt-release-age.md).
2. QML-привязки используют JavaScript, а JavaScript полностью запрещён: [ADR-0003](docs/decisions/0003-qml-javascript.md). Толкование запрета меняет только владелец.
3. Документированный Mica API требует build 22621 при минимуме 22000: [ADR-0007](docs/decisions/0007-mica-minimum-build.md).

Ни один редактор, импорт/экспорт, MCP-транспорт или sync-сервер пока не считается реализованным. CRDT-спайк запущен в Linux development-среде; Windows CI ядра и CRDT прошёл (21 тест); raw measurements сохранены в docs/spikes/results/windows-ci-run.json. Это не заменяет Windows 11 GUI-приёмку. Остальные обязательные спайки остаются планами.

## Лицензия проекта

Владелец ещё не выбрал модель распространения и лицензию исходников (ADR-0008). [licenses.md](docs/licenses.md) содержит факты и список ожидающей проверки сторонней составляющей; он не является разрешением на включение непроверенных зависимостей.

