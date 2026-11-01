//! Receipt-first, session-generation fenced Atelier Character and custody writes.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::character_wheel::{WheelPremiumGate, WheelRuleset, eligibility};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterRevision;
use oteryn_gameplay_progression::atelier::{
    AtelierAction, AtelierState, FragmentKind, Gem, GemQuality,
};
use oteryn_gameplay_progression::wheel::{WheelDomain, base_vocation};
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;

#[path = "character_atelier/actions.rs"]
mod actions;
#[path = "character_atelier/custody.rs"]
pub(crate) mod custody;
#[path = "character_atelier/resources.rs"]
mod resources;
pub use resources::ResourceBalances;

/// Server-resolved content and coin change facts. Never decoded from a client request.
#[derive(Clone)]
pub struct AtelierResources<'a> {
    pub content: &'a dyn super::item_loop_move::ItemLoopContent,
    pub coins: super::item_fee_burn::FeeChangeFacts,
    pub fragment: super::item_mint::TypedDefinitionRef,
}
impl std::fmt::Debug for AtelierResources<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AtelierResources").finish_non_exhaustive()
    }
}

type Result<T> = std::result::Result<T, CharacterProgressionError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtelierMutation {
    Initialize,
    SetLock { gem: [u8; 16], locked: bool },
    Reveal { quality: GemQuality },
    SwitchDomain { gem: [u8; 16] },
    Dismantle { gem: [u8; 16] },
    GradeUp { kind: FragmentKind, mod_id: u16 },
}

#[derive(Debug, Clone)]
pub struct AtelierRequest<'a> {
    pub occurrence: [u8; 16],
    pub expected_atelier_revision: u64,
    pub mutation: AtelierMutation,
    pub resources: Option<Box<AtelierResources<'a>>>,
}

impl AtelierRequest<'_> {
    pub fn initialize(character: crate::domain::CharacterId) -> Self {
        Self {
            occurrence: initial_occurrence(character.as_bytes()),
            expected_atelier_revision: 0,
            mutation: AtelierMutation::Initialize,
            resources: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtelierOutcome {
    Committed {
        revision: CharacterRevision,
        state: AtelierState,
    },
    AlreadyCommitted {
        revision: CharacterRevision,
        state: AtelierState,
    },
    Refused,
}

impl DurabilityRoot {
    /// Load the committed vessel/grade projection under the same Character lock as writers.
    pub async fn read_character_atelier(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
    ) -> Result<Option<AtelierState>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE").bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let state=load_in_transaction(&mut tx,character).await?;
            tx.commit(deadline).await?;
            Ok(Ok(state))
        })).await?
    }

    /// The sole Atelier Character writer; runtime calls go through a revision slot.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_atelier<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: AtelierRequest<'_>,
        ruleset: std::sync::Arc<WheelRuleset>,
        gate: &G,
    ) -> Result<AtelierOutcome> {
        self.commit_character_atelier_with_admission(
            authority,
            node,
            fence,
            request,
            ruleset,
            gate,
            &|| true,
        )
        .await
    }

    /// A fresh client mutation may be refused by the Channel's existing owner.
    /// Historical receipts precede this hook; initialization never starts a UI clock.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_atelier_with_admission<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: AtelierRequest<'_>,
        ruleset: std::sync::Arc<WheelRuleset>,
        gate: &G,
        admit: &(impl Fn() -> bool + Sync),
    ) -> Result<AtelierOutcome> {
        let initialize = request.mutation == AtelierMutation::Initialize;
        if initialize {
            if request.occurrence != initial_occurrence(fence.character_id.as_bytes())
                || request.expected_atelier_revision != 0
            {
                return Err(CharacterProgressionError::InvalidInput);
            }
        } else {
            super::item_mint_audit::check_uuid_v7(&request.occurrence)
                .map_err(|_| CharacterProgressionError::InvalidInput)?;
        }
        let mut hasher = Sha256::new();
        hasher.update(b"oteryn:atelier:request:v1");
        hasher.update(fence.character_id.as_bytes());
        hasher.update(request.expected_atelier_revision.to_be_bytes());
        hasher.update(
            serde_json::to_vec(&request.mutation)
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        );
        let binding = hasher.finalize().to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = (gate, admit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| {
            let (gate, admit) = *context;
            Box::pin(async move {
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                // Same occurrence serializes even when two requests arrived at different nodes.
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-atelier:' || encode($1,'hex'), 0))")
                    .bind(request.occurrence.as_slice()).execute(&mut **tx).await?;
                if let Some(row) = sqlx::query("SELECT request_sha256,committed_character_revision::text,outcome_json FROM game_character_atelier_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).fetch_optional(&mut **tx).await?
                {
                    if row.try_get::<Vec<u8>, _>("request_sha256")? != binding {
                        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                    }
                    let revision = CharacterRevision::new(numeric_u64(&row, "committed_character_revision")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                    let state = decode_state(row.try_get("outcome_json")?)?;
                    tx.commit(deadline).await?;
                    return Ok(Ok(AtelierOutcome::AlreadyCommitted { revision, state }));
                }
                if !initialize && !admit() {
                    return Ok(Ok(AtelierOutcome::Refused));
                }
                // Premium latch precedes the Character root, matching the Wheel writer.
                let Some(account) = sqlx::query_scalar::<_, String>("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else {
                    return Ok(Err(CharacterProgressionError::AuthorityRejected));
                };
                let permitted = gate.allocation_permitted(&mut tx, uuid_text(&account)?).await?;
                let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                    Ok(root) => root,
                    Err(error) => return Ok(Err(error)),
                };
                let current_account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if account != current_account {
                    return Ok(Err(CharacterProgressionError::AuthorityRejected));
                }
                let Some(progression) = sqlx::query("SELECT character_revision::text,level,profile_revision,ruleset_revision,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else {
                    return Ok(Err(CharacterProgressionError::MissingProgressionState));
                };
                if numeric_u64(&progression, "character_revision")? != root.revision {
                    return Err(DurabilityError::InvalidStoredState);
                }
                if !state_matches_root(&progression, &root) {
                    return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                }
                let vocation: String = sqlx::query_scalar("SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.unwrap_or_default();
                let level = u32::try_from(progression.try_get::<i64,_>("level")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                let promoted = super::character_promotion::read_durable_promotion(&mut tx, fence.character_id).await?;
                if eligibility(&ruleset, &vocation, level, promoted, permitted).is_err() {
                    return Ok(Ok(AtelierOutcome::Refused));
                }
                let stored: Option<serde_json::Value> = sqlx::query_scalar("SELECT state_json FROM game_character_atelier_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                let mut state = match (initialize, stored) {
                    (true, None) => AtelierState { vocation, ..AtelierState::default() },
                    (false, Some(json)) => decode_state(json)?,
                    (false, None) => return Ok(Ok(AtelierOutcome::Refused)),
                    _ => return Err(DurabilityError::InvalidStoredState),
                };
                let data = WheelGemData::embedded().map_err(|_|DurabilityError::InvalidStoredState)?;
                if ruleset.revision() != data.revision() {
                    return Ok(Ok(AtelierOutcome::Refused));
                }
                if state.revision != request.expected_atelier_revision {
                    return Ok(Ok(AtelierOutcome::Refused));
                }
                if initialize {
                    let secret: Vec<u8> = sqlx::query_scalar("SELECT secret FROM game_atelier_decision_root WHERE singleton=true")
                        .fetch_one(&mut **tx).await?;
                    initialize_gems(&mut state, fence.character_id.as_bytes(), request.occurrence, &secret)?;
                } else {
                    if !actions::apply(&mut tx, &fence, &request, &mut state).await? {
                        return Ok(Ok(AtelierOutcome::Refused));
                    }
                }
                let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                let json = serde_json::to_value(&state).map_err(|_| DurabilityError::InvalidStoredState)?;
                // Receipt first, then both Character successors, then the Atelier state.
                sqlx::query("INSERT INTO game_character_atelier_receipts(character_id,occurrence,request_sha256,before_revision,after_revision,outcome_json,kind,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT character_id,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8::text::numeric,$9::text::numeric,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(binding)
                    .bind(i64::try_from(request.expected_atelier_revision).map_err(|_| DurabilityError::InvalidStoredState)?)
                    .bind(i64::try_from(state.revision).map_err(|_| DurabilityError::InvalidStoredState)?)
                    .bind(&json).bind(actions::kind(&request.mutation)).bind(root.revision.to_string()).bind(next.to_string())
                    .execute(&mut **tx).await?;
                for query in [
                    "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                    "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                ] {
                    let changed = sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut **tx).await?;
                    if changed.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
                }
                sqlx::query("INSERT INTO game_character_atelier_state(character_id,atelier_revision,gold,fragments_basic,fragments_supreme,state_json) VALUES(encode($1,'hex')::uuid,$2,0,0,0,$3) ON CONFLICT(character_id) DO UPDATE SET atelier_revision=excluded.atelier_revision,state_json=excluded.state_json")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(i64::try_from(state.revision).map_err(|_| DurabilityError::InvalidStoredState)?).bind(json).execute(&mut **tx).await?;
                let revision = CharacterRevision::new(next).map_err(|_| DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;
                Ok(Ok(AtelierOutcome::Committed { revision, state }))
            })
        }).await?
    }
}

fn decode_state(json: serde_json::Value) -> std::result::Result<AtelierState, DurabilityError> {
    serde_json::from_value(json).map_err(|_| DurabilityError::InvalidStoredState)
}

pub(crate) fn initial_occurrence(character: &[u8; 16]) -> [u8; 16] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:atelier:GEM_INIT:v1");
    hash.update(character);
    let digest = hash.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x80;
    id[8] = (id[8] & 63) | 0x80;
    id
}

fn initialize_gems(
    state: &mut AtelierState,
    character: &[u8; 16],
    occurrence: [u8; 16],
    secret: &[u8],
) -> std::result::Result<(), DurabilityError> {
    if secret.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let data = WheelGemData::embedded().map_err(|_| DurabilityError::InvalidStoredState)?;
    let vocation = base_vocation(&state.vocation).ok_or(DurabilityError::InvalidStoredState)?;
    let family = &data.gems()["vocations"][vocation];
    let mut hash = Sha256::new();
    hash.update(secret);
    hash.update(b"gem_initial");
    hash.update(character);
    hash.update(data.revision().as_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut index = 0_u64;
    for domain in [
        WheelDomain::Green,
        WheelDomain::Red,
        WheelDomain::Purple,
        WheelDomain::Blue,
    ] {
        for quality in [GemQuality::Lesser, GemQuality::Regular] {
            let mut mods = Vec::new();
            let count = if quality == GemQuality::Lesser { 1 } else { 2 };
            for list in ["basic_mods_position_1", "basic_mods_position_2"]
                .into_iter()
                .take(count)
            {
                let choices: Vec<u16> = family[list]
                    .as_array()
                    .ok_or(DurabilityError::InvalidStoredState)?
                    .iter()
                    .filter_map(|v| v.as_u64().and_then(|v| u16::try_from(v).ok()))
                    .filter(|v| !mods.contains(v))
                    .collect();
                if choices.is_empty() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let draw = deterministic_decision_u64(
                    &root,
                    DecisionOccurrenceId::from_bytes(occurrence),
                    "gem_initial",
                    index,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?;
                mods.push(choices[(draw % choices.len() as u64) as usize]);
                index += 1;
            }
            let mut hash = Sha256::new();
            hash.update(b"oteryn:atelier:initial_gem:v1");
            hash.update(occurrence);
            hash.update(index.to_be_bytes());
            let digest = hash.finalize();
            let mut id = [0; 16];
            id.copy_from_slice(&digest[..16]);
            let id = u128::from_be_bytes(id);
            state.gems.insert(
                id,
                Gem {
                    id,
                    quality,
                    domain,
                    mods,
                    locked: false,
                    vessel: false,
                    ruleset_revision: data.revision().to_owned(),
                },
            );
        }
    }
    state.revision = 1;
    Ok(())
}

pub(super) async fn load_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
) -> std::result::Result<Option<AtelierState>, DurabilityError> {
    let json: Option<serde_json::Value> = sqlx::query_scalar("SELECT state_json FROM game_character_atelier_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let mut state = json.map(decode_state).transpose()?;
    if let Some(state) = &mut state {
        let vessels = super::character_wheel::read_vessels(tx, character).await?;
        for gem in state.gems.values_mut() {
            gem.vessel = vessels.contains(&Some(gem.id.to_be_bytes()));
        }
    }
    Ok(state)
}
