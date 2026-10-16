# Трассировка требований

ID закреплены в HTML-комментариях источников. IDs не перенумеровывать; изменённую формулировку и маршруты обновлять явно. Подпункты интерфейсных функций дополнительно перечислены с PAR-ID в parity. Проверки — запланированные методы из testing/acceptance.md, не заявления об их прохождении.

Всего пунктов: **442**. Глобальный DoD выполнен: **0**.

| ID | Источник / раздел | Требование | Фазы | Проверки (план) | Статус |
|---|---|---|---|---|---|
| REQ-00-001 | AGENTS.md §0 | Офисный пакет TrineDocs: Docs, Sheets, Slides. Текущие целевые ОС — Windows 10/11 x64; macOS отложена владельцем (ADR-0025). | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-002 | AGENTS.md §0 | Функциональность уровня Word / Excel / PowerPoint, свои фичи сверху, real-time совместная работа и встроенный AI-агент через MCP, который редактирует документ вживую с видимым курсором. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-003 | AGENTS.md §0 | Этот файл, docs/MCP.md и docs/SPEC-AUDIT.md задают требования. План владельца от 2026-10-13 в docs/plan/codex-plan.md и ADR-0025 заменяет старые решения о Qt, CRDT, платформе и порядке реализации. Полный паритет остаётся целью; частичная реализация не считается приёмкой. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-004 | AGENTS.md §0 | **Правило полноты:** если функция есть в Word / Excel / PowerPoint или очевидно ожидается пользователем офисного пакета, но не упомянута здесь, — она всё равно в объёме проекта. Внеси её в матрицу паритета (`docs/parity.md`) с приоритетом и реализуй по плану. «Не написано» ≠ «не нужно». | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-005 | AGENTS.md §0 | Владелец разрешил параллельную реализацию Docs/Sheets/Slides и оболочки 2–4 агентами. Приёмочные критерии проверять отдельно; перед сборкой установщика обязательны два независимых ревью. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-006 | AGENTS.md §0 | 2. Перед фазой — план в `docs/plan/phase-N.md`; после — отчёт: что сделано, что не работает, известные баги, решения. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-007 | AGENTS.md §0 | 3. Сначала тонкий вертикальный срез, потом расширение. **Заглушки не выдавать за готовые функции**: недоделанная команда в UI помечается и отключается. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-008 | AGENTS.md §0 | 4. Конфликт требования с реальностью (производительность, лицензия, возможности библиотеки) — остановись и опиши варианты в `docs/decisions/NNNN-*.md` (ADR). Не обходи молча. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-009 | AGENTS.md §0 | 5. Стек не меняется без ADR. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-00-010 | AGENTS.md §0 | 6. Каждую зависимость и её лицензию — в `docs/licenses.md`. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-01-001 | AGENTS.md §1 | Платформы: Windows 10 и Windows 11 x64. macOS, совместная работа через сервер и Google Drive отложены по плану владельца (ADR-0025). | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-01-002 | AGENTS.md §1 | Один Rust workspace, общий слой операций и форматов для трёх редакторов. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-01-003 | AGENTS.md §1 | В приложении запрещены Electron, Tauri, WebView и JavaScript/TypeScript. GUI рисуется нативным egui. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-01-004 | AGENTS.md §1 | UI: egui/eframe =0.36.2 (glow), Rust =1.98.1, edition 2024. Qt/QML/CXX-Qt заменены решением владельца ADR-0025. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-01-005 | AGENTS.md §1 | Прямые зависимости пинятся =x.y.z, транзитивные — Cargo.lock; релизы младше 14 дней исключаются. Реестр версий, лицензий и непроверенных пунктов — docs/licenses.md. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-01-006 | AGENTS.md §1 | Никакого AGPL, unsafe в коде проекта или unwrap/expect в рабочем коде. Системные шрифты не входят в установщик. Лицензии зависимостей и notices поставляются отдельно. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-001 | AGENTS.md §2 | \| UI \| egui/eframe =0.36.2, glow \| нативное окно и три редактора \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-002 | AGENTS.md §2 | \| Модель/операции \| Rust, serde JSON, ops \| UI владеет моделью, undo по авторам \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-003 | AGENTS.md §2 | \| История \| авторские undo-группы и recovery snapshots \| CRDT/сетевая совместная работа отложены \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-004 | AGENTS.md §2 | \| Вёрстка текста \| свои страницы/строки, egui + системные TTF метрики \| точность и Unicode отдельно проверяются \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-005 | AGENTS.md §2 | \| Рендер \| общий Page/Primitive, egui, pdf-writer/ttf-parser/subsetter \| экран/PDF, ограничения фиксируются \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-006 | AGENTS.md §2 | \| Формулы \| собственные lexer/parser/evaluator \| parity Microsoft и regression corpus \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-007 | AGENTS.md §2 | \| OOXML \| quick-xml + zip, calamine, rust_xlsxwriter \| собственные DOCX/PPTX, XLSX \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-008 | AGENTS.md §2 | \| Legacy \| XLS/XLSB/ODS — calamine при безопасных лимитах; DOC/PPT отложены \| неподдержанное честно отклоняется \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-009 | AGENTS.md §2 | \| MCP \| rmcp =3.5.0 \| официальный Rust SDK, Streamable HTTP и stdio-шим \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-010 | AGENTS.md §2 | \| Async \| tokio =1.53.1 + bounded mpsc/oneshot \| фоновые задачи, UI без ожидания сети \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-011 | AGENTS.md §2 | \| Настройки/файлы \| JSON + zip(JSON/assets) \| APPDATA, recent, атомарная запись с .bak \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-012 | AGENTS.md §2 | \| Сервер совместной работы \| отложен владельцем \| не объявлять работающим \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-013 | AGENTS.md §2 | \| Логи \| локальные файлы в APPDATA/TrineDocs/logs \| без токенов и текста документов \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-014 | AGENTS.md §2 | \| Дистрибуция \| NSIS LZMA solid, Windows x64 \| текущий пользователь без UAC; подпись пока отсутствует \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-02-015 | AGENTS.md §2 | \| Иконки/шрифты \| свои кодовые иконки, системные Windows шрифты \| встроенные Inter/JetBrains не используются приложением \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-03-001 | AGENTS.md §3 | Внутри TrineDocs.app / TrineDocs.exe: | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-002 | AGENTS.md §3 | Rust egui UI: Главная, вкладки Docs/Sheets/Slides, панели и палитра команд. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-003 | AGENTS.md §3 | Собственные canvas editors и egui input/IME; доступность пользовательского canvas проверяется отдельно. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-004 | AGENTS.md §3 | Общий crate trinedocs; C++/QML-моста нет (ADR-0025). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-005 | AGENTS.md §3 | - Rust core (cargo workspace): | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-006 | AGENTS.md §3 | - `model` — модель документа поверх CRDT; `ops` — все команды правки. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-007 | AGENTS.md §3 | - `layout` — вёрстка страниц/ячеек/слайдов; `render` — display list + бэкенды. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-008 | AGENTS.md §3 | - `anim` — анимации и переходы слайдов; `calc` — CalcEngine (IronCalc). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-009 | AGENTS.md §3 | - `ooxml` — docx/xlsx/pptx; `legacy` — LibreOffice-конвертер. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-010 | AGENTS.md §3 | - `export` — PDF/CSV/MD/HTML/PNG/…; `print` — пагинация печати. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-011 | AGENTS.md §3 | - `cloud` — Google Drive/Docs; `sync` — клиент синхронизации. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-012 | AGENTS.md §3 | - `mcp` — MCP-сервер, агент, IPC-шим; `library` — недавние, превью, FTS. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-013 | AGENTS.md §3 | - `storage` — SQLite, автосейв, восстановление; `settings` — настройки. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-014 | AGENTS.md §3 | - Локальный IPC (named pipe / unix socket) → `trinedocs-mcp` (stdio-шим). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-015 | AGENTS.md §3 | - WebSocket (TLS) → Sync Server (Rust). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-016 | AGENTS.md §3 | Единый путь изменения содержимого: UI и MCP → ops. UI-команда готовит копию модели; commit создаёт undo-группу с автором. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-017 | AGENTS.md §3 | Источник истины — типизированная модель в UI-потоке. CRDT/сеть отложены; их старые требования сохраняются как будущий объём, не как выполненная возможность. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-018 | AGENTS.md §3 | - **Стабильные ID** у каждого абзаца, блока, ячейки, листа, слайда, фигуры. Используются в UI, MCP, комментариях, ссылках. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-019 | AGENTS.md §3 | UI применяет model ops в logic перед отрисовкой; файловые операции/MCP/снимки выполняются в отдельных потоках. Производительность измеряется отдельно. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-020 | AGENTS.md §3 | Общий Page/Primitive → egui/PDF; неподдержанные повороты/шейпинг/сложная вёрстка в отчёте. Печать через PDF-просмотрщик с честной подписью. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-021 | AGENTS.md §3 | - **Внутренняя модель ≠ OOXML.** Неподдержанное хранится как непрозрачные фрагменты с привязкой к месту и пишется обратно. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-022 | AGENTS.md §3 | Undo/redo и AI review по авторам через ops; выборочный откат с последующими правками требует сохранения данных и конфликт-проверки. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-03-023 | AGENTS.md §3 | - Импорт OOXML и LibreOffice выполняется в отдельном дочернем процессе с лимитами памяти/времени, отменой и корректным завершением (SPEC-AUDIT §3.C.5; ADR-0006). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-001 | AGENTS.md §4 | \| Клавиша → пиксели \| ≤ 16 мс p95 (100 стр., 100k ячеек, 100 слайдов) \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-002 | AGENTS.md §4 | \| Анимации UI и скролл \| 60 fps; 120 fps на ProMotion \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-003 | AGENTS.md §4 | \| Холодный старт до Главной \| ≤ 1,5 с (M1 / типичный ПК с Win11) \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-004 | AGENTS.md §4 | \| Открытие docx 100 стр. \| первая страница ≤ 1 с, остальное в фоне \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-005 | AGENTS.md §4 | \| Удалённая правка \| RTT + ≤ 20 мс на сервере \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-006 | AGENTS.md §4 | \| Пересчёт таблицы \| изменение ячейки → зависимые ≤ 50 мс для 100k формул (многопоточно, инкрементально) \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-007 | AGENTS.md §4 | \| Память \| документ 100 стр. ≤ 300 МБ \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-008 | AGENTS.md §4 | - UI-поток никогда не ждёт сеть, диск, AI, LibreOffice. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-009 | AGENTS.md §4 | - Офлайн — полная работа; слияние без конфликтов при подключении. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-010 | AGENTS.md §4 | - Никаких потерь данных: автосохранение, восстановление после падения. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-04-011 | AGENTS.md §4 | - Цели не считаются доказанными до измерений по `docs/testing/acceptance.md`. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-05-001 | AGENTS.md §5 | - Контент в центре, интерфейс отступает. Сетка 4 px, радиусы 6/10 px, тонкие разделители вместо рамок, минимум теней. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-002 | AGENTS.md §5 | - По образцу владельца: светлая оболочка, вкладки команд и сгруппированная лента Office, белые страницы в центре; палитра команд сохраняется. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-003 | AGENTS.md §5 | UI использует системные шрифты Windows, загружаемые во время выполнения; встроенные шрифты не входят в exe (ADR-0025). | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-004 | AGENTS.md §5 | - Lucide: сетка 24, stroke 1.75, скругления. Свои иконки — в том же стиле. Цвет только через `currentColor` + токены. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-005 | AGENTS.md §5 | - Docs — синий, Sheets — зелёный, Slides — оранжевый. Точечные акценты. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-006 | AGENTS.md §5 | - Светлая (по умолчанию), тёмная, системная. Системная тема — egui/eframe. Переключение на лету, кросс-фейд 150–200 мс. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI, T-00-POLICY | partial |
| REQ-05-007 | AGENTS.md §5 | - Цвета — только токены singleton `Theme.qml`: background, surface, surface-raised, border, text-primary/secondary/disabled, accent, accent-docs/sheets/slides, danger, warning, success, selection, ai-cursor, remote-cursor-1…8. Хардкод цветов в других QML запрещён (CI). | 0 | T-00-SPEC, T-00-BUILD, T-00-UI, T-00-POLICY | partial |
| REQ-05-008 | AGENTS.md §5 | - «Тёмная бумага» — только отображение; печать/экспорт как в документе. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-009 | AGENTS.md §5 | - Контраст ≥ WCAG AA; высокий контраст Windows. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI, T-00-POLICY | partial |
| REQ-05-010 | AGENTS.md §5 | - Windows 11: Mica (`DwmSetWindowAttribute`, `DWMWA_SYSTEMBACKDROP_TYPE`), своя строка заголовка с Snap Layouts (hit-test кнопки «развернуть»), системные скругления, jump list «Недавние». | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-011 | AGENTS.md §5 | - macOS: единая строка заголовка со «светофором», vibrancy сайдбара, системное меню, pinch-zoom, инерционный скролл, полноэкранный режим, вкладки окна. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-012 | AGENTS.md §5 | - Ctrl ↔ Cmd, раскладки как в Office: B/I/U, S; F2 — ячейка; F5 — показ. Полный список — `docs/shortcuts.md`. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-013 | AGENTS.md §5 | - Микроанимации 120–220 мс, out-cubic: панели, меню, вкладки, элементы. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-014 | AGENTS.md §5 | - Плавные курсоры пользователя, участников и AI. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-05-015 | AGENTS.md §5 | - Уважать системное «уменьшить движение». | 0 | T-00-SPEC, T-00-BUILD, T-00-UI, T-00-POLICY | partial |
| REQ-06-001 | AGENTS.md §6 | - Создать Документ / Таблицу / Презентацию + шаблоны. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-002 | AGENTS.md §6 | - Недавние: сетка/список; превью (страница/слайд/лист), имя, режим, путь/облако, время, последний автор. Открыть, открыть расположение, закрепить, удалить из списка, переименовать. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-003 | AGENTS.md §6 | - Закреплённые, Общие со мной, Google Drive (если подключён). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-004 | AGENTS.md §6 | - Поиск названий/содержимого (SQLite FTS5), фильтры типа/даты. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-005 | AGENTS.md §6 | - Drag-and-drop файлов, ассоциации двойного клика Проводник/Finder. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-006 | AGENTS.md §6 | - Превью в фоне, кэш. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-007 | AGENTS.md §6 | - Несколько окон, смешанные вкладки Docs/Sheets/Slides; перетаскивание вкладки в новое окно. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-008 | AGENTS.md §6 | - Переключатель документов, split-view: один документ в двух местах / два документа. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-06-009 | AGENTS.md §6 | - Восстановление сессии (опция). | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-07-001 | AGENTS.md §7 | Создать · Создать из шаблона · Открыть… · Открыть недавние ▸ · Закрыть · Сохранить · Сохранить как… · Сохранить копию… · Экспорт ▸ · Печать… · Предпросмотр печати · Параметры страницы… · Поделиться… · История версий · Восстановить… · Свойства документа · Защита ▸ · Отправить в Google Drive ▸ · Выход. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-002 | AGENTS.md §7 | - Нативный формат (`.trinedoc`, `.trinesheet`, `.trineslide` — zip: Loro-снапшот, ассеты, превью, метаданные). Полная история, комментарии, AI-изменения. ⚠ Финальные расширения: ADR-0009, ждёт владельца. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-003 | AGENTS.md §7 | - Office-файлы сохраняются напрямую в исходном формате (.docx → Ctrl/Cmd+S пишет .docx). История и CRDT — sidecar-кэш приложения. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-004 | AGENTS.md §7 | - Облачные документы: автоматическое сохранение, вместо кнопки статус «Сохранено / Синхронизация / Офлайн». | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-005 | AGENTS.md §7 | - Нативные QFileDialog. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-006 | AGENTS.md §7 | - «Сохранить как»: формат, имя, папка; для потерь — предупреждение с конкретным списком (Morph → исчезание; связи → статичные), продолжить/отмена. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-007 | AGENTS.md §7 | - «Сохранить копию» не переключает текущий файл. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-008 | AGENTS.md §7 | - Атомарно: соседний временный файл → fsync → атомарная замена. Исходник не повреждается. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-009 | AGENTS.md §7 | - Бэкап предыдущей версии: опция, по умолчанию вкл. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-010 | AGENTS.md §7 | - Watcher: внешняя правка → Перезагрузить / Сравнить / Оставить. Сравнение и слияние — трёхсторонние: моя версия / версия на диске / общая база (SPEC-AUDIT §3.C.6). | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-011 | AGENTS.md §7 | - Office lock `~$имя`; занят → Только чтение / Копия. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-012 | AGENTS.md §7 | - Длинные пути Windows; плейсхолдеры iCloud/OneDrive/Dropbox. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-013 | AGENTS.md §7 | - Каждая правка → SQLite CRDT-журнал фоново, батчи ≤ 1 с. Падение теряет ≤ 1 с. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-014 | AGENTS.md §7 | - После сбоя — список восстановления с превью. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-015 | AGENTS.md §7 | - Office-автосейв: переключатель в шапке. Выкл → журнал сохраняется, файл только по «Сохранить». | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-016 | AGENTS.md §7 | - Шкала CRDT: именованные/автоматические точки, авторы и AI. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-017 | AGENTS.md §7 | - Просмотр версии, дифф, восстановление, версия как копия. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-018 | AGENTS.md §7 | - Название, автор, теги, даты, статистика слов/символов/страниц/формул. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-019 | AGENTS.md §7 | - OOXML Agile Encryption чтение/запись — P1; пароль нативного формата. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-020 | AGENTS.md §7 | - Только чтение, защита листа/книги, «окончательный». | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-07-021 | AGENTS.md §7 | - Инспектор личных данных: автор, комментарии, скрытый текст, история. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-001 | AGENTS.md §8 | \| .docx, .dotx \| ✓ \| ✓ \| ooxml \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-002 | AGENTS.md §8 | \| .docm \| ✓, макросы не исполняются, сохраняются \| ✓ \| ooxml \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-003 | AGENTS.md §8 | \| .doc, .rtf, .odt \| ✓ \| ✓ \| LibreOffice ↔ OOXML \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-004 | AGENTS.md §8 | \| .pdf \| P2: картинки/текст \| ✓; PDF/A — P1 \| Skia PDF \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-005 | AGENTS.md §8 | \| .txt \| ✓ \| ✓ \| свой \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-006 | AGENTS.md §8 | \| .md \| ✓ \| ✓ \| CommonMark + GFM-таблицы \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-007 | AGENTS.md §8 | \| .html \| ✓ \| ✓ \| свой \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-008 | AGENTS.md §8 | \| .epub \| — \| P2 \| свой \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-009 | AGENTS.md §8 | \| Google Docs \| ✓ \| ✓ \| Drive API \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-010 | AGENTS.md §8 | \| .xlsx, .xltx \| ✓ \| ✓ \| ooxml; IronCalc reader/writer, если покрывает \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-011 | AGENTS.md §8 | \| .xlsm \| ✓, VBA не исполняется, сохраняется \| ✓ \| ooxml \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-012 | AGENTS.md §8 | \| .xlsb \| P2 \| P2 \| LibreOffice \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-013 | AGENTS.md §8 | \| .xls, .ods \| ✓ \| ✓ \| LibreOffice \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-014 | AGENTS.md §8 | \| .csv, .tsv \| ✓: мастер UTF-8/UTF-16/Windows-1251 и др.; разделитель, кавычки, типы, локаль \| ✓: лист или все листы в zip \| свой \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-015 | AGENTS.md §8 | \| .pdf \| — \| ✓: область, масштаб \| Skia PDF \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-016 | AGENTS.md §8 | \| .html, .md \| — \| ✓: таблица \| свой \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-017 | AGENTS.md §8 | \| Google Sheets \| ✓ \| ✓ \| Drive API \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-08-018 | AGENTS.md §8 | \| .pptx, .potx, .ppsx \| ✓ \| ✓ \| ooxml \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-019 | AGENTS.md §8 | \| .pptm \| ✓, макросы не исполняются, сохраняются \| ✓ \| ooxml \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-020 | AGENTS.md §8 | \| .ppt, .odp \| ✓ \| ✓ \| LibreOffice \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-021 | AGENTS.md §8 | \| .pdf \| — \| ✓: слайды/заметки/раздаточные \| Skia PDF \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-022 | AGENTS.md §8 | \| .png, .jpg, .svg \| — \| ✓: слайд/все, разрешение \| Skia \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-023 | AGENTS.md §8 | \| .mp4 \| — \| P2: анимации, ffmpeg отдельным процессом, проверить лицензию сборки \| — \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-024 | AGENTS.md §8 | \| .gif \| — \| P2 \| — \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-025 | AGENTS.md §8 | \| Google Slides \| ✓ \| ✓ \| Drive API \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-026 | AGENTS.md §8 | - Экспорт: формат, область (всё/выделение/лист/слайды), параметры, превью. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-027 | AGENTS.md §8 | - Потери — предупреждение как «Сохранить как». | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-028 | AGENTS.md §8 | - LibreOffice не входит в установщик; докачивается при первой нужной операции; отдельный процесс, таймаут, изолированный временный каталог, отмена. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-08-029 | AGENTS.md §8 | - Безопасный импорт: лимиты zip-бомб (размер/коэффициент), XXE отключён, внешние xlsx-ссылки не обновляются, OLE/макросы не исполняются, ссылки только после подтверждения. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-09-001 | AGENTS.md §9 | - OAuth 2.0 desktop: loopback + PKCE; scope `drive.file` (только открытое/созданное через приложение). Верификация scope — проверить, ADR. | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-09-002 | AGENTS.md §9 | - Токены в Windows Credential Manager / macOS Keychain. | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-09-003 | AGENTS.md §9 | - Экспорт: docx/xlsx/pptx → Drive API конвертация или Office-файл; ссылка «Открыть в Google». | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-09-004 | AGENTS.md §9 | - Импорт: Google Picker или доступные файлы → Drive API export → обычный импорт. Проверить лимиты размера, понятная ошибка при превышении. | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-09-005 | AGENTS.md §9 | - Повторная отправка обновляет тот же fileId; связь хранится локально. | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-09-006 | AGENTS.md §9 | - Одноразовый обмен; живая синхронизация только своим сервером. | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-09-007 | AGENTS.md §9 | - OneDrive/Dropbox — P2. | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-10-001 | AGENTS.md §10 | - Свой предпросмотр + системный диалог через Qt Print Support; QPainter display list (вектор, текст остаётся текстом). | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-10-002 | AGENTS.md §10 | - Постраничный предпросмотр, масштаб, листание, точное совпадение. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-10-003 | AGENTS.md §10 | - Принтер, копии, разбор; диапазон все/текущая/выделение/«1-3, 5»; чёт/нечёт; duplex длинный/короткий край; ориентация; A4/Letter/A3/Legal/свой; поля; масштаб вписать/%; 1/2/4/6/9/16 страниц на листе; цвет/серый; качество. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-10-004 | AGENTS.md §10 | - Собственная «Печать в PDF» всегда доступна. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-10-005 | AGENTS.md §10 | - Подготовка в фоне, прогресс, отмена. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-10-006 | AGENTS.md §10 | Колонтитулы, номера, разрывы; скрытый текст; комментарии/правки как в документе / на полях / без; фон; обновление полей. | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-10-007 | AGENTS.md §10 | Область печати; повтор строк/столбцов; вписать в N по ширине/высоте; сетка, заголовки; порядок вниз-вправо / вправо-вниз; разметка с drag разрывов; выделение/лист/книга; колонтитулы. | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-10-008 | AGENTS.md §10 | Слайды 1 на страницу; страницы заметок; раздаточные 1/2/3/4/6/9 (при 3 линии для заметок); структура; рамка; скрытые; последний кадр анимаций. | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-11-001 | AGENTS.md §11 | `docs/parity.md`: функция → P0 MVP / P1 v1 / P2 позже → статус → импорт/экспорт. Реализация по приоритетам. ⚠ Границы v1 и перестановка фаз: ADR-0005, ждёт владельца; ничего не удалено из конечного объёма. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-11-002 | AGENTS.md §11 | P0: буфер обмена (внутренний/HTML/RTF/текст/изображения; Word/Excel/браузер), без форматирования, специальная вставка (Sheets значения/формулы/форматы/транспонировать); поиск/замена (регистр, целые слова, regex, формат); масштаб; линейки; сетка/направляющие; системная орфография ru/en (Windows Spell Checking API/NSSpellChecker); автозамена; спецсимволы/эмодзи; ссылки; комментарии с ответами/@; undo/redo с историей. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-11-003 | AGENTS.md §11 | P1: опциональная AI-грамматика, перевод выделения, OCR (Windows.Media.Ocr / Vision), системный TTS. | 8 | T-08-PARITY | planned |
| REQ-11-004 | AGENTS.md §11 | P0: шрифты, размер, жирный/курсив/подчёркнутый/зачёркнутый, над/подстрочный, регистр, текст/выделение, очистка, формат по образцу; стили (заголовки 1–6, обычный, цитата, код, свои, наборы); выравнивание, отступы, межстрочный, до/после, табуляции, висячие строки, «не отрывать»; списки маркированные/нумерованные/многоуровневые/перезапуск; таблицы (вставка, merge/split, границы, заливка, ширина, повтор заголовка, сортировка, формулы); изображения (масштаб, поворот, crop, обтекание в тексте/рамка/контур/за/перед, привязка); фигуры/надписи; разрывы страниц/разделов; поля, ориентация, размер; колонтитулы первая/чёт/нечёт; номера; колонки; разметка/веб/чтение/фокус; навигация заголовков; статистика. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-11-005 | AGENTS.md §11 | P1: обновляемое оглавление; сноски/концевые; поля дата/страница/количество/автор; закладки/перекрёстные ссылки; track changes принять/отклонить; сравнение; OMML редактор/рендер/LaTeX; SmartArt рендер/базовая правка; водяные знаки; буквица; подложка/рамки страницы; авто/ручные переносы; языки; библиография/цитаты; подписи; список иллюстраций. | 8 | T-08-PARITY | planned |
| REQ-11-006 | AGENTS.md §11 | P2: mail merge CSV/Sheets; предметный указатель; формы/content controls; главный документ; водяные знаки-изображения. | 9 | T-09-RELEASE, T-SECURITY | planned |
| REQ-11-007 | AGENTS.md §11 | P0: число/текст/дата-время/bool/ошибка; формулы Excel (математика, статистика, текст, даты, логика, XLOOKUP/VLOOKUP/HLOOKUP/INDEX/MATCH/XMATCH, базовые финансовые); абсолютные/относительные/межлистовые ссылки; completion/аргументы/зависимые диапазоны; числовые форматы общий/число/валюта/финансовый/%/дата/время/дробь/экспонента/текст/коды Excel; шрифты/цвета/границы/заливка/выравнивание/перенос/поворот/merge; размеры/autofit; вставка/удаление/скрытие строк/столбцов; freeze; автозаполнение рядов/дат/маркер; многоуровневая сортировка; автофильтр; листы add/rename/move/hide/tab-color; диаграммы column/bar/line/area/pie/doughnut/scatter; статус сумма/среднее/количество; локаль чисел/дат, десятичная запятая ru-RU. | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-11-008 | AGENTS.md §11 | P1: динамические массивы LET/LAMBDA/FILTER/SORT/UNIQUE/SEQUENCE; условное форматирование правила/гистограммы/шкалы/иконки; validation списки/диапазоны/сообщения; имена/диспетчер; сводные/диаграммы; срезы; спарклайны; таблицы/структурированные ссылки; группировка/структура; промежуточные итоги; дубликаты; текст по столбцам; flash fill; защита листа/книги; комбинированные/вторая ось/trendlines; комментарии/заметки; итерации; ручной пересчёт; трассировка; watch window; локализованные функции СУММ ↔ SUM (хранение canonical English). | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-11-009 | AGENTS.md §11 | P2: Power Query-подобный импорт; what-if goal seek/scenarios/data tables; solver; внешние книги; 3D-ссылки. VBA не поддерживается: непрозрачный round-trip с предупреждением. Неподдержанная функция → `#NAME?` с подсказкой, список в parity. | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-11-010 | AGENTS.md §11 | P0: слайды add/duplicate/delete/hide/move/sections; layouts/master; темы colors/fonts/background; text autofit; базовые фигуры/линии/connectors; crop/shape-mask; таблицы; диаграммы; smart guides; align/distribute; group; layers; rotate/flip; notes; sorter; F5/Shift+F5; второй монитор presenter next/notes/timer/clock; pointer/ink. | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-11-011 | AGENTS.md §11 | P1: анимации entrance/emphasis/exit/motion/custom; click/with/after/object triggers; delay/duration/repeat/paragraph; timeline; переходы/Morph-аналог matched-object interpolation; pptx `p:timing`/transitions round-trip; видео/аудио Qt Multimedia + trim; SmartArt; Zoom к слайду/разделу; custom shows; rehearsal. | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-11-012 | AGENTS.md §11 | P2: recording voice; video export; live captions. | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-11-013 | AGENTS.md §11 | - Ctrl/Cmd+K: команда, файл, функция, настройка. | 8 | T-08-PARITY | planned |
| REQ-11-014 | AGENTS.md §11 | - Slash `/`, Markdown `#`, `-`, `1.`, `**`, `>`. | 8 | T-08-PARITY | planned |
| REQ-11-015 | AGENTS.md §11 | - Живой диапазон/диаграмма Sheets → Docs/Slides; Office-экспорт статичный с предупреждением. | 8 | T-08-PARITY | planned |
| REQ-11-016 | AGENTS.md §11 | - Машина времени: шкала, сравнение, восстановление. | 8 | T-08-PARITY | planned |
| REQ-11-017 | AGENTS.md §11 | - Фокус/чистое чтение. | 8 | T-08-PARITY | planned |
| REQ-11-018 | AGENTS.md §11 | - AI-чат с курсором. | 8 | T-08-PARITY | planned |
| REQ-11-019 | AGENTS.md §11 | - AI-шаблоны с заполняемыми полями. | 8 | T-08-PARITY | planned |
| REQ-12-001 | AGENTS.md §12 | - Шапка «Чат», Ctrl/Cmd+J; боковая панель не перекрывает документ, detach. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-002 | AGENTS.md §12 | - Текущий документ/выделение; другие открытые только по запросу; видимые чипы переданного контекста. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-003 | AGENTS.md §12 | - Агент — участник CRDT; `ai-cursor`, метка AI, плавное движение. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-004 | AGENTS.md §12 | - Потоковые tool calls, батчи ~30 мс, мягкая затухающая подсветка. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-005 | AGENTS.md §12 | - Семантические модельные ops без DOM/скриншотов; задержка генерации зависит от модели, локальная обработка отдельно измеряется. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-006 | AGENTS.md §12 | - Пользователь печатает рядом одновременно. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-007 | AGENTS.md §12 | - Шаг — undo group; «Изменения AI»: всё/по одному принять/отклонить; suggest/сразу. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-008 | AGENTS.md §12 | - «Стоп» прерывает генерацию, откатывает незавершённый шаг. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-009 | AGENTS.md §12 | - Trait `LlmProvider`: OpenAI-compatible, Anthropic, Ollama/LM Studio compatible endpoints; ключи в OS keychain; выбор модели settings/панель. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-010 | AGENTS.md §12 | - Встроенный агент через тот же MCP in-process, одна реализация. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-011 | AGENTS.md §12 | - Первое явное уведомление об отправке контента провайдеру; AI off на документ/приложение. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-12-012 | AGENTS.md §12 | - По умолчанию только ключ пользователя; собственный оплачиваемый прокси требует ADR и владельца (SPEC-AUDIT §3.C.11). | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-13-001 | AGENTS.md §13 | Полная спецификация `docs/MCP.md`. Встроенный сервер автозапускается; stdio-шим `trinedocs-mcp`; loopback Streamable HTTP; регистрация клиентов в один клик с согласием; CRDT-участник с курсором. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-14-001 | AGENTS.md §14 | - Курсоры/выделения, имена/цвета, аватары, follow, typing. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-14-002 | AGENTS.md §14 | - Owner/editor/commenter/viewer; email/link invitation, роль, срок. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-14-003 | AGENTS.md §14 | - Threads, @, in-app/system notifications. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-14-004 | AGENTS.md §14 | - Офлайн-слияние, sync status. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-14-005 | AGENTS.md §14 | - Сервер: document rooms, Loro relay, presence, PostgreSQL snapshots, S3 assets, JWT, rate limit, Docker, backups. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-14-006 | AGENTS.md §14 | - 50 активных участников в одном документе. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-15-001 | AGENTS.md §15 | - Без аккаунта: вся локальная работа, Office/native, BYOK AI, MCP. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-15-002 | AGENTS.md §15 | - С аккаунтом: cloud/collaboration/sharing. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-15-003 | AGENTS.md §15 | - Email magic link/password, Apple, Google; сессии в keychain. | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-16-001 | AGENTS.md §16 | Общие: язык, тема, бумага, сессия, автосейв, папка, native/Office default. Редактирование: autocorrect, spelling, languages, cm/in/pt. Вид: ribbon/compact, UI scale, animations. AI: provider/model/keys/mode/limits. MCP. Аккаунты TrineDocs/Google. Печать defaults. Privacy: telemetry/crashes только opt-in. Updates stable/beta/auto. Remappable shortcuts. Reset. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-17-001 | AGENTS.md §17 | - Системные/встроенные шрифты. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-17-002 | AGENTS.md §17 | - Метрические fallback: Calibri→Carlito; Cambria→Caladea; Arial→Liberation Sans; Times New Roman→Liberation Serif; Courier New→Liberation Mono. Баннер замены. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-17-003 | AGENTS.md §17 | - PDF subset embedding, чтение embedded docx/pptx fonts. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-17-004 | AGENTS.md §17 | - Complex scripts/bidi/RTL/CJK/vertical — P1. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-17-005 | AGENTS.md §17 | - CJK IME обязательно с фазы 1. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-17-006 | AGENTS.md §17 | - Hyphenation ru/en. | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-18-001 | AGENTS.md §18 | Русский и английский словарь оболочки с тестом полноты. Локализация редакторов и доступность оцениваются в release-review отдельно. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-18-002 | AGENTS.md §18 | - Числа/даты/валюты по системе/документу. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-18-003 | AGENTS.md §18 | - Полная клавиатура, фокус, Narrator/VoiceOver; QAccessible дерево абзацев/ячеек/фигур; UI scale; high contrast; reduced motion. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-001 | AGENTS.md §19 | - Импорт/OOXML cargo-fuzz, resource limits, без `unwrap()` в production. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-002 | AGENTS.md §19 | - Макросы/OLE/внешние ссылки/DDE не исполняются. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-003 | AGENTS.md §19 | - Секреты только OS keychain. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-004 | AGENTS.md §19 | - Opt-in telemetry/crash без документа. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-005 | AGENTS.md §19 | - Unit каждый crate; CRDT property concurrent convergence; layout/print PDF goldens; UI обе темы; ≥200 real OOXML round-trip; MCP Inspector + own client e2e; criterion CI, >10% regression red. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-006 | AGENTS.md §19 | - GitHub Actions Windows 11 x64/macOS arm64; build/tests/rustfmt/clippy/qmllint/colors/signing. macOS job отложен по ADR-0001, это не означает macOS DoD. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-19-007 | AGENTS.md §19 | - Windows Authenticode certificate/Azure Trusted Signing; MSIX (+optional EXE). Mac Developer ID/notarized DMG. Store P2, sandbox/MCP/LO ADR. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-20-001 | AGENTS.md §20 | \| 0. Скелет и дизайн-система \| workspace, CXX-Qt, QML shell, Theme, темы, иконки, шрифты, Mica/vibrancy, title/Snap, CI, явно обозначенная неподписанная dev-сборка \| темы без перезапуска на обеих ОС, зелёный application CI; сейчас Windows-first, Mac приёмка отложена \| | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-20-002 | AGENTS.md §20 | \| 1. Главная и ядро Docs \| Home/create/recent/search; Loro model/ops; layout/tiles/input/IME/selection/undo/P0; native; autosave/recovery \| 100 стр., ≤16 мс; kill теряет ≤1 с; спайки layout/CRDT обязательны \| | 1 | T-01-CRDT, T-01-LAYOUT, T-01-INPUT, T-01-RECOVERY | planned |
| REQ-20-003 | AGENTS.md §20 | \| 2. Docs файлы/экспорт/печать \| save/as/copy, atomic/watcher/lock, docx/PDF/print/preview/page setup \| 50 real docx round-trip; print=preview=PDF \| | 2 | T-02-FILES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-20-004 | AGENTS.md §20 | \| 3. MCP/AI \| full MCP/autostart/shim/registration/chat/cursor/streaming/review \| внешний агент правит live; чат переписывает при вводе рядом \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-20-005 | AGENTS.md §20 | \| 4. Collaboration \| sync/presence/sharing/roles/comments/offline \| 2 клиента + AI, offline merge, 50 participants \| | 4 | T-04-SYNC, T-SECURITY | planned |
| REQ-20-006 | AGENTS.md §20 | \| 5. Sheets \| P0, grid/formulas/charts/xlsx/csv/print \| 100k cells 60fps, real xlsx round-trip \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-20-007 | AGENTS.md §20 | \| 6. Slides \| P0, show/presenter/pptx/notes/handouts, then P1 anim/transitions \| 100 slides; anim 60fps; anim pptx round-trip; старт разрешён владельцем после приёмки предыдущих фаз (ADR-0005) \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-20-008 | AGENTS.md §20 | \| 7. Legacy/Google \| LO; Drive всех режимов \| doc/xls/ppt open/save; Google export; старт разрешён владельцем после приёмки предыдущих фаз (ADR-0005) \| | 7 | T-07-LEGACY, T-07-GOOGLE, T-SECURITY | planned |
| REQ-20-009 | AGENTS.md §20 | \| 8. P1/свои \| track changes/TOC/OMML/pivot/conditional/commands/links/time machine \| P1 закрыты или перенесены ADR; старт разрешён владельцем после приёмки предыдущих фаз (ADR-0005) \| | 8 | T-08-PARITY | planned |
| REQ-20-010 | AGENTS.md §20 | \| 9. Выпуск \| installers/notarization/updates/opt-in crashes/a11y/i18n \| clean install/update обеих ОС; старт разрешён владельцем после приёмки предыдущих фаз (ADR-0005) \| | 9 | T-09-RELEASE, T-SECURITY | planned |
| REQ-20-011 | AGENTS.md §20 | Перестановка Docs → Sheets → MCP/AI → collaboration → export/print из SPEC-AUDIT — **предложение**, не принято без владельца. Пока сохраняется базовая нумерация фаз. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-20-012 | AGENTS.md §20 | В конце каждой фазы: работает / не работает / баги / следующий шаг, плюс аудит по SPEC-AUDIT. | 0 | T-00-SPEC, T-00-BUILD, T-00-UI | planned |
| REQ-M00-001 | docs/MCP.md §0 | MCP предоставляет семантическое чтение и правку моделей через ops → UI-поток. CRDT/сервер совместной работы отложены владельцем ADR-0025. Реализованные инструменты и пробелы в docs/audit/release-review.md. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M00-002 | docs/MCP.md §0 | Crate `mcp` на **rmcp**, официальном Rust SDK. Версия протокола — актуальная стабильная при реализации; на 2026-10-12 официальная `/specification/latest` указывает **2026-07-28** (FACT-006). Утверждение «сейчас 2025-11-25» исправлено (CHANGE-004). Поддержка 2026-07-28 конкретным rmcp и клиентами ещё не подтверждена; breaking changes требуют спайка 4.6 и ADR-0010. Предыдущие версии поддерживаются в пределах возможностей закреплённого rmcp. Непроверенные возможности SDK/клиентов не объявляются рабочими. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-001 | docs/MCP.md §1 | - Сервер стартует с приложением, по умолчанию вкл.; фоновая tokio-задача не задерживает окно. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-002 | docs/MCP.md §1 | - «Работать в фоне»: Windows tray / macOS menu bar; последнее окно закрывается, MCP остаётся. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-003 | docs/MCP.md §1 | - «При входе»: Windows MSIX Startup task / current-user Run; macOS `SMAppService`; без окна. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-004 | docs/MCP.md §1 | - Если приложения нет, шим запускает `--background`, ждёт готовности до 10 с. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-005 | docs/MCP.md §1 | - Поставляется маленький Rust-бинарник; проксирует JSON-RPC в приложение, сам документы не редактирует. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-006 | docs/MCP.md §1 | Windows stdio-шим проксирует запросы на loopback Streamable HTTP, порт/токен в APPDATA/TrineDocs/mcp.json (ADR-0025). | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-007 | docs/MCP.md §1 | - macOS unix socket в `~/Library/Application Support/TrineDocs/`, 0600. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-008 | docs/MCP.md §1 | При отсутствии приложения шим запускает TrineDocs --background и ждёт loopback до 10 секунд. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-009 | docs/MCP.md §1 | - IPC: same OS user SID/uid + одноразовый токен в owner-only файле. Конкретная handshake/replay-защита — threat model. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-010 | docs/MCP.md §1 | - Только `127.0.0.1` / `::1`; настройка порта, следующий свободный при занятости, фактический порт виден UI. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-011 | docs/MCP.md §1 | Bearer-токен CSPRNG при первом запуске, файл конфигурации в пользовательском APPDATA. Keychain/ротация требуют отдельной реализации; не объявлять их выполненными. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-012 | docs/MCP.md §1 | - Проверка Origin **каждого** запроса; чужой browser Origin отклоняется. Отдельно определить разрешённые Origin, отсутствие Origin у native clients и Host validation; ADR-0010. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-013 | docs/MCP.md §1 | Streamable HTTP запускается по умолчанию в отдельном потоке, переключатель MCP отзывает клиентский доступ (план владельца/ADR-0025). | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M01-014 | docs/MCP.md §1 | Встроенный чат — тот же сервер, in-memory transport, одна реализация инструментов. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-001 | docs/MCP.md §2 | - MCP → Подключения: обнаруженные Claude Desktop, Codex CLI, Cursor, VS Code, Windsurf и др., «Подключить». | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-002 | docs/MCP.md §2 | - Согласие → дифф → бэкап → дописать только запись `trinedocs` с stdio-командой абсолютного пути к шиму. «Отключить» удаляет только принадлежащую приложению запись. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-003 | docs/MCP.md §2 | - **Кандидатные пути из исходного задания; не использовать до сверки официальной документации:** | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-004 | docs/MCP.md §2 | - Claude Desktop: `~/Library/Application Support/Claude/claude_desktop_config.json` / `%APPDATA%\Claude\claude_desktop_config.json`, `mcpServers`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-005 | docs/MCP.md §2 | - Codex CLI: `~/.codex/config.toml`, `[mcp_servers.trinedocs]`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-006 | docs/MCP.md §2 | - Cursor: `~/.cursor/mcp.json`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-007 | docs/MCP.md §2 | - VS Code/Windsurf: найти актуальные пути/форматы, FACT-REGISTRY. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-008 | docs/MCP.md §2 | - Копировать JSON-конфиг / команду для иных клиентов. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M02-009 | docs/MCP.md §2 | - Какие клиенты подключены, последний вызов, «Отключить сессию». | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-001 | docs/MCP.md §3 | - Клиент: read-only / edit / full (edit + экспорт, PDF, create/delete, Google). Новый внешний и встроенный чат по умолчанию edit. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-002 | docs/MCP.md §3 | - Первый новый клиент: имя, запрошенный доступ, Разрешить / Один раз / Запретить. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-003 | docs/MCP.md §3 | - Elicitation либо локальный диалог: delete document/sheet/slide, >N абзацев/ячеек (настройка), экспорт за пределы приложения, overwrite disk, Google upload. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-004 | docs/MCP.md §3 | - По умолчанию только открытые и Недавние. Arbitrary path только явный opt-in и allowlisted folders, canonical path/reparse checks. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-005 | docs/MCP.md §3 | - Аудит каждый вызов: клиент/tool/doc/time/result, переход/undo в «Активность MCP». | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-006 | docs/MCP.md §3 | - Rate limit, response size, pagination, timeout. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-007 | docs/MCP.md §3 | - Документ — недоверенные данные; text в явно обозначенных полях, не команды. Это явно сказано в tool descriptions. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M03-008 | docs/MCP.md §3 | - Наследование доступа по токену, сессии и текущей роли; отзыв должен прекращать уже начатую незафиксированную правку (ADR-0010). | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M04-001 | docs/MCP.md §4 | - Stable IDs: `doc_`, `blk_`, `tbl_`, `img_`, `sht_`, `sld_`, `shp_`, `cmt_`, `ver_`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M04-002 | docs/MCP.md §4 | Текстовые offset считают Unicode scalar values, не байты UTF-8. Grapheme-safe навигация остаётся отдельным требованием и до реализации отмечается gap. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M04-003 | docs/MCP.md §4 | - Sheets A1 (`Лист1!B2:D10`, Cyrillic names) или `{ sheet_id, row_id, col_id }`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M04-004 | docs/MCP.md §4 | - Slides/shapes по ID; порядок ответов 0..N. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M04-005 | docs/MCP.md §4 | - Read содержит компактный CRDT version vector. Optional `base_version`: если затронутые объекты менялись → `conflict` + текущий фрагмент; без него правка текущего состояния со слиянием CRDT. Границы «затронутого» должны быть формализованы. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M04-006 | docs/MCP.md §4 | - pt; colors `#RRGGBB` / theme name `accent1`; font family name. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M05-001 | docs/MCP.md §5 | - `inputSchema`, `outputSchema`, честные `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M05-002 | docs/MCP.md §5 | - Изменение: обязательный `document_id`, optional `base_version`; `mode` apply(default)/suggest/dry_run; `label` undo/AI. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M05-003 | docs/MCP.md §5 | - Ответ `{ applied, version, changed_ids[], diff_summary, undo_group_id }`. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M05-004 | docs/MCP.md §5 | - Для create/open (ещё нет открытого document_id) и app/asset операций схема адресации уточняется ADR-0010, не придумывать фиктивный doc ID. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M05-005 | docs/MCP.md §5 | - Длительные export/LO/Google/bulk: tasks при поддержке клиента, иначе progress; cancellation. В 2026-07-28 Tasks — opt-in extension, явно согласуется поддержка обеих сторон; наличие инструмента в rmcp не доказывает поддержку клиентом. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M05-006 | docs/MCP.md §5 | - Structured errors `not_found`, `conflict`, `permission_denied`, `needs_confirmation`, `invalid_range`, `unsupported_feature`, `too_large`, `timeout`, `internal`, сообщение/дальнейшие действия. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-001 | docs/MCP.md §6 | \| `app_status` \| версия, открытые/активный документ, selection, права клиента \| ✓ \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-002 | docs/MCP.md §6 | \| `list_documents` \| open/recent/pinned/cloud, тип/query/date, pagination \| ✓ \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-003 | docs/MCP.md §6 | \| `search_documents` \| FTS библиотеки \| ✓ \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-004 | docs/MCP.md §6 | \| `open_document` \| library ID / разрешённый path / link, показать пользователю \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-005 | docs/MCP.md §6 | \| `create_document` \| Docs/Sheets/Slides, template, Markdown/table/deck seed \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-006 | docs/MCP.md §6 | \| `close_document` \| с save/без, confirmation \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-007 | docs/MCP.md §6 | \| `save_document` \| текущий файл \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-008 | docs/MCP.md §6 | \| `save_document_as` \| path/format/loss report, confirmation \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-009 | docs/MCP.md §6 | \| `get_document_info` \| properties/stats/format/path/author/presence/sync \| ✓ \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-010 | docs/MCP.md §6 | \| `set_document_properties` \| title/author/tags \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-011 | docs/MCP.md §6 | \| `focus` \| scroll/object/selection показать пользователю \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-012 | docs/MCP.md §6 | \| `get_selection` \| текущая selection пользователя \| ✓ \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-013 | docs/MCP.md §6 | \| `set_selection` \| установить selection \| — \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-014 | docs/MCP.md §6 | \| `get_outline` \| Docs headings/blocks IDs; Sheets ranges/tables/charts/pivots; Slides titles/shapes \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-015 | docs/MCP.md §6 | \| `read_content` \| range/objects: markdown ID anchors / full formatted json / text; block pagination \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-016 | docs/MCP.md §6 | \| `find` \| text/regex/format/value/formula, positions/context \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-017 | docs/MCP.md §6 | \| `render_preview` \| PNG page/range/slide, ограниченное разрешение \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-018 | docs/MCP.md §6 | \| `insert_content` \| position + Markdown/JSON paragraphs/lists/tables/asset images/links, style mapping \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-019 | docs/MCP.md §6 | \| `replace_range` \| replacement, preserve surrounding format/explicit format \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-020 | docs/MCP.md §6 | \| `delete_range` \| range/blocks \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-021 | docs/MCP.md §6 | \| `move_blocks` \| reorder blocks \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-022 | docs/MCP.md §6 | \| `format_text` \| bold/italic/underline/strike/font/size/color/highlight/super/sub/case/clear \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-023 | docs/MCP.md §6 | \| `set_paragraph_format` \| style/align/indents/spacing/tabs/keep/breaks \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-024 | docs/MCP.md §6 | \| `apply_style` / `define_style` \| named style apply/create/update \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-025 | docs/MCP.md §6 | \| `set_list` \| type/level/restart \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-026 | docs/MCP.md §6 | \| `insert_table` / `edit_table` \| rows/cols/merge/borders/fill/width/header \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-027 | docs/MCP.md §6 | \| `insert_image` \| asset/allowed file/generated; size/wrap/caption \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-028 | docs/MCP.md §6 | \| `insert_break` \| page/section/column \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-029 | docs/MCP.md §6 | \| `set_page_setup` \| paper/orientation/margins/columns/headers/page numbering \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-030 | docs/MCP.md §6 | \| `insert_field` \| TOC/page/date/crossref/footnote \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-031 | docs/MCP.md §6 | \| `update_fields` \| TOC/fields \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-032 | docs/MCP.md §6 | \| `replace_all` \| whole-document replacement, count preview \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-033 | docs/MCP.md §6 | \| `read_cells` \| values/formulas/formats/display text; values-only \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-034 | docs/MCP.md §6 | \| `write_cells` \| atomic 2D array into range, explicit/inferred types \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-035 | docs/MCP.md §6 | \| `set_formula` \| cell/range relative fill, syntax validation before apply \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-036 | docs/MCP.md §6 | \| `explain_formula` \| dependencies/intermediate values, RO \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-037 | docs/MCP.md §6 | \| `format_cells` \| number/font/color/fill/borders/align/wrap/rotation \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-038 | docs/MCP.md §6 | \| `merge_cells` / `unmerge_cells` \| merge \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-039 | docs/MCP.md §6 | \| `insert_rows` / `insert_cols` / `delete_rows` / `delete_cols` / `resize` / `hide` \| structure \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-040 | docs/MCP.md §6 | \| `sort_range` / `filter_range` \| multi-key sort/autofilter \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-041 | docs/MCP.md §6 | \| `freeze_panes` \| freeze \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-042 | docs/MCP.md §6 | \| `add_sheet` / `rename_sheet` / `move_sheet` / `delete_sheet` \| sheet operations \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-043 | docs/MCP.md §6 | \| `create_chart` / `update_chart` \| type/data/axes/title/legend/position \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-044 | docs/MCP.md §6 | \| `create_pivot` / `update_pivot` \| source/rows/columns/values/filters \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-045 | docs/MCP.md §6 | \| `conditional_format` \| rules \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-046 | docs/MCP.md §6 | \| `data_validation` \| validation \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-047 | docs/MCP.md §6 | \| `define_name` \| named ranges \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-048 | docs/MCP.md §6 | \| `recalculate` \| sheet/book \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-049 | docs/MCP.md §6 | \| `import_csv_to_sheet` \| string/asset CSV, parse parameters \| | 5 | T-05-CALC, T-05-GRID, T-02-ROUNDTRIP | planned |
| REQ-M06-050 | docs/MCP.md §6 | \| `list_slides` \| ID/index/layout/title/hidden/preview, RO \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-051 | docs/MCP.md §6 | \| `add_slide` / `duplicate_slide` / `delete_slide` / `reorder_slides` / `hide_slide` \| slide operations \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-052 | docs/MCP.md §6 | \| `set_layout` / `apply_theme` \| layout/theme \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-053 | docs/MCP.md §6 | \| `add_shape` / `update_shape` / `delete_shape` \| text/shape/image/table/chart; pos/size/rotation/fill/stroke/formatted text \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-054 | docs/MCP.md §6 | \| `align_shapes` / `group_shapes` / `set_z_order` \| composition \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-055 | docs/MCP.md §6 | \| `set_notes` \| speaker notes \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-056 | docs/MCP.md §6 | \| `add_animation` / `update_animation` / `remove_animation` \| effect/trigger/delay/duration/order \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-057 | docs/MCP.md §6 | \| `set_transition` \| type/duration/auto advance \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-058 | docs/MCP.md §6 | \| `generate_deck` \| structure/theme/titles/bullets/notes, batch task \| | 6 | T-06-SLIDES, T-02-ROUNDTRIP, T-02-PRINT | planned |
| REQ-M06-059 | docs/MCP.md §6 | \| `list_comments` / `add_comment` / `reply_comment` / `resolve_comment` \| comments/threads \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-060 | docs/MCP.md §6 | \| `list_suggestions` \| track changes/AI suggest, RO \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-061 | docs/MCP.md §6 | \| `accept_suggestions` / `reject_suggestions` \| by IDs/all; edit + user confirmation для чужих \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-062 | docs/MCP.md §6 | \| `list_versions` / `create_version` \| history/named snapshot \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-063 | docs/MCP.md §6 | \| `diff_versions` \| diff, RO \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-064 | docs/MCP.md §6 | \| `restore_version` \| restore, confirmation \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-065 | docs/MCP.md §6 | \| `export_document` \| все AGENTS §8 formats/parameters; asset ID/allowed path, losses, task \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-066 | docs/MCP.md §6 | \| `print_to_pdf` \| print range/handouts/area/scale \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-067 | docs/MCP.md §6 | \| `print_document` \| только через user confirmation dialog \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-068 | docs/MCP.md §6 | \| `google_export` \| connected account + confirmation; Google/Office, returns URL \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-069 | docs/MCP.md §6 | \| `google_import` \| accessible Drive file \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-070 | docs/MCP.md §6 | \| `upload_asset` \| base64 limited image/file → asset ID \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-071 | docs/MCP.md §6 | \| `get_asset` \| preview/export asset \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-072 | docs/MCP.md §6 | `apply_ops`: по исходному требованию атомарный пакет операций **любых типов выше** одного документа, всё/ничего, одна undo group. ⚠ Это конфликтует с необратимыми side effects (печать, сеть). ADR-0010 предлагает ограничить пакет транзакционными модельными ops, а side effects выделить отдельно; предложение пока не принято и не заменяет требование. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M06-073 | docs/MCP.md §6 | Все UI-команды должны иметь ops/MCP-путь и наоборот. Недостающие инструменты находятся в `docs/mcp-gaps.md`; `apply_ops` не служит маскировкой отсутствующей схемы конкретной операции. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-001 | docs/MCP.md §7 | \| `trinedocs://documents` \| open/recent \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-002 | docs/MCP.md §7 | \| `trinedocs://doc/{id}/outline` \| outline \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-003 | docs/MCP.md §7 | \| `trinedocs://doc/{id}/content` \| Markdown Docs / Sheets summary / Slides text \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-004 | docs/MCP.md §7 | \| `trinedocs://doc/{id}/sheet/{name}/{range}` \| CSV/JSON cells \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-005 | docs/MCP.md §7 | \| `trinedocs://doc/{id}/slide/{sid}/preview.png` \| slide preview \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-006 | docs/MCP.md §7 | \| `trinedocs://doc/{id}/selection` \| user selection \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-007 | docs/MCP.md §7 | \| `trinedocs://doc/{id}/comments` \| comments \| | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M07-008 | docs/MCP.md §7 | Subscriptions: document edit → resource-updated; resource set change → list_changed. Проверяются revocation/privacy/pagination, percent-encoding name/range. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M08-001 | docs/MCP.md §8 | `summarize`; `proofread` suggest; `rewrite_selection` formal/simple/shorter/longer; `translate`; `make_table_from_text`; `analyze_sheet` anomalies/formulas; `build_chart`; `deck_from_document`; `speaker_notes`; `fix_formulas` #REF!/#DIV/0! proposals. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M09-001 | docs/MCP.md §9 | - MCP edit session — CRDT participant, cursor, встроенный AI / внешнее имя Claude/Codex, ai palette. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M09-002 | docs/MCP.md §9 | - Smooth cursor → next edit, fading highlights. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M09-003 | docs/MCP.md §9 | - «Агент редактирует», Stop → cancel session/rollback unfinished step. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M09-004 | docs/MCP.md §9 | - Concurrent user/remote edits converge. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M09-005 | docs/MCP.md §9 | - Agent undo groups, AI review panel, suggest=track changes. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M10-001 | docs/MCP.md §10 | - «Активность MCP»: live calls, masked content parameters by settings, duration/errors/jump to edits. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M10-002 | docs/MCP.md §10 | - Log level settings, MCP logging notifications. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M10-003 | docs/MCP.md §10 | - Inspector-compatible debug endpoint только dev build. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M11-001 | docs/MCP.md §11 | - Каждому tool: schema/errors/rights/dry_run/suggest/base_version conflicts. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M11-002 | docs/MCP.md §11 | - rmcp e2e client: stdio→shim→IPC, bearer HTTP/Origin, in-process. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M11-003 | docs/MCP.md §11 | - Live concurrent agent/user/participant: same state, correct cursors. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M11-004 | docs/MCP.md §11 | - MCP Inspector/Claude Desktop/Codex CLI ручная проверка каждого релиза. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M11-005 | docs/MCP.md §11 | - Reject missing bearer/foreign Origin/other-user IPC; injection fixture не вызывает действий вне запроса. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-M11-006 | docs/MCP.md §11 | - Tasks/elicitation capability fallback реально тестируется, не предполагается по наличию фичи в протоколе. | 3 | T-03-MCP, T-03-AI, T-SECURITY | planned |
| REQ-A00-001 | docs/SPEC-AUDIT.md §0 | Приоритет равен AGENTS.md и MCP.md. При расхождении действует это дополнение, **кроме жёстких ограничений раздела 1 AGENTS.md** (платформы, Electron/Tauri/WebView/TS/JS, стек, лицензии), которые меняет только владелец через ADR. Это включает и заменяет прежнее короткое дополнение из 11 пунктов, если оно было. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A00-002 | docs/SPEC-AUDIT.md §0 | Исполнитель — соавтор и аудитор. Спецификация содержит потенциально неверные/устаревшие факты, противоречия/дыры и нереалистичные места. Найти до дорогой реализации, проверить первоисточники, исправить и зафиксировать. Молчаливый обход и слепое выполнение замеченной ошибки запрещены. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-001 | docs/SPEC-AUDIT.md §1 | 1. Полностью читать AGENTS, MCP, этот файл. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-002 | docs/SPEC-AUDIT.md §1 | 2. Каждому пункту AGENTS/MCP стабильный `REQ-<раздел>-<номер>`. `docs/traceability.md`: ID → описание → фаза → проверки → статус. Отсутствие фазы/проверки — дефект. Для двух источников раздел MCP получает префикс M (например `REQ-M03-004`), чтобы IDs не пересекались; это редакционное уточнение записывается в changelog. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-003 | docs/SPEC-AUDIT.md §1 | 3. Аудит §3 → `docs/audit/00-spec-audit.md` по §8. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-004 | docs/SPEC-AUDIT.md §1 | 4. Рискованные спайки §4 → `docs/spikes/NN-название.md`, цифры, go/no-go. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-005 | docs/SPEC-AUDIT.md §1 | 5. Исправления по §6 → `docs/spec-changelog.md`. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-006 | docs/SPEC-AUDIT.md §1 | 6. Владельцу отчёт ≤1 страницы: блокеры, решения владельца, исправления, непроверенное. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A01-007 | docs/SPEC-AUDIT.md §1 | Фаза 0 начинается параллельно после проверки AGENTS §§1–5/20; не ждёт весь аудит. **Фаза 1 не начинается без спайков вёрстки и CRDT.** Указание владельца 2026-10-13 позволяет пока работать/проверять Windows; macOS-замеры откладываются и никогда не выдаются за выполненные (ADR-0001). | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A02-001 | docs/SPEC-AUDIT.md §2 | - Только свежие первоисточники: official docs/releases/repos/licenses, ECMA-376/ISO 29500/MS-OI29500/MCP/OAuth/Google. Дата из системы. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A02-002 | docs/SPEC-AUDIT.md §2 | - Версии, лицензии, лимиты, цены, config paths, platforms — никогда из памяти, читать. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A02-003 | docs/SPEC-AUDIT.md §2 | - `facts.md`: утверждение → статус → URL → дата. `✔ подтверждено`, `✘ неверно (исправлено)`, `? не удалось проверить` + причина, `➕ пробел`. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A02-004 | docs/SPEC-AUDIT.md §2 | - Конфликт: оба источника, причина доверия. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A02-005 | docs/SPEC-AUDIT.md §2 | - Юридические вопросы: факты, «передать владельцу/юристу», без юридических выводов. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A02-006 | docs/SPEC-AUDIT.md §2 | - Не выдумывать; допустимо «не знаю, вот как проверить». | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-001 | docs/SPEC-AUDIT.md §3 | - Qt: версия 6.12 LTS/open-source patches, Supported Platforms Windows/macOS; LGPL vs GPL-only/коммерческие Charts/Graphs/Quick 3D/Multimedia/Print Support/SVG/PDF/WebEngine и др.; dynamic linking. CXX-Qt latest/6.12 compatibility; QStyleHints, Mica, Snap custom Qt Quick title, SMAppService. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-002 | docs/SPEC-AUDIT.md §3 | - Rust: Loro version/API/format stability/shallow snapshot/UndoManager/marks/MovableTree/List/memory/performance, Yrs. IronCalc version/full Excel function list/dynamic arrays/LET/LAMBDA/xlsx round-trip. skia-safe targets/textlayout/pdf/size/build time. rmcp version/tools/outputSchema/resources/subscriptions/prompts/elicitation/tasks/progress/cancel/HTTP/auth. Stable MCP spec. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-003 | docs/SPEC-AUDIT.md §3 | - Clients: Claude/Codex/Cursor/VS Code/Windsurf actual config paths/formats, elicitation/tasks in each. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-004 | docs/SPEC-AUDIT.md §3 | - Google: drive.file scopes/verification, export size/quotas, Picker need, OAuth publication, desktop loopback+PKCE. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-005 | docs/SPEC-AUDIT.md §3 | - Distribution: MSIX signing/sideload, Azure Trusted Signing availability/conditions/alternatives, Apple notarization, Sparkle/WinSparkle versions/licenses, Store sandbox vs shim/LO/pipes/sockets. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-006 | docs/SPEC-AUDIT.md §3 | - System APIs: Windows spelling, NSSpellChecker, Windows.Media.Ocr, Vision, TTS, Quick Look/preview handler, QAccessible canvas. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-007 | docs/SPEC-AUDIT.md §3 | - Files/fonts: LO license/headless flags/distribution licenses/size/safe download; Carlito/Caladea/Liberation metric compatibility/licenses; Inter/JetBrains/Lucide licenses. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-008 | docs/SPEC-AUDIT.md §3 | - OOXML: ECMA-376/ISO and MS-OI29500 departures. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-009 | docs/SPEC-AUDIT.md §3 | - AGENTS vs MCP tools/formats/rights/links. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-010 | docs/SPEC-AUDIT.md §3 | - Каждая AGENTS §11 функция: фаза, parity, tests, MCP. Всё UI доступно через ops/tool и наоборот. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-011 | docs/SPEC-AUDIT.md §3 | - Измеримые/reproducible phase acceptance/CI. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-012 | docs/SPEC-AUDIT.md §3 | - Dependencies: export/print need layout/render. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-013 | docs/SPEC-AUDIT.md §3 | - Совместная достижимость нефункциональных целей. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-014 | docs/SPEC-AUDIT.md §3 | **[ВЛАДЕЛЕЦ]**: варианты, плюсы/минусы, стоимость, рекомендация, статус «ждёт владельца»; продолжать независимую работу. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-015 | docs/SPEC-AUDIT.md §3 | 1. v1: предложение Docs P0 → Sheets P0 → MCP/AI → collaboration → export/print; Slides/animations/Google/P1 позже. **[ВЛАДЕЛЕЦ]** Slides и следующие только после подтверждения. Подтверждение продолжать до последней фазы получено2026-10-13; номерные критерии приёмки сохраняются (ADR-0005). | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-016 | docs/SPEC-AUDIT.md §3 | 2. Бесплатно/subscription/open source → Qt license/Store/server cost. **[ВЛАДЕЛЕЦ]** | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-017 | docs/SPEC-AUDIT.md §3 | 3. SkParagraph только paragraph; pagination/wrapping/footnotes/columns/table split — наш Word-layer. Спайк 4.1 до решения. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-018 | docs/SPEC-AUDIT.md §3 | 4. IronCalc gaps: fork/custom/alternative; CalcEngine trait сохраняется. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-019 | docs/SPEC-AUDIT.md §3 | 5. OOXML/LO child process, memory/time limits, termination; corrupt file не валит app. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-020 | docs/SPEC-AUDIT.md §3 | 6. External file three-way merge local/disk/base с compare UI. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-021 | docs/SPEC-AUDIT.md §3 | 7. Корпус GovDocs1/LO/POI/OSS fixtures: проверить каждую лицензию. LibreOffice автоматический layout baseline + manual Word sample, px/pt tolerance/metric. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-022 | docs/SPEC-AUDIT.md §3 | 8. CRDT history compression/shallow thresholds adjustable; time machine/undo impact ADR. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-023 | docs/SPEC-AUDIT.md §3 | 9. Server email/invites/@, account/data deletion, GDPR и аналоги, monitoring/alerts/backup/restore/cost N, E2EE choice (search/preview/AI), abuse. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-024 | docs/SPEC-AUDIT.md §3 | 10. Coauthor missing fonts: fallback/embedding/cloud with licenses ADR. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-025 | docs/SPEC-AUDIT.md §3 | 11. AI default BYOK, API vs consumer subscription billing проверять и честно UI. Proxy/quotas — отдельный ADR **[ВЛАДЕЛЕЦ]**, не самовольно. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-026 | docs/SPEC-AUDIT.md §3 | 12. Windows ARM64: сначала x64 emulation; Qt/Skia/pipes/IME test. Native ADR **[ВЛАДЕЛЕЦ]**. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-027 | docs/SPEC-AUDIT.md §3 | 13. Explorer/Finder previews Quick Look/handler P1 parity. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-028 | docs/SPEC-AUDIT.md §3 | 14. TrineDocs similar names/domains/document companies/trademark classes facts, без legal conclusion. Name/extensions **[ВЛАДЕЛЕЦ]** ADR и conflicts. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-029 | docs/SPEC-AUDIT.md §3 | 15. Uninstall: MCP configs/startup/keychain/cache; update signatures/rollback; file/database migrations/versioning/forward-backward compatibility. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-030 | docs/SPEC-AUDIT.md §3 | 16. EULA/privacy/About/THIRD-PARTY-NOTICES/opt-in/content retention by AI. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-031 | docs/SPEC-AUDIT.md §3 | 17. Max rows/cols against Excel, pages/slides/file size, overflow/memory. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-032 | docs/SPEC-AUDIT.md §3 | 18. Excel dates 1900/1904/leap-bug; 15 digits; circular refs; locale arg separator; recalc order; volatile functions. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-033 | docs/SPEC-AUDIT.md §3 | 19. Word docDefaults/latent styles/abstractNum/num/fields/w:compat/version-sensitive layout. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-034 | docs/SPEC-AUDIT.md §3 | 20. Clipboard read/write formats each OS, Word/Excel/PowerPoint/browser, drag-and-drop. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-035 | docs/SPEC-AUDIT.md §3 | - End-to-end сценарии: first launch; file association; edit/save; crash/recover; huge file; print; Google export; Claude connection; invite; offline/reconnect; update; uninstall. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-036 | docs/SPEC-AUDIT.md §3 | - Сверка Office ribbons по официальным Microsoft docs: Home/Insert/Layout/References/Mailings/Review/View/Formulas/Data/Design/Animations/Slide Show. Missing → parity with priority. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-037 | docs/SPEC-AUDIT.md §3 | - STRIDE MCP IPC/HTTP, import, sync → `docs/security/threat-model.md`; unmitigated → audit. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A03-038 | docs/SPEC-AUDIT.md §3 | - Experienced Word/Excel red team собственной parity. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-001 | docs/SPEC-AUDIT.md §4 | Ограниченное время, метод, **обе ОС**, measured numbers, go/no-go/change. При провале числа и ADR альтернативы. Windows-first разрешает отложить Mac результаты, не исключает их из итогового DoD. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-002 | docs/SPEC-AUDIT.md §4 | Complex real docx: multipage tables/floating images/columns/footnotes/headers/lists/styles; compare LO metric. (a) own on SkParagraph; (b) mature OSS Rust text/grid/block components, licenses; (c) LO MPL layout engine feasibility vs architecture/perf. Criteria: accuracy, ≤16ms incremental paragraph edit, effort/license/maintenance. Не решать без данных. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-003 | docs/SPEC-AUDIT.md §4 | Loro 100 pages rich text/100k cells/100 slides; memory/open/merge/history 1m ops/undo/shallow. If fails compare Yrs. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-004 | docs/SPEC-AUDIT.md §4 | Skia tiles→Qt CPU texture upload vs GPU RHI Metal/D3D11/12: latency/CPU/GPU/complexity/ProMotion/scaling/60-120fps. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-005 | docs/SPEC-AUDIT.md §4 | CJK IME composition/selection/drag/mac magnifier; QAccessible tree Narrator/VoiceOver viable quality. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-006 | docs/SPEC-AUDIT.md §4 | IronCalc vs full official Excel list/test corpus: coverage/correctness/100k perf/incremental, plan B. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-007 | docs/SPEC-AUDIT.md §4 | rmcp one edit: stdio shim→IPC→app; shim startup; Claude/Codex; actually test elicitation/tasks. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A04-008 | docs/SPEC-AUDIT.md §4 | Clean Qt+CXX-Qt+Skia both CI targets: time/size/cache, signed/notarized empty artifact, clean install/startup. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A05-001 | docs/SPEC-AUDIT.md §5 | Architecture/scope/cost/license/security → `docs/decisions/NNNN-название.md`: context/options/criteria/data(links)/decision/consequences/status proposed/accepted/await owner/superseded. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A05-002 | docs/SPEC-AUDIT.md §5 | Owner decisions not self-approved: comparison/recommendation/reason/blockers, review in 5min. Own decisions accept+record+proceed; no barrage of minor questions. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A06-001 | docs/SPEC-AUDIT.md §6 | - Wrong low-risk fact → AGENTS/MCP edit + changelog what/why/source/date. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A06-002 | docs/SPEC-AUDIT.md §6 | - High-risk → ADR+owner, mark ⚠ ADR in spec until decision. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A06-003 | docs/SPEC-AUDIT.md §6 | - Hard targets/platform/bans/mandatory features never silently weaken; prove issue with data, owner alone can relax/delete. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A06-004 | docs/SPEC-AUDIT.md §6 | - Never delete requirement to fix defect; no fake stub functionality. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A06-005 | docs/SPEC-AUDIT.md §6 | - Errors in this file corrected same way. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A07-001 | docs/SPEC-AUDIT.md §7 | Start: reread relevant sections, recheck >30d facts and phase-critical facts; mini-audit in phase plan. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A07-002 | docs/SPEC-AUDIT.md §7 | During: all deviations in changelog. **Definition of Done**: code; unit and necessary property/golden; both themes; both OS; perf budget; keyboard/screen reader; ru/en; MCP tool+test; parity+traceability; docs. Owner Windows-first permits provisional Windows acceptance only; global DoD remains unfinished. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A07-003 | docs/SPEC-AUDIT.md §7 | End `docs/audit/phase-N.md`: measured vs targets; all requirements done/not/partial with reasons; new risks; red team corrupt files/races/network/crash/MCP revocation; owner needs. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A08-001 | docs/SPEC-AUDIT.md §8 | Start: status/severity counts, top-5 risks. Severity blocker/high/medium/low; action fixed(changelog N)/ADR/owner/spike/justified accept risk. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A09-001 | docs/SPEC-AUDIT.md §9 | - Report failures, missed target/unreproduced/spike fail. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A09-002 | docs/SPEC-AUDIT.md §9 | - No done for stubs/single OS/no tests. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A09-003 | docs/SPEC-AUDIT.md §9 | - Numbers measured with bench; estimates labelled estimates. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A09-004 | docs/SPEC-AUDIT.md §9 | - No flattery/blind agreement; argue poor/costly choices with evidence/alternatives. | 0 | T-00-SPEC, T-FACTS | planned |
| REQ-A09-005 | docs/SPEC-AUDIT.md §9 | - Reports/ADR/docs Russian; code/identifiers/commits English. | 0 | T-00-SPEC, T-FACTS | planned |
