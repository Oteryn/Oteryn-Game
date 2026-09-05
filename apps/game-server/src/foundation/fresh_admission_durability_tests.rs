use super::*;
use crate::foundation::PRE_ADMISSION_PROFILE;
use crate::foundation::admission_authority_publication::*;
use crate::foundation::fresh_admission_durability::*;
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterLease, CharacterWorldEligibilityClaimV1,
    ConnectionGeneration, FreshAdmissionCommit, GameSessionAuthoritySnapshot, GameSessionId,
    GameSessionState, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(last: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = last;
    id
}

// These sources exist before verification, authorization or receipt construction.
struct Independent {
    source: PublishedFreshSource,
    live_transport: bool,
    rows: Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
    snapshot: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
}
impl fresh_source_sealed::Sealed for Independent {}
impl AdmissionAuthorityPublicationCurrentSourceV1 for Independent {
    fn current_publications(
        &self,
        _: &[AdmissionAuthorityGuardKeyV1],
    ) -> Result<
        Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
        AdmissionAuthorityPublicationErrorV1,
    > {
        Ok(self.rows.clone())
    }
}
impl FreshAdmissionCurrentSourceV1 for Independent {
    fn has_live_transport(&self, transport: AuthenticatedTransportRefV1) -> bool {
        self.live_transport && transport == self.snapshot.commit().initial_transport()
    }
    fn current_session(
        &self,
        _: GameSessionId,
    ) -> Result<
        GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        FreshAdmissionDurabilityErrorV1,
    > {
        Ok(self.snapshot)
    }
}
impl Independent {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let key = SigningKey::from_bytes(&[31; 32]);
        let source =
            published_fresh_source(key.verifying_key().to_bytes()).map_err(|e| format!("{e:?}"))?;
        let f = &source.current;
        let session = GameSessionId::decode(&id(9))?;
        let transport = AuthenticatedTransportRefV1::decode(&[9; 16])?;
        let commit = FreshAdmissionCommit::from_facts(
            session,
            FreshAdmissionFacts::new([7; 32], f.character_id, f.world_id, f.channel_id, 2, 1)?,
            transport,
        )?;
        let snapshot = GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            GameSessionState::Active,
            ConnectionGeneration::new(1)?,
            Some(transport),
            CharacterLease::new(f.character_id, 2)?,
            Some(CharacterWorldEligibilityClaimV1::new(
                f.character_id,
                f.world_id,
            )),
            RuntimeScopeRefV1::channel(f.world_id, f.channel_id),
            ScopeOwnershipGeneration::new(1)?,
        )
        .map_err(|e| format!("{e:?}"))?;
        let state = vec![
            (
                AdmissionAuthorityGuardKeyV1::Account {
                    account_id: f.account_id.clone(),
                },
                AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
                11,
                AdmissionAuthorityGuardStateV1::Account {
                    security: source
                        .account_security(&f.account_id, 100)
                        .map_err(|e| format!("{e:?}"))?,
                    presence: None,
                },
            ),
            (
                AdmissionAuthorityGuardKeyV1::Character(f.character_id),
                AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
                13,
                AdmissionAuthorityGuardStateV1::Character {
                    account_id: f.account_id.clone(),
                    world_id: f.world_id,
                    eligible: true,
                    lease_generation: 1,
                    holder: None,
                },
            ),
            (
                AdmissionAuthorityGuardKeyV1::Runtime(RuntimeScopeRefV1::channel(
                    f.world_id,
                    f.channel_id,
                )),
                AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
                17,
                AdmissionAuthorityGuardStateV1::Runtime {
                    ownership_generation: 1,
                    ready: true,
                    route_revision: f.route_revision.clone(),
                    runtime_observation_revision: f.runtime_observation_revision.clone(),
                    protocol_major: 1,
                    transport_profile: 1,
                    ruleset_revision: f.ruleset_revision.clone(),
                    content_revision: f.content_revision.clone(),
                    map_revision: f.map_revision.clone(),
                    world_policy_revision: f.world_policy_revision.clone(),
                    offer_revision: f.offer_revision.clone(),
                },
            ),
            (
                AdmissionAuthorityGuardKeyV1::SigningTrust {
                    key_id: "fresh-1".into(),
                    profile: PRE_ADMISSION_PROFILE.into(),
                },
                AdmissionPublicationPurposeV1::FixedFreshSigningTrust,
                11,
                AdmissionAuthorityGuardStateV1::SigningTrust {
                    public_key: source.key,
                    trusted: true,
                },
            ),
        ];
        let rows = state
            .into_iter()
            .map(|(key, purpose, revision, state)| {
                Some(AdmissionAuthorityPublicationChangeV1 {
                    key,
                    source: AdmissionPublicationSourceV1 {
                        authority: if purpose
                            == AdmissionPublicationPurposeV1::FixedFreshSigningTrust
                        {
                            "independently-authenticated-test-source"
                        } else {
                            "game-owning-source"
                        }
                        .into(),
                        purpose,
                        source_revision: 7,
                        decision_identity: "source-decision-seven".into(),
                        source_observed_at: 99,
                        clock_uncertainty_seconds: 0,
                    },
                    precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
                        expected_publication_revision: revision - 1,
                    },
                    publication_revision: revision,
                    state,
                })
            })
            .collect();
        Ok(Self {
            source,
            rows,
            snapshot,
            live_transport: true,
        })
    }
    fn authorize(&self) -> Result<FreshAdmissionCommitAuthorizationV1, Box<dyn std::error::Error>> {
        self.authorize_payload(fresh_payload())
    }
    fn authorize_payload(
        &self,
        payload: String,
    ) -> Result<FreshAdmissionCommitAuthorizationV1, Box<dyn std::error::Error>> {
        let key = SigningKey::from_bytes(&[31; 32]);
        let grant = signed_token(
            &key,
            r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
            payload,
        );
        let facts = verify_fresh_grant_durability_v1(
            &grant,
            100,
            &FreshDurabilityTrustContext::from_owning_source(&self.source),
            &FreshDurabilityCurrentAuthorityV1::from_owning_source(&self.source),
        )
        .map_err(|e| format!("{e:?}"))?;
        Ok(FreshAdmissionCommitAuthorizationV1::new(
            &facts,
            GameSessionId::decode(&id(9))?,
            AuthenticatedTransportRefV1::decode(&[9; 16])?,
            self,
            100,
        )
        .map_err(|e| format!("{e:?}"))?)
    }
    fn commit_sources(&mut self) -> TestResult {
        for row in self.rows[..2].iter_mut().flatten() {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: row.publication_revision,
            };
            row.publication_revision += 1;
            row.source.source_revision += 1;
            row.source.decision_identity = "decision-eight".into();
            match &mut row.state {
                AdmissionAuthorityGuardStateV1::Account { security, presence } => {
                    security.provenance.publication_revision = row.publication_revision;
                    *presence = Some((
                        self.source.current.character_id,
                        GameSessionId::decode(&id(9))?,
                    ));
                }
                AdmissionAuthorityGuardStateV1::Character {
                    lease_generation,
                    holder,
                    ..
                } => {
                    *lease_generation = 2;
                    *holder = Some(GameSessionId::decode(&id(9))?);
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}
struct Queue {
    requests: Vec<FreshAdmissionAuditBindingV1>,
    available: bool,
}
impl FreshAdmissionDurabilityPortV1 for Queue {
    fn submit(&mut self, request: &FreshAdmissionCommitRequestV1) -> FreshAdmissionSubmissionV1 {
        self.requests.push(request.binding().clone());
        if self.available {
            FreshAdmissionSubmissionV1::Accepted
        } else {
            FreshAdmissionSubmissionV1::Unavailable
        }
    }
    fn reconcile(&mut self, original: &FreshAdmissionAuditBindingV1) -> FreshAdmissionSubmissionV1 {
        self.requests.push(original.clone());
        FreshAdmissionSubmissionV1::Accepted
    }
}
fn committed(
    flow: &FreshAdmissionDurabilityFlowV1,
) -> Result<FreshAdmissionCommitReceiptV1, FreshAdmissionDurabilityErrorV1> {
    FreshAdmissionCommitReceiptV1::restore(flow.binding().clone(), 100)
}
#[test]
fn fresh_submit_yields_without_controller() -> TestResult {
    let source = Independent::new()?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let mut queue = Queue {
        requests: vec![],
        available: true,
    };
    flow.submit(&mut queue).map_err(|e| format!("{e:?}"))?;
    assert_eq!(flow.phase(), FreshAdmissionPhaseV1::PendingCommit);
    assert!(flow.controller().is_none());
    assert_eq!(queue.requests, vec![flow.binding().clone()]);
    Ok(())
}
#[test]
fn fresh_unavailable_submission_has_no_authority() -> TestResult {
    let source = Independent::new()?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    flow.submit(&mut Queue {
        requests: vec![],
        available: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(flow.phase(), FreshAdmissionPhaseV1::Ready);
    assert!(flow.controller().is_none());
    Ok(())
}
#[test]
fn fresh_direct_and_reconciled_completion_adopt_only_current_sources() -> TestResult {
    for reconcile in [false, true] {
        let mut source = Independent::new()?;
        let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
        let mut queue = Queue {
            requests: vec![],
            available: true,
        };
        flow.submit(&mut queue).map_err(|e| format!("{e:?}"))?;
        let receipt = committed(&flow).map_err(|e| format!("{e:?}"))?;
        source.commit_sources()?;
        if reconcile {
            flow.accept_completion(
                flow.binding().clone(),
                FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable,
            )
            .map_err(|e| format!("{e:?}"))?;
            flow.reconcile(&mut queue).map_err(|e| format!("{e:?}"))?;
            flow.accept_reconciliation(FreshAdmissionDurableReconciliationSnapshotV1 {
                receipt: receipt.clone(),
                current_session: source.snapshot,
            })
            .map_err(|e| format!("{e:?}"))?;
        } else {
            flow.accept_completion(
                flow.binding().clone(),
                FreshAdmissionDurableOutcomeV1::Committed(receipt.clone()),
            )
            .map_err(|e| format!("{e:?}"))?;
        }
        assert!(flow.controller().is_none());
        flow.adopt(&source, 100).map_err(|e| format!("{e:?}"))?;
        assert_eq!(flow.controller(), Some(source.snapshot.commit()));
        assert_eq!(flow.receipt(), Some(&receipt));
    }
    Ok(())
}

#[test]
fn fresh_ambiguous_outcome_only_reconciles_original_binding() -> TestResult {
    let source = Independent::new()?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let mut queue = Queue {
        requests: vec![],
        available: true,
    };
    flow.submit(&mut queue).map_err(|e| format!("{e:?}"))?;
    let original = flow.binding().clone();
    flow.accept_completion(
        original.clone(),
        FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable,
    )
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        flow.submit(&mut queue),
        Err(FreshAdmissionDurabilityErrorV1::WrongPhase)
    );
    flow.reconcile(&mut queue).map_err(|e| format!("{e:?}"))?;
    assert_eq!(queue.requests, vec![original.clone(), original]);
    assert_eq!(
        flow.reconcile(&mut queue),
        Err(FreshAdmissionDurabilityErrorV1::WrongPhase)
    );
    assert!(flow.controller().is_none());
    Ok(())
}
#[test]
fn fresh_completion_requires_exact_request_and_order() -> TestResult {
    let source = Independent::new()?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let receipt = committed(&flow).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        flow.accept_completion(
            flow.binding().clone(),
            FreshAdmissionDurableOutcomeV1::Committed(receipt.clone())
        ),
        Err(FreshAdmissionDurabilityErrorV1::WrongPhase)
    );
    flow.accept_submission(FreshAdmissionSubmissionV1::Accepted)
        .map_err(|e| format!("{e:?}"))?;
    let mut wrong = flow.binding().clone();
    wrong.transport = AuthenticatedTransportRefV1::decode(&[8; 16])?;
    assert_eq!(
        flow.accept_completion(
            wrong,
            FreshAdmissionDurableOutcomeV1::Committed(receipt.clone())
        ),
        Err(FreshAdmissionDurabilityErrorV1::WrongBinding)
    );
    flow.accept_completion(
        flow.binding().clone(),
        FreshAdmissionDurableOutcomeV1::Committed(receipt.clone()),
    )
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        flow.accept_completion(
            flow.binding().clone(),
            FreshAdmissionDurableOutcomeV1::Committed(receipt)
        ),
        Err(FreshAdmissionDurabilityErrorV1::WrongPhase)
    );
    assert!(flow.controller().is_none());
    Ok(())
}
#[test]
fn fresh_same_key_exact_retry_preserves_original_commit_and_changed_binding_rejects() -> TestResult
{
    let source = Independent::new()?;
    let flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let receipt = committed(&flow).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        receipt.classify_retry(flow.binding()),
        FreshAdmissionDurableOutcomeV1::ExistingCommitted(receipt.clone())
    );
    for field in 0..6 {
        let mut binding = flow.binding().clone();
        match field {
            0 => binding.candidate_session = GameSessionId::decode(&id(8))?,
            1 => binding.transport = AuthenticatedTransportRefV1::decode(&[8; 16])?,
            2 => binding.account_id.push('x'),
            3 => binding.accepted_deadline += 1,
            4 => binding.credential_times.2 += 1,
            _ => binding.expected_guards[2].publication_revision += 1,
        }
        assert_eq!(
            binding.facts.replay_key(),
            flow.binding().facts.replay_key()
        );
        assert_eq!(
            receipt.classify_retry(&binding),
            FreshAdmissionDurableOutcomeV1::RejectedReplayConflict,
            "field {field}"
        );
    }
    assert_eq!(receipt.decided_at(), 100);
    Ok(())
}
#[test]
fn fresh_all_known_noncommit_outcomes_remain_authority_free() -> TestResult {
    for outcome in [
        FreshAdmissionDurableOutcomeV1::RejectedReplayConflict,
        FreshAdmissionDurableOutcomeV1::RejectedIncumbent,
        FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority,
        FreshAdmissionDurableOutcomeV1::RejectedCollision(
            FreshAdmissionCollisionV1::CandidateSession,
        ),
        FreshAdmissionDurableOutcomeV1::RejectedCollision(
            FreshAdmissionCollisionV1::TransportReference,
        ),
    ] {
        let source = Independent::new()?;
        let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
        flow.accept_submission(FreshAdmissionSubmissionV1::Accepted)
            .map_err(|e| format!("{e:?}"))?;
        flow.accept_completion(flow.binding().clone(), outcome)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(flow.phase(), FreshAdmissionPhaseV1::Rejected);
        assert!(flow.controller().is_none());
        assert!(flow.receipt().is_none());
    }
    Ok(())
}
fn accepted(
    source: &mut Independent,
    reconcile: bool,
) -> Result<FreshAdmissionDurabilityFlowV1, Box<dyn std::error::Error>> {
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let receipt = committed(&flow).map_err(|e| format!("{e:?}"))?;
    source.commit_sources()?;
    if reconcile {
        flow = FreshAdmissionDurabilityFlowV1::resume_reconciliation(flow.binding().clone());
        flow.reconcile(&mut Queue {
            requests: vec![],
            available: true,
        })
        .map_err(|e| format!("{e:?}"))?;
        flow.accept_reconciliation(FreshAdmissionDurableReconciliationSnapshotV1 {
            receipt,
            current_session: source.snapshot,
        })
        .map_err(|e| format!("{e:?}"))?;
    } else {
        flow.accept_submission(FreshAdmissionSubmissionV1::Accepted)
            .map_err(|e| format!("{e:?}"))?;
        flow.accept_completion(
            flow.binding().clone(),
            FreshAdmissionDurableOutcomeV1::Committed(receipt),
        )
        .map_err(|e| format!("{e:?}"))?;
    }
    flow.adopt(source, 100).map_err(|e| format!("{e:?}"))?;
    assert!(flow.controller().is_some());
    Ok(flow)
}
#[test]
fn fresh_postcommit_independent_guard_mutations_clear_projection() -> TestResult {
    for reconcile in [false, true] {
        for field in 0..28 {
            let mut source = Independent::new()?;
            let mut flow = accepted(&mut source, reconcile)?;
            let index = match field {
                0..=8 => 0,
                9..=13 => 1,
                14..=23 => 2,
                _ => 3,
            };
            let row = source.rows[index].as_mut().ok_or("missing fixture row")?;
            match (&mut row.state, field) {
                (AdmissionAuthorityGuardStateV1::Account { presence, .. }, 0) => *presence = None,
                (AdmissionAuthorityGuardStateV1::Account { presence, .. }, 1) => {
                    *presence = Some((
                        source.source.current.character_id,
                        GameSessionId::decode(&id(8))?,
                    ))
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 2) => {
                    security.allowed = false
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 3) => {
                    security.minimum_generation = 2
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 4) => {
                    security.account_id.push('x')
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 5) => {
                    security.provenance.source_revision -= 1
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 6) => {
                    security.provenance.source_observed_at = 101
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 7) => {
                    security.provenance.source_authority = "other-authority".into()
                }
                (AdmissionAuthorityGuardStateV1::Account { security, .. }, 8) => {
                    security.provenance.source_observed_at = 94
                }
                (AdmissionAuthorityGuardStateV1::Character { account_id, .. }, 9) => {
                    account_id.push('x')
                }
                (AdmissionAuthorityGuardStateV1::Character { world_id, .. }, 10) => {
                    *world_id = WorldId::decode(&id(8))?
                }
                (AdmissionAuthorityGuardStateV1::Character { eligible, .. }, 11) => {
                    *eligible = false
                }
                (
                    AdmissionAuthorityGuardStateV1::Character {
                        lease_generation, ..
                    },
                    12,
                ) => *lease_generation = 3,
                (AdmissionAuthorityGuardStateV1::Character { holder, .. }, 13) => *holder = None,
                (AdmissionAuthorityGuardStateV1::Runtime { ready, .. }, 14) => *ready = false,
                (
                    AdmissionAuthorityGuardStateV1::Runtime {
                        ownership_generation,
                        ..
                    },
                    15,
                ) => *ownership_generation = 2,
                (AdmissionAuthorityGuardStateV1::Runtime { route_revision, .. }, 16) => {
                    *route_revision = "route-2".into()
                }
                (
                    AdmissionAuthorityGuardStateV1::Runtime {
                        runtime_observation_revision,
                        ..
                    },
                    17,
                ) => *runtime_observation_revision = "runtime-2".into(),
                (
                    AdmissionAuthorityGuardStateV1::Runtime {
                        ruleset_revision, ..
                    },
                    18,
                ) => *ruleset_revision = "rules-2".into(),
                (
                    AdmissionAuthorityGuardStateV1::Runtime {
                        content_revision, ..
                    },
                    19,
                ) => *content_revision = "content-2".into(),
                (AdmissionAuthorityGuardStateV1::Runtime { map_revision, .. }, 20) => {
                    *map_revision = "map-2".into()
                }
                (
                    AdmissionAuthorityGuardStateV1::Runtime {
                        world_policy_revision,
                        ..
                    },
                    21,
                ) => *world_policy_revision = "policy-2".into(),
                (AdmissionAuthorityGuardStateV1::Runtime { offer_revision, .. }, 22) => {
                    *offer_revision = "offer-2".into()
                }
                (
                    AdmissionAuthorityGuardStateV1::Runtime {
                        transport_profile, ..
                    },
                    23,
                ) => *transport_profile = 2,
                (AdmissionAuthorityGuardStateV1::SigningTrust { trusted, .. }, 24) => {
                    *trusted = false
                }
                (AdmissionAuthorityGuardStateV1::SigningTrust { public_key, .. }, 25) => {
                    *public_key = [8; 32]
                }
                (_, 26) => row.source.source_observed_at = 94,
                (_, 27) => row.source.source_revision = 6,
                _ => unreachable!(),
            }
            assert!(
                flow.adopt(&source, 100).is_err(),
                "guard field {field}, reconcile {reconcile}"
            );
            assert!(
                flow.controller().is_none(),
                "guard field {field}, reconcile {reconcile}"
            );
            assert!(flow.receipt().is_some());
        }
    }
    Ok(())
}
#[test]
fn fresh_current_session_mutations_never_reactivate_or_rollback() -> TestResult {
    for reconcile in [false, true] {
        for field in 0..9 {
            let mut source = Independent::new()?;
            let mut flow = accepted(&mut source, reconcile)?;
            let s = source.snapshot;
            source.snapshot = GameSessionAuthoritySnapshot::from_current_facts(
                s.commit(),
                match field {
                    0 => GameSessionState::Terminal,
                    1 => GameSessionState::Reconnectable,
                    _ => s.session_state(),
                },
                ConnectionGeneration::new(if field == 2 { 2 } else { 1 })?,
                match field {
                    3 => None,
                    4 => Some(AuthenticatedTransportRefV1::decode(&[8; 16])?),
                    _ => s.current_transport(),
                },
                CharacterLease::new(
                    s.current_character_lease().character_id(),
                    if field == 5 { 3 } else { 2 },
                )?,
                if field == 6 {
                    None
                } else {
                    s.current_character_world_eligibility()
                },
                if field == 7 {
                    RuntimeScopeRefV1::channel(
                        source.source.current.world_id,
                        ChannelId::decode(&id(8))?,
                    )
                } else {
                    s.current_runtime_scope()
                },
                ScopeOwnershipGeneration::new(if field == 8 { 2 } else { 1 })?,
            )
            .map_err(|e| format!("{e:?}"))?;
            assert!(
                flow.adopt(&source, 100).is_err(),
                "session field {field}, reconcile {reconcile}"
            );
            assert!(flow.controller().is_none());
            assert!(flow.receipt().is_some());
        }
    }
    Ok(())
}
#[test]
fn fresh_restart_without_current_floor_stays_closed() -> TestResult {
    let mut source = Independent::new()?;
    let mut flow = accepted(&mut source, true)?;
    source.rows[0] = None;
    assert!(flow.adopt(&source, 100).is_err());
    assert!(flow.controller().is_none());
    Ok(())
}
#[test]
fn fresh_equal_game_source_revision_cannot_publish_acquired_state() -> TestResult {
    let mut source = Independent::new()?;
    let mut flow = accepted(&mut source, false)?;
    let row = source.rows[1].as_mut().ok_or("missing fixture")?;
    row.source.source_revision = 7;
    row.source.decision_identity = "source-decision-seven".into();
    assert_eq!(
        flow.adopt(&source, 100),
        Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
    );
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn fresh_equal_platform_revision_cannot_change_security_decision() -> TestResult {
    let mut source = Independent::new()?;
    let auth = source.authorize_payload(fresh_payload().replace(
        "\"account_security_generation\":\"1\"",
        "\"account_security_generation\":\"2\"",
    ))?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(auth);
    let receipt = committed(&flow).map_err(|e| format!("{e:?}"))?;
    flow.accept_submission(FreshAdmissionSubmissionV1::Accepted)
        .map_err(|e| format!("{e:?}"))?;
    flow.accept_completion(
        flow.binding().clone(),
        FreshAdmissionDurableOutcomeV1::Committed(receipt),
    )
    .map_err(|e| format!("{e:?}"))?;
    source.commit_sources()?;
    flow.adopt(&source, 100).map_err(|e| format!("{e:?}"))?;
    if let Some(AdmissionAuthorityPublicationChangeV1 {
        state: AdmissionAuthorityGuardStateV1::Account { security, .. },
        ..
    }) = &mut source.rows[0]
    {
        security.minimum_generation = 2;
    }
    assert_eq!(
        flow.adopt(&source, 100),
        Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
    );
    assert!(flow.controller().is_none());
    Ok(())
}
#[test]
fn fresh_restart_without_original_live_transport_cannot_rebind_reference() -> TestResult {
    let mut source = Independent::new()?;
    let mut flow = accepted(&mut source, true)?;
    source.live_transport = false;
    assert_eq!(
        flow.adopt(&source, 100),
        Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
    );
    assert!(flow.controller().is_none());
    Ok(())
}
#[test]
fn fresh_historical_restore_cannot_extend_original_deadline() -> TestResult {
    let source = Independent::new()?;
    let flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let mut binding = flow.binding().clone();
    binding.accepted_deadline += 1;
    assert_eq!(
        FreshAdmissionCommitReceiptV1::restore(binding, 100),
        Err(FreshAdmissionDurabilityErrorV1::Invalid)
    );
    Ok(())
}

#[test]
fn fresh_delayed_commit_does_not_reage_receipt_or_require_unoccupied_presence() -> TestResult {
    let mut source = Independent::new()?;
    let mut flow = accepted(&mut source, false)?;
    assert_eq!(
        flow.adopt(&source, 99),
        Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
    );
    for row in source.rows.iter_mut().flatten() {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: row.publication_revision,
        };
        row.publication_revision += 1;
        row.source.source_revision += 1;
        row.source.source_observed_at = 200;
        row.source.decision_identity = "current-200".into();
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
            security.provenance.publication_revision = row.publication_revision;
            security.provenance.source_revision += 1;
            security.provenance.accepted_source_revision += 1;
            security.provenance.source_observed_at = 200;
            security.provenance.decision_identity = "security-200".into();
            security.provenance.accepted_decision_identity = "security-200".into();
        }
    }
    flow.adopt(&source, 200).map_err(|e| format!("{e:?}"))?;
    assert!(flow.controller().is_some());
    assert_eq!(
        flow.receipt()
            .map(FreshAdmissionCommitReceiptV1::decided_at),
        Some(100)
    );
    assert_eq!(flow.binding().accepted_deadline, 103);
    Ok(())
}

#[test]
fn fresh_authorization_rejects_current_guard_changes_and_absence() -> TestResult {
    for field in 0..7 {
        let mut source = Independent::new()?;
        assert!(source.authorize().is_ok());
        match field {
            0..=3 => source.rows[field] = None,
            4 => {
                if let Some(AdmissionAuthorityPublicationChangeV1 {
                    state: AdmissionAuthorityGuardStateV1::Runtime { protocol_major, .. },
                    ..
                }) = &mut source.rows[2]
                {
                    *protocol_major = 2;
                }
            }
            5 => {
                if let Some(AdmissionAuthorityPublicationChangeV1 {
                    state: AdmissionAuthorityGuardStateV1::Account { presence, .. },
                    ..
                }) = &mut source.rows[0]
                {
                    *presence = Some((
                        source.source.current.character_id,
                        GameSessionId::decode(&id(8))?,
                    ));
                }
            }
            _ => {
                if let Some(row) = &mut source.rows[1] {
                    row.publication_revision += 1;
                    row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                        expected_publication_revision: 13,
                    };
                }
            }
        }
        assert!(source.authorize().is_err(), "authorization field {field}");
    }
    Ok(())
}
#[test]
fn fresh_final_guarded_decision_rechecks_original_deadline_and_exact_rows() -> TestResult {
    let source = Independent::new()?;
    let auth = source.authorize()?;
    assert_eq!(auth.validate_at_decision(&source.rows, Some(100)), Ok(()));
    assert_eq!(auth.validate_at_decision(&source.rows, Some(103)), Ok(()));
    for now in [None, Some(99), Some(104), Some(i64::MAX)] {
        assert_eq!(
            auth.validate_at_decision(&source.rows, now),
            Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
        );
    }
    let mut rows = source.rows.clone();
    rows[1] = None;
    assert_eq!(
        auth.validate_at_decision(&rows, Some(100)),
        Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
    );
    Ok(())
}

#[test]
fn fresh_unavailable_reconciliation_yields_and_retries_only_original() -> TestResult {
    let source = Independent::new()?;
    let binding = source.authorize()?.binding().clone();
    let mut flow = FreshAdmissionDurabilityFlowV1::resume_reconciliation(binding.clone());
    let mut queue = Queue {
        requests: vec![],
        available: true,
    };
    flow.reconcile(&mut queue).map_err(|e| format!("{e:?}"))?;
    flow.accept_reconciliation_unavailable(&binding)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(flow.phase(), FreshAdmissionPhaseV1::ReconciliationRequired);
    flow.reconcile(&mut queue).map_err(|e| format!("{e:?}"))?;
    assert_eq!(queue.requests, vec![binding.clone(), binding]);
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn fresh_queue_retains_owned_authorization_for_later_guarded_decision() -> TestResult {
    struct OwnedQueue(Option<FreshAdmissionCommitRequestV1>);
    impl FreshAdmissionDurabilityPortV1 for OwnedQueue {
        fn submit(
            &mut self,
            request: &FreshAdmissionCommitRequestV1,
        ) -> FreshAdmissionSubmissionV1 {
            self.0 = Some(request.clone());
            FreshAdmissionSubmissionV1::Accepted
        }
        fn reconcile(&mut self, _: &FreshAdmissionAuditBindingV1) -> FreshAdmissionSubmissionV1 {
            FreshAdmissionSubmissionV1::Unavailable
        }
    }
    let source = Independent::new()?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(source.authorize()?);
    let mut queue = OwnedQueue(None);
    flow.submit(&mut queue).map_err(|e| format!("{e:?}"))?;
    let retained = queue.0.ok_or("queue lost owned request")?;
    assert_eq!(retained.binding(), flow.binding());
    assert_eq!(
        retained.validate_at_decision(&source.rows, Some(103)),
        Ok(())
    );
    assert_eq!(
        retained.validate_at_decision(&source.rows, Some(104)),
        Err(FreshAdmissionDurabilityErrorV1::StaleAuthority)
    );
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn fresh_authorization_signing_guard_must_match_verified_source_revision() -> TestResult {
    let mut source = Independent::new()?;
    assert!(source.authorize().is_ok());
    if let Some(row) = &mut source.rows[3] {
        row.source.source_revision += 1;
    }
    assert!(source.authorize().is_err());
    Ok(())
}
