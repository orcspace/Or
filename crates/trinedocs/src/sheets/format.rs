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
    let conditional = sections
        .iter()
        .take(2)
        .any(|section| condition(section, *number).is_some());
    let section = if conditional {
        sections
            .iter()
            .take(3)
            .find(|section| condition(section, *number).unwrap_or(true))
    } else if *number == 0.0 && sections.len() > 2 {
        sections.get(2)
    } else if negative && sections.len() > 1 {
        sections.get(1)
    } else {
        sections.first()
    };
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
    let lower = section.to_ascii_lowercase();
    let bare = outside_quotes(&lower);
    if bare
        .chars()
        .any(|c| matches!(c, 'y' | 'm' | 'd' | 'h' | 's'))
    {
        return date_display(numeric, &section);
    }
    if bare.contains('/') && bare.contains('?') {
        let sign = if numeric < 0.0 { "-" } else { "" };
        let n = numeric.abs();
        let whole = n.floor();
        let fraction = n.fract();
        let max_den = if bare.contains("??") { 99 } else { 9 };
        let mut best = (0u32, 1u32, f64::INFINITY);
        for den in 1..=max_den {
            let numerator = (fraction * f64::from(den)).round() as u32;
            let diff = (fraction - f64::from(numerator) / f64::from(den)).abs();
            if diff < best.2 {
                best = (numerator, den, diff);
            }
        }
        if best.0 == 0 {
            return format!("{sign}{whole:.0}");
        }
        if best.0 == best.1 {
            return format!("{sign}{:.0}", whole + 1.0);
        }
        return format!(
            "{sign}{}{}/{}",
            if whole == 0.0 {
                String::new()
            } else {
                format!("{whole:.0} ")
            },
            best.0,
            best.1
        );
    }
    let percent = bare.chars().filter(|c| *c == '%').count();
    let mut n = numeric * 100f64.powi(percent as i32);
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
    let pattern = section.get(start..end).unwrap_or("0");
    let tail = section.get(end..).unwrap_or("");
    let scale = tail.chars().take_while(|c| *c == ',').count();
    n /= 1000f64.powi(scale as i32);
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
    let rendered = if let Some(exponent) = pattern.to_uppercase().find('E') {
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
fn scientific(n: f64, decimals: usize, exponent_digits: usize, positive_sign: bool) -> String {
    let raw = format!("{n:E}");
    let (mantissa, exponent) = raw.split_once('E').unwrap_or((&raw, "0"));
    let mut mantissa = round_half_away(mantissa.parse::<f64>().unwrap_or(0.0), decimals);
    let mut exponent: i32 = exponent.parse().unwrap_or(0);
    if mantissa.abs() >= 10.0 {
        mantissa /= 10.0;
        exponent += 1;
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
        let text = scientific(n, mantissa, 2, true);
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
    for c in code.chars() {
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
    let optional = decimals.chars().rev().take_while(|c| *c == '#').count();
    for _ in 0..optional.min(decimals.len() - mandatory) {
        if text.ends_with('0') {
            text.pop();
        } else {
            break;
        }
    }
    if text.ends_with('.') {
        text.pop();
    }
    let sign = usize::from(text.starts_with('-'));
    let end = text.find('.').unwrap_or(text.len());
    let digits = end - sign;
    if digits < minimum_digits {
        text.insert_str(sign, &"0".repeat(minimum_digits - digits));
    } else if minimum_digits == 0 && text.get(sign..end) == Some("0") {
        text.remove(sign);
        if text == "-" {
            text.clear();
        }
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
fn date_display(serial: f64, code: &str) -> String {
    if !(0.0..=2958465.99999).contains(&serial) {
        return "#NUM!".into();
    }
    let seconds = (serial.fract() * 86400.0).round() as u32;
    let date_tokens = outside_quotes(&code.to_ascii_lowercase());
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
    let ampm = outside_quotes(&code.to_ascii_lowercase()).contains("am/pm");
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
            let next = chars[i..]
                .iter()
                .find(|v| v.is_ascii_alphabetic())
                .map(|v| v.to_ascii_lowercase());
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
                out.push_str(names.get(weekday).copied().unwrap_or(""));
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
                let name = names.get((m - 1) as usize).copied().unwrap_or("");
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
