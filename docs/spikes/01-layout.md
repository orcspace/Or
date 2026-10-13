# Спайк 4.1 Вёрстка

Статус: **не запущен**, go/no-go **не вынесен**. Это план эксперимента, не результат.

## Метод

Сложные лицензированные DOCX: multipage tables, wrap images, columns, notes, headers, lists, style inheritance. LO baseline, Word manual subset; сравнить own SkParagraph layer, audited OSS components, LO worker layout.

## Стенды

Windows11x64 minimum22000/current: CPU/GPU/RAM/display/DPI/build/toolchain/fixture hashes будут записаны при запуске. Локальная среда сейчас Linux, без rustc/cargo/Qt/CMake/pwsh; она не является Windows стендом. macOSarm64 отложена владельцем ADR-0001.

## Критерии и ограничение времени

Одинаковая страница и glyph/image geometry по будущему ADR-0015; edit paragraph→incremental layout≤16ms. Размер/accuracy/effort/license/maintenance записать. Блокирует phase1.

Плановый timebox: один изолированный prototype cycle; для layout/CRDT сначала согласовать licensed fixtures и executable measurement harness. Это организационный план, не обещание числовых трудозатрат. После timebox зафиксировать результат, даже если no-go.

## Таблица результатов

| Метрика | Windows11x64 | macOSarm64 | Данные/артефакт |
|---|---|---|---|
| Производительность/корректность |не измерено|отложено владельцем|нет|
| Размер/RSS/buildtime |не измерено|отложено владельцем|нет|
| Решение |BLOCKED/NOT_RUN|DEFERRED|нет оснований для go|

Провал/отсутствие измерений не заменять оценкой на глаз; если цель не достигнута — raw numbers и ADR альтернативы.
