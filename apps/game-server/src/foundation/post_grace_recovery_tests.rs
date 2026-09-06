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
