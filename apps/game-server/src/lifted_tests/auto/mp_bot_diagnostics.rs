//! Runner diagnostics must identify the exact bot, endpoint, and reconnect stage.
use std::net::SocketAddr;
mod diagnostics {
    include!("../../qualification/client_phases/mp/diagnostics.rs");
}
use diagnostics::{logout_failure_cause, logout_failure_detail, role_context};
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

#[test]
fn mp_logout_failure_names_server_close_instead_of_blind_reset() -> TestResult {
    let reset =
        "logout attempt 1 failed: transport I/O failed: Connection reset by peer (os error 104)";
    let busy_eof = "logout Busy retry liveness failed after attempt 2: LogoutOutcome { .. }; error=transport I/O failed: unexpected end of file";
    for (before_send, error, cause) in [
        (false, reset, "server-closed-during-logout"),
        (false, busy_eof, "server-closed-during-logout"),
        (
            true,
            "transport I/O failed: Connection reset by peer (os error 104)",
            "server-closed-before-logout-sent",
        ),
    ] {
        assert_eq!(logout_failure_cause(before_send, error), cause);
        assert_eq!(
            logout_failure_detail(cause, 10, error),
            format!(
                "cause={cause} sequence_at_logout_begin=10 server_record=serve_end-with-server_sequence-equal-to-observed_server_sequence detail={error}"
            )
        );
    }
    Ok(())
}

#[test]
fn mp_logout_failure_keeps_non_transport_causes_without_server_record() -> TestResult {
    for (before_send, error, cause) in [
        (
            false,
            "logout attempt 1 failed: server closed the connection with E1105 (trace 01a1)",
            "server-protocol-error",
        ),
        (
            true,
            "server closed the connection with E1105",
            "server-protocol-error",
        ),
        (
            false,
            "MP logout refused:LogoutOutcome { .. };attempts=4;busy_timed_out=true",
            "logout-failed",
        ),
        (false, "MP logout combat release deadline", "logout-failed"),
        (
            true,
            "timed out: server silence",
            "liveness-failed-before-logout",
        ),
    ] {
        assert_eq!(logout_failure_cause(before_send, error), cause);
        assert_eq!(
            logout_failure_detail(cause, 7, error),
            format!("cause={cause} sequence_at_logout_begin=7 detail={error}")
        );
    }
    Ok(())
}
