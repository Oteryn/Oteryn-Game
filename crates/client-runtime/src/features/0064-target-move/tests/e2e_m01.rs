//! Replay the exact command results captured during one verified M01 journey.

use crate::target_move::{WalkDisposition, WalkResponse, project};
use oteryn_session::{CommandStatus, GatedCommandResult};
use std::path::Path;

type ReplayResult<T> = Result<T, Box<dyn std::error::Error>>;

fn read_result(bytes: &[u8]) -> ReplayResult<GatedCommandResult> {
    let split = bytes
        .iter()
        .position(|&byte| byte == b'\n')
        .ok_or("missing M01 receipt header")?;
    let header = std::str::from_utf8(&bytes[..split])?;
    let fields: Vec<_> = header.split_ascii_whitespace().collect();
    let [command_id, status, sequence] = fields.as_slice() else {
        return Err("malformed M01 receipt header".into());
    };
    let status = match *status {
        "1" => CommandStatus::Accepted,
        "2" => CommandStatus::Rejected,
        "3" => CommandStatus::DuplicateReplay,
        "4" => CommandStatus::DuplicateOutcomeExpired,
        _ => return Err("unknown M01 status".into()),
    };
    Ok(GatedCommandResult {
        command_id: command_id.parse()?,
        status,
        server_sequence: sequence.parse()?,
        payload: bytes[split + 1..].to_vec(),
    })
}

fn replay(root: &Path, run: &str, batch: &str) -> ReplayResult<()> {
    if run.is_empty() || run.contains(char::is_whitespace) || batch != "128:1" {
        return Err("M01 replay requires a run identity and batch 128:1".into());
    }
    if std::fs::read_to_string(root.join("m01-run"))? != format!("M01.client {batch} {run}\n") {
        return Err("M01 capture run or batch mismatch".into());
    }
    if std::fs::read(root.join("m01-complete"))? != b"M01.client\n" {
        return Err("M01 live journey did not complete".into());
    }
    let mut previous_id = 0;
    let mut previous_sequence = 0;
    for name in [
        "autowalk",
        "interrupt-walk",
        "stop",
        "turn-east",
        "turn-south",
        "turn-west",
        "turn-north",
        "follow",
    ] {
        let result = read_result(&std::fs::read(root.join(format!("m01-{name}")))?)?;
        assert!(result.command_id > previous_id, "{name} command identity");
        assert!(
            result.server_sequence > previous_sequence,
            "{name} server sequence"
        );
        assert_eq!(
            project(&result)?,
            WalkResponse {
                status: CommandStatus::Accepted,
                disposition: Some(WalkDisposition::Accepted),
            },
            "{name}"
        );
        previous_id = result.command_id;
        previous_sequence = result.server_sequence;
    }
    Ok(())
}

#[test]
fn replays_m01_target_move_receipts() -> ReplayResult<()> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("M01 capture directory required")?;
    let run = std::env::var("M01_CAPTURE_RUN")?;
    let batch = std::env::var("OTERYN_E2E_ROW_BATCH")?;
    replay(Path::new(&root), &run, &batch)
}

#[test]
fn rejects_inconsistent_m01_receipt() -> ReplayResult<()> {
    let result = read_result(b"7 1 20\n\x08\x03")?;
    assert!(project(&result).is_err());
    Ok(())
}
