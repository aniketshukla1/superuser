
/// Why a request didn't produce a response.
#[derive(Debug)]
pub enum EgressError {
    /// The network's own error.
    Http(reqwest::Error),
    /// Refused here. Nothing was sent.
    LocalOnly(String),
}

impl std::fmt::Display for EgressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EgressError::Http(e) => write!(f, "{e}"),
            EgressError::LocalOnly(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for EgressError {}
