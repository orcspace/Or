// Vendored from GridCraft (https://github.com/storytold/gridcraft), crates/core.
// Copyright (c) 2026 ArtCraft Team and the GridCraft contributors.
// Licensed under the MIT License or the Apache License, Version 2.0, at your option.
// Adapted for TrineDocs: only the value, date and parsing parts the function library needs;
// see THIRD-PARTY-NOTICES.md and ADR-0028.

//! Core types for the worksheet function library: values and Excel's coercion rules, error
//! values, date serials and typed-input parsing.

pub mod date;
pub mod parse;
pub mod value;

pub use date::DateSystem;
pub use value::{
    Array, CellError, Value, compare, compare_numbers, compare_text, number_to_text, sort_compare,
};

/// `0` → `A`, `16383` → `XFD`.
pub fn col_to_letters(col: u32) -> String {
    let mut n = col as u64 + 1;
    let mut out = Vec::with_capacity(3);
    while n > 0 {
        let rem = ((n - 1) % 26) as u8;
        out.push(b'A' + rem);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}
