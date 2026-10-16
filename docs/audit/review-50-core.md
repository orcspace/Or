# Независимое ревью Sheets — 2026-10-15

Проверены Sheets model/formula/format/IO, дополнительно прочитаны ops, recovery, native и file worker. Здесь только новые исправленные дефекты. Общая причина в нескольких функциях считается один раз. Строки — исходные, до правок; S = crates/trinedocs/src/sheets.

| ID | Приоритет | Исходное место | Воспроизведение / исправление |
|---|---|---|---|
| C01 | P1 | S/formula.rs:2952 numbers | A1=2, A2="" формулой: AVERAGEA(A1:A2) давал2 вместо1. Пустой текст учитывается как0, пустая ячейка пропускается. |
| C02 | P1 | S/formula.rs:2956 numbers | AVERAGEA("2",4) давал2 вместо3; MINA("2",4) —0 вместо2. Числовые строки прямых аргументов преобразуются, текст ссылок остаётся нулём. |
| C03 | P1 | S/formula.rs:1614,2529,3099 | SUMPRODUCT и семейство *IFS принимали диапазоны2×1 и1×2 как одинаковые. Проверяются обе размерности, возвращается #VALUE!. Одна причина для всех семейств. |
| C04 | P2 | S/formula.rs:2662 | TEXTBEFORE("", "-") / TEXTAFTER("", "-") давали #N/A вместо пустого текста. Добавлена обработка пустой строки. |
| C05 | P1 | S/formula.rs:1084 | TEXTBEFORE("a-b","-",1,0,0,1/0) вычислял неиспользованный fallback и давал ошибку вместо"a". Резервное выражение вычисляется при отсутствии разделителя. |
| C06 | P2 | S/formula.rs:2664 | TEXTBEFORE/TEXTAFTER принимали match_mode=2 и match_end=2 как true. Допустимы только0/1, прочее — #VALUE!. |
| C07 | P1 | S/formula.rs:1449 | FACTDOUBLE(200) отклонялся общим пределом FACT=170 при конечном результате. Раздельные пределы и усечение дробного аргумента. |
| C08 | P1 | S/formula.rs:1581 | COMBIN/COMBINA/PERMUT(1000,1) давали #NUM! вместо1000. Убран предел170; симметрия сочетаний, ограниченные циклы и finite-проверки защищают вычисление. |
| C09 | P1 | S/formula.rs:2845 | DECIMAL("100000000000000000000",10) давал #NUM! из-за u64 вместо1e20. Ограниченный разбор в f64 с обнаружением переполнения. |
| C10 | P1 | S/formula.rs:1538 | CEILING(-2.5,-2) давал -2 вместо -4, FLOOR — -4 вместо -2. Старые функции учитывают знак significance; положительное число с отрицательным significance отклоняется. |
| C11 | P1 | S/formula.rs:1967,2017,2065 | A1=#DIV/0!, MATCH(A1,B1:B2,0) терял ошибку и давал #N/A; XLOOKUP мог вернуть fallback. Семейство lookup сохраняет ошибку искомого значения. |
| C12 | P1 | S/formula.rs:2974 compare | Пустая Z1: Z1=FALSE и FALSE=Z1 давали false. Пустая ссылка при сравнении с bool соответствует FALSE. |
| C13 | P2 | S/formula.rs:3193 serial_date | YEAR(0), MONTH(0), DAY(0) давали1899/12/31 вместо совместимого1900/1/0. Исправлена нулевая календарная точка. |
| C14 | P2 | S/format.rs:58 | Форматы только m/mm/mmm/mmmm выводились буквально. Токен месяца участвует в распознавании дат. |
| C15 | P2 | S/format.rs:474 | ddd/dddd выводили число дня вместо названия дня недели. Добавлены короткие/полные названия. |
| C16 | P2 | S/format.rs:496 | mmmmm выводил полное имя месяца вместо первой буквы. Исправлена обработка длины токена. |
| C17 | P2 | S/format.rs:137 | Число12, формат00000: вывод12 вместо00012. Сохраняются обязательные целые нули и знак. |
| C18 | P2 | S/format.rs:137 | Формат0.## выводил12.30 вместо12.3. Необязательные конечные # не создают лишние нули, обязательные0 сохраняются. |
| C19 | P2 | S/format.rs:325 strip_brackets | В формате "[code] "0 исчезала строка[code]. Кавычки/escape защищают буквальные скобки от удаления директив. |
| C20 | P2 | S/format.rs:21 | Формат "Customer: "@ не применялся к тексту; буквальный "@" в четвёртой секции подставлялся. Подстановка только для настоящего токена@. |
| C21 | P2 | S/format.rs:36 | Число25, формат[>=100]0;"small": вывод25 вместо small. Условная секция выбирается до удаления директив. |
| C22 | P1 | S/io.rs:838 | Даты workbookPr date1904 читались в системе1900 со сдвигом1462дня. Нормализуются календарные значения; durations не сдвигаются. |
| C23 | P1 | S/io.rs:969,1027,1154,1176 | b/strike val=false включали оформление; wrapText/hidden=true терялись. Общий разбор XML boolean поддерживает0/1/false/true. |
| C24 | P1 | S/io.rs:1028 | XLSX textRotation=135 делал книгу невалидной. Значения91–180 переводятся в отрицательные углы90−rotation. |
| C25 | P1 | S/io.rs:1055,1067 | Relative Target с .. и style part по другому relationship-пути теряли оформление/размеры. Пути разрешаются относительно workbook.xml, styles берётся из relationships. |

Добавлены6 регрессионных тестов в sheets/tests.rs,2 в format.rs,3 в io.rs, включая отдельные сценарии и границы. Rustfmt успешно разобрал/отформатировал4 файла, git diff --check для них прошёл. Cargo test/check/build/clippy, запуск приложения и установщик не выполнялись по просьбе владельца. Это аудит исходников, не подтверждение типизации или GUI-приёмки.

Независимое чтение: второй ревьюер редакторов и ведущий прочитали formula/format/io diff; блокеров по исходнику не обнаружили. Взаимно прочитан финальный Docs/Slides diff (editor/model/IO): блокирующих дефектов по исходнику не обнаружено. Проверены Copy у TableCursor, поля облегчённого ribbon Element, borrowed rendering, clipboard IDs, table atomicity, HTML links и enabled gates. Cargo/GUI не запускались. Nested HTML list hierarchy и browser repair для tables внутри list остаются ограничениями.

Первичные источники: [AVERAGEA](https://support.microsoft.com/en-us/excel/functions/averagea-function), [MINA](https://support.microsoft.com/en-us/excel/functions/mina-function), [SUMPRODUCT](https://support.microsoft.com/en-us/excel/functions/sumproduct-function), [MAXIFS](https://support.microsoft.com/en-us/excel/functions/maxifs-function), [SUMIFS](https://support.microsoft.com/en-us/excel/functions/sumifs-function), [TEXTBEFORE](https://support.microsoft.com/en-us/excel/functions/textbefore-function), [FACTDOUBLE](https://support.microsoft.com/en-us/excel/functions/factdouble-function), [COMBIN](https://support.microsoft.com/en-us/excel/functions/combin-function), [DECIMAL](https://support.microsoft.com/en-us/excel/functions/decimal-function), [CEILING](https://support.microsoft.com/en-us/excel/functions/ceiling-function), [FLOOR](https://support.microsoft.com/en-us/excel/functions/floor-function), [числовые форматы](https://support.microsoft.com/en-us/excel/review-guidelines-for-customizing-a-number-format), [системы дат](https://support.microsoft.com/en-us/excel/date-systems-in-excel), [OOXML Alignment](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.alignment), [OOXML BooleanPropertyType](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.booleanpropertytype). API зависимостей сверены с локальными calamine0.36.1 и rust_xlsxwriter0.99.1.

Остаются: полное формульное покрытие, XLSX opaque round-trip/conditional formatting, placement сложных диаграмм, локализованные имена месяцев, сложные условные/научные форматы. Нормализация1904 исправляет значения дат, но не доказывает произвольную совместимость1904-формул с числовыми литералами.
