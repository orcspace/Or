# Паритет Slides с PowerPoint

Проверено по первоисточникам Microsoft 2026-10-13. Реализация находится в `crates/trinedocs/src/slides`. Этот документ описывает фактический объём; полное соответствие PowerPoint не достигнуто.

| Функции | Приоритет | Статус | Форматы и проверки |
|---|---|---|---|
| Добавление, дублирование, удаление, скрытие, перемещение слайдов, стабильные ID | P0 | Реализовано | Unit-тесты ID, порядок и копирование; группы сохраняются в копии с новыми ID; перетаскивание миниатюр |
| Разделы | P0 | Частично: имя раздела у слайда, заголовки разделов в миниатюрах | Нативный формат; сворачивание раздела отсутствует |
| Титульный, текстовый, две колонки и пустой макеты | P0 | Создание и смена геометрии с сохранением текста/ID | PPTX экспортирует абсолютное положение; интерактивный редактор мастеров отсутствует |
| Темы, цвет фона, 16:9 и 4:3 | P0 | Реализовано | Реальные theme/master/layout parts в PPTX |
| Текст, жирный, курсив, цвет, размер | P0 | Реализовано для целого объекта | Без смешанного форматирования внутри объекта; ввод через панель/двойной клик |
| Прямоугольник, эллипс, линия, стрелка | P0 | Реализовано | Прочие геометрии, SmartArt, WordArt отсутствуют |
| Выделение, Ctrl/Shift мультивыделение, движение, 8 ручек, Shift пропорции | P0 | Реализовано | Холст и числовые свойства |
| Поворот | P0 | Частично | Текст/прямоугольник/эллипс на холсте; PPTX geometry; общий PDF display list не содержит вращения; переносы текста по метрикам системного шрифта общие для экрана и PDF |
| Группы | P0 | Частично | Модель+совместное перемещение; PPTX экспорт плоских фигур; вложенные внешние группы не импортируются |
| Слои, копирование объектов, удаление | P0 | Реализовано | Панель свойств; Ctrl+C/V внутренний буфер объектов, Ctrl+D; обмен форматированными объектами с PowerPoint отсутствует |
| Выравнивание/распределение, сетка | P0 | Реализовано | Выравнивание относительно слайда; распределение выделенных объектов; Bring front/back сохраняет относительный порядок выбранных |
| Умные направляющие, масштабирование/отражение/обрезка картинок | P0 | Нет | Изображения PNG/JPEG выбираются/читаются в фоне с лимитами и сохраняют пропорции при вставке; crop, mask и flip отсутствуют |
| Таблицы, добавление/удаление строк и столбцов | P0 | Частично | Значения, размеры и единый стиль в canvas/native; внешний PPTX теряет uniform color/bold/italic/fill/stroke и cell font size; merge/split отсутствуют |
| Диаграммы | P0 | Частично: столбчатая, редактируемые значения | Реальные DrawingML chart parts; прочие типы и связь с Sheets отсутствуют |
| Заметки | P0 | Реализовано | PPTX notesSlide/notesMaster; панель в редакторе/показе |
| F5, Shift+F5, полный экран, стрелки/пробел/клик/Esc | P0 | Реализовано | Пропуск скрытых слайдов; Home/End; B/W; unit навигации |
| Указка/перо, очистка | P0 | Реализовано | Рисунки во время показа, не сохраняются в deck |
| Докладчик: следующий слайд, заметки, таймер | P0 | Частично: на текущем экране | Второй монитор не реализован и подписан в UI |
| Переходы fade/push/wipe, автопереход | P1 | Реализовано | Временная интерполяция в UI, реальные PPTX transitions и advTm без эффекта; 60 fps не измерены |
| Входные анимации fade/fly-in/zoom, click/with/after, delay/duration | P1 | Реализовано в локальном показе | Нативный JSON сохраняет; PPTX `p:timing` пишется и читается (Fade, Fly In справа, Zoom; порядок шагов `order`). Не проверено в PowerPoint |
| Цепочки эффектов входа/выделения/выхода и прямой путь | P1 | Исходный код: до 33 шагов на объект, порядок и click/with/after | Fade/Fly In/Zoom, Pulse/Spin, Move dx/dy, Fade/Fly Out/Zoom Out; PPTX timing. Новые тесты добавлены, локально не запускались |
| Кривые пути, repeat/object-trigger/Morph/timeline | P1 | Нет/приближение импорта | Не считаются паритетом; предупреждения о потерях |
| PPTX/PPSX/POTX/PPTM открытие | P0 | Частично | Текст, геометрия, картинки, таблицы, диаграммы, заметки и переходы; макросы/внешние relationships не выполняются |
| PPTX сохранение и нативный ZIP(JSON) | P0 | Реализовано поддерживаемое подмножество | Нативный и PPTX round-trip unit-тесты; собственная модель добавляется customXml JSON, используется только при совпадении manifest остальных частей |
| PDF | P0 | Реализован через общий render | Display list: текст, таблицы, диаграммы, линии/контуры, прямоугольники/эллипсы, изображения; PDF не сохраняет rotation |
| PNG | P0 | CPU экспорт 2× всех показываемых слайдов: меню — основной и нумерованные PNG рядом; MCP `export` — новая папка | Shared Page/egui tessellation; texture/font/alpha/clipping; 8192/32MP и resource budgets. Rotation отсутствует как в PDF; не экспорт текущего слайда |
| Handouts/notes-print | P0 | Нет | Раздаточные и notes-print отсутствуют; не считаются готовыми |
| Медиа аудио/видео/trim/recording/video-export | P1/P2 | Нет | Нет отключённых команд, выдающих это за готовое |
| Comments, accessibility, spelling, custom shows, rehearsal, live captions, collaboration | P1/P2 | Нет в Slides | Не считается Office-паритетом |
| AI/MCP semantic edits, ops/undo authors | P0 | API модели реализован, сервер интегрирует оболочка | Проверяются общей интеграцией; UI редактирует копию и возвращает changed |

Импорт внешнего PPTX не обещает полного сохранения: неизвестные стили, мастеровые placeholders, группы, медиа и timing пропускаются с предупреждением. Сохранение чужого файла должно сопровождаться предупреждением оболочки. Нативный формат сохраняет всю реализованную модель.

Вторая волна ревью: исправлены Shift resize боковых ручек и anchors, pivot text rotation, относительный z-order, стабильная адресация текущего слайда в показе, мягкие переносы/CRLF/tabs, multiline notes/table PDF, title/arrow/auto transitions PPTX. Общий Envelope используется как app, так и slides IO API. Результаты в `docs/audit/review-2-slides.md`.

Безопасность: deep model validation ограничивает суммарный текст 8 МБ, изображения 64 МБ, таблицы 100k ячеек, диаграммы 100k точек, объекты 100k; каждый PNG/JPEG ограничен 8192 по стороне и 32 млн пикселей до decode. IDs групп проверяются с next_id и object IDs. Conservative layout preflight ограничивает estimate до 10k строк на объект/ячейку и 100k по всей презентации; слишком большая модель отклоняется без усечения. Ограничение XML 16 МБ, 500 тыс. элементов и глубины 64; DTD запрещён; внешние relationships игнорируются; пути relationship не выходят из ZIP; картинки на холсте ограничены 32 млн пикселей, входные картинки UI — 32 МБ. ZIP-лимиты задаёт общий `package`.

Источники:

- [PowerPoint help: полный каталог возможностей](https://support.microsoft.com/en-gb/office/powerpoint-help-5395bb85-eb6f-47fa-b5af-52996bac819e).
- [Макеты](https://support.microsoft.com/en-us/powerpoint/what-is-a-slide-layout).
- [Выравнивание и распределение](https://support.microsoft.com/en-us/office/graphics-visuals/align-or-arrange-objects).
- [Фигуры](https://support.microsoft.com/en-us/powerpoint/add-shapes).
- [Направляющие](https://support.microsoft.com/en-us/powerpoint/training/guides-for-arranging-things-on-a-slide).
- [Структура PresentationML](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document).
- [Presentation schema](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-presentations).
- [Slide masters](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-slide-masters).
- [Open XML каталог презентаций и анимаций](https://learn.microsoft.com/en-us/office/open-xml/presentation/overview).

Корпус 50 внешних PPTX, ручная проверка чистой Windows 10/11, DPI/IME/второй монитор и валидатор Microsoft Open XML ещё не выполнены. Unit-тесты не заменяют эти проверки.

PNG added after second review: по сообщению root после source fixes 126 total unit tests и fmt/clippy PASS. Final dist/installer + внешнее PNG visual QA ещё ожидаются; screenshots UI smoke не подменяют slide export.

Уточнение 2026-10-16: любой MCP-клиент получает `slides_design_guide` до создания/редизайна; `slides_search_images` → `slides_add_image_url`, либо `slides_paste_image` из буфера. Нейрогенерация не добавляется. Новые цепочки эффектов и экспорт ожидают выполнения Rust/GUI-проверок; прежние результаты ревью ниже не подтверждают эти изменения.
