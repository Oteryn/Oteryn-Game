//! W04 replay: the RAID_SCHEDULE_QUERY state reply applied by the production client reducer
//! `apply` (`protocol.rs`); the request is built by the production `command`.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;

type Error = Box<dyn std::error::Error>;

fn captured() -> Result<(GatedCommandResult, GatedCommandResult, String), Error> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing live capture directory")?;
    let raw = std::fs::read(std::path::Path::new(&root).join("w04-exchange.json"))?;
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(&raw) {
        write!(digest, "{byte:02x}")?;
    }
    assert_eq!(digest, std::env::var("W04_CAPTURE_SHA256")?);
    let exchange: serde_json::Value = serde_json::from_slice(&raw)?;
    let session = exchange["session"]
        .as_str()
        .ok_or("missing captured session")?;
    assert!(!session.is_empty());
    let decode = |value: &serde_json::Value| -> Result<GatedCommandResult, Error> {
        let (command_id, server_sequence, status, payload): (u64, u64, u32, Vec<u8>) =
            serde_json::from_value(value.clone())?;
        let status = match status {
            1 => CommandStatus::Accepted,
            2 => CommandStatus::Rejected,
            _ => return Err("unexpected captured command status".into()),
        };
        Ok(GatedCommandResult {
            command_id,
            server_sequence,
            status,
            payload,
        })
    };
    let accepted = decode(&exchange["accepted"])?;
    let refused = decode(&exchange["refused"])?;
    assert_ne!(accepted.command_id, 0);
    assert_ne!(accepted.server_sequence, 0);
    assert_eq!(accepted.command_id.checked_add(1), Some(refused.command_id));
    assert!(accepted.server_sequence < refused.server_sequence);
    assert_eq!(accepted.status, CommandStatus::Accepted);
    assert_eq!(refused.status, CommandStatus::Rejected);
    assert!(refused.payload.is_empty());
    let expected = exchange["expected"]
        .as_str()
        .ok_or("missing expected state")?;
    Ok((accepted, refused, expected.to_owned()))
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

#[test]
#[ignore = "requires a current W04 live capture"]
fn e2e_w04_replays_raid_schedule_state_and_refusal() -> Result<(), Error> {
    let (accepted, refused, expected) = captured()?;
    assert_exchange(&accepted, &refused, &expected)
}

fn assert_exchange(
    accepted: &GatedCommandResult,
    refused: &GatedCommandResult,
    expected: &str,
) -> Result<(), Error> {
    let mut panel = Panel {
        selected: true,
        open: true,
        pending: true,
        pending_kind: 157,
        ..Panel::default()
    };
    let query = command(&panel, Intent::Query)?;
    assert_eq!(query.capability, 149);
    assert_eq!(query.kind, wire::COMMAND_TYPE_RAID_SCHEDULE_QUERY);
    assert!(query.payload.is_empty());
    panel.pending_kind = query.kind;

    let before = panel.summary.clone();
    apply(&mut panel, &query, accepted);
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "Raid schedule updated");
    let key = value(expected, "key")?;
    let confirmed = if key.is_empty() {
        "No raid scheduled".to_owned()
    } else {
        format!(
            "{key} | {}\r\nCommitted steps: {} | Active creatures: {}",
            if value(expected, "running")? == "true" {
                "Running"
            } else {
                "Scheduled"
            },
            value(expected, "steps")?,
            value(expected, "creatures")?
        )
    };
    assert_eq!(panel.summary, confirmed);

    assert_ne!(before, confirmed);

    panel.pending = true;
    apply(&mut panel, &query, refused);
    assert!(!panel.pending);
    assert_eq!(panel.summary, confirmed);
    assert_eq!(panel.feedback, "The server refused the request");
    Ok(())
}

#[test]
fn w04_malformed_state_preserves_confirmed_summary() -> Result<(), Error> {
    let mut panel = Panel {
        selected: true,
        open: true,
        pending: true,
        pending_kind: wire::COMMAND_TYPE_RAID_SCHEDULE_QUERY,
        summary: "confirmed".to_owned(),
        ..Panel::default()
    };
    let query = command(&panel, Intent::Query)?;
    let malformed = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![0x0a],
    };
    apply(&mut panel, &query, &malformed);
    assert!(!panel.pending);
    assert_eq!(panel.summary, "confirmed");
    assert_eq!(panel.feedback, "Invalid raid schedule reply");
    Ok(())
}
