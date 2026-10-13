# Спайк 4.2 CRDT

Статус: **не запущен**, go/no-go **не вынесен**. Это план эксперимента, не результат.

## Метод

Loro100pages richtext/100kcells/100slides, 1m edit history; RSS/open/merge/snapshot size/local undo/replay/shallow;3peers/offline long gap; compare Yrs if no-go.

## Стенды

Windows11x64 minimum22000/current: CPU/GPU/RAM/display/DPI/build/toolchain/fixture hashes будут записаны при запуске. Локальная среда сейчас Linux, без rustc/cargo/Qt/CMake/pwsh; она не является Windows стендом. macOSarm64 отложена владельцем ADR-0001.

## Критерии и ограничение времени

Convergence/cursor correctness/no lost offline edits; undo only own; history/retention budgets после данных. Блокирует phase1.

Плановый timebox: один изолированный prototype cycle; для layout/CRDT сначала согласовать licensed fixtures и executable measurement harness. Это организационный план, не обещание числовых трудозатрат. После timebox зафиксировать результат, даже если no-go.

## Таблица результатов

| Метрика | Windows11x64 | macOSarm64 | Данные/артефакт |
|---|---|---|---|
| Производительность/корректность |не измерено|отложено владельцем|нет|
| Размер/RSS/buildtime |не измерено|отложено владельцем|нет|
| Решение |BLOCKED/NOT_RUN|DEFERRED|нет оснований для go|

Провал/отсутствие измерений не заменять оценкой на глаз; если цель не достигнута — raw numbers и ADR альтернативы.
