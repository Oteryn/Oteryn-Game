//! Non-secret role and stage context shared by MP evidence and its focused tests.
use std::net::SocketAddr;

pub(super) fn role_context(
    ordinal: usize,
    index: usize,
    profile: Option<&str>,
    endpoint: SocketAddr,
    stage: &str,
) -> String {
    format!(
        "bot={ordinal} pair_index={index} profile={} endpoint={endpoint} stage={stage}",
        profile.unwrap_or("unprovisioned")
    )
}
