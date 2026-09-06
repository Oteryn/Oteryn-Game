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
