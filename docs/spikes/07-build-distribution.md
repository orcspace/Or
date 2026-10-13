# Спайк 4.7 Clean build/distribution

Статус: **не запущен**, go/no-go **не вынесен**. Это план эксперимента, не результат.

## Метод

Windows clean pinned Qt6.12+CXXQt0.10+Skia features, record complete deps/licenses/build cache; DLL deployment, unsigned development artifact vs real signed MSIX. Mac/notarization позже.

## Стенды

Windows11x64 minimum22000/current: CPU/GPU/RAM/display/DPI/build/toolchain/fixture hashes будут записаны при запуске. Локальная среда сейчас Linux, без rustc/cargo/Qt/CMake/pwsh; она не является Windows стендом. macOSarm64 отложена владельцем ADR-0001.

## Критерии и ограничение времени

Record buildtime/size/cache, cleanWin11install/window/IME/Snap. Actual signing requires identity, never fake. Exact Qt age gate and QML interpretation resolved before running.

Плановый timebox: один изолированный prototype cycle; для layout/CRDT сначала согласовать licensed fixtures и executable measurement harness. Это организационный план, не обещание числовых трудозатрат. После timebox зафиксировать результат, даже если no-go.

## Таблица результатов

| Метрика | Windows11x64 | macOSarm64 | Данные/артефакт |
|---|---|---|---|
| Производительность/корректность |не измерено|отложено владельцем|нет|
| Размер/RSS/buildtime |не измерено|отложено владельцем|нет|
| Решение |BLOCKED/NOT_RUN|DEFERRED|нет оснований для go|

Провал/отсутствие измерений не заменять оценкой на глаз; если цель не достигнута — raw numbers и ADR альтернативы.
