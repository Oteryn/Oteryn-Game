//! Platform reconciliation notice sent after a Character database restore.

use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::purpose_descriptor,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

pub const PATH: &str = "/internal/v1/game-auth/game-restore/notice";
pub const NOTICE_BYTES: usize = 512;
pub const RESPONSE_BYTES: usize = 256;
pub const PURPOSE: &str = "OTERYN_GAME_RESTORE_NOTICE";
pub const CONFIG_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreNotice {
    pub source_authority: String,
    pub restore_event_id: [u8; 16],
    pub restored_generation: u64,
    pub new_recovery_generation: u64,
    pub restore_point: u64,
    pub backup_label: String,
}

#[derive(Serialize)]
struct NoticeWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    restore_event_id: String,
    restored_generation: String,
    new_recovery_generation: String,
    restore_point: String,
    backup_label: &'a str,
}

fn source_authority_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

fn label_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn uuid_text(id: &[u8; 16]) -> Option<String> {
    if id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80 {
        return None;
    }
    Some(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        id[0],
        id[1],
        id[2],
        id[3],
        id[4],
        id[5],
        id[6],
        id[7],
        id[8],
        id[9],
        id[10],
        id[11],
        id[12],
        id[13],
        id[14],
        id[15]
    ))
}

pub fn encode(notice: &RestoreNotice) -> Result<String, SourceError> {
    let event = uuid_text(&notice.restore_event_id).ok_or(SourceError::InvalidInput)?;
    if notice.restored_generation == 0
        || notice.new_recovery_generation <= notice.restored_generation
        || notice.restore_point == 0
        || !label_valid(&notice.backup_label)
        || !source_authority_valid(&notice.source_authority)
    {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&NoticeWire {
        contract_version: 1,
        operation: "PublishGameRestoreNoticeV1",
        source_authority: &notice.source_authority,
        restore_event_id: event,
        restored_generation: notice.restored_generation.to_string(),
        new_recovery_generation: notice.new_recovery_generation.to_string(),
        restore_point: notice.restore_point.to_string(),
        backup_label: &notice.backup_label,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > NOTICE_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ack {
    Reconciling { reopened: u64, outstanding: u64 },
    Acknowledged { reopened: u64 },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AckWire {
    contract_version: u8,
    restore_event_id: String,
    status: String,
    reopened: u64,
    outstanding: u64,
}

pub fn decode_response(raw: &[u8], expected_event: &[u8; 16]) -> Result<Ack, SourceError> {
    if raw.len() > RESPONSE_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    let wire: AckWire = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    let canonical = serde_json::to_vec(&wire).map_err(|_| SourceError::InvalidInput)?;
    let expected = uuid_text(expected_event).ok_or(SourceError::InvalidInput)?;
    if canonical != raw || wire.contract_version != 1 || wire.restore_event_id != expected {
        return Err(SourceError::InvalidInput);
    }
    match (wire.status.as_str(), wire.outstanding) {
        ("RECONCILING", outstanding) if outstanding != 0 => Ok(Ack::Reconciling {
            reopened: wire.reopened,
            outstanding,
        }),
        ("ACKNOWLEDGED", 0) => Ok(Ack::Acknowledged {
            reopened: wire.reopened,
        }),
        _ => Err(SourceError::InvalidInput),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotDelivered {
    InvalidNotice,
    Malformed,
    Unauthenticated,
    Conflict,
    TooLarge,
    RateLimited,
    InvalidResponse,
    Unavailable,
    UnexpectedStatus,
}

impl NotDelivered {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::InvalidNotice => "invalid_notice",
            Self::Malformed => "malformed",
            Self::Unauthenticated => "unauthenticated",
            Self::Conflict => "conflict",
            Self::TooLarge => "too_large",
            Self::RateLimited => "rate_limited",
            Self::InvalidResponse => "invalid_response",
            Self::Unavailable => "unavailable",
            Self::UnexpectedStatus => "unexpected_status",
        }
    }

    #[must_use]
    pub const fn definite(self) -> bool {
        matches!(
            self,
            Self::InvalidNotice
                | Self::Malformed
                | Self::Unauthenticated
                | Self::Conflict
                | Self::TooLarge
                | Self::UnexpectedStatus
        )
    }
}

pub fn classify(status: u16, raw: &[u8], expected_event: &[u8; 16]) -> Result<Ack, NotDelivered> {
    if status == 200 {
        return decode_response(raw, expected_event).map_err(|_| NotDelivered::InvalidResponse);
    }
    let empty = |class| {
        if raw.is_empty() {
            class
        } else {
            NotDelivered::InvalidResponse
        }
    };
    Err(match status {
        400 => empty(NotDelivered::Malformed),
        401 => empty(NotDelivered::Unauthenticated),
        409 => empty(NotDelivered::Conflict),
        413 => empty(NotDelivered::TooLarge),
        429 => empty(NotDelivered::RateLimited),
        503 => NotDelivered::Unavailable,
        _ => NotDelivered::UnexpectedStatus,
    })
}

pub struct RestoreNoticeDescriptor(ProducerDescriptor);

impl RestoreNoticeDescriptor {
    pub fn new(
        connect_endpoint: (String, u16),
        peer_name: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
        other_purposes: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        purpose_descriptor(
            PURPOSE,
            connect_endpoint,
            peer_name,
            roots,
            client_chain,
            client_key,
            other_purposes,
        )
        .map(Self)
    }
}

pub async fn deliver(
    descriptor: &RestoreNoticeDescriptor,
    capacity: &TransientCapacity,
    body: &str,
    expected_event: &[u8; 16],
) -> Result<Ack, NotDelivered> {
    let mut permit = capacity
        .try_queue()
        .map_err(|_| NotDelivered::Unavailable)?;
    permit
        .try_activate()
        .map_err(|_| NotDelivered::Unavailable)?;
    let (status, raw) = http1_mtls::exchange_with_status(
        &descriptor.0,
        Operation::PublishGameRestoreNoticeV1,
        body,
        &mut permit,
    )
    .await
    .map_err(|_| NotDelivered::Unavailable)?;
    classify(status, &raw, expected_event)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub first: Duration,
    pub max: Duration,
    pub attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            first: Duration::from_secs(1),
            max: Duration::from_secs(30),
            attempts: 40,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub result: Result<Ack, NotDelivered>,
    pub attempts: u32,
}

pub async fn publish_until_acknowledged(
    descriptor: &RestoreNoticeDescriptor,
    notice: &RestoreNotice,
    policy: RetryPolicy,
) -> Report {
    let body = match encode(notice) {
        Ok(body) => body,
        Err(_) => {
            return Report {
                result: Err(NotDelivered::InvalidNotice),
                attempts: 0,
            };
        }
    };
    let capacity = TransientCapacity::new();
    let mut attempts = 0;
    let mut backoff = policy.first;
    loop {
        attempts += 1;
        let result = deliver(descriptor, &capacity, &body, &notice.restore_event_id).await;
        let stop = matches!(result, Ok(Ack::Acknowledged { .. }))
            || matches!(result, Err(error) if error.definite());
        if stop || attempts >= policy.attempts.max(1) {
            return Report { result, attempts };
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(policy.max);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreNoticeConfig {
    pub endpoint: SocketAddr,
    pub peer_name: String,
    pub trust_roots_file: PathBuf,
    pub client_certificate_file: PathBuf,
    pub client_key_file: PathBuf,
    pub source_authority: String,
}

impl RestoreNoticeConfig {
    pub fn parse(document: &[u8]) -> Result<Self, SourceError> {
        if document.len() > CONFIG_BYTES {
            return Err(SourceError::InvalidDescriptor);
        }
        let text = std::str::from_utf8(document).map_err(|_| SourceError::InvalidDescriptor)?;
        let config: Self = toml::from_str(text).map_err(|_| SourceError::InvalidDescriptor)?;
        let paths = [
            &config.trust_roots_file,
            &config.client_certificate_file,
            &config.client_key_file,
        ];
        if config.endpoint.port() == 0
            || config.peer_name.is_empty()
            || config.peer_name.len() > 128
            || !paths.iter().all(|path| path.is_absolute())
            || config.client_key_file == config.client_certificate_file
            || !source_authority_valid(&config.source_authority)
        {
            return Err(SourceError::InvalidDescriptor);
        }
        Ok(config)
    }
}

#[cfg(test)]
#[path = "restore_notice_tests.rs"]
mod tests;
