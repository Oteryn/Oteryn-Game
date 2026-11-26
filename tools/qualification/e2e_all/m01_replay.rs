//! Shared M01 capture reader for the live qualification row and native replay regression tests.
use super::{CommandStatus, GatedCommandResult, WalkDisposition, WalkResponse, project};
use std::path::Path;

pub(super) fn selects_m01(batch: &str) -> bool {
    if batch == "all" {
        return true;
    }
    batch.split_once(':').is_some_and(|(start, len)| {
        start
            .parse::<usize>()
            .ok()
            .zip(len.parse::<usize>().ok())
            .is_some_and(|(start, len)| start <= 128 && 128 - start < len)
    })
}

type ReplayResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn read_result(bytes: &[u8], session: &str) -> ReplayResult<GatedCommandResult> {
    let split = bytes
        .iter()
        .position(|&byte| byte == b'\n')
        .ok_or("missing M01 receipt header")?;
    let header = std::str::from_utf8(&bytes[..split])?;
    let fields: Vec<_> = header.split_ascii_whitespace().collect();
    let [recorded_session, command_id, status, sequence] = fields.as_slice() else {
        return Err("malformed M01 receipt header".into());
    };
    if *recorded_session != session {
        return Err("M01 receipt session mismatch".into());
    }
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

pub(super) fn replay(root: &Path, run: &str, batch: &str, session: &str) -> ReplayResult<()> {
    if run.is_empty() || run.contains(char::is_whitespace) || !selects_m01(batch) {
        return Err("M01 replay requires a run identity and a batch selecting M01.client".into());
    }
    if std::fs::read_to_string(root.join("m01-run"))?
        != format!("M01.client {batch} {run} {session}\n")
    {
        return Err("M01 capture run, batch, or session mismatch".into());
    }
    if std::fs::read(root.join("m01-complete"))? != b"M01.client\n" {
        return Err("M01 live journey did not complete".into());
    }
    let mut previous_id = 0;
    let mut previous_sequence = 0;
    for name in [
        "autowalk",
        "return-walk",
        "interrupt-walk",
        "stop",
        "turn-east",
        "turn-south",
        "turn-west",
        "turn-north",
        "follow",
    ] {
        let result = read_result(&std::fs::read(root.join(format!("m01-{name}")))?, session)?;
        if result.command_id <= previous_id || result.server_sequence <= previous_sequence {
            return Err(
                format!("M01 {name} did not advance command identity and server sequence").into(),
            );
        }
        let expected = WalkResponse {
            status: CommandStatus::Accepted,
            disposition: Some(WalkDisposition::Accepted),
        };
        if project(&result)? != expected {
            return Err(format!("M01 {name} did not replay as Accepted").into());
        }
        previous_id = result.command_id;
        previous_sequence = result.server_sequence;
    }
    Ok(())
}
