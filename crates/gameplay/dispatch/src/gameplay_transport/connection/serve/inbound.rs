//! Resolve the selected event to a frame, or complete its refresh/probe.

use super::*;

use super::poll::Next;

pub(super) struct Input {
    pub(super) frame: Vec<u8>,
    pub(super) arrived_while_buffered: bool,
    pub(super) buffered_original: bool,
}

pub(super) async fn receive<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    next: Next,
) -> Result<Option<Input>, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let buffered_original = matches!(&next, Next::StepDue);
    let (frame, arrived_while_buffered) = match next {
        // SPEED-1: the buffered step is due; its frame runs now, before any frame held
        // behind it.
        Next::StepDue => match session.buffered.take() {
            Some(step) => (step.frame, false),
            None => return Ok(None),
        },
        Next::Held(frame, behind_step) => (frame, behind_step),
        Next::Serene => {
            refresh::serene(stream, authority, session).await?;
            return Ok(None);
        }
        Next::QuestLog => {
            refresh::quest_log(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Chat => {
            refresh::chat(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Npc => {
            refresh::npc(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Combat => {
            refresh::combat(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Chase => {
            refresh::chase(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Visibility => {
            refresh::visibility(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Frame(Ok(frame)) if session.buffered.is_some() => {
            // SPEED-1: read while a step waits in the buffer. A liveness ack is not a command
            // and is answered at once; anything else waits for the buffered step's result.
            let ack = matches!(
                decode_wire_envelope(&frame),
                Ok(envelope) if envelope.connection_generation() == session.generation
                    && envelope.message_type() == MessageType::LivenessAck
            );
            if !ack {
                session.held.push_back((frame, true));
                return Ok(None);
            }
            match decode_wire_envelope(&frame)
                .and_then(|envelope| envelope.liveness_ack(session.generation))
                .and_then(|ack| session.liveness.ack(ack.probe_id))
            {
                Ok(()) => return Ok(None),
                Err(error) => return Err(close(stream, session.admitted, error).await),
            }
        }
        Next::Frame(Ok(frame)) => (frame, false),
        Next::Frame(Err(_)) => {
            return Err(ServeEnd::Disconnected);
        }
        Next::Probe => {
            let probe = match session.liveness.tick() {
                Ok(probe_id) => encode_liveness_probe(session.generation, probe_id),
                Err(LivenessEnd::ControlLost) => {
                    return Err(ServeEnd::ControlLost);
                }
                Err(LivenessEnd::ProbesExhausted) => {
                    return Err(ServeEnd::Disconnected);
                }
            };
            let Ok(probe) = probe else {
                return Err(ServeEnd::Disconnected);
            };
            // A peer that does not consume even one small probe within a cadence is not
            // in control; an unbounded write would also stall the cadence itself.
            match tokio::time::timeout(session.policy.interval, write_frame(stream, &probe)).await {
                Ok(Ok(())) => return Ok(None),
                Ok(Err(_)) => {
                    return Err(ServeEnd::Disconnected);
                }
                Err(_) => return Err(ServeEnd::ControlLost),
            }
        }
    };
    Ok(Some(Input {
        frame,
        arrived_while_buffered,
        buffered_original,
    }))
}
