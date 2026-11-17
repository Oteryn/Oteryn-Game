//! P15 replay: the server's exercise-training replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0054-training/protocol.rs`; requests are
//! built by the production `command` so the reducer sees the exact kind the app sends.
use super::*;
use oteryn_client_ui::training::panel_text;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

fn value(expected: &str, key: &str) -> Result<u32, Box<dyn std::error::Error>> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent"))?
        .parse()
        .map_err(Into::into)
}

struct Replies {
    status: Vec<u8>,
    refused: Vec<u8>,
    running: Vec<u8>,
    stopped: Vec<u8>,
    expected: String,
}

fn replies() -> Result<Replies, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Replies {
            status: std::fs::read(root.join("p15-status.bin"))?,
            refused: std::fs::read(root.join("p15-refused.bin"))?,
            running: std::fs::read(root.join("p15-running.bin"))?,
            stopped: std::fs::read(root.join("p15-stopped.bin"))?,
            expected: std::fs::read_to_string(root.join("p15-expected.txt"))?,
        });
    }
    let encode = |disposition, revision, charges| {
        adapter::loop_training::encode_training_result(adapter::loop_training::TrainingResult {
            disposition,
            revision,
            pool_seconds: 0,
            charges,
        })
        .map_err(|error| format!("{error:?}"))
    };
    Ok(Replies {
        status: encode(2, 5, 500)?,
        refused: encode(4, 0, 0)?,
        running: encode(1, 6, 500)?,
        stopped: encode(2, 7, 497)?,
        expected: "status_disposition=2\nstatus_charges=500\nrefused_disposition=4\nrunning_disposition=1\nrunning_charges=500\nstopped_disposition=2\nstopped_charges=497\n".into(),
    })
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
        pending_kind: 67,
        ..Panel::default()
    };
    panel.fields = ["".into(), "7".into(), "9".into()];
    panel
}

fn summary(charges: u32) -> String {
    format!("Remaining charges: {charges}\r\nOffline pool: 0 seconds")
}

#[test]
fn e2e_p15_replays_exercise_training_replies() -> Result<(), Box<dyn std::error::Error>> {
    let replies = replies()?;
    let panel = pending();
    let start = command(&panel, Intent::Start)?;
    assert_eq!((start.capability, start.kind), (54, 67));
    assert_eq!(start.payload, [8, 1, 16, 7, 24, 9]);
    let status = command(&panel, Intent::Status)?;
    assert_eq!(status.payload, [8, 2]);
    let stop = command(&panel, Intent::Stop)?;
    assert_eq!(stop.payload, [8, 3]);

    // STATUS before any run: stopped, with the durable charges the server reported.
    let status_charges = value(&replies.expected, "status_charges")?;
    assert_eq!(value(&replies.expected, "status_disposition")?, 2);
    let mut stopped_status = panel.clone();
    apply(&mut stopped_status, &status, &result(CommandStatus::Accepted, &replies.status));
    assert_eq!(stopped_status.feedback, "Exercise training stopped or settled");
    assert_eq!(stopped_status.charges, Some(status_charges));
    assert_eq!(stopped_status.pool_seconds, Some(0));
    assert_eq!(stopped_status.summary, summary(status_charges));

    // START with a missing weapon or dummy is refused and leaves the panel without charges.
    assert_eq!(value(&replies.expected, "refused_disposition")?, 4);
    let mut refused = panel.clone();
    apply(&mut refused, &start, &result(CommandStatus::Accepted, &replies.refused));
    assert!(!refused.pending);
    assert_eq!(refused.feedback, "The server refused the training request");
    assert_eq!((refused.charges, refused.pool_seconds), (None, None));
    assert!(refused.summary.is_empty());

    // START of a carried weapon on an authored dummy: running with the weapon's charges.
    let running_charges = value(&replies.expected, "running_charges")?;
    assert_eq!(value(&replies.expected, "running_disposition")?, 1);
    let mut running = panel.clone();
    apply(&mut running, &start, &result(CommandStatus::Accepted, &replies.running));
    assert_eq!(running.feedback, "Exercise training is running");
    assert_eq!(running.charges, Some(running_charges));
    assert_eq!(running.summary, summary(running_charges));
    assert_eq!(
        panel_text(&running),
        format!("Training\r\n\r\n{}\r\nExercise training is running", summary(running_charges))
    );

    // STOP settles the run; the charges never exceed the running count.
    let stopped_charges = value(&replies.expected, "stopped_charges")?;
    assert_eq!(value(&replies.expected, "stopped_disposition")?, 2);
    assert!(stopped_charges <= running_charges);
    let mut stopped = running.clone();
    stopped.pending = true;
    apply(&mut stopped, &stop, &result(CommandStatus::Accepted, &replies.stopped));
    assert_eq!(stopped.feedback, "Exercise training stopped or settled");
    assert_eq!(stopped.charges, Some(stopped_charges));
    assert_eq!(stopped.summary, summary(stopped_charges));

    // A rejected status and malformed payloads leave the charges and summary as they were.
    for (code, payload) in [
        (CommandStatus::Rejected, replies.running.as_slice()),
        (CommandStatus::Accepted, &replies.running[..replies.running.len() - 1]),
        (CommandStatus::Accepted, &[]),
    ] {
        let mut invalid = running.clone();
        invalid.pending = true;
        apply(&mut invalid, &start, &result(code, payload));
        assert!(!invalid.pending);
        assert_eq!(invalid.charges, Some(running_charges));
        assert_eq!(invalid.summary, summary(running_charges));
    }
    // A reply for another request kind changes nothing.
    let mut other = Panel {
        pending_kind: 66,
        ..panel
    };
    let before = other.clone();
    apply(&mut other, &start, &result(CommandStatus::Accepted, &replies.running));
    assert_eq!(other, before);
    Ok(())
}
