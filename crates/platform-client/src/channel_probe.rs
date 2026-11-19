//! client-channel-probe-v1 (Oteryn-Platform d2e54f2e, PR #1939): the probe target of each
//! Channel and the round trip the Change Channel picker shows. A probe never consumes a login
//! ticket: it is a credential-free `HEAD` on a regional edge, measured on one kept-alive
//! connection (one untimed warm-up, then three timed requests, the median shown).
use crate::{PlatformClientError, contains_forbidden_gameplay_field};
use reqwest::{Client, Url, redirect::Policy};
use serde_json::Value;
use std::time::{Duration, Instant};

/// The longest `probe_url`, in bytes.
pub const MAX_PROBE_URL_BYTES: usize = 256;
/// At most this many probe entries in one list.
pub const MAX_PROBES: usize = crate::MAX_CHANNELS_PER_WORLD;
/// Each probe request ends after this long; a timeout shows no ping.
pub const PROBE_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
/// The picker refreshes its measurements at most this often.
pub const PROBE_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
/// Timed requests after the warm-up.
const TIMED_SAMPLES: usize = 3;

/// One Channel and the edge its round trip is measured against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelProbe {
    pub channel_id: [u8; 16],
    pub probe_url: Url,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelProbes {
    pub epoch: u64,
    pub probes: Vec<ChannelProbe>,
}

/// `GET /v1/client/channel-probes`: a route key anywhere fails closed; a URL that is not plain
/// https (port, query, fragment, credentials, over 256 bytes) or a repeated Channel is invalid.
pub fn parse_channel_probes(bytes: &[u8]) -> Result<ChannelProbes, PlatformClientError> {
    if bytes.len() > crate::DEFAULT_MAX_RESPONSE_BYTES {
        return Err(PlatformClientError::ResponseTooLarge);
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_error| PlatformClientError::InvalidPayload)?;
    if contains_forbidden_gameplay_field(&value) {
        return Err(PlatformClientError::ForbiddenGameplayField);
    }
    let invalid = || PlatformClientError::InvalidPayload;
    let root = value.as_object().ok_or_else(invalid)?;
    let epoch = root
        .get("epoch")
        .and_then(Value::as_u64)
        .ok_or_else(invalid)?;
    let entries = root
        .get("probes")
        .and_then(Value::as_array)
        .filter(|entries| entries.len() <= MAX_PROBES)
        .ok_or_else(invalid)?;
    let mut probes: Vec<ChannelProbe> = Vec::with_capacity(entries.len());
    for entry in entries {
        let entry = entry.as_object().ok_or_else(invalid)?;
        let channel_id = entry
            .get("channel_id")
            .and_then(Value::as_str)
            .and_then(crate::native_login::parse_uuid_v7)
            .ok_or_else(invalid)?;
        let probe_url = entry
            .get("probe_url")
            .and_then(Value::as_str)
            .and_then(probe_url)
            .ok_or_else(invalid)?;
        if probes.iter().any(|probe| probe.channel_id == channel_id) {
            return Err(invalid());
        }
        probes.push(ChannelProbe {
            channel_id,
            probe_url,
        });
    }
    Ok(ChannelProbes { epoch, probes })
}

fn probe_url(text: &str) -> Option<Url> {
    if text.is_empty() || text.len() > MAX_PROBE_URL_BYTES || text.contains(['?', '#']) {
        return None;
    }
    let url = Url::parse(text).ok()?;
    (url.scheme() == "https"
        && url.host_str().is_some()
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none())
    .then_some(url)
}

impl crate::PlatformClient {
    /// The probe list of the Channels this client may enter.
    pub async fn fetch_channel_probes(
        &self,
        cancellation: oteryn_foundation::CancellationToken,
    ) -> Result<ChannelProbes, PlatformClientError> {
        self.fetch_json(
            "v1/client/channel-probes",
            "channel-probes",
            None,
            cancellation,
            parse_channel_probes,
        )
        .await
    }
}

/// The client every probe of one refresh shares, so each edge keeps its connection alive.
pub fn probe_client() -> Result<Client, PlatformClientError> {
    Client::builder()
        .redirect(Policy::none())
        .timeout(PROBE_REQUEST_TIMEOUT)
        .connect_timeout(PROBE_REQUEST_TIMEOUT)
        .build()
        .map_err(|_error| PlatformClientError::ClientCreation)
}

async fn probe_once(client: &Client, url: &Url) -> Option<Duration> {
    let started = Instant::now();
    let response = client.head(url.clone()).send().await.ok()?;
    // A redirect, a rate limit or any failure is no measurement.
    response.status().is_success().then(|| started.elapsed())
}

/// The median of the timed round trips to one edge, in ms; `None` when any request fails.
pub async fn measure(client: &Client, url: &Url) -> Option<u32> {
    probe_once(client, url).await?;
    let mut samples = Vec::with_capacity(TIMED_SAMPLES);
    for _ in 0..TIMED_SAMPLES {
        samples.push(probe_once(client, url).await?);
    }
    samples.sort();
    u32::try_from(samples.get(TIMED_SAMPLES / 2)?.as_millis()).ok()
}

/// Each Channel's ping; Channels sharing one `probe_url` share one measurement.
pub async fn measure_all(client: &Client, probes: &[ChannelProbe]) -> Vec<([u8; 16], Option<u32>)> {
    let mut measured: Vec<(&Url, Option<u32>)> = Vec::new();
    let mut pings = Vec::with_capacity(probes.len());
    for probe in probes {
        let ping = match measured.iter().find(|(url, _)| **url == probe.probe_url) {
            Some((_, ping)) => *ping,
            None => {
                let ping = measure(client, &probe.probe_url).await;
                measured.push((&probe.probe_url, ping));
                ping
            }
        };
        pings.push((probe.channel_id, ping));
    }
    pings
}
