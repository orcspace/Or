use super::model::{Book, MAX_COLS, MAX_ROWS, Range, Value, address, parse_address};
use chrono::{Datelike, Timelike};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

type CalcResult<T> = Result<T, String>;
#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Number(f64),
    Text(String),
    Word(String),
    Quoted(String),
    Op(String),
    End,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}
fn lexer(input: &str) -> CalcResult<Vec<Token>> {
    let mut out = Vec::new();
    let mut i = usize::from(input.starts_with('='));
    while i < input.len() {
        let ch = input
            .get(i..)
            .and_then(|s| s.chars().next())
            .ok_or("#VALUE!")?;
        if ch.is_whitespace() {
            i += ch.len_utf8();
            continue;
        }
        let start = i;
        let kind = if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut text = String::new();
            let mut closed = false;
            i += 1;
            while i < input.len() {
                let c = input
                    .get(i..)
                    .and_then(|s| s.chars().next())
                    .ok_or("#VALUE!")?;
                i += c.len_utf8();
                if c == quote {
                    if input.get(i..).is_some_and(|s| s.starts_with(quote)) {
                        text.push(c);
                        i += 1;
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    text.push(c);
                }
            }
            if !closed {
                return Err("#VALUE!".into());
            }
            if quote == '"' {
                TokenKind::Text(text)
            } else {
                TokenKind::Quoted(text)
            }
        } else if ch.is_ascii_digit()
            || (ch == '.'
                && input
                    .get(i + 1..)
                    .and_then(|s| s.chars().next())
                    .is_some_and(|c| c.is_ascii_digit()))
        {
            i += ch.len_utf8();
            while input
                .as_bytes()
                .get(i)
                .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                i += 1;
            }
            if input
                .as_bytes()
                .get(i)
                .is_some_and(|c| *c == b'e' || *c == b'E')
            {
                i += 1;
                if input
                    .as_bytes()
                    .get(i)
                    .is_some_and(|c| *c == b'+' || *c == b'-')
                {
                    i += 1;
                }
                while input.as_bytes().get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
            }
            let n = input
                .get(start..i)
                .ok_or("#VALUE!")?
                .parse::<f64>()
                .map_err(|_| "#VALUE!")?;
            if !n.is_finite() {
                return Err("#NUM!".into());
            }
            TokenKind::Number(n)
        } else if ch.is_alphabetic() || matches!(ch, '_' | '$' | '#' | '\\') {
            i += ch.len_utf8();
            while i < input.len() {
                let c = input
                    .get(i..)
                    .and_then(|s| s.chars().next())
                    .ok_or("#VALUE!")?;
                if c.is_alphanumeric()
                    || c == '_'
                    || c == '.'
                    || c == '$'
                    || (ch == '#' && (c == '!' || c == '/' || c == '?'))
                {
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            TokenKind::Word(input.get(start..i).ok_or("#VALUE!")?.into())
        } else {
            i += ch.len_utf8();
            let mut op = ch.to_string();
            if (ch == '<' || ch == '>') && input.as_bytes().get(i) == Some(&b'=') {
                op.push('=');
                i += 1;
            } else if ch == '<' && input.as_bytes().get(i) == Some(&b'>') {
                op.push('>');
                i += 1;
            }
            if !matches!(
                op.as_str(),
                "+" | "-"
                    | "*"
                    | "/"
                    | "^"
                    | "&"
                    | "%"
                    | "="
                    | "<"
                    | ">"
                    | "<="
                    | ">="
                    | "<>"
                    | "("
                    | ")"
                    | ","
                    | ";"
                    | ":"
                    | "!"
            ) {
                return Err("#VALUE!".into());
            }
            TokenKind::Op(op)
        };
        out.push(Token {
            kind,
            start,
            end: i,
        });
        if out.len() > 1024 {
            return Err("#NUM!".into());
        }
    }
    out.push(Token {
        kind: TokenKind::End,
        start: input.len(),
        end: input.len(),
    });
    Ok(out)
}
fn alias(name: &str) -> String {
    let upper = name.to_uppercase();
    let canonical = upper.strip_prefix("_XLFN.").unwrap_or(&upper);
    let canonical = canonical.strip_prefix("_XLWS.").unwrap_or(canonical);
    match canonical {
        "СУММ" => "SUM",
        "СРЗНАЧ" => "AVERAGE",
        "МИН" => "MIN",
        "МАКС" => "MAX",
        "СЧЁТ" | "СЧЕТ" => "COUNT",
        "СЧЁТЗ" | "СЧЕТЗ" => "COUNTA",
        "СЧИТАТЬПУСТОТЫ" => "COUNTBLANK",
        "ЕСЛИ" => "IF",
        "ЕСЛИОШИБКА" => "IFERROR",
        "ЕСНД" => "IFNA",
        "И" => "AND",
        "ИЛИ" => "OR",
        "НЕ" => "NOT",
        "ИСКЛИЛИ" => "XOR",
        "ВПР" => "VLOOKUP",
        "ГПР" => "HLOOKUP",
        "ПРОСМОТРX" | "ПРОСМОТРХ" => "XLOOKUP",
        "ИНДЕКС" => "INDEX",
        "ПОИСКПОЗ" => "MATCH",
        "ПОИСКПОЗX" | "ПОИСКПОЗХ" => "XMATCH",
        "СУММЕСЛИ" => "SUMIF",
        "СУММЕСЛИМН" => "SUMIFS",
        "СЧЁТЕСЛИ" | "СЧЕТЕСЛИ" => "COUNTIF",
        "СЧЁТЕСЛИМН" | "СЧЕТЕСЛИМН" => "COUNTIFS",
        "СРЗНАЧЕСЛИ" => "AVERAGEIF",
        "СРЗНАЧЕСЛИМН" => "AVERAGEIFS",
        "ОКРУГЛ" => "ROUND",
        "ОКРУГЛВВЕРХ" => "ROUNDUP",
        "ОКРУГЛВНИЗ" => "ROUNDDOWN",
        "ЦЕЛОЕ" => "INT",
        "ОСТАТ" => "MOD",
        "СТЕПЕНЬ" => "POWER",
        "КОРЕНЬ" => "SQRT",
        "ПИ" => "PI",
        "СЛЧИС" => "RAND",
        "СЛУЧМЕЖДУ" => "RANDBETWEEN",
        "ПРОИЗВЕД" => "PRODUCT",
        "СУММПРОИЗВ" => "SUMPRODUCT",
        "СЦЕПИТЬ" => "CONCATENATE",
        "СЦЕП" => "CONCAT",
        "ОБЪЕДИНИТЬ" => "TEXTJOIN",
        "ЛЕВСИМВ" => "LEFT",
        "ПРАВСИМВ" => "RIGHT",
        "ПСТР" => "MID",
        "ДЛСТР" => "LEN",
        "СТРОЧН" => "LOWER",
        "ПРОПИСН" => "UPPER",
        "ПРОПНАЧ" => "PROPER",
        "СЖПРОБЕЛЫ" => "TRIM",
        "ПОДСТАВИТЬ" => "SUBSTITUTE",
        "ЗАМЕНИТЬ" => "REPLACE",
        "НАЙТИ" => "FIND",
        "ПОИСК" => "SEARCH",
        "ТЕКСТ" => "TEXT",
        "ЗНАЧЕН" => "VALUE",
        "ПОВТОР" => "REPT",
        "СОВПАД" => "EXACT",
        "СЕГОДНЯ" => "TODAY",
        "ТДАТА" => "NOW",
        "ДАТА" => "DATE",
        "ВРЕМЯ" => "TIME",
        "ГОД" => "YEAR",
        "МЕСЯЦ" => "MONTH",
        "ДЕНЬ" => "DAY",
        "ЧАС" => "HOUR",
        "МИНУТЫ" => "MINUTE",
        "СЕКУНДЫ" => "SECOND",
        "ДЕНЬНЕД" => "WEEKDAY",
        "ДАТАМЕС" => "EDATE",
        "КОНМЕСЯЦА" => "EOMONTH",
        "РАЗНДАТ" => "DATEDIF",
        "ЧИСТРАБДНИ" => "NETWORKDAYS",
        "РАБДЕНЬ" => "WORKDAY",
        "ПЛТ" => "PMT",
        "БС" => "FV",
        "ПС" => "PV",
        "ЧПС" => "NPV",
        "ВСД" => "IRR",
        "ЕЧИСЛО" => "ISNUMBER",
        "ЕТЕКСТ" => "ISTEXT",
        "ЕПУСТО" => "ISBLANK",
        "ЕОШИБКА" => "ISERROR",
        "ЕНД" => "ISNA",
        "ЕОШ" => "ISERR",
        "ЕЛОГИЧ" => "ISLOGICAL",
        "СТРОКА" => "ROW",
        "СТОЛБЕЦ" => "COLUMN",
        "ЧСТРОК" => "ROWS",
        "ЧИСЛСТОЛБ" => "COLUMNS",
        "МЕДИАНА" => "MEDIAN",
        "МОДА" => "MODE",
        "СТАНДОТКЛОН" => "STDEV",
        "ДИСП" => "VAR",
        "ВЫБОР" => "CHOOSE",
        "ПЕРЕКЛЮЧ" => "SWITCH",
        "ИСТИНА" => "TRUE",
        "ЛОЖЬ" => "FALSE",
        "НД" => "NA",
        "МАКСЕСЛИ" => "MAXIFS",
        "МИНЕСЛИ" => "MINIFS",
        "МАКСА" => "MAXA",
        "МИНА" => "MINA",
        "УРЕЗСРЕДНЕЕ" => "TRIMMEAN",
        "СКОС" => "SKEW",
        "ЭКСЦЕСС" => "KURT",
        "КОРРЕЛ" => "CORREL",
        "ПИРСОН" => "PEARSON",
        "КВПИРСОН" => "RSQ",
        "НАКЛОН" => "SLOPE",
        "ОТРЕЗОК" => "INTERCEPT",
        "КОВАР" => "COVAR",
        "КОВАРИАЦИЯ.Г" => "COVARIANCE.P",
        "КОВАРИАЦИЯ.В" => "COVARIANCE.S",
        "ТЕКСТДО" => "TEXTBEFORE",
        "ТЕКСТПОСЛЕ" => "TEXTAFTER",
        "КПЕР" => "NPER",
        "СТАВКА" => "RATE",
        "ПРПЛТ" => "IPMT",
        "ОСПЛТ" => "PPMT",
        "РИМСКОЕ" => "ROMAN",
        "АРАБСКОЕ" => "ARABIC",
        "ДЕС.В.ДВ" => "DEC2BIN",
        "ДЕС.В.ВОСЬМ" => "DEC2OCT",
        "ДЕС.В.ШЕСТН" => "DEC2HEX",
        "ДВ.В.ДЕС" => "BIN2DEC",
        "ВОСЬМ.В.ДЕС" => "OCT2DEC",
        "ШЕСТН.В.ДЕС" => "HEX2DEC",
        "ОСНОВАНИЕ" => "BASE",
        "ДЕС" => "DECIMAL",
        "НОМНЕДЕЛИ" => "WEEKNUM",
        "НОМНЕДЕЛИ.ISO" => "ISOWEEKNUM",
        "ДНЕЙ360" => "DAYS360",
        "ПРОМЕЖУТОЧНЫЕ.ИТОГИ" => "SUBTOTAL",
        _ => return canonical.into(),
    }
    .into()
}
pub fn canonicalize(input: &str) -> String {
    if !input.starts_with('=') {
        return input.into();
    }
    let Ok(tokens) = lexer(input) else {
        return input.into();
    };
    let mut changes = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if let TokenKind::Word(w) = &t.kind
            && tokens
                .get(i + 1)
                .is_some_and(|t| t.kind == TokenKind::Op("(".into()))
        {
            changes.push((t.start, t.end, alias(w)));
        }
        if t.kind == TokenKind::Op(";".into()) {
            changes.push((t.start, t.end, ",".into()));
        }
    }
    replace_spans(input, changes)
}
fn replace_spans(input: &str, changes: Vec<(usize, usize, String)>) -> String {
    let mut result = input.to_string();
    for (start, end, replacement) in changes.into_iter().rev() {
        if start <= end && result.is_char_boundary(start) && result.is_char_boundary(end) {
            result.replace_range(start..end, &replacement);
        }
    }
    result
}
fn token_reference(
    tokens: &[Token],
    index: usize,
) -> Option<(u32, u32, bool, bool, Option<String>)> {
    let t = tokens.get(index)?;
    let TokenKind::Word(w) = &t.kind else {
        return None;
    };
    if tokens
        .get(index + 1)
        .is_some_and(|t| matches!(&t.kind,TokenKind::Op(o)if o=="("||o=="!"))
    {
        return None;
    }
    let (r, c) = parse_address(w)?;
    let ca = w.starts_with('$');
    let ra = w.chars().skip(1).any(|c| c == '$');
    let sheet = if index >= 2
        && tokens
            .get(index - 1)
            .is_some_and(|t| t.kind == TokenKind::Op("!".into()))
    {
        match &tokens.get(index - 2)?.kind {
            TokenKind::Word(s) | TokenKind::Quoted(s) => Some(s.clone()),
            _ => None,
        }
    } else if index >= 2
        && tokens
            .get(index - 1)
            .is_some_and(|t| t.kind == TokenKind::Op(":".into()))
    {
        token_reference(tokens, index - 2).and_then(|x| x.4)
    } else {
        None
    };
    Some((r, c, ra, ca, sheet))
}
fn reference_text(r: i64, c: i64, ra: bool, ca: bool) -> String {
    if r < 0 || c < 0 || r >= i64::from(MAX_ROWS) || c >= i64::from(MAX_COLS) {
        return "#REF!".into();
    }
    let a = address(r as u32, c as u32);
    let split = a.find(|ch: char| ch.is_ascii_digit()).unwrap_or(a.len());
    format!(
        "{}{}{}{}",
        if ca { "$" } else { "" },
        a.get(..split).unwrap_or(""),
        if ra { "$" } else { "" },
        a.get(split..).unwrap_or("")
    )
}
pub fn shift_references(input: &str, rows: i64, cols: i64) -> String {
    if !input.starts_with('=') {
        return input.into();
    }
    let Ok(tokens) = lexer(input) else {
        return input.into();
    };
    let mut changes = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if let Some((r, c, ra, ca, _)) = token_reference(&tokens, i) {
            changes.push((
                t.start,
                t.end,
                i64::from(r)
                    .checked_add(if ra { 0 } else { rows })
                    .zip(i64::from(c).checked_add(if ca { 0 } else { cols }))
                    .map(|(r, c)| reference_text(r, c, ra, ca))
                    .unwrap_or_else(|| "#REF!".into()),
            ));
        }
    }
    replace_spans(input, changes)
}
pub(super) fn shift_bounds(
    start: u32,
    end: u32,
    index: u32,
    count: u32,
    delete: bool,
) -> Option<(u32, u32)> {
    let stop = index.checked_add(count)?;
    if !delete {
        return Some((
            if start >= index {
                start.checked_add(count)?
            } else {
                start
            },
            if end >= index {
                end.checked_add(count)?
            } else {
                end
            },
        ));
    }
    if start >= index && end < stop {
        return None;
    }
    let first = if start >= stop {
        start - count
    } else if start >= index {
        index
    } else {
        start
    };
    let last = if end >= stop {
        end - count
    } else if end >= index {
        index.checked_sub(1)?
    } else {
        end
    };
    Some((first, last))
}
pub fn shift_structural(
    input: &str,
    current: &str,
    changed: &str,
    index: u32,
    count: u32,
    rows: bool,
    delete: bool,
) -> String {
    let Ok(tokens) = lexer(input) else {
        return input.into();
    };
    let mut changes = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((r, c, ra, ca, sheet)) = token_reference(&tokens, i) {
            if sheet.as_deref().unwrap_or(current).to_lowercase() != changed.to_lowercase() {
                i += 1;
                continue;
            }
            let ranged = tokens
                .get(i + 1)
                .is_some_and(|t| t.kind == TokenKind::Op(":".into()));
            if ranged && let Some((r2, c2, ra2, ca2, _)) = token_reference(&tokens, i + 2) {
                let (a, b) = if rows {
                    (r.min(r2), r.max(r2))
                } else {
                    (c.min(c2), c.max(c2))
                };
                let replacement = match shift_bounds(a, b, index, count, delete) {
                    Some((first, last)) => {
                        let (v1, v2) = if (if rows { r } else { c }) <= (if rows { r2 } else { c2 })
                        {
                            (first, last)
                        } else {
                            (last, first)
                        };
                        let one = reference_text(
                            if rows { i64::from(v1) } else { i64::from(r) },
                            if rows { i64::from(c) } else { i64::from(v1) },
                            ra,
                            ca,
                        );
                        let two = reference_text(
                            if rows { i64::from(v2) } else { i64::from(r2) },
                            if rows { i64::from(c2) } else { i64::from(v2) },
                            ra2,
                            ca2,
                        );
                        if one == "#REF!" || two == "#REF!" {
                            "#REF!".into()
                        } else {
                            format!("{one}:{two}")
                        }
                    }
                    None => "#REF!".into(),
                };
                changes.push((tokens[i].start, tokens[i + 2].end, replacement));
                i += 3;
                continue;
            }
            let n = if rows { r } else { c };
            let replacement = shift_bounds(n, n, index, count, delete)
                .map(|(v, _)| {
                    reference_text(
                        if rows { i64::from(v) } else { i64::from(r) },
                        if rows { i64::from(c) } else { i64::from(v) },
                        ra,
                        ca,
                    )
                })
                .unwrap_or_else(|| "#REF!".into());
            changes.push((tokens[i].start, tokens[i].end, replacement));
        }
        i += 1;
    }
    replace_spans(input, changes)
}
pub fn invalidate_sheet(input: &str, deleted: &str) -> String {
    let Ok(tokens) = lexer(input) else {
        return input.into();
    };
    let mut changes = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if !tokens
            .get(i + 1)
            .is_some_and(|t| t.kind == TokenKind::Op("!".into()))
        {
            continue;
        }
        if let TokenKind::Word(name) | TokenKind::Quoted(name) = &t.kind
            && name.to_lowercase() == deleted.to_lowercase()
            && token_reference(&tokens, i + 2).is_some()
        {
            let end = if tokens
                .get(i + 3)
                .is_some_and(|t| t.kind == TokenKind::Op(":".into()))
                && token_reference(&tokens, i + 4).is_some()
            {
                i + 4
            } else {
                i + 2
            };
            changes.push((t.start, tokens[end].end, "#REF!".into()));
        }
    }
    replace_spans(input, changes)
}
pub fn rename_sheet(input: &str, old: &str, new: &str) -> String {
    let Ok(tokens) = lexer(input) else {
        return input.into();
    };
    let mut changes = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if tokens
            .get(i + 1)
            .is_some_and(|t| t.kind == TokenKind::Op("!".into()))
            && let TokenKind::Word(name) | TokenKind::Quoted(name) = &t.kind
            && name.to_lowercase() == old.to_lowercase()
        {
            changes.push((t.start, t.end, format!("'{}'", new.replace('\'', "''"))));
        }
    }
    replace_spans(input, changes)
}
#[derive(Clone, Debug)]
enum Expr {
    Literal(Value),
    Reference(Option<String>, Range),
    Name(String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}
struct Parser {
    tokens: Vec<Token>,
    index: usize,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> TokenKind {
        self.tokens
            .get(self.index)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::End)
    }
    fn next(&mut self) -> TokenKind {
        let t = self.peek();
        self.index += 1;
        t
    }
    fn eat(&mut self, op: &str) -> bool {
        if self.peek() == TokenKind::Op(op.into()) {
            self.index += 1;
            true
        } else {
            false
        }
    }
    fn expression(&mut self, min: u8) -> CalcResult<Expr> {
        self.depth += 1;
        if self.depth > 128 {
            return Err("#NUM!".into());
        }
        let mut left = self.primary()?;
        loop {
            if self.peek() == TokenKind::Op("%".into()) {
                self.index += 1;
                left = Expr::Unary("%".into(), Box::new(left));
                bounded_expression_depth(&left)?;
                continue;
            }
            let TokenKind::Op(op) = self.peek() else {
                break;
            };
            let precedence = match op.as_str() {
                "=" | "<>" | "<" | ">" | "<=" | ">=" => 1,
                "&" => 2,
                "+" | "-" => 3,
                "*" | "/" => 4,
                "^" => 5,
                _ => 0,
            };
            if precedence == 0 || precedence < min {
                break;
            }
            self.index += 1;
            let right = self.expression(precedence + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
            bounded_expression_depth(&left)?;
        }
        bounded_expression_depth(&left)?;
        self.depth -= 1;
        Ok(left)
    }
    fn primary(&mut self) -> CalcResult<Expr> {
        match self.next() {
            TokenKind::Number(n) => Ok(Expr::Literal(Value::Number(n))),
            TokenKind::Text(s) => Ok(Expr::Literal(Value::Text(s))),
            TokenKind::Op(o) if o == "+" || o == "-" => {
                Ok(Expr::Unary(o, Box::new(self.expression(6)?)))
            }
            TokenKind::Op(o) if o == "(" => {
                let result = self.expression(1)?;
                if !self.eat(")") {
                    return Err("#VALUE!".into());
                }
                Ok(result)
            }
            TokenKind::Word(w) | TokenKind::Quoted(w) => {
                if self.eat("(") {
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            if matches!(self.peek(),TokenKind::Op(ref o)if o==","||o==";"||o==")") {
                                args.push(Expr::Literal(Value::Empty));
                            } else {
                                args.push(self.expression(1)?);
                            }
                            if self.eat(")") {
                                break;
                            }
                            if !self.eat(",") && !self.eat(";") {
                                return Err("#VALUE!".into());
                            }
                            if args.len() > 255 {
                                return Err("#VALUE!".into());
                            }
                        }
                    }
                    return Ok(Expr::Call(alias(&w), args));
                }
                let (sheet, word) = if self.eat("!") {
                    let next = self.next();
                    let TokenKind::Word(a) = next else {
                        return Err("#REF!".into());
                    };
                    (Some(w), a)
                } else {
                    (None, w)
                };
                if let Some((r, c)) = parse_address(&word) {
                    let end = if self.eat(":") {
                        match self.next() {
                            TokenKind::Word(a) => parse_address(&a).ok_or("#REF!")?,
                            _ => return Err("#REF!".into()),
                        }
                    } else {
                        (r, c)
                    };
                    let range = Range::new(r, c, end.0, end.1);
                    range.validate().map_err(|_| "#NUM!")?;
                    return Ok(Expr::Reference(sheet, range));
                }
                if sheet.is_some() {
                    return Err("#REF!".into());
                }
                match word.to_uppercase().as_str() {
                    "TRUE" | "ИСТИНА" => Ok(Expr::Literal(Value::Bool(true))),
                    "FALSE" | "ЛОЖЬ" => Ok(Expr::Literal(Value::Bool(false))),
                    s if s.starts_with('#') => Ok(Expr::Literal(Value::Error(s.into()))),
                    _ => Ok(Expr::Name(word)),
                }
            }
            _ => Err("#VALUE!".into()),
        }
    }
}
// The parser's call depth does not bound a left-associative or postfix AST chain.
// Reject before a deep boxed tree can reach recursive evaluation or destruction.
fn bounded_expression_depth(expr: &Expr) -> CalcResult<()> {
    let mut pending = vec![(expr, 1usize)];
    while let Some((expr, depth)) = pending.pop() {
        if depth > 128 { return Err("#NUM!".into()); }
        match expr {
            Expr::Unary(_, value) => pending.push((value, depth + 1)),
            Expr::Binary(_, left, right) => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            Expr::Call(_, args) => pending.extend(args.iter().map(|expr| (expr, depth + 1))),
            _ => (),
        }
    }
    Ok(())
}
fn parse(input: &str) -> CalcResult<Expr> {
    if input.len() > 32767 {
        return Err("#NUM!".into());
    }
    let mut p = Parser {
        tokens: lexer(input)?,
        index: 0,
        depth: 0,
    };
    let expr = p.expression(1)?;
    if p.peek() != TokenKind::End {
        return Err("#VALUE!".into());
    }
    Ok(expr)
}
#[derive(Clone)]
struct Grid {
    values: Vec<Value>,
    rows: usize,
    cols: usize,
    reference: bool,
    origin: Option<(usize,u32,u32)>,
}
impl Grid {
    fn scalar(value: Value) -> Self {
        Self {
            values: vec![value],
            rows: 1,
            cols: 1,
            reference: false,
            origin: None,
        }
    }
    fn first(&self) -> Value {
        self.values.first().cloned().unwrap_or_default()
    }
    fn number(&self) -> CalcResult<f64> {
        self.first().number()
    }
    fn text(&self) -> String {
        self.first().display()
    }
    fn boolean(&self) -> CalcResult<bool> {
        self.first().boolean()
    }
    fn at(&self, row: usize, col: usize) -> CalcResult<Value> {
        if row >= self.rows || col >= self.cols {
            return Err("#REF!".into());
        }
        self.values
            .get(row * self.cols + col)
            .cloned()
            .ok_or("#REF!".into())
    }
}
struct Engine<'a> {
    book: &'a Book,
    cache: HashMap<(usize, u32, u32), Value>,
    visiting: HashSet<(usize, u32, u32)>,
    depth: usize,
    now: f64,
    random: u64,
    work: usize,
    materialized: usize,
    budget_exhausted: bool,
    cached_text: usize,
    cache_text_limit: usize,
}
impl<'a> Engine<'a> {
    fn new(book: &'a Book) -> Self {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            book,
            cache: HashMap::new(),
            visiting: HashSet::new(),
            depth: 0,
            now: calendar_serial(chrono::Local::now().naive_local()),
            random: elapsed.as_nanos() as u64 | 1,
            work: 0,
            materialized: 0,
            budget_exhausted: false,
            cached_text: 0,
            cache_text_limit: 64*1024*1024,
        }
    }
    fn root_cell(&mut self, sheet: usize, row: u32, col: u32) -> Value {
        // Share valid dependency results, but give each independent calculation its own budget.
        self.work = 0;
        self.materialized = 0;
        self.budget_exhausted = false;
        self.cell(sheet, row, col)
    }
    fn cell(&mut self, sheet: usize, row: u32, col: u32) -> Value {
        let key = (sheet, row, col);
        if let Some(value) = self.cache.get(&key) {
            return value.clone();
        }
        if self.book.sheets.get(sheet).and_then(|s|s.cell(row,col)).is_none() {
            return Value::Empty;
        }
        if self.depth >= 256 {
            self.budget_exhausted = true;
            return Value::Error("#REF!".into());
        }
        if !self.visiting.insert(key) {
            return Value::Error("#REF!".into());
        }
        self.depth += 1;
        let input = self
            .book
            .sheets
            .get(sheet)
            .and_then(|s| s.cell(row, col))
            .map(|c| c.input.clone())
            .unwrap_or_default();
        let mut value = if input.starts_with('=') {
            match parse(&input).and_then(|e| self.eval(&e, sheet, row, col)) {
                Ok(grid) => formula_result(grid.first()),
                Err(e) => Value::Error(e),
            }
        } else {
            literal(&input)
        };
        self.depth -= 1;
        self.visiting.remove(&key);
        // A limit reached through another cell is transient: that cell may fit as a root.
        if !self.budget_exhausted {
            let bytes=match &value {Value::Text(text)=>text.len(),_=>0};
            if self.cached_text.saturating_add(bytes)>self.cache_text_limit {
                value=Value::Error("#NUM!".into());
            } else {self.cached_text+=bytes;}
            self.cache.insert(key,value.clone());
        }
        value
    }
    fn eval(&mut self, expr: &Expr, sheet: usize, row: u32, col: u32) -> CalcResult<Grid> {
        self.work += 1;
        if self.depth >= 128 || self.work > 1_000_000 {
            self.budget_exhausted = true;
            return Err("#NUM!".into());
        }
        self.depth += 1;
        let result = self.eval_inner(expr, sheet, row, col);
        self.depth -= 1;
        result
    }
    fn eval_inner(&mut self, expr: &Expr, sheet: usize, row: u32, col: u32) -> CalcResult<Grid> {
        match expr {
            Expr::Literal(v) => Ok(Grid::scalar(v.clone())),
            Expr::Reference(name, range) => {
                let target = match name {
                    Some(n) => self
                        .book
                        .sheets
                        .iter()
                        .position(|s| s.name.to_lowercase() == n.to_lowercase())
                        .ok_or("#REF!")?,
                    None => sheet,
                };
                let mut values = Vec::new();
                for r in range.r0..=range.r1 {
                    for c in range.c0..=range.c1 {
                        let value = self.cell(target, r, c);
                        self.work += 1;
                        let bytes = match &value {
                            Value::Text(t) | Value::Error(t) => t.len() + 32,
                            _ => 32,
                        };
                        self.materialized = self.materialized.saturating_add(bytes);
                        if self.work > 1_000_000 || self.materialized > 64 * 1024 * 1024 {
                            self.budget_exhausted = true;
                            return Err("#NUM!".into());
                        }
                        values.push(value);
                    }
                }
                Ok(Grid {
                    values,
                    rows: (range.r1 - range.r0 + 1) as usize,
                    cols: (range.c1 - range.c0 + 1) as usize,
                    reference: true,
                    origin: Some((target,range.r0,range.c0)),
                })
            }
            Expr::Name(n) => {
                let formula = self
                    .book
                    .names
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == n.to_lowercase())
                    .map(|(_, v)| v.clone())
                    .ok_or("#NAME?")?;
                if self.depth >= 256 {
                    return Err("#REF!".into());
                }
                self.depth += 1;
                let result = parse(&formula).and_then(|e| self.eval(&e, sheet, row, col));
                self.depth -= 1;
                result
            }
            Expr::Unary(op, e) => {
                let n = self.eval(e, sheet, row, col)?.number()?;
                finite(match op.as_str() {
                    "-" => -n,
                    "%" => n / 100.0,
                    _ => n,
                })
                .map(|v| Grid::scalar(Value::Number(v)))
            }
            Expr::Binary(op, a, b) => {
                let a = self.eval(a, sheet, row, col)?.first();
                let b = self.eval(b, sheet, row, col)?.first();
                if let Value::Error(e) = &a {
                    return Err(e.clone());
                }
                if let Value::Error(e) = &b {
                    return Err(e.clone());
                }
                if op == "&" {
                    let left = a.display();
                    let right = b.display();
                    if left.len().saturating_add(right.len()) > 32767 {
                        return Err("#VALUE!".into());
                    }
                    return Ok(Grid::scalar(Value::Text(format!("{left}{right}"))));
                }
                if matches!(op.as_str(), "=" | "<>" | "<" | ">" | "<=" | ">=") {
                    let order = compare(&a, &b);
                    let result = match op.as_str() {
                        "=" => order.is_eq(),
                        "<>" => !order.is_eq(),
                        "<" => order.is_lt(),
                        ">" => order.is_gt(),
                        "<=" => !order.is_gt(),
                        _ => !order.is_lt(),
                    };
                    return Ok(Grid::scalar(Value::Bool(result)));
                }
                let x = a.number()?;
                let y = b.number()?;
                if op == "/" && y == 0.0 {
                    return Err("#DIV/0!".into());
                }
                let n = match op.as_str() {
                    "+" => x + y,
                    "-" => x - y,
                    "*" => x * y,
                    "/" => x / y,
                    "^" => power(x, y)?,
                    _ => return Err("#VALUE!".into()),
                };
                Ok(Grid::scalar(Value::Number(finite(n)?)))
            }
            Expr::Call(name, args) => self.call(name, args, sheet, row, col),
        }
    }
    /// SUBTOTAL ignores cells holding nested SUBTOTAL results and, for codes 101–111, rows
    /// the user has hidden.
    fn skip_subtotal_cells(
        &self,
        grid: &mut Grid,
        target: Option<&str>,
        range: &Range,
        sheet: usize,
        hidden: bool,
    ) {
        let index = match target {
            Some(name) => self
                .book
                .sheets
                .iter()
                .position(|s| s.name.to_lowercase() == name.to_lowercase()),
            None => Some(sheet),
        };
        let Some(source) = index.and_then(|i| self.book.sheets.get(i)) else {
            return;
        };
        let cols = grid.cols.max(1);
        for (i, value) in grid.values.iter_mut().enumerate() {
            let r = range.r0 + (i / cols) as u32;
            let c = range.c0 + (i % cols) as u32;
            let nested = source.cell(r, c).is_some_and(|cell| {
                cell.input.starts_with('=')
                    && parse(&cell.input).is_ok_and(|expr| contains_subtotal(&expr))
            });
            if nested || (hidden && source.hidden_rows.contains(&r)) {
                *value = Value::Empty;
            }
        }
    }
    fn reference(&self, expr: &Expr, depth: usize) -> CalcResult<Option<(Option<String>, Range)>> {
        if depth >= 32 {
            return Err("#REF!".into());
        }
        match expr {
            Expr::Reference(target, range) => {
                if target.as_ref().is_some_and(|name| {
                    !self
                        .book
                        .sheets
                        .iter()
                        .any(|sheet| sheet.name.to_lowercase() == name.to_lowercase())
                }) {
                    return Err("#REF!".into());
                }
                Ok(Some((target.clone(), *range)))
            }
            Expr::Name(name) => {
                let input = self
                    .book
                    .names
                    .iter()
                    .find(|(key, _)| key.to_lowercase() == name.to_lowercase())
                    .map(|(_, input)| input)
                    .ok_or("#NAME?")?;
                self.reference(&parse(input)?, depth + 1)
            }
            _ => Ok(None),
        }
    }
    fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        sheet: usize,
        row: u32,
        col: u32,
    ) -> CalcResult<Grid> {
        if name == "IF" {
            if !(2..=3).contains(&args.len()) {
                return Err("#VALUE!".into());
            }
            let condition = self.eval(&args[0], sheet, row, col)?.boolean()?;
            return if condition {
                self.eval(&args[1], sheet, row, col)
            } else if let Some(no) = args.get(2) {
                self.eval(no, sheet, row, col)
            } else {
                Ok(Grid::scalar(Value::Bool(false)))
            };
        }
        if name == "IFERROR" || name == "IFNA" {
            if args.len() != 2 {
                return Err("#VALUE!".into());
            }
            let value = self.eval(&args[0], sheet, row, col);
            let error = match &value {
                Err(e) => Some(e.as_str()),
                Ok(g) => match g.values.first() {
                    Some(Value::Error(e)) => Some(e.as_str()),
                    _ => None,
                },
            };
            if error.is_some_and(|e| name == "IFERROR" || e == "#N/A") {
                return self.eval(&args[1], sheet, row, col);
            }
            return value;
        }
        if name == "IFS" {
            if args.is_empty() || !args.len().is_multiple_of(2) {
                return Err("#VALUE!".into());
            }
            for pair in args.as_chunks::<2>().0 {
                if self.eval(&pair[0], sheet, row, col)?.boolean()? {
                    return self.eval(&pair[1], sheet, row, col);
                }
            }
            return Err("#N/A".into());
        }
        if name == "CHOOSE" {
            if args.len() < 2 {
                return Err("#VALUE!".into());
            }
            let n = self.eval(&args[0], sheet, row, col)?.number()?.trunc();
            if n < 1.0 || n >= args.len() as f64 {
                return Err("#VALUE!".into());
            }
            return self.eval(&args[n as usize], sheet, row, col);
        }
        if name == "SWITCH" {
            if args.len() < 3 {
                return Err("#VALUE!".into());
            }
            let v = self.eval(&args[0], sheet, row, col)?.first();
            if let Value::Error(error) = &v {
                return Err(error.clone());
            }
            for pair in args[1..].as_chunks::<2>().0 {
                let candidate = self.eval(&pair[0], sheet, row, col)?.first();
                if let Value::Error(error) = &candidate {
                    return Err(error.clone());
                }
                if compare(&v, &candidate).is_eq() {
                    return self.eval(&pair[1], sheet, row, col);
                }
            }
            return if args.len().is_multiple_of(2) {
                self.eval(args.last().ok_or("#VALUE!")?, sheet, row, col)
            } else {
                Err("#N/A".into())
            };
        }
        if matches!(name, "ROWS" | "COLUMNS") {
            if args.len() != 1 { return Err("#VALUE!".into()); }
            if let Some((_, range)) = self.reference(&args[0], 0)? {
                let size = if name == "ROWS" { range.r1 - range.r0 + 1 } else { range.c1 - range.c0 + 1 };
                return Ok(Grid::scalar(Value::Number(f64::from(size))));
            }
        }
        if name == "ROW" || name == "COLUMN" {
            if args.is_empty() {
                return Ok(Grid::scalar(Value::Number(f64::from(if name == "ROW" {
                    row + 1
                } else {
                    col + 1
                }))));
            }
            if args.len() != 1 {
                return Err("#VALUE!".into());
            }
            if let Some((_, range)) = self.reference(&args[0], 0)? {
                return Ok(Grid::scalar(Value::Number(f64::from(if name == "ROW" {
                    range.r0 + 1
                } else {
                    range.c0 + 1
                }))));
            }
            let grid=self.eval(&args[0],sheet,row,col)?;
            if let Some((_,row,col))=grid.origin {
                return Ok(Grid::scalar(Value::Number(f64::from(if name=="ROW" {row+1}else{col+1}))));
            }
            return Err("#VALUE!".into());
        }
        let mut evaluated = Vec::new();
        for (index, expr) in args.iter().enumerate() {
            if (name == "XLOOKUP" && index == 3)
                || (matches!(name, "TEXTBEFORE" | "TEXTAFTER") && index == 5)
            {
                evaluated.push(Grid::scalar(Value::Empty));
                continue;
            }
            let projected;
            let expr = if matches!(name, "SUMIF" | "AVERAGEIF")
                && index == 2
                && let Some((target, range)) = self.reference(expr, 0)?
            {
                let criteria: &Grid = evaluated.first().ok_or("#VALUE!")?;
                let r1 = range
                    .r0
                    .checked_add(criteria.rows as u32 - 1)
                    .ok_or("#REF!")?;
                let c1 = range
                    .c0
                    .checked_add(criteria.cols as u32 - 1)
                    .ok_or("#REF!")?;
                let range = Range::new(range.r0, range.c0, r1, c1);
                range.validate().map_err(|_| "#REF!")?;
                projected = Expr::Reference(target.clone(), range);
                &projected
            } else {
                expr
            };
            let mut value = self.eval(expr, sheet, row, col);
            if matches!(name, "SUMIF" | "AVERAGEIF") && index == 2
                && let Ok(grid) = &value
                && let Some((target, r0, c0)) = grid.origin
                && let Some(criteria) = evaluated.first()
                && (grid.rows != criteria.rows || grid.cols != criteria.cols)
            {
                let r1 = r0.checked_add(criteria.rows as u32 - 1).ok_or("#REF!")?;
                let c1 = c0.checked_add(criteria.cols as u32 - 1).ok_or("#REF!")?;
                let range = Range::new(r0, c0, r1, c1);
                range.validate().map_err(|_| "#REF!")?;
                let target = self.book.sheets.get(target).ok_or("#REF!")?.name.clone();
                value = self.eval(&Expr::Reference(Some(target), range), sheet, row, col);
            }
            if name == "SUBTOTAL" && index > 0 && let Ok(grid) = &mut value
                && let Some((target, r0, c0)) = grid.origin
            {
                let hidden = evaluated.first().and_then(|code| code.number().ok())
                    .is_some_and(|code| code >= 100.0);
                let range = Range::new(r0, c0, r0 + grid.rows as u32 - 1, c0 + grid.cols as u32 - 1);
                let target = self.book.sheets.get(target).ok_or("#REF!")?.name.clone();
                self.skip_subtotal_cells(grid, Some(&target), &range, sheet, hidden);
            }
            evaluated.push(if name.starts_with("IS") || name == "TYPE" {
                value.unwrap_or_else(|e| Grid::scalar(Value::Error(e)))
            } else {
                value?
            });
        }
        let (name, a) = if name == "SUBTOTAL" {
            if evaluated.len() < 2 {
                return Err("#VALUE!".into());
            }
            let code = evaluated[0].number()?.trunc();
            let function = match code as i64 {
                1 | 101 => "AVERAGE",
                2 | 102 => "COUNT",
                3 | 103 => "COUNTA",
                4 | 104 => "MAX",
                5 | 105 => "MIN",
                6 | 106 => "PRODUCT",
                7 | 107 => "STDEV",
                8 | 108 => "STDEVP",
                9 | 109 => "SUM",
                10 | 110 => "VAR",
                11 | 111 => "VARP",
                _ => return Err("#VALUE!".into()),
            };
            (function, &evaluated[1..])
        } else {
            (name, evaluated.as_slice())
        };
        let n = |i: usize| -> CalcResult<f64> { a.get(i).ok_or("#VALUE!")?.number() };
        let default_n = |i: usize, v: f64| -> CalcResult<f64> {
            if let Some(g) = a.get(i) {
                if g.first() == Value::Empty && !g.reference {
                    Ok(v)
                } else {
                    g.number()
                }
            } else {
                Ok(v)
            }
        };
        let text = |i: usize| -> CalcResult<String> {
            let g = a.get(i).ok_or("#VALUE!")?;
            if let Value::Error(e) = g.first() {
                Err(e)
            } else {
                Ok(g.text())
            }
        };
        let boolv = |i: usize| -> CalcResult<bool> { a.get(i).ok_or("#VALUE!")?.boolean() };
        let num = |v: f64| -> CalcResult<Grid> { Ok(Grid::scalar(Value::Number(finite(v)?))) };
        let string = |v: String| -> CalcResult<Grid> {
            if v.len() > 32767 {
                Err("#VALUE!".into())
            } else {
                Ok(Grid::scalar(Value::Text(v)))
            }
        };
        let boolean = |v: bool| -> CalcResult<Grid> { Ok(Grid::scalar(Value::Bool(v))) };
        let arity = |min: usize, max: usize| -> CalcResult<()> {
            if a.len() < min || a.len() > max {
                Err("#VALUE!".into())
            } else {
                Ok(())
            }
        };
        match name {
            "TRUE" | "FALSE" => {
                arity(0, 0)?;
                boolean(name == "TRUE")
            }
            "NA" => {
                arity(0, 0)?;
                Err("#N/A".into())
            }
            "PI" => {
                arity(0, 0)?;
                num(std::f64::consts::PI)
            }
            "RAND" => {
                arity(0, 0)?;
                self.random ^= self.random << 13;
                self.random ^= self.random >> 7;
                self.random ^= self.random << 17;
                num((self.random >> 11) as f64 / (1u64 << 53) as f64)
            }
            "RANDBETWEEN" => {
                arity(2, 2)?;
                let low = n(0)?.ceil();
                let high = n(1)?.floor();
                if low > high {
                    return Err("#NUM!".into());
                }
                self.random = self
                    .random
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1);
                let fraction = (self.random >> 11) as f64 / (1u64 << 53) as f64;
                let span = high - low + 1.0;
                num(if span.is_finite() {
                    (low + (fraction * span).floor()).min(high)
                } else {
                    interpolate(low, high, fraction).floor().clamp(low, high)
                })
            }
            "TODAY" | "NOW" => {
                arity(0, 0)?;
                num(if name == "TODAY" {
                    self.now.floor()
                } else {
                    self.now
                })
            }
            "SUM" | "AVERAGE" | "AVERAGEA" | "MIN" | "MAX" | "PRODUCT" | "COUNT" | "COUNTA"
            | "COUNTBLANK" | "MEDIAN" | "MODE" | "MODE.SNGL" | "STDEV" | "STDEV.S" | "STDEVP"
            | "STDEV.P" | "VAR" | "VAR.S" | "VARP" | "VAR.P" | "DEVSQ" | "AVEDEV" | "GEOMEAN"
            | "HARMEAN" | "SUMSQ" => {
                arity(1, if name == "COUNTBLANK" { 1 } else { 255 })?;
                if name == "COUNTA" {
                    return num(a
                        .iter()
                        .flat_map(|g| &g.values)
                        .filter(|v| **v != Value::Empty)
                        .count() as f64);
                }
                if name == "COUNTBLANK" {
                    return num(a
                        .iter()
                        .flat_map(|g| &g.values)
                        .filter(|v| **v == Value::Empty || **v == Value::Text(String::new()))
                        .count() as f64);
                }
                if name == "COUNT" {
                    return num(a
                        .iter()
                        .flat_map(|g| g.values.iter().map(move |v| (g.reference, v)))
                        .filter(|(reference, v)| {
                            matches!(v, Value::Number(_))
                                || (!reference && v.number().is_ok() && **v != Value::Empty)
                        })
                        .count() as f64);
                }
                let mut vals = numbers(a, name == "AVERAGEA")?;
                let sum = scaled_sum(&vals);
                let count = vals.len() as f64;
                if name == "SUM" {
                    return num(sum);
                }
                if name == "PRODUCT" {
                    return num(if vals.is_empty() {
                        0.0
                    } else {
                        balanced_product(&vals)
                    });
                }
                if name == "MIN" {
                    return num(vals.iter().copied().reduce(f64::min).unwrap_or(0.0));
                }
                if name == "MAX" {
                    return num(vals.iter().copied().reduce(f64::max).unwrap_or(0.0));
                }
                if name == "SUMSQ" {
                    return num(vals.iter().map(|x| x * x).sum());
                }
                if vals.is_empty() {
                    return Err("#DIV/0!".into());
                }
                let magnitude = vals.iter().copied().map(f64::abs).fold(0.0, f64::max);
                let scale = if magnitude > 1e150 || magnitude < 1e-150 {
                    magnitude
                } else {
                    1.0
                };
                let normalized_mean = if scale == 0.0 {
                    0.0
                } else {
                    vals.iter().map(|value| value / scale).sum::<f64>() / count
                };
                let mean = normalized_mean * scale;
                let normalized_dev = if scale == 0.0 {
                    0.0
                } else {
                    vals.iter()
                        .map(|value| (value / scale - normalized_mean).powi(2))
                        .sum::<f64>()
                };
                let dev = normalized_dev * scale * scale;
                match name {
                    "AVERAGE" | "AVERAGEA" => num(mean),
                    "MEDIAN" => {
                        vals.sort_by(f64::total_cmp);
                        let i = vals.len() / 2;
                        num(if vals.len().is_multiple_of(2) {
                            interpolate(vals[i - 1], vals[i], 0.5)
                        } else {
                            vals[i]
                        })
                    }
                    "MODE" | "MODE.SNGL" => {
                        vals.sort_by(f64::total_cmp);
                        let mut best = 0usize;
                        let mut result = 0.0;
                        let mut count = 0;
                        let mut last = f64::NAN;
                        for v in vals {
                            if v == last {
                                count += 1;
                            } else {
                                count = 1;
                                last = v;
                            }
                            if count > best {
                                best = count;
                                result = v;
                            }
                        }
                        if best < 2 {
                            Err("#N/A".into())
                        } else {
                            num(result)
                        }
                    }
                    "DEVSQ" => num(dev),
                    "AVEDEV" => num(if scale == 0.0 {
                        0.0
                    } else {
                        vals.iter()
                            .map(|v| (v / scale - normalized_mean).abs())
                            .sum::<f64>()
                            / count
                            * scale
                    }),
                    "GEOMEAN" => {
                        if vals.iter().any(|v| *v <= 0.0) {
                            Err("#NUM!".into())
                        } else {
                            num((vals.iter().map(|v| v.ln()).sum::<f64>() / count).exp())
                        }
                    }
                    "HARMEAN" => {
                        if vals.iter().any(|v| *v <= 0.0) {
                            Err("#NUM!".into())
                        } else {
                            let minimum = vals.iter().copied().fold(f64::INFINITY, f64::min);
                            num(minimum * (count / vals.iter().map(|v| minimum / v).sum::<f64>()))
                        }
                    }
                    _ => {
                        let sample = !matches!(name, "VARP" | "VAR.P" | "STDEVP" | "STDEV.P");
                        if sample && vals.len() < 2 {
                            return Err("#DIV/0!".into());
                        }
                        let normalized_variance =
                            normalized_dev / (count - f64::from(u8::from(sample)));
                        num(if name.starts_with("STDEV") {
                            normalized_variance.sqrt() * scale
                        } else {
                            normalized_variance * scale * scale
                        })
                    }
                }
            }
            "AND" | "OR" | "XOR" => {
                arity(1, 255)?;
                let mut vals = Vec::new();
                for g in a {
                    for v in &g.values {
                        if !g.reference
                            || matches!(v, Value::Number(_) | Value::Bool(_) | Value::Error(_))
                        {
                            vals.push(v.boolean()?);
                        }
                    }
                }
                if vals.is_empty() {
                    return Err("#VALUE!".into());
                }
                boolean(if name == "AND" {
                    vals.iter().all(|v| *v)
                } else if name == "OR" {
                    vals.iter().any(|v| *v)
                } else {
                    vals.iter().filter(|v| **v).count() % 2 == 1
                })
            }
            "NOT" => {
                arity(1, 1)?;
                boolean(!boolv(0)?)
            }
            "ISNUMBER" | "ISTEXT" | "ISNONTEXT" | "ISBLANK" | "ISLOGICAL" | "ISERROR" | "ISERR"
            | "ISNA" | "ISEVEN" | "ISODD" | "ISFORMULA" | "N" | "T" | "TYPE" => {
                arity(1, 1)?;
                let value = a[0].first();
                match name {
                    "ISNUMBER" => boolean(matches!(value, Value::Number(_))),
                    "ISTEXT" => boolean(matches!(value, Value::Text(_))),
                    "ISNONTEXT" => boolean(!matches!(value, Value::Text(_))),
                    "ISBLANK" => boolean(value == Value::Empty),
                    "ISLOGICAL" => boolean(matches!(value, Value::Bool(_))),
                    "ISERROR" => boolean(matches!(value, Value::Error(_))),
                    "ISERR" => boolean(matches!(&value,Value::Error(e)if e!="#N/A")),
                    "ISNA" => boolean(matches!(&value,Value::Error(e)if e=="#N/A")),
                    "ISEVEN" | "ISODD" => {
                        boolean((n(0)?.trunc().abs() % 2.0 == 0.0) == (name == "ISEVEN"))
                    }
                    "ISFORMULA" => {
                        if let Some((target, r)) = args
                            .first()
                            .map(|expr| self.reference(expr, 0))
                            .transpose()?
                            .flatten()
                        {
                            let si = match &target {
                                Some(name) => self
                                    .book
                                    .sheets
                                    .iter()
                                    .position(|s| s.name.to_lowercase() == name.to_lowercase())
                                    .ok_or("#REF!")?,
                                None => sheet,
                            };
                            boolean(
                                self.book
                                    .sheets
                                    .get(si)
                                    .and_then(|s| s.cell(r.r0, r.c0))
                                    .is_some_and(|c| c.input.starts_with('=')),
                            )
                        } else if let Some((sheet,row,col))=a[0].origin {
                            boolean(self.book.sheets.get(sheet).and_then(|sheet|sheet.cell(row,col))
                                .is_some_and(|cell|cell.input.starts_with('=')))
                        }else{boolean(false)}
                    }
                    "N" => num(match value {
                        Value::Number(v) => v,
                        Value::Bool(v) => f64::from(u8::from(v)),
                        Value::Error(e) => return Err(e),
                        _ => 0.0,
                    }),
                    "T" => string(if let Value::Text(s) = value {
                        s
                    } else if let Value::Error(e) = value {
                        return Err(e);
                    } else {
                        String::new()
                    }),
                    _ if a[0].rows > 1 || a[0].cols > 1 => num(64.0),
                    _ => num(match value {
                        Value::Number(_) | Value::Empty => 1.0,
                        Value::Text(_) => 2.0,
                        Value::Bool(_) => 4.0,
                        Value::Error(_) => 16.0,
                    }),
                }
            }
            "ABS" | "INT" | "SIGN" | "SQRT" | "EXP" | "LN" | "LOG10" | "SIN" | "COS" | "TAN"
            | "ASIN" | "ACOS" | "ATAN" | "SINH" | "COSH" | "TANH" | "ASINH" | "ACOSH" | "ATANH"
            | "DEGREES" | "RADIANS" | "FACT" | "FACTDOUBLE" | "EVEN" | "ODD" => {
                arity(1, 1)?;
                let x = n(0)?;
                num(match name {
                    "ABS" => x.abs(),
                    "INT" => x.floor(),
                    "SIGN" => {
                        if x == 0.0 {
                            0.0
                        } else {
                            x.signum()
                        }
                    }
                    "SQRT" => x.sqrt(),
                    "EXP" => x.exp(),
                    "LN" => x.ln(),
                    "LOG10" => x.log10(),
                    "SIN" => x.sin(),
                    "COS" => x.cos(),
                    "TAN" => x.tan(),
                    "ASIN" => x.asin(),
                    "ACOS" => x.acos(),
                    "ATAN" => x.atan(),
                    "SINH" => x.sinh(),
                    "COSH" => x.cosh(),
                    "TANH" => x.tanh(),
                    "ASINH" => x.asinh(),
                    "ACOSH" => x.acosh(),
                    "ATANH" => x.atanh(),
                    "DEGREES" => x.to_degrees(),
                    "RADIANS" => x.to_radians(),
                    "FACT" | "FACTDOUBLE" => {
                        let limit = if name == "FACTDOUBLE" { 300.0 } else { 170.0 };
                        if x < 0.0 || x.trunc() > limit {
                            return Err("#NUM!".into());
                        }
                        (1..=x.trunc() as u32)
                            .filter(|i| name == "FACT" || i % 2 == x.trunc() as u32 % 2)
                            .map(f64::from)
                            .product()
                    }
                    "EVEN" | "ODD" => {
                        let mut v = x.abs().ceil();
                        let parity = if name == "EVEN" { 0.0 } else { 1.0 };
                        if v % 2.0 != parity {
                            v += 1.0;
                        }
                        if x < 0.0 { -v } else { v }
                    }
                    _ => 0.0,
                })
            }
            "ROUND" | "ROUNDUP" | "ROUNDDOWN" | "TRUNC" => {
                arity(if name == "TRUNC" { 1 } else { 2 }, 2)?;
                let digits = default_n(1, 0.0)?.trunc();
                let value=n(0)?;
                if digits>308.0 {return num(value);}
                if digits< -308.0 {
                    return if name=="ROUNDUP" && value!=0.0 {Err("#NUM!".into())} else {num(0.0)};
                }
                let scale = 10f64.powf(digits);
                let x = value * scale;
                // A finite number this large already has no fractional digits to remove.
                if !x.is_finite() && digits >= 0.0 {
                    return num(value);
                }
                // Like Excel, 2.675 rounds to 2.68 although its binary value is below the tie.
                num(match name {
                    "ROUND" => super::format::round_scaled(x),
                    "ROUNDUP" => {
                        let x = super::format::snap(x);
                        x.signum() * x.abs().ceil()
                    }
                    _ => super::format::snap(x).trunc(),
                } / scale)
            }
            "MOD" | "POWER" | "LOG" | "ATAN2" | "QUOTIENT" | "MROUND" | "CEILING"
            | "CEILING.MATH" | "CEILING.PRECISE" | "FLOOR" | "FLOOR.MATH" | "FLOOR.PRECISE" => {
                arity(
                    if matches!(
                        name,
                        "LOG" | "CEILING.MATH" | "CEILING.PRECISE" | "FLOOR.MATH" | "FLOOR.PRECISE"
                    ) {
                        1
                    } else {
                        2
                    },
                    if name.ends_with(".MATH") { 3 } else { 2 },
                )?;
                let x = n(0)?;
                let y = default_n(1, if name == "LOG" { 10.0 } else { 1.0 })?;
                match name {
                    "POWER" => num(power(x, y)?),
                    "LOG" => num(x.log(y)),
                    "ATAN2" => {
                        if x == 0.0 && y == 0.0 {
                            Err("#DIV/0!".into())
                        } else {
                            num(y.atan2(x))
                        }
                    }
                    "MOD" => {
                        if y == 0.0 {
                            Err("#DIV/0!".into())
                        } else {
                            let remainder=x % y;
                            num(if remainder==0.0 {0.0}
                                else if remainder.is_sign_negative()!=y.is_sign_negative() {remainder+y}
                                else {remainder})
                        }
                    }
                    "QUOTIENT" => {
                        if y == 0.0 {
                            Err("#DIV/0!".into())
                        } else {
                            num((x / y).trunc())
                        }
                    }
                    "MROUND" => {
                        if x != 0.0 && y != 0.0 && x.is_sign_negative() != y.is_sign_negative() {
                            Err("#NUM!".into())
                        } else if y == 0.0 {
                            num(0.0)
                        } else {
                            let ratio=x/y;
                            num(if ratio.is_finite() {super::format::round_scaled(ratio)*y}else{x})
                        }
                    }
                    _ => {
                        if y == 0.0 {
                            return num(0.0);
                        }
                        if matches!(name, "CEILING" | "FLOOR") {
                            if x > 0.0 && y < 0.0 {
                                return Err("#NUM!".into());
                            }
                            if !(x/y).is_finite() {return num(x);}
                            let ratio = super::format::snap(x / y);
                            return num(if name == "CEILING" {
                                ratio.ceil() * y
                            } else {
                                ratio.floor() * y
                            });
                        }
                        let sig = y.abs();
                        if !(x/sig).is_finite() {return num(x);}
                        let ratio = super::format::snap(x / sig);
                        let away = default_n(2, 0.0)? != 0.0 && x < 0.0;
                        num(if name.starts_with("CEILING") {
                            if away {
                                ratio.floor() * sig
                            } else {
                                ratio.ceil() * sig
                            }
                        } else if away {
                            ratio.ceil() * sig
                        } else {
                            ratio.floor() * sig
                        })
                    }
                }
            }
            "GCD" | "LCM" => {
                arity(1, 255)?;
                let vals = numbers(a, false)?;
                let mut result = if name == "GCD" { 0u64 } else { 1 };
                for x in vals {
                    if !(0.0..9007199254740992.0).contains(&x) {
                        return Err("#NUM!".into());
                    }
                    let v = x.trunc() as u64;
                    let d = gcd(result, v);
                    result = if name == "GCD" {
                        d
                    } else if d == 0 {
                        0
                    } else {
                        result
                            .checked_div(d)
                            .and_then(|n| n.checked_mul(v))
                            .ok_or("#NUM!")?
                    };
                }
                if result >= 9_007_199_254_740_992 {
                    return Err("#NUM!".into());
                }
                num(result as f64)
            }
            "COMBIN" | "COMBINA" | "PERMUT" => {
                arity(2, 2)?;
                let mut total = n(0)?.trunc();
                let mut pick = n(1)?.trunc();
                if total < 0.0 || pick < 0.0 {
                    return Err("#NUM!".into());
                }
                if pick == 0.0 {
                    return num(1.0);
                }
                if name == "COMBINA" {
                    total += pick - 1.0;
                }
                if !total.is_finite() || pick > total {
                    return Err("#NUM!".into());
                }
                if name != "PERMUT" {
                    pick = pick.min(total - pick);
                }
                // C(2048,1024) and 171! already exceed finite spreadsheet numbers.
                if pick > if name == "PERMUT" { 170.0 } else { 1024.0 } {
                    return Err("#NUM!".into());
                }
                let mut result = 1.0;
                for i in 0..pick as u32 {
                    let factor = if name == "PERMUT" {
                        total - f64::from(i)
                    } else {
                        (total - f64::from(i)) / f64::from(i + 1)
                    };
                    result = finite(result * factor)?;
                }
                num(result)
            }
            "SUMPRODUCT" => {
                arity(1, 255)?;
                let count = a[0].values.len();
                if a.iter().any(|g| g.rows != a[0].rows || g.cols != a[0].cols) {
                    return Err("#VALUE!".into());
                }
                let mut terms=Vec::with_capacity(count);
                let mut factors=Vec::with_capacity(a.len());
                for i in 0..count {
                    factors.clear();
                    for grid in a {
                        factors.push(match grid.values.get(i) {
                            Some(Value::Number(value))=>*value,
                            Some(Value::Error(error))=>return Err(error.clone()),
                            _=>0.0,
                        });
                    }
                    terms.push(balanced_product(&factors));
                }
                num(scaled_sum(&terms))
            }
            "LARGE" | "SMALL" | "PERCENTILE" | "PERCENTILE.INC" | "QUARTILE" | "QUARTILE.INC"
            | "RANK" | "RANK.EQ" => {
                arity(2, if name.starts_with("RANK") { 3 } else { 2 })?;
                if name.starts_with("RANK") {
                    let target = n(0)?;
                    let mut vals = numbers(&a[1..2], false)?;
                    vals.sort_by(f64::total_cmp);
                    if default_n(2, 0.0)? == 0.0 {
                        vals.reverse();
                    }
                    return num(vals.iter().position(|v| *v == target).ok_or("#N/A")? as f64 + 1.0);
                }
                let mut vals = numbers(&a[..1], false)?;
                if vals.is_empty() {
                    return Err("#NUM!".into());
                }
                vals.sort_by(f64::total_cmp);
                let k = n(1)?;
                if name == "LARGE" || name == "SMALL" {
                    if k < 1.0 || k.trunc() > vals.len() as f64 {
                        return Err("#NUM!".into());
                    }
                    return num(vals[if name == "SMALL" {
                        k.trunc() as usize - 1
                    } else {
                        vals.len() - k.trunc() as usize
                    }]);
                }
                let p = if name.starts_with("QUARTILE") {
                    k.trunc() / 4.0
                } else {
                    k
                };
                if !(0.0..=1.0).contains(&p) {
                    return Err("#NUM!".into());
                }
                let pos = p * (vals.len() - 1) as f64;
                let low = pos.floor() as usize;
                let high = pos.ceil() as usize;
                num(interpolate(vals[low], vals[high], pos.fract()))
            }
            "SUMIF" | "COUNTIF" | "AVERAGEIF" | "SUMIFS" | "COUNTIFS" | "AVERAGEIFS" => {
                conditional(name, a).and_then(num)
            }
            "CONCAT" | "CONCATENATE" => {
                arity(1, 255)?;
                let mut out = String::new();
                for v in a.iter().flat_map(|g| &g.values) {
                    if let Value::Error(e) = v {
                        return Err(e.clone());
                    }
                    out.push_str(&v.display());
                    if out.len() > 32767 {
                        return Err("#VALUE!".into());
                    }
                }
                string(out)
            }
            "TEXTJOIN" => {
                arity(3, 255)?;
                let delimiter = text(0)?;
                let ignore = boolv(1)?;
                let mut out = String::new();
                let mut first = true;
                for v in a[2..].iter().flat_map(|g| &g.values) {
                    if let Value::Error(e) = v {
                        return Err(e.clone());
                    }
                    let s = v.display();
                    if !ignore || !s.is_empty() {
                        let separator = if first { "" } else { &delimiter };
                        if out
                            .len()
                            .saturating_add(separator.len())
                            .saturating_add(s.len())
                            > 32767
                        {
                            return Err("#VALUE!".into());
                        }
                        out.push_str(separator);
                        out.push_str(&s);
                        first = false;
                    }
                }
                string(out)
            }
            "LEN" => {
                arity(1, 1)?;
                num(text(0)?.chars().count() as f64)
            }
            "LOWER" | "UPPER" | "PROPER" | "TRIM" | "CLEAN" => {
                arity(1, 1)?;
                let s = text(0)?;
                string(match name {
                    "LOWER" => s.to_lowercase(),
                    "UPPER" => s.to_uppercase(),
                    "TRIM" => s
                        .split(' ')
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<_>>()
                        .join(" "),
                    "CLEAN" => s.chars().filter(|c| u32::from(*c) >= 32).collect(),
                    _ => {
                        let mut start = true;
                        let mut out = String::new();
                        for c in s.chars() {
                            if start {
                                out.extend(c.to_uppercase());
                            } else {
                                out.extend(c.to_lowercase());
                            }
                            start = !c.is_alphabetic();
                        }
                        out
                    }
                })
            }
            "LEFT" | "RIGHT" | "MID" => {
                arity(
                    if name == "MID" { 3 } else { 1 },
                    if name == "MID" { 3 } else { 2 },
                )?;
                let s: Vec<_> = text(0)?.chars().collect();
                let count = if name == "MID" {
                    n(2)?
                } else {
                    default_n(1, 1.0)?
                }
                .trunc();
                if count < 0.0 {
                    return Err("#VALUE!".into());
                }
                let start = if name == "MID" {
                    let x = n(1)?.trunc();
                    if x < 1.0 {
                        return Err("#VALUE!".into());
                    }
                    x as usize - 1
                } else if name == "RIGHT" {
                    s.len().saturating_sub(count as usize)
                } else {
                    0
                };
                string(s.into_iter().skip(start).take(count as usize).collect())
            }
            "REPT" => {
                arity(2, 2)?;
                let s = text(0)?;
                let count = n(1)?.trunc();
                if count < 0.0 || (!s.is_empty() && count > 32767.0) || s.len().saturating_mul(count as usize) > 32767
                {
                    return Err("#VALUE!".into());
                }
                if s.is_empty() { string(String::new()) }
                else { string(s.repeat(count as usize)) }
            }
            "EXACT" => {
                arity(2, 2)?;
                boolean(text(0)? == text(1)?)
            }
            "SUBSTITUTE" => {
                arity(3, 4)?;
                let original = text(0)?;
                let old = text(1)?;
                let new = text(2)?;
                if old.is_empty() {
                    return string(original);
                }
                if a.len() == 3 {
                    let count = original.matches(&old).count();
                    let remaining = original
                        .len()
                        .saturating_sub(count.saturating_mul(old.len()));
                    if remaining.saturating_add(count.saturating_mul(new.len())) > 32767 {
                        return Err("#VALUE!".into());
                    }
                    return string(original.replace(&old, &new));
                }
                let instance = n(3)?.trunc();
                if instance < 1.0 {
                    return Err("#VALUE!".into());
                }
                let mut result = String::new();
                let mut start = 0;
                let mut count = 0;
                for (pos, _) in original.match_indices(&old) {
                    count += 1;
                    if count == instance as usize {
                        result.push_str(&original[..pos]);
                        result.push_str(&new);
                        start = pos + old.len();
                        break;
                    }
                }
                if start == 0 {
                    string(original)
                } else {
                    result.push_str(&original[start..]);
                    string(result)
                }
            }
            "REPLACE" => {
                arity(4, 4)?;
                let chars: Vec<_> = text(0)?.chars().collect();
                let start = n(1)?.trunc();
                let count = n(2)?.trunc();
                if start < 1.0 || count < 0.0 {
                    return Err("#VALUE!".into());
                }
                let pos = (start as usize - 1).min(chars.len());
                string(format!(
                    "{}{}{}",
                    chars[..pos].iter().collect::<String>(),
                    text(3)?,
                    chars[pos.saturating_add(count as usize).min(chars.len())..]
                        .iter()
                        .collect::<String>()
                ))
            }
            "FIND" | "SEARCH" => {
                arity(2, 3)?;
                let needle = text(0)?;
                let hay = text(1)?;
                let start = default_n(2, 1.0)?.trunc();
                if start < 1.0 || start > hay.chars().count().max(1) as f64 {
                    return Err("#VALUE!".into());
                }
                let chars: Vec<_> = hay.chars().collect();
                let offset = start as usize - 1;
                let pos = if name == "FIND" {
                    chars[offset..]
                        .iter()
                        .collect::<String>()
                        .find(&needle)
                        .map(|byte| {
                            chars[offset..].iter().collect::<String>()[..byte]
                                .chars()
                                .count()
                        })
                } else if !needle.contains(['*','?','~']) {
                    literal_search(&chars[offset..],&needle)
                } else {
                    wildcard_search(&needle, &chars[offset..], 1_000_000)?
                };
                num((offset + pos.ok_or("#VALUE!")? + 1) as f64)
            }
            "VALUE" | "NUMBERVALUE" => {
                arity(1, if name == "VALUE" { 1 } else { 3 })?;
                let raw = text(0)?;
                if name == "VALUE" {
                    return num(value_text(&raw, self.book.locale_ru)?);
                }
                let mut s = raw.trim().chars().filter(|c| !matches!(*c, ' ' | '\u{a0}')).collect::<String>();
                let mut percent = 0usize;
                while s.ends_with('%') {
                    s.pop();
                    percent += 1;
                }
                if name == "NUMBERVALUE" {
                    let decimal = if a.len() > 1 {
                        text(1)?
                    } else {
                        if self.book.locale_ru { "," } else { "." }.into()
                    };
                    let group = if a.len() > 2 {
                        text(2)?
                    } else {
                        if self.book.locale_ru { "\u{a0}" } else { "," }.into()
                    };
                    let decimal = decimal.chars().next().ok_or("#VALUE!")?;
                    let group = group.chars().next().ok_or("#VALUE!")?;
                    if decimal == group {
                        return Err("#VALUE!".into());
                    }
                    let mut normalized = String::new();
                    let mut fractional = false;
                    for c in s.chars().filter(|c| !matches!(*c, ' ' | '\u{a0}')) {
                        if c == decimal {
                            if fractional {
                                return Err("#VALUE!".into());
                            }
                            fractional = true;
                            normalized.push('.');
                        } else if c == group {
                            if fractional {
                                return Err("#VALUE!".into());
                            }
                        } else if c.is_ascii_digit() || matches!(c, '+' | '-' | 'e' | 'E') {
                            normalized.push(c);
                        } else {
                            return Err("#VALUE!".into());
                        }
                    }
                    if normalized.is_empty() {
                        return num(0.0);
                    }
                    s = normalized;
                } else {
                    s = s.replace([' ', '\u{a0}'], "");
                    // With both separators the later one is the decimal mark: 1,000.5 / 1.000,5.
                    s = match (s.rfind(','), s.rfind('.')) {
                        (Some(comma), Some(dot)) if dot > comma => s.replace(',', ""),
                        (Some(_), Some(_)) => s.replace('.', "").replace(',', "."),
                        _ => s.replace(',', "."),
                    };
                }
                let mut value = s.parse::<f64>().map_err(|_| "#VALUE!")?;
                for _ in 0..percent { value /= 100.0; }
                num(value)
            }
            "TEXT" => {
                arity(2, 2)?;
                let value = a[0].first();
                if let Value::Error(error) = value {
                    return Err(error);
                }
                string(super::format::display(
                    &value,
                    &text(1)?,
                    self.book.locale_ru,
                ))
            }
            "CHAR" | "UNICHAR" => {
                arity(1, 1)?;
                let code = n(0)?.trunc();
                if code < 1.0
                    || code
                        > if name == "CHAR" {
                            255.0
                        } else {
                            0x10ffff as f64
                        }
                {
                    return Err("#VALUE!".into());
                }
                if name == "CHAR" {
                    let byte = [code as u8];
                    let encoding = ansi_encoding(self.book.locale_ru);
                    let (text, errors) = encoding.decode_without_bom_handling(&byte);
                    if errors {
                        return Err("#VALUE!".into());
                    }
                    string(text.into_owned())
                } else {
                    string(char::from_u32(code as u32).ok_or("#VALUE!")?.to_string())
                }
            }
            "CODE" | "UNICODE" => {
                arity(1, 1)?;
                let first = text(0)?.chars().next().ok_or("#VALUE!")?.to_string();
                if name == "CODE" {
                    let (bytes, _, errors) = ansi_encoding(self.book.locale_ru).encode(&first);
                    num(if errors {
                        63.0
                    } else {
                        f64::from(*bytes.first().ok_or("#VALUE!")?)
                    })
                } else {
                    num(u32::from(first.chars().next().ok_or("#VALUE!")?) as f64)
                }
            }
            "ROWS" | "COLUMNS" => {
                arity(1, 1)?;
                num(if name == "ROWS" { a[0].rows } else { a[0].cols } as f64)
            }
            "INDEX" => {
                arity(2, 3)?;
                let mut r = n(1)?.trunc();
                let mut c = if a.len() == 2 && a[0].rows > 1 && a[0].cols > 1 {
                    0.0
                } else {
                    if a.len() > 2 { n(2)?.trunc() } else { 1.0 }
                };
                // In a single row the only index counts columns: INDEX(A1:C1,2) is B1.
                if a.len() == 2 && a[0].rows == 1 {
                    (r, c) = (1.0, r);
                }
                if r < 0.0 || c < 0.0 || r > a[0].rows as f64 || c > a[0].cols as f64 {
                    return Err("#REF!".into());
                }
                let (r0, r1) = if r == 0.0 { (0, a[0].rows) } else { (r as usize-1, r as usize) };
                let (c0, c1) = if c == 0.0 { (0, a[0].cols) } else { (c as usize-1, c as usize) };
                let mut values = Vec::new();
                for row in r0..r1 {
                    for col in c0..c1 { values.push(a[0].at(row,col)?); }
                }
                Ok(Grid { values, rows:r1-r0, cols:c1-c0, reference:a[0].reference,
                    origin:a[0].origin.map(|(sheet,row,col)|(sheet,row+r0 as u32,col+c0 as u32)) })
            }
            "MATCH" | "XMATCH" => {
                arity(2, if name == "MATCH" { 3 } else { 4 })?;
                let needle = a[0].first();
                if let Value::Error(error) = &needle {
                    return Err(error.clone());
                }
                let mode_value = default_n(2, if name == "MATCH" { 1.0 } else { 0.0 })?;
                let search_value = default_n(3, 1.0)?;
                if a[1].rows > 1 && a[1].cols > 1
                    || !matches!(mode_value, -1.0 | 0.0 | 1.0 | 2.0)
                    || name == "MATCH" && mode_value == 2.0
                    || !matches!(search_value, -2.0 | -1.0 | 1.0 | 2.0)
                {
                    return Err("#VALUE!".into());
                }
                let mode = mode_value as i32;
                let search = search_value as i32;
                let vals = &a[1].values;
                if name == "XMATCH" && search.abs() == 2 && mode != 2 {
                    return num(binary_lookup(vals, &needle, mode, search).ok_or("#N/A")? as f64+1.0);
                }
                let mut best = None;
                let mut indices: Vec<_> = (0..vals.len()).collect();
                if search < 0 {
                    indices.reverse();
                }
                for i in indices {
                    let v = &vals[i];
                    if lookup_equal(
                        v,
                        &needle,
                        (name == "XMATCH" && mode == 2) || (name == "MATCH" && mode == 0),
                    ) && ((name != "MATCH" || mode == 0) || lookup_kind(v, &needle))
                    {
                        if name != "MATCH" || mode == 0 {
                            return num(i as f64 + 1.0);
                        }
                        best = Some(i);
                        continue;
                    }
                    let eligible = lookup_kind(v, &needle)
                        && if name == "MATCH" {
                            (mode > 0 && compare(v, &needle).is_le())
                                || (mode < 0 && compare(v, &needle).is_ge())
                        } else {
                            (mode == -1 && compare(v, &needle).is_le())
                                || (mode == 1 && compare(v, &needle).is_ge())
                        };
                    if eligible
                        && best.is_none_or(|b: usize| {
                            if (name == "MATCH" && mode > 0) || (name == "XMATCH" && mode == -1) {
                                compare(v, &vals[b]).is_gt()
                                    || name == "MATCH" && compare(v, &vals[b]).is_eq()
                            } else {
                                compare(v, &vals[b]).is_lt()
                                    || name == "MATCH" && compare(v, &vals[b]).is_eq()
                            }
                        })
                    {
                        best = Some(i);
                    }
                }
                num(best.ok_or("#N/A")? as f64 + 1.0)
            }
            "VLOOKUP" | "HLOOKUP" => {
                arity(3, 4)?;
                let needle = a[0].first();
                if let Value::Error(error) = &needle {
                    return Err(error.clone());
                }
                let index = n(2)?.trunc();
                let vertical = name == "VLOOKUP";
                let table = &a[1];
                if index < 1.0 { return Err("#VALUE!".into()); }
                if index > if vertical { table.cols } else { table.rows } as f64 {
                    return Err("#REF!".into());
                }
                let approximate = a.get(3).map(Grid::boolean).transpose()?.unwrap_or(true);
                let count = if vertical { table.rows } else { table.cols };
                let mut chosen = None;
                for i in 0..count {
                    let v = if vertical {
                        table.at(i, 0)?
                    } else {
                        table.at(0, i)?
                    };
                    if lookup_equal(&v, &needle, !approximate)
                        && (!approximate || lookup_kind(&v, &needle))
                    {
                        chosen = Some(i);
                        if !approximate {
                            break;
                        }
                    }
                    if approximate && lookup_kind(&v, &needle) && compare(&v, &needle).is_le() {
                        chosen = Some(i);
                    }
                }
                let i = chosen.ok_or("#N/A")?;
                Ok(Grid::scalar(if vertical {
                    table.at(i, index as usize - 1)?
                } else {
                    table.at(index as usize - 1, i)?
                }))
            }
            "XLOOKUP" => {
                arity(3, 6)?;
                let vertical = a[1].cols == 1 && a[1].rows == a[2].rows;
                let horizontal = a[1].rows == 1 && a[1].cols == a[2].cols;
                if !vertical && !horizontal { return Err("#VALUE!".into()); }
                let mode_value = default_n(4, 0.0)?;
                let search_value = default_n(5, 1.0)?;
                if a[1].rows > 1 && a[1].cols > 1
                    || !matches!(mode_value, -1.0 | 0.0 | 1.0 | 2.0)
                    || !matches!(search_value, -2.0 | -1.0 | 1.0 | 2.0)
                {
                    return Err("#VALUE!".into());
                }
                let mode = mode_value as i32;
                let search = search_value as i32;
                let needle = a[0].first();
                if let Value::Error(error) = &needle {
                    return Err(error.clone());
                }
                if search.abs() == 2 && mode != 2 {
                    if let Some(i) = binary_lookup(&a[1].values, &needle, mode, search) {
                        return lookup_result(&a[2], i, vertical);
                    }
                    return if let Some(fallback) = args.get(3)
                        .filter(|expr| !matches!(expr, Expr::Literal(Value::Empty))) {
                        self.eval(fallback, sheet, row, col)
                    } else { Err("#N/A".into()) };
                }
                let mut indices: Vec<_> = (0..a[1].values.len()).collect();
                if search < 0 {
                    indices.reverse();
                }
                let mut best = None;
                for i in indices {
                    let v = &a[1].values[i];
                    if lookup_equal(v, &needle, mode == 2) {
                        return lookup_result(&a[2], i, vertical);
                    }
                    let eligible = lookup_kind(v, &needle)
                        && ((mode == -1 && compare(v, &needle).is_le())
                            || (mode == 1 && compare(v, &needle).is_ge()));
                    if eligible
                        && best.is_none_or(|b: usize| {
                            if mode == -1 {
                                compare(v, &a[1].values[b]).is_gt()
                            } else {
                                compare(v, &a[1].values[b]).is_lt()
                            }
                        })
                    {
                        best = Some(i);
                    }
                }
                if let Some(i) = best {
                    lookup_result(&a[2], i, vertical)
                } else if let Some(fallback) = args
                    .get(3)
                    .filter(|expr| !matches!(expr, Expr::Literal(Value::Empty)))
                {
                    self.eval(fallback, sheet, row, col)
                } else {
                    Err("#N/A".into())
                }
            }
            "DATE" => {
                arity(3, 3)?;
                let yr = n(0)?.trunc();
                let mo = n(1)?.trunc();
                let da = n(2)?.trunc();
                if !(0.0..=9999.0).contains(&yr) || mo.abs() > 120000.0 || da.abs() > 3700000.0 {
                    return Err("#NUM!".into());
                }
                let mut year = yr as i64;
                if (0..1900).contains(&year) {
                    year += 1900;
                }
                let month = mo as i64;
                let day = da as i64;
                let year = year + (month - 1).div_euclid(12);
                let month = (month - 1).rem_euclid(12) + 1;
                if !(1900..=9999).contains(&year) {
                    return Err("#NUM!".into());
                }
                let result = date_serial(year, month, 1) + (day - 1) as f64;
                if !(0.0..=2958465.0).contains(&result) {
                    return Err("#NUM!".into());
                }
                num(result)
            }
            "TIME" => {
                arity(3, 3)?;
                let parts = [n(0)?.trunc(), n(1)?.trunc(), n(2)?.trunc()];
                if parts.iter().any(|part| *part > 32767.0) {
                    return Err("#NUM!".into());
                }
                let seconds = parts[0] * 3600.0 + parts[1] * 60.0 + parts[2];
                if seconds < 0.0 {
                    return Err("#NUM!".into());
                }
                num(seconds.rem_euclid(86400.0) / 86400.0)
            }
            "YEAR" | "MONTH" | "DAY" | "HOUR" | "MINUTE" | "SECOND" => {
                arity(1, 1)?;
                let serial = n(0)?;
                if !(0.0..2958466.0).contains(&serial) {
                    return Err("#NUM!".into());
                }
                let (y, m, d) = serial_date(serial);
                let tolerance = (serial.abs()*f64::EPSILON*86400.0*2.0).min(1e-4);
                let sec = (serial.fract()*86400.0+tolerance).floor().min(86399.0) as i64;
                num(match name {
                    "YEAR" => y as f64,
                    "MONTH" => m as f64,
                    "DAY" => d as f64,
                    "HOUR" => ((sec / 3600) % 24) as f64,
                    "MINUTE" => ((sec / 60) % 60) as f64,
                    _ => (sec % 60) as f64,
                })
            }
            "DAYS" => {
                arity(2, 2)?;
                let date = |index: usize| -> CalcResult<f64> {
                    match a.get(index).ok_or("#VALUE!")?.first() {
                        Value::Text(text) => date_text_serial(&text, self.book.locale_ru),
                        value => value.number(),
                    }
                };
                let end = date(0)?;
                let start = date(1)?;
                if !(0.0..2958466.0).contains(&end) || !(0.0..2958466.0).contains(&start) {
                    return Err("#NUM!".into());
                }
                num(end - start)
            }
            "WEEKDAY" => {
                arity(1, 2)?;
                let serial = n(0)?.floor();
                if !(0.0..=2958465.0).contains(&serial) {
                    return Err("#NUM!".into());
                }
                let mode = default_n(1, 1.0)? as i64;
                let dow = (serial as i64 - 1).rem_euclid(7);
                num(match mode {
                    1 => (dow + 1) as f64,
                    2 => (dow + 6).rem_euclid(7) as f64 + 1.0,
                    3 => (dow + 6).rem_euclid(7) as f64,
                    11..=17 => (dow - (mode - 10)).rem_euclid(7) as f64 + 1.0,
                    _ => return Err("#NUM!".into()),
                })
            }
            "EDATE" | "EOMONTH" => {
                arity(2, 2)?;
                let serial = n(0)?.floor();
                let offset = n(1)?.trunc();
                if !(0.0..=2958465.0).contains(&serial) || offset.abs() > 120000.0 {
                    return Err("#NUM!".into());
                }
                let (y, m, d) = serial_date(serial);
                let delta = offset as i64;
                let new = y * 12 + m - 1 + delta;
                let year = new.div_euclid(12);
                let month = new.rem_euclid(12) + 1;
                if !(1900..=9999).contains(&year) {
                    return Err("#NUM!".into());
                }
                let end = days_in_month(year, month);
                num(date_serial(
                    year,
                    month,
                    if name == "EDATE" { d.min(end) } else { end },
                ))
            }
            "DATEVALUE" => {
                arity(1, 1)?;
                num(date_text_serial(&text(0)?, self.book.locale_ru)?)
            }
            "TIMEVALUE" => {
                arity(1, 1)?;
                num(time_text_serial(&text(0)?)?)
            }
            "DATEDIF" => {
                arity(3, 3)?;
                let start = n(0)?.floor();
                let end = n(1)?.floor();
                if !(0.0..=2958465.0).contains(&start)
                    || !(0.0..=2958465.0).contains(&end)
                    || start > end
                {
                    return Err("#NUM!".into());
                }
                let (y1, m1, d1) = serial_date(start);
                let (y2, m2, d2) = serial_date(end);
                let months = (y2 - y1) * 12 + m2 - m1 - i64::from(d2 < d1);
                num(match text(2)?.to_uppercase().as_str() {
                    "D" => end - start,
                    "M" => months as f64,
                    "Y" => months.div_euclid(12) as f64,
                    "YM" => months.rem_euclid(12) as f64,
                    "YD" => {
                        let mut candidate = date_serial(y2, m1, d1);
                        if candidate > end {
                            candidate = date_serial(y2 - 1, m1, d1);
                        }
                        end - candidate
                    }
                    "MD" => {
                        if d2 >= d1 {
                            (d2 - d1) as f64
                        } else {
                            let previous = if m2 == 1 {
                                days_in_month(y2 - 1, 12)
                            } else {
                                days_in_month(y2, m2 - 1)
                            };
                            (previous - d1 + d2) as f64
                        }
                    }
                    _ => return Err("#NUM!".into()),
                })
            }
            "NETWORKDAYS" | "WORKDAY" => {
                arity(2, 3)?;
                let initial = n(0)?.floor();
                if !(0.0..=2958465.0).contains(&initial) {
                    return Err("#VALUE!".into());
                }
                let start = initial as i64;
                let mut holidays = std::collections::BTreeSet::new();
                if let Some(grid) = a.get(2) {
                    for date in numbers(std::slice::from_ref(grid), false)? {
                        if !(0.0..2958466.0).contains(&date) {
                            return Err("#VALUE!".into());
                        }
                        holidays.insert(date.floor() as i64);
                    }
                }
                let amount = n(1)?.trunc();
                if name == "NETWORKDAYS" {
                    if !(0.0..=2958465.0).contains(&amount) {
                        return Err("#VALUE!".into());
                    }
                    let end = amount as i64;
                    let count = working_days(start.min(end), start.max(end), &holidays);
                    num(count as f64 * if end < start { -1.0 } else { 1.0 })
                } else {
                    if amount.abs() > 2958466.0 {
                        return Err("#NUM!".into());
                    }
                    if amount == 0.0 {
                        return num(start as f64);
                    }
                    let wanted = amount.abs() as i64;
                    if amount > 0.0 {
                        if working_days(start + 1, 2958465, &holidays) < wanted {
                            return Err("#NUM!".into());
                        }
                        let (mut low, mut high) = (start + 1, 2958465);
                        while low < high {
                            let middle = low + (high - low) / 2;
                            if working_days(start + 1, middle, &holidays) >= wanted {
                                high = middle;
                            } else {
                                low = middle + 1;
                            }
                        }
                        num(low as f64)
                    } else {
                        if working_days(0, start - 1, &holidays) < wanted {
                            return Err("#NUM!".into());
                        }
                        let (mut low, mut high) = (0, start - 1);
                        while low < high {
                            let middle = low + (high - low + 1) / 2;
                            if working_days(middle, start - 1, &holidays) >= wanted {
                                low = middle;
                            } else {
                                high = middle - 1;
                            }
                        }
                        num(low as f64)
                    }
                }
            }
            "PMT" | "PV" | "FV" => {
                arity(3, 5)?;
                let rate = n(0)?;
                let periods = n(1)?;
                let third = n(2)?;
                let fourth = default_n(3, 0.0)?;
                let kind = if default_n(4, 0.0)? == 0.0 { 0.0 } else { 1.0 };
                if periods == 0.0 && (name == "PMT") {
                    return Err("#DIV/0!".into());
                }
                if rate>0.0 && periods>0.0 && rate.ln_1p()*periods>700.0 && matches!(name,"PMT"|"PV") {
                    let exponent=-rate.ln_1p()*periods;
                    let discounted_future=exponential_scale(fourth,exponent);
                    let difference=-exponent.exp_m1();
                    return if name=="PMT" {
                        num(-(third+discounted_future)*(rate/(1.0+rate*kind))/difference)
                    }else {
                        num(-(third*((1.0+rate*kind)/rate)*difference+discounted_future))
                    };
                }
                let growth = growth(rate, periods);
                let annuity = annuity(rate, periods, kind);
                num(match name {
                    "PMT" => {
                        if annuity == 0.0 {
                            return Err("#DIV/0!".into());
                        } else {
                            -(third * growth + fourth) / annuity
                        }
                    }
                    "PV" => {
                        if growth == 0.0 {
                            return Err("#DIV/0!".into());
                        } else {
                            -(third * annuity + fourth) / growth
                        }
                    }
                    _ => (-fourth) * growth - third * annuity,
                })
            }
            "NPV" => {
                arity(2, 255)?;
                let rate = n(0)?;
                let vals = numbers(&a[1..], false)?;
                if rate == -1.0 {
                    return Err("#DIV/0!".into());
                }
                let discounted=vals.iter().enumerate().map(|(index,value)| {
                    if *value==0.0 {return 0.0;}
                    let base=1.0+rate;
                    let logarithm=if rate> -1.0 {rate.ln_1p()}else{base.abs().ln()};
                    let sign=if base<0.0 && (index+1)%2==1 {-value.signum()}else{value.signum()};
                    sign*(value.abs().ln()-logarithm*(index+1) as f64).exp()
                }).collect::<Vec<_>>();
                num(scaled_sum(&discounted))
            }
            "IRR" => {
                arity(1, 2)?;
                let vals = numbers(&a[..1], false)?;
                let scale = vals.iter().map(|v| v.abs()).fold(0.0, f64::max);
                let vals: Vec<_> = vals.into_iter().map(|v| v / scale).collect();
                if !vals.iter().any(|v| *v < 0.0) || !vals.iter().any(|v| *v > 0.0) {
                    return Err("#NUM!".into());
                }
                let mut rate = default_n(1, 0.1)?;
                for _ in 0..100 {
                    let f = vals
                        .iter()
                        .enumerate()
                        .map(|(i, v)| v / (1.0 + rate).powf(i as f64))
                        .sum::<f64>();
                    let df = vals
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, v)| -(i as f64) * v / (1.0 + rate).powf(i as f64 + 1.0))
                        .sum::<f64>();
                    if df == 0.0 {
                        break;
                    }
                    let next = rate - f / df;
                    if !next.is_finite() || next <= -1.0 {
                        break;
                    }
                    if (next - rate).abs() < 1e-10 {
                        return num(next);
                    }
                    rate = next;
                }
                Err("#NUM!".into())
            }
            "SLN" => {
                arity(3, 3)?;
                let life = n(2)?;
                if life == 0.0 {
                    Err("#DIV/0!".into())
                } else {
                    num((n(0)? - n(1)?) / life)
                }
            }
            "SYD" => {
                arity(4, 4)?;
                let life = n(2)?;
                let period = n(3)?;
                if life <= 0.0 || period <= 0.0 || period > life {
                    Err("#NUM!".into())
                } else {
                    num((n(0)? - n(1)?) * (life - period + 1.0) * 2.0 / (life * (life + 1.0)))
                }
            }
            "NPER" => {
                arity(3, 5)?;
                let rate = n(0)?;
                let payment = n(1)?;
                let present = n(2)?;
                let future = default_n(3, 0.0)?;
                let kind = if default_n(4, 0.0)? == 0.0 { 0.0 } else { 1.0 };
                // NPER is invariant when every cash flow is scaled by the same factor.
                let scale = payment.abs().max(present.abs()).max(future.abs());
                if scale == 0.0 { return Err("#NUM!".into()); }
                let (payment, present, future) = (payment / scale, present / scale, future / scale);
                if rate == 0.0 {
                    if payment == 0.0 {
                        return Err("#NUM!".into());
                    }
                    return num(-(present + future) / payment);
                }
                if rate <= -1.0 {
                    return Err("#NUM!".into());
                }
                let delta =
                    -rate * (present + future) / (payment * (1.0 + rate * kind) + present * rate);
                if !delta.is_finite() || delta <= -1.0 {
                    return Err("#NUM!".into());
                }
                num(delta.ln_1p() / rate.ln_1p())
            }
            "RATE" => {
                arity(3, 6)?;
                let periods = n(0)?;
                let payment = n(1)?;
                let present = n(2)?;
                let future = default_n(3, 0.0)?;
                let kind = if default_n(4, 0.0)? == 0.0 { 0.0 } else { 1.0 };
                let mut rate = default_n(5, 0.1)?;
                if periods <= 0.0 {
                    return Err("#NUM!".into());
                }
                let balance = |r: f64| -> f64 {
                    present * growth(r, periods) + payment * annuity(r, periods, kind) + future
                };
                for _ in 0..100 {
                    let step = 1e-7;
                    let slope = (balance(rate + step) - balance(rate - step)) / (2.0 * step);
                    if slope == 0.0 || !slope.is_finite() {
                        break;
                    }
                    let next = rate - balance(rate) / slope;
                    if !next.is_finite() || next <= -1.0 {
                        break;
                    }
                    if (next - rate).abs() < 1e-10 {
                        return num(next);
                    }
                    rate = next;
                }
                Err("#NUM!".into())
            }
            "IPMT" | "PPMT" => {
                arity(4, 6)?;
                let rate = n(0)?;
                let period = n(1)?.trunc();
                let periods = n(2)?;
                let present = n(3)?;
                let future = default_n(4, 0.0)?;
                let kind = if default_n(5, 0.0)? == 0.0 { 0.0 } else { 1.0 };
                if period < 1.0 || period > periods.ceil() || rate <= -1.0 {
                    return Err("#NUM!".into());
                }
                let annuity = |r: f64, count: f64| annuity(r, count, kind);
                let total = annuity(rate, periods);
                if total == 0.0 {
                    return Err("#DIV/0!".into());
                }
                let payment = -(present * growth(rate, periods) + future) / total;
                // Balance after `count` periods with the FV sign convention.
                let fv = |count: f64| -> f64 {
                    -(present * growth(rate, count) + payment * annuity(rate, count))
                };
                let interest = if period == 1.0 {
                    if kind != 0.0 { 0.0 } else { -present * rate }
                } else if kind != 0.0 {
                    (fv(period - 2.0) - payment) * rate
                } else {
                    fv(period - 1.0) * rate
                };
                num(if name == "IPMT" {
                    interest
                } else {
                    payment - interest
                })
            }
            "MAXIFS" | "MINIFS" => {
                if a.len() < 3 || a.len().is_multiple_of(2) {
                    return Err("#VALUE!".into());
                }
                let target = &a[0];
                let pairs = a[1..].as_chunks::<2>().0;
                for pair in pairs {
                    if let Value::Error(error) = pair[1].first() {
                        return Err(error);
                    }
                }
                if pairs
                    .iter()
                    .any(|p| p[0].rows != target.rows || p[0].cols != target.cols)
                {
                    return Err("#VALUE!".into());
                }
                let mut best: Option<f64> = None;
                for (i, value) in target.values.iter().enumerate() {
                    if !pairs.iter().all(|p| {
                        p[0].values
                            .get(i)
                            .is_some_and(|v| criterion(v, &p[1].first()))
                    }) {
                        continue;
                    }
                    match value {
                        Value::Number(_) | Value::Empty => {
                            let x = if let Value::Number(x) = value { *x } else { 0.0 };
                            best = Some(match best {
                                Some(b) if name == "MAXIFS" => b.max(x),
                                Some(b) => b.min(x),
                                None => x,
                            });
                        }
                        Value::Error(e) => return Err(e.clone()),
                        _ => (),
                    }
                }
                num(best.unwrap_or(0.0))
            }
            "MAXA" | "MINA" => {
                arity(1, 255)?;
                let vals = numbers(a, true)?;
                let pick = if name == "MAXA" { f64::max } else { f64::min };
                num(vals.into_iter().reduce(pick).unwrap_or(0.0))
            }
            "TRIMMEAN" => {
                arity(2, 2)?;
                let mut vals = numbers(&a[..1], false)?;
                let percent = n(1)?;
                if vals.is_empty() || !(0.0..1.0).contains(&percent) {
                    return Err("#NUM!".into());
                }
                vals.sort_by(f64::total_cmp);
                let trim = (vals.len() as f64 * percent).floor() as usize / 2;
                let kept = &vals[trim..vals.len() - trim];
                let scale = kept.iter().copied().map(f64::abs).fold(0.0,f64::max);
                num(if scale == 0.0 { 0.0 } else {
                    kept.iter().map(|value| value/scale).sum::<f64>()/kept.len() as f64*scale
                })
            }
            "SKEW" | "KURT" => {
                arity(1, 255)?;
                let vals = numbers(a, false)?;
                let count = vals.len() as f64;
                let minimum = if name == "SKEW" { 3.0 } else { 4.0 };
                if count < minimum {
                    return Err("#DIV/0!".into());
                }
                let scale = vals.iter().copied().map(f64::abs).fold(0.0,f64::max);
                if scale == 0.0 { return Err("#DIV/0!".into()); }
                let vals: Vec<_> = vals.iter().map(|value| value/scale).collect();
                let mean = vals.iter().sum::<f64>() / count;
                let sd =
                    (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1.0)).sqrt();
                if sd == 0.0 {
                    return Err("#DIV/0!".into());
                }
                let moment = |power: i32| {
                    vals.iter()
                        .map(|v| ((v - mean) / sd).powi(power))
                        .sum::<f64>()
                };
                num(if name == "SKEW" {
                    count / ((count - 1.0) * (count - 2.0)) * moment(3)
                } else {
                    count * (count + 1.0) / ((count - 1.0) * (count - 2.0) * (count - 3.0))
                        * moment(4)
                        - 3.0 * (count - 1.0).powi(2) / ((count - 2.0) * (count - 3.0))
                })
            }
            "CORREL" | "PEARSON" | "RSQ" | "SLOPE" | "INTERCEPT" | "COVAR" | "COVARIANCE.P"
            | "COVARIANCE.S" => {
                arity(2, 2)?;
                if a[0].values.len() != a[1].values.len() {
                    return Err("#N/A".into());
                }
                let mut pairs = Vec::new();
                for (y, x) in a[0].values.iter().zip(&a[1].values) {
                    match (y, x) {
                        (Value::Error(e), _) | (_, Value::Error(e)) => return Err(e.clone()),
                        (Value::Number(y), Value::Number(x)) => pairs.push((*y, *x)),
                        _ => (),
                    }
                }
                if pairs.is_empty() {
                    return Err("#DIV/0!".into());
                }
                let count = pairs.len() as f64;
                if matches!(name, "CORREL" | "PEARSON" | "RSQ") {
                    let scale_y = pairs.iter().map(|p| p.0.abs()).fold(0.0, f64::max);
                    let scale_x = pairs.iter().map(|p| p.1.abs()).fold(0.0, f64::max);
                    if scale_x == 0.0 || scale_y == 0.0 {
                        return Err("#DIV/0!".into());
                    }
                    let mean_y = pairs.iter().map(|p| p.0 / scale_y).sum::<f64>() / count;
                    let mean_x = pairs.iter().map(|p| p.1 / scale_x).sum::<f64>() / count;
                    let (mut xy, mut xx, mut yy) = (0.0, 0.0, 0.0);
                    for (y, x) in &pairs {
                        let x = x / scale_x - mean_x;
                        let y = y / scale_y - mean_y;
                        xy += x * y;
                        xx += x * x;
                        yy += y * y;
                    }
                    if xx == 0.0 || yy == 0.0 {
                        return Err("#DIV/0!".into());
                    }
                    let correlation = (xy / (xx.sqrt() * yy.sqrt())).clamp(-1.0, 1.0);
                    return num(if name == "RSQ" {
                        correlation * correlation
                    } else {
                        correlation
                    });
                }
                let scale_y = pairs.iter().map(|p| p.0.abs()).fold(0.0,f64::max).max(f64::MIN_POSITIVE);
                let scale_x = pairs.iter().map(|p| p.1.abs()).fold(0.0,f64::max).max(f64::MIN_POSITIVE);
                let pairs: Vec<_> = pairs.iter().map(|(y,x)| (y/scale_y,x/scale_x)).collect();
                let mean_y = pairs.iter().map(|p| p.0).sum::<f64>() / count;
                let mean_x = pairs.iter().map(|p| p.1).sum::<f64>() / count;
                let sxy = pairs
                    .iter()
                    .map(|(y, x)| (x - mean_x) * (y - mean_y))
                    .sum::<f64>();
                let sxx = pairs.iter().map(|p| (p.1 - mean_x).powi(2)).sum::<f64>();
                let syy = pairs.iter().map(|p| (p.0 - mean_y).powi(2)).sum::<f64>();
                match name {
                    "COVAR" | "COVARIANCE.P" => num(scaled_product(sxy/count,scale_y,scale_x)),
                    "COVARIANCE.S" => {
                        if pairs.len() < 2 {
                            Err("#DIV/0!".into())
                        } else {
                            num(scaled_product(sxy/(count-1.0),scale_y,scale_x))
                        }
                    }
                    "SLOPE" | "INTERCEPT" => {
                        if sxx == 0.0 {
                            return Err("#DIV/0!".into());
                        }
                        let normalized_slope = sxy/sxx;
                        num(if name == "SLOPE" {
                            let ratio=scale_y/scale_x;
                            if normalized_slope == 0.0 { 0.0 }
                            else if ratio.is_finite() && ratio != 0.0 { normalized_slope*ratio }
                            else { (normalized_slope*scale_y)/scale_x }
                        } else {
                            (mean_y-normalized_slope*mean_x)*scale_y
                        })
                    }
                    _ => {
                        let scale = (sxx * syy).sqrt();
                        if scale == 0.0 {
                            return Err("#DIV/0!".into());
                        }
                        let r = sxy / scale;
                        num(if name == "RSQ" { r * r } else { r })
                    }
                }
            }
            "TEXTBEFORE" | "TEXTAFTER" => {
                arity(2, 6)?;
                let chars: Vec<char> = text(0)?.chars().collect();
                let delimiter = text(1)?;
                let instance = default_n(2, 1.0)?.trunc();
                let match_mode = default_n(3, 0.0)?;
                let end_mode = default_n(4, 0.0)?;
                if !matches!(match_mode, 0.0 | 1.0) || !matches!(end_mode, 0.0 | 1.0) {
                    return Err("#VALUE!".into());
                }
                let insensitive = match_mode == 1.0;
                let match_end = end_mode == 1.0;
                let len = chars.len();
                if instance == 0.0 || instance.abs() > len.max(1) as f64 {
                    return Err("#VALUE!".into());
                }
                if chars.is_empty() {
                    return string(String::new());
                }
                let fold = |c: char| -> char {
                    if insensitive {
                        c.to_lowercase().next().unwrap_or(c)
                    } else {
                        c
                    }
                };
                let hay: Vec<char> = chars.iter().map(|c| fold(*c)).collect();
                let needle: Vec<char> = delimiter.chars().map(fold).collect();
                let mut found: Vec<(usize, usize)> = Vec::new();
                if needle.is_empty() {
                    return string(if (name == "TEXTBEFORE") == (instance > 0.0) {
                        String::new()
                    } else { chars.iter().collect() });
                } else {
                    if instance > 0.0 {
                        let mut i = 0;
                        while i + needle.len() <= len {
                            if hay[i..i + needle.len()] == needle[..] {
                                found.push((i, i + needle.len()));
                                i += needle.len();
                            } else {
                                i += 1;
                            }
                        }
                    } else {
                        let mut end = len;
                        while end >= needle.len() {
                            let start = end - needle.len();
                            if hay[start..end] == needle[..] {
                                found.push((start, end));
                                end = start;
                            } else {
                                end -= 1;
                            }
                        }
                        found.reverse();
                    }
                    if match_end {
                        if instance > 0.0 {
                            found.push((len, len));
                        } else {
                            found.insert(0, (0, 0));
                        }
                    }
                }
                let wanted = instance.abs() as usize;
                let hit = if instance > 0.0 {
                    found.get(wanted - 1)
                } else {
                    found
                        .len()
                        .checked_sub(wanted)
                        .and_then(|index| found.get(index))
                };
                match hit {
                    Some(&(start, end)) => string(if name == "TEXTBEFORE" {
                        chars[..start].iter().collect()
                    } else {
                        chars[end..].iter().collect()
                    }),
                    None => match args.get(5) {
                        Some(fallback) => self.eval(fallback, sheet, row, col),
                        None => Err("#N/A".into()),
                    },
                }
            }
            "ROMAN" => {
                arity(1,2)?;
                let value=n(0)?.trunc();
                if !(0.0..=3999.0).contains(&value) {return Err("#VALUE!".into());}
                let form=match a.get(1).map(Grid::first) {
                    Some(Value::Bool(true))=>0.0,Some(Value::Bool(false))=>4.0,
                    _=>default_n(1,0.0)?.trunc(),
                };
                if !(0.0..=4.0).contains(&form) {return Err("#VALUE!".into());}
                string(roman_form(value as u32,form as usize))
            }
            "ARABIC" => {
                arity(1, 1)?;
                let source = text(0)?;
                let trimmed = source.trim();
                let (negative, digits) = match trimmed.strip_prefix('-') {
                    Some(rest) => (true, rest),
                    None => (false, trimmed),
                };
                if digits.len() > 255 {
                    return Err("#VALUE!".into());
                }
                let values = digits
                    .chars()
                    .map(|c| match c.to_ascii_uppercase() {
                        'I' => Ok(1.0),
                        'V' => Ok(5.0),
                        'X' => Ok(10.0),
                        'L' => Ok(50.0),
                        'C' => Ok(100.0),
                        'D' => Ok(500.0),
                        'M' => Ok(1000.0),
                        _ => Err("#VALUE!".to_string()),
                    })
                    .collect::<CalcResult<Vec<f64>>>()?;
                let mut total = 0.0;
                for (i, value) in values.iter().enumerate() {
                    if values.get(i + 1).is_some_and(|next| next > value) {
                        total -= value;
                    } else {
                        total += value;
                    }
                }
                num(if negative { -total } else { total })
            }
            "DEC2BIN" | "DEC2OCT" | "DEC2HEX" => {
                arity(1, 2)?;
                let (radix, bits) = radix_bits(name);
                let x = n(0)?.trunc();
                let limit = 2f64.powi(bits - 1);
                if x < -limit || x >= limit {
                    return Err("#NUM!".into());
                }
                // Negative numbers use the 10-digit two's complement form, as Excel does.
                let value = (if x < 0.0 { x + 2.0 * limit } else { x }) as u64;
                let digits = to_base(value, radix);
                if x < 0.0 {
                    return string(digits);
                }
                match a.get(1).filter(|g| g.first() != Value::Empty) {
                    Some(places) => {
                        let places = places.number()?.trunc();
                        if places < digits.len() as f64 || places > 10.0 {
                            return Err("#NUM!".into());
                        }
                        string(format!("{digits:0>width$}", width = places as usize))
                    }
                    None => string(digits),
                }
            }
            "BIN2DEC" | "OCT2DEC" | "HEX2DEC" => {
                arity(1, 1)?;
                let (radix, bits) = radix_bits(name);
                let source = text(0)?;
                let digits = source.trim();
                if digits.len() > 10 || !digits.chars().all(|c| c.is_digit(radix)) {
                    return Err("#NUM!".into());
                }
                let value = if digits.is_empty() {
                    0
                } else {
                    u64::from_str_radix(digits, radix).map_err(|_| "#NUM!".to_string())?
                };
                let limit = 1u64 << (bits - 1);
                num(if value >= limit {
                    value as f64 - 2.0 * limit as f64
                } else {
                    value as f64
                })
            }
            "BASE" => {
                arity(2, 3)?;
                let x = n(0)?.trunc();
                let radix = n(1)?.trunc();
                let width = default_n(2, 0.0)?.trunc();
                if !(0.0..9007199254740992.0).contains(&x)
                    || !(2.0..=36.0).contains(&radix)
                    || !(0.0..=255.0).contains(&width)
                {
                    return Err("#NUM!".into());
                }
                let digits = to_base(x as u64, radix as u32);
                string(format!("{digits:0>width$}", width = width as usize))
            }
            "DECIMAL" => {
                arity(2, 2)?;
                let source = text(0)?;
                let radix = n(1)?.trunc();
                if !(2.0..=36.0).contains(&radix) {
                    return Err("#NUM!".into());
                }
                let radix = radix as u32;
                let digits = source.trim();
                if digits.len() > 255 || !digits.chars().all(|c| c.is_digit(radix)) {
                    return Err("#NUM!".into());
                }
                if digits.is_empty() {
                    return num(0.0);
                }
                let mut value = 0.0;
                for digit in digits.chars() {
                    value = finite(
                        value * f64::from(radix) + f64::from(digit.to_digit(radix).ok_or("#NUM!")?),
                    )?;
                }
                num(value)
            }
            "WEEKNUM" | "ISOWEEKNUM" => {
                arity(1, if name == "WEEKNUM" { 2 } else { 1 })?;
                let serial = n(0)?.floor();
                if !(1.0..=2958465.0).contains(&serial) {
                    return Err("#NUM!".into());
                }
                let day = serial as i64;
                let mode = default_n(1, 1.0)? as i64;
                if name == "ISOWEEKNUM" || mode == 21 {
                    // ISO 8601: the week belongs to the year that holds its Thursday.
                    let thursday = day - (day - 2).rem_euclid(7) + 3;
                    let (year, _, _) = serial_date(thursday as f64);
                    let jan1 = date_serial(year, 1, 1) as i64;
                    return num(((thursday - jan1) / 7 + 1) as f64);
                }
                let first = match mode {
                    1 | 17 => 0,
                    2 | 11 => 1,
                    12..=16 => mode - 10,
                    _ => return Err("#NUM!".into()),
                };
                let (year, _, _) = serial_date(serial);
                let jan1 = date_serial(year, 1, 1) as i64;
                let offset = ((jan1 - 1).rem_euclid(7) - first).rem_euclid(7);
                num(((day - jan1 + offset) / 7 + 1) as f64)
            }
            "DAYS360" => {
                arity(2, 3)?;
                let start = n(0)?.floor();
                let end = n(1)?.floor();
                if !(0.0..=2958465.0).contains(&start) || !(0.0..=2958465.0).contains(&end) {
                    return Err("#NUM!".into());
                }
                let european = a.get(2).map(Grid::boolean).transpose()?.unwrap_or(false);
                let (y1, m1, mut d1) = serial_date(start);
                let (y2, mut m2, mut d2) = serial_date(end);
                if european {
                    d1 = d1.min(30);
                    d2 = d2.min(30);
                } else {
                    if d1 == days_in_month(y1, m1) {
                        d1 = 30;
                    }
                    if d2 == days_in_month(y2, m2) {
                        if d1 < 30 {
                            d2 = 1;
                            m2 += 1;
                        } else {
                            d2 = 30;
                        }
                    }
                }
                num(((y2 - y1) * 360 + (m2 - m1) * 30 + d2 - d1) as f64)
            }
            _ => Err("#NAME?".into()),
        }
    }
}
fn ansi_encoding(ru: bool) -> &'static encoding_rs::Encoding {
    if ru {
        encoding_rs::WINDOWS_1251
    } else {
        encoding_rs::WINDOWS_1252
    }
}
fn time_text_serial(source: &str) -> CalcResult<f64> {
    if let Some((date,time)) = split_date_time(source.trim()) {
        date_text_serial(date,false).or_else(|_| date_text_serial(date,true))?;
        clock_text_serial(time)
    } else { clock_text_serial(source) }
}
fn clock_text_serial(source: &str) -> CalcResult<f64> {
    let input = source.trim().to_ascii_uppercase();
    let (clock, afternoon) = if let Some(clock) = input.strip_suffix("AM") {
        (clock.trim(), Some(false))
    } else if let Some(clock) = input.strip_suffix("PM") {
        (clock.trim(), Some(true))
    } else {
        (input.as_str(), None)
    };
    let vals = clock
        .split(':')
        .map(|part| part.trim().parse::<f64>().map_err(|_| "#VALUE!"))
        .collect::<Result<Vec<_>, _>>()?;
    if !(if afternoon.is_some() { 1 } else { 2 }..=3).contains(&vals.len()) {
        return Err("#VALUE!".into());
    }
    let mut hour = vals[0];
    let minute = vals.get(1).copied().unwrap_or(0.0);
    let second = vals.get(2).copied().unwrap_or(0.0);
    if vals.iter().any(|v| !v.is_finite() || *v < 0.0)
        || hour.fract() != 0.0 || minute.fract() != 0.0
        || hour > if afternoon.is_some() { 12.0 } else { 23.0 }
        || minute >= 60.0 || second >= 60.0 {
        return Err("#VALUE!".into());
    }
    if let Some(afternoon) = afternoon {
        hour = hour % 12.0 + if afternoon { 12.0 } else { 0.0 };
    }
    Ok((hour*3600.0+minute*60.0+second)/86400.0)
}
fn value_text(source: &str, ru: bool) -> CalcResult<f64> {
    let text = source.trim();
    if let Some((date,time)) = split_date_time(text)
        && let (Ok(date),Ok(time)) = (date_text_serial(date,ru),clock_text_serial(time)) {
        return Ok(date+time);
    }
    let numeric = text.trim_end_matches('%').replace(',',".").parse::<f64>().is_ok();
    if !numeric && let Ok(date) = date_text_serial(text, ru) {
        return Ok(date);
    }
    if let Ok(time) = time_text_serial(text) {
        return Ok(time);
    }
    for (offset, _) in text.char_indices().filter(|(_, c)| matches!(c, ' ' | 'T')) {
        if let (Ok(date), Ok(time)) = (
            date_text_serial(&text[..offset], ru),
            time_text_serial(&text[offset + 1..]),
        ) {
            return Ok(date + time);
        }
    }
    let mut number = text;
    let mut negative = false;
    if let Some(inner) = number.strip_prefix('(').and_then(|n| n.strip_suffix(')')) {
        number = inner.trim();
        negative = true;
    }
    let currency = |c: char| matches!(c, '$' | '€' | '£' | '¥' | '₽' | '₸');
    number = number.trim_matches(currency).trim();
    let mut percent = 0usize;
    while let Some(inner) = number.strip_suffix('%') {
        number = inner.trim_end();
        percent += 1;
    }
    let decimal = match (number.rfind(','), number.rfind('.')) {
        (Some(comma), Some(dot)) => {
            if comma > dot {
                ','
            } else {
                '.'
            }
        }
        _ if ru && number.contains(',') => ',',
        _ => '.',
    };
    let group = if decimal == ',' { '.' } else { ',' };
    let compact: String = number
        .chars()
        .filter(|c| !matches!(c, ' ' | '\u{a0}'))
        .collect();
    let mut normalized = String::new();
    let mut fractional = false;
    for c in compact.chars() {
        if c == decimal {
            if fractional {
                return Err("#VALUE!".into());
            }
            normalized.push('.');
            fractional = true;
        } else if c == group {
            if fractional {
                return Err("#VALUE!".into());
            }
        } else {
            normalized.push(c);
        }
    }
    let mut value = normalized.parse::<f64>().map_err(|_| "#VALUE!")?;
    if !value.is_finite() {
        return Err("#NUM!".into());
    }
    for _ in 0..percent {
        value /= 100.0;
    }
    finite(if negative { -value } else { value })
}
fn working_days(first: i64, last: i64, holidays: &std::collections::BTreeSet<i64>) -> i64 {
    if first > last {
        return 0;
    }
    let days = last - first + 1;
    let weekday = |date: i64| (date - 2).rem_euclid(7) < 5;
    days / 7 * 5
        + (0..days % 7)
            .filter(|offset| weekday(first + offset))
            .count() as i64
        - holidays
            .range(first..=last)
            .filter(|date| weekday(**date))
            .count() as i64
}
fn growth(rate: f64, periods: f64) -> f64 {
    if rate > -1.0 {
        (rate.ln_1p() * periods).exp()
    } else {
        (1.0 + rate).powf(periods)
    }
}
fn annuity(rate: f64, periods: f64, kind: f64) -> f64 {
    if rate == 0.0 {
        periods
    } else if rate > -1.0 {
        (rate.ln_1p() * periods).exp_m1() / rate * (1.0 + rate * kind)
    } else {
        ((1.0 + rate).powf(periods) - 1.0) / rate * (1.0 + rate * kind)
    }
}
fn interpolate(low: f64, high: f64, fraction: f64) -> f64 {
    if low.is_sign_negative() == high.is_sign_negative() {
        low + (high - low) * fraction
    } else {
        low * (1.0 - fraction) + high * fraction
    }
}
fn split_date_time(source: &str) -> Option<(&str,&str)> {
    for (offset,ch) in source.char_indices() {
        if ch != 'T' && !ch.is_whitespace() { continue; }
        let time = source[offset+ch.len_utf8()..].trim();
        let token = time.split_whitespace().next().unwrap_or("");
        if token.contains(':') {
            return Some((source[..offset].trim(),time));
        }
    }
    None
}
fn date_text_serial(source: &str, locale_ru: bool) -> CalcResult<f64> {
    let source = source.trim();
    let date = if let Some((date,time)) = split_date_time(source) {
        clock_text_serial(time)?;
        date
    } else { source };
    if date.starts_with('-') { return Err("#VALUE!".into()); }
    let pieces: Vec<_> = date.split(['-', '/', '.', ' ']).filter(|piece| !piece.is_empty()).collect();
    if !(2..=3).contains(&pieces.len()) { return Err("#VALUE!".into()); }
    let month_name = |source: &str| -> Option<i64> {
        Some(match source.to_lowercase().as_str() {
            "jan"|"january"|"янв"|"январь"|"января" => 1,
            "feb"|"february"|"фев"|"февраль"|"февраля" => 2,
            "mar"|"march"|"мар"|"март"|"марта" => 3,
            "apr"|"april"|"апр"|"апрель"|"апреля" => 4,
            "may"|"май"|"мая" => 5,
            "jun"|"june"|"июн"|"июнь"|"июня" => 6,
            "jul"|"july"|"июл"|"июль"|"июля" => 7,
            "aug"|"august"|"авг"|"август"|"августа" => 8,
            "sep"|"sept"|"september"|"сен"|"сентябрь"|"сентября" => 9,
            "oct"|"october"|"окт"|"октябрь"|"октября" => 10,
            "nov"|"november"|"ноя"|"ноябрь"|"ноября" => 11,
            "dec"|"december"|"дек"|"декабрь"|"декабря" => 12,
            _ => return None,
        })
    };
    let integer = |piece: &str| piece.parse::<i64>().map_err(|_| "#VALUE!".to_string());
    let explicit_year = pieces.get(2).copied();
    let year_token = if pieces[0].len() == 4 && pieces.len() == 3 { Some(pieces[0]) } else { explicit_year };
    let (mut year,month,day) = if pieces[0].len() == 4 && pieces.len() == 3 {
        (integer(pieces[0])?,integer(pieces[1])?,integer(pieces[2])?)
    } else {
        let year = explicit_year.map(integer).transpose()?.unwrap_or_else(|| i64::from(chrono::Local::now().year()));
        if let Some(month) = month_name(pieces[0]) {
            (year,month,integer(pieces[1])?)
        } else if let Some(month) = month_name(pieces[1]) {
            (year,month,integer(pieces[0])?)
        } else if locale_ru {
            (year,integer(pieces[1])?,integer(pieces[0])?)
        } else {
            (year,integer(pieces[0])?,integer(pieces[1])?)
        }
    };
    if year_token.is_some_and(|piece| piece.len() <= 2) && (0..100).contains(&year) {
        year += if year <= 29 { 2000 } else { 1900 };
    }
    if !(1900..=9999).contains(&year) || !(1..=12).contains(&month)
        || day < 1 || day > days_in_month(year,month) {
        return Err("#VALUE!".into());
    }
    Ok(date_serial(year,month,day))
}

fn contains_subtotal(expr: &Expr) -> bool {
    match expr {
        Expr::Call(name, args) => name == "SUBTOTAL" || args.iter().any(contains_subtotal),
        Expr::Unary(_, expr) => contains_subtotal(expr),
        Expr::Binary(_, left, right) => contains_subtotal(left) || contains_subtotal(right),
        _ => false,
    }
}
fn lookup_kind(value: &Value, needle: &Value) -> bool {
    matches!(
        (value, needle),
        (Value::Number(_), Value::Number(_) | Value::Empty)
            | (Value::Text(_), Value::Text(_))
            | (Value::Bool(_), Value::Bool(_))
    )
}
fn power(base: f64, exponent: f64) -> CalcResult<f64> {
    if base == 0.0 && exponent == 0.0 {
        return Err("#NUM!".into());
    }
    if base == 0.0 && exponent < 0.0 {
        return Err("#DIV/0!".into());
    }
    finite(base.powf(exponent))
}
fn formula_result(value: Value) -> Value {
    if value == Value::Empty {
        Value::Number(0.0)
    } else {
        value
    }
}
fn finite(value: f64) -> CalcResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("#NUM!".into())
    }
}
fn literal(input: &str) -> Value {
    if input.is_empty() {
        return Value::Empty;
    }
    if let Some(s) = input.strip_prefix('\'') {
        return Value::Text(s.into());
    }
    if input.eq_ignore_ascii_case("TRUE") || input.eq_ignore_ascii_case("ИСТИНА") {
        return Value::Bool(true);
    }
    if input.eq_ignore_ascii_case("FALSE") || input.eq_ignore_ascii_case("ЛОЖЬ") {
        return Value::Bool(false);
    }
    if matches!(
        input,
        "#REF!" | "#DIV/0!" | "#VALUE!" | "#NAME?" | "#N/A" | "#NUM!" | "#NULL!"
    ) {
        return Value::Error(input.into());
    }
    let s = input.trim();
    let (s, mult) = if let Some(s) = s.strip_suffix('%') {
        (s, 0.01)
    } else {
        (s, 1.0)
    };
    if let Ok(n) = s.replace(',', ".").parse::<f64>()
        && n.is_finite()
    {
        return Value::Number(n * mult);
    }
    Value::Text(input.into())
}
fn numbers(grids: &[Grid], text_zero: bool) -> CalcResult<Vec<f64>> {
    let mut vals = Vec::new();
    for g in grids {
        for value in &g.values {
            match value {
                Value::Number(n) => vals.push(*n),
                Value::Error(e) => return Err(e.clone()),
                Value::Bool(b) if text_zero || !g.reference => vals.push(f64::from(u8::from(*b))),
                Value::Text(_) if text_zero => {
                    vals.push(if g.reference {
                        0.0
                    } else {
                        value.number().unwrap_or(0.0)
                    });
                }
                Value::Text(_) if !g.reference => vals.push(value.number()?),
                _ => (),
            }
        }
    }
    Ok(vals)
}
fn compare(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::Empty, Value::Number(b)) => {
            0f64.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::Number(a), Value::Empty) => {
            a.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::Text(a), Value::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (Value::Empty, Value::Text(b)) => String::new().cmp(&b.to_lowercase()),
        (Value::Text(a), Value::Empty) => a.to_lowercase().cmp(&String::new()),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Empty, Value::Bool(b)) => false.cmp(b),
        (Value::Bool(a), Value::Empty) => a.cmp(&false),
        (Value::Error(a), Value::Error(b)) => a.cmp(b),
        (Value::Empty, Value::Empty) => std::cmp::Ordering::Equal,
        _ => {
            let kind = |v: &Value| match v {
                Value::Empty | Value::Number(_) => 0,
                Value::Text(_) => 1,
                Value::Bool(_) => 2,
                Value::Error(_) => 3,
            };
            kind(a).cmp(&kind(b))
        }
    }
}

fn lookup_equal(value: &Value, needle: &Value, wildcards: bool) -> bool {
    if wildcards && let (Value::Text(value), Value::Text(pattern)) = (value, needle) {
        return wildcard(
            &pattern.to_lowercase().chars().collect::<Vec<_>>(),
            &value.to_lowercase().chars().collect::<Vec<_>>(),
        );
    }
    compare(value, needle).is_eq()
}
#[cfg(test)]
fn wildcard_prefix(pattern: &str, text: &str) -> bool {
    let mut p: Vec<char> = pattern.chars().collect();
    if p.iter().rev().take_while(|ch|**ch=='~').count()%2==1 {p.push('~');}
    p.push('*');
    wildcard(&p, &text.chars().collect::<Vec<_>>())
}
fn wildcard(pattern: &[char], text: &[char]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let mut star = None;
    let mut mark = 0;
    while ti < text.len() {
        let escaped=pattern.get(pi)==Some(&'~')
            && pattern.get(pi+1).is_some_and(|ch|matches!(ch,'*'|'?'|'~'));
        if escaped && pattern.get(pi+1)==text.get(ti) {
            pi+=2;ti+=1;
        } else if !escaped && (pattern.get(pi)==Some(&'?') || pattern.get(pi)==text.get(ti)) {
            pi += 1;
            ti += 1;
        } else if pattern.get(pi) == Some(&'*') {
            star = Some(pi);
            pi += 1;
            mark = ti;
        } else if let Some(s) = star {
            mark += 1;
            ti = mark;
            pi = s + 1;
        } else {
            return false;
        }
    }
    while pattern.get(pi) == Some(&'*') {
        pi += 1;
    }
    pi == pattern.len()
}
fn criterion(value: &Value, criteria: &Value) -> bool {
    let raw = if *criteria == Value::Empty {
        "0".into()
    } else {
        criteria.display()
    };
    let (op, rest) = ["<>", ">=", "<=", "=", ">", "<"]
        .iter()
        .find_map(|op| raw.strip_prefix(op).map(|rest| (*op, rest)))
        .unwrap_or(("=", raw.as_str()));
    if rest.is_empty() && matches!(op, "=" | "<>") {
        let empty = matches!(value, Value::Empty) || matches!(value,Value::Text(s)if s.is_empty());
        return if op == "=" { empty } else { !empty };
    }
    let target = literal(rest);
    // A criterion only matches values of its own kind: ">5" skips text and blanks, while
    // numbers stored as text still count as numbers.
    let coerced;
    let value = match (&target, value) {
        (Value::Number(_), Value::Text(s)) => {
            coerced = literal(s);
            &coerced
        }
        _ => value,
    };
    if std::mem::discriminant(value) != std::mem::discriminant(&target) {
        return op == "<>";
    }
    if matches!(op, "=" | "<>") && matches!(target, Value::Text(_)) {
        let matches = wildcard(
            &rest.to_lowercase().chars().collect::<Vec<_>>(),
            &value.display().to_lowercase().chars().collect::<Vec<_>>(),
        );
        return if op == "=" { matches } else { !matches };
    }
    let order = compare(value, &target);
    match op {
        "=" => order.is_eq(),
        "<>" => !order.is_eq(),
        ">" => order.is_gt(),
        "<" => order.is_lt(),
        ">=" => !order.is_lt(),
        "<=" => !order.is_gt(),
        _ => false,
    }
}
fn conditional(name: &str, a: &[Grid]) -> CalcResult<f64> {
    let plural = name.ends_with("IFS");
    let count = name.starts_with("COUNT");
    let average = name.starts_with("AVERAGE");
    let (sum, conditions): (&Grid, Vec<(&Grid, &Grid)>) = if plural {
        let offset = usize::from(!count);
        if a.len() < offset + 2 || !(a.len() - offset).is_multiple_of(2) {
            return Err("#VALUE!".into());
        }
        (
            &a[0],
            a[offset..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| (&p[0], &p[1]))
                .collect(),
        )
    } else {
        if !(2..=if count { 2 } else { 3 }).contains(&a.len()) {
            return Err("#VALUE!".into());
        }
        (a.get(2).unwrap_or(&a[0]), vec![(&a[0], &a[1])])
    };
    for (_, criteria) in &conditions {
        if let Value::Error(error) = criteria.first() {
            return Err(error);
        }
    }
    if conditions.iter().any(|(range, _)| {
        range.values.len() != sum.values.len()
            || plural && (range.rows != sum.rows || range.cols != sum.cols)
    }) {
        return Err("#VALUE!".into());
    }
    let mut values=Vec::new();
    let mut result = 0.0;
    let mut matched = 0;
    for i in 0..sum.values.len() {
        if conditions
            .iter()
            .all(|(g, c)| g.values.get(i).is_some_and(|v| criterion(v, &c.first())))
        {
            if count {
                result += 1.0;
            } else if let Some(Value::Number(v)) = sum.values.get(i) {
                values.push(*v);
                matched += 1;
            } else if let Some(Value::Error(e)) = sum.values.get(i) {
                return Err(e.clone());
            }
        }
    }
    if average {
        if matched == 0 {
            Err("#DIV/0!".into())
        } else {
            let scale=values.iter().map(|value|value.abs()).fold(0.0,f64::max);
            Ok(if scale==0.0 {0.0}else {
                scaled_sum(&values.iter().map(|value|value/scale).collect::<Vec<_>>())/f64::from(matched)*scale
            })
        }
    } else {
        Ok(if count {result}else{scaled_sum(&values)})
    }
}
const ROMAN: [(u32, &str); 13] = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];
/// Radix and two's-complement width of the Excel DEC2x/x2DEC families.
fn radix_bits(name: &str) -> (u32, i32) {
    match name {
        "DEC2BIN" | "BIN2DEC" => (2, 10),
        "DEC2OCT" | "OCT2DEC" => (8, 30),
        _ => (16, 40),
    }
}
fn to_base(mut value: u64, radix: u32) -> String {
    if value == 0 {
        return "0".into();
    }
    let mut digits = Vec::new();
    while value > 0 {
        let digit = (value % u64::from(radix)) as u32;
        digits.push(
            char::from_digit(digit, radix)
                .unwrap_or('0')
                .to_ascii_uppercase(),
        );
        value /= u64::from(radix);
    }
    digits.iter().rev().collect()
}
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
pub(super) fn calendar_serial(now: chrono::NaiveDateTime) -> f64 {
    date_serial(
        i64::from(now.year()),
        i64::from(now.month()),
        i64::from(now.day()),
    ) + (f64::from(now.num_seconds_from_midnight()) + f64::from(now.nanosecond()) / 1_000_000_000.0)
        / 86400.0
}
pub fn date_serial(year: i64, month: i64, day: i64) -> f64 {
    if year == 1900 && month == 2 && day == 29 {
        return 60.0;
    }
    let days = days_from_civil(year, month, day) - days_from_civil(1899, 12, 31);
    (days + if days >= 60 { 1 } else { 0 }) as f64
}
pub fn serial_date(serial: f64) -> (i64, i64, i64) {
    let day = serial.floor() as i64;
    if day == 0 {
        return (1900, 1, 0);
    }
    if day == 60 {
        return (1900, 2, 29);
    }
    civil_from_days(days_from_civil(1899, 12, 31) + day - if day > 60 { 1 } else { 0 })
}
pub fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year == 1900 || year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    (y, m, d)
}
pub fn validate(input: &str) -> CalcResult<()> {
    if !input.starts_with('=') {
        return Ok(());
    }
    parse(input).map(|_| ())
}
pub fn evaluate(book: &Book, sheet: u64, input: &str) -> Value {
    let Some(si) = book.sheets.iter().position(|s| s.id == sheet) else {
        return Value::Error("#REF!".into());
    };
    if !input.starts_with('=') {
        return literal(input);
    }
    match parse(input).and_then(|expr| Engine::new(book).eval(&expr, si, 0, 0)) {
        Ok(g) => formula_result(g.first()),
        Err(e) => Value::Error(e),
    }
}
pub fn recalculate(book: &mut Book) {
    let keys: Vec<_> = book
        .sheets
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            s.cells
                .keys()
                .filter_map(move |key| parse_address(key).map(|(r, c)| (i, r, c)))
        })
        .collect();
    let values = {
        let mut engine = Engine::new(book);
        keys.iter()
            .map(|&(s, r, c)| engine.root_cell(s, r, c))
            .collect::<Vec<_>>()
    };
    for ((si, r, c), value) in keys.into_iter().zip(values) {
        if let Some(cell) = book.sheets.get_mut(si).and_then(|s| s.cell_mut(r, c)) {
            cell.value = value;
        }
    }
}
pub fn dependencies(input: &str) -> Vec<(Option<String>, Range)> {
    fn visit(e: &Expr, out: &mut Vec<(Option<String>, Range)>) {
        match e {
            Expr::Reference(s, r) => out.push((s.clone(), *r)),
            Expr::Unary(_, e) => visit(e, out),
            Expr::Binary(_, a, b) => {
                visit(a, out);
                visit(b, out);
            }
            Expr::Call(_, args) => {
                for e in args {
                    visit(e, out);
                }
            }
            _ => (),
        }
    }
    let mut result = Vec::new();
    if let Ok(expr) = parse(input) {
        visit(&expr, &mut result);
    }
    result
}
#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn budget_failures_are_not_cached_as_dependency_results() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_input(id, 0, 0, "=B1+1")?;
        book.set_input(id, 0, 1, "=2+3")?;
        let mut engine = Engine::new(&book);
        // A dependency reached near the end of a caller's budget must get a fresh try.
        engine.work = 999_998;
        assert_eq!(engine.cell(0, 0, 0), Value::Error("#NUM!".into()));
        assert!(!engine.cache.contains_key(&(0, 0, 0)));
        assert!(!engine.cache.contains_key(&(0, 0, 1)));
        assert_eq!(engine.root_cell(0, 0, 1), Value::Number(5.0));
        assert_eq!(engine.root_cell(0, 0, 0), Value::Number(6.0));
        Ok(())
    }
}

pub fn explain(book: &Book, sheet: u64, row: u32, col: u32) -> Result<serde_json::Value, String> {
    use serde_json::json;
    if row >= MAX_ROWS || col >= MAX_COLS {
        return Err("invalid_range".into());
    }
    let source = book.sheet_by_id(sheet).ok_or("Лист не найден")?;
    let cell = source.cell(row, col);
    let input = cell.map(|cell| cell.input.as_str()).unwrap_or("");
    let value = cell.map(|cell| &cell.value).unwrap_or(&Value::Empty);
    let kind = match value {
        Value::Empty => "empty", Value::Number(_) => "number", Value::Text(_) => "text",
        Value::Bool(_) => "boolean", Value::Error(_) => "error",
    };
    struct Walk<'a> {
        book: &'a Book, sheet: u64, dependencies: Vec<serde_json::Value>,
        names: HashSet<String>, omitted: usize,
    }
    impl Walk<'_> {
        fn push(&mut self, value: serde_json::Value) {
            if self.dependencies.len() < 64 { self.dependencies.push(value); }
            else { self.omitted += 1; }
        }
        fn visit(&mut self, expr: &Expr, depth: usize) {
            if depth >= 32 || self.dependencies.len() >= 64 {
                self.omitted += 1;
                return;
            }
            match expr {
                Expr::Reference(name, range) => {
                    let target = match name {
                        Some(name) => self.book.sheets.iter().find(|s| s.name.to_lowercase() == name.to_lowercase()),
                        None => self.book.sheet_by_id(self.sheet),
                    };
                    if let Some(target) = target {
                        let mut sample = Vec::new();
                        for r in range.r0..=range.r1 {
                            for c in range.c0..=range.c1 {
                                if sample.len() == 4 { break; }
                                sample.push(json!({
                                    "address":address(r,c),
                                    "value_type":match target.cell(r,c).map(|cell| &cell.value) {
                                        None | Some(Value::Empty) => "empty",
                                        Some(Value::Number(_)) => "number",
                                        Some(Value::Text(_)) => "text",
                                        Some(Value::Bool(_)) => "boolean",
                                        Some(Value::Error(_)) => "error",
                                    },
                                    "input":target.cell(r,c).map(|v| v.input.chars().take(80).collect::<String>()),
                                    "display":target.display(r,c,self.book.locale_ru).chars().take(120).collect::<String>()
                                }));
                            }
                            if sample.len() == 4 { break; }
                        }
                        self.push(json!({
                            "kind":"range","sheet_id":target.id,"sheet":target.name,
                            "range":format!("{}:{}",address(range.r0,range.c0),address(range.r1,range.c1)),
                            "cells":(u64::from(range.r1-range.r0)+1)*(u64::from(range.c1-range.c0)+1),
                            "hidden":target.hidden,"sample":sample
                        }));
                    } else { self.push(json!({"kind":"range","sheet":name,"error":"#REF!"})); }
                }
                Expr::Name(name) => {
                    if !self.names.insert(name.to_lowercase()) {
                        self.push(json!({"kind":"name","name":name,"status":"already_visited"}));
                        return;
                    }
                    let definition = self.book.names.iter()
                        .find(|(n,_)| n.to_lowercase() == name.to_lowercase())
                        .map(|(_,input)| input.clone());
                    if let Some(input) = definition {
                        self.push(json!({"kind":"name","name":name,"input":input.chars().take(256).collect::<String>()}));
                        match parse(&input) {
                            Ok(expr) => self.visit(&expr, depth+1),
                            Err(_) => self.push(json!({"kind":"name","name":name,"error":"invalid_formula"})),
                        }
                    } else { self.push(json!({"kind":"name","name":name,"error":"#NAME?"})); }
                }
                Expr::Unary(_, expr) => self.visit(expr, depth+1),
                Expr::Binary(_, a, b) => { self.visit(a, depth+1); self.visit(b, depth+1); }
                Expr::Call(_, args) => { for expr in args { self.visit(expr, depth+1); } }
                Expr::Literal(_) => {}
            }
        }
    }
    let mut walk = Walk { book, sheet, dependencies: Vec::new(), names: HashSet::new(), omitted: 0 };
    let syntax_error = if input.starts_with('=') {
        match parse(input) {
            Ok(expr) => { walk.visit(&expr, 0); None }
            Err(error) => Some(error),
        }
    } else { None };
    Ok(json!({
        "sheet_id":sheet,"address":address(row,col),"input":input.chars().take(4096).collect::<String>(),
        "input_truncated":input.chars().count()>4096,"value_type":kind,
        "display":source.display(row,col,book.locale_ru).chars().take(512).collect::<String>(),
        "formula":input.starts_with('='),"syntax_error":syntax_error,
        "dependencies":walk.dependencies,"omitted":walk.omitted,"truncated":walk.omitted>0,
        "scope":"Direct references and named expressions; cached values, no evaluation or mutation."
    }))
}
#[cfg(test)]
mod passport_tests {
    use super::*;
    #[test]
    fn passport_is_read_only_and_resolves_named_dependencies() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "7")?;
        book.names.insert("Tax".into(), "=A1".into());
        book.set_cell(id, 0, 1, "=Tax+1")?;
        let before = book.clone();
        let report = explain(&book, id, 0, 1)?;
        assert_eq!(report["value_type"], "number");
        assert_eq!(report["dependencies"][0]["name"], "Tax");
        assert_eq!(report["dependencies"][1]["range"], "A1:A1");
        assert_eq!(report["dependencies"][1]["sample"][0]["value_type"], "number");
        assert_eq!(book, before);
        assert!(explain(&book, id, MAX_ROWS, 0).is_err());
        Ok(())
    }
    #[test]
    fn passport_bounds_named_cycles_and_reports_missing_sheets() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.names.insert("Loop".into(), "=Loop".into());
        book.set_input(id, 0, 0, "=Loop+'Missing'!A1")?;
        let report = explain(&book, id, 0, 0)?;
        assert_eq!(report["dependencies"][1]["status"], "already_visited");
        assert_eq!(report["dependencies"][2]["error"], "#REF!");
        assert!(report.to_string().len() < 4096);
        Ok(())
    }
}

fn lookup_result(grid: &Grid, index: usize, vertical: bool) -> CalcResult<Grid> {
    let (rows, cols) = if vertical { (1,grid.cols) } else { (grid.rows,1) };
    let mut values = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            values.push(if vertical { grid.at(index,col)? } else { grid.at(row,index)? });
        }
    }
    Ok(Grid { values, rows, cols, reference:grid.reference,
        origin:grid.origin.map(|(sheet,row,col)|(sheet,row+if vertical {index as u32}else{0},col+if vertical {0}else{index as u32})) })
}
fn binary_lookup(values: &[Value], needle: &Value, mode: i32, search: i32) -> Option<usize> {
    let (mut low, mut high) = (0,values.len());
    while low < high {
        let middle = low+(high-low)/2;
        let order = compare(&values[middle],needle);
        let order = if search < 0 { order.reverse() } else { order };
        if order.is_eq() { return Some(middle); }
        if order.is_lt() { low=middle+1; } else { high=middle; }
    }
    let index = match (mode, search < 0) {
        (-1,false) | (1,true) => low.checked_sub(1)?,
        (1,false) | (-1,true) => low,
        _ => return None,
    };
    values.get(index).filter(|value| lookup_kind(value,needle)).map(|_| index)
}

#[cfg(test)]
mod defect_batch_one {
    use super::*;
    fn book() -> Result<Book,String> {
        let mut book = Book::default();
        let id=book.active_id();
        for row in 0..3 {
            book.set_input(id,row,0,&(row+1).to_string())?;
            book.set_input(id,row,1,&((row+1)*10).to_string())?;
            book.set_input(id,row,2,&((row+1)*100).to_string())?;
        }
        Ok(book)
    }
    #[test]
    fn index_arrays_and_reference_identity() -> Result<(),String> {
        let mut book=book()?;
        let id=book.active_id();
        assert_eq!(evaluate(&book,id,"=SUM(INDEX(A1:B2,0,2))"),Value::Number(30.0));
        assert_eq!(evaluate(&book,id,"=SUM(INDEX(A1:B2,2))"),Value::Number(22.0));
        assert_eq!(evaluate(&book,id,"=SUM(INDEX(A1:B2,2,0))"),Value::Number(22.0));
        book.set_input(id,0,0,"'3")?;
        assert_eq!(evaluate(&book,id,"=SUM(INDEX(A1:A2,1))"),Value::Number(0.0));
        Ok(())
    }
    #[test]
    fn lookup_index_error_and_array_return() -> Result<(),String> {
        let book=book()?;
        let id=book.active_id();
        assert_eq!(evaluate(&book,id,"=VLOOKUP(1,A1:B2,0,FALSE)"),Value::Error("#VALUE!".into()));
        assert_eq!(evaluate(&book,id,"=HLOOKUP(1,A1:B2,0,FALSE)"),Value::Error("#VALUE!".into()));
        assert_eq!(evaluate(&book,id,"=SUM(XLOOKUP(2,A1:A2,B1:C2))"),Value::Number(220.0));
        assert_eq!(evaluate(&book,id,"=SUM(XLOOKUP(10,A1:C1,A2:C3))"),Value::Number(50.0));
        Ok(())
    }
    #[test]
    fn type_arrays_and_empty_repetition() -> Result<(),String> {
        let book=book()?;
        let id=book.active_id();
        assert_eq!(evaluate(&book,id,"=TYPE(A1:B2)"),Value::Number(64.0));
        assert_eq!(evaluate(&book,id,"=REPT(\"\",40000)"),Value::Text(String::new()));
        Ok(())
    }
    #[test]
    fn empty_delimiters_match_immediately() {
        let book=Book::default();
        let id=book.active_id();
        assert_eq!(evaluate(&book,id,"=TEXTBEFORE(\"abc\",\"\",2)"),Value::Text(String::new()));
        assert_eq!(evaluate(&book,id,"=TEXTAFTER(\"abc\",\"\",-2)"),Value::Text(String::new()));
    }
    #[test]
    fn minifs_treats_matched_blanks_as_zero() -> Result<(),String> {
        let mut book=Book::default();
        let id=book.active_id();
        book.set_input(id,1,0,"10")?;
        book.set_input(id,0,1,"1")?;
        book.set_input(id,1,1,"1")?;
        assert_eq!(evaluate(&book,id,"=MINIFS(A1:A2,B1:B2,1)"),Value::Number(0.0));
        book.set_input(id,1,0,"-10")?;
        assert_eq!(evaluate(&book,id,"=MAXIFS(A1:A2,B1:B2,1)"),Value::Number(0.0));
        Ok(())
    }
    #[test]
    fn binary_lookup_modes() -> Result<(),String> {
        let mut book=Book::default();
        let id=book.active_id();
        for (row,value) in [1,3,5].into_iter().enumerate() {
            book.set_input(id,row as u32,0,&value.to_string())?;
            book.set_input(id,row as u32,1,&(value*10).to_string())?;
        }
        for (row,value) in [5,3,1].into_iter().enumerate() {
            book.set_input(id,row as u32,2,&value.to_string())?;
        }
        assert_eq!(evaluate(&book,id,"=XMATCH(3,A1:A3,0,2)"),Value::Number(2.0));
        assert_eq!(evaluate(&book,id,"=XLOOKUP(4,A1:A3,B1:B3,,-1,2)"),Value::Number(30.0));
        assert_eq!(evaluate(&book,id,"=XMATCH(4,C1:C3,-1,-2)"),Value::Number(2.0));
        assert_eq!(evaluate(&book,id,"=XMATCH(1,C1:C3,0,-2)"),Value::Number(3.0));
        Ok(())
    }
}

fn scaled_product(coefficient: f64, first: f64, second: f64) -> f64 {
    if coefficient == 0.0 { return 0.0; }
    let product=first*second;
    if product.is_finite() && product != 0.0 { coefficient*product }
    else if first.abs() >= second.abs() { (coefficient*first)*second }
    else { (coefficient*second)*first }
}
#[cfg(test)]
mod defect_batch_two {
    use super::*;
    #[test]
    fn optional_blank_references_and_numeric_value_text() {
        let book=Book::default();
        let id=book.active_id();
        assert_eq!(evaluate(&book,id,"=LOG(100,A1)"),Value::Error("#NUM!".into()));
        assert_eq!(evaluate(&book,id,"=LOG(100)"),Value::Number(2.0));
        assert_eq!(evaluate(&book,id,"=VALUE(\"1.2\")"),Value::Number(1.2));
    }
    #[test]
    fn clock_fraction_and_last_day() {
        let book=Book::default();
        let id=book.active_id();
        for (input,expected) in [
            ("=SECOND(59.9/86400)",59.0),("=MINUTE(59.9/86400)",0.0),
            ("=HOUR(86399.9/86400)",23.0),("=YEAR(2958465.999999)",9999.0),
            ("=EDATE(2958465.5,0)",2958465.0)
        ] { assert_eq!(evaluate(&book,id,input),Value::Number(expected),"{input}"); }
    }
    fn number(book: &Book,input: &str) -> Result<f64,String> {
        match evaluate(book,book.active_id(),input) {
            Value::Number(value) => Ok(value),
            value => Err(format!("{input}: {value:?}")),
        }
    }
    #[test]
    fn large_statistics_remain_finite() -> Result<(),String> {
        let mut book=Book::default();
        let id=book.active_id();
        for (row,value) in [1.0,2.0,4.0,8.0].into_iter().enumerate() {
            book.set_input(id,row as u32,0,&value.to_string())?;
            book.set_input(id,row as u32,1,&(value*1e200).to_string())?;
        }
        for function in ["SKEW","KURT"] {
            let normal=number(&book,&format!("={function}(A1:A4)"))?;
            let large=number(&book,&format!("={function}(B1:B4)"))?;
            assert!((normal-large).abs()<1e-12);
        }
        book.set_input(id,0,2,"1e308")?;
        book.set_input(id,1,2,"1e308")?;
        assert_eq!(number(&book,"=TRIMMEAN(C1:C2,0)")?,1e308);
        Ok(())
    }
    #[test]
    fn normalized_regression_statistics() -> Result<(),String> {
        let mut book=Book::default();
        let id=book.active_id();
        for (row,value) in [1e200,2e200,3e200].into_iter().enumerate() {
            book.set_input(id,row as u32,0,&value.to_string())?;
            book.set_input(id,row as u32,1,&value.to_string())?;
        }
        assert!((number(&book,"=SLOPE(A1:A3,B1:B3)")?-1.0).abs()<1e-12);
        assert_eq!(number(&book,"=INTERCEPT(A1:A3,B1:B3)")?,0.0);
        for (row,value) in [1e154,2e154,3e154].into_iter().enumerate() {
            book.set_input(id,row as u32,0,&value.to_string())?;
        }
        let covariance=number(&book,"=COVARIANCE.P(A1:A3,A1:A3)")?;
        assert!((covariance/1e307-6.666666666666667).abs()<1e-12);
        Ok(())
    }
    #[test]
    fn mixed_date_time_text_preserves_each_contract() -> Result<(),String> {
        let book=Book::default();
        let id=book.active_id();
        let date=date_serial(2024,1,15);
        assert_eq!(number(&book,"=DATEVALUE(\"2024-01-15 12:30\")")?,date);
        assert!((number(&book,"=TIMEVALUE(\"2024-01-15 12:30\")")?-12.5/24.0).abs()<1e-12);
        assert!((number(&book,"=VALUE(\"2024-01-15 12:30\")")?-(date+12.5/24.0)).abs()<1e-10);
        Ok(())
    }
    #[test]
    fn named_months_and_short_years() -> Result<(),String> {
        let mut book=Book::default();
        book.locale_ru=false;
        assert_eq!(number(&book,"=DAYS(\"15-MAR-2021\",\"1-FEB-2021\")")?,42.0);
        assert_eq!(number(&book,"=DATEVALUE(\"1/15/24\")")?,date_serial(2024,1,15));
        assert_eq!(number(&book,"=DATEVALUE(\"1/15/30\")")?,date_serial(1930,1,15));
        book.locale_ru=true;
        assert_eq!(number(&book,"=DATEVALUE(\"15 января 2024\")")?,date_serial(2024,1,15));
        Ok(())
    }
    #[test]
    fn date_without_year_uses_current_year() -> Result<(),String> {
        let book=Book::default();
        let expected=date_serial(i64::from(chrono::Local::now().year()),3,15);
        assert_eq!(number(&book,"=DATEVALUE(\"15-MAR\")")?,expected);
        Ok(())
    }
    #[test]
    fn timevalue_keeps_fractional_seconds() -> Result<(),String> {
        let book=Book::default();
        assert!((number(&book,"=TIMEVALUE(\"12:34:56.5\")")?-45296.5/86400.0).abs()<1e-12);
        assert_eq!(evaluate(&book,book.active_id(),"=TIMEVALUE(\"12:34:NaN\")"),Value::Error("#VALUE!".into()));
        Ok(())
    }
}

#[cfg(test)]
mod additional_calc_defect_tests {
    use super::*;
    #[test]
    fn extreme_rounding_precision_does_not_reject_finite_answers() {
        let book=Book::default();let id=book.active_id();
        for input in ["=ROUND(123,400)","=ROUNDUP(123,400)","=ROUNDDOWN(123,400)","=TRUNC(123,400)"] {
            assert_eq!(evaluate(&book,id,input),Value::Number(123.0),"{input}");
        }
        for input in ["=ROUND(123,-400)","=ROUNDDOWN(123,-400)","=TRUNC(123,-400)","=ROUNDUP(0,-400)"] {
            assert_eq!(evaluate(&book,id,input),Value::Number(0.0),"{input}");
        }
    }
    #[test]
    fn escaped_wildcards_do_not_fall_back_to_matching_a_literal_tilde() {
        assert!(!wildcard(&"~*".chars().collect::<Vec<_>>(),&"~abc".chars().collect::<Vec<_>>()));
        assert!(!wildcard(&"~?".chars().collect::<Vec<_>>(),&"~a".chars().collect::<Vec<_>>()));
        assert!(wildcard(&"~*".chars().collect::<Vec<_>>(),&['*']));
        assert!(wildcard_prefix("~","~tail"));
        assert!(wildcard_prefix("~~","~tail"));
        assert!(!wildcard_prefix("~*","~tail"));
    }
    #[test]
    fn empty_cells_do_not_accumulate_in_the_calculation_cache() {
        let book=Book::default();let mut engine=Engine::new(&book);
        for row in 0..10000 {assert_eq!(engine.root_cell(0,row,0),Value::Empty);}
        assert!(engine.cache.is_empty());
    }
    #[test]
    fn independent_text_results_share_a_workbook_cache_budget() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_input(id,0,0,"=REPT(\"x\",8)")?;
        book.set_input(id,1,0,"=REPT(\"y\",8)")?;
        let mut engine=Engine::new(&book);engine.cache_text_limit=12;
        assert_eq!(engine.root_cell(0,0,0),Value::Text("xxxxxxxx".into()));
        assert_eq!(engine.root_cell(0,1,0),Value::Error("#NUM!".into()));
        assert_eq!(engine.cached_text,8);
        assert_eq!(engine.root_cell(0,0,0),Value::Text("xxxxxxxx".into()));
        Ok(())
    }
}

/// Sum in a shared scale so a finite cancellation does not pass through infinity.
fn scaled_sum(values:&[f64])->f64 {
    if values.iter().any(|value|!value.is_finite()) {return values.iter().sum();}
    let scale=values.iter().map(|value|value.abs()).fold(0.0,f64::max);
    if scale==0.0{return 0.0;}
    let (mut sum,mut correction)=(0.0,0.0);
    for value in values {
        let value=value/scale;
        let next=sum+value;
        correction+=if sum.abs()>=value.abs() {(sum-next)+value}else{(value-next)+sum};
        sum=next;
    }
    (sum+correction)*scale
}
fn balanced_product(values:&[f64])->f64 {
    if values.iter().any(|value|*value==0.0) {return 0.0;}
    let (mut mantissa,mut exponent,mut negative)=(1.0,0i64,false);
    for value in values {
        negative^=value.is_sign_negative();
        let mut magnitude=value.abs();
        let mut adjustment=0i64;
        if magnitude<f64::MIN_POSITIVE {magnitude*=4503599627370496.0;adjustment=-52;}
        let bits=magnitude.to_bits();
        exponent+=i64::from(((bits>>52)&0x7ff) as u16)-1023+adjustment;
        mantissa*=f64::from_bits((bits&0x000f_ffff_ffff_ffff)|(1023u64<<52));
        if mantissa>=2.0 {mantissa*=0.5;exponent+=1;}
    }
    let value=if exponent>1023 {f64::INFINITY}
        else if exponent< -1075 {0.0}
        else if exponent== -1075 {mantissa*0.5*f64::from_bits(1)}
        else if exponent< -1022 {mantissa*f64::MIN_POSITIVE*f64::from_bits(((exponent+1022+1023) as u64)<<52)}
        else {mantissa*f64::from_bits(((exponent+1023) as u64)<<52)};
    if negative {-value}else{value}
}
#[cfg(test)]
mod next_arithmetic_defects {
    use super::*;
    fn number(book:&Book,input:&str)->Result<f64,String> {
        match evaluate(book,book.active_id(),input) {Value::Number(value)=>Ok(value),value=>Err(format!("{input}: {value:?}"))}
    }
    #[test]
    fn modulo_does_not_overflow_a_discarded_quotient()->Result<(),String> {
        let book=Book::default();
        assert_eq!(number(&book,"=MOD(1e308,1)")?,0.0);
        assert_eq!(number(&book,"=MOD(-3,2)")?,1.0);
        assert_eq!(number(&book,"=MOD(3,-2)")?,-1.0);Ok(())
    }
    #[test]
    fn rounding_to_tiny_multiples_keeps_already_integral_large_values()->Result<(),String> {
        let book=Book::default();
        for function in ["MROUND","CEILING","FLOOR","CEILING.MATH","FLOOR.PRECISE"] {
            assert_eq!(number(&book,&format!("={function}(1e308,1e-308)"))?,1e308);
        }Ok(())
    }
    #[test]
    fn products_balance_large_and_small_factors_and_zero()->Result<(),String> {
        let book=Book::default();
        assert!((number(&book,"=PRODUCT(1e200,1e200,1e-200,1e-200)")?-1.0).abs()<1e-12);
        assert_eq!(number(&book,"=SUMPRODUCT(1e308,1e308,0)")?,0.0);Ok(())
    }
    #[test]
    fn aggregate_cancellation_does_not_overflow_a_finite_result()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        for (row,value) in ["1e308","1e308","-1e308"].iter().enumerate() {book.set_input(id,row as u32,0,value)?;}
        assert_eq!(number(&book,"=SUM(A1:A3)")?,1e308);
        assert_eq!(number(&book,"=SUMIF(A1:A3,\"<>0\")")?,1e308);
        assert!((number(&book,"=AVERAGEIF(A1:A3,\"<>0\")")?/1e308-1.0/3.0).abs()<1e-12);Ok(())
    }
    #[test]
    fn npv_keeps_a_finite_late_discounted_cash_flow()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        for row in 0..1023 {book.set_input(id,row,0,"0")?;}
        book.set_input(id,1023,0,"1e308")?;
        let value=number(&book,"=NPV(1,A1:A1024)")?;
        assert!(value>0.55 && value<0.56);Ok(())
    }
    #[test]
    fn annuity_payments_and_present_values_use_discounted_growth()->Result<(),String> {
        let book=Book::default();
        assert!((number(&book,"=PMT(0.1,10000,1)")?+0.1).abs()<1e-12);
        assert!((number(&book,"=PV(0.1,10000,-1)")?-10.0).abs()<1e-12);Ok(())
    }
}

fn exponential_scale(value:f64,logarithm:f64)->f64 {
    if value==0.0 {return 0.0;}
    value.signum()*(value.abs().ln()+logarithm).exp()
}

fn roman_form(mut value:u32,form:usize)->String {
    if form==0 {
        let mut out=String::new();
        for (amount,symbol) in ROMAN {while value>=amount {out.push_str(symbol);value-=amount;}}
        return out;
    }
    let symbols=[("I",1u32),("V",5),("X",10),("L",50),("C",100),("D",500),("M",1000)];
    let mut choices=symbols.iter().map(|(symbol,value)|(*value,symbol.to_string())).collect::<Vec<_>>();
    for high in 1..symbols.len() {
        let classic=high-if high%2==0 {2}else{1};
        for low in classic.saturating_sub(form)..=classic {
            choices.push((symbols[high].1-symbols[low].1,format!("{}{}",symbols[low].0,symbols[high].0)));
        }
    }
    choices.sort_by(|a,b|b.0.cmp(&a.0).then_with(||a.1.len().cmp(&b.1.len())));
    let mut out=String::new();
    for (amount,symbol) in choices {while value>=amount {out.push_str(&symbol);value-=amount;}}
    out
}
#[cfg(test)]
mod next_reference_roman_tests {
    use super::*;
    #[test]
    fn computed_references_keep_their_source_coordinates_and_formula_identity()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_input(id,4,2,"=1+1")?;
        for (formula,expected) in [("=ROW(INDEX(B4:D8,2,2))",Value::Number(5.0)),
            ("=COLUMN(INDEX(B4:D8,2,2))",Value::Number(3.0)),
            ("=ISFORMULA(INDEX(B4:D8,2,2))",Value::Bool(true)),
            ("=ROW(CHOOSE(2,A1,C5))",Value::Number(5.0))] {
            assert_eq!(evaluate(&book,id,formula),expected,"{formula}");
        }Ok(())
    }
    #[test]
    fn roman_accepts_all_documented_forms_and_boolean_aliases() {
        let book=Book::default();let id=book.active_id();
        for (form,expected) in [("0","CDXCIX"),("1","LDVLIV"),("2","XDIX"),("3","VDIV"),("4","ID"),("TRUE","CDXCIX"),("FALSE","ID")] {
            assert_eq!(evaluate(&book,id,&format!("=ROMAN(499,{form})")),Value::Text(expected.into()));
        }
    }
}

fn literal_search(source:&[char],needle:&str)->Option<usize> {
    let mut folded=String::new();let mut offsets=Vec::new();
    for (index,ch) in source.iter().enumerate() {
        offsets.push((folded.len(),index));folded.extend(ch.to_lowercase());
    }
    let pattern=needle.chars().flat_map(char::to_lowercase).collect::<String>();
    let found=folded.find(&pattern)?;
    if offsets.is_empty(){return Some(0);}
    let slot=offsets.partition_point(|(offset,_)|*offset<=found).saturating_sub(1);
    offsets.get(slot).map(|(_,index)|*index)
}
#[cfg(test)]
mod next_search_formula_tests {
    use super::*;
    #[test]
    fn empty_find_text_does_not_allow_a_start_after_the_haystack() {
        let book=Book::default();let id=book.active_id();
        for input in ["=FIND(\"\",\"abc\",4)","=SEARCH(\"\",\"abc\",4)"] {
            assert_eq!(evaluate(&book,id,input),Value::Error("#VALUE!".into()));
        }
        assert_eq!(evaluate(&book,id,"=FIND(\"\",\"abc\",3)"),Value::Number(3.0));
    }
    #[test]
    fn literal_search_handles_long_misses_and_keeps_original_unicode_positions() {
        let source="a".repeat(32767).chars().collect::<Vec<_>>();
        assert_eq!(literal_search(&source,"missing"),None);
        assert_eq!(literal_search(&"😀Привет".chars().collect::<Vec<_>>(),"ВЕТ"),Some(4));
    }
}

#[cfg(test)]
mod hundred_numeric_tests {
    use super::*;
    #[test]
    fn numbervalue_ignores_spaces_between_percent_marks() {
        let book = Book::default();
        let Value::Number(value) = evaluate(&book, book.active_id(), "=NUMBERVALUE(\"9% %\")") else { panic!("number"); };
        assert!((value - 0.0009).abs() < 1e-15);
    }
    #[test]
    fn numbervalue_applies_many_percent_marks_without_an_infinite_divisor() {
        let book = Book::default();
        let input = format!("=NUMBERVALUE(\"1e308{}\")", "%".repeat(200));
        let Value::Number(value) = evaluate(&book, book.active_id(), &input) else { panic!("number"); };
        assert!((value / 1e-92 - 1.0).abs() < 1e-12);
    }
    #[test]
    fn nper_is_invariant_under_large_cash_flow_scaling() {
        let book = Book::default();
        let small = evaluate(&book, book.active_id(), "=NPER(0.1,-1,1,1)");
        let large = evaluate(&book, book.active_id(), "=NPER(0.1,-1e308,1e308,1e308)");
        let (Value::Number(small), Value::Number(large)) = (small, large) else { panic!("numbers"); };
        assert!((small - large).abs() < 1e-12);
    }
    #[test]
    fn random_between_opposite_large_bounds_stays_finite() {
        let book = Book::default();
        for _ in 0..16 {
            let Value::Number(value) = evaluate(&book, book.active_id(), "=RANDBETWEEN(-1e308,1e308)") else { panic!("number"); };
            assert!((-1e308..=1e308).contains(&value));
        }
    }
}

#[cfg(test)]
mod hundred_parser_reference_tests {
    use super::*;
    #[test]
    fn left_associative_and_postfix_trees_are_bounded_before_drop() {
        assert!(parse(&format!("={}1", "1+".repeat(8000))).is_err());
        assert!(parse(&format!("=1{}", "%".repeat(8000))).is_err());
        assert!(parse("=SUM(1,2,3)+4").is_ok());
    }
    #[test]
    fn rows_and_columns_do_not_materialize_direct_references() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_input(id, 0, 0, "=1/0")?;
        let mut engine = Engine::new(&book);
        engine.materialized = 64 * 1024 * 1024;
        assert_eq!(engine.eval(&parse("=ROWS(A1:B2)")?, 0, 0, 0)?.first(), Value::Number(2.0));
        assert_eq!(engine.eval(&parse("=COLUMNS(A1:D2)")?, 0, 0, 0)?.first(), Value::Number(4.0));
        assert!(engine.cache.is_empty());
        assert_eq!(evaluate(&book, id, "=ROWS('missing'!A1:B2)"), Value::Error("#REF!".into()));
        assert_eq!(evaluate(&book, id, "=COLUMNS('missing'!A1:B2)"), Value::Error("#REF!".into()));
        Ok(())
    }
}

fn wildcard_search(pattern: &str, text: &[char], mut budget: usize) -> CalcResult<Option<usize>> {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let mut folded = Vec::new();
    let mut starts = Vec::new();
    for (index, ch) in text.iter().enumerate() {
        starts.push((folded.len(), index));
        folded.extend(ch.to_lowercase());
    }
    starts.push((folded.len(), text.len()));
    for (start, original) in starts {
        let source = &folded[start..];
        let (mut pi, mut ti, mut star, mut mark) = (0usize, 0usize, None, 0usize);
        loop {
            if budget == 0 { return Err("#NUM!".into()); }
            budget -= 1;
            if pi == pattern.len() { return Ok(Some(original)); }
            if pattern.get(pi) == Some(&'*') {
                star = Some(pi); pi += 1; mark = ti; continue;
            }
            if ti == source.len() { break; }
            let escaped = pattern.get(pi) == Some(&'~')
                && pattern.get(pi+1).is_some_and(|ch| matches!(ch, '*' | '?' | '~'));
            if escaped && pattern.get(pi+1) == source.get(ti) {
                pi += 2; ti += 1;
            } else if !escaped && (pattern.get(pi) == Some(&'?') || pattern.get(pi) == source.get(ti)) {
                pi += 1; ti += 1;
            } else if let Some(index) = star {
                mark += 1;
                if mark > source.len() { break; }
                ti = mark; pi = index + 1;
            } else { break; }
        }
    }
    Ok(None)
}
#[cfg(test)]
mod hundred_range_and_wildcard_tests {
    use super::*;
    #[test]
    fn wildcard_search_reuses_one_folded_buffer_and_bounds_backtracking() {
        let book = Book::default();
        let query = format!("=SEARCH(\"?z\",\"{}\")", "a".repeat(32000));
        assert_eq!(evaluate(&book, book.active_id(), &query), Value::Error("#VALUE!".into()));
        assert_eq!(wildcard_search("*a*a*a*z", &"a".repeat(100).chars().collect::<Vec<_>>(), 10), Err("#NUM!".into()));
        assert_eq!(wildcard_search("~?", &"a?b".chars().collect::<Vec<_>>(), 100), Ok(Some(1)));
    }
    #[test]
    fn computed_reference_consumers_keep_projection_and_subtotal_visibility() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "1")?; book.set_cell(id, 1, 0, "1")?;
        book.set_cell(id, 0, 1, "10")?; book.set_cell(id, 1, 1, "20")?;
        assert_eq!(evaluate(&book, id, "=SUMIF(A1:A2,1,INDEX(B1:B2,1))"), Value::Number(30.0));
        book.active_sheet_mut().ok_or("sheet")?.hidden_rows.insert(1);
        assert_eq!(evaluate(&book, id, "=SUBTOTAL(109,INDEX(B1:B2,0))"), Value::Number(10.0));
        Ok(())
    }
}
