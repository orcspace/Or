# Сторонние компоненты в репозитории

## Inter 4.1

Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter).

SIL Open Font License 1.1: полный исходный текст и copyright — [assets/fonts/Inter/OFL.txt](assets/fonts/Inter/OFL.txt). Четыре TTF включены без изменений; источник и SHA-256 — в [manifest](assets/fonts/manifest.json).

## JetBrains Mono 2.304

Copyright 2020 The JetBrains Mono Project Authors (https://github.com/JetBrains/JetBrainsMono).

SIL Open Font License 1.1: [assets/fonts/JetBrainsMono/OFL.txt](assets/fonts/JetBrainsMono/OFL.txt). Четыре TTF включены без изменений; источник и SHA-256 — в manifest шрифтов.

## Lucide 1.48.0

Copyright (c) 2026 Lucide Icons and Contributors. Сохраняется также upstream notice Feather: Copyright (c) 2013-present Cole Bemis.

ISC: оба полных upstream notices — [assets/icons/LICENSE.txt](assets/icons/LICENSE.txt). Для 36 SVG изменена толщина контура на 1.75 и нормализован XML. Immutable upstream commit, исходные/производные SHA-256 — в [manifest](assets/icons/manifest.json).

## GridCraft — библиотека функций листа

Copyright (c) 2026 ArtCraft Team and the GridCraft contributors (https://github.com/storytold/gridcraft).

MIT OR Apache-2.0, выбран MIT: полный текст — [crates/trinedocs/src/sheets/xlfn/LICENSE-MIT](crates/trinedocs/src/sheets/xlfn/LICENSE-MIT). Модули `gridcraft-functions` и часть `gridcraft-core` скопированы с адаптацией путей и удалением serde/wasm; в каждом файле — заголовок с источником. Решение: ADR-0028. Текст лицензии добавляется в THIRD-PARTY-NOTICES.txt установщика.

## Изолированный CRDT-эксперимент

Loro 1.16.2 (MIT) и 155 других внешних пакетов закреплены отдельным Cargo.lock. Полный реестр заявленных лицензий/источников: [docs/dependencies/crdt-spike.md](docs/dependencies/crdt-spike.md). Сборки эксперимента не входят в выпуск приложения. Source notices всего графа ещё нужно собрать перед распространением его бинарников.

Qt, CXX-Qt, Skia и остальные кандидаты пока не включены. При их подключении расширить этот файл и комплект лицензий; текущий перечень не заявляет завершённую юридическую проверку будущего выпуска.
