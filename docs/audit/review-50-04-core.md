# Пакет 04 — независимое ревью Sheets

База `0fb6408`. **20 новых причин**; варианты функций/осей одной причины сгруппированы. Workspace-фича ведущего и исправления предыдущих пакетов в счёт не входят.

| ID | P | Дефект до изменения | Исправление |
|---|---|---|---|
| C4-01 | P2 | Одновременные underline+strike рисовали одну линию на canvas. | Обе координаты рисуются независимо. PDF был исправлен раньше, этот UI-путь оставался сломан. |
| C4-02 | P1 | Charts с 1e-200 были плоскими; scatter 0.01–0.02 сжимался; ±1e308 переполнял координаты. | Фактические границы без floor=1; нормализация половинами при переполнении span. Общая причина масштаба — один пункт. |
| C4-03 | P1 | ДАННЫЕ не находило имя Данные. | Unicode lowercasing при чтении имён и разрешении ссылок. MCP patch имён — отдельное исправление ведущего. |
| C4-04 | P1 | SUMIF/AVERAGEIF с named sum_range=B1 не проецировал размер criteria_range. | Имя/алиас разрешается до reference; проекция проверяет границы. |
| C4-05 | P1 | DAYS(2.75,1.5) давал 1 вместо 1.25; текстовые ISO-даты отвергались. | Прямое числовое вычитание и общий DATEVALUE parser для текста. |
| C4-06 | P1 | DAYS принимал отрицательные и выходящие за 9999 год serials. | Отдельная проверка домена возвращает #NUM!; нераспознаваемый текст — #VALUE!. |
| C4-07 | P1 | PMT/PV/FV/NPER/RATE/IPMT/PPMT использовали type=2/-1/0.5 как множитель. | Ноль означает конец периода, любое ненулевое значение — начало, без усечения. |
| C4-08 | P1 | MROUND(1e-200,-1e-200) обходил #NUM! из-за underflow произведения. | Знаки ненулевых аргументов сравниваются напрямую. |
| C4-09 | P1 | CORREL/PEARSON/RSQ для ±1e200 и ±1e-200 теряли конечный ответ 1. | Независимое масштабирование осей до центрирования и накопления моментов. |
| C4-10 | P1 | HARMEAN трёх 1e-308 давал 0 из-за переполнения reciprocal sum. | Масштабирование минимумом положительных данных. |
| C4-11 | P1 | MEDIAN(1e308,1e308), PERCENTILE([-1e308,1e308],0.5) переполнялись. | Общая интерполяция учитывает знаки границ, избегая переполнения midpoint/difference. |
| C4-12 | P1 | AVERAGE(1e308,1e308), STDEV.P(±1e-200), VAR.P(±1e154) теряли конечный результат. | Масштабирование extreme-scale данных для базовых средних/отклонений; sqrt до возврата масштаба. Общий алгоритм — один пункт. |
| C4-13 | P1 | COUNTBLANK/LARGE/SMALL/PERCENTILE/QUARTILE молча игнорировали лишние аргументы. | Исправлены общие arity-ветки; optional order RANK сохранён. |
| C4-14 | P1 | Cached formula text "001"/"TRUE"/"#N/A"/"NaN" в XLSX читался как число/bool/error. | Ambiguous Value::Text получает OOXML t="str" и исходный текст. Реальные Number/Bool/Error сохраняют тип. |
| C4-15 | P1 | XLSX round-trip расширял колонку 6px и строку 1.5px до 12px. | Минимум 1px, fractional points для высоты, специальная формула Excel для ширины <1 character. |
| C4-16 | P1 | absoluteAnchor переносился в A1; twoCellAnchor терял размер; custom dimensions игнорировались. | Разбор pos/from/to/ext; геометрия после чтения размеров и hidden axes. |
| C4-17 | P2 | Custom 0.0E-00 выводил плюс положительного exponent. | E- скрывает положительный плюс, E+ сохраняет. |
| C4-18 | P1 | Scientific 2.675/9.95 округлялись в 2.67/9.9. | Общий half-away алгоритм для mantissa, перенос в exponent; subnormal не делится на 10^-324. |
| C4-19 | P1 | Fill частично менял клетки до поздней ошибки; rename/sort/insert оставляли input >32767. | Clone + полная validate перед commit для rename/fill/sort/structural operations. Все четыре операции считаются одной причиной. |
| C4-20 | P1 | move_sheet фонового видимого листа переключал активный лист пользователя. | Сохраняется active sheet ID; selection/cell IDs не меняются. |

Добавлены **15 регрессий**: 8 model/formula, 2 editor, 2 format, 3 IO. XLSX cache проверяется напрямую calamine без внутреннего recalc, canvas — по output shapes. Тесты **не запускались**. `rustfmt --edition 2024 --check` для шести файлов и `git diff --check` прошли. API quick-xml/rust_xlsxwriter/calamine прочитаны по локальным исходникам. Сборки, cargo test/check/clippy, GUI и CI не запускались; типизация и runtime не подтверждены.

Первичные источники сверены 2026-10-15: [DAYS: вычитание/DATEVALUE/домен](https://support.microsoft.com/en-au/office/days-function-57740535-d549-4395-8728-0f07bff0b9df), [SUMIF: проекция sum_range](https://support.microsoft.com/en-us/excel/functions/sumif-function), [MROUND: знаки](https://support.microsoft.com/en-us/excel/functions/mround-function), [MS-OE376 RATE: nonzero type](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/694915de-291c-41b3-923a-01b02b0e9f73), [PMT](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/467f8e9a-a8ca-4efa-8d04-d8b010c02181), [OOXML CellValues](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellvalues), [absoluteAnchor pos/ext](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.spreadsheet.absoluteanchor), [custom E-/E+](https://support.microsoft.com/en-us/excel/review-guidelines-for-customizing-a-number-format).

Ограничения: дополнительный clone/validate требует измерений на больших книгах. Text dates ограничены существующим parser ISO/ru/en полных числовых дат, без month names/partial dates/time. Chart сохраняет прежние лимиты preview. Negative absolute coordinates, offsets/геометрия вне лимитов модели не расширены. Fractional column widths округляются до пикселя. Полный Excel corpus и engineering scientific patterns не заявляются. Независимое чтение Docs/Slides/root выполняется до публикации; runtime-приёмка отложена владельцем.

Независимое чтение production diff Docs editor/model/io и Slides editor/io завершено: проверены stable-cell rebase/canonical merge, baseline TextEdit до/после widget, typing inheritance, удаление внутренних blank-page spans, namespace lookup по stack и малая геометрия. Блокеров не найдено. Прочитаны root app/ops/mcp/workspace/tools/chat: проверены allowlist/versions, отмена/отзыв прав перед commit, atomic preflight, сохранение Sheets view, transient worker history и асинхронный запуск chat. Найден schema blocker: structured_error({error}) не соответствует новому workspace outputSchema; ведущий расширил workspace outputSchema через anyOf [success, {error:string}], сохранив structured_error. Эта финальная правка перечитана независимо; blocker закрыт. Ревью ограничено чтением исходника, без типизации/исполнения.
