//! Closed TOML configuration for `oteryn-game-server serve` and
//! `oteryn-game-ops` (OPS-NODE-BOOT-01 D1/D2).
//!
//! Unknown or duplicate keys reject, every value is required, no production
//! default exists, and secrets are given only as file references. Errors name
//! the failing key, never its value.

use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

/// Registered maxima (`NTRANS-CONNECTIONS`, `NTRANS-HANDSHAKE`).
pub const MAX_CONNECTIONS: u32 = 256;
pub const MAX_HANDSHAKE_UNITS: u32 = 64;
/// Upper bound on the configuration document itself.
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;
/// Bounded operator-assignment wait (D3 step 6).
pub const MAX_ASSIGNMENT_WAIT_MS: u64 = 24 * 60 * 60 * 1000;
pub const MAX_ENTRY_DEADLINE_MS: u64 = 60_000;

/// A rejected configuration. `key` names the offending key; the value is never
/// echoed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub key: &'static str,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid configuration key `{}`", self.key)
    }
}

impl std::error::Error for ConfigError {}

const fn reject<T>(key: &'static str) -> Result<T, ConfigError> {
    Err(ConfigError { key })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    pub transport_ip: IpAddr,
    pub port: u16,
    pub tls_server_name: String,
    pub database: String,
    pub username: String,
    pub password_file: PathBuf,
    pub root_ca_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterFenceConfig {
    pub fence_directory: PathBuf,
    pub authority_scope_id: String,
    pub issuer_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListenerConfig {
    pub address: SocketAddr,
    pub entry_deadline_ms: u64,
    pub max_connections: u32,
    pub max_handshake_units: u32,
    pub certificate_chain_file: PathBuf,
    pub private_key_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeConfig {
    pub world_id: String,
    pub channel_id: String,
    pub assignment_wait_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlConfig {
    pub socket_path: PathBuf,
}

/// D5 declared revisions plus the Runtime readiness source authority.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessConfig {
    pub source_authority: String,
    pub route_revision: String,
    pub runtime_observation_revision: String,
    pub ruleset_revision: String,
    pub content_revision: String,
    pub map_revision: String,
    pub world_policy_revision: String,
    pub offer_revision: String,
}

/// The one Platform producer (D1). The operation paths are compiled into the
/// binary and not configurable.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformConfig {
    pub source_authority: String,
    pub endpoint: SocketAddr,
    pub peer_name: String,
    pub trust_roots_file: PathBuf,
    pub client_certificate_file: PathBuf,
    pub client_key_file: PathBuf,
    pub descriptor_revision: u64,
    pub installed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchConfig {
    pub authorization_file: PathBuf,
    /// Required only while the S2 store is uninitialized.
    pub s2_authorization_file: Option<PathBuf>,
}

/// `oteryn-game-server serve --config <path>`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeConfig {
    pub listener: ListenerConfig,
    pub scope: ScopeConfig,
    pub database: DatabaseConfig,
    pub character: CharacterFenceConfig,
    pub control: ControlConfig,
    pub readiness: ReadinessConfig,
    pub platform: PlatformConfig,
    pub launch: LaunchConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorConfig {
    /// Durable request and authorization files are created only here.
    pub state_directory: PathBuf,
    /// The node's service user; node-read files are handed to it.
    pub service_uid: u32,
}

/// `oteryn-game-ops --config <path>`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpsConfig {
    pub operator: OperatorConfig,
    pub database: DatabaseConfig,
    pub character: CharacterFenceConfig,
}

fn token(value: &str, maximum: usize, extra: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b"._:-".contains(&byte) || extra.contains(&byte)
        })
}

fn absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, std::path::Component::ParentDir))
}

fn v7_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(byte),
        })
        && bytes[14] == b'7'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

fn check_database(database: &DatabaseConfig) -> Result<(), ConfigError> {
    if database.port == 0 {
        return reject("database.port");
    }
    if !token(&database.tls_server_name, 253, b"") {
        return reject("database.tls_server_name");
    }
    if !token(&database.database, 63, b"") {
        return reject("database.database");
    }
    if !token(&database.username, 63, b"") {
        return reject("database.username");
    }
    if !absolute(&database.password_file) {
        return reject("database.password_file");
    }
    if !absolute(&database.root_ca_file) {
        return reject("database.root_ca_file");
    }
    Ok(())
}

fn check_character(character: &CharacterFenceConfig) -> Result<(), ConfigError> {
    if !absolute(&character.fence_directory) {
        return reject("character.fence_directory");
    }
    if !token(&character.authority_scope_id, 128, b"") {
        return reject("character.authority_scope_id");
    }
    if !token(&character.issuer_identity, 128, b"") {
        return reject("character.issuer_identity");
    }
    Ok(())
}

fn parse<T: serde::de::DeserializeOwned>(document: &[u8]) -> Result<T, ConfigError> {
    if document.len() > MAX_CONFIG_BYTES {
        return reject("<document>");
    }
    let text = std::str::from_utf8(document).map_err(|_| ConfigError { key: "<document>" })?;
    // Duplicate keys, unknown keys and missing values are all parse errors.
    toml::from_str(text).map_err(|_| ConfigError { key: "<document>" })
}

impl NodeConfig {
    /// Parse and validate one node configuration document.
    pub fn parse(document: &[u8]) -> Result<Self, ConfigError> {
        let config: Self = parse(document)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let listener = &self.listener;
        if !(1..=MAX_ENTRY_DEADLINE_MS).contains(&listener.entry_deadline_ms) {
            return reject("listener.entry_deadline_ms");
        }
        if !(1..=MAX_CONNECTIONS).contains(&listener.max_connections) {
            return reject("listener.max_connections");
        }
        if !(1..=MAX_HANDSHAKE_UNITS).contains(&listener.max_handshake_units) {
            return reject("listener.max_handshake_units");
        }
        if !absolute(&listener.certificate_chain_file) {
            return reject("listener.certificate_chain_file");
        }
        if !absolute(&listener.private_key_file) {
            return reject("listener.private_key_file");
        }
        if !v7_uuid(&self.scope.world_id) {
            return reject("scope.world_id");
        }
        if !v7_uuid(&self.scope.channel_id) {
            return reject("scope.channel_id");
        }
        if !(1..=MAX_ASSIGNMENT_WAIT_MS).contains(&self.scope.assignment_wait_ms) {
            return reject("scope.assignment_wait_ms");
        }
        check_database(&self.database)?;
        check_character(&self.character)?;
        if !absolute(&self.control.socket_path) {
            return reject("control.socket_path");
        }
        let readiness = &self.readiness;
        if !token(&readiness.source_authority, 128, b"/") {
            return reject("readiness.source_authority");
        }
        for (key, value) in [
            ("readiness.route_revision", &readiness.route_revision),
            (
                "readiness.runtime_observation_revision",
                &readiness.runtime_observation_revision,
            ),
            ("readiness.ruleset_revision", &readiness.ruleset_revision),
            ("readiness.content_revision", &readiness.content_revision),
            ("readiness.map_revision", &readiness.map_revision),
            (
                "readiness.world_policy_revision",
                &readiness.world_policy_revision,
            ),
            ("readiness.offer_revision", &readiness.offer_revision),
        ] {
            if !token(value, 128, b"") {
                return reject(key);
            }
        }
        let platform = &self.platform;
        if !token(&platform.source_authority, 128, b"/") {
            return reject("platform.source_authority");
        }
        // The Runtime readiness namespace is never a Platform source authority.
        if platform.source_authority == readiness.source_authority
            || platform.source_authority == crate::character_bootstrap_intent::ISSUER_AUTHORITY
            || readiness.source_authority == crate::character_bootstrap_intent::ISSUER_AUTHORITY
        {
            return reject("readiness.source_authority");
        }
        if platform.endpoint.port() == 0 {
            return reject("platform.endpoint");
        }
        if !token(&platform.peer_name, 128, b"") {
            return reject("platform.peer_name");
        }
        for (key, path) in [
            ("platform.trust_roots_file", &platform.trust_roots_file),
            (
                "platform.client_certificate_file",
                &platform.client_certificate_file,
            ),
            ("platform.client_key_file", &platform.client_key_file),
        ] {
            if !absolute(path) {
                return reject(key);
            }
        }
        if platform.descriptor_revision == 0 {
            return reject("platform.descriptor_revision");
        }
        if platform.installed_at < 0 {
            return reject("platform.installed_at");
        }
        if !absolute(&self.launch.authorization_file) {
            return reject("launch.authorization_file");
        }
        if self
            .launch
            .s2_authorization_file
            .as_deref()
            .is_some_and(|path| !absolute(path))
        {
            return reject("launch.s2_authorization_file");
        }
        Ok(())
    }
}

impl OpsConfig {
    /// Parse and validate one operator configuration document.
    pub fn parse(document: &[u8]) -> Result<Self, ConfigError> {
        let config: Self = parse(document)?;
        if !absolute(&config.operator.state_directory) {
            return reject("operator.state_directory");
        }
        // The operator identity is root and never the service user (D2).
        if config.operator.service_uid == 0 {
            return reject("operator.service_uid");
        }
        check_database(&config.database)?;
        check_character(&config.character)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    pub(crate) const NODE: &str = r#"
[listener]
address = "127.0.0.1:7171"
entry_deadline_ms = 20000
max_connections = 256
max_handshake_units = 64
certificate_chain_file = "/etc/oteryn/node/gameplay.crt"
private_key_file = "/etc/oteryn/node/gameplay.key"

[scope]
world_id = "01890f4c-3b2a-7c01-8d11-9a321b7c0002"
channel_id = "01890f4c-3b2a-7c01-8d11-9a321b7c0003"
assignment_wait_ms = 600000

[database]
transport_ip = "127.0.0.1"
port = 5432
tls_server_name = "db.internal"
database = "oteryn"
username = "node_login"
password_file = "/etc/oteryn/node/pg-password"
root_ca_file = "/etc/oteryn/node/db-ca.pem"

[character]
fence_directory = "/var/lib/oteryn/character-fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"

[control]
socket_path = "/run/oteryn-node/control.sock"

[readiness]
source_authority = "oteryn:runtime:world-1:channel-1"
route_revision = "route-1"
runtime_observation_revision = "observation-1"
ruleset_revision = "ruleset-1"
content_revision = "content-1"
map_revision = "map-1"
world_policy_revision = "policy-1"
offer_revision = "offer-1"

[platform]
source_authority = "urn:oteryn:platform:game-auth"
endpoint = "10.0.0.5:8443"
peer_name = "game-auth.internal"
trust_roots_file = "/etc/oteryn/node/platform-roots.pem"
client_certificate_file = "/etc/oteryn/node/platform-client.crt"
client_key_file = "/etc/oteryn/node/platform-client.key"
descriptor_revision = 1
installed_at = 1790000000

[launch]
authorization_file = "/var/lib/oteryn-ops/launch-node-a.json"
s2_authorization_file = "/var/lib/oteryn-ops/s2-fresh-store.json"
"#;

    #[test]
    fn complete_node_configuration_parses() {
        let config = NodeConfig::parse(NODE.as_bytes()).expect("valid configuration");
        assert_eq!(config.listener.max_connections, 256);
        assert_eq!(config.platform.descriptor_revision, 1);
    }

    #[test]
    fn unknown_duplicate_and_missing_keys_reject() {
        let unknown = NODE.replace("[control]\n", "[control]\ndebug = true\n");
        assert!(NodeConfig::parse(unknown.as_bytes()).is_err());
        let duplicate = NODE.replace("port = 5432\n", "port = 5432\nport = 5433\n");
        assert!(NodeConfig::parse(duplicate.as_bytes()).is_err());
        let missing = NODE.replace("map_revision = \"map-1\"\n", "");
        assert!(NodeConfig::parse(missing.as_bytes()).is_err());
        let url = NODE.replace(
            "transport_ip = \"127.0.0.1\"\n",
            "url = \"postgresql://u:p@host/db\"\n",
        );
        assert!(NodeConfig::parse(url.as_bytes()).is_err());
    }

    #[test]
    fn over_maximum_limits_and_bad_values_name_the_key_only() {
        // The readiness case below uses the compiled Character issuer.
        assert_eq!(
            crate::character_bootstrap_intent::ISSUER_AUTHORITY,
            "OTERYN_PLATFORM_CHARACTER_AUTHORITY"
        );
        for (from, to, key) in [
            (
                "max_connections = 256",
                "max_connections = 257",
                "listener.max_connections",
            ),
            (
                "max_handshake_units = 64",
                "max_handshake_units = 65",
                "listener.max_handshake_units",
            ),
            (
                "password_file = \"/etc/oteryn/node/pg-password\"",
                "password_file = \"pg-password\"",
                "database.password_file",
            ),
            (
                "world_id = \"01890f4c-3b2a-7c01-8d11-9a321b7c0002\"",
                "world_id = \"01890f4c-3b2a-4c01-8d11-9a321b7c0002\"",
                "scope.world_id",
            ),
            (
                "source_authority = \"oteryn:runtime:world-1:channel-1\"",
                "source_authority = \"urn:oteryn:platform:game-auth\"",
                "readiness.source_authority",
            ),
            (
                "source_authority = \"oteryn:runtime:world-1:channel-1\"",
                "source_authority = \"OTERYN_PLATFORM_CHARACTER_AUTHORITY\"",
                "readiness.source_authority",
            ),
            (
                "route_revision = \"route-1\"",
                "route_revision = \"route 1\"",
                "readiness.route_revision",
            ),
        ] {
            let changed = NODE.replace(from, to);
            assert_ne!(changed, NODE, "{from}");
            let error = NodeConfig::parse(changed.as_bytes()).expect_err(key);
            assert_eq!(error.key, key);
            assert!(
                !error
                    .to_string()
                    .contains(to.split('=').nth(1).unwrap_or(to).trim())
            );
        }
    }

    #[test]
    fn operator_configuration_requires_a_non_root_service_user() {
        let ops = r#"
[operator]
state_directory = "/var/lib/oteryn-ops"
service_uid = 990

[database]
transport_ip = "127.0.0.1"
port = 5432
tls_server_name = "db.internal"
database = "oteryn"
username = "control_login"
password_file = "/etc/oteryn/ops/pg-password"
root_ca_file = "/etc/oteryn/ops/db-ca.pem"

[character]
fence_directory = "/var/lib/oteryn/character-fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
"#;
        assert!(OpsConfig::parse(ops.as_bytes()).is_ok());
        let root = ops.replace("service_uid = 990", "service_uid = 0");
        assert_eq!(
            OpsConfig::parse(root.as_bytes()).err(),
            Some(ConfigError {
                key: "operator.service_uid"
            })
        );
    }
}
