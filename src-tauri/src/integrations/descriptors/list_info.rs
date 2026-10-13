use super::*;

/// Catalog entry + whether a credential is stored for it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectorInfo {
    #[serde(flatten)]
    pub connector: Connector,
    pub connected: bool,
}

pub fn list_info() -> Vec<ConnectorInfo> {
    CONNECTORS
        .iter()
        .map(|c| ConnectorInfo {
            connector: c.clone(),
            connected: crate::core::secrets::has_integration_token(crate::integrations::token_key(c.id)),
        })
        .collect()
}
