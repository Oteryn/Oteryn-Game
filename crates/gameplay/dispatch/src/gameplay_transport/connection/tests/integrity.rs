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
        // Virtual time: the 100 flood commands share one instant, so the refusal's sleep
        // frees the whole window. On wall time a scheduling gap inside the flood leaves
        // later slots unexpired and the recovering step is refused too.
        tokio::time::pause();
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

fn run_detector_test<F: std::future::Future<Output = Result<(), Box<dyn Error>>>>(
    test: impl FnOnce() -> F + Send + 'static,
) -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run(test()).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "detector test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn sec1_detect_cadence_burst_flags_without_changing_outcomes() -> Result<(), Box<dyn Error>> {
    run_detector_test(|| async {
        tokio::time::pause();
        for dispatch in [
            Dispatch::Step(StepOutcome::too_early(), 0),
            Dispatch::Use(UseOutcome {
                disposition: UseDisposition::Exhausted,
                ..UseOutcome::rejected()
            }),
            Dispatch::Spell(SpellCastOutcome {
                disposition: SpellCastDisposition::CoolingDown,
                vitals: None,
            }),
            Dispatch::Use(UseOutcome {
                disposition: UseDisposition::TooFar,
                ..UseOutcome::rejected()
            }),
        ]
        .iter()
        {
            let mut probe = Dispatch::sec1_detect_probe();
            for _ in 0..7 {
                assert_eq!(probe(dispatch).0, [false; 3]);
            }
            let (flags, _) = probe(dispatch);
            assert!(flags.into_iter().any(|flag| flag));
        }
        // Use and spell refusals share the exhaust detector, but connections do not.
        let mut mixed = Dispatch::sec1_detect_probe();
        for _ in 0..4 {
            assert_eq!(
                mixed(&Dispatch::Use(UseOutcome {
                    disposition: UseDisposition::Exhausted,
                    ..UseOutcome::rejected()
                }))
                .0,
                [false; 3]
            );
        }
        for _ in 0..4 {
            mixed(&Dispatch::Spell(SpellCastOutcome {
                disposition: SpellCastDisposition::CoolingDown,
                vitals: None,
            }));
        }
        assert!(mixed(&Dispatch::Unregistered).0[1]);
        assert_eq!(
            Dispatch::sec1_detect_probe()(&Dispatch::Unregistered).0,
            [false; 3]
        );
        let authority = PacedAuthority {
            steps: RefCell::new(Vec::new()),
            turn: std::time::Duration::ZERO,
            receipts: false,
        };
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        let commands: Vec<_> = (1..=10)
            .map(|id| command(1, id, step, StepDirection::East))
            .collect();
        let (end, frames) = drive_open(&authority, paced_session(&[13])?, &commands, 12).await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        assert_eq!(authority.steps.borrow().len(), 2);
        let mut accepted = 0;
        let mut refused = 0;
        for frame in frames {
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.status == CommandStatus::Accepted {
                    accepted += 1;
                    assert_eq!(result.payload, encode_step_result(StepDisposition::Moved));
                } else {
                    refused += 1;
                    assert_eq!(result.status, CommandStatus::Rejected);
                    assert_eq!(result.payload, vec![0x08, 0x04]);
                }
            }
        }
        assert_eq!((accepted, refused), (2, 8));
        Ok(())
    })
}

#[test]
fn sec1_detect_state_is_bounded_under_flood() -> Result<(), Box<dyn Error>> {
    run_detector_test(|| async {
        tokio::time::pause();
        let mut probe = Dispatch::sec1_detect_probe();
        let refusal = Dispatch::Step(StepOutcome::too_early(), 0);
        let (_, size) = probe(&refusal);
        assert!(size <= 4096);
        for _ in 0..100_000 {
            assert_eq!(probe(&refusal).1, size);
        }
        assert!(probe(&refusal).0[0]);
        Ok(())
    })
}

#[test]
fn sec1_detect_no_flag_for_paced_play() -> Result<(), Box<dyn Error>> {
    run_detector_test(|| async {
        tokio::time::pause();
        let mut probe = Dispatch::sec1_detect_probe();
        for _ in 0..32 {
            assert_eq!(
                probe(&Dispatch::Step(StepOutcome::too_early(), 0)).0,
                [false; 3]
            );
            assert_eq!(
                probe(&Dispatch::Spell(SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: None
                }))
                .0,
                [false; 3]
            );
            tokio::time::advance(std::time::Duration::from_secs(1)).await;
        }
        Ok(())
    })
}
