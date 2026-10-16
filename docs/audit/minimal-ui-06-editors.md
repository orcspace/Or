# Минималистичный интерфейс06 — Docs и Slides

Основа: codex/minimal-ui-06,5078a6d. Этот пакет меняет интерфейс, без квоты50 и без изменений model/io/ops.

## Что изменено

Docs: Office-вкладки сохранены. Вместо постоянно раскрытой двухрядной ленты — одна горизонтально прокручиваемая строка: вставка, шрифт/размер, Ж/К/Ч и цвет, меню стилей/абзаца, поиск. Cut/copy/format painter, остальные font effects, highlight/clear и replace/select-all доступны через «Ещё» с именованными подразделами. Quick и подробные controls имеют разные ID scopes. Галерея стилей стала компактным списком с типографическим preview. Insert table/image/link остаются прямыми кнопками с прежними popup; headers/page numbers/symbols/date/page commands остаются в меню. Layout/View/Table controls доступны через именованные popup. Белая страница и её тонкая граница сохранены, декоративная тень убрана.

Slides: Главная — new slide/layout, font/size/B/I/color, «Ещё», «Объект» и «Выравнивание». Сохранены clipboard, duplicate/delete slide/object, group/order/rotation, presets и alignment/distribution. Insert table/image/text, shapes/chart menu, themes и start presentation доступны через прежние action handlers. View panels/zoom и параметры показа скрываются в меню. Sidebar без внешней карточки: тихие заголовки, меньше заливки; thumbnail hover и активный accent marker сохранены. Properties: slide details, geometry/rotation, appearance и animation раскрываются по необходимости. Сам текст объекта и данные table/chart остаются доступны с прежними per-object/cell ID и reconcile/remember.

Вызов команды Docs из popup закрывает меню перед возвратом keyboard focus в canvas/активную table cell либо find field. TableCommand сохраняет прежний focus path; Slides menu «Редактировать текст» закрывает popup перед begin_edit. Неподдерживаемые rotate table/chart и вложенные Docs tables не объявлены работающими, существующие disabled/error guards сохранены. Сочетания клавиш и единый commit через ops не менялись.

## Проверка и независимое review

rustfmt этих двух editor.rs выполнен под claims; статические rustfmt --check и git diff --check проверены. Никаких builds/cargo check/test/clippy, GUI/app, установщика, CI/release и запусков регрессий не выполнялось. Новые тесты, повторяющие стили, не добавлялись.

Второй агент перечитал полный Docs/Slides UI diff: action handlers/guards, quick/menu IDs, TableCommand и refocus/open_find/close_all, stable text/cell/property IDs. Подтверждённых блокеров по исходнику не нашёл.

Независимое чтение соседних Sheets/chat: command_action/commit, formula/location IDs, hidden/protected guards, selection attachment, stop/retry/login и группировка технических Tool событий сохранены. Composer-текст при MCP off уточнён до «Инструменты документов выключены»: context/выделение продолжают передаваться. Финальное удаление декоративных provider dots не скрывает Missing/LoggedOut labels и status tooltips.

Независимое чтение root App/chrome: новые palette/accent/surface/sidebar helpers, hit areas и focus stroke сверены с локальным egui0.36.2 Widgets::style. Переданы два замечания: вернуть truncate для bounded doc_tab (horizontal default Extend может игнорировать предложенную ширину add_sized), обновить pinned empty-state hint после переноса pin в ···. Обе поправки ведущего перечитаны; отдельный dirty marker сохраняется при обрезании длинного title, Ctrl+Shift+A соответствует исходному shortcut. Оставшихся подтверждённых блокеров по исходнику не найдено.

## Основания и ограничения

Первичные источники, сверены16.10.2026: [Windows Command bar](https://learn.microsoft.com/en-us/windows/apps/design/controls/command-bar), [Fluent2 Toolbar](https://fluent2.microsoft.design/components/web/react/core/toolbar/usage), [Fluent2 Layout](https://fluent2.microsoft.design/layout), [Fluent2 Typography](https://fluent2.microsoft.design/typography). Применён принцип основных/вторичных команд и4px ритм; это собственный native egui UI, не перенос Fluent React/WinUI.

Полная визуальная, keyboard/IME, touch/accessibility и Office-приёмка не доказаны. Узкая ширина допускает горизонтальную прокрутку command row; сложные popup/длинные локализованные строки требуют последующего разрешённого запуска. Светлая и тёмная тема получают общие tokens от chrome; белый document content не затемнялся. Новых зависимостей, шрифтов или растровых ассетов нет.

Финальная малая дельта Docs/Slides использует общие chrome::sidebar_frame (белая theme surface) и chrome::separator (явная18px высота в quick row вместо заполнения доступной высоты). Action handlers и IDs при этом не менялись. Все claims освобождены после edits/fmt.
