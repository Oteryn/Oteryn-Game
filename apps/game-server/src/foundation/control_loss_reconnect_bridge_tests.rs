// Assertions keep fixture construction explicit without unchecked unwraps.
trait TestValue<T> {
    fn require(self, context: &str) -> T;
}
impl<T, E: std::fmt::Debug> TestValue<T> for Result<T, E> {
    fn require(self, context: &str) -> T {
        assert!(self.is_ok(), "{context}: {:?}", self.as_ref().err());
        match self {
            Ok(value) => value,
            Err(_) => unreachable!("asserted success"),
        }
    }
}
impl<T> TestValue<T> for Option<T> {
    fn require(self, context: &str) -> T {
        assert!(self.is_some(), "{context}");
        match self {
            Some(value) => value,
            None => unreachable!("asserted presence"),
        }
    }
}
use super::*;

#[test]
fn complete_bridge_not_entitled_does_not_create_protection() {
    let protection = RecoveryProtectionContinuityV1 {
        usage: RecoveryProtectionUseV1::NotEntitled,
        rearm: RecoveryProtectionRearmV1::NotRearmed {
            generation: 17,
            stable_control_started_at: None,
            accepted_deadline: None,
        },
    };
    assert_eq!(
        complete_reconnect_protection(protection, 105),
        Ok(protection)
    );
}

#[test]
fn complete_bridge_activation_keeps_entitlement_namespace_and_rearm() {
    let protection = RecoveryProtectionContinuityV1 {
        usage: RecoveryProtectionUseV1::Unused {
            entitlement_generation: 91,
        },
        rearm: RecoveryProtectionRearmV1::Satisfied {
            generation: 17,
            established_at: 90,
        },
    };
    let activated = complete_reconnect_protection(protection, 105).require("valid protection");
    assert_eq!(
        activated.usage,
        RecoveryProtectionUseV1::Activated {
            entitlement_generation: 91,
            activated_at: 105,
            deadline: 109,
        }
    );
    assert_eq!(activated.rearm, protection.rearm);
    assert_eq!(complete_reconnect_protection(activated, 110), Ok(activated));
    assert!(complete_reconnect_protection(protection, i64::MAX).is_err());
}

use super::super::admission_authority_publication::*;
use super::super::fnd04_verifier::*;
use base64::Engine as _;
#[derive(Clone)]
struct LossOwner(ControlLossObservationV1);
impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for LossOwner {}
impl ControlLossSourceV1 for LossOwner {
    fn resolve_loss(
        &self,
        _: GameSessionId,
        _: i64,
    ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
        Ok(self.0.clone())
    }
}
fn loss_owner() -> Result<LossOwner, Box<dyn std::error::Error>> {
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
    Ok(LossOwner(ControlLossObservationV1 {
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
fn credential_fixture()
-> Result<(String, RecoverySource, RecoveryCurrentEvidence), Fnd04ConsumerError> {
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

impl Fnd04EvidenceAuthority for RecoverySource {
    fn signing_key(
        &self,
        scope: Fnd04EvidenceScope,
        key: &str,
        now: i64,
    ) -> Result<[u8; 32], Fnd04EvidenceError> {
        if scope != Fnd04EvidenceScope::ExistingActorRecovery
            || key != self.signing.key_id
            || !self.signing.trusted
            || now < self.signing.provenance.source_observed_at
            || now > self.signing.provenance.source_observed_at + 5
        {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.signing.public_key)
    }
    fn account_minimum_generation(
        &self,
        scope: Fnd04EvidenceScope,
        account: &str,
        now: i64,
    ) -> Result<u64, Fnd04EvidenceError> {
        if scope != Fnd04EvidenceScope::ExistingActorRecovery
            || account != self.security.account_id
            || !self.security.allowed
            || now < self.security.provenance.source_observed_at
            || now > self.security.provenance.source_observed_at + 5
        {
            return Err(Fnd04EvidenceError::UnavailableOrStale);
        }
        Ok(self.security.minimum_generation)
    }
}
#[derive(Clone)]
struct BridgeOwner {
    current: CompleteReconnectCurrentV1,
    security: RecoverySource,
    fast: Option<CompleteFastReconnectBindingV1>,
    fast_adoption: Option<CompleteFastReconnectAdoptionV1>,
    active_proof: Option<CompleteReconnectProofCurrentV1>,
}
impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for BridgeOwner {}
impl CompleteReconnectSourceV1 for BridgeOwner {
    fn resolve_reconnect(
        &self,
        _: &ReconnectIdentityV1,
        _: i64,
    ) -> Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1> {
        Ok(self.current.clone())
    }
    fn verify_fast_reconnect(
        &self,
        _: &ReconnectIdentityV1,
        now: i64,
    ) -> Result<CompleteFastReconnectBindingV1, ReconnectDurabilityErrorV1> {
        let mut binding = self
            .fast
            .clone()
            .ok_or(ReconnectDurabilityErrorV1::StaleAuthority)?;
        if !self.security.security.allowed
            || self.security.security.minimum_generation
                > binding.compatibility.account_security_generation()
        {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        binding.verified_at = now;
        Ok(binding)
    }
    fn recovery_v1_authority(&self) -> Option<&dyn Fnd04EvidenceAuthority> {
        Some(&self.security)
    }
    fn recovery_v2_source(&self) -> Option<&dyn RecoveryDurabilityEvidenceSourceV2> {
        Some(&self.security)
    }
}
impl CompleteReconnectAdoptionSourceV1 for BridgeOwner {
    fn current_reconnect_proof(
        &self,
        _: &ReconnectIdentityV1,
        _: i64,
    ) -> Result<CompleteReconnectProofCurrentV1, ReconnectDurabilityErrorV1> {
        self.active_proof
            .clone()
            .ok_or(ReconnectDurabilityErrorV1::StaleAuthority)
    }
    fn current_fast_reconnect_proof(
        &self,
        _: &ReconnectIdentityV1,
        _: i64,
    ) -> Result<CompleteFastReconnectAdoptionV1, ReconnectDurabilityErrorV1> {
        self.fast_adoption
            .clone()
            .ok_or(ReconnectDurabilityErrorV1::StaleAuthority)
    }
    fn resolve_complete_reconnect_adoption(
        &self,
        _: &ReconnectIdentityV1,
        _: i64,
    ) -> Result<CompleteReconnectSnapshotV1, ReconnectDurabilityErrorV1> {
        Ok(self.current.snapshot.clone())
    }
}
impl CompleteReconnectClaimSourceV1 for BridgeOwner {
    fn prepare_complete_reconnect_claim(
        &self,
        operation: &CompleteReconnectOperationV1,
        now: i64,
    ) -> Result<CompleteReconnectClaimResolutionV1, AdmissionAuthorityPublicationErrorV1> {
        let current = self.current.snapshot.clone();
        let mut successors = current.claims.clone();
        for row in &mut successors {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: row.publication_revision,
            };
            row.publication_revision += 1;
            row.source.source_revision += 1;
            row.source.decision_identity.push_str("-next");
            row.source.source_observed_at = now;
        }
        if let AdmissionAuthorityGuardStateV1::Account { presence, security } =
            &mut successors[0].state
        {
            security.provenance.publication_revision = 2;
            *presence = Some((
                operation.identity.character_id(),
                operation.identity.game_session_id(),
            ));
        }
        if let AdmissionAuthorityGuardStateV1::Character { holder, .. } = &mut successors[1].state {
            *holder = Some(operation.identity.game_session_id());
        }
        Ok(CompleteReconnectClaimResolutionV1 {
            transition: AdmissionClaimTransitionEvidenceV1 {
                predecessors: current.claims.clone(),
                successors,
                prepared_at: now,
            },
            current,
        })
    }
}
fn claims(
    session: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
) -> Vec<AdmissionAuthorityPublicationChangeV1> {
    let account = "00000000-0000-4000-8000-000000000001";
    let mut provenance = provenance(FreshEvidencePurposeV1::PlatformSecurity);
    provenance.scope = Fnd04EvidenceScope::FreshAdmission;
    provenance.source_revision = 6;
    provenance.accepted_source_revision = 6;
    provenance.source_observed_at = 99;
    provenance.publication_revision = 1;
    let source = AdmissionPublicationSourceV1 {
        authority: "game-owner".into(),
        purpose: AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
        source_revision: 1,
        decision_identity: "account-1".into(),
        source_observed_at: 100,
        clock_uncertainty_seconds: 0,
    };
    let account_row = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Account {
            account_id: account.into(),
        },
        source: source.clone(),
        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        },
        publication_revision: 1,
        state: AdmissionAuthorityGuardStateV1::Account {
            security: FreshAccountSecurityObservationV1 {
                account_id: account.into(),
                minimum_generation: 1,
                allowed: true,
                provenance,
            },
            presence: Some((
                session.commit().character_id(),
                session.commit().game_session_id(),
            )),
        },
    };
    let character = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Character(session.commit().character_id()),
        source: AdmissionPublicationSourceV1 {
            purpose: AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
            ..source
        },
        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        },
        publication_revision: 1,
        state: AdmissionAuthorityGuardStateV1::Character {
            account_id: account.into(),
            world_id: session.commit().world_id(),
            eligible: true,
            lease_generation: 2,
            holder: Some(session.commit().game_session_id()),
        },
    };
    vec![account_row, character]
}
fn bridge_fixture(replacement: bool) -> (BridgeOwner, ReconnectIdentityV1, String) {
    let (token, security, recovery) = credential_fixture().require("credential fixture");
    let mut loss = loss_owner().require("loss fixture");
    loss.0.protection.usage = RecoveryProtectionUseV1::NotEntitled;
    let loss_auth = ControlLossAuthorizationV1::authorize(
        &loss,
        loss.0.session.commit().game_session_id(),
        100,
    )
    .require("genuine loss");
    let effect = loss_auth.validate_final(&loss, 100).require("loss effect");
    let mut session = effect.successor();
    let candidate_id = if replacement {
        session.session_state = GameSessionState::Terminal;
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 9;
        GameSessionId::decode(&id).require("candidate")
    } else {
        session.commit().game_session_id()
    };
    let identity = ReconnectIdentityV1::new(
        candidate_id,
        ReconnectAttemptRef::new(20).require("attempt"),
        &recovery.account_id,
        recovery.character_id,
        recovery.world_id,
        session.current_runtime_scope(),
    )
    .require("identity");
    let candidate = ReconnectCandidateBindingV1::new(
        candidate_id,
        identity.reconnect_attempt_ref(),
        ConnectionGeneration::new(2).require("generation"),
        AuthenticatedTransportRefV1::decode(&[9; 16]).require("transport"),
        103,
    )
    .require("candidate");
    let snapshot = CompleteReconnectSnapshotV1 {
        loss: effect.operation().clone(),
        loss_decided_at: 100,
        source_authority: session.current_runtime_scope(),
        source_revision: 7,
        accepted_source_revision: 7,
        observed_at: 100,
        session,
        account_presence: loss.0.account_presence.clone(),
        actor_present: true,
        runtime_ready: true,
        placement_identity: loss.0.placement_identity,
        placement_revision: 8,
        protection: loss.0.protection,
        budget: RetainedRecoveryBudgetV1::restore(
            loss.0.loss_epoch,
            RecoveryEpochStateV1::Open,
            true,
            vec![],
        )
        .require("budget"),
        candidate,
        proof_transition: CompleteReconnectProofTransitionV1 {
            owner: session.current_runtime_scope(),
            revision: 11,
            accepted_revision: 11,
            observed_at: 100,
            predecessor_session: session.commit().game_session_id(),
            predecessor_generation: 41,
            successor_session: candidate.game_session_id(),
            successor_generation: if replacement { 1 } else { 43 },
            candidate,
        },
        fnd02: Fnd02ReconciliationFenceV1::new(
            CommandId::new(1).require("command"),
            vec![],
            7,
            vec![],
        )
        .require("fnd02"),
        recovery,
        claims: claims(session),
    };
    (
        BridgeOwner {
            current: CompleteReconnectCurrentV1 {
                snapshot,
                prepared: None,
            },
            security,
            fast: None,
            fast_adoption: None,
            active_proof: None,
        },
        identity,
        token,
    )
}
fn proof(owner: &BridgeOwner, token: &str, v2: bool, now: i64) -> CompleteReconnectProofV1 {
    if v2 {
        CompleteReconnectProofV1::V2(Box::new(
            verify_recovery_grant_durability_v2(
                token,
                now,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&owner.security),
                &owner.current.snapshot.recovery,
            )
            .require("verified"),
        ))
    } else {
        CompleteReconnectProofV1::V1Token(token.into())
    }
}
struct Report(Option<CompleteReconnectCompletionV1>);
impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for Report {}
impl CompleteReconnectCompletionSourceV1 for Report {
    fn take_complete_reconnect_completion(
        &mut self,
        _: &CompleteReconnectDurabilityOperationV1,
    ) -> Result<Option<CompleteReconnectCompletionV1>, ReconnectDurabilityErrorV1> {
        Ok(self.0.take())
    }
}
fn report(flow: &mut CompleteReconnectFlowV1, outcome: CompleteReconnectOutcomeV1) {
    let operation = flow.operation().clone();
    flow.accept_completion(&mut Report(Some(CompleteReconnectCompletionV1 {
        operation,
        outcome,
    })))
    .require("completion");
}
fn prepare_bridge(
    owner: &mut BridgeOwner,
    identity: ReconnectIdentityV1,
    token: &str,
    v2: bool,
) -> CompleteReconnectFlowV1 {
    let auth = CompleteReconnectAuthorizationV1::authorize(
        owner,
        identity,
        proof(owner, token, v2, 100),
        100,
    )
    .require("authorize");
    let claims = if auth.operation().mode == CompleteReconnectModeV1::EarlyTerminalReplacement {
        Some(
            CompleteReconnectClaimTransitionV1::prepare(owner, &auth, 100)
                .require("claim transition"),
        )
    } else {
        None
    };
    let mut flow = CompleteReconnectFlowV1::begin(auth, claims).require("flow");
    let request = flow
        .take_request(CompleteReconnectRequestKindV1::Prepare)
        .require("prepare request");
    let effect = request
        .validate_locked(owner, 100)
        .require("prepare effect");
    assert_eq!(
        effect.session().session_state(),
        owner.current.snapshot.session.session_state()
    );
    assert_eq!(effect.protection(), owner.current.snapshot.protection);
    owner.current.snapshot.budget = effect.budget().clone();
    owner.current.prepared = Some(Box::new(flow.operation().recovery.clone()));
    report(
        &mut flow,
        CompleteReconnectOutcomeV1::Prepared { decided_at: 100 },
    );
    flow
}
#[test]
fn complete_bridge_real_not_entitled_v1_v2_and_replacement_round_trip() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        let (mut owner, identity, token) = bridge_fixture(replacement);
        let mut flow = prepare_bridge(&mut owner, identity, &token, v2);
        let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(
            flow.operation().recovery.clone(),
            proof(&owner, &token, v2, 101),
            &owner,
            101,
        )
        .require("fresh final authorization");
        flow.resume_prepared(authorization, &owner, 101)
            .require("resume");
        let request = flow
            .take_request(CompleteReconnectRequestKindV1::Commit)
            .require("commit");
        let effect = request.validate_locked(&owner, 101).require("final");
        assert_eq!(
            effect.protection().usage,
            RecoveryProtectionUseV1::NotEntitled
        );
        assert_eq!(effect.session().current_connection_generation().get(), 2);
        assert_eq!(effect.budget().state(), RecoveryEpochStateV1::Restored);
        report(
            &mut flow,
            CompleteReconnectOutcomeV1::Committed { decided_at: 101 },
        );
        assert!(flow.adopt_current(&owner, 101).is_err());
        owner.current.snapshot.session = effect.session();
        owner.current.snapshot.budget = effect.budget().clone();
        owner.current.snapshot.protection = effect.protection();
        owner.current.snapshot.claims = effect.claims().to_vec();
        owner.current.snapshot.observed_at = 101;
        install_proof(&mut owner, &effect, 101);
        let controller = flow
            .adopt_current(&owner, 101)
            .require("independent current adoption");
        assert_eq!(
            controller.transport(),
            effect.session().current_transport().require("transport")
        );
    }
}

#[test]
fn complete_bridge_v2_restored_authorization_rejects_regressed_provenance() {
    let (mut owner, identity, token) = bridge_fixture(false);
    let flow = prepare_bridge(&mut owner, identity, &token, true);
    owner.security.security.provenance.source_revision = 6;
    owner.security.security.provenance.accepted_source_revision = 6;
    let fresh = proof(&owner, &token, true, 101);
    assert!(
        CompleteReconnectAuthorizationV1::reauthorize_history(
            flow.operation().recovery.clone(),
            fresh,
            &owner,
            101
        )
        .is_err()
    );
}
#[test]
fn complete_bridge_current_loss_must_match_retained_commit_and_generation() {
    let (mut owner, identity, token) = bridge_fixture(false);
    owner.current.snapshot.session.commit.initial_transport =
        AuthenticatedTransportRefV1::decode(&[77; 16]).require("other");
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            proof(&owner, &token, false, 100),
            100
        )
        .is_err()
    );
}

fn mutate_current(owner: &mut BridgeOwner, case: usize) {
    let s = &mut owner.current.snapshot;
    match case {
        0 => s.loss_decided_at += 1,
        1 => {
            s.source_revision = 6;
            s.accepted_source_revision = 6;
        }
        2 => s.observed_at = 99,
        3 => s.actor_present = false,
        4 => s.runtime_ready = false,
        5 => s.placement_identity = [8; 16],
        6 => s.placement_revision += 1,
        7 => {
            s.session.session_state = GameSessionState::Active;
            s.session.current_transport = Some(s.candidate.transport_ref());
        }
        8 => s.session.current_character_lease.generation += 1,
        9 => s.session.current_scope_generation = ScopeOwnershipGeneration::new(4).require("scope"),
        10 => {
            s.candidate.transport_ref =
                AuthenticatedTransportRefV1::decode(&[8; 16]).require("transport")
        }
        11 => {
            s.candidate.connection_generation = ConnectionGeneration::new(3).require("generation")
        }
        12 => {
            s.session.current_control_loss_epoch =
                Some(ControlLossEpochRefV1::new(2).require("epoch"))
        }
        13 => s.session.current_original_grace_deadline = Some(121),
        14 => {
            s.protection.usage = RecoveryProtectionUseV1::Unused {
                entitlement_generation: 91,
            }
        }
        15 => {
            s.protection.rearm = RecoveryProtectionRearmV1::Satisfied {
                generation: 99,
                established_at: 90,
            }
        }
        16 => s.budget.entries.push(RetainedRecoveryAttemptV1 {
            attempt: ReconnectAttemptRef::new(31).require("attempt"),
            transport: AuthenticatedTransportRefV1::decode(&[7; 16]).require("transport"),
            disposition: RetainedRecoveryAttemptDispositionV1::Terminal,
        }),
        17 => s.fnd02.server_sequence += 1,
        18 => s.recovery.map_revision = "map-2".into(),
        19 => s.claims.clear(),
        20 => s.claims[1].source.source_revision += 1,
        21 => owner.security.security.minimum_generation = 2,
        22 => owner.security.security.allowed = false,
        23 => owner.security.signing.trusted = false,
        24 => s.account_presence.account_id = "00000000-0000-4000-8000-000000000099".into(),
        _ => unreachable!(),
    }
}
#[test]
fn complete_bridge_independent_final_authority_matrix() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        let (mut owner, identity, token) = bridge_fixture(replacement);
        let mut flow = prepare_bridge(&mut owner, identity, &token, v2);
        let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(
            flow.operation().recovery.clone(),
            proof(&owner, &token, v2, 101),
            &owner,
            101,
        )
        .require("authorization");
        flow.resume_prepared(authorization, &owner, 101)
            .require("resume");
        let request = flow
            .take_request(CompleteReconnectRequestKindV1::Commit)
            .require("request");
        assert!(request.validate_locked(&owner, 101).is_ok());
        for case in 0..25 {
            let mut changed = owner.clone();
            mutate_current(&mut changed, case);
            assert!(
                request.validate_locked(&changed, 101).is_err(),
                "replacement={replacement}, v2={v2}, case={case}"
            );
        }
        assert!(request.validate_locked(&owner, 99).is_err());
        assert!(request.validate_locked(&owner, 104).is_err());
        assert!(request.validate_locked(&owner, 103).is_ok());
    }
}

#[test]
fn complete_bridge_restart_is_reconciliation_only_and_original_l_is_immutable() {
    let (mut owner, identity, token) = bridge_fixture(false);
    let mut flow = prepare_bridge(&mut owner, identity, &token, false);
    let operation = flow.operation().clone();
    let mut restored = CompleteReconnectFlowV1::restore(operation.clone()).require("history");
    assert!(
        restored
            .take_request(CompleteReconnectRequestKindV1::Commit)
            .is_err()
    );
    assert!(
        restored
            .take_request(CompleteReconnectRequestKindV1::Prepare)
            .is_err()
    );
    let request = restored
        .take_request(CompleteReconnectRequestKindV1::Reconcile)
        .require("read");
    assert!(request.validate_locked(&owner, 101).is_err());
    report(
        &mut restored,
        CompleteReconnectOutcomeV1::Prepared { decided_at: 100 },
    );
    assert!(
        restored
            .take_request(CompleteReconnectRequestKindV1::Commit)
            .is_err()
    );
    let auth = CompleteReconnectAuthorizationV1::reauthorize_history(
        operation.recovery.clone(),
        proof(&owner, &token, false, 101),
        &owner,
        101,
    )
    .require("fresh proof");
    restored
        .resume_prepared(auth, &owner, 101)
        .require("resume");
    assert!(
        restored
            .take_request(CompleteReconnectRequestKindV1::Commit)
            .require("commit")
            .validate_locked(&owner, 101)
            .is_ok()
    );
    report(
        &mut flow,
        CompleteReconnectOutcomeV1::Committed { decided_at: 101 },
    );
    let receipt = flow.receipt().require("receipt").clone();
    owner.security.security.allowed = false;
    report(
        &mut flow,
        CompleteReconnectOutcomeV1::Committed { decided_at: 101 },
    );
    assert_eq!(flow.receipt(), Some(&receipt));
    let bad = CompleteReconnectCompletionV1 {
        operation: operation.clone(),
        outcome: CompleteReconnectOutcomeV1::Committed { decided_at: 102 },
    };
    assert_eq!(
        flow.accept_completion(&mut Report(Some(bad))),
        Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
    );
    let mut changed = operation;
    changed.recovery.original.loss_decided_at = 101;
    assert!(
        flow.accept_completion(&mut Report(Some(CompleteReconnectCompletionV1 {
            operation: changed,
            outcome: CompleteReconnectOutcomeV1::Committed { decided_at: 101 }
        })))
        .is_err()
    );
}

fn replace_protection(
    owner: &mut BridgeOwner,
    protection: RecoveryProtectionContinuityV1,
    resumed: bool,
) {
    let s = &mut owner.current.snapshot;
    s.protection = protection;
    s.loss.observation.protection = protection;
    if resumed {
        let prior_epoch = ControlLossEpochRefV1::new(1).require("epoch");
        let transport = s
            .loss
            .observation
            .session
            .current_transport()
            .require("prior controller");
        let prior = RetainedRecoveryBudgetV1::restore(
            prior_epoch,
            RecoveryEpochStateV1::Restored,
            true,
            vec![RetainedRecoveryAttemptV1 {
                attempt: ReconnectAttemptRef::new(1).require("attempt"),
                transport,
                disposition: RetainedRecoveryAttemptDispositionV1::Committed,
            }],
        )
        .require("prior history");
        let loss = &mut s.loss.observation;
        loss.session.current_control_loss_epoch = Some(prior_epoch);
        loss.session.current_original_grace_deadline = Some(95);
        loss.history = ControlLossHistoryV1::Resumed {
            budget: prior,
            original_grace_deadline: 95,
            protection,
        };
        let epoch = ControlLossEpochRefV1::new(2).require("next epoch");
        loss.loss_epoch = epoch;
        loss.decision_identity = epoch;
        loss.accepted_decision_identity = epoch;
        s.session.current_control_loss_epoch = Some(epoch);
        s.budget =
            RetainedRecoveryBudgetV1::restore(epoch, RecoveryEpochStateV1::Open, true, vec![])
                .require("next budget");
    }
    s.loss.validate_historical().require("lawful complete loss");
}
#[test]
fn complete_bridge_all_protection_histories_and_budgets() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        for activated in [false, true] {
            let (mut owner, identity, token) = bridge_fixture(replacement);
            let protection = RecoveryProtectionContinuityV1 {
                usage: if activated {
                    RecoveryProtectionUseV1::Activated {
                        entitlement_generation: 91,
                        activated_at: 90,
                        deadline: 94,
                    }
                } else {
                    RecoveryProtectionUseV1::Unused {
                        entitlement_generation: 91,
                    }
                },
                rearm: RecoveryProtectionRearmV1::NotRearmed {
                    generation: 17,
                    stable_control_started_at: Some(80),
                    accepted_deadline: Some(140),
                },
            };
            replace_protection(&mut owner, protection, activated);
            for attempt in 1..8 {
                owner
                    .current
                    .snapshot
                    .budget
                    .entries
                    .push(RetainedRecoveryAttemptV1 {
                        attempt: ReconnectAttemptRef::new(attempt + 30).require("attempt"),
                        transport: AuthenticatedTransportRefV1::decode(&[attempt as u8; 16])
                            .require("transport"),
                        disposition: RetainedRecoveryAttemptDispositionV1::Terminal,
                    });
            }
            let mut full = owner.clone();
            full.current
                .snapshot
                .budget
                .entries
                .push(RetainedRecoveryAttemptV1 {
                    attempt: ReconnectAttemptRef::new(99).require("attempt"),
                    transport: AuthenticatedTransportRefV1::decode(&[99; 16]).require("transport"),
                    disposition: RetainedRecoveryAttemptDispositionV1::TransportCollision,
                });
            assert!(
                CompleteReconnectAuthorizationV1::authorize(
                    &full,
                    identity.clone(),
                    proof(&full, &token, v2, 100),
                    100
                )
                .is_err()
            );
            let mut flow = prepare_bridge(&mut owner, identity, &token, v2);
            assert_eq!(owner.current.snapshot.budget.entries().len(), 8);
            let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(
                flow.operation().recovery.clone(),
                proof(&owner, &token, v2, 101),
                &owner,
                101,
            )
            .require("last slot retries");
            flow.resume_prepared(authorization, &owner, 101)
                .require("resume");
            let effect = flow
                .take_request(CompleteReconnectRequestKindV1::Commit)
                .require("request")
                .validate_locked(&owner, 101)
                .require("commit");
            assert_eq!(effect.budget().entries().len(), 8);
            assert_eq!(
                &effect.budget().entries()[..7],
                &owner.current.snapshot.budget.entries()[..7]
            );
            assert_eq!(effect.protection().rearm, protection.rearm);
            assert_eq!(
                effect.protection().usage,
                if activated {
                    protection.usage
                } else {
                    RecoveryProtectionUseV1::Activated {
                        entitlement_generation: 91,
                        activated_at: 101,
                        deadline: 105,
                    }
                }
            );
            assert_eq!(effect.session().current_connection_generation().get(), 2);
            assert_eq!(effect.operation().recovery.original.loss_decided_at, 100);
        }
    }
}

#[test]
fn complete_bridge_v2_attempt_cannot_extend_original_verified_deadline() {
    let (mut owner, identity, token) = bridge_fixture(false);
    owner.current.snapshot.candidate.prepared_deadline = 110;
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            proof(&owner, &token, true, 100),
            100
        )
        .is_err()
    );
}
#[test]
fn complete_bridge_history_checks_signed_identity_and_common_fields() {
    let (owner, identity, token) = bridge_fixture(false);
    let auth = CompleteReconnectAuthorizationV1::authorize(
        &owner,
        identity,
        proof(&owner, &token, true, 100),
        100,
    )
    .require("authorized");
    let mut op = auth.operation().clone();
    if let CompleteReconnectCredentialV1::Recovery(value) = &mut op.credential {
        value.grant_nonce = [5; 32];
    }
    assert!(op.validate_historical().is_err());
    let mut op = auth.operation().clone();
    if let CompleteReconnectCredentialV1::Recovery(value) = &mut op.credential {
        value.revisions[2] = "other-map".into();
    }
    assert!(op.validate_historical().is_err());
}

fn committed_bridge(replacement: bool, v2: bool) -> (CompleteReconnectFlowV1, BridgeOwner) {
    let (mut owner, identity, token) = bridge_fixture(replacement);
    let mut flow = prepare_bridge(&mut owner, identity, &token, v2);
    let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(
        flow.operation().recovery.clone(),
        proof(&owner, &token, v2, 101),
        &owner,
        101,
    )
    .require("authorization");
    flow.resume_prepared(authorization, &owner, 101)
        .require("resume");
    let effect = flow
        .take_request(CompleteReconnectRequestKindV1::Commit)
        .require("request")
        .validate_locked(&owner, 101)
        .require("commit");
    report(
        &mut flow,
        CompleteReconnectOutcomeV1::Committed { decided_at: 101 },
    );
    owner.current.snapshot.session = effect.session();
    owner.current.snapshot.budget = effect.budget().clone();
    owner.current.snapshot.protection = effect.protection();
    owner.current.snapshot.claims = effect.claims().to_vec();
    owner.current.snapshot.observed_at = 101;
    install_proof(&mut owner, &effect, 101);
    (flow, owner)
}
#[test]
fn complete_bridge_adoption_revalidates_current_trust_without_reopening_expiry() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        let (mut flow, mut owner) = committed_bridge(replacement, v2);
        assert!(flow.adopt_current(&owner, 101).is_ok());
        for case in [21, 22, 23] {
            let mut changed = owner.clone();
            mutate_current(&mut changed, case);
            assert!(
                flow.adopt_current(&changed, 101).is_err(),
                "v2={v2},case={case}"
            );
        }
        owner.current.snapshot.observed_at = 120;
        if let Some(proof) = &mut owner.active_proof {
            proof.observed_at = 120;
            proof.revision += 1;
            proof.accepted_revision += 1;
        }
        for provenance in [
            &mut owner.security.signing.provenance,
            &mut owner.security.security.provenance,
        ] {
            provenance.source_observed_at = 120;
            provenance.source_revision += 1;
            provenance.accepted_source_revision += 1;
            provenance.publication_revision += 1;
            provenance.decision_identity = "refreshed".into();
            provenance.accepted_decision_identity = "refreshed".into();
        }
        assert!(
            flow.adopt_current(&owner, 120).is_ok(),
            "current committed controller survives original JWT expiry"
        );
    }
}

#[test]
fn complete_bridge_fast_proof_requires_sealed_current_binding() {
    let (owner, identity, _) = bridge_fixture(false);
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            CompleteReconnectProofV1::Fast,
            100
        )
        .is_err(),
        "no source-verified bearer binding"
    );
}

fn enable_fast(owner: &mut BridgeOwner, identity: &ReconnectIdentityV1) {
    let evidence = |purpose| {
        AuthorityEvidenceFenceV1::new(
            "owning-source",
            purpose,
            "existing-session",
            "7",
            "accepted-decision",
            100,
        )
        .require("source evidence")
    };
    let compatibility = ReconnectCompatibilityEvidenceV1::new(
        1,
        1,
        "rules-1",
        "content-1",
        "map-1",
        "policy-1",
        1,
        evidence("account-security"),
        evidence("fast-proof"),
        None,
    )
    .require("compatibility");
    owner.fast = Some(CompleteFastReconnectBindingV1 {
        session: identity.game_session_id(),
        predecessor: owner
            .current
            .snapshot
            .session
            .current_connection_generation(),
        attempt: identity.reconnect_attempt_ref(),
        transport: owner.current.snapshot.candidate.transport_ref(),
        proof_generation: 41,
        replacement_proof_generation: 43,
        verified_at: 100,
        compatibility,
    });
}
#[test]
fn complete_bridge_fast_roundtrip_rotates_only_at_commit_and_checks_proof_owner() {
    for protection_case in 0..3 {
        let (mut owner, identity, _) = bridge_fixture(false);
        if protection_case > 0 {
            let protection = RecoveryProtectionContinuityV1 {
                usage: if protection_case == 1 {
                    RecoveryProtectionUseV1::Unused {
                        entitlement_generation: 91,
                    }
                } else {
                    RecoveryProtectionUseV1::Activated {
                        entitlement_generation: 91,
                        activated_at: 90,
                        deadline: 94,
                    }
                },
                rearm: RecoveryProtectionRearmV1::NotRearmed {
                    generation: 17,
                    stable_control_started_at: None,
                    accepted_deadline: None,
                },
            };
            replace_protection(&mut owner, protection, protection_case == 2);
        }
        enable_fast(&mut owner, &identity);
        let auth = CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            CompleteReconnectProofV1::Fast,
            100,
        )
        .require("actual owner verified proof");
        let mut flow = CompleteReconnectFlowV1::begin(auth, None).require("flow");
        let prepare = flow
            .take_request(CompleteReconnectRequestKindV1::Prepare)
            .require("prepare")
            .validate_locked(&owner, 100)
            .require("reserve");
        assert_eq!(prepare.fast_proof_rotation(), None);
        owner.current.snapshot.budget = prepare.budget().clone();
        owner.current.prepared = Some(Box::new(flow.operation().recovery.clone()));
        report(
            &mut flow,
            CompleteReconnectOutcomeV1::Prepared { decided_at: 100 },
        );
        let auth = CompleteReconnectAuthorizationV1::reauthorize_history(
            flow.operation().recovery.clone(),
            CompleteReconnectProofV1::Fast,
            &owner,
            101,
        )
        .require("current proof rechecked");
        flow.resume_prepared(auth, &owner, 101).require("resume");
        let request = flow
            .take_request(CompleteReconnectRequestKindV1::Commit)
            .require("commit request");
        assert!(request.validate_locked(&owner, 101).is_ok());
        for case in 0..5 {
            let mut changed = owner.clone();
            let binding = changed.fast.as_mut().require("binding");
            match case {
                0 => binding.proof_generation = 42,
                1 => {
                    binding.transport =
                        AuthenticatedTransportRefV1::decode(&[7; 16]).require("transport")
                }
                2 => binding.predecessor = ConnectionGeneration::new(2).require("generation"),
                3 => binding.replacement_proof_generation = 44,
                4 => changed.fast = None,
                _ => unreachable!(),
            }
            assert!(
                request.validate_locked(&changed, 101).is_err(),
                "fast case {case}"
            );
        }
        let effect = request.validate_locked(&owner, 101).require("effect");
        assert_eq!(effect.fast_proof_rotation(), Some((41, 43)));
        report(
            &mut flow,
            CompleteReconnectOutcomeV1::Committed { decided_at: 101 },
        );
        owner.current.snapshot.session = effect.session();
        owner.current.snapshot.budget = effect.budget().clone();
        owner.current.snapshot.protection = effect.protection();
        owner.current.snapshot.observed_at = 101;
        install_proof(&mut owner, &effect, 101);
        assert!(
            flow.adopt_current(&owner, 101).is_err(),
            "missing rotated proof owner"
        );
        owner.fast_adoption = Some(CompleteFastReconnectAdoptionV1 {
            session: effect.session().commit().game_session_id(),
            connection: effect.session().current_connection_generation(),
            transport: effect.session().current_transport().require("transport"),
            proof_generation: 43,
            observed_at: 101,
            compatibility: owner.fast.as_ref().require("binding").compatibility.clone(),
        });
        assert!(flow.adopt_current(&owner, 101).is_ok());
        for case in 0..4 {
            let mut changed = owner.clone();
            match case {
                0 => changed.security.security.allowed = false,
                1 => changed.security.security.minimum_generation = 2,
                2 => changed.active_proof = None,
                3 => changed.active_proof.as_mut().require("proof").observed_at = 100,
                _ => unreachable!(),
            }
            assert!(
                flow.adopt_current(&changed, 101).is_err(),
                "fast current security/proof case {case}"
            );
        }
        owner
            .fast_adoption
            .as_mut()
            .require("adoption")
            .proof_generation = 41;
        assert!(
            flow.adopt_current(&owner, 101).is_err(),
            "old bearer generation cannot adopt"
        );
    }
}

#[test]
fn complete_bridge_current_claims_enforce_eligibility_and_shared_security_floor() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        for case in 0..3 {
            let (mut owner, identity, token) = bridge_fixture(replacement);
            match case {
                0 => {
                    if let AdmissionAuthorityGuardStateV1::Account { security, .. } =
                        &mut owner.current.snapshot.claims[0].state
                    {
                        security.minimum_generation = 2;
                    }
                }
                1 => {
                    if let AdmissionAuthorityGuardStateV1::Account { security, .. } =
                        &mut owner.current.snapshot.claims[0].state
                    {
                        security.allowed = false;
                    }
                }
                2 => {
                    if let AdmissionAuthorityGuardStateV1::Character { eligible, .. } =
                        &mut owner.current.snapshot.claims[1].state
                    {
                        *eligible = false;
                    }
                }
                _ => unreachable!(),
            }
            assert!(
                CompleteReconnectAuthorizationV1::authorize(
                    &owner,
                    identity,
                    proof(&owner, &token, v2, 100),
                    100
                )
                .is_err(),
                "replacement={replacement},case={case}"
            );
        }
    }
}
#[test]
fn complete_bridge_v1_capture_has_no_partial_success_and_checks_current_key_floor() {
    let (owner, identity, token) = bridge_fixture(false);
    let auth = CompleteReconnectAuthorizationV1::authorize(
        &owner,
        identity.clone(),
        proof(&owner, &token, false, 100),
        100,
    )
    .require("verified capture");
    let captured = auth
        .operation()
        .credential
        .recovery()
        .require("recovery")
        .v1_trust
        .as_ref()
        .require("v1");
    assert_eq!(captured.signing_key_id, "recovery-1");
    assert_eq!(
        captured.signing_public_key,
        owner.security.signing.public_key
    );
    assert_eq!(captured.minimum_generation, 1);
    let mut invalid = token.clone();
    invalid.push('x');
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            CompleteReconnectProofV1::V1Token(invalid),
            100
        )
        .is_err()
    );
    assert!(!format!("{:?}", CompleteReconnectProofV1::V1Token(token.clone())).contains(&token));
    let (mut flow, owner) = committed_bridge(false, false);
    for case in 0..3 {
        let mut changed = owner.clone();
        match case {
            0 => changed.security.signing.public_key = [1; 32],
            1 => changed.security.security.minimum_generation = 0,
            2 => changed.security.signing.trusted = false,
            _ => unreachable!(),
        }
        assert!(flow.adopt_current(&changed, 101).is_err());
    }
}

#[test]
fn complete_bridge_prepare_and_adoption_independent_current_matrix() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        let (owner, identity, token) = bridge_fixture(replacement);
        let auth = CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            proof(&owner, &token, v2, 100),
            100,
        )
        .require("authorize");
        let claims = if replacement {
            Some(CompleteReconnectClaimTransitionV1::prepare(&owner, &auth, 100).require("claims"))
        } else {
            None
        };
        let mut flow = CompleteReconnectFlowV1::begin(auth, claims).require("flow");
        let request = flow
            .take_request(CompleteReconnectRequestKindV1::Prepare)
            .require("prepare");
        assert!(request.validate_locked(&owner, 101).is_ok());
        for case in 0..25 {
            let mut changed = owner.clone();
            mutate_current(&mut changed, case);
            assert!(
                request.validate_locked(&changed, 101).is_err(),
                "prepare replacement={replacement},v2={v2},case={case}"
            );
        }
        let (mut flow, owner) = committed_bridge(replacement, v2);
        assert!(flow.adopt_current(&owner, 101).is_ok());
        for case in (0..25).filter(|case| *case != 7) {
            let mut changed = owner.clone();
            mutate_current(&mut changed, case);
            assert!(
                flow.adopt_current(&changed, 101).is_err(),
                "adoption replacement={replacement},v2={v2},case={case}"
            );
        }
        let mut changed = owner.clone();
        changed.current.snapshot.session.current_transport =
            Some(AuthenticatedTransportRefV1::decode(&[77; 16]).require("other transport"));
        assert!(flow.adopt_current(&changed, 101).is_err());
    }
}
struct BadClaims<'a>(&'a BridgeOwner, usize);
impl super::super::fnd04_verifier::recovery_source_sealed::Sealed for BadClaims<'_> {}
impl CompleteReconnectClaimSourceV1 for BadClaims<'_> {
    fn prepare_complete_reconnect_claim(
        &self,
        operation: &CompleteReconnectOperationV1,
        now: i64,
    ) -> Result<CompleteReconnectClaimResolutionV1, AdmissionAuthorityPublicationErrorV1> {
        let mut resolution = self.0.prepare_complete_reconnect_claim(operation, now)?;
        match self.1 {
            0 => {
                if let AdmissionAuthorityGuardStateV1::Account { presence, .. } =
                    &mut resolution.transition.successors[0].state
                {
                    *presence = Some((
                        operation.identity.character_id(),
                        operation.original.session.commit().game_session_id(),
                    ));
                }
            }
            1 => {
                if let AdmissionAuthorityGuardStateV1::Character { holder, .. } =
                    &mut resolution.transition.successors[1].state
                {
                    *holder = Some(operation.original.session.commit().game_session_id());
                }
            }
            2 => {
                if let AdmissionAuthorityGuardStateV1::Character {
                    lease_generation, ..
                } = &mut resolution.transition.successors[1].state
                {
                    *lease_generation += 1;
                }
            }
            3 => resolution.transition.predecessors[0].publication_revision += 1,
            4 => resolution.transition.successors[1].source.authority = "other-owner".into(),
            5 => resolution.transition.prepared_at += 1,
            _ => unreachable!(),
        }
        Ok(resolution)
    }
}
#[test]
fn complete_bridge_terminal_claim_effect_binds_exact_successor_and_predecessors() {
    let (owner, identity, token) = bridge_fixture(true);
    let auth = CompleteReconnectAuthorizationV1::authorize(
        &owner,
        identity,
        proof(&owner, &token, true, 100),
        100,
    )
    .require("auth");
    assert!(CompleteReconnectClaimTransitionV1::prepare(&owner, &auth, 100).is_ok());
    for case in 0..6 {
        assert!(
            CompleteReconnectClaimTransitionV1::prepare(&BadClaims(&owner, case), &auth, 100)
                .is_err(),
            "case {case}"
        );
    }
}

#[test]
fn complete_bridge_full_reconciliation_payload_survives_operation_and_commit() {
    let (mut owner, identity, token) = bridge_fixture(false);
    owner.current.snapshot.fnd02 = Fnd02ReconciliationFenceV1::new(
        CommandId::new(65).require("command"),
        (1..=64)
            .map(|id| {
                PendingCommandReconciliationV1::new(
                    CommandId::new(id).require("pending"),
                    PendingCommandDispositionV1::PendingOriginal,
                )
            })
            .collect(),
        77,
        (1..=256)
            .map(|domain| StateDomainRevisionV1::new(domain, 1).require("revision"))
            .collect(),
    )
    .require("full reconciliation");
    let expected = owner.current.snapshot.fnd02.clone();
    let mut flow = prepare_bridge(&mut owner, identity, &token, false);
    let auth = CompleteReconnectAuthorizationV1::reauthorize_history(
        flow.operation().recovery.clone(),
        proof(&owner, &token, false, 101),
        &owner,
        101,
    )
    .require("auth");
    flow.resume_prepared(auth, &owner, 101).require("resume");
    let effect = flow
        .take_request(CompleteReconnectRequestKindV1::Commit)
        .require("request")
        .validate_locked(&owner, 101)
        .require("effect");
    assert_eq!(effect.operation().recovery.original.fnd02, expected);
    assert_eq!(
        effect.operation().recovery.original.fnd02.pending().len(),
        64
    );
    assert_eq!(
        effect
            .operation()
            .recovery
            .original
            .fnd02
            .domain_revisions()
            .len(),
        256
    );
}
#[test]
fn complete_bridge_restored_completion_cannot_report_prepare_after_commit() {
    let (flow, _) = committed_bridge(false, false);
    let mut restored =
        CompleteReconnectFlowV1::restore(flow.operation().clone()).require("restore");
    report(
        &mut restored,
        CompleteReconnectOutcomeV1::Committed { decided_at: 101 },
    );
    let report = CompleteReconnectCompletionV1 {
        operation: restored.operation().clone(),
        outcome: CompleteReconnectOutcomeV1::Prepared { decided_at: 102 },
    };
    assert_eq!(
        restored.accept_completion(&mut Report(Some(report))),
        Err(ReconnectDurabilityErrorV1::IdempotencyConflict)
    );
}
#[test]
fn complete_bridge_unbounded_fields_and_unknown_versions_fail_closed() {
    let (owner, identity, token) = bridge_fixture(false);
    let auth = CompleteReconnectAuthorizationV1::authorize(
        &owner,
        identity.clone(),
        proof(&owner, &token, false, 100),
        100,
    )
    .require("auth");
    let mut unknown = auth.operation().clone();
    unknown.version = 2;
    assert!(unknown.validate_historical().is_err());
    let mut changed = owner.clone();
    changed.current.snapshot.claims[0].source.authority = "x".repeat(65_537);
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &changed,
            identity.clone(),
            proof(&changed, &token, false, 100),
            100
        )
        .is_err()
    );
    let mut changed = owner.clone();
    changed.current.snapshot.recovery.map_revision = "x".repeat(65_537);
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &changed,
            identity,
            CompleteReconnectProofV1::V1Token(token),
            100
        )
        .is_err()
    );
}

#[test]
fn complete_bridge_unresolved_prepared_budget_cannot_authorize_another_candidate() {
    let (mut owner, identity, token) = bridge_fixture(false);
    owner
        .current
        .snapshot
        .budget
        .entries
        .push(RetainedRecoveryAttemptV1 {
            attempt: ReconnectAttemptRef::new(99).require("other attempt"),
            transport: AuthenticatedTransportRefV1::decode(&[77; 16]).require("other transport"),
            disposition: RetainedRecoveryAttemptDispositionV1::Prepared,
        });
    assert!(
        CompleteReconnectAuthorizationV1::authorize(
            &owner,
            identity,
            proof(&owner, &token, false, 100),
            100
        )
        .is_err()
    );
}

#[test]
fn complete_bridge_current_claim_history_must_be_well_formed() {
    for case in 0..4 {
        let (mut owner, identity, token) = bridge_fixture(false);
        match case {
            0 => owner.current.snapshot.claims[0].source.source_observed_at = 101,
            1 => owner.current.snapshot.claims[1].source.source_revision = 0,
            2 => owner.current.snapshot.claims[0].publication_revision = 2,
            3 => {
                if let AdmissionAuthorityGuardStateV1::Account { security, .. } =
                    &mut owner.current.snapshot.claims[0].state
                {
                    security.provenance.accepted_source_revision = 5;
                }
            }
            _ => unreachable!(),
        }
        assert!(
            CompleteReconnectAuthorizationV1::authorize(
                &owner,
                identity,
                proof(&owner, &token, false, 100),
                100
            )
            .is_err(),
            "case {case}"
        );
    }
}

#[test]
fn complete_bridge_recovery_commit_requires_common_proof_transition() {
    let (mut owner, identity, token) = bridge_fixture(false);
    let mut flow = prepare_bridge(&mut owner, identity, &token, false);
    let auth = CompleteReconnectAuthorizationV1::reauthorize_history(
        flow.operation().recovery.clone(),
        proof(&owner, &token, false, 101),
        &owner,
        101,
    )
    .require("auth");
    flow.resume_prepared(auth, &owner, 101).require("resume");
    let effect = flow
        .take_request(CompleteReconnectRequestKindV1::Commit)
        .require("request")
        .validate_locked(&owner, 101)
        .require("commit");
    assert!(
        effect.proof_transition().is_some(),
        "recovery also rotates the session proof"
    );
}

fn install_proof(owner: &mut BridgeOwner, effect: &CompleteReconnectEffectV1, now: i64) {
    let transition = effect
        .proof_transition()
        .require("committed proof transition");
    owner.active_proof = Some(CompleteReconnectProofCurrentV1 {
        owner: transition.owner,
        revision: transition.revision + 1,
        accepted_revision: transition.revision + 1,
        observed_at: now,
        session: transition.successor_session,
        connection: transition.candidate.connection_generation(),
        transport: transition.candidate.transport_ref(),
        proof_generation: transition.successor_generation,
    });
}

#[test]
fn complete_bridge_all_modes_require_current_activated_successor_proof() {
    for (replacement, v2) in [(false, false), (false, true), (true, true)] {
        let (mut flow, owner) = committed_bridge(replacement, v2);
        assert!(flow.adopt_current(&owner, 101).is_ok());
        for case in 0..8 {
            let mut changed = owner.clone();
            let proof = changed.active_proof.as_mut().require("proof");
            match case {
                0 => proof.proof_generation += 1,
                1 => proof.connection = ConnectionGeneration::new(3).require("generation"),
                2 => {
                    proof.transport =
                        AuthenticatedTransportRefV1::decode(&[6; 16]).require("transport")
                }
                3 => proof.observed_at = 100,
                4 => {
                    proof.revision = 11;
                    proof.accepted_revision = 11;
                }
                5 => proof.accepted_revision = 10,
                6 => proof.observed_at = 102,
                7 => changed.active_proof = None,
                _ => unreachable!(),
            }
            assert!(
                flow.adopt_current(&changed, 101).is_err(),
                "replacement={replacement},v2={v2},proof case={case}"
            );
        }
    }
}

#[test]
fn early_terminal_prepare_transfers_claims_before_candidate_commit() {
    let (owner, identity, token) = bridge_fixture(true);
    let auth = CompleteReconnectAuthorizationV1::authorize(
        &owner, identity, proof(&owner, &token, true, 100), 100,
    ).require("terminal authorization");
    let transition = CompleteReconnectClaimTransitionV1::prepare(&owner, &auth, 100)
        .require("independent claim source");
    let mut flow = CompleteReconnectFlowV1::begin(auth, Some(transition)).require("flow");
    let expected = flow.operation().replacement.as_ref().require("receipt")
        .transition.successors.clone();
    let effect = flow.take_request(CompleteReconnectRequestKindV1::Prepare).require("request")
        .validate_locked(&owner, 100).require("prepare");
    assert_eq!(effect.claims(), expected);
    assert_eq!(effect.session().commit(), owner.current.snapshot.session.commit());
}
