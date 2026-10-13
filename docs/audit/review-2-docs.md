# Большое ревью 2: Docs и повторная независимая проверка native/PDF/render

Дата: 2026-10-13. Автор: агент Docs второго ревью. Выполнено после отчётов первой волны и до сборки установщика. Исходники чужих владельцев не изменялись; все находки в native, package, PDF и оболочке переданы основному агенту.

Область: полный повторный проход docs/model.rs, docs/io.rs, docs/editor.rs; независимое чтение native.rs, pdf.rs, render.rs, package.rs, fonts.rs, ops.rs и путей сохранения app.rs/file_io.rs. Проверялось исполнение сценариев, безопасность входных форматов, потеря форматирования/текста, ограничения памяти, стабильные ID, ввод и положение курсора.

## Найдено и исправлено в Docs

| ID | Уровень | Сценарий | Исправление и регрессия |
|---|---|---|---|
| D-R2-01 | high | XML declaration → внешний перенос строки → document. Parser выбирал whitespace #text первым корнем; обычный DOCX не открывался. Несколько XML roots принимались. | Разрешён только один корневой element; whitespace снаружи игнорируется, посторонний текст/второй root отклоняется. Тесты outer whitespace и pretty DOCX. |
| D-R2-02 | high | Изменить один символ форматированной ячейки инспектором. Все runs заменялись одним форматом и заново создавались paragraph ID. | Minimal scalar diff → replace_range вложенного Doc. Нетронутые runs и IDs сохраняются. Regression bold/italic и split. |
| D-R2-03 | high | Форматированные таблицы в DOCX/нативном файле на экране/PDF показывались plain 11pt, без цветов, highlight и оформления. | Rich segments по реальным системным метрикам, variable line heights, общий render_row, продолжение высокой строки на следующих страницах. Проверки rich primitives и длинной строки без потери текста. |
| D-R2-04 | high | Отключить bold в Heading1, сохранить DOCX, открыть. Отсутствующий w:b наследовал bold из style, отменённое оформление возвращалось. | Явные true/false для b/i/strike и single/none для underline. Regression unbold heading. |
| D-R2-05 | high | Сохранить документ с картинкой и открыть несколько раз. Каждый раз появлялся дополнительный пустой абзац-контейнер рисунка. | Собственный TrineDocsImageBlock style у контейнера; импорт reanchor на предыдущий paragraph без добавления пустого model paragraph. Три последовательных roundtrips. |
| D-R2-06 | high | Agent вставляет newline перед пользовательским caret либо объединяет абзацы. Caret оставался в первом абзаце или прежнем индексе. | Diff основного текста с Unicode scalar offsets и обратным отображением в paragraph/offset. Regression split/merge и перемещение соседнего paragraph. Несколько независимых edits между кадрами пока могут приблизительно отображать позицию внутри изменённого middle. |
| D-R2-07 | high | IME Commit пустой строкой при выбранном тексте удалял выделение, хотя composition отменена или не активна. | Пустой commit только очищает preedit; newline/CR commit игнорируются согласно закреплённому egui. Regression cancellation и реальный Japanese commit на уровне модели. Это не ручной тест Windows IME. |
| D-R2-08 | medium | Табличный border_color менялся после DOCX reopen на hardcoded default. | Чтение tblBorders color; усилен existing roundtrip. Разные цвета отдельных границ модель ещё не поддерживает. |
| D-R2-09 | medium | Отрицательный first_indent сохранялся отрицательным firstLine. | Запись положительного hanging либо firstLine, не обоих. Проверка XML и повторного открытия. |
| D-R2-10 | medium | Выбрать Insert link без selection. Вставленный URL оставался обычным текстом, поскольку caret уже стоял в конце. | После вставки выделяется созданный range и получает link/underline. |
| D-R2-11 | medium | Картинка: picker/read/decode синхронно блокировали UI. | Read-only background thread, bounded util::read_file, safe image decode, результат через Arc/Mutex с repaint. Отмена безопасна, ошибка видна, удалённый anchor не подменяется другим местом. Ручной picker ещё не проверен. |
| D-R2-12 | medium | Tab/стрелки могли отдавать фокус следующему egui widget вместо canvas navigation. Ctrl+C без selection очищал системный clipboard. | Focus lock filter для canvas; copy/cut без selection игнорируются. Требуется ручная проверка Windows clipboard. |
| D-R2-13 | medium | Date вставлялась по UTC и могла отставать от местной даты. | chrono::Local date; зависимость добавлена главным агентом. |
| D-R2-14 | medium | Find строил полный Vec matches и перебирал каждый start × needle; MCP pagination делалась слишком поздно. | Новый find_page: ≤100 абзацев, ≤1000 matches; KMP O(n+m), исходные scalar semantics и overlapping matches. Общий find использует тот же линейный поиск. |
| D-R2-15 | medium | Egui temp EditorState клонировал весь Layout и предыдущий Doc каждый кадр. | Arc cache для Layout/previous, неизменный snapshot повторно используется. Root по-прежнему клонирует активную модель; цели 16ms/100pages не доказаны. |
| D-R2-16 | high/bounds | Валидация разрешала строку выше body area; custom узкие table widths и font size плохо учитывались page estimate. | Отказ для строки, которая не помещается в body; estimate учитывает действительные widths, размеры и soft breaks. Добавлен invalid page regression. |
| D-R2-17 | medium | Изображения/вложенные tables внутри DOCX cells молча терялись при импорте. | Конкретные losses; видимый список ограничений импорта внутри Docs UI. Полного сохранения unsupported OOXML ещё нет. |

alloc_id повторно проверен: монотонный O(1), scanning всех paragraphs в прошлой версии уже устранён. Добавлен regression 10 001 paragraph ID без таймингового утверждения.

## Повторная проверка исправлений первой волны

По свежим исходникам подтверждены единые PDF /Resources, /Font, /XObject и /W builders; style families у TextStyled в egui; полный hash images для cache; глубокий Document::validate, вызывающий валидаторы каждой модели; bounded read_file и проверка image dimensions до decode. Table rows больше не обрезаются take() по высоте страницы. DTD/XXE запрещены, network relationships не загружаются, ZIP traversal/duplicates и declared inflation bounded.

Эти подтверждения являются статическим чтением, а не результатом внешнего PDF renderer или Word.

## Новые находки в чужих модулях

| ID | Уровень | Путь и сценарий | Переданное действие |
|---|---|---|---|
| D-R2-N01 | high | native::encode сериализует Vec<u8> images числовыми массивами в document.json. Допустимые image byte budgets дают JSON part >64MB; encode/write_zip сохраняет его, read_zip затем откажет. | Ограничить сериализованный part и ZIP output/inflation либо вынести assets в отдельные части. Не создавать файл, который приложение не откроет. |
| D-R2-N02 | high | package::write_zip не отражает input part/aggregate/output bounds read_zip; случайный PNG/DOCX/PPTX большой export может стать неоткрываемым. | Symmetric limits на запись и явная ошибка до atomic_write. |
| D-R2-N03 | high/data loss | app Ctrl+S существующего DOCX идёт прямо в write; конкретные Doc.losses не запрашивают подтверждение. Предупреждение только generic SaveFormat после picker. Неподдержанный исходный OOXML может быть удалён без ясного сообщения. | Конкретный список losses/export warnings для UI и MCP save, подтверждение конкретной операции. Docs теперь показывает losses, но это не заменяет Save preflight. |
| D-R2-N04 | medium/bounds | Public pdf::write проверяет page dimensions, но не finite/bounds каждого Primitive и совокупные image decode allocations. | Validate primitives и aggregate pixel/byte budgets до PDF allocation. Root сообщил, что реализует эту защиту; повторный код после исправления должен быть проверен. |
| D-R2-N05 | medium/gap | PDF baseline size×0.90; экран egui использует font ascender/kerning. Arial/Segoe first font не имеет всех CJK. | Kerning/shaping/font fallback и pixel comparison не достигнуты. Сейчас отсутствующий glyph выдаёт явную ошибку, а не молча удаляет его. |

Состояние чужих исправлений указывает общий release-review после root tests. Настоящее font shaping, полный Unicode grapheme editing, полноценная rich canvas правка таблиц, merged cells, footnotes/sections/track changes/mail merge/accessibility, все Word функции и реальные OOXML corpus остаются незавершёнными.

## Реально выполнено и ещё не выполнено

В этой волне выполнены чтение текущих исходников/зависимостей и отчётов, анализ конкретных сценариев, правки только согласованной Docs области, проверка новых текстовых исходников и поиск запрещённых production unwrap/expect/panic/unsafe: в Docs их не найдено. Все новые tests переданы root для общего cargo test; агент не запускал конкурирующую сборку target, установщик или Windows UI.

Добавлены regression tests: bounded Unicode find/overlap; oversized lines; rich table rendering; 10k ID allocator; preserved cell formats; IME cancel/commit; caret split/merge; outer XML whitespace/multiple roots; pretty DOCX; explicit unbold/hanging indent; image repeated roundtrip. Existing 50 generated DOCX cases остаются синтетическими. Их нельзя считать 50 реальными документами.

Не выполнены этой волной: независимый Word/LibreOffice consumer, визуальный PDF render comparison, fuzzing, реальный corpus ≥50 DOCX, Windows 10/11 IME/DPI/Narrator/clipboard, cold start и 16ms measurements. Эти пункты нельзя объявлять зелёными по статическому ревью.

Первоисточники семантики: [Microsoft Bold: отсутствие элемента наследует style, false явно отключает](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.bold?view=openxml-3.0.1), [Microsoft Indentation / hanging](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.indentation?view=openxml-3.0.1); прочитаны 2026-10-13. IME cancellation/focus filter сверены с закреплёнными egui 0.36.2 src/widgets/text_edit/builder.rs и src/data/input/event_filter.rs.

## Повторное чтение исправлений root

После общего rustfmt/clippy --fix и последующих изменений прочитаны актуальные native.rs, package.rs, pdf.rs, app.rs. Подтверждены по исходникам:

- D-R2-N01: native Envelope v2 вынес binary image assets в assets/{id}.bin; manifest проверяет canonical path/ID, после восстановления вызывается глубокий validate. Version 1 остаётся читаемой.
- D-R2-N02: write_zip проверяет число частей, names/traversal/duplicates, размер каждой части, aggregate inflated budget и итог ≤64MB. Большие части хранятся Stored, поэтому собственный выход не попадает под read inflation ratio.
- D-R2-N03: Ctrl+S существующего non-native файла и MCP save теперь используют SaveFormat; в окне выводятся конкретные Doc.losses/Deck.export_warnings.
- D-R2-N04: PDF primitive coordinates/dimensions finite и bounded; общий текст ≤16MB, decoded images ≤32 million pixels; validation до font/PDF allocations.
- D-R2-N05: baseline теперь берёт font ascender, константа 0.90 убрана. Kerning/shaping/CJK fallback и visual equivalence всё ещё требуют внешнего QA.

Найдены ещё два дефекта в свежих root changes, переданы владельцу:

| ID | Уровень | Сценарий | Необходимое исправление |
|---|---|---|---|
| D-R2-N06 | high/regression | PDF positive(w/h)>0 отклоняет нулевую chart bar Rect h=0. Slides model/test специально содержит такой primitive. Deck допускает stroke_width=0, PDF отклоняет Line width=0. | Допустить zero extents/width как no-op и пропустить их при PDF render либо не выдавать такие primitives из модели. Валидная диаграмма с нулевым значением должна экспортироваться. |
| D-R2-N07 | medium | MCP save → SaveFormat → «Отмена»: mcp_saves entry/reply не очищаются. После timeout следующие save_document блокируются contains_key. | Отмена любого save confirmation и remove(tab) должны ответить error и удалить entry; cleanup closed replies. |

ImageTask result получил alias ImageResult для оставшегося clippy::type_complexity. Общий повтор tests/clippy запускает root; это дополнение не объявляет их завершёнными.

## Дополнительная независимая проверка UI smoke и Windows policy

После root сообщения о 107 app + 13 shell tests PASS (до новых UI smoke changes) выполнено read-only чтение main.rs, ui_smoke.rs и App::new/ui/logic. Собственная сборка не запускалась.

shell-core содержал MIN_WINDOWS_BUILD=22000 и regression, специально отвергающий Win10 19045. Это противоречит AGENTS и принятому ADR-0025. Под отдельной claim изменены только shell-core platform.rs/lib.rs: минимальная policy build10240, Win10/Win11 x64 workstation accepts, прежние server/ARM restrictions сохранены, Mica/backdrop capability по-прежнему начинается с22621. Новые meaningful tests проверяют build10240/14393/17763/19045/22000/22621/26100, lower boundary/NTmajor, capability separation server/ARM. Это unit policy, не native compatibility proof. [Microsoft release history](https://learn.microsoft.com/en-us/windows/release-health/release-information) подтверждает Win10 1507 RTM build10240; источник прочитан 2026-10-13.

Переданы root:
- UI-SMOKE-01 high: main учитывает только наличие result.json. Старый result при повторном запуске может превратить новый timeout/screenshot failure в exit0. Нужен свежий run identity или удаление только предыдущего result перед запуском.
- UI-SMOKE-02 medium: --ui-smoke-test без output directory запускает обычный GUI без smoke timeout; ошибочный CI command может зависнуть. Нужна явная argument error.
- UI-SMOKE-03 medium: fonts::install error записывается в App.error, но smoke всё равно может объявить passed. Smoke должен fail при обязательной font installation error.
- UI-SMOKE-04 limitation: четыре screenshots подтверждают захват кадров, но не работоспособность editing/actions и не визуальную правильность. Требуется просмотр actual artifacts и UI scenarios.
- docs/testing/acceptance.md всё ещё содержит старое Win10 reject/min22000 и Win11-only baseline; main не делает actual WindowsHost probe/validate. Передано root для его scope.

Подтверждено: UI smoke выключает MCP/settings load, не открывает nonflag arguments как документы, не показывает существующие recovery entries; sample tabs помечены clean saved_version0. Это уменьшает нежелательные filesystem side effects, но smoke mode остаётся обычным app UI, и пользовательские изменения в тестовом окне могут создать recovery. Статические findings не подменяют manual Windows10/11/DPI/IME/Narrator QA.

## Независимое read-only ревью NSIS и installation smoke

Прочитаны installer/trinedocs.nsi, tools/test_installer.ps1, tools/build_release.ps1, release.yml, обновлённые main.rs/ui_smoke.rs/App. Выполнен PowerShell AST parse test_installer.ps1 без syntax errors; проверен read-only Join-Path с absolute child. NSIS build, установка, реестр и uninstall этим агентом не запускались и не изменялись. Единственная мутация этой части — данный report под отдельной claim.

Подтверждено по исходникам:

- RequestExecutionLevel user, current shell context, HKCU через SHCTX, LOCALAPPDATA/Programs по умолчанию. All-users требует Admin и выбирает ProgramFiles64/HKLM; автоматического UAC повышения нет, UI явно требует запуск от администратора.
- Office extensions получают только OpenWithProgids и SupportedTypes; их default values/UserChoice не перехватываются.
- Native defaults сохраняются в AssociationBackup и возвращаются только если текущий default всё ещё собственный TrineDocs ProgID. Более поздний чужой default не перезаписывается.
- Uninstall удаляет конкретные payload filenames/ярлыки, без wildcards и recursive RMDir; unknown files в install/start-menu directory и APPDATA сохраняются. Перед registry actions проверяется совпадение InstallLocation с INSTDIR.
- UI-SMOKE-01/02/03 исправлены по свежим исходникам: prepare удаляет старый result, main требует directory argument, verify проверяет status/rendered и существование/минимальные размеры всех четырёх PNG; App.error с font installation failure останавливает smoke. Наличие четырёх кадров всё ещё не доказывает editing/visual correctness.

Новые находки, все переданы root:

| ID | Уровень | Сценарий | Исправление / необходимая проверка |
|---|---|---|---|
| INSTALL-R2-01 | high/test isolation | App smoke Start-Process ArgumentList array содержит необрамлённый path. В Output с пробелами main получает лишь первый token, пишет вне предполагаемой папки. При reused output files/mcp старые result могут удовлетворить проверку. | Безопасно передать argv через ProcessStartInfo.ArgumentList либо корректный quoted command; свежие/unique smoke directories или run identity. NSIS /D= и _?= имеют отдельную семантику: последнее значение без quotes. |
| INSTALL-R2-02 | high/data safety | Отсутствие uninstall registry key не означает отсутствие Desktop/StartMenu TrineDocs.lnk и Classes/TrineDocs.*. Test installer перезапишет и uninstall удалит такие существующие записи/ярлыки. | Abort до установки при existing owned keys/shortcut filenames либо exact snapshot/restore; не трогать неизвестные остатки на рабочем профиле. |
| INSTALL-R2-03 | medium/false PASS | Test запускает installed Uninstall.exe с _?=Destination. Этот флаг запрещает self-copy to temp; exe исполняется из install directory и остаётся занятым. Delete собственного Uninstall.exe не гарантирован; test проверяет только main exe/unkey. | Запускать external copy с _?=Destination или после exit удалить только verified known Uninstall.exe, затем empty-directory removal. Проверять весь payload/shim/shortcuts/ProgIDs/OpenWith cleanup и сохранение unknown-file sentinel. |
| INSTALL-R2-04 | medium/test arguments | Join-Path taskRoot с absolute Setup/Output не сохраняет absolute child. Реальная read-only проверка дала V:/Desk/Or/C:/test/path.exe. | IsPathRooted → GetFullPath исходного пути; relative → Join-Path taskRoot. |
| INSTALL-R2-05 | medium/test cleanup | try/finally начинается после запуска installer и payload existence check. Partial install failure оставляет files/registration. APPDATA изменяется для процесса, но не восстанавливается при возврате/ошибке. | Общий finally вокруг installation и smoke с контролируемым cleanup и восстановлением env/cwd. Проверять ExitCode и результаты в fresh scope. |
| INSTALL-R2-06 | medium/hang | Start-Process -Wait без external timeout может висеть до CI job timeout, если процесс не дошёл до внутреннего UI timeout или завис worker/shutdown. | Bounded wait, завершить именно spawned process/tree, дождаться выхода и выполнить cleanup. |
| INSTALL-R2-07 | medium/native metadata | AssociationBackup строкой не различает missing default и существующий пустой REG_SZ. При uninstall empty backup удаляет value, меняя точное состояние и потенциальное HKCU/HKLM inheritance. | Сохранить presence flag и восстановить explicit empty string, если она существовала. |
| INSTALL-R2-08 | medium/upgrade | File copies идут прямо поверх существующих binaries, ошибка проверяется после всех четырёх. Locked main/shim может оставить смешанную версию. Uninstall удаляет main перед проверкой shim lock и может abort после частичного удаления. | Transactional staging/preflight либо явный отказ при existing/locked install в preview. Не заявлять upgrade/rollback validated по clean install. |
| INSTALL-R2-09 | medium/relocation | .onInit не читает прежний InstallLocation. Reinstall в default вместо прежней custom directory переписывает registration, старый uninstaller затем отказывает по mismatch; старые files остаются. | Reuse registered location или требовать uninstall старой установки перед изменением пути. |

All-users/UAC, переустановка, locked binaries, foreign desktop shortcuts, custom native defaults/empty values, default HKLM fallback, cleanup после partial failures и reparse paths в этой волне не запускались. Lexical Destination-under-Output guard и new destination полезны, но не доказывают отсутствие filesystem reparse redirection. В script нет recursive deletion, поэтому нет обнаруженного wildcard/recursive удаления пользовательских документов.

Первоисточники: [NSIS scripting reference: SHCTX/shell context/Delete/RMDir](https://nsis.sourceforge.io/Docs/Chapter4.html), [NSIS command-line semantics: /D и _?=](https://nsis.sourceforge.io/Docs/Chapter3.html), [Microsoft Start-Process argv quoting](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/start-process?view=powershell-7.5). Прочитаны 2026-10-13. Все findings выше являются анализом текущего кода; успешный installer smoke должен быть зафиксирован root отдельно с actual artifacts/ExitCodes.

## Финальный повторный проход installer/test/UI после исправлений

Read-only перечитаны актуальные NSIS, installation test и UI smoke. Подтверждены исправления по source, установка этим агентом по-прежнему не выполнялась.

| Finding | Обновлённое состояние |
|---|---|
| INSTALL-R2-01 | Закрыт по исходникам: app argv передаётся отдельными ProcessStartInfo.ArgumentList entries; Output должен быть новым; NSIS /D и _?= передаются raw последними без quotes. |
| INSTALL-R2-02 | Закрыт для test current-user scope: prior own Classes keys проверяются в HKCU/HKLM, current Desktop/StartMenu guard, existing HKCU/HKLM uninstall registration запрещена. |
| INSTALL-R2-03 | Исправлен основной дефект: external uninstall copy, _?=installed path, assertion установленного Uninstall.exe после выхода. Unknown-file sentinel и полная проверка каждого notice/shortcut/ProgID в test ещё отсутствуют. |
| INSTALL-R2-04 | Закрыт: explicit IsPathRooted branches для Setup/Output. |
| INSTALL-R2-05 | Частично закрыт: установка и payload checks находятся внутри try/finally; исходный APPDATA сохранён. Остаются exception-path детали ниже. |
| INSTALL-R2-06 | Закрыт по исходникам: bounded 60s wait, Kill именно spawned process/tree и await exit, ненулевой код вызывает failure. |
| INSTALL-R2-07 | Закрыт для обычного string default: backup presence DWORD различает missing и explicit empty REG_SZ. |
| INSTALL-R2-08 | Preview не выполняет upgrade: existing registration/files запрещены до copy. Locked main/shim preflight обоих до Delete; silent MessageBoxes имеют defaults; WriteUninstaller errors checked с cleanup. Race между закрытием preflight handles и Delete не является transactional rollback. |
| INSTALL-R2-09 | Закрыт явным запретом preview reinstall/relocation при existing install; полноценные upgrades не объявляются готовыми. |
| UI-SMOKE-01/02/03 | Исправления stale result/missing output argument/font failure сохраняются в актуальных source. Screenshots не заменяют ручные UI scenarios. |

Новое критическое препятствие для clean per-user preview по этому статическому проходу не обнаружено. Остаются два medium exception paths, переданы root для коротких исправлений:

1. APPDATA assignment расположен в конце finally после Copy-Item/Invoke-NSIS/Remove-Item. Любая ошибка cleanup, включая uninstall timeout, пропустит восстановление environment. Требуется внутренний try/finally с безусловным restore.
2. В NSIS error branch после четырёх File copies нет cleanup этих уже записанных known files. WriteUninstaller/registration ещё не существуют; installation test не может вызвать registered uninstall, а повторный install запрещён existing-files guard. Безопасный cleanup четырёх payload filenames возможен, потому что их первоначальное отсутствие проверено до copy.

Root сообщил о 123 tests/clippy PASS до последующего изменения aged dependency pins; итоговая verification после pins в момент этого review ещё выполнялась. Это сообщение не является заявлением passed installer/e2e или итогового release gate.

## Independent read-only review нового PNG export

Прочитаны png.rs, native PNG routing, app command handling, shared render/Page и Deck::page/export_warnings. Raster использует собственный CPU triangle renderer поверх egui tessellation; новых зависимостей нет. Root выполняет следующие compilation/tests, этот агент не конкурировал за target во время LTO и не запускал raster самостоятельно.

Статически подтверждено: finite output dimensions/scale, сторона≤8192 и≤32MP output; bounded input text/items/image bytes, declared image dimensions checked; texture pixel products checked до indexing; triangle chunks_exact(3) и fixed-size arrays гарантируют постоянные indices0–2; dynamic mesh vertex/texture access через get; patch bounds проверяются до row/col арифметики. Учитываются clipping, winding/top-left shared edge и общий candidate work≤200M. Это математические/code bounds, не property-fuzz proof.

Premultiplied color×texture и source-over over opaque background соответствуют закреплённому egui_glow 0.36.2: normal sRGB-unaware textures, gamma-space shader multiplication и ONE/ONE_MINUS_SRC_ALPHA. CPU sampler linear/clamp соответствует используемым render textures. Отсутствие обработки textures_delta.free не создаёт межэкспортный cache leak: context/map созданы для одного кадра и затем уничтожаются; deltas.set применяются до tessellation.

Переданы root findings:

| ID | Уровень | Сценарий | Действие |
|---|---|---|---|
| PNG-R2-01 | high/text loss | Нет проверки системного glyph availability. Arial/Segoe не содержит всех CJK; egui нарисует missing/replacement glyph, PNG при этом успешно сохраняется. | Проверять used characters по font/style до run_ui и возвращать explicit unsupported-glyph error либо обеспечить реальный fallback; текущий PDF уже делает явный отказ. |
| PNG-R2-02 | high/allocation | MAX_TEXTURE_PIXELS проверяется только после run_ui. Внутри render::paint все images уже decode/load_texture. Aggregate bytes≤64MB и per-image≤32MP не ограничивают aggregate pixels: много разных highly compressed images способны выделить гигабайты до guard. | Aggregate dimension pixel budget в preflight до любого decode/context paint. Font atlas budget нужно учитывать отдельно. |
| PNG-R2-03 | medium/visual loss | Deck::page не сохраняет rotation; редактор вращает text/rect/ellipse. Export warnings говорят только о PDF, поэтому PNG получается без поворота без конкретной подписи. | PNG-specific warning/reject либо Page transforms/rotation support. |
| PNG-R2-04 | medium/selection clarity | Native PNG берёт deck.slides.first() при2×, независимо от активного слайда; UI пока generic PNG. | Явно назвать «PNG первого слайда» или предоставить выбор active/all; не обещать полный deck export этим маршрутом. |
| PNG-R2-05 | medium/resource timing | Candidate work ограничивается после полной tessellation. Public Page до200k complex primitives может выделить большие shapes/meshes до worker-work rejection. Native first slide ограничен10k elements, но ellipse outline добавляет48 primitives на объект. | Budget сложности/estimated vertices до tessellation либо более строгий item cap; measured memory tests и adversarial cases. |

Фактические PNG glyph/shape/alpha unit tests добавлены владельцем Slides, их результаты указывает root. Независимое сравнение с GPU screenshot/rotation/CJK/patch-delta/extreme scale и corpus raster fuzz не выполнены этим агентом.

Дополнительно перечитаны последние medium installer исправления: APPDATA теперь возвращается внутренним finally даже при cleanup failure; NSIS File-copy error branch очищает четыре known payload файла и делает nonrecursive RMDir. Обе ранее открытые exception-path находки закрыты по исходникам. Installer/e2e фактические результаты остаются отдельным gate.

## Повторное подтверждение PNG fixes

Перечитаны актуальные png.rs, меню app.rs и Deck::export_warnings. Оба high findings устранены до создания context/рисования:

- PNG-R2-01 закрыт: used non-whitespace/non-control characters собираются по regular/bold/italic style; системный TTF проверяется glyph_index, missing glyph даёт конкретную U+... error. CJK fallback как функция остаётся отсутствующим, но unsupported text больше не выдаётся за успешный PNG.
- PNG-R2-02 закрыт: aggregate decoded dimensions≤32MP проверяются в preflight до context. Safe decode дополнительно проверяет фактическую целостность каждого изображения; временный decode result освобождается перед следующим объектом.
- PNG-R2-03 закрыт как silent-loss finding: предупреждение явно говорит о потере rotation в PDF и PNG. Поддержка rotations при raster export остаётся незавершённой функцией и не объявляется реализованной.
- PNG-R2-04 закрыт: меню явно названо «PNG · первый слайд»; native маршрут по-прежнему экспортирует первый слайд в2×, это обозначенный объём команды.
- PNG-R2-05 закрыт по консервативному source guard: geometry estimate≤1M до context/tessellation; text×16, ellipse diameter×scale×16, прочие primitives32. Это bounded preflight, а не measured peak-memory proof.

TexturesDelta API обработан как texture ID → последовательность full/patch deltas; применяется каждый delta в порядке до tessellation. Triangles извлекаются as_chunks::<3>(), remainder запрещён прежней length%3 проверкой, dynamic vertex access guarded. В этой повторной проверке новых critical/blocker findings не обнаружено.

Root сообщил о 126 Rust tests PASS (112 app +14 shell) после PNG fixes. Агент не запускал собственную сборку и не заменяет этим сообщением фактический installer/UI smoke gate. Independent pixel comparison, CJK fallback, rotations support, all-slide PNG routing и измерения memory/performance остаются ограничениями. Все исходники в этом task только читались; изменён данный report под claim.
