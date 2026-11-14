//! Frozen enforcement wire, shared with its durable deciding owner (no puller in Part A).
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
