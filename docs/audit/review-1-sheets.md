# Большое ревью 1: Sheets → Docs / общая инфраструктура

Дата: 2026-10-13. Проверка кода независимым агентом Sheets. Сценарии ниже подтверждены чтением реализации и API зависимостей; Windows GUI и fuzz в этом ревью не запускались. Общий запуск cargo выполняет root.

| ID | Уровень | Место / сценарий | Доказательство | Статус |
|---|---|---|---|---|
| S-R1-01 | blocker | `pdf.rs` прежние строки 86–87: PDF с обычным текстом и несколькими картинками | Цикл вызывал `p.resources()` для каждого шрифта/изображения. В pdf-writer `resources()` делает `insert(/Resources)`, а не дополняет dict. Дубли ключей оставляют последний dict, поэтому F1 или предыдущие XObjects не определены. | Root исправил: один resources, один fonts/XObjects writer; визуальная проверка нужна. |
| S-R1-02 | high | `pdf.rs:39`: ширины нескольких глифов | `cid.widths()` делает `insert(/W).array()`; вызов в каждом iteration дублировал /W и терял предыдущие widths. | Root исправил: единый widths writer. |
| S-R1-03 | high | `pdf.rs` прежняя строка 74, `docs/io.rs` прежняя 401, render image: PNG с огромными dimensions | Проверка числа пикселей после `image::load_from_memory` происходит после выделения растрового буфера. Размер ZIP/PNG не ограничивает распакованный растр. | Root добавил `util::decode_image` с проверкой dimensions до decode; Docs агент использует общий helper. |
| S-R1-04 | high | `util::read_file`: файл вырос после metadata | `fs::read` после проверки длины читает до EOF без лимита. | Root сменил на ограниченное чтение; static verification выполнена, race test не запускался. |
| S-R1-05 | high | `native.rs:validate`: нативная книга с duplicate IDs, next_id=1, A0, очень большими floats/диаграммами | Проверялись только top-level counts. Допустимое JSON-состояние могло ломать адресацию MCP, экспорт или UI. | Sheets реализовал `Book::validate` с глубокими ограничениями и negative tests; root должен вызывать его. Docs агент реализовал свой validator. |
| S-R1-06 | high | `docs/model.rs:render_table`: ячейка выше страницы | Раньше row_height обрезался, затем строки брались через `.take()`; хвост текста исчезал на экране/PDF без отчёта. | Передано Docs; агент реализует multi-page split. |
| S-R1-07 | high | `docs/io.rs` прежние 268/270: DOCX-таблица >10k строк или >256 колонок | `.take(10000)` и `.take(256)` молча отбрасывали остаток. | Docs заменил truncation на явный отказ по bounds. |
| S-R1-08 | high | `recovery.rs:13,20`: временная ошибка диска при snapshot | Worker игнорирует ошибку atomic_write; snapshot возвращает true при enqueue, app отмечает версию как сохранённую. Пока следующей правки нет, сохранение не повторяется и обещанное восстановление не обеспечено. | Root уведомлён; пока требует error/retry channel или worker retries. |
| S-R1-09 | medium | `recovery.rs:21`: закрытие при заполненной очереди | clear использует try_send и теряет Clear при full; snapshot закрытого/сохранённого документа остаётся и предлагается при следующем запуске. | Root уведомлён; требуется priority/coalescing queue. |
| S-R1-10 | medium | `file_io.rs:FileLock::acquire`: падение процесса | Lock-файл переживает crash, проверка владельца/liveness отсутствует; повторное открытие становится read-only. | Не исправлено в этом scope; нужен безопасный stale-lock recovery. |

## Собственные исправления по перекрёстному ревью других агентов

- XLSX preflight теперь проверяет все XML с корнем worksheet, независимо от стандартной директории, и ограничивает dimensions, фактические/неявные addresses и суммарные dense allocations до передачи calamine.
- ODS preflight ограничивает произведение повторов строк/столбцов, всю книгу, прогнозируемый объём клонированного текста и text:s repeats. Calamine создаёт dense Vec в Range::new до последующих пользовательских проверок; проверка только после read была бы поздней.
- XLS/XLSB отключены с явным сообщением до изоляции/защищённого binary import. Безопасные bounds не доказаны; поддержку нельзя объявлять рабочей.
- Формулы: Excel equal-precedence операции идут слева направо. `2^3^2` исправлено с 512 на 64. Источник: [Microsoft: order of operations](https://support.microsoft.com/en-gb/excel/the-order-in-which-excel-performs-operations-in-formulas).
- Все date offsets и serial values ограничены до cast в i64; `-1E308` больше не попадает в i64::MIN.abs и переполнение дней. Добавлено 11 extreme argument cases.
- Лимит глубины вычисления и сложности token stream защищает от stack overflow на длинной формуле.
- ISERROR/ISNA/TYPE сохраняют ошибку аргумента для проверки типа; lazy IF/IFS/IFERROR избегают вычисления невыбранной ветки.
- SIGN(0) исправлен на 0.

## Что не доказано

Корпус 50 DOCX/XLSX/PPTX, performance 100k, CJK IME, внешние Office roundtrip и визуальное совпадение PDF ещё не проверены этим агентом. Частичные функции и пропуски перечислены в `docs/parity-sheets.md`; отсутствие crash на unit examples не означает полный паритет.
