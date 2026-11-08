//! I10 replay: server house-furniture wrap replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0207-house-furniture-wrap/protocol.rs`
//! (capability 207, command 402). The request is the exact wire form the capture phase sent.
use super::*;
use oteryn_client_runtime::raw_feature::house_wrap::{Action, Refusal, Reply, Request};

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .filter_map(|at| u8::from_str_radix(text.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect()
}

/// The refusal text the player must see, stated independently of the reducer.
fn visible(code: u8) -> &'static str {
    match code {
        1 => "Enter your house first",
        2 => "You cannot edit this house",
        3 => "This item cannot be wrapped or placed",
        4 => "Furniture or house access changed. Refresh the list",
        5 => "The placement is blocked. Move and refresh the list",
        6 => "Your backpack is full",
        _ => "unexpected reply code",
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

#[test]
fn e2e_i10_replays_wrap_unwrap_and_stale_refusals() -> Result<(), Box<dyn std::error::Error>> {
    // (action, expected revision, server reply payload) for the wrap, unwrap and stale requests.
    let captured = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let expected = std::fs::read_to_string(path.join("i10-expected.txt"))?;
        let mut replies = Vec::new();
        for (key, name, action, revision) in [
            ("wrap", "i10-wrap.bin", Action::Wrap, 0),
            ("unwrap", "i10-unwrap.bin", Action::Unwrap, 0),
            ("refusal", "i10-refusal.bin", Action::Wrap, u64::MAX),
        ] {
            let payload = std::fs::read(path.join(name))?;
            let line = expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or("expected capture line absent")?;
            assert_eq!(hex(line), payload, "{key} capture disagrees");
            replies.push((action, revision, payload));
        }
        replies
    } else {
        vec![
            (Action::Wrap, 0, Reply::Refused(Refusal::NotInHouse).encode()),
            (Action::Unwrap, 0, Reply::Refused(Refusal::NotInHouse).encode()),
            (Action::Wrap, u64::MAX, Reply::Refused(Refusal::Stale).encode()),
        ]
    };
    for (action, revision, payload) in &captured {
        let [code] = payload.as_slice() else {
            return Err("captured refusal is not one byte".into());
        };
        let panel = apply_reply(&request(*action, *revision), CommandStatus::Rejected, payload);
        assert_eq!(panel.feedback, visible(*code));
        assert!(panel.catalogue.is_none());
    }

    // The same refusal bytes on an accepted command are not a refusal, and a committed reply on a
    // rejected command is not a success.
    let command = request(Action::Wrap, 0);
    let refusal = &captured[0].2;
    assert_eq!(
        apply_reply(&command, CommandStatus::Accepted, refusal).feedback,
        "Invalid furniture response"
    );
    assert_eq!(
        apply_reply(&command, CommandStatus::Rejected, &Reply::Committed(7).encode()).feedback,
        "Invalid furniture response"
    );
    assert_eq!(
        apply_reply(&command, CommandStatus::Accepted, &Reply::Committed(7).encode()).feedback,
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
            payload: refusal.clone(),
        },
    );
    assert!(panel.pending && panel.feedback.is_empty());
    Ok(())
}
