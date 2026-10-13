# STRIDE: MCP, импорт и sync

Статус: модель угроз первого прохода; **меры запланированы, не реализованы**. Активы: содержимое/история документов, ключи/токены, local paths, permissions, corpus/assets, клиентские конфиги. Trust boundaries: external MCP client↔shim↔OS IPC↔ops; loopback HTTP↔ops; hostile file↔worker↔bounded model; internet↔sync↔DB/S3. AI/document text недоверены.

| ID | Surface / STRIDE | Сценарий | Митигация (план) | Проверка |
|---|---|---|---|---|
| SEC-M-S | MCP spoofing | другой пользователь/процесс выдаёт себя за trusted client | pipe ACL+peer SID/process identity; unix UID/0600; single-use expiring handshake token; client confirmation | other-user IPC, replay, token theft T-03-MCP |
| SEC-M-T | MCP tampering | изменение операции/path в гонке | full schema validation, base-version policy, canonical paths/reparse checks, ops atomicity | stale range/races/path escape T-SECURITY |
| SEC-M-R | MCP repudiation | AI denies destructive change | audit client/session/tool/doc/time/result+undo group; protected bounded audit log, content masked | attribution/drop/error audit T-03-MCP |
| SEC-M-I | MCP disclosure | document injection triggers upload/read outside scope | document text untrusted fields; tools authorization, roots, context chips/confirm export; no arbitrary read | injection fixture cannot cause actions outside requested scope T-SECURITY |
| SEC-M-D | MCP DoS | loopback floods/huge JSON/base64/regex | per-client rate/size/timeouts/pagination/task cancel; regex bounded; UI async | exhaustion keeps UI responsive T-03-MCP |
| SEC-M-E | MCP privilege escalation | browser DNS rebinding/CSRF or session role revoked | loopback-only, bearer, Origin+Host policy, no wildcard; current authorization every call/commit, revocation cancels staged mutation | missing bearer/null/foreign Origin/host/revoke T-SECURITY |
| SEC-I-S | import spoofing | extension/MIME mismatch, forged LO binary | magic/package validation, download signature/hash+trusted URL, exact pinned artifacts | mismatch/tampered download T-SECURITY |
| SEC-I-T | import tampering | zip traversal/relations/duplicate names overwrite | reject absolute/.. paths/duplicate entries/symlinks, bounded package reader, no active content; opaque parts no execution | traversal, opaque macro byte-preservation T-SECURITY |
| SEC-I-R | import repudiation | partial import changes original unexpectedly | provenance/hash/loss report, staged model ops, source immutable until save | failed import no original changes T-02-FILES |
| SEC-I-I | import disclosure | XXE/external relations/DDE/remote font beacon | disable entities/network/OLE/macros/DDE; no file-url fetch, confirmation for links; worker least privilege | XXE/local path/network fixture T-SECURITY |
| SEC-I-D | import DoS | zip/XML/image bombs or hung LO | expanded/compressed/entry/depth limits, memory/time Job Objects, cancel process tree, progress outside UI | bomb/hang/cancel and survive T-07-LEGACY |
| SEC-I-E | import privilege escalation | native parser compromise escapes worker | isolated child restricted token/job/container where verified, minimal file/IPC capabilities, no inherited secrets | malicious parser fixture/worker privilege tests T-SECURITY |
| SEC-S-S | sync spoofing | stolen JWT/forged presence/room joins | TLS, signed short-lived auth, refresh rotation, verified subject+room role | expired/wrong user token T-04-SYNC |
| SEC-S-T | sync tampering | hostile CRDT update/asset IDs | update size/model validation, membership/version checks, content-hash assets, ownership authorization | malformed updates/cross-doc assets T-SECURITY |
| SEC-S-R | sync repudiation | edit/invite cannot be attributed | author audit, immutable event IDs, minimal retention policy, restore evidence | blame/restore audit T-04-SYNC |
| SEC-S-I | sync disclosure | IDOR/S3 public leak/backup access | document role per request, private bucket scoped URLs, retention/deletion backup policy; E2EE owner ADR | guessed IDs/expired links/backup data T-SECURITY |
| SEC-S-D | sync DoS | many clients/uploads/email abuse | room/client/global limits, quotas/backpressure, asset bounds, invitation throttles, monitor/alerts |50participant+abuse load T-04-SYNC |
| SEC-S-E | sync privilege escalation | reader/commenter sends raw editing CRDT op | server validates authorized operation semantics, role revocation at commit, never blindly relay all updates | reader edit/commenter write/revocation races T-SECURITY |

Открыто: OS sandbox feasibility; role enforcement on opaque CRDT updates; exact http Origin/native client policy; no implemented keychain/token protection; E2EE owner choice; production rate/resource values require measurements. Все эти пункты остаются в аудите. Список мер не доказывает защиту.

Privacy: telemetry/crash opt-in, no content/keys; crash dumps can contain secrets even without intended fields → sanitization/consent review. Secret-bearing logs disabled. User documents cannot grant new permissions to an agent.

