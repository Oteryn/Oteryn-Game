//! PostgreSQL qualification of PARTY-1's World party writer: the shared-XP toggle, its
//! command/flag authority, the scope census and presence reads, consent defaults and
//! HIDDEN privacy. Mounted from the composition root under `cfg(test)`, so only the library test target
//! compiles and runs these bodies; without OTERYN_TEST_POSTGRES_ADMIN_URL each case
//! reports PRE-ROUTING / NONCANONICAL and returns (the repository harness convention).
use crate::bestiary_postgres_harness as harness;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_item_transaction::{
    SpellItemAuthority, SpellItemError, assert_spell_item_authority_in_transaction,
};
use crate::durability::world_party::{
    PreparedWorldPartyReceipt, apply_world_party_command_in_transaction,
    read_world_party_in_transaction,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId, PartyAction};
use harness::{CHANNEL, Harness, TestResult, WORLD, debug, id};
use sqlx::{Postgres, Transaction};

// Literal existing native signed-fixture producer/bootstrap/Fresh orchestration.
// It creates no SpellItemAuthority and copies no authority/currentness guard.
// Each negative case below changes independently current native facts or an
// expected binding after real FreshAdmissionStore::commit has succeeded.
// Independent signed fixture owner derived from the existing FreshAdmission qualification.
// Production receipts are created only by FreshAdmissionStore::commit below.
mod fresh_source {
    use super::{CHANNEL, TestResult, WORLD, debug, id};
    fn checked<T, E: std::fmt::Debug>(r: Result<T, E>) -> TestResult<T> {
        r.map_err(|e| debug(e).into())
    }
    pub(super) fn uuid_text(id: &[u8; 16]) -> String {
        let h: String = id.iter().map(|v| format!("{v:02x}")).collect();
        format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        )
    }
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
        session_seed: u8,
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
        pub fn new(
            now: i64,
            character: [u8; 16],
            account_seed: u8,
            session_seed: u8,
        ) -> TestResult<Self> {
            let mut source = Self {
                now,
                key: SigningKey::from_bytes(&[31; 32]),
                session_seed,
                rows: vec![],
                current: FreshCurrentEvidence {
                    account_id: uuid_text(&id(account_seed)),
                    character_id: checked(CharacterId::decode(&character))?,
                    world_id: checked(WorldId::decode(&id(WORLD)))?,
                    channel_id: checked(ChannelId::decode(&id(CHANNEL)))?,
                    character_lease_generation: 1,
                    scope_ownership_generation: 1,
                    route_revision: "route-1".into(),
                    runtime_observation_revision: "runtime-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    content_revision: "content-1".into(),
                    map_revision: "map-1".into(),
                    world_policy_revision: "world-policy-1".into(),
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
            payload["jti"] = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode([self.session_seed; 32])
                .into();
            payload["attempt_ref"] = uuid_text(&id(self.session_seed)).into();
            payload["account_id"] = self.current.account_id.clone().into();
            payload["character_id"] = uuid_text(self.current.character_id.as_bytes()).into();
            payload["world_id"] = uuid_text(self.current.world_id.as_bytes()).into();
            payload["channel_id"] = uuid_text(self.current.channel_id.as_bytes()).into();
            payload["ruleset_revision"] = self.current.ruleset_revision.clone().into();
            payload["world_policy_revision"] = self.current.world_policy_revision.clone().into();
            let token = signed_token(&self.key, header, payload.to_string());
            let facts = checked(verify_fresh_grant_durability_v1(
                &token,
                self.now,
                &FreshDurabilityTrustContext::from_owning_source(self),
                &FreshDurabilityCurrentAuthorityV1::from_owning_source(self),
            ))?;
            let authorization = checked(FreshAdmissionCommitAuthorizationV1::new(
                &facts,
                checked(GameSessionId::decode(&id(self.session_seed)))?,
                checked(AuthenticatedTransportRefV1::decode(&id(self.session_seed)))?,
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
            r#"{{"iss":"urn:oteryn:platform:game-admission","aud":"urn:oteryn:game:admission","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-pre-admission-v1","purpose":"fresh_entry","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"00000000-0000-4000-8000-000000000001","character_id":"00000000-0000-7000-8000-000000000002","world_id":"00000000-0000-7000-8000-000000000003","channel_id":"00000000-0000-7000-8000-000000000004","account_security_generation":"1","route_revision":"route-1","runtime_observation_revision":"runtime-1","scope_ownership_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1","offer_revision":"offer-1"}}"#
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

async fn bootstrap_party_character(
    h: &Harness,
    character: u8,
    account: u8,
    now: i64,
) -> TestResult<crate::domain::CharacterId> {
    // The independent producer fixture supplies a bounded authenticated decision. Only the
    // real bootstrap writer creates the root, issuer floor, operation receipt and audit data.
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    };
    if character == 60 {
        let descriptor = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: now,
        };
        let provenance = FreshStoreProvenance {
            namespace: "party-fixture-source".into(),
            authorization: "independent-platform-fixture".into(),
            source_authority: "platform".into(),
            initialized_at: now,
        };
        assert!(
            h.root
                .record_native_source_descriptor_issuance(
                    "platform",
                    descriptor.clone(),
                    Some(provenance.clone())
                )
                .await
                .map_err(debug)?
        );
        h.root
            .initialize_native_admission_source(&h.node, provenance, descriptor)
            .await
            .map_err(debug)?;
    }
    let account_text = fresh_source::uuid_text(&id(account));
    let revision = u64::from(character);
    let security = serde_json::json!({
        "version": 1, "operation": "ReadAccountSecurityV1", "result": "observed",
        "source_authority": "platform", "source_revision": revision.to_string(),
        "decision_identity": revision.to_string(), "source_observed_at": now.to_string(),
        "clock_uncertainty_seconds": "0", "account_id": account_text,
        "purpose": "platform_security", "scope": "fresh_admission", "allowed": true,
        "minimum_valid_generation": "1",
    });
    h.root
        .accept_native_source_observation(
            &h.node,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(account_text.clone())
                    .map_err(debug)?,
                source_revision: revision,
                decision_identity: revision.to_string(),
                observed_at: now,
                semantic_facts: serde_json::to_vec(&security)?,
            },
        )
        .await
        .map_err(debug)?;
    let wire = serde_json::json!({
        "contract_version": 3, "variant": "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority": "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id": fresh_source::uuid_text(&id(character + 20)),
        "source_revision": revision.to_string(),
        "operation_id": fresh_source::uuid_text(&id(character)),
        "operation": "INITIAL_CHARACTER_BOOTSTRAP", "account_id": account_text,
        "target_world_id": fresh_source::uuid_text(&id(WORLD)),
        "requested_sex":"Male","requested_name": format!("Party Player {}{}", char::from(b'A' + character / 26), char::from(b'a' + character % 26)),
        "interpretation_context": {"profile_revision":"profile-1", "ruleset_revision":"ruleset-1",
            "content_revision":"content-1", "starter_template_revision":"starter-1"},
        "issued_at_source": (now - 1).to_string(), "expires_at_source": (now + 120).to_string(),
        "audience":"OTERYN_GAME_CHARACTER_AUTHORITY",
    });
    let intent = crate::character_bootstrap_intent::decode_producer_response(
        &serde_json::to_vec(&wire)?,
        id(character),
    )
    .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let bootstrapped = h
        .root
        .bootstrap_character(&recovery, &h.node, &intent)
        .await
        .map_err(debug)?;
    assert_eq!(bootstrapped.revision.get(), 1);
    Ok(bootstrapped.character_id)
}

async fn committed_fresh_player(
    h: &Harness,
    character: u8,
    account: u8,
    session: u8,
    now: i64,
) -> TestResult<
    crate::foundation::GameSessionAuthoritySnapshot<crate::foundation::AuthenticatedTransportRefV1>,
> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    let character_id = bootstrap_party_character(h, character, account, now).await?;
    let mut source = fresh_source::Source::new(now, *character_id.as_bytes(), account, session)?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    // Keep independently committed runtime/signing publications exactly as actually read.
    // The fixture does not overwrite the actual assignment owner or invent its high-water.
    let rows = guards
        .load(&[source.rows[2].key.clone(), source.rows[3].key.clone()])
        .await
        .map_err(debug)?;
    for (index, row) in rows.into_iter().enumerate() {
        if let Some(row) = row {
            source.rows[index + 2] = row;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    let request = source.request()?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let outcome = store.commit(&request).await.map_err(debug)?;
    let FreshAdmissionDurableOutcomeV1::Committed(receipt) = outcome else {
        return Err(format!("genuine Fresh admission not committed:{outcome:?}").into());
    };
    assert_eq!(receipt.operation(), request.operation());
    assert!(receipt.decided_at() >= now);
    let current = store
        .current_session(GameSessionId::decode(&id(session)).map_err(debug)?)
        .await
        .map_err(debug)?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    assert_eq!(current.current_connection_generation().get(), 1);
    assert_eq!(current.current_character_lease().generation(), 2);
    Ok(current)
}

type CurrentSession =
    crate::foundation::GameSessionAuthoritySnapshot<crate::foundation::AuthenticatedTransportRefV1>;

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness, CurrentSession, CurrentSession) -> TestResult,
{
    // Repository convention: an unrouted run reports PRE-ROUTING / NONCANONICAL and returns.
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = async {
            let now: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&h.pool)
                    .await?;
            let leader = committed_fresh_player(&h, 60, 62, 70, now).await?;
            let member = committed_fresh_player(&h, 61, 63, 71, now).await?;
            body(&h, leader, member).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}

fn current_fence(current: &CurrentSession) -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(
            *current.current_character_lease().character_id().as_bytes(),
        )
        .map_err(debug)?,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
    })
}

async fn lane_permit() -> TestResult<crate::durability::spell_owner_commit::SpellLanePermit> {
    crate::durability::spell_owner_commit::SpellLane::new(
        crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
    )
    .acquire()
    .await
    .map_err(|_| "spell lane unresolved".into())
}

async fn authority(
    tx: &mut Transaction<'_, Postgres>,
    h: &Harness,
    current: &CurrentSession,
    number: u64,
) -> TestResult<SpellItemAuthority> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert_spell_item_authority_in_transaction(
        tx,
        &lane_permit().await?,
        &h.root,
        &recovery,
        &h.node,
        &current_fence(current)?,
        CommandRef::new(
            current.current_game_session_id(),
            CommandId::new(number).map_err(debug)?,
        ),
        [1; 32],
    )
    .await
    .map_err(|e| debug(e).into())
}

// Membership rows are explicitly administrative constraint fixtures; no
// actor/current session/lease/Node or SpellItemAuthority is fabricated here.
// The inherited13 native cases separately qualify Invite/Accept/consent.
async fn membership(h: &Harness, leader: &CurrentSession, member: &CurrentSession) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,3)")
        .bind(id(90).as_slice()).bind(id(WORLD).as_slice())
        .bind(leader.current_character_lease().character_id().as_bytes().as_slice())
        .execute(&mut *tx).await?;
    for (seq, current) in [(1_i64, leader), (2, member)] {
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,encode($4,'hex')::uuid)")
            .bind(current.current_character_lease().character_id().as_bytes().as_slice())
            .bind(id(90).as_slice()).bind(seq).bind(id(CHANNEL).as_slice())
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn apply(
    h: &Harness,
    current: &CurrentSession,
    number: u64,
    action: PartyAction,
) -> TestResult<PreparedWorldPartyReceipt> {
    let mut tx = h.pool.begin().await?;
    let proof = authority(&mut tx, h, current, number).await?;
    let receipt = apply_world_party_command_in_transaction(&mut tx, &proof, action, None)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(receipt)
}

struct RuntimePlayers {
    runtime: crate::foundation::ChannelRuntimeV1,
    leader: crate::foundation::ExactActorRef,
    member: crate::foundation::ExactActorRef,
}

fn runtime_players(
    h: &Harness,
    leader: &CurrentSession,
    member: &CurrentSession,
) -> TestResult<RuntimePlayers> {
    use {
        crate::foundation::{ChannelId, ChannelRuntimeV1, WorldId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .map_err(debug)?;
    let mut actors = Vec::new();
    for current in [leader, member] {
        let reservation = runtime
            .reserve_fresh_session(current.current_game_session_id())
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        actors.push(actor);
    }
    Ok(RuntimePlayers {
        runtime,
        leader: actors[0],
        member: actors[1],
    })
}

async fn visible_action(
    h: &Harness,
    leader: &CurrentSession,
    member: &CurrentSession,
    players: &RuntimePlayers,
    command: u64,
    action: PartyAction,
) -> TestResult<PreparedWorldPartyReceipt> {
    use crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction;
    let mut tx = h.pool.begin().await?;
    let authority = authority(&mut tx, h, member, command).await?;
    let visible = prove_visible_party_target_in_transaction(
        &mut tx,
        &h.root,
        &authority,
        &players.runtime,
        players.member,
        players.leader,
        leader.current_game_session_id(),
    )
    .await
    .map_err(debug)?
    .ok_or("actual visible leader unavailable")?;
    let receipt =
        apply_world_party_command_in_transaction(&mut tx, &authority, action, Some(&visible))
            .await
            .map_err(debug)?;
    tx.commit().await?;
    Ok(receipt)
}

async fn block_visible(
    h: &Harness,
    leader: &CurrentSession,
    member: &CurrentSession,
    players: &RuntimePlayers,
    command: u64,
) -> TestResult<PreparedWorldPartyReceipt> {
    visible_action(
        h,
        leader,
        member,
        players,
        command,
        PartyAction::Block {
            target: *leader.current_character_lease().character_id().as_bytes(),
        },
    )
    .await
}

#[test]
fn party_missing_settings_defaults_allow_invite_accept_and_current_channel_entry() -> TestResult {
    run(
        "party_missing_setting_defaults",
        async |h, leader, member| {
            let players = runtime_players(h, &leader, &member)?;
            let settings: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_social_settings WHERE character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(leader.current_character_lease().character_id().as_bytes().as_slice())
            .bind(member.current_character_lease().character_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(settings, 0);
            let invitation = visible_action(
                h,
                &leader,
                &member,
                &players,
                1,
                PartyAction::Invite {
                    target: *leader.current_character_lease().character_id().as_bytes(),
                },
            )
            .await?;
            assert_eq!(invitation.disposition, "APPLIED");
            let accepted = apply(
                h,
                &leader,
                1,
                PartyAction::Accept {
                    party: invitation.party_id.ok_or("created party missing")?,
                },
            )
            .await?;
            assert_eq!(accepted.disposition, "APPLIED");
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let snapshot = h
                .root
                .refresh_current_world_party_presence(
                    &lane_permit().await?,
                    &recovery,
                    &h.node,
                    &players.runtime,
                    players.leader,
                    leader.current_game_session_id(),
                )
                .await
                .map_err(debug)?;
            assert_eq!(snapshot.members.len(), 2);
            assert!(snapshot.members.iter().all(|member| member.presence
                == crate::foundation::PartyPresence::Channel
                && member.channel == Some(id(CHANNEL))
                && member.presence_revision == 1));
            Ok(())
        },
    )
}

#[test]
fn party_hidden_before_create_and_accept_never_produces_a_channel_record() -> TestResult {
    run("party_hidden_join", async |h, leader, member| {
        let players = runtime_players(h, &leader, &member)?;
        apply(
            h,
            &member,
            1,
            PartyAction::SetChannelVisibility { hidden: true },
        )
        .await?;
        apply(h, &leader, 1, PartyAction::SetPartyInvites { anyone: true }).await?;
        let invitation = visible_action(
            h,
            &leader,
            &member,
            &players,
            2,
            PartyAction::Invite {
                target: *leader.current_character_lease().character_id().as_bytes(),
            },
        )
        .await?;
        assert_eq!(invitation.disposition, "APPLIED");
        apply(
            h,
            &leader,
            2,
            PartyAction::SetChannelVisibility { hidden: true },
        )
        .await?;
        let party = invitation.party_id.ok_or("created party missing")?;
        let accepted = apply(h, &leader, 3, PartyAction::Accept { party }).await?;
        assert_eq!(accepted.disposition, "APPLIED");
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let snapshots = h
            .root
            .read_current_world_party_views(
                &lane_permit().await?,
                &recovery,
                &h.node,
                &players.runtime,
            )
            .await
            .map_err(debug)?;
        assert_eq!(snapshots.len(), 2);
        assert!(snapshots.iter().all(|snapshot| snapshot.members.len() == 2
            && snapshot.members.iter().all(|member| member.presence
                == crate::foundation::PartyPresence::Hidden
                && member.channel.is_none()
                && member.presence_revision == 1)));
        Ok(())
    })
}

#[test]
fn party_scope_census_reads_committed_solo_membership_toggle_and_presence() -> TestResult {
    run("party_scope_census", async |h, leader, member| {
        let players = runtime_players(h, &leader, &member)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let permit = lane_permit().await?;
        let solo = h
            .root
            .read_current_world_party_views(&permit, &recovery, &h.node, &players.runtime)
            .await
            .map_err(debug)?;
        assert_eq!(solo.len(), 2);
        assert!(
            solo.iter()
                .all(|snapshot| snapshot.party_id.is_none() && snapshot.members.is_empty())
        );
        membership(h, &leader, &member).await?;
        apply(
            h,
            &leader,
            1,
            PartyAction::SetSharedExperience { enabled: true },
        )
        .await?;
        apply(
            h,
            &member,
            1,
            PartyAction::SetChannelVisibility { hidden: true },
        )
        .await?;
        let committed = h
            .root
            .read_current_world_party_views(&permit, &recovery, &h.node, &players.runtime)
            .await
            .map_err(debug)?;
        assert_eq!(committed.len(), 2);
        for snapshot in committed {
            assert_eq!(snapshot.party_id, Some(id(90)));
            assert_eq!(snapshot.revision, Some(3));
            assert!(snapshot.shared_xp_enabled);
            assert_eq!(snapshot.members.len(), 2);
            let hidden = snapshot
                .members
                .iter()
                .find(|entry| {
                    entry.character == *member.current_character_lease().character_id().as_bytes()
                })
                .ok_or("member missing")?;
            assert_eq!(hidden.presence, crate::foundation::PartyPresence::Hidden);
            assert_eq!(hidden.presence_revision, 2);
            assert_eq!(hidden.channel, None);
        }
        Ok(())
    })
}

#[test]
fn party_scope_census_refuses_missing_current_actor_claim_without_partial_data() -> TestResult {
    run("party_scope_current_claim", async |h, leader, member| {
        let players = runtime_players(h, &leader, &member)?;
        membership(h, &leader, &member).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let permit = lane_permit().await?;
        assert_eq!(
            h.root
                .read_current_world_party_views(&permit, &recovery, &h.node, &players.runtime)
                .await
                .map_err(debug)?
                .len(),
            2
        );
        // Change exactly one independently resolved current fact. Retained session,
        // immutable Fresh receipt, runtime actor and Node/scope remain valid.
        sqlx::query("UPDATE game_durability_admission_character_guards SET eligible=false WHERE character_id=encode($1,'hex')::uuid")
            .bind(member.current_character_lease().character_id().as_bytes().as_slice()).execute(&h.pool).await?;
        assert!(
            h.root
                .read_current_world_party_views(&permit, &recovery, &h.node, &players.runtime)
                .await
                .is_err()
        );
        Ok(())
    })
}

#[test]
fn party_scope_census_rejects_foreign_physical_transaction_and_revoked_node() -> TestResult {
    run("party_scope_fences", async |h, leader, member| {
        use crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction;
        let players = runtime_players(h, &leader, &member)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let permit = lane_permit().await?;
        let mut prepared = h.pool.begin().await?;
        let scope = assert_spell_item_scope_in_transaction(
            &mut prepared,
            &permit,
            &h.root,
            &recovery,
            &h.node,
            leader.current_runtime_scope(),
            1,
        )
        .await
        .map_err(debug)?;
        let mut foreign = h.pool.begin().await?;
        assert!(
            crate::durability::world_party::read_world_party_views_for_scope_in_transaction(
                &mut foreign,
                &h.root,
                &scope,
                &players.runtime
            )
            .await
            .is_err()
        );
        foreign.rollback().await?;
        prepared.rollback().await?;
        h.root
            .revoke_node_registration(h.node.fact())
            .await
            .map_err(debug)?;
        assert!(
            h.root
                .read_current_world_party_views(&permit, &recovery, &h.node, &players.runtime)
                .await
                .is_err()
        );
        Ok(())
    })
}

#[test]
fn party_visibility_and_invite_consent_are_current_atomic_idempotent_operations() -> TestResult {
    run("party_consent_operations", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        for (command, hidden, revision, presence_revision) in [
            (1, true, 2, 2),
            (2, true, 2, 2),
            (3, false, 3, 3),
            (4, false, 3, 3),
        ] {
            let receipt = apply(
                h,
                &member,
                command,
                PartyAction::SetChannelVisibility { hidden },
            )
            .await?;
            assert_eq!(receipt.revision, Some(revision));
            let row: (i16, Option<Vec<u8>>, String) = sqlx::query_as("SELECT presence_state,uuid_send(presence_channel_id),presence_revision::text FROM game_party_members WHERE character_id=encode($1,'hex')::uuid")
                .bind(member.current_character_lease().character_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                row,
                (
                    if hidden { 2 } else { 1 },
                    if hidden {
                        None
                    } else {
                        Some(id(CHANNEL).to_vec())
                    },
                    presence_revision.to_string()
                )
            );
            let replay = apply(
                h,
                &member,
                command,
                PartyAction::SetChannelVisibility { hidden },
            )
            .await?;
            assert!(replay.replayed);
            assert_eq!(replay.receipt_id, receipt.receipt_id);
        }
        for (command, anyone, setting) in [(5, false, 2_i16), (6, true, 1_i16)] {
            let receipt =
                apply(h, &member, command, PartyAction::SetPartyInvites { anyone }).await?;
            assert_eq!(receipt.revision, Some(3));
            let actual: i16 = sqlx::query_scalar("SELECT party_invites FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid")
                .bind(member.current_character_lease().character_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(actual, setting);
        }
        let mut tx = h.pool.begin().await?;
        let proof = authority(&mut tx, h, &member, 7).await?;
        apply_world_party_command_in_transaction(
            &mut tx,
            &proof,
            PartyAction::SetChannelVisibility { hidden: true },
            None,
        )
        .await
        .map_err(debug)?;
        tx.rollback().await?;
        let actual: i16 = sqlx::query_scalar("SELECT channel_visibility FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid")
            .bind(member.current_character_lease().character_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(actual, 1);
        assert_eq!(state(h).await?.1, "3");
        Ok(())
    })
}

#[test]
fn party_visible_block_cancels_only_the_blocked_leaders_invitation_and_unblock_is_owned()
-> TestResult {
    run("party_block_operations", async |h, leader, member| {
        let players = runtime_players(h, &leader, &member)?;
        let target = *leader.current_character_lease().character_id().as_bytes();
        let owner = *member.current_character_lease().character_id().as_bytes();
        assert_eq!(
            apply(h, &member, 1, PartyAction::Block { target })
                .await?
                .disposition,
            "NOT_VISIBLE"
        );
        assert_eq!(h.count("game_character_social_blocks").await?, 0);
        // Constraint fixture: a leader-only party whose sole invitation is to the blocker.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,3)")
            .bind(id(90).as_slice()).bind(id(WORLD).as_slice()).bind(target.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,encode($3,'hex')::uuid)")
            .bind(target.as_slice()).bind(id(90).as_slice()).bind(id(CHANNEL).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,created_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp(),clock_timestamp()+interval '5 minutes')")
            .bind(id(90).as_slice()).bind(owner.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let blocked = block_visible(h, &leader, &member, &players, 2).await?;
        assert_eq!(blocked.disposition, "APPLIED");
        assert_eq!(h.count("game_character_social_blocks").await?, 1);
        assert_eq!(h.count("game_party_invitations").await?, 0);
        assert_eq!(h.count("game_parties").await?, 0);
        // Another Character's unblock cannot mutate the blocker's own list.
        apply(h, &leader, 1, PartyAction::Unblock { target: owner }).await?;
        assert_eq!(h.count("game_character_social_blocks").await?, 1);
        let replay = block_visible(h, &leader, &member, &players, 2).await?;
        assert!(replay.replayed);
        assert_eq!(replay.receipt_id, blocked.receipt_id);
        apply(h, &member, 3, PartyAction::Unblock { target }).await?;
        assert_eq!(h.count("game_character_social_blocks").await?, 0);
        Ok(())
    })
}

#[test]
fn party_block_limit_refuses_the_101st_and_allows_an_existing_block_at_100() -> TestResult {
    run("party_block_bound", async |h, leader, member| {
        let players = runtime_players(h, &leader, &member)?;
        let owner = *member.current_character_lease().character_id().as_bytes();
        let target = *leader.current_character_lease().character_id().as_bytes();
        // Historical block targets are made by the real creator, retaining the
        // creation receipts/history that native recovery independently validates.
        let mut historical = Vec::new();
        for number in 100..200 {
            // Each account-security observation is fresh for only a few seconds, so
            // every bootstrap reads Game time again instead of reusing the first one.
            let now: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&h.pool)
                    .await?;
            let character = bootstrap_party_character(h, number, number, now).await?;
            sqlx::query("INSERT INTO game_character_social_blocks(character_id,blocked_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(owner.as_slice()).bind(character.as_bytes().as_slice()).execute(&h.pool).await?;
            historical.push(*character.as_bytes());
        }
        assert_eq!(
            block_visible(h, &leader, &member, &players, 1)
                .await?
                .disposition,
            "RESOURCE_LIMIT"
        );
        assert_eq!(h.count("game_character_social_blocks").await?, 100);
        apply(
            h,
            &member,
            2,
            PartyAction::Unblock {
                target: historical[0],
            },
        )
        .await?;
        assert_eq!(
            block_visible(h, &leader, &member, &players, 3)
                .await?
                .disposition,
            "APPLIED"
        );
        assert_eq!(
            block_visible(h, &leader, &member, &players, 4)
                .await?
                .disposition,
            "APPLIED"
        );
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid AND blocked_character_id=encode($2,'hex')::uuid)")
            .bind(owner.as_slice()).bind(target.as_slice()).fetch_one(&h.pool).await?;
        assert!(exists);
        assert_eq!(h.count("game_character_social_blocks").await?, 100);
        Ok(())
    })
}

async fn state(h: &Harness) -> TestResult<(bool, String, i64, i64)> {
    Ok(sqlx::query_as("SELECT shared_xp_enabled,revision::text,(SELECT count(*) FROM game_world_party_receipts),(SELECT count(*) FROM game_world_party_audit_outbox) FROM game_parties WHERE party_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn shared_xp_leader_enable_disable_is_durable_same_transaction_data() -> TestResult {
    run("party_shared_xp_toggle", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        assert_eq!(state(h).await?, (false, "1".into(), 0, 0));
        for (command, enabled, revision) in [(1, true, 2), (2, false, 3)] {
            let mut tx = h.pool.begin().await?;
            let proof = authority(&mut tx, h, &leader, command).await?;
            let receipt = apply_world_party_command_in_transaction(
                &mut tx,
                &proof,
                PartyAction::SetSharedExperience { enabled },
                None,
            )
            .await
            .map_err(debug)?;
            assert_eq!(receipt.disposition, "APPLIED");
            assert_eq!(receipt.revision, Some(revision));
            let snapshot = read_world_party_in_transaction(&mut tx, &proof)
                .await
                .map_err(debug)?;
            assert_eq!(snapshot.shared_xp_enabled, enabled);
            assert_eq!(snapshot.party_id, Some(id(90)));
            assert_eq!(
                snapshot.leader,
                Some(*leader.current_character_lease().character_id().as_bytes())
            );
            assert_eq!(snapshot.members.len(), 2);
            let same_xid: bool = sqlx::query_scalar("SELECT created_xact_id=pg_current_xact_id() FROM game_world_party_receipts WHERE receipt_id=encode($1,'hex')::uuid")
                .bind(receipt.receipt_id.as_slice()).fetch_one(&mut *tx).await?;
            assert!(same_xid);
            let audit_matches: bool = sqlx::query_scalar("SELECT r.intent=a.envelope AND r.binding=sha256(r.intent) FROM game_world_party_receipts r JOIN game_world_party_audit_outbox a USING(receipt_id) WHERE r.receipt_id=encode($1,'hex')::uuid")
                .bind(receipt.receipt_id.as_slice()).fetch_one(&mut *tx).await?;
            assert!(audit_matches);
            tx.commit().await?;
            assert_eq!(
                state(h).await?,
                (
                    enabled,
                    revision.to_string(),
                    i64::try_from(command)?,
                    i64::try_from(command)?
                )
            );
        }
        Ok(())
    })
}

#[test]
fn shared_xp_noop_preserves_revision_but_records_each_occurrence() -> TestResult {
    run("party_shared_xp_noop", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        for (command, enabled, revision) in [
            (1, false, 1),
            (2, true, 2),
            (3, true, 2),
            (4, false, 3),
            (5, false, 3),
        ] {
            let receipt = apply(
                h,
                &leader,
                command,
                PartyAction::SetSharedExperience { enabled },
            )
            .await?;
            assert_eq!(receipt.disposition, "APPLIED");
            assert_eq!(receipt.revision, Some(revision));
            assert!(!receipt.replayed);
            assert_eq!(
                state(h).await?,
                (
                    enabled,
                    revision.to_string(),
                    command as i64,
                    command as i64
                )
            );
            let replay = apply(
                h,
                &leader,
                command,
                PartyAction::SetSharedExperience { enabled },
            )
            .await?;
            assert!(replay.replayed);
            assert_eq!(replay.receipt_id, receipt.receipt_id);
            assert_eq!(
                state(h).await?,
                (
                    enabled,
                    revision.to_string(),
                    command as i64,
                    command as i64
                )
            );
        }
        Ok(())
    })
}

#[test]
fn shared_xp_member_and_solo_never_gain_leader_permission() -> TestResult {
    run("party_shared_xp_role", async |h, leader, member| {
        let solo = apply(
            h,
            &leader,
            1,
            PartyAction::SetSharedExperience { enabled: true },
        )
        .await?;
        assert_eq!(solo.disposition, "NOT_LEADER");
        assert_eq!(solo.party_id, None);
        let mut read = h.pool.begin().await?;
        let proof = authority(&mut read, h, &leader, 2).await?;
        assert!(
            !read_world_party_in_transaction(&mut read, &proof)
                .await
                .map_err(debug)?
                .shared_xp_enabled
        );
        read.rollback().await?;
        membership(h, &leader, &member).await?;
        let denied = apply(
            h,
            &member,
            1,
            PartyAction::SetSharedExperience { enabled: true },
        )
        .await?;
        assert_eq!(denied.disposition, "NOT_LEADER");
        assert_eq!(state(h).await?, (false, "1".into(), 2, 2));
        Ok(())
    })
}

#[test]
fn shared_xp_occurrence_replays_history_without_reverting_later_flag() -> TestResult {
    run("party_shared_xp_replay", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        let first = apply(
            h,
            &leader,
            1,
            PartyAction::SetSharedExperience { enabled: true },
        )
        .await?;
        apply(
            h,
            &leader,
            2,
            PartyAction::SetSharedExperience { enabled: false },
        )
        .await?;
        let before = state(h).await?;
        let replay = apply(
            h,
            &leader,
            1,
            PartyAction::SetSharedExperience { enabled: true },
        )
        .await?;
        assert!(replay.replayed);
        assert_eq!(replay.receipt_id, first.receipt_id);
        assert_eq!(replay.revision, first.revision);
        assert_eq!(state(h).await?, before);
        let mut tx = h.pool.begin().await?;
        let proof = authority(&mut tx, h, &leader, 1).await?;
        assert!(matches!(
            apply_world_party_command_in_transaction(
                &mut tx,
                &proof,
                PartyAction::SetSharedExperience { enabled: false },
                None,
            )
            .await,
            Err(SpellItemError::Rejected("party command conflict"))
        ));
        tx.rollback().await?;
        assert_eq!(state(h).await?, before);
        Ok(())
    })
}

#[test]
fn shared_xp_rollback_removes_flag_revision_receipt_and_audit_together() -> TestResult {
    run("party_shared_xp_rollback", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        let before = state(h).await?;
        let mut tx = h.pool.begin().await?;
        let proof = authority(&mut tx, h, &leader, 1).await?;
        apply_world_party_command_in_transaction(
            &mut tx,
            &proof,
            PartyAction::SetSharedExperience { enabled: true },
            None,
        )
        .await
        .map_err(debug)?;
        tx.rollback().await?;
        assert_eq!(state(h).await?, before);
        let retried = apply(
            h,
            &leader,
            1,
            PartyAction::SetSharedExperience { enabled: true },
        )
        .await?;
        assert!(!retried.replayed);
        assert_eq!(state(h).await?, (true, "2".into(), 1, 1));
        Ok(())
    })
}

#[test]
fn shared_xp_consumes_current_leadership_after_real_transfer() -> TestResult {
    run(
        "party_shared_xp_current_leader",
        async |h, leader, member| {
            membership(h, &leader, &member).await?;
            let mut read = h.pool.begin().await?;
            let proof = authority(&mut read, h, &leader, 1).await?;
            let old = read_world_party_in_transaction(&mut read, &proof)
                .await
                .map_err(debug)?;
            read.commit().await?;
            apply(
                h,
                &leader,
                2,
                PartyAction::TransferLeader {
                    target: *member.current_character_lease().character_id().as_bytes(),
                },
            )
            .await?;
            let stale = apply(
                h,
                &leader,
                3,
                PartyAction::SetSharedExperience { enabled: true },
            )
            .await?;
            assert_eq!(stale.disposition, "NOT_LEADER");
            assert_eq!(
                old.leader,
                Some(*leader.current_character_lease().character_id().as_bytes())
            );
            let accepted = apply(
                h,
                &member,
                1,
                PartyAction::SetSharedExperience { enabled: true },
            )
            .await?;
            assert_eq!(accepted.disposition, "APPLIED");
            assert_eq!(state(h).await?, (true, "3".into(), 3, 3));
            Ok(())
        },
    )
}

#[test]
fn shared_xp_proof_from_another_physical_transaction_refuses() -> TestResult {
    run("party_shared_xp_physical_tx", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        let before = state(h).await?;
        let mut first = h.pool.begin().await?;
        let proof = authority(&mut first, h, &leader, 1).await?;
        first.rollback().await?;
        let mut second = h.pool.begin().await?;
        assert!(matches!(
            apply_world_party_command_in_transaction(
                &mut second,
                &proof,
                PartyAction::SetSharedExperience { enabled: true },
                None,
            )
            .await,
            Err(SpellItemError::Rejected(
                "authority belongs to another physical transaction"
            ))
        ));
        second.rollback().await?;
        assert_eq!(state(h).await?, before);
        Ok(())
    })
}

#[test]
fn shared_xp_stale_expected_session_lease_and_scope_each_refuse() -> TestResult {
    run(
        "party_shared_xp_expected_fences",
        async |h, leader, member| {
            membership(h, &leader, &member).await?;
            let before = state(h).await?;
            for kind in 0..3 {
                let mut expected = current_fence(&leader)?;
                match kind {
                    0 => {
                        expected.connection_generation =
                            crate::foundation::ConnectionGeneration::new(
                                expected.connection_generation.get() + 1,
                            )
                            .map_err(debug)?
                    }
                    1 => expected.character_lease_generation += 1,
                    _ => {
                        expected.scope_ownership_generation =
                            crate::foundation::ScopeOwnershipGeneration::new(
                                expected.scope_ownership_generation.get() + 1,
                            )
                            .map_err(debug)?
                    }
                }
                let seal = h.recovery.seal_current().map_err(debug)?;
                let recovery = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let mut tx = h.pool.begin().await?;
                assert!(
                    assert_spell_item_authority_in_transaction(
                        &mut tx,
                        &lane_permit().await?,
                        &h.root,
                        &recovery,
                        &h.node,
                        &expected,
                        CommandRef::new(
                            leader.current_game_session_id(),
                            CommandId::new(1).map_err(debug)?
                        ),
                        [1; 32],
                    )
                    .await
                    .is_err()
                );
                tx.rollback().await?;
                assert_eq!(state(h).await?, before);
            }
            Ok(())
        },
    )
}

#[test]
fn shared_xp_revoked_actual_node_source_refuses_with_expected_facts_unchanged() -> TestResult {
    run("party_shared_xp_revoked_node", async |h, leader, member| {
        membership(h, &leader, &member).await?;
        let before = state(h).await?;
        h.root
            .revoke_node_registration(h.node.fact())
            .await
            .map_err(debug)?;
        let mut tx = h.pool.begin().await?;
        // Expected session/lease/scope remain exact; independently current
        // Node registration alone was revoked through the real Root owner.
        assert!(authority(&mut tx, h, &leader, 1).await.is_err());
        tx.rollback().await?;
        assert_eq!(state(h).await?, before);
        Ok(())
    })
}
#[test]
fn shared_xp_revoked_actual_scope_refuses_with_current_node_unchanged() -> TestResult {
    run(
        "party_shared_xp_revoked_scope",
        async |h, leader, member| {
            use crate::durability::runtime_scope_assignment::{
                AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
            };
            membership(h, &leader, &member).await?;
            let before = state(h).await?;
            let scope = leader.current_runtime_scope();
            let predecessor = h
                .root
                .read_runtime_scope_predecessor(scope)
                .await
                .map_err(debug)?
                .ok_or("actual current scope predecessor missing")?;
            let outcome = h
                .writer
                .submit(&AssignmentRequest {
                    operation_key: OperationKey::from_bytes([88; 32]),
                    actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                    command: AssignmentCommand::Revoke { scope, predecessor },
                })
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, AssignmentOutcome::Committed(_)));
            h.root
                .require_current_node_registration(&h.node)
                .await
                .map_err(debug)?;
            let mut tx = h.pool.begin().await?;
            assert!(authority(&mut tx, h, &leader, 1).await.is_err());
            tx.rollback().await?;
            assert_eq!(state(h).await?, before);
            Ok(())
        },
    )
}
