// Generated from the accepted public test fixture in tests/support/postgres.rs.
// Only explicit fixture identities and interpretation label are adapted; original
// typed verification, publication, claim preparation and commit flow are retained.
type TestResult<T = ()> = super::TestResult<T>;
fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
    value.map_err(super::fixture::debug)
}
fn uuid(tag: u8) -> [u8; 16] {
    super::fixture::id(match tag {
        2 => 41,
        3 => 42,
        4 => 43,
        other => other,
    })
}
fn session(_: u8) -> TestResult<crate::foundation::GameSessionId> {
    checked(crate::foundation::GameSessionId::decode(
        &super::fixture::id(50),
    ))
}
fn transport(_: u8) -> TestResult<crate::foundation::AuthenticatedTransportRefV1> {
    checked(crate::foundation::AuthenticatedTransportRefV1::decode(
        &[9; 16],
    ))
}
pub mod fresh {
    use super::{TestResult, checked};
    use crate::foundation::admission_authority_publication::*;
    use crate::foundation::fnd04_verifier::*;
    use crate::foundation::fresh_admission_durability::*;
    use crate::foundation::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    pub struct Source {
        pub now: i64,
        pub current: FreshCurrentEvidence,
        pub rows: Vec<AdmissionAuthorityPublicationChangeV1>,
        key: SigningKey,
    }
    impl fresh_source_sealed::Sealed for Source {}

    fn provenance(purpose: FreshEvidencePurposeV1, now: i64) -> FreshEvidenceProvenanceV1 {
        FreshEvidenceProvenanceV1 {
            source_authority: "independent-platform-fixture".into(),
            purpose,
            scope: Fnd04EvidenceScope::FreshAdmission,
            source_revision: 1,
            accepted_source_revision: 1,
            decision_identity: "platform-observation-1".into(),
            accepted_decision_identity: "platform-observation-1".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
            publication_revision: 1,
        }
    }
    impl FreshDurabilityEvidenceSourceV1 for Source {
        fn signing_trust(
            &self,
            key_id: &str,
            _now: i64,
        ) -> Result<FreshSigningTrustObservationV1, Fnd04EvidenceError> {
            if key_id != "fresh-1" {
                return Err(Fnd04EvidenceError::ExplicitlyDenied);
            }
            Ok(FreshSigningTrustObservationV1 {
                key_id: key_id.into(),
                public_key: self.key.verifying_key().to_bytes(),
                trusted: true,
                provenance: provenance(FreshEvidencePurposeV1::SigningTrust, self.now),
            })
        }
        fn account_security(
            &self,
            account_id: &str,
            _now: i64,
        ) -> Result<FreshAccountSecurityObservationV1, Fnd04EvidenceError> {
            if account_id != self.current.account_id {
                return Err(Fnd04EvidenceError::ExplicitlyDenied);
            }
            Ok(FreshAccountSecurityObservationV1 {
                account_id: account_id.into(),
                minimum_generation: 1,
                allowed: true,
                provenance: provenance(FreshEvidencePurposeV1::PlatformSecurity, self.now),
            })
        }
    }
    impl FreshDurabilityCurrentSourceV1 for Source {
        fn current(
            &self,
            account_id: &str,
            character_id: CharacterId,
            _now: i64,
        ) -> Result<FreshPublishedCurrentObservationV1, Fnd04ConsumerError> {
            if account_id != self.current.account_id || character_id != self.current.character_id {
                return Err(Fnd04ConsumerError::FreshAccountCharacterConflict);
            }
            Ok(FreshPublishedCurrentObservationV1 {
                facts: self.current.clone(),
                account_publication_revision: 1,
                character_publication_revision: 1,
                runtime_publication_revision: 1,
                expected_lease_generation: 1,
                proposed_lease_generation: 2,
                account_presence_available: true,
                character_eligible: true,
                runtime_ready: true,
            })
        }
    }
    impl AdmissionAuthorityPublicationCurrentSourceV1 for Source {
        fn current_publications(
            &self,
            keys: &[AdmissionAuthorityGuardKeyV1],
        ) -> Result<
            Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
            AdmissionAuthorityPublicationErrorV1,
        > {
            Ok(keys
                .iter()
                .map(|key| self.rows.iter().find(|row| &row.key == key).cloned())
                .collect())
        }
    }
    impl AdmissionAuthorityOwningPublisherV1 for Source {
        fn resolve_publication(
            &self,
            _now: i64,
        ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
        {
            Ok(self.rows.clone())
        }
    }
    impl AdmissionClaimOwningSourceV1 for Source {
        fn prepare_fresh_claim(
            &self,
            binding: &FreshAdmissionAuditBindingV1,
            now: i64,
        ) -> Result<AdmissionClaimTransitionEvidenceV1, AdmissionAuthorityPublicationErrorV1>
        {
            let predecessors = self.rows[..2].to_vec();
            let mut successors = predecessors.clone();
            for row in &mut successors {
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                    expected_publication_revision: row.publication_revision,
                };
                row.publication_revision = row
                    .publication_revision
                    .checked_add(1)
                    .ok_or(AdmissionAuthorityPublicationErrorV1::Invalid)?;
                row.source.source_revision = row
                    .source
                    .source_revision
                    .checked_add(1)
                    .ok_or(AdmissionAuthorityPublicationErrorV1::Invalid)?;
                row.source.decision_identity = "game-acquire-2".into();
                row.source.source_observed_at = now;
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account { security, presence } => {
                        security.provenance.publication_revision = row.publication_revision;
                        *presence = Some((self.current.character_id, binding.candidate_session));
                    }
                    AdmissionAuthorityGuardStateV1::Character {
                        lease_generation,
                        holder,
                        ..
                    } => {
                        *lease_generation = lease_generation
                            .checked_add(1)
                            .ok_or(AdmissionAuthorityPublicationErrorV1::Invalid)?;
                        *holder = Some(binding.candidate_session);
                    }
                    _ => return Err(AdmissionAuthorityPublicationErrorV1::Invalid),
                }
            }
            Ok(AdmissionClaimTransitionEvidenceV1 {
                predecessors,
                successors,
                prepared_at: now,
            })
        }
    }
    impl Source {
        pub fn new(now: i64) -> TestResult<Self> {
            let mut source = Self {
                now,
                key: SigningKey::from_bytes(&[31; 32]),
                rows: vec![],
                current: FreshCurrentEvidence {
                    account_id: "28020304-0506-7008-800a-0b0c0d0e0f28".into(),
                    character_id: checked(CharacterId::decode(&super::uuid(2)))?,
                    world_id: checked(WorldId::decode(&super::uuid(3)))?,
                    channel_id: checked(ChannelId::decode(&super::uuid(4)))?,
                    character_lease_generation: 1,
                    scope_ownership_generation: 1,
                    route_revision: "route-1".into(),
                    runtime_observation_revision: "runtime-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    content_revision: "content-1".into(),
                    map_revision: "map-1".into(),
                    world_policy_revision: "policy-1".into(),
                    offer_revision: "offer-1".into(),
                },
            };
            let f = &source.current;
            let security = checked(source.account_security(&f.account_id, now))?;
            let entries = vec![
                (
                    AdmissionAuthorityGuardKeyV1::Account {
                        account_id: f.account_id.clone(),
                    },
                    AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
                    AdmissionAuthorityGuardStateV1::Account {
                        security,
                        presence: None,
                    },
                ),
                (
                    AdmissionAuthorityGuardKeyV1::Character(f.character_id),
                    AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
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
                    AdmissionAuthorityGuardStateV1::SigningTrust {
                        public_key: source.key.verifying_key().to_bytes(),
                        trusted: true,
                    },
                ),
            ];
            source.rows = entries
                .into_iter()
                .map(
                    |(key, purpose, state)| AdmissionAuthorityPublicationChangeV1 {
                        key,
                        state,
                        publication_revision: 1,
                        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                            restored_publication_high_water: Some(0),
                        },
                        source: AdmissionPublicationSourceV1 {
                            authority: if purpose
                                == AdmissionPublicationPurposeV1::FixedFreshSigningTrust
                            {
                                "independent-platform-fixture"
                            } else {
                                "independent-game-fixture"
                            }
                            .into(),
                            purpose,
                            source_revision: 1,
                            decision_identity: if purpose
                                == AdmissionPublicationPurposeV1::FixedFreshSigningTrust
                            {
                                "platform-observation-1"
                            } else {
                                "game-observation-1"
                            }
                            .into(),
                            source_observed_at: now,
                            clock_uncertainty_seconds: 0,
                        },
                    },
                )
                .collect();
            Ok(source)
        }
        pub fn request(&self) -> TestResult<FreshAdmissionCommitRequestV1> {
            let header = r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#;
            let mut payload: serde_json::Value = serde_json::from_str(&fresh_payload())?;
            payload["iat"] = self.now.into();
            payload["nbf"] = self.now.into();
            payload["exp"] = (self.now + 10).into();
            let token = signed_token(&self.key, header, payload.to_string());
            let facts = checked(verify_fresh_grant_durability_v1(
                &token,
                self.now,
                &FreshDurabilityTrustContext::from_owning_source(self),
                &FreshDurabilityCurrentAuthorityV1::from_owning_source(self),
            ))?;
            let authorization = checked(FreshAdmissionCommitAuthorizationV1::new(
                &facts,
                super::session(9)?,
                super::transport(9)?,
                self,
                self.now,
            ))?;
            let transition = checked(FreshAdmissionClaimTransitionV1::prepare(
                self,
                &authorization,
                self.now,
            ))?;
            let mut flow = checked(FreshAdmissionDurabilityFlowV1::begin(
                authorization,
                transition,
            ))?;
            let mut capture = Capture(None);
            checked(flow.submit(&mut capture))?;
            capture
                .0
                .ok_or_else(|| "fresh fixture did not submit".into())
        }
    }
    struct Capture(Option<FreshAdmissionCommitRequestV1>);
    impl FreshAdmissionDurabilityPortV1 for Capture {
        fn submit(
            &mut self,
            request: &FreshAdmissionCommitRequestV1,
        ) -> FreshAdmissionSubmissionV1 {
            if self.0.is_some() {
                return FreshAdmissionSubmissionV1::Unavailable;
            }
            self.0 = Some(request.clone());
            FreshAdmissionSubmissionV1::Accepted
        }
        fn reconcile(&mut self, _: &FreshAdmissionOperationV1) -> FreshAdmissionSubmissionV1 {
            FreshAdmissionSubmissionV1::Unavailable
        }
    }
    fn fresh_payload() -> String {
        let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([7; 32]);
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-admission","aud":"urn:oteryn:game:admission","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-pre-admission-v1","purpose":"fresh_entry","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"28020304-0506-7008-800a-0b0c0d0e0f28","character_id":"29020304-0506-7008-800a-0b0c0d0e0f29","world_id":"2a020304-0506-7008-800a-0b0c0d0e0f2a","channel_id":"2b020304-0506-7008-800a-0b0c0d0e0f2b","account_security_generation":"1","route_revision":"route-1","runtime_observation_revision":"runtime-1","scope_ownership_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"ruleset-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1","offer_revision":"offer-1"}}"#
        )
    }

    fn signed_token(signing_key: &SigningKey, header: &str, payload: String) -> String {
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let encoded_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{encoded_header}.{encoded_payload}");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.sign(signing_input.as_bytes()).to_bytes());
        format!("{signing_input}.{signature}")
    }
}
