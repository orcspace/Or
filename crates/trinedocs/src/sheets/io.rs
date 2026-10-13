use super::model::{
    Align, Book, CellFormat, ChartKind, MAX_RANGE_CELLS, Value, parse_address, parse_range,
};
use calamine::{Data, Reader as _, open_workbook_auto_from_rs};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use rust_xlsxwriter::{
    Chart, ChartType, Color, Format, FormatAlign, FormatBorder, FormatUnderline, Formula, Image,
    Workbook,
};
use std::{collections::BTreeMap, io::Cursor};

const MAX_FILE: usize = 64 * 1024 * 1024;
pub fn read(bytes: &[u8], ext: &str) -> Result<Book, String> {
    if bytes.len() > MAX_FILE {
        return Err("Файл превышает 64 МБ".into());
    }
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "csv" | "tsv" | "txt" => {
            let text = decode_text(bytes)?;
            let sep = if ext.eq_ignore_ascii_case("tsv") {
                '\t'
            } else {
                detect_separator(&text)
            };
            let data = parse_csv(&text, sep)?;
            let mut book = Book::default();
            book.set_range(book.active_id(), 0, 0, &data)?;
            Ok(book)
        }
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xls" | "xlsb" | "ods" => read_excel(bytes),
        _ => Err(format!("Не поддерживается импорт {ext}")),
    }
}
pub fn write(book: &Book, ext: &str) -> Result<Vec<u8>, String> {
    book.validate()?;
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "xlsx" => write_xlsx(book),
        "csv" | "tsv" => {
            let s = book.active_sheet().ok_or("Нет активного листа")?;
            let (r, c) = s.extent();
            if (u64::from(r) + 1) * (u64::from(c) + 1) > MAX_RANGE_CELLS as u64 {
                return Err("Экспортируемый диапазон превышает 100 000 ячеек".into());
            }
            let rows = book.read_range(s.id, 0, 0, r, c)?;
            Ok(format!(
                "\u{feff}{}",
                to_csv(
                    &rows,
                    if ext.eq_ignore_ascii_case("tsv") {
                        '\t'
                    } else {
                        ';'
                    }
                )
            )
            .into_bytes())
        }
        "html" => {
            let mut out = String::from(
                "<!doctype html><meta charset=\"utf-8\"><style>td{border:1px solid #d6dbe1;padding:4px}table{border-collapse:collapse}</style>",
            );
            for s in &book.sheets {
                let (r, c) = s.extent();
                if (u64::from(r) + 1) * (u64::from(c) + 1) > MAX_RANGE_CELLS as u64 {
                    return Err("Слишком большой диапазон HTML".into());
                }
                out.push_str(&format!(
                    "<h2>{}</h2><table>",
                    crate::package::xml_escape(&s.name)
                ));
                for row in 0..=r {
                    out.push_str("<tr>");
                    for col in 0..=c {
                        let fmt = s.cell(row, col).map(|c| c.fmt.clone()).unwrap_or_default();
                        let fill = fmt
                            .fill
                            .map(|v| format!("background:#{:02x}{:02x}{:02x};", v[0], v[1], v[2]))
                            .unwrap_or_default();
                        out.push_str(&format!(
                            "<td style=\"{}color:#{:02x}{:02x}{:02x};font-size:{}pt;{}{}\">{}</td>",
                            fill,
                            fmt.color[0],
                            fmt.color[1],
                            fmt.color[2],
                            fmt.size,
                            if fmt.bold { "font-weight:bold;" } else { "" },
                            if fmt.italic { "font-style:italic;" } else { "" },
                            crate::package::xml_escape(&s.display(row, col, book.locale_ru))
                        ));
                    }
                    out.push_str("</tr>");
                }
                out.push_str("</table>");
            }
            Ok(out.into_bytes())
        }
        "md" => {
            let s = book.active_sheet().ok_or("Нет активного листа")?;
            let (r, c) = s.extent();
            let rows = book.read_range(s.id, 0, 0, r, c)?;
            let mut out = format!("# {}\n\n", s.name);
            for (i, row) in rows.iter().enumerate() {
                out.push_str(&format!(
                    "| {} |\n",
                    row.iter()
                        .map(|s| s.replace('|', "\\|").replace('\n', "<br>"))
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
                if i == 0 {
                    out.push_str(&format!("|{}|\n", vec![" --- "; row.len()].join("|")));
                }
            }
            Ok(out.into_bytes())
        }
        _ => Err(format!("Не поддерживается сохранение {ext}")),
    }
}
pub fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if !(bytes.len() - 2).is_multiple_of(2) {
            return Err("Нечётная длина UTF-16".into());
        }
        let little = bytes.starts_with(&[0xff, 0xfe]);
        let units = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                if little {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|e| e.to_string());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok(s.into()),
        Err(_) => {
            let (text, _, errors) = encoding_rs::WINDOWS_1251.decode(bytes);
            if errors {
                Err("Недопустимая кодировка CSV".into())
            } else {
                Ok(text.into_owned())
            }
        }
    }
}
pub fn detect_separator(text: &str) -> char {
    ['\t', ';', ',']
        .into_iter()
        .max_by_key(|sep| {
            let mut quoted = false;
            let mut count = 0;
            for c in text.chars().take(8192) {
                if c == '"' {
                    quoted = !quoted;
                } else if c == '\n' && !quoted {
                    break;
                } else if c == *sep && !quoted {
                    count += 1;
                }
            }
            count
        })
        .unwrap_or(',')
}
pub fn parse_csv(text: &str, separator: char) -> Result<Vec<Vec<String>>, String> {
    if !matches!(separator, ',' | ';' | '\t' | '|') {
        return Err("Недопустимый разделитель CSV".into());
    }
    let text = text.trim_start_matches('\u{feff}');
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut after_quote = false;
    let mut chars = text.chars().peekable();
    let mut count = 0usize;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cell.push('"');
                    chars.next();
                } else {
                    quoted = false;
                    after_quote = true;
                }
            } else {
                cell.push(c);
            }
        } else if c == separator || c == '\n' || c == '\r' {
            row.push(std::mem::take(&mut cell));
            count += 1;
            after_quote = false;
            if count > MAX_RANGE_CELLS {
                return Err("CSV превышает 100 000 ячеек".into());
            }
            if c == '\n' || c == '\r' {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                rows.push(std::mem::take(&mut row));
            }
        } else if c == '"' && cell.is_empty() && !after_quote {
            quoted = true;
        } else if after_quote {
            if c != ' ' && c != '\t' {
                return Err("CSV: неожиданный символ после кавычки".into());
            }
        } else if c == '"' {
            return Err("CSV: кавычка внутри неэкранированного поля".into());
        } else {
            cell.push(c);
        }
        if cell.len() > 32767 {
            return Err("Поле CSV превышает 32 767 байт".into());
        }
    }
    if quoted {
        return Err("CSV: незакрытая кавычка".into());
    }
    if !cell.is_empty() || !row.is_empty() || after_quote || text.ends_with(separator) {
        row.push(cell);
        rows.push(row);
    }
    if rows.iter().map(Vec::len).sum::<usize>() > MAX_RANGE_CELLS {
        return Err("CSV превышает 100 000 ячеек".into());
    }
    Ok(rows)
}
pub fn to_csv(rows: &[Vec<String>], separator: char) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|s| {
                    if s.contains([separator, '\n', '\r', '"']) {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(&separator.to_string())
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
fn attrs(e: &BytesStart<'_>, _reader: &Reader<&[u8]>) -> Result<BTreeMap<String, String>, String> {
    e.attributes()
        .map(|a| {
            let a = a.map_err(|e| e.to_string())?;
            let key = a.key.as_ref().to_string();
            let value = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map_err(|e| e.to_string())?
                .into_owned();
            Ok((key, value))
        })
        .collect()
}
fn local(e: &BytesStart<'_>) -> String {
    e.local_name().as_ref().to_string()
}
fn each_tag(
    bytes: &[u8],
    mut f: impl FnMut(&str, &BTreeMap<String, String>) -> Result<(), String>,
) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_str(text);
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) | Event::Empty(e) => {
                f(&local(&e), &attrs(&e, &reader)?)?;
            }
            Event::DocType(_) => return Err("DTD в офисных файлах запрещён".into()),
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(())
}
fn worksheet_root(bytes: &[u8]) -> Result<bool, String> {
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) | Event::Empty(e) => return Ok(local(&e) == "worksheet"),
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => return Ok(false),
            _ => (),
        }
    }
}
fn ods_preflight(bytes: &[u8]) -> Result<(), String> {
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let mut depth = 0usize;
    let mut table_rows = 0u64;
    let mut table_cols = 0u64;
    let mut row_cols = 0u64;
    let mut row_repeat = 1u64;
    let mut cell_repeat = 1u64;
    let mut cell_bytes = 0u64;
    let mut row_bytes = 0u64;
    let mut total_bytes = 0u64;
    let mut total_dense = 0u64;
    let mut in_cell = false;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                if !empty {
                    depth += 1;
                    if depth > 128 {
                        return Err("ODS: слишком глубокая структура".into());
                    }
                }
                let tag = local(&e);
                let a = attrs(&e, &reader)?;
                let repeat = |name: &str| -> Result<u64, String> {
                    let v = a
                        .iter()
                        .find(|(k, _)| k.rsplit(':').next() == Some(name))
                        .map(|(_, v)| v.parse::<u64>())
                        .transpose()
                        .map_err(|_| "ODS: неверное повторение")?
                        .unwrap_or(1);
                    if v == 0 {
                        return Err("ODS: нулевое повторение".into());
                    }
                    Ok(v)
                };
                match tag.as_str() {
                    "table" => {
                        table_rows = 0;
                        table_cols = 0;
                    }
                    "table-row" => {
                        row_repeat = repeat("number-rows-repeated")?;
                        row_cols = 0;
                        row_bytes = 0;
                        if row_repeat > 1_048_576 {
                            return Err("ODS: слишком много строк".into());
                        }
                        if empty {
                            table_rows = table_rows
                                .checked_add(row_repeat)
                                .ok_or("ODS: размер переполнен")?;
                        }
                    }
                    "table-cell" | "covered-table-cell" => {
                        cell_repeat = repeat("number-columns-repeated")?;
                        cell_bytes = a.values().map(|v| v.len() as u64).sum::<u64>() + 64;
                        row_cols = row_cols
                            .checked_add(cell_repeat)
                            .ok_or("ODS: размер переполнен")?;
                        if row_cols > 16384 {
                            return Err("ODS: слишком много столбцов".into());
                        }
                        in_cell = !empty;
                        if empty {
                            row_bytes = row_bytes
                                .checked_add(
                                    cell_bytes
                                        .checked_mul(cell_repeat)
                                        .ok_or("ODS: размер переполнен")?,
                                )
                                .ok_or("ODS: размер переполнен")?;
                        }
                    }
                    "s" if in_cell => {
                        let count = repeat("c")?;
                        cell_bytes = cell_bytes
                            .checked_add(count)
                            .ok_or("ODS: текст переполнен")?;
                    }
                    _ => (),
                }
            }
            Event::Text(t) if in_cell => {
                cell_bytes = cell_bytes
                    .checked_add(t.xml10_content().len() as u64)
                    .ok_or("ODS: текст переполнен")?
            }
            Event::GeneralRef(_) if in_cell => {
                cell_bytes = cell_bytes.checked_add(8).ok_or("ODS: текст переполнен")?
            }
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                match e.local_name().as_ref() {
                    "table-cell" | "covered-table-cell" => {
                        in_cell = false;
                        row_bytes = row_bytes
                            .checked_add(
                                cell_bytes
                                    .checked_mul(cell_repeat)
                                    .ok_or("ODS: размер переполнен")?,
                            )
                            .ok_or("ODS: размер переполнен")?;
                    }
                    "table-row" => {
                        table_rows = table_rows
                            .checked_add(row_repeat)
                            .ok_or("ODS: размер переполнен")?;
                        table_cols = table_cols.max(row_cols);
                        total_bytes = total_bytes
                            .checked_add(
                                row_bytes
                                    .checked_mul(row_repeat)
                                    .ok_or("ODS: размер переполнен")?,
                            )
                            .ok_or("ODS: размер переполнен")?;
                    }
                    "table" => {
                        total_dense = total_dense
                            .checked_add(
                                table_rows
                                    .checked_mul(table_cols)
                                    .ok_or("ODS: размер переполнен")?,
                            )
                            .ok_or("ODS: размер переполнен")?;
                    }
                    _ => (),
                }
            }
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => break,
            _ => (),
        }
        if table_rows > 1_048_576
            || table_rows
                .checked_mul(table_cols)
                .is_none_or(|v| v > 2_000_000)
            || total_dense > 2_000_000
            || total_bytes > 128 * 1024 * 1024
            || cell_bytes > 32767 + 4096
        {
            return Err("ODS: превышен безопасный предел распакованных ячеек/текста".into());
        }
    }
    Ok(())
}
fn xml_bounds(bytes: &[u8]) -> Result<Option<Vec<usize>>, String> {
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let mut root = String::new();
    let mut depth = 0usize;
    let mut tags = 0usize;
    let mut lengths = Vec::new();
    let mut in_string = false;
    let mut length = 0usize;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                if !empty {
                    depth += 1;
                }
                tags += 1;
                if depth > 128 || tags > 4_000_000 {
                    return Err("XLSX: слишком сложный XML".into());
                }
                let tag = local(&e);
                let a = attrs(&e, &reader)?;
                if root.is_empty() {
                    root = tag.clone();
                }
                if tag == "sst" {
                    for key in ["count", "uniqueCount"] {
                        if let Some(value) = a.get(key) {
                            let count = value
                                .parse::<u64>()
                                .map_err(|_| "XLSX: неверный count sharedStrings")?;
                            if count > 1_000_000 {
                                return Err("XLSX: слишком большой count sharedStrings".into());
                            }
                        }
                    }
                }
                if tag == "si" && root == "sst" {
                    in_string = !empty;
                    length = 0;
                    if empty {
                        lengths.push(0);
                    }
                }
                if tag == "f" {
                    if a.get("si")
                        .is_some_and(|v| v.parse::<u64>().map_or(true, |n| n > 100000))
                    {
                        return Err("XLSX: слишком большой индекс shared formula".into());
                    }
                    if let Some(reference) = a.get("ref") {
                        let mut p = reference.split(':');
                        let first = p
                            .next()
                            .and_then(parse_address)
                            .ok_or("XLSX: неверная область shared formula")?;
                        let last = p.next().and_then(parse_address).unwrap_or(first);
                        if p.next().is_some()
                            || (u64::from(first.0.abs_diff(last.0)) + 1)
                                * (u64::from(first.1.abs_diff(last.1)) + 1)
                                > 2_000_000
                        {
                            return Err("XLSX: слишком большая область shared formula".into());
                        }
                    }
                }
                if tag == "workbookPr"
                    && a.get("date1904")
                        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                {
                    return Err("Система дат Excel 1904 пока не поддерживается. Сохраните копию книги с системой 1900 в Excel.".into());
                }
            }
            Event::Text(t) if in_string => length = length.saturating_add(t.xml10_content().len()),
            Event::CData(t) if in_string => length = length.saturating_add(t.len()),
            Event::GeneralRef(_) if in_string => length = length.saturating_add(8),
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                if e.local_name().as_ref() == "si" && root == "sst" {
                    in_string = false;
                    lengths.push(length);
                }
            }
            Event::DocType(_) => return Err("DTD в офисных файлах запрещён".into()),
            Event::Eof => break,
            _ => (),
        }
        if length > 32767 || lengths.len() > 1_000_000 {
            return Err("XLSX: слишком длинная строка sharedStrings".into());
        }
    }
    if depth != 0 {
        return Err("XLSX: незавершённая структура XML".into());
    }
    Ok((root == "sst").then_some(lengths))
}
fn worksheet_bounds(bytes: &[u8], strings: &[usize]) -> Result<(u64, usize), String> {
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let (mut row, mut next_col) = (0u32, 0u32);
    let (mut minr, mut minc) = (u32::MAX, u32::MAX);
    let (mut maxr, mut maxc) = (0u32, 0u32);
    let mut count = 0usize;
    let mut text_bytes = 0usize;
    let mut cell_type = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut in_text = false;
    let mut in_formula = false;
    let mut formula_length = 0usize;
    let mut shared_id = None;
    let mut shared_anchor = false;
    let mut formula_lengths = BTreeMap::new();
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let tag = local(&e);
                let a = attrs(&e, &reader)?;
                match tag.as_str() {
                    "row" => {
                        if let Some(raw) = a.get("r") {
                            let value = raw.parse::<u32>().map_err(|_| "XLSX: неверная строка")?;
                            row = value.checked_sub(1).ok_or("XLSX: нулевая строка")?;
                        }
                        next_col = 0;
                        if row >= super::model::MAX_ROWS {
                            return Err("XLSX: строка за пределами листа".into());
                        }
                    }
                    "dimension" => {
                        if let Some(reference) = a.get("ref") {
                            let mut p = reference.split(':');
                            let first = p
                                .next()
                                .and_then(parse_address)
                                .ok_or("Неверные dimensions XLSX")?;
                            let last = p.next().and_then(parse_address).unwrap_or(first);
                            if p.next().is_some()
                                || (u64::from(first.0.abs_diff(last.0)) + 1)
                                    * (u64::from(first.1.abs_diff(last.1)) + 1)
                                    > 2_000_000
                            {
                                return Err("Слишком большие dimensions XLSX".into());
                            }
                        }
                    }
                    "c" => {
                        let (r, c) = if let Some(addr) = a.get("r") {
                            parse_address(addr).ok_or("Недопустимый адрес ячейки XLSX")?
                        } else {
                            (row, next_col)
                        };
                        if r >= super::model::MAX_ROWS || c >= super::model::MAX_COLS {
                            return Err("Недопустимый адрес ячейки XLSX".into());
                        }
                        next_col = c + 1;
                        minr = minr.min(r);
                        minc = minc.min(c);
                        maxr = maxr.max(r);
                        maxc = maxc.max(c);
                        count += 1;
                        if count > 1_000_000 {
                            return Err("Лист превышает 1 млн ячеек".into());
                        }
                        if a.get("s")
                            .is_some_and(|v| v.parse::<u64>().map_or(true, |n| n > 65535))
                        {
                            return Err("Неверный индекс стиля XLSX".into());
                        }
                        cell_type = a.get("t").cloned().unwrap_or_default();
                        value.clear();
                    }
                    "v" => {
                        in_value = !empty;
                        value.clear();
                    }
                    "t" if cell_type == "inlineStr" => in_text = !empty,
                    "f" => {
                        shared_id = a.get("si").and_then(|v| v.parse::<usize>().ok());
                        shared_anchor = a.contains_key("ref");
                        formula_length = 0;
                        in_formula = !empty;
                        if empty && let Some(id) = shared_id {
                            formula_length = *formula_lengths
                                .get(&id)
                                .ok_or("XLSX: shared formula до её основания")?;
                            text_bytes =
                                text_bytes.saturating_add(formula_length.saturating_mul(2) + 256);
                        }
                    }
                    _ => (),
                }
            }
            Event::Text(t) => {
                let text = t.xml10_content();
                let size = text.len();
                if in_value {
                    if value.len().saturating_add(size) > 32767 {
                        return Err("XLSX: слишком длинное значение".into());
                    }
                    value.push_str(&text);
                }
                if in_text {
                    text_bytes = text_bytes.saturating_add(size);
                }
                if in_formula {
                    formula_length = formula_length.saturating_add(size);
                }
            }
            Event::CData(t) => {
                if in_text {
                    text_bytes = text_bytes.saturating_add(t.len());
                }
                if in_formula {
                    formula_length = formula_length.saturating_add(t.len());
                }
            }
            Event::GeneralRef(r) => {
                if in_value {
                    let ch = r
                        .resolve_char_ref()
                        .map_err(|e| e.to_string())?
                        .ok_or("XLSX: именованная entity в числовом поле")?;
                    value.push(ch);
                    if value.len() > 32767 {
                        return Err("XLSX: слишком длинное значение".into());
                    }
                }
                if in_text {
                    text_bytes = text_bytes.saturating_add(8);
                }
                if in_formula {
                    formula_length = formula_length.saturating_add(8);
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                "row" => {
                    row = row.checked_add(1).ok_or("XLSX: строка переполнена")?;
                    next_col = 0;
                }
                "v" => {
                    in_value = false;
                    if cell_type == "s" {
                        let index = value
                            .parse::<usize>()
                            .map_err(|_| "XLSX: неверная ссылка sharedStrings")?;
                        text_bytes = text_bytes.saturating_add(
                            *strings.get(index).ok_or("XLSX: нет строки sharedStrings")?,
                        );
                    }
                }
                "t" => in_text = false,
                "f" => {
                    in_formula = false;
                    if shared_anchor && let Some(id) = shared_id {
                        formula_lengths.insert(id, formula_length);
                    } else if formula_length == 0
                        && let Some(id) = shared_id
                    {
                        formula_length = *formula_lengths
                            .get(&id)
                            .ok_or("XLSX: shared formula до её основания")?;
                    }
                    text_bytes = text_bytes.saturating_add(formula_length.saturating_mul(2) + 256);
                }
                _ => (),
            },
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => break,
            _ => (),
        }
        if formula_length > 32767 || text_bytes > 64 * 1024 * 1024 {
            return Err("XLSX: превышен предел распакованного текста/формул".into());
        }
    }
    let area = if count == 0 {
        0
    } else {
        (u64::from(maxr - minr) + 1) * (u64::from(maxc - minc) + 1)
    };
    if area > 2_000_000 {
        return Err("Слишком большой разреженный диапазон XLSX".into());
    }
    Ok((area, text_bytes))
}
fn preflight(parts: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    if parts.contains_key("xl/workbook.bin") {
        return Err(
            "Импорт XLSB отключён до изоляции бинарного импортёра. Используйте XLSX или CSV."
                .into(),
        );
    }
    let mut shared_strings = Vec::new();
    for (name, bytes) in parts {
        if name.ends_with(".xml")
            && let Some(strings) = xml_bounds(bytes)?
        {
            if !shared_strings.is_empty() {
                return Err("XLSX: несколько sharedStrings частей не поддерживаются".into());
            }
            shared_strings = strings;
        }
    }
    let mut dense_total = 0u64;
    let mut text_total = 0usize;
    for (name, bytes) in parts {
        if name == "content.xml" {
            ods_preflight(bytes)?;
        }
        if name.ends_with(".xml") && worksheet_root(bytes)? {
            let (area, text) = worksheet_bounds(bytes, &shared_strings)?;
            dense_total = dense_total
                .checked_add(area)
                .ok_or("Размер книги переполнен")?;
            text_total = text_total.saturating_add(text);
            if dense_total > 4_000_000 || text_total > 64 * 1024 * 1024 {
                return Err("XLSX: превышен безопасный предел книги".into());
            }
        }
    }
    Ok(())
}
fn read_excel(bytes: &[u8]) -> Result<Book, String> {
    if !bytes.starts_with(b"PK") {
        return Err(
            "Импорт XLS отключён до изоляции бинарного импортёра. Используйте XLSX или CSV.".into(),
        );
    }
    let parts = {
        let p = crate::package::read_zip(bytes)?;
        preflight(&p)?;
        Some(p)
    };
    let mut imported = open_workbook_auto_from_rs(Cursor::new(bytes))
        .map_err(|e| format!("Не удалось прочитать книгу: {e}"))?;
    let names = imported.sheet_names().to_vec();
    if names.len() > 1024 {
        return Err("Книга содержит больше 1024 листов".into());
    }
    let mut book = Book {
        sheets: Vec::new(),
        next_id: 1,
        ..Book::default()
    };
    book.names = imported.defined_names().iter().cloned().collect();
    let mut count = 0usize;
    for name in names {
        let id = book.add_sheet(&name)?;
        let range = imported
            .worksheet_range(&name)
            .map_err(|e| format!("Лист {name}: {e}"))?;
        let formulas = Some(
            imported
                .worksheet_formula(&name)
                .map_err(|e| format!("Формулы листа {name}: {e}"))?,
        );
        let start = range.start().unwrap_or((0, 0));
        for (r, c, value) in range.cells() {
            if *value == Data::Empty {
                continue;
            }
            count += 1;
            if count > 1_000_000 {
                return Err("Книга превышает 1 млн непустых ячеек".into());
            }
            let row = start.0 + r as u32;
            let col = start.1 + c as u32;
            if matches!(value,Data::Float(n)if !n.is_finite()) {
                return Err("XLSX: нечисловое числовое значение".into());
            }
            let input = if let Some(formula) = formulas
                .as_ref()
                .and_then(|f| f.get_value((row, col)))
                .filter(|s| !s.is_empty())
            {
                if formula.starts_with('=') {
                    formula.clone()
                } else {
                    format!("={formula}")
                }
            } else {
                match value {
                    Data::String(s) => format!("'{s}"),
                    Data::Int(n) => n.to_string(),
                    Data::Float(n) => n.to_string(),
                    Data::Bool(v) => if *v { "TRUE" } else { "FALSE" }.into(),
                    Data::DateTime(d) => d.as_f64().to_string(),
                    Data::DateTimeIso(s) | Data::DurationIso(s) => format!("'{s}"),
                    Data::Error(e) => e.to_string(),
                    Data::Empty => String::new(),
                }
            };
            book.set_input(id, row, col, &input)?;
            if matches!(value, Data::DateTime(_))
                && let Some(cell) = book.sheet_by_id_mut(id).and_then(|s| s.cell_mut(row, col))
            {
                cell.fmt.number = "yyyy-mm-dd hh:mm:ss".into();
            }
        }
        if let Some(formulas) = formulas {
            let start = formulas.start().unwrap_or((0, 0));
            for (r, c, f) in formulas.cells() {
                if !f.is_empty() {
                    let row = start.0 + r as u32;
                    let col = start.1 + c as u32;
                    book.set_input(id, row, col, &format!("={}", f.trim_start_matches('=')))?;
                }
            }
        }
    }
    if book.sheets.is_empty() {
        return Err("Книга не содержит листов".into());
    }
    if let Some(parts) = parts {
        read_metadata(&mut book, &parts)?;
        read_charts(&mut book, &parts)?;
    }
    book.recalculate();
    book.validate()?;
    Ok(book)
}
fn rgb(a: &BTreeMap<String, String>) -> Option<[u8; 3]> {
    let s = a.get("rgb")?;
    let s = if s.len() == 8 {
        s.get(2..)?
    } else {
        s.as_str()
    };
    if s.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(s.get(..2)?, 16).ok()?,
        u8::from_str_radix(s.get(2..4)?, 16).ok()?,
        u8::from_str_radix(s.get(4..6)?, 16).ok()?,
    ])
}
fn builtin(id: u32) -> String {
    match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "mm-dd-yy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm",
        19 => "h:mm:ss",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yy h:mm",
        49 => "@",
        _ => "General",
    }
    .into()
}
fn styles(bytes: &[u8]) -> Result<Vec<CellFormat>, String> {
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let mut section = String::new();
    let mut fonts = Vec::new();
    let mut fills = Vec::new();
    let mut formats = Vec::new();
    let mut num_formats = BTreeMap::new();
    let mut current_font = CellFormat::default();
    let mut current_fill = None;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) | Event::Empty(e) => {
                let tag = local(&e);
                let a = attrs(&e, &reader)?;
                if matches!(tag.as_str(), "fonts" | "fills" | "cellXfs" | "cellStyleXfs") {
                    section = tag.clone();
                }
                if tag == "font" {
                    current_font = CellFormat::default();
                }
                if tag == "fill" {
                    current_fill = None;
                }
                if section == "fonts" {
                    match tag.as_str() {
                        "name" => {
                            if let Some(v) = a.get("val") {
                                current_font.font = v.clone();
                            }
                        }
                        "sz" => {
                            if let Some(v) = a.get("val").and_then(|s| s.parse().ok()) {
                                current_font.size = v;
                            }
                        }
                        "b" => current_font.bold = a.get("val").is_none_or(|s| s != "0"),
                        "i" => current_font.italic = a.get("val").is_none_or(|s| s != "0"),
                        "u" => {
                            current_font.underline =
                                a.get("val").is_none_or(|s| s != "none" && s != "0")
                        }
                        "strike" => current_font.strike = a.get("val").is_none_or(|s| s != "0"),
                        "color" => {
                            if let Some(color) = rgb(&a) {
                                current_font.color = color;
                            }
                        }
                        _ => (),
                    }
                }
                if section == "fills" && tag == "fgColor" {
                    current_fill = rgb(&a);
                }
                if tag == "numFmt"
                    && let (Some(id), Some(code)) = (
                        a.get("numFmtId").and_then(|s| s.parse::<u32>().ok()),
                        a.get("formatCode"),
                    )
                {
                    num_formats.insert(id, code.clone());
                }
                if section == "cellXfs" && tag == "xf" {
                    let font = a
                        .get("fontId")
                        .and_then(|s| s.parse::<usize>().ok())
                        .and_then(|i| fonts.get(i))
                        .cloned()
                        .unwrap_or_default();
                    let mut fmt: CellFormat = font;
                    let id = a
                        .get("numFmtId")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0);
                    fmt.number = num_formats.get(&id).cloned().unwrap_or_else(|| builtin(id));
                    fmt.fill = a
                        .get("fillId")
                        .and_then(|s| s.parse::<usize>().ok())
                        .and_then(|i| fills.get(i))
                        .copied()
                        .flatten();
                    fmt.border = a.get("borderId").is_some_and(|s| s != "0");
                    formats.push(fmt);
                }
                if section == "cellXfs"
                    && tag == "alignment"
                    && let Some(fmt) = formats.last_mut()
                {
                    fmt.align = match a.get("horizontal").map(String::as_str) {
                        Some("left") => Align::Left,
                        Some("center") => Align::Center,
                        Some("right") => Align::Right,
                        _ => Align::General,
                    };
                    fmt.wrap = a.get("wrapText").is_some_and(|s| s == "1");
                    fmt.rotation = a
                        .get("textRotation")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
            }
            Event::End(e) => {
                let tag = e.local_name().as_ref().to_string();
                if tag == "font" {
                    fonts.push(current_font.clone());
                }
                if tag == "fill" {
                    fills.push(current_fill);
                }
                if tag == section {
                    section.clear();
                }
            }
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(formats)
}
fn read_metadata(book: &mut Book, parts: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let formats = parts
        .get("xl/styles.xml")
        .map(|b| styles(b))
        .transpose()?
        .unwrap_or_default();
    let mut targets = BTreeMap::new();
    if let Some(bytes) = parts.get("xl/_rels/workbook.xml.rels") {
        each_tag(bytes, |tag, a| {
            if tag == "Relationship"
                && let (Some(id), Some(target)) = (a.get("Id"), a.get("Target"))
            {
                targets.insert(
                    id.clone(),
                    if target.starts_with('/') {
                        target.trim_start_matches('/').into()
                    } else {
                        format!("xl/{target}")
                    },
                );
            }
            Ok(())
        })?;
    }
    let mut paths = Vec::new();
    if let Some(bytes) = parts.get("xl/workbook.xml") {
        each_tag(bytes, |tag, a| {
            if tag == "sheet"
                && let (Some(name), Some(id)) = (a.get("name"), a.get("r:id"))
                && let Some(path) = targets.get(id)
            {
                paths.push((
                    name.clone(),
                    path.clone(),
                    a.get("state")
                        .is_some_and(|s| s == "hidden" || s == "veryHidden"),
                ));
            }
            Ok(())
        })?;
    }
    for (name, path, hidden) in paths {
        let Some(id) = book.sheets.iter().find(|s| s.name == name).map(|s| s.id) else {
            continue;
        };
        if let Some(s) = book.sheet_by_id_mut(id) {
            s.hidden = hidden;
        }
        read_sheet_images(book, id, &path, parts);
        let Some(bytes) = parts.get(&path) else {
            continue;
        };
        each_tag(bytes, |tag, a| {
            if tag == "c"
                && let Some((r, c)) = a.get("r").and_then(|s| parse_address(s))
            {
                let index = a
                    .get("s")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                if let Some(fmt) = formats.get(index) {
                    if book.sheet_by_id(id).and_then(|s| s.cell(r, c)).is_none() {
                        book.set_input(id, r, c, "")?;
                    }
                    if let Some(cell) = book.sheet_by_id_mut(id).and_then(|s| s.cell_mut(r, c)) {
                        cell.fmt = fmt.clone();
                    }
                }
            }
            let s = book.sheet_by_id_mut(id).ok_or("Лист не найден")?;
            match tag {
                "col" => {
                    let min = a
                        .get("min")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(1)
                        .saturating_sub(1);
                    let max = a
                        .get("max")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(1)
                        .saturating_sub(1);
                    if max >= super::model::MAX_COLS || min > max {
                        return Err("Неверный диапазон столбцов".into());
                    }
                    for c in min..=max {
                        if let Some(w) = a
                            .get("width")
                            .and_then(|v| v.parse::<f32>().ok())
                            .filter(|w| w.is_finite() && *w > 0.0)
                        {
                            s.widths.insert(c, (w * 7.0 + 5.0).clamp(12.0, 1800.0));
                        }
                        if a.get("hidden").is_some_and(|v| v == "1") {
                            s.hidden_cols.insert(c);
                        }
                    }
                }
                "row" => {
                    let r = a
                        .get("r")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(1)
                        .saturating_sub(1);
                    if r >= super::model::MAX_ROWS {
                        return Err("Неверный номер строки".into());
                    }
                    if let Some(h) = a
                        .get("ht")
                        .and_then(|v| v.parse::<f32>().ok())
                        .filter(|h| h.is_finite() && *h > 0.0)
                    {
                        s.row_heights
                            .insert(r, (h * 96.0 / 72.0).clamp(12.0, 600.0));
                    }
                    if a.get("hidden").is_some_and(|v| v == "1") {
                        s.hidden_rows.insert(r);
                    }
                }
                "mergeCell" => {
                    if let Some(r) = a.get("ref").and_then(|v| parse_range(v).ok()) {
                        s.merges.push(r);
                    }
                }
                "pane" => {
                    if a.get("state").is_some_and(|v| v.starts_with("frozen")) {
                        s.frozen_rows = a
                            .get("ySplit")
                            .and_then(|v| v.parse::<f32>().ok())
                            .unwrap_or(0.0) as u32;
                        s.frozen_cols = a
                            .get("xSplit")
                            .and_then(|v| v.parse::<f32>().ok())
                            .unwrap_or(0.0) as u32;
                    }
                }
                "tabColor" => s.tab_color = rgb(a),
                "autoFilter" => s.filter = a.get("ref").and_then(|v| parse_range(v).ok()),
                _ => (),
            }
            Ok(())
        })?;
    }
    Ok(())
}
fn read_charts(book: &mut Book, parts: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for (name, bytes) in parts {
        if !name.starts_with("xl/charts/chart") || !name.ends_with(".xml") {
            continue;
        }
        let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
        let mut kind = ChartKind::Column;
        let mut refs = Vec::new();
        let mut title = String::new();
        let mut in_title = false;
        loop {
            match reader.read_event().map_err(|e| e.to_string())? {
                Event::Start(e) | Event::Empty(e) => {
                    let tag = local(&e);
                    match tag.as_str() {
                        "title" => in_title = true,
                        "barChart" => kind = ChartKind::Column,
                        "lineChart" => kind = ChartKind::Line,
                        "areaChart" => kind = ChartKind::Area,
                        "pieChart" => kind = ChartKind::Pie,
                        "doughnutChart" => kind = ChartKind::Doughnut,
                        "scatterChart" => kind = ChartKind::Scatter,
                        "barDir" => {
                            if attrs(&e, &reader)?.get("val").is_some_and(|v| v == "bar") {
                                kind = ChartKind::Bar;
                            }
                        }
                        "f" => {
                            let text = reader
                                .read_text(e.name())
                                .map_err(|e| e.to_string())?
                                .xml10_content()
                                .into_owned();
                            let text = quick_xml::escape::unescape(&text)
                                .map_err(|e| e.to_string())?
                                .into_owned();
                            refs.extend(super::formula::dependencies(&format!("={text}")));
                        }
                        "t" if in_title => {
                            let text = reader
                                .read_text(e.name())
                                .map_err(|e| e.to_string())?
                                .xml10_content()
                                .into_owned();
                            title.push_str(
                                &quick_xml::escape::unescape(&text).map_err(|e| e.to_string())?,
                            );
                        }
                        _ => (),
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == "title" {
                        in_title = false;
                    }
                }
                Event::DocType(_) => return Err("DTD запрещён".into()),
                Event::Eof => break,
                _ => (),
            }
        }
        if let Some((Some(sheet), first)) = refs.first()
            && let Some(id) = book
                .sheets
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(sheet))
                .map(|s| s.id)
        {
            let mut range = *first;
            for (_, r) in &refs {
                range.r0 = range.r0.min(r.r0);
                range.c0 = range.c0.min(r.c0);
                range.r1 = range.r1.max(r.r1);
                range.c1 = range.c1.max(r.c1);
            }
            if range.validate().is_ok() {
                book.add_chart(id, kind, range, &title)?;
            }
        }
    }
    Ok(())
}
/// `xl/worksheets/sheet1.xml` → `xl/worksheets/_rels/sheet1.xml.rels`.
fn rels_path(part: &str) -> String {
    match part.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{part}.rels"),
    }
}
/// Resolves a relationship target relative to the part that owns it.
fn resolve_target(part: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.into();
    }
    let mut segments: Vec<&str> = part.split('/').collect();
    segments.pop();
    for segment in target.split('/') {
        match segment {
            ".." => {
                segments.pop();
            }
            "." | "" => {}
            other => segments.push(other),
        }
    }
    segments.join("/")
}
fn relationships(
    part: &str,
    parts: &BTreeMap<String, Vec<u8>>,
) -> BTreeMap<String, (String, String)> {
    let mut rels = BTreeMap::new();
    if let Some(bytes) = parts.get(&rels_path(part)) {
        let _ = each_tag(bytes, |tag, a| {
            if tag == "Relationship"
                && let (Some(id), Some(target)) = (a.get("Id"), a.get("Target"))
            {
                rels.insert(
                    id.clone(),
                    (
                        a.get("Type").cloned().unwrap_or_default(),
                        resolve_target(part, target),
                    ),
                );
            }
            Ok(())
        });
    }
    rels
}
/// Pictures from the sheet's drawing part. Best effort: a broken drawing never blocks opening.
fn read_sheet_images(
    book: &mut Book,
    id: u64,
    sheet_path: &str,
    parts: &BTreeMap<String, Vec<u8>>,
) {
    for (kind, drawing) in relationships(sheet_path, parts).into_values() {
        if !kind.ends_with("/drawing") {
            continue;
        }
        let media = relationships(&drawing, parts);
        let Some(Ok(text)) = parts.get(&drawing).map(|b| std::str::from_utf8(b)) else {
            continue;
        };
        for anchor in drawing_pictures(text).unwrap_or_default() {
            let Some(bytes) = media
                .get(&anchor.embed)
                .and_then(|(_, target)| parts.get(target))
            else {
                continue;
            };
            if let Ok(image_id) = book.add_image(id, anchor.row, anchor.col, bytes.clone())
                && let Some(image) = book
                    .sheet_by_id_mut(id)
                    .and_then(|s| s.images.iter_mut().find(|i| i.id == image_id))
            {
                image.x_offset = (anchor.col_off as f32 / 9525.0).clamp(0.0, 4000.0);
                image.y_offset = (anchor.row_off as f32 / 9525.0).clamp(0.0, 4000.0);
                if anchor.cx > 0 && anchor.cy > 0 {
                    image.width = (anchor.cx as f32 / 9525.0).clamp(1.0, 8000.0);
                    image.height = (anchor.cy as f32 / 9525.0).clamp(1.0, 8000.0);
                }
                image.alt = anchor.alt;
            }
        }
    }
}
#[derive(Default)]
struct PictureAnchor {
    row: u32,
    col: u32,
    row_off: u64,
    col_off: u64,
    cx: u64,
    cy: u64,
    embed: String,
    alt: String,
}
fn drawing_pictures(text: &str) -> Result<Vec<PictureAnchor>, String> {
    let mut reader = Reader::from_str(text);
    let mut pictures = Vec::new();
    let mut current: Option<PictureAnchor> = None;
    let mut in_from = false;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let tag = local(&e);
                match tag.as_str() {
                    "oneCellAnchor" | "twoCellAnchor" | "absoluteAnchor" => {
                        current = Some(PictureAnchor::default());
                    }
                    "from" => in_from = true,
                    "row" | "col" | "rowOff" | "colOff" if in_from => {
                        let value = reader
                            .read_text(e.name())
                            .map_err(|e| e.to_string())?
                            .trim()
                            .parse::<u64>()
                            .unwrap_or(0);
                        if let Some(anchor) = current.as_mut() {
                            match tag.as_str() {
                                "row" => anchor.row = u32::try_from(value).unwrap_or(0),
                                "col" => anchor.col = u32::try_from(value).unwrap_or(0),
                                "rowOff" => anchor.row_off = value,
                                _ => anchor.col_off = value,
                            }
                        }
                    }
                    _ => picture_attrs(&e, &reader, &tag, current.as_mut())?,
                }
            }
            Event::Empty(e) => picture_attrs(&e, &reader, &local(&e), current.as_mut())?,
            Event::End(e) => match e.local_name().as_ref() {
                "from" => in_from = false,
                "oneCellAnchor" | "twoCellAnchor" | "absoluteAnchor" => {
                    if let Some(anchor) = current.take()
                        && !anchor.embed.is_empty()
                    {
                        pictures.push(anchor);
                    }
                }
                _ => {}
            },
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(pictures)
}
fn picture_attrs(
    e: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
    tag: &str,
    anchor: Option<&mut PictureAnchor>,
) -> Result<(), String> {
    let Some(anchor) = anchor else {
        return Ok(());
    };
    match tag {
        "ext" if anchor.cx == 0 => {
            let a = attrs(e, reader)?;
            anchor.cx = a.get("cx").and_then(|v| v.parse().ok()).unwrap_or(0);
            anchor.cy = a.get("cy").and_then(|v| v.parse().ok()).unwrap_or(0);
        }
        "blip" => {
            if let Some(embed) = attrs(e, reader)?.get("r:embed") {
                anchor.embed = embed.clone();
            }
        }
        "cNvPr" => {
            if let Some(descr) = attrs(e, reader)?.get("descr") {
                anchor.alt = descr.clone();
            }
        }
        _ => {}
    }
    Ok(())
}
fn xlsx_format(fmt: &CellFormat) -> Format {
    let mut f = Format::new()
        .set_font_name(&fmt.font)
        .set_font_size(f64::from(fmt.size.clamp(1.0, 409.0)))
        .set_font_color(Color::RGB(rgb_number(fmt.color)))
        .set_num_format(&fmt.number);
    if fmt.bold {
        f = f.set_bold();
    }
    if fmt.italic {
        f = f.set_italic();
    }
    if fmt.underline {
        f = f.set_underline(FormatUnderline::Single);
    }
    if fmt.strike {
        f = f.set_font_strikethrough();
    }
    if fmt.wrap {
        f = f.set_text_wrap();
    }
    if fmt.rotation != 0 {
        f = f.set_rotation(fmt.rotation.clamp(-90, 90));
    }
    if let Some(color) = fmt.fill {
        f = f.set_background_color(Color::RGB(rgb_number(color)));
    }
    if fmt.border {
        f = f.set_border(FormatBorder::Thin);
    }
    match fmt.align {
        Align::Left => f.set_align(FormatAlign::Left),
        Align::Center => f.set_align(FormatAlign::Center),
        Align::Right => f.set_align(FormatAlign::Right),
        Align::General => f,
    }
}
fn rgb_number(v: [u8; 3]) -> u32 {
    u32::from(v[0]) * 65536 + u32::from(v[1]) * 256 + u32::from(v[2])
}
fn write_xlsx(book: &Book) -> Result<Vec<u8>, String> {
    if book.sheets.is_empty() {
        return Err("Книга не содержит листов".into());
    }
    let mut workbook = Workbook::new();
    for (name, value) in &book.names {
        workbook
            .define_name(name, value)
            .map_err(|e| e.to_string())?;
    }
    for (index, sheet) in book.sheets.iter().enumerate() {
        let ws = workbook.add_worksheet();
        ws.set_name(&sheet.name).map_err(|e| e.to_string())?;
        ws.set_active(index == book.active);
        ws.set_hidden(sheet.hidden);
        if let Some(color) = sheet.tab_color {
            ws.set_tab_color(Color::RGB(rgb_number(color)));
        }
        ws.set_freeze_panes(sheet.frozen_rows, sheet.frozen_cols as u16)
            .map_err(|e| e.to_string())?;
        for (col, width) in &sheet.widths {
            ws.set_column_width_pixels(*col as u16, width.clamp(12.0, 1800.0) as u32)
                .map_err(|e| e.to_string())?;
        }
        for (row, height) in &sheet.row_heights {
            ws.set_row_height_pixels(*row, height.clamp(12.0, 600.0) as u32)
                .map_err(|e| e.to_string())?;
        }
        for row in &sheet.hidden_rows {
            ws.set_row_hidden(*row).map_err(|e| e.to_string())?;
        }
        for col in &sheet.hidden_cols {
            ws.set_column_hidden(*col as u16)
                .map_err(|e| e.to_string())?;
        }
        if let Some(range) = sheet.filter {
            ws.autofilter(range.r0, range.c0 as u16, range.r1, range.c1 as u16)
                .map_err(|e| e.to_string())?;
        }
        for range in &sheet.merges {
            let text = sheet
                .cell(range.r0, range.c0)
                .map(|c| c.value.display())
                .unwrap_or_default();
            let fmt = sheet
                .cell(range.r0, range.c0)
                .map(|c| xlsx_format(&c.fmt))
                .unwrap_or_default();
            ws.merge_range(
                range.r0,
                range.c0 as u16,
                range.r1,
                range.c1 as u16,
                &text,
                &fmt,
            )
            .map_err(|e| e.to_string())?;
        }
        for (key, cell) in &sheet.cells {
            let (row, col) = parse_address(key).ok_or("Недопустимый адрес ячейки")?;
            let col = col as u16;
            let fmt = xlsx_format(&cell.fmt);
            if cell.input.starts_with('=') {
                ws.write_formula_with_format(
                    row,
                    col,
                    Formula::new(&cell.input).set_result(cell.value.display()),
                    &fmt,
                )
                .map_err(|e| e.to_string())?;
            } else {
                match &cell.value {
                    Value::Number(n) => {
                        ws.write_number_with_format(row, col, *n, &fmt)
                            .map_err(|e| e.to_string())?;
                    }
                    Value::Text(s) => {
                        ws.write_string_with_format(row, col, s, &fmt)
                            .map_err(|e| e.to_string())?;
                    }
                    Value::Bool(b) => {
                        ws.write_boolean_with_format(row, col, *b, &fmt)
                            .map_err(|e| e.to_string())?;
                    }
                    Value::Error(e) => {
                        ws.write_formula_with_format(row, col, Formula::new(e).set_result(e), &fmt)
                            .map_err(|e| e.to_string())?;
                    }
                    Value::Empty => {
                        ws.write_blank(row, col, &fmt).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        for model in &sheet.charts {
            let mut chart = Chart::new(match model.kind {
                ChartKind::Column => ChartType::Column,
                ChartKind::Bar => ChartType::Bar,
                ChartKind::Line => ChartType::Line,
                ChartKind::Area => ChartType::Area,
                ChartKind::Pie => ChartType::Pie,
                ChartKind::Doughnut => ChartType::Doughnut,
                ChartKind::Scatter => ChartType::Scatter,
            });
            chart.title().set_name(&model.title);
            chart
                .set_width(model.width.clamp(80.0, 4000.0) as u32)
                .set_height(model.height.clamp(80.0, 4000.0) as u32);
            let r = model.range;
            let data_start = if r.r1 > r.r0 { r.r0 + 1 } else { r.r0 };
            let start_col = if r.c1 > r.c0 { r.c0 + 1 } else { r.c0 };
            for c in start_col..=r.c1 {
                let series = chart.add_series();
                series.set_values((sheet.name.as_str(), data_start, c as u16, r.r1, c as u16));
                if r.c1 > r.c0 {
                    series.set_categories((
                        sheet.name.as_str(),
                        data_start,
                        r.c0 as u16,
                        r.r1,
                        r.c0 as u16,
                    ));
                }
                if data_start > r.r0 {
                    series.set_name((sheet.name.as_str(), r.r0, c as u16));
                }
                if matches!(model.kind, ChartKind::Pie | ChartKind::Doughnut) {
                    break;
                }
            }
            ws.insert_chart(model.row, model.col as u16, &chart)
                .map_err(|e| e.to_string())?;
        }
        for model in &sheet.images {
            let mut image = Image::new_from_buffer(&model.bytes).map_err(|e| e.to_string())?;
            let (w, h) = (image.width().max(1.0), image.height().max(1.0));
            image = image
                .set_scale_width(f64::from(model.width) / w)
                .set_scale_height(f64::from(model.height) / h);
            if !model.alt.is_empty() {
                image = image.set_alt_text(&model.alt);
            }
            ws.insert_image_with_offset(
                model.row,
                model.col as u16,
                &image,
                model.x_offset.max(0.0) as u32,
                model.y_offset.max(0.0) as u32,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    workbook.save_to_buffer().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::super::model::Range;
    use super::*;
    #[test]
    fn csv_roundtrip() {
        let rows = vec![
            vec![
                "a;b".into(),
                "a\"b".into(),
                "line\r\nnext".into(),
                "".into(),
            ],
            vec!["ёж".into(), "2,5".into()],
        ];
        assert_eq!(parse_csv(&to_csv(&rows, ';'), ';'), Ok(rows));
        assert!(parse_csv("\"unclosed", ',').is_err());
        assert!(parse_csv("\"x\"z", ',').is_err());
        assert_eq!(
            parse_csv("a,b,\r\n", ','),
            Ok(vec![vec!["a".into(), "b".into(), "".into()]])
        );
    }
    #[test]
    fn xlsx_roundtrip() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_range(
            id,
            0,
            0,
            &[
                vec!["Товар".into(), "Цена".into()],
                vec!["Яблоко".into(), "12.5".into()],
                vec!["Итог".into(), "=SUM(B2,2.5)".into()],
            ],
        )?;
        let fmt = CellFormat {
            bold: true,
            number: "0.00".into(),
            fill: Some([255, 220, 100]),
            ..CellFormat::default()
        };
        book.set_format(id, Range::new(2, 1, 2, 1), fmt.clone())?;
        book.add_chart(id, ChartKind::Column, Range::new(0, 0, 2, 1), "Цены")?;
        let bytes = write(&book, "xlsx")?;
        let restored = read(&bytes, "xlsx")?;
        let sheet = restored.active_sheet().ok_or("Нет листа")?;
        assert_eq!(
            sheet.cell(2, 1).map(|c| &c.value),
            Some(&Value::Number(15.0))
        );
        assert_eq!(sheet.cell(2, 1).map(|c| &c.fmt), Some(&fmt));
        assert_eq!(sheet.charts.len(), 1);
        let again = read(&write(&restored, "xlsx")?, "xlsx")?;
        let (a, b) = (
            restored.active_sheet().ok_or("Нет листа")?,
            again.active_sheet().ok_or("Нет листа")?,
        );
        assert_eq!(a.cells, b.cells, "второе поколение xlsx отличается");
        assert_eq!(a.charts.len(), b.charts.len());
        Ok(())
    }
    #[test]
    fn xlsx_pictures_round_trip_with_position_and_size() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(40, 20)
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let image = book.add_image(id, 3, 2, png.into_inner())?;
        if let Some(i) = book
            .sheet_by_id_mut(id)
            .and_then(|s| s.images.iter_mut().find(|i| i.id == image))
        {
            i.width = 160.0;
            i.height = 80.0;
            i.x_offset = 10.0;
            i.alt = "Логотип".into();
        }
        let restored = read(&write(&book, "xlsx")?, "xlsx")?;
        let sheet = restored.active_sheet().ok_or("Нет листа")?;
        assert_eq!(sheet.images.len(), 1);
        let i = sheet.images.first().ok_or("Нет рисунка")?;
        assert_eq!((i.row, i.col), (3, 2));
        assert_eq!((i.width.round(), i.height.round()), (160.0, 80.0));
        assert_eq!(i.x_offset.round(), 10.0);
        assert_eq!(i.alt, "Логотип");
        Ok(())
    }
    #[test]
    fn preflight_rejects_custom_path_dense_bomb() -> Result<(), String> {
        let bytes=crate::package::write_zip(vec![("custom/sheet.xml".into(),b"<worksheet><dimension ref=\"A1:XFD1048576\"/><sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row><row r=\"1048576\"><c r=\"XFD1048576\"><v>2</v></c></row></sheetData></worksheet>".to_vec())])?;
        let error = read(&bytes, "xlsx")
            .err()
            .ok_or("Expected preflight error")?;
        assert!(error.contains("dimensions") || error.contains("разреженный"));
        Ok(())
    }
    #[test]
    fn ods_repeat_product_and_text_bombs_are_rejected() {
        for xml in [
            "<document><table><table-row number-rows-repeated=\"1048576\"><table-cell number-columns-repeated=\"16384\"/></table-row></table></document>",
            "<document><table><table-row><table-cell><p><s c=\"18446744073709551615\"/></p></table-cell></table-row></table></document>",
            "<document><table><table-row number-rows-repeated=\"100000\"><table-cell number-columns-repeated=\"10\" string-value=\"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz\"/></table-row></table></document>",
        ] {
            assert!(ods_preflight(xml.as_bytes()).is_err(), "{xml}");
        }
        assert!(
            ods_preflight(
                b"<document><table><table-row><table-cell/></table-row></table></document>"
            )
            .is_ok()
        );
    }
    #[test]
    fn legacy_guard_is_explicit() {
        assert!(read_excel(&[0xd0, 0xcf, 0x11, 0xe0]).is_err());
        let mut parts = BTreeMap::new();
        parts.insert("xl/workbook.bin".into(), Vec::new());
        assert!(preflight(&parts).is_err());
    }

    #[test]
    fn shared_indices_and_reserves_are_bounded() {
        for xml in [
            "<sst count=\"1\" uniqueCount=\"18446744073709551615\"><si><t>x</t></si></sst>",
            "<worksheet><sheetData><row r=\"1\"><c r=\"A1\"><f t=\"shared\" si=\"18446744073709551615\" ref=\"A1:A2\">A2</f></c></row></sheetData></worksheet>",
            "<worksheet><sheetData><row><c><f t=\"shared\" si=\"1\" ref=\"A1:XFD1048576\">A2</f></c></row></sheetData></worksheet>",
            "<workbook><workbookPr date1904=\"1\"/></workbook>",
        ] {
            let mut parts = BTreeMap::new();
            parts.insert("custom.xml".into(), xml.as_bytes().to_vec());
            assert!(preflight(&parts).is_err(), "{xml}");
        }
    }
    #[test]
    fn implicit_rows_and_expanded_shared_strings_are_bounded() {
        assert_eq!(worksheet_bounds(b"<worksheet><sheetData><row><c><v>1</v></c></row><row><c><v>2</v></c></row></sheetData></worksheet>",&[]),Ok((2,0)));
        let mut xml = String::from("<worksheet><sheetData>");
        for _ in 0..2100 {
            xml.push_str("<row><c t=\"s\"><v>0</v></c></row>");
        }
        xml.push_str("</sheetData></worksheet>");
        assert!(worksheet_bounds(xml.as_bytes(), &[32767]).is_err());
        assert!(worksheet_bounds(b"<worksheet><sheetData><row><c><f t=\"shared\" si=\"0\"/></c></row></sheetData></worksheet>",&[]).is_err());
    }
}
