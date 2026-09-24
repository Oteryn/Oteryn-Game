//! Durable operator request and authorization files (OPS-NODE-BOOT-01 D2/D3).
//!
//! Each file holds the complete request that guards one database or fence
//! mutation, so an ambiguous outcome is always reconciled from exactly the
//! retained request. Files are never regenerated or edited.

use serde::{Deserialize, Serialize};

/// Upper bound on any operator file.
pub const MAX_OPERATOR_FILE_BYTES: usize = 16 * 1024;

/// A malformed or over-bound operator file. Content is never echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidOperatorFile;

fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, InvalidOperatorFile> {
    if bytes.len() > MAX_OPERATOR_FILE_BYTES {
        return Err(InvalidOperatorFile);
    }
    serde_json::from_slice(bytes).map_err(|_| InvalidOperatorFile)
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, InvalidOperatorFile> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| InvalidOperatorFile)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_OPERATOR_FILE_BYTES {
        return Err(InvalidOperatorFile);
    }
    Ok(bytes)
}

/// Lowercase hexadecimal of exactly `N` bytes.
pub fn hex_bytes<const N: usize>(text: &str) -> Result<[u8; N], InvalidOperatorFile> {
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(InvalidOperatorFile);
    }
    let mut bytes = [0_u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|_| InvalidOperatorFile)?;
    }
    Ok(bytes)
}

/// Lowercase hexadecimal encoding.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

macro_rules! operator_file {
    ($name:ident) => {
        impl $name {
            pub fn decode(bytes: &[u8]) -> Result<Self, InvalidOperatorFile> {
                decode(bytes)
            }

            pub fn encode(&self) -> Result<Vec<u8>, InvalidOperatorFile> {
                encode(self)
            }
        }
    };
}

/// The complete launch-authorization issuance request (D2). The node reads
/// the secret and binding; the operator reconciles the issuance from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchAuthorizationFile {
    pub version: u8,
    /// 32-byte launch secret, hexadecimal.
    pub secret: String,
    pub launch_binding: String,
    /// The superseded `NodeId` (UUID text), if any.
    pub supersedes: Option<String>,
}
operator_file!(LaunchAuthorizationFile);

/// The complete S2 fresh-store authorization (D2), fixed at issuance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct S2AuthorizationFile {
    pub version: u8,
    pub namespace: String,
    pub authorization: String,
    pub source_authority: String,
    pub initialized_at: i64,
    pub descriptor_revision: u64,
    pub installed_at: i64,
    /// Canonical descriptor facts, hexadecimal.
    pub descriptor_facts: String,
}
operator_file!(S2AuthorizationFile);

/// One Channel-assignment command with its operation key (D2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignmentRequestFile {
    pub version: u8,
    /// 32-byte operation key, hexadecimal.
    pub operation_key: String,
    pub actor: String,
    /// `assign`, `replace` or `revoke`.
    pub command: String,
    pub world_id: String,
    pub channel_id: String,
    pub predecessor_generation: Option<u64>,
    pub predecessor_source_revision: Option<u64>,
    pub predecessor_guard_publication_revision: Option<u64>,
    pub target_node_id: Option<String>,
    pub target_registration_revision: Option<u64>,
}
operator_file!(AssignmentRequestFile);

/// The fresh Character recovery request (D2): its inputs reproduce the same
/// generation-one transition on a re-run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterRecoveryRequestFile {
    pub version: u8,
    /// 16-byte UUIDv7 recovery event id, hexadecimal.
    pub recovery_event_id: String,
    pub issued_at: u64,
}
operator_file!(CharacterRecoveryRequestFile);

/// Fresh random bytes from the operating system.
pub fn random_bytes<const N: usize>() -> std::io::Result<[u8; N]> {
    use std::io::Read;
    let mut bytes = [0_u8; N];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// A fresh UUIDv7 from the current time and operating-system randomness.
pub fn uuid_v7() -> std::io::Result<[u8; 16]> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_millis();
    let mut bytes = random_bytes::<16>()?;
    bytes[..6].copy_from_slice(&(millis as u64).to_be_bytes()[2..]);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn files_round_trip_and_reject_unknown_fields() {
        let file = LaunchAuthorizationFile {
            version: 1,
            secret: hex(&[7; 32]),
            launch_binding: "node-a".into(),
            supersedes: None,
        };
        let bytes = file.encode().expect("encode");
        assert_eq!(LaunchAuthorizationFile::decode(&bytes), Ok(file));
        assert!(
            LaunchAuthorizationFile::decode(
                br#"{"version":1,"secret":"","launch_binding":"a","supersedes":null,"extra":1}"#
            )
            .is_err()
        );
        assert_eq!(hex_bytes::<2>("0aff"), Ok([0x0a, 0xff]));
        assert!(hex_bytes::<2>("0AFF").is_err());
        assert!(hex_bytes::<2>("0af").is_err());
    }

    #[test]
    fn generated_uuids_are_v7() {
        let id = uuid_v7().expect("uuid");
        assert_eq!(id[6] >> 4, 7);
        assert_eq!(id[8] & 0xc0, 0x80);
        assert_ne!(uuid_v7().expect("uuid"), id);
    }
}
