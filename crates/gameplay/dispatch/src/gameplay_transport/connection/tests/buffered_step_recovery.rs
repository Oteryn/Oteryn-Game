use super::*;
use crate::gameplay_transport::session_ingress::SessionCommands;

struct OwnedPacedAuthority {
    movement: PacedAuthority,
    commands: SessionCommands,
    pause_second_step: Option<tokio::sync::Notify>,
}

impl FreshAdmissionAuthority for OwnedPacedAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    fn session_commands(&self) -> Option<&SessionCommands> {
        Some(&self.commands)
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.movement.observe(actor).await
    }

    fn step_wait_receipts(&self) -> bool {
        true
    }

    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        let second_step = self.movement.steps.borrow().len() == 1;
        if second_step && let Some(entered) = &self.pause_second_step {
            entered.notify_one();
            return std::future::pending().await;
        }
        self.movement.paced_step(actor, session, direction).await
    }
}

#[test]
fn owned_buffered_step_disconnect_replays_rejection_and_resumes_next_command()
-> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let authority = OwnedPacedAuthority {
                    movement: PacedAuthority {
                        steps: RefCell::new(Vec::new()),
                        turn: std::time::Duration::ZERO,
                        receipts: true,
                    },
                    commands: SessionCommands::default(),
                    pause_second_step: None,
                };
                let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
                let (end, frames) = drive_session(
                    &authority,
                    paced_session(&[13])?,
                    &[
                        command(1, 1, step, StepDirection::East),
                        command(1, 2, step, StepDirection::East),
                    ],
                )
                .await?;
                let ConnectionEnd::AdmittedThenDisconnected(mut ended) = end else {
                    return Err(format!("unexpected end {end:?}").into());
                };
                assert_eq!(frames.len(), baseline().len() + 2);
                assert_eq!(authority.movement.steps.borrow().len(), 1);
                assert_eq!(
                    (
                        ended.continuity.next_command_id,
                        ended.continuity.server_sequence
                    ),
                    (3, 2)
                );
                authority
                    .commands
                    .recover_trade_continuity(&mut ended)
                    .ok_or("recover buffered cancellation")?;
                assert!(authority.commands.resumable(&ended));
                assert_eq!(
                    (
                        ended.continuity.next_command_id,
                        ended.continuity.server_sequence
                    ),
                    (3, 3)
                );
                ended.continuity.connection_generation = 2;
                let (_, frames) = drive_open(
                    &authority,
                    ended,
                    &[
                        command(2, 2, step, StepDirection::East),
                        command(2, 3, step, StepDirection::East),
                    ],
                    3,
                )
                .await?;
                let replay = frames
                    .get(baseline().len())
                    .ok_or("retained cancellation replay")?;
                assert_eq!(
                    replay,
                    &encode_command_result(
                        2,
                        4,
                        2,
                        CommandStatus::DuplicateReplay,
                        &encode_step_result(StepDisposition::Rejected)
                    )?
                );
                assert_eq!(
                    authority.movement.steps.borrow().len(),
                    2,
                    "the buffered original never executes"
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "buffered step regression thread panicked")?
        .map_err(Into::into)
}

#[test]
fn owned_step_taken_from_buffer_stays_reserved_when_dispatch_is_cancelled()
-> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let authority = OwnedPacedAuthority {
                    movement: PacedAuthority {
                        steps: RefCell::new(Vec::new()),
                        turn: std::time::Duration::ZERO,
                        receipts: true,
                    },
                    commands: SessionCommands::default(),
                    pause_second_step: Some(tokio::sync::Notify::new()),
                };
                let mut admitted = paced_session(&[13])?;
                let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
                let (mut server, mut client) = tokio::io::duplex(1 << 21);
                for id in [1, 2] {
                    client.write_all(&framed(&command(1, id, step, StepDirection::East))).await?;
                }
                let mut served = Box::pin(serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS));
                let entered = authority.pause_second_step.as_ref().ok_or("step barrier")?;
                tokio::select! {
                    end = &mut served => return Err(format!("connection ended before step owner: {end:?}").into()),
                    () = entered.notified() => {},
                    () = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        return Err("buffered step never entered its movement owner".into());
                    }
                }
                drop(served);
                let pending = crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(2)?,
                );
                assert!(authority.commands.pending_reconciliation(pending));
                assert!(!authority.commands.original_terminalized(pending));
                admitted.continuity.next_command_id = 3;
                admitted.continuity.server_sequence = 2;
                authority.commands.recover_trade_continuity(&mut admitted).ok_or("unchanged unknown continuity")?;
                assert!(!authority.commands.resumable(&admitted));
                assert_eq!(admitted.continuity.server_sequence, 2);
                assert_eq!(authority.movement.steps.borrow().len(), 1);
                Ok(())
            }).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "in-flight step regression thread panicked")?
        .map_err(Into::into)
}
