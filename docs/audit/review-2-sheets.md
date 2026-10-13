# Большое ревью 2: Sheets и независимая проверка MCP/ops/recovery

Дата: 2026-10-13. Автор: агент Sheets review 2. Проверены sheets/{formula,model,io,format,editor,tests}. Независимо повторно прочитаны mcp/tools.rs, ops.rs, recovery.rs, native.rs и закреплённые исходники calamine 0.36.1 (xlsx/cells_reader.rs, xlsx/mod.rs, datatype.rs). Первая волна отчётов Docs/Sheets/Slides прочитана до проверки. Это самостоятельная вторая волна, выполненная до сборки NSIS.

Динамические cargo/test/fmt/clippy проверки запускает root под общей claim. Агент не выполнял Office COM, Windows GUI, fuzzy imports, synthetic performance benchmarks или installer smoke. Ни один из таких результатов ниже не выдуман. Root сообщил о завершённом общем запуске: 109 app +14 shell =123 PASS; cargo fmt/clippy clean; debug file/MCP/UI smoke PASS. Это подтверждение перед изменением молодых dependency pins; после смены Cargo.lock проверки надо повторить. Агент самостоятельно не запускал общий cargo/GUI.

## Найденные и исправленные дефекты собственной области

| ID | Уровень | Конкретный сценарий и причина | Исправление и проверка |
|---|---|---|---|
| S-R2-01 | high | TODAY/NOW = unix_seconds/86400 +25569 давали UTC. В Asia/Almaty локальное 00:30 могло стать предыдущей датой. | chrono::Local → local calendar serial. Helper принимает NaiveDateTime; fixed-date regression не меняет process timezone. |
| S-R2-02 | high | Удаление первой/последней строки SUM(A1:A3) заменяло удалённую endpoint на #REF! и портило ещё существующие значения. | Разбор пары A1:A3 как range, shrinking общих bounds; полный удалённый range становится #REF!, одиночные refs сохраняют стандартное #REF!. Абсолютные $, reversed ranges, другая/кириллическая sheet name проверяются регрессией. |
| S-R2-03 | high | Удаление листа оставляло name references. Добавление листа с тем же именем оживляло формулу и связывало её с чужими данными. | Полные references удалённого листа превращаются в #REF!, включая defined names; active index корректно сохраняется после удаления предыдущего листа. |
| S-R2-04 | high | Row/column insertion проверяла только cells. Высоты/hidden/merge/chart anchor могли выйти за Excel bounds; partial delete оставлял merges вместо удаления. | Metadata/range/anchor preflight перед изменениями. Общий interval adjust; полностью удалённые merge/conditional/filter/chart range убираются. Selection и frozen pane сдвигаются. Negative operation test проверяет неизменность модели. |
| S-R2-05 | medium | Defined names не обновлялись после rename и structural edits. | Names проходят те же rename/structural transformations. Простые absolute names включены в regression; scoped names/structured refs остаются ограничением. |
| S-R2-06 | high | Native JSON key "$A$1"/"a1" проходил validate, но cell() искал "A1" и скрывал данные. Пересекающиеся merges тоже принимались. | Канонический key==address обязательный. Merge validation через sweep/BTreeSet O(n log n), без pairwise quadratic проверок. |
| S-R2-07 | high/security | REPLACE("abc",2,1E308,"x") делал pos + usize::MAX; text "NaN" в MID/INDEX становился NaN number, затем usize subtraction underflow. | Saturating count arithmetic и finite coercion Value::Text→f64. Публичный shift_references использует checked_add. Extreme regression. |
| S-R2-08 | high/security | SUBSTITUTE с коротким old и длинным replacement выделял гигабайты прежде проверки 32767; &/TEXTJOIN могли материализовать огромный результат. Большие repeated reference grids расходовали неограниченную память. | Bounds перед expansion/concat и incremental TEXTJOIN; input len ≤32767, cumulative 1 млн work/64 МБ reference materialization budget с #NUM! по пределу. Это предел реализации, не Excel performance achievement. |
| S-R2-09 | high | XLOOKUP с найденным ключом и if_not_found=NA() заранее вычислял NA и возвращал ошибку. Lookup exact "<3" применял критерий сравнения, а не literal. MATCH mode 0 не имел wildcard. | Lazy fallback только при отсутствии значения; отдельный literal/wildcard matcher. MATCH/XMATCH/XLOOKUP проверяют форму массива и modes; binary search ±2 явно возвращает #VALUE! до его полной реализации. |
| S-R2-10 | high/security | calamine read_shared_strings() делал reserve(uniqueCount) из untrusted XML, read_formula_record() — resize(si+1) до проверок модели. Большой индекс в нескольких байтах вызывал OOM. | До calamine ограничены count/uniqueCount ≤1 млн, si ≤100k, f/@ref area, XML depth/DTD. Добавлены huge integer cases; чтение кода зависимости подтверждает порядок allocation. |
| S-R2-11 | high/security | Preflight implicit row не увеличивал row после </row>. Shared string 32767 bytes, повторённая тысячами cell refs, раздувалась до большого Range<String/Data>. | Отдельный streaming worksheet bounds parser отслеживает implicit rows и aggregate expanded text/formulas ≤64 МБ, проверяет shared strings refs. SI до anchor отвергается явно. Regression 2100 repeated strings превышает бюджет. |
| S-R2-12 | high | Date1904 raw serial читался и записывался как 1900; отображение менялось на 1462 дня. | workbookPr date1904=1/true отклоняется с объяснением. Конверсия полной системы даты пока не реализована; честный unsupported gate вместо неправильной даты. |
| S-R2-13 | high | worksheet_formula().ok() скрывал parse error; xlsx сохранялся после тихой потери формул. | Ошибка formula import теперь обязательный Result с названием листа. XLS/XLSB остаются запрещены до изоляции бинарного импортёра. |
| S-R2-14 | medium | mm:ss воспринималось как месяц; Unicode prefix делал char offset ошибочным byte offset; [h] вырезался; AM/PM был literal. | Context token parser без byte slicing, elapsed [h]/[m]/[s] preserved, 12-hour AM/PM; mixed Unicode/date/time regression. Полный Excel custom formatter остаётся недостигнутым. |
| S-R2-15 | medium | Bold в grid имитировался увеличением size, italic не применялся; scatter negative X выпадал за plot из xv/xmax. | Правильные системные font families bold/italic/bold-italic; scatter X нормализуется по xmin..xmax. Полный font selection/wrap/rotation и внешнее visual QA остаются задачами. |

## Перекрёстное повторное ревью общего кода

| ID | Уровень | Сценарий | Передача / состояние исходников |
|---|---|---|---|
| X-R2-01 | high | MCP create_document: path=None, version=saved_version=0, dirty=false. Закрытие пустой новой книги не спрашивало сохранить; recovery не снимал её. | Передано root. После исправления Tab::new устанавливает saved_version=u64::MAX для path=None. Свежий код прочитан; runtime диалога ещё не проверен. |
| X-R2-02 | high | Recovery clear(id) ещё в очереди; новое редактирование вызывает snapshot(). Наличие clear заставляло пропустить insert, но вернуть true. App помечал dropped snapshot version как queued. | Root исправил unconditional pending.insert; worker делает clear прежде snapshots. Добавлен clear_followed_by_snapshot_is_durable. Тест ожидает общего запуска. |
| X-R2-03 | medium | Undo/Redo pop и assignment модели выполнялись раньше checked_add(version). На u64::MAX ошибочный вызов уже менял содержимое и историю. | Root вычисляет next_version до pop. Свежий код прочитан; added undo_failure_is_atomic ожидает test run. |
| X-R2-04 | medium/security | Author::Agent client.chars().take(80), а undo сравнивал с полным client/session label. Длинное имя удаляло session suffix и запрещало own undo. | Root сохраняет полный client.to_string(); Server ограничивает имя до 64 chars до добавления session ID. Свежий код повторно прочитан после rustfmt; проблема truncation устранена. |
| X-R2-05 | high/performance | sheets_read_range до 100k ×32767 chars, docs_get_outline/find и model reads сериализовались в UI прежде cap. | Передано root. Root сообщил bounded read-range 256 и bounded Docs find API. Свежий code подтверждает range cap 256, outline pagination 100, Docs find_page bound. Runtime/UI latency и общий response byte cap остаются у root. |
| X-R2-06 | medium/recovery | В одной worker итерации clear+new snapshot: remove old failed и clear requeued, new snapshot может успешно записаться; следующий retry clear удаляет уже новый snapshot. | Root заменил clear/pending на ordered Work с sequence/latest и error retry только для актуальной операции. Новый snapshot делает предыдущий failed clear устаревшим. Свежий Queue::finish прочитан; dynamic regression входит в общий запуск root. |
| X-R2-07 | high | sheets_read_range вычислял checked_sub+1 в u32 до Range::validate: r0=0,r1=u32::MAX → debug panic/release wrap. sheets_get_cell_info с row/col=u32::MAX попадал в address(row+1,col+1). | Root добавил r.validate() прежде count и MAX_ROWS/MAX_COLS guards в get_cell_info. Свежий код повторно прочитан; overflow путь закрыт. |
| X-R2-08 | high/semantics | mode/base_version с неверным JSON типом as_str/as_u64→None молча принимался как apply/no version guard. Shared schema SDK не является гарантией validation. | Root добавил presence+type validation для mode/base_version перед special cases. Свежий код прочитан. |
| X-R2-09 | medium | create_document title до 1 МБ не проверяется; set_selection игнорирует sheet_id, sheets_sort не имеет header аргумента. | Root ограничил title и переключает active sheet при set_selection с sheet_id. Сортировка header остаётся частичной API и отмечена как ограничение. |


Исходники recovery теперь повторяют failed writes и coalesce pending snapshots, поэтому S-R1-08/09 улучшены. Потеря записи из-за ошибочного clear suppression устранена. Но очередь на disk failure и shutdown должна иметь реальные filesystem tests, а crash recovery не выводится из одного happy-path теста.

Native Document::validate теперь делегирует Book::validate. Ops содержит общий read-only preflight, MCP undo защищён latest own author; чужая пользовательская правка безопасно отклоняется. Selective AI revert при последующих пользовательских шагах пока возвращает конфликт, а не реализует операционное преобразование.

## Остаточные ограничения и непройденная приёмка

- Лист рисует видимые ячейки, но root клонирует целую модель каждый кадр; пересчёт full-book, incremental graph отсутствует. 100k/60 fps и ≤50 мс не измерены и не подтверждены.
- Sort имеет частичную типовую семантику: mixed numeric/text/bool/errors/blanks и Excel blank-last порядок требуют отдельного comparison corpus. Filter UI, conditional-rule editor, special paste, series/date fill, pivots/validation/tables/dynamic arrays ещё неполны.
- Безопасные budgets уже ограничивают больше данных, чем Excel. Превышение явно возвращает ошибку. Полная поддержка XLS/XLSB, 1904 dates и child-process isolation не выполнена.
- XLSX style metadata импортируется из стандартного xl пути; theme/indexed colors, master metadata/unknown OOXML preservation, nonstandard relationships и full chart styles/axes/anchor roundtrip неполны.
- Shared string/si allocation tests являются синтетическими. Корпус 50 реальных XLSX, cargo-fuzz, внешнее Microsoft Excel roundtrip и Windows 10/11 IME/DPI/clipboard ручные сценарии не запускались этим агентом.
- PDF сохраняет populated page blocks без far-cell truncation, но chart export, print area/repeat headers/scale и layout с wraps/merges/rotation не соответствуют всему плану.
- Полный каталог Excel функций содержится в parity-sheets.md. Наличие ~140 имён и >150 numerical cases не доказывает полный паритет или одинаковые пограничные Excel semantics.
- Установщик сам по себе не закрывает эти пробелы; preview не следует называть полностью готовой заменой Excel.

## Первичные источники

- [Microsoft: NOW](https://support.microsoft.com/en-us/excel/functions/now-function) и [VBA Now: system date/time](https://learn.microsoft.com/office/vba/language/reference/user-interface-help/now-function) — локальное текущее календарное время. Вывод о проблеме UTC сделан из сравнения с кодом.
- [Microsoft: detect formula errors](https://support.microsoft.com/en-us/excel/detect-formula-errors-in-excel) — continuous range автоматически обновляется при удалении промежуточного столбца.
- [Microsoft: date systems](https://support.microsoft.com/en-us/excel/date-systems-in-excel) — системы 1900/1904 и разница 1462 дня.
- Закреплённые локальные исходники calamine 0.36.1: sharedStrings reserve в xlsx/mod.rs; shared formula resize и row counter в xlsx/cells_reader.rs. Из них выводятся preflight bounds, не из предположений.

Все источники прочитаны 2026-10-13. Два ревью состоялись как анализ конкретного состояния кода; общий тестовый/релизный итог публикует root после исправлений и запуска проверок.

## Итог общего запуска после исправлений

По сообщению root: 123 tests PASS (109 app/14 shell), fmt/clippy без предупреждений; debug sample files +MCP+UI smoke PASS. Второе ревью нашло и закрыло реальные overflow/data-loss/recovery проблемы. Однако новый dependency audit обнаружил 28 pins моложе14 дней; текущие успешные тесты не являются разрешением обойти этот gate. После downgrade полного lock необходим новый тестовый запуск и обновление отчёта.
