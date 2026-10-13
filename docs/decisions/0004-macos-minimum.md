# ADR-0004: Минимальная macOS

Статус: **принято по указанной в задании таблице**, проверка 2026-10-12 UTC.

[Supported Platforms Qt 6.12](https://doc.qt.io/qt-6/supported-platforms.html): target macOS 14.4+, build Xcode 16/macOS 15 SDK или выше. TrineDocs выбирает только arm64, хотя Qt поддерживает также Intel.

Решение: deployment target 14.4 при будущей Mac-сборке, не 12/13/14.0 из памяти. По ADR-0001 пока не реализовано и не протестировано. Перепроверять таблицу при pin/upgrade Qt.

