//! Product Performance — a slowdown traced to the series that moved, with links.
//!
//! The job: investigate a slowdown since a release, find the highest-confidence hotspot, and
//! return a short write-up with charts and direct links, facts kept apart from hypotheses. It
//! never changes alerts or production settings.
//!
//! Here the metrics come from Prometheus, or anything that speaks its API, live and read-only
//! (`integrations::prometheus`). **The code does the arithmetic; the model does the reading.**
//! `metrics_compare` sets the windows either side of a change, compares every series, and says of
//! each whether it moved clearly, possibly, or within its usual wobble — the evidence a
//! "highest-confidence hotspot" has to stand on. Every answer links to the same query in
//! Prometheus, and the series it read are kept as a CSV in the conversation's folder, so a chart
//! of them is drawn from the data rather than retyped. Nothing it has can change an alert or a
//! setting: the only thing it writes is that CSV.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use serde_json::Value;

use crate::integrations::prometheus::{self as prom, Answer, Conn, Series};

#[cfg(test)]
mod tests;

mod shown;
mod durations_moments;
mod arithmetic;
mod charts_data;
mod tools;
mod signals_health_report;

pub use shown::*;
pub use durations_moments::*;
pub use arithmetic::*;
pub use charts_data::*;
pub use tools::*;
pub use signals_health_report::*;
