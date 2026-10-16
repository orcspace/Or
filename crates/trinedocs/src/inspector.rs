use crate::{native::Document, sheets::model::Value, slides::model::ElementKind};
use std::sync::mpsc::{self, Receiver};

const LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Paragraph(u64),
    Cell { sheet: u64, row: u32, col: u32 },
    Element { slide: u64, element: u64 },
}
impl Target {
    pub fn available(self, document: &Document) -> bool {
        match (self, document) {
            (Self::Paragraph(id), Document::Docs(doc)) => doc.paragraphs.iter().any(|p| p.id == id),
            (Self::Cell { sheet, row, col }, Document::Sheets(book)) => {
                book.sheet_by_id(sheet).is_some_and(|s| {
                    !s.hidden
                        && !s.hidden_rows.contains(&row)
                        && !s.hidden_cols.contains(&col)
                        && !s.merges.iter().any(|range| {
                            range.contains(row, col) && (row != range.r0 || col != range.c0)
                        })
                        && s.cell(row, col).is_some()
                })
            }
            (Self::Element { slide, element }, Document::Slides(deck)) => deck
                .slide(slide)
                .is_some_and(|s| !s.hidden && s.element(element).is_some()),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    #[default]
    All,
    Structure,
    Images,
    Formulas,
    Bounds,
    Readability,
}
impl Category {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "structure" => Some(Self::Structure),
            "images" => Some(Self::Images),
            "formulas" => Some(Self::Formulas),
            "bounds" => Some(Self::Bounds),
            "readability" => Some(Self::Readability),
            _ => None,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Structure => "structure",
            Self::Images => "images",
            Self::Formulas => "formulas",
            Self::Bounds => "bounds",
            Self::Readability => "readability",
        }
    }

    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Structure,
        Self::Images,
        Self::Formulas,
        Self::Bounds,
        Self::Readability,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "Все замечания",
            Self::Structure => "Структура",
            Self::Images => "Изображения",
            Self::Formulas => "Формулы",
            Self::Bounds => "Границы слайда",
            Self::Readability => "Читаемость",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub category: Category,
    pub location: String,
    pub message: String,
    pub target: Target,
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub omitted: usize,
    pub scope: &'static str,
}
impl Report {
    pub fn filtered(&self, query: &str, category: Category) -> Vec<&Finding> {
        let query = query.trim().to_lowercase();
        self.findings
            .iter()
            .filter(|finding| {
                (category == Category::All || category == finding.category)
                    && (query.is_empty()
                        || finding.location.to_lowercase().contains(&query)
                        || finding.message.to_lowercase().contains(&query))
            })
            .collect()
    }
    fn push(
        &mut self,
        category: Category,
        location: String,
        message: impl Into<String>,
        target: Target,
    ) {
        if self.findings.len() < LIMIT {
            self.findings.push(Finding {
                category,
                location,
                message: message.into(),
                target,
            });
        } else {
            self.omitted += 1;
        }
    }
}
pub struct Inspection {
    pub id: u64,
    pub version: u64,
    pub title: String,
    pub report: Report,
}
impl Inspection {
    pub fn page(
        &self,
        query: &str,
        category: Category,
        index: usize,
        count: usize,
        current_version: u64,
    ) -> serde_json::Value {
        use serde_json::json;
        let filtered = self.report.filtered(query, category);
        let count = count.clamp(1, 100);
        let findings: Vec<_> = filtered.iter().skip(index).take(count).map(|finding| {
            let target = match finding.target {
                Target::Paragraph(id) => json!({"kind":"paragraph","paragraph_id":id}),
                Target::Cell { sheet, row, col } => json!({"kind":"cell","sheet_id":sheet,"row":row,"col":col}),
                Target::Element { slide, element } => json!({"kind":"element","slide_id":slide,"element_id":element}),
            };
            json!({"category":finding.category.code(),"location":finding.location,"message":finding.message,"target":target})
        }).collect();
        json!({"status":"completed","document_id":self.id,"version":self.version,"current_version":current_version,
            "stale":self.version != current_version,"scope":self.report.scope,
            "findings":findings,"offset":index,"page_size":count,"filtered_total":filtered.len(),
            "stored_total":self.report.findings.len(),"omitted":self.report.omitted,
            "truncated":self.report.omitted > 0})
    }

    pub fn copy_report(&self, query: &str, category: Category, unchanged: bool) -> String {
        use std::fmt::Write;
        let short = |text: &str, limit: usize| text.chars().take(limit).collect::<String>();
        let findings = self.report.filtered(query, category);
        let mut text = format!(
            "Проверка: {}\nДокумент {}, версия {}\n{}\nОбласть: {}\nФильтр: {}; поиск: {}\nСовпадений среди сохранённых замечаний: {} из {}. Не включено в отчёт: {}.\n",
            short(&self.title, 200),
            self.id,
            self.version,
            if unchanged {
                "Снимок актуален."
            } else {
                "УСТАРЕЛО: документ изменён или закрыт после проверки."
            },
            short(self.report.scope, 500),
            category.label(),
            short(query, 160),
            findings.len(),
            self.report.findings.len(),
            self.report.omitted
        );
        for finding in findings {
            let _ = writeln!(
                text,
                "\n{} — {}\n{}",
                finding.category.label(),
                short(&finding.location, 512),
                short(&finding.message, 512)
            );
        }
        text
    }
}

pub struct Task {
    pub id: u64,
    pub version: u64,
    result: Receiver<Inspection>,
}
impl Task {
    pub fn start(
        id: u64,
        version: u64,
        title: String,
        document: Document,
        ctx: egui::Context,
    ) -> Result<Self, String> {
        let (tx, result) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("document-inspector".into())
            .spawn(move || {
                let report = inspect(&document);
                let _ = tx.send(Inspection {
                    id,
                    version,
                    title,
                    report,
                });
                ctx.request_repaint();
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            id,
            version,
            result,
        })
    }
    pub fn poll(&self) -> Option<Result<Inspection, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(Ok(result)),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Проверка документа прервана".into()))
            }
        }
    }
}
pub fn inspect(document: &Document) -> Report {
    let mut report = Report::default();
    match document {
        Document::Docs(doc) => {
            report.scope = "Уровни и пустые заголовки, описания изображений.";
            let mut previous = 0;
            for (index, paragraph) in doc.paragraphs.iter().enumerate() {
                if let Some(level) = paragraph
                    .style
                    .strip_prefix("Heading")
                    .and_then(|level| level.parse::<u8>().ok())
                    .filter(|level| (1..=6).contains(level))
                {
                    let location = format!("Абзац {}", index + 1);
                    if paragraph.text().trim().is_empty() {
                        report.push(
                            Category::Structure,
                            location.clone(),
                            "Пустой заголовок.",
                            Target::Paragraph(paragraph.id),
                        );
                    }
                    if level > previous + 1 {
                        report.push(
                            Category::Structure,
                            location,
                            format!("Пропущен уровень заголовка: {previous} → {level}."),
                            Target::Paragraph(paragraph.id),
                        );
                    }
                    previous = level;
                }
            }
            for (index, image) in doc.images.iter().enumerate() {
                if image.alt.trim().is_empty() {
                    report.push(
                        Category::Images,
                        format!("Изображение {}", index + 1),
                        "Добавьте описание изображения.",
                        Target::Paragraph(image.after_paragraph_id),
                    );
                }
            }
        }
        Document::Sheets(book) => {
            report.scope = "Ошибки значений и формул на всех листах, включая скрытые.";
            for sheet in &book.sheets {
                for (address, cell) in &sheet.cells {
                    if let Value::Error(error) = &cell.value {
                        let name: String = sheet.name.chars().take(80).collect();
                        let error: String = error.chars().take(120).collect();
                        if let Some((row, col)) = crate::sheets::model::parse_address(address) {
                            report.push(
                                Category::Formulas,
                                format!("{name}!{address}"),
                                format!("Ошибка ячейки: {error}."),
                                Target::Cell {
                                    sheet: sheet.id,
                                    row,
                                    col,
                                },
                            );
                        }
                    }
                }
            }
        }
        Document::Slides(deck) => {
            report.scope = "Выход объектов за слайд; мелкий шрифт, контраст и переполнение обычного текста. Это оценка, без таблиц, диаграмм и фона-изображения.";
            for (slide_index, slide) in deck.slides.iter().enumerate() {
                if slide.hidden {
                    continue;
                }
                for (element_index, element) in slide.elements.iter().enumerate() {
                    let location =
                        format!("Слайд {}, объект {}", slide_index + 1, element_index + 1);
                    let target = Target::Element {
                        slide: slide.id,
                        element: element.id,
                    };
                    if element.x < 0.0
                        || element.y < 0.0
                        || element.x + element.w > deck.width
                        || element.y + element.h > deck.height
                    {
                        report.push(
                            Category::Bounds,
                            location.clone(),
                            "Объект выходит за границы слайда.",
                            target,
                        );
                    }
                    if element.text.trim().is_empty()
                        || !matches!(
                            &element.kind,
                            ElementKind::Text | ElementKind::Rect | ElementKind::Ellipse
                        )
                    {
                        continue;
                    }
                    if element.font_size < 18.0 {
                        report.push(
                            Category::Readability,
                            location.clone(),
                            "Шрифт меньше 18 pt: проверьте читаемость при показе.",
                            target,
                        );
                    }
                    let background = if matches!(&element.kind, ElementKind::Text) {
                        slide.background
                    } else {
                        element.fill
                    };
                    let required = if element.font_size >= 24.0
                        || element.bold && element.font_size >= 18.66
                    {
                        3.0
                    } else {
                        4.5
                    };
                    if contrast(element.color, background) < required {
                        report.push(
                            Category::Readability,
                            location.clone(),
                            "Низкий контраст текста с фоном.",
                            target,
                        );
                    }
                    let lines = crate::slides::model::text_lines(element).len();
                    if lines as f32 * element.font_size * 1.2 + 10.0 > element.h {
                        report.push(
                            Category::Readability,
                            location,
                            "Текст может не помещаться в рамку.",
                            target,
                        );
                    }
                }
            }
        }
    }
    report
}
fn contrast(first: [u8; 3], second: [u8; 3]) -> f64 {
    let luminance = |color: [u8; 3]| {
        let [r, g, b] = color.map(|channel| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let a = luminance(first);
    let b = luminance(second);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn docs_structure_checks_do_not_modify_the_model() {
        let mut doc = crate::docs::model::Doc::from_text("Title\n\nBody");
        doc.paragraphs[0].style = "Heading1".into();
        doc.paragraphs[1].style = "Heading3".into();
        let document = Document::Docs(doc);
        let before = document.clone();
        let report = inspect(&document);
        assert_eq!(report.findings.len(), 2);
        assert_eq!(document, before);
    }
    #[test]
    fn report_bounds_results_without_hiding_the_total() {
        let mut doc = crate::docs::model::Doc::from_text(&"\n".repeat(250));
        for paragraph in &mut doc.paragraphs {
            paragraph.style = "Heading1".into();
        }
        let expected = doc.paragraphs.len();
        let report = inspect(&Document::Docs(doc));
        assert_eq!(report.findings.len(), LIMIT);
        assert_eq!(report.findings.len() + report.omitted, expected);
    }
    #[test]
    fn sheets_report_includes_errors_on_hidden_sheets() -> Result<(), String> {
        let mut book = crate::sheets::model::Book::default();
        let id = book.add_sheet("Hidden")?;
        book.set_cell(id, 0, 0, "=1/0")?;
        book.sheet_by_id_mut(id).ok_or("sheet")?.hidden = true;
        let report = inspect(&Document::Sheets(book));
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].location, "Hidden!A1");
        Ok(())
    }
    #[test]
    fn slides_ignore_hidden_slides_and_check_actual_text() -> Result<(), String> {
        let mut deck = crate::slides::model::Deck::default();
        let slide = deck.slides[0].id;
        let element = deck.add_element(slide, ElementKind::Text)?;
        let item = deck
            .slide_mut(slide)
            .and_then(|slide| slide.element_mut(element))
            .ok_or("element")?;
        item.text = "Text".into();
        item.color = [255; 3];
        item.font_size = 12.0;
        item.h = 1.0;
        assert!(inspect(&Document::Slides(deck.clone())).findings.len() >= 3);
        deck.slide_mut(slide).ok_or("slide")?.hidden = true;
        assert!(inspect(&Document::Slides(deck)).findings.is_empty());
        assert!((contrast([0; 3], [255; 3]) - 21.0).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn hidden_and_merged_children_are_not_navigation_targets() -> Result<(), String> {
        let mut book = crate::sheets::model::Book::default();
        let sheet = book.active_id();
        book.set_range(sheet, 0, 0, &[vec!["".into(), "".into()]])?;
        book.merge(sheet, crate::sheets::model::Range::new(0, 0, 0, 1))?;
        let anchor = Target::Cell {
            sheet,
            row: 0,
            col: 0,
        };
        let child = Target::Cell {
            sheet,
            row: 0,
            col: 1,
        };
        assert!(anchor.available(&Document::Sheets(book.clone())));
        assert!(!child.available(&Document::Sheets(book.clone())));
        book.sheet_by_id_mut(sheet)
            .ok_or("sheet")?
            .hidden_rows
            .insert(0);
        assert!(!anchor.available(&Document::Sheets(book)));
        Ok(())
    }

    #[test]
    fn filtered_copy_preserves_staleness_scope_and_omission_information() {
        let mut report = Report {
            scope: "Проверяются только заголовки.",
            omitted: 8,
            ..Default::default()
        };
        report.push(
            Category::Structure,
            "Абзац 7".into(),
            "Пропущен уровень.",
            Target::Paragraph(7),
        );
        report.push(
            Category::Images,
            "Изображение 2".into(),
            "Нет описания.",
            Target::Paragraph(7),
        );
        assert_eq!(report.filtered("АБЗАЦ", Category::All).len(), 1);
        assert!(report.filtered("АБЗАЦ", Category::Images).is_empty());
        let inspection = Inspection {
            id: 4,
            version: 9,
            title: "Document".into(),
            report,
        };
        let copy = inspection.copy_report("УРОВЕНЬ", Category::Structure, false);
        assert!(copy.contains("УСТАРЕЛО") && copy.contains("версия 9"));
        assert!(copy.contains("Не включено в отчёт: 8"));
        assert!(copy.contains("Пропущен уровень") && !copy.contains("Нет описания"));
        assert!(copy.contains("Проверяются только заголовки"));
    }
}
