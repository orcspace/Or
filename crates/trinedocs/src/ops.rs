use crate::{docs::model::TextEditDelta, native::Document};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Instant};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Author {
    User,
    Agent { name: String },
}
#[derive(Clone, Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "Document owns heap buffers; boxing adds an allocation per UI operation."
)]
pub enum Op {
    Replace {
        document: Document,
        label: String,
        author: Author,
        base_version: Option<u64>,
    },
    Undo,
    Redo,
}
#[derive(Clone, Debug)]
pub struct Change {
    pub id: u64,
    pub before: Document,
    pub after: Document,
    pub label: String,
    pub author: Author,
    pub accepted: bool,
    pub docs_edits: Option<Vec<TextEditDelta>>,
}
#[derive(Clone, Debug)]
pub struct Tab {
    pub id: u64,
    pub title: String,
    pub path: Option<PathBuf>,
    pub document: Document,
    pub version: u64,
    pub saved_version: u64,
    pub undo: Vec<Change>,
    pub redo: Vec<Change>,
    pub ai: Vec<Change>,
    pub last_edit: Instant,
    pub last_author: Author,
    pub read_only: bool,
    pub save_locked: bool,
    pub pending_docs_edits: Option<Vec<TextEditDelta>>,
}
impl Tab {
    pub fn new(id: u64, document: Document, title: String, path: Option<PathBuf>) -> Self {
        let saved_version = if path.is_some() { 0 } else { u64::MAX };
        Self {
            id,
            title,
            path,
            document,
            version: 0,
            saved_version,
            undo: Vec::new(),
            redo: Vec::new(),
            ai: Vec::new(),
            last_edit: Instant::now(),
            last_author: Author::User,
            read_only: false,
            save_locked: false,
            pending_docs_edits: Some(Vec::new()),
        }
    }
    pub fn dirty(&self) -> bool {
        self.version != self.saved_version
    }
    pub fn apply(&mut self, op: Op) -> Result<(), String> {
        self.apply_with_docs_edits(op, None)
    }
    pub fn apply_with_docs_edits(
        &mut self,
        op: Op,
        docs_edits: Option<Vec<TextEditDelta>>,
    ) -> Result<(), String> {
        if self.read_only {
            return Err("Документ открыт только для чтения".into());
        }
        match op {
            Op::Replace {
                mut document,
                label,
                author,
                base_version,
            } => {
                if base_version.is_some_and(|v| v != self.version) {
                    return Err(format!("conflict: текущая версия {}", self.version));
                }
                if document.kind() != self.document.kind() {
                    return Err("Тип открытого документа нельзя изменить операцией правки".into());
                }
                preserve_allocator_floor(&mut document, allocator_floor(&self.document));
                document.validate()?;
                if document == self.document {
                    return Ok(());
                }
                self.preflight_docs_edits(docs_edits.as_deref())?;
                let group = matches!(author, Author::User)
                    && author == self.last_author
                    && self.last_edit.elapsed().as_secs_f32() < 1.0
                    && self.version != self.saved_version
                    && self.undo.last().is_some_and(|c| c.label == label);
                self.version = self.version.checked_add(1).ok_or("Исчерпаны версии")?;
                let change = Change {
                    id: self.version,
                    before: self.document.clone(),
                    after: document.clone(),
                    label,
                    author: author.clone(),
                    accepted: false,
                    docs_edits: docs_edits.clone(),
                };
                if group {
                    if let Some(last) = self.undo.last_mut() {
                        last.after = document.clone();
                        match (&mut last.docs_edits, &docs_edits) {
                            (Some(old), Some(new)) => old.extend_from_slice(new),
                            _ => last.docs_edits = None,
                        }
                    }
                } else {
                    self.undo.push(change.clone());
                }
                if matches!(author, Author::Agent { .. }) {
                    self.ai.push(change);
                }
                self.enqueue_docs_edits(docs_edits.as_deref());
                self.document = document;
                self.redo.clear();
                self.last_edit = Instant::now();
                self.last_author = author;
                if self.undo.len() > 100 {
                    self.undo.remove(0);
                }
                if self.ai.len() > 100 {
                    self.ai.remove(0);
                }
            }
            Op::Undo => {
                let next_version = self.version.checked_add(1).ok_or("Исчерпаны версии")?;
                let mut restored = self
                    .undo
                    .last()
                    .ok_or("Нет правок для отмены")?
                    .before
                    .clone();
                preserve_allocator_floor(&mut restored, allocator_floor(&self.document));
                restored.validate()?;
                let edits = self
                    .undo
                    .last()
                    .and_then(|c| c.docs_edits.as_ref())
                    .map(|edits| {
                        edits
                            .iter()
                            .rev()
                            .map(TextEditDelta::inverse)
                            .collect::<Vec<_>>()
                    });
                self.preflight_docs_edits(edits.as_deref())?;
                let c = self.undo.pop().ok_or("Нет правок для отмены")?;
                self.enqueue_docs_edits(edits.as_deref());
                self.document = restored;
                self.redo.push(c);
                self.version = next_version;
                self.last_edit = Instant::now() - std::time::Duration::from_secs(2);
            }
            Op::Redo => {
                let next_version = self.version.checked_add(1).ok_or("Исчерпаны версии")?;
                let mut restored = self
                    .redo
                    .last()
                    .ok_or("Нет правок для повтора")?
                    .after
                    .clone();
                preserve_allocator_floor(&mut restored, allocator_floor(&self.document));
                restored.validate()?;
                let edits = self.redo.last().and_then(|c| c.docs_edits.clone());
                self.preflight_docs_edits(edits.as_deref())?;
                let c = self.redo.pop().ok_or("Нет правок для повтора")?;
                self.enqueue_docs_edits(edits.as_deref());
                self.document = restored;
                self.undo.push(c);
                self.version = next_version;
                self.last_edit = Instant::now() - std::time::Duration::from_secs(2);
            }
        }
        Ok(())
    }
    fn preflight_docs_edits(&self, edits: Option<&[TextEditDelta]>) -> Result<(), String> {
        if let Some(edits) = edits
            && matches!(&self.document, Document::Docs(_))
        {
            let pending = self
                .pending_docs_edits
                .as_ref()
                .ok_or("conflict: дождитесь отображения предыдущего изменения Docs")?;
            if edits.len() > 10000 || pending.len().saturating_add(edits.len()) > 20000 {
                return Err(
                    "too_large: очередь позиционных изменений Docs; дождитесь кадра UI".into(),
                );
            }
        }
        Ok(())
    }
    fn enqueue_docs_edits(&mut self, edits: Option<&[TextEditDelta]>) {
        if matches!(&self.document, Document::Docs(_)) {
            match (&mut self.pending_docs_edits, edits) {
                (Some(pending), Some(edits)) => pending.extend_from_slice(edits),
                _ => self.pending_docs_edits = None,
            }
        }
    }
    pub fn reject_ai(&mut self, id: u64) -> Result<(), String> {
        let c = self
            .ai
            .iter()
            .find(|c| c.id == id && !c.accepted)
            .cloned()
            .ok_or("Правка AI не найдена")?;
        let mut expected_after = c.after.clone();
        preserve_allocator_floor(&mut expected_after, allocator_floor(&self.document));
        preserve_sheets_view(&mut expected_after, &self.document);
        if self.document != expected_after {
            return Err(
                "Есть последующие правки: отмените их сначала, чтобы сохранить данные".into(),
            );
        }
        let edits = c
            .docs_edits
            .as_ref()
            .map(|edits| edits.iter().rev().map(TextEditDelta::inverse).collect());
        let mut restored = c.before;
        preserve_sheets_view(&mut restored, &self.document);
        self.apply_with_docs_edits(
            Op::Replace {
                document: restored,
                label: format!("Откат AI: {}", c.label),
                author: Author::User,
                base_version: None,
            },
            edits,
        )?;
        self.ai.retain(|c| c.id != id);
        Ok(())
    }
}
fn preserve_sheets_view(destination: &mut Document, source: &Document) {
    if let (Document::Sheets(destination), Document::Sheets(source)) = (destination, source) {
        if let Some(sheet_id) = source.sheets.get(source.active).map(|sheet| sheet.id)
            && let Some(index) = destination
                .sheets
                .iter()
                .position(|sheet| sheet.id == sheet_id && !sheet.hidden)
        {
            destination.active = index;
        } else {
            destination.active = destination
                .active
                .min(destination.sheets.len().saturating_sub(1));
            if destination
                .sheets
                .get(destination.active)
                .is_some_and(|sheet| sheet.hidden)
                && let Some(visible) = destination.sheets.iter().position(|sheet| !sheet.hidden)
            {
                destination.active = visible;
            }
        }
        destination.selection = source.selection.clone();
        destination.selection.row = destination
            .selection
            .row
            .min(crate::sheets::model::MAX_ROWS - 1);
        destination.selection.anchor_row = destination
            .selection
            .anchor_row
            .min(crate::sheets::model::MAX_ROWS - 1);
        destination.selection.col = destination
            .selection
            .col
            .min(crate::sheets::model::MAX_COLS - 1);
        destination.selection.anchor_col = destination
            .selection
            .anchor_col
            .min(crate::sheets::model::MAX_COLS - 1);
    }
}

fn allocator_floor(document: &Document) -> u64 {
    match document {
        Document::Docs(d) => d.next_id,
        Document::Sheets(b) => b.next_id,
        Document::Slides(d) => d.next_id,
    }
}

fn preserve_allocator_floor(document: &mut Document, floor: u64) {
    let next_id = match document {
        Document::Docs(d) => &mut d.next_id,
        Document::Sheets(b) => &mut b.next_id,
        Document::Slides(d) => &mut d.next_id,
    };
    *next_id = (*next_id).max(floor);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_undo_conflict_and_user_preservation() -> Result<(), String> {
        let d = Document::create("docs")?;
        let mut t = Tab::new(1, d.clone(), "test".into(), None);
        let mut edited = d.clone();
        if let Document::Docs(ref mut v) = edited {
            *v = crate::docs::model::Doc::from_text("AI");
        }
        t.apply(Op::Replace {
            document: edited.clone(),
            label: "insert".into(),
            author: Author::Agent {
                name: "test".into(),
            },
            base_version: Some(0),
        })?;
        assert_eq!(t.ai.len(), 1);
        assert!(
            t.apply(Op::Replace {
                document: d.clone(),
                label: "bad".into(),
                author: Author::User,
                base_version: Some(0)
            })
            .is_err()
        );
        t.apply(Op::Undo)?;
        let mut restored = d.clone();
        preserve_allocator_floor(&mut restored, allocator_floor(&edited));
        assert_eq!(t.document, restored);
        t.apply(Op::Redo)?;
        assert_eq!(t.document, edited);
        t.reject_ai(1)?;
        assert_eq!(t.document, restored);
        Ok(())
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;
    #[test]
    fn save_boundary_splits_user_undo_groups() -> Result<(), String> {
        let mut tab = Tab::new(
            1,
            Document::create("docs")?,
            "t".into(),
            Some("t.docx".into()),
        );
        for value in ["saved", "saved plus more"] {
            let mut document = tab.document.clone();
            if let Document::Docs(doc) = &mut document {
                let end = crate::docs::model::Position {
                    paragraph: 0,
                    offset: doc.paragraphs[0].len(),
                };
                doc.replace_range(crate::docs::model::Position::default(), end, value, None)?;
            }
            tab.apply(Op::Replace {
                document,
                label: "typing".into(),
                author: Author::User,
                base_version: None,
            })?;
            if value == "saved" {
                tab.saved_version = tab.version;
            }
        }
        assert_eq!(tab.undo.len(), 2);
        tab.apply(Op::Undo)?;
        assert!(matches!(&tab.document, Document::Docs(doc) if doc.plain_text() == "saved"));
        Ok(())
    }
    #[test]
    fn undo_failure_is_atomic() -> Result<(), String> {
        let mut t = Tab::new(1, Document::create("docs")?, "t".into(), None);
        assert!(t.dirty());
        let mut doc = t.document.clone();
        if let Document::Docs(d) = &mut doc {
            d.insert_text(d.paragraphs[0].id, 0, "a")?;
        }
        t.apply(Op::Replace {
            document: doc,
            label: "test".into(),
            author: Author::User,
            base_version: None,
        })?;
        t.version = u64::MAX;
        let before = t.document.clone();
        let count = t.undo.len();
        assert!(t.apply(Op::Undo).is_err());
        assert_eq!(t.document, before);
        assert_eq!(t.undo.len(), count);
        Ok(())
    }
}

#[cfg(test)]
mod allocator_tests {
    use super::*;

    fn add_object(document: &mut Document, ordinal: u32, label: &str) -> Result<u64, String> {
        match document {
            Document::Docs(doc) => {
                let p = doc.paragraphs.last().ok_or("Нет абзаца")?;
                let paragraph_id = p.id;
                let offset = p.len();
                doc.insert_text(paragraph_id, offset, &format!("\n{label}"))?;
                doc.paragraphs
                    .last()
                    .map(|p| p.id)
                    .ok_or("Нет нового абзаца".into())
            }
            Document::Sheets(book) => {
                let sheet = book.sheets.first().ok_or("Нет листа")?.id;
                book.set_cell(sheet, 0, ordinal, label)
            }
            Document::Slides(deck) => {
                let slide = deck.slides.first().ok_or("Нет слайда")?.id;
                let kind = match ordinal % 3 {
                    0 => crate::slides::model::ElementKind::Text,
                    1 => crate::slides::model::ElementKind::Rect,
                    _ => crate::slides::model::ElementKind::Ellipse,
                };
                let id = deck.add_element(slide, kind)?;
                if let Some(element) = deck
                    .slide_mut(slide)
                    .and_then(|s| s.elements.iter_mut().find(|e| e.id == id))
                {
                    element.text = label.into();
                }
                Ok(id)
            }
        }
    }

    fn commit(
        tab: &mut Tab,
        document: Document,
        label: &str,
        author: Author,
    ) -> Result<(), String> {
        tab.apply(Op::Replace {
            document,
            label: label.into(),
            author,
            base_version: Some(tab.version),
        })
    }

    #[test]
    fn undo_then_new_objects_never_reuse_ids_in_all_editors() -> Result<(), String> {
        for kind in ["docs", "sheets", "slides"] {
            let mut tab = Tab::new(1, Document::create(kind)?, kind.into(), None);
            let mut first = tab.document.clone();
            let retired_id = add_object(&mut first, 0, "First")?;
            let used_floor = allocator_floor(&first);
            commit(&mut tab, first, "add first", Author::User)?;
            tab.apply(Op::Undo)?;
            assert!(allocator_floor(&tab.document) >= used_floor, "{kind}");
            let mut second = tab.document.clone();
            let replacement_id = add_object(&mut second, 1, "Different")?;
            assert!(replacement_id > retired_id, "{kind}: deleted ID was reused");
            commit(&mut tab, second, "add different", Author::User)?;
            assert!(tab.redo.is_empty(), "{kind}");
        }
        Ok(())
    }

    #[test]
    fn redo_and_ai_rollback_keep_high_water_and_ignore_only_counter_difference()
    -> Result<(), String> {
        for kind in ["docs", "sheets", "slides"] {
            let mut tab = Tab::new(1, Document::create(kind)?, kind.into(), None);
            let original = tab.document.clone();
            let mut first = original.clone();
            let first_id = add_object(&mut first, 0, "Agent")?;
            commit(
                &mut tab,
                first.clone(),
                "AI object",
                Author::Agent {
                    name: "agent".into(),
                },
            )?;
            let ai_id = tab.ai.last().ok_or("Нет AI правки")?.id;
            let mut second = tab.document.clone();
            let second_id = add_object(&mut second, 1, "User later")?;
            let floor = allocator_floor(&second);
            commit(&mut tab, second.clone(), "later object", Author::User)?;
            assert!(
                tab.reject_ai(ai_id).is_err(),
                "{kind}: real later content must conflict"
            );
            tab.apply(Op::Undo)?;
            let mut expected_first = first.clone();
            preserve_allocator_floor(&mut expected_first, floor);
            assert_eq!(tab.document, expected_first, "{kind}");
            tab.apply(Op::Redo)?;
            assert_eq!(tab.document, second, "{kind}");
            assert!(allocator_floor(&tab.document) >= floor, "{kind}");
            tab.apply(Op::Undo)?;
            tab.reject_ai(ai_id)?;
            let mut expected_original = original;
            preserve_allocator_floor(&mut expected_original, floor);
            assert_eq!(tab.document, expected_original, "{kind}");
            assert!(tab.ai.iter().all(|c| c.id != ai_id));
            let mut third = tab.document.clone();
            let third_id = add_object(&mut third, 2, "After rollback")?;
            assert!(third_id > first_id && third_id > second_id, "{kind}");
            commit(&mut tab, third, "after AI rollback", Author::User)?;
        }
        Ok(())
    }

    #[test]
    fn replacement_cannot_change_editor_kind_or_history() -> Result<(), String> {
        let mut tab = Tab::new(1, Document::create("docs")?, "test".into(), None);
        let before = tab.document.clone();
        assert!(
            commit(
                &mut tab,
                Document::create("sheets")?,
                "different kind",
                Author::User
            )
            .is_err()
        );
        assert_eq!(tab.document, before);
        assert_eq!(tab.version, 0);
        assert!(tab.undo.is_empty() && tab.redo.is_empty() && tab.ai.is_empty());
        Ok(())
    }

    #[test]
    fn replacement_of_old_snapshot_cannot_lower_allocator_floor() -> Result<(), String> {
        for kind in ["docs", "sheets", "slides"] {
            let original = Document::create(kind)?;
            let mut tab = Tab::new(1, original.clone(), kind.into(), None);
            let mut current = original.clone();
            add_object(&mut current, 0, "Object")?;
            let floor = allocator_floor(&current);
            commit(&mut tab, current, "object", Author::User)?;
            commit(&mut tab, original, "old model", Author::User)?;
            assert_eq!(allocator_floor(&tab.document), floor, "{kind}");
            let mut next = tab.document.clone();
            assert!(add_object(&mut next, 1, "New")? >= floor, "{kind}");
        }
        Ok(())
    }

    #[test]
    fn redo_overflow_and_readonly_leave_model_history_and_counter_unchanged() -> Result<(), String>
    {
        let mut tab = Tab::new(1, Document::create("docs")?, "test".into(), None);
        let mut edited = tab.document.clone();
        add_object(&mut edited, 0, "Object")?;
        commit(&mut tab, edited, "object", Author::User)?;
        tab.apply(Op::Undo)?;
        let before = tab.document.clone();
        let undo = tab.undo.len();
        let redo = tab.redo.len();
        tab.version = u64::MAX;
        assert!(tab.apply(Op::Redo).is_err());
        assert_eq!(tab.document, before);
        assert_eq!(tab.undo.len(), undo);
        assert_eq!(tab.redo.len(), redo);
        tab.version = 2;
        tab.read_only = true;
        assert!(tab.apply(Op::Redo).is_err());
        assert_eq!(tab.document, before);
        assert_eq!(tab.undo.len(), undo);
        assert_eq!(tab.redo.len(), redo);
        Ok(())
    }

    #[test]
    fn file_save_lock_allows_edit_undo_and_redo() -> Result<(), String> {
        let mut tab = Tab::new(
            1,
            Document::create("docs")?,
            "locked".into(),
            Some("locked.docx".into()),
        );
        tab.save_locked = true;
        let before = tab.document.clone();
        let mut document = before.clone();
        if let Document::Docs(d) = &mut document {
            let pid = d.paragraphs.first().ok_or("Нет абзаца")?.id;
            d.insert_text(pid, 0, "Можно редактировать")?;
        }
        let after = document.clone();
        tab.apply(Op::Replace {
            document,
            label: "edit".into(),
            author: Author::User,
            base_version: None,
        })?;
        assert!(tab.dirty());
        tab.apply(Op::Undo)?;
        assert_eq!(tab.document, before);
        tab.apply(Op::Redo)?;
        assert_eq!(tab.document, after);
        assert!(tab.save_locked);
        Ok(())
    }
}

#[cfg(test)]
mod sheets_view_tests {
    use super::*;
    use crate::sheets::model::{Book, Selection};

    fn agent_change(tab: &mut Tab, book: Book) -> Result<u64, String> {
        tab.apply(Op::Replace {
            document: Document::Sheets(book),
            label: "AI cells".into(),
            author: Author::Agent {
                name: "agent".into(),
            },
            base_version: Some(tab.version),
        })?;
        tab.ai
            .last()
            .map(|change| change.id)
            .ok_or("Нет AI правки".into())
    }

    #[test]
    fn ai_rollback_of_unhide_selects_a_visible_sheet() -> Result<(), String> {
        let mut book = Book::default();
        let visible = book.active_id();
        let hidden = book.add_sheet("Hidden")?;
        book.sheet_by_id_mut(hidden).ok_or("Missing sheet")?.hidden = true;
        let mut tab = Tab::new(1, Document::Sheets(book.clone()), "book".into(), None);
        book.sheet_by_id_mut(hidden).ok_or("Missing sheet")?.hidden = false;
        let change = agent_change(&mut tab, book)?;
        if let Document::Sheets(book) = &mut tab.document {
            book.active = book
                .sheets
                .iter()
                .position(|sheet| sheet.id == hidden)
                .ok_or("Missing sheet")?;
        }
        tab.reject_ai(change)?;
        let Document::Sheets(restored) = &tab.document else {
            return Err("Missing workbook".into());
        };
        assert_eq!(restored.active_id(), visible);
        assert!(restored.sheet_by_id(hidden).ok_or("Missing sheet")?.hidden);
        restored.validate()
    }
    #[test]
    fn ai_rollback_preserves_navigation_without_content_conflict() -> Result<(), String> {
        let mut book = Book::default();
        let first = book.active_id();
        let second = book.add_sheet("Другой лист")?;
        let mut tab = Tab::new(
            1,
            Document::Sheets(book.clone()),
            "book".into(),
            Some("book.xlsx".into()),
        );
        book.set_cell(first, 0, 0, "AI")?;
        let used_floor = book.next_id;
        let change = agent_change(&mut tab, book)?;
        let selection = Selection {
            row: 7,
            col: 4,
            anchor_row: 5,
            anchor_col: 2,
        };
        if let Document::Sheets(current) = &mut tab.document {
            current.active = current
                .sheets
                .iter()
                .position(|sheet| sheet.id == second)
                .ok_or("Нет листа")?;
            current.selection = selection.clone();
        }
        assert_eq!(tab.version, 1);
        tab.reject_ai(change)?;
        let Document::Sheets(restored) = &tab.document else {
            return Err("Нет книги".into());
        };
        assert_eq!(restored.active_id(), second);
        assert_eq!(restored.selection, selection);
        assert!(
            restored
                .sheet_by_id(first)
                .ok_or("Нет первого листа")?
                .cell(0, 0)
                .is_none()
        );
        assert!(restored.next_id >= used_floor);
        assert!(tab.ai.is_empty());
        restored.validate()
    }

    #[test]
    fn ai_rollback_maps_current_sheet_id_across_restored_indices() -> Result<(), String> {
        let mut book = Book::default();
        let deleted = book.active_id();
        let selected = book.add_sheet("Выбранный")?;
        let mut tab = Tab::new(1, Document::Sheets(book.clone()), "book".into(), None);
        book.delete_sheet(deleted)?;
        let change = agent_change(&mut tab, book)?;
        if let Document::Sheets(current) = &mut tab.document {
            current.active = 0;
            current.selection = Selection {
                row: 10,
                col: 8,
                anchor_row: 10,
                anchor_col: 8,
            };
        }
        tab.reject_ai(change)?;
        let Document::Sheets(restored) = &tab.document else {
            return Err("Нет книги".into());
        };
        assert_eq!(restored.active, 1);
        assert_eq!(restored.active_id(), selected);
        assert_eq!((restored.selection.row, restored.selection.col), (10, 8));
        restored.validate()
    }

    #[test]
    fn ai_rollback_selected_created_sheet_falls_back_to_valid_view() -> Result<(), String> {
        let mut book = Book::default();
        let original = book.active_id();
        let mut tab = Tab::new(1, Document::Sheets(book.clone()), "book".into(), None);
        let created = book.add_sheet("AI новый")?;
        let change = agent_change(&mut tab, book)?;
        if let Document::Sheets(current) = &mut tab.document {
            current.active = current
                .sheets
                .iter()
                .position(|sheet| sheet.id == created)
                .ok_or("Нет листа")?;
            current.selection = Selection {
                row: 25,
                col: 3,
                anchor_row: 25,
                anchor_col: 3,
            };
        }
        tab.reject_ai(change)?;
        let Document::Sheets(restored) = &tab.document else {
            return Err("Нет книги".into());
        };
        assert_eq!(restored.active_id(), original);
        assert!(restored.sheet_by_id(created).is_none());
        assert_eq!((restored.selection.row, restored.selection.col), (25, 3));
        restored.validate()
    }

    #[test]
    fn ai_rollback_still_conflicts_with_later_cell_change() -> Result<(), String> {
        let mut book = Book::default();
        let sheet = book.active_id();
        let mut tab = Tab::new(1, Document::Sheets(book.clone()), "book".into(), None);
        book.set_cell(sheet, 0, 0, "AI")?;
        let change = agent_change(&mut tab, book.clone())?;
        book.set_cell(sheet, 0, 1, "User")?;
        tab.apply(Op::Replace {
            document: Document::Sheets(book),
            label: "User cell".into(),
            author: Author::User,
            base_version: Some(tab.version),
        })?;
        if let Document::Sheets(current) = &mut tab.document {
            current.selection = Selection {
                row: 4,
                col: 4,
                anchor_row: 4,
                anchor_col: 4,
            };
        }
        let before = tab.document.clone();
        let version = tab.version;
        let history = (tab.undo.len(), tab.redo.len(), tab.ai.len());
        assert!(tab.reject_ai(change).is_err());
        assert_eq!(tab.document, before);
        assert_eq!(tab.version, version);
        assert_eq!((tab.undo.len(), tab.redo.len(), tab.ai.len()), history);
        Ok(())
    }
}
