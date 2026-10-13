# ADR-0010: Дыры контрактов MCP

Статус: **предложено**, высокорисковые изменения ждут владельца до внедрения.

Исходные проблемы: apply_ops требует atomic любых операций, но сеть/печать нельзя rollback; create_document требует document_id до создания; suggest на новой таблице/слайде без track changes model; stop/reject после чужих concurrent edits; Unicode «символ» не определён; edit default конфликтует с create/print_to_pdf full; open/close/focus side effects отличны от model changes; stale version granularity; bind stream chunks before tool JSON fully validated; rotation/revocation/rate limiting/session recovery.

Варианты atomic: (A) только model ops, side effects separate tasks; (B) staged model/files with compensating transactions, принтер/Google всё равно не atomic. Рекомендация A; сохранять первоначальное требование помеченным конфликтом до решения, не выдавать реализацию B за all-or-nothing.

Unicode options: grapheme-cluster index / code-point offset с обязательной boundary validation. Рекомендация code-point offset + stable CRDT cursor для rebasing, но без данных Loro Rust API решение не принято.

Для каждого tool явная таблица required access/confirmation/scope/idempotency/rollback. Auth: token opaque session-bound, IPC peer identity проверяется OS; Origin allowlist + reject null/foreign browser + Host validation, absent Origin native allowed только с bearer; exact policies тестируются. One-time tokens single-use expiry, no logging. Paths canonicalized, symlinks/reparse denied outside roots. Long ops cancellation must not undo another participant's changes.

Расширить schemas для save_copy, table sort/formulas, image update/delete, shape changes Docs, headers, sheet tab colors/notes/protection, presenter/show и settings безопасной subset (mcp-gaps). UI↔tool coverage проверяется каталогом, не наличием одной apply_ops.

Стоимость: разработка схем/validation/security тестов, оценки после rmcp/client spike; чисел нет.

