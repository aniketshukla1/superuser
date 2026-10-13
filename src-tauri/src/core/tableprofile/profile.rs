use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    /// Numbers — the only type that belongs on a value axis.
    Number,
    /// Dates. A natural x-axis, and the reason a chart is a line rather than bars.
    Date,
    /// Few distinct values relative to the rows: a grouping, not a label.
    Category,
    /// Mostly distinct text — names, ids, free text. Plotting one is a thousand bars.
    Text,
    Empty,
}

#[derive(Debug, Clone, Serialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ColumnType,
    pub filled: usize,
    pub distinct: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Categories only: commonest values first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub top: Vec<(String, usize)>,
    /// A couple of real values, so the agent sees the shape instead of inferring it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sample: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub rows: usize,
    pub columns: Vec<Column>,
}

pub(super) fn as_number(s: &str) -> Option<f64> {
    let t = s.trim().replace([',', '$', '£', '€', '%'], "");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

/// Dates by shape, not by parsing every locale. Being wrong here costs a chart type, not
/// correctness, and a full date parser for every regional format is a library.
pub(super) fn looks_like_date(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 6 || t.len() > 32 {
        return false;
    }
    let seps = t.chars().filter(|c| *c == '-' || *c == '/').count();
    let digits = t.chars().filter(char::is_ascii_digit).count();
    (seps == 2
        && (4..=8).contains(&digits)
        && t.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '/'))
        || (t.len() >= 10 && t.contains('T') && t.chars().take(10).filter(|c| *c == '-').count() == 2)
}

/// Profile rows of strings, the first being the header.
///
/// Split from any file format on purpose: a workbook, a CSV and a query result are all rows
/// of strings by the time they arrive, and what to SAY about them is the part no format
/// changes.
pub fn profile(rows: &[Vec<String>]) -> Profile {
    let Some((header, body)) = rows.split_first() else {
        return Profile { rows: 0, columns: Vec::new() };
    };
    let width = header.len().max(body.iter().map(Vec::len).max().unwrap_or(0));
    let mut columns = Vec::with_capacity(width);

    for c in 0..width {
        let name = header.get(c).cloned().unwrap_or_else(|| format!("column {}", c + 1));
        let values: Vec<&str> = body
            .iter()
            .map(|r| r.get(c).map(String::as_str).unwrap_or("").trim())
            .filter(|v| !v.is_empty())
            .collect();

        let filled = values.len();
        let mut counts: std::collections::HashMap<&str, usize> = Default::default();
        for v in &values {
            *counts.entry(*v).or_default() += 1;
        }
        let distinct = counts.len();
        let nums: Vec<f64> = values.iter().filter_map(|v| as_number(v)).collect();
        let dates = values.iter().filter(|v| looks_like_date(v)).count();

        // Majority rules: empty, then numbers, then dates, then cardinality.
        //
        // The number/date order is NOT load-bearing and this comment used to claim it was.
        // `looks_like_date` requires separators and `as_number` rejects them, so a value
        // cannot satisfy both — swapping the two arms changes nothing, which is what
        // mutating them proved. Left in this order because it reads in decreasing
        // specificity, not because a precedence is being expressed.
        let kind = if filled == 0 {
            ColumnType::Empty
        } else if nums.len() * 2 > filled {
            ColumnType::Number
        } else if dates * 2 > filled {
            ColumnType::Date
        // At least one repeat, and few enough values to be a grouping. `* 3` demanded each
        // value appear three times, which on a four-row table classified a plain two-value
        // column (EU, EU, US, US) as free text — and free text is the one thing you cannot
        // group a chart by. `* 2` is "something repeats", which is what a category means.
        } else if distinct <= 25 && distinct * 2 <= filled.max(1) {
            ColumnType::Category
        } else {
            ColumnType::Text
        };

        let (min, max) = if kind == ColumnType::Number && !nums.is_empty() {
            (
                Some(nums.iter().cloned().fold(f64::INFINITY, f64::min)),
                Some(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
            )
        } else {
            (None, None)
        };

        let top: Vec<(String, usize)> = if kind == ColumnType::Category {
            let mut v: Vec<(String, usize)> =
                counts.iter().map(|(k, n)| ((*k).to_string(), *n)).collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            v.truncate(8);
            v
        } else {
            Vec::new()
        };

        columns.push(Column {
            name,
            kind,
            filled,
            distinct,
            min,
            max,
            top,
            sample: values.iter().take(3).map(|v| v.to_string()).collect(),
        });
    }

    Profile { rows: body.len(), columns }
}
