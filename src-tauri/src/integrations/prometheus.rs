//! Prometheus — live, read-only metric queries, for the Product Performance assistant.
//!
//! Anything that speaks Prometheus's HTTP API works: Prometheus itself, Thanos, Cortex, Mimir,
//! VictoriaMetrics, and Grafana Cloud's Prometheus endpoint. Nothing is copied into memory: an
//! assistant asks when it needs to, and the answer is as fresh as the server's.
//!
//! **Read-only by construction.** These GET endpoints are the only ones ever called: query,
//! query_range, series, label values, metadata, alerts and buildinfo. The admin API (deleting
//! series, snapshots) and anything else that writes is never reached, so "do not change alerts or
//! production settings" isn't a promise a model has to keep.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{json, Value};

#[cfg(test)]
mod tests;

mod alert;
mod conn;
mod short_names;

pub use alert::*;
pub use short_names::*;
