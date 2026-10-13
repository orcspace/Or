# Большое ревью 1: Docs, Slides, PDF, импорт книг

Дата: 2026-10-13. Автор: агент Docs. Проверены docs/model.rs, docs/io.rs, docs/editor.rs; независимое чтение slides/model.rs, slides/io.rs, slides/editor.rs, slides/present.rs, pdf.rs, render.rs, fonts.rs, native.rs, package.rs, sheets/io.rs. Все находки переданы владельцам модулей. Чужие исходники агент Docs не редактировал.

Это статическое ревью и проверка внесённых исправлений по исходникам. Проверку cargo test/clippy/fmt запускает основной агент под общей claim; результаты и внешнее визуальное QA сюда не подменяются.

| Уровень | Сценарий и место первоначальной ошибки | Исправление / состояние |
|---|---|---|
| Blocker | pdf.rs, сборка Page Resources: повторный resources().fonts() на каждый шрифт и x_objects() на каждое изображение создавал повторные /Resources и /Font dictionary keys. Страница с normal/bold и картинкой могла сохранить лишь последний набор ресурсов. | Основной агент объединил словари: один /Resources, один /Font и один /XObject. Свежие исходники проверены. Требуется внешняя проверка PDF и regression нескольких шрифтов/картинок. |
| High | pdf.rs, CID widths: вызов cid.widths() в каждой итерации создавал повторные /W, и метрики могли потеряться. | Один builder /W на все CID. Свежие исходники проверены. |
| High | render.rs игнорировал bold/italic у TextStyled; экран расходился с DOCX и PDF. | Выбор системных bold/italic/bold-italic families реализован основным агентом. |
| High | render.rs и slides/editor.rs: кэш картинок по размеру/короткому префиксу и ID совпадал между двумя документами с одинаковыми PNG headers и ID. Могла отображаться чужая картинка. | Ключ теперь хэш полного массива bytes. Свежие исходники проверены. |
| High | docs/model.rs render_table: высокая строка ограничивалась высотой страницы, последующий take() молча удалял хвост текста из экрана/PDF. | Строка таблицы теперь продолжается на следующих страницах без усечения. Добавлена регрессия long_table_row_has_no_text_loss. |
| High | docs/io.rs: take(10000) rows / take(256) cells молча обрезал большие таблицы. | Превышение пределов возвращает ошибку, все разрешённые элементы читаются. |
| High | docs/io.rs экспорт и editor.rs вставка декодировали изображение прежде ограничения количества пикселей. | Используется util::decode_image: сначала заголовок/dimensions, затем decode. Docs::validate дополнительно проверяет заголовки и общий byte budget. |
| High | native.rs Document::validate проверял лишь верхние counts. JSON мог содержать повторные stable ID, неверный next_id, огромные вложенные ячейки, NaN/Infinity/недопустимую геометрию. | Docs получил глубокий Doc::validate, включая все вложенные IDs, counters, font/geometry, budgets текста/изображений и оценку страниц. Основной агент должен подключить модельные валидаторы в native.rs и ops. До подключения находка открыта. |
| High | sheets/io.rs preflight проверял только xl/worksheets/*.xml. Relationship мог ссылаться на worksheet в другом пути. Calamine создаёт плотный Range до проверки cells.len(). Две дальние ячейки могли потребовать огромную память. | Агент Sheets проверяет root worksheet для каждого XML part и суммарный dense budget до Calamine. Свежие исходники проверены. |
| High | ODS repeats: независимая проверка множителей пропускала product строк × столбцов и повторённый текст. XLS/XLSB мог выделить плотный Range до ограничений приложения. | Агент Sheets ввёл aggregate ODS budgets; XLS/XLSB временно отклоняются до изоляции бинарного импортёра. Ограничение должно быть видно пользователю и в parity. |
| High | slides/model.rs pages и editor: отсутствовали arrowheads/strokes, перенос длинного текста различался, отрицательная диаграмма отображалась как положительная. | Агент Slides добавил arrowheads/strokes, общий text_lines, signed chart baseline. Исправления подтверждены агентом и прочитанными исходниками. Внешний pixel QA остаётся обязательным. |
| Medium | slides/present.rs: по advance_after вызывался next(), который сначала запускал on_click animations; после таймера по одной анимации запускалось каждый кадр. | Таймер теперь переходит непосредственно к следующему не скрытому слайду. Click animations остаются в обычном next(). |
| High, gap | fonts.rs/pdf.rs: первый системный Arial/Segoe может не иметь японских/CJK глифов. PDF отменяет весь экспорт при отсутствующем глифе. | Безопасная явная ошибка вместо молчаливой потери. Полный font fallback и Japanese IME/display/PDF parity ещё не реализованы/не проверены. |
| Medium, gap | PDF baseline size × 0.90 отличается от реальных ascender metrics egui; PDF не учитывает kerning/shaping. | Общий layout уменьшает расхождение, но не доказывает пиксельное совпадение. Нужны renderer comparison и shaping/font fallback. |
| Medium, gap | Slides rotation в PDF, второй монитор, PNG export; Docs table rich canvas, grapheme navigation и accessibility tree отсутствуют/частичны. | Честные ограничения в parity; нельзя считать весь пользовательский план выполненным. |

## Проверка snapshot и форматов

Docs DOCX экспорт не сохраняет внутренний JSON snapshot: при повторном открытии читается реальный OOXML. Поэтому изменения Word не маскируются старой копией модели.

Slides PPTX snapshot защищён manifest всех non-customXml parts. Изменение любого OOXML part или состава архива инвалидирует snapshot; есть regression modified_ooxml_does_not_load_stale_native_model. Сам round-trip через snapshot не доказывает соответствие OOXML. Отдельные тесты должны удалить customXml и проверить форматирование/структуру, затем открыть в независимом Office-compatible потребителе.

Модуль slides/io.rs имеет отдельный legacy trineslide JSON путь, отличный от центрального Envelope. App использует native.rs, но прямой API не взаимозаменяем. Рекомендация: удалить дублирующий маршрут или делегировать центральному encode/decode.

## Добавленные проверки Docs

Unicode scalar insert/delete; стабильность ID и форматирования при многострочной вставке; частичное оформление; неперекрывающаяся замена; переход длинного абзаца и длинной строки таблицы между страницами; вложенные ID/next_id и плохая геометрия; сохранение позиции пользователя при правке агента; русский word navigation; basic Markdown input; DOCX hyperlink identity при одинаковом тексте; footer PAGE field; inline page break; mixed HTML text order; 50 генерируемых DOCX variations.

50 генерируемых вариаций являются синтетическим regression corpus. Требуемые пользователем 50 реальных DOCX / XLSX / PPTX, fuzzing и ручные Windows 10/11 проверки пока не выполнены.

## Заключение

Найдены и исправляются конкретные ошибки потери данных, PDF ресурсов и resource bounds. Релиз можно оценивать после общей проверки тестов, второй независимой волны ревью, визуального QA и проверки установщика. Полный паритет Word/Excel/PowerPoint на основании текущего кода не подтверждён.
