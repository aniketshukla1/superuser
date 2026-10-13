use super::*;

/// The most rows a picture is allowed to carry. Past this, a chart is a file.
pub const MAX_ROWS: usize = 2_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
}

/// Whether this source names a file whose rows can be read.
pub fn is_readable(source: &str) -> bool {
    matches!(extension(source).as_deref(), Some("csv" | "tsv" | "json"))
}

pub(super) fn extension(source: &str) -> Option<String> {
    Path::new(source.trim()).extension().map(|e| e.to_string_lossy().to_lowercase())
}

/// Read the rows a source names, relative to the run's folder.
///
/// Errors are sentences: every caller hands them to a model that has to decide what to do
/// next, and "invalid utf-8 at byte 91" is not something to act on.
pub fn read(root: &Path, source: &str) -> Result<Rows, String> {
    let rel = source.trim();
    if rel.is_empty() || rel.split('/').any(|c| c == "..") {
        return Err("that source isn't a path in your folder".into());
    }
    let ext = extension(rel).ok_or_else(|| format!("`{rel}` has no extension, so there is no telling what is in it"))?;
    // Checked before the file is touched, so an unreadable KIND says so rather than
    // reporting whatever went wrong with opening it.
    if !is_readable(rel) {
        return Err(format!("`.{ext}` isn't something this can read rows from — use a CSV, a TSV or a JSON array"));
    }
    let full = root.join(rel);
    let text = std::fs::read_to_string(&full).map_err(|e| format!("couldn't read `{rel}`: {e}"))?;
    let rows = match ext.as_str() {
        "csv" => parse_separated(&text, ','),
        "tsv" => parse_separated(&text, '\t'),
        _ => parse_json(&text),
    }?;
    if rows.rows.is_empty() {
        return Err(format!("`{rel}` has no rows in it"));
    }
    Ok(rows)
}

/// A number if it reads as one, otherwise the text. What a chart needs to know about a
/// column is whether it can be measured, and a string that happens to be "12" cannot be.
pub(super) fn typed(cell: &str) -> Value {
    let t = cell.trim();
    if t.is_empty() {
        return Value::Null;
    }
    // A leading currency symbol or a thousands separator is still a number to a person.
    let cleaned: String = t.trim_start_matches(['$', '£', '€']).replace(',', "");
    if let Ok(n) = cleaned.parse::<f64>() {
        if n.is_finite() {
            return serde_json::Number::from_f64(n).map(Value::Number).unwrap_or_else(|| Value::String(t.into()));
        }
    }
    Value::String(t.to_string())
}

/// One line of a delimited file, respecting double quotes.
pub(super) fn split_line(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => out.push(std::mem::take(&mut cell)),
            c => cell.push(c),
        }
    }
    out.push(cell);
    out
}

pub(super) fn parse_separated(text: &str, sep: char) -> Result<Rows, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or("that file is empty")?;
    let columns: Vec<String> = split_line(header, sep).into_iter().map(|c| c.trim().to_string()).collect();
    if columns.iter().all(|c| c.is_empty()) {
        return Err("the first line has no column names in it".into());
    }
    let mut rows = Vec::new();
    for line in lines.take(MAX_ROWS) {
        let cells = split_line(line, sep);
        let mut row = Map::new();
        for (i, name) in columns.iter().enumerate() {
            if name.is_empty() {
                continue;
            }
            row.insert(name.clone(), cells.get(i).map(|c| typed(c)).unwrap_or(Value::Null));
        }
        rows.push(Value::Object(row));
    }
    Ok(Rows { columns, rows })
}

pub(super) fn parse_json(text: &str) -> Result<Rows, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("that file isn't valid JSON: {e}"))?;
    // An array of objects, or an object with one array in it — both are what people have.
    let array = match &v {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o
            .values()
            .find_map(|inner| inner.as_array().cloned())
            .ok_or("that JSON has no array of rows in it")?,
        _ => return Err("that JSON is neither an array of rows nor an object holding one".into()),
    };
    let rows: Vec<Value> = array.into_iter().filter(|r| r.is_object()).take(MAX_ROWS).collect();
    if rows.is_empty() {
        return Err("that JSON array has no objects in it".into());
    }
    let mut columns: Vec<String> = Vec::new();
    for row in &rows {
        for key in row.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
            if !columns.contains(&key) {
                columns.push(key);
            }
        }
    }
    Ok(Rows { columns, rows })
}
