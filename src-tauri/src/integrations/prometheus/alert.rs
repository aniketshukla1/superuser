use super::*;

pub(super) const TIMEOUT: Duration = Duration::from_secs(30);

/// Series kept from one answer. A `by (pod)` over a big fleet is thousands; what anyone can use
/// is the top of the list, and every tool says how many there were.
pub const MAX_SERIES: usize = 500;

/// Where the connection is kept (`secrets::set_integration_token`).
pub const ID: &str = "prometheus";

/// A connection: the address and one of four ways in — none, basic auth (Grafana Cloud: the
/// instance id and an API token), a bearer token, and a tenant for Mimir and Cortex.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Conn {
    /// Where the API lives, without a trailing slash: `http://prometheus:9090`, or
    /// `https://prometheus-prod-01-eu-west-0.grafana.net/api/prom`.
    pub url: String,
    pub username: String,
    pub password: String,
    pub token: String,
    /// Sent as `X-Scope-OrgID`.
    pub org_id: String,
}

/// One series: its labels and its points, as (unix seconds, value).
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub labels: BTreeMap<String, String>,
    pub points: Vec<(f64, f64)>,
}

/// What a query returned.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// Series, and how many there were before `MAX_SERIES`.
    Series(Vec<Series>, usize),
    Scalar(f64, f64),
    Text(f64, String),
}

/// An alert firing or pending.
#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    pub name: String,
    pub state: String,
    pub severity: String,
    /// RFC 3339, as Prometheus gave it.
    pub active_at: String,
    pub summary: String,
    /// The rest of its labels: which instance, job, service.
    pub labels: BTreeMap<String, String>,
    pub value: String,
}

/// A metric's type and help, from `/api/v1/metadata`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub kind: String,
    pub help: String,
    pub unit: String,
}
