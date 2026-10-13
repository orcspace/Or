# Паритет Docs / Word

Дата: 2026-10-13. Это матрица фактической реализации, а не заявление полного паритета с Microsoft Word. Указание владельца расширяет объём до всех ожидаемых функций; незавершённые пункты остаются в объёме.

Первоисточники:
- [Microsoft Word help & learning](https://support.microsoft.com/en-us/word/): основные разделы Format text, Pages & layouts, Pictures, Save & print, Mail merge; также диаграммы, диктовка, исправления и совместная работа.
- [Track changes in Word](https://support.microsoft.com/en-us/word/training/track-changes-in-word): отдельные режимы отслеживания и принятия исправлений. Undo-группы агента не эквивалентны этому механизму.
- [Word keyboard shortcuts](https://support.microsoft.com/en-us/office/keyboard-shortcuts-in-word-95ef89dd-7142-4b50-afb2-f762f663ceb2).
- [Structure of a WordprocessingML document](https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document).

| Область | Приоритет | Реализация | Ограничения / проверка |
|---|---|---|---|
| Создание и Unicode-текст | P0 | Doc с абзацами, форматированными runs, стабильными u64 ID | Смещения в Unicode scalar values; полная grapheme-навигация пока отсутствует |
| Собственный постраничный редактор | P0 | Canvas, разбиение длинного абзаца по строкам между страницами | Метрики системного Arial/Segoe UI; нет комплексного bidi/shaping и переносов по словарю |
| Мышь и клавиатура | P0 | Клик, Shift, drag, двойной/тройной клик, стрелки, Ctrl-слова, Home/End, PgUp/PgDn, Backspace/Delete, Tab списков | Ручной тест Windows обязателен; PgUp/PgDn перемещают 25 строк |
| IME | P0 | Preedit/commit/delete-surrounding, IME cursor rect, безопасная отмена пустым commit | Японский IME и сложные combining sequences ещё не проверены вручную |
| Буфер обмена | P0 | Текст наружу, внутренние runs сохраняются при вставке совпадающего текста в том же редакторе | CF_HTML/RTF Windows-буфер отсутствует; междокументный rich copy отсутствует |
| Undo/redo | P0 | Через общий ops приложения, редактор возвращает изменённую копию | Группирование и авторы проверяются общим слоем |
| Текстовое оформление | P0 | B/I/U/S, размер, цвет, выделение, super/sub, ссылки, очистка/образец | Имя шрифта хранится и экспортируется в DOCX; экран/PDF используют общий системный шрифт |
| Абзацы | P0 | L/C/R, интервалы, отступы, стили Normal/Title/Heading1–3/Quote, списки 9 уровней, разрыв страницы | Justify импортируется/экспортируется, экран пока отображает как Left; команда Justify в UI не предлагается |
| Markdown-ввод | P0 | #, ##, ###, -, *, 1., > с пробелом превращаются в стили/списки | Это ограниченный набор автозамены, не полноценный Markdown parser |
| Поиск/замена и структура | P0 | Ctrl+F/H, линейный Unicode scalar поиск с регистром, следующий результат, заменить/всё, список заголовков; bounded find_page для MCP | Поиск только в основном тексте; таблицы не включены, regex отсутствует |
| Статистика | P0 | Число слов, знаков, страниц, позиция курсора | Статистика обновляется из модели; производительность на 100 страницах не измерена |
| Таблицы | P0 | Ячейки со стабильными ID и абзацами, добавление/удаление строк/столбцов, fill/border, DOCX tbl | Rich runs/размер/цвет сохраняются при инспекторной правке и отрисовке; длинные строки продолжаются между страницами. Rich canvas ячеек и merge/split cells отсутствуют; complex table layout остаётся ограниченным |
| Картинки | P0 | PNG/JPEG блок после абзаца, фоновый picker/read, размер, описание, DOCX media/wp:inline | Плавающие картинки преобразуются в отдельные блоки; wrap/crop/rotate/эффекты отсутствуют |
| Страница | P0 | A4/Letter/A3/Legal, поворот, поля, единый header/footer, номера страниц | Разделы DOCX приводятся к одной настройке; разные колонтитулы первой/чётной страницы отсутствуют |
| DOCX/DOTX/DOCM импорт | P0 | rPr/pPr, наследование styles, numbering, таблицы, embedded images, core properties, section/page | Макросы/OLE не исполняются; unsupported content не сохраняется непрозрачно, выводится список потерь |
| DOCX/DOTX/DOCM экспорт | P0 | OOXML document/styles/numbering/media/core/header/footer/rels | Макросы удаляются; unsupported original XML теряется. Сохранение после импорта требует предупреждения приложения |
| TXT | P0 | UTF-8/UTF-16/Windows-1251 чтение, UTF-8 запись | Экспорт убирает форматирование |
| Markdown | P0 | Заголовки/списки/quote, basic bold/italic/strike; экспорт таблиц | Markdown таблицы/ссылки/изображения не импортируются полностью, вложенные inline patterns ограничены |
| HTML | P0 | Локальный data-only импорт основных tags; styled spans и таблицы экспортируются | Scripts не исполняются; сложный CSS, сетевые изображения и таблицы не импортируются |
| PDF/печать | P0 | Doc::pages() выдаёт общие Page primitives; PDF реализует общий слой | Нет внешней проверки совпадения со скриншотами; kerning/shaping и baseline требуют визуального ревью |
| Сноски/концевые сноски | P0 | Не реализовано | Нужна отдельная модель и layout потока сносок |
| Колонки/разделы/keep rules/вдовы-сироты | P0 | Не реализовано | DOCX выдаёт предупреждение по многоколоночной вёрстке и нескольким разделам |
| Поля/TOC/перекрёстные ссылки/закладки | P0 | Импорт сохранённого текста полей, номера страниц собственным механизмом | Динамическое обновление полей и TOC отсутствует |
| Review: комментарии/Track Changes/сравнение | P0 | Не реализовано в Docs-модели | Требует версий диапазонов, anchored comments и tracked insert/delete; AI undo не подменяет Track Changes |
| References: citations/bibliography/captions/index | P1 | Не реализовано | Требуются модели и форматирование по стандартам |
| Mail merge: письма/конверты/этикетки | P1 | Не реализовано | Источники данных, шаблоны полей и печатные размеры |
| Shapes/SmartArt/charts/WordArt/equations | P1 | Не реализовано | Текущая модель поддерживает только text/table/image |
| Правописание/грамматика/тезаурус/перевод/диктовка | P1 | Не реализовано | Языковые и системные службы; офлайн функциональность проверять отдельно |
| Styles: пользовательские стили/темы/шаблоны | P1 | Частичный импорт и предустановленные styles | UI создания/редактирования named styles, docDefaults advanced и theme fonts отсутствует |
| View: split/read/web/draft/outline/ruler | P1 | Постраничный вид, zoom, список заголовков | Остальные режимы не реализованы |
| Accessibility/Narrator | P0 | Egui оболочка | Полное accessibility tree содержимого собственного canvas не реализовано |
| Защита/шифрование/подпись/IRM/макросы/add-ins | P1 | Не реализовано | Макросы и OLE сознательно не исполняются; зашифрованный Office input возвращает ошибку ZIP |
| Совместная работа/Drive/macOS | P2 | Отложено владельцем в актуальном плане | Не считается выполненным паритетом |

Тесты в src/docs: Unicode insert/delete; многострочная вставка с сохранением formatting и ID; частичное форматирование; неперекрывающаяся replacement; межстраничное разбиение; table IDs; rebase cursor после правки агента; навигация по русским словам; Markdown input; DOCX round-trip с таблицей/цветом/стилем/header; UTF-16/Windows-1251; DTD/XXE rejection; malformed ZIP/XML; unsupported extension. Добавлены регрессии вложенных ID/next_id, плохой геометрии, длинной строки таблицы без потерь, hyperlink identity, mixed HTML order, footer PAGE, inline page break и 50 синтетических DOCX variations. Результаты общего cargo test указывает основной release-review, до его выполнения зелёный статус не заявляется.

Непроверено: корпус 50+ реальных DOCX, Office compatibility validation, fuzzing, 100-page p95/performance, Windows IME, Narrator, ручной clipboard с Word и пиксельное сравнение PDF/экран. До этих проверок и закрытия P0 нельзя называть Docs полной заменой Word.

Второе ревью: [review-2-docs.md](audit/review-2-docs.md). Исправлены rich table edits/render, loss warnings, caret rebase через split/merge, DOCX explicit false style overrides, image container paragraph growth, outer XML whitespace/multiple roots, IME cancellation, clipboard/focus filter, local date, bounded linear search и Arc layout cache. Общую сборку/тесты и внешнее QA отражает release-review; полная приёмка Word остаётся открытой.
