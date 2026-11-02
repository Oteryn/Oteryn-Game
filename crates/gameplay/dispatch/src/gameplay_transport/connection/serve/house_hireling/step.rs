//! Existing SPEED-1 pacing and bounded ingress while the actual House owns the actor.
use super::*;
#[path = "logout.rs"]
mod logout;

pub(in crate::gameplay_transport::connection::serve) enum Event {
    Input(inbound::Input),
    Probe,
    Services,
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};

    #[test]
    fn hireling_house_step_buffer_keeps_original_and_bounds_later_wire_frames()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?
            .block_on(async {
                let frame = |id| {
                    encode_client_command(
                        1,
                        &ClientCommandValue {
                            command_id: id,
                            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                            payload: &oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                                StepDirection::East,
                            ),
                        },
                    )
                };
                let (mut server, mut client) = tokio::io::duplex(1 << 20);
                let mut queue = Queue::default();
                let due = tokio::time::Instant::now() + std::time::Duration::from_millis(50);
                let original = frame(1)?;
                queue.buffer(original.clone(), due);
                for id in 2..=65 {
                    write_frame(&mut client, &frame(id)?).await?;
                }
                let mut cadence = tokio::time::interval_at(
                    due + std::time::Duration::from_secs(1),
                    std::time::Duration::from_secs(1),
                );
                let mut services = tokio::time::interval_at(
                    due + std::time::Duration::from_secs(1),
                    std::time::Duration::from_secs(1),
                );
                let Event::Input(input) = queue
                    .next(&mut server, 1, &mut cadence, &mut services)
                    .await
                    .map_err(|_| "House ingress")?
                else {
                    return Err("buffered step must run before cadence".into());
                };
                assert_eq!(input.frame, original);
                assert!(input.buffered_original);
                assert!(!input.arrived_while_buffered);
                assert_eq!(tokio::time::Instant::now(), due);
                assert_eq!(queue.held.len(), MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP);
                for id in 2..=65 {
                    let Event::Input(input) = queue
                        .next(&mut server, 1, &mut cadence, &mut services)
                        .await
                        .map_err(|_| "House held ingress")?
                    else {
                        return Err("held command must retain order".into());
                    };
                    assert_eq!(input.frame, frame(id)?);
                    assert!(input.arrived_while_buffered);
                    assert!(!input.buffered_original);
                }
                Ok(())
            })
    }
}

#[derive(Default)]
pub(in crate::gameplay_transport::connection::serve) struct Queue {
    frames: FrameReader,
    buffered: Option<BufferedStep>,
    held: std::collections::VecDeque<(Vec<u8>, bool)>,
}

impl Queue {
    pub(in crate::gameplay_transport::connection::serve) fn buffer(
        &mut self,
        frame: Vec<u8>,
        due: tokio::time::Instant,
    ) {
        self.buffered = Some(BufferedStep { frame, due });
    }

    pub(in crate::gameplay_transport::connection::serve) async fn next<S: AsyncRead + Unpin>(
        &mut self,
        stream: &mut S,
        generation: u64,
        cadence: &mut tokio::time::Interval,
        services: &mut tokio::time::Interval,
    ) -> Result<Event, ServeEnd> {
        enum Ready {
            Frame(Result<Vec<u8>, ServeEnd>),
            Due,
            Held,
            Probe,
            Services,
        }
        loop {
            let mut due = self
                .buffered
                .as_ref()
                .map(|step| Box::pin(tokio::time::sleep_until(step.due)));
            let held_ready = self.buffered.is_none() && !self.held.is_empty();
            let reading = self.held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
            let ready = {
                let mut read = std::pin::pin!(self.frames.next(stream));
                std::future::poll_fn(|cx| {
                    if cadence.poll_tick(cx).is_ready() {
                        return std::task::Poll::Ready(Ready::Probe);
                    }
                    if services.poll_tick(cx).is_ready() {
                        return std::task::Poll::Ready(Ready::Services);
                    }
                    if due
                        .as_mut()
                        .is_some_and(|due| due.as_mut().poll(cx).is_ready())
                    {
                        return std::task::Poll::Ready(Ready::Due);
                    }
                    if held_ready {
                        return std::task::Poll::Ready(Ready::Held);
                    }
                    if reading {
                        return read
                            .as_mut()
                            .poll(cx)
                            .map(|frame| Ready::Frame(frame.map_err(|_| ServeEnd::Disconnected)));
                    }
                    std::task::Poll::Pending
                })
                .await
            };
            let (frame, arrived_while_buffered, buffered_original) = match ready {
                Ready::Probe => return Ok(Event::Probe),
                Ready::Services => return Ok(Event::Services),
                Ready::Due => {
                    let step = self.buffered.take().ok_or(ServeEnd::Disconnected)?;
                    (step.frame, false, true)
                }
                Ready::Held => {
                    let (frame, behind) = self.held.pop_front().ok_or(ServeEnd::Disconnected)?;
                    (frame, behind, false)
                }
                Ready::Frame(frame) => {
                    let frame = frame?;
                    if self.buffered.is_some()
                        && !matches!(decode_wire_envelope(&frame),
                        Ok(envelope) if envelope.connection_generation() == generation
                            && envelope.message_type() == MessageType::LivenessAck)
                    {
                        self.held.push_back((frame, true));
                        continue;
                    }
                    (frame, false, false)
                }
            };
            return Ok(Event::Input(inbound::Input {
                frame,
                arrived_while_buffered,
                buffered_original,
            }));
        }
    }
}

impl State {
    pub(in crate::gameplay_transport::connection::serve) async fn step<
        A: FreshAdmissionAuthority,
    >(
        &mut self,
        authority: &A,
        admitted: &AdmittedSession,
        command: oteryn_protocol_oteryn::ClientCommandView<'_>,
        arrived_while_buffered: bool,
    ) -> route::StepAction {
        let dispatch = match decode_step_intent(command.payload) {
            Err(_) => Dispatch::Step(StepOutcome::rejected(), 0),
            Ok(_) if arrived_while_buffered => {
                let now = tokio::time::Instant::now();
                let left = self
                    .pacer
                    .wait_until(now)
                    .map(|due| due.duration_since(now));
                Dispatch::Step(
                    StepOutcome::too_early(),
                    if self.step_wait_receipts {
                        too_early_step_wait_ms(left)
                    } else {
                        0
                    },
                )
            }
            Ok(direction) => {
                authority
                    .yield_chase_to_client(self.actor, admitted.game_session_id)
                    .await;
                let wait = match self.pacer.wait_until(tokio::time::Instant::now()) {
                    Some(due) => Some(due),
                    None => {
                        begin_actor_step(&mut self.pacer, authority, self.actor, &self.step_holder)
                            .await
                    }
                };
                if let Some(due) = wait {
                    return route::StepAction::Wait(due);
                }
                let (outcome, duration) = authority
                    .paced_step(self.actor, admitted.game_session_id, direction)
                    .await;
                record_actor_step(
                    &mut self.pacer,
                    authority,
                    self.actor,
                    &self.step_holder,
                    duration,
                )
                .await;
                let wait =
                    if self.step_wait_receipts && outcome.disposition == StepDisposition::Moved {
                        moved_step_wait_ms(duration)
                    } else {
                        0
                    };
                Dispatch::Step(outcome, wait)
            }
        };
        route::StepAction::Dispatch(dispatch)
    }
}
