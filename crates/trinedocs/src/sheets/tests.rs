use super::{formula, model::*};
#[test]
fn numbervalue_uses_locale_and_validates_separators() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=NUMBERVALUE(\"1 234,5\")", Value::Number(1234.5)),
        ("=NUMBERVALUE(\"\")", Value::Number(0.0)),
        (
            "=NUMBERVALUE(\"1.234,5\",\",extra\",\".extra\")",
            Value::Number(1234.5),
        ),
        (
            "=NUMBERVALUE(\"1.2,3\",\".\",\",\")",
            Value::Error("#VALUE!".into()),
        ),
        (
            "=NUMBERVALUE(\"1.5\",\".\",\".\")",
            Value::Error("#VALUE!".into()),
        ),
        (
            "=NUMBERVALUE(\"1.5\",\"\",\",\")",
            Value::Error("#VALUE!".into()),
        ),
    ] {
        book.set_cell(id, 0, 0, input)?;
        assert_eq!(
            book.active_sheet()
                .and_then(|sheet| sheet.cell(0, 0))
                .map(|cell| &cell.value),
            Some(&expected),
            "{input}"
        );
    }
    book.locale_ru = false;
    book.set_cell(id, 0, 0, "=NUMBERVALUE(\"1,234.5\")")?;
    assert_eq!(
        book.active_sheet()
            .and_then(|sheet| sheet.cell(0, 0))
            .map(|cell| &cell.value),
        Some(&Value::Number(1234.5))
    );
    Ok(())
}
#[test]
fn ctrl_arrow_data_edge_follows_excel_blocks() -> Result<(), String> {
    let book = seeded()?;
    let sheet = book.active_sheet().ok_or("sheet")?;
    assert_eq!(sheet.data_edge(0, 0, 1, 0), (2, 0));
    assert_eq!(sheet.data_edge(2, 0, 1, 0), (MAX_ROWS - 1, 0));
    assert_eq!(sheet.data_edge(2, 3, -1, 0), (1, 3));
    assert_eq!(sheet.data_edge(9, 0, -1, 0), (2, 0));
    assert_eq!(sheet.data_edge(0, 0, -1, 0), (0, 0));
    assert_eq!(sheet.data_edge(0, 0, 0, 1), (0, 5));
    assert_eq!(sheet.data_edge(0, 5, 0, 1), (0, MAX_COLS - 1));
    assert_eq!(sheet.data_edge(2, 9, 0, -1), (2, 2));
    assert_eq!(sheet.data_edge(1, 0, 0, -1), (1, 0));
    Ok(())
}
#[test]
fn subtotal_skips_nested_subtotals_and_hidden_rows() -> Result<(), String> {
    let mut book = seeded()?;
    let id = book.active_id();
    book.set_cell(id, 3, 0, "=SUBTOTAL(9,A1:A3)")?;
    book.set_cell(id, 4, 0, "=ПРОМЕЖУТОЧНЫЕ.ИТОГИ(9;A1:A4)")?;
    let value = |book: &Book, row| {
        book.active_sheet()
            .and_then(|sheet| sheet.cell(row, 0))
            .map(|cell| cell.value.clone())
    };
    assert_eq!(value(&book, 3), Some(Value::Number(6.0)));
    assert_eq!(value(&book, 4), Some(Value::Number(6.0)));
    book.sheets.get_mut(0).ok_or("sheet")?.hidden_rows.insert(1);
    assert_eq!(
        formula::evaluate(&book, id, "=SUBTOTAL(109,A1:A3)"),
        Value::Number(4.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=SUBTOTAL(9,A1:A3)"),
        Value::Number(6.0)
    );
    Ok(())
}
#[test]
fn images_follow_structural_edits_without_losing_data() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    book.add_image(id, 3, 4, png.into_inner())?;
    book.edit_dimension(id, 1, 2, true, false)?;
    book.edit_dimension(id, 2, 3, false, false)?;
    let picture = book
        .active_sheet()
        .and_then(|sheet| sheet.images.first())
        .ok_or("image")?;
    assert_eq!((picture.row, picture.col), (5, 7));
    let bytes = picture.bytes.clone();
    book.edit_dimension(id, 4, 2, true, true)?;
    book.edit_dimension(id, 6, 2, false, true)?;
    let picture = book
        .active_sheet()
        .and_then(|sheet| sheet.images.first())
        .ok_or("image")?;
    assert_eq!((picture.row, picture.col), (4, 6));
    assert_eq!(picture.bytes, bytes);
    for rows in [true, false] {
        if let Some(picture) = book
            .active_sheet_mut()
            .and_then(|sheet| sheet.images.first_mut())
        {
            if rows {
                picture.row = MAX_ROWS - 1;
            } else {
                picture.col = MAX_COLS - 1;
            }
        }
        let before = book.clone();
        assert!(book.edit_dimension(id, 0, 1, rows, false).is_err());
        assert_eq!(book, before);
    }
    book.validate()?;
    Ok(())
}

#[test]
fn invalid_formats_and_failed_replace_leave_book_unchanged() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "a")?;
    book.set_cell(id, 0, 1, &"a".repeat(16384))?;
    let before = book.clone();
    assert!(book.replace_all(id, "a", "aa").is_err());
    assert_eq!(book, before);
    for size in [0.0, 500.0, f32::INFINITY] {
        assert!(
            book.set_format(
                id,
                Range::new(0, 0, 1, 1),
                CellFormat {
                    size,
                    ..CellFormat::default()
                }
            )
            .is_err()
        );
        assert_eq!(book, before);
    }
    assert_eq!(book.replace_all(id, "missing", "x")?, 0);
    assert_eq!(book.replace_all(id, "a", "z")?, 2);
    assert_eq!(
        book.active_sheet()
            .and_then(|sheet| sheet.cell(0, 0))
            .map(|cell| &cell.value),
        Some(&Value::Text("z".into()))
    );
    book.validate()?;
    Ok(())
}

#[test]
fn limited_search_returns_first_matches_in_grid_order() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    for (row, col) in [(9, 0), (1, 1), (1, 0), (0, 25)] {
        book.set_cell(id, row, col, "match")?;
    }
    assert!(book.find(id, "match", false, 0)?.is_empty());
    let positions: Vec<_> = book
        .find(id, "match", false, 2)?
        .into_iter()
        .map(|(r, c, _, _)| (r, c))
        .collect();
    assert_eq!(positions, vec![(0, 25), (1, 0)]);
    Ok(())
}

#[test]
fn sheet_deletion_and_reorder_keep_a_visible_active_sheet() -> Result<(), String> {
    let mut book = Book::default();
    let first = book.active_id();
    let hidden = book.add_sheet("Hidden")?;
    if let Some(sheet) = book.sheet_by_id_mut(hidden) {
        sheet.hidden = true;
    }
    let before = book.clone();
    assert!(book.delete_sheet(first).is_err());
    assert_eq!(book, before);
    book.move_sheet(hidden, 0)?;
    assert_eq!(book.active_id(), first);
    let visible = book.add_sheet("Visible")?;
    book.delete_sheet(first)?;
    assert_eq!(book.active_id(), visible);
    book.validate()?;
    Ok(())
}

#[test]
fn canonical_formula_length_is_checked_before_any_mutation() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "x")?;
    let input = format!("=И(\"{}\")", "a".repeat(32760));
    assert_eq!(input.len(), 32767);
    assert_eq!(formula::canonicalize(&input).len(), 32768);
    let before = book.clone();
    assert!(book.set_input(id, 1, 0, &input).is_err());
    assert_eq!(book, before);
    assert!(
        book.set_range(id, 0, 0, &[vec!["changed".into(), input.clone()]])
            .is_err()
    );
    assert_eq!(book, before);
    assert!(book.replace_all(id, "x", &input).is_err());
    assert_eq!(book, before);
    book.validate()?;
    Ok(())
}

fn seeded() -> Result<Book, String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec![
                "1".into(),
                "red".into(),
                "10".into(),
                "10".into(),
                "20".into(),
                "30".into(),
            ],
            vec![
                "2".into(),
                "blue".into(),
                "20".into(),
                "100".into(),
                "200".into(),
                "300".into(),
            ],
            vec!["3".into(), "red".into(), "30".into()],
        ],
    )?;
    Ok(book)
}
#[test]
fn numeric_formula_corpus() -> Result<(), String> {
    let b = seeded()?;
    let id = b.active_id();
    let cases: &[(&str, f64)] = &[
        ("=1+2*3", 7.0),
        ("=(1+2)*3", 9.0),
        ("=2^3^2", 64.0),
        ("=50%", 0.5),
        ("=100*10%", 10.0),
        ("=SUM(A1:A3)", 6.0),
        ("=СУММ(A1:A3)", 6.0),
        ("=AVERAGE(A1:A3)", 2.0),
        ("=MIN(A1:A3)", 1.0),
        ("=MAX(A1:A3)", 3.0),
        ("=COUNT(A1:C3)", 6.0),
        ("=COUNTA(A1:C3)", 9.0),
        ("=COUNTBLANK(A1:D4)", 5.0),
        ("=MEDIAN(A1:A3)", 2.0),
        ("=MEDIAN(1,2,3,4)", 2.5),
        ("=MODE(1,2,2,3)", 2.0),
        ("=MODE.SNGL(1,1,2)", 1.0),
        ("=STDEV(A1:A3)", 1.0),
        ("=STDEV.S(A1:A3)", 1.0),
        ("=STDEVP(A1:A3)", 0.816496580927726),
        ("=STDEV.P(A1:A3)", 0.816496580927726),
        ("=VAR(A1:A3)", 1.0),
        ("=VAR.S(A1:A3)", 1.0),
        ("=VARP(A1:A3)", 0.666666666666667),
        ("=VAR.P(A1:A3)", 0.666666666666667),
        ("=DEVSQ(A1:A3)", 2.0),
        ("=AVEDEV(A1:A3)", 0.666666666666667),
        ("=GEOMEAN(2,8)", 4.0),
        ("=HARMEAN(2,6)", 3.0),
        ("=SUMSQ(A1:A3)", 14.0),
        ("=PRODUCT(A1:A3)", 6.0),
        ("=AVERAGEA(2,TRUE,\"text\")", 1.0),
        ("=SUMIF(B1:B3,\"red\",C1:C3)", 40.0),
        ("=COUNTIF(B1:B3,\"red\")", 2.0),
        ("=COUNTIF(B1:B3,\"r*\")", 2.0),
        ("=AVERAGEIF(B1:B3,\"red\",C1:C3)", 20.0),
        ("=SUMIFS(C1:C3,B1:B3,\"red\",A1:A3,\">1\")", 30.0),
        ("=COUNTIFS(B1:B3,\"red\",A1:A3,\">1\")", 1.0),
        ("=AVERAGEIFS(C1:C3,B1:B3,\"red\",A1:A3,\">1\")", 30.0),
        ("=SUMPRODUCT(A1:A3,C1:C3)", 140.0),
        ("=IF(TRUE,5,1/0)", 5.0),
        ("=IF(FALSE,1/0,6)", 6.0),
        ("=IFERROR(1/0,7)", 7.0),
        ("=IFNA(NA(),8)", 8.0),
        ("=IFS(FALSE,1,TRUE,9)", 9.0),
        ("=CHOOSE(2,10,20,30)", 20.0),
        ("=SWITCH(2,1,10,2,20,99)", 20.0),
        ("=SWITCH(3,1,10,2,20,99)", 99.0),
        ("=ABS(-3)", 3.0),
        ("=INT(-2.2)", -3.0),
        ("=SIGN(-1)", -1.0),
        ("=SIGN(0)", 0.0),
        ("=SQRT(9)", 3.0),
        ("=EXP(0)", 1.0),
        ("=LN(1)", 0.0),
        ("=LOG10(1000)", 3.0),
        ("=SIN(0)", 0.0),
        ("=COS(0)", 1.0),
        ("=TAN(0)", 0.0),
        ("=ASIN(0)", 0.0),
        ("=ACOS(1)", 0.0),
        ("=ATAN(0)", 0.0),
        ("=SINH(0)", 0.0),
        ("=COSH(0)", 1.0),
        ("=TANH(0)", 0.0),
        ("=ASINH(0)", 0.0),
        ("=ACOSH(1)", 0.0),
        ("=ATANH(0)", 0.0),
        ("=DEGREES(PI())", 180.0),
        ("=RADIANS(180)", std::f64::consts::PI),
        ("=FACT(5)", 120.0),
        ("=FACTDOUBLE(6)", 48.0),
        ("=EVEN(3)", 4.0),
        ("=ODD(4)", 5.0),
        ("=ROUND(1.235,2)", 1.24),
        ("=ROUND(2.675,2)", 2.68),
        ("=ROUNDUP(0.1+0.2,1)", 0.3),
        ("=FLOOR(0.3,0.1)", 0.3),
        ("=MROUND(0.15,0.1)", 0.2),
        ("=ROUNDUP(-1.231,2)", -1.24),
        ("=ROUNDDOWN(-1.239,2)", -1.23),
        ("=TRUNC(-1.9)", -1.0),
        ("=ROUND(123,-1)", 120.0),
        ("=MOD(-3,2)", 1.0),
        ("=POWER(2,3)", 8.0),
        ("=LOG(8,2)", 3.0),
        ("=ATAN2(1,0)", 0.0),
        ("=QUOTIENT(7,2)", 3.0),
        ("=MROUND(10,3)", 9.0),
        ("=CEILING(4.2,2)", 6.0),
        ("=FLOOR(4.2,2)", 4.0),
        ("=CEILING.MATH(-4.2)", -4.0),
        ("=CEILING.PRECISE(-4.2)", -4.0),
        ("=FLOOR.MATH(-4.2)", -5.0),
        ("=FLOOR.PRECISE(-4.2)", -5.0),
        ("=GCD(12,18)", 6.0),
        ("=LCM(4,6)", 12.0),
        ("=COMBIN(5,2)", 10.0),
        ("=COMBINA(3,2)", 6.0),
        ("=PERMUT(5,2)", 20.0),
        ("=LARGE(A1:A3,2)", 2.0),
        ("=SMALL(A1:A3,2)", 2.0),
        ("=PERCENTILE.INC(A1:A3,0.25)", 1.5),
        ("=PERCENTILE(A1:A3,0.75)", 2.5),
        ("=QUARTILE(A1:A3,1)", 1.5),
        ("=QUARTILE.INC(A1:A3,3)", 2.5),
        ("=RANK(2,A1:A3)", 2.0),
        ("=RANK.EQ(1,A1:A3,1)", 1.0),
        ("=LEN(\"ёж🙂\")", 3.0),
        ("=FIND(\"ж\",\"ёжик\")", 2.0),
        ("=SEARCH(\"Ж*\",\"ёжик\")", 2.0),
        ("=VALUE(\"12.5%\")", 0.125),
        ("=NUMBERVALUE(\"1.234,5\",\",\",\".\")", 1234.5),
        ("=CODE(\"A\")", 65.0),
        ("=UNICODE(\"ё\")", 1105.0),
        ("=ROWS(A1:C3)", 3.0),
        ("=COLUMNS(A1:C3)", 3.0),
        ("=ROW(A2)", 2.0),
        ("=COLUMN(B2)", 2.0),
        ("=INDEX(A1:C3,2,3)", 20.0),
        ("=INDEX(D1:F1,2)", 20.0),
        ("=VALUE(\"1,000.5\")", 1000.5),
        ("=VALUE(\"1.000,5\")", 1000.5),
        ("=MATCH(2,A1:A3,0)", 2.0),
        ("=XMATCH(2,A1:A3)", 2.0),
        ("=VLOOKUP(2,A1:C3,3,FALSE)", 20.0),
        ("=HLOOKUP(20,D1:F2,2,FALSE)", 200.0),
        ("=XLOOKUP(2,A1:A3,C1:C3)", 20.0),
        ("=DATE(2024,1,1)", 45292.0),
        ("=DATE(1900,2,29)", 60.0),
        ("=TIME(12,0,0)", 0.5),
        ("=YEAR(45292)", 2024.0),
        ("=MONTH(45292)", 1.0),
        ("=DAY(45292)", 1.0),
        ("=HOUR(0.5)", 12.0),
        ("=MINUTE(TIME(12,34,56))", 34.0),
        ("=SECOND(TIME(12,34,56))", 56.0),
        ("=DAYS(45293,45292)", 1.0),
        ("=WEEKDAY(45292)", 2.0),
        ("=WEEKDAY(45292,2)", 1.0),
        ("=WEEKDAY(45292,3)", 0.0),
        ("=EDATE(45292,1)", 45323.0),
        ("=EOMONTH(45292,0)", 45322.0),
        ("=DATEVALUE(\"2024-01-01\")", 45292.0),
        ("=TIMEVALUE(\"12:00:00\")", 0.5),
        ("=DATEDIF(DATE(2020,1,1),DATE(2024,1,1),\"Y\")", 4.0),
        ("=DATEDIF(DATE(2024,1,1),DATE(2024,2,2),\"M\")", 1.0),
        ("=DATEDIF(DATE(2024,1,1),DATE(2024,1,3),\"D\")", 2.0),
        ("=NETWORKDAYS(DATE(2024,1,1),DATE(2024,1,7))", 5.0),
        ("=WORKDAY(DATE(2024,1,1),5)", 45299.0),
        ("=PMT(0,10,100)", -10.0),
        ("=PV(0,10,-10)", 100.0),
        ("=FV(0,10,-10)", 100.0),
        ("=NPV(0.1,110)", 100.0),
        ("=ROW()", 1.0),
        ("=SLN(100,0,10)", 10.0),
        ("=SYD(100,0,10,1)", 18.18181818181818),
        ("=N(TRUE)", 1.0),
        ("=N(\"text\")", 0.0),
        ("=TYPE(1)", 1.0),
        ("=TYPE(\"text\")", 2.0),
        ("=TYPE(TRUE)", 4.0),
        ("=TYPE(NA())", 16.0),
        ("=MAXIFS(C1:C3,B1:B3,\"red\")", 30.0),
        ("=MINIFS(C1:C3,B1:B3,\"red\",A1:A3,\">1\")", 30.0),
        ("=МАКСЕСЛИ(C1:C3;B1:B3;\"green\")", 0.0),
        ("=MAXA(A1:A3,TRUE)", 3.0),
        ("=MINA(A1:B3)", 0.0),
        ("=TRIMMEAN(A1:A3,0.7)", 2.0),
        (
            "=SKEW(1,2,3,10)",
            2.0 / 3.0 * 180.0 / (50.0f64 / 3.0).powf(1.5),
        ),
        (
            "=KURT(1,2,3,10)",
            20.0 / 6.0 * 1394.0 / (50.0f64 / 3.0).powi(2) - 13.5,
        ),
        ("=CORREL(A1:A3,C1:C3)", 1.0),
        ("=PEARSON(A1:A3,C1:C3)", 1.0),
        ("=RSQ(C1:C3,A1:A3)", 1.0),
        ("=SLOPE(C1:C3,A1:A3)", 10.0),
        ("=INTERCEPT(C1:C3,A1:A3)", 0.0),
        ("=COVAR(A1:A3,C1:C3)", 20.0 / 3.0),
        ("=COVARIANCE.S(A1:A3,C1:C3)", 10.0),
        ("=NPER(0,-10,100)", 10.0),
        ("=NPER(0.05,PMT(0.05,12,1000),1000)", 12.0),
        ("=RATE(12,PMT(0.05,12,1000),1000)", 0.05),
        ("=IPMT(0.1,1,3,8000)", -800.0),
        ("=IPMT(0.1,1,3,8000,0,1)", 0.0),
        (
            "=PPMT(0.1,1,3,8000)+PPMT(0.1,2,3,8000)+PPMT(0.1,3,3,8000)",
            -8000.0,
        ),
        (
            "=PPMT(0.1,1,3,8000,0,1)+PPMT(0.1,2,3,8000,0,1)+PPMT(0.1,3,3,8000,0,1)",
            -8000.0,
        ),
        (
            "=PPMT(0.1,2,3,8000)+IPMT(0.1,2,3,8000)-PMT(0.1,3,8000)",
            0.0,
        ),
        ("=ARABIC(\"MCMXCIV\")", 1994.0),
        ("=ARABIC(\"-xiv\")", -14.0),
        ("=BIN2DEC(\"1111111111\")", -1.0),
        ("=BIN2DEC(101)", 5.0),
        ("=HEX2DEC(\"ff\")", 255.0),
        ("=OCT2DEC(\"17\")", 15.0),
        ("=DECIMAL(\"zz\",36)", 1295.0),
        ("=WEEKNUM(DATE(2024,1,7))", 2.0),
        ("=WEEKNUM(DATE(2024,1,7),2)", 1.0),
        ("=ISOWEEKNUM(DATE(2021,1,1))", 53.0),
        ("=WEEKNUM(DATE(2021,1,4),21)", 1.0),
        ("=DAYS360(DATE(2024,1,1),DATE(2024,12,31))", 360.0),
        ("=DAYS360(DATE(2024,1,31),DATE(2024,3,31),TRUE)", 60.0),
        ("=SUBTOTAL(9,A1:A3)", 6.0),
        ("=SUBTOTAL(1,C1:C3)", 20.0),
        ("=SUBTOTAL(103,A1:B3)", 6.0),
    ];
    for &(input, expected) in cases {
        let actual = formula::evaluate(&b, id, input);
        let Value::Number(n) = actual else {
            return Err(format!("{input}: {actual:?}, expected {expected}"));
        };
        assert!(
            (n - expected).abs() <= 1e-8 * expected.abs().max(1.0),
            "{input}: {n}, expected {expected}"
        );
    }
    assert!(cases.len() >= 150);
    Ok(())
}
#[test]
fn text_boolean_and_errors() -> Result<(), String> {
    let b = seeded()?;
    let id = b.active_id();
    let text = &[
        ("=\"a\"&\"b\"", "ab"),
        ("=CONCAT(\"a\",\"b\")", "ab"),
        ("=0.1+0.2&\"\"", "0.3"),
        ("=TEXT(2.5,\"0\")", "3"),
        ("=CONCATENATE(\"a\",\"b\")", "ab"),
        ("=TEXTJOIN(\"-\",TRUE,\"a\",\"\",\"b\")", "a-b"),
        ("=LOWER(\"ABCЁ\")", "abcё"),
        ("=UPPER(\"abcё\")", "ABCЁ"),
        ("=PROPER(\"hello WORLD\")", "Hello World"),
        ("=TRIM(\" a  b \")", "a b"),
        ("=CLEAN(CHAR(10)&\"abc\")", "abc"),
        ("=LEFT(\"ёжик\",2)", "ёж"),
        ("=RIGHT(\"ёжик\",2)", "ик"),
        ("=MID(\"ёжик\",2,2)", "жи"),
        ("=REPT(\"x\",3)", "xxx"),
        ("=SUBSTITUTE(\"abab\",\"a\",\"x\")", "xbxb"),
        ("=SUBSTITUTE(\"abab\",\"a\",\"x\",2)", "abxb"),
        ("=REPLACE(\"abcd\",2,2,\"XY\")", "aXYd"),
        ("=TEXT(12.5,\"0.00\")", "12.50"),
        ("=CHAR(65)", "A"),
        ("=UNICHAR(1105)", "ё"),
        ("=T(\"yes\")", "yes"),
        ("=TEXTBEFORE(\"a-b-c\",\"-\")", "a"),
        ("=TEXTAFTER(\"a-b-c\",\"-\",-1)", "c"),
        ("=TEXTAFTER(\"a-b-c\",\"-\",2)", "c"),
        ("=TEXTBEFORE(\"aXbxc\",\"x\",2,1)", "aXb"),
        ("=TEXTBEFORE(\"a-b\",\"-\",2,0,1)", "a-b"),
        ("=TEXTAFTER(\"abc\",\"-\",1,0,0,\"none\")", "none"),
        ("=ТЕКСТПОСЛЕ(\"ёж-ик\";\"-\")", "ик"),
        ("=ROMAN(1994)", "MCMXCIV"),
        ("=ROMAN(0)", ""),
        ("=DEC2BIN(-1)", "1111111111"),
        ("=DEC2HEX(255,4)", "00FF"),
        ("=DEC2OCT(8)", "10"),
        ("=BASE(255,16)", "FF"),
        ("=BASE(5,2,8)", "00000101"),
    ];
    for &(input, expected) in text {
        assert_eq!(
            formula::evaluate(&b, id, input),
            Value::Text(expected.into()),
            "{input}"
        );
    }
    for (input, expected) in [
        ("=TRUE()", true),
        ("=FALSE()", false),
        ("=AND(TRUE,TRUE)", true),
        ("=OR(FALSE,TRUE)", true),
        ("=XOR(TRUE,TRUE)", false),
        ("=NOT(FALSE)", true),
        ("=EXACT(\"A\",\"a\")", false),
        ("=ISNUMBER(1)", true),
        ("=ISTEXT(\"x\")", true),
        ("=ISNONTEXT(1)", true),
        ("=ISBLANK(Z1)", true),
        ("=ISLOGICAL(TRUE)", true),
        ("=ISERROR(1/0)", true),
        ("=ISERR(NA())", false),
        ("=ISNA(NA())", true),
        ("=ISEVEN(2)", true),
        ("=ISODD(3)", true),
        ("=1<2", true),
        ("=2>=2", true),
        ("=\"ABC\"=\"abc\"", true),
    ] {
        assert_eq!(
            formula::evaluate(&b, id, input),
            Value::Bool(expected),
            "{input}"
        );
    }
    for (input, expected) in [
        ("=1/0", "#DIV/0!"),
        ("=SQRT(-1)", "#NUM!"),
        ("=FOOBAR(1)", "#NAME?"),
        ("=SUM(", "#VALUE!"),
        ("=NA()", "#N/A"),
        ("=INDEX(A1:B2,3,1)", "#REF!"),
        ("=IF(TRUE)", "#VALUE!"),
        ("=ROUND(1)", "#VALUE!"),
        ("=REPT(\"x\",-1)", "#VALUE!"),
        ("=VLOOKUP(99,A1:C3,3,FALSE)", "#N/A"),
        ("=LOG(1,1)", "#NUM!"),
        ("=TEXTBEFORE(\"abc\",\"-\")", "#N/A"),
        ("=TEXTBEFORE(\"abc\",\"b\",0)", "#VALUE!"),
        ("=DEC2BIN(512)", "#NUM!"),
        ("=DEC2HEX(255,1)", "#NUM!"),
        ("=HEX2DEC(\"XYZ\")", "#NUM!"),
        ("=ROMAN(4000)", "#VALUE!"),
        ("=ARABIC(\"ABC\")", "#VALUE!"),
        ("=SUBTOTAL(12,A1:A3)", "#VALUE!"),
        ("=MAXIFS(C1:C3,B1:B2,\"red\")", "#VALUE!"),
        ("=SLOPE(A1:A3,G1:G3)", "#DIV/0!"),
        ("=SKEW(1,2)", "#DIV/0!"),
    ] {
        assert_eq!(
            formula::evaluate(&b, id, input),
            Value::Error(expected.into()),
            "{input}"
        );
    }
    Ok(())
}
#[test]
fn stable_ids_and_formula_transforms() -> Result<(), String> {
    let mut b = seeded()?;
    let id = b.active_id();
    let first = b
        .sheet_by_id(id)
        .and_then(|s| s.cell(0, 0))
        .map(|c| c.id)
        .ok_or("Missing cell")?;
    b.set_cell(id, 0, 0, "5")?;
    assert_eq!(
        b.sheet_by_id(id).and_then(|s| s.cell(0, 0)).map(|c| c.id),
        Some(first)
    );
    b.set_cell(id, 5, 2, "=$A$1+A1+\"A1\"+LOG10(100)")?;
    b.edit_dimension(id, 0, 1, true, false)?;
    assert_eq!(
        b.sheet_by_id(id).and_then(|s| s.cell(1, 0)).map(|c| c.id),
        Some(first)
    );
    assert_eq!(
        b.sheet_by_id(id)
            .and_then(|s| s.cell(6, 2))
            .map(|c| c.input.as_str()),
        Some("=$A$2+A2+\"A1\"+LOG10(100)")
    );
    assert_eq!(
        formula::shift_references("=$A1+A$1+$A$1+'Имя 1'!B2+\"A1\"+LOG10(10)", 1, 2),
        "=$A2+C$1+$A$1+'Имя 1'!D3+\"A1\"+LOG10(10)"
    );
    assert_eq!(formula::shift_references("=A1", -1, 0), "=#REF!");
    assert_eq!(
        formula::canonicalize("=СУММ(1;2)&\"СУММ(\""),
        "=SUM(1,2)&\"СУММ(\""
    );
    Ok(())
}
#[test]
fn multisheet_cycles_and_finance() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    let other = b.add_sheet("Другой лист")?;
    b.set_cell(other, 0, 0, "7")?;
    b.set_cell(id, 0, 0, "='Другой лист'!A1*2")?;
    assert_eq!(
        b.active_sheet()
            .and_then(|s| s.cell(0, 0))
            .map(|c| c.value.clone()),
        Some(Value::Number(14.0))
    );
    b.rename_sheet(other, "Переименован")?;
    assert_eq!(
        b.active_sheet()
            .and_then(|s| s.cell(0, 0))
            .map(|c| c.value.clone()),
        Some(Value::Number(14.0))
    );
    b.set_range(id, 0, 0, &[vec!["-100".into()], vec!["110".into()]])?;
    let v = formula::evaluate(&b, id, "=IRR(A1:A2)");
    assert!(matches!(v,Value::Number(n)if (n-0.1).abs()<1e-8));
    b.set_range(id, 0, 0, &[vec!["=A2".into()], vec!["=A1".into()]])?;
    assert!(
        b.active_sheet()
            .and_then(|s| s.cell(0, 0))
            .is_some_and(|c| matches!(c.value, Value::Error(_)))
    );
    let long = format!("={}", "1+".repeat(2000)) + "1";
    assert!(matches!(formula::evaluate(&b, id, &long), Value::Error(_)));
    Ok(())
}

#[test]
fn extreme_date_arguments_return_errors() {
    let book = Book::default();
    let id = book.active_id();
    for formula in [
        "=EDATE(1,-1E308)",
        "=WORKDAY(1,-1E308)",
        "=DATE(2024,-1E308,1)",
        "=NETWORKDAYS(-1E308,1E308)",
        "=WEEKDAY(-1E308)",
        "=YEAR(1E308)",
        "=DATEDIF(-1E308,1E308,\"D\")",
        "=EOMONTH(1E308,1)",
        "=DATE(2024,1,-1E308)",
        "=DATE(2024,1,1E308)",
        "=NETWORKDAYS(1,1E308)",
    ] {
        assert!(
            matches!(
                super::formula::evaluate(&book, id, formula),
                Value::Error(_)
            ),
            "{formula}"
        );
    }
}

#[test]
fn malformed_native_book_is_rejected() -> Result<(), String> {
    let mut b = Book::default();
    assert!(b.validate().is_ok());
    b.next_id = 1;
    assert!(b.validate().is_err());
    b.next_id = 2;
    b.selection.row = MAX_ROWS;
    assert!(b.validate().is_err());
    b.selection.row = 0;
    let id = b.active_id();
    b.set_cell(id, 0, 0, "1")?;
    if let Some(c) = b.active_sheet_mut().and_then(|s| s.cell_mut(0, 0)) {
        c.fmt.size = f32::INFINITY;
    }
    assert!(b.validate().is_err());
    if let Some(c) = b.active_sheet_mut().and_then(|s| s.cell_mut(0, 0)) {
        c.fmt.size = 11.0;
        c.id = id;
    }
    assert!(b.validate().is_err());
    Ok(())
}

#[test]
fn pdf_keeps_far_rows_and_columns() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    b.set_cell(id, 0, 0, "top-left")?;
    b.set_cell(id, 10000, 26, "far-cell")?;
    let pages = b.pages();
    assert_eq!(pages.len(), 2);
    assert!(
        pages
            .iter()
            .flat_map(|p| &p.items)
            .any(|p| matches!(p,crate::render::Primitive::TextStyled{text,..}if text=="far-cell"))
    );
    assert_eq!(b.print_page_count(), 2);
    Ok(())
}

#[test]
fn compare_types_and_empty_criteria() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    b.set_range(
        id,
        0,
        0,
        &[vec!["0".into()], vec!["'".into()], vec!["".into()]],
    )?;
    assert_eq!(formula::evaluate(&b, id, "=1=\"1\""), Value::Bool(false));
    assert_eq!(
        formula::evaluate(&b, id, "=COUNTIF(A1:A3,\"\")"),
        Value::Number(2.0)
    );
    assert_eq!(
        formula::evaluate(&b, id, "=COUNTIF(A1:A3,\"<>\")"),
        Value::Number(1.0)
    );
    Ok(())
}

#[test]
fn criteria_skip_other_kinds_of_values() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    // 7, text, a number stored as text, a blank and 3.
    b.set_range(
        id,
        0,
        0,
        &[
            vec!["7".into()],
            vec!["abc".into()],
            vec!["'9".into()],
            vec!["".into()],
            vec!["3".into()],
        ],
    )?;
    for (input, expected) in [
        ("=COUNTIF(A1:A5,\">5\")", 2.0),
        ("=COUNTIF(A1:A5,\"<5\")", 1.0),
        ("=COUNTIF(A1:A5,\"<>7\")", 4.0),
        ("=COUNTIF(A1:A5,\">a\")", 1.0),
        ("=SUMIF(A1:A5,\"<5\")", 3.0),
    ] {
        assert_eq!(
            formula::evaluate(&b, id, input),
            Value::Number(expected),
            "{input}"
        );
    }
    Ok(())
}

#[test]
fn structural_ranges_shrink_and_deleted_sheets_stay_invalid() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    b.set_range(
        id,
        0,
        0,
        &[vec!["1".into()], vec!["2".into()], vec!["3".into()]],
    )?;
    b.set_cell(id, 5, 0, "=SUM(A1:A3)")?;
    b.names.insert("Total".into(), "'Лист1'!$A$1:$A$3".into());
    b.merge(id, Range::new(0, 1, 2, 2))?;
    b.edit_dimension(id, 0, 1, true, true)?;
    assert_eq!(
        b.sheet_by_id(id)
            .and_then(|s| s.cell(4, 0))
            .map(|c| c.input.as_str()),
        Some("=SUM(A1:A2)")
    );
    assert_eq!(
        b.sheet_by_id(id)
            .and_then(|s| s.cell(4, 0))
            .map(|c| c.value.clone()),
        Some(Value::Number(5.0))
    );
    assert_eq!(
        b.names.get("Total").map(String::as_str),
        Some("'Лист1'!$A$1:$A$2")
    );
    assert_eq!(
        b.sheet_by_id(id).map(|s| s.merges.clone()),
        Some(vec![Range::new(0, 1, 1, 2)])
    );
    assert_eq!(
        formula::shift_structural("=SUM($A$3:$A$1)", "Лист1", "Лист1", 0, 1, true, true),
        "=SUM($A$2:$A$1)"
    );
    assert_eq!(
        formula::shift_structural("=SUM(A1:B2)+C1", "Лист1", "Лист1", 0, 2, true, true),
        "=SUM(#REF!)+#REF!"
    );
    assert_eq!(
        formula::shift_structural("='лист1'!A1", "Другой", "Лист1", 0, 1, true, false),
        "='лист1'!A2"
    );
    let other = b.add_sheet("Другой")?;
    b.set_cell(id, 0, 4, "='Другой'!A1")?;
    b.delete_sheet(other)?;
    b.add_sheet("Другой")?;
    assert_eq!(
        b.sheet_by_id(id)
            .and_then(|s| s.cell(0, 4))
            .map(|c| c.value.clone()),
        Some(Value::Error("#REF!".into()))
    );
    b.validate()?;
    Ok(())
}
#[test]
fn structural_metadata_is_bounded_and_transactional() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    if let Some(s) = b.active_sheet_mut() {
        s.row_heights.insert(MAX_ROWS - 1, 20.0);
        s.frozen_rows = 3;
    }
    let before = b.clone();
    assert!(b.edit_dimension(id, 0, 1, true, false).is_err());
    assert_eq!(b, before);
    if let Some(s) = b.active_sheet_mut() {
        s.row_heights.clear();
    }
    b.selection.row = 2;
    b.selection.anchor_row = 2;
    b.merge(id, Range::new(1, 0, 2, 0))?;
    b.edit_dimension(id, 1, 2, true, true)?;
    assert!(
        b.active_sheet()
            .is_some_and(|s| s.merges.is_empty() && s.frozen_rows == 1)
    );
    assert_eq!(b.selection.row, 1);
    b.validate()?;
    let chart = b.add_chart(
        id,
        ChartKind::Column,
        Range::new(MAX_ROWS - 1, 0, MAX_ROWS - 1, 0),
        "edge",
    )?;
    assert!(
        b.active_sheet()
            .is_some_and(|s| s.charts.iter().any(|c| c.id == chart && c.row < MAX_ROWS))
    );
    b.validate()?;
    Ok(())
}
#[test]
fn local_clock_uses_calendar_not_utc_epoch() -> Result<(), String> {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 13)
        .and_then(|d| d.and_hms_opt(0, 30, 0))
        .ok_or("date")?;
    let serial = formula::calendar_serial(date);
    assert_eq!(serial.floor(), formula::date_serial(2026, 10, 13));
    assert!((serial.fract() - 0.5 / 24.0).abs() < 1e-10);
    Ok(())
}
#[test]
fn lookups_are_lazy_literal_and_validate_modes() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    b.set_range(
        id,
        0,
        0,
        &[
            vec!["'<3".into(), "10".into()],
            vec!["'cat".into(), "20".into()],
            vec!["'car".into(), "30".into()],
        ],
    )?;
    assert_eq!(
        formula::evaluate(&b, id, "=XLOOKUP(\"cat\",A1:A3,B1:B3,NA())"),
        Value::Number(20.0)
    );
    assert_eq!(
        formula::evaluate(&b, id, "=XLOOKUP(\"missing\",A1:A3,B1:B3,\"fallback\")"),
        Value::Text("fallback".into())
    );
    assert_eq!(
        formula::evaluate(&b, id, "=VLOOKUP(\"<3\",A1:B3,2,FALSE)"),
        Value::Number(10.0)
    );
    assert_eq!(
        formula::evaluate(&b, id, "=MATCH(\"ca?\",A1:A3,0)"),
        Value::Number(2.0)
    );
    assert_eq!(
        formula::evaluate(&b, id, "=XMATCH(\"ca?\",A1:A3,2,-1)"),
        Value::Number(3.0)
    );
    assert_eq!(
        formula::evaluate(&b, id, "=XLOOKUP(\"cat\",A1:A3,B1:B3,,3)"),
        Value::Error("#VALUE!".into())
    );
    assert_eq!(
        formula::evaluate(&b, id, "=MATCH(10,A1:B3,0)"),
        Value::Error("#VALUE!".into())
    );
    assert_eq!(formula::shift_references("=A2", i64::MAX, 0), "=#REF!");
    Ok(())
}
#[test]
fn text_expansion_and_nonfinite_coercion_are_bounded() -> Result<(), String> {
    let mut b = Book::default();
    let id = b.active_id();
    assert_eq!(
        formula::evaluate(&b, id, "=REPLACE(\"abc\",2,1E308,\"x\")"),
        Value::Text("ax".into())
    );
    assert_eq!(
        formula::evaluate(&b, id, "=MID(\"abc\",\"NaN\",1)"),
        Value::Error("#NUM!".into())
    );
    b.set_cell(id, 0, 0, "=REPT(\"x\",20000)")?;
    assert_eq!(
        formula::evaluate(&b, id, "=A1&A1"),
        Value::Error("#VALUE!".into())
    );
    assert_eq!(
        formula::evaluate(&b, id, "=SUBSTITUTE(A1,\"x\",A1)"),
        Value::Error("#VALUE!".into())
    );
    assert_eq!(
        formula::evaluate(&b, id, "=TEXTJOIN(\"\",FALSE,A1,A1)"),
        Value::Error("#VALUE!".into())
    );
    let cell = b
        .active_sheet()
        .and_then(|s| s.cell(0, 0))
        .cloned()
        .ok_or("cell")?;
    if let Some(s) = b.active_sheet_mut() {
        s.cells.clear();
        s.cells.insert("$A$1".into(), cell);
    }
    assert!(b.validate().is_err());
    Ok(())
}
#[test]
fn overlapping_native_merges_are_rejected() {
    let mut b = Book::default();
    if let Some(s) = b.active_sheet_mut() {
        s.merges = vec![Range::new(0, 0, 2, 2), Range::new(2, 2, 4, 4)];
    }
    assert!(b.validate().is_err());
    if let Some(s) = b.active_sheet_mut() {
        s.merges = vec![
            Range::new(0, 0, 2, 2),
            Range::new(3, 0, 4, 2),
            Range::new(0, 3, 2, 4),
        ];
    }
    assert!(b.validate().is_ok());
}

#[test]
fn review_aggregate_text_and_shape_semantics() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["2".into(), "3".into()],
            vec!["=\"\"".into(), "4".into()],
        ],
    )?;
    for (input, expected) in [
        ("=AVERAGEA(A1:A2)", Value::Number(1.0)),
        ("=MINA(A1:A2)", Value::Number(0.0)),
        ("=AVERAGEA(\"2\",4)", Value::Number(3.0)),
        ("=MINA(\"2\",4)", Value::Number(2.0)),
        ("=SUMPRODUCT(A1:A2,A1:B1)", Value::Error("#VALUE!".into())),
        (
            "=SUMIFS(A1:A2,A1:B1,\">0\")",
            Value::Error("#VALUE!".into()),
        ),
        (
            "=COUNTIFS(A1:A2,\">0\",A1:B1,\">0\")",
            Value::Error("#VALUE!".into()),
        ),
        (
            "=MAXIFS(A1:A2,A1:B1,\">0\")",
            Value::Error("#VALUE!".into()),
        ),
        (
            "=MINIFS(A1:A2,A1:B1,\">0\")",
            Value::Error("#VALUE!".into()),
        ),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
    Ok(())
}

#[test]
fn review_text_delimiter_defaults_and_lazy_fallback() {
    let book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=TEXTBEFORE(\"\",\"-\")", Value::Text(String::new())),
        ("=TEXTAFTER(\"\",\"-\")", Value::Text(String::new())),
        (
            "=TEXTBEFORE(\"a-b\",\"-\",1,0,0,1/0)",
            Value::Text("a".into()),
        ),
        (
            "=TEXTAFTER(\"a-b\",\"-\",1,0,0,1/0)",
            Value::Text("b".into()),
        ),
        (
            "=TEXTBEFORE(\"abc\",\"-\",1,0,0,\"missing\")",
            Value::Text("missing".into()),
        ),
        (
            "=TEXTBEFORE(\"a-b\",\"-\",1,2)",
            Value::Error("#VALUE!".into()),
        ),
        (
            "=TEXTAFTER(\"a-b\",\"-\",1,0,2)",
            Value::Error("#VALUE!".into()),
        ),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
}

#[test]
fn review_combinatorics_and_radix_do_not_use_integer_storage_limits() {
    let book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=COMBIN(1000,1)", 1000.0),
        ("=COMBIN(1000,999)", 1000.0),
        ("=COMBINA(1000,1)", 1000.0),
        ("=COMBINA(0,0)", 1.0),
        ("=PERMUT(1000,1)", 1000.0),
        ("=DECIMAL(\"100000000000000000000\",10)", 1e20),
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Number(expected),
            "{input}"
        );
    }
    let expected = (1..=100).map(|i| 2.0 * f64::from(i)).product::<f64>();
    assert_eq!(
        formula::evaluate(&book, id, "=FACTDOUBLE(200)"),
        Value::Number(expected)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=COMBIN(1E308,1E307)"),
        Value::Error("#NUM!".into())
    );
    assert_eq!(
        formula::evaluate(&book, id, "=FACTDOUBLE(301)"),
        Value::Error("#NUM!".into())
    );
}

#[test]
fn review_legacy_rounding_preserves_significance_sign() {
    let book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=CEILING(-2.5,-2)", Value::Number(-4.0)),
        ("=FLOOR(-2.5,-2)", Value::Number(-2.0)),
        ("=CEILING(2.5,-2)", Value::Error("#NUM!".into())),
        ("=FLOOR(2.5,-2)", Value::Error("#NUM!".into())),
        ("=CEILING(-2.5,2)", Value::Number(-2.0)),
        ("=FLOOR(-2.5,2)", Value::Number(-4.0)),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
}

#[test]
fn review_lookup_errors_and_empty_boolean_comparison() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "#DIV/0!")?;
    for input in [
        "=MATCH(A1,B1:B2,0)",
        "=XMATCH(A1,B1:B2)",
        "=VLOOKUP(A1,B1:C2,2,FALSE)",
        "=HLOOKUP(A1,B1:C2,2,FALSE)",
        "=XLOOKUP(A1,B1:B2,C1:C2,\"missing\")",
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Error("#DIV/0!".into()),
            "{input}"
        );
    }
    assert_eq!(formula::evaluate(&book, id, "=Z1=FALSE"), Value::Bool(true));
    assert_eq!(formula::evaluate(&book, id, "=FALSE=Z1"), Value::Bool(true));
    Ok(())
}

#[test]
fn review_calendar_serial_zero_is_excel_january_day_zero() {
    let book = Book::default();
    let id = book.active_id();
    for (input, expected) in [("=YEAR(0)", 1900.0), ("=MONTH(0)", 1.0), ("=DAY(0)", 0.0)] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Number(expected),
            "{input}"
        );
    }
}
