//! CSV, small and strict

use super::*;

/// One field as CSV. Quoted when it must be, and a field a spreadsheet would run as a formula
/// (`=`, `+`, `-`, `@` first) is written as text: a merchant name is text someone else chose.
pub(super) fn csv_field(value: &str) -> String {
    let v = if value.starts_with(['=', '+', '-', '@']) { format!("'{value}") } else { value.to_string() };
    if v.contains([',', '"', '\n', '\r']) || v != v.trim() {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

pub(crate) fn csv_line(fields: &[&str]) -> String {
    fields.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(",")
}

/// Rows of fields: quotes, doubled quotes and line breaks inside quotes handled.
pub(crate) fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Every receipt in the ledger, oldest first. A ledger that can't be read reads as empty.
pub fn load(data_dir: &Path) -> Vec<Expense> {
    let Ok(text) = std::fs::read_to_string(ledger_path(data_dir)) else { return Vec::new() };
    csv_rows(&text).into_iter().skip(1).filter_map(|f| Expense::from_fields(&f)).collect()
}

/// What adding a receipt did.
#[derive(Debug, Clone, PartialEq)]
pub enum Added {
    Logged,
    /// This email was logged before.
    AlreadyLogged,
}

/// Add a receipt. The same email is never logged twice; the same purchase from two emails is
/// logged with a note, and the summary lists it as a possible duplicate.
pub fn append(data_dir: &Path, mut e: Expense) -> Result<Added, String> {
    let existing = load(data_dir);
    if !e.message_id.is_empty() && existing.iter().any(|x| x.message_id == e.message_id) {
        return Ok(Added::AlreadyLogged);
    }
    if let Some(twin) = existing.iter().find(|x| same_purchase(x, &e)) {
        let flag = format!("possible duplicate of {}", twin.id);
        e.note = if e.note.is_empty() { flag } else { format!("{}; {flag}", e.note) };
    }
    let mut text = std::fs::read_to_string(ledger_path(data_dir)).unwrap_or_default();
    if text.trim().is_empty() {
        text = format!("{}\n", COLUMNS.join(","));
    } else if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&csv_line(&e.fields()));
    text.push('\n');
    std::fs::create_dir_all(folder(data_dir)).map_err(|err| format!("couldn't make the expenses folder: {err}"))?;
    crate::core::store::write_atomic(&ledger_path(data_dir), &text).map_err(|err| format!("couldn't write the ledger: {err}"))?;
    Ok(Added::Logged)
}

pub(super) fn same_purchase(a: &Expense, b: &Expense) -> bool {
    a.date == b.date && a.amount == b.amount && a.currency == b.currency && a.merchant.eq_ignore_ascii_case(&b.merchant)
}
