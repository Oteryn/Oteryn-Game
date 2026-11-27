//! Resolve the selected event to a frame, or complete its refresh/probe.

use super::*;

use super::poll::Next;

pub(super) struct Input {
    pub(super) frame: Vec<u8>,
    pub(super) arrived_while_buffered: bool,
    pub(super) buffered_original: bool,
    pub(super) received_at: tokio::time::Instant,
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
    if matches!(
        next,
        Next::Serene | Next::QuestLog | Next::Chat | Next::Combat | Next::Npc
    ) {
        crate::durability::DurabilityRoot::with_background_holder(
            authority.background_operation(receive_inner(stream, authority, session, next)),
        )
        .await
    } else {
        receive_inner(stream, authority, session, next).await
    }
}

async fn receive_inner<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    next: Next,
) -> Result<Option<Input>, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    // Filter before a held queue or item-walk/hotkey retransmission peek can
    // treat old source-session bytes as a current successor operation.
    let incoming = match &next {
        Next::Frame(Ok(frame)) | Next::Held(frame, _, _) => Some(frame),
        _ => None,
    };
    if let Some(frame) = incoming {
        match command_session_matches(frame, session.admitted, session.require_session_binding) {
            Ok(false) => return Ok(None),
            Ok(true) => {}
            Err(error) => return Err(close(stream, session.admitted, error).await),
        }
    }
    let buffered_original = matches!(&next, Next::StepDue);
    let (frame, arrived_while_buffered, received_at) = match next {
        // SPEED-1: the buffered step is due; its frame runs now, before any frame held
        // behind it.
        Next::StepDue => match session.buffered.take() {
            Some(step) => (step.frame, false, tokio::time::Instant::now()),
            None => return Ok(None),
        },
        Next::Held(frame, behind_step, received_at) => (frame, behind_step, received_at),
        Next::ServerMessage => {
            return match tokio::time::timeout(
                session.policy.interval,
                zones_nologout_transport::drain(
                    stream,
                    authority,
                    &mut session.admitted,
                    &mut session.sequence,
                ),
            )
            .await
            {
                Ok(Some(())) => Ok(None),
                Ok(None) => Err(ServeEnd::server("serve_state_unavailable")),
                Err(_) => Err(ServeEnd::ControlLost("server_message_timeout")),
            };
        }
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
        Next::TargetMove => {
            target_move::tick(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Chase => {
            refresh::chase(stream, authority, session).await?;
            return Ok(None);
        }
        Next::Turn(turn) => {
            if turn.actor != session.actor
                && session
                    .visibility
                    .as_ref()
                    .is_some_and(|view| view.shows(turn.entity()))
                && authority.turn_is_visible(session.actor, turn).await
                && let Some(publication) = super::turn::prepare(session, turn)?
            {
                publication.write(stream, session).await?;
            }
            return Ok(None);
        }
        Next::Visibility => {
            refresh::visibility(stream, authority, session).await?;
            // Browse observations can outlast the visibility interval. Start the next
            // existing interval after completion so another ready refresh cannot keep
            // winning over buffered steps and client frames in the priority poll.
            if session
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1)
                && let Some(refresh) = session.visibility_refresh.as_mut()
            {
                refresh.reset();
            }
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
                session
                    .held
                    .push_back((frame, true, tokio::time::Instant::now()));
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
        Next::Frame(Ok(frame)) => (frame, false, tokio::time::Instant::now()),
        Next::Frame(Err(error)) => {
            return Err(ServeEnd::Transport(TransportEnd::Read(error.kind())));
        }
        Next::Probe => {
            // Idle work (including the durable logout) stays within the existing database
            // pass deadline. Timeout follows ordinary control-loss reconciliation.
            match tokio::time::timeout(crate::durability::DB_PASS_DEADLINE, async {
                if authority.account_closure_check(session.admitted).await {
                    return Err(ServeEnd::server("account_closed"));
                }
                idle::tick(stream, authority, session).await
            })
            .await
            {
                Ok(result) => result?,
                Err(_) => return Err(ServeEnd::server("idle_work_deadline")),
            }
            write_liveness_probe(stream, session).await?;
            // Existing periodic display work shares the liveness cadence. No independent
            // achievement timer or timestamp cursor is needed for committed kill/event facts.
            let interval = session.policy.interval;
            let updates = async {
                server_save_countdown_transport::emit(
                    stream,
                    authority,
                    &mut session.admitted,
                    &mut session.sequence,
                )
                .await
                .ok_or_else(|| ServeEnd::server("serve_state_unavailable"))?;
                super::party_membership::refresh(stream, authority, session)
                    .await
                    .ok_or_else(|| ServeEnd::server("serve_state_unavailable"))?;
                super::native_cues::refresh(stream, authority, session).await?;
                achievement_notices::refresh(stream, authority, session).await
            };
            return match tokio::time::timeout(interval, updates).await {
                Ok(Ok(())) => Ok(None),
                Ok(Err(end)) => Err(end),
                Err(_) => Err(ServeEnd::ControlLost("updates_timeout")),
            };
        }
    };
    Ok(Some(Input {
        frame,
        arrived_while_buffered,
        buffered_original,
        received_at,
    }))
}

/// Advance the probe and write it without taking any actor or durability turn.
pub(super) async fn write_liveness_probe<S: AsyncWrite + Unpin>(
    stream: &mut S,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    let probe_id = match session.liveness.tick() {
        Ok(probe_id) => probe_id,
        Err(LivenessEnd::ControlLost) => return Err(ServeEnd::ControlLost("liveness_missed")),
        Err(LivenessEnd::ProbesExhausted) => {
            return Err(ServeEnd::server("liveness_probe_unavailable"));
        }
    };
    let probe = encode_liveness_probe(session.generation, probe_id)
        .map_err(|_| ServeEnd::server("liveness_probe_unavailable"))?;
    // A peer that does not consume a probe within one cadence is no longer in control.
    match tokio::time::timeout(session.policy.interval, write_frame(stream, &probe)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(ServeEnd::Transport(TransportEnd::Write(error.kind()))),
        Err(_) => Err(ServeEnd::ControlLost("probe_write_timeout")),
    }
}

#[cfg(test)]
#[path = "inbound_tests.rs"]
mod tests;
