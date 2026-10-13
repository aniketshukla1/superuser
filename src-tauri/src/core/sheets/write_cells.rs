use super::*;

/// One cell to set. `value` is interpreted, not stored blindly — see `Value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellEdit {
    /// A1-style reference: "B7".
    pub cell: String,
    pub value: String,
    /// For a formula, its computed result — the *cached value* in the file.
    ///
    /// This is not decoration. A formula cell written with an empty `<v/>` is
    /// dropped entirely by readers that don't recalculate, including SheetJS (which
    /// the panel uses) and our own `files::extract` (which is how the agent reads
    /// its own work back). Measured 2026-08-21: a `=SUM(B2:B6)` written without a
    /// result round-tripped to **nothing** — the formula was in the XML and every
    /// reader returned `undefined` for the cell.
    ///
    /// `None` keeps the old behaviour, so a caller with no engine to compute with
    /// (the agent's `sheet_write`) is unchanged.
    #[serde(default)]
    pub result: Option<String>,
}

/// What a string means once it is in a spreadsheet.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Formula(String),
    Text(String),
    Empty,
}

/// Read a cell's intent from the text a model produced.
///
/// The rules are deliberately few, because surprising coercion is worse than none:
/// a leading `=` is a formula, something that parses as a number is a number, and
/// everything else is text. Note what is NOT coerced — "007" and "+44 20 7946"
/// stay text, because turning an identifier into 7 is the classic spreadsheet
/// injury and it is not recoverable from the saved file.
pub fn interpret(raw: &str) -> Value {
    let t = raw.trim();
    if t.is_empty() {
        return Value::Empty;
    }
    if let Some(f) = t.strip_prefix('=') {
        return Value::Formula(f.to_string());
    }
    // A number that was written like an identifier is an identifier.
    let looks_padded = t.len() > 1 && t.starts_with('0') && !t.starts_with("0.");
    if !looks_padded {
        if let Ok(n) = t.parse::<f64>() {
            if n.is_finite() {
                return Value::Number(n);
            }
        }
    }
    Value::Text(t.to_string())
}

/// Is this a plausible A1 reference?
///
/// Checked before touching the file: the library will happily accept nonsense and
/// create a cell nobody can find, and a silent no-op is the hardest kind of bug to
/// see in a spreadsheet.
pub fn is_cell_ref(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.len() > 10 {
        return false;
    }
    let mut chars = t.chars();
    let mut letters = 0;
    let mut digits = 0;
    for c in chars.by_ref() {
        if c.is_ascii_alphabetic() && digits == 0 {
            letters += 1;
        } else if c.is_ascii_digit() {
            digits += 1;
        } else {
            return false;
        }
    }
    letters > 0 && letters <= 3 && digits > 0
}

/// The sheet to act on: the named one, or the first if no name was given.
pub(super) fn sheet_name(book: &umya_spreadsheet::Workbook, wanted: Option<&str>) -> Result<String, String> {
    let names: Vec<String> = book.get_sheet_collection().iter().map(|s| s.name().to_string()).collect();
    match wanted.map(str::trim).filter(|s| !s.is_empty()) {
        None => names.first().cloned().ok_or_else(|| "this workbook has no sheets".into()),
        Some(w) => names
            .iter()
            .find(|n| n.eq_ignore_ascii_case(w))
            .cloned()
            // Naming a sheet that isn't there is a mistake worth reporting, not one
            // to paper over by writing to whichever sheet happened to be first.
            .ok_or_else(|| format!("no sheet called `{w}` — this workbook has: {}", names.join(", "))),
    }
}

/// Create a workbook with the given sheets. Fails if something is already there,
/// so "create" can never be a silent overwrite of the user's work.
pub fn create(path: &Path, sheets: &[String]) -> Result<(), String> {
    if path.exists() {
        return Err(format!("{} already exists — edit it instead of replacing it", path.display()));
    }
    let mut book = umya_spreadsheet::new_file();
    let first = sheets.first().cloned().unwrap_or_else(|| "Sheet1".into());
    // new_file() ships a "Sheet1"; rename it rather than leaving an empty stray.
    book.get_sheet_by_name_mut("Sheet1")
        .map_err(|e| format!("couldn't prepare the first sheet: {e}"))?
        .set_name(&first);
    for name in sheets.iter().skip(1) {
        book.new_sheet(name).map_err(|e| format!("couldn't add sheet `{name}`: {e}"))?;
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    umya_spreadsheet::writer::xlsx::write(&book, path).map_err(|e| format!("couldn't write the workbook: {e}"))
}

/// Set specific cells, leaving the rest of the workbook exactly as it was.
pub fn write_cells(path: &Path, sheet: Option<&str>, edits: &[CellEdit]) -> Result<usize, String> {
    for e in edits {
        if !is_cell_ref(&e.cell) {
            return Err(format!("`{}` isn't a cell reference like B7", e.cell));
        }
    }
    // Read the file that is there, so everything not named here survives.
    let mut book = if path.exists() {
        umya_spreadsheet::reader::xlsx::read(path).map_err(|e| format!("couldn't open {}: {e}", path.display()))?
    } else {
        umya_spreadsheet::new_file()
    };
    let name = sheet_name(&book, sheet)?;
    let ws = book
        .get_sheet_by_name_mut(&name)
        .map_err(|e| format!("no sheet called `{name}`: {e}"))?;
    for e in edits {
        let cell = ws.get_cell_mut(e.cell.trim().to_uppercase().as_str());
        match interpret(&e.value) {
            Value::Number(n) => {
                cell.set_value_number(n);
            }
            Value::Formula(f) => {
                cell.set_formula(f);
                // Excel recalculates on open; nothing else does. Without a cached
                // result the cell is invisible to every other reader.
                if let Some(r) = e.result.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
                    match r.parse::<f64>() {
                        Ok(n) if n.is_finite() => {
                            cell.set_formula_result_number(n);
                        }
                        _ => {
                            cell.set_formula_result_default(r);
                        }
                    }
                }
            }
            Value::Text(t) => {
                cell.set_value_string(t);
            }
            Value::Empty => {
                cell.set_value_string("");
            }
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    umya_spreadsheet::writer::xlsx::write(&book, path).map_err(|e| format!("couldn't save: {e}"))?;
    Ok(edits.len())
}

/// The sheets in a workbook, in order.
pub fn sheet_names(path: &Path) -> Result<Vec<String>, String> {
    let book = umya_spreadsheet::reader::xlsx::read(path)
        .map_err(|e| format!("couldn't open {}: {e}", path.display()))?;
    Ok(book.get_sheet_collection().iter().map(|s| s.name().to_string()).collect())
}
