# Реестр ADR

| ADR | Решение | Статус | Что блокирует |
|---|---|---|---|
| 0001 | Windows-first, macOS позже | принято владельцем 2026-10-13 | Mac DoD отложен |
| 0002 | Qt release-age | принято: соблюдаем запрет; исключение только владелец | Qt install/build |
| 0003 | QML и запрет JavaScript | ждёт владельца | интерактивная QML-оболочка |
| 0004 | Минимальная macOS 14.4 | принято по первоисточнику | Mac validation позже |
| 0005 | Все фазы до9, исходный порядок | разрешено владельцем2026-10-13 | criteria gates сохраняются |
| 0006 | Изоляция импорта | принято как обязательное дополнение | конкретная sandbox-реализация после спайка |
| 0007 | Mica на build 22000 | ждёт владельца | DoD нативной Windows-оболочки |
| 0008 | Распространение/лицензия/подпись | ждёт владельца | production release |
| 0009 | Название/расширения | ждёт владельца | public naming/file associations |
| 0010 | Транзакции/права/Unicode MCP | предложено | полные схемы MCP |
| 0011 | CRDT history/compaction | предложено, спайк 4.2 | очистка истории, full time machine |
| 0012 | Layout и CalcEngine | предложено, спайки 4.1/4.5 | фазa 1 и formulas completion |
| 0013 | Шрифты соавторов | предложено | одинаковая вёрстка collaborative |
| 0014 | Update/uninstall/migrations | предложено | release/update |
| 0015 | Корпус/эталоны | предложено | acceptance round-trip/layout |
| 0016 | Сервер/E2EE/операции | ждёт владельца | cloud cost/E2EE |
| 0017 | Clipboard/drag/drop | предложено | interoperable P0 clipboard |
| 0018 | Масштаб/Excel semantics | предложено | корректность CalcEngine |
| 0019 | Word compatibility | предложено | faithful layout/OOXML |
| 0020 | BYOK/AI proxy/privacy | BYOK принято заданием; proxy ждёт владельца | paid AI proxy |
| 0021 | Локальный файл three-way merge | предложено | compare/reload merge |
| 0022 | Windows ARM emulation/native | ждёт владельца для native; emulation unvalidated | ARM support claim |
| 0023 | Explorer/Finder preview | предложено, P1 | native preview |
| 0024 | Loro import atomicity | предложено, воспроизведено спайком | нельзя считать Err rollback; ops/sync нужны разные контракты |
| 0026 | Картинки из буфера обмена (arboard) | предложено | Ctrl+V картинок в Docs/Slides/Sheets |
| 0028 | Перенос функций WordCraft/GridCraft/DeckCraft | принято | библиотека функций листа GridCraft, regex |

Статус «предложено» не является разрешением ослабить требование. Сроки и затраты, если не измерены, не выдаются за числа.

