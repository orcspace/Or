# Первичный аудит спецификации

**Статус: первый проход, полный фактологический аудит не завершён.** Реального GUI/EXE, спайков и performance results пока нет. Mac-проверки отложены владельцем.

Позиций: **36**. Статусы: ✔ подтверждено: 3; ➕ противоречие: 3; ✘ неверно (исправлено): 1; ? не удалось проверить совместимость: 1; ? не удалось проверить полностью: 2; ? не удалось проверить: 1; ➕ риск: 1; ➕ пробел: 2; ? нет результатов: 1; ➕ незакрытый риск: 1; ➕ пробел разобран, не реализован: 20.

Серьёзность: блокер: 4; низкая: 1; высокая: 19; средняя: 12.

## Топ-5 рисков

1. QML/JS hard-ban contradiction (owner).
2. Qt6.12 age gate (обход запрещён).
3. Mica22621 vs platform22000 (owner).
4. Word page layout и CRDT масштаб/history без обязательных spike data; блок phase1.
5. MCP2026 revision/SDK/client gaps, permissions/Unicode/atomic side-effects.

| ID | Раздел / REQ | Утверждение или пробел | Статус | Доказательство (URL, дата) | Серьёзность | Действие |
|---|---|---|---|---|---|---|
| AUD-001 | REQ-01-005 | Qt6.12.0 ещё младше14суток | ✔ подтверждено | [FACT-001](https://www.qt.io/blog/qt-6.12-released), official Qt release2026-09-30, проверка2026-10-12 | блокер | ADR-0002, ожидание age gate |
| AUD-002 | REQ-01-003 | QML expressions требуют JS при абсолютном запрете | ➕ противоречие | [FACT-004](https://doc.qt.io/qt-6/qtqml-syntax-propertybinding.html) official Qt bindings,2026-10-12 | блокер | ADR-0003, ждёт владельца |
| AUD-003 | REQ-05-010 | Mica documented22621 vs supported22000 | ➕ противоречие | [FACT-005](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type) Microsoft DWM enum,2026-10-12 | блокер | ADR-0007, ждёт владельца |
| AUD-004 | REQ-01-001 | macOSminimum14.4 заполнен по Supported Platforms | ✔ подтверждено | [FACT-002](https://doc.qt.io/qt-6/supported-platforms.html) Qt platforms,2026-10-12 | низкая | исправлено, CHANGE-003/ADR-0004 |
| AUD-005 | REQ-M00-002 | Текущая MCP версия не2025-11-25 | ✘ неверно (исправлено) | [FACT-006](https://modelcontextprotocol.io/specification/latest) latest redirects2026-07-28,2026-10-12 | высокая | CHANGE-004; SDK support spike4.6 |
| AUD-006 | REQ-01-004 | CXXQtlatest0.10.0 подтверждён, Qt6.12 compatibility неизвестна | ? не удалось проверить совместимость | [FACT-003](https://docs.rs/crate/cxx-qt/latest); real build отсутствует | высокая | спайк4.7; не подключать как проверенное |
| AUD-007 | REQ-01-006 | Per-module/transitive Qt license audit не завершён | ? не удалось проверить полностью | [FACT-013](https://doc.qt.io/qt-6/licensing.html)/014; official list не полный SBOM | высокая | licenses.md/ADR-0008, exact source audit |
| AUD-008 | REQ-M00-002 | rmcp/clients config/tasks/elicitation актуальный контракт не проверен | ? не удалось проверить | facts.md pending inventory, installed clients недоступны | высокая | спайк4.6/ADR-0010 |
| AUD-009 | REQ-09-001 | Google drive.file non-sensitive, basic verification; exports10MB | ✔ подтверждено | [FACT-007](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)/008, Google official docs,2026-10-12 | средняя | facts; OAuth publication details ещё проверить |
| AUD-010 | REQ-03-022 | Shallow history ограничивает merge старых peers/time machine | ➕ риск | [FACT-009](https://www.loro.dev/docs/concepts/shallow_snapshots) Loro docs,2026-10-12 | высокая | ADR-0011/спайк4.2 |
| AUD-011 | REQ-M06-072 | apply_ops atomic любых типов vs printer/network side effects | ➕ противоречие | внутренний contract analysis MCP§6.9,2026-10-12 | высокая | ADR-0010, не снижать атомарность молча |
| AUD-012 | REQ-M05-004 | create ID/Unicode unit/suggest concurrent stop/rights scope не специфицированы | ➕ пробел | MCP§4/5/6 internal analysis,2026-10-12 | высокая | ADR-0010, schemas до реализации |
| AUD-013 | REQ-19-006 | Hosted core checks не Win11x64 UI acceptance | ➕ пробел | [FACT-012](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) + текущая средаLinux/no Qt/Rust | средняя | отдельный real desktop bench в plan0 |
| AUD-014 | REQ-A01-007 | Обязательные layout/CRDT spikes не выполнены | ? нет результатов | spikes01/02: NOT_RUN, текущая среда не target OS | блокер | не стартовать phase1; windows experiment после gates |
| AUD-015 | REQ-A03-037 | Все18STRIDE mitigations пока design, не protection | ➕ незакрытый риск | security/threat-model.md; runtime modules отсутствуют | высокая | T-SECURITY при реализации; без ложного closed |
| AUD-016 | REQ-A03-036 | Полная ribbon parity сверка ещё не выполнена | ? не удалось проверить полностью | [FACT-017](https://support.microsoft.com/en-us/word/view-and-compare-documents-side-by-side) only subset; остальные official tabs pending | средняя | продолжить inventory,350PAR не full parity claim |
| AUD-017 | REQ-A03-015 | C.1: v1: предложение Docs P0 → Sheets P0 → MCP/AI → collaboration → export/print; Slides/animations/Google/P1 позже. **[ВЛАДЕЛЕЦ]** Slides и следующие только после подтверждения. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0005 / ждёт владельца |
| AUD-018 | REQ-A03-016 | C.2: Бесплатно/subscription/open source → Qt license/Store/server cost. **[ВЛАДЕЛЕЦ]** | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0008 / ждёт владельца |
| AUD-019 | REQ-A03-017 | C.3: SkParagraph только paragraph; pagination/wrapping/footnotes/columns/table split — наш Word-layer. Спайк 4.1 до решения. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0012 / проверка/спайк pending |
| AUD-020 | REQ-A03-018 | C.4: IronCalc gaps: fork/custom/alternative; CalcEngine trait сохраняется. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0012 / проверка/спайк pending |
| AUD-021 | REQ-A03-019 | C.5: OOXML/LO child process, memory/time limits, termination; corrupt file не валит app. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0006 / проверка/спайк pending |
| AUD-022 | REQ-A03-020 | C.6: External file three-way merge local/disk/base с compare UI. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0021 / проверка/спайк pending |
| AUD-023 | REQ-A03-021 | C.7: Корпус GovDocs1/LO/POI/OSS fixtures: проверить каждую лицензию. LibreOffice автоматический layout baseline + manual Word sample, px/pt tolerance/metric. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0015 / проверка/спайк pending |
| AUD-024 | REQ-A03-022 | C.8: CRDT history compression/shallow thresholds adjustable; time machine/undo impact ADR. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0011 / проверка/спайк pending |
| AUD-025 | REQ-A03-023 | C.9: Server email/invites/@, account/data deletion, GDPR и аналоги, monitoring/alerts/backup/restore/cost N, E2EE choice (search/preview/AI), abuse. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0016 / проверка/спайк pending |
| AUD-026 | REQ-A03-024 | C.10: Coauthor missing fonts: fallback/embedding/cloud with licenses ADR. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0013 / проверка/спайк pending |
| AUD-027 | REQ-A03-025 | C.11: AI default BYOK, API vs consumer subscription billing проверять и честно UI. Proxy/quotas — отдельный ADR **[ВЛАДЕЛЕЦ]**, не самовольно. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0020 / ждёт владельца |
| AUD-028 | REQ-A03-026 | C.12: Windows ARM64: сначала x64 emulation; Qt/Skia/pipes/IME test. Native ADR **[ВЛАДЕЛЕЦ]**. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0022 / ждёт владельца |
| AUD-029 | REQ-A03-027 | C.13: Explorer/Finder previews Quick Look/handler P1 parity. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0023 / проверка/спайк pending |
| AUD-030 | REQ-A03-028 | C.14: TrineDocs similar names/domains/document companies/trademark classes facts, без legal conclusion. Name/extensions **[ВЛАДЕЛЕЦ]** ADR и conflicts. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0009 / ждёт владельца |
| AUD-031 | REQ-A03-029 | C.15: Uninstall: MCP configs/startup/keychain/cache; update signatures/rollback; file/database migrations/versioning/forward-backward compatibility. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0014 / проверка/спайк pending |
| AUD-032 | REQ-A03-030 | C.16: EULA/privacy/About/THIRD-PARTY-NOTICES/opt-in/content retention by AI. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0008/0020 / проверка/спайк pending |
| AUD-033 | REQ-A03-031 | C.17: Max rows/cols against Excel, pages/slides/file size, overflow/memory. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | средняя | ADR-0018 / проверка/спайк pending |
| AUD-034 | REQ-A03-032 | C.18: Excel dates 1900/1904/leap-bug; 15 digits; circular refs; locale arg separator; recalc order; volatile functions. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0018 / проверка/спайк pending |
| AUD-035 | REQ-A03-033 | C.19: Word docDefaults/latent styles/abstractNum/num/fields/w:compat/version-sensitive layout. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0019 / проверка/спайк pending |
| AUD-036 | REQ-A03-034 | C.20: Clipboard read/write formats each OS, Word/Excel/PowerPoint/browser, drag-and-drop. | ➕ пробел разобран, не реализован | SPEC-AUDIT§3.C + linked ADR; data/price/license checks указаны как pending,2026-10-12 | высокая | ADR-0017 / проверка/спайк pending |

Ссылки на первоисточники/дату раскрыты в [facts.md](../facts.md); FACT ссылка не заменяет реальные spike measurements. Claims, не проверенные в полном объёме, явно остаются «?». Недостающие tool contracts — [mcp-gaps.md](../mcp-gaps.md). Сведения о юридических/ценовых вопросах передать владельцу/юристу, не трактовать этот отчёт как заключение.

## Проверка сценариев (первый проход)

| Сценарий | Выявленный недостающий контракт | Проверка |
|---|---|---|
| Первый запуск | OS rejection, theme/locale/a11y, optional AI/privacy согласие |T-00-UI|
| Двойной клик DOCX | process reuse/IPC/file association, read-only/locks, loading worker errors |T-02-FILES|
| Правка/save/crash | common base/sidecar IDs/atomic journal/disk-full/migration |T-01-RECOVERY|
| Огромный hostile файл | bounded worker/preview/progressive import/resource UI |T-SECURITY|
| Печать | irreversible confirmation vs apply_ops, printer capabilities |T-02-PRINT|
| Google | basic verification/10MBerror/Picker without WebView/scope constraints |T-07-GOOGLE|
| MCP client | backup/diff/owner consent/config migration/shim startup/role revoke |T-03-MCP|
| Invite/offline/reconnect | email/S3 role auth/old peer after shallow/permissions revoked offline |T-04-SYNC|
| Update/uninstall | rollback/formats/owned config only/no unexpected data delete |T-09-RELEASE|

## Следующий проход

Разрешить блокеры окна; проверить exact Qt/CXX packages и licenses на Windows; выполнить core CI. Затем exhaustive official fact/ribbon audit, layout/CRDT experimental data. Изменение v1 order/distribution/name/E2EE только владельцем. Независимые spec/core работы продолжаются.
