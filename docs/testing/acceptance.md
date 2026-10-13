# План проверок и измерений

Все ниже — **методы**, не результаты. Нет performance цифр, полученных на целевых ОС. Pure core unit tests и проверка реестра не заменяют UI/MCP/import e2e.

| Test family | Фаза | Метод и проходной критерий |
|---|---|---|
| T-00-SPEC |0/all|Registry без duplicate/missing IDs; каждая source clause совпадает; phase+test route; generated docs не устарели; статус done требует evidence|
| T-00-POLICY |0|Core tests: Win10/11 x64 accepts build10240+, server/ARM/pre-Win10 reject; theme system/live preference/highcontrast; AA4.5 for ordinary text; reduced motion0|
| T-00-BUILD |0|Clean pinned Rust/egui static-CRT exe build; record exact host/version/arch/time/size; forbidden module check; age/license gate|
| T-00-UI |0|Light/dark/system screenshot+keyboard/Narrator/ru/en/DPI; title drag/max hit-test/Snap/corners/Mica; cold start to first Home pixels≤1.5s|
| T-01-CRDT |1,4|Random concurrent ops across3+replicas, reorder/reconnect/offline, same canonical state; local-only undo, agent separate groups; 1m history spike|
| T-01-LAYOUT |1,2|Licensed complex DOCX fixtures, LO baseline + Word manual subset; compare page count/glyph positions/overflow; tolerances require ADR-0015|
| T-01-INPUT |1|Automated100-page edits+physical input; timestamps native key event→presented frame, p95≤16ms; CJK composition/RTL edge cases; Narrator semantic tree|
| T-01-RECOVERY |1,2|Kill at every write stage, reopen journal; lost input≤1s; atomic saved file old/new valid, no corrupt middle state; disk full/permission loss|
| T-02-ROUNDTRIP |2,5,6|≥200 licensed real OOXML total, first50DOCX; semantic diff + opaque-part byte hashes + layout; fixture provenance/license mandatory|
| T-02-FILES |2|Atomic replace/backup/long path/locks/placeholders/watch races/three-way compare; failed save preserves original; save-copy doesn't switch path|
| T-02-PRINT |2,5,6|Same display-list output screen/PDF; text remains text; paper/DPI/page order/ranges/duplex/n-up/notes; real printer representative sample|
| T-03-MCP |3|Each tool schemas/readOnly/destructive/idempotency/rights/suggest/dry_run/version/errors/pagination/rate-limit/stop; rmcp shim/IPC/HTTP/inproc e2e|
| T-03-AI |3|Real provider+BYOK notification, context chips; agent live while user edits; partial tool JSON never applied; stop rolls back only unfinished agent step|
| T-04-SYNC |4|2 clients+AI concurrent/offline convergence; permissions revocation;50active participants, record RTT and server processing≤20ms|
| T-05-CALC |5|Official Excel function inventory + cases;1900/1904/leap/15digits/locales/cycles/volatile;100kformula update dependent≤50ms|
| T-05-GRID |5|100k cells, virtual scroll≥60fps with known formula density; clipboards/formats/freeze/sort/filter/chart; memory stats|
| T-06-SLIDES |6|100 slides, F5/ShiftF5 + second monitor presenter, align/group/master, media, anim60fps and p:timing roundtrip|
| T-07-LEGACY |7|Separate LO worker limits/timeout/cancel/network disabled; real DOC/XLS/PPT/ODF fixtures; failure survives UI|
| T-07-GOOGLE |7|drive.file constrained files; import/export conversion/repeatedfileId/10MBerror/401refresh/429retry/cancel; scope not widened|
| T-08-PARITY |8|Every P1 feature: UI/ops/MCP/docs/source-format fidelity; inherited global theme/a11y/i18n/performance checks|
| T-09-RELEASE |9|Clean install/update/rollback/uninstall/migration; Authenticode when certificate exists; NSIS per-user Windows10/11 x64; future Mac DeveloperID/notarization/Dmg|
| T-SECURITY |all|STRIDE mitigations tests: zip/path/reparse/XXE/OLE/DDE/macros/injection/token/origin/IPCUID/JWTasset role; no secret/content leaks|
| T-FACTS |all|Read official current versions/licenses/API paths, UTC date; ≤30d refresh for phase-critical facts|

## Стенды и воспроизводимость

Windows baseline: реальные Win10x64 и Win11x64, минимальный policy build10240; CPU/RAM/GPU/display/refresh/DPI exact values записать до замеров. M1 Mac baseline позже по ADR-0001. Hosted Server runner разрешён только как core/build infrastructure, не native UX proof.

Key-to-pixel: native event timestamp до actual presented frame, не до завершения ops. Отдельно breakdown queue/model/layout/raster/upload/present. Выборка минимум1000 правок после явно описанного warmup; p95 и distribution, результаты raw machine-readable. Cold start считать process spawn→first fully usable Home; cache/first install отдельно. Оценки до измерений помечать оценками.

Performance regression gate требует сохранённого baseline на сопоставимом стенде; >10% red. Пока baseline не существует — gate не выдаёт фиктивный green. Corpus/licensing/layout tolerance решение ADR-0015 до golden acceptance.

