/// Historical result fixture only: it supplies no NPC/current-fence or money authority.
/// Actual continuation admission/drain authority is qualified separately with native Root/PG.
struct RetainedBankOutcomeFixture {
    commands: RefCell<Vec<UseCommand>>,
    replacement_steps: RefCell<usize>,
    unknown: bool,
    closed_gate: bool,
}
impl FreshAdmissionAuthority for RetainedBankOutcomeFixture {
    async fn enter_input(&self) -> Option<InputPermit<'_>> {
        if self.closed_gate {
            None
        } else {
            Some(InputPermit::ungated())
        }
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
    async fn step(&self, _: ExactActorRef, _: StepDirection) -> StepOutcome {
        *self.replacement_steps.borrow_mut() += 1;
        StepOutcome::rejected()
    }
    async fn resume_npc_bank(
        &self,
        _: ExactActorRef,
        command: UseCommand,
    ) -> Option<NpcTalkOutcome> {
        self.commands.borrow_mut().push(command);
        Some(NpcTalkOutcome {
            unknown: self.unknown,
            inventory_changed: false,
            disposition: NpcIntentDisposition::Ok,
            conversation: None,
            window: None,
        })
    }
}

#[test]
fn original_bank_command_precedes_replacement_text_npc_and_command_type()
-> Result<(), Box<dyn Error>> {
    run(async {
        // All replacements keep the original sequenced CommandId. Neither decoding a new
        // talk nor a different/unselected command may orphan its accepted original work.
        for (kind, payload) in [
            (COMMAND_TYPE_NPC_TALK_INTENT, b"no".as_slice()),
            (
                COMMAND_TYPE_NPC_TALK_INTENT,
                b"different NPC bytes".as_slice(),
            ),
            (
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                b"replacement step".as_slice(),
            ),
            (999, b"".as_slice()),
        ] {
            let authority = RetainedBankOutcomeFixture {
                commands: RefCell::new(Vec::new()),
                replacement_steps: RefCell::new(0),
                unknown: false,
                closed_gate: false,
            };
            let mut command = Vec::new();
            scalar(&mut command, 1, 1);
            scalar(&mut command, 2, u64::from(kind));
            bytes(&mut command, 4, payload);
            let (end, frames) = drive_session(
                &authority,
                resumed_positioned()?,
                &[envelope(7, RESUMED_GENERATION, &command)],
            )
            .await?;
            assert_eq!(*authority.replacement_steps.borrow(), 0);
            assert_eq!(authority.commands.borrow().len(), 1);
            assert_eq!(authority.commands.borrow()[0].command_id, 1);
            assert_eq!(
                authority.commands.borrow()[0].game_session_id,
                GameSessionId::decode(&SESSION)?
            );
            let mut expected = baseline_at(RESUMED_GENERATION);
            expected.push(encode_command_result(
                RESUMED_GENERATION,
                1,
                1,
                CommandStatus::Accepted,
                &encode_npc_intent_result(NpcIntentDisposition::Ok),
            )?);
            assert_eq!(frames, expected);
            assert!(
                matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==2)
            );
        }
        Ok(())
    })
}

#[test]
fn unresolved_original_bank_work_never_sequences_a_replacement_command()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = RetainedBankOutcomeFixture {
            commands: RefCell::new(Vec::new()),
            replacement_steps: RefCell::new(0),
            unknown: true,
            closed_gate: false,
        };
        let (end, frames) = drive_session(
            &authority,
            resumed_positioned()?,
            &[command(
                RESUMED_GENERATION,
                1,
                u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                StepDirection::East,
            )],
        )
        .await?;
        assert_eq!(frames, baseline_at(RESUMED_GENERATION));
        assert_eq!(*authority.replacement_steps.borrow(), 0);
        assert!(
            matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==1)
        );
        Ok(())
    })
}

#[test]
fn closed_input_gate_holds_unclassified_original_bank_command() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = RetainedBankOutcomeFixture {
            commands: RefCell::new(Vec::new()),
            replacement_steps: RefCell::new(0),
            unknown: false,
            closed_gate: true,
        };
        let (end, frames) = drive_session(
            &authority,
            resumed_positioned()?,
            &[command(
                RESUMED_GENERATION,
                1,
                u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                StepDirection::East,
            )],
        )
        .await?;
        assert_eq!(frames, baseline_at(RESUMED_GENERATION));
        assert!(authority.commands.borrow().is_empty());
        assert_eq!(*authority.replacement_steps.borrow(), 0);
        assert!(
            matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==1)
        );
        Ok(())
    })
}

/// NPC-ECON-1: a fresh admission is the GameSession's first connection, so no original Bank
/// command can be retained. Its first command dispatches without a Bank journal lookup, even
/// when that lookup would be unknown (Bank storage unavailable), and a closed input gate
/// refuses it as usual instead of holding it.
#[test]
fn fresh_session_first_command_skips_retained_bank_lookup() -> Result<(), Box<dyn Error>> {
    run(async {
        for closed_gate in [false, true] {
            let authority = RetainedBankOutcomeFixture {
                commands: RefCell::new(Vec::new()),
                replacement_steps: RefCell::new(0),
                unknown: true,
                closed_gate,
            };
            let (end, frames) = drive_session(
                &authority,
                positioned()?,
                &[command(
                    ADMITTED_GENERATION,
                    1,
                    u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                    StepDirection::East,
                )],
            )
            .await?;
            assert!(authority.commands.borrow().is_empty());
            assert_eq!(
                *authority.replacement_steps.borrow(),
                usize::from(!closed_gate)
            );
            assert_eq!(frames.len(), baseline().len() + 1);
            assert_eq!(frames[..baseline().len()], baseline()[..]);
            assert!(
                matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==2)
            );
        }
        Ok(())
    })
}
