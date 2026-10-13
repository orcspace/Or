# Проверяемые факты

Дата из среды: **2026-10-12 UTC**, у владельца **2026-10-13 Asia/Bishkek**. Проверенные факты ниже относятся к первому проходу. Полный аудит всех API/лицензий не завершён. «?» означает отсутствие подтверждения, не отрицательный результат.

| ID | Утверждение | Статус | Источник и результат | Проверено UTC |
|---|---|---|---|---|
| FACT-001 | Qt 6.12.0 LTS выпущен 2026-09-30 | ✔ подтверждено | [релиз](https://www.qt.io/blog/qt-6.12-released), [versions](https://doc.qt.io/qt-6/qt-releases.html) | 2026-10-12 |
| FACT-002 | Qt 6.12 target macOS 14.4+; Xcode16/macOS15 SDK+ | ✔ подтверждено | [Supported Platforms](https://doc.qt.io/qt-6/supported-platforms.html); приложение только arm64 | 2026-10-12 |
| FACT-003 | CXX-Qt latest stable 0.10.0, 2026-08-24 | ✔ подтверждено | [crates metadata](https://docs.rs/crate/cxx-qt/latest), [release](https://github.com/KDAB/cxx-qt/releases/tag/v0.10.0); совместимость 6.12 этим не доказана | 2026-10-12 |
| FACT-004 | QML property bindings — JavaScript expressions | ✔ подтверждено | [Qt binding syntax](https://doc.qt.io/qt-6/qtqml-syntax-propertybinding.html), даже reference-to-property | 2026-10-12 |
| FACT-005 | DWMSBT documented minimum — Windows11 build22621 | ✔ подтверждено | [Microsoft enum](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type); конфликт с22000 | 2026-10-12 |
| FACT-006 | «Текущий MCP 2025-11-25» устарело; latest→2026-07-28 | ✘ неверно (исправлено) | [latest](https://modelcontextprotocol.io/specification/latest) перенаправляет в2026-07-28; Tasks opt-in extension | 2026-10-12 |
| FACT-007 | drive.file — non-sensitive, per-file, basic verification | ✔ подтверждено | [официальные scopes](https://developers.google.com/workspace/drive/api/guides/api-specific-auth); не означает «вообще нет верификации» | 2026-10-12 |
| FACT-008 | Drive files.export ограничен10MB | ✔ подтверждено | [files.export](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/export); речь о exported byte content | 2026-10-12 |
| FACT-009 | Shallow Loro удаляет историю; peers до границы не синхронизируются обычно | ✔ подтверждено | [Loro shallow](https://www.loro.dev/docs/concepts/shallow_snapshots); нужен rebootstrap/offline policy, Rust pin ещё не выбран | 2026-10-12 |
| FACT-010 | IronCalc0.8 объявляет dynamic arrays/LET/LAMBDA | ✔ подтверждено | [release notes](https://blog.ironcalc.com/2026/08/01/Changelog-v0.8.html); это не proof полного Excel паритета/100k speed | 2026-10-12 |
| FACT-011 | Rust1.98.1 выпущен2026-09-03,1.99.0 —2026-10-01 | ✔ подтверждено | [release index](https://blog.rust-lang.org/releases/), [notes](https://doc.rust-lang.org/releases.html); выбрана1.98.1 по14-day policy | 2026-10-12 |
| FACT-012 | GitHub windows-2025 и arm Windows11 runners отличаются | ✔ подтверждено | [runner table](https://docs.github.com/en/actions/reference/runners/github-hosted-runners); Win11x64 interactive acceptance не подменять core hosted CI | 2026-10-12 |
| FACT-013 | Qt Quick3D/Graphs/CanvasPainter/HTTPServer/NetworkAuthorization GPL-only open-source | ✔ подтверждено | [Qt licensing](https://doc.qt.io/qt-6/licensing.html); module licenses/transitive notices каждого разрешённого ещё читать | 2026-10-12 |
| FACT-014 | LGPL dynamic replacement/notice obligations существуют | ✔ подтверждено | [Qt LGPL text](https://doc.qt.io/qt-6/lgpl.html); compliance окончательно владельцу/юристу | 2026-10-12 |
| FACT-015 | Excel sheet:1048576rows×16384cols, cell32767chars, precision15digits | ✔ подтверждено | [официальные limits](https://support.microsoft.com/en-us/excel/excel-specifications-and-limits); это целевая совместимость, не измеренный capacity приложения | 2026-10-12 |
| FACT-016 | Snap custom titlebar требует native maximize hit testing | ✔ подтверждено | [Microsoft Snap](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-snap-layout-menu); Qt implementation ещё не испытана | 2026-10-12 |
| FACT-017 | Word имеет side-by-side/synchronous scrolling и alt text | ✔ подтверждено | [view](https://support.microsoft.com/en-us/word/view-and-compare-documents-side-by-side), [a11y](https://support.microsoft.com/en-us/accessibility/word/make-your-word-documents-accessible-to-people-with-disabilities); добавлено в parity | 2026-10-12 |
| FACT-018 | Checkout pin fbc6f… MIT, commit2026-07-16 | ✔ подтверждено | [commit](https://github.com/actions/checkout/commit/fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09), [license](https://github.com/actions/checkout/blob/fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09/LICENSE); только CI, не runtime | 2026-10-12 |

## Ещё проверить (полный фактологический аудит остаётся открытым)

| Позиция | Статус/причина | Как проверить до использования |
|---|---|---|
| Open-source Qt6.12 patches, module SVG/PrintSupport/Multimedia/PDF/WebEngine licenses, third-party SBOM | ? полный package/license проход не выполнен | exact6.12 source LICENSES + SBOM; WebEngine остаётся запрещён независимо от лицензии |
| CXX-Qt0.10/Qt6.12 и Rust1.98.1 actual build, CXX-Qt license | ? source/build audit не выполнен | exact crates/Cargo.lock/licenses + Windows spike4.7 |
| Loro current1.x, UndoManager/MovableList/API/format stability/Rust shallow | ? JS docs не заменяют Rust version-specific contract | crates.io metadata + Rust docs/source + spike4.2 |
| Yrs version/license/perf | ? резерв не исследован | pinned repo/crate/bench |
| IronCalc full function coverage/date semantics/xlsx fidelity/license/version pin | ? release claims не equivalence tests | official Microsoft function inventory vs pinned source + spike4.5 |
| skia-safe version/feature/build/size/licenses/transitive Skia/HarfBuzz | ? не исследовано | pinned Cargo metadata/features/licenses + Windows spike4.3/4.7 |
| rmcp latest/features/auth/subscriptions/2026-07-28 support | ? не исследовано | official Rust SDK release/source + spike4.6 |
| Claude/Codex/Cursor/VSCode/Windsurf configs and elicitation/tasks | ? candidate paths из задания не подтверждены | current official client docs + real installation test |
| Google quotas/desktop PKCE/loopback/Picker/publication | ? кроме scope/export limit не завершено | Google OAuth desktop and Drive quota docs; no WebView Picker embed |
| Authenticode/MSIX/TrustedSigning eligibility/pricing; Apple notarization/Store restrictions | ? аккаунты/условия не проверены | official docs under owner country/legal entity; no legal conclusion |
| Sparkle/WinSparkle versions/licenses/update signatures | ? не исследовано | release/LICENSE/security docs |
| QStyleHints, SMAppService, spell/OCR/TTS/QAccessible/QuickLook/preview handlers | ? не исследовано полностью | official Qt/Apple/Microsoft APIs + Windows spike4.4 |
| LO distribution/licenses/headless flags/size/signature verification/download | ? не исследовано | official exact-release artifacts/LICENSE + worker smoke test |
| Font/Lucide licenses and metric equality | ? имена лицензий в задании не proof | per-version LICENSE/OFL/embedding metadata + glyph/layout tests |
| OOXML specs/MS-OI29500/supported macro preservation/encryption | ? сопоставление не сделано | official standards + fixtures; unknown-part byte preservation tests |
| Brand/domains/extensions/trademark classes | ? поиск не выполнен | domain/registry/search facts, legal owner review |
| AI consumer vs API billing/data retention provider-specific | ? не исследовано | official API pricing/terms/retention for selected provider |
| Full Office ribbon parity | ? первый проход лишь частичный | Microsoft per-tab features, feature-by-feature inventory; никакого «полного паритета» пока |

Если URL не прочитан, он не становится доказательством. До подключения зависимости необходим exact pin и проверенный license entry; дата повторной проверки ≤30 дней для зависимой фазы.


## Повторная проверка2026-10-13 UTC

| ID | Утверждение | Статус | Источник |
|---|---|---|---|
| FACT-019 | Loro1.16.2 release2026-09-21,MIT; Rust insert/mark positions вUnicode scalars, separate UTF8/UTF16 APIs |✔подтверждено|[exact crate](https://crates.io/api/v1/crates/loro/1.16.2), downloaded source+LICENSE+checksum; Rust smoke tests passed|
| FACT-020 | Inter latest4.1 release2024-11-16, OFL-1.1; exactTTF family Inter |✔подтверждено|[release](https://github.com/rsms/inter/releases/tag/v4.1), releaseZIP/license/name tables|
| FACT-021 | JetBrainsMono latest2.304 release2023-01-14,OFL-1.1; family JetBrainsMono |✔подтверждено|[release](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304), releaseZIP/OFL/name tables|
| FACT-022 | Lucide1.48.0 release2026-09-24 ISC; newer tags younger14days |✔подтверждено|[release](https://github.com/lucide-icons/lucide/releases/tag/1.48.0), exacttag/source/LICENSE|
| FACT-023 | Windows11 contrast colors include COLOR_3DFACE/BTNTEXT, WINDOW/WINDOWTEXT, HIGHLIGHT/HIGHLIGHTTEXT,HOTLIGHT |✔подтверждено|[GetSysColor supported contrast table](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsyscolor); same page marks aliasBTNFACE unsupported but table explicitly supports3DFACE, choose documented contrast table alias|

rmcp metadata current3.5.1 is younger14days; candidate3.5.0(2026-09-28) aged but features/protocol/client build not yet investigated. Ironcalc_base metadata0.8.3(2026-07-31) andskia-safe0.153.3(2026-09-04) eligible; not linked or claimed working. Primary metadata source: crates.io/api/v1/crates/{name}; read2026-10-13.
