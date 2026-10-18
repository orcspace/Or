# Матрица паритета

Первичный feature inventory по заданию и первым проверенным пробелам. Полная сверка всех вкладок Office ещё не завершена. Внутри каждого PAR-пункта варианты поведения проверяются отдельно. Ни одна функция не объявлена готовой. P2 не удаляется из конечного объёма; поздние фазы ждут решения владельца ADR-0005.

Всего функций/вариантов: **366**.

| PAR ID | Режим | Функция | P | Фаза | REQ | MCP / пробел | Import | Export | Статус |
|---|---|---|---|---|---|---|---|---|---|
| PAR-COM-001 | Общее | Rich clipboard internal/HTML/RTF/text/image | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-002 | Общее | Вставка из Word/Excel/PowerPoint/браузера | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-003 | Общее | Вставить без формата | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-004 | Общее | Специальная вставка значения/формулы/форматы/transpose | P0 | 1 | REQ-11-002 | UI typed clipboard: все/значения/ввод/форматы, transpose; относительные merges в режиме Всё, partial cut/несовпадающее пересечение отклоняются атомарно. GAP: внешние Office/HTML/RTF, cut-reference movement, отдельная MCP schema и runtime acceptance. | internal clipboard | n/a | partial_unverified |
| PAR-COM-005 | Общее | Поиск/замена case/words/regex/format | P0 | 1 | REQ-11-002 | Sheets literal Unicode search; Docs body search от курсора, назад/вперёд, Enter/Shift+Enter и checked bulk replace с preflight. Существующие MCP tools сохраняются; GAP: Docs tables, words/regex/format, остальные режимы и runtime acceptance. | n/a | n/a | partial_unverified |
| PAR-COM-006 | Общее | Zoom | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-007 | Общее | Линейки | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-008 | Общее | Сетка/направляющие | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-009 | Общее | Орфография ru/en | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-010 | Общее | Автозамена | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-011 | Общее | Спецсимволы/эмодзи | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-012 | Общее | Гиперссылки | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-013 | Общее | Комментарии | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-014 | Общее | Ответы в тредах | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-015 | Общее | @упоминания | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-016 | Общее | Undo/redo | P0 | 1 | REQ-11-002 | undo/redo через существующие ops; Ctrl+Shift+Z маршрутизируется в redo. GAP: полный history scope/межрежимная и runtime keyboard acceptance. | n/a | n/a | partial_unverified |
| PAR-COM-017 | Общее | История undo | P0 | 1 | REQ-11-002 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-018 | Общее | AI-грамматика opt-in | P1 | 8 | REQ-11-003 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-019 | Общее | Перевод выделения | P1 | 8 | REQ-11-003 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-020 | Общее | OCR | P1 | 8 | REQ-11-003 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-COM-021 | Общее | Системное чтение вслух | P1 | 8 | REQ-11-003 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-001 | Docs | Семейство шрифта | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-002 | Docs | Размер шрифта | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-003 | Docs | Жирный | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-004 | Docs | Курсив | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-005 | Docs | Подчёркивание | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-006 | Docs | Зачёркивание | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-007 | Docs | Надстрочный | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-008 | Docs | Подстрочный | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-009 | Docs | Регистр | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-010 | Docs | Цвет текста | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-011 | Docs | Выделение цветом | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-012 | Docs | Очистить формат | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-013 | Docs | Формат по образцу | P0 | 1 | REQ-11-004 | format_text | planned | planned | planned |
| PAR-DOC-014 | Docs | Заголовки1–6 | P0 | 1 | REQ-11-004 | apply_style / define_style | planned | planned | planned |
| PAR-DOC-015 | Docs | Обычный стиль | P0 | 1 | REQ-11-004 | apply_style / define_style | planned | planned | planned |
| PAR-DOC-016 | Docs | Цитата | P0 | 1 | REQ-11-004 | apply_style / define_style | planned | planned | planned |
| PAR-DOC-017 | Docs | Код | P0 | 1 | REQ-11-004 | Code style экспортируется HTML pre с сохранением whitespace. GAP: полный code editing/language semantics и runtime round-trip. | subset | HTML pre subset | partial_unverified |
| PAR-DOC-018 | Docs | Свои стили | P0 | 1 | REQ-11-004 | apply_style / define_style | planned | planned | planned |
| PAR-DOC-019 | Docs | Наборы стилей | P0 | 1 | REQ-11-004 | apply_style / define_style | planned | planned | planned |
| PAR-DOC-020 | Docs | Выравнивание абзаца | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-021 | Docs | Отступы | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-022 | Docs | Межстрочный интервал | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-023 | Docs | До/после | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-024 | Docs | Табуляции | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-025 | Docs | Висячие строки | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-026 | Docs | Не отрывать от следующего | P0 | 1 | REQ-11-004 | set_paragraph_format | planned | planned | planned |
| PAR-DOC-027 | Docs | Маркированный список | P0 | 1 | REQ-11-004 | set_list | planned | planned | planned |
| PAR-DOC-028 | Docs | Нумерованный список | P0 | 1 | REQ-11-004 | set_list | planned | planned | planned |
| PAR-DOC-029 | Docs | Многоуровневый список | P0 | 1 | REQ-11-004 | set_list | planned | planned | planned |
| PAR-DOC-030 | Docs | Перезапуск нумерации | P0 | 1 | REQ-11-004 | set_list | planned | planned | planned |
| PAR-DOC-031 | Docs | Вставить таблицу | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-032 | Docs | Объединить ячейки таблицы | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-033 | Docs | Разбить ячейки таблицы | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-034 | Docs | Границы таблицы | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-035 | Docs | Заливка таблицы | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-036 | Docs | Ширины | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-037 | Docs | Повтор заголовка | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-038 | Docs | Сортировка таблицы | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-039 | Docs | Формулы в таблице | P0 | 1 | REQ-11-004 | insert_table / edit_table; GAP: sort/formula contract | planned | planned | planned |
| PAR-DOC-040 | Docs | Вставка изображения | P0 | 1 | REQ-11-004 | HTML PNG/JPEG data URI: width/height и aspect ratio, ошибки изображения возвращают loss notice; insert_image остаётся. GAP: внешние URL, полный browser sizing и runtime acceptance. | data images subset | subset | partial_unverified |
| PAR-DOC-041 | Docs | Масштаб изображения | P0 | 1 | REQ-11-004 | Модель сохраняет положительную дробную геометрию; HTML absolute dimensions/aspect ratio. GAP: container sizing/crop, полный UI/MCP и runtime acceptance. | subset | subset | partial_unverified |
| PAR-DOC-042 | Docs | Поворот изображения | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-043 | Docs | Обрезка | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-044 | Docs | Обтекание в тексте | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-045 | Docs | Обтекание рамкой | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-046 | Docs | Обтекание по контуру | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-047 | Docs | За текстом | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-048 | Docs | Перед текстом | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-049 | Docs | Привязка изображения | P0 | 1 | REQ-11-004 | insert_image; GAP: image update/delete | planned | planned | planned |
| PAR-DOC-050 | Docs | Фигуры | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-051 | Docs | Надписи | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-052 | Docs | Разрывы страниц | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-053 | Docs | Разрывы разделов | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-054 | Docs | Поля страницы | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-055 | Docs | Ориентация | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-056 | Docs | Размер бумаги | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-057 | Docs | Колонтитул первой страницы | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-058 | Docs | Чётные/нечётные колонтитулы | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-059 | Docs | Номера страниц | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-060 | Docs | Колонки | P0 | 1 | REQ-11-004 | insert_break / set_page_setup; GAP: Docs shape editing | planned | planned | planned |
| PAR-DOC-061 | Docs | Разметка страниц | P0 | 1 | REQ-11-004 | get_outline / get_document_info; GAP: view control | n/a | n/a | planned |
| PAR-DOC-062 | Docs | Веб-просмотр | P0 | 1 | REQ-11-004 | get_outline / get_document_info; GAP: view control | n/a | n/a | planned |
| PAR-DOC-063 | Docs | Чтение | P0 | 1 | REQ-11-004 | get_outline / get_document_info; GAP: view control | n/a | n/a | planned |
| PAR-DOC-064 | Docs | Фокус | P0 | 1 | REQ-11-004 | get_outline / get_document_info; GAP: view control | n/a | n/a | planned |
| PAR-DOC-065 | Docs | Навигация заголовков | P0 | 1 | REQ-11-004 | get_outline / get_document_info; GAP: view control | n/a | n/a | planned |
| PAR-DOC-066 | Docs | Статистика | P0 | 1 | REQ-11-004 | get_outline / get_document_info; GAP: view control | n/a | n/a | planned |
| PAR-DOC-067 | Docs | Оглавление автоматическое/обновление | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-068 | Docs | Сноски | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-069 | Docs | Концевые сноски | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-070 | Docs | Поля дата/страница/число страниц/автор | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-071 | Docs | Закладки | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-072 | Docs | Перекрёстные ссылки | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-073 | Docs | Track changes | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-074 | Docs | Принять/отклонить правки | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-075 | Docs | Сравнение документов | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-076 | Docs | OMML рендер | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-077 | Docs | OMML редактор | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-078 | Docs | LaTeX-ввод | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-079 | Docs | SmartArt базовая правка | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-080 | Docs | Водяной знак текст | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-081 | Docs | Буквица | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-082 | Docs | Подложка страницы | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-083 | Docs | Рамки страницы | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-084 | Docs | Переносы авто/ручные | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-085 | Docs | Языки абзацев | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-086 | Docs | Библиография | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-087 | Docs | Цитаты | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-088 | Docs | Подписи рисунков/таблиц | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-089 | Docs | Список иллюстраций | P1 | 8 | REQ-11-005 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-090 | Docs | Mail merge CSV/Sheets | P2 | 9 | REQ-11-006 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-091 | Docs | Предметный указатель | P2 | 9 | REQ-11-006 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-092 | Docs | Content controls/формы | P2 | 9 | REQ-11-006 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-093 | Docs | Главный документ | P2 | 9 | REQ-11-006 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-094 | Docs | Водяной знак изображение | P2 | 9 | REQ-11-006 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-001 | Sheets | Число | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-002 | Sheets | Текст | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-003 | Sheets | Дата/время | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-004 | Sheets | Boolean | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-005 | Sheets | Error | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-006 | Sheets | Формулы математические | P0 | 5 | REQ-11-007 | Существующий formula engine; RANDBETWEEN сохраняет конечность для огромных конечных границ, boxed AST ограничен. GAP: полный Excel parity и runtime acceptance; большие границы проверены только инвариантом движка. | formula subset | formula subset | partial_unverified |
| PAR-SHT-007 | Sheets | Формулы статистические | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-008 | Sheets | Формулы текстовые | P0 | 5 | REQ-11-007 | NUMBERVALUE пробелы/проценты; SEARCH использует один folded buffer и общий backtracking budget. GAP: полный набор Excel text functions и runtime acceptance. | formula subset | formula subset | partial_unverified |
| PAR-SHT-009 | Sheets | Формулы дата/время | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-010 | Sheets | Формулы логические | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-011 | Sheets | XLOOKUP | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-012 | Sheets | VLOOKUP | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-013 | Sheets | HLOOKUP | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-014 | Sheets | INDEX | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-015 | Sheets | MATCH | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-016 | Sheets | XMATCH | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-017 | Sheets | Финансовые базовые | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-018 | Sheets | Абсолютные ссылки | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-019 | Sheets | Относительные ссылки | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-020 | Sheets | Ссылки между листами | P0 | 5 | REQ-11-007 | read_cells / write_cells / set_formula | planned | planned | planned |
| PAR-SHT-021 | Sheets | Автодополнение функций | P0 | 5 | REQ-11-007 | explain_formula; GAP: UI hints API | n/a | n/a | planned |
| PAR-SHT-022 | Sheets | Подсказка аргументов | P0 | 5 | REQ-11-007 | explain_formula; GAP: UI hints API | n/a | n/a | planned |
| PAR-SHT-023 | Sheets | Подсветка зависимых диапазонов | P0 | 5 | REQ-11-007 | explain_formula; GAP: UI hints API | n/a | n/a | planned |
| PAR-SHT-024 | Sheets | Формат общий | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-025 | Sheets | Формат число | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-026 | Sheets | Валюта | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-027 | Sheets | Финансовый формат | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-028 | Sheets | Проценты | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-029 | Sheets | Дата | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-030 | Sheets | Время | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-031 | Sheets | Дробь | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-032 | Sheets | Экспонента | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-033 | Sheets | Текстовый формат | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-034 | Sheets | Пользовательские коды Excel | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-035 | Sheets | Шрифт/цвет | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-036 | Sheets | Границы | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-037 | Sheets | Заливка | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-038 | Sheets | Выравнивание | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-039 | Sheets | Перенос текста | P0 | 5 | REQ-11-007 | format_cells | planned | planned | planned |
| PAR-SHT-040 | Sheets | Поворот текста | P0 | 5 | REQ-11-007 | CellFormat.rotation передаётся в native TextShape на холсте; galley содержит underline/strike. GAP: общий PDF/print renderer и runtime visual acceptance. | subset | subset | partial_unverified |
| PAR-SHT-041 | Sheets | Merge/unmerge | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-042 | Sheets | Ширина столбца | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-043 | Sheets | Высота строки | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-044 | Sheets | Autofit | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-045 | Sheets | Вставить строки/столбцы | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-046 | Sheets | Удалить строки/столбцы | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-047 | Sheets | Скрыть строки/столбцы | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-048 | Sheets | Закрепление областей | P0 | 5 | REQ-11-007 | merge_cells / unmerge_cells / resize / insert_rows / insert_cols / delete_rows / delete_cols / hide / freeze_panes | planned | planned | planned |
| PAR-SHT-049 | Sheets | Автозаполнение ряды | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-050 | Sheets | Автозаполнение даты | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-051 | Sheets | Маркер заполнения | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-052 | Sheets | Многоуровневая сортировка | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-053 | Sheets | Автофильтр | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-054 | Sheets | Добавить лист | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-055 | Sheets | Переименовать лист | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-056 | Sheets | Переместить лист | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-057 | Sheets | Скрыть лист | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-058 | Sheets | Цвет ярлыка | P0 | 5 | REQ-11-007 | sort_range / filter_range / add_sheet / rename_sheet / move_sheet; GAP: autofill/hide sheet/tab color | planned | planned | planned |
| PAR-SHT-059 | Sheets | Диаграмма гистограмма | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-060 | Sheets | Диаграмма линейчатая | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-061 | Sheets | Диаграмма график | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-062 | Sheets | Диаграмма с областями | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-063 | Sheets | Круговая диаграмма | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-064 | Sheets | Кольцевая диаграмма | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-065 | Sheets | Точечная диаграмма | P0 | 5 | REQ-11-007 | create_chart / update_chart | planned | planned | planned |
| PAR-SHT-066 | Sheets | Статус сумма/среднее/количество | P0 | 5 | REQ-11-007 | UI read-only selection statistics: sparse обход для полного столбца/листа, устойчивое среднее; dedicated MCP summary пока нет. GAP: runtime/performance acceptance. | n/a | n/a | partial_unverified |
| PAR-SHT-067 | Sheets | Локаль чисел/дат | P0 | 5 | REQ-11-007 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-068 | Sheets | Десятичная запятая ru-RU | P0 | 5 | REQ-11-007 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-069 | Sheets | Динамические массивы | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-070 | Sheets | LET | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-071 | Sheets | LAMBDA | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-072 | Sheets | FILTER | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-073 | Sheets | SORT | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-074 | Sheets | UNIQUE | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-075 | Sheets | SEQUENCE | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-076 | Sheets | Условное форматирование правила | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-077 | Sheets | Data bars | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-078 | Sheets | Color scales | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-079 | Sheets | Icon sets | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-080 | Sheets | Validation lists/ranges/messages | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-081 | Sheets | Именованные диапазоны | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-082 | Sheets | Диспетчер имён | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-083 | Sheets | Сводные таблицы | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-084 | Sheets | Сводные диаграммы | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-085 | Sheets | Срезы | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-086 | Sheets | Спарклайны | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-087 | Sheets | Форматировать как таблицу | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-088 | Sheets | Structured references | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-089 | Sheets | Группировка/структура | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-090 | Sheets | Промежуточные итоги | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-091 | Sheets | Удалить дубликаты | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-092 | Sheets | Текст по столбцам | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-093 | Sheets | Мгновенное заполнение | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-094 | Sheets | Защита листа/книги | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-095 | Sheets | Комбинированные диаграммы | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-096 | Sheets | Вторая ось | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-097 | Sheets | Линии тренда | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-098 | Sheets | Комментарии ячеек | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-099 | Sheets | Заметки ячеек | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-100 | Sheets | Итеративные вычисления | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-101 | Sheets | Ручной пересчёт | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-102 | Sheets | Трассировка влияющих/зависимых | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-103 | Sheets | Watch window | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-104 | Sheets | СУММ↔SUM display English storage | P1 | 8 | REQ-11-008 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-105 | Sheets | Power Query-подобный импорт | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-106 | Sheets | Подбор параметра | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-107 | Sheets | Сценарии | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-108 | Sheets | Таблицы данных what-if | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-109 | Sheets | Solver | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-110 | Sheets | Внешние книги | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-111 | Sheets | 3D-ссылки | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-112 | Sheets | VBA opaque preservation без исполнения | P2 | 9 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SHT-113 | Sheets | Unsupported function #NAME? explanation | P0 | 5 | REQ-11-009 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SLD-001 | Slides | Добавить слайд | P0 | 6 | REQ-11-010 | add_slide / duplicate_slide / delete_slide / hide_slide / reorder_slides; GAP: sections | planned | planned | planned |
| PAR-SLD-002 | Slides | Дублировать слайд | P0 | 6 | REQ-11-010 | add_slide / duplicate_slide / delete_slide / hide_slide / reorder_slides; GAP: sections | planned | planned | planned |
| PAR-SLD-003 | Slides | Удалить слайд | P0 | 6 | REQ-11-010 | add_slide / duplicate_slide / delete_slide / hide_slide / reorder_slides; GAP: sections | planned | planned | planned |
| PAR-SLD-004 | Slides | Скрыть слайд | P0 | 6 | REQ-11-010 | add_slide / duplicate_slide / delete_slide / hide_slide / reorder_slides; GAP: sections | planned | planned | planned |
| PAR-SLD-005 | Slides | Переставить слайды | P0 | 6 | REQ-11-010 | add_slide / duplicate_slide / delete_slide / hide_slide / reorder_slides; GAP: sections | planned | planned | planned |
| PAR-SLD-006 | Slides | Разделы | P0 | 6 | REQ-11-010 | PPTX section extension сохраняет названия и последовательные группы; UI/model section поля существующие. GAP: исходные section GUID, dedicated section MCP contract и runtime acceptance. | subset | subset | partial_unverified |
| PAR-SLD-007 | Slides | Макеты | P0 | 6 | REQ-11-010 | Layout смена сохраняет дополнительные placeholder-объекты; supported layout metadata импортируется. GAP: master editing, полная placeholder hierarchy и runtime acceptance. | subset | subset | partial_unverified |
| PAR-SLD-008 | Slides | Мастер-слайды | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-009 | Slides | Цвета темы | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-010 | Slides | Шрифты темы | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-011 | Slides | Фон темы | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-012 | Slides | Text autofit | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-013 | Slides | Базовые фигуры | P0 | 6 | REQ-11-010 | slides_update_element element.fill_visible и UI: отдельный noFill для Rect/Ellipse, контур/текст сохраняются, Page/PPTX путь. GAP: все DrawingML fills/effects, runtime/Office acceptance. | subset | subset | partial_unverified |
| PAR-SLD-014 | Slides | Линии | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-015 | Slides | Соединители | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-016 | Slides | Crop изображений | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-017 | Slides | Shape mask | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-018 | Slides | Таблицы | P0 | 6 | REQ-11-010 | PPTX: пустая/неровная таблица дополнена до прямоугольника с общим лимитом100000 ячеек; UI multiline cell editor. GAP: rotated canvas tables, полный Office layout и runtime acceptance. | subset | subset | partial_unverified |
| PAR-SLD-019 | Slides | Диаграммы | P0 | 6 | REQ-11-010 | PPTX import: title и series fill, отсутствующий chart part пропускается с warning. GAP: полный chart style/series parity и runtime acceptance. | subset | subset | partial_unverified |
| PAR-SLD-020 | Slides | Smart guides | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-021 | Slides | Align | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-022 | Slides | Distribute | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-023 | Slides | Group | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-024 | Slides | Z-order | P0 | 6 | REQ-11-010 | set_layout / apply_theme / add_shape / update_shape / align_shapes / group_shapes / set_z_order; GAP: master editing/distribute schema | planned | planned | planned |
| PAR-SLD-025 | Slides | Rotate | P0 | 6 | REQ-11-010 | PPTX p:xfrm rotation для graphicFrame; canvas picking учитывает повёрнутые границы поддержанных фигур. GAP: rotated table/chart canvas, handles, Page/PDF rotation и runtime acceptance. | subset | subset | partial_unverified |
| PAR-SLD-026 | Slides | Flip | P0 | 6 | REQ-11-010 | PPTX picture flipH/flipV нормализуется в PNG с pixel budget. GAP: UI/MCP flip для всех объектов, lossless original encoding и runtime acceptance. | picture subset | normalized picture subset | partial_unverified |
| PAR-SLD-027 | Slides | Заметки докладчика | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-028 | Slides | Сортировщик слайдов | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-029 | Slides | Показ с начала F5 | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-030 | Slides | Показ текущего ShiftF5 | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-031 | Slides | Presenter second monitor | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-032 | Slides | Следующий слайд | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-033 | Slides | Presenter notes | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-034 | Slides | Таймер | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-035 | Slides | Часы | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-036 | Slides | Указка | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-037 | Slides | Маркер/чернила | P0 | 6 | REQ-11-010 | set_notes; GAP: presentation control | planned | planned | planned |
| PAR-SLD-038 | Slides | Entrance animation | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-039 | Slides | Emphasis animation | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-040 | Slides | Exit animation | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-041 | Slides | Motion path | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-042 | Slides | Custom motion path | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-043 | Slides | Click trigger | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-044 | Slides | With previous trigger | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-045 | Slides | After previous trigger | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-046 | Slides | Object trigger | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-047 | Slides | Delay | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-048 | Slides | Duration | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-049 | Slides | Repeat | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-050 | Slides | Per-paragraph animation | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-051 | Slides | Animation timeline | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-052 | Slides | Transitions | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-053 | Slides | Morph matching objects | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-054 | Slides | p:timing roundtrip | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-055 | Slides | Transitions roundtrip | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-056 | Slides | Video | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-057 | Slides | Audio | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-058 | Slides | Media trim | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-059 | Slides | SmartArt | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-060 | Slides | Zoom slide/section | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-061 | Slides | Custom shows | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-062 | Slides | Rehearsal timings | P1 | 8 | REQ-11-011 | add_animation / update_animation / remove_animation / set_transition; GAP: media/SmartArt/show settings | planned | planned | planned |
| PAR-SLD-063 | Slides | Показ с голосовой записью | P2 | 9 | REQ-11-012 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SLD-064 | Slides | Video export | P2 | 9 | REQ-11-012 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-SLD-065 | Slides | Живые субтитры | P2 | 9 | REQ-11-012 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-001 | TrineDocs | Палитра команд Ctrl/Cmd+K | P0 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-002 | TrineDocs | Slash меню | P0 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-003 | TrineDocs | Markdown shortcuts | P0 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-004 | TrineDocs | Фокус | P0 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-005 | TrineDocs | Чистое чтение | P0 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-006 | TrineDocs | Живые связи Sheets→Docs | P1 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-007 | TrineDocs | Живые связи Sheets→Slides | P1 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-008 | TrineDocs | Office статичный link fallback warning | P1 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-009 | TrineDocs | Машина времени | P1 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-010 | TrineDocs | Умные AI-шаблоны | P1 | 8 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-011 | TrineDocs | AI-чат | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-012 | TrineDocs | Курсор AI | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-013 | TrineDocs | Контекст chips | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-014 | TrineDocs | Streaming validated ops | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-015 | TrineDocs | AI undo groups | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-016 | TrineDocs | Принять/отклонить AI | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-017 | TrineDocs | Stop rollback unfinished step | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-018 | TrineDocs | BYOK | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-019 | TrineDocs | AI off per-doc/app | P0 | 3 | REQ-11-013 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-095 | Docs | Формат .docx, .dotx открыть: ✓; сохранить: ✓ | P0 | 2 | REQ-08-001 | DOCX: hidden style/direct text фильтруется с loss notice, double-strike→single с notice, table vAlign/gridBefore, numFmt none и block ins/moveTo. GAP: lossless hidden/revision history, полный OOXML corpus и Office acceptance. | subset | subset | partial_unverified |
| PAR-DOC-096 | Docs | Формат .docm открыть: ✓, макросы не исполняются, сохраняются; сохранить: ✓ | P0 | 2 | REQ-08-002 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-DOC-097 | Docs | Формат .doc, .rtf, .odt открыть: ✓; сохранить: ✓ | P0 | 7 | REQ-08-003 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-DOC-098 | Docs | Формат .pdf открыть: P2: картинки/текст; сохранить: ✓; PDF/A — P1 | P2 | 9 | REQ-08-004 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-DOC-099 | Docs | Формат .txt открыть: ✓; сохранить: ✓ | P0 | 2 | REQ-08-005 | open/export: BOM UTF-8/16/32, строгий legacy decode; текст таблиц и span-поля в TXT. GAP: runtime/encoding corpus, полная структура отсутствует по природе TXT. | text subset | body/table text | partial_unverified |
| PAR-DOC-100 | Docs | Формат .md открыть: ✓; сохранить: ✓ | P0 | 2 | REQ-08-006 | Markdown subset: fence info/indent, indented code и blank lines, angle destinations/quoted titles, emphasis flanking, native LF→hard break. GAP: полный CommonMark/GFM, image display size и runtime round-trip. | subset | subset | partial_unverified |
| PAR-DOC-101 | Docs | Формат .html открыть: ✓; сохранить: ✓ | P0 | 2 | REQ-08-007 | HTML subset: normal whitespace/CSS inheritance, pre/code, bounded tables/merges/row groups/rowspan0/widths/rich cells, data images и title/author. GAP: nested tables/images in cells, полный CSS/layout и runtime corpus. | subset | subset | partial_unverified |
| PAR-DOC-102 | Docs | Формат .epub открыть: —; сохранить: P2 | P2 | 9 | REQ-08-008 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-DOC-103 | Docs | Формат Google Docs открыть: ✓; сохранить: ✓ | P0 | 7 | REQ-08-009 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SHT-114 | Sheets | Формат .xlsx, .xltx открыть: ✓; сохранить: ✓ | P0 | 5 | REQ-08-010 | XLSX subset: selection export; cellStyleXfs apply*=false; chart worksheet/drawing/part ownership и cell-anchor; signed drawing offsets. Cross-sheet chart source явно отклоняется. GAP: subcell chart offsets, complex series/conditional formats, полный corpus и Office acceptance. | subset | subset | partial_unverified |
| PAR-SHT-115 | Sheets | Формат .xlsm открыть: ✓, VBA не исполняется, сохраняется; сохранить: ✓ | P0 | 5 | REQ-08-011 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SHT-116 | Sheets | Формат .xlsb открыть: P2; сохранить: P2 | P2 | 9 | REQ-08-012 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SHT-117 | Sheets | Формат .xls, .ods открыть: ✓; сохранить: ✓ | P0 | 7 | REQ-08-013 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SHT-118 | Sheets | Формат .csv, .tsv открыть: ✓: мастер UTF-8/UTF-16/Windows-1251 и др.; разделитель, кавычки, типы, локаль; сохранить: ✓: лист или все листы в zip | P0 | 5 | REQ-08-014 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SHT-119 | Sheets | Формат .pdf открыть: —; сохранить: ✓: область, масштаб | P0 | 5 | REQ-08-015 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SHT-120 | Sheets | Формат .html, .md открыть: —; сохранить: ✓: таблица | P0 | 5 | REQ-08-016 | HTML сохраняет merged value/format при скрытом anchor; CSV/MD streaming output budget; HTML data images и основной text formatting. GAP: charts/полный conditional formatting/floating image geometry/runtime acceptance. | unsupported | subset | partial_unverified |
| PAR-SHT-121 | Sheets | Формат Google Sheets открыть: ✓; сохранить: ✓ | P0 | 7 | REQ-08-017 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-066 | Slides | Формат .pptx, .potx, .ppsx открыть: ✓; сохранить: ✓ | P0 | 6 | REQ-08-018 | PPTX subset: theme/layout name, placeholder defaults/partial xfrm/text inheritance, chart title/fill, section groups, picture flips, group uniform font/stroke scale. GAP: mixed text, complex groups/masters, полный OOXML corpus и Office acceptance. | subset | subset | partial_unverified |
| PAR-SLD-067 | Slides | Формат .pptm открыть: ✓, макросы не исполняются, сохраняются; сохранить: ✓ | P0 | 6 | REQ-08-019 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-068 | Slides | Формат .ppt, .odp открыть: ✓; сохранить: ✓ | P0 | 7 | REQ-08-020 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-069 | Slides | Формат .pdf открыть: —; сохранить: ✓: слайды/заметки/раздаточные | P0 | 6 | REQ-08-021 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-070 | Slides | Формат .png, .jpg, .svg открыть: —; сохранить: ✓: слайд/все, разрешение | P0 | 6 | REQ-08-022 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-071 | Slides | Формат .mp4 открыть: —; сохранить: P2: анимации, ffmpeg отдельным процессом, проверить лицензию сборки | P2 | 9 | REQ-08-023 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-072 | Slides | Формат .gif открыть: —; сохранить: P2 | P2 | 9 | REQ-08-024 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-SLD-073 | Slides | Формат Google Slides открыть: ✓; сохранить: ✓ | P0 | 7 | REQ-08-025 | export_document / open_document; format implementation pending | planned | planned | planned |
| PAR-TRI-020 | TrineDocs | Explorer preview handler | P1 | 8 | REQ-A03-027 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-021 | TrineDocs | Finder Quick Look | P1 | 8 | REQ-A03-027 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-022 | TrineDocs | 6. External file three-way merge local/disk/base с compare UI. | P1 | 8 | REQ-A03-020 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-023 | TrineDocs | 8. CRDT history compression/shallow thresholds adjustable; time machine/undo impact ADR. | P1 | 8 | REQ-A03-022 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-024 | TrineDocs | 9. Server email/invites/@, account/data deletion, GDPR и аналоги, monitoring/alerts/backup/restore/cost N, E2EE choice (search/preview/AI), abuse. | P1 | 8 | REQ-A03-023 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-025 | TrineDocs | 10. Coauthor missing fonts: fallback/embedding/cloud with licenses ADR. | P1 | 8 | REQ-A03-024 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-026 | TrineDocs | 15. Uninstall: MCP configs/startup/keychain/cache; update signatures/rollback; file/database migrations/versioning/forward-backward compatibility. | P1 | 8 | REQ-A03-029 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-027 | TrineDocs | 17. Max rows/cols against Excel, pages/slides/file size, overflow/memory. | P1 | 8 | REQ-A03-031 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-028 | TrineDocs | 18. Excel dates 1900/1904/leap-bug; 15 digits; circular refs; locale arg separator; recalc order; volatile functions. | P1 | 8 | REQ-A03-032 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-029 | TrineDocs | 19. Word docDefaults/latent styles/abstractNum/num/fields/w:compat/version-sensitive layout. | P1 | 8 | REQ-A03-033 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-TRI-030 | TrineDocs | 20. Clipboard read/write formats each OS, Word/Excel/PowerPoint/browser, drag-and-drop. | P1 | 8 | REQ-A03-034 | GAP: точная ops/tool-схема ещё не определена | planned | planned | planned |
| PAR-DOC-104 | Docs | Alt text visual objects (FACT-017) | P1 | 8 | REQ-11-005 | GAP: add explicit tool/ops schema | planned | planned | planned |
| PAR-DOC-105 | Docs | Synchronous side-by-side scrolling (FACT-017) | P1 | 8 | REQ-11-005 | GAP: add explicit tool/ops schema | planned | planned | planned |
| PAR-TRI-031 | TrineDocs | Фильтр недавних и закреплённых файлов по Docs/Sheets/Slides, очистка поиска | P0 | 1 | REQ-06-004 | UI view-only; MCP list_documents остаётся отдельно | n/a | n/a | implemented_unverified |
| PAR-TRI-032 | TrineDocs | Палитра команд: выбор стрелками вверх/вниз, запуск выбранной команды по Enter | P0 | 1 | REQ-05-012 | UI view-only; команды используют существующие ops | n/a | n/a | implemented_unverified |
| PAR-TRI-033 | Docs | Проверка структуры документа: пропущенные уровни заголовков, пустые заголовки и alt text | P1 | 8 | REQ-11-005 | UI и inspect_document: структура заголовков и alt описания, stable paragraph targets; фильтр/поиск/копия, первые200 с omitted/stale. GAP: полный structure audit, runtime/GUI acceptance. | planned | n/a | partial_unverified |
| PAR-TRI-034 | Sheets | Паспорт формулы: зависимости, типы входов и пошаговое объяснение результата без изменения ячейки | P1 | 8 | REQ-11-008 | sheets_explain_formula и UI Паспорт формулы: cached value/type, direct/named dependencies до64, 4 samples с типами, parse/missing reference diagnostics, no evaluation/mutation. GAP: полный trace вычисления и runtime acceptance. | n/a | clipboard report | partial_unverified |
| PAR-TRI-035 | Slides | Проверка читаемости слайдов: переполнение текста, контраст и слишком мелкий шрифт | P1 | 8 | REQ-11-011 | UI и inspect_document: обычный текст/границы объектов, stable element targets; фильтр/поиск/копия, первые200 с omitted/stale. GAP: таблицы/диаграммы/повороты/фон, runtime/GUI acceptance. | planned | n/a | partial_unverified |
| PAR-TRI-036 | TrineDocs | Карточка AI-пакета: затронутые объекты, краткий diff и конфликты до применения | P1 | 8 | REQ-03-022 | GAP: preview ops batch без мутаций и повторная проверка base_version при commit | n/a | n/a | planned |
| PAR-TRI-037 | Sheets | Список ошибок ячеек и формул на всех листах с адресами и ограниченной выдачей | P1 | 8 | REQ-11-008 | UI и inspect_document: bounded ошибки всех листов, включая скрытые; stable cell targets, фильтр/поиск/копия, первые200 с omitted/stale. GAP: полный паспорт формулы, runtime/GUI acceptance. | n/a | n/a | implemented_unverified |
| PAR-TRI-038 | Docs | Карта фактического оформления: отличия прямого форматирования от стилей и переход к примерам | P1 | 8 | REQ-11-005 | inspect_document kind=formatting и UI инспектора: размер/bold/italic относительно встроенных стилей TrineDocs, stable paragraph targets, bounded report. GAP: произвольные стили Office, таблицы и runtime acceptance. | n/a | clipboard report | partial_unverified |
| PAR-TRI-039 | Slides | Автоматическая повестка презентации из разделов и заголовков с явным предпросмотром | P1 | 8 | REQ-11-011 | UI Вставка → Повестка: preview видимых заголовков/разделов до200, редактируемые страницы в начале, atomic insert и undo. Заголовки80/разделы40 знаков, min480×320pt. MCP: обычные slides ops; GAP: dedicated generator tool и runtime acceptance. | current deck | ordinary editable slides | implemented_unverified |
| PAR-TRI-040 | TrineDocs | Параллельная подготовка AI-правок нескольких открытых Docs/Sheets/Slides с общим preflight commit | P1 | 8 | REQ-M06-072 | workspace_documents/apply/job_status/cancel; 4 документа/32 операции; GAP: полный apply_ops, дисковые side effects, persistent jobs и runtime-приёмка | not applicable | not applicable | partial |
| PAR-TRI-041 | TrineDocs | Поиск, категории и копирование отчёта проверки документа; bounded read-only AI inspector с фоновым снимком | P1 | 8 | REQ-11-002 | inspect_document: preparing/completed, snapshot/current version, refresh, query/kind, index/count до100; первые200 с omitted/stale; один фоновый inspector. GAP: runtime/visual acceptance и полный анализ Office. | n/a | clipboard plain text | partial_unverified |

| PAR-TRI-042 | Общее | Адаптация вкладок ленты и доступные палитры цветов | P0 | 1 | REQ-11-002 | UI view-only: горизонтальная прокрутка ленты, scoped font filter, имена swatches и видимый фокус. GAP: runtime keyboard/AccessKit/visual acceptance. | n/a | n/a | implemented_unverified |
| PAR-TRI-043 | Sheets | Отдельная строка статистики выделения с локалью и устойчивым средним | P1 | 8 | REQ-11-008 | UI read-only: сумма/среднее/заполненность отдельно от вкладок, sparse statistics полных строк/столбцов; overflow суммы —. GAP: runtime/performance/visual acceptance. | n/a | n/a | implemented_unverified |
| PAR-TRI-044 | Slides | Чернила докладчика в координатах слайда и разделение клавиш панели/показа | P1 | 8 | REQ-11-011 | UI-only presentation state: normalized ink/clipping, 100000 точек/1000 штрихов и clear, blank/end click, manual navigation перед timer. GAP: runtime resize/keyboard/visual acceptance. | n/a | n/a | implemented_unverified |
| PAR-TRI-045 | Slides | Фильтр навигатора по заголовку/разделу, фиксированные карточки и вставка перед drop-marker | P1 | 8 | REQ-11-011 | Фильтр view-only; перенос через существующие model/ops и undo. Native buttons с полными подписями. GAP: runtime DnD/keyboard/AccessKit/visual acceptance. | n/a | n/a | implemented_unverified |

| PAR-TRI-046 | Общее | Постраничное чтение структуры крупных Docs/Sheets через MCP | P1 | 8 | REQ-11-002 | docs_read_objects: table_id, row/rows/col/columns; Sheets geometry/objects/names: index/count и total_*; metadata noFill/table layout. GAP: rich cell content paging, snapshot across edits и runtime/MCP acceptance. | n/a | bounded JSON subset | partial_unverified |

Дополнение 2026-10-18: [50 исправлений кода с регрессиями](audit/bugs-50-2026-10-18.md) и [отчёт фазы](plan/phase-capabilities-50-fixes-2026-10-18-report.md). Сборка и тесты запрещены владельцем; runtime-приёмка и полный Office/MCP-паритет этим срезом не подтверждены.

Дополнение UI 2026-10-18: [ещё 50 дефектов BUG-051…100](audit/bugs-51-100-2026-10-18.md) и [отчёт UI-фазы](plan/phase-ui-50-more-fixes-2026-10-18-report.md). Все добавления остаются implemented/partial_unverified; 50 новых тестов только написаны, не запускались.

Следующий проход2026-10-18: [BUG-101…150](audit/bugs-101-150-2026-10-18.md) и [отчёт](plan/phase-ui-50-next-2026-10-18-report.md). Статусы указывают наличие кода; сборка, тесты и runtime-приёмка не проводились.

Проход1002026-10-18: [BUG-151…250](audit/bugs-151-250-2026-10-18.md), [отчёт](plan/phase-100-more-2026-10-18-report.md). Ровно100 новых сценариев и100 новых регрессионных тестов, не запускавшихся. JSON синхронизирован с этой таблицей, включая ранее пропущенный PAR-TRI-045; все статусы отражают только просмотренный код.
