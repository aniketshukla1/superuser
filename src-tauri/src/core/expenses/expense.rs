use super::*;

/// The assistant that reviews: a built-in that can only read (`personas_builtin/expenses.md`).
pub const ASSISTANT: &str = "expenses";

/// Receipts read per local day at most — each is a model call, and the mail waits past it.
pub const DAILY_CAP: u32 = 60;

/// What a receipt is filed under. A reply outside this list files it under none, and the
/// summary says so rather than inventing one.
pub const CATEGORIES: &[&str] = &["Travel", "Lodging", "Meals", "Transport", "Software", "Office", "Phone & internet", "Other"];

/// One receipt in the ledger.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Expense {
    pub id: String,
    /// The purchase date, `YYYY-MM-DD`.
    pub date: String,
    pub merchant: String,
    /// A decimal with two places, `12.34`.
    pub amount: String,
    /// ISO 4217, `USD`; empty when the receipt didn't say, and the summary flags it.
    pub currency: String,
    /// One of `CATEGORIES`, or empty.
    pub category: String,
    pub note: String,
    /// Who sent the receipt.
    pub source: String,
    /// The receipt email's subject — what a total reconciles back to.
    pub subject: String,
    pub message_id: String,
    /// When it was logged, RFC 3339.
    pub added: String,
}

pub(super) const COLUMNS: &[&str] = &["id", "date", "merchant", "amount", "currency", "category", "note", "source", "subject", "message_id", "added"];

impl Expense {
    pub(super) fn fields(&self) -> [&str; 11] {
        [
            &self.id, &self.date, &self.merchant, &self.amount, &self.currency, &self.category, &self.note, &self.source,
            &self.subject, &self.message_id, &self.added,
        ]
    }

    pub(super) fn from_fields(f: &[String]) -> Option<Self> {
        let get = |i: usize| f.get(i).cloned().unwrap_or_default();
        (f.len() >= 4).then(|| Expense {
            id: get(0),
            date: get(1),
            merchant: get(2),
            amount: get(3),
            currency: get(4),
            category: get(5),
            note: get(6),
            source: get(7),
            subject: get(8),
            message_id: get(9),
            added: get(10),
        })
    }

    /// The amount in hundredths, for adding up without floating point.
    pub(super) fn cents(&self) -> i64 {
        parse_cents(&self.amount).unwrap_or(0)
    }
}

pub fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join("expenses")
}

pub fn ledger_path(data_dir: &Path) -> PathBuf {
    folder(data_dir).join("ledger.csv")
}
