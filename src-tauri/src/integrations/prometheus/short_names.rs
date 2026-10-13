use super::*;

/// The API's envelope, as an answer or a sentence about what went wrong.
pub(super) fn envelope(status: u16, text: &str, host: &str) -> Result<Value, String> {
    if matches!(status, 401 | 403) {
        return Err(format!("{host} refused the login ({status}) — check the username and password, or the token"));
    }
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return Err(if status == 404 {
            format!("{host} has no Prometheus API at that address (404) — is it the address you open Prometheus at?")
        } else {
            format!("{host} answered {status} with something that isn't Prometheus's API")
        });
    };
    if v["status"] == "success" {
        return Ok(v);
    }
    let err = v["error"].as_str().unwrap_or("no reason given");
    Err(match v["errorType"].as_str().unwrap_or("") {
        "bad_data" => format!("the query didn't parse: {err}"),
        "timeout" => format!("the query took too long: {err}. Narrow it — a shorter range, a larger step, or fewer series"),
        "canceled" => format!("Prometheus cancelled the query: {err}"),
        "execution" => format!("the query failed: {err}"),
        _ => format!("Prometheus said ({status}): {err}"),
    })
}

pub(super) fn warnings(v: &Value) -> Vec<String> {
    v["warnings"].as_array().map(|a| a.iter().filter_map(|w| w.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// A sample value: Prometheus sends numbers as strings, `NaN` and `±Inf` included.
pub(super) fn num(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) => match s.as_str() {
            "+Inf" | "Inf" => Some(f64::INFINITY),
            "-Inf" => Some(f64::NEG_INFINITY),
            other => other.parse().ok(),
        },
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

pub(super) fn labels_of(metric: &Value) -> BTreeMap<String, String> {
    metric
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default()
}

/// `data` of a query or query_range answer.
pub fn parse_result(data: &Value) -> Result<Answer, String> {
    let result = &data["result"];
    let point = |p: &Value| Some((p[0].as_f64()?, num(&p[1])?));
    match data["resultType"].as_str().unwrap_or("") {
        kind @ ("vector" | "matrix") => {
            let all = result.as_array().map(Vec::as_slice).unwrap_or_default();
            let series = all
                .iter()
                .take(MAX_SERIES)
                .map(|s| Series {
                    labels: labels_of(&s["metric"]),
                    // Native-histogram samples carry `histogram` in place of a value; they
                    // have no single number, so they're left out.
                    points: if kind == "vector" {
                        point(&s["value"]).into_iter().collect()
                    } else {
                        s["values"].as_array().map(|ps| ps.iter().filter_map(point).collect()).unwrap_or_default()
                    },
                })
                .collect();
            Ok(Answer::Series(series, all.len()))
        }
        "scalar" => point(result).map(|(t, v)| Answer::Scalar(t, v)).ok_or_else(|| "Prometheus's scalar answer was unreadable".into()),
        "string" => Ok(Answer::Text(result[0].as_f64().unwrap_or(0.0), result[1].as_str().unwrap_or("").to_string())),
        other => Err(format!("Prometheus answered with a result type this doesn't read: “{other}”")),
    }
}

/// `data` of `/api/v1/alerts`, firing first, then the longest-running.
pub fn parse_alerts(data: &Value) -> Vec<Alert> {
    let mut alerts: Vec<Alert> = data["alerts"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|a| {
            let mut labels = labels_of(&a["labels"]);
            let name = labels.remove("alertname").unwrap_or_default();
            let severity = labels.remove("severity").unwrap_or_default();
            let ann = &a["annotations"];
            Alert {
                name,
                state: a["state"].as_str().unwrap_or("").to_string(),
                severity,
                active_at: a["activeAt"].as_str().unwrap_or("").to_string(),
                summary: ann["summary"].as_str().or(ann["description"].as_str()).or(ann["message"].as_str()).unwrap_or("").to_string(),
                labels,
                value: a["value"].as_str().unwrap_or("").to_string(),
            }
        })
        .collect();
    alerts.sort_by(|a, b| (a.state != "firing").cmp(&(b.state != "firing")).then(a.active_at.cmp(&b.active_at)));
    alerts
}

/// Labels as PromQL writes them: `name{key="value", …}`.
pub fn fmt_labels(labels: &BTreeMap<String, String>) -> String {
    let name = labels.get("__name__").cloned().unwrap_or_default();
    let rest: Vec<String> = labels.iter().filter(|(k, _)| *k != "__name__").map(|(k, v)| format!("{k}=\"{v}\"")).collect();
    match (name.is_empty(), rest.is_empty()) {
        (_, true) if !name.is_empty() => name,
        (true, true) => "{}".into(),
        _ => format!("{name}{{{}}}", rest.join(", ")),
    }
}

/// A short name for each series: the values of the labels that tell them apart (`/checkout`,
/// `/cart`), or the whole label set when nothing does. Unique, in the order given.
pub fn short_names(series: &[&BTreeMap<String, String>]) -> Vec<String> {
    let keys: Vec<&String> = {
        let mut all: Vec<&String> = series.iter().flat_map(|l| l.keys()).collect();
        all.sort();
        all.dedup();
        all.into_iter()
            .filter(|k| {
                let first = series.first().and_then(|l| l.get(*k));
                series.iter().any(|l| l.get(*k) != first)
            })
            .collect()
    };
    let mut seen = std::collections::HashSet::new();
    series
        .iter()
        .map(|l| {
            let short = keys.iter().filter_map(|k| l.get(*k)).filter(|v| !v.is_empty()).cloned().collect::<Vec<_>>().join(" ");
            let mut name = if short.is_empty() { fmt_labels(l) } else { short };
            if name == "{}" {
                name = "value".into();
            }
            let mut unique = name.clone();
            let mut n = 2;
            while !seen.insert(unique.clone()) {
                unique = format!("{name} ({n})");
                n += 1;
            }
            unique
        })
        .collect()
}
