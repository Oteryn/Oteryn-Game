//! Qualification-only inbound capture and ordered vitals replay.
use super::*;
use oteryn_protocol_oteryn::actor_spell::{ActorVitals, STATE_DOMAIN_ACTOR_VITALS};
use std::cell::RefCell;
use std::future::Future;
use std::io::Cursor;

const MAX_CAPTURE_BYTES: usize = 64 * 1024 * 1024;

tokio::task_local! {
    static INBOUND: RefCell<Vec<u8>>;
}

/// Record exactly the length prefixes and inbound frame bodies consumed by this future.
pub async fn capture_inbound<T>(future: impl Future<Output = T>) -> (T, Vec<u8>) {
    INBOUND
        .scope(RefCell::new(Vec::new()), async {
            let output = future.await;
            (output, INBOUND.with(|bytes| bytes.take()))
        })
        .await
}

pub(crate) fn record(prefix: &[u8; 4], body: &[u8]) -> io::Result<()> {
    INBOUND
        .try_with(|bytes| {
            let mut bytes = bytes.borrow_mut();
            if bytes.len() + prefix.len() + body.len() > MAX_CAPTURE_BYTES {
                return Err(io::Error::other(
                    "qualification capture exceeded its byte limit",
                ));
            }
            bytes.extend_from_slice(prefix);
            bytes.extend_from_slice(body);
            Ok(())
        })
        .unwrap_or(Ok(()))
}

/// Re-admit from the recorded join and replay the exact vitals envelopes in sequence.
pub async fn replay_vitals(
    bytes: &[u8],
    admission: Admission<'_>,
    game_session_id: GameSessionId,
    checkpoints: &[(u64, ActorVitals)],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if bytes.len() > MAX_CAPTURE_BYTES {
        return Err("oversize capture".into());
    }
    let mut wire = Cursor::new(bytes);
    let mut start = None;
    let mut join_end = None;
    while (wire.position() as usize) < bytes.len() {
        let offset = wire.position() as usize;
        let body = read_frame(&mut wire).await?;
        let frame = decode_wire_envelope(&body)?;
        if frame.message_type() == MessageType::ServerAccepted {
            let accepted = decode_server_accepted(frame.payload())?;
            start = (accepted.game_session_id == game_session_id).then_some(offset);
            join_end = None;
        } else if start.is_some() && frame.message_type() == MessageType::SnapshotCommit {
            join_end = Some(wire.position() as usize);
            break;
        }
    }
    let start = start.ok_or("capture missing the matching session admission")?;
    let join_end = join_end.ok_or("capture missing the matching session snapshot")?;
    let stream = tokio::io::join(Cursor::new(&bytes[start..join_end]), tokio::io::sink());
    let mut session = Session::admit(stream, admission).await?;
    let mut checkpoints = checkpoints.iter().peekable();
    while (wire.position() as usize) < bytes.len() {
        let body = read_frame(&mut wire).await?;
        let frame = decode_wire_envelope(&body)?;
        frame.validate(Direction::ServerToClient, true)?;
        session.check_generation(&frame)?;
        match frame.message_type() {
            MessageType::LivenessProbe => continue,
            MessageType::CommandResult | MessageType::StateDelta => {}
            _ => return Err("unexpected frame during vitals replay".into()),
        }
        let sequence = session.next_sequence(&frame)?;
        if frame.message_type() == MessageType::StateDelta
            && decode_state_delta(frame.payload())?.domain_id == STATE_DOMAIN_ACTOR_VITALS
        {
            session.apply_state_delta(sequence, frame.payload())?;
        }
        if let Some((expected_sequence, expected)) = checkpoints.peek()
            && sequence >= *expected_sequence
        {
            if sequence != *expected_sequence || session.actor_vitals() != Some(expected) {
                return Err(format!(
                    "vitals replay disagreed at server sequence {expected_sequence}"
                )
                .into());
            }
            checkpoints.next();
        }
    }
    if checkpoints.next().is_some() {
        return Err("capture ended before all vitals checkpoints".into());
    }
    Ok(())
}
