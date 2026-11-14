//! Frozen enforcement wire, shared with its durable deciding owner (no puller in Part A).
pub const ENABLED_VAR: &str = "OTERYN_PLATFORM_ENFORCEMENT_ENABLED";

pub fn enabled() -> bool {
    std::env::var(ENABLED_VAR).is_ok_and(|value| enabled_value(Some(&value)))
}
fn enabled_value(value: Option<&str>) -> bool {
    value == Some("true")
}

pub use crate::durability::platform_streams::enforcement_wire::*;

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/../../../docs/contracts/fixtures/game-enforcement-wire-v1/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap_or_default()
    }
    #[test]
    fn enforcement_flag_defaults_off() {
        for value in [None, Some("false"), Some("1"), Some("TRUE"), Some("")] {
            assert!(!enabled_value(value));
        }
        assert!(enabled_value(Some("true")));
    }
    #[test]
    fn enforcement_fixtures_round_trip_byte_exact() -> TestResult {
        for name in [
            "command-apply",
            "command-revoke",
            "read-request",
            "reconcile-page",
            "reconcile-request",
            "result-ack",
            "result-applied",
            "result-revoked",
            "stream-empty",
            "stream-page",
            "stream-request",
        ] {
            let raw = fixture(name);
            let message = Message::parse(&raw).ok_or("invalid fixture")?;
            assert_eq!(message.encode().ok_or("encode")?, raw, "{name}");
            assert_ne!(raw.last(), Some(&b'\n'));
        }
        for name in ["applied", "revoked"] {
            let result =
                EnforcementResult::parse(&fixture(&format!("result-{name}"))).ok_or("result")?;
            let command = fixture(if name == "applied" {
                "command-apply"
            } else {
                "command-revoke"
            });
            assert_eq!(result.command_digest, digest(&command));
        }
        let page = ReconcilePage::parse(&fixture("reconcile-page")).ok_or("reconcile")?;
        assert_eq!(
            page.items[0].result_digest.as_deref(),
            Some(digest(&fixture("result-applied")).as_str())
        );
        Ok(())
    }
    #[test]
    fn enforcement_command_rejects_malformed() -> TestResult {
        let raw = String::from_utf8(fixture("command-apply"))?;
        for bad in [
            raw.replace("\"command_version\":1", "\"command_version\":1,\"extra\":1"),
            raw.replace(
                "\"command_version\":1",
                "\"command_version\":1,\"command_version\":1",
            ),
            raw.replace("\"command_version\":1,", ""),
            raw.replace("\"decision_revision\":1", "\"decision_revision\":1.5"),
            raw.replace("\"decision_revision\":1", "\"decision_revision\":\"1\""),
            raw.replace("\"reason_code\":7", "\"reason_code\":0"),
            raw.replace(
                "019a0000-0000-7000-8000-000000000101",
                "019a0000-0000-4000-8000-000000000101",
            ),
            raw.replace("\"APPLY\"", "\"REPLACE\""),
            raw.replace("\"decision_revision\":1", "\"decision_revision\":1e0"),
            raw.replace(
                "\"prior_decision_revision\":null",
                "\"prior_decision_revision\":1",
            ),
            raw.replace("1700003600000", "1700000000000"),
            raw.replace("\"MUTE\"", "\"BAN\""),
            raw.replace("\"reason_code\":7", "\"reason_code\":null"),
            raw.replace("\"prior_decision_revision\":null,", ""),
        ] {
            assert!(Command::parse(bad.as_bytes()).is_none(), "{bad}");
        }
        let revoke = String::from_utf8(fixture("command-revoke"))?;
        assert!(
            Command::parse(
                revoke
                    .replace(
                        "\"effective_at_ms\":null",
                        "\"effective_at_ms\":1700000000000"
                    )
                    .as_bytes()
            )
            .is_none()
        );
        let expire = revoke
            .replace("\"REVOKE\"", "\"EXPIRE\"")
            .replace(
                "\"prior_decision_revision\":1",
                "\"prior_decision_revision\":null",
            )
            .replace("\"reason_code\":12", "\"reason_code\":null");
        assert!(Command::parse(expire.as_bytes()).is_some());
        assert!(
            Command::parse(
                expire
                    .replace(
                        "\"effective_at_ms\":null",
                        "\"effective_at_ms\":1700000000000"
                    )
                    .as_bytes()
            )
            .is_none()
        );
        assert!(
            Command::parse(
                revoke
                    .replace(
                        "\"prior_decision_revision\":1",
                        "\"prior_decision_revision\":null"
                    )
                    .as_bytes()
            )
            .is_none()
        );
        Ok(())
    }
}
