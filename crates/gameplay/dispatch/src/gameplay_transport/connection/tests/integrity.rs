//! SEC-1 through the production connection dispatcher and retained command stream.
use super::*;

struct IntegrityAuthority {
    movement: StepAuthority,
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
}

impl IntegrityAuthority {
    fn new() -> Self {
        Self {
            movement: StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            },
            commands: Default::default(),
        }
    }
}

impl FreshAdmissionAuthority for IntegrityAuthority {
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

    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        self.movement.paced_step(actor, session, direction).await
    }
}

#[test]
fn sec1_flood_refuses_without_owner_effect_and_connection_recovers() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = IntegrityAuthority::new();
        // Unknown types also cost capacity. None can reach the movement owner.
        let mut commands: Vec<_> = (1..=100)
            .map(|id| command(1, id, 0x7fff, StepDirection::East))
            .collect();
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        commands.extend([
            command(1, 101, step, StepDirection::East),
            command(1, 101, step, StepDirection::East), // retained refusal, never a step
            command(1, 102, step, StepDirection::North),
        ]);
        let started = tokio::time::Instant::now();
        let (end, frames) = drive_admitted(&authority, &commands).await?;
        assert!(started.elapsed() >= std::time::Duration::from_secs(1));
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        assert_eq!(*authority.movement.steps.borrow(), [StepDirection::North]);
        let results: Vec<_> = frames
            .iter()
            .map(|frame| decode_wire_envelope(frame))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|envelope| envelope.message_type() == MessageType::CommandResult)
            .map(|envelope| oteryn_protocol_oteryn::decode_command_result(envelope.payload()))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(results.len(), 103);
        assert_eq!(results[100].status, CommandStatus::Rejected);
        // A legacy session has no TOO_EARLY capability, so the existing step codec folds it.
        assert_eq!(
            results[100].payload,
            encode_step_result(StepDisposition::Rejected)
        );
        assert_eq!(results[101].status, CommandStatus::DuplicateReplay);
        assert_eq!(results[101].payload, results[100].payload);
        assert_eq!(results[102].status, CommandStatus::Accepted);
        assert_eq!(
            results[102].payload,
            encode_step_result(StepDisposition::Blocked)
        );
        Ok(())
    })
}

#[test]
fn sec1_replay_flood_is_backpressured_without_reexecuting() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = IntegrityAuthority::new();
        let step = command(
            1,
            1,
            u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
            StepDirection::North,
        );
        let started = tokio::time::Instant::now();
        let (end, frames) = drive_admitted(&authority, &vec![step; 101]).await?;
        assert!(started.elapsed() >= std::time::Duration::from_secs(1));
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        assert_eq!(*authority.movement.steps.borrow(), [StepDirection::North]);
        let envelope = decode_wire_envelope(frames.last().ok_or("last result")?)?;
        let last = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
        assert_eq!(last.status, CommandStatus::DuplicateReplay);
        assert_eq!(last.payload, encode_step_result(StepDisposition::Blocked));
        Ok(())
    })
}

#[test]
fn sec1_flood_preserves_use_and_cast_refusal_payloads() -> Result<(), Box<dyn Error>> {
    run(async {
        for (command_type, payload) in [
            (
                2,
                encode_use_result(UseDisposition::Exhausted).map_err(|_| "use result")?,
            ),
            (
                3,
                encode_spell_cast_result(SpellCastDisposition::CoolingDown),
            ),
        ] {
            let authority = IntegrityAuthority::new();
            let mut commands: Vec<_> = (1..=100)
                .map(|id| command(1, id, 0x7fff, StepDirection::East))
                .collect();
            commands.push(command(1, 101, command_type, StepDirection::East));
            let (end, frames) = drive_admitted(&authority, &commands).await?;
            assert!(
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                "{end:?}"
            );
            let envelope = decode_wire_envelope(frames.last().ok_or("last result")?)?;
            let last = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
            assert_eq!(last.status, CommandStatus::Rejected);
            assert_eq!(last.payload, payload);
        }
        Ok(())
    })
}
