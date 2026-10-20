use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
#[derive(Clone)]
struct Owner(ControlLossObservationV1);
impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for Owner {}
impl ControlLossSourceV1 for Owner {
    fn resolve_loss(
        &self,
        _: GameSessionId,
        _: i64,
    ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
        Ok(self.0.clone())
    }
}
fn owner() -> Result<Owner, Box<dyn std::error::Error>> {
    let id = |last| {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = last;
        b
    };
    let character = CharacterId::decode(&id(2)).map_err(|_| "character")?;
    let world = WorldId::decode(&id(3)).map_err(|_| "world")?;
    let channel = ChannelId::decode(&id(4)).map_err(|_| "channel")?;
    let transport = AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(|_| "transport")?;
    let commit = FreshAdmissionCommit::from_facts(
        GameSessionId::decode(&id(5)).map_err(|_| "session")?,
        FreshAdmissionFacts::new([3; 32], character, world, channel, 2, 3)?,
        transport,
    )?;
    Ok(Owner(ControlLossObservationV1 {
        source_authority: RuntimeScopeRefV1::channel(world, channel),
        source_revision: 7,
        accepted_source_revision: 7,
        decision_identity: ControlLossEpochRefV1::new(1).map_err(|_| "decision")?,
        accepted_decision_identity: ControlLossEpochRefV1::new(1).map_err(|_| "decision")?,
        observed_at: 100,
        session: GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            GameSessionState::Active,
            ConnectionGeneration::new(1).map_err(|_| "generation")?,
            Some(transport),
            CharacterLease::new(character, 2)?,
            Some(CharacterWorldEligibilityClaimV1::new(character, world)),
            RuntimeScopeRefV1::channel(world, channel),
            ScopeOwnershipGeneration::new(3).map_err(|_| "scope")?,
        )
        .map_err(|_| "snapshot")?,
        account_presence: AccountPresenceClaimV1::new(
            "00000000-0000-4000-8000-000000000001",
            character,
        )
        .map_err(|_| "account")?,
        placement_identity: id(6),
        placement_revision: 8,
        actor_present: true,
        runtime_ready: true,
        cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
        loss_epoch: ControlLossEpochRefV1::new(1).map_err(|_| "epoch")?,
        loss_origin: 100,
        original_grace_deadline: 120,
        history: ControlLossHistoryV1::FreshOrigin,
        protection: RecoveryProtectionContinuityV1 {
            usage: RecoveryProtectionUseV1::Unused {
                entitlement_generation: 1,
            },
            rearm: RecoveryProtectionRearmV1::NotRearmed {
                generation: 1,
                stable_control_started_at: None,
                accepted_deadline: None,
            },
        },
    }))
}
#[test]
fn owning_loss_first_loss_preserves_claims_and_removes_only_exact_controller() -> TestResult {
    let owner = owner()?;
    let auth = ControlLossAuthorizationV1::authorize(
        &owner,
        owner.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "authorize")?;
    let effect = auth.validate_final(&owner, 100).map_err(|_| "final")?;
    assert_eq!(effect.predecessor(), owner.0.session);
    assert_eq!(
        effect.successor().session_state(),
        GameSessionState::Reconnectable
    );
    assert_eq!(effect.successor().current_transport(), None);
    assert_eq!(
        effect.successor().current_connection_generation(),
        owner.0.session.current_connection_generation()
    );
    assert_eq!(effect.successor().commit(), owner.0.session.commit());
    assert_eq!(
        effect.successor().current_character_lease(),
        owner.0.session.current_character_lease()
    );
    assert_eq!(
        effect.operation().observation.account_presence,
        owner.0.account_presence
    );
    assert_eq!(
        effect.operation().observation.protection,
        owner.0.protection
    );
    assert_eq!(
        effect.successor().current_original_grace_deadline(),
        Some(120)
    );
    Ok(())
}

#[test]
fn owning_loss_non_authoritative_causes_never_start_epoch() -> TestResult {
    for cause in [
        ControlLossCauseV1::HealthyController,
        ControlLossCauseV1::SocketClosedOnly,
        ControlLossCauseV1::ProcessRestartOnly,
        ControlLossCauseV1::GracefulLogout,
        ControlLossCauseV1::HealthyMigration,
        ControlLossCauseV1::Suspected,
    ] {
        let mut source = owner()?;
        let auth = ControlLossAuthorizationV1::authorize(
            &source,
            source.0.session.commit().game_session_id(),
            100,
        )
        .map_err(|_| "positive")?;
        source.0.cause = cause;
        assert!(
            ControlLossAuthorizationV1::authorize(
                &source,
                source.0.session.commit().game_session_id(),
                100
            )
            .is_err()
        );
        assert!(auth.validate_final(&source, 100).is_err());
    }
    Ok(())
}
#[test]
fn owning_loss_final_revalidates_each_independently_changed_current_fence() -> TestResult {
    for mutation in 0..19 {
        let original = owner()?;
        let auth = ControlLossAuthorizationV1::authorize(
            &original,
            original.0.session.commit().game_session_id(),
            100,
        )
        .map_err(|_| "authorize")?;
        let mut changed = original.clone();
        changed.0.source_revision = 8;
        changed.0.accepted_source_revision = 8;
        changed.0.observed_at = 101;
        assert!(
            auth.validate_final(&changed, 101).is_ok(),
            "positive {mutation}"
        );
        match mutation {
            0 => {
                changed.0.session.current_transport =
                    Some(AuthenticatedTransportRefV1::decode(&[8; 16]).map_err(|_| "transport")?)
            }
            1 => {
                changed.0.session.current_connection_generation =
                    ConnectionGeneration::new(2).map_err(|_| "generation")?
            }
            2 => {
                changed.0.session.current_scope_generation =
                    ScopeOwnershipGeneration::new(4).map_err(|_| "owner")?
            }
            3 => {
                changed.0.session.current_character_lease =
                    CharacterLease::new(changed.0.session.commit().character_id(), 3)?
            }
            4 => changed.0.session.session_state = GameSessionState::Terminal,
            5 => changed.0.actor_present = false,
            6 => changed.0.runtime_ready = false,
            7 => changed.0.placement_identity[15] += 1,
            8 => changed.0.placement_revision += 1,
            9 => {
                changed.0.account_presence = AccountPresenceClaimV1::new(
                    "00000000-0000-4000-8000-000000000002",
                    changed.0.session.commit().character_id(),
                )
                .map_err(|_| "account")?
            }
            10 => {
                changed.0.source_authority = RuntimeScopeRefV1::instance(
                    changed.0.session.commit().world_id(),
                    changed.0.placement_identity,
                )
                .map_err(|_| "source scope")?
            }
            11 => {
                changed.0.decision_identity =
                    ControlLossEpochRefV1::new(9).map_err(|_| "decision")?
            }
            12 => {
                changed.0.loss_epoch = ControlLossEpochRefV1::new(2).map_err(|_| "epoch")?;
                changed.0.decision_identity = changed.0.loss_epoch;
                changed.0.accepted_decision_identity = changed.0.loss_epoch;
                assert!(
                    ControlLossAuthorizationV1::authorize(
                        &changed,
                        changed.0.session.commit().game_session_id(),
                        101
                    )
                    .is_ok()
                );
            }
            13 => changed.0.loss_origin = 101,
            14 => changed.0.original_grace_deadline += 1,
            15 => changed.0.protection.usage = RecoveryProtectionUseV1::NotEntitled,
            16 => {
                changed.0.protection.rearm = RecoveryProtectionRearmV1::Satisfied {
                    generation: 2,
                    established_at: 100,
                }
            }
            17 => changed.0.session.current_character_world_eligibility = None,
            _ => changed.0.accepted_source_revision = 7,
        }
        assert!(
            auth.validate_final(&changed, 101).is_err(),
            "mutation {mutation}"
        );
    }
    Ok(())
}
fn resumed() -> Result<Owner, Box<dyn std::error::Error>> {
    let mut source = owner()?;
    let epoch = ControlLossEpochRefV1::new(1).map_err(|_| "epoch")?;
    let transport = AuthenticatedTransportRefV1::decode(&[3; 16]).map_err(|_| "transport")?;
    source.0.session.current_connection_generation =
        ConnectionGeneration::new(2).map_err(|_| "generation")?;
    source.0.session.current_transport = Some(transport);
    source.0.session.current_control_loss_epoch = Some(epoch);
    source.0.session.current_original_grace_deadline = Some(90);
    source.0.loss_epoch = ControlLossEpochRefV1::new(2).map_err(|_| "new epoch")?;
    source.0.decision_identity = source.0.loss_epoch;
    source.0.accepted_decision_identity = source.0.loss_epoch;
    source.0.protection.usage = RecoveryProtectionUseV1::Activated {
        entitlement_generation: 1,
        activated_at: 80,
        deadline: 84,
    };
    let budget = RetainedRecoveryBudgetV1::restore(
        epoch,
        RecoveryEpochStateV1::Restored,
        true,
        vec![RetainedRecoveryAttemptV1 {
            attempt: ReconnectAttemptRef::new(1)?,
            transport,
            disposition: RetainedRecoveryAttemptDispositionV1::Committed,
        }],
    )
    .map_err(|_| "budget")?;
    source.0.history = ControlLossHistoryV1::Resumed {
        budget,
        original_grace_deadline: 90,
        protection: source.0.protection,
    };
    Ok(source)
}
#[test]
fn owning_loss_after_resumed_control_preserves_consumption_and_original_history() -> TestResult {
    let source = resumed()?;
    let auth = ControlLossAuthorizationV1::authorize(
        &source,
        source.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "later loss")?;
    let effect = auth.validate_final(&source, 100).map_err(|_| "final")?;
    assert_eq!(effect.operation().observation.history, source.0.history);
    assert_eq!(
        effect.operation().observation.protection,
        source.0.protection
    );
    assert_eq!(
        effect.successor().current_control_loss_epoch(),
        Some(source.0.loss_epoch)
    );
    assert_eq!(
        effect.successor().current_connection_generation(),
        source.0.session.current_connection_generation()
    );
    let mut missing = source.clone();
    missing.0.history = ControlLossHistoryV1::FreshOrigin;
    assert!(
        ControlLossAuthorizationV1::authorize(
            &missing,
            source.0.session.commit().game_session_id(),
            100
        )
        .is_err()
    );
    let mut reused = source.clone();
    reused.0.loss_epoch = ControlLossEpochRefV1::new(1).map_err(|_| "old epoch")?;
    reused.0.decision_identity = reused.0.loss_epoch;
    reused.0.accepted_decision_identity = reused.0.loss_epoch;
    assert!(
        ControlLossAuthorizationV1::authorize(
            &reused,
            source.0.session.commit().game_session_id(),
            100
        )
        .is_err()
    );
    let mut reset = source.clone();
    reset.0.protection.usage = RecoveryProtectionUseV1::Unused {
        entitlement_generation: 2,
    };
    assert!(
        ControlLossAuthorizationV1::authorize(
            &reset,
            source.0.session.commit().game_session_id(),
            100
        )
        .is_err()
    );
    Ok(())
}

struct Completion(Option<ControlLossCompletionV1>);
impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for Completion {}
impl ControlLossCompletionSourceV1 for Completion {
    fn take_loss_completion(
        &mut self,
        _: &ControlLossOperationV1,
    ) -> Result<Option<ControlLossCompletionV1>, ReconnectDurabilityErrorV1> {
        Ok(self.0.take())
    }
}
#[test]
fn owning_loss_lost_completion_reconciles_original_receipt_without_new_authority() -> TestResult {
    let source = owner()?;
    let auth = ControlLossAuthorizationV1::authorize(
        &source,
        source.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "authorize")?;
    let operation = auth.operation().clone();
    let mut flow = ControlLossFlowV1::begin(auth);
    let request = flow.take_request().map_err(|_| "request")?;
    assert!(request.validate_final(&source, 100).is_ok());
    assert!(flow.take_request().is_err());
    flow.accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
        operation: operation.clone(),
        outcome: ControlLossOutcomeV1::Ambiguous,
    })))
    .map_err(|_| "ambiguous")?;
    assert_eq!(flow.phase(), ControlLossPhaseV1::ReconciliationRequired);
    let mut restart = ControlLossFlowV1::restore(operation.clone()).map_err(|_| "restore")?;
    assert!(restart.take_request().is_err());
    let committed = ControlLossCompletionV1 {
        operation: operation.clone(),
        outcome: ControlLossOutcomeV1::Committed { decided_at: 100 },
    };
    restart
        .accept_completion(&mut Completion(Some(committed.clone())))
        .map_err(|_| "committed")?;
    let receipt = restart.receipt().ok_or("receipt")?.clone();
    assert_eq!(receipt.operation(), &operation);
    assert_eq!(receipt.decided_at(), 100);
    restart
        .accept_completion(&mut Completion(Some(committed)))
        .map_err(|_| "exact retry")?;
    assert_eq!(restart.receipt(), Some(&receipt));
    let mut healthy = source.clone();
    healthy.0.cause = ControlLossCauseV1::HealthyController;
    assert!(request.validate_final(&healthy, 101).is_err());
    assert!(restart.take_request().is_err());
    Ok(())
}
#[test]
fn owning_loss_conflicting_completion_cannot_replace_immutable_operation() -> TestResult {
    let source = owner()?;
    let auth = ControlLossAuthorizationV1::authorize(
        &source,
        source.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "authorize")?;
    let original = auth.operation().clone();
    let mut flow = ControlLossFlowV1::begin(auth);
    let _ = flow.take_request().map_err(|_| "request")?;
    let mut changed = original.clone();
    changed.observation.original_grace_deadline += 1;
    assert!(
        flow.accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
            operation: changed,
            outcome: ControlLossOutcomeV1::Committed { decided_at: 100 }
        })))
        .is_err()
    );
    assert!(flow.receipt().is_none());
    assert_eq!(flow.operation(), &original);
    Ok(())
}

#[test]
fn owning_loss_missing_source_and_invalid_provenance_fail_at_both_boundaries() -> TestResult {
    struct Missing;
    impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for Missing {}
    impl ControlLossSourceV1 for Missing {
        fn resolve_loss(
            &self,
            _: GameSessionId,
            _: i64,
        ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        }
    }
    let source = owner()?;
    let session = source.0.session.commit().game_session_id();
    let auth =
        ControlLossAuthorizationV1::authorize(&source, session, 100).map_err(|_| "authorize")?;
    assert!(ControlLossAuthorizationV1::authorize(&Missing, session, 100).is_err());
    assert!(auth.validate_final(&Missing, 100).is_err());
    for mutation in 0..10 {
        let mut changed = source.clone();
        match mutation {
            0 => changed.0.source_revision = 0,
            1 => changed.0.accepted_source_revision = 6,
            2 => {
                changed.0.source_authority = RuntimeScopeRefV1::instance(
                    changed.0.session.commit().world_id(),
                    changed.0.placement_identity,
                )
                .map_err(|_| "source scope")?
            }
            3 => {
                changed.0.decision_identity =
                    ControlLossEpochRefV1::new(9).map_err(|_| "decision")?
            }
            4 => {
                changed.0.accepted_decision_identity =
                    ControlLossEpochRefV1::new(9).map_err(|_| "decision")?
            }
            5 => changed.0.observed_at = 101,
            6 => changed.0.loss_origin = 101,
            7 => changed.0.original_grace_deadline = 100,
            8 => changed.0.placement_identity = [0; 16],
            _ => changed.0.session.current_transport = None,
        }
        assert!(
            ControlLossAuthorizationV1::authorize(&changed, session, 100).is_err(),
            "authorize {mutation}"
        );
        assert!(
            auth.validate_final(&changed, 100).is_err(),
            "final {mutation}"
        );
    }
    assert!(auth.validate_final(&source, 99).is_err());
    Ok(())
}
#[test]
fn owning_loss_history_versions_and_terminal_dispositions_are_closed() -> TestResult {
    let source = owner()?;
    let auth = ControlLossAuthorizationV1::authorize(
        &source,
        source.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "authorize")?;
    let operation = auth.operation().clone();
    let mut unknown = operation.clone();
    unknown.version = 2;
    assert!(ControlLossFlowV1::restore(unknown).is_err());
    let mut raw = operation.clone();
    raw.observation.cause = ControlLossCauseV1::SocketClosedOnly;
    assert!(ControlLossFlowV1::restore(raw).is_err());
    let mut rejected = ControlLossFlowV1::restore(operation.clone()).map_err(|_| "restore")?;
    rejected
        .accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
            operation: operation.clone(),
            outcome: ControlLossOutcomeV1::Rejected,
        })))
        .map_err(|_| "reject")?;
    assert!(
        rejected
            .accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
                operation: operation.clone(),
                outcome: ControlLossOutcomeV1::Committed { decided_at: 100 }
            })))
            .is_err()
    );
    assert!(rejected.take_request().is_err());
    let mut committed = ControlLossFlowV1::restore(operation.clone()).map_err(|_| "restore")?;
    assert!(
        committed
            .accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
                operation: operation.clone(),
                outcome: ControlLossOutcomeV1::Committed { decided_at: 99 }
            })))
            .is_err()
    );
    committed
        .accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
            operation: operation.clone(),
            outcome: ControlLossOutcomeV1::Committed { decided_at: 100 },
        })))
        .map_err(|_| "commit")?;
    assert!(
        committed
            .accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
                operation: operation.clone(),
                outcome: ControlLossOutcomeV1::Committed { decided_at: 101 }
            })))
            .is_err()
    );
    assert!(
        committed
            .accept_completion(&mut Completion(Some(ControlLossCompletionV1 {
                operation,
                outcome: ControlLossOutcomeV1::Rejected
            })))
            .is_err()
    );
    assert_eq!(committed.receipt().ok_or("receipt")?.decided_at(), 100);
    Ok(())
}
#[test]
fn owning_loss_committed_retry_cannot_generate_second_epoch_or_extend_grace() -> TestResult {
    let source = owner()?;
    let auth = ControlLossAuthorizationV1::authorize(
        &source,
        source.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "authorize")?;
    let effect = auth.validate_final(&source, 100).map_err(|_| "final")?;
    let mut committed = source.clone();
    committed.0.session = effect.successor();
    assert!(
        ControlLossAuthorizationV1::authorize(
            &committed,
            source.0.session.commit().game_session_id(),
            101
        )
        .is_err()
    );
    assert!(auth.validate_final(&committed, 101).is_err());
    assert_eq!(
        effect.successor().current_original_grace_deadline(),
        Some(120)
    );
    assert_eq!(
        effect.successor().current_control_loss_epoch(),
        Some(source.0.loss_epoch)
    );
    Ok(())
}

#[test]
fn owning_loss_cannot_rearm_protection_after_the_loss_origin() -> TestResult {
    let mut source = owner()?;
    source.0.observed_at = 101;
    source.0.protection.rearm = RecoveryProtectionRearmV1::Satisfied {
        generation: 2,
        established_at: 101,
    };
    assert!(
        ControlLossAuthorizationV1::authorize(
            &source,
            source.0.session.commit().game_session_id(),
            101
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn owning_loss_resumed_history_must_match_the_current_winner_transport() -> TestResult {
    let mut source = resumed()?;
    let ControlLossHistoryV1::Resumed { budget, .. } = &mut source.0.history else {
        return Err("history".into());
    };
    budget.entries[0].transport =
        AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(|_| "transport")?;
    assert!(
        ControlLossAuthorizationV1::authorize(
            &source,
            source.0.session.commit().game_session_id(),
            100
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn owning_loss_after_post_grace_generation_one_preserves_rearm_evidence() -> TestResult {
    let mut source = resumed()?;
    let transport = source.0.session.current_transport().ok_or("transport")?;
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = 9;
    source.0.session.commit.game_session_id =
        GameSessionId::decode(&bytes).map_err(|_| "new session")?;
    source.0.session.commit.initial_transport = transport;
    source.0.session.current_connection_generation =
        ConnectionGeneration::new(1).map_err(|_| "generation")?;
    source.0.protection.rearm = RecoveryProtectionRearmV1::Satisfied {
        generation: 2,
        established_at: 99,
    };
    let ControlLossHistoryV1::Resumed { protection, .. } = &mut source.0.history else {
        return Err("history".into());
    };
    *protection = source.0.protection;
    let auth = ControlLossAuthorizationV1::authorize(
        &source,
        source.0.session.commit().game_session_id(),
        100,
    )
    .map_err(|_| "generation-one loss")?;
    let effect = auth.validate_final(&source, 100).map_err(|_| "final")?;
    assert_eq!(effect.successor().current_connection_generation().get(), 1);
    assert_eq!(
        effect.operation().observation.protection,
        source.0.protection
    );
    assert_eq!(effect.operation().observation.history, source.0.history);
    Ok(())
}
