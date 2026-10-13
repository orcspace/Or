# Краткий отчёт владельцу — 2026-10-13 Бишкек

Начат Windows11x64-проект в orcspace/Or. Сохранены три спецификации, Rust core, stable IDs/traceability/parity и первичный аудит. **Окна/EXE ещё нет, фазa0 не принята.** Mac отложен по твоему указанию.

Блокеры окна:

- Qt6.12.0 вышел30сентября и пока нарушает правило14дней. Без исключения ждём age gate, продолжая независимый core.
- QML bindings используют JavaScript, а JS запрещён. ADR-0003 предлагает разрешить только декларативные QML expressions/тонкие обращения в Rust, сохранив запрет JS/TS-файлов и business logic. Это меняет hard ban, нужно решение владельца.
- DWMSBT Mica API minimum22621 при supported22000. ADR-0007: либо solid fallback на22000, либо поднятие минимума, либо исследование поддержанного альтернативного пути. Владелец выбирает, требования не ослаблены.

Исправлено по первоисточникам: macOSminimum14.4; MCPcurrent2026-07-28 и opt-in Tasks. Проверены Qt/CXX-Qt release dates, Google drive.file scope и10MBexportlimit, IronCalc0.8 feature announcement, Loro shallow restrictions, Excel limits; это не доказательство рабочего паритета.

Локально4теста spec-integrity прошли. Rust unit/format/clippy — ещё проверить в CI; native UI tests/performance/spikes отсутствуют. Семь файлов spikes являются планами, не результатами. Полный фактологический/ribbon/license аудит не завершён, неизвестное явно перечислено в facts.md.

Следующие owner decisions (не блокируют текущий pure core): границы v1/порядок фаз, distribution/license/signing, brand/extensions, E2EE/cloud budget. Slides/следующие не начнутся без твоего подтверждения. Собственный AI proxy не создавался.
