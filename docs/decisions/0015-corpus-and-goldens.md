# ADR-0015: Корпус и эталоны

Статус: **предложено; лицензии источников не проверены**. Дата:2026-10-12UTC.

## Контекст

GovDocs1/bug attachments/OSS fixtures нельзя автоматически считать redistributable. LibreOffice output — ориентир, не абсолютный Word oracle.

## Варианты и рекомендация

A: собственные синтетические fixtures+licensed real corpus; B: скачать всё без provenance (отвергнуто); C: private licensed corpus без redistributing. Рекомендация A + разрешённый manual Word subset.

## Критерии, данные и последствия

Для каждого fixture sourceURL/license/permission/hash/Officeversion/expected unsupported parts. Metrics pagecount/overflows/glyph offsets/image geometry plus pixel diff masked anti-aliasing; numeric tolerances после spike4.1, не выдуманы.≥200OOXML, first50DOCX.

Замеры/числовые затраты пока отсутствуют. Это не принятое ослабление требований и не результат выполненного спайка; независимую фазу0 продолжать.
