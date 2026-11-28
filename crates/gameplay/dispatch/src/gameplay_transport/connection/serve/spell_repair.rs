//! Keep the admitted connection alive while its final spell participant retries.
use super::*;
use std::task::Poll;

enum PendingSpellEvent {
    Probe,
    Repaired(bool),
    Frame(std::io::Result<Vec<u8>>),
}

pub(super) type RepairedIo = (FrameReader, Liveness, tokio::time::Interval);

pub(super) async fn wait<S, A>(
    stream: &mut S,
    admitted: &mut AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
    join_deadline: Option<tokio::time::Instant>,
) -> Result<RepairedIo, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let generation = admitted.continuity.connection_generation;
    let mut frames = FrameReader::default();
    let mut liveness = Liveness::new(policy);
    let mut cadence = tokio::time::interval_at(
        tokio::time::Instant::now() + policy.interval,
        policy.interval,
    );
    cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Retain the existing refused-actor bound, including retries and wire writes.
    let liveness_deadline =
        tokio::time::Instant::now() + policy.interval.saturating_mul(policy.missed_limit);
    let deadline = join_deadline.map_or(liveness_deadline, |join| join.min(liveness_deadline));
    let waiting = async {
        let repair = authority.repair_pending_spells(*admitted);
        let mut repair = std::pin::pin!(repair);
        loop {
            let next = {
                let mut tick = std::pin::pin!(cadence.tick());
                let mut read = std::pin::pin!(frames.next(stream));
                // Match the live connection's priority: flooding ACKs cannot starve probes.
                std::future::poll_fn(|cx| {
                    if tick.as_mut().poll(cx).is_ready() {
                        return Poll::Ready(PendingSpellEvent::Probe);
                    }
                    if let Poll::Ready(repaired) = repair.as_mut().poll(cx) {
                        return Poll::Ready(PendingSpellEvent::Repaired(repaired));
                    }
                    read.as_mut().poll(cx).map(PendingSpellEvent::Frame)
                })
                .await
            };
            match next {
                PendingSpellEvent::Probe => {
                    let id = match liveness.tick() {
                        Ok(id) => id,
                        Err(LivenessEnd::ControlLost) => {
                            return Err(ServeEnd::ControlLost("spell_repair_liveness"));
                        }
                        Err(LivenessEnd::ProbesExhausted) => return Err(ServeEnd::Disconnected),
                    };
                    let probe = encode_liveness_probe(generation, id)
                        .map_err(|_| ServeEnd::Disconnected)?;
                    match tokio::time::timeout(policy.interval, write_frame(stream, &probe)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(_)) => return Err(ServeEnd::Disconnected),
                        Err(_) => return Err(ServeEnd::ControlLost("spell_repair_probe_timeout")),
                    }
                }
                PendingSpellEvent::Repaired(repaired) => {
                    return if repaired {
                        Ok(())
                    } else {
                        Err(ServeEnd::Closed(FoundationProtocolError::NotSupported))
                    };
                }
                PendingSpellEvent::Frame(frame) => {
                    let frame = frame.map_err(|_| ServeEnd::Disconnected)?;
                    decode_wire_envelope(&frame)
                        .and_then(|envelope| envelope.liveness_ack(generation))
                        .and_then(|ack| liveness.ack(ack.probe_id))
                        .map_err(ServeEnd::Closed)?;
                }
            }
        }
    };
    let result = match join_deadline {
        Some(join) => {
            tokio::time::timeout_at(
                deadline,
                crate::durability::with_holder_deadline(join, waiting),
            )
            .await
        }
        None => tokio::time::timeout_at(deadline, waiting).await,
    };
    if join_deadline
        .is_some_and(|join| join <= liveness_deadline && tokio::time::Instant::now() >= join)
    {
        send_join_unavailable(stream, generation).await;
        return Err(ServeEnd::Disconnected);
    }
    match result {
        Ok(Ok(())) => {
            admitted.first_entry = FirstEntryOutcome::Reconciled;
            // Preserve outstanding probe IDs and partial frame bytes across promotion.
            Ok((frames, liveness, cadence))
        }
        Ok(Err(ServeEnd::Closed(error))) => Err(close(stream, *admitted, error).await),
        Ok(Err(end)) => Err(end),
        Err(_) => Err(close(stream, *admitted, FoundationProtocolError::NotSupported).await),
    }
}
