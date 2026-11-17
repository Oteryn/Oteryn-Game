//! Runner diagnostics must identify the exact bot, endpoint, and reconnect stage.
use std::net::SocketAddr;
#[path = "../../qualification/client_phases/mp/diagnostics.rs"]
mod diagnostics;
use diagnostics::role_context;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn mp_bot_diagnostic_distinguishes_shared_pair_indices_and_unprovisioned_role() -> TestResult {
    let endpoint: SocketAddr = "127.0.0.1:17181".parse()?;
    for (ordinal, index, profile) in [
        (0, 0, Some("knight")),
        (1, 0, Some("druid")),
        (2, 1, Some("paladin")),
        (3, 1, None),
    ] {
        let text = role_context(ordinal, index, profile, endpoint, "admissions-pump");
        assert!(text.contains(&format!("bot={ordinal} pair_index={index}")));
        assert!(text.contains(&format!("profile={}", profile.unwrap_or("unprovisioned"))));
        assert!(text.contains("endpoint=127.0.0.1:17181 stage=admissions-pump"));
    }
    Ok(())
}

#[test]
fn mp_bot_diagnostic_preserves_reconnect_stage_and_ipv6_endpoint() -> TestResult {
    let endpoint: SocketAddr = "[::1]:17181".parse()?;
    for stage in [
        "connect-initial",
        "connect-for-provision",
        "connect-profiled",
        "connect-relog",
        "logout-all",
    ] {
        let text = role_context(1, 0, Some("druid"), endpoint, stage);
        assert_eq!(
            text,
            format!("bot=1 pair_index=0 profile=druid endpoint=[::1]:17181 stage={stage}")
        );
    }
    Ok(())
}
