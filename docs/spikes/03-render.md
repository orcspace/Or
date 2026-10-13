# Спайк 4.3 Skia→Qt

Статус: **не запущен**, go/no-go **не вынесен**. Это план эксперимента, не результат.

## Метод

CPU tile512×512 raster/upload vs GPU RHI D3D11/12 (Metal позже), known DPR/refresh; no UI thread disk/network/raster; shared display list.

## Стенды

Windows11x64 minimum22000/current: CPU/GPU/RAM/display/DPI/build/toolchain/fixture hashes будут записаны при запуске. Локальная среда сейчас Linux, без rustc/cargo/Qt/CMake/pwsh; она не является Windows стендом. macOSarm64 отложена владельцем ADR-0001.

## Критерии и ограничение времени

Record key→present p95≤16ms, scroll60fps/120ProMotion later, CPU/GPU/RSS/upload latency. No provisional GPU choice без данных.

Плановый timebox: один изолированный prototype cycle; для layout/CRDT сначала согласовать licensed fixtures и executable measurement harness. Это организационный план, не обещание числовых трудозатрат. После timebox зафиксировать результат, даже если no-go.

## Таблица результатов

| Метрика | Windows11x64 | macOSarm64 | Данные/артефакт |
|---|---|---|---|
| Производительность/корректность |не измерено|отложено владельцем|нет|
| Размер/RSS/buildtime |не измерено|отложено владельцем|нет|
| Решение |BLOCKED/NOT_RUN|DEFERRED|нет оснований для go|

Провал/отсутствие измерений не заменять оценкой на глаз; если цель не достигнута — raw numbers и ADR альтернативы.
