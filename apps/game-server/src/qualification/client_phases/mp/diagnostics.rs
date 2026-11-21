// Non-secret role and stage context shared by MP evidence and its focused tests.
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

/// Classifies a failed logout from what the bot itself observed. The harness never closes a
/// live transport before this point, so a transport error means the server closed it first.
/// `before_send` is true when the pre-logout liveness drain already failed.
pub(super) fn logout_failure_cause(before_send: bool, error: &str) -> &'static str {
    if error.contains("server closed the connection with E") {
        "server-protocol-error"
    } else if !error.contains("transport I/O failed: ") {
        if before_send {
            "liveness-failed-before-logout"
        } else {
            "logout-failed"
        }
    } else if before_send {
        "server-closed-before-logout-sent"
    } else {
        "server-closed-during-logout"
    }
}

/// The row's failure text. A server-side close names the server record that carries its
/// reason: the `serve_end` whose `server_sequence` equals the bot's observed sequence.
pub(super) fn logout_failure_detail(cause: &str, sequence_at_begin: u64, error: &str) -> String {
    let record = if cause.starts_with("server-closed") {
        " server_record=serve_end-with-server_sequence-equal-to-observed_server_sequence"
    } else {
        ""
    };
    format!("cause={cause} sequence_at_logout_begin={sequence_at_begin}{record} detail={error}")
}
