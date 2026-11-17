use oteryn_admin_contracts::WireError;
use serde::Deserialize;
use std::path::PathBuf;
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub agent_id: String,
    pub platform_endpoint: String,
    pub mtls_certificate_path: PathBuf,
    pub mtls_private_key_path: PathBuf,
    pub mtls_ca_path: PathBuf,
    pub signing_key_path: PathBuf,
    pub policy_file_path: PathBuf,
    pub db_url_env_name: String,
    pub owner_epoch: u64,
    pub worlds: Vec<String>,
    pub nodes: Vec<String>,
    pub build_revision: String,
}
impl Config {
    pub fn parse(text: &str) -> Result<Self, WireError> {
        let config: Self = toml::from_str(text)?;
        let endpoint = reqwest::Url::parse(&config.platform_endpoint)?;
        if endpoint.scheme() != "https"
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
        {
            return Err("Platform endpoint must be an HTTPS origin".into());
        }
        if config.agent_id.is_empty()
            || config.db_url_env_name.is_empty()
            || config.owner_epoch > 9_007_199_254_740_991
        {
            return Err("invalid agent configuration".into());
        }
        Ok(config)
    }
}
