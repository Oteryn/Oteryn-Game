//! I10 replay: server house-furniture wrap replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0207-house-furniture-wrap/protocol.rs`
//! (capability 207, command 402). Live captures include the sent request payload, the observed
//! command status as a big-endian u32, and the server reply payload.
use std::path::{Path, PathBuf};

use super::*;
use oteryn_client_runtime::raw_feature::house_wrap::{Action, Refusal, Reply, Request};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PROBES: [(&str, Action, u64, Refusal); 3] = [
    ("wrap", Action::Wrap, 0, Refusal::NotInHouse),
    ("unwrap", Action::Unwrap, 0, Refusal::NotInHouse),
    ("refusal", Action::Wrap, u64::MAX, Refusal::Stale),
];

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .filter_map(|at| u8::from_str_radix(text.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect()
}

fn decode_status(bytes: &[u8]) -> TestResult<CommandStatus> {
    match u32::from_be_bytes(bytes.try_into()?) {
        1 => Ok(CommandStatus::Accepted),
        2 => Ok(CommandStatus::Rejected),
        3 => Ok(CommandStatus::DuplicateReplay),
        4 => Ok(CommandStatus::DuplicateOutcomeExpired),
        code => Err(format!("unknown command status {code}").into()),
    }
}

fn request(action: Action, revision: u64) -> FeatureCommand {
    FeatureCommand {
        capability: 207,
        kind: wire::COMMAND_WRAP,
        payload: Request {
            action,
            item: [0; 16],
            expected_revision: revision,
            acl_revision: 0,
            position: [32_370, 32_241, 7],
        }
        .encode(),
    }
}

fn apply_reply(command: &FeatureCommand, status: CommandStatus, payload: &[u8]) -> Panel {
    let mut panel = Panel {
        selected: true,
        pending: true,
        pending_payload: command.payload.clone(),
        ..Panel::default()
    };
    apply(
        &mut panel,
        command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status,
            payload: payload.to_vec(),
        },
    );
    assert!(!panel.pending && panel.pending_payload.is_empty());
    panel
}

fn replay_probes(capture: Option<&Path>) -> TestResult {
    let manifest = capture
        .map(|path| std::fs::read_to_string(path.join("i10-expected.txt")))
        .transpose()?;
    let capture = capture.zip(manifest.as_deref());
    for (key, action, revision, refusal) in PROBES {
        let (command, status_bytes, payload) = if let Some((path, manifest)) = capture {
            let command = FeatureCommand {
                capability: wire::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
                kind: wire::COMMAND_WRAP,
                payload: std::fs::read(path.join(format!("i10-{key}-request.bin")))?,
            };
            let status = std::fs::read(path.join(format!("i10-{key}-status.bin")))?;
            let payload = std::fs::read(path.join(format!("i10-{key}.bin")))?;
            let line = manifest
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or("expected capture line absent")?;
            assert_eq!(hex(line), payload, "{key} capture disagrees");
            (command, status, payload)
        } else {
            (
                request(action, revision),
                (CommandStatus::Rejected as u32).to_be_bytes().to_vec(),
                Reply::Refused(refusal).encode(),
            )
        };
        let decoded = Request::decode(&command.payload)
            .ok_or_else(|| format!("{key}: invalid furniture request"))?;
        // The live row supplies the item, access revision and position from its own setup.
        assert_eq!(decoded.action, action, "{key} request action");
        assert_eq!(
            decoded.expected_revision, revision,
            "{key} request revision"
        );
        let status = decode_status(&status_bytes)?;
        assert_eq!(status, CommandStatus::Rejected, "{key} command status");
        let reply =
            Reply::decode(&payload).ok_or_else(|| format!("{key}: invalid furniture reply"))?;
        if reply != Reply::Refused(refusal) {
            return Err(format!("{key}: expected {refusal:?}, got {reply:?}").into());
        }
        let panel = apply_reply(&command, status, &payload);
        assert_eq!(panel.feedback, refusal_feedback(refusal), "{key} feedback");
        assert!(panel.catalogue.is_none());
    }
    Ok(())
}

#[test]
fn e2e_i10_rejects_not_in_house_for_the_stale_probe() -> TestResult {
    let capture = tempfile::tempdir()?;
    let path = capture.path();
    std::fs::write(
        path.join("i10-expected.txt"),
        "wrap=01\nunwrap=01\nrefusal=01\n",
    )?;
    for (key, action, revision, _) in PROBES {
        std::fs::write(
            path.join(format!("i10-{key}-request.bin")),
            request(action, revision).payload,
        )?;
        std::fs::write(
            path.join(format!("i10-{key}-status.bin")),
            (CommandStatus::Rejected as u32).to_be_bytes(),
        )?;
        std::fs::write(
            path.join(format!("i10-{key}.bin")),
            Reply::Refused(Refusal::NotInHouse).encode(),
        )?;
    }
    let Err(error) = replay_probes(Some(path)) else {
        return Err("replay accepted NotInHouse for the stale probe".into());
    };
    assert_eq!(
        error.to_string(),
        "refusal: expected Stale, got Refused(NotInHouse)"
    );
    Ok(())
}

#[test]
fn e2e_i10_replays_wrap_unwrap_and_stale_refusals() -> TestResult {
    let capture = std::env::var_os("E2E_ALL_CAPTURE_DIR").map(PathBuf::from);
    replay_probes(capture.as_deref())?;
    assert_eq!(
        refusal_feedback(Refusal::NotInHouse),
        "Enter your house first"
    );
    assert_eq!(
        refusal_feedback(Refusal::Stale),
        "Furniture or house access changed. Refresh the list"
    );

    // The same refusal bytes on an accepted command are not a refusal, and a committed reply on a
    // rejected command is not a success.
    let command = request(Action::Wrap, 0);
    let refusal = Reply::Refused(Refusal::NotInHouse).encode();
    assert_eq!(
        apply_reply(&command, CommandStatus::Accepted, &refusal).feedback,
        "Invalid furniture response"
    );
    assert_eq!(
        apply_reply(
            &command,
            CommandStatus::Rejected,
            &Reply::Committed(7).encode()
        )
        .feedback,
        "Invalid furniture response"
    );
    assert_eq!(
        apply_reply(
            &command,
            CommandStatus::Accepted,
            &Reply::Committed(7).encode()
        )
        .feedback,
        "Furniture updated (revision 7). Refresh the list"
    );
    // Truncated and unknown replies are invalid.
    for payload in [&[][..], &[7], &[0, 1]] {
        assert_eq!(
            apply_reply(&command, CommandStatus::Rejected, payload).feedback,
            "Invalid furniture response"
        );
    }

    // A reply for a different request than the pending one changes nothing.
    let mut panel = Panel {
        selected: true,
        pending: true,
        pending_payload: request(Action::Unwrap, 0).payload,
        ..Panel::default()
    };
    apply(
        &mut panel,
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Rejected,
            payload: refusal,
        },
    );
    assert!(panel.pending && panel.feedback.is_empty());
    Ok(())
}
