# Storage, recovery, and operations review — 2026-10-13

Scope: `ops.rs`, `recovery.rs`, and `file_io.rs`, with read-only inspection of shared native serialization, atomic writes, and application integration. Source review prioritizes durable document recovery, failed-operation atomicity, and conservative conflict handling.

## Fixed

- **Recovery restoration loss window:** the application previously deleted the source recovery immediately on restoring a tab, before the restored document had its own durable snapshot. `Recovery::restore(previous_id, snapshot)` queues a handoff that atomically writes the destination before deleting source ZIP/legacy JSON. Failure retains the durable source. The coordinating agent integrates this API into the restore action.
- **Recovery queue lifecycle:** coalesced writes retain restoration cleanup. Clears after queued or in-flight restoration retain the source ID too, preventing previously discarded/saved documents from reappearing on next launch. Failed stale work cannot replace newer queued work.
- **Recovery loader contamination:** temporary/backup files could consume the first 100 directory slots and even be imported as snapshots. The loader accepts regular numeric-ID `.trinezip`/`.json` files, verifies the header ID, ignores clutter, and chooses one newest valid version per ID (ZIP preferred on ties). The limit applies to valid recovered documents, with newer IDs first.
- **Save-as/export conflict checks:** targets with no supplied expected timestamp previously skipped all rechecks during expensive encoding. Saving now captures the target state before encoding and rejects creation, disappearance, or timestamp change during encoding, including Save As/export. Metadata errors are propagated instead of being treated as missing files.
- **Undo save boundary:** rapid edits after a completed save could merge into the preceding typing group and undo both saved and later text at once. A completed save now seals that immediate group boundary.
- **AI rollback and hidden Sheets:** reverting an AI unhide while the user views that sheet could preserve a now-hidden active sheet and fail validation. Rollback maps the viewed sheet only when visible in the restored model and otherwise selects a valid visible fallback.

## Verification

Nine new focused regression tests cover durable handoff success/failure, queued/in-flight coalescing and clear, loader clutter/deduplication, target creation/modification during export, undo save boundaries, and hidden-sheet AI rollback. These tests passed in the root workspace run, as did clippy, formatting and `git diff --check`. See [the consolidated validation record](full-review.md).

## Independent cross-review

- The Slides reviewer found the first handoff implementation lost source cleanup when superseded by Clear. This was corrected for both queued and in-flight work and covered by regressions.
- Read-only review of the MCP changes identified schema fields omitted by `skip_serializing_if` being rejected as unknown patch keys. This was sent to the MCP owner for correction.
- Read-only review of chat cancellation identified queued streamed text being dropped by Stop before polling. This was sent to the coordinating agent for correction.

## Remaining limitations

- Timestamp comparisons plus cooperative marker locks are not a filesystem compare-and-swap: a noncooperating writer can still race between the final check and the atomic rename, and same-timestamp edits are not detected. No broader filesystem safety claim is made.
- Recovery errors retry; a persistent disk/serialization failure keeps the source when handing off but cannot create new durability. Failure reporting remains important.
- Undo groups do not retain internal checkpoints for every version. The coordinating agent added the dispatch boundary in `app.rs`: subsequent typing starts a new undo group while a save is in progress.
- Process crash/power-loss injection, network shares, and hardware durability were not exercised in this source review. The existing fsync/atomic-write path was read, not replaced.
