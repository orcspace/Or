# Docs review — 2026-10-13

Scope: `crates/trinedocs/src/docs/{editor,model,io}.rs`. Reviewed text replacement, stable paragraph/block anchors, clipboard/input/IME, page deletion, table/image insertion, validation, and DOCX/Markdown/HTML boundaries. This is a source review with targeted regression coverage, not a claim of Office parity or exhaustive GUI acceptance.

## Fixed

- **HTML content loss:** visible text directly inside body/div containers vanished when sibling paragraphs existed; table cell contents vanished entirely. The importer now preserves inline groups in document order and retains each cell's text as paragraphs. The loss notice explicitly describes table flattening. Inherited container CSS, basic inline bold formatting, and hidden script/style/head exclusion remain covered.
- **HTML heading semantics:** a body containing only h4–h6 could be treated as loose text and lose its heading style. All supported heading tags now use the same block path.
- **Office extension mismatch:** upper-case DOTX/DOCM exports used the ordinary DOCX content type, despite accepting the extensions. Export now normalizes the extension once before selecting its package content type.
- **Markdown legacy line endings:** lone CR separators remained inside paragraph text. Import now treats CR, LF, and CRLF consistently.
- **Editor Enter:** an empty numbered/bullet list continued endlessly; Enter now ends the list without introducing another paragraph. Enter at the end of Title/Heading starts a Normal body paragraph and resets typing style. Soft breaks preserve heading style; page-break commands retain their existing behavior.

- **Find usability:** Escape closes the search while a search/replacement field owns focus and restores canvas focus. Changing the query or case mode resets the next-match index.

## Verification

Seven new regression tests cover the scenarios above, including visible-text order, hidden content, table cells with multiple paragraphs, content types, and preservation of soft/page breaks. These tests passed in the root workspace run, as did clippy, formatting and `git diff --check`. See [the consolidated validation record](full-review.md).

## Remaining limits

- HTML tables preserve visible text but are flattened, rather than retained as editable table structures; CSS layouts and network images remain unsupported and explicitly disclosed. The existing fallback for non-XML HTML is a plain-text approximation, not a standards-complete HTML parser.
- Markdown import/export is a limited syntax subset. Nested emphasis, literal markup escaping, links, tables, and exact rich-format round trips require further work; this review does not certify lossless Markdown serialization.
- DOCX unsupported fragments (comments, notes, fields, revisions, floating layout) still rely on loss notices rather than complete opaque round trips. Those are existing parity gaps and need a larger format-preservation design.
- Custom-canvas accessibility, real Windows IME, 100-page latency, real Office corpus interoperability, and visual QA in both themes were not verified by this source-only reviewer.
