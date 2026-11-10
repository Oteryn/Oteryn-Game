//! P14 replay: the server's OFFLINE_TRAINING replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0054-training/protocol.rs`; the request is
//! built by the production `command`, so the reducer sees the exact kind the app sends.
use super::*;
use oteryn_client_ui::training::panel_text;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

fn value(expected: &str, key: &str) -> Result<u64, Box<dyn std::error::Error>> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent"))?
        .parse()
        .map_err(Into::into)
}

/// (refused reply, selected reply, expected file text).
type Replies = (Vec<u8>, Vec<u8>, String);

fn replies() -> Result<Replies, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("p14-refused.bin"))?,
            std::fs::read(root.join("p14-selected.bin"))?,
            std::fs::read_to_string(root.join("p14-expected.txt"))?,
        ));
    }
    let encode = |disposition, revision, pool_seconds, charges| {
        adapter::loop_training::encode_training_result(adapter::loop_training::TrainingResult {
            disposition,
            revision,
            pool_seconds,
            charges,
        })
        .map_err(|error| format!("{error:?}"))
    };
    Ok((
        encode(4, 0, 0, 0)?,
        encode(3, 12, 5400, 0)?,
        "refused_disposition=4\nselected_disposition=3\nselected_pool_seconds=5400\nselected_charges=0\n"
            .into(),
    ))
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending() -> Panel {
    let mut panel = Panel {
        selected: true,
        open: true,
        pending: true,
        pending_kind: 66,
        ..Panel::default()
    };
    panel.fields[0] = "4242".into();
    panel
}

#[test]
fn e2e_p14_replays_offline_training_replies() -> Result<(), Box<dyn std::error::Error>> {
    let (refused, selected, expected) = replies()?;
    assert!(!refused.is_empty() && !selected.is_empty());
    let panel = pending();
    let command = command(&panel, Intent::Offline)?;
    assert_eq!((command.capability, command.kind), (54, 66));
    assert_eq!(command.payload, [8, 0x92, 0x21]);

    // The refused reply: the server's wire says disposition 4 and nothing changes in the pool.
    assert_eq!(value(&expected, "refused_disposition")?, 4);
    let mut after_refusal = panel.clone();
    apply(&mut after_refusal, &command, &result(CommandStatus::Accepted, &refused));
    assert!(!after_refusal.pending);
    assert_eq!(
        after_refusal.feedback,
        "The server refused the training request"
    );
    assert_eq!(after_refusal.charges, None);
    assert_eq!(after_refusal.pool_seconds, None);
    assert!(after_refusal.summary.is_empty());

    // The statue reply: disposition 3 with the durable pool the server granted.
    assert_eq!(value(&expected, "selected_disposition")?, 3);
    let pool = u32::try_from(value(&expected, "selected_pool_seconds")?)?;
    let charges = u32::try_from(value(&expected, "selected_charges")?)?;
    assert!(pool > 0);
    let mut after_select = panel.clone();
    apply(&mut after_select, &command, &result(CommandStatus::Accepted, &selected));
    assert!(!after_select.pending);
    assert_eq!(
        after_select.feedback,
        "Offline training selected; the server is logging you out"
    );
    assert_eq!(after_select.pool_seconds, Some(pool));
    assert_eq!(after_select.charges, Some(charges));
    assert_eq!(
        after_select.summary,
        format!("Remaining charges: {charges}\r\nOffline pool: {pool} seconds")
    );
    assert_eq!(
        panel_text(&after_select),
        format!(
            "Training\r\n\r\nRemaining charges: {charges}\r\nOffline pool: {pool} seconds\r\nOffline training selected; the server is logging you out"
        )
    );

    // A rejected status never reads the payload as a pool grant.
    let mut rejected = panel.clone();
    apply(&mut rejected, &command, &result(CommandStatus::Rejected, &selected));
    assert_eq!(rejected.feedback, "The server refused the training request");
    assert_eq!((rejected.charges, rejected.pool_seconds), (None, None));

    // A truncated payload is an invalid reply and leaves the pool and summary unchanged.
    for payload in [&selected[..selected.len() - 1], &[8][..], &[]] {
        let mut invalid = panel.clone();
        apply(&mut invalid, &command, &result(CommandStatus::Accepted, payload));
        assert_eq!(invalid.feedback, "Invalid training reply");
        assert_eq!((invalid.charges, invalid.pool_seconds), (None, None));
        assert!(invalid.summary.is_empty());
    }

    // A reply for another request kind, or with nothing pending, changes nothing.
    let mut other_kind = Panel {
        pending_kind: 67,
        ..panel.clone()
    };
    let before = other_kind.clone();
    apply(&mut other_kind, &command, &result(CommandStatus::Accepted, &selected));
    assert_eq!(other_kind, before);
    let mut idle = Panel {
        pending: false,
        ..panel
    };
    let before = idle.clone();
    apply(&mut idle, &command, &result(CommandStatus::Accepted, &selected));
    assert_eq!(idle, before);
    Ok(())
}
