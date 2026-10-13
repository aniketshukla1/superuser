//! Stripe integration via a secret API key. Pulls recent charges and customers
//! into the Memory Tree (read-only).

use super::SyncOutcome;
use crate::core::memory::{MemoryEngine, MemoryInput, Origin};
use serde_json::Value;

async fn get(client: &crate::core::egress::Client, key: &str, url: &str) -> Result<Value, String> {
    let resp = client
        .get(url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| format!("Network error reaching Stripe: {e}"))?;
    match resp.status().as_u16() {
        200 => resp.json().await.map_err(|e| e.to_string()),
        401 => Err("Stripe rejected the key. Use a secret key (sk_…) from Developers → API keys.".into()),
        s => Err(format!("Stripe returned status {s}.")),
    }
}

pub async fn verify(key: &str) -> Result<String, String> {
    let client = crate::core::egress::client("stripe");
    let acct = get(&client, key, "https://api.stripe.com/v1/account").await?;
    let name = acct["business_profile"]["name"].as_str().or(acct["email"].as_str()).or(acct["id"].as_str()).unwrap_or("Stripe account");
    Ok(name.to_string())
}

fn money(amount: i64, currency: &str) -> String {
    format!("{:.2} {}", amount as f64 / 100.0, currency.to_uppercase())
}

pub async fn sync(key: &str, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let client = crate::core::egress::client("stripe");
    let mut out = SyncOutcome::default();

    let charges = get(&client, key, "https://api.stripe.com/v1/charges?limit=25").await?;
    if let Some(items) = charges["data"].as_array() {
        for c in items {
            let id = c["id"].as_str().unwrap_or("");
            if id.is_empty() { continue; }
            let amount = money(c["amount"].as_i64().unwrap_or(0), c["currency"].as_str().unwrap_or("usd"));
            let status = c["status"].as_str().unwrap_or("");
            let desc = c["description"].as_str().unwrap_or("");
            let email = c["billing_details"]["email"].as_str().or(c["receipt_email"].as_str()).unwrap_or("");
            let filename = format!("stripe-charge-{id}.md");
            let existed = memory.vault_path.join(&filename).exists();
            let input = MemoryInput {
                filename: Some(filename),
                title: format!("Charge {amount} — {status}"),
                category: "Stripe".into(),
                tags: vec!["stripe".into(), "charge".into()],
                summary: format!("{amount} · {status}{}", if email.is_empty() { String::new() } else { format!(" · {email}") }),
                body: format!("**{amount}** — {status}\n\n- {}\n- Customer: {}", if desc.is_empty() { "(no description)" } else { desc }, if email.is_empty() { "—" } else { email }),
                origin: Origin::External,
                key: None,
                supersedes: None,
            };
            if memory.save(&input).is_ok() && !existed {
                out.created += 1;
                out.titles.push(format!("Stripe charge {amount} {status}"));
            }
        }
    }
    Ok(out)
}
