# ADR-0022: Windows ARM64

Статус: **ждёт владельца для native build; эмуляция не проверена**. Дата:2026-10-12UTC.

## Контекст

Hard platform Windowsx64; SPEC-AUDIT допускает x64 through emulation как исследуемый путь. Native ARM target не разрешён.

## Варианты и рекомендация

A: x64 native first; B: x64 ARM emulation validate; C: nativeARM owner change. Рекомендация A сейчас, B позже по проверке Qt/Skia/pipes/IME. Do not infer from Windows runner label.

## Критерии, данные и последствия

Current policy rejects unvalidated emulation; это временный validation gate, не утверждение невозможности. Test emulated actual OS then update supported matrix; hard-native ARM change только owner ADR.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
