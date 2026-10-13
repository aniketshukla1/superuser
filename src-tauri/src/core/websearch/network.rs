//! network (integration-tested with --ignored)

use super::*;

pub async fn search(query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    let client = crate::core::egress::client("web");
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header("User-Agent", UA)
        .form(&[("q", query)])
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| format!("Web search failed: {e}"))?;
    let html = resp.text().await.map_err(|e| e.to_string())?;
    Ok(parse_results(&html, limit))
}

/// Parse a SearXNG JSON response (`/search?format=json`) into results.
pub fn parse_searxng(json: &serde_json::Value, limit: usize) -> Vec<SearchResult> {
    json["results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    let url = r["url"].as_str()?.to_string();
                    if !url.starts_with("http") { return None; }
                    Some(SearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url,
                        snippet: r["content"].as_str().unwrap_or("").to_string(),
                    })
                })
                .take(limit)
                .collect()
        })
        .unwrap_or_default()
}

/// Search via a self-hosted SearXNG instance (keyless, private metasearch).
pub async fn search_searxng(base_url: &str, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let client = crate::core::egress::client("web");
    let resp = client
        .get(&url)
        .query(&[("q", query), ("format", "json")])
        .header("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| format!("SearXNG search failed (is it running at {base_url}?): {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("SearXNG returned {} — make sure JSON output is enabled in its settings.", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("SearXNG response wasn't JSON: {e}"))?;
    Ok(parse_searxng(&json, limit))
}

pub async fn fetch_text(url: &str, max_chars: usize) -> Result<String, String> {
    let client = crate::core::egress::client("web");
    let resp = client
        .get(url)
        .header("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| format!("Couldn't fetch {url}: {e}"))?;
    let html = resp.text().await.map_err(|e| e.to_string())?;
    Ok(html_to_text(&html).chars().take(max_chars).collect())
}
