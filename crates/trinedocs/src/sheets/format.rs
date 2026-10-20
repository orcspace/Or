use super::formula::serial_date;
use super::model::Value;

pub const NUMBER_FORMATS: &[(&str, &str)] = &[
    ("Общий", "General"),
    ("Число", "0.00"),
    ("Целое", "0"),
    ("Денежный ₽", "#,##0.00 \"₽\""),
    ("Денежный $", "\"$\"#,##0.00"),
    ("Денежный €", "#,##0.00 \"€\""),
    ("Процент", "0.00%"),
    ("Дата", "dd.mm.yyyy"),
    ("Дата ISO", "yyyy-mm-dd"),
    ("Время", "hh:mm:ss"),
    ("Дробь", "# ?/?"),
    ("Экспонента", "0.00E+00"),
    ("Текст", "@"),
];
pub fn display(value: &Value, code: &str, ru: bool) -> String {
    let Value::Number(number) = value else {
        if let Value::Text(text) = value {
            let sections = split_sections(code);
            let section = sections.get(3).or_else(|| {
                sections
                    .first()
                    .filter(|section| outside_quotes(section).contains('@'))
            });
            if let Some(section) = section {
                return text_section(section, text);
            }
        }
        return value.display();
    };
    if !number.is_finite() {
        return "#NUM!".into();
    }
    if code.eq_ignore_ascii_case("General") || code == "@" || code.is_empty() {
        return localize(&general_number(*number, 10), ru);
    }
    let sections = split_sections(code);
    let negative = *number < 0.0;
    let conditional = sections.iter().take(2).any(|section| condition(section,*number).is_some());
    let section = selected_numeric_section(&sections, *number);
    let Some(section) = section else {
        return String::new();
    };
    if section.is_empty() {
        return String::new();
    }
    let section = strip_brackets(section);
    let numeric = if negative && sections.len() > 1 && !conditional {
        number.abs()
    } else {
        *number
    };
    if section.eq_ignore_ascii_case("General") {
        return localize(&general_number(numeric, 10), ru);
    }
    let lower = section.to_ascii_lowercase();
    let bare = outside_quotes(&lower);
    if bare
        .chars()
        .any(|c| matches!(c, 'y' | 'm' | 'd' | 'h' | 's')) || bare.contains("a/p")
    {
        return date_display(numeric, &section,ru);
    }
    let percent = bare.chars().filter(|c| *c == '%').count();
    if bare.split_once('/').is_some_and(|(left, right)| {
        left.chars().last().is_some_and(|ch| matches!(ch, '0' | '#' | '?'))
            && right.chars().next().is_some_and(|ch| ch.is_ascii_digit() || matches!(ch, '#' | '?'))
    }) {
        let (left, right) = bare.split_once('/').unwrap_or(("", ""));
        let denominator = right
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
            .filter(|den| *den > 0 && *den <= 1_000_000);
        let mixed = left.split_whitespace().count() > 1;
        let sign = if numeric < 0.0 { "-" } else { "" };
        let n = decimal_scale(numeric.abs(), 2 * percent as i32);
        if !n.is_finite() {return "#NUM!".into();}
        let whole = if mixed { n.floor() } else { 0.0 };
        let fraction = n - whole;
        let max_den = 10u32.pow(
            right
                .chars()
                .take_while(|c| matches!(c, '?' | '#' | '0'))
                .count()
                .clamp(1, 3) as u32,
        ) - 1;
        let mut best = (0.0, 1u32, f64::INFINITY);
        let candidates = denominator.map_or(1..=max_den, |den| den..=den);
        for den in candidates {
            let numerator = round_half_away(fraction * f64::from(den), 0);
            if !numerator.is_finite() { continue; }
            let diff = (fraction - numerator / f64::from(den)).abs();
            if diff < best.2 {
                best = (numerator, den, diff);
            }
        }
        if !best.2.is_finite() { return "#NUM!".into(); }
        let body=if mixed && best.0==0.0 {format!("{sign}{whole:.0}")}
            else if mixed && best.0==f64::from(best.1) {format!("{sign}{:.0}",whole+1.0)}
            else {format!("{sign}{}{:.0}/{}",
                if mixed && whole!=0.0 {format!("{whole:.0} ")}else{String::new()},best.0,best.1)};
        let tokens=outside_tokens(&section);
        let start=tokens.iter().find(|(_,c)|matches!(c,'0'|'#'|'?')).map(|(i,_)|*i).unwrap_or(0);
        let slash=tokens.iter().find(|(_,c)|*c=='/').map(|(i,_)|*i).unwrap_or(0);
        let end=section[slash+1..].char_indices().take_while(|(_,c)|c.is_ascii_digit() || matches!(c,'?'|'#'))
            .last().map(|(i,c)|slash+1+i+c.len_utf8()).unwrap_or(slash+1);
        return format!("{}{}{}",unquote(&section[..start]),body,unquote(&section[end..]));
    }
    let mut n = numeric;
    let tokens = outside_tokens(&section);
    let first = tokens
        .iter()
        .copied()
        .find(|(_, c)| matches!(c, '0' | '#' | '?'))
        .map(|(i, _)| i);
    let last = tokens
        .iter()
        .copied()
        .rev()
        .find(|(_, c)| matches!(c, '0' | '#' | '?'))
        .map(|(i, c)| i + c.len_utf8());
    let (start, end) = match (first, last) {
        (Some(a), Some(b)) => (a, b),
        _ => return unquote(&section),
    };
    let mut pattern = section.get(start..end).unwrap_or("0").to_string();
    let tail = section.get(end..).unwrap_or("");
    let decimal=outside_tokens(&pattern).into_iter().find(|(_,c)|*c=='.').map(|(i,_)|i);
    let inner_scale=decimal.map(|i|pattern[..i].chars().rev().take_while(|c|*c==',').count()).unwrap_or(0);
    if let Some(decimal)=decimal && inner_scale>0 {
        pattern.replace_range(decimal-inner_scale..decimal,"");
    }
    let scale = tail.chars().take_while(|c| *c == ',').count()+inner_scale;
    n = decimal_scale(n, 2 * percent as i32 - 3 * scale as i32);
    if !n.is_finite() {
        return "#NUM!".into();
    }
    let decimal_pattern: String = pattern
        .split('.')
        .nth(1)
        .unwrap_or("")
        .chars()
        .take_while(|c| matches!(c, '0' | '#' | '?'))
        .take(15)
        .collect();
    let decimals = decimal_pattern.len();
    let integer_pattern = pattern.split(['.', 'E', 'e']).next().unwrap_or("");
    let minimum_digits = integer_pattern.chars().filter(|c| *c == '0').count();
    let fixed = || numeric_pattern(n, &decimal_pattern, minimum_digits);
    let rendered = if let Some(masked)=integer_literal_mask(n,&pattern) {
        masked
    } else if let Some(exponent) = pattern.to_uppercase().find('E') {
        let digits = pattern
            .get(exponent..)
            .unwrap_or("")
            .chars()
            .filter(|c| *c == '0')
            .count()
            .max(1);
        localize(
            &scientific(
                n,
                decimals,
                digits,
                integer_pattern
                    .chars()
                    .filter(|c| matches!(c, '0' | '#' | '?'))
                    .count()
                    .clamp(1, 15),
                !pattern
                    .get(exponent + 1..)
                    .is_some_and(|tail| tail.starts_with('-')),
            ),
            ru,
        )
    } else if pattern.split('.').next().is_some_and(|s| s.contains(',')) {
        group(&fixed(), ru)
    } else {
        localize(&fixed(), ru)
    };
    format!(
        "{}{}{}",
        unquote(section.get(..start).unwrap_or("")),
        rendered,
        unquote(tail.trim_start_matches(','))
    )
}
/// Apply only the net decimal exponent; never form an infinite scale factor.
fn decimal_scale(mut value: f64, mut exponent: i32) -> f64 {
    while exponent >= 3 { value *= 1000.0; exponent -= 3; }
    while exponent <= -3 { value /= 1000.0; exponent += 3; }
    while exponent > 0 { value *= 10.0; exponent -= 1; }
    while exponent < 0 { value /= 10.0; exponent += 1; }
    value
}
/// Rounds half away from zero like Excel (Rust's formatter rounds ties to even, and binary
/// fractions such as 1.005 sit just below the tie).
fn round_half_away(n: f64, decimals: usize) -> f64 {
    let scale = 10f64.powi(decimals.min(15) as i32);
    let scaled = n * scale;
    if scaled.is_finite() {
        round_scaled(scaled) / scale
    } else {
        n
    }
}
/// Float noise allowed around `x`: a few ulps, capped so that at 15+ significant digits
/// (where one ulp is a sizeable fraction) real fractions are not mistaken for noise.
fn noise(x: f64) -> f64 {
    (8.0 * f64::EPSILON * x.abs().max(1.0)).min(1e-3)
}
/// Rounds to an integer, treating values within float noise of .5 as the tie.
pub fn round_scaled(scaled: f64) -> f64 {
    let tie = (scaled.fract().abs() - 0.5).abs() <= noise(scaled);
    if tie {
        scaled.trunc() + scaled.signum()
    } else {
        scaled.round()
    }
}
/// An integer when `x` is one up to float noise (0.3/0.1 = 2.9999999999999996 is 3).
pub fn snap(x: f64) -> f64 {
    if (x - x.round()).abs() <= noise(x) {
        x.round()
    } else {
        x
    }
}
fn fixed(n: f64, decimals: usize) -> String {
    format!("{:.decimals$}", round_half_away(n, decimals))
}
/// Scientific formatting preserves the requested exponent sign and half-away ties.
fn scientific(
    n: f64,
    decimals: usize,
    exponent_digits: usize,
    integer_digits: usize,
    positive_sign: bool,
) -> String {
    let raw = format!("{n:E}");
    let (mantissa, exponent) = raw.split_once('E').unwrap_or((&raw, "0"));
    let raw_exponent: i32 = exponent.parse().unwrap_or(0);
    let group = integer_digits as i32;
    let mut exponent = raw_exponent.div_euclid(group) * group;
    let mut mantissa = round_half_away(
        mantissa.parse::<f64>().unwrap_or(0.0) * 10f64.powi(raw_exponent - exponent),
        decimals,
    );
    if mantissa.abs() >= 10f64.powi(group) {
        mantissa /= 10f64.powi(group);
        exponent += group;
    }
    let sign = if exponent < 0 {
        "-"
    } else if positive_sign {
        "+"
    } else {
        ""
    };
    format!(
        "{mantissa:.decimals$}E{sign}{:0exponent_digits$}",
        exponent.unsigned_abs()
    )
}
/// Excel's General rendering with at most `digits` significant digits: 10 for a cell (what
/// fits a standard column), 15 when a number becomes text in a formula.
pub fn general_number(n: f64, digits: usize) -> String {
    if !n.is_finite() {
        return "#NUM!".into();
    }
    if n == 0.0 {
        return "0".into();
    }
    let exponent = n.abs().log10().floor() as i32;
    if exponent >= (digits as i32).max(11) || exponent < -9 {
        let mantissa = if digits <= 10 { 5 } else { digits - 1 };
        let text = scientific(n, mantissa, 2, 1, true);
        let (m, e) = text.split_once('E').unwrap_or((&text, ""));
        let m = if m.contains('.') {
            m.trim_end_matches('0').trim_end_matches('.')
        } else {
            m
        };
        return format!("{m}E{e}");
    }
    let decimals = (digits as i32 - 1 - exponent).clamp(0, 15) as usize;
    let text = fixed(n, decimals);
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}
fn localize(input: &str, ru: bool) -> String {
    if ru {
        input.replace('.', ",")
    } else {
        input.into()
    }
}
fn group(input: &str, ru: bool) -> String {
    let (whole, fraction) = input
        .split_once('.')
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((input, None));
    let sign = if whole.starts_with('-') { "-" } else { "" };
    let digits = whole.trim_start_matches('-');
    let mut out = String::from(sign);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(if ru { '\u{00a0}' } else { ',' });
        }
        out.push(c);
    }
    if let Some(frac) = fraction {
        out.push(if ru { ',' } else { '.' });
        out.push_str(frac);
    }
    out
}
fn split_sections(code: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quote = false;
    let mut escape = false;
    let mut skip = false;
    for c in code.chars() {
        if skip { if let Some(last)=out.last_mut() {last.push(c);} skip=false; continue; }
        if c == ';' && !quote && !escape {
            out.push(String::new());
            continue;
        }
        if let Some(last) = out.last_mut() {
            last.push(c);
        }
        if c == '"' && !escape {
            quote = !quote;
        }
        if !quote && !escape && matches!(c,'_'|'*') {skip=true;}
        escape = c == '\\' && !escape;
    }
    out
}
fn unquote(code: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    let mut quote = false;
    let mut escape = false;
    for c in code.chars() {
        if escape {
            out.push(c);
            escape = false;
            continue;
        }
        if skip {
            skip = false;
            continue;
        }
        if c == '"' {
            quote = !quote;
            continue;
        }
        if !quote && c == '\\' {
            escape = true;
            continue;
        }
        if !quote && (c == '_' || c == '*') {
            skip = true;
            continue;
        }
        out.push(c);
    }
    out
}
fn outside_quotes(code: &str) -> String {
    outside_tokens(code).into_iter().map(|(_, c)| c).collect()
}
fn outside_tokens(code: &str) -> Vec<(usize, char)> {
    let mut quote = false;
    let mut escape = false;
    let mut skip = false;
    let mut result = Vec::new();
    for (index, c) in code.char_indices() {
        if escape || skip {
            escape = false;
            skip = false;
            continue;
        }
        if c == '"' {
            quote = !quote;
        } else if !quote {
            if c == '\\' {
                escape = true;
            } else if c == '_' || c == '*' {
                skip = true;
            } else {
                result.push((index, c));
            }
        }
    }
    result
}
fn numeric_pattern(number: f64, decimals: &str, minimum_digits: usize) -> String {
    let mut text = fixed(number, decimals.len());
    let mandatory = decimals.rfind('0').map_or(0, |index| index + 1);
    if let Some(dot) = text.find('.') {
        let mut significant = text.len() - dot - 1;
        while significant > mandatory && text.as_bytes().get(dot + significant) == Some(&b'0') {
            significant -= 1;
        }
        let fraction = text[dot + 1..dot + 1 + significant].to_string();
        text.truncate(dot);
        let padding = decimals.chars().skip(significant).filter(|c| *c == '?').count();
        if significant > 0 || padding > 0 {
            text.push('.');
            text.push_str(&fraction);
            text.extend(std::iter::repeat_n(' ', padding));
        }
    }
    let sign = usize::from(text.starts_with('-'));
    let end = text.find('.').unwrap_or(text.len());
    let digits = end - sign;
    if digits < minimum_digits {
        text.insert_str(sign, &"0".repeat(minimum_digits - digits));
    } else if minimum_digits == 0 && text.get(sign..end) == Some("0") {
        text.remove(sign);
        if text == "-" { text.clear(); }
    }
    text
}
fn text_section(code: &str, value: &str) -> String {
    let mut result = String::new();
    let mut start = 0;
    for (index, token) in outside_tokens(code) {
        if token == '@' {
            result.push_str(&unquote(&code[start..index]));
            result.push_str(value);
            start = index + 1;
        }
    }
    result.push_str(&unquote(&code[start..]));
    result
}
fn selected_numeric_section(sections:&[String], number:f64)->Option<&String> {
    let conditional=sections.iter().take(2).any(|section|condition(section,number).is_some());
    if conditional {
        sections.iter().take(3).find(|section|condition(section,number).unwrap_or(true))
    } else if number==0.0 && sections.len()>2 {sections.get(2)}
    else if number<0.0 && sections.len()>1 {sections.get(1)}
    else {sections.first()}
}
/// Named number-format colors override the cell's font color only in the chosen section.
pub fn color(value:&Value,code:&str)->Option<[u8;3]> {
    let sections=split_sections(code);
    let section=match value {
        Value::Number(number)=>selected_numeric_section(&sections,*number)?,
        Value::Text(_)=>sections.get(3).or_else(||sections.first().filter(|section|outside_quotes(section).contains('@')))?,
        _=>return None,
    };
    let tokens=outside_quotes(section);
    for tail in tokens.split('[').skip(1) {
        let Some((name,_))=tail.split_once(']') else {continue;};
        let color=match name.to_ascii_lowercase().as_str() {
            "black"=>[0,0,0],"blue"=>[0,0,255],"cyan"=>[0,255,255],"green"=>[0,255,0],
            "magenta"=>[255,0,255],"red"=>[255,0,0],"white"=>[255,255,255],"yellow"=>[255,255,0],
            _=>continue,
        };
        return Some(color);
    }
    None
}
fn condition(code: &str, number: f64) -> Option<bool> {
    let tokens = outside_tokens(code);
    let text: String = tokens.into_iter().map(|(_, c)| c).collect();
    for tail in text.split('[').skip(1) {
        let (clause, _) = tail.split_once(']')?;
        for operator in ["<=", ">=", "<>", "=", "<", ">"] {
            if let Some(value) = clause.strip_prefix(operator)
                && let Ok(value) = value.parse::<f64>()
            {
                return Some(match operator {
                    "<=" => number <= value,
                    ">=" => number >= value,
                    "<>" => number != value,
                    "=" => number == value,
                    "<" => number < value,
                    _ => number > value,
                });
            }
        }
    }
    None
}
fn strip_brackets(code: &str) -> String {
    let mut out = String::new();
    let mut bracket = String::new();
    let mut inside = false;
    let mut quoted = false;
    let mut escaped = false;
    for c in code.chars() {
        if escaped {
            out.push(c);
            escaped = false;
            continue;
        }
        if !inside && c == '\\' {
            out.push(c);
            escaped = true;
            continue;
        }
        if !inside && c == '"' {
            quoted = !quoted;
        }
        if !quoted && c == '[' {
            inside = true;
            bracket.clear();
        } else if !quoted && c == ']' {
            inside = false;
            // `[$€-407]` is a currency symbol with a locale; `[$-F800]` has no symbol.
            if let Some(currency) = bracket.strip_prefix('$') {
                let symbol = currency.split('-').next().unwrap_or("");
                if !symbol.is_empty() {
                    out.push('"');
                    out.push_str(symbol);
                    out.push('"');
                }
            } else if matches!(
                bracket.to_ascii_lowercase().as_str(),
                "h" | "hh" | "m" | "mm" | "s" | "ss"
            ) {
                out.push('[');
                out.push_str(&bracket);
                out.push(']');
            }
        } else if inside {
            bracket.push(c);
        } else {
            out.push(c);
        }
    }
    out
}
fn date_display(serial: f64, code: &str, ru: bool) -> String {
    if !(0.0..2958466.0).contains(&serial) {
        return "#NUM!".into();
    }
    let date_tokens = outside_quotes(&code.to_ascii_lowercase());
    let fraction_digits=["s.","s]."].iter().filter_map(|pattern| {
        date_tokens.split_once(pattern).map(|(_,tail)| tail.chars().take_while(|c|*c=='0').count().min(9))
    }).max().unwrap_or(0);
    let scale=10f64.powi(fraction_digits as i32);
    let units=(serial.fract()*86400.0*scale).round();
    let seconds=(units/scale).floor() as u32;
    let fraction=(units-f64::from(seconds)*scale).round() as u64;
    let has_time = date_tokens.chars().any(|c| c == 'h' || c == 's');
    let has_date = date_tokens.chars().any(|c| c == 'y' || c == 'd');
    let date_serial = serial.floor()
        + if has_date && has_time && seconds >= 86400 {
            1.0
        } else {
            0.0
        };
    if date_serial > 2958465.0 {
        return "#NUM!".into();
    }
    let (y, m, d) = serial_date(date_serial);
    let hour = (seconds / 3600) % 24;
    let minute = (seconds / 60) % 60;
    let second = seconds % 60;
    let chars: Vec<_> = code.chars().collect();
    let mut i = 0;
    let mut out = String::new();
    let mut quote = false;
    let ampm = date_tokens.contains("am/pm") || date_tokens.contains("a/p");
    let mut previous = ' ';
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            quote = !quote;
            i += 1;
            continue;
        }
        if c == '\\' {
            if let Some(next) = chars.get(i + 1) {
                out.push(*next);
            }
            i += 2;
            continue;
        }
        if quote {
            out.push(c);
            i += 1;
            continue;
        }
        if matches!(c, '_' | '*') {
            if c == '_' { out.push(' '); }
            i = (i + 2).min(chars.len());
            continue;
        }
        if chars[i..].iter().take(3).collect::<String>().eq_ignore_ascii_case("A/P") {
            out.push(if hour<12 {'A'}else{'P'});
            i+=3;
            continue;
        }
        let remaining = chars[i..].iter().take(5).collect::<String>();
        if remaining.eq_ignore_ascii_case("AM/PM") {
            out.push_str(if hour < 12 { "AM" } else { "PM" });
            i += 5;
            continue;
        }
        if c == '[' {
            let rest = chars[i + 1..]
                .iter()
                .take_while(|c| **c != ']')
                .collect::<String>();
            let lower = rest.to_ascii_lowercase();
            if matches!(lower.as_str(), "h" | "hh" | "m" | "mm" | "s" | "ss") {
                let factor = match lower.chars().next() {
                    Some('h') => 24.0,
                    Some('m') => 1440.0,
                    _ => 86400.0,
                };
                let rounded_seconds = serial.floor() * 86400.0 + f64::from(seconds);
                let total = (rounded_seconds / (86400.0 / factor)).floor();
                out.push_str(&format!("{total:0width$.0}", width = lower.len()));
                previous = lower.chars().next().unwrap_or(' ');
                i += rest.chars().count() + 2;
                if lower.starts_with('s') {
                    append_time_fraction(&mut out,&chars,&mut i,fraction,fraction_digits,ru);
                }
                continue;
            }
        }
        let token = c.to_ascii_lowercase();
        if matches!(token, 'y' | 'm' | 'd' | 'h' | 's') {
            let start = i;
            while chars
                .get(i)
                .is_some_and(|v| v.to_ascii_lowercase() == token)
            {
                i += 1;
            }
            let count = i - start;
            let next = outside_quotes(&chars[i..].iter().collect::<String>())
                .chars().find(|v|v.is_ascii_alphabetic()).map(|v|v.to_ascii_lowercase());
            let minutes = token == 'm' && (previous == 'h' || next == Some('s'));
            let n = match token {
                'y' => {
                    if count <= 2 {
                        y % 100
                    } else {
                        y
                    }
                }
                'm' => {
                    if minutes {
                        i64::from(minute)
                    } else {
                        m
                    }
                }
                'd' => d,
                'h' => {
                    if ampm {
                        i64::from(if hour.is_multiple_of(12) {
                            12
                        } else {
                            hour % 12
                        })
                    } else {
                        i64::from(hour)
                    }
                }
                _ => i64::from(second),
            };
            if token == 'd' && count >= 3 {
                let weekday = (date_serial as i64 - 1).rem_euclid(7) as usize;
                let names = if count == 3 {
                    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
                } else {
                    [
                        "Sunday",
                        "Monday",
                        "Tuesday",
                        "Wednesday",
                        "Thursday",
                        "Friday",
                        "Saturday",
                    ]
                };
                let russian=if count==3 {
                    ["вс","пн","вт","ср","чт","пт","сб"]
                } else {
                    ["воскресенье","понедельник","вторник","среда","четверг","пятница","суббота"]
                };
                out.push_str(if ru {russian.get(weekday).copied().unwrap_or("")} else {names.get(weekday).copied().unwrap_or("")});
            } else if token == 'm' && !minutes && count >= 3 {
                let names = if count == 3 {
                    [
                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                        "Nov", "Dec",
                    ]
                } else {
                    [
                        "January",
                        "February",
                        "March",
                        "April",
                        "May",
                        "June",
                        "July",
                        "August",
                        "September",
                        "October",
                        "November",
                        "December",
                    ]
                };
                let day_number=date_tokens.split(|c|c!='d').any(|run| !run.is_empty() && run.len()<=2);
                let russian=if count==3 {
                    ["янв","фев","мар","апр","май","июн","июл","авг","сен","окт","ноя","дек"]
                } else if day_number {
                    ["января","февраля","марта","апреля","мая","июня","июля","августа","сентября","октября","ноября","декабря"]
                } else {
                    ["январь","февраль","март","апрель","май","июнь","июль","август","сентябрь","октябрь","ноябрь","декабрь"]
                };
                let name = if ru {russian.get((m-1) as usize).copied().unwrap_or("")}
                    else {names.get((m - 1) as usize).copied().unwrap_or("")};
                if count == 5 {
                    out.extend(name.chars().take(1));
                } else {
                    out.push_str(name);
                }
            } else if count >= 2 {
                out.push_str(&format!(
                    "{n:0width$}",
                    width = if token == 'y' && count > 2 { 4 } else { 2 }
                ));
            } else {
                out.push_str(&n.to_string());
            }
            if token=='s' {
                append_time_fraction(&mut out,&chars,&mut i,fraction,fraction_digits,ru);
            }
            previous = token;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}
#[cfg(test)]
mod tests {
    #[test]
    fn review5_scientific_integer_placeholders_control_the_exponent_group() {
        for (number, code, expected) in [
            (12_200_000.0, "#0.0E+0", "12.2E+6"),
            (12_345.0, "##0.0E+0", "12.3E+3"),
            (0.012345, "##0.0E+0", "12.3E-3"),
            (999.99, "##0.0E+0", "1.0E+3"),
        ] {
            assert_eq!(display(&Value::Number(number), code, false), expected);
        }
    }
    #[test]
    fn review5_fraction_patterns_keep_fixed_denominators_and_improper_values() {
        for (number, code, expected) in [
            (1.25, "# ?/8", "1 2/8"),
            (1.25, "?/??", "5/4"),
            (0.125, "# ???/???", "1/8"),
            (-1.25, "# ?/8", "-1 2/8"),
        ] {
            assert_eq!(display(&Value::Number(number), code, false), expected);
        }
    }
    #[test]
    fn review4_scientific_exponent_sign_follows_the_format() {
        for (number, code, expected) in [
            (1234.0, "0.0E-00", "1.2E03"),
            (0.01234, "0.0E-00", "1.2E-02"),
            (1234.0, "0.0E+00", "1.2E+03"),
        ] {
            assert_eq!(display(&Value::Number(number), code, false), expected);
        }
    }
    #[test]
    fn review4_scientific_ties_round_away_and_carry_into_the_exponent() {
        for (number, code, expected) in [
            (2.675, "0.00E+00", "2.68E+00"),
            (-2.675, "0.00E+00", "-2.68E+00"),
            (9.95, "0.0E+00", "1.0E+01"),
        ] {
            assert_eq!(display(&Value::Number(number), code, false), expected);
        }
        assert!(!display(&Value::Number(f64::from_bits(1)), "0.0E+00", false).contains("NaN"));
    }

    use super::*;
    #[test]
    fn review2_elapsed_units_share_the_clock_rounding_carry() {
        let value = Value::Number((25.0 * 3600.0 + 3599.9) / 86400.0);
        for (code, expected) in [
            ("[h]:mm:ss", "26:00:00"),
            ("[m]:ss", "1560:00"),
            ("[s]", "93600"),
        ] {
            assert_eq!(display(&value, code, false), expected, "{code}");
        }
    }

    #[test]
    fn quoted_and_escaped_format_literals_do_not_become_placeholders() {
        for (format, expected) in [
            ("\"v0 \"0.00", "v0 12.34"),
            ("0.00\" 000\"", "12.34 000"),
            ("\\00.00", "012.34"),
            ("\"A_B*% \"0.00", "A_B*% 12.34"),
            ("0.00\\\"", "12.34\""),
        ] {
            assert_eq!(
                display(&Value::Number(12.34), format, false),
                expected,
                "{format}"
            );
        }
    }
    #[test]
    fn finite_large_numbers_do_not_overflow_decimal_rounding() {
        let rendered = display(&Value::Number(1e308), "0.00", false);
        assert!(rendered.ends_with(".00"));
        assert!(!rendered.contains("inf"));
        assert_eq!(display(&Value::Number(1e308), "0.00%", false), "#NUM!");
    }
    #[test]
    fn rounded_midnight_keeps_calendar_date_consistent() {
        let serial = super::super::formula::date_serial(2026, 10, 13) + 86399.9 / 86400.0;
        assert_eq!(
            display(&Value::Number(serial), "yyyy-mm-dd hh:mm:ss", false),
            "2026-10-14 00:00:00"
        );
        assert_eq!(
            display(&Value::Number(serial), "yyyy-mm-dd", false),
            "2026-10-13"
        );
        assert_eq!(
            display(
                &Value::Number(59.0 + 86399.9 / 86400.0),
                "yyyy-mm-dd hh:mm:ss",
                false
            ),
            "1900-02-29 00:00:00"
        );
    }

    #[test]
    fn review_number_placeholders_and_conditions() {
        for (number, code, expected) in [
            (12.0, "00000", "00012"),
            (-12.0, "00000", "-00012"),
            (12.3, "0.##", "12.3"),
            (12.0, "0.0#", "12.0"),
            (0.25, "#.##", ".25"),
            (25.0, "[>=100]0;\"small\"", "small"),
            (125.0, "[>=100]0;\"small\"", "125"),
            (-25.0, "[>=100]0;[<0]0;\"small\"", "-25"),
            (12.0, "\"[code] \"0", "[code] 12"),
            (12.0, "\\[\\]0", "[]12"),
        ] {
            assert_eq!(
                display(&Value::Number(number), code, false),
                expected,
                "{code}"
            );
        }
    }
    #[test]
    fn review_calendar_tokens_and_text_placeholder() {
        let serial = super::super::formula::date_serial(2024, 1, 1);
        for (code, expected) in [
            ("m", "1"),
            ("mm", "01"),
            ("mmm", "Jan"),
            ("mmmm", "January"),
            ("mmmmm", "J"),
            ("ddd", "Mon"),
            ("dddd", "Monday"),
        ] {
            assert_eq!(
                display(&Value::Number(serial), code, false),
                expected,
                "{code}"
            );
        }
        assert_eq!(
            display(&Value::Text("Acme".into()), "\"Customer: \"@", false),
            "Customer: Acme"
        );
        assert_eq!(
            display(&Value::Text("Acme".into()), "0;0;0;\"@\" @", false),
            "@ Acme"
        );
    }
    #[test]
    fn formats() {
        let cases = [
            (0.125, "0.00%", false, "12.50%"),
            (1234.5, "#,##0.00", false, "1,234.50"),
            (-15.0, "0;(0)", false, "(15)"),
            (60.0, "yyyy-mm-dd", false, "1900-02-29"),
            (0.5, "hh:mm:ss", false, "12:00:00"),
            (12.34, "0.00", true, "12,34"),
            (0.0, "0;0;", false, ""),
            (1.5, "# ?/?", false, "1 1/2"),
            (2.5, "0", false, "3"),
            (1.005, "0.00", false, "1.01"),
            (1234.5, "0.00E+00", false, "1.23E+03"),
            (0.00012, "0.0E+00", true, "1,2E-04"),
            (1234.5, "#,##0.00 [$€-407]", false, "1,234.50 €"),
            (0.1 + 0.2, "General", false, "0.3"),
            (std::f64::consts::PI, "General", true, "3,141592654"),
            (123456789012.0, "General", false, "1.23457E+11"),
            (12345678901.0, "General", false, "12345678901"),
            (1e-10, "General", false, "1E-10"),
        ];
        for (n, fmt, ru, want) in cases {
            assert_eq!(display(&Value::Number(n), fmt, ru), want, "{fmt}");
        }
    }
    #[test]
    fn time_context_unicode_and_elapsed_formats() {
        let serial =
            super::super::formula::date_serial(2026, 10, 13) + 0.5 + 34.0 / 1440.0 + 56.0 / 86400.0;
        assert_eq!(
            display(&Value::Number(serial), "yyyy-mm-dd hh:mm:ss", false),
            "2026-10-13 12:34:56"
        );
        assert_eq!(display(&Value::Number(serial), "mm:ss", false), "34:56");
        assert_eq!(
            display(&Value::Number(serial), "\"₽ \"yyyy-mm-dd hh:mm", false),
            "₽ 2026-10-13 12:34"
        );
        assert_eq!(display(&Value::Number(1.5), "[h]:mm:ss", false), "36:00:00");
        assert_eq!(
            display(&Value::Number(0.75), "h:mm AM/PM", false),
            "6:00 PM"
        );
    }
}

fn append_time_fraction(out: &mut String,chars: &[char],index: &mut usize,fraction: u64,precision: usize,ru: bool) {
    if chars.get(*index)!=Some(&'.') {return;}
    let digits=chars[*index+1..].iter().take_while(|c|**c=='0').count().min(9);
    if digits==0 {return;}
    let value=if precision>=digits {fraction/10u64.pow((precision-digits) as u32)}
        else {fraction*10u64.pow((digits-precision) as u32)};
    out.push(if ru {','}else{'.'});
    out.push_str(&format!("{value:0width$}",width=digits));
    *index+=digits+1;
}
#[cfg(test)]
mod format_defect_tests {
    use super::*;
    #[test]
    fn short_am_pm_format_is_a_clock_token() {
        assert_eq!(display(&Value::Number(0.75),"h:mm A/P",false),"6:00 P");
        assert_eq!(display(&Value::Number(0.25),"A/P",false),"A");
    }
    #[test]
    fn fractional_seconds_are_formatted_and_carried_at_the_requested_precision() {
        assert_eq!(display(&Value::Number(56.789/86400.0),"ss.000",false),"56.789");
        assert_eq!(display(&Value::Number(59.96/86400.0),"mm:ss.0",false),"01:00.0");
        assert_eq!(display(&Value::Number(1.0+0.125/86400.0),"[s].000",false),"86400.125");
    }
    #[test]
    fn russian_calendar_tokens_follow_the_workbook_locale() {
        let date=super::super::formula::date_serial(2024,1,1);
        assert_eq!(display(&Value::Number(date),"dddd",true),"понедельник");
        assert_eq!(display(&Value::Number(date),"d mmmm yyyy",true),"1 января 2024");
        assert_eq!(display(&Value::Number(date),"mmmm",false),"January");
        assert_eq!(display(&Value::Number(2958465.999999),"yyyy-mm-dd",false),"9999-12-31");
    }
}

fn integer_literal_mask(number: f64,pattern: &str) -> Option<String> {
    if !pattern.contains(['"','\\']) {return None;}
    let mut parts=Vec::new();
    let (mut quoted,mut escaped)=(false,false);
    for ch in pattern.chars() {
        if escaped {parts.push((false,ch));escaped=false;continue;}
        if ch=='"' {quoted=!quoted;continue;}
        if !quoted && ch=='\\' {escaped=true;continue;}
        if quoted {parts.push((false,ch));}
        else if matches!(ch,'0'|'#'|'?') {parts.push((true,ch));}
        else if matches!(ch,'('|')'|' '|'-'|'+') {parts.push((false,ch));}
        else {return None;}
    }
    let mut digits:Vec<_>=fixed(number.abs(),0).chars().collect();
    let mut out=Vec::new();
    for (slot,ch) in parts.into_iter().rev() {
        if slot {
            if let Some(digit)=digits.pop() {out.push(digit);}
            else if ch=='0' {out.push('0');}
            else if ch=='?' {out.push(' ');}
        } else {out.push(ch);}
    }
    while let Some(digit)=digits.pop() {out.push(digit);}
    if number<0.0 {out.push('-');}
    Some(out.into_iter().rev().collect())
}
#[cfg(test)]
mod second_format_defects {
    use super::*;
    #[test]
    fn spacing_and_fill_arguments_do_not_split_format_sections() {
        assert_eq!(split_sections("0_;"),vec!["0_;".to_string()]);
        assert_eq!(split_sections("0*;;(0)"),vec!["0*;".to_string(),"(0)".to_string()]);
    }
    #[test]
    fn integer_format_keeps_embedded_literal_masks() {
        assert_eq!(display(&Value::Number(1234.0),r#"00"-"00"#,false),"12-34");
        assert_eq!(display(&Value::Number(1234567.0),r#"(000) 00"-"00"#,false),"(123) 45-67");
        assert_eq!(display(&Value::Number(42.0),r#"00"-"00"#,false),"00-42");
    }
    #[test]
    fn scaling_commas_before_a_decimal_are_applied() {
        assert_eq!(display(&Value::Number(12345.0),"0,.0",false),"12.3");
        assert_eq!(display(&Value::Number(12345000.0),r#"0,,.00" M""#,false),"12.35 M");
    }
    #[test]
    fn fraction_formats_keep_percentages_and_literal_affixes() {
        assert_eq!(display(&Value::Number(0.125),"# ?/?%",false),"12 1/2%");
        assert_eq!(display(&Value::Number(1.25),r##""$"# ?/?" kg""##,false),"$1 1/4 kg");
    }
    #[test]
    fn quoted_words_cannot_change_month_tokens_into_minutes() {
        assert_eq!(display(&Value::Number(super::super::formula::date_serial(2024,1,15)),
            r#"mm" seconds""#,false),"01 seconds");
    }
}

#[cfg(test)]
mod hundred_format_tests {
    use super::*;
    #[test]
    fn bracketed_general_keeps_numeric_rendering() {
        assert_eq!(display(&Value::Number(1234.5), "[Red]General", false), "1234.5");
        assert_eq!(display(&Value::Number(12.5), "[>=10][Blue]General;0", true), "12,5");
    }
    #[test]
    fn question_decimal_slots_reserve_spaces_instead_of_zeroes() {
        assert_eq!(display(&Value::Number(1.2), "0.??", false), "1.2 ");
        assert_eq!(display(&Value::Number(1.0), "0.??", false), "1.  ");
        assert_eq!(display(&Value::Number(1.2), "0.0?", false), "1.2 ");
        assert_eq!(display(&Value::Number(1.0), "0.##", false), "1");
    }
    #[test]
    fn date_spacing_and_fill_arguments_are_not_calendar_tokens() {
        let serial = super::super::formula::date_serial(2024, 1, 15);
        assert_eq!(display(&Value::Number(serial), "yyyy_m-mm", false), "2024 -01");
        assert_eq!(display(&Value::Number(serial), "yyyy*mmm", false), "202401");
    }
}

#[cfg(test)]
mod phase200_format_defects {
    use super::*;
    #[test]
    fn numeric_scaling_keeps_finite_results_without_infinite_scale_factors() {
        let code=format!("0.00E+00{}", "%".repeat(155));
        assert!(display(&Value::Number(1e-300),&code,false).starts_with("1.00E+10"));
        let code=format!("0.00E+00{}", ",".repeat(110));
        assert_eq!(display(&Value::Number(1e308),&code,false),"1.00E-22");
        assert_eq!(display(&Value::Number(1e308),"0.00E+00,,%",false),"1.00E+304%");
        assert_eq!(display(&Value::Number(0.0),&format!("0{}", "%".repeat(200)),false),format!("0{}", "%".repeat(200)));
    }
    #[test]
    fn fraction_masks_accept_zero_and_hash_numerator_placeholders() {
        assert_eq!(display(&Value::Number(1.5),"0/0",false),"3/2");
        assert_eq!(display(&Value::Number(1.5),"#/#",false),"3/2");
        assert_eq!(display(&Value::Number(1.5),"0/8",false),"12/8");
    }
    #[test]
    fn unrepresentable_fraction_numerators_never_fall_back_to_zero() {
        assert_eq!(display(&Value::Number(1e308),"?/8",false),"#NUM!");
        assert_eq!(display(&Value::Number(0.0),"?/8",false),"0/8");
    }
}

#[cfg(test)]
mod phase200_number_color_defect {
    use super::*;
    #[test]
    fn chosen_number_format_section_supplies_its_own_color() {
        let code="[Blue]0;[Red](0);[Green]0;[Magenta]@";
        assert_eq!(color(&Value::Number(-2.0),code),Some([255,0,0]));
        assert_eq!(color(&Value::Number(2.0),code),Some([0,0,255]));
        assert_eq!(color(&Value::Number(0.0),code),Some([0,255,0]));
        assert_eq!(color(&Value::Text("x".into()),code),Some([255,0,255]));
        assert_eq!(color(&Value::Number(2.0),"[>5][Red]0;[Blue]0"),Some([0,0,255]));
        assert_eq!(color(&Value::Number(2.0),"\"[Red]\"0"),None);
    }
}
