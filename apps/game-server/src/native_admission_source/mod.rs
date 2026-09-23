use std::{fmt, io};
pub mod descriptor;
pub mod http1_mtls;
pub const HANDSHAKE_INBOUND_BYTES: usize = 65_536;
pub const PIPELINE_SLOTS: usize = 2;
pub const QUEUED_REQUESTS: usize = 8;
pub const CONNECT_DEADLINE_MS: u64 = 1_000;
pub const HANDSHAKE_DEADLINE_MS: u64 = 2_000;
pub const EXCHANGE_DEADLINE_MS: u64 = 3_000;
#[derive(Debug)]
pub enum SourceError {
    InvalidDescriptor,
    CapacityExceeded,
    InvalidInput,
    Unauthorized,
    Unavailable,
    Io(io::Error),
    Tls(rustls::Error),
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDescriptor => "invalid source descriptor",
            Self::CapacityExceeded => "native source capacity exceeded",
            Self::InvalidInput => "invalid native source input",
            Self::Unauthorized => "native source unauthorized",
            Self::Unavailable => "native source unavailable",
            Self::Io(_) => "native source I/O unavailable",
            Self::Tls(_) => "native source TLS unavailable",
        })
    }
}
impl std::error::Error for SourceError {}
impl From<io::Error> for SourceError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rustls::Error> for SourceError {
    fn from(e: rustls::Error) -> Self {
        Self::Tls(e)
    }
}
#[derive(Debug)]
pub struct TransientCapacity {
    active: std::sync::atomic::AtomicUsize,
    queued: std::sync::atomic::AtomicUsize,
}
impl TransientCapacity {
    pub const fn new() -> Self {
        Self {
            active: std::sync::atomic::AtomicUsize::new(0),
            queued: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    pub fn try_queue(&self) -> Result<QueuePermit<'_>, SourceError> {
        self.queued
            .fetch_update(
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
                |n| (n < QUEUED_REQUESTS).then_some(n + 1),
            )
            .map_err(|_| SourceError::CapacityExceeded)?;
        Ok(QueuePermit {
            owner: self,
            active: false,
            queued_at: std::time::Instant::now(),
        })
    }
}
pub struct QueuePermit<'a> {
    owner: &'a TransientCapacity,
    active: bool,
    queued_at: std::time::Instant,
}
impl QueuePermit<'_> {
    pub(super) fn require_active(&self) -> Result<(), SourceError> {
        if self.active {
            Ok(())
        } else {
            Err(SourceError::CapacityExceeded)
        }
    }
    pub fn try_activate(&mut self) -> Result<(), SourceError> {
        if self.active {
            return Err(SourceError::CapacityExceeded);
        }
        if self.queued_at.elapsed() >= std::time::Duration::from_millis(1000) {
            return Err(SourceError::Unavailable);
        }
        self.owner
            .active
            .fetch_update(
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
                |n| (n < PIPELINE_SLOTS).then_some(n + 1),
            )
            .map_err(|_| SourceError::CapacityExceeded)?;
        self.owner
            .queued
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        self.active = true;
        Ok(())
    }
}
impl Drop for QueuePermit<'_> {
    fn drop(&mut self) {
        let c = if self.active {
            &self.owner.active
        } else {
            &self.owner.queued
        };
        c.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}

/// Executes only transport and decoding. The caller retains the active permit
/// through publication or definitive reconciliation; this function cannot publish.
pub async fn query(
    descriptor: &descriptor::ProducerDescriptor,
    request: &crate::admission_evidence::Request<'_>,
    permit: &mut QueuePermit<'_>,
) -> Result<crate::admission_evidence::Response, SourceError> {
    use crate::admission_evidence::{Request, decode_response, encode_request};
    use descriptor::Operation;
    permit.require_active()?;
    let operation = match request {
        Request::Account {
            recovery: false, ..
        } => Operation::ReadAccountSecurityV1,
        Request::Account { recovery: true, .. } => Operation::ReadRecoveryAccountSecurityV2,
        Request::Trust {
            recovery: false, ..
        } => Operation::ReadFreshSigningTrustV1,
        Request::Trust { recovery: true, .. } => Operation::ReadRecoverySigningTrustV2,
    };
    let started = tokio::time::Instant::now();
    let encoded = encode_request(request).map_err(|_| SourceError::InvalidInput)?;
    let raw = http1_mtls::exchange(descriptor, operation, &encoded, permit).await?;
    let decoded = decode_response(request, &descriptor.source_authority, &raw)
        .map_err(|_| SourceError::InvalidInput)?;
    if started.elapsed() >= std::time::Duration::from_millis(EXCHANGE_DEADLINE_MS) {
        return Err(SourceError::Unavailable);
    }
    Ok(decoded)
}

/// Configured issuer namespace of the Character bootstrap-intent producer.
pub const CHARACTER_BOOTSTRAP_INTENT_ISSUER: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";
const CHARACTER_BOOTSTRAP_INTENT_REQUEST_BYTES: usize = 256;
const CHARACTER_BOOTSTRAP_INTENT_RESPONSE_BYTES: usize = 4096;

/// Narrow `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1` reconciliation read over
/// the purpose-separated TLS 1.3 mTLS producer. Transport only: the exact
/// bounded body is returned for the Game intent decoder. Every non-200 producer
/// outcome (malformed, unauthorized, unknown or expired, unavailable) is bounded
/// unavailability and never a fallback authorization.
pub async fn read_character_bootstrap_intent(
    descriptor: &descriptor::ProducerDescriptor,
    request_body: &str,
    permit: &mut QueuePermit<'_>,
) -> Result<Vec<u8>, SourceError> {
    permit.require_active()?;
    if descriptor.source_authority != CHARACTER_BOOTSTRAP_INTENT_ISSUER {
        return Err(SourceError::InvalidDescriptor);
    }
    if request_body.is_empty() || request_body.len() > CHARACTER_BOOTSTRAP_INTENT_REQUEST_BYTES {
        return Err(SourceError::InvalidInput);
    }
    let started = tokio::time::Instant::now();
    let raw = http1_mtls::exchange(
        descriptor,
        descriptor::Operation::ReadCharacterBootstrapIntentV1,
        request_body,
        permit,
    )
    .await?;
    if raw.is_empty() || raw.len() > CHARACTER_BOOTSTRAP_INTENT_RESPONSE_BYTES {
        return Err(SourceError::InvalidInput);
    }
    if started.elapsed() >= std::time::Duration::from_millis(EXCHANGE_DEADLINE_MS) {
        return Err(SourceError::Unavailable);
    }
    Ok(raw)
}
