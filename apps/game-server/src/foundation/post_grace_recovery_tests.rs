use super::*;

fn provenance(purpose: FreshEvidencePurposeV1) -> FreshEvidenceProvenanceV1 {
    FreshEvidenceProvenanceV1 {
        source_authority: "recovery-producer".to_owned(),
        purpose,
        scope: Fnd04EvidenceScope::ExistingActorRecovery,
        source_revision: 7,
        accepted_source_revision: 7,
        decision_identity: "decision-7".to_owned(),
        accepted_decision_identity: "decision-7".to_owned(),
        source_observed_at: 100,
        clock_uncertainty_seconds: 2,
        publication_revision: 9,
    }
}

#[test]
fn post_grace_recovery_source_deadline_is_scoped_conservative_and_checked() {
    let valid = provenance(FreshEvidencePurposeV1::PlatformSecurity);
    assert_eq!(
        recovery_source_deadline(&valid, FreshEvidencePurposeV1::PlatformSecurity, 103),
        Ok(103)
    );
    let mut wrong = valid.clone();
    wrong.scope = Fnd04EvidenceScope::FreshAdmission;
    assert_eq!(
        recovery_source_deadline(&wrong, FreshEvidencePurposeV1::PlatformSecurity, 100),
        Err(Fnd04ConsumerError::RecoverySecurityEvidenceStale)
    );
    let mut overflow = valid.clone();
    overflow.source_observed_at = i64::MAX;
    assert!(
        recovery_source_deadline(
            &overflow,
            FreshEvidencePurposeV1::PlatformSecurity,
            i64::MAX
        )
        .is_err()
    );
    assert!(
        recovery_source_deadline(&valid, FreshEvidencePurposeV1::PlatformSecurity, 104).is_err()
    );
    assert!(
        recovery_source_deadline(&valid, FreshEvidencePurposeV1::PlatformSecurity, 99).is_err()
    );
}

#[derive(Clone)]
struct RecoverySource {
    signing: RecoverySigningTrustObservationV2,
    security: RecoveryAccountSecurityObservationV2,
}
impl recovery_source_sealed::Sealed for RecoverySource {}
impl RecoveryDurabilityEvidenceSourceV2 for RecoverySource {
    fn signing_trust(
        &self,
        _: &str,
        _: i64,
    ) -> Result<RecoverySigningTrustObservationV2, Fnd04EvidenceError> {
        Ok(self.signing.clone())
    }
    fn account_security(
        &self,
        _: &str,
        _: i64,
    ) -> Result<RecoveryAccountSecurityObservationV2, Fnd04EvidenceError> {
        Ok(self.security.clone())
    }
}
fn fixture() -> Result<(String, RecoverySource, RecoveryCurrentEvidence), Fnd04ConsumerError> {
    use ed25519_dalek::{Signer, SigningKey};
    let key = SigningKey::from_bytes(&[23; 32]);
    let header = r#"{"alg":"Ed25519","kid":"recovery-1","typ":"oteryn-recovery+jwt"}"#;
    let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([8; 32]);
    let payload = format!(
        r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"00000000-0000-4000-8000-000000000001","character_id":"00000000-0000-7000-8000-000000000002","world_id":"00000000-0000-7000-8000-000000000003","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"}}"#
    );
    let input = format!(
        "{}.{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload)
    );
    let token = format!(
        "{input}.{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(key.sign(input.as_bytes()).to_bytes())
    );
    let source = RecoverySource {
        signing: RecoverySigningTrustObservationV2 {
            key_id: "recovery-1".into(),
            public_key: key.verifying_key().to_bytes(),
            trusted: true,
            provenance: provenance(FreshEvidencePurposeV1::SigningTrust),
        },
        security: RecoveryAccountSecurityObservationV2 {
            account_id: "00000000-0000-4000-8000-000000000001".into(),
            minimum_generation: 1,
            allowed: true,
            provenance: provenance(FreshEvidencePurposeV1::PlatformSecurity),
        },
    };
    let mut character = [0; 16];
    character[6] = 0x70;
    character[8] = 0x80;
    character[15] = 2;
    let mut world = character;
    world[15] = 3;
    let current = RecoveryCurrentEvidence {
        account_id: "00000000-0000-4000-8000-000000000001".into(),
        character_id: CharacterId::decode(&character)
            .map_err(|_| Fnd04ConsumerError::RecoveryMalformed)?,
        world_id: WorldId::decode(&world).map_err(|_| Fnd04ConsumerError::RecoveryMalformed)?,
        ruleset_revision: "rules-1".into(),
        content_revision: "content-1".into(),
        map_revision: "map-1".into(),
        world_policy_revision: "policy-1".into(),
    };
    Ok((token, source, current))
}
#[test]
fn post_grace_verified_recovery_preserves_deadline_and_rejects_fresh_scope()
-> Result<(), Fnd04ConsumerError> {
    let (token, source, current) = fixture()?;
    let verified = verify_recovery_grant_durability_v2(
        &token,
        100,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &current,
    )?;
    assert_eq!(verified.accepted_deadline(), 103);
    assert_eq!(verified.verified_at(), 100);
    assert_eq!(verified.facts().grant_nonce(), [8; 32]);
    let mut wrong = source.clone();
    wrong.signing.provenance.scope = Fnd04EvidenceScope::FreshAdmission;
    assert!(
        verify_recovery_grant_durability_v2(
            &token,
            100,
            &RecoveryDurabilityTrustContextV2::from_owning_source(&wrong),
            &current
        )
        .is_err()
    );
    assert!(
        verify_recovery_grant_durability_v2(
            &token,
            100,
            &RecoveryDurabilityTrustContextV2::unavailable(),
            &current
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn post_grace_revalidation_consumes_new_denial_and_rejects_same_revision_contradiction()
-> Result<(), Fnd04ConsumerError> {
    let (token, source, current) = fixture()?;
    let verified = verify_recovery_grant_durability_v2(
        &token,
        100,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &current,
    )?;
    let mut denied = source.clone();
    denied.security.allowed = false;
    denied.security.provenance.source_revision += 1;
    denied.security.provenance.accepted_source_revision += 1;
    denied.security.provenance.publication_revision += 1;
    denied.security.provenance.decision_identity = "denied-8".into();
    denied.security.provenance.accepted_decision_identity = "denied-8".into();
    assert_eq!(
        verified
            .revalidate(
                101,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&denied),
                &current
            )
            .err(),
        Some(Fnd04ConsumerError::RecoverySecurityStateRevoked)
    );
    let mut contradiction = source.clone();
    contradiction.security.provenance.decision_identity = "contradictory".into();
    contradiction.security.provenance.accepted_decision_identity = "contradictory".into();
    assert_eq!(
        verified
            .revalidate(
                101,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&contradiction),
                &current
            )
            .err(),
        Some(Fnd04ConsumerError::RecoverySecurityEvidenceStale)
    );
    assert!(
        verified
            .revalidate(
                99,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &current
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_exact_source_replay_allows_only_local_publication_advance()
-> Result<(), Fnd04ConsumerError> {
    let (token, source, current) = fixture()?;
    let verified = verify_recovery_grant_durability_v2(
        &token,
        100,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &current,
    )?;
    let mut replay = source.clone();
    replay.security.provenance.publication_revision += 1;
    replay.signing.provenance.publication_revision += 1;
    let refreshed = verified.revalidate(
        101,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&replay),
        &current,
    )?;
    assert_eq!(refreshed.accepted_deadline(), 103);
    assert_eq!(refreshed.security().provenance.publication_revision, 10);
    Ok(())
}

#[test]
fn post_grace_source_replay_never_normalizes_source_owned_fields() -> Result<(), Fnd04ConsumerError>
{
    let (token, source, current) = fixture()?;
    let verified = verify_recovery_grant_durability_v2(
        &token,
        100,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &current,
    )?;
    let mutations: [fn(&mut RecoverySource); 6] = [
        |s| s.security.provenance.source_observed_at += 1,
        |s| s.security.provenance.clock_uncertainty_seconds = 1,
        |s| s.security.provenance.publication_revision -= 1,
        |s| {
            s.security.provenance.source_revision += 1;
            s.security.provenance.accepted_source_revision += 1;
            s.security.provenance.decision_identity = "decision-8".into();
            s.security.provenance.accepted_decision_identity = "decision-8".into();
        },
        |s| {
            s.security.provenance.source_revision += 1;
            s.security.provenance.accepted_source_revision += 1;
            s.security.provenance.publication_revision += 1;
        },
        |s| s.signing.provenance.source_authority = "other-producer".into(),
    ];
    for change in mutations {
        let mut changed = source.clone();
        change(&mut changed);
        assert_eq!(
            verified
                .revalidate(
                    101,
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&changed),
                    &current
                )
                .err(),
            Some(Fnd04ConsumerError::RecoverySecurityEvidenceStale)
        );
    }
    Ok(())
}

#[test]
fn post_grace_rejects_zero_source_security_generation() -> Result<(), Fnd04ConsumerError> {
    let (token, mut source, current) = fixture()?;
    source.security.minimum_generation = 0;
    assert!(
        verify_recovery_grant_durability_v2(
            &token,
            100,
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &current
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn post_grace_preserves_signed_credential_attempt_identity() -> Result<(), Fnd04ConsumerError> {
    let (token, source, current) = fixture()?;
    let verified = verify_recovery_grant_durability_v2(
        &token,
        100,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &current,
    )?;
    let mut expected = [0; 16];
    expected[6] = 0x70;
    expected[8] = 0x80;
    expected[15] = 1;
    assert_eq!(verified.credential_attempt_ref(), expected);
    Ok(())
}

#[test]
fn post_grace_retained_budget_requires_completeness_and_preserves_finality()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let epoch = ControlLossEpochRefV1::new(4).map_err(|_| "epoch")?;
    let entries = Vec::new();
    assert!(
        RetainedRecoveryBudgetV1::restore(
            epoch,
            RecoveryEpochStateV1::Open,
            false,
            entries.clone()
        )
        .is_err()
    );
    let closed = RetainedRecoveryBudgetV1::restore(
        epoch,
        RecoveryEpochStateV1::Restored,
        true,
        entries.clone(),
    )
    .map_err(|_| "closed history")?;
    let attempt = ReconnectAttemptRef::new(1)?;
    let transport = AuthenticatedTransportRefV1::decode(&[1; 16]).map_err(|_| "transport")?;
    assert!(closed.check_candidate(attempt, transport).is_err());
    let open = RetainedRecoveryBudgetV1::restore(epoch, RecoveryEpochStateV1::Open, true, entries)
        .map_err(|_| "open history")?;
    assert_eq!(
        open.check_candidate(attempt, transport),
        Ok(ReconnectAttemptReservationV1::New)
    );
    Ok(())
}

#[test]
fn post_grace_budget_counts_eight_across_sessions_without_reviving_terminal_attempts()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let mut entries = Vec::new();
    for number in 1..=8 {
        entries.push(RetainedRecoveryAttemptV1 {
            attempt: ReconnectAttemptRef::new(number)?,
            transport: AuthenticatedTransportRefV1::decode(&[number as u8; 16])
                .map_err(|_| "transport")?,
            disposition: if number == 8 {
                RetainedRecoveryAttemptDispositionV1::Prepared
            } else {
                RetainedRecoveryAttemptDispositionV1::Terminal
            },
        });
    }
    let epoch = ControlLossEpochRefV1::new(4).map_err(|_| "epoch")?;
    let budget =
        RetainedRecoveryBudgetV1::restore(epoch, RecoveryEpochStateV1::Open, true, entries.clone())
            .map_err(|_| "budget")?;
    assert_eq!(
        budget.check_candidate(entries[7].attempt, entries[7].transport),
        Ok(ReconnectAttemptReservationV1::Existing)
    );
    assert!(
        budget
            .check_candidate(entries[0].attempt, entries[0].transport)
            .is_err()
    );
    assert_eq!(
        budget.check_candidate(ReconnectAttemptRef::new(9)?, entries[0].transport),
        Err(ReconnectDurabilityErrorV1::AttemptCapacityExceeded)
    );
    entries.push(RetainedRecoveryAttemptV1 {
        attempt: ReconnectAttemptRef::new(9)?,
        ..entries[0]
    });
    assert!(
        RetainedRecoveryBudgetV1::restore(epoch, RecoveryEpochStateV1::Open, true, entries)
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_closed_timing_keeps_historical_grace_separate() {
    use crate::foundation::*;
    let timing = RecoveryTimingV2::TerminalSessionPostGrace {
        original_grace_deadline: 99,
        attempt_deadline: 103,
    };
    assert!(timing.validate_at(100).is_ok());
    assert!(timing.validate_at(99).is_err());
    assert!(timing.validate_at(104).is_err());
    assert!(
        RecoveryTimingV2::TerminalSessionPostGrace {
            original_grace_deadline: 103,
            attempt_deadline: 103
        }
        .validate_at(103)
        .is_err()
    );
}

#[derive(Clone)]
struct ActorSource(crate::foundation::PostGraceActorObservationV1);
impl recovery_source_sealed::Sealed for ActorSource {}
impl crate::foundation::PostGraceActorSourceV1 for ActorSource {
    fn resolve_current_actor(
        &self,
        _: &str,
        _: CharacterId,
        _: i64,
    ) -> Result<
        crate::foundation::PostGraceActorObservationV1,
        crate::foundation::ReconnectDurabilityErrorV1,
    > {
        Ok(self.0.clone())
    }
}
fn actor_fixture() -> Result<ActorSource, Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (_, _, current) = fixture().map_err(|_| "credential fixture")?;
    let id = |last| {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    };
    let channel = ChannelId::decode(&id(4)).map_err(|_| "channel")?;
    let commit = FreshAdmissionCommit::from_facts(
        GameSessionId::decode(&id(5)).map_err(|_| "session")?,
        FreshAdmissionFacts::new(
            [3; 32],
            current.character_id,
            current.world_id,
            channel,
            2,
            3,
        )?,
        AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(|_| "transport")?,
    )?;
    let epoch = ControlLossEpochRefV1::new(4).map_err(|_| "epoch")?;
    let snapshot = GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Terminal,
        ConnectionGeneration::new(7).map_err(|_| "generation")?,
        None,
        CharacterLease::new(current.character_id, 2)?,
        Some(CharacterWorldEligibilityClaimV1::new(
            current.character_id,
            current.world_id,
        )),
        RuntimeScopeRefV1::channel(current.world_id, channel),
        ScopeOwnershipGeneration::new(3).map_err(|_| "scope")?,
    )
    .map_err(|_| "snapshot")?
    .with_control_loss_continuity(epoch, 99)
    .map_err(|_| "continuity")?;
    Ok(ActorSource(PostGraceActorObservationV1 {
        source_authority: "game-actor-owner".into(),
        source_revision: 11,
        accepted_source_revision: 11,
        decision_identity: "actor-11".into(),
        accepted_decision_identity: "actor-11".into(),
        source_observed_at: 100,
        account_presence: Some(
            AccountPresenceClaimV1::new(&current.account_id, current.character_id)
                .map_err(|_| "presence")?,
        ),
        current,
        predecessor: snapshot,
        present_uncontrolled: true,
        runtime_ready: true,
        reconciliation: Fnd02ReconciliationFenceV1::new(
            CommandId::new(1).map_err(|_| "command")?,
            vec![],
            4,
            vec![],
        )
        .map_err(|_| "reconciliation")?,
        placement_identity: [5; 16],
        placement_revision: 2,
        account_security_source_revision: 7,
        budget: RetainedRecoveryBudgetV1::restore(epoch, RecoveryEpochStateV1::Open, true, vec![])
            .map_err(|_| "budget")?,
        protection: Some(RecoveryProtectionContinuityV1 {
            usage: RecoveryProtectionUseV1::Unused {
                entitlement_generation: 1,
            },
            rearm: RecoveryProtectionRearmV1::Satisfied {
                generation: 7,
                established_at: 90,
            },
        }),
    }))
}
#[test]
fn post_grace_live_preparation_requires_sealed_current_actor_and_freezes_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (token, source, current) = fixture().map_err(|_| "fixture")?;
    let trust = RecoveryDurabilityTrustContextV2::from_owning_source(&source);
    let verified =
        verify_recovery_grant_durability_v2(&token, 100, &trust, &current).map_err(|_| "verify")?;
    let actor = actor_fixture()?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 6;
    let candidate = GameSessionId::decode(&id).map_err(|_| "candidate")?;
    let attempt = ReconnectAttemptRef::new(1)?;
    let transport = AuthenticatedTransportRefV1::decode(&[6; 16]).map_err(|_| "transport")?;
    let prepared = PostGraceRecoveryAuthorizationV1::prepare(
        &verified,
        &trust,
        &PostGraceActorAuthorityV1::from_owning_source(&actor),
        candidate,
        attempt,
        transport,
        100,
    )
    .map_err(|_| "prepare")?;
    assert_eq!(prepared.attempt_deadline(), 103);
    assert_eq!(prepared.candidate_generation().get(), 1);
    assert_eq!(
        prepared.predecessor().current_connection_generation().get(),
        7
    );
    assert!(
        PostGraceRecoveryAuthorizationV1::prepare(
            &verified,
            &trust,
            &PostGraceActorAuthorityV1::unavailable(),
            candidate,
            attempt,
            transport,
            100
        )
        .is_err()
    );
    let mut absent = actor.clone();
    absent.0.present_uncontrolled = false;
    assert!(
        prepared
            .revalidate(
                &trust,
                &PostGraceActorAuthorityV1::from_owning_source(&absent),
                101
            )
            .is_err()
    );
    let mut closed = actor.clone();
    closed.0.budget = RetainedRecoveryBudgetV1::restore(
        closed.0.budget.epoch(),
        RecoveryEpochStateV1::Restored,
        true,
        vec![],
    )
    .map_err(|_| "closed budget")?;
    assert!(
        prepared
            .revalidate(
                &trust,
                &PostGraceActorAuthorityV1::from_owning_source(&closed),
                101
            )
            .is_err()
    );
    assert!(
        prepared
            .revalidate(
                &trust,
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                104
            )
            .is_err()
    );
    Ok(())
}

fn prepared_fixture() -> Result<
    (
        crate::foundation::PostGraceRecoveryAuthorizationV1,
        RecoverySource,
        ActorSource,
    ),
    Box<dyn std::error::Error>,
> {
    use crate::foundation::*;
    let (token, source, current) = fixture().map_err(|_| "fixture")?;
    let trust = RecoveryDurabilityTrustContextV2::from_owning_source(&source);
    let verified =
        verify_recovery_grant_durability_v2(&token, 100, &trust, &current).map_err(|_| "verify")?;
    let actor = actor_fixture()?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 6;
    let prepared = PostGraceRecoveryAuthorizationV1::prepare(
        &verified,
        &trust,
        &PostGraceActorAuthorityV1::from_owning_source(&actor),
        GameSessionId::decode(&id).map_err(|_| "candidate")?,
        ReconnectAttemptRef::new(1)?,
        AuthenticatedTransportRefV1::decode(&[6; 16]).map_err(|_| "transport")?,
        100,
    )
    .map_err(|_| "prepare")?;
    Ok((prepared, source, actor))
}
#[test]
fn post_grace_current_actor_mutations_are_independent_of_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (prepared, source, actor) = prepared_fixture()?;
    let changes: [fn(&mut PostGraceActorObservationV1); 7] = [
        |a| a.present_uncontrolled = false,
        |a| a.placement_identity = [9; 16],
        |a| a.placement_revision += 1,
        |a| a.account_presence = None,
        |a| a.protection = None,
        |a| a.account_security_source_revision += 1,
        |a| a.current.ruleset_revision = "changed-rules".into(),
    ];
    for change in changes {
        let mut changed = actor.clone();
        changed.0.source_revision += 1;
        changed.0.accepted_source_revision += 1;
        changed.0.decision_identity = "actor-12".into();
        changed.0.accepted_decision_identity = "actor-12".into();
        changed.0.source_observed_at = 101;
        change(&mut changed.0);
        assert!(
            prepared
                .revalidate(
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                    &PostGraceActorAuthorityV1::from_owning_source(&changed),
                    101
                )
                .is_err()
        );
    }
    let mut same_revision = actor.clone();
    same_revision.0.decision_identity = "contradiction".into();
    same_revision.0.accepted_decision_identity = "contradiction".into();
    assert!(
        prepared
            .revalidate(
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&same_revision),
                101
            )
            .is_err()
    );
    Ok(())
}
#[test]
fn post_grace_refresh_cannot_extend_the_frozen_attempt() -> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (prepared, mut source, mut actor) = prepared_fixture()?;
    for p in [
        &mut source.security.provenance,
        &mut source.signing.provenance,
    ] {
        p.source_revision += 1;
        p.accepted_source_revision += 1;
        p.publication_revision += 1;
        p.decision_identity = "source-8".into();
        p.accepted_decision_identity = "source-8".into();
        p.source_observed_at = 102;
    }
    actor.0.source_revision += 1;
    actor.0.accepted_source_revision += 1;
    actor.0.decision_identity = "actor-12".into();
    actor.0.accepted_decision_identity = "actor-12".into();
    actor.0.source_observed_at = 102;
    actor.0.account_security_source_revision = 8;
    let refreshed = prepared
        .revalidate(
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &PostGraceActorAuthorityV1::from_owning_source(&actor),
            102,
        )
        .map_err(|_| "refresh")?;
    assert_eq!(refreshed.attempt_deadline(), 103);
    assert!(
        refreshed
            .revalidate(
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                104
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_revalidation_cannot_drop_its_retained_attempt()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (initial, source, mut actor) = prepared_fixture()?;
    actor.0.budget = RetainedRecoveryBudgetV1::restore(
        actor.0.budget.epoch(),
        RecoveryEpochStateV1::Open,
        true,
        vec![RetainedRecoveryAttemptV1 {
            attempt: initial.attempt(),
            transport: initial.transport(),
            disposition: RetainedRecoveryAttemptDispositionV1::Prepared,
        }],
    )
    .map_err(|_| "budget")?;
    let prepared = PostGraceRecoveryAuthorizationV1::prepare(
        initial.verified(),
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &PostGraceActorAuthorityV1::from_owning_source(&actor),
        initial.candidate(),
        initial.attempt(),
        initial.transport(),
        100,
    )
    .map_err(|_| "prepare existing")?;
    actor.0.budget = RetainedRecoveryBudgetV1::restore(
        actor.0.budget.epoch(),
        RecoveryEpochStateV1::Open,
        true,
        vec![],
    )
    .map_err(|_| "dropped budget")?;
    actor.0.source_revision += 1;
    actor.0.accepted_source_revision += 1;
    actor.0.decision_identity = "actor-12".into();
    actor.0.accepted_decision_identity = "actor-12".into();
    actor.0.source_observed_at = 101;
    assert!(
        prepared
            .revalidate(
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                101
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_newer_security_observation_cannot_lower_generation_floor()
-> Result<(), Box<dyn std::error::Error>> {
    use ed25519_dalek::{Signer, SigningKey};
    let (token, mut source, current) = fixture().map_err(|_| "fixture")?;
    let segments: Vec<_> = token.split('.').collect();
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(segments[1])?;
    let payload = String::from_utf8(decoded)?.replace(
        "\"account_security_generation\":\"1\"",
        "\"account_security_generation\":\"2\"",
    );
    let input = format!(
        "{}.{}",
        segments[0],
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload)
    );
    let token = format!(
        "{input}.{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            SigningKey::from_bytes(&[23; 32])
                .sign(input.as_bytes())
                .to_bytes()
        )
    );
    source.security.minimum_generation = 2;
    let verified = verify_recovery_grant_durability_v2(
        &token,
        100,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &current,
    )
    .map_err(|_| "verify")?;
    source.security.minimum_generation = 1;
    source.security.provenance.source_revision += 1;
    source.security.provenance.accepted_source_revision += 1;
    source.security.provenance.publication_revision += 1;
    source.security.provenance.decision_identity = "source-8".into();
    source.security.provenance.accepted_decision_identity = "source-8".into();
    assert!(
        verified
            .revalidate(
                101,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &current
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_current_fnd02_and_runtime_readiness_are_not_reconstructed()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (prepared, source, actor) = prepared_fixture()?;
    let mut changed = actor.clone();
    changed.0.reconciliation = Fnd02ReconciliationFenceV1::new(
        CommandId::new(1).map_err(|_| "command")?,
        vec![],
        99,
        vec![],
    )
    .map_err(|_| "reconciliation")?;
    changed.0.source_revision += 1;
    changed.0.accepted_source_revision += 1;
    changed.0.decision_identity = "actor-12".into();
    changed.0.accepted_decision_identity = "actor-12".into();
    assert!(
        prepared
            .revalidate(
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&changed),
                101
            )
            .is_err()
    );
    let mut unready = actor.clone();
    unready.0.runtime_ready = false;
    assert!(
        prepared
            .revalidate(
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&unready),
                101
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_original_operation_is_immutable_across_source_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (prepared, mut source, mut actor) = prepared_fixture()?;
    let original = prepared.operation().clone();
    for p in [
        &mut source.security.provenance,
        &mut source.signing.provenance,
    ] {
        p.source_revision += 1;
        p.accepted_source_revision += 1;
        p.publication_revision += 1;
        p.decision_identity = "source-8".into();
        p.accepted_decision_identity = "source-8".into();
        p.source_observed_at = 102;
    }
    actor.0.source_revision += 1;
    actor.0.accepted_source_revision += 1;
    actor.0.decision_identity = "actor-12".into();
    actor.0.accepted_decision_identity = "actor-12".into();
    actor.0.source_observed_at = 102;
    actor.0.account_security_source_revision = 8;
    let refreshed = prepared
        .revalidate(
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &PostGraceActorAuthorityV1::from_owning_source(&actor),
            102,
        )
        .map_err(|_| "refresh")?;
    assert_eq!(refreshed.operation(), &original);
    assert_ne!(
        refreshed.verified().security().provenance.source_revision,
        original.credential.security.provenance.source_revision
    );
    Ok(())
}

#[test]
fn post_grace_historical_operation_rejects_unknown_version_and_changed_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    let (prepared, _, _) = prepared_fixture()?;
    let original = prepared.operation();
    assert!(original.validate_historical().is_ok());
    let mut unknown = original.clone();
    unknown.version = 99;
    assert!(unknown.validate_historical().is_err());
    let mut changed = original.clone();
    changed.credential.accepted_deadline += 1;
    assert!(changed.validate_historical().is_err());
    Ok(())
}

#[derive(Clone)]
struct ClaimOwner {
    actor: ActorSource,
    transition:
        crate::foundation::admission_authority_publication::AdmissionClaimTransitionEvidenceV1,
}
impl recovery_source_sealed::Sealed for ClaimOwner {}
impl crate::foundation::admission_authority_publication::PostGraceClaimOwningSourceV1
    for ClaimOwner
{
    fn prepare_post_grace_claim(
        &self,
        _: &crate::foundation::PostGraceRecoveryOperationV1,
        _: i64,
    ) -> Result<
        crate::foundation::admission_authority_publication::PostGraceClaimResolutionV1,
        crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationErrorV1,
    > {
        Ok(
            crate::foundation::admission_authority_publication::PostGraceClaimResolutionV1 {
                current_actor: self.actor.0.clone(),
                transition: self.transition.clone(),
            },
        )
    }
}
fn claim_fixture() -> Result<ClaimOwner, Box<dyn std::error::Error>> {
    use crate::foundation::admission_authority_publication::*;
    use crate::foundation::*;
    let actor = actor_fixture()?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 6;
    let candidate = GameSessionId::decode(&id).map_err(|_| "candidate")?;
    let mut historical = provenance(FreshEvidencePurposeV1::PlatformSecurity);
    historical.scope = Fnd04EvidenceScope::FreshAdmission;
    historical.source_revision = 6;
    historical.accepted_source_revision = 6;
    historical.decision_identity = "source-6".into();
    historical.accepted_decision_identity = "source-6".into();
    historical.source_observed_at = 80;
    historical.clock_uncertainty_seconds = 0;
    let account = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Account {
            account_id: actor.0.current.account_id.clone(),
        },
        source: AdmissionPublicationSourceV1 {
            authority: "game-account-owner".into(),
            purpose: AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
            source_revision: 3,
            decision_identity: "owner-3".into(),
            source_observed_at: 90,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 8,
        },
        publication_revision: 9,
        state: AdmissionAuthorityGuardStateV1::Account {
            security: FreshAccountSecurityObservationV1 {
                account_id: actor.0.current.account_id.clone(),
                minimum_generation: 1,
                allowed: true,
                provenance: historical,
            },
            presence: Some((
                actor.0.current.character_id,
                actor.0.predecessor.commit().game_session_id(),
            )),
        },
    };
    let character = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Character(actor.0.current.character_id),
        source: AdmissionPublicationSourceV1 {
            authority: "game-character-owner".into(),
            purpose: AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
            source_revision: 4,
            decision_identity: "character-4".into(),
            source_observed_at: 90,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 11,
        },
        publication_revision: 12,
        state: AdmissionAuthorityGuardStateV1::Character {
            account_id: actor.0.current.account_id.clone(),
            world_id: actor.0.current.world_id,
            eligible: true,
            lease_generation: 2,
            holder: Some(actor.0.predecessor.commit().game_session_id()),
        },
    };
    let mut next_account = account.clone();
    next_account.publication_revision = 10;
    next_account.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: 9,
    };
    next_account.source.source_revision = 4;
    next_account.source.decision_identity = "owner-4".into();
    next_account.source.source_observed_at = 100;
    if let AdmissionAuthorityGuardStateV1::Account { security, presence } = &mut next_account.state
    {
        security.provenance.publication_revision = 10;
        *presence = Some((actor.0.current.character_id, candidate));
    }
    let mut next_character = character.clone();
    next_character.publication_revision = 13;
    next_character.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: 12,
    };
    next_character.source.source_revision = 5;
    next_character.source.decision_identity = "character-5".into();
    next_character.source.source_observed_at = 100;
    if let AdmissionAuthorityGuardStateV1::Character { holder, .. } = &mut next_character.state {
        *holder = Some(candidate);
    }
    Ok(ClaimOwner {
        actor,
        transition: AdmissionClaimTransitionEvidenceV1 {
            predecessors: vec![account, character],
            successors: vec![next_account, next_character],
            prepared_at: 100,
        },
    })
}
#[test]
fn post_grace_claim_preserves_stale_fresh_history_but_requires_current_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::admission_authority_publication::*;
    use crate::foundation::*;
    let (authorization, source, actor) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let claims = PostGraceClaimTransitionV1::prepare(&owner, &authorization, 100)
        .map_err(|_| "claim prepare")?;
    let rows = owner
        .transition
        .predecessors
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    assert!(
        claims
            .validate_locked(
                &authorization,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                &rows,
                100
            )
            .is_ok()
    );
    assert_eq!(claims.evidence().transition, owner.transition);
    assert!(
        claims
            .validate_locked(
                &authorization,
                &RecoveryDurabilityTrustContextV2::unavailable(),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                &rows,
                100
            )
            .is_err()
    );
    let mut changed_rows = rows.clone();
    changed_rows[0] = None;
    assert!(
        claims
            .validate_locked(
                &authorization,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                &changed_rows,
                100
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_claim_rejects_origin_relabel_reaging_and_stale_holder_cas()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::admission_authority_publication::*;
    let (authorization, _, _) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let changes: [fn(&mut ClaimOwner); 5] = [
        |o| {
            if let AdmissionAuthorityGuardStateV1::Account { security, .. } =
                &mut o.transition.successors[0].state
            {
                security.provenance.source_observed_at += 1;
            }
        },
        |o| {
            for row in [
                &mut o.transition.predecessors[0],
                &mut o.transition.successors[0],
            ] {
                if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                    security.provenance.scope = Fnd04EvidenceScope::ExistingActorRecovery;
                }
            }
        },
        |o| {
            for row in [
                &mut o.transition.predecessors[0],
                &mut o.transition.successors[0],
            ] {
                if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                    security.provenance.source_authority = "another-source".into();
                }
            }
        },
        |o| {
            o.transition.successors[0].precondition =
                AdmissionPublicationPreconditionV1::CompareAndSet {
                    expected_publication_revision: 8,
                }
        },
        |o| {
            if let AdmissionAuthorityGuardStateV1::Character { holder, .. } =
                &mut o.transition.successors[1].state
            {
                *holder = Some(o.actor.0.predecessor.commit().game_session_id());
            }
        },
    ];
    for change in changes {
        let mut changed = owner.clone();
        change(&mut changed);
        assert!(PostGraceClaimTransitionV1::prepare(&changed, &authorization, 100).is_err());
    }
    Ok(())
}
#[test]
fn post_grace_claim_requires_the_selected_current_purpose_floor()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::admission_authority_publication::*;
    use crate::foundation::*;
    let (authorization, source, actor) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let claims =
        PostGraceClaimTransitionV1::prepare(&owner, &authorization, 100).map_err(|_| "claim")?;
    let rows = owner
        .transition
        .predecessors
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    let mut stale = source.clone();
    stale.security.provenance.accepted_source_revision += 1;
    assert!(
        claims
            .validate_locked(
                &authorization,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&stale),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                &rows,
                100
            )
            .is_err()
    );
    let mut substituted = source.clone();
    substituted.security.provenance.scope = Fnd04EvidenceScope::FreshAdmission;
    assert!(
        claims
            .validate_locked(
                &authorization,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&substituted),
                &PostGraceActorAuthorityV1::from_owning_source(&actor),
                &rows,
                100
            )
            .is_err()
    );
    assert!(
        fresh_source_deadline(
            &source.security.provenance,
            FreshEvidencePurposeV1::PlatformSecurity,
            100
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn post_grace_claim_time_recovery_may_follow_newer_retained_fresh_history()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::admission_authority_publication::*;
    use crate::foundation::*;
    let (original, mut source, mut actor) = prepared_fixture()?;
    source.security.provenance.source_revision = 9;
    source.security.provenance.accepted_source_revision = 9;
    source.security.provenance.publication_revision = 11;
    source.security.provenance.decision_identity = "source-9".into();
    source.security.provenance.accepted_decision_identity = "source-9".into();
    source.security.provenance.source_observed_at = 102;
    actor.0.source_revision = 12;
    actor.0.accepted_source_revision = 12;
    actor.0.decision_identity = "actor-12".into();
    actor.0.accepted_decision_identity = "actor-12".into();
    actor.0.source_observed_at = 102;
    actor.0.account_security_source_revision = 9;
    let current = original
        .revalidate(
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &PostGraceActorAuthorityV1::from_owning_source(&actor),
            102,
        )
        .map_err(|_| "refresh")?;
    let mut owner = claim_fixture()?;
    owner.actor = actor;
    for (index, row) in [
        &mut owner.transition.predecessors[0],
        &mut owner.transition.successors[0],
    ]
    .into_iter()
    .enumerate()
    {
        row.publication_revision = 10 + index as u64;
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: 9 + index as u64,
        };
        row.source.source_revision = 4 + index as u64;
        row.source.decision_identity = format!("owner-{}", 4 + index);
        row.source.source_observed_at = 101 + index as i64;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
            security.provenance.source_revision = 8;
            security.provenance.accepted_source_revision = 8;
            security.provenance.decision_identity = "source-8".into();
            security.provenance.accepted_decision_identity = "source-8".into();
            security.provenance.source_observed_at = 101;
            security.provenance.publication_revision = row.publication_revision;
        }
    }
    owner.transition.successors[1].source.source_observed_at = 102;
    owner.transition.prepared_at = 102;
    let claim = PostGraceClaimTransitionV1::prepare(&owner, &current, 102)
        .map_err(|_| "claim-time newer recovery")?;
    assert_eq!(claim.evidence().operation, *original.operation());
    Ok(())
}

#[derive(Default)]
struct RecoveryQueue {
    submitted: Vec<crate::foundation::PostGraceDurabilityRequestV1>,
}
impl crate::foundation::PostGraceDurabilityPortV1 for RecoveryQueue {
    fn submit(
        &mut self,
        request: &crate::foundation::PostGraceDurabilityRequestV1,
    ) -> crate::foundation::PostGraceSubmissionV1 {
        self.submitted.push(request.clone());
        crate::foundation::PostGraceSubmissionV1::Accepted
    }
}
struct RecoveryCompletion(Option<crate::foundation::PostGraceDurableCompletionV1>);
impl recovery_source_sealed::Sealed for RecoveryCompletion {}
impl crate::foundation::PostGraceCompletionSourceV1 for RecoveryCompletion {
    fn take_completion(
        &mut self,
        _: &crate::foundation::admission_authority_publication::PostGraceClaimEvidenceV1,
        _: crate::foundation::PostGraceFlowPhaseV1,
    ) -> Result<
        Option<crate::foundation::PostGraceDurableCompletionV1>,
        crate::foundation::ReconnectDurabilityErrorV1,
    > {
        Ok(self.0.take())
    }
}
#[test]
fn post_grace_split_flow_never_installs_controller_from_preparation_or_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (authorization, source, actor) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue)
        .map_err(|_| "submit prepare")?;
    assert!(flow.controller().is_none());
    let prepared = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingPrepare,
        outcome: PostGraceDurableOutcomeV1::Prepared,
    };
    flow.poll(&mut RecoveryCompletion(Some(prepared)))
        .map_err(|_| "prepared")?;
    flow.submit_commit(
        &mut queue,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &PostGraceActorAuthorityV1::from_owning_source(&actor),
        100,
    )
    .map_err(|_| "submit commit")?;
    let rows = owner
        .transition
        .predecessors
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    let decision = queue.submitted[1]
        .validate_locked(
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &PostGraceActorAuthorityV1::from_owning_source(&actor),
            &rows,
            100,
        )
        .map_err(|_| "locked decision")?;
    let completion = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingCommit,
        outcome: PostGraceDurableOutcomeV1::Committed {
            decided_at: 100,
            decision: Box::new(decision),
        },
    };
    flow.poll(&mut RecoveryCompletion(Some(completion)))
        .map_err(|_| "completion")?;
    assert_eq!(flow.phase(), PostGraceFlowPhaseV1::AwaitingAdoption);
    assert!(flow.receipt().is_some());
    assert!(flow.controller().is_none());
    Ok(())
}

fn committed_fixture() -> Result<
    (
        crate::foundation::PostGraceDurabilityFlowV1,
        RecoverySource,
        ActorSource,
        ClaimOwner,
    ),
    Box<dyn std::error::Error>,
> {
    use crate::foundation::*;
    let (authorization, source, actor) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue).map_err(|_| "prepare")?;
    let completion = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingPrepare,
        outcome: PostGraceDurableOutcomeV1::Prepared,
    };
    flow.poll(&mut RecoveryCompletion(Some(completion)))
        .map_err(|_| "prepared")?;
    flow.submit_commit(
        &mut queue,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &PostGraceActorAuthorityV1::from_owning_source(&actor),
        100,
    )
    .map_err(|_| "commit")?;
    let rows = owner
        .transition
        .predecessors
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    let decision = queue.submitted[1]
        .validate_locked(
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &PostGraceActorAuthorityV1::from_owning_source(&actor),
            &rows,
            100,
        )
        .map_err(|_| "decision")?;
    let completion = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingCommit,
        outcome: PostGraceDurableOutcomeV1::Committed {
            decided_at: 100,
            decision: Box::new(decision),
        },
    };
    flow.poll(&mut RecoveryCompletion(Some(completion)))
        .map_err(|_| "completed")?;
    Ok((flow, source, actor, owner))
}
#[derive(Clone)]
struct AdoptionSource(crate::foundation::PostGraceAdoptionCurrentV1);
impl recovery_source_sealed::Sealed for AdoptionSource {}
impl crate::foundation::PostGraceAdoptionSourceV1 for AdoptionSource {
    fn current_adoption(
        &self,
        _: &crate::foundation::admission_authority_publication::PostGraceClaimEvidenceV1,
        _: i64,
    ) -> Result<
        crate::foundation::PostGraceAdoptionCurrentV1,
        crate::foundation::ReconnectDurabilityErrorV1,
    > {
        Ok(self.0.clone())
    }
}
fn adoption_fixture() -> Result<AdoptionSource, Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (_, source, _) = fixture().map_err(|_| "source")?;
    let mut actor = actor_fixture()?.0;
    let owner = claim_fixture()?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 6;
    let candidate = GameSessionId::decode(&id).map_err(|_| "candidate")?;
    let transport = AuthenticatedTransportRefV1::decode(&[6; 16]).map_err(|_| "transport")?;
    let commit = FreshAdmissionCommit::from_facts(
        candidate,
        FreshAdmissionFacts::new(
            [3; 32],
            actor.current.character_id,
            actor.current.world_id,
            actor.predecessor.commit().channel_id(),
            2,
            3,
        )?,
        transport,
    )?;
    let session = GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Active,
        ConnectionGeneration::new(1).map_err(|_| "generation")?,
        Some(transport),
        CharacterLease::new(actor.current.character_id, 2)?,
        Some(CharacterWorldEligibilityClaimV1::new(
            actor.current.character_id,
            actor.current.world_id,
        )),
        actor.predecessor.current_runtime_scope(),
        ScopeOwnershipGeneration::new(3).map_err(|_| "scope")?,
    )
    .map_err(|_| "session")?
    .with_control_loss_continuity(
        actor.budget.epoch(),
        actor
            .predecessor
            .current_original_grace_deadline()
            .ok_or("original grace")?,
    )
    .map_err(|_| "retained session continuity")?;
    actor.source_revision = 12;
    actor.accepted_source_revision = 12;
    actor.decision_identity = "restored-12".into();
    actor.accepted_decision_identity = "restored-12".into();
    actor.present_uncontrolled = false;
    actor.budget = RetainedRecoveryBudgetV1::restore(
        actor.budget.epoch(),
        RecoveryEpochStateV1::Restored,
        true,
        vec![RetainedRecoveryAttemptV1 {
            attempt: ReconnectAttemptRef::new(1)?,
            transport,
            disposition: RetainedRecoveryAttemptDispositionV1::Committed,
        }],
    )
    .map_err(|_| "restored budget")?;
    actor.protection = Some(RecoveryProtectionContinuityV1 {
        usage: RecoveryProtectionUseV1::Activated {
            entitlement_generation: 1,
            activated_at: 100,
            deadline: 104,
        },
        rearm: RecoveryProtectionRearmV1::Satisfied {
            generation: 7,
            established_at: 90,
        },
    });
    Ok(AdoptionSource(PostGraceAdoptionCurrentV1 {
        actor,
        session,
        actor_present: true,
        controller: Some((candidate, commit.connection_generation(), transport)),
        live_transport: Some(transport),
        security: source.security,
        signing: source.signing,
        claims: owner.transition.successors,
    }))
}
#[test]
fn post_grace_direct_and_reconciled_adoption_require_current_sources()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (mut direct, _, _, _) = committed_fixture()?;
    let current = adoption_fixture()?;
    direct.adopt(&current, 101).map_err(|_| "direct adoption")?;
    assert_eq!(
        direct.controller().ok_or("controller")?.generation().get(),
        1
    );
    let receipt = direct.receipt().ok_or("receipt")?.clone();
    let mut restored = PostGraceDurabilityFlowV1::restore_history(receipt.operation().clone())
        .map_err(|_| "history")?;
    assert!(restored.controller().is_none());
    let mut queue = RecoveryQueue::default();
    assert!(restored.submit_prepare(&mut queue).is_err());
    restored.reconcile(&mut queue).map_err(|_| "reconcile")?;
    let completion = PostGraceDurableCompletionV1 {
        operation: receipt.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingReconciliation,
        outcome: PostGraceDurableOutcomeV1::Committed {
            decided_at: receipt.decided_at(),
            decision: Box::new(receipt.decision().clone()),
        },
    };
    restored
        .poll(&mut RecoveryCompletion(Some(completion)))
        .map_err(|_| "historical completion")?;
    assert!(restored.controller().is_none());
    restored
        .adopt(&current, 101)
        .map_err(|_| "reconciled adoption")?;
    assert_eq!(restored.controller(), direct.controller());
    let mut missing = current.clone();
    missing.0.actor_present = false;
    assert!(restored.adopt(&missing, 101).is_err());
    assert!(restored.controller().is_none());
    Ok(())
}

#[test]
fn post_grace_prepared_restart_requires_new_sealed_authorization()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (authorization, source, actor) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let flow = PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut restored = PostGraceDurabilityFlowV1::restore_history(flow.operation().clone())
        .map_err(|_| "restore")?;
    let (token, _, current) = fixture().map_err(|_| "token")?;
    let trust = RecoveryDurabilityTrustContextV2::from_owning_source(&source);
    let actor_context = PostGraceActorAuthorityV1::from_owning_source(&actor);
    let verified =
        verify_recovery_grant_durability_v2(&token, 101, &trust, &current).map_err(|_| "verify")?;
    let authorization = PostGraceRecoveryAuthorizationV1::reauthorize_history(
        flow.operation().operation.clone(),
        verified,
        &trust,
        &actor_context,
        101,
    )
    .map_err(|_| "reauthorize")?;
    assert!(
        restored
            .resume_prepared(authorization.clone(), &trust, &actor_context, 101)
            .is_err()
    );
    let mut queue = RecoveryQueue::default();
    restored.reconcile(&mut queue).map_err(|_| "reconcile")?;
    let completion = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingReconciliation,
        outcome: PostGraceDurableOutcomeV1::Prepared,
    };
    restored
        .poll(&mut RecoveryCompletion(Some(completion)))
        .map_err(|_| "prepared")?;
    assert!(
        restored
            .submit_commit(&mut queue, &trust, &actor_context, 101)
            .is_err()
    );
    restored
        .resume_prepared(authorization, &trust, &actor_context, 101)
        .map_err(|_| "resume")?;
    restored
        .submit_commit(&mut queue, &trust, &actor_context, 101)
        .map_err(|_| "commit")?;
    assert_eq!(restored.operation(), flow.operation());
    assert_eq!(queue.submitted[1].operation(), flow.operation());
    Ok(())
}

#[test]
fn post_grace_late_adoption_rechecks_current_authority_not_old_credential_expiry()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut flow, _, _, _) = committed_fixture()?;
    let mut current = adoption_fixture()?;
    assert!(flow.adopt(&current, 120).is_err());
    for provenance in [
        &mut current.0.security.provenance,
        &mut current.0.signing.provenance,
    ] {
        provenance.source_revision = 8;
        provenance.accepted_source_revision = 8;
        provenance.publication_revision = 10;
        provenance.decision_identity = "source-8".into();
        provenance.accepted_decision_identity = "source-8".into();
        provenance.source_observed_at = 120;
    }
    current.0.actor.account_security_source_revision = 8;
    flow.adopt(&current, 120)
        .map_err(|_| "late current adoption")?;
    assert!(flow.controller().is_some());
    assert_eq!(flow.receipt().ok_or("receipt")?.decided_at(), 100);
    let mut denied = current.clone();
    denied.0.security.allowed = false;
    assert!(flow.adopt(&denied, 120).is_err());
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn post_grace_adoption_rejects_each_current_fence_change() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::foundation::*;
    let (flow, _, _, _) = committed_fixture()?;
    let baseline = adoption_fixture()?;
    let mut variants = Vec::new();
    let mut v = baseline.clone();
    v.0.actor_present = false;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.live_transport = None;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.controller = None;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.present_uncontrolled = true;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.runtime_ready = false;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.account_presence = None;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.placement_revision += 1;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.account_security_source_revision += 1;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.security.allowed = false;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.security.minimum_generation = 2;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.security.provenance.scope = Fnd04EvidenceScope::FreshAdmission;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.signing.trusted = false;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.protection = None;
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.protection.as_mut().ok_or("protection")?.usage = RecoveryProtectionUseV1::Activated {
        entitlement_generation: 1,
        activated_at: 101,
        deadline: 105,
    };
    variants.push(v);
    let mut v = baseline.clone();
    v.0.claims.clear();
    variants.push(v);
    let mut v = baseline.clone();
    v.0.actor.budget = RetainedRecoveryBudgetV1::restore(
        v.0.actor.budget.epoch(),
        RecoveryEpochStateV1::Open,
        true,
        vec![],
    )
    .map_err(|_| "budget")?;
    variants.push(v);
    for (index, current) in variants.iter().enumerate() {
        let mut candidate = flow.clone();
        assert!(candidate.adopt(current, 101).is_err(), "fence {index}");
        assert!(candidate.controller().is_none());
    }
    Ok(())
}

#[test]
fn post_grace_missing_ambiguous_and_mismatched_completions_keep_identity()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (authorization, _, _) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let operation = flow.operation().clone();
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue).map_err(|_| "prepare")?;
    assert!(
        !flow
            .poll(&mut RecoveryCompletion(None))
            .map_err(|_| "missing")?
    );
    assert_eq!(flow.phase(), PostGraceFlowPhaseV1::PendingPrepare);
    let bad = PostGraceDurableCompletionV1 {
        operation: operation.clone(),
        phase: PostGraceFlowPhaseV1::PendingCommit,
        outcome: PostGraceDurableOutcomeV1::Prepared,
    };
    assert!(flow.poll(&mut RecoveryCompletion(Some(bad))).is_err());
    let ambiguous = PostGraceDurableCompletionV1 {
        operation: operation.clone(),
        phase: PostGraceFlowPhaseV1::PendingPrepare,
        outcome: PostGraceDurableOutcomeV1::Ambiguous,
    };
    flow.poll(&mut RecoveryCompletion(Some(ambiguous)))
        .map_err(|_| "ambiguous")?;
    assert_eq!(flow.phase(), PostGraceFlowPhaseV1::ReconciliationRequired);
    assert!(flow.controller().is_none());
    flow.reconcile(&mut queue).map_err(|_| "reconcile")?;
    assert_eq!(queue.submitted[1].operation(), &operation);
    assert_eq!(queue.submitted[1].kind(), PostGraceRequestKindV1::Reconcile);
    Ok(())
}

#[test]
fn post_grace_terminal_collision_is_typed_and_never_reopens()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (authorization, _, _) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue).map_err(|_| "prepare")?;
    let completion = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingPrepare,
        outcome: PostGraceDurableOutcomeV1::Rejected {
            reason: PostGraceTerminalReasonV1::TransportCollision,
        },
    };
    flow.poll(&mut RecoveryCompletion(Some(completion)))
        .map_err(|_| "rejected")?;
    assert_eq!(
        flow.terminal_reason(),
        Some(PostGraceTerminalReasonV1::TransportCollision)
    );
    assert_eq!(flow.phase(), PostGraceFlowPhaseV1::Rejected);
    assert!(flow.submit_prepare(&mut queue).is_err());
    assert!(flow.reconcile(&mut queue).is_err());
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn post_grace_adoption_rejects_changed_canonical_candidate_origin()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (mut flow, _, _, _) = committed_fixture()?;
    let mut current = adoption_fixture()?;
    let session = current.0.session;
    let changed = FreshAdmissionCommit::from_facts(
        session.commit().game_session_id(),
        FreshAdmissionFacts::new(
            [3; 32],
            session.commit().character_id(),
            session.commit().world_id(),
            session.commit().channel_id(),
            2,
            3,
        )?,
        AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(|_| "transport")?,
    )?;
    current.0.session = GameSessionAuthoritySnapshot::from_current_facts(
        changed,
        GameSessionState::Active,
        session.current_connection_generation(),
        session.current_transport(),
        session.current_character_lease(),
        session.current_character_world_eligibility(),
        session.current_runtime_scope(),
        session.current_scope_generation(),
    )
    .map_err(|_| "session")?;
    assert!(flow.adopt(&current, 101).is_err());
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn post_grace_prepare_and_final_locked_matrix_reads_independent_current_fences()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (authorization, source, actor) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue).map_err(|_| "prepare")?;
    let prepared = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingPrepare,
        outcome: PostGraceDurableOutcomeV1::Prepared,
    };
    flow.poll(&mut RecoveryCompletion(Some(prepared)))
        .map_err(|_| "prepared")?;
    flow.submit_commit(
        &mut queue,
        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
        &PostGraceActorAuthorityV1::from_owning_source(&actor),
        100,
    )
    .map_err(|_| "commit")?;
    let rows = owner
        .transition
        .predecessors
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    let actor_changes: [fn(&mut PostGraceActorObservationV1); 12] = [
        |a| a.present_uncontrolled = false,
        |a| a.runtime_ready = false,
        |a| a.account_presence = None,
        |a| a.placement_identity = [9; 16],
        |a| a.placement_revision += 1,
        |a| a.protection = None,
        |a| a.account_security_source_revision += 1,
        |a| a.current.ruleset_revision = "rules-2".into(),
        |a| a.current.content_revision = "content-2".into(),
        |a| a.current.map_revision = "map-2".into(),
        |a| a.current.world_policy_revision = "policy-2".into(),
        |a| a.current.account_id = "00000000-0000-4000-8000-000000000009".into(),
    ];
    let source_changes: [fn(&mut RecoverySource); 8] = [
        |s| s.security.allowed = false,
        |s| s.security.minimum_generation = 2,
        |s| s.security.provenance.scope = Fnd04EvidenceScope::FreshAdmission,
        |s| s.security.provenance.accepted_source_revision += 1,
        |s| s.signing.trusted = false,
        |s| s.signing.key_id = "wrong-key".into(),
        |s| s.signing.provenance.scope = Fnd04EvidenceScope::FreshAdmission,
        |s| s.security.provenance.decision_identity = "contradiction".into(),
    ];
    for request in &queue.submitted {
        assert!(
            request
                .validate_locked(
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                    &PostGraceActorAuthorityV1::from_owning_source(&actor),
                    &rows,
                    101
                )
                .is_ok()
        );
        for (index, change) in actor_changes.iter().enumerate() {
            let mut changed = actor.clone();
            changed.0.source_revision = 12;
            changed.0.accepted_source_revision = 12;
            changed.0.decision_identity = "actor-12".into();
            changed.0.accepted_decision_identity = "actor-12".into();
            changed.0.source_observed_at = 101;
            change(&mut changed.0);
            assert!(
                request
                    .validate_locked(
                        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                        &PostGraceActorAuthorityV1::from_owning_source(&changed),
                        &rows,
                        101
                    )
                    .is_err(),
                "{:?} actor {index}",
                request.kind()
            );
        }
        for (index, change) in source_changes.iter().enumerate() {
            let mut changed = source.clone();
            change(&mut changed);
            assert!(
                request
                    .validate_locked(
                        &RecoveryDurabilityTrustContextV2::from_owning_source(&changed),
                        &PostGraceActorAuthorityV1::from_owning_source(&actor),
                        &rows,
                        101
                    )
                    .is_err(),
                "{:?} source {index}",
                request.kind()
            );
        }
        // A coherently newer source observation is an independently valid
        // control. Canonical mutations must not hide behind a same-revision
        // contradiction, which is a different invariant.
        let mut refreshed_actor = actor.clone();
        refreshed_actor.0.source_revision = 12;
        refreshed_actor.0.accepted_source_revision = 12;
        refreshed_actor.0.decision_identity = "actor-12".into();
        refreshed_actor.0.accepted_decision_identity = "actor-12".into();
        refreshed_actor.0.source_observed_at = 101;
        assert!(
            request
                .validate_locked(
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                    &PostGraceActorAuthorityV1::from_owning_source(&refreshed_actor),
                    &rows,
                    101
                )
                .is_ok(),
            "{:?} refreshed canonical control",
            request.kind()
        );
        for index in 0..8 {
            let mut changed = refreshed_actor.clone();
            let old = changed.0.predecessor;
            let state = match index {
                0 => GameSessionState::Active,
                1 => GameSessionState::Reconnectable,
                _ => GameSessionState::Terminal,
            };
            let snapshot = GameSessionAuthoritySnapshot::from_current_facts(
                old.commit(),
                state,
                if index == 2 {
                    ConnectionGeneration::new(8).map_err(|_| "generation")?
                } else {
                    old.current_connection_generation()
                },
                if index == 3 {
                    Some(AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(|_| "transport")?)
                } else {
                    None
                },
                if index == 4 {
                    CharacterLease::new(old.commit().character_id(), 3)?
                } else {
                    old.current_character_lease()
                },
                if index == 5 {
                    None
                } else {
                    old.current_character_world_eligibility()
                },
                old.current_runtime_scope(),
                if index == 6 {
                    ScopeOwnershipGeneration::new(4).map_err(|_| "scope")?
                } else {
                    old.current_scope_generation()
                },
            )
            .map_err(|_| "snapshot")?
            .with_control_loss_continuity(
                old.current_control_loss_epoch().ok_or("epoch")?,
                if index == 7 { 100 } else { 99 },
            )
            .map_err(|_| "continuity")?;
            changed.0.predecessor = snapshot;
            assert!(
                request
                    .validate_locked(
                        &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                        &PostGraceActorAuthorityV1::from_owning_source(&changed),
                        &rows,
                        101
                    )
                    .is_err(),
                "{:?} canonical {index}",
                request.kind()
            );
        }
        let mut missing = rows.clone();
        missing[0] = None;
        assert!(
            request
                .validate_locked(
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                    &PostGraceActorAuthorityV1::from_owning_source(&actor),
                    &missing,
                    101
                )
                .is_err()
        );
        let mut changed = rows.clone();
        changed[1].as_mut().ok_or("row")?.publication_revision += 1;
        assert!(
            request
                .validate_locked(
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                    &PostGraceActorAuthorityV1::from_owning_source(&actor),
                    &changed,
                    101
                )
                .is_err()
        );
        assert!(
            request
                .validate_locked(
                    &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
                    &PostGraceActorAuthorityV1::from_owning_source(&actor),
                    &rows,
                    104
                )
                .is_err()
        );
        assert!(
            request
                .validate_locked(
                    &RecoveryDurabilityTrustContextV2::unavailable(),
                    &PostGraceActorAuthorityV1::from_owning_source(&actor),
                    &rows,
                    101
                )
                .is_err()
        );
    }
    Ok(())
}

#[test]
fn post_grace_signed_credential_matrix_preserves_reauth_profile_and_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    use ed25519_dalek::{Signer, SigningKey};
    let (token, source, current) = fixture().map_err(|_| "fixture")?;
    let parts = token.split('.').collect::<Vec<_>>();
    let payload =
        String::from_utf8(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?)?;
    let changes = [
        ("oteryn-reauth-recovery-v1", "oteryn-fast-reconnect-v1"),
        ("existing_actor_recovery", "fresh_admission"),
        (
            "urn:oteryn:platform:game-recovery",
            "urn:oteryn:platform:game-reconnect",
        ),
        ("urn:oteryn:game:recovery", "urn:oteryn:game:admission"),
        ("\"protocol_major\":1", "\"protocol_major\":2"),
        ("\"transport_profile\":1", "\"transport_profile\":2"),
        ("rules-1", "rules-2"),
        ("content-1", "content-2"),
        ("map-1", "map-2"),
        ("policy-1", "policy-2"),
        ("\"nbf\":100", "\"nbf\":106"),
        ("\"exp\":110", "\"exp\":95"),
        (
            "\"account_security_generation\":\"1\"",
            "\"account_security_generation\":\"0\"",
        ),
    ];
    let trust = RecoveryDurabilityTrustContextV2::from_owning_source(&source);
    assert!(verify_recovery_grant_durability_v2(&token, 100, &trust, &current).is_ok());
    for (index, (old, new)) in changes.iter().enumerate() {
        assert!(payload.contains(old));
        let changed = payload.replace(old, new);
        let input = format!(
            "{}.{}",
            parts[0],
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(changed)
        );
        let signed = format!(
            "{input}.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
                SigningKey::from_bytes(&[23; 32])
                    .sign(input.as_bytes())
                    .to_bytes()
            )
        );
        assert!(
            verify_recovery_grant_durability_v2(&signed, 100, &trust, &current).is_err(),
            "signed binding {index}"
        );
    }
    Ok(())
}

#[test]
fn post_grace_adoption_preserves_full_budget_and_consumed_protection()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (original, source, mut actor) = prepared_fixture()?;
    let mut owner = claim_fixture()?;
    let mut entries = vec![RetainedRecoveryAttemptV1 {
        attempt: original.attempt(),
        transport: original.transport(),
        disposition: RetainedRecoveryAttemptDispositionV1::Prepared,
    }];
    for attempt in 2..=8 {
        entries.push(RetainedRecoveryAttemptV1 {
            attempt: ReconnectAttemptRef::new(attempt)?,
            transport: AuthenticatedTransportRefV1::decode(&[attempt as u8; 16])
                .map_err(|_| "transport")?,
            disposition: if attempt % 2 == 0 {
                RetainedRecoveryAttemptDispositionV1::Terminal
            } else {
                RetainedRecoveryAttemptDispositionV1::TransportCollision
            },
        });
    }
    actor.0.budget = RetainedRecoveryBudgetV1::restore(
        actor.0.budget.epoch(),
        RecoveryEpochStateV1::Open,
        true,
        entries.clone(),
    )
    .map_err(|_| "budget")?;
    actor.0.protection.as_mut().ok_or("protection")?.usage = RecoveryProtectionUseV1::Activated {
        entitlement_generation: 1,
        activated_at: 90,
        deadline: 94,
    };
    owner.actor = actor.clone();
    let trust = RecoveryDurabilityTrustContextV2::from_owning_source(&source);
    let actor_context = PostGraceActorAuthorityV1::from_owning_source(&actor);
    let authorization = PostGraceRecoveryAuthorizationV1::prepare(
        original.verified(),
        &trust,
        &actor_context,
        original.candidate(),
        original.attempt(),
        original.transport(),
        100,
    )
    .map_err(|_| "authorization")?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue).map_err(|_| "prepare")?;
    let prepared = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingPrepare,
        outcome: PostGraceDurableOutcomeV1::Prepared,
    };
    flow.poll(&mut RecoveryCompletion(Some(prepared)))
        .map_err(|_| "prepared")?;
    flow.submit_commit(&mut queue, &trust, &actor_context, 100)
        .map_err(|_| "commit")?;
    let rows = owner
        .transition
        .predecessors
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    let decision = queue.submitted[1]
        .validate_locked(&trust, &actor_context, &rows, 100)
        .map_err(|_| "decision")?;
    let committed = PostGraceDurableCompletionV1 {
        operation: flow.operation().clone(),
        phase: PostGraceFlowPhaseV1::PendingCommit,
        outcome: PostGraceDurableOutcomeV1::Committed {
            decided_at: 100,
            decision: Box::new(decision),
        },
    };
    flow.poll(&mut RecoveryCompletion(Some(committed)))
        .map_err(|_| "committed")?;
    let mut current = adoption_fixture()?;
    current.0.actor.protection = actor.0.protection;
    entries[0].disposition = RetainedRecoveryAttemptDispositionV1::Committed;
    current.0.actor.budget = RetainedRecoveryBudgetV1::restore(
        actor.0.budget.epoch(),
        RecoveryEpochStateV1::Restored,
        true,
        entries.clone(),
    )
    .map_err(|_| "restored budget")?;
    flow.adopt(&current, 101)
        .map_err(|_| "adopt full retained history")?;
    assert!(flow.controller().is_some());
    assert_eq!(current.0.actor.protection, actor.0.protection);
    entries.pop();
    current.0.actor.budget = RetainedRecoveryBudgetV1::restore(
        actor.0.budget.epoch(),
        RecoveryEpochStateV1::Restored,
        true,
        entries,
    )
    .map_err(|_| "compacted")?;
    assert!(flow.adopt(&current, 101).is_err());
    assert!(flow.controller().is_none());
    Ok(())
}

#[test]
fn post_grace_completion_rejects_changed_operation_and_mixed_timing()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (authorization, _, _) = prepared_fixture()?;
    let owner = claim_fixture()?;
    let mut flow =
        PostGraceDurabilityFlowV1::begin(authorization, &owner, 100).map_err(|_| "begin")?;
    let mut queue = RecoveryQueue::default();
    flow.submit_prepare(&mut queue).map_err(|_| "prepare")?;
    for index in 0..4 {
        let mut operation = flow.operation().clone();
        match index {
            0 => {
                operation.operation.transport =
                    AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(|_| "transport")?
            }
            1 => operation.operation.attempt = ReconnectAttemptRef::new(9)?,
            2 => operation.transition.successors[0].publication_revision += 1,
            _ => operation.operation.credential.accepted_deadline += 1,
        };
        let completion = PostGraceDurableCompletionV1 {
            operation,
            phase: PostGraceFlowPhaseV1::PendingPrepare,
            outcome: PostGraceDurableOutcomeV1::Prepared,
        };
        assert!(
            flow.poll(&mut RecoveryCompletion(Some(completion)))
                .is_err()
        );
        assert_eq!(flow.phase(), PostGraceFlowPhaseV1::PendingPrepare);
    }
    let mut history = flow.operation().clone();
    history.operation.timing = RecoveryTimingV2::SameSession(
        ReconnectContinuityV1::new(
            history.operation.actor.budget.epoch(),
            99,
            98,
            ProtectionEntitlementV1::unused(),
        )
        .map_err(|_| "same session timing")?,
    );
    assert!(PostGraceDurabilityFlowV1::restore_history(history).is_err());
    Ok(())
}

#[test]
fn post_grace_direct_and_reconciled_adoption_require_canonical_loss_continuity()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let (direct, _, _, _) = committed_fixture()?;
    let receipt = direct.receipt().ok_or("receipt")?.clone();
    let mut restored = PostGraceDurabilityFlowV1::restore_history(receipt.operation().clone())
        .map_err(|_| "history")?;
    restored
        .reconcile(&mut RecoveryQueue::default())
        .map_err(|_| "reconcile")?;
    restored
        .poll(&mut RecoveryCompletion(Some(
            PostGraceDurableCompletionV1 {
                operation: receipt.operation().clone(),
                phase: PostGraceFlowPhaseV1::PendingReconciliation,
                outcome: PostGraceDurableOutcomeV1::Committed {
                    decided_at: receipt.decided_at(),
                    decision: Box::new(receipt.decision().clone()),
                },
            },
        )))
        .map_err(|_| "completion")?;
    let baseline = adoption_fixture()?;
    for template in [direct, restored] {
        for mutation in 0..3 {
            let mut flow = template.clone();
            flow.adopt(&baseline, 101).map_err(|_| "valid control")?;
            assert!(flow.controller().is_some());
            let mut changed = baseline.clone();
            let old = changed.0.session;
            let absent = GameSessionAuthoritySnapshot::from_current_facts(
                old.commit(),
                old.session_state(),
                old.current_connection_generation(),
                old.current_transport(),
                old.current_character_lease(),
                old.current_character_world_eligibility(),
                old.current_runtime_scope(),
                old.current_scope_generation(),
            )
            .map_err(|_| "independent current snapshot")?;
            changed.0.session = match mutation {
                0 => absent,
                1 => absent
                    .with_control_loss_continuity(
                        ControlLossEpochRefV1::new(9).map_err(|_| "changed epoch")?,
                        99,
                    )
                    .map_err(|_| "epoch")?,
                _ => absent
                    .with_control_loss_continuity(changed.0.actor.budget.epoch(), 98)
                    .map_err(|_| "changed grace")?,
            };
            assert!(flow.adopt(&changed, 101).is_err(), "continuity {mutation}");
            assert!(flow.controller().is_none());
        }
    }
    Ok(())
}

#[test]
fn post_grace_adopted_snapshot_supports_later_owning_loss_without_continuity_reset()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    struct Owner(ControlLossObservationV1);
    impl recovery_source_sealed::Sealed for Owner {}
    impl ControlLossSourceV1 for Owner {
        fn resolve_loss(
            &self,
            _: GameSessionId,
            _: i64,
        ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
            Ok(self.0.clone())
        }
    }
    let (mut flow, _, _, _) = committed_fixture()?;
    let current = adoption_fixture()?;
    flow.adopt(&current, 101).map_err(|_| "adoption")?;
    let protection = current.0.actor.protection.ok_or("protection")?;
    let epoch = ControlLossEpochRefV1::new(5).map_err(|_| "next epoch")?;
    let owner = Owner(ControlLossObservationV1 {
        source_authority: current.0.session.current_runtime_scope(),
        source_revision: 13,
        accepted_source_revision: 13,
        decision_identity: epoch,
        accepted_decision_identity: epoch,
        observed_at: 102,
        session: current.0.session,
        account_presence: current.0.actor.account_presence.clone().ok_or("account")?,
        placement_identity: current.0.actor.placement_identity,
        placement_revision: current.0.actor.placement_revision,
        actor_present: true,
        runtime_ready: true,
        cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
        loss_epoch: epoch,
        loss_origin: 102,
        original_grace_deadline: 122,
        history: ControlLossHistoryV1::Resumed {
            budget: current.0.actor.budget.clone(),
            original_grace_deadline: 99,
            protection,
        },
        protection,
    });
    let auth = ControlLossAuthorizationV1::authorize(
        &owner,
        current.0.session.commit().game_session_id(),
        102,
    )
    .map_err(|_| "later genuine loss")?;
    let effect = auth.validate_final(&owner, 102).map_err(|_| "final")?;
    assert_eq!(effect.predecessor(), current.0.session);
    assert_eq!(effect.successor().current_connection_generation().get(), 1);
    assert_eq!(effect.operation().observation.history, owner.0.history);
    assert_eq!(effect.operation().observation.protection, protection);
    assert_eq!(effect.successor().current_control_loss_epoch(), Some(epoch));
    Ok(())
}

#[test]
fn post_grace_lifecycle_source_fields_cannot_exceed_complete_operation_cap()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::foundation::*;
    let mut p = provenance(FreshEvidencePurposeV1::PlatformSecurity);
    p.source_authority = "x".repeat(65_536);
    assert!(recovery_source_deadline(&p, FreshEvidencePurposeV1::PlatformSecurity, 100).is_ok());
    p.source_authority.push('x');
    assert!(recovery_source_deadline(&p, FreshEvidencePurposeV1::PlatformSecurity, 100).is_err());
    let (auth, source, mut actor) = prepared_fixture()?;
    actor.0.source_authority = "x".repeat(65_536);
    let prepare = |owner: &ActorSource| {
        PostGraceRecoveryAuthorizationV1::prepare(
            auth.verified(),
            &RecoveryDurabilityTrustContextV2::from_owning_source(&source),
            &PostGraceActorAuthorityV1::from_owning_source(owner),
            auth.candidate(),
            auth.attempt(),
            auth.transport(),
            100,
        )
    };
    assert!(prepare(&actor).is_ok());
    actor.0.source_authority.push('x');
    assert!(prepare(&actor).is_err());
    let (auth, _, _) = prepared_fixture()?;
    let mut claims = claim_fixture()?;
    claims.transition.predecessors[0].source.authority = "x".repeat(65_536);
    claims.transition.successors[0].source.authority = "x".repeat(65_536);
    assert!(
        crate::foundation::admission_authority_publication::PostGraceClaimTransitionV1::prepare(
            &claims, &auth, 100
        )
        .is_ok()
    );
    claims.transition.predecessors[0].source.authority.push('x');
    claims.transition.successors[0].source.authority.push('x');
    assert!(
        crate::foundation::admission_authority_publication::PostGraceClaimTransitionV1::prepare(
            &claims, &auth, 100
        )
        .is_err()
    );
    Ok(())
}
