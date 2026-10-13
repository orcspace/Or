# Зависимости и лицензии

Runtime core сейчас **без внешних Cargo dependencies**, внешние assets не включены. Qt/CXX-Qt/Skia и остальные ниже — кандидаты стека, не установлены. Отсутствие dependency в Cargo.lock не освобождает будущие packages/fonts/media от проверки.

| Компонент | Pin / состояние | Лицензия/доказательство | Допуск |
|---|---|---|---|
| Rust toolchain |1.98.1, release2026-09-03| toolchain distribution licenses ещё собрать отдельно | build tool, не bundled runtime dependency |
| actions/checkout |fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09|MIT, [точный LICENSE](https://github.com/actions/checkout/blob/fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09/LICENSE), проверено2026-10-12|CI only; embedded action transitive packages upstream, runtime не поставляются |
| Qt6.12 |кандидат6.12.0, age gate|LGPLv3 для подходящих частей, [official licensing](https://doc.qt.io/qt-6/licensing.html); каждый выбранный модуль и SBOM pending|нет до exact package/module/license audit и ADR-0002/0003 |
| Qt Graphs/Quick3D/CanvasPainter/HTTPServer/NetworkAuthorization и прочие GPL-only |не подключать|official GPL-only list (FACT-013)|запрещены current license policy |
| Qt Charts/PDF/WebEngine/Multimedia/SVG/PrintSupport |не подключены|per-module exact6.12 check pending; WebEngine запрещён архитектурно|не подтверждено |
| CXX-Qt/cxx/cxx-qt-lib/build |candidate0.10.0; transitive pins pending|LICENSE exact crates ещё проверить|не подключено |
| Loro/Yrs |pending exact versions|LICENSE crates + transitive pending|не подключено |
| IronCalc |release0.8 fact; crate pin pending|LICENSE/source + transitive pending|не подключено |
| skia-safe/Skia/HarfBuzz |pending|source licenses/notices/SBOM pending|не подключено |
| quick-xml/zip/tokio/rusqlite/tracing/rmcp/axum |pending|per-crate LICENSE + transitive Cargo tree pending|не подключено |
| PostgreSQL/S3 SDK/SQLite amalgamation |pending|specific versions/deployment licenses pending|не подключено |
| LibreOffice |on-demand candidate|distribution/source/license audit pending|не скачивается |
| Sparkle/WinSparkle/MSIX tooling |ADR choice pending|exact releases licenses/terms pending|не подключено |
| Lucide/Inter/JetBrains Mono |задание: ISC/OFL; exact pin не выбран|license files ещё не читаны|assets не включены |
| Carlito/Caladea/Liberation |pending|OFL/embedding rights должны проверяться по actual files|не включены |
| ffmpeg |P2 separate process|лицензия зависит от build flags; exact binary/config pending|не включён |

AGPL запрещён. Dynamic Qt DLL должны оставаться заменяемыми; THIRD-PARTY-NOTICES, GPL/LGPL texts, source offer/source access, EULA/reverse-engineering clause и installer information проверяются до релиза владельцем/юристом. Здесь не делаются юридические выводы о достаточности мер.

