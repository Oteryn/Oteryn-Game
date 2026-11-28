//! Opt-in, inbound-only wire capture for the C20d qualification journey.
use super::*;
use std::{cell::RefCell, io::Cursor};

const MAX_CAPTURE_BYTES: usize = 32 * 1024 * 1024;

tokio::task_local! {
    static INBOUND: RefCell<Option<Vec<u8>>>;
}

/// Run a qualification journey with a bounded inbound frame recorder on this task.
pub async fn capture_inbound<T>(journey: impl Future<Output = T>) -> T {
    INBOUND.scope(RefCell::new(Some(Vec::new())), journey).await
}

pub(super) fn record(prefix: [u8; 4], body: &[u8]) {
    let _ = INBOUND.try_with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(bytes) = slot.as_mut() {
            if bytes.len() + prefix.len() + body.len() > MAX_CAPTURE_BYTES {
                *slot = None;
            } else {
                bytes.extend_from_slice(&prefix);
                bytes.extend_from_slice(body);
            }
        }
    });
}

/// Stop the recorder. An overflow invalidates the whole transcript.
pub fn finish_capture() -> Option<Vec<u8>> {
    INBOUND
        .try_with(|slot| slot.borrow_mut().take())
        .ok()
        .flatten()
}

/// Re-admit from the original wire baseline and validate the complete sequence through respawn.
/// Only the two domains under test are projected from the post-admission stream.
pub async fn replay_respawn(
    capture: Vec<u8>,
    admission: Admission<'_>,
    live_session: GameSessionId,
    final_sequence: u64,
    temple: world_spatial::ActorPosition,
    max_health: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if capture.is_empty() || capture.len() > MAX_CAPTURE_BYTES {
        return Err("invalid C20d capture length".into());
    }
    let mut session = Session::admit(
        tokio::io::join(Cursor::new(capture), tokio::io::sink()),
        admission,
    )
    .await?;
    if session.join_snapshot().game_session_id != live_session
        || session.world_spatial.actor_position == temple
        || !session
            .actor_vitals
            .is_some_and(|vitals| vitals.max_health == max_health)
    {
        return Err("C20d replay baseline does not match the live session".into());
    }
    let mut damaged = false;
    let mut returned = false;
    let mut first = [0];
    while session.stream.read(&mut first).await? != 0 {
        let frame = framing::read_frame_after(&mut session.stream, first[0]).await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        session.check_generation(&envelope)?;
        match envelope.message_type() {
            MessageType::LivenessProbe => {
                decode_liveness_probe(envelope.payload())?;
            }
            MessageType::CommandResult => {
                session.next_sequence(&envelope)?;
                decode_command_result(envelope.payload())?;
            }
            MessageType::StateDelta => {
                let sequence = session.next_sequence(&envelope)?;
                let delta = decode_state_delta(envelope.payload())?;
                if matches!(
                    delta.domain_id,
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
                        | actor_spell::STATE_DOMAIN_ACTOR_VITALS
                ) {
                    match session.apply_state_delta(sequence, envelope.payload())? {
                        SessionEvent::ActorVitals(delta) => {
                            damaged |= delta.value.health < delta.value.max_health
                                && session.world_spatial.actor_position != temple;
                        }
                        SessionEvent::WorldSpatial(delta) => {
                            returned |= damaged && delta.value.actor_position == temple;
                        }
                        _ => return Err("unexpected C20d replay projection".into()),
                    }
                }
            }
            _ => return Err("unexpected frame in C20d replay".into()),
        }
    }
    if !returned
        || session.last_server_sequence != final_sequence
        || session.world_spatial.actor_position != temple
        || !session
            .actor_vitals
            .is_some_and(|v| v.health == max_health && v.max_health == max_health)
    {
        return Err("C20d replay did not reproduce the live respawn".into());
    }
    Ok(())
}
