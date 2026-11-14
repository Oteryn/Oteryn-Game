//! NBREL3: a refused guard consumes only its reserved command, never the session.
use super::*;
use crate::durability::DurabilityError;

struct GuardAuthority {
    movement: StepAuthority,
    failure: Cell<Option<fn() -> DurabilityError>>,
    activity: bool,
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
}

impl GuardAuthority {
    fn guard(&self) -> Result<(), DurabilityError> {
        self.failure.take().map_or(Ok(()), |failure| Err(failure()))
    }
}

impl FreshAdmissionAuthority for GuardAuthority {
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn house_transfer_input(&self, _: AdmittedSession) -> Result<(), DurabilityError> {
        if self.activity { Ok(()) } else { self.guard() }
    }

    async fn activity_transfer_input(&self, _: AdmittedSession) -> Result<(), DurabilityError> {
        if self.activity { self.guard() } else { Ok(()) }
    }

    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        self.movement.paced_step(actor, session, direction).await
    }
}

fn guard_case(failure: fn() -> DurabilityError, transient: bool) -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async {
                for activity in [false, true] {
                    let authority = GuardAuthority {
                        movement: StepAuthority {
                            steps: RefCell::new(Vec::new()),
                            tutorial_notices: Default::default(),
                        },
                        failure: Cell::new(Some(failure)),
                        activity,
                        commands: Default::default(),
                    };
                    let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
                    let refused = command(1, 1, step, StepDirection::East);
                    let (end, frames) = drive_admitted(
                        &authority,
                        &[
                            refused.clone(),
                            refused,
                            command(1, 2, step, StepDirection::North),
                        ],
                    )
                    .await?;
                    assert!(
                        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                        "{end:?}"
                    );
                    let results: Vec<_> = frames
                        .iter()
                        .map(|frame| decode_wire_envelope(frame))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .filter(|envelope| envelope.message_type() == MessageType::CommandResult)
                        .map(|envelope| {
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if transient {
                        assert_eq!(
                            results.len(),
                            3,
                            "busy guard must keep the command stream open"
                        );
                        assert_eq!(results[0].status, CommandStatus::Rejected);
                        assert!(results[0].payload.is_empty());
                        assert_eq!(results[1].status, CommandStatus::DuplicateReplay);
                        assert_eq!(results[1].payload, results[0].payload);
                        assert_eq!(results[2].status, CommandStatus::Accepted);
                        assert_eq!(*authority.movement.steps.borrow(), [StepDirection::North]);
                    } else {
                        assert!(
                            results.is_empty(),
                            "fence mismatch must disconnect before a result"
                        );
                        assert!(authority.movement.steps.borrow().is_empty());
                    }
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "guard test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn house_transfer_guard_deadline_defers_and_replay_never_executes() -> Result<(), Box<dyn Error>> {
    guard_case(|| DurabilityError::RootPassDeadlineExceeded, true)
}

#[test]
fn house_transfer_guard_unavailable_defers_and_replay_never_executes() -> Result<(), Box<dyn Error>>
{
    guard_case(|| DurabilityError::RootUnavailable, true)
}

#[test]
fn house_transfer_guard_fence_mismatch_disconnects() -> Result<(), Box<dyn Error>> {
    guard_case(|| DurabilityError::Unavailable, false)
}
