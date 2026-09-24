//! Startable GameNode composition (OPS-NODE-BOOT-01): the closed
//! configuration, descriptor-based file checks and the operator file formats
//! shared by `oteryn-game-server serve` and `oteryn-game-ops`.

pub mod config;
pub mod descriptor_facts;
pub mod operator_files;
pub mod secure_file;

use crate::durability::{DurabilityError, DurabilityRoot, DurabilityRootConfig};
use config::DatabaseConfig;
use secure_file::{FileClass, FileError};

/// Upper bounds on the database credential files.
pub const MAX_PASSWORD_BYTES: usize = 1024;
pub const MAX_ROOT_CA_BYTES: usize = 64 * 1024;

/// A startup input failure. `key` names the configuration key whose value or
/// file failed; neither value nor content is ever carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupError {
    File { key: &'static str, error: FileError },
    Invalid { key: &'static str },
    Database(&'static str),
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File { key, error } => write!(formatter, "file for `{key}` rejected: {error:?}"),
            Self::Invalid { key } => write!(formatter, "invalid value for `{key}`"),
            Self::Database(stage) => write!(formatter, "durability root unavailable at {stage}"),
        }
    }
}

impl std::error::Error for StartupError {}

/// Read one checked file for configuration key `key`.
pub fn read_file(
    key: &'static str,
    path: &std::path::Path,
    class: FileClass,
    owner: u32,
    max: usize,
) -> Result<Vec<u8>, StartupError> {
    secure_file::read_checked(path, class, owner, max)
        .map_err(|error| StartupError::File { key, error })
}

/// Connect the explicit, TLS-verified durability root from the D1 fields and
/// require it to report ready. The password and CA files are secret files of
/// `owner`.
pub async fn connect_root(
    database: &DatabaseConfig,
    owner: u32,
) -> Result<DurabilityRoot, StartupError> {
    let password = read_file(
        "database.password_file",
        &database.password_file,
        FileClass::Secret,
        owner,
        MAX_PASSWORD_BYTES,
    )?;
    let password = std::str::from_utf8(&password)
        .map_err(|_| StartupError::Invalid {
            key: "database.password_file",
        })?
        .trim_end_matches(['\n', '\r']);
    let root_ca = read_file(
        "database.root_ca_file",
        &database.root_ca_file,
        FileClass::Secret,
        owner,
        MAX_ROOT_CA_BYTES,
    )?;
    let config = DurabilityRootConfig::new(
        database.transport_ip,
        database.port,
        &database.tls_server_name,
        &database.database,
        &database.username,
        password,
        &root_ca,
    )
    .map_err(|_: DurabilityError| StartupError::Invalid { key: "database" })?;
    let root = DurabilityRoot::new(config).map_err(|_| StartupError::Database("root"))?;
    match root.maintain_ready_once().await {
        Ok(true) => Ok(root),
        _ => Err(StartupError::Database("ready")),
    }
}

/// Parse canonical lowercase UUID text into its 16 bytes.
pub fn uuid_bytes(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 || [8, 13, 18, 23].iter().any(|index| bytes[*index] != b'-') {
        return None;
    }
    let digits: String = text.chars().filter(|character| *character != '-').collect();
    operator_files::hex_bytes::<16>(&digits).ok()
}

/// Canonical lowercase UUID text of 16 bytes.
#[must_use]
pub fn uuid_text(bytes: &[u8; 16]) -> String {
    let hex = operator_files::hex(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
