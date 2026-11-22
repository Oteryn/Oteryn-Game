//! Keep the admitted connection alive while its final spell participant retries.
use super::*;
use std::task::Poll;

enum PendingSpellEvent {
    Probe,
    Repaired(bool),
    Frame(std::io::Result<Vec<u8>>),
}

pub(super) async fn wait<S, A>(
    stream: &mut S,
    admitted: &mut AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
) -> Result<(FrameReader, Liveness, tokio::time::Interval), ServeEnd>
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
    let deadline =
        tokio::time::Instant::now() + policy.interval.saturating_mul(policy.missed_limit);
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
                        record_join_progress("spell_repair_returned_false", admitted.first_entry);
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
    match tokio::time::timeout_at(deadline, waiting).await {
        Ok(Ok(())) => {
            admitted.first_entry = FirstEntryOutcome::Reconciled;
            // Preserve outstanding probe IDs and partial frame bytes across promotion.
            Ok((frames, liveness, cadence))
        }
        Ok(Err(ServeEnd::Closed(error))) => Err(close(stream, *admitted, error).await),
        Ok(Err(end)) => Err(end),
        Err(_) => {
            record_join_progress("spell_repair_deadline_expired", admitted.first_entry);
            Err(close(stream, *admitted, FoundationProtocolError::NotSupported).await)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, time::Duration};

    struct RepairAuthority {
        stall: bool,
    }

    impl FreshAdmissionAuthority for RepairAuthority {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn repair_pending_spells(&self, _: AdmittedSession) -> bool {
            if self.stall {
                std::future::pending().await
            } else {
                false
            }
        }
    }

    #[test]
    fn pending_spell_repair_reports_distinct_e1200_close_phases() -> Result<(), Box<dyn Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| exercise_repair_closes().map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "spell repair connection thread panicked")??;
        Ok(())
    }

    fn exercise_repair_closes() -> Result<(), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?;
        let fixture_id = |tag| {
            let mut bytes = [tag; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let world_id = WorldId::decode(&fixture_id(1))?;
        let channel_id = ChannelId::decode(&fixture_id(2))?;
        let admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&fixture_id(3))?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::AwaitingSpellRepair,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let policy = LivenessPolicy {
            interval: Duration::from_millis(20),
            missed_limit: 1,
        };
        let error = FoundationProtocolError::NotSupported;
        let expected_wire =
            encode_protocol_error(error, admitted.continuity.connection_generation)?;
        for (stall, expected_phase) in [
            (false, "spell_repair_returned_false"),
            (true, "spell_repair_deadline_expired"),
        ] {
            let mut result = Ok(());
            let lines = oteryn_error_codes::capture(|| {
                result = runtime.block_on(async {
                    let (mut server, mut client) = tokio::io::duplex(4096);
                    let end = serve_admitted_actor(
                        &mut server,
                        admitted,
                        &RepairAuthority { stall },
                        policy,
                    )
                    .await;
                    assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, closed) if closed == error));
                    drop(server);
                    // The deadline case may write one probe before closing the connection.
                    let mut frame = read_frame(&mut client).await?;
                    if decode_wire_envelope(&frame)?.message_type() == MessageType::LivenessProbe {
                        frame = read_frame(&mut client).await?;
                    }
                    assert_eq!(frame, expected_wire);
                    Ok::<(), Box<dyn Error>>(())
                });
            });
            result?;
            let phases: Vec<_> = lines
                .iter()
                .filter(|line| line.contains("event=join_progress "))
                .filter_map(|line| {
                    line.split_whitespace()
                        .find_map(|field| field.strip_prefix("phase="))
                })
                .collect();
            assert_eq!(phases, [expected_phase], "{lines:?}");
        }
        Ok(())
    }
}
