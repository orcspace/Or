# MCP deeper review — 2026-10-13

Reviewed `tools.rs`, `server.rs`, `register.rs`, `shim.rs`, and `integration.rs` against `docs/MCP.md`. Runtime approval, revocation and disk-operation permission routing are implemented in `app.rs` and were inspected read-only.

## Fixed

- Shared Sheets/Slides bulk replacement matches Unicode scalar sequences in linear time and validates the output length before allocation. Limits: 32,767 bytes per Sheets cell, 32 MiB of changed Sheets inputs per call, and 8 MiB of changed Slides text per call. Failure preserves the live model and version.
- Every read-only tool now enforces the same 1 MiB serialized response limit through a bounded writer; large Sheets find/object/list responses no longer bypass the transport response limit.
- Boolean arguments are validated before dispatch. An invalid string `contents: "false"` can no longer silently default to destructive `true`.
- Patches reject unknown fields instead of silently dropping misspelled properties while reporting success.
- Per-type omitted-field allowlists preserve patching Docs paragraph/table defaults hidden by `skip_serializing_if`; misspelled field names still fail atomically.
- Selection rejects hidden Sheets tabs, and general Sheets content reads default to the active sheet.
- Docs table-format range lengths use checked arithmetic; `usize::MAX` end indices return an error instead of overflow/panic.
- Unknown tool names are rejected before cloning/editing models.
- Client registration requires an absolute shim executable path.
- The stdio shim validates hexadecimal bearer tokens and bounds HTTP initialization to ten seconds.

## Validation

Added regression tests for Unicode/non-overlapping matching, replacement expansion/atomicity, unknown patch keys, hidden-sheet selection, read response limits, invalid destructive booleans, extreme table bounds, and absolute shim paths. These tests passed in the root workspace run, as did clippy, formatting and `git diff --check`. Live MCP integration was not rerun; see [the consolidated validation record](full-review.md).

## Remaining limitations

- Read responses build their JSON value before the streaming size check. Oversized results are rejected before delivery, but large object/image serialization can still allocate more than 1 MiB temporarily.
- Pagination is not supported uniformly by object/list APIs; oversized responses return `too_large` rather than exposing complete large collections.
- Dedicated per-session request rate limits and owner-only connection-file ACLs are not established by this review.
- Shim readiness still begins with a TCP reachability probe; stale connection metadata pointing at another listener may fail initialization rather than recover automatically.
- Protocol-version compatibility, Tasks and broader tool parity remain separate acceptance work; existing integration smoke coverage is not exhaustive certification.
