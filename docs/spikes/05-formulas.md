# Спайк 4.5 IronCalc

Статус: **не запущен**, go/no-go **не вынесен**. Это план эксперимента, не результат.

## Метод

Exact pin audited source vs full official Excel function list, function-case corpus/date1900-1904/precision/locale/cycles/spill;100k dependency graph densities.

## Стенды

Windows11x64 minimum22000/current: CPU/GPU/RAM/display/DPI/build/toolchain/fixture hashes будут записаны при запуске. Локальная среда сейчас Linux, без rustc/cargo/Qt/CMake/pwsh; она не является Windows стендом. macOSarm64 отложена владельцем ADR-0001.

## Критерии и ограничение времени

100k dependent update≤50ms target; coverage perfunction/not release claim; planB ADR. Distinguish parse/recalc/import cached values.

Плановый timebox: один изолированный prototype cycle; для layout/CRDT сначала согласовать licensed fixtures и executable measurement harness. Это организационный план, не обещание числовых трудозатрат. После timebox зафиксировать результат, даже если no-go.

## Таблица результатов

| Метрика | Windows11x64 | macOSarm64 | Данные/артефакт |
|---|---|---|---|
| Производительность/корректность |не измерено|отложено владельцем|нет|
| Размер/RSS/buildtime |не измерено|отложено владельцем|нет|
| Решение |BLOCKED/NOT_RUN|DEFERRED|нет оснований для go|

Провал/отсутствие измерений не заменять оценкой на глаз; если цель не достигнута — raw numbers и ADR альтернативы.
