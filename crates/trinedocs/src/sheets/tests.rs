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
        ("=TEXT(12.5,\"0.00\")", "12,50"),
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

#[test]
fn review2_reference_errors_are_not_reinterpreted_as_text_or_values() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "#DIV/0!")?;
    book.set_cell(id, 0, 1, "=1")?;
    for input in [
        "=SWITCH(A1,1,10,99)",
        "=SWITCH(1,A1,10,99)",
        "=TEXT(A1,\"0\")",
        "=NUMBERVALUE(\"1\",A1)",
        "=NUMBERVALUE(\"1\",\".\",A1)",
        "=COUNTIF(B1:B2,A1)",
        "=SUMIFS(B1:B2,B1:B2,A1)",
        "=MAXIFS(B1:B2,B1:B2,A1)",
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Error("#DIV/0!".into()),
            "{input}"
        );
    }
    assert_eq!(
        formula::evaluate(&book, id, "=ISFORMULA('Missing'!B1)"),
        Value::Error("#REF!".into())
    );
    assert_eq!(
        formula::evaluate(&book, id, "=ISFORMULA(B1)"),
        Value::Bool(true)
    );
    Ok(())
}

#[test]
fn review2_formula_boundaries_keep_blank_cells_distinct_from_zero_results() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "=Z99")?;
    for (input, expected) in [
        ("=A1", Value::Number(0.0)),
        ("=ISBLANK(A1)", Value::Bool(false)),
        ("=ISBLANK(Z99)", Value::Bool(true)),
        ("=COUNT(A1)", Value::Number(1.0)),
        ("=COUNT(Z99)", Value::Number(0.0)),
        ("=-0=0", Value::Bool(true)),
        ("=-0<0", Value::Bool(false)),
        ("=0>-0", Value::Bool(false)),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
    Ok(())
}

#[test]
fn review2_conditional_aggregates_project_from_the_top_left_sum_cell() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["x".into(), "10".into(), "100".into()],
            vec!["x".into(), "20".into(), "200".into()],
        ],
    )?;
    for (input, expected) in [
        ("=SUMIF(A1:A2,\"x\",B1)", 30.0),
        ("=SUMIF(A1:A2,\"x\",B1:C1)", 30.0),
        ("=SUMIF(A1:A2,\"x\",B1:C2)", 30.0),
        ("=AVERAGEIF(A1:A2,\"x\",B1)", 15.0),
        ("=AVERAGEIF(A1:A2,\"x\",B1:C1)", 15.0),
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Number(expected),
            "{input}"
        );
    }
    assert_eq!(
        formula::evaluate(&book, id, "=SUMIF(A1:A2,\"x\",B1048576)"),
        Value::Error("#REF!".into())
    );
    Ok(())
}

#[test]
fn review2_locales_large_rounding_and_ooxml_function_prefixes() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    assert_eq!(
        formula::evaluate(&book, id, "=TEXT(1.25,\"0.00\")"),
        Value::Text("1,25".into())
    );
    assert_eq!(
        formula::evaluate(&book, id, "=DATEVALUE(\"13/10/2026\")"),
        Value::Number(formula::date_serial(2026, 10, 13))
    );
    book.locale_ru = false;
    assert_eq!(
        formula::evaluate(&book, id, "=TEXT(1.25,\"0.00\")"),
        Value::Text("1.25".into())
    );
    assert_eq!(
        formula::evaluate(&book, id, "=DATEVALUE(\"10/13/2026\")"),
        Value::Number(formula::date_serial(2026, 10, 13))
    );
    for input in [
        "=ROUND(1E308,2)",
        "=ROUNDUP(1E308,2)",
        "=ROUNDDOWN(1E308,2)",
        "=TRUNC(1E308,2)",
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Number(1e308),
            "{input}"
        );
    }
    book.set_range(id, 0, 0, &[vec!["1".into(), "42".into()]])?;
    assert_eq!(
        formula::evaluate(&book, id, "=_xlfn.XLOOKUP(1,A1,B1)"),
        Value::Number(42.0)
    );
    assert_eq!(
        formula::canonicalize("=_xlfn._xlws.XLOOKUP(1,A1,B1)"),
        "=XLOOKUP(1,A1,B1)"
    );
    for input in ["=TIME(32768,0,0)", "=TIME(0,32768,0)", "=TIME(0,0,32768)"] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Error("#NUM!".into()),
            "{input}"
        );
    }
    assert_eq!(
        formula::evaluate(&book, id, "=TIME(27,0,0)"),
        Value::Number(0.125)
    );
    Ok(())
}

#[test]
fn review2_reverse_delimiter_search_and_nasd_month_ends() {
    let book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        (
            "=TEXTBEFORE(\"ababa\",\"aba\",-1)",
            Value::Text("ab".into()),
        ),
        (
            "=TEXTAFTER(\"ababa\",\"aba\",-1)",
            Value::Text(String::new()),
        ),
        ("=TEXTBEFORE(\"aaaaa\",\"aa\",-2)", Value::Text("a".into())),
        (
            "=TEXTBEFORE(\"ababa\",\"aba\",1)",
            Value::Text(String::new()),
        ),
        (
            "=DAYS360(DATE(2023,1,30),DATE(2023,2,28))",
            Value::Number(30.0),
        ),
        (
            "=DAYS360(DATE(2024,1,30),DATE(2024,2,29))",
            Value::Number(30.0),
        ),
        (
            "=DAYS360(DATE(2023,1,30),DATE(2023,2,28),TRUE)",
            Value::Number(28.0),
        ),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
}

#[test]
fn review2_sort_compares_value_types_and_keeps_blanks_after_data() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["10".into(), "number".into()],
            vec!["'2".into(), "text".into()],
            vec!["".into(), "blank".into()],
            vec!["2".into(), "small".into()],
        ],
    )?;
    let range = Range::new(0, 0, 3, 1);
    book.sort(id, range, &[(0, false)], false)?;
    let labels = |book: &Book| -> Result<Vec<String>, String> {
        Ok(book
            .read_range(id, 0, 1, 3, 1)?
            .into_iter()
            .map(|row| row[0].clone())
            .collect())
    };
    assert_eq!(labels(&book)?, ["small", "number", "text", "blank"]);
    book.sort(id, range, &[(0, true)], false)?;
    assert_eq!(labels(&book)?, ["text", "number", "small", "blank"]);
    Ok(())
}

#[test]
fn review2_print_count_matches_hidden_cells_and_draws_both_decorations() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "visible")?;
    book.set_cell(id, 100, 0, "hidden row")?;
    book.set_cell(id, 0, 26, "hidden column")?;
    let sheet = book.active_sheet_mut().ok_or("sheet")?;
    sheet.hidden_rows.insert(100);
    sheet.hidden_cols.insert(26);
    let cell = sheet.cell_mut(0, 0).ok_or("cell")?;
    cell.fmt.underline = true;
    cell.fmt.strike = true;
    let pages = book.pages();
    assert_eq!(book.print_page_count(), pages.len());
    assert_eq!(pages.len(), 1);
    let decorations = pages[0]
        .items
        .iter()
        .filter(|item| matches!(item, crate::render::Primitive::Line {width, ..} if *width == 0.5))
        .count();
    assert_eq!(decorations, 2);
    book.active_sheet_mut()
        .ok_or("sheet")?
        .hidden_rows
        .insert(0);
    assert_eq!(book.print_page_count(), book.pages().len());
    Ok(())
}

#[test]
fn review3_deleting_sheet_keeps_reference_looking_plain_text() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    let other = book.add_sheet("Other")?;
    book.set_cell(id, 0, 0, "Other!A1")?;
    book.set_cell(id, 1, 0, "'Other!A1")?;
    book.set_cell(id, 2, 0, "=Other!A1")?;
    book.delete_sheet(other)?;
    assert_eq!(
        book.active_sheet()
            .and_then(|sheet| sheet.cell(0, 0))
            .map(|cell| cell.input.as_str()),
        Some("Other!A1")
    );
    assert_eq!(
        formula::evaluate(&book, id, "=A1"),
        Value::Text("Other!A1".into())
    );
    assert_eq!(
        formula::evaluate(&book, id, "=A2"),
        Value::Text("Other!A1".into())
    );
    assert_eq!(
        formula::evaluate(&book, id, "=A3"),
        Value::Error("#REF!".into())
    );
    Ok(())
}
#[test]
fn review3_subtotals_use_expression_structure_and_named_reference_identity() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["SUBTOTAL(".into()],
            vec!["=SUBTOTAL(9,B1:B2)".into()],
            vec!["7".into()],
        ],
    )?;
    book.set_range(id, 0, 1, &[vec!["2".into()], vec!["3".into()]])?;
    book.names.insert("Data".into(), "=A1:A3".into());
    book.names.insert("Alias".into(), "=Data".into());
    assert_eq!(
        formula::evaluate(&book, id, "=SUBTOTAL(3,A1:A3)"),
        Value::Number(2.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=SUBTOTAL(9,Alias)"),
        Value::Number(7.0)
    );
    book.active_sheet_mut()
        .ok_or("sheet")?
        .hidden_rows
        .insert(2);
    assert_eq!(
        formula::evaluate(&book, id, "=SUBTOTAL(109,Data)"),
        Value::Number(0.0)
    );
    book.names.insert("Cell".into(), "=A2".into());
    assert_eq!(
        formula::evaluate(&book, id, "=ISFORMULA(Cell)"),
        Value::Bool(true)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=ROW(Cell)"),
        Value::Number(2.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=COLUMN(Cell)"),
        Value::Number(1.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=ROW('Missing'!A1)"),
        Value::Error("#REF!".into())
    );
    Ok(())
}
#[test]
fn review3_empty_criteria_reference_means_zero_and_power_domains_match_office() -> Result<(), String>
{
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[vec!["0".into()], vec!["0".into()], vec!["".into()]],
    )?;
    assert_eq!(
        formula::evaluate(&book, id, "=COUNTIF(A1:A3,Z1)"),
        Value::Number(2.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=COUNTIF(A1:A3,\"\")"),
        Value::Number(1.0)
    );
    for (input, expected) in [
        ("=POWER(0,0)", Value::Error("#NUM!".into())),
        ("=0^0", Value::Error("#NUM!".into())),
        ("=POWER(0,-1)", Value::Error("#DIV/0!".into())),
        ("=0^-1", Value::Error("#DIV/0!".into())),
        ("=POWER(0,1)", Value::Number(0.0)),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
    Ok(())
}
#[test]
fn review3_approximate_lookup_uses_last_tie_and_compatible_types() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["1".into(), "10".into()],
            vec!["2".into(), "20".into()],
            vec!["2".into(), "30".into()],
        ],
    )?;
    for (input, expected) in [
        ("=MATCH(2,A1:A3,1)", Value::Number(3.0)),
        ("=MATCH(2,A1:A3,0)", Value::Number(2.0)),
        ("=XMATCH(2,A1:A3)", Value::Number(2.0)),
        ("=VLOOKUP(2,A1:B3,2,TRUE)", Value::Number(30.0)),
        ("=VLOOKUP(2,A1:B3,2,FALSE)", Value::Number(20.0)),
        ("=MATCH(\"cat\",A1:A3,1)", Value::Error("#N/A".into())),
        (
            "=VLOOKUP(\"cat\",A1:B3,2,TRUE)",
            Value::Error("#N/A".into()),
        ),
        (
            "=XLOOKUP(\"cat\",A1:A3,B1:B3,, -1)",
            Value::Error("#N/A".into()),
        ),
    ] {
        assert_eq!(formula::evaluate(&book, id, input), expected, "{input}");
    }
    Ok(())
}
#[test]
fn review3_sort_does_not_move_hidden_rows() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[vec!["3".into()], vec!["1".into()], vec!["2".into()]],
    )?;
    book.active_sheet_mut()
        .ok_or("sheet")?
        .hidden_rows
        .insert(1);
    let hidden_id = book
        .active_sheet()
        .and_then(|sheet| sheet.cell(1, 0))
        .ok_or("hidden")?
        .id;
    book.sort(id, Range::new(0, 0, 2, 0), &[(0, false)], false)?;
    assert_eq!(
        book.read_range(id, 0, 0, 2, 0)?,
        vec![vec!["2"], vec!["1"], vec!["3"]]
    );
    assert_eq!(
        book.active_sheet()
            .and_then(|sheet| sheet.cell(1, 0))
            .ok_or("hidden")?
            .id,
        hidden_id
    );
    Ok(())
}

#[test]
fn review3_timevalue_accepts_twelve_hour_time_without_changing_twenty_four_hour_rules() {
    let book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=TIMEVALUE(\"6:45 PM\")", 18.75 / 24.0),
        ("=TIMEVALUE(\"12:00 AM\")", 0.0),
        ("=TIMEVALUE(\"12:00 PM\")", 0.5),
        ("=TIMEVALUE(\" 6 pm \")", 0.75),
        ("=TIMEVALUE(\"18:45\")", 18.75 / 24.0),
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Number(expected),
            "{input}"
        );
    }
    for input in [
        "=TIMEVALUE(\"13:15 PM\")",
        "=TIMEVALUE(\"12:60 AM\")",
        "=TIMEVALUE(\"24:00\")",
        "=TIMEVALUE(\"word\")",
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Error("#VALUE!".into()),
            "{input}"
        );
    }
}
#[test]
fn review3_range_paste_rejects_merged_cells_before_modifying_any_destination() -> Result<(), String>
{
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "original")?;
    book.merge(id, Range::new(0, 0, 0, 1))?;
    let before = book.clone();
    assert!(
        book.set_range(id, 0, 0, &[vec!["new".into(), "hidden".into()]])
            .is_err()
    );
    assert_eq!(book, before);
    assert!(book.set_range(id, 0, 1, &[vec!["hidden".into()]]).is_err());
    assert_eq!(book, before);
    book.set_range(id, 0, 0, &[vec!["visible".into()]])?;
    assert_eq!(book.read_range(id, 0, 0, 0, 0)?, vec![vec!["visible"]]);
    assert!(
        book.active_sheet()
            .and_then(|sheet| sheet.cell(0, 1))
            .is_none()
    );
    Ok(())
}

#[test]
fn review4_unicode_names_and_named_sum_ranges_keep_identity() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["yes".into(), "10".into()],
            vec!["yes".into(), "20".into()],
        ],
    )?;
    book.names.insert("Данные".into(), "=B1:B2".into());
    book.names.insert("Начало".into(), "=B1".into());
    book.names.insert("Копия".into(), "=НАЧАЛО".into());
    for (formula, expected) in [
        ("=SUM(ДАННЫЕ)", 30.0),
        ("=ROW(НАЧАЛО)", 1.0),
        ("=SUMIF(A1:A2,\"yes\",КОПИЯ)", 30.0),
        ("=AVERAGEIF(A1:A2,\"yes\",КОПИЯ)", 15.0),
    ] {
        assert_eq!(
            formula::evaluate(&book, id, formula),
            Value::Number(expected),
            "{formula}"
        );
    }
    Ok(())
}
#[test]
fn review4_days_preserves_numeric_fractions_and_rejects_invalid_serials() {
    let book = Book::default();
    let id = book.active_id();
    assert_eq!(
        formula::evaluate(&book, id, "=DAYS(2.75,1.5)"),
        Value::Number(1.25)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=DAYS(\"2026-10-15\",\"2026-10-13\")"),
        Value::Number(2.0)
    );
    for formula in ["=DAYS(-1,1)", "=DAYS(1,2958466)", "=DAYS(3000000,1)"] {
        assert_eq!(
            formula::evaluate(&book, id, formula),
            Value::Error("#NUM!".into()),
            "{formula}"
        );
    }
    assert_eq!(
        formula::evaluate(&book, id, "=DAYS(\"invalid\",1)"),
        Value::Error("#VALUE!".into())
    );
}
#[test]
fn review4_financial_payment_type_is_zero_or_nonzero_not_a_multiplier() {
    let book = Book::default();
    let id = book.active_id();
    for (prefix, standard) in [
        ("PMT(0.05,12,1000,0,", "=PMT(0.05,12,1000,0,1)"),
        ("PV(0.05,12,-100,0,", "=PV(0.05,12,-100,0,1)"),
        ("FV(0.05,12,-100,0,", "=FV(0.05,12,-100,0,1)"),
        ("NPER(0.05,-100,1000,0,", "=NPER(0.05,-100,1000,0,1)"),
        ("RATE(12,-100,1000,0,", "=RATE(12,-100,1000,0,1)"),
        ("IPMT(0.05,2,12,1000,0,", "=IPMT(0.05,2,12,1000,0,1)"),
        ("PPMT(0.05,2,12,1000,0,", "=PPMT(0.05,2,12,1000,0,1)"),
    ] {
        let expected = formula::evaluate(&book, id, standard);
        assert!(
            matches!(&expected, Value::Number(_)),
            "{standard}: {expected:?}"
        );
        for kind in ["2", "-1", "0.5"] {
            let actual = format!("={prefix}{kind})");
            assert_eq!(formula::evaluate(&book, id, &actual), expected, "{actual}");
        }
    }
}
#[test]
fn review4_mround_compares_signs_before_a_product_can_underflow() {
    let book = Book::default();
    let id = book.active_id();
    for formula in ["=MROUND(1E-200,-1E-200)", "=MROUND(-1E-200,1E-200)"] {
        assert_eq!(
            formula::evaluate(&book, id, formula),
            Value::Error("#NUM!".into())
        );
    }
    assert_eq!(
        formula::evaluate(&book, id, "=MROUND(2E-200,1E-200)"),
        Value::Number(2E-200)
    );
}
#[test]
fn review4_statistics_keep_finite_results_at_large_and_small_scales() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["-1E200".into(), "-1E-200".into()],
            vec!["0".into(), "0".into()],
            vec!["1E200".into(), "1E-200".into()],
        ],
    )?;
    for (formula, expected) in [
        ("=CORREL(A1:A3,B1:B3)", 1.0),
        ("=PEARSON(A1:A3,B1:B3)", 1.0),
        ("=RSQ(A1:A3,B1:B3)", 1.0),
        ("=HARMEAN(1E-308,1E-308,1E-308)", 1E-308),
        ("=AVERAGE(1E308,1E308)", 1E308),
        ("=STDEV.P(-1E-200,1E-200)", 1E-200),
        ("=VAR.P(-1E154,1E154)", 1E308),
        ("=MEDIAN(1E308,1E308)", 1E308),
    ] {
        let Value::Number(actual) = formula::evaluate(&book, id, formula) else {
            return Err(formula.into());
        };
        assert!(
            ((actual / expected) - 1.0).abs() < 1E-12,
            "{formula}: {actual}"
        );
    }
    book.set_range(id, 0, 2, &[vec!["-1E308".into()], vec!["1E308".into()]])?;
    assert_eq!(
        formula::evaluate(&book, id, "=PERCENTILE(C1:C2,0.5)"),
        Value::Number(0.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=CORREL(A1:A3,A1:A1)"),
        Value::Error("#N/A".into())
    );
    Ok(())
}
#[test]
fn review4_statistical_argument_counts_reject_ignored_parameters() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_range(id, 0, 0, &[vec!["1".into()], vec!["2".into()]])?;
    for formula in [
        "=COUNTBLANK(A1,A2)",
        "=LARGE(A1:A2,1,99)",
        "=SMALL(A1:A2,1,99)",
        "=PERCENTILE(A1:A2,0.5,99)",
        "=QUARTILE(A1:A2,2,99)",
    ] {
        assert_eq!(
            formula::evaluate(&book, id, formula),
            Value::Error("#VALUE!".into()),
            "{formula}"
        );
    }
    assert_eq!(
        formula::evaluate(&book, id, "=RANK(1,A1:A2,1)"),
        Value::Number(1.0)
    );
    Ok(())
}
#[test]
fn review4_rewrites_fail_atomically_when_reference_growth_exceeds_cell_limit() -> Result<(), String>
{
    let mut book = Book::default();
    let id = book.active_id();
    let formula = format!("=A1&\"{}\"", "x".repeat(32767 - 6));
    assert_eq!(formula.len(), 32767);
    book.set_cell(id, 0, 1, &formula)?;
    let before = book.clone();
    assert!(
        book.fill(id, Range::new(0, 1, 0, 1), Range::new(1, 1, 9, 1))
            .is_err()
    );
    assert_eq!(book, before);
    assert!(book.edit_dimension(id, 0, 9, true, false).is_err());
    assert_eq!(book, before);
    let other = book.add_sheet("B")?;
    let prefix = "=B!A1&\"";
    book.set_cell(
        id,
        0,
        1,
        &format!("{prefix}{}\"", "x".repeat(32767 - prefix.len() - 1)),
    )?;
    let before = book.clone();
    assert!(
        book.rename_sheet(other, "A much longer sheet name")
            .is_err()
    );
    assert_eq!(book, before);
    book.set_cell(id, 0, 1, &formula)?;
    for row in 0..10 {
        book.set_input(id, row, 0, &(10 - row).to_string())?;
    }
    book.recalculate();
    let before = book.clone();
    assert!(
        book.sort(id, Range::new(0, 0, 9, 1), &[(0, false)], false)
            .is_err()
    );
    assert_eq!(book, before);
    book.validate()
}
#[test]
fn review4_reordering_a_background_sheet_preserves_active_id_selection_and_cell_ids()
-> Result<(), String> {
    let mut book = Book::default();
    let first = book.active_id();
    let second = book.add_sheet("Second")?;
    book.add_sheet("Third")?;
    let cell = book.set_cell(first, 3, 4, "editing here")?;
    book.selection = Selection {
        row: 3,
        col: 4,
        anchor_row: 1,
        anchor_col: 2,
    };
    let selection = book.selection.clone();
    book.move_sheet(second, 0)?;
    assert_eq!(book.active_id(), first);
    assert_eq!(book.selection, selection);
    assert_eq!(
        book.active_sheet()
            .and_then(|sheet| sheet.cell(3, 4))
            .map(|value| value.id),
        Some(cell)
    );
    book.move_sheet(first, 2)?;
    assert_eq!(book.active_id(), first);
    assert_eq!(book.selection, selection);
    book.validate()
}

#[test]
fn review5_names_reject_control_characters_without_changing_the_book() -> Result<(), String> {
    let mut book = Book::default();
    let before = book.clone();
    for name in ["Bad\nName", "Bad\tName", "Bad\u{0001}Name"] {
        assert!(book.add_sheet(name).is_err());
        assert!(book.rename_sheet(book.active_id(), name).is_err());
        assert_eq!(book, before);
    }
    Ok(())
}
#[test]
fn review5_empty_fill_stays_sparse_and_preserves_unrelated_cells() -> Result<(), String> {
    let mut book = Book::default();
    let sheet = book.active_id();
    let keep = book.set_cell(sheet, 20, 20, "keep")?;
    book.set_cell(sheet, 1, 1, "erase")?;
    book.fill(sheet, Range::new(0, 0, 0, 0), Range::new(1, 1, 10, 10))?;
    let model = book.sheet_by_id(sheet).ok_or("sheet")?;
    assert_eq!(model.cells.len(), 1);
    assert_eq!(model.cell(20, 20).ok_or("keep")?.id, keep);
    Ok(())
}
#[test]
fn review5_fill_rejects_covered_merge_targets_before_changing_any_cells() -> Result<(), String> {
    let mut book = Book::default();
    let sheet = book.active_id();
    book.set_cell(sheet, 0, 0, "Source")?;
    book.merge(sheet, Range::new(1, 1, 2, 2))?;
    let before = book.clone();
    for target in [Range::new(1, 1, 3, 3), Range::new(2, 2, 2, 2)] {
        assert!(book.fill(sheet, Range::new(0, 0, 0, 0), target).is_err());
        assert_eq!(book, before);
    }
    book.fill(sheet, Range::new(0, 0, 0, 0), Range::new(1, 1, 1, 1))?;
    assert_eq!(
        book.sheet_by_id(sheet)
            .and_then(|s| s.cell(1, 1))
            .ok_or("anchor")?
            .value,
        Value::Text("Source".into())
    );
    Ok(())
}

#[test]
fn review5_value_uses_locale_and_recognizes_currency_dates_and_times() {
    for (ru, expression, expected) in [
        (false, "=VALUE(\"1,000\")", 1000.0),
        (false, "=VALUE(\"$1,000\")", 1000.0),
        (false, "=VALUE(\"(1,000.5)\")", -1000.5),
        (true, "=VALUE(\"1 000,5 ₽\")", 1000.5),
        (true, "=VALUE(\"1.25\")", 1.25),
        (false, "=VALUE(\"16:48:00\")-VALUE(\"12:00:00\")", 0.2),
    ] {
        let book = Book {
            locale_ru: ru,
            ..Default::default()
        };
        let Value::Number(actual) = formula::evaluate(&book, book.active_id(), expression) else {
            panic!("{expression}");
        };
        assert!((actual - expected).abs() < 1E-12, "{expression}: {actual}");
    }
    let book = Book::default();
    assert_eq!(
        formula::evaluate(&book, book.active_id(), "=VALUE(\"2026-10-16\")"),
        formula::evaluate(&book, book.active_id(), "=DATE(2026,10,16)")
    );
}
#[test]
fn review5_char_code_use_ansi_while_unicode_functions_keep_scalar_values() {
    for (ru, formula, expected) in [
        (false, "=CHAR(128)", Value::Text("€".into())),
        (false, "=CODE(\"€\")", Value::Number(128.0)),
        (true, "=CHAR(192)", Value::Text("А".into())),
        (true, "=CODE(\"А\")", Value::Number(192.0)),
        (true, "=UNICODE(\"А\")", Value::Number(1040.0)),
        (true, "=UNICHAR(1040)", Value::Text("А".into())),
    ] {
        let book = Book {
            locale_ru: ru,
            ..Default::default()
        };
        assert_eq!(
            formula::evaluate(&book, book.active_id(), formula),
            expected,
            "{formula}"
        );
    }
}
#[test]
fn review5_working_dates_cover_large_valid_spans_and_validate_holidays() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=WORKDAY(2,200000)", 280002.0),
        ("=WORKDAY(280002,-200000)", 2.0),
        ("=NETWORKDAYS(2,1400002)", 1000001.0),
        ("=NETWORKDAYS(1400002,2)", -1000001.0),
    ] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Number(expected),
            "{input}"
        );
    }
    book.set_cell(id, 0, 0, "-1")?;
    for input in ["=WORKDAY(40000,10,A1)", "=NETWORKDAYS(40000,40010,A1)"] {
        assert_eq!(
            formula::evaluate(&book, id, input),
            Value::Error("#VALUE!".into())
        );
    }
    book.set_range(
        id,
        0,
        0,
        &[vec!["3".into()], vec!["3".into()], vec!["7".into()]],
    )?;
    assert_eq!(
        formula::evaluate(&book, id, "=WORKDAY(2,5,A1:A3)"),
        Value::Number(10.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=NETWORKDAYS(2,9,A1:A3)"),
        Value::Number(5.0)
    );
    Ok(())
}
#[test]
fn review5_finance_retains_the_zero_rate_limit_and_irr_is_scale_invariant() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    for (input, expected) in [
        ("=PMT(1E-20,10,100)", -10.0),
        ("=PV(1E-20,10,-10)", 100.0),
        ("=FV(1E-20,10,-10)", 100.0),
        ("=NPER(1E-20,-10,100)", 10.0),
        ("=PPMT(1E-20,1,10,100)", -10.0),
    ] {
        let Value::Number(actual) = formula::evaluate(&book, id, input) else {
            return Err(input.into());
        };
        assert!((actual - expected).abs() < 1E-10, "{input}: {actual}");
    }
    book.set_range(
        id,
        0,
        0,
        &[
            vec!["-1E308".into()],
            vec!["1E308".into()],
            vec!["1E308".into()],
        ],
    )?;
    let Value::Number(actual) = formula::evaluate(&book, id, "=IRR(A1:A3)") else {
        return Err("IRR".into());
    };
    assert!((actual - (5_f64.sqrt() - 1.0) / 2.0).abs() < 1E-9);
    Ok(())
}
#[test]
fn review5_gcd_and_lcm_use_the_exact_integer_domain() {
    let book = Book::default();
    let id = book.active_id();
    assert_eq!(
        formula::evaluate(&book, id, "=GCD(9005000000000000,1)"),
        Value::Number(1.0)
    );
    assert_eq!(
        formula::evaluate(&book, id, "=LCM(100000000,100000003)"),
        Value::Error("#NUM!".into())
    );
}

#[test]
fn root_review5_defined_name_length_counts_unicode_characters() -> Result<(), String> {
    let mut book = Book::default();
    book.names.insert("Я".repeat(255), "=A1".into());
    book.validate()?;
    book.names.insert("Ю".repeat(256), "=A1".into());
    assert!(book.validate().is_err());
    Ok(())
}

#[test]
fn root_review5_defined_name_backslash_prefix_and_unicode_periods_are_evaluable() {
    let mut book = Book::default();
    book.names.insert(r"\Бюджет.2026".into(), "=42".into());
    book.names
        .insert("_Budget2026".into(), r"=\Бюджет.2026".into());
    book.names.insert("Данные.2".into(), "=5".into());
    assert_eq!(
        formula::evaluate(&book, book.active_id(), r"=\БЮДЖЕТ.2026"),
        Value::Number(42.0)
    );
    assert_eq!(
        formula::evaluate(&book, book.active_id(), "=_Budget2026+Данные.2"),
        Value::Number(47.0)
    );
}

#[test]
fn library_functions_fill_the_gaps() -> Result<(), String> {
    let b = seeded()?;
    let id = b.active_id();
    for (input, expected) in [
        ("=NORM.S.DIST(1,TRUE)", 0.841_344_746_068_543),
        ("=_xlfn.NORM.S.DIST(1,TRUE)", 0.841_344_746_068_543),
        ("=FISHER(0.75)", 0.972_955_074_527_656_6),
        ("=COMBINA(4,3)", 20.0),
        ("=GEOMEAN(A1:A3)", 1.817_120_592_832_139_7),
        ("=PERCENTRANK.INC(A1:A3,2)", 0.5),
        ("=EFFECT(0.0525,4)", 0.053_542_667_370_758_3),
        ("=MDETERM(C1:D2)", 800.0),
        ("=ERROR.TYPE(1/0)", 2.0),
        ("=SEQUENCE(3)", 1.0),
        ("=SUM(SEQUENCE(3))", 6.0),
    ] {
        let actual = formula::evaluate(&b, id, input);
        let Value::Number(n) = actual else {
            return Err(format!("{input}: {actual:?}, expected {expected}"));
        };
        assert!(
            (n - expected).abs() <= 1e-9 * expected.abs().max(1.0),
            "{input}: {n}, expected {expected}"
        );
    }
    for (input, expected) in [
        (
            "=REGEXEXTRACT(\"tel 555-1234\",\"[0-9]{3}-[0-9]{4}\")",
            "555-1234",
        ),
        ("=REGEXREPLACE(\"a1b22c\",\"[0-9]+\",\"#\")", "a#b#c"),
        ("=TEXTSPLIT(\"a,b\",\",\")", "a"),
    ] {
        assert_eq!(
            formula::evaluate(&b, id, input),
            Value::Text(expected.into()),
            "{input}"
        );
    }
    assert_eq!(
        formula::evaluate(&b, id, "=REGEXTEST(\"abc123\",\"[0-9]+\")"),
        Value::Bool(true)
    );
    assert_eq!(
        formula::evaluate(&b, id, "=NORM.S.DIST(\"x\",TRUE)"),
        Value::Error("#VALUE!".into())
    );
    assert_eq!(
        formula::evaluate(&b, id, "=COMBINA(1)"),
        Value::Error("#VALUE!".into())
    );

    let mut db = Book::default();
    let sheet = db.active_id();
    db.set_range(
        sheet,
        0,
        0,
        &[
            vec!["Fruit".into(), "Sales".into(), "".into(), "Fruit".into()],
            vec!["apple".into(), "10".into(), "".into(), "apple".into()],
            vec!["pear".into(), "20".into()],
            vec!["apple".into(), "5".into()],
        ],
    )?;
    assert_eq!(
        formula::evaluate(&db, sheet, "=DSUM(A1:B4,\"Sales\",D1:D2)"),
        Value::Number(15.0)
    );
    assert_eq!(
        formula::evaluate(&db, sheet, "=DCOUNT(A1:B4,2,D1:D2)"),
        Value::Number(2.0)
    );
    Ok(())
}

fn shown(book: &Book, row: u32, col: u32) -> Option<Value> {
    book.active_sheet()
        .and_then(|sheet| sheet.value(row, col))
        .cloned()
}

#[test]
fn array_results_spill_into_empty_cells() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 0, 0, "=SEQUENCE(3,2)")?;
    assert_eq!(shown(&book, 0, 0), Some(Value::Number(1.0)));
    assert_eq!(shown(&book, 0, 1), Some(Value::Number(2.0)));
    assert_eq!(shown(&book, 2, 0), Some(Value::Number(5.0)));
    assert_eq!(shown(&book, 2, 1), Some(Value::Number(6.0)));
    // A formula that reads the spilled area sees the values from the earlier pass.
    book.set_cell(id, 0, 3, "=SUM(A1:B3)")?;
    assert_eq!(shown(&book, 0, 3), Some(Value::Number(21.0)));
    Ok(())
}

#[test]
fn obstructed_spill_shows_spill_error_and_recovers() -> Result<(), String> {
    let mut book = Book::default();
    let id = book.active_id();
    book.set_cell(id, 1, 0, "x")?;
    book.set_cell(id, 0, 0, "=SEQUENCE(3)")?;
    assert_eq!(shown(&book, 0, 0), Some(Value::Error("#SPILL!".into())));
    assert_eq!(shown(&book, 2, 0), None);
    book.set_cell(id, 1, 0, "")?;
    assert_eq!(shown(&book, 0, 0), Some(Value::Number(1.0)));
    assert_eq!(shown(&book, 2, 0), Some(Value::Number(3.0)));
    Ok(())
}
