use super::model::{
    Align, Book, CellFormat, ChartKind, ConditionalRule, MAX_RANGE_CELLS, Value, parse_address, parse_range,
};
use calamine::{Data, Reader as _, open_workbook_auto_from_rs};
use quick_xml::{
    NsReader as Reader, Writer,
    events::{BytesRef, BytesStart, BytesText, Event},
};
use rust_xlsxwriter::{
    Chart, ChartType, Color, Format, FormatAlign, FormatBorder, FormatUnderline, Formula, Image,
    Workbook, ConditionalFormatCell, ConditionalFormatCellRule,
};
use std::{collections::BTreeMap, io::Cursor};

const MAX_FILE: usize = 64 * 1024 * 1024;
pub fn read(bytes: &[u8], ext: &str) -> Result<Book, String> {
    if bytes.len() > MAX_FILE {
        return Err("Файл превышает 64 МБ".into());
    }
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    match ext.as_str() {
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
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" => write_xlsx(book),
        "csv" | "tsv" => write_text_book(book, &ext, MAX_FILE),
        "html" => write_html_book(book,1_000_000,MAX_FILE),
        "md" => write_text_book(book, &ext, MAX_FILE),
        _ => Err(format!("Не поддерживается сохранение {ext}")),
    }
}
fn markdown_text(text: &str) -> String {
    let mut out = String::new();
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    for c in normalized.chars() {
        match c {
            '\n' => out.push_str("<br>"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if c.is_ascii_punctuation() => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}
pub fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if let Some(decoded)=crate::util::decode_utf32_bom(bytes) {return decoded;}
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
    if let Some(content)=bytes.strip_prefix(&[0xef,0xbb,0xbf]) {
        return std::str::from_utf8(content).map(str::to_owned).map_err(|error|format!("Некорректный UTF-8 после BOM: {error}"));
    }
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
    ['|', '\t', ';', ',']
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
        } else {
            // Like Excel, stray quotes and text after a closing quote are kept as text:
            // `5" pipe` and `"x"z` are common in pasted text.
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
                    if s.contains([separator, '\n', '\r', '"']) || row.len() == 1 && s.is_empty() {
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
fn attrs(e: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<BTreeMap<String, String>, String> {
    e.attributes()
        .map(|a| {
            let a = a.map_err(|e| e.to_string())?;
            let (namespace, local) = reader.resolver().resolve_attribute(a.key);
            let key = match namespace {
                quick_xml::name::ResolveResult::Bound(namespace)
                    if matches!(
                        namespace.as_ref(),
                        "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                            | "http://purl.oclc.org/ooxml/officeDocument/relationships"
                    ) =>
                {
                    format!("r:{}", local.as_ref())
                }
                _ => a.key.as_ref().to_string(),
            };
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
fn xml_reference(reference: &BytesRef<'_>) -> Result<char,String> {
    if let Some(character)=reference.resolve_char_ref().map_err(|error|error.to_string())? {return Ok(character);}
    match reference.as_ref() {
        "amp"=>Ok('&'),"lt"=>Ok('<'),"gt"=>Ok('>'),"quot"=>Ok('"'),"apos"=>Ok('\''),
        _=>Err("XLSX: неизвестная XML entity".into()),
    }
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
            Event::GeneralRef(reference) if in_string => length = length.saturating_add(xml_reference(&reference)?.len_utf8()),
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
    let mut inline_length = 0usize;
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
                        inline_length = 0;
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
                    inline_length = inline_length.saturating_add(size);
                }
                if in_formula {
                    formula_length = formula_length.saturating_add(size);
                }
            }
            Event::CData(t) => {
                if in_value {
                    if value.len().saturating_add(t.len())>32767 {return Err("XLSX: слишком длинное значение".into());}
                    value.push_str(&t);
                }
                if in_text {
                    text_bytes = text_bytes.saturating_add(t.len());
                    inline_length = inline_length.saturating_add(t.len());
                }
                if in_formula {
                    formula_length = formula_length.saturating_add(t.len());
                }
            }
            Event::GeneralRef(r) => {
                let ch=xml_reference(&r)?;
                if in_value {
                    value.push(ch);
                    if value.len() > 32767 {
                        return Err("XLSX: слишком длинное значение".into());
                    }
                }
                if in_text {
                    text_bytes = text_bytes.saturating_add(ch.len_utf8());
                    inline_length = inline_length.saturating_add(ch.len_utf8());
                }
                if in_formula {
                    formula_length = formula_length.saturating_add(ch.len_utf8());
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                "row" => {
                    row = row.checked_add(1).ok_or("XLSX: строка переполнена")?;
                    next_col = 0;
                }
                "v" => {
                    in_value = false;
                    if cell_type == "str" { text_bytes=text_bytes.saturating_add(value.len()); }
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
        if formula_length > 32767 || inline_length > 32767 || text_bytes > 64 * 1024 * 1024 {
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
        if name.to_ascii_lowercase().ends_with(".xml")
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
        if name.to_ascii_lowercase().ends_with(".xml") && worksheet_root(bytes)? {
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
    let mut date_1904 = false;
    if let Some(workbook) = parts
        .as_ref()
        .and_then(|parts| parts.get("xl/workbook.xml"))
    {
        each_tag(workbook, |tag, attributes| {
            if tag == "workbookPr" {
                date_1904 = xml_bool(attributes.get("date1904"), false);
            }
            Ok(())
        })?;
    }
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
                    Data::DateTime(d) => normalized_datetime(d, date_1904).to_string(),
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
fn normalized_datetime(value: &calamine::ExcelDateTime, date_1904: bool) -> f64 {
    value.as_f64()
        + if date_1904 && !value.is_duration() {
            1462.0
        } else {
            0.0
        }
}
fn xml_bool(value: Option<&String>, default: bool) -> bool {
    value.map_or(default, |value| {
        value == "1" || value.eq_ignore_ascii_case("true")
    })
}
fn rgb(a: &BTreeMap<String, String>) -> Option<[u8; 3]> {
    rgb_with_palette(a, &[])
}
fn rgb_with_palette(a: &BTreeMap<String, String>, palette: &[[u8; 3]]) -> Option<[u8; 3]> {
    rgb_with_theme(a,palette,&[])
}
fn rgb_with_theme(a: &BTreeMap<String,String>, palette: &[[u8;3]], theme: &[[u8;3]]) -> Option<[u8;3]> {
    let base = rgb_base(a,palette,theme)?;
    let tint = a.get("tint").and_then(|value| value.parse::<f64>().ok()).unwrap_or(0.0);
    Some(if tint.is_finite() && (-1.0..=1.0).contains(&tint) { tint_color(base,tint) } else { base })
}
fn rgb_base(a: &BTreeMap<String,String>, palette: &[[u8;3]], theme: &[[u8;3]]) -> Option<[u8;3]> {
    if let Some(index)=a.get("theme").and_then(|value| value.parse::<usize>().ok()) {
        return theme.get(index).copied();
    }
    if let Some(index) = a.get("indexed").and_then(|v| v.parse::<usize>().ok()) {
        const DEFAULT: [u32; 64] = [
            0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF,
            0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF,
            0x800000, 0x008000, 0x000080, 0x808000, 0x800080, 0x008080, 0xC0C0C0, 0x808080,
            0x9999FF, 0x993366, 0xFFFFCC, 0xCCFFFF, 0x660066, 0xFF8080, 0x0066CC, 0xCCCCFF,
            0x000080, 0xFF00FF, 0xFFFF00, 0x00FFFF, 0x800080, 0x800000, 0x008080, 0x0000FF,
            0x00CCFF, 0xCCFFFF, 0xCCFFCC, 0xFFFF99, 0x99CCFF, 0xFF99CC, 0xCC99FF, 0xFFCC99,
            0x3366FF, 0x33CCCC, 0x99CC00, 0xFFCC00, 0xFF9900, 0xFF6600, 0x666699, 0x969696,
            0x003366, 0x339966, 0x003300, 0x333300, 0x993300, 0x993366, 0x333399, 0x333333,
        ];
        if !palette.is_empty() {
            return palette.get(index).copied();
        }
        let colour = *DEFAULT.get(index)?;
        return Some([(colour >> 16) as u8, (colour >> 8) as u8, colour as u8]);
    }
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
        // Built-in ids 5-8 and 37-40 are locale currency/accounting formats.
        5 | 37 => "#,##0 ;(#,##0)",
        6 | 38 => "#,##0 ;[Red](#,##0)",
        7 | 39 => "#,##0.00;(#,##0.00)",
        8 | 40 => "#,##0.00;[Red](#,##0.00)",
        // Id 14 is the system short date; TrineDocs follows the Russian locale.
        14 => "dd.mm.yyyy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "dd.mm.yyyy h:mm",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mm:ss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => "General",
    }
    .into()
}
fn indexed_palette(bytes: &[u8]) -> Result<Vec<[u8; 3]>, String> {
    let mut palette = Vec::new();
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let mut inside = false;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) | Event::Empty(e) if local(&e) == "indexedColors" => inside = true,
            Event::Start(e) | Event::Empty(e) if inside && local(&e) == "rgbColor" => {
                let colour = rgb(&attrs(&e, &reader)?).ok_or("Недопустимая indexedColor")?;
                palette.push(colour);
                if palette.len() > 64 {
                    return Err("Слишком много indexedColors".into());
                }
            }
            Event::End(e) if e.local_name().as_ref() == "indexedColors" => inside = false,
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => break,
            _ => (),
        }
    }
    Ok(palette)
}
#[cfg(test)]
fn styles(bytes: &[u8]) -> Result<Vec<CellFormat>, String> {
    styles_with_theme(bytes,&[])
}
#[cfg(test)]
fn styles_with_theme(bytes: &[u8],theme:&[[u8;3]])->Result<Vec<CellFormat>,String> {
    styles_with_theme_fonts(bytes,theme,&BTreeMap::new())
}
fn styles_with_theme_fonts(bytes: &[u8],theme:&[[u8;3]],faces:&BTreeMap<String,String>)->Result<Vec<CellFormat>,String> {
    let palette = indexed_palette(bytes)?;
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let mut section = String::new();
    let mut fonts = Vec::new();
    let mut fills = Vec::new();
    let mut borders = Vec::new();
    let mut current_border = false;
    let mut current_fill_active = false;
    let mut formats = Vec::new();
    let mut base_formats:Vec<CellFormat> = Vec::new();
    let mut font_scheme=String::new();
    let mut num_formats = BTreeMap::new();
    let mut current_font = CellFormat::default();
    let mut current_fill = None;
    let mut apply_style_alignment = true;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let tag = local(&e);
                let a = attrs(&e, &reader)?;
                // `<font/>` and `<fill/>` have no end tag but still take an index.
                if empty && tag == "font" && section == "fonts" {
                    fonts.push(CellFormat::default());
                }
                if empty && tag == "fill" && section == "fills" {
                    fills.push(None);
                }
                if matches!(
                    tag.as_str(),
                    "fonts" | "fills" | "borders" | "cellXfs" | "cellStyleXfs"
                ) {
                    section = tag.clone();
                }
                if tag == "font" {
                    font_scheme.clear();
                    current_font = CellFormat::default();
                }
                if tag == "fill" {
                    current_fill = None;
                    current_fill_active = false;
                }
                if tag == "border" {
                    current_border = false;
                    if empty {
                        borders.push(false);
                    }
                }
                if section == "borders"
                    && matches!(
                        tag.as_str(),
                        "left" | "right" | "top" | "bottom" | "diagonal" | "start" | "end"
                    )
                {
                    current_border |= a
                        .get("style")
                        .is_some_and(|style| !style.is_empty() && style != "none");
                }
                if section == "fills" && tag == "patternFill" {
                    current_fill_active = a
                        .get("patternType")
                        .is_some_and(|pattern| pattern != "none");
                }
                if section == "fonts" {
                    match tag.as_str() {
                        "scheme"=>font_scheme=a.get("val").cloned().unwrap_or_default(),
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
                        "b" => current_font.bold = xml_bool(a.get("val"), true),
                        "i" => current_font.italic = xml_bool(a.get("val"), true),
                        "u" => {
                            current_font.underline =
                                a.get("val").is_none_or(|s| s != "none" && s != "0")
                        }
                        "strike" => current_font.strike = xml_bool(a.get("val"), true),
                        "color" => {
                            if let Some(color) = rgb_with_theme(&a, &palette,theme) {
                                current_font.color = color;
                            }
                        }
                        _ => (),
                    }
                }
                if section == "fills" && tag == "fgColor" && current_fill_active {
                    current_fill = rgb_with_theme(&a, &palette,theme);
                }
                if tag == "numFmt"
                    && let (Some(id), Some(code)) = (
                        a.get("numFmtId").and_then(|s| s.parse::<u32>().ok()),
                        a.get("formatCode"),
                    )
                {
                    num_formats.insert(id, code.clone());
                }
                if matches!(section.as_str(),"cellXfs"|"cellStyleXfs") && tag=="xf" {
                    let mut fmt=if section=="cellXfs" {
                        a.get("xfId").and_then(|value|value.parse::<usize>().ok())
                            .and_then(|index|base_formats.get(index)).cloned().unwrap_or_default()
                    }else{CellFormat::default()};
                    apply_style_alignment = section != "cellStyleXfs" || xml_bool(a.get("applyAlignment"), true);
                    if (section != "cellStyleXfs" || xml_bool(a.get("applyFont"), true))
                        && let Some(font)=a.get("fontId").and_then(|v|v.parse::<usize>().ok()).and_then(|i|fonts.get(i)) {
                        fmt.font=font.font.clone();fmt.size=font.size;fmt.bold=font.bold;fmt.italic=font.italic;
                        fmt.underline=font.underline;fmt.strike=font.strike;fmt.color=font.color;
                    }
                    if (section != "cellStyleXfs" || xml_bool(a.get("applyNumberFormat"), true))
                        && let Some(id)=a.get("numFmtId").and_then(|v|v.parse::<u32>().ok()) {
                        fmt.number=num_formats.get(&id).cloned().unwrap_or_else(||builtin(id));
                    }
                    if (section != "cellStyleXfs" || xml_bool(a.get("applyFill"), true))
                        && let Some(fill)=a.get("fillId").and_then(|v|v.parse::<usize>().ok()).and_then(|i|fills.get(i)) {fmt.fill=*fill;}
                    if (section != "cellStyleXfs" || xml_bool(a.get("applyBorder"), true))
                        && let Some(border)=a.get("borderId").and_then(|v|v.parse::<usize>().ok()).and_then(|i|borders.get(i)) {fmt.border=*border;}
                    if section=="cellXfs" {formats.push(fmt);}else{base_formats.push(fmt);}
                }
                if matches!(section.as_str(),"cellXfs"|"cellStyleXfs") && tag=="alignment" && apply_style_alignment {
                    let target=if section=="cellXfs" {&mut formats}else{&mut base_formats};
                    if let Some(fmt)=target.last_mut() {
                    fmt.align = match a.get("horizontal").map(String::as_str) {
                        Some("left") => Align::Left,
                        Some("center") => Align::Center,
                        Some("right") => Align::Right,
                        _ => Align::General,
                    };
                    fmt.wrap = xml_bool(a.get("wrapText"), false);
                    let rotation: i16 = a
                        .get("textRotation")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    fmt.rotation = if (91..=180).contains(&rotation) {
                        90 - rotation
                    } else {
                        rotation
                    };
                    }
                }
            }
            Event::End(e) => {
                let tag = e.local_name().as_ref().to_string();
                if tag == "font" {
                    if let Some(face)=faces.get(&font_scheme) {current_font.font=face.clone();}
                    fonts.push(current_font.clone());
                }
                if tag == "fill" {
                    fills.push(current_fill);
                }
                if tag == "border" {
                    borders.push(current_border);
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
fn differential_fills(bytes:&[u8],palette:&[[u8;3]],theme:&[[u8;3]]) -> Result<Vec<Option<[u8;3]>>,String> {
    let mut reader=Reader::from_str(std::str::from_utf8(bytes).map_err(|error|error.to_string())?);
    let mut fills=Vec::new();
    let (mut inside,mut unsupported,mut foreground,mut background,mut solid)=(false,false,None,None,false);
    loop {
        let event=reader.read_event().map_err(|error|error.to_string())?;
        let empty=matches!(event,Event::Empty(_));
        match event {
            Event::Start(element)|Event::Empty(element) => {
                let tag=local(&element);
                if tag=="dxf" {
                    inside=!empty;unsupported=false;foreground=None;background=None;solid=false;
                    if empty {fills.push(None);}
                } else if inside {
                    let attributes=attrs(&element,&reader)?;
                    match tag.as_str() {
                        "font"|"border"|"numFmt"|"alignment"|"protection"|"gradientFill"=>unsupported=true,
                        "patternFill"=>{
                            solid=attributes.get("patternType").is_some_and(|value|value=="solid");
                            unsupported|=attributes.get("patternType").is_some_and(|value|!matches!(value.as_str(),"none"|"solid"));
                        }
                        "fgColor"=>foreground=rgb_with_theme(&attributes,palette,theme),
                        "bgColor"=>{
                            if attributes.get("indexed").is_none_or(|value|value!="64") {
                                background=rgb_with_theme(&attributes,palette,theme);
                            }
                        }
                        _=>(),
                    }
                }
            }
            Event::End(element) if element.local_name().as_ref()=="dxf" => {
                fills.push(if unsupported {None}else if solid {foreground.or(background)}else{background.or(foreground)});
                inside=false;
            }
            Event::DocType(_)=>return Err("DTD запрещён".into()),
            Event::Eof=>break,
            _=>(),
        }
        if fills.len()>65536 {return Err("Слишком много дифференциальных форматов".into());}
    }
    Ok(fills)
}
fn conditional_rules(bytes:&[u8],fills:&[Option<[u8;3]>]) -> Result<Vec<ConditionalRule>,String> {
    let mut reader=Reader::from_str(std::str::from_utf8(bytes).map_err(|error|error.to_string())?);
    let mut ranges=Vec::new();
    let mut current:Option<BTreeMap<String,String>>=None;
    let mut formulas=Vec::new();
    let mut result=Vec::new();
    loop {
        let event=reader.read_event().map_err(|error|error.to_string())?;
        let empty=matches!(event,Event::Empty(_));
        match event {
            Event::Start(element)|Event::Empty(element) => {
                let tag=local(&element);
                let attributes=attrs(&element,&reader)?;
                match tag.as_str() {
                    "conditionalFormatting"=>{
                        ranges=attributes.get("sqref").ok_or("Не поддерживается conditionalFormatting без sqref")?
                            .split_whitespace().map(parse_range).collect::<Result<Vec<_>,_>>()?;
                        if ranges.is_empty()||ranges.len()>10000 {return Err("Недопустимые области conditionalFormatting".into());}
                    }
                    "cfRule"=>{
                        if empty {return Err("Не поддерживается пустое правило conditionalFormatting".into());}
                        current=Some(attributes);formulas.clear();
                    }
                    "formula" if current.is_some()=>{
                        if empty {formulas.push(String::new());}
                        else {
                            let raw=reader.read_text(element.name()).map_err(|error|error.to_string())?;
                            formulas.push(quick_xml::escape::unescape(&raw).map_err(|error|error.to_string())?.into_owned());
                        }
                    }
                    _=>(),
                }
            }
            Event::End(element) if element.local_name().as_ref()=="cfRule" => {
                let attributes=current.take().ok_or("Нет начала правила conditionalFormatting")?;
                let operator=match attributes.get("operator").map(String::as_str) {
                    Some("greaterThan")=>">",Some("lessThan")=>"<",Some("equal")=>"=",
                    Some("greaterThanOrEqual")=>">=",Some("lessThanOrEqual")=>"<=",
                    _=>return Err("Не поддерживается оператор conditionalFormatting".into()),
                };
                if attributes.get("type").is_none_or(|kind|kind!="cellIs")||formulas.len()!=1 {
                    return Err("Поддерживаются только числовые cellIs правила условной заливки".into());
                }
                let threshold=formulas[0].trim().trim_start_matches('=').parse::<f64>()
                    .ok().filter(|value|value.is_finite()).ok_or("Не поддерживается формула порога conditionalFormatting")?;
                let index=attributes.get("dxfId").and_then(|value|value.parse::<usize>().ok())
                    .ok_or("Нет формата conditionalFormatting")?;
                let fill=fills.get(index).copied().flatten().ok_or("Не поддерживается формат conditionalFormatting: нужна только сплошная заливка")?;
                let priority=attributes.get("priority").and_then(|value|value.parse::<u32>().ok())
                    .filter(|value|*value>0).ok_or("Нет приоритета conditionalFormatting")?;
                for range in &ranges {
                    result.push((priority,ConditionalRule{range:*range,operator:operator.into(),threshold,fill}));
                    if result.len()>10000 {return Err("Слишком много правил conditionalFormatting".into());}
                }
            }
            Event::End(element) if element.local_name().as_ref()=="conditionalFormatting"=>ranges.clear(),
            Event::DocType(_)=>return Err("DTD запрещён".into()),
            Event::Eof=>break,
            _=>(),
        }
    }
    result.sort_by_key(|(priority,_)|*priority);
    Ok(result.into_iter().map(|(_,rule)|rule).collect())
}
fn read_metadata(book: &mut Book, parts: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let workbook_rels = relationships("xl/workbook.xml", parts);
    let style_path = workbook_rels
        .values()
        .find(|(kind, _)| kind.ends_with("/styles"))
        .map(|(_, target)| target.as_str())
        .unwrap_or("xl/styles.xml");
    let palette = parts
        .get(style_path)
        .map(|bytes| indexed_palette(bytes))
        .transpose()?
        .unwrap_or_default();
    let theme_path=workbook_rels.values().find(|(kind,_)| kind.ends_with("/theme")).map(|(_,path)| path.as_str());
    let theme=theme_path.and_then(|path| parts.get(path)).map(|bytes| theme_colors(bytes)).transpose()?.unwrap_or_default();
    let faces=theme_path.and_then(|path|parts.get(path)).map(|bytes|theme_font_faces(bytes)).transpose()?.unwrap_or_default();
    let formats = parts
        .get(style_path)
        .map(|b| styles_with_theme_fonts(b,&theme,&faces))
        .transpose()?
        .unwrap_or_default();
    let differential = parts.get(style_path).map(|bytes| differential_fills(bytes,&palette,&theme))
        .transpose()?.unwrap_or_default();
    let mut targets = BTreeMap::new();
    if let Some(bytes) = parts.get("xl/_rels/workbook.xml.rels") {
        each_tag(bytes, |tag, a| {
            if tag == "Relationship"
                && let (Some(id), Some(target)) = (a.get("Id"), a.get("Target"))
            {
                targets.insert(id.clone(), resolve_target("xl/workbook.xml", target));
            }
            Ok(())
        })?;
    }
    let mut paths = Vec::new();
    if let Some(bytes) = parts.get("xl/workbook.xml") {
        each_tag(bytes, |tag, a| {
            if tag == "workbookView"
                && let Some(active) = a
                    .get("activeTab")
                    .and_then(|value| value.parse::<usize>().ok())
                && active < book.sheets.len()
            {
                book.active = active;
            }
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
    let mut selections=BTreeMap::new();
    for (name, path, hidden) in paths {
        let Some(id) = book.sheets.iter().find(|s| s.name == name).map(|s| s.id) else {
            continue;
        };
        if let Some(s) = book.sheet_by_id_mut(id) {
            s.hidden = hidden;
            // Standard XLSX fallback differs from a newly created native sheet.
            // Explicit sheetFormatPr values below take precedence.
            s.default_col_width=64.0;
            s.default_row_height=20.0;
        }
        let Some(bytes) = parts.get(&path) else {
            continue;
        };
        let mut pane=String::from("topLeft");
        let mut sheet_selections=Vec::new();
        let mut row_styles=BTreeMap::new();
        let mut col_styles=BTreeMap::new();
        let (mut current_row,mut next_row,mut next_col)=(0u32,0u32,0u32);
        each_tag(bytes, |tag, a| {
            if tag=="pane" {pane=a.get("activePane").cloned().unwrap_or_else(||"topLeft".into());}
            if tag=="selection" && let Some(selection)=xlsx_selection(a) {
                sheet_selections.push((a.get("pane").map(String::as_str).unwrap_or("topLeft").to_string(),selection));
            }
            if tag == "row" {
                current_row=match a.get("r") {
                    Some(value) => value.parse::<u32>().ok().and_then(|v| v.checked_sub(1)).ok_or("Неверный номер строки")?,
                    None => next_row,
                };
                if current_row >= super::model::MAX_ROWS { return Err("Неверный номер строки".into()); }
                next_row=current_row+1;
                next_col=0;
            }
            if tag == "c" {
                let (r,c)=match a.get("r") {
                    Some(value) => parse_address(value).ok_or("Неверный адрес ячейки")?,
                    None => (current_row,next_col),
                };
                if c >= super::model::MAX_COLS { return Err("Неверный столбец ячейки".into()); }
                next_col=c+1;
                let index = a
                    .get("s")
                    .and_then(|s| s.parse::<usize>().ok())
                    .or_else(|| row_styles.get(&r).copied())
                    .or_else(|| col_styles.get(&c).copied())
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
                "sheetFormatPr" => {
                    if let Some(raw)=a.get("defaultColWidth") {
                        let width=raw.parse::<f64>().ok().filter(|value|value.is_finite()&&*value>0.0)
                            .ok_or("Недопустимая defaultColWidth")?;
                        let pixels=((256.0*width+18.0)/256.0*7.0).trunc();
                        if !(1.0..=1800.0).contains(&pixels) {return Err("defaultColWidth вне поддерживаемых пределов".into());}
                        s.default_col_width=pixels as f32;
                    }
                    if let Some(raw)=a.get("defaultRowHeight") {
                        let points=raw.parse::<f64>().ok().filter(|value|value.is_finite()&&*value>0.0)
                            .ok_or("Недопустимая defaultRowHeight")?;
                        let pixels=points*4.0/3.0;
                        if !(1.0..=600.0).contains(&pixels) {return Err("defaultRowHeight вне поддерживаемых пределов".into());}
                        s.default_row_height=pixels as f32;
                    }
                }
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
                        if let Some(style)=a.get("style").and_then(|value| value.parse::<usize>().ok()) {
                            col_styles.insert(c,style);
                        }
                        if let Some(w) = a
                            .get("width")
                            .and_then(|v| v.parse::<f32>().ok())
                            .filter(|w| w.is_finite() && *w > 0.0)
                        {
                            // ECMA-376 §18.3.1.13: the stored width already includes
                            // padding, pixels = trunc((256w + trunc(128/7)) / 256 * 7).
                            let pixels = ((256.0 * w + 18.0) / 256.0 * 7.0).trunc();
                            s.widths.insert(c, pixels.clamp(1.0, 1800.0));
                        }
                        if xml_bool(a.get("hidden"), false) {
                            s.hidden_cols.insert(c);
                        }
                    }
                }
                "row" => {
                    let r=current_row;
                    if xml_bool(a.get("customFormat"),a.contains_key("s"))
                        && let Some(style)=a.get("s").and_then(|value| value.parse::<usize>().ok()) {
                        row_styles.insert(r,style);
                    }
                    if let Some(h) = a
                        .get("ht")
                        .and_then(|v| v.parse::<f32>().ok())
                        .filter(|h| h.is_finite() && *h > 0.0)
                    {
                        s.row_heights.insert(r, (h * 96.0 / 72.0).clamp(1.0, 600.0));
                    }
                    if xml_bool(a.get("hidden"), false) {
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
                "tabColor" => s.tab_color = rgb_with_theme(a, &palette,&theme),
                "autoFilter" => s.filter = a.get("ref").and_then(|v| parse_range(v).ok()),
                _ => (),
            }
            Ok(())
        })?;
        if let Some((_,selection))=sheet_selections.iter().find(|(name,_)|name==&pane).or_else(||sheet_selections.first()) {
            selections.insert(id,selection.clone());
        }
        let rules=conditional_rules(bytes,&differential)?;
        book.sheet_by_id_mut(id).ok_or("Лист не найден")?.conditional=rules;
        read_sheet_images(book, id, &path, parts)?;
    }
    if book.active_sheet().is_some_and(|sheet| sheet.hidden) {
        book.active = book
            .sheets
            .iter()
            .position(|sheet| !sheet.hidden)
            .ok_or("Книга не содержит видимых листов")?;
    }
    if let Some(selection)=selections.remove(&book.active_id()) {book.selection=selection;}
    Ok(())
}
struct ChartLocation {
    sheet: u64,
    path: String,
    anchor: PictureAnchor,
}
fn chart_locations(book: &Book, parts: &BTreeMap<String, Vec<u8>>) -> Result<Vec<ChartLocation>, String> {
    let mut locations = Vec::new();
    let workbook_rels = relationships("xl/workbook.xml", parts);
    let mut sheets = Vec::new();
    if let Some(workbook) = parts.get("xl/workbook.xml") {
        each_tag(workbook, |tag, attrs| {
            if tag == "sheet" && let (Some(name), Some(id)) = (attrs.get("name"), attrs.get("r:id"))
                && let Some(sheet) = book.sheets.iter().find(|sheet| sheet.name == *name)
                && let Some((_, path)) = workbook_rels.get(id) {
                    sheets.push((sheet.id, path.clone()));
                }
            Ok(())
        })?;
    }
    for (sheet, path) in sheets {
        let Some(bytes) = parts.get(&path) else { continue; };
        let rels = relationships(&path, parts);
        let mut drawing_ids = Vec::new();
        each_tag(bytes, |tag, attrs| {
            if tag == "drawing" && let Some(id) = attrs.get("r:id") { drawing_ids.push(id.clone()); }
            Ok(())
        })?;
        for id in drawing_ids {
            let Some((_, drawing)) = rels.get(&id).filter(|(kind, _)| kind.ends_with("/drawing")) else { continue; };
            let drawing_rels = relationships(drawing, parts);
            let Some(bytes) = parts.get(drawing) else { continue; };
            let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
            for anchor in drawing_pictures(text)? {
                if let Some((_, path)) = drawing_rels.get(&anchor.chart).filter(|(kind, _)| kind.ends_with("/chart")) {
                    locations.push(ChartLocation { sheet, path: path.clone(), anchor });
                    if locations.len() > 10000 { return Err("Слишком много импортируемых диаграмм".into()); }
                }
            }
        }
    }
    Ok(locations)
}
fn read_charts(book: &mut Book, parts: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for location in chart_locations(book, parts)? {
        let bytes=parts.get(&location.path).ok_or("Не найдена XML-часть диаграммы")?;
        let mut reader = Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
        let mut kind = ChartKind::Column;
        let mut refs = Vec::new();
        let mut header_refs = Vec::new();
        let mut category_refs = Vec::new();
        let mut value_refs = Vec::new();
        let mut plot_types=0usize;
        let mut in_categories = false;
        let mut title = String::new();
        let mut title_refs=Vec::new();
        let mut in_title = false;
        let mut in_plot_area = false;
        let mut in_series_name = false;
        loop {
            let event = reader.read_event().map_err(|e| e.to_string())?;
            let empty = matches!(event, Event::Empty(_));
            match event {
                Event::Start(e) | Event::Empty(e) => {
                    let tag = local(&e);
                    if in_plot_area && tag.ends_with("Chart") {
                        if !matches!(tag.as_str(),"barChart"|"lineChart"|"areaChart"|"pieChart"|"doughnutChart"|"scatterChart") {
                            return Err(format!("Тип диаграммы {tag} пока не поддерживается; импорт остановлен без замены на столбчатую"));
                        }
                        plot_types+=1;
                        if plot_types>1 {return Err("Комбинированные диаграммы пока не поддерживаются".into());}
                    }
                    match tag.as_str() {
                        "plotArea" if !empty => in_plot_area = true,
                        "cat"|"xVal" if !empty => in_categories = true,
                        "tx" if !in_title && !empty => in_series_name = true,
                        "title" if !in_plot_area && !empty => in_title = true,
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
                            let found=super::formula::dependencies(&format!("={text}"));
                            if in_title {title_refs.extend(found);}
                            else if in_series_name {header_refs.extend(found);}
                            else if in_categories {category_refs.extend(found.clone());refs.extend(found);}
                            else {value_refs.extend(found.clone());refs.extend(found);}
                            if refs.len().saturating_add(header_refs.len()).saturating_add(title_refs.len())>100000 {
                                return Err("Слишком много ссылок в диаграмме".into());
                            }
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
                    if matches!(e.local_name().as_ref(),"cat"|"xVal") {in_categories=false;}
                    if e.local_name().as_ref() == "plotArea" { in_plot_area = false; }
                    if e.local_name().as_ref() == "tx" { in_series_name = false; }
                    if e.local_name().as_ref() == "title" {
                        in_title = false;
                    }
                }
                Event::DocType(_) => return Err("DTD запрещён".into()),
                Event::Eof => break,
                _ => (),
            }
        }
        if title.is_empty() && let Some((Some(sheet),range))=title_refs.first()
            && let Some(sheet)=book.sheets.iter().find(|value|value.name.to_lowercase() == sheet.to_lowercase()) {
            title=sheet.display(range.r0,range.c0,book.locale_ru);
        }
        let (source,first)=refs.first().ok_or("Диаграмма не содержит поддерживаемых ссылок на данные")?;
        if value_refs.is_empty() {return Err("Диаграмма не содержит поддерживаемых рядов данных".into());}
        let sheet=source.clone().unwrap_or_else(||book.sheet_by_id(location.sheet).map(|sheet|sheet.name.clone()).unwrap_or_default());
        let id=book.sheets.iter().find(|value|value.name.to_lowercase()==sheet.to_lowercase()).map(|sheet|sheet.id)
            .ok_or("Не найден лист данных диаграммы")?;
        {
            let mut range = *first;
            for (_, r) in &refs {
                range.r0 = range.r0.min(r.r0);
                range.c0 = range.c0.min(r.c0);
                range.r1 = range.r1.max(r.r1);
                range.c1 = range.c1.max(r.c1);
            }
            let data_columns:std::collections::BTreeSet<_>=refs.iter().map(|(_,range)|range.c0).collect();
            if refs.iter().any(|(_,part)|part.c0!=part.c1||part.r0!=range.r0||part.r1!=range.r1)
                || data_columns.len() as u32!=range.c1-range.c0+1
                || category_refs.iter().any(|(_,part)|part.c0!=range.c0) {
                return Err("Несмежные или горизонтальные источники диаграммы пока не поддерживаются; импорт остановлен без включения посторонних ячеек".into());
            }
            let has_headers=range.r0>0 && header_refs.iter().any(|(source,header)|
                source.as_ref().is_none_or(|source|source.to_lowercase()==sheet.to_lowercase())
                    && header.r0==header.r1 && header.r0+1==range.r0
                    && header.c0==header.c1 && header.c0>=range.c0 && header.c0<=range.c1);
            if has_headers {range.r0-=1;}
            range.validate().map_err(|error|format!("Диапазон диаграммы: {error}"))?;
            {
                let owner = book.sheet_by_id(location.sheet).ok_or("Нет листа диаграммы")?;
                if owner.id != id || refs.iter().any(|(source, _)| source.as_ref()
                    .is_some_and(|source| source.to_lowercase() != owner.name.to_lowercase())) {
                    return Err("Диаграмма с данными другого листа пока не поддерживается; импорт остановлен без переноса её на чужой лист".into());
                }
                let (row, col, _, _, width, height) = picture_geometry(owner, &location.anchor);
                let chart_id = book.add_chart(location.sheet, kind, range, &title)?;
                if let Some(chart) = book.sheet_by_id_mut(location.sheet)
                    .and_then(|sheet| sheet.charts.iter_mut().find(|chart| chart.id == chart_id)) {
                    chart.row = row; chart.col = col;
                    chart.has_headers=has_headers;
                    chart.has_categories=!category_refs.is_empty()
                        && !value_refs.iter().any(|(_,value)|value.c0==range.c0);
                    if width > 0.0 && height > 0.0 {
                        chart.width = width.clamp(1.0, 4000.0);
                        chart.height = height.clamp(1.0, 4000.0);
                    }
                }
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
/// Import only drawings referenced by the worksheet; fail explicitly when a referenced picture is lost.
fn read_sheet_images(
    book: &mut Book,
    id: u64,
    sheet_path: &str,
    parts: &BTreeMap<String, Vec<u8>>,
) -> Result<(),String> {
    let worksheet=parts.get(sheet_path).ok_or("Нет части листа с рисунками")?;
    let rels=relationships(sheet_path,parts);
    let mut drawings=Vec::new();
    each_tag(worksheet,|tag,attrs| {
        if tag=="drawing" {
            let reference=attrs.get("r:id").ok_or("Нет связи рисунков листа")?;
            let (kind,target)=rels.get(reference).ok_or("Не найдена связь рисунков листа")?;
            if !kind.ends_with("/drawing") {return Err("Связь рисунков имеет неверный тип".into());}
            drawings.push(target.clone());
        }
        Ok(())
    })?;
    for drawing in drawings {
        let media=relationships(&drawing,parts);
        let bytes=parts.get(&drawing).ok_or("Не найдена часть рисунков листа")?;
        let text=std::str::from_utf8(bytes).map_err(|error|error.to_string())?;
        for anchor in drawing_pictures(text)? {
            if anchor.embed.is_empty() {continue;} // Chart anchors are handled separately.
            let (kind,target)=media.get(&anchor.embed).ok_or("Не найдена связь изображения")?;
            if !kind.ends_with("/image") {return Err("Связь изображения имеет неверный тип".into());}
            let bytes=parts.get(target).ok_or("Не найдены байты изображения")?;
            let sheet=book.sheet_by_id(id).ok_or("Лист не найден")?;
            let (row,col,x_offset,y_offset,width,height)=picture_geometry(sheet,&anchor);
            let image_id=book.add_image(id,row,col,bytes.clone())?;
            let image=book.sheet_by_id_mut(id).and_then(|sheet|sheet.images.iter_mut().find(|image|image.id==image_id))
                .ok_or("Импортированный рисунок не найден")?;
            image.x_offset=x_offset;image.y_offset=y_offset;
            if width>0.0 && height>0.0 {
                image.width=width.clamp(1.0,8000.0);image.height=height.clamp(1.0,8000.0);
            }
            image.alt=anchor.alt;
        }
    }
    Ok(())
}
#[derive(Default)]
struct PicturePoint {
    row: u32,
    col: u32,
    row_off: i64,
    col_off: i64,
}
#[derive(Default)]
struct PictureAnchor {
    from: PicturePoint,
    to: Option<PicturePoint>,
    absolute: Option<(i64, i64)>,
    cx: u64,
    cy: u64,
    embed: String,
    chart: String,
    alt: String,
}
fn drawing_pictures(text: &str) -> Result<Vec<PictureAnchor>, String> {
    let mut reader = Reader::from_str(text);
    let mut pictures = Vec::new();
    let mut current: Option<PictureAnchor> = None;
    let mut point = None;
    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(e) => {
                let tag = local(&e);
                match tag.as_str() {
                    "oneCellAnchor" | "twoCellAnchor" | "absoluteAnchor" => {
                        current = Some(PictureAnchor::default());
                        point = None;
                    }
                    "from" => point = Some(false),
                    "to" => {
                        point = Some(true);
                        if let Some(anchor) = current.as_mut() {
                            anchor.to = Some(PicturePoint::default());
                        }
                    }
                    "row" | "col" | "rowOff" | "colOff" if point.is_some() => {
                        let value = reader
                            .read_text(e.name())
                            .map_err(|error| error.to_string())?
                            .trim()
                            .parse::<i64>()
                            .map_err(|_| "Неверная координата рисунка")?;
                        if let Some(anchor) = current.as_mut() {
                            let position = if point == Some(true) {
                                anchor
                                    .to
                                    .as_mut()
                                    .ok_or("Нет конечной координаты рисунка")?
                            } else {
                                &mut anchor.from
                            };
                            match tag.as_str() {
                                "row" => {
                                    position.row = u32::try_from(value)
                                        .map_err(|_| "Строка рисунка вне листа")?
                                }
                                "col" => {
                                    position.col = u32::try_from(value)
                                        .map_err(|_| "Столбец рисунка вне листа")?
                                }
                                "rowOff" => position.row_off = value,
                                _ => position.col_off = value,
                            }
                        }
                    }
                    _ => picture_attrs(&e, &reader, &tag, current.as_mut())?,
                }
            }
            Event::Empty(e) => picture_attrs(&e, &reader, &local(&e), current.as_mut())?,
            Event::End(e) => match e.local_name().as_ref() {
                "from" | "to" => point = None,
                "oneCellAnchor" | "twoCellAnchor" | "absoluteAnchor" => {
                    if let Some(anchor) = current.take()
                        && (!anchor.embed.is_empty() || !anchor.chart.is_empty())
                    {
                        pictures.push(anchor);
                    }
                }
                _ => {}
            },
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => {
                if current.is_some() {return Err("Незавершённая привязка рисунка".into());}
                break;
            },
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
    let Some(anchor) = anchor else { return Ok(()) };
    let values = attrs(e, reader)?;
    match tag {
        "ext" if anchor.cx == 0 => {
            anchor.cx = values
                .get("cx")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            anchor.cy = values
                .get("cy")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
        "pos" => {
            if let (Some(x), Some(y)) = (
                values.get("x").and_then(|value| value.parse().ok()),
                values.get("y").and_then(|value| value.parse().ok()),
            ) {
                anchor.absolute = Some((x, y));
            }
        }
        "chart" => {
            if let Some(id) = values.get("r:id") { anchor.chart = id.clone(); }
        }
        "blip" => {
            if let Some(embed) = values.get("r:embed") {
                anchor.embed = embed.clone();
            }
        }
        "cNvPr" => {
            if let Some(description) = values.get("descr") {
                anchor.alt = description.clone();
            }
        }
        _ => {}
    }
    Ok(())
}
fn drawing_axis_position(
    index: u32,
    base: f32,
    sizes: &BTreeMap<u32, f32>,
    hidden: &std::collections::BTreeSet<u32>,
) -> f32 {
    (index as f32 * base
        + sizes
            .range(..index)
            .filter(|(position, _)| !hidden.contains(position))
            .map(|(_, size)| *size - base)
            .sum::<f32>()
        - hidden.range(..index).count() as f32 * base)
        .max(0.0)
}
fn drawing_axis_index(
    pixel: f32,
    count: u32,
    base: f32,
    sizes: &BTreeMap<u32, f32>,
    hidden: &std::collections::BTreeSet<u32>,
) -> u32 {
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = low + (high - low) / 2;
        if drawing_axis_position(middle + 1, base, sizes, hidden) <= pixel {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low.min(count - 1)
}
fn picture_geometry(
    sheet: &super::model::Sheet,
    anchor: &PictureAnchor,
) -> (u32, u32, f32, f32, f32, f32) {
    let x = drawing_axis_position(anchor.from.col, sheet.default_col_width, &sheet.widths, &sheet.hidden_cols)
        + anchor.from.col_off as f32 / 9525.0;
    let y = drawing_axis_position(
        anchor.from.row,
        sheet.default_row_height,
        &sheet.row_heights,
        &sheet.hidden_rows,
    ) + anchor.from.row_off as f32 / 9525.0;
    let (x, y) = anchor
        .absolute
        .map_or((x, y), |(x, y)| (x as f32 / 9525.0, y as f32 / 9525.0));
    let (x, y) = (x.max(0.0), y.max(0.0));
    let (row, col, x_offset, y_offset) = if anchor.absolute.is_some()
        || anchor.from.row_off < 0 || anchor.from.col_off < 0 {
        let row = drawing_axis_index(
            y,
            super::model::MAX_ROWS,
            sheet.default_row_height,
            &sheet.row_heights,
            &sheet.hidden_rows,
        );
        let col = drawing_axis_index(
            x,
            super::model::MAX_COLS,
            sheet.default_col_width,
            &sheet.widths,
            &sheet.hidden_cols,
        );
        (
            row,
            col,
            x - drawing_axis_position(col, sheet.default_col_width, &sheet.widths, &sheet.hidden_cols),
            y - drawing_axis_position(row, sheet.default_row_height, &sheet.row_heights, &sheet.hidden_rows),
        )
    } else {
        (
            anchor.from.row,
            anchor.from.col,
            anchor.from.col_off as f32 / 9525.0,
            anchor.from.row_off as f32 / 9525.0,
        )
    };
    let (width, height) = anchor
        .to
        .as_ref()
        .map(|end| {
            (
                drawing_axis_position(end.col, sheet.default_col_width, &sheet.widths, &sheet.hidden_cols)
                    + end.col_off as f32 / 9525.0
                    - x,
                drawing_axis_position(end.row, sheet.default_row_height, &sheet.row_heights, &sheet.hidden_rows)
                    + end.row_off as f32 / 9525.0
                    - y,
            )
        })
        .filter(|(width, height)| *width > 0.0 && *height > 0.0)
        .unwrap_or((anchor.cx as f32 / 9525.0, anchor.cy as f32 / 9525.0));
    (
        row,
        col,
        x_offset.clamp(0.0, 4000.0),
        y_offset.clamp(0.0, 4000.0),
        width,
        height,
    )
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
fn pin_drawing_columns(ws:&mut rust_xlsxwriter::Worksheet,sheet:&super::model::Sheet,col:u32,offset:f32,width:f32)->Result<(),String> {
    let end=drawing_axis_index(drawing_axis_position(col,sheet.default_col_width,&sheet.widths,&sheet.hidden_cols)+offset+width,
        super::model::MAX_COLS,sheet.default_col_width,&sheet.widths,&sheet.hidden_cols);
    for column in col..=end {
        if !sheet.widths.contains_key(&column)&&!sheet.hidden_cols.contains(&column) {
            ws.set_column_width_pixels(column as u16,sheet.default_col_width.round() as u32).map_err(|error|error.to_string())?;
        }
    }
    Ok(())
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
        ws.set_default_row_height(f64::from(sheet.default_row_height)*0.75);
        if index == book.active {
            ws.set_selection(book.selection.row, book.selection.col as u16,
                book.selection.anchor_row, book.selection.anchor_col as u16)
                .map_err(|error| error.to_string())?;
        }
        if let Some(color) = sheet.tab_color {
            ws.set_tab_color(Color::RGB(rgb_number(color)));
        }
        ws.set_freeze_panes(sheet.frozen_rows, sheet.frozen_cols as u16)
            .map_err(|e| e.to_string())?;
        for (col, width) in &sheet.widths {
            ws.set_column_width_pixels(*col as u16, width.round().clamp(1.0, 1800.0) as u32)
                .map_err(|e| e.to_string())?;
        }
        for (row, height) in &sheet.row_heights {
            ws.set_row_height(*row, f64::from(height.clamp(1.0, 600.0)) * 72.0 / 96.0)
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
        for range in sheet
            .merges
            .iter()
            .filter(|range| range.r0 != range.r1 || range.c0 != range.c1)
        {
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
        for rule in &sheet.conditional {
            let comparison=match rule.operator.as_str() {
                ">"=>ConditionalFormatCellRule::GreaterThan(rule.threshold),
                "<"=>ConditionalFormatCellRule::LessThan(rule.threshold),
                "="=>ConditionalFormatCellRule::EqualTo(rule.threshold),
                ">="=>ConditionalFormatCellRule::GreaterThanOrEqualTo(rule.threshold),
                "<="=>ConditionalFormatCellRule::LessThanOrEqualTo(rule.threshold),
                _=>return Err("Недопустимое условное форматирование".into()),
            };
            let format=ConditionalFormatCell::new().set_rule(comparison)
                .set_format(Format::new().set_background_color(Color::RGB(rgb_number(rule.fill)))).set_stop_if_true(true);
            let range=rule.range;
            ws.add_conditional_format(range.r0,range.c0 as u16,range.r1,range.c1 as u16,&format)
                .map_err(|error|error.to_string())?;
        }
        for model in &sheet.charts {
            pin_drawing_columns(ws,sheet,model.col,0.0,model.width)?;
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
                .set_width(model.width.round().clamp(1.0, 4000.0) as u32)
                .set_height(model.height.round().clamp(1.0, 4000.0) as u32);
            let r = model.range;
            let data_start = if model.has_headers && r.r1 > r.r0 { r.r0 + 1 } else { r.r0 };
            let start_col = if model.has_categories && r.c1 > r.c0 { r.c0 + 1 } else { r.c0 };
            for c in start_col..=r.c1 {
                let series = chart.add_series();
                series.set_values((sheet.name.as_str(), data_start, c as u16, r.r1, c as u16));
                if (model.has_categories && r.c1 > r.c0) || model.kind==ChartKind::Scatter {
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
            pin_drawing_columns(ws,sheet,model.col,model.x_offset,model.width)?;
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
    let bytes = workbook
        .save_to_buffer()
        .map_err(|error| error.to_string())?;
    patch_formula_string_results(bytes, book)
}
fn ambiguous_formula_string(cell: &super::model::Cell) -> Option<&str> {
    if !cell.input.starts_with('=') {
        return None;
    }
    let Value::Text(text) = &cell.value else {
        return None;
    };
    (text.parse::<f64>().is_ok()
        || matches!(
            text.as_str(),
            "TRUE"
                | "FALSE"
                | "#DIV/0!"
                | "#N/A"
                | "#NAME?"
                | "#NULL!"
                | "#NUM!"
                | "#REF!"
                | "#VALUE!"
                | "#GETTING_DATA"
        ))
    .then_some(text.as_str())
}
fn typed_formula_strings(xml: &[u8], sheet: &super::model::Sheet) -> Result<Vec<u8>, String> {
    let mut reader = Reader::from_reader(xml);
    let mut writer = Writer::new(Vec::with_capacity(xml.len()));
    let mut result: Option<&str> = None;
    loop {
        let event = reader.read_event().map_err(|error| error.to_string())?;
        match event {
            Event::Empty(e) if e.local_name().as_ref()=="sheetFormatPr" => {
                let values=attrs(&e,&reader)?;
                let mut element=e.into_owned();element.clear_attributes();
                for (key,value) in values.iter().filter(|(key,_)|!matches!(key.as_str(),"defaultColWidth"|"defaultRowHeight"|"customHeight")) {
                    element.push_attribute((key.as_str(),value.as_str()));
                }
                let width=((f64::from(sheet.default_col_width.round())+0.5)/7.0-18.0/256.0).to_string();
                let height=(f64::from(sheet.default_row_height)*0.75).to_string();
                element.push_attribute(("defaultColWidth",width.as_str()));
                element.push_attribute(("defaultRowHeight",height.as_str()));
                element.push_attribute(("customHeight","1"));
                writer.write_event(Event::Empty(element)).map_err(|error|error.to_string())?;
            }
            Event::Start(e) if e.local_name().as_ref() == "c" => {
                let values = attrs(&e, &reader)?;
                result = values
                    .get("r")
                    .and_then(|address| sheet.cells.get(address))
                    .and_then(ambiguous_formula_string);
                if result.is_some() {
                    let mut element = e.into_owned();
                    element.clear_attributes();
                    for (key, value) in values.iter().filter(|(key, _)| key.as_str() != "t") {
                        element.push_attribute((key.as_str(), value.as_str()));
                    }
                    element.push_attribute(("t", "str"));
                    writer
                        .write_event(Event::Start(element))
                        .map_err(|error| error.to_string())?;
                } else {
                    writer
                        .write_event(Event::Start(e))
                        .map_err(|error| error.to_string())?;
                }
            }
            Event::Start(e) if e.local_name().as_ref() == "row" => writer
                .write_event(Event::Start(exact_row_height(e, &reader, sheet)?))
                .map_err(|error| error.to_string())?,
            Event::Empty(e) if e.local_name().as_ref() == "row" => writer
                .write_event(Event::Empty(exact_row_height(e, &reader, sheet)?))
                .map_err(|error| error.to_string())?,
            Event::Start(e) if e.local_name().as_ref() == "v" && result.is_some() => {
                writer
                    .write_event(Event::Start(e.borrow()))
                    .map_err(|error| error.to_string())?;
                writer
                    .write_event(Event::Text(BytesText::new(result.unwrap_or(""))))
                    .map_err(|error| error.to_string())?;
                reader
                    .read_to_end(e.name())
                    .map_err(|error| error.to_string())?;
                writer
                    .write_event(Event::End(e.to_end()))
                    .map_err(|error| error.to_string())?;
            }
            Event::End(e) if e.local_name().as_ref() == "c" => {
                result = None;
                writer
                    .write_event(Event::End(e))
                    .map_err(|error| error.to_string())?;
            }
            Event::Eof => break,
            event => writer
                .write_event(event)
                .map_err(|error| error.to_string())?,
        }
    }
    Ok(writer.into_inner())
}
// rust_xlsxwriter stores row heights as whole pixels; restore the model's exact value.
fn exact_row_height(
    e: BytesStart<'_>,
    reader: &Reader<&[u8]>,
    sheet: &super::model::Sheet,
) -> Result<BytesStart<'static>, String> {
    let values = attrs(&e, reader)?;
    let height = values
        .get("r")
        .and_then(|r| r.parse::<u32>().ok())
        .and_then(|r| r.checked_sub(1))
        .and_then(|r| sheet.row_heights.get(&r))
        .filter(|height| height.fract() != 0.0);
    let mut element = e.into_owned();
    if let Some(height) = height {
        element.clear_attributes();
        for (key, value) in values.iter().filter(|(key, _)| key.as_str() != "ht") {
            element.push_attribute((key.as_str(), value.as_str()));
        }
        let points = (f64::from(height.clamp(1.0, 600.0)) * 0.75).to_string();
        element.push_attribute(("ht", points.as_str()));
    }
    Ok(element)
}
fn sheet_needs_patch(sheet: &super::model::Sheet) -> bool {
    sheet.default_col_width != 64.0 || sheet.default_row_height != 20.0 || sheet
        .row_heights
        .values()
        .any(|height| height.fract() != 0.0)
        || sheet
            .cells
            .values()
            .any(|cell| ambiguous_formula_string(cell).is_some())
}
fn patch_formula_string_results(bytes: Vec<u8>, book: &Book) -> Result<Vec<u8>, String> {
    if !book.sheets.iter().any(sheet_needs_patch) {
        return Ok(bytes);
    }
    let mut parts = crate::package::read_zip(&bytes)?;
    for (index, sheet) in book.sheets.iter().enumerate() {
        if !sheet_needs_patch(sheet) {
            continue;
        }
        let path = format!("xl/worksheets/sheet{}.xml", index + 1);
        let xml = parts
            .get(&path)
            .ok_or("Нет XML-части экспортируемого листа")?;
        let updated = typed_formula_strings(xml, sheet)?;
        parts.insert(path, updated);
    }
    crate::package::write_zip(parts.into_iter().collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn review5_indexed_colours_and_inactive_fill_and_empty_border_preserve_visuals()
    -> Result<(), String> {
        let model=styles(br#"<styleSheet><fonts><font><color indexed="10"/></font></fonts><fills><fill><patternFill patternType="none"><fgColor rgb="FFFF0000"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor indexed="12"/></patternFill></fill></fills><borders><border/><border><left/><right/><top/><bottom/></border><border><left style="thin"/></border></borders><cellXfs><xf fontId="0" fillId="0" borderId="1"/><xf fontId="0" fillId="1" borderId="2"/></cellXfs></styleSheet>"#)?;
        assert_eq!(model[0].color, [255, 0, 0]);
        assert_eq!(model[0].fill, None);
        assert!(!model[0].border);
        assert_eq!(model[1].fill, Some([0, 0, 255]));
        assert!(model[1].border);
        let custom=styles(br#"<styleSheet><fonts><font><color indexed="0"/></font></fonts><cellXfs><xf fontId="0"/></cellXfs><colors><indexedColors><rgbColor rgb="00ABCDEF"/></indexedColors></colors></styleSheet>"#)?;
        assert_eq!(custom[0].color, [0xAB, 0xCD, 0xEF]);
        Ok(())
    }
    #[test]
    fn review5_relationship_attribute_aliases_work_in_workbook_and_drawing() -> Result<(), String> {
        let mut book = Book::default();
        book.active_sheet_mut().ok_or("sheet")?.name = "Sheet1".into();
        let parts=BTreeMap::from([
            ("xl/workbook.xml".into(),br#"<workbook xmlns:rel="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" rel:id="sheet"/></sheets></workbook>"#.to_vec()),
            ("xl/_rels/workbook.xml.rels".into(),br#"<Relationships><Relationship Id="sheet" Target="worksheets/sheet1.xml"/></Relationships>"#.to_vec()),
            ("xl/styles.xml".into(),br#"<styleSheet><fonts><font><b/></font></fonts><cellXfs><xf fontId="0"/></cellXfs></styleSheet>"#.to_vec()),
            ("xl/worksheets/sheet1.xml".into(),br#"<worksheet><sheetData><row r="1"><c r="A1" s="0"/></row></sheetData></worksheet>"#.to_vec()),
        ]);
        read_metadata(&mut book, &parts)?;
        assert!(
            book.active_sheet()
                .and_then(|s| s.cell(0, 0))
                .ok_or("cell")?
                .fmt
                .bold
        );
        let drawings = drawing_pictures(
            r#"<wsDr xmlns:link="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><oneCellAnchor><from><row>0</row><col>0</col></from><ext cx="9525" cy="9525"/><pic><blip link:embed="picture"/></pic></oneCellAnchor></wsDr>"#,
        )?;
        assert_eq!(drawings.len(), 1);
        assert_eq!(drawings[0].embed, "picture");
        Ok(())
    }
    #[test]
    fn review5_export_preserves_small_chart_width_and_literal_markdown_and_html_whitespace()
    -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_range(
            id,
            0,
            0,
            &[
                vec!["Name".into(), "Value".into()],
                vec!["one".into(), "1".into()],
            ],
        )?;
        let chart = book.add_chart(
            id,
            ChartKind::Column,
            super::super::model::Range::new(0, 0, 1, 1),
            "Small",
        )?;
        let model = book
            .active_sheet_mut()
            .ok_or("sheet")?
            .charts
            .iter_mut()
            .find(|c| c.id == chart)
            .ok_or("chart")?;
        model.width = 23.0;
        model.height = 37.0;
        let parts = crate::package::read_zip(&write(&book, "xlsx")?)?;
        let drawing = std::str::from_utf8(parts.get("xl/drawings/drawing1.xml").ok_or("drawing")?)
            .map_err(|e| e.to_string())?;
        assert!(drawing.contains("<xdr:colOff>219075</xdr:colOff>"));
        book.set_cell(
            id,
            0,
            0,
            "*literal* [link](https://invalid) <b>&name\nnext  line",
        )?;
        let md = String::from_utf8(write(&book, "md")?).map_err(|e| e.to_string())?;
        assert!(md.contains(
            r"\*literal\* \[link\]\(https\:\/\/invalid\) &lt;b&gt;&amp;name<br>next  line"
        ));
        let html = String::from_utf8(write(&book, "html")?).map_err(|e| e.to_string())?;
        assert!(html.contains("white-space:pre-wrap"));
        assert!(html.contains("&lt;b&gt;&amp;name\nnext  line"));
        Ok(())
    }
    #[test]
    fn review4_xlsx_formula_cache_keeps_numeric_boolean_and_error_looking_text_typed()
    -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_range(
            id,
            0,
            0,
            &[vec![
                "=\"001\"".into(),
                "=\"TRUE\"".into(),
                "=\"#N/A\"".into(),
                "=123".into(),
                "=TRUE".into(),
                "=\"NaN\"".into(),
                "=NA()".into(),
            ]],
        )?;
        let bytes = write(&book, "xlsx")?;
        let mut imported =
            open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(|error| error.to_string())?;
        let range = imported
            .worksheet_range("Лист1")
            .map_err(|error| error.to_string())?;
        for (col, expected) in [
            (0, Data::String("001".into())),
            (1, Data::String("TRUE".into())),
            (2, Data::String("#N/A".into())),
            (3, Data::Float(123.0)),
            (4, Data::Bool(true)),
            (5, Data::String("NaN".into())),
        ] {
            assert_eq!(range.get_value((0, col)), Some(&expected));
        }
        assert!(matches!(range.get_value((0, 6)), Some(Data::Error(_))));
        Ok(())
    }
    #[test]
    fn review4_xlsx_keeps_small_dimensions_and_fractional_row_height() -> Result<(), String> {
        let mut book = Book::default();
        let sheet = book.active_sheet_mut().ok_or("sheet")?;
        sheet.widths.insert(0, 6.0);
        sheet.row_heights.insert(0, 1.5);
        let bytes = write(&book, "xlsx")?;
        let again = read(&bytes, "xlsx")?;
        let sheet = again.active_sheet().ok_or("sheet")?;
        assert!((sheet.widths.get(&0).copied().unwrap_or(0.0) - 6.0).abs() < 0.1);
        assert!((sheet.row_heights.get(&0).copied().unwrap_or(0.0) - 1.5).abs() < 0.01);
        Ok(())
    }
    #[test]
    fn review4_drawing_anchors_use_absolute_position_and_both_cell_markers() -> Result<(), String> {
        let mut sheet = super::super::model::Sheet::default();
        sheet.widths.insert(0, 180.0);
        sheet.row_heights.insert(0, 42.0);
        let xml = format!(
            r#"<wsDr xmlns:r="urn:r"><absoluteAnchor><pos x="{}" y="{}"/><ext cx="{}" cy="{}"/><pic><blip r:embed="p"/></pic></absoluteAnchor><twoCellAnchor><from><col>0</col><colOff>{}</colOff><row>0</row><rowOff>{}</rowOff></from><to><col>2</col><colOff>{}</colOff><row>2</row><rowOff>{}</rowOff></to><pic><ext cx="9525" cy="9525"/><blip r:embed="p"/></pic></twoCellAnchor></wsDr>"#,
            190 * 9525,
            47 * 9525,
            40 * 9525,
            30 * 9525,
            10 * 9525,
            5 * 9525,
            20 * 9525,
            6 * 9525
        );
        let anchors = drawing_pictures(&xml)?;
        assert_eq!(anchors.len(), 2);
        assert_eq!(
            picture_geometry(&sheet, &anchors[0]),
            (1, 1, 10.0, 5.0, 40.0, 30.0)
        );
        assert_eq!(
            picture_geometry(&sheet, &anchors[1]),
            (0, 0, 10.0, 5.0, 286.0, 69.0)
        );
        Ok(())
    }

    use super::super::model::Range;
    use super::*;

    #[test]
    fn review2_builtin_twelve_hour_formats_keep_the_period() {
        for (id, expected) in [(18, "1:30 PM"), (19, "1:30:00 PM")] {
            assert_eq!(
                super::super::format::display(&Value::Number(13.5 / 24.0), &builtin(id), false),
                expected
            );
        }
        assert_eq!(
            super::super::format::display(&Value::Number(13.5 / 24.0), &builtin(20), false),
            "13:30"
        );
    }
    #[test]
    fn review2_degenerate_merged_ranges_do_not_break_xlsx_save() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "kept")?;
        book.merge(id, Range::new(0, 0, 0, 0))?;
        assert!(book.active_sheet().ok_or("sheet")?.merges.is_empty());
        book.merge(id, Range::new(0, 0, 1, 0))?;
        book.edit_dimension(id, 1, 1, true, true)?;
        assert!(book.active_sheet().ok_or("sheet")?.merges.is_empty());
        let bytes = write(&book, "xlsx")?;
        let reopened = read(&bytes, "xlsx")?;
        assert_eq!(
            reopened
                .active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| cell.value.clone()),
            Some(Value::Text("kept".into()))
        );
        Ok(())
    }

    #[test]
    fn review_xlsx_rotation_and_boolean_values() -> Result<(), String> {
        let formats = styles(br#"<styleSheet><fonts count="1"><font><b val="false"/><i val="0"/><strike val="false"/></font></fonts><cellXfs count="2"><xf fontId="0"><alignment textRotation="135" wrapText="true"/></xf><xf fontId="0"><alignment textRotation="180"/></xf></cellXfs></styleSheet>"#)?;
        assert_eq!(formats.len(), 2);
        assert_eq!(formats[0].rotation, -45);
        assert_eq!(formats[1].rotation, -90);
        assert!(formats[0].wrap);
        assert!(!formats[0].bold && !formats[0].italic && !formats[0].strike);
        formats[0].validate()?;
        Ok(())
    }

    #[test]
    fn review_metadata_resolves_relative_sheet_and_style_targets() -> Result<(), String> {
        let mut book = Book::default();
        book.sheets[0].name = "Sheet1".into();
        let parts = BTreeMap::from([
            ("xl/workbook.xml".into(), br#"<workbook xmlns:r="urn:r"><sheets><sheet name="Sheet1" r:id="s1"/></sheets></workbook>"#.to_vec()),
            ("xl/_rels/workbook.xml.rels".into(), br#"<Relationships><Relationship Id="s1" Target="worksheets/../worksheets/custom.xml"/><Relationship Id="st" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles/custom.xml"/></Relationships>"#.to_vec()),
            ("xl/styles/custom.xml".into(), br#"<styleSheet><fonts><font><b/></font></fonts><cellXfs><xf fontId="0"/></cellXfs></styleSheet>"#.to_vec()),
            ("xl/worksheets/custom.xml".into(), br#"<worksheet><cols><col min="1" max="1" hidden="true"/></cols><sheetData><row r="1" hidden="true"><c r="A1" s="0"/></row></sheetData></worksheet>"#.to_vec()),
        ]);
        read_metadata(&mut book, &parts)?;
        let sheet = book.active_sheet().ok_or("no sheet")?;
        assert!(sheet.hidden_rows.contains(&0) && sheet.hidden_cols.contains(&0));
        assert!(sheet.cell(0, 0).ok_or("missing styled cell")?.fmt.bold);
        book.validate()?;
        Ok(())
    }

    #[test]
    fn review_1904_dates_normalize_but_durations_keep_their_value() {
        use calamine::{ExcelDateTime, ExcelDateTimeType};
        let value = ExcelDateTime::new(0.5, ExcelDateTimeType::DateTime, true);
        assert_eq!(
            normalized_datetime(&value, true),
            super::super::formula::date_serial(1904, 1, 1) + 0.5
        );
        let duration = ExcelDateTime::new(2.5, ExcelDateTimeType::TimeDelta, true);
        assert_eq!(normalized_datetime(&duration, true), 2.5);
        let phantom = ExcelDateTime::new(60.25, ExcelDateTimeType::DateTime, false);
        assert_eq!(normalized_datetime(&phantom, false), 60.25);
        for raw in [1e308, -1.5] {
            let extreme = ExcelDateTime::new(raw, ExcelDateTimeType::DateTime, false);
            assert_eq!(normalized_datetime(&extreme, false), raw);
        }
        let negative = ExcelDateTime::new(-1.5, ExcelDateTimeType::DateTime, true);
        assert_eq!(normalized_datetime(&negative, true), 1460.5);
    }
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
        assert_eq!(parse_csv("\"x\"z", ','), Ok(vec![vec!["xz".to_string()]]));
        assert_eq!(
            parse_csv("5\" pipe\tОн сказал \"да\"", '\t'),
            Ok(vec![vec![
                "5\" pipe".to_string(),
                "Он сказал \"да\"".to_string()
            ]])
        );
        assert_eq!(
            parse_csv("a,b,\r\n", ','),
            Ok(vec![vec!["a".into(), "b".into(), "".into()]])
        );
    }
    #[test]
    fn dotted_tsv_extensions_keep_tabs() -> Result<(), String> {
        let mut book = Book::default();
        book.set_range(
            book.active_id(),
            0,
            0,
            &[vec!["a;b;c".into(), "value".into()]],
        )?;
        for ext in ["tsv", ".tsv", ".TSV"] {
            let bytes = write(&book, ext)?;
            assert!(decode_text(&bytes)?.contains("\t"));
            let restored = read(&bytes, ext)?;
            assert_eq!(
                restored.read_range(restored.active_id(), 0, 0, 0, 1)?,
                vec![vec!["a;b;c".to_string(), "value".to_string()]]
            );
        }
        Ok(())
    }
    #[test]
    fn xlsx_keeps_active_sheet() -> Result<(), String> {
        let mut book = Book::default();
        book.add_sheet("Second")?;
        book.active = 1;
        let restored = read(&write(&book, "xlsx")?, "xlsx")?;
        assert_eq!(restored.active, 1);
        assert_eq!(
            restored.active_sheet().map(|sheet| sheet.name.as_str()),
            Some("Second")
        );
        Ok(())
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

fn tint_color(color: [u8;3], tint: f64) -> [u8;3] {
    if tint == 0.0 { return color; }
    let [r,g,b]=color.map(|v| f64::from(v)/255.0);
    let max=r.max(g).max(b);
    let min=r.min(g).min(b);
    let delta=max-min;
    let light=(max+min)/2.0;
    let saturation=if delta == 0.0 { 0.0 } else { delta/(1.0-(2.0*light-1.0).abs()) };
    let hue=if delta == 0.0 { 0.0 } else if max == r { ((g-b)/delta).rem_euclid(6.0) }
        else if max == g { (b-r)/delta+2.0 } else { (r-g)/delta+4.0 };
    let light=if tint < 0.0 { light*(1.0+tint) } else { light*(1.0-tint)+tint };
    let chroma=(1.0-(2.0*light-1.0).abs())*saturation;
    let x=chroma*(1.0-(hue.rem_euclid(2.0)-1.0).abs());
    let offset=light-chroma/2.0;
    let rgb=match hue as u32 {
        0 => [chroma,x,0.0],1 => [x,chroma,0.0],2 => [0.0,chroma,x],
        3 => [0.0,x,chroma],4 => [x,0.0,chroma],_ => [chroma,0.0,x],
    };
    rgb.map(|v| ((v+offset)*255.0).round().clamp(0.0,255.0) as u8)
}
fn theme_colors(bytes: &[u8]) -> Result<Vec<[u8;3]>,String> {
    let slots=["lt1","dk1","lt2","dk2","accent1","accent2","accent3","accent4","accent5","accent6","hlink","folHlink"];
    let mut colors=vec![[255;3],[0;3],[238,236,225],[31,73,125],[79,129,189],[192,80,77],
        [155,187,89],[128,100,162],[75,172,198],[247,150,70],[0,0,255],[128,0,128]];
    let mut reader=Reader::from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?);
    let mut inside=false;
    let mut slot=None;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e)|Event::Empty(e) => {
                let tag=local(&e);
                if tag == "clrScheme" { inside=true; }
                else if inside {
                    if let Some(index)=slots.iter().position(|name| *name==tag) { slot=Some(index); }
                    else if matches!(tag.as_str(),"srgbClr"|"sysClr") {
                        let a=attrs(&e,&reader)?;
                        let value=a.get(if tag=="sysClr" { "lastClr" } else { "val" });
                        if let (Some(index),Some(value))=(slot,value) {
                            let attributes=BTreeMap::from([("rgb".into(),value.clone())]);
                            if let Some(color)=rgb(&attributes) { colors[index]=color; }
                        }
                    }
                }
            }
            Event::End(e) if e.local_name().as_ref()=="clrScheme" => { inside=false; slot=None; }
            Event::DocType(_) => return Err("DTD запрещён".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(colors)
}

#[cfg(test)]
mod metadata_defect_tests {
    use super::*;
    fn metadata(styles: &str,sheet: &str) -> Result<Book,String> {
        let mut book=Book::default();
        let name=crate::package::xml_escape(&book.sheets[0].name);
        let parts=BTreeMap::from([
            ("xl/workbook.xml".into(),format!("<workbook xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"{name}\" r:id=\"s1\"/></sheets></workbook>").into_bytes()),
            ("xl/_rels/workbook.xml.rels".into(),b"<Relationships><Relationship Id=\"s1\" Type=\"http://x/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"style\" Type=\"http://x/styles\" Target=\"styles.xml\"/><Relationship Id=\"theme\" Type=\"http://x/theme\" Target=\"theme/theme1.xml\"/></Relationships>".to_vec()),
            ("xl/styles.xml".into(),styles.as_bytes().to_vec()),
            ("xl/worksheets/sheet1.xml".into(),sheet.as_bytes().to_vec()),
            ("xl/theme/theme1.xml".into(),b"<theme><clrScheme><accent1><srgbClr val=\"FF0000\"/></accent1></clrScheme></theme>".to_vec()),
        ]);
        read_metadata(&mut book,&parts)?;
        Ok(book)
    }
    fn formats() -> &'static str {
        r#"<styleSheet><fonts><font/><font><b/></font><font><i/></font></fonts><cellXfs><xf fontId="0"/><xf fontId="1"/><xf fontId="2"/></cellXfs></styleSheet>"#
    }
    #[test]
    fn xlsx_theme_and_tint_colors() -> Result<(),String> {
        let styles=r#"<styleSheet><fonts><font><color theme="4"/></font></fonts><fills><fill><patternFill patternType="solid"><fgColor theme="4" tint="0.5"/></patternFill></fill></fills><cellXfs><xf fontId="0" fillId="0"/></cellXfs></styleSheet>"#;
        let book=metadata(styles,r#"<worksheet><sheetPr><tabColor theme="4"/></sheetPr><sheetData><row r="1"><c r="A1"/></row></sheetData></worksheet>"#)?;
        let cell=book.sheets[0].cell(0,0).ok_or("cell")?;
        assert_eq!(cell.fmt.color,[255,0,0]);
        assert_eq!(cell.fmt.fill,Some([255,128,128]));
        assert_eq!(book.sheets[0].tab_color,Some([255,0,0]));
        assert_eq!(tint_color([0,0,0],0.5),[128,128,128]);
        Ok(())
    }
    #[test]
    fn xlsx_row_and_column_format_inheritance() -> Result<(),String> {
        let book=metadata(formats(),r#"<worksheet><cols><col min="1" max="2" style="1"/></cols><sheetData><row r="1"><c r="A1"/></row><row r="2" s="2" customFormat="1"><c r="A2"/><c r="B2" s="0"/></row></sheetData></worksheet>"#)?;
        assert!(book.sheets[0].cell(0,0).ok_or("A1")?.fmt.bold);
        assert!(book.sheets[0].cell(1,0).ok_or("A2")?.fmt.italic);
        assert!(!book.sheets[0].cell(1,0).ok_or("A2")?.fmt.bold);
        assert!(!book.sheets[0].cell(1,1).ok_or("B2")?.fmt.italic);
        Ok(())
    }
    #[test]
    fn xlsx_implicit_metadata_coordinates() -> Result<(),String> {
        let book=metadata(formats(),r#"<worksheet><sheetData><row ht="15" hidden="1"><c s="1"/></row><row ht="30"><c s="0"/><c s="1"/></row><row r="4"><c r="C4" s="1"/><c s="1"/></row></sheetData></worksheet>"#)?;
        let sheet=&book.sheets[0];
        assert_eq!(sheet.row_heights.get(&0),Some(&20.0));
        assert_eq!(sheet.row_heights.get(&1),Some(&40.0));
        assert_eq!(sheet.hidden_rows.iter().copied().collect::<Vec<_>>(),vec![0]);
        assert!(sheet.cell(0,0).ok_or("A1")?.fmt.bold);
        assert!(sheet.cell(1,1).ok_or("B2")?.fmt.bold);
        assert!(sheet.cell(3,3).ok_or("D4")?.fmt.bold);
        Ok(())
    }
    #[test]
    fn pipe_separator_and_empty_field_roundtrip() -> Result<(),String> {
        assert_eq!(detect_separator("one|two"),'|');
        assert_eq!(detect_separator("single"),',');
        let rows=vec![vec!["a".into()],vec!["".into()]];
        assert_eq!(parse_csv(&to_csv(&rows,','),',')?,rows);
        assert_eq!(parse_csv(&to_csv(&[vec!["".into()]],','),',')?,vec![vec!["".to_string()]]);
        Ok(())
    }
}

fn theme_font_faces(bytes:&[u8])->Result<BTreeMap<String,String>,String> {
    let mut reader=Reader::from_str(std::str::from_utf8(bytes).map_err(|e|e.to_string())?);
    let mut faces=BTreeMap::new();let mut kind=None;
    loop {
        match reader.read_event().map_err(|e|e.to_string())? {
            Event::Start(e)|Event::Empty(e)=>{
                match local(&e).as_str() {
                    "majorFont"=>kind=Some("major"),
                    "minorFont"=>kind=Some("minor"),
                    "latin"=>if let Some(kind)=kind && let Some(face)=attrs(&e,&reader)?.get("typeface")
                        .filter(|face|!face.is_empty() && face.len()<=256) {faces.insert(kind.into(),face.clone());},
                    _=>(),
                }
            }
            Event::End(e)=>if matches!(e.local_name().as_ref(),"majorFont"|"minorFont"){kind=None;},
            Event::DocType(_)=>return Err("DTD запрещён".into()),
            Event::Eof=>break,_=>(),
        }
    }Ok(faces)
}
fn xlsx_selection(attrs:&BTreeMap<String,String>)->Option<super::model::Selection> {
    let (row,col)=parse_address(attrs.get("activeCell")?)?;
    let mut result=super::model::Selection{row,col,anchor_row:row,anchor_col:col};
    for token in attrs.get("sqref").map(String::as_str).unwrap_or("").split_whitespace() {
        let (first,last)=token.split_once(':').unwrap_or((token,token));
        let (r0,c0)=parse_address(first)?;let (r1,c1)=parse_address(last)?;
        let range=super::model::Range::new(r0,c0,r1,c1);
        if range.contains(row,col) && (row==range.r0 || row==range.r1) && (col==range.c0 || col==range.c1) {
            result.anchor_row=if row==range.r0{range.r1}else{range.r0};
            result.anchor_col=if col==range.c0{range.c1}else{range.c0};break;
        }
    }Some(result)
}
#[cfg(test)]
mod additional_xlsx_defect_tests {
    use super::*;
    use crate::package::xml_escape as esc;
    #[test]
    fn cell_xfs_inherit_missing_properties_from_the_referenced_style()->Result<(),String> {
        let bytes=br#"<styleSheet><fonts><font><name val="Georgia"/><b/><color rgb="FF123456"/></font></fonts><fills><fill><patternFill patternType="solid"><fgColor rgb="FFAABBCC"/></patternFill></fill></fills><cellStyleXfs><xf fontId="0" fillId="0" numFmtId="2"><alignment horizontal="right" wrapText="1"/></xf></cellStyleXfs><cellXfs><xf xfId="0"/></cellXfs></styleSheet>"#;
        let formats=styles_with_theme(bytes,&[])?;let fmt=formats.first().ok_or("format")?;
        assert_eq!(fmt.font,"Georgia");assert!(fmt.bold);assert_eq!(fmt.fill,Some([170,187,204]));
        assert_eq!(fmt.number,"0.00");assert_eq!(fmt.align,Align::Right);assert!(fmt.wrap);Ok(())
    }
    #[test]
    fn spreadsheet_font_scheme_resolves_custom_theme_faces()->Result<(),String> {
        let faces=theme_font_faces(br#"<theme><fontScheme><majorFont><latin typeface="Georgia"/></majorFont><minorFont><latin typeface="Verdana"/></minorFont></fontScheme></theme>"#)?;
        let formats=styles_with_theme_fonts(br#"<styleSheet><fonts><font><name val="Calibri"/><scheme val="minor"/></font><font><scheme val="major"/></font></fonts><cellXfs><xf fontId="0"/><xf fontId="1"/></cellXfs></styleSheet>"#,&[],&faces)?;
        assert_eq!(formats[0].font,"Verdana");assert_eq!(formats[1].font,"Georgia");Ok(())
    }
    #[test]
    fn worksheet_selection_uses_the_active_pane_and_preserves_the_anchor()->Result<(),String> {
        let mut book=Book::default();let name=book.sheets[0].name.clone();
        let parts=BTreeMap::from([
            ("xl/workbook.xml".into(),format!(r#"<workbook><sheets><sheet name="{}" r:id="s"/></sheets></workbook>"#,esc(&name)).into_bytes()),
            ("xl/_rels/workbook.xml.rels".into(),br#"<Relationships><Relationship Id="s" Target="worksheets/sheet1.xml"/></Relationships>"#.to_vec()),
            ("xl/worksheets/sheet1.xml".into(),br#"<worksheet><sheetViews><sheetView><pane state="frozen" activePane="bottomRight"/><selection pane="topLeft" activeCell="A1"/><selection pane="bottomRight" activeCell="E10" sqref="D8:E10"/></sheetView></sheetViews></worksheet>"#.to_vec()),
        ]);
        read_metadata(&mut book,&parts)?;
        assert_eq!((book.selection.row,book.selection.col,book.selection.anchor_row,book.selection.anchor_col),(9,4,7,3));Ok(())
    }
    #[test]
    fn linked_chart_titles_do_not_expand_the_plot_data_range()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();let name=book.sheets[0].name.clone();
        book.set_cell(id,9,25,"Revenue")?;book.set_cell(id,0,0,"1")?;book.set_cell(id,1,0,"2")?;
        let chart = format!("<chartSpace><chart><title><strRef><f>'{}'!$Z$10</f></strRef></title><plotArea><barChart><ser><val><numRef><f>'{}'!$A$1:$A$2</f></numRef></val></ser></barChart></plotArea></chart></chartSpace>",esc(&name),esc(&name));
        let parts = chart_fixture_parts(&book, 0, "art/custom.xml", &chart);
        read_charts(&mut book,&parts)?;
        let chart=book.sheets[0].charts.first().ok_or("chart")?;
        assert_eq!(chart.title,"Revenue");assert_eq!(chart.range,super::super::model::Range::new(0,0,1,0));Ok(())
    }
}

fn html_push(output:&mut String,value:&str,limit:usize)->Result<(),String> {
    if output.len().checked_add(value.len()).is_none_or(|length|length>limit) {return Err("HTML превышает лимит размера экспорта".into());}
    output.push_str(value);Ok(())
}
fn html_font(value:&str)->String {
    let mut escaped=String::from("'");
    for ch in value.chars() {
        match ch {'\\'=>escaped.push_str("\\\\"),'\''=>escaped.push_str("\\'"),
            '\n'|'\r'=>escaped.push_str("\\A "),ch=>escaped.push(ch)}
    }
    escaped.push('\'');escaped
}
fn write_html_book(book:&Book,cell_limit:u64,byte_limit:usize)->Result<Vec<u8>,String> {
    let mut dense=0u64;let mut extents=Vec::new();
    for sheet in book.sheets.iter().filter(|sheet|!sheet.hidden) {
        let (mut rows,mut cols)=sheet.extent();
        for range in &sheet.merges {rows=rows.max(range.r1);cols=cols.max(range.c1);}
        let cells=(u64::from(rows)+1)*(u64::from(cols)+1);
        if cells>MAX_RANGE_CELLS as u64 {return Err("Слишком большой диапазон HTML".into());}
        dense=dense.checked_add(cells).ok_or("Слишком большой HTML")?;
        if dense>cell_limit {return Err("Суммарный диапазон HTML превышает безопасный лимит".into());}
        extents.push((sheet,rows,cols));
    }
    let mut out=String::new();
    html_push(&mut out,"<!doctype html><html lang=\"ru\"><head><meta charset=\"utf-8\"><style>table{border-collapse:collapse;table-layout:fixed}td{padding:4px}img{max-width:100%}</style></head><body>",byte_limit)?;
    for (sheet,rows,cols) in extents {
        let mut merged=BTreeMap::new();let mut covered=0u64;
        for range in &sheet.merges {
            covered=covered.saturating_add(u64::from(range.r1-range.r0+1)*u64::from(range.c1-range.c0+1));
            if covered>cell_limit {return Err("Слишком большой индекс объединённых ячеек HTML".into());}
            for row in range.r0..=range.r1 {for col in range.c0..=range.c1 {merged.entry((row,col)).or_insert(range);}}
        }
        html_push(&mut out,&format!("<section><h2>{}</h2><table><colgroup>",crate::package::xml_escape(&sheet.name)),byte_limit)?;
        for col in (0..=cols).filter(|col|!sheet.hidden_cols.contains(col)) {
            html_push(&mut out,&format!("<col style=\"width:{}px\">",sheet.widths.get(&col).copied().unwrap_or(sheet.default_col_width)),byte_limit)?;
        }
        html_push(&mut out,"</colgroup>",byte_limit)?;
        for row in (0..=rows).filter(|row|!sheet.hidden_rows.contains(row)) {
            html_push(&mut out,&format!("<tr style=\"height:{}px\">",sheet.row_heights.get(&row).copied().unwrap_or(sheet.default_row_height)),byte_limit)?;
            for col in (0..=cols).filter(|col|!sheet.hidden_cols.contains(col)) {
                let merge=merged.get(&(row,col)).copied();
                let display_anchor = merge.and_then(|range| {
                    Some(((range.r0..=range.r1).find(|row| !sheet.hidden_rows.contains(row))?,
                        (range.c0..=range.c1).find(|col| !sheet.hidden_cols.contains(col))?))
                });
                if merge.is_some() && display_anchor != Some((row, col)) { continue; }
                let (source_row, source_col) = merge.map_or((row, col), |range| (range.r0, range.c0));
                let span=merge.map(|range|format!(" rowspan=\"{}\" colspan=\"{}\"",
                    (range.r0..=range.r1).filter(|row|!sheet.hidden_rows.contains(row)).count(),
                    (range.c0..=range.c1).filter(|col|!sheet.hidden_cols.contains(col)).count())).unwrap_or_default();
                let mut format=sheet.cell(source_row,source_col).map(|cell|cell.fmt.clone()).unwrap_or_default();
                if let Some(rgb)=sheet.cell(source_row,source_col).and_then(|cell|super::format::color(&cell.value,&format.number)) {format.color=rgb;}
                let align=match format.align {Align::Left=>"left",Align::Center=>"center",Align::Right=>"right",
                    Align::General=>if sheet.cell(source_row,source_col).is_some_and(|cell|matches!(cell.value,Value::Number(_))) {"right"}else{"left"}};
                let decoration=format!("{}{}",if format.underline {"underline "}else{""},if format.strike {"line-through"}else{""});
                let mut style=format!("color:#{:02X}{:02X}{:02X};font-family:{};font-size:{}pt;font-weight:{};font-style:{};text-decoration:{};text-align:{};white-space:{};border:{}",
                    format.color[0],format.color[1],format.color[2],html_font(&format.font),format.size,
                    if format.bold {"bold"}else{"normal"},if format.italic {"italic"}else{"normal"},decoration,align,
                    if format.wrap {"pre-wrap"}else{"pre"},if format.border {"1px solid #666"}else{"none"});
                if let Some(fill)=format.fill {style.push_str(&format!(";background:#{:02X}{:02X}{:02X}",fill[0],fill[1],fill[2]));}
                html_push(&mut out,&format!("<td{span} style=\"{}\">{}</td>",
                    crate::package::xml_escape(&style),crate::package::xml_escape(&sheet.display(source_row,source_col,book.locale_ru))),byte_limit)?;
            }
            html_push(&mut out,"</tr>",byte_limit)?;
        }
        html_push(&mut out,"</table>",byte_limit)?;
        for image in &sheet.images {
            let encoded=image.bytes.len().div_ceil(3).saturating_mul(4);
            if out.len().saturating_add(encoded).saturating_add(image.alt.len().saturating_mul(6)).saturating_add(512)>byte_limit {
                return Err("Изображения превышают лимит HTML".into());
            }
            let mime=if matches!(image::guess_format(&image.bytes),Ok(image::ImageFormat::Jpeg)) {"image/jpeg"}else{"image/png"};
            html_push(&mut out,&format!("<figure><img src=\"data:{mime};base64,{}\" alt=\"{}\" width=\"{}\" height=\"{}\"><figcaption>{} · {}</figcaption></figure>",
                html_image_base64(&image.bytes),crate::package::xml_escape(&image.alt),image.width,image.height,
                super::model::address(image.row,image.col),crate::package::xml_escape(&image.alt)),byte_limit)?;
        }
        html_push(&mut out,"</section>",byte_limit)?;
    }
    html_push(&mut out,"</body></html>",byte_limit)?;Ok(out.into_bytes())
}
fn html_image_base64(bytes:&[u8])->String {
    const ALPHABET:&[u8;64]=b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result=String::with_capacity(bytes.len().div_ceil(3)*4);
    for chunk in bytes.chunks(3) {
        let value=(u32::from(chunk[0])<<16)|(u32::from(*chunk.get(1).unwrap_or(&0))<<8)|u32::from(*chunk.get(2).unwrap_or(&0));
        result.push(char::from(ALPHABET[((value>>18)&63) as usize]));result.push(char::from(ALPHABET[((value>>12)&63) as usize]));
        result.push(if chunk.len()>1 {char::from(ALPHABET[((value>>6)&63) as usize])}else{'='});
        result.push(if chunk.len()>2 {char::from(ALPHABET[(value&63) as usize])}else{'='});
    }result
}
#[cfg(test)]
mod next_html_export_tests {
    use super::*;
    fn text(book:&Book)->Result<String,String>{String::from_utf8(write(book,"html")?).map_err(|e|e.to_string())}
    #[test]
    fn html_export_preserves_merged_ranges_and_does_not_repeat_covered_cells()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,0,0,"merged")?;
        book.sheets[0].merges.push(super::super::model::Range::new(0,0,1,1));
        let output=text(&book)?;
        assert!(output.contains("rowspan=\"2\" colspan=\"2\""));assert_eq!(output.matches("<td").count(),1);Ok(())
    }
    #[test]
    fn html_export_keeps_shared_text_format_and_row_column_geometry()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,0,0,"text")?;
        let cell=book.sheets[0].cell_mut(0,0).ok_or("cell")?;
        cell.fmt.font="A';color:red;/*".into();cell.fmt.underline=true;cell.fmt.strike=true;
        cell.fmt.align=Align::Right;cell.fmt.wrap=true;cell.fmt.border=true;
        book.sheets[0].widths.insert(0,150.0);book.sheets[0].row_heights.insert(0,42.0);
        let output=text(&book)?;
        assert!(output.contains("width:150px"));assert!(output.contains("height:42px"));
        assert!(output.contains("text-decoration:underline line-through"));assert!(output.contains("text-align:right"));
        assert!(output.contains("white-space:pre-wrap"));assert!(output.contains("font-family:&apos;A\\&apos;"));Ok(())
    }
    #[test]
    fn html_export_includes_sheet_images_and_their_cell_anchor()->Result<(),String> {
        let mut png=Cursor::new(Vec::new());image::DynamicImage::new_rgba8(2,2)
            .write_to(&mut png,image::ImageFormat::Png).map_err(|e|e.to_string())?;
        let mut book=Book::default();let id=book.active_id();book.add_image(id,1,2,png.into_inner())?;
        let output=text(&book)?;assert!(output.contains("data:image/png;base64,"));
        assert!(output.contains("C2 ·"));Ok(())
    }
    #[test]
    fn html_export_limits_cover_the_whole_workbook_and_output_bytes()->Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.set_cell(id,9,9,"x")?;
        let second=book.add_sheet("Second")?;book.set_cell(second,9,9,"y")?;
        assert!(write_html_book(&book,150,MAX_FILE).is_err());
        assert!(write_html_book(&book,1000,500).is_err());Ok(())
    }
}

#[cfg(test)]
mod next_csv_bom_tests {
    #[test]
    fn a_declared_utf8_csv_never_falls_back_to_windows1251() {
        assert!(super::decode_text(&[0xef,0xbb,0xbf,0xff]).is_err());
        assert_eq!(super::decode_text("Привет".as_bytes()),Ok("Привет".into()));
    }
}

fn write_text_book(book: &Book, ext: &str, limit: usize) -> Result<Vec<u8>, String> {
    let sheet = book.active_sheet().ok_or("Нет активного листа")?;
    let (last_row, last_col) = sheet.extent();
    let cells = (u64::from(last_row) + 1) * (u64::from(last_col) + 1);
    if cells > MAX_RANGE_CELLS as u64 { return Err("Экспортируемый диапазон превышает100000 ячеек".into()); }
    let mut output = String::new();
    let push = |output: &mut String, text: &str| -> Result<(), String> {
        if output.len().checked_add(text.len()).is_none_or(|bytes| bytes > limit) {
            return Err("Текстовый экспорт превышает лимит размера файла".into());
        }
        output.push_str(text); Ok(())
    };
    if ext == "md" { push(&mut output, &format!("# {}\n\n", markdown_text(&sheet.name)))?; }
    else { push(&mut output, "\u{feff}")?; }
    let separator = if ext == "tsv" { '\t' } else { ';' };
    for row in 0..=last_row {
        if ext == "md" { push(&mut output, "| ")?; }
        for col in 0..=last_col {
            if col > 0 { push(&mut output, if ext == "md" { " | " } else if separator == '\t' { "\t" } else { ";" })?; }
            let value = sheet.display(row, col, book.locale_ru);
            let rendered = if ext == "md" { markdown_text(&value) }
                else if value.contains([separator, '\n', '\r', '"']) || last_col == 0 && value.is_empty() {
                    format!("\"{}\"", value.replace('"', "\"\""))
                } else { value };
            push(&mut output, &rendered)?;
        }
        if ext == "md" {
            push(&mut output, " |\n")?;
            if row == 0 {
                push(&mut output, "|")?;
                for _ in 0..=last_col { push(&mut output, " --- |")?; }
                push(&mut output, "\n")?;
            }
        } else if row < last_row { push(&mut output, "\r\n")?; }
    }
    Ok(output.into_bytes())
}
#[cfg(test)]
mod hundred_export_tests {
    use super::*;
    #[test]
    fn xlsx_export_preserves_the_active_selection() -> Result<(), String> {
        let mut book = Book::default();
        book.selection = super::super::model::Selection { row: 4, col: 3, anchor_row: 2, anchor_col: 1 };
        let decoded = read(&write(&book, "xlsx")?, "xlsx")?;
        assert_eq!(decoded.selection, book.selection);
        Ok(())
    }
    #[test]
    fn style_xf_explicitly_disabled_components_are_not_inherited() -> Result<(), String> {
        let xml = br#"<styleSheet><fonts count="2"><font/><font><b/><name val="Decorative"/></font></fonts>
            <fills><fill/><fill><patternFill patternType="solid"><fgColor rgb="FFFF0000"/></patternFill></fill></fills>
            <borders><border/><border><left style="thin"/></border></borders>
            <cellStyleXfs><xf fontId="1" fillId="1" borderId="1" numFmtId="2"
                applyFont="0" applyFill="0" applyBorder="0" applyNumberFormat="0" applyAlignment="0"><alignment horizontal="right"/></xf></cellStyleXfs>
            <cellXfs><xf xfId="0"/></cellXfs></styleSheet>"#;
        let formats = styles(xml)?;
        assert_eq!(formats.first(), Some(&CellFormat::default()));
        Ok(())
    }
    #[test]
    fn csv_and_markdown_enforce_the_output_budget_during_serialization() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "'<<<<\"\"\"\"")?;
        for ext in ["csv", "tsv", "md"] {
            assert!(write_text_book(&book, ext, 8).is_err());
            assert!(write_text_book(&book, ext, 1024).is_ok());
        }
        Ok(())
    }
    #[test]
    fn html_merged_content_remains_visible_when_its_anchor_column_is_hidden() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        book.set_cell(id, 0, 0, "Visible merged title")?;
        book.merge(id, super::super::model::Range::new(0, 0, 0, 1))?;
        book.active_sheet_mut().ok_or("sheet")?.hidden_cols.insert(0);
        let html = String::from_utf8(write(&book, "html")?).map_err(|error| error.to_string())?;
        assert!(html.contains("Visible merged title"));
        assert_eq!(html.matches("Visible merged title").count(), 1);
        assert!(html.contains("colspan=\"1\""));
        Ok(())
    }
}

#[cfg(test)]
fn chart_fixture_parts(book: &Book, owner: usize, chart_path: &str, chart: &str) -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("xl/workbook.xml".into(), format!(r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="{}" r:id="s"/></sheets></workbook>"#, crate::package::xml_escape(&book.sheets[owner].name)).into_bytes()),
        ("xl/_rels/workbook.xml.rels".into(), br#"<Relationships><Relationship Id="s" Type="worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#.to_vec()),
        ("xl/worksheets/sheet1.xml".into(), br#"<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><drawing r:id="d"/></worksheet>"#.to_vec()),
        ("xl/worksheets/_rels/sheet1.xml.rels".into(), br#"<Relationships><Relationship Id="d" Type="office/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#.to_vec()),
        ("xl/drawings/drawing1.xml".into(), br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:oneCellAnchor><xdr:from><xdr:col>4</xdr:col><xdr:row>5</xdr:row></xdr:from><xdr:ext cx="1905000" cy="952500"/><xdr:graphicFrame><c:chart r:id="c"/></xdr:graphicFrame></xdr:oneCellAnchor></xdr:wsDr>"#.to_vec()),
        ("xl/drawings/_rels/drawing1.xml.rels".into(), format!(r#"<Relationships><Relationship Id="c" Type="office/chart" Target="/{}"/></Relationships>"#, crate::package::xml_escape(chart_path)).into_bytes()),
        (chart_path.into(), chart.as_bytes().to_vec()),
    ])
}
#[cfg(test)]
mod hundred_chart_picture_tests {
    use super::*;
    #[test]
    fn linked_chart_parts_use_the_drawing_owner_and_geometry() -> Result<(), String> {
        let mut book = Book::default();
        let name = book.sheets[0].name.clone();
        let chart = format!("<chartSpace><chart><plotArea><barChart><ser><val><numRef><f>'{name}'!A1:A2</f></numRef></val></ser></barChart></plotArea></chart></chartSpace>");
        let parts = chart_fixture_parts(&book, 0, "art/quarterly.xml", &chart);
        read_charts(&mut book, &parts)?;
        let saved = book.sheets[0].charts.first().ok_or("chart")?;
        assert_eq!((saved.row, saved.col, saved.width, saved.height), (5, 4, 200.0, 100.0));
        book.add_sheet("Other")?;
        let parts = chart_fixture_parts(&book, 1, "art/other.xml", &chart);
        assert!(read_charts(&mut book, &parts).is_err());
        Ok(())
    }
    #[test]
    fn axis_titles_do_not_become_the_chart_title() -> Result<(), String> {
        let mut book = Book::default();
        let name = book.sheets[0].name.clone();
        let chart = format!("<chartSpace><chart><title><tx><rich><t>Sales</t></rich></tx></title><plotArea><barChart><ser><val><numRef><f>'{name}'!A1:A2</f></numRef></val></ser></barChart><catAx><title><tx><rich><t>Year</t></rich></tx></title></catAx></plotArea></chart></chartSpace>");
        read_charts(&mut book, &chart_fixture_parts(&Book::default(), 0, "art/title.xml", &chart))?;
        assert_eq!(book.sheets[0].charts.first().ok_or("chart")?.title, "Sales");
        Ok(())
    }
    #[test]
    fn negative_drawing_offsets_are_normalized_into_the_previous_cell() -> Result<(), String> {
        let xml = r#"<wsDr xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><oneCellAnchor><from><col>1</col><row>1</row><colOff>-95250</colOff><rowOff>-9525</rowOff></from><ext cx="95250" cy="95250"/><pic><blip r:embed="p"/></pic></oneCellAnchor></wsDr>"#;
        let anchors = drawing_pictures(xml)?;
        let anchor = anchors.first().ok_or("anchor")?;
        let geometry = picture_geometry(&super::super::model::Sheet::default(), anchor);
        assert_eq!(geometry, (0, 0, 86.0, 25.0, 10.0, 10.0));
        Ok(())
    }
}

#[cfg(test)]
mod hundred_chart_unicode_tests {
    use super::*;
    #[test]
    fn chart_sheet_references_resolve_cyrillic_case_insensitively() -> Result<(), String> {
        let mut book = Book::default();
        let id = book.active_id();
        let name = book.sheets[0].name.to_uppercase();
        book.set_cell(id, 9, 25, "Доход")?;
        let chart = format!("<chartSpace><chart><title><strRef><f>'{name}'!Z10</f></strRef></title><plotArea><barChart><ser><val><numRef><f>'{name}'!A1:A2</f></numRef></val></ser></barChart></plotArea></chart></chartSpace>");
        let parts = chart_fixture_parts(&book, 0, "art/unicode.xml", &chart);
        read_charts(&mut book, &parts)?;
        assert_eq!(book.sheets[0].charts.len(), 1);
        assert_eq!(book.sheets[0].charts[0].title, "Доход");
        Ok(())
    }
}

#[cfg(test)]
mod phase200_preflight_defects {
    use super::*;
    #[test]
    fn xml_entity_budgets_use_decoded_utf8_lengths() -> Result<(),String> {
        let xml=format!("<sst><si><t>{}</t></si></sst>","&amp;".repeat(10000));
        assert_eq!(xml_bounds(xml.as_bytes())?,Some(vec![10000]));
        let xml=format!("<worksheet><sheetData><row><c><f>\"{}\"</f></c></row></sheetData></worksheet>","&amp;".repeat(10000));
        assert!(worksheet_bounds(xml.as_bytes(),&[]).is_ok());
        let xml=br#"<worksheet><sheetData><row><c t="str"><v>A&amp;B&#x410;</v></c></row></sheetData></worksheet>"#;
        assert_eq!(worksheet_bounds(xml,&[])?.1,5);
        Ok(())
    }
    #[test]
    fn formula_string_caches_are_charged_before_calamine_import() -> Result<(),String> {
        let xml=br#"<worksheet><sheetData><row><c t="str"><f>1</f><v>cached text</v></c></row></sheetData></worksheet>"#;
        assert_eq!(worksheet_bounds(xml,&[])?.1,258+"cached text".len());
        let xml=br#"<worksheet><sheetData><row><c t="str"><v><![CDATA[cached text]]></v></c></row></sheetData></worksheet>"#;
        assert_eq!(worksheet_bounds(xml,&[])?.1,"cached text".len());
        Ok(())
    }
    #[test]
    fn inline_rich_text_checks_the_combined_cell_length() {
        let xml=format!("<worksheet><sheetData><row><c t=\"inlineStr\"><is><r><t>{}</t></r><r><t>{}</t></r></is></c></row></sheetData></worksheet>","a".repeat(20000),"b".repeat(20000));
        assert!(worksheet_bounds(xml.as_bytes(),&[]).is_err());
        let xml=xml.replace(&"b".repeat(20000),&"b".repeat(10000));
        assert!(worksheet_bounds(xml.as_bytes(),&[]).is_ok());
    }
    #[test]
    fn uppercase_xml_parts_receive_the_same_resource_preflight() {
        let bytes=br#"<worksheet><dimension ref="A1:XFD1048576"/></worksheet>"#.to_vec();
        let parts=BTreeMap::from([("custom/Sheet.XML".into(),bytes)]);
        assert!(preflight(&parts).is_err());
        let parts=BTreeMap::from([("custom/Strings.XML".into(),br#"<sst uniqueCount="1000001"/>"#.to_vec())]);
        assert!(preflight(&parts).is_err());
    }
}

#[cfg(test)]
mod phase200_drawing_defects {
    use super::*;
    fn parts(drawing:&str,referenced:bool)->BTreeMap<String,Vec<u8>> {
        BTreeMap::from([
            ("xl/worksheets/sheet1.xml".into(),if referenced {
                br#"<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><drawing r:id="d"/></worksheet>"#.to_vec()
            } else {b"<worksheet/>".to_vec()}),
            ("xl/worksheets/_rels/sheet1.xml.rels".into(),br#"<Relationships><Relationship Id="d" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#.to_vec()),
            ("xl/drawings/drawing1.xml".into(),drawing.as_bytes().to_vec()),
            ("xl/drawings/_rels/drawing1.xml.rels".into(),br#"<Relationships><Relationship Id="p" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/p.png"/></Relationships>"#.to_vec()),
            ("xl/media/p.png".into(),b"broken image".to_vec()),
        ])
    }
    const DRAWING:&str=r#"<wsDr xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><oneCellAnchor><from><col>0</col><row>0</row></from><ext cx="9525" cy="9525"/><pic><blip r:embed="p"/></pic></oneCellAnchor></wsDr>"#;
    #[test]
    fn referenced_invalid_pictures_fail_instead_of_disappearing() {
        let mut book=Book::default();let id=book.active_id();
        assert!(read_sheet_images(&mut book,id,"xl/worksheets/sheet1.xml",&parts(DRAWING,true)).is_err());
        assert!(read_sheet_images(&mut book,id,"xl/worksheets/sheet1.xml",&parts("<wsDr><oneCellAnchor>",true)).is_err());
    }
    #[test]
    fn orphan_drawing_relationships_do_not_import_unreferenced_content() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        read_sheet_images(&mut book,id,"xl/worksheets/sheet1.xml",&parts(DRAWING,false))?;
        assert!(book.sheets[0].images.is_empty());
        Ok(())
    }
}

#[cfg(test)]
mod phase200_conditional_defect {
    use super::*;
    #[test]
    fn numeric_conditional_fills_round_trip_through_xlsx() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_cell(id,0,0,"20")?;
        for (index,operator) in [">","<","=",">=","<="].into_iter().enumerate() {
            book.sheets[0].conditional.push(ConditionalRule{
                range:super::super::model::Range::new(0,0,1,1),operator:operator.into(),
                threshold:index as f64-2.0,fill:[index as u8,100,200],
            });
        }
        let imported=read(&write(&book,"xlsx")?,"xlsx")?;
        assert_eq!(imported.sheets[0].conditional,book.sheets[0].conditional);
        let formats=differential_fills(br#"<styleSheet><dxfs><dxf/><dxf><fill><patternFill><bgColor rgb="FFABCDEF"/></patternFill></fill></dxf></dxfs></styleSheet>"#,&[],&[])?;
        assert_eq!(formats,vec![None,Some([0xAB,0xCD,0xEF])]);
        assert!(conditional_rules(br#"<worksheet><conditionalFormatting sqref="A1"><cfRule type="expression" dxfId="1" priority="1"><formula>A1&gt;2</formula></cfRule></conditionalFormatting></worksheet>"#,&formats).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod phase200_default_geometry_defect {
    use super::*;
    #[test]
    fn worksheet_default_geometry_survives_import_export_and_old_native_files() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();book.sheets[0].name="Sheet1".into();
        let parts=BTreeMap::from([
            ("xl/workbook.xml".into(),br#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" r:id="s"/></sheets></workbook>"#.to_vec()),
            ("xl/_rels/workbook.xml.rels".into(),br#"<Relationships><Relationship Id="s" Target="worksheets/sheet1.xml"/></Relationships>"#.to_vec()),
            ("xl/worksheets/sheet1.xml".into(),br#"<worksheet><sheetFormatPr defaultColWidth="20" defaultRowHeight="30"/></worksheet>"#.to_vec()),
        ]);
        read_metadata(&mut book,&parts)?;
        assert_eq!(book.sheets[0].default_col_width,140.0);
        assert_eq!(book.sheets[0].default_row_height,40.0);
        book.set_cell(id,0,0,"text")?;
        let imported=read(&write(&book,"xlsx")?,"xlsx")?;
        assert_eq!(imported.sheets[0].default_col_width,140.0);
        assert_eq!(imported.sheets[0].default_row_height,40.0);
        let old:super::super::model::Sheet=serde_json::from_str("{}").map_err(|error|error.to_string())?;
        assert_eq!((old.default_col_width,old.default_row_height),(96.0,26.0));
        Ok(())
    }
}

#[cfg(test)]
mod phase200_chart_roles_defect {
    use super::*;
    #[test]
    fn chart_round_trip_keeps_header_and_category_roles() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_range(id,0,0,&[vec!["Name".into(),"Sales".into()],vec!["first".into(),"10".into()],vec!["last".into(),"20".into()]])?;
        book.add_chart(id,ChartKind::Column,super::super::model::Range::new(0,0,2,1),"Sales")?;
        let imported=read(&write(&book,"xlsx")?,"xlsx")?;
        let chart=imported.sheets[0].charts.first().ok_or("chart")?;
        assert_eq!(chart.range,book.sheets[0].charts[0].range);
        assert!(chart.has_headers&&chart.has_categories);
        book.sheets[0].charts[0].has_headers=false;
        book.sheets[0].charts[0].has_categories=false;
        let imported=read(&write(&book,"xlsx")?,"xlsx")?;
        let chart=imported.sheets[0].charts.first().ok_or("chart")?;
        assert!(!chart.has_headers&&!chart.has_categories);
        assert_eq!(chart.range,book.sheets[0].charts[0].range);
        Ok(())
    }
}

#[cfg(test)]
mod phase200_chart_integrity_defects {
    use super::*;
    fn chart(series:&str,kind:&str)->String {
        format!("<chartSpace><chart><plotArea><{kind}>{series}</{kind}></plotArea></chart></chartSpace>")
    }
    #[test]
    fn referenced_chart_parts_and_sources_cannot_silently_disappear() -> Result<(),String> {
        let mut book=Book::default();
        let xml=chart("<ser><val><numRef><f>A1:A2</f></numRef></val></ser>","barChart");
        let mut parts=chart_fixture_parts(&book,0,"art/part.xml",&xml);
        parts.remove("art/part.xml");
        assert!(read_charts(&mut book,&parts).is_err());
        let xml=chart("<ser><val><numLit><pt idx=\"0\"><v>1</v></pt></numLit></val></ser>","barChart");
        assert!(read_charts(&mut book,&chart_fixture_parts(&Book::default(),0,"art/literal.xml",&xml)).is_err());
        let xml=chart("<ser><val><numRef><f>A1:A2</f></numRef></val></ser>","barChart");
        read_charts(&mut book,&chart_fixture_parts(&Book::default(),0,"art/local.xml",&xml))?;
        assert_eq!(book.sheets[0].charts.len(),1);
        Ok(())
    }
    #[test]
    fn unsupported_chart_types_are_not_falsely_imported_as_columns() {
        let mut book=Book::default();let name=book.sheets[0].name.clone();
        let series=format!("<ser><val><numRef><f>'{name}'!A1:A2</f></numRef></val></ser>");
        let xml=chart(&series,"radarChart");
        assert!(read_charts(&mut book,&chart_fixture_parts(&Book::default(),0,"art/radar.xml",&xml)).is_err());
        let xml=format!("<chartSpace><chart><plotArea><barChart>{series}</barChart><lineChart>{series}</lineChart></plotArea></chart></chartSpace>");
        assert!(read_charts(&mut book,&chart_fixture_parts(&Book::default(),0,"art/mixed.xml",&xml)).is_err());
        assert!(book.sheets[0].charts.is_empty());
    }
    #[test]
    fn disjoint_chart_sources_do_not_pull_intervening_unrelated_cells() {
        let mut book=Book::default();let name=book.sheets[0].name.clone();
        let series=format!("<ser><val><numRef><f>'{name}'!A1:A2</f></numRef></val></ser><ser><val><numRef><f>'{name}'!C1:C2</f></numRef></val></ser>");
        assert!(read_charts(&mut book,&chart_fixture_parts(&Book::default(),0,"art/gap.xml",&chart(&series,"barChart"))).is_err());
        assert!(book.sheets[0].charts.is_empty());
    }
    #[test]
    fn a_single_column_scatter_chart_exports_its_visible_x_values() -> Result<(),String> {
        let mut book=Book::default();let id=book.active_id();
        book.set_range(id,0,0,&[vec!["Value".into()],vec!["1".into()],vec!["2".into()]])?;
        book.add_chart(id,ChartKind::Scatter,super::super::model::Range::new(0,0,2,0),"One")?;
        let bytes=write(&book,"xlsx")?;
        let imported=read(&bytes,"xlsx")?;
        assert_eq!(imported.sheets[0].charts.first().ok_or("chart")?.kind,ChartKind::Scatter);
        Ok(())
    }
}
