//! Function hints for the formula bar: name completion and the signature of the call the
//! cursor is in. Pure text analysis, independent of the UI.

use super::formula::RU_NAMES;
use super::xlfn;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionInfo {
    pub name: &'static str,
    pub signature: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

/// Functions the engine evaluates itself that the library catalog does not describe.
const ENGINE_ONLY: &[FunctionInfo] = &[
    FunctionInfo {
        name: "COLUMN",
        signature: "COLUMN([reference])",
        description: "Номер столбца ссылки или текущей ячейки.",
        category: "Ссылки и массивы",
    },
    FunctionInfo {
        name: "ISFORMULA",
        signature: "ISFORMULA(reference)",
        description: "ИСТИНА, если ячейка содержит формулу.",
        category: "Проверка свойств",
    },
    FunctionInfo {
        name: "ROW",
        signature: "ROW([reference])",
        description: "Номер строки ссылки или текущей ячейки.",
        category: "Ссылки и массивы",
    },
    FunctionInfo {
        name: "SUBTOTAL",
        signature: "SUBTOTAL(function_num, ref1, [ref2], ...)",
        description: "Промежуточный итог диапазона: 1–11 с учётом скрытых строк, 101–111 без них.",
        category: "Математические",
    },
    FunctionInfo {
        name: "TEXT",
        signature: "TEXT(value, format_text)",
        description: "Число в виде текста по формату, например \"0,00\" или \"ДД.ММ.ГГГГ\".",
        category: "Текстовые",
    },
];

fn category(category: xlfn::Category) -> &'static str {
    use xlfn::Category::*;
    match category {
        Financial => "Финансовые",
        Logical => "Логические",
        Text => "Текстовые",
        DateTime => "Дата и время",
        Lookup => "Ссылки и массивы",
        MathTrig => "Математические",
        Statistical => "Статистические",
        Engineering => "Инженерные",
        Information => "Проверка свойств",
        Database => "Базы данных",
        Compatibility => "Совместимость",
        Web => "Веб",
        Cube => "Кубы",
        AddIn => "Надстройки",
    }
}

/// Every function the formula bar can suggest, sorted by name.
pub fn catalog() -> &'static [FunctionInfo] {
    static CATALOG: OnceLock<Vec<FunctionInfo>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut all: Vec<FunctionInfo> = xlfn::all()
            .iter()
            .map(|spec| FunctionInfo {
                name: spec.name,
                signature: spec.signature,
                description: spec.description,
                category: category(spec.category),
            })
            .collect();
        for extra in ENGINE_ONLY {
            if !all.iter().any(|f| f.name == extra.name) {
                all.push(*extra);
            }
        }
        all.sort_by(|a, b| a.name.cmp(b.name));
        all.dedup_by(|a, b| a.name == b.name);
        all
    })
}

pub fn find(name: &str) -> Option<&'static FunctionInfo> {
    let upper = name.to_uppercase();
    let upper = upper.strip_prefix("_XLFN.").unwrap_or(&upper);
    let canonical = RU_NAMES
        .iter()
        .find(|(ru, _)| *ru == upper)
        .map_or(upper, |(_, en)| *en);
    catalog().iter().find(|f| f.name == canonical)
}

/// A completion: the text to insert and the function it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub insert: &'static str,
    pub info: &'static FunctionInfo,
}

/// What the formula bar should show for `text` with the caret at char index `cursor`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hint {
    /// Char range of the name being typed, replaced when a completion is accepted.
    pub word: Option<(usize, usize)>,
    pub completions: Vec<Completion>,
    /// Innermost function call around the caret and the zero-based argument index.
    pub call: Option<(&'static FunctionInfo, usize)>,
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

/// At most `limit` functions whose English or Russian name starts with `prefix`.
pub fn completions(prefix: &str, limit: usize) -> Vec<Completion> {
    let prefix = prefix.to_uppercase();
    if prefix.is_empty() {
        return Vec::new();
    }
    let cyrillic = prefix.chars().any(|c| matches!(c, 'А'..='я' | 'Ё' | 'ё'));
    let mut out: Vec<Completion> = Vec::new();
    if cyrillic {
        for &(ru, en) in RU_NAMES {
            if ru.starts_with(&prefix)
                && let Some(info) = catalog().iter().find(|f| f.name == en)
                && !out.iter().any(|c| c.info.name == info.name)
            {
                out.push(Completion { insert: ru, info });
            }
        }
        out.sort_by_key(|c| (c.insert.chars().count(), c.insert));
    } else {
        out.extend(
            catalog()
                .iter()
                .filter(|f| f.name.starts_with(&prefix))
                .map(|info| Completion {
                    insert: info.name,
                    info,
                }),
        );
        // Exact match first, then shorter names: `SUM` before `SUMIF`.
        out.sort_by_key(|c| (c.insert != prefix, c.insert.len(), c.insert));
    }
    out.truncate(limit);
    out
}

pub fn hint(text: &str, cursor: usize, limit: usize) -> Hint {
    let mut result = Hint::default();
    if !text.starts_with('=') {
        return result;
    }
    let chars: Vec<char> = text.chars().take(cursor).collect();
    // Stack of open calls: the function (if the parenthesis follows a known name) and argument.
    let mut stack: Vec<(Option<&'static FunctionInfo>, usize)> = Vec::new();
    let mut quote: Option<char> = None;
    let mut i = 1;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            if c == q {
                if chars.get(i + 1) == Some(&q) {
                    i += 1;
                } else {
                    quote = None;
                }
            }
        } else {
            match c {
                '"' | '\'' => quote = Some(c),
                '(' => {
                    let end = i;
                    let mut start = end;
                    while start > 1 && is_name_char(chars[start - 1]) {
                        start -= 1;
                    }
                    let name: String = chars[start..end].iter().collect();
                    stack.push((find(&name), 0));
                }
                ')' => {
                    stack.pop();
                }
                ',' | ';' => {
                    if let Some(top) = stack.last_mut() {
                        top.1 += 1;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    result.call = stack
        .iter()
        .rev()
        .find_map(|(info, arg)| info.map(|info| (info, *arg)));
    if quote.is_some() {
        return result;
    }
    let end = chars.len();
    let mut start = end;
    while start > 1 && is_name_char(chars[start - 1]) {
        start -= 1;
    }
    let word: String = chars[start..end].iter().collect();
    let before = start.checked_sub(1).and_then(|i| chars.get(i)).copied();
    // Not after a sheet name, a range colon or an absolute-reference dollar; must start with a letter.
    if word.chars().next().is_some_and(char::is_alphabetic)
        && !matches!(before, Some('!' | ':' | '$'))
        && text.chars().nth(end) != Some('(')
    {
        result.completions = completions(&word, limit);
        if !result.completions.is_empty() {
            result.word = Some((start, end));
        }
    }
    result
}

/// Replaces the typed name with `completion` and an opening parenthesis; returns the new text
/// and caret position (char index).
pub fn accept(text: &str, word: (usize, usize), completion: &Completion) -> (String, usize) {
    let (start, end) = word;
    let mut out: String = text.chars().take(start).collect();
    out.push_str(completion.insert);
    out.push('(');
    let caret = out.chars().count();
    out.extend(text.chars().skip(end));
    (out, caret)
}

/// Splits a signature such as `IF(test, [a], [b])` into the name and its parameters, so the UI
/// can emphasise the current one.
pub fn parameters(signature: &str) -> (&str, Vec<&str>) {
    let Some((name, rest)) = signature.split_once('(') else {
        return (signature, Vec::new());
    };
    let inner = rest.strip_suffix(')').unwrap_or(rest);
    let params = inner
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    (name, params)
}

/// The parameter index to emphasise for argument `arg`, following a trailing `...` repetition.
pub fn parameter_index(params: &[&str], arg: usize) -> Option<usize> {
    if params.is_empty() {
        return None;
    }
    if arg < params.len() {
        if params[arg] == "..." {
            return arg.checked_sub(1);
        }
        return Some(arg);
    }
    match params.last() {
        Some(&"...") => params.len().checked_sub(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(hint: &Hint) -> Vec<&str> {
        hint.completions.iter().map(|c| c.insert).collect()
    }

    #[test]
    fn catalog_covers_engine_and_library() {
        for name in [
            "SUM",
            "IF",
            "VLOOKUP",
            "NORM.S.DIST",
            "ROW",
            "TEXT",
            "SUBTOTAL",
        ] {
            assert!(find(name).is_some(), "{name}");
        }
        assert_eq!(find("суммесли").map(|f| f.name), Some("SUMIF"));
        assert_eq!(find("_xlfn.xlookup").map(|f| f.name), Some("XLOOKUP"));
        let all = catalog();
        assert!(all.len() > 450);
        assert!(all.windows(2).all(|w| w[0].name < w[1].name));
    }

    #[test]
    fn completes_names_by_prefix() {
        let h = hint("=SU", 3, 8);
        assert_eq!(h.word, Some((1, 3)));
        assert_eq!(names(&h).first(), Some(&"SUM"));
        assert!(names(&h).contains(&"SUMIF"));
        assert!(names(&h).iter().all(|n| n.starts_with("SU")));
        let h = hint("=sum", 4, 8);
        assert_eq!(names(&h).first(), Some(&"SUM"));
        let h = hint("=1+ВП", 5, 8);
        assert_eq!(names(&h), vec!["ВПР"]);
        assert_eq!(h.completions[0].info.name, "VLOOKUP");
    }

    #[test]
    fn no_completions_where_names_do_not_belong() {
        assert!(hint("SU", 2, 8).completions.is_empty());
        assert!(hint("=\"SU", 4, 8).completions.is_empty());
        assert!(hint("=A1:SU", 6, 8).completions.is_empty());
        assert!(hint("=Sheet1!SU", 10, 8).completions.is_empty());
        assert!(hint("=12", 3, 8).completions.is_empty());
        assert!(hint("=SUM(", 5, 8).completions.is_empty());
    }

    #[test]
    fn tracks_the_innermost_call_and_argument() {
        let h = hint("=IF(A1>0, SUM(B1:B3; C1), ", 26, 8);
        let (info, arg) = h.call.unwrap_or((&ENGINE_ONLY[0], 99));
        assert_eq!((info.name, arg), ("IF", 2));
        let h = hint("=IF(A1>0, SUM(B1:B3; C1", 23, 8);
        let (info, arg) = h.call.unwrap_or((&ENGINE_ONLY[0], 99));
        assert_eq!((info.name, arg), ("SUM", 1));
        let h = hint("=ЕСЛИ(\"a,b\", ", 12, 8);
        let (info, arg) = h.call.unwrap_or((&ENGINE_ONLY[0], 99));
        assert_eq!((info.name, arg), ("IF", 1));
        assert_eq!(hint("=(1+2", 5, 8).call, None);
    }

    #[test]
    fn accepting_inserts_name_and_parenthesis() {
        let h = hint("=1+su*2", 5, 8);
        let first = h.completions[0];
        let (text, caret) = accept("=1+su*2", h.word.unwrap_or((0, 0)), &first);
        assert_eq!(text, "=1+SUM(*2");
        assert_eq!(caret, 7);
    }

    #[test]
    fn splits_signature_parameters() {
        let (name, params) = parameters("SUM(number1, [number2], ...)");
        assert_eq!(name, "SUM");
        assert_eq!(params, vec!["number1", "[number2]", "..."]);
        assert_eq!(parameter_index(&params, 0), Some(0));
        assert_eq!(parameter_index(&params, 1), Some(1));
        assert_eq!(parameter_index(&params, 5), Some(1));
        let (_, params) = parameters("PI()");
        assert_eq!(parameter_index(&params, 0), None);
        let (_, params) = parameters("ABS(number)");
        assert_eq!(parameter_index(&params, 3), None);
    }
}
