//! W04 replay: the RAID_SCHEDULE_QUERY state reply applied by the production client reducer
//! `apply` (`protocol.rs`); the request is built by the production `command`.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;

type Error = Box<dyn std::error::Error>;

fn captured() -> Result<(Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("w04-state.bin"))?,
            std::fs::read_to_string(root.join("w04-expected.txt"))?,
        ));
    }
    let state = wire::RaidScheduleState {
        raid_key: "raid:badger".into(),
        running: true,
        committed_steps: 2,
        active_creatures: 5,
    };
    Ok((
        wire::encode_raid_schedule_state(&state).map_err(|error| format!("{error:?}"))?,
        "key=raid:badger\nrunning=true\nsteps=2\ncreatures=5\n".into(),
    ))
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

#[test]
fn e2e_w04_replays_raid_schedule_state_and_refusal() -> Result<(), Error> {
    let (payload, expected) = captured()?;
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

    apply(&mut panel, &query, &result(CommandStatus::Accepted, &payload));
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "Raid schedule updated");
    let key = value(&expected, "key")?;
    let confirmed = if key.is_empty() {
        "No raid scheduled".to_owned()
    } else {
        format!(
            "{key} | {}\r\nCommitted steps: {} | Active creatures: {}",
            if value(&expected, "running")? == "true" {
                "Running"
            } else {
                "Scheduled"
            },
            value(&expected, "steps")?,
            value(&expected, "creatures")?
        )
    };
    assert_eq!(panel.summary, confirmed);

    // A rejected query and a malformed state leave the confirmed summary untouched.
    for (status, bytes) in [
        (CommandStatus::Rejected, &[][..]),
        (CommandStatus::Accepted, &[0x0a][..]),
    ] {
        panel.pending = true;
        apply(&mut panel, &query, &result(status, bytes));
        assert!(!panel.pending);
        assert_eq!(panel.summary, confirmed);
    }
    assert_eq!(panel.feedback, "Invalid raid schedule reply");
    Ok(())
}
