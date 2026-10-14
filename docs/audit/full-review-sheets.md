# Sheets review — 2026-10-13

Scope: `crates/trinedocs/src/sheets/{model,editor,formula,format,io,tests}.rs`.

## Fixed

- Images now follow inserted/deleted rows and columns. Deleting their anchor relocates it to the deletion boundary without discarding image bytes. Insertion that would push an image outside the grid fails before any mutation.
- `set_format` validates every format field before creating cells or changing formats. Invalid API input cannot make the workbook unsavable.
- Replace all validates the complete replacement set and lengths before changing cells. A failure preserves the workbook; replacement with no changes does not mark it dirty. Length checks precede string expansion.
- Cell, range and replace-all input also validate the length after localized formula canonicalization, before mutation or ID allocation.
- Limited model search returns matches in row/column order, rather than truncating lexicographically ordered addresses. A zero limit returns no matches. Only returned matches clone text.
- The search UI uses the displayed, localized/formatted value, clarifies “Заменить все”, and labels the close control on hover.
- Dotted and uppercase TSV extensions use tabs consistently for both import and export.
- XLSX import preserves `workbookView.activeTab` and selects a visible sheet when the stored active sheet is hidden.
- Deleting the final visible sheet fails atomically. Deletion and reordering keep a visible active sheet; native validation rejects books with no visible sheet or a hidden active sheet.

## Validation

- Added regression tests for structural image edits/overflow, invalid-format and replacement atomicity, limited search order, dotted TSV roundtrip, XLSX active-sheet roundtrip, and visible active-sheet invariants.
- `rustfmt --edition 2024` passed on the four changed Rust files.
- All Sheets tests passed in the root workspace run, as did clippy; see [the consolidated validation record](full-review.md). No separate concurrent cargo build was started in this review.

## Remaining limits observed

- XLSX conditional-formatting import/export is absent despite the model and canvas supporting simple threshold rules. Existing external conditional formatting is not roundtripped.
- Binary XLS/XLSB import is explicitly disabled pending isolated import handling.
- Freeze rendering is limited to 20 rows/10 columns and the editor displays this limitation.
- Full formula parity, sorting semantics, and exhaustive external OOXML roundtrip are not certified by this targeted review.

These limitations are not presented as completed functionality or full Office parity.
