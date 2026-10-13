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
        if let Value::Text(text) = value
            && let Some(section) = split_sections(code).get(3)
        {
            return unquote(section).replace('@', text);
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
    let section = if *number == 0.0 && sections.len() > 2 {
        &sections[2]
    } else if negative && sections.len() > 1 {
        &sections[1]
    } else {
        &sections[0]
    };
    if section.is_empty() {
        return String::new();
    }
    let section = strip_brackets(section);
    let numeric = if negative && sections.len() > 1 {
        number.abs()
    } else {
        *number
    };
    let lower = section.to_ascii_lowercase();
    let bare = outside_quotes(&lower);
    if bare.contains('y') || bare.contains('d') || bare.contains('h') || bare.contains('s') {
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
    let first = section
        .char_indices()
        .find(|(_, c)| matches!(c, '0' | '#' | '?'))
        .map(|(i, _)| i);
    let last = section
        .char_indices()
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
    let decimals = pattern
        .split('.')
        .nth(1)
        .map(|s| {
            s.chars()
                .take_while(|c| matches!(c, '0' | '#' | '?'))
                .count()
        })
        .unwrap_or(0)
        .min(15);
    let rendered = if let Some(exponent) = pattern.to_uppercase().find('E') {
        let digits = pattern
            .get(exponent..)
            .unwrap_or("")
            .chars()
            .filter(|c| *c == '0')
            .count()
            .max(1);
        localize(&scientific(n, decimals, digits), ru)
    } else if pattern.split('.').next().is_some_and(|s| s.contains(',')) {
        group(&fixed(n, decimals), ru)
    } else {
        localize(&fixed(n, decimals), ru)
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
    round_scaled(n * scale) / scale
}
/// Rounds to an integer, treating values within float noise of .5 as the tie.
pub fn round_scaled(scaled: f64) -> f64 {
    let tie = (scaled.fract().abs() - 0.5).abs() <= 8.0 * f64::EPSILON * scaled.abs().max(1.0);
    if tie {
        scaled.trunc() + scaled.signum()
    } else {
        scaled.round()
    }
}
/// An integer when `x` is one up to float noise (0.3/0.1 = 2.9999999999999996 is 3).
pub fn snap(x: f64) -> f64 {
    if (x - x.round()).abs() <= 8.0 * f64::EPSILON * x.abs().max(1.0) {
        x.round()
    } else {
        x
    }
}
fn fixed(n: f64, decimals: usize) -> String {
    format!("{:.decimals$}", round_half_away(n, decimals))
}
/// `1.23E+03`: Excel always signs the exponent and pads it to the pattern's digit count.
fn scientific(n: f64, decimals: usize, exponent_digits: usize) -> String {
    let text = format!("{n:.decimals$E}");
    let (mantissa, exponent) = text.split_once('E').unwrap_or((&text, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let sign = if exponent < 0 { '-' } else { '+' };
    format!(
        "{mantissa}E{sign}{:0exponent_digits$}",
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
        let text = scientific(n, mantissa, 2);
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
    for c in code.chars() {
        if skip {
            skip = false;
            continue;
        }
        if c == '"' || c == '\\' {
            continue;
        }
        if c == '_' || c == '*' {
            skip = true;
            continue;
        }
        out.push(c);
    }
    out
}
fn outside_quotes(code: &str) -> String {
    let mut quote = false;
    let mut escape = false;
    let mut result = String::new();
    for c in code.chars() {
        if c == '"' && !escape {
            quote = !quote;
        } else if !quote && !escape {
            result.push(c);
        }
        escape = c == '\\' && !escape;
    }
    result
}
fn strip_brackets(code: &str) -> String {
    let mut out = String::new();
    let mut bracket = String::new();
    let mut inside = false;
    for c in code.chars() {
        if c == '[' {
            inside = true;
            bracket.clear();
        } else if c == ']' {
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
    let (y, m, d) = serial_date(serial);
    let seconds = (serial.fract() * 86400.0).round() as u32;
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
                let total = (serial * factor + 1e-7).floor();
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
            if token == 'm' && !minutes && count >= 3 {
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
                out.push_str(names.get((m - 1) as usize).copied().unwrap_or(""));
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
    use super::*;
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
