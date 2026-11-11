//! N009/N014: Game pulls Platform's sanction and loyalty streams (`PLATFORM_SANCTION_LOYALTY_EXPORT_V1`,
//! owner decision 2026-11-11) over private `/internal/v1` mutual TLS, built by the shared Platform
//! client builder ([`crate::premium::client`]). Each stream has its own client identity, never the
//! Premium one and never the other stream's. Platform never calls Game SQL.
//!
//! Each enabled stream has one task: it reads the durable cursor, requests the next page after
//! it, checks the page and projects it with the cursor in one transaction
//! ([`DurabilityRoot::apply_platform_stream_page`]). A full page is followed at once; a short one
//! waits [`IDLE_POLL`]; a failure backs off like the Premium pull. Both streams are off unless
//! their variable is `true`, and an enabled stream needs its own identity.
use crate::durability::DurabilityRoot;
use crate::durability::platform_streams::{
    LoyaltySnapshot, PageCounts, PageOutcome, PlatformStream, SanctionDecision, StreamPage,
};
use crate::premium::client::{
    ClientConfigError, IDENTITY_VAR, PLATFORM_CA_VAR, PULL_TIMEOUT, PremiumClientConfig,
    PullFailure, URL_VAR, platform_http, post_json,
};
use oteryn_protocol_oteryn::sanction_record as wire;
use serde::Deserialize;
use std::time::Duration;

pub const SANCTION_STREAM_PATH: &str = "/internal/v1/support/sanctions/stream/read";
pub const LOYALTY_STREAM_PATH: &str = "/internal/v1/products-entitlements/loyalty-snapshots/stream";
pub const SANCTION_REQUEST_SCHEMA: &str = "oteryn.sanction_stream_request.v1";
pub const LOYALTY_REQUEST_SCHEMA: &str = "oteryn.loyalty_stream_request.v1";
pub const SANCTION_PAGE_SCHEMA: &str = "oteryn.sanction_stream_page.v1";
pub const LOYALTY_PAGE_SCHEMA: &str = "oteryn.loyalty_stream_page.v1";
/// `true` enables the stream; unset or `false` leaves it off; any other value is invalid.
pub const SANCTIONS_ENABLED_VAR: &str = "OTERYN_PLATFORM_SANCTION_STREAM_ENABLED";
pub const LOYALTY_ENABLED_VAR: &str = "OTERYN_PLATFORM_LOYALTY_STREAM_ENABLED";
/// Platform's private origin (`https://host:port`) and CA bundle (PEM file), shared by both streams.
pub const STREAMS_URL_VAR: &str = "OTERYN_PLATFORM_STREAMS_URL";
pub const STREAMS_PLATFORM_CA_VAR: &str = "OTERYN_PLATFORM_STREAMS_PLATFORM_CA_PEM";
/// Each stream's own client identity: a PEM file with its certificate chain and private key.
pub const SANCTION_IDENTITY_VAR: &str = "OTERYN_PLATFORM_SANCTION_STREAM_CLIENT_IDENTITY_PEM";
pub const LOYALTY_IDENTITY_VAR: &str = "OTERYN_PLATFORM_LOYALTY_STREAM_CLIENT_IDENTITY_PEM";
/// Records asked for per page; the contract allows 1..=500.
pub const PAGE_LIMIT: usize = 200;
pub const MAX_LIMIT: usize = 500;
/// The page body read cap, before parsing.
pub const MAX_PAGE_BYTES: usize = 262_144;
pub const MAX_CURSOR_BYTES: usize = 256;
/// The wait after a page shorter than the limit (caught up) or a superseded page.
pub const IDLE_POLL: Duration = Duration::from_secs(30);
const MAX_SPENT_PREMIUM_DAYS: i64 = 2_147_483_647;
const MAX_LOYALTY_POINTS: i64 = 2_147_483_647;

/// The request body of one page.
pub fn request_body(stream: PlatformStream, after: Option<&str>, limit: usize) -> Option<Vec<u8>> {
    if !(1..=MAX_LIMIT).contains(&limit) || after.is_some_and(|cursor| !valid_cursor(cursor)) {
        return None;
    }
    let schema = match stream {
        PlatformStream::Sanctions => SANCTION_REQUEST_SCHEMA,
        PlatformStream::Loyalty => LOYALTY_REQUEST_SCHEMA,
    };
    serde_json::to_vec(&serde_json::json!({
        "schema": schema,
        "after": after,
        "limit": limit,
    }))
    .ok()
}

/// A checked page: its records and the cursor to resume after (`None` keeps the stored one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub records: StreamPage,
    pub next_cursor: Option<String>,
    /// As many records as asked for: more may follow at once.
    pub full: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SanctionPageJson {
    schema: String,
    items: Vec<SanctionItemJson>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SanctionItemJson {
    sanction_stream_id: String,
    decision_revision: i64,
    account_id: String,
    character_id: Option<String>,
    kind: u8,
    reason_code: u16,
    reason_text: String,
    starts_at_ms: i64,
    ends_at_ms: Option<i64>,
    revoked: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoyaltyPageJson {
    schema: String,
    items: Vec<LoyaltyItemJson>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoyaltyItemJson {
    account_id: String,
    spent_premium_days: i64,
    loyalty_points: i64,
    platform_revision: i64,
}

/// Decode and check one page asked for with `limit`. Any record outside the export shape refuses
/// the whole page: nothing of it is projected and the cursor stays.
pub fn decode_page(stream: PlatformStream, raw: &[u8], limit: usize) -> Option<Page> {
    if raw.len() > MAX_PAGE_BYTES {
        return None;
    }
    let (records, count, next_cursor) = match stream {
        PlatformStream::Sanctions => {
            let page: SanctionPageJson = serde_json::from_slice(raw).ok()?;
            if page.schema != SANCTION_PAGE_SCHEMA {
                return None;
            }
            let count = page.items.len();
            let items = page
                .items
                .into_iter()
                .map(sanction)
                .collect::<Option<Vec<_>>>()?;
            (StreamPage::Sanctions(items), count, page.next_cursor)
        }
        PlatformStream::Loyalty => {
            let page: LoyaltyPageJson = serde_json::from_slice(raw).ok()?;
            if page.schema != LOYALTY_PAGE_SCHEMA {
                return None;
            }
            let count = page.items.len();
            let items = page
                .items
                .into_iter()
                .map(loyalty)
                .collect::<Option<Vec<_>>>()?;
            (StreamPage::Loyalty(items), count, page.next_cursor)
        }
    };
    if count > limit
        || (count > 0 && next_cursor.is_none())
        || next_cursor
            .as_deref()
            .is_some_and(|cursor| !valid_cursor(cursor))
    {
        return None;
    }
    Some(Page {
        records,
        next_cursor,
        full: count == limit,
    })
}

fn sanction(item: SanctionItemJson) -> Option<SanctionDecision> {
    let stream_id = uuid(&item.sanction_stream_id)?;
    if stream_id[6] >> 4 != 7 || stream_id[8] & 0xc0 != 0x80 || item.decision_revision <= 0 {
        return None;
    }
    // One shape rule set: the record must fit the client reply it will be shown in.
    let entry = wire::SanctionEntry {
        kind: wire::SanctionKind::from_code(item.kind)?,
        character_scope: item.character_id.is_some(),
        revoked: item.revoked,
        reason_code: item.reason_code,
        starts_at_ms: item.starts_at_ms,
        ends_at_ms: item.ends_at_ms,
        reason: item.reason_text,
    };
    let character_id = match item.character_id.as_deref() {
        Some(text) => Some(uuid(text)?),
        None => None,
    };
    wire::encode_reply(&wire::SanctionRecord {
        more: false,
        entries: vec![entry.clone()],
    })?;
    Some(SanctionDecision {
        sanction_stream_id: stream_id,
        decision_revision: item.decision_revision,
        account_id: uuid(&item.account_id)?,
        character_id,
        kind: i16::from(item.kind),
        reason_code: i32::from(entry.reason_code),
        reason_text: entry.reason,
        starts_at_ms: entry.starts_at_ms,
        ends_at_ms: entry.ends_at_ms,
        revoked: entry.revoked,
    })
}

fn loyalty(item: LoyaltyItemJson) -> Option<LoyaltySnapshot> {
    if !(0..=MAX_SPENT_PREMIUM_DAYS).contains(&item.spent_premium_days)
        || !(0..=MAX_LOYALTY_POINTS).contains(&item.loyalty_points)
        || item.platform_revision <= 0
    {
        return None;
    }
    Some(LoyaltySnapshot {
        account_id: uuid(&item.account_id)?,
        spent_premium_days: item.spent_premium_days,
        loyalty_points: item.loyalty_points,
        platform_revision: item.platform_revision,
    })
}

/// A canonical lowercase hyphenated UUID.
fn uuid(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 || [8, 13, 18, 23].iter().any(|&at| bytes[at] != b'-') {
        return None;
    }
    let digits: Vec<u8> = bytes.iter().copied().filter(|&b| b != b'-').collect();
    if digits.len() != 32 {
        return None;
    }
    let nibble = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    let mut id = [0u8; 16];
    for (slot, pair) in id.iter_mut().zip(digits.chunks_exact(2)) {
        *slot = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(id)
}

/// Platform's opaque cursor: 1..=256 printable ASCII bytes without spaces.
fn valid_cursor(cursor: &str) -> bool {
    (1..=MAX_CURSOR_BYTES).contains(&cursor.len()) && cursor.bytes().all(|b| b.is_ascii_graphic())
}

/// Whether `value` of `var` turns its stream on.
pub fn enabled(
    var: &'static str,
    value: Option<std::ffi::OsString>,
) -> Result<bool, ClientConfigError> {
    let Some(value) = value else {
        return Ok(false);
    };
    match value.to_str() {
        Some("false") => Ok(false),
        Some("true") => Ok(true),
        _ => Err(ClientConfigError::Invalid(var)),
    }
}

/// The base64 body of the first certificate (the leaf) of a PEM identity, whitespace removed.
fn leaf_certificate(pem: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(pem).ok()?;
    let (_, rest) = text.split_once("-----BEGIN CERTIFICATE-----")?;
    let (body, _) = rest.split_once("-----END CERTIFICATE-----")?;
    let body: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    (!body.is_empty()).then_some(body)
}

/// Every enabled stream's identity must carry a certificate that no other Platform client of
/// this node presents: not the Premium identity and not the other stream's. Platform answers a
/// repeated identity with 503, so a shared one is refused here instead.
pub fn check_identities(
    identities: &[(&'static str, &[u8])],
    premium_identity: Option<&[u8]>,
) -> Result<(), ClientConfigError> {
    let premium = premium_identity.and_then(leaf_certificate);
    let mut seen = Vec::new();
    for &(var, pem) in identities {
        let leaf = leaf_certificate(pem).ok_or(ClientConfigError::Invalid(var))?;
        if premium.as_ref() == Some(&leaf) || seen.contains(&leaf) {
            return Err(ClientConfigError::Invalid(var));
        }
        seen.push(leaf);
    }
    Ok(())
}

/// One stream's puller.
#[derive(Debug, Clone)]
pub struct StreamPuller {
    stream: PlatformStream,
    http: reqwest::Client,
    endpoint: reqwest::Url,
}

impl StreamPuller {
    /// `config` carries this stream's own identity; `identity_var` names it in errors.
    pub fn new(
        config: &PremiumClientConfig,
        stream: PlatformStream,
        identity_var: &'static str,
    ) -> Result<Self, ClientConfigError> {
        let rename = |ClientConfigError::Invalid(var)| {
            ClientConfigError::Invalid(match var {
                URL_VAR => STREAMS_URL_VAR,
                PLATFORM_CA_VAR => STREAMS_PLATFORM_CA_VAR,
                _ => identity_var,
            })
        };
        let (http, origin) = platform_http(config).map_err(rename)?;
        let path = match stream {
            PlatformStream::Sanctions => SANCTION_STREAM_PATH,
            PlatformStream::Loyalty => LOYALTY_STREAM_PATH,
        };
        let endpoint = origin
            .join(path)
            .map_err(|_| ClientConfigError::Invalid(STREAMS_URL_VAR))?;
        Ok(Self {
            stream,
            http,
            endpoint,
        })
    }

    /// The pullers the environment enables; none when both variables are unset or `false`.
    /// An enabled stream without its own valid identity, or with one another Platform client
    /// of this node presents, is a configuration error and starts no puller.
    pub fn from_env() -> Result<Vec<Self>, ClientConfigError> {
        let mut streams = Vec::new();
        for (stream, enable_var, identity_var) in [
            (
                PlatformStream::Sanctions,
                SANCTIONS_ENABLED_VAR,
                SANCTION_IDENTITY_VAR,
            ),
            (
                PlatformStream::Loyalty,
                LOYALTY_ENABLED_VAR,
                LOYALTY_IDENTITY_VAR,
            ),
        ] {
            if enabled(enable_var, std::env::var_os(enable_var))? {
                streams.push((stream, identity_var));
            }
        }
        if streams.is_empty() {
            return Ok(Vec::new());
        }
        let read = |var: &'static str| {
            std::env::var_os(var)
                .and_then(|path| std::fs::read(path).ok())
                .ok_or(ClientConfigError::Invalid(var))
        };
        let origin = std::env::var(STREAMS_URL_VAR)
            .map_err(|_| ClientConfigError::Invalid(STREAMS_URL_VAR))?;
        let platform_ca_pem = read(STREAMS_PLATFORM_CA_VAR)?;
        let premium_identity = match std::env::var_os(IDENTITY_VAR) {
            Some(_) => Some(read(IDENTITY_VAR)?),
            None => None,
        };
        let identities = streams
            .iter()
            .map(|&(stream, var)| Ok((stream, var, read(var)?)))
            .collect::<Result<Vec<_>, ClientConfigError>>()?;
        check_identities(
            &identities
                .iter()
                .map(|(_, var, pem)| (*var, pem.as_slice()))
                .collect::<Vec<_>>(),
            premium_identity.as_deref(),
        )?;
        identities
            .into_iter()
            .map(|(stream, var, identity_pem)| {
                let config = PremiumClientConfig {
                    origin: origin.clone(),
                    identity_pem,
                    platform_ca_pem: platform_ca_pem.clone(),
                };
                Self::new(&config, stream, var)
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        match self.stream {
            PlatformStream::Sanctions => "sanctions",
            PlatformStream::Loyalty => "loyalty",
        }
    }
}

/// Why one pull did not advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    Store,
    Pull(PullFailure),
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    More,
    Wait,
}

async fn pull_once(
    root: &DurabilityRoot,
    puller: &StreamPuller,
    event: fn(&str),
) -> Result<Step, Failure> {
    let after = root
        .platform_stream_cursor(puller.stream)
        .await
        .map_err(|_| Failure::Store)?;
    let body = request_body(puller.stream, after.as_deref(), PAGE_LIMIT).ok_or(Failure::Store)?;
    let raw = tokio::time::timeout(
        PULL_TIMEOUT,
        post_json(&puller.http, &puller.endpoint, body, MAX_PAGE_BYTES),
    )
    .await
    .unwrap_or(Err(PullFailure::Timeout))
    .map_err(Failure::Pull)?;
    let page = decode_page(puller.stream, &raw, PAGE_LIMIT).ok_or(Failure::Malformed)?;
    match root
        .apply_platform_stream_page(after, page.records, page.next_cursor)
        .await
        .map_err(|_| Failure::Store)?
    {
        PageOutcome::Advanced(counts) => {
            if counts != PageCounts::default() {
                event(&format!(
                    "event=platform_stream_page stream={} applied={} replayed={} stale={} skipped={}",
                    puller.name(),
                    counts.applied,
                    counts.replayed,
                    counts.stale,
                    counts.skipped
                ));
            }
            Ok(if page.full { Step::More } else { Step::Wait })
        }
        PageOutcome::Superseded => Ok(Step::Wait),
    }
}

async fn run(
    root: DurabilityRoot,
    puller: StreamPuller,
    event: fn(&str),
    heartbeat: oteryn_task_supervisor::Heartbeat,
) {
    let mut attempt = 0u32;
    loop {
        heartbeat.beat();
        let delay = match pull_once(&root, &puller, event).await {
            Ok(Step::More) => {
                attempt = 0;
                Duration::ZERO
            }
            Ok(Step::Wait) => {
                attempt = 0;
                IDLE_POLL
            }
            Err(failure) => {
                event(&format!(
                    "event=platform_stream_failed stream={} reason={failure:?}",
                    puller.name()
                ));
                let retry_after = match failure {
                    Failure::Pull(pull) => pull.retry_after(),
                    Failure::Store | Failure::Malformed => None,
                };
                let delay = crate::premium::refresh::after_failure(
                    attempt,
                    retry_after,
                    crate::premium::refresh::jitter(),
                );
                attempt = attempt.saturating_add(1);
                delay
            }
        };
        let _idle = heartbeat.expected_idle();
        tokio::time::sleep(delay).await;
    }
}

/// The running pullers of one node. Dropping it (node shutdown) aborts every task.
#[derive(Default)]
pub struct PlatformStreams(Vec<tokio::task::JoinHandle<()>>);

impl PlatformStreams {
    /// Start one task per puller; must run inside the Tokio runtime.
    pub fn spawn(root: &DurabilityRoot, pullers: Vec<StreamPuller>, event: fn(&str)) -> Self {
        Self(
            pullers
                .into_iter()
                .map(|puller| {
                    let root = root.clone();
                    let name = match puller.stream {
                        PlatformStream::Sanctions => "platform_sanction_stream",
                        PlatformStream::Loyalty => "platform_loyalty_stream",
                    };
                    oteryn_task_supervisor::spawn_supervised_with_heartbeat(
                        name,
                        None,
                        move |heartbeat| run(root, puller, event, heartbeat),
                    )
                })
                .collect(),
        )
    }
}

impl Drop for PlatformStreams {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

#[cfg(test)]
#[path = "platform_streams/tests.rs"]
mod tests;
