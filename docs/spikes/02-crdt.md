# Спайк 4.2 CRDT — промежуточные результаты

Дата: 2026-10-13. Статус: **частично выполнен**, итоговый go/no-go **не вынесен**. Фаза 1 не разблокирована: нет результатов вёрстки и целевой Windows 11 приёмки.

## Метод и стенд

[Прототип](../../spikes/crdt/README.md), Rust 1.98.1, release, exact Loro 1.16.2. Граф из 156 внешних Cargo-пакетов закреплён и проверен по crates.io: ≥14 полных суток, stable, не yanked, checksum и заявленная лицензия. Проверка всех notices перед распространением ещё требуется.

Development-стенд: Linux 6.18.44 x86_64, glibc 2.39, виртуализированная среда. Это не Windows 11 или macOS приёмка. Один прогон, без percentile claims; raw JSON — в [results](results/). Windows CI измеряется отдельно; Windows Server runner тоже не заменяет Windows 11 GUI-приёмку. Mac отложен владельцем.

Docs: 100 групп страниц, 200 rich-text блоков, смешанные письменности, bold/italic marks; не реальная пагинация. Sheets: 1000×100 строковых ячеек со стабильными IDs, без формул. Slides: 100×5 синтетических фигур, без рендера. History: миллион committed Map-записей, объединённых Loro в 735 changes; не миллион независимых undo-групп.

open_ms включает import и первый полный обход восстановленного содержимого, для Docs — все delta/marks. import_ms показывает lazy import без чтения. Сравнение с оригиналом идёт после измеряемого обхода. Размеры — байты экспорта, не RSS.

| Workload | Создание, мс | Import, мс | Открытие с чтением, мс | Snapshot, байт | Shallow, байт |
|---|---:|---:|---:|---:|---:|
| Docs | 6.098 | 0.566 | 19.998 | 80051 | 46956 |
| Sheets 100k | 304.831 | 1.661 | 290.087 | 2540646 | 1594190 |
| Slides 100 | 5.913 | 0.247 | 3.007 | 23933 | 13534 |
| History 1m | 1990.507 | 6.913 | 7.004 | 4073477 | 320 |

Три реплики, concurrent text/cell edits и marks, разный порядок доставки и дубликаты: одинаковое состояние. Merge + local undo: 17.415 мс, remote edit сохранён. Четыре теста проходят: convergence, snapshot/marks restore, local-only undo, Unicode scalar/UTF-16/UTF-8. Grapheme-safe MCP не реализован.

## Решение и ограничения

Можно продолжать исследование Loro; полноценный go не доказан. Не измерены целевой RSS, распределение задержек, старые offline peers после shallow compaction, случайные property-based правки и длинная история undo. Map history требует второго сценария rich-text editing. Shallow 320 байт не означает сохранение машины времени.

Windows workflow записывает host/CPU/OS и sampled peak working set каждые 10 мс; последний короткий пик может быть пропущен. После целевых Windows 11 и history/compaction данных обновить ADR-0011; сравнивать Yrs при no-go. Отсутствие данных не заменять оценками.

## Windows CI: измеренный результат

[Прогон 38339267237](https://github.com/orcspace/Or/actions/runs/38339267237) успешен, source commit 2ef720272a2b0448202c493df8c6b919f2367fec. Четыре CRDT tests прошли; release build занял 2 мин 30 с. Это время только optimized build после предыдущей dev/test сборки, а не полная cold build.

Стенд: Microsoft Windows Server 2025 Datacenter, build 26100, AMD EPYC 7763 64-Core Processor, 4 видимых logical CPUs, 16771836 KiB RAM, x86_64. Windows Server — инфраструктурная проверка, не поддерживаемая клиентская платформа и не Windows 11 приёмка.

| Workload | Создание, мс | Открытие с чтением, мс | Sampled peak, MiB |
|---|---:|---:|---:|
| docs | 5.099 | 8.506 | 14.633 |
| sheets | 191.272 | 255.466 | 73.496 |
| slides | 3.391 | 3.241 | 2.152 |
| history | 1650.145 | 2.286 | 77.242 |

Три реплики + local undo: 12.701 мс, convergence=true, local_undo=true. Один прогон, никакого p95/GUI fps из этих чисел не выводится. Sampled peak — Windows OS peak working set, опрашиваемый каждые 10 мс; короткий финальный пик мог быть пропущен. Сопоставление с бюджетом полноценного редактора некорректно: здесь нет Qt, layout, картинок, calc и render.

[Raw JSON со стендом и provenance](results/windows-ci-run.json). Все ограничения предыдущего раздела сохраняются; go/no-go для продукта не вынесен.
