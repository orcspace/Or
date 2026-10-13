# Зависимости и лицензии

Production core сейчас **без внешних Cargo dependencies**. Включены Inter/JetBrains Mono и Lucide assets; Loro и его зависимости используются только изолированным CRDT-экспериментом. Qt/CXX-Qt/Skia и остальные ниже — кандидаты, не установлены. Отсутствие dependency в production Cargo.lock не освобождает packages/fonts/media от проверки.

| Компонент | Pin / состояние | Лицензия/доказательство | Допуск |
|---|---|---|---|
| Rust toolchain |1.98.1, release2026-09-03| toolchain distribution licenses ещё собрать отдельно | build tool, не bundled runtime dependency |
| actions/checkout |fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09|MIT, [точный LICENSE](https://github.com/actions/checkout/blob/fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09/LICENSE), проверено2026-10-12|CI only; embedded action transitive packages upstream, runtime не поставляются |
| Qt6.12 |кандидат6.12.0, age gate|LGPLv3 для подходящих частей, [official licensing](https://doc.qt.io/qt-6/licensing.html); каждый выбранный модуль и SBOM pending|нет до exact package/module/license audit и ADR-0002/0003 |
| Qt Graphs/Quick3D/CanvasPainter/HTTPServer/NetworkAuthorization и прочие GPL-only |не подключать|official GPL-only list (FACT-013)|запрещены current license policy |
| Qt Charts/PDF/WebEngine/Multimedia/SVG/PrintSupport |не подключены|per-module exact6.12 check pending; WebEngine запрещён архитектурно|не подтверждено |
| CXX-Qt/cxx/cxx-qt-lib/build |candidate0.10.0; transitive pins pending|LICENSE exact crates ещё проверить|не подключено |
| Loro/Yrs |Loro1.16.2 в isolated spike; Yrs pending|LoroMIT, crate/LICENSE прочитан;156 declared licenses в dependencies/crdt-spike.md; source notices transitive pending|не production graph |
| IronCalc |release0.8 fact; crate pin pending|LICENSE/source + transitive pending|не подключено |
| skia-safe/Skia/HarfBuzz |pending|source licenses/notices/SBOM pending|не подключено |
| quick-xml/zip/tokio/rusqlite/tracing/rmcp/axum |pending|per-crate LICENSE + transitive Cargo tree pending|не подключено |
| PostgreSQL/S3 SDK/SQLite amalgamation |pending|specific versions/deployment licenses pending|не подключено |
| LibreOffice |on-demand candidate|distribution/source/license audit pending|не скачивается |
| Sparkle/WinSparkle/MSIX tooling |ADR choice pending|exact releases licenses/terms pending|не подключено |
| Lucide/Inter/JetBrains Mono |1.48.0 /4.1 /2.304|ISC/OFL-1.1, exact notices в assets; manifests/SHA256 verified|36 SVG и8 TTF включены; Qt integration pending |
| Carlito/Caladea/Liberation |pending|OFL/embedding rights должны проверяться по actual files|не включены |
| ffmpeg |P2 separate process|лицензия зависит от build flags; exact binary/config pending|не включён |

AGPL запрещён. Dynamic Qt DLL должны оставаться заменяемыми; THIRD-PARTY-NOTICES, GPL/LGPL texts, source offer/source access, EULA/reverse-engineering clause и installer information проверяются до релиза владельцем/юристом. Здесь не делаются юридические выводы о достаточности мер.


## Дополнения2026-10-13

- Loro1.16.2:MIT; original crate/LICENSE прочитан, crateSHA256 сверена с официальным registry. Все156experimental pins с declared SPDX/sourceURLs — [полный реестр](dependencies/crdt-spike.md), [machine data](dependencies/crdt-spike.json). НетAGPL, checksums/stability/age validated. Source notices/правовой анализ всех транзитивных компонентов ещё не завершён; production library graph не принят автоматически.
- Inter4.1:OFL-1.1, copyright2016TheInterProjectAuthors; exact official [release](https://github.com/rsms/inter/releases/tag/v4.1).8font-file bundle включает4Inter и4JetBrainsMono; fonts не изменены.
- JetBrainsMono2.304:OFL-1.1,copyright2020JetBrainsMonoProjectAuthors; exact [release](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304).OFL notices включены рядом с fonts.
- Lucide1.48.0:ISC,commitf53e5bfff0f909f3f451933538744330650d3ce0,[release](https://github.com/lucide-icons/lucide/releases/tag/1.48.0).36SVGs modified only stroke-width→1.75/XML normalization; LICENSE retained. Source and derivative SHA256 вmanifest.
- Font/icon assets уже включены вsource tree, но Qt font registration/native render ещё не реализованы. Предыдущая таблица описывает состояние первого прохода12октября, обновления перечислены здесь.
