use super::*;

/// Read a rectangle of values as rows of strings.
///
/// Capped, because a workbook can be enormous and a tool result goes into the
/// model's context: an unbounded read is how one spreadsheet eats a whole turn.
/// Split one delimited line, honouring quotes.
///
/// Not a full RFC 4180 parser, but it handles the case that actually breaks naive splitting:
/// a comma inside a quoted field. Real exports are full of `"Smith, J."` and `"1,200"`, and
/// splitting on every comma turns one column into two and shifts the whole row.
pub(super) fn split_delimited(line: &str, delim: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                // "" inside a quoted field is one literal quote.
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == delim && !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Read a CSV or TSV as rows.
///
/// Separate from the workbook reader because they share nothing: one unzips XML, the other
/// splits lines. They meet at `read_rows`, which is where every caller already is.
pub(super) fn read_delimited(path: &Path, max_rows: usize, max_cols: usize) -> Result<Vec<Vec<String>>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("couldn't open {}: {e}", path.display()))?;
    let delim = if path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() == Some("tsv") {
        '\t'
    } else {
        ','
    };
    Ok(text
        .lines()
        // A trailing newline is not a row, and a blank line mid-file is not a row of empty
        // columns — either would show up in a profile as missing data that is not there.
        .filter(|l| !l.trim().is_empty())
        .take(max_rows.max(1))
        .map(|l| {
            let mut row = split_delimited(l, delim);
            row.truncate(max_cols.max(1));
            row.iter_mut().for_each(|c| *c = c.trim().to_string());
            row
        })
        .collect())
}

pub fn read_rows(path: &Path, sheet: Option<&str>, max_rows: usize, max_cols: usize) -> Result<Vec<Vec<String>>, String> {
    // CSV and TSV first: the workbook reader would report one as "corrupt", which is what it
    // did — a dropped CSV failed sheet_profile with a message about a broken xlsx, and the
    // model quite reasonably told the user their file was damaged.
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext == "csv" || ext == "tsv" {
        return read_delimited(path, max_rows, max_cols);
    }
    let book = umya_spreadsheet::reader::xlsx::read(path)
        .map_err(|e| format!("couldn't open {}: {e}", path.display()))?;
    let name = sheet_name(&book, sheet)?;
    let ws = book.get_sheet_by_name(&name).map_err(|e| format!("no sheet called `{name}`: {e}"))?;
    let (max_col, max_row) = ws.get_highest_column_and_row();
    let rows = (max_row as usize).min(max_rows.max(1));
    let cols = (max_col as usize).min(max_cols.max(1));
    let mut out = Vec::with_capacity(rows);
    for r in 1..=rows {
        let mut row = Vec::with_capacity(cols);
        for c in 1..=cols {
            row.push(ws.get_value((c as u32, r as u32)));
        }
        // Trailing empties carry nothing; a wall of commas in the model's context is
        // noise it pays for.
        while row.last().map(|v| v.is_empty()).unwrap_or(false) {
            row.pop();
        }
        out.push(row);
    }
    Ok(out)
}
