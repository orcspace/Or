# План завершающей фазы: предварительная Windows сборка

Дата: 2026-10-13. Репозиторий orcspace/Or. Это план проверяемой preview сборки, не объявление полного Office-паритета и не приёмка стабильного v0.1.0.

## Исходные подтверждённые факты

- Две независимые волны ревью сохранены в docs/audit/review-1-*.md и review-2-*.md; дополнительно проверены NSIS/release workflow/test harness.
- По сообщению root после последних PNG/API/preflight fixes: 112 app +14 shell =126 тестов PASS; fmt и clippy -D warnings PASS.
- Debug files/MCP/native-window UI smoke выполнены root; эти результаты нужно повторить для финальных dist binaries.
- Rust1.98.1, exact Cargo lock; NSIS3.13 portable hash подтверждён по primary release catalogue. CI actions закреплены exact SHA.
- Source dependency gate: root подтвердил444 exact registry versions; Windows graph259 normal/build/proc-macro crates и notices. Mac-only исключения не выдаются за проверенный Windows payload.
- Внешний MuPDF parser/render QA PDF выполнен root; новый CPU PNG first-slide export ещё требует независимого visual QA.
- Final dist повторно компилируется. Размеры, установщик, install/uninstall и remote CI/Release пока не считаются пройденными.

## Порядок и критерии проверки

1. Дождаться завершения всех dist writers. Проверить, что TrineDocs.exe и trinedocs-mcp.exe построены из последней версии source/lock, с static CRT, icon/VERSIONINFO/manifest. Зафиксировать SHA-256, размеры и PE imports. Отдельно сопоставить installer/tag version с EXE VERSIONINFO; metadata mismatch записать честно.
2. На финальном dist выполнить --smoke-test, --mcp-smoke-test, --ui-smoke-test в новых изолированных output/profile directories. Exit0 плюс новые result.json обязательны. Files проверяют native/Office/PDF, MCP — HTTP+stdio/auth/Host/Origin/body cap/session identity, UI — реальный native render/snapshots. GUI smoke не доказывает keyboard/IME/dialogs/performance.
3. Независимым PDF/PNG reader проверить PDF no-xref-repair/text/pages и PNG dimensions/nonblank text/shapes/images/alpha. Просмотреть образцы Docs/Sheets/Slides. PNG — первый слайд2×; rotation недоступна в shared Page, это подписано в UI. Synthetic unit corpus не подменяет 50 внешних файлов каждого формата.
4. После исходных двух ревью собрать NSIS verified portable3.13. Только известный ZIP hash до extraction/execution, без default PATH fallback. Include application+shim+notices+release-review. Цели main≤10MiB/setup≤5MiB сравнить с реальными bytes; превышение явно записать.
5. Запустить tools/test_installer.ps1 с новым Output и Destination, в том числе путём с пробелами, например target/QA spaced output/installed. До действий проверить отсутствие existing own UNKEY/ProgIDs/shortcuts. APPDATA только изолированная; app ArgumentList точный, NSIS /D и _?= последние unquoted directory arguments. Timeout/cleanup обязательны. Проверить Office defaults до/после, native default restoration, отсутствие payload/registration после uninstall и сохранность APPDATA/user files.
6. Отдельно проверить failure scenarios: existing install требует uninstall; running main/shim блокирует uninstall до обычного удаления; readonly destination/registry refusal не должны быть выданы за успех. All-users требует explicit elevated запуск; чистые Windows10/11 и all-users приёмка остаются отдельными результатами, пока не выполнены.
7. Обновить docs/audit/release-review.md только фактами. Commit source/reviews/plan/license evidence на preview branch, проверить spec_registry.py check (CLI действительно check), fmt/clippy/tests и artifact contents. Remote GitHub CI должен пройти build+installer smoke; artifact source SHA/tag должны совпадать.
8. Опубликовать unsigned prerelease с уникальным preview tag без overwrite существующего Release. Принимать ref metadata только env, без shell interpolation. Загрузить TrineDocs-Setup.exe и SHA256SUMS.txt, проверить доступность реального asset. Пользователю отправлять подтверждённую ссылку на installer, не URL предполагаемого будущего файла.

## Правила выполнения

Перед mutation — atomic flow claim для всех затронутых paths относительно V:\Desk. При неизвестном scope/installer writes держать "."; не писать при granted=false. После grant перечитать files, hash-guarded edits, ждать всех writers; release перед read-only QA/завершением. Cargo/target writers координирует root. Не перезаписывать существующие пользовательские installs/configs ради smoke.

## Открытые ограничения

Полный Word/Excel/PowerPoint набор не реализован. Advanced layout/track changes/mail merge, полный Excel function/dynamic/pivot/query/VBA набор, сложные PowerPoint masters/media/animations и second monitor неполны. Font/CJK/RTL/mixed formatting, native dialog shutdown/stale locks, live agent cursor/streaming/Stop/logs, full external corpus/fuzzing/Windows10/IME/DPI/accessibility/FPS не подтверждены. Unknown OOXML может теряться с warning; table external PPTX formatting и shared Page rotation неполны. Нет Authenticode; EXE/installer size goals ещё измеряются.

Успешный preview installer/CI не переводит глобальный DoD в done и не позволяет назвать сборку полностью готовой заменой Office.
