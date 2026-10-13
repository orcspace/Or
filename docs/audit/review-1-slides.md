# Ревью 1 — независимая проверка оболочки, MCP, Sheets и Slides

Дата: 2026-10-13. Автор: агент Slides; исходники других владельцев не изменялись. Проверка выполнена до сборки установщика. Строки указаны для исходного компактного кода; после rustfmt ориентироваться на функцию и сценарий.

Проверены `app.rs`, `file_io.rs`, `native.rs`, `ops.rs`, `settings.rs`, `recovery.rs`, `package.rs`, `mcp/server.rs`, `mcp/shim.rs`, `mcp/tools.rs`, `sheets/model.rs`, `sheets/formula.rs`, `sheets/io.rs`, `sheets/format.rs`, `sheets/editor.rs`. Отдельно принят cross-review агента Docs для Slides. Это анализ исполнения и сценариев, а не переименование результатов тестов в «ревью».

Найдено в чужом коде: 7 high и 7 medium. Они переданы root и владельцу Sheets; исправление требует повторной проверки во второй волне. В Slides устранены 7 дефектов; ограничения форматов остаются явно описанными.

| ID | Место | Серьёзность | Сценарий и результат | Требуемое действие |
|---|---|---|---|---|
| R1-S01 | app.rs:41, Event::Opened | high | Править открытый файл, затем снова открыть его через меню/Недавние: модель заменяется с диска без проверки dirty и без вопроса, undo очищается | При уже открытом файле переключать вкладку; перезагрузка — отдельная подтверждённая операция |
| R1-S02 | file_io.rs:31–32, Job::Save; app.rs:29, write | high | Сохранить как существующий файл с чужим ~$ lock: atomic_write выполняется до попытки acquire lock. Save открыт и для read_only вкладки | Проверить владение lock до записи; readonly только Save copy/as в другой путь |
| R1-S03 | app.rs:116, close_after_save; file_io.rs:26, PickSave | high | Закрыть изменённый новый файл → «Сохранить» → отменить picker → позже Ctrl+S: старый close_after_save неожиданно закрывает вкладку | Явный Event::CancelledSave и сброс continuation; сбрасывать также на ошибке/отмене format dialog |
| R1-S04 | mcp/tools.rs:39 и create_document; app.rs:62–69, special tools | high | undo(mode=dry_run) всё равно вызывает apply(Undo). create/save/close также обрабатываются до общей проверки mode/base_version | Единый preflight режимов до любого изменения/записи; dry-run должен иметь нулевые side effects |
| R1-S05 | native.rs:12, Document::validate | high | Native JSON / replace_document_model с повторяющимися stable ID, слишком большими размерами/шрифтами/геометрией или неверными индексами проходит только shallow validation | Делегировать полным валидаторам Docs/Book/Deck; проверить ID/границы/finite/next_id и лимиты всей модели |
| R1-S06 | sheets/formula.rs:163; sheets/tests.rs:14 | high | =2^3^2 хранится/проверяется как 512. По порядку одинаковых операторов Excel слева направо результат должен быть 64 | Левая ассоциативность ^; исправить неверный ожидаемый тест, добавить скобочные варианты |
| R1-S07 | sheets/formula.rs:433–442, даты | high | =EDATE(1,-1E308), =WORKDAY(1,-1E308), =DATE(2024,-1E308,1) приводят finite f64 к i64::MIN затем abs(); NETWORKDAYS(-1E308,1E308) переполняет hi-lo; WEEKDAY(-1E308) переполняет serial-1 | Проверять допустимый диапазон f64 перед cast; checked math и лимит итераций; крайние regression cases |
| R1-S08 | app.rs:108, «Оставить правки» | medium | stamps.remove выключает последующее сравнение watcher для вкладки навсегда; будущие внешние правки не обнаруживаются | Обновлять подтверждённую исходную stamp, продолжать watcher |
| R1-S09 | app.rs:118, Pending::Exit | medium | «Сохранить» при закрытии приложения сохраняет/закрывает одну вкладку и забывает намерение завершить приложение | Состояние exit continuation: последовательно сохранить dirty вкладки, затем Close |
| R1-S10 | mcp/shim.rs:29, reqwest::Client::new | medium/security | Клиент по умолчанию имеет auto_sys_proxy=true: HTTP_PROXY может отправить запрос с Bearer за пределы loopback | ClientBuilder::no_proxy; локальная проверка localhost должна охватывать транспорт клиента |
| R1-S11 | mcp/shim.rs:30; app.rs:58, allowed.contains(client) | medium/security | Все stdio-клиенты называются «TrineDocs stdio»; разрешение одному распространяется на остальных. Прямой клиент может указать уже разрешённое имя | Идентичность/разрешение по сессии или отдельному credentials; client name — только метка |
| R1-S12 | mcp/tools.rs:36, read_content/list/read; app.rs:66 dispatch | medium | Полный документ сериализуется в UI без pagination/response cap; большой read_content и несколько вызовов блокируют UI и увеличивают память | Пагинация/объекты/лимит ответа; тяжёлые чтения вне UI после bounded snapshot |
| R1-S13 | ops.rs:45–53, Undo/Redo | medium | Agent undo вызывает глобальный undo и может отменить последнюю пользовательскую правку; проверка read_only отсутствует у Undo/Redo | Авторские undo-группы либо безопасный отказ при несовпадении автора; общая readonly проверка |
| R1-S14 | file_io.rs:42, Worker::Drop; settings load/save | medium | Drop синхронно ждёт worker, который может находиться в picker/долгой записи. Settings/recovery write errors местами игнорируются, пользователь не видит потерю durability | Контролируемое завершение worker и видимый status сохранения/recovery failure; проверить shutdown отдельно |

Наблюдения, не объявленные исправленными: вся активная модель клонируется каждый UI кадр; incremental recalc/реальный граф зависимостей ещё не доказаны. Живой MCP курсор сейчас представлен только status label, без точной рамки/каретки. Эти пробелы являются ограничениями реализации, а не доказанными гонками.

Исправления собственной области Slides после независимого cross-review:

| Дефект | Сценарий | Исправление и проверка |
|---|---|---|
| Коллизия image texture cache | Две презентации содержат одинаковые ID, размер и первые 32 байта PNG, но разные pixels | Ключ по хэшу полного содержимого; безопасный decode_image из общего util |
| Отрицательные значения диаграммы рисовались вверх | Данные [-10,30] | Общие chart_bounds; нулевая baseline и signed bars на экране и PDF; добавлен unit |
| Потеря переноса текста PDF | Длинный текст в узком text box | Общие text_lines по метрикам системного шрифта; UI/PDF используют одинаковые строки, unit сохраняет весь Unicode текст |
| Контур прямоугольника отсутствовал в PDF | Цветная фигура с контуром | Контур внесён в display list |
| Стрелка PDF не имела наконечника | ElementKind::Arrow | Добавлены линии наконечника с той же геометрией, что egui painter |
| Автопереход расходовал click animations каждый frame | Slide advance_after плюс on_click элементы | Автопереход перемещает сразу на следующий видимый слайд, не имитирует клики и прекращается на конце |
| PPTX custom JSON мог скрыть правки PowerPoint | Изменить OOXML слайда после экспорта из TrineDocs | JSON доверяется только при совпадении manifest остальных ZIP частей; regression test изменяет внешний XML |

Ранее в реализации до ревью устранены ошибочные DrawingML chart/table namespaces, rot на graphicFrame transform и использование per-frame drag_delta вместо total_drag_delta. Есть проверки native/PPTX round-trip, namespaces, DTD/повреждённого XML, ID и показа. Новые тесты пока ожидают общего запуска root; зелёным релизное состояние не объявляется.

Подтверждения защиты: ZIP до 64 МБ / inflated 256 МБ, путь ZIP enclosure, duplicate entries запрещены; DTD отклоняется; внешние PPTX relationships не загружаются; rmcp 3.5.0 default Host allowlist содержит только localhost/127.0.0.1/::1, enforce_origin_validation с пустым списком отклоняет присутствующий Origin. Bearer сравнивается без раннего различения байтов.

Первичный источник для R1-S06: [Microsoft — порядок вычисления операторов Excel](https://support.microsoft.com/en-gb/excel/the-order-in-which-excel-performs-operations-in-formulas), прочитан 2026-10-13. Результат 64 является выводом из опубликованного порядка одинаковых операторов; Excel COM в этой проверке не запускался. Для transport proxy и Host/Origin прочитаны закреплённые исходники reqwest 0.13.5 и rmcp 3.5.0 из Cargo registry.

До установщика необходима вторая волна ревью именно исправленных путей и проверка сценариев. Полный Office-паритет, второй монитор, реальные 50 PPTX/50 XLSX/50 DOCX, IME/DPI, 60 fps и совпадение PDF с экраном для вращённых объектов не достигнуты; подробный список в `docs/parity-slides.md`.
