#![cfg(any())] // CP slot: enable the ordinary native Root test target.
//! Actual native crate-root module context for the current World party owner.
//! Each declaration below loads its original native physical source. No authority facade,
//! helper visibility widening, duplicated Premium owner, or production registration is added.
//! Qualification records the exact root-declaration/path provenance separately.
extern crate self as oteryn_game_server;

#[cfg(test)]
#[path = "../src/monster_lab.rs"]
mod monster_lab;

#[allow(dead_code)]
#[allow(clippy::duplicate_mod)] // Foundation's standalone tests include Ability source.
#[path = "../src/ability/mod.rs"]
mod ability;
#[path = "../src/achievement_catalogue.rs"]
mod achievement_catalogue;
#[allow(
    dead_code,
    unused_imports,
    reason = "ai:: has no production caller yet; a later wiring task consumes it from ai_think"
)]
#[path = "../src/ai/mod.rs"]
mod ai;
#[path = "../src/ai_monster_melee.rs"]
mod ai_monster_melee;
// Ordinary declarations retain the native non-mod.rs implicit child directories. A direct
// #[path="../src/ai_think.rs"] would instead search src/ for its implicit children.
#[path = "../src"]
mod native_file_modules {
    #[allow(
        dead_code,
        reason = "ai_think has no production caller yet; AI-4/a later wiring task composes it into ChannelRuntimeV1's owner cycle"
    )]
    pub(crate) mod ai_think;
    #[allow(
        dead_code,
        reason = "unactivated local Movement proof awaits the separate production composition lease"
    )]
    pub(crate) mod movement;
}
use native_file_modules::{ai_think, movement};
#[allow(
    dead_code,
    reason = "nonshipping fixed-one-creature Combat proof awaits production composition"
)]
#[path = "../src/combat.rs"]
mod combat;
#[path = "../src/combat/attack/mod.rs"]
mod combat_attack;
#[allow(
    dead_code,
    reason = "B3-2 has no production caller yet; a later admission stage wires one"
)]
#[path = "../src/combat/pickup.rs"]
mod combat_pickup;
#[allow(
    dead_code,
    reason = "PvP stays off until the PVP-0 go-live gate; PVP-RT-1b wires the legality stage"
)]
#[path = "../src/combat/pvp/mod.rs"]
mod combat_pvp;
#[path = "../src/interaction/chest_use.rs"]
mod interaction_chest_use;
#[allow(dead_code)] // Preserve the original Ability module lint scope for this native-only adapter.
#[path = "../src/ability/player_lethal.rs"]
mod player_lethal;

#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(
    dead_code,
    unused_imports,
    reason = "CHAT-1a local chat rules await the CHAT_V1 wire and runtime of CHAT-1b"
)]
#[path = "../src/chat/mod.rs"]
mod chat;
#[path = "../src/content/mod.rs"]
pub mod content;
#[path = "../src/domain/mod.rs"]
pub mod domain;
#[path = "../src/durability/mod.rs"]
pub mod durability;
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[path = "../src/gameplay_transport/mod.rs"]
mod gameplay_transport;

pub use gameplay_transport::{
    FreshEvidenceSource, GameplayListenerConfig, GameplaySeamOwners, GameplayServeError,
    serve_gameplay,
};
#[path = "../src/house_scope/mod.rs"]
pub mod house_scope;
#[path = "../src/map/mod.rs"]
pub mod map;

#[path = "../src/node/mod.rs"]
pub mod node;
#[path = "../src/premium/mod.rs"]
pub mod premium;
#[allow(
    dead_code,
    reason = "player spell core (P3a) awaits the cast protocol and runtime state contracts (P3b)"
)]
#[path = "../src/spell/mod.rs"]
mod spell;
#[path = "../src/wheel_gem_data.rs"]
pub mod wheel_gem_data;

#[allow(
    dead_code,
    unused_imports,
    reason = "GAME-INTERACTION identity; the §7 revert driver is its first library consumer"
)]
#[path = "../src/interaction/mod.rs"]
mod interaction;
#[allow(
    dead_code,
    reason = "§7 timed-revert driver awaits its live Channel/Instance scope owner"
)]
#[path = "../src/world_object_revert.rs"]
mod world_object_revert;
#[allow(dead_code)]
#[path = "../src/world_runtime.rs"]
pub(crate) mod world_runtime;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "shared PostgreSQL support; each mount uses a subset"
)]
#[path = "../src/../tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[cfg(test)]
#[path = "../src/../tests/support/spell_stance_runtime_postgres_cases.rs"]
mod spell_stance_runtime_postgres_cases;

#[cfg(test)]
#[path = "../src/../tests/support/spell_familiar_runtime_postgres_cases.rs"]
mod spell_familiar_runtime_postgres_cases;

#[cfg(test)]
#[path = "../src/durability/world_party_tests.rs"]
mod world_party_runtime_postgres_cases;

#[cfg(test)]
#[path = "../src/foundation/recovery_tests.rs"]
mod foundation_recovery_tests;

#[cfg(test)]
#[path = "../src/foundation/final_review_regressions.rs"]
mod foundation_final_review_regressions;

#[cfg(test)]
#[path = "../src/foundation/final_review_round2_regressions.rs"]
mod foundation_final_review_round2_regressions;

#[cfg(test)]
#[path = "../src/foundation/final_review_round2_runtime_rollback.rs"]
mod foundation_final_review_round2_runtime_rollback;

#[path = "../src/admission_evidence.rs"]
pub mod admission_evidence;
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/bone_phase_registry.rs"]
mod bone_phase_registry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_appearance_content.rs"]
mod creature_appearance_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_area_heal.rs"]
mod creature_area_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_attack_geometry.rs"]
mod creature_attack_geometry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_auto_attack.rs"]
mod creature_auto_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_chain_attack.rs"]
mod creature_chain_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_condition_content.rs"]
mod creature_condition_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_damage_spell.rs"]
mod creature_damage_spell;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_defense_presentation.rs"]
mod creature_defense_presentation;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/crystal_callbacks.rs"]
mod crystal_callbacks;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/crystal_death_composition.rs"]
mod crystal_death_composition;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/crystal_herald_quest.rs"]
mod crystal_herald_quest;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/self_heal_qualification.rs"]
mod self_heal_qualification;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/smelly_cheese.rs"]
mod smelly_cheese;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/smelly_damage.rs"]
mod smelly_damage;
#[allow(dead_code)]
#[path = "../src/smelly_encounter_dispatch.rs"]
mod smelly_encounter_dispatch;
#[path = "../src/source_callback_intent.rs"]
mod source_callback_intent;
#[allow(dead_code)]
#[path = "../src/source_callback_spawn.rs"]
mod source_callback_spawn;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/source_critical_baseline.rs"]
mod source_critical_baseline;
#[allow(
    dead_code,
    reason = "Private native encounter owner ABI retained for source-qualified library dispatch; shipping AI loop is not activated"
)]
#[path = "../src/source_encounter_seven.rs"]
mod source_encounter_seven;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/source_threshold_heal.rs"]
mod source_threshold_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/weak_spot_speech.rs"]
mod weak_spot_speech;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/welter_consume.rs"]
mod welter_consume;
#[allow(
    dead_code,
    reason = "LOOP-CLOCK-1 registers the pure World clock; the WORLD_LIGHT_V1 offer composes it later"
)]
#[path = "../src/world_clock.rs"]
mod world_clock;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/monster_combat_lane.rs"]
mod monster_combat_lane;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/monster_owner_cycle.rs"]
mod monster_owner_cycle;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/monster_summon.rs"]
mod monster_summon;

#[cfg(test)]
#[path = "../src/canonical_monster_successor_tests.rs"]
mod canonical_monster_successor_tests;

#[allow(dead_code)]
#[path = "../src/creature_icicle.rs"]
mod creature_icicle;

#[path = "../src/durability/creature_source_items.rs"]
mod creature_source_items;

#[cfg(test)]
mod shared_xp_cases {
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
                if account_id != self.current.account_id
                    || character_id != self.current.character_id
                {
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
            ) -> Result<
                Vec<AdmissionAuthorityPublicationChangeV1>,
                AdmissionAuthorityPublicationErrorV1,
            > {
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
                            *presence =
                                Some((self.current.character_id, binding.candidate_session));
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
                payload["world_policy_revision"] =
                    self.current.world_policy_revision.clone().into();
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
            DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation,
            NativeSourceSubject, SourceObservation,
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
            "contract_version": 2, "variant": "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
            "issuer_authority": "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
            "issuer_decision_id": fresh_source::uuid_text(&id(character + 20)),
            "source_revision": revision.to_string(),
            "operation_id": fresh_source::uuid_text(&id(character)),
            "operation": "INITIAL_CHARACTER_BOOTSTRAP", "account_id": account_text,
            "target_world_id": fresh_source::uuid_text(&id(WORLD)),
            "requested_name": format!("Party Player {}{}", char::from(b'A' + character / 26), char::from(b'a' + character % 26)),
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
        crate::foundation::GameSessionAuthoritySnapshot<
            crate::foundation::AuthenticatedTransportRefV1,
        >,
    > {
        use crate::durability::admission_authority_guards::AdmissionGuardStore;
        use crate::durability::fresh_admission::FreshAdmissionStore;
        use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
        use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
        let character_id = bootstrap_party_character(h, character, account, now).await?;
        let mut source =
            fresh_source::Source::new(now, *character_id.as_bytes(), account, session)?;
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

    type CurrentSession = crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >;

    fn run<F>(tag: &'static str, body: F) -> TestResult
    where
        F: AsyncFnOnce(&Harness, CurrentSession, CurrentSession) -> TestResult,
    {
        // An unconfigured native PG target must fail, never report a skipped test as PASS.
        let admin = harness::configured_admin().ok_or("native PostgreSQL admin URL required")?;
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, tag, true).await?;
            let result = async {
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                )
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
    async fn membership(
        h: &Harness,
        leader: &CurrentSession,
        member: &CurrentSession,
    ) -> TestResult {
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
                    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor,
                    OperationKey,
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
}
