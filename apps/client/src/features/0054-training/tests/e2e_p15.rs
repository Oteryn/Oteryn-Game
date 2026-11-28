//! P15 replay: the server's exercise-training replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0054-training/protocol.rs`; requests are
//! built by the production `command` so the reducer sees the exact kind the app sends.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::training::panel_text;

fn value(expected: &str, key: &str) -> Result<u32, Box<dyn std::error::Error>> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent"))?
        .parse()
        .map_err(Into::into)
}

struct Replies {
    status: GatedCommandResult,
    refused: GatedCommandResult,
    running: GatedCommandResult,
    stopped: GatedCommandResult,
    expected: String,
}

fn captured_replies() -> Result<Replies, Box<dyn std::error::Error>> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing P15 live capture directory")?;
    let root = std::path::Path::new(&root);
    let (session, status) = captured_result(root, "p15-status.json")?;
    let (refusal_session, refused) = captured_result(root, "p15-refused.json")?;
    let (running_session, running) = captured_result(root, "p15-running.json")?;
    let (stop_session, stopped) = captured_result(root, "p15-stopped.json")?;
    assert_ne!(session, [0; 16]);
    assert_eq!(session, refusal_session);
    assert_eq!(session, running_session);
    assert_eq!(session, stop_session);
    let replies = Replies {
        status,
        refused,
        running,
        stopped,
        expected: std::fs::read_to_string(root.join("p15-expected.txt"))?,
    };
    for (before, after) in [
        (&replies.status, &replies.refused),
        (&replies.refused, &replies.running),
        (&replies.running, &replies.stopped),
    ] {
        assert!(after.command_id > before.command_id);
        assert!(after.server_sequence > before.server_sequence);
    }
    Ok(replies)
}

fn fixture_replies() -> Result<Replies, Box<dyn std::error::Error>> {
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
        status: result(CommandStatus::Accepted, &encode(2, 5, 500)?),
        refused: result(CommandStatus::Rejected, &encode(4, 0, 0)?),
        running: result(CommandStatus::Accepted, &encode(1, 6, 500)?),
        stopped: result(CommandStatus::Accepted, &encode(2, 7, 499)?),
        expected: "status_disposition=2\nstatus_charges=500\nrefused_disposition=4\nrunning_disposition=1\nrunning_charges=500\nstopped_disposition=2\nstopped_charges=499\nrestored_charges=499\n".into(),
    })
}

fn captured_result(
    root: &std::path::Path,
    name: &str,
) -> Result<([u8; 16], GatedCommandResult), Box<dyn std::error::Error>> {
    let (session, command_id, server_sequence, status, payload): (
        [u8; 16],
        u64,
        u64,
        u32,
        Vec<u8>,
    ) = serde_json::from_slice(&std::fs::read(root.join(name))?)?;
    if command_id == 0 || server_sequence == 0 {
        return Err(format!("invalid captured sequence in {name}").into());
    }
    let status = match status {
        1 => CommandStatus::Accepted,
        2 => CommandStatus::Rejected,
        _ => return Err(format!("unexpected captured command status in {name}").into()),
    };
    Ok((
        session,
        GatedCommandResult {
            command_id,
            server_sequence,
            status,
            payload,
        },
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
fn p15_applies_fixture_exercise_training_replies() -> Result<(), Box<dyn std::error::Error>> {
    assert_training_replies(fixture_replies()?, true)
}

#[test]
#[ignore = "requires a fresh P15.client live capture"]
fn e2e_p15_replays_exercise_training_replies() -> Result<(), Box<dyn std::error::Error>> {
    assert_training_replies(captured_replies()?, false)
}

fn assert_training_replies(
    replies: Replies,
    exercise_invalid_inputs: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(replies.status.status, CommandStatus::Accepted);
    assert_eq!(replies.refused.status, CommandStatus::Rejected);
    assert_eq!(replies.running.status, CommandStatus::Accepted);
    assert_eq!(replies.stopped.status, CommandStatus::Accepted);
    assert_eq!(
        adapter::training_result(&replies.refused.payload),
        Some(wire::TrainingResult::REFUSED)
    );
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
    apply(&mut stopped_status, &status, &replies.status);
    assert_eq!(
        stopped_status.feedback,
        "Exercise training stopped or settled"
    );
    assert_eq!(stopped_status.charges, Some(status_charges));
    assert_eq!(stopped_status.pool_seconds, Some(0));
    assert_eq!(stopped_status.summary, summary(status_charges));

    // Refused START leaves the previously observed state intact.
    assert_eq!(value(&replies.expected, "refused_disposition")?, 4);
    let mut refused = stopped_status.clone();
    refused.pending = true;
    apply(&mut refused, &start, &replies.refused);
    assert!(!refused.pending);
    assert_eq!(refused.feedback, "The server refused the training request");
    assert_eq!(refused.charges, stopped_status.charges);
    assert_eq!(refused.pool_seconds, stopped_status.pool_seconds);
    assert_eq!(refused.summary, stopped_status.summary);

    // START of a carried weapon on an authored dummy: running with the weapon's charges.
    let running_charges = value(&replies.expected, "running_charges")?;
    assert_eq!(value(&replies.expected, "running_disposition")?, 1);
    let mut running = panel.clone();
    apply(&mut running, &start, &replies.running);
    assert_eq!(running.feedback, "Exercise training is running");
    assert_eq!(running.charges, Some(running_charges));
    assert_eq!(running.summary, summary(running_charges));
    assert_eq!(
        panel_text(&running),
        format!(
            "Training\r\n\r\n{}\r\nExercise training is running",
            summary(running_charges)
        )
    );

    // STOP settles the run; the charges never exceed the running count.
    let stopped_charges = value(&replies.expected, "stopped_charges")?;
    assert_eq!(value(&replies.expected, "stopped_disposition")?, 2);
    assert_eq!(Some(stopped_charges), running_charges.checked_sub(1));
    assert_eq!(
        value(&replies.expected, "restored_charges")?,
        stopped_charges
    );
    let mut stopped = running.clone();
    stopped.pending = true;
    apply(&mut stopped, &stop, &replies.stopped);
    assert_eq!(stopped.feedback, "Exercise training stopped or settled");
    assert_eq!(stopped.charges, Some(stopped_charges));
    assert_eq!(stopped.summary, summary(stopped_charges));

    if exercise_invalid_inputs {
        assert_invalid_inputs(panel, running, running_charges, &start, &replies.running);
    }
    Ok(())
}

fn assert_invalid_inputs(
    panel: Panel,
    running: Panel,
    running_charges: u32,
    start: &oteryn_client_runtime::raw_feature::FeatureCommand,
    reply: &GatedCommandResult,
) {
    // A rejected status and malformed payloads leave the charges and summary as they were.
    for (code, payload) in [
        (CommandStatus::Rejected, reply.payload.as_slice()),
        (
            CommandStatus::Accepted,
            &reply.payload[..reply.payload.len() - 1],
        ),
        (CommandStatus::Accepted, &[]),
    ] {
        let mut invalid = running.clone();
        invalid.pending = true;
        apply(&mut invalid, start, &result(code, payload));
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
    apply(&mut other, start, reply);
    assert_eq!(other, before);
}
