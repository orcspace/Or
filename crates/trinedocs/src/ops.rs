use crate::native::Document;
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
        }
    }
    pub fn dirty(&self) -> bool {
        self.version != self.saved_version
    }
    pub fn apply(&mut self, op: Op) -> Result<(), String> {
        if self.read_only {
            return Err("Документ открыт только для чтения".into());
        }
        match op {
            Op::Replace {
                document,
                label,
                author,
                base_version,
            } => {
                if self.read_only {
                    return Err("Документ открыт только для чтения".into());
                }
                if base_version.is_some_and(|v| v != self.version) {
                    return Err(format!("conflict: текущая версия {}", self.version));
                }
                document.validate()?;
                if document == self.document {
                    return Ok(());
                }
                let group = matches!(author, Author::User)
                    && author == self.last_author
                    && self.last_edit.elapsed().as_secs_f32() < 1.0
                    && self.undo.last().is_some_and(|c| c.label == label);
                self.version = self.version.checked_add(1).ok_or("Исчерпаны версии")?;
                let change = Change {
                    id: self.version,
                    before: self.document.clone(),
                    after: document.clone(),
                    label,
                    author: author.clone(),
                    accepted: false,
                };
                if group {
                    if let Some(last) = self.undo.last_mut() {
                        last.after = document.clone();
                    }
                } else {
                    self.undo.push(change.clone());
                }
                if matches!(author, Author::Agent { .. }) {
                    self.ai.push(change);
                }
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
                let c = self.undo.pop().ok_or("Нет правок для отмены")?;
                self.document = c.before.clone();
                self.redo.push(c);
                self.version = next_version;
                self.last_edit = Instant::now() - std::time::Duration::from_secs(2);
            }
            Op::Redo => {
                let next_version = self.version.checked_add(1).ok_or("Исчерпаны версии")?;
                let c = self.redo.pop().ok_or("Нет правок для повтора")?;
                self.document = c.after.clone();
                self.undo.push(c);
                self.version = next_version;
                self.last_edit = Instant::now() - std::time::Duration::from_secs(2);
            }
        }
        Ok(())
    }
    pub fn reject_ai(&mut self, id: u64) -> Result<(), String> {
        let c = self
            .ai
            .iter()
            .find(|c| c.id == id && !c.accepted)
            .cloned()
            .ok_or("Правка AI не найдена")?;
        if self.document != c.after {
            return Err(
                "Есть последующие правки: отмените их сначала, чтобы сохранить данные".into(),
            );
        }
        self.apply(Op::Replace {
            document: c.before,
            label: format!("Откат AI: {}", c.label),
            author: Author::User,
            base_version: None,
        })?;
        self.ai.retain(|c| c.id != id);
        Ok(())
    }
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
        assert_eq!(t.document, d);
        t.apply(Op::Redo)?;
        assert_eq!(t.document, edited);
        t.reject_ai(1)?;
        assert_eq!(t.document, d);
        Ok(())
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;
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
