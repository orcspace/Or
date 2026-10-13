# docs/SPEC-AUDIT.md — Дополнение к AGENTS.md и docs/MCP.md: ты аудируешь и чинишь спецификацию сам

Приоритет равен AGENTS.md и MCP.md. При расхождении действует это дополнение, **кроме жёстких ограничений раздела 1 AGENTS.md** (платформы, Electron/Tauri/WebView/TS/JS, стек, лицензии), которые меняет только владелец через ADR. Это включает и заменяет прежнее короткое дополнение из 11 пунктов, если оно было. <!-- REQ-A00-001 -->

## 0. Роль

Исполнитель — соавтор и аудитор. Спецификация содержит потенциально неверные/устаревшие факты, противоречия/дыры и нереалистичные места. Найти до дорогой реализации, проверить первоисточники, исправить и зафиксировать. Молчаливый обход и слепое выполнение замеченной ошибки запрещены. <!-- REQ-A00-002 -->

## 1. Порядок

1. Полностью читать AGENTS, MCP, этот файл. <!-- REQ-A01-001 -->
2. Каждому пункту AGENTS/MCP стабильный `REQ-<раздел>-<номер>`. `docs/traceability.md`: ID → описание → фаза → проверки → статус. Отсутствие фазы/проверки — дефект. Для двух источников раздел MCP получает префикс M (например `REQ-M03-004`), чтобы IDs не пересекались; это редакционное уточнение записывается в changelog. <!-- REQ-A01-002 -->
3. Аудит §3 → `docs/audit/00-spec-audit.md` по §8. <!-- REQ-A01-003 -->
4. Рискованные спайки §4 → `docs/spikes/NN-название.md`, цифры, go/no-go. <!-- REQ-A01-004 -->
5. Исправления по §6 → `docs/spec-changelog.md`. <!-- REQ-A01-005 -->
6. Владельцу отчёт ≤1 страницы: блокеры, решения владельца, исправления, непроверенное. <!-- REQ-A01-006 -->

Фаза 0 начинается параллельно после проверки AGENTS §§1–5/20; не ждёт весь аудит. **Фаза 1 не начинается без спайков вёрстки и CRDT.** Указание владельца 2026-10-13 позволяет пока работать/проверять Windows; macOS-замеры откладываются и никогда не выдаются за выполненные (ADR-0001). <!-- REQ-A01-007 -->

## 2. Проверка фактов

- Только свежие первоисточники: official docs/releases/repos/licenses, ECMA-376/ISO 29500/MS-OI29500/MCP/OAuth/Google. Дата из системы. <!-- REQ-A02-001 -->
- Версии, лицензии, лимиты, цены, config paths, platforms — никогда из памяти, читать. <!-- REQ-A02-002 -->
- `facts.md`: утверждение → статус → URL → дата. `✔ подтверждено`, `✘ неверно (исправлено)`, `? не удалось проверить` + причина, `➕ пробел`. <!-- REQ-A02-003 -->
- Конфликт: оба источника, причина доверия. <!-- REQ-A02-004 -->
- Юридические вопросы: факты, «передать владельцу/юристу», без юридических выводов. <!-- REQ-A02-005 -->
- Не выдумывать; допустимо «не знаю, вот как проверить». <!-- REQ-A02-006 -->

## 3. Аудит

### A. Фактические утверждения — каждое

- Qt: версия 6.12 LTS/open-source patches, Supported Platforms Windows/macOS; LGPL vs GPL-only/коммерческие Charts/Graphs/Quick 3D/Multimedia/Print Support/SVG/PDF/WebEngine и др.; dynamic linking. CXX-Qt latest/6.12 compatibility; QStyleHints, Mica, Snap custom Qt Quick title, SMAppService. <!-- REQ-A03-001 -->
- Rust: Loro version/API/format stability/shallow snapshot/UndoManager/marks/MovableTree/List/memory/performance, Yrs. IronCalc version/full Excel function list/dynamic arrays/LET/LAMBDA/xlsx round-trip. skia-safe targets/textlayout/pdf/size/build time. rmcp version/tools/outputSchema/resources/subscriptions/prompts/elicitation/tasks/progress/cancel/HTTP/auth. Stable MCP spec. <!-- REQ-A03-002 -->
- Clients: Claude/Codex/Cursor/VS Code/Windsurf actual config paths/formats, elicitation/tasks in each. <!-- REQ-A03-003 -->
- Google: drive.file scopes/verification, export size/quotas, Picker need, OAuth publication, desktop loopback+PKCE. <!-- REQ-A03-004 -->
- Distribution: MSIX signing/sideload, Azure Trusted Signing availability/conditions/alternatives, Apple notarization, Sparkle/WinSparkle versions/licenses, Store sandbox vs shim/LO/pipes/sockets. <!-- REQ-A03-005 -->
- System APIs: Windows spelling, NSSpellChecker, Windows.Media.Ocr, Vision, TTS, Quick Look/preview handler, QAccessible canvas. <!-- REQ-A03-006 -->
- Files/fonts: LO license/headless flags/distribution licenses/size/safe download; Carlito/Caladea/Liberation metric compatibility/licenses; Inter/JetBrains/Lucide licenses. <!-- REQ-A03-007 -->
- OOXML: ECMA-376/ISO and MS-OI29500 departures. <!-- REQ-A03-008 -->

### B. Согласованность

- AGENTS vs MCP tools/formats/rights/links. <!-- REQ-A03-009 -->
- Каждая AGENTS §11 функция: фаза, parity, tests, MCP. Всё UI доступно через ops/tool и наоборот. <!-- REQ-A03-010 -->
- Измеримые/reproducible phase acceptance/CI. <!-- REQ-A03-011 -->
- Dependencies: export/print need layout/render. <!-- REQ-A03-012 -->
- Совместная достижимость нефункциональных целей. <!-- REQ-A03-013 -->

### C. Известные пробелы — каждый получает факты/ADR

**[ВЛАДЕЛЕЦ]**: варианты, плюсы/минусы, стоимость, рекомендация, статус «ждёт владельца»; продолжать независимую работу. <!-- REQ-A03-014 -->

1. v1: предложение Docs P0 → Sheets P0 → MCP/AI → collaboration → export/print; Slides/animations/Google/P1 позже. **[ВЛАДЕЛЕЦ]** Slides и следующие только после подтверждения. <!-- REQ-A03-015 -->
2. Бесплатно/subscription/open source → Qt license/Store/server cost. **[ВЛАДЕЛЕЦ]** <!-- REQ-A03-016 -->
3. SkParagraph только paragraph; pagination/wrapping/footnotes/columns/table split — наш Word-layer. Спайк 4.1 до решения. <!-- REQ-A03-017 -->
4. IronCalc gaps: fork/custom/alternative; CalcEngine trait сохраняется. <!-- REQ-A03-018 -->
5. OOXML/LO child process, memory/time limits, termination; corrupt file не валит app. <!-- REQ-A03-019 -->
6. External file three-way merge local/disk/base с compare UI. <!-- REQ-A03-020 -->
7. Корпус GovDocs1/LO/POI/OSS fixtures: проверить каждую лицензию. LibreOffice автоматический layout baseline + manual Word sample, px/pt tolerance/metric. <!-- REQ-A03-021 -->
8. CRDT history compression/shallow thresholds adjustable; time machine/undo impact ADR. <!-- REQ-A03-022 -->
9. Server email/invites/@, account/data deletion, GDPR и аналоги, monitoring/alerts/backup/restore/cost N, E2EE choice (search/preview/AI), abuse. <!-- REQ-A03-023 -->
10. Coauthor missing fonts: fallback/embedding/cloud with licenses ADR. <!-- REQ-A03-024 -->
11. AI default BYOK, API vs consumer subscription billing проверять и честно UI. Proxy/quotas — отдельный ADR **[ВЛАДЕЛЕЦ]**, не самовольно. <!-- REQ-A03-025 -->
12. Windows ARM64: сначала x64 emulation; Qt/Skia/pipes/IME test. Native ADR **[ВЛАДЕЛЕЦ]**. <!-- REQ-A03-026 -->
13. Explorer/Finder previews Quick Look/handler P1 parity. <!-- REQ-A03-027 -->
14. TrineDocs similar names/domains/document companies/trademark classes facts, без legal conclusion. Name/extensions **[ВЛАДЕЛЕЦ]** ADR и conflicts. <!-- REQ-A03-028 -->
15. Uninstall: MCP configs/startup/keychain/cache; update signatures/rollback; file/database migrations/versioning/forward-backward compatibility. <!-- REQ-A03-029 -->
16. EULA/privacy/About/THIRD-PARTY-NOTICES/opt-in/content retention by AI. <!-- REQ-A03-030 -->
17. Max rows/cols against Excel, pages/slides/file size, overflow/memory. <!-- REQ-A03-031 -->
18. Excel dates 1900/1904/leap-bug; 15 digits; circular refs; locale arg separator; recalc order; volatile functions. <!-- REQ-A03-032 -->
19. Word docDefaults/latent styles/abstractNum/num/fields/w:compat/version-sensitive layout. <!-- REQ-A03-033 -->
20. Clipboard read/write formats each OS, Word/Excel/PowerPoint/browser, drag-and-drop. <!-- REQ-A03-034 -->

### D. Самостоятельный поиск

- End-to-end сценарии: first launch; file association; edit/save; crash/recover; huge file; print; Google export; Claude connection; invite; offline/reconnect; update; uninstall. <!-- REQ-A03-035 -->
- Сверка Office ribbons по официальным Microsoft docs: Home/Insert/Layout/References/Mailings/Review/View/Formulas/Data/Design/Animations/Slide Show. Missing → parity with priority. <!-- REQ-A03-036 -->
- STRIDE MCP IPC/HTTP, import, sync → `docs/security/threat-model.md`; unmitigated → audit. <!-- REQ-A03-037 -->
- Experienced Word/Excel red team собственной parity. <!-- REQ-A03-038 -->

## 4. Обязательные спайки

Ограниченное время, метод, **обе ОС**, measured numbers, go/no-go/change. При провале числа и ADR альтернативы. Windows-first разрешает отложить Mac результаты, не исключает их из итогового DoD. <!-- REQ-A04-001 -->

### 4.1 Вёрстка (блокирует фазу 1)

Complex real docx: multipage tables/floating images/columns/footnotes/headers/lists/styles; compare LO metric. (a) own on SkParagraph; (b) mature OSS Rust text/grid/block components, licenses; (c) LO MPL layout engine feasibility vs architecture/perf. Criteria: accuracy, ≤16ms incremental paragraph edit, effort/license/maintenance. Не решать без данных. <!-- REQ-A04-002 -->

### 4.2 CRDT

Loro 100 pages rich text/100k cells/100 slides; memory/open/merge/history 1m ops/undo/shallow. If fails compare Yrs. <!-- REQ-A04-003 -->

### 4.3 Render/Qt

Skia tiles→Qt CPU texture upload vs GPU RHI Metal/D3D11/12: latency/CPU/GPU/complexity/ProMotion/scaling/60-120fps. <!-- REQ-A04-004 -->

### 4.4 Input/accessibility

CJK IME composition/selection/drag/mac magnifier; QAccessible tree Narrator/VoiceOver viable quality. <!-- REQ-A04-005 -->

### 4.5 Formulas

IronCalc vs full official Excel list/test corpus: coverage/correctness/100k perf/incremental, plan B. <!-- REQ-A04-006 -->

### 4.6 MCP e2e

rmcp one edit: stdio shim→IPC→app; shim startup; Claude/Codex; actually test elicitation/tasks. <!-- REQ-A04-007 -->

### 4.7 Build/distribution

Clean Qt+CXX-Qt+Skia both CI targets: time/size/cache, signed/notarized empty artifact, clean install/startup. <!-- REQ-A04-008 -->

## 5. ADR

Architecture/scope/cost/license/security → `docs/decisions/NNNN-название.md`: context/options/criteria/data(links)/decision/consequences/status proposed/accepted/await owner/superseded. <!-- REQ-A05-001 -->

Owner decisions not self-approved: comparison/recommendation/reason/blockers, review in 5min. Own decisions accept+record+proceed; no barrage of minor questions. <!-- REQ-A05-002 -->

## 6. Исправления

- Wrong low-risk fact → AGENTS/MCP edit + changelog what/why/source/date. <!-- REQ-A06-001 -->
- High-risk → ADR+owner, mark ⚠ ADR in spec until decision. <!-- REQ-A06-002 -->
- Hard targets/platform/bans/mandatory features never silently weaken; prove issue with data, owner alone can relax/delete. <!-- REQ-A06-003 -->
- Never delete requirement to fix defect; no fake stub functionality. <!-- REQ-A06-004 -->
- Errors in this file corrected same way. <!-- REQ-A06-005 -->

## 7. Постоянно по фазам

Start: reread relevant sections, recheck >30d facts and phase-critical facts; mini-audit in phase plan. <!-- REQ-A07-001 -->

During: all deviations in changelog. **Definition of Done**: code; unit and necessary property/golden; both themes; both OS; perf budget; keyboard/screen reader; ru/en; MCP tool+test; parity+traceability; docs. Owner Windows-first permits provisional Windows acceptance only; global DoD remains unfinished. <!-- REQ-A07-002 -->

End `docs/audit/phase-N.md`: measured vs targets; all requirements done/not/partial with reasons; new risks; red team corrupt files/races/network/crash/MCP revocation; owner needs. <!-- REQ-A07-003 -->

## 8. Отчёт

| ID | Раздел / REQ | Утверждение или пробел | Статус | Доказательство (URL, дата) | Серьёзность | Действие |
|---|---|---|---|---|---|---|

Start: status/severity counts, top-5 risks. Severity blocker/high/medium/low; action fixed(changelog N)/ADR/owner/spike/justified accept risk. <!-- REQ-A08-001 -->

## 9. Честность

- Report failures, missed target/unreproduced/spike fail. <!-- REQ-A09-001 -->
- No done for stubs/single OS/no tests. <!-- REQ-A09-002 -->
- Numbers measured with bench; estimates labelled estimates. <!-- REQ-A09-003 -->
- No flattery/blind agreement; argue poor/costly choices with evidence/alternatives. <!-- REQ-A09-004 -->
- Reports/ADR/docs Russian; code/identifiers/commits English. <!-- REQ-A09-005 -->
