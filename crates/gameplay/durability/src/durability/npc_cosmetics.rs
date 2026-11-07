//! S06: closed current-World NPC exchanges. Costs, quest tracks and account
//! appearance facts share one fenced transaction and one immutable source receipt.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetic, EarnedCosmetics, UnlockFact,
};
use crate::domain::{AccountId, CharacterId};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::npc_cosmetics as rules;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

mod actor_observation;
pub use actor_observation::NpcSourceActorObservation;
mod effects;
mod greet_context;
mod payment_facts;
pub use greet_context::NpcSourceGreetContext;
pub use payment_facts::NpcSourcePaymentFacts;
mod program;
pub(crate) use program::NpcProgramContext;
pub use program::{requires_ajax_fire, has_readonly_source_greet};
pub use program::{NpcSourceActorEffect, NpcSourceEffects};
pub(crate) use program::{source_fee_cash_minimum, source_fee_price};

#[derive(Clone, Debug)]
pub struct NpcCosmeticRequest {
    pub command: CommandRef,
    pub npc: String,
    pub message: String,
    pub topic: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpcCosmeticOutcome {
    pub reply: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reply_lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advance_notices: Vec<String>,
    pub topic: u16,
    pub inventory_changed: bool,
    pub committed: bool,
    pub close: bool,
    #[serde(default)]
    pub magic_effect: Option<u16>,
    /// Transient delivery witness: retained receipts decode as replay, never fresh.
    #[serde(skip)]
    pub fresh: bool,
}

fn binding(
    character: CharacterId,
    request: &NpcCosmeticRequest,
) -> Result<Vec<u8>, DurabilityError> {
    Ok(Sha256::new()
        .chain_update(b"oteryn:npc-cosmetic:v1\0")
        .chain_update(character.as_bytes())
        .chain_update(request.command.game_session_id().as_bytes())
        .chain_update(request.command.command_id().get().to_be_bytes())
        .chain_update(
            serde_json::to_vec(&(&request.npc, &request.message, request.topic))
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .finalize()
        .to_vec())
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    request: &NpcCosmeticRequest,
) -> Result<Option<NpcCosmeticOutcome>, CharacterProgressionError> {
    let row = sqlx::query("SELECT character_id=encode($3,'hex')::uuid AS same_character,request_binding,outcome FROM game_npc_cosmetic_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).bind(character.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    let Some(row) = row else {
        return Ok(None);
    };
    if !row
        .try_get::<bool, _>("same_character")
        .map_err(DurabilityError::from)?
        || row
            .try_get::<Vec<u8>, _>("request_binding")
            .map_err(DurabilityError::from)?
            != binding(character, request)?
    {
        return Err(CharacterProgressionError::ConflictingOccurrence);
    }
    Ok(Some(
        serde_json::from_value(row.try_get("outcome").map_err(DurabilityError::from)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    ))
}

impl DurabilityRoot {
    /// Historical read only: a receipt may reproduce a result but never obtain a
    /// current NPC, controller, scope or Character lease from its stored fields.
    pub async fn reconcile_npc_cosmetic(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        request: NpcCosmeticRequest,
    ) -> Result<Option<NpcCosmeticOutcome>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let outcome = replay(&mut tx, character, &request).await;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcome)
                })
            })
            .await?
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the NPC source composes independent gameplay, item-lane and active-content owners"
    )]
    pub async fn commit_npc_cosmetic(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcCosmeticRequest,
        permit: &SpellLanePermit,
        native: &NativeGameplayState,
    ) -> Result<NpcCosmeticOutcome, CharacterProgressionError> {
        if request.command.game_session_id() != fence.game_session_id
            || !rules::supported(&request.npc)
            || request.message.len() > 256
            || request.message.is_empty()
            || (request.topic > 3
                && !(request.npc == "oteryn:npc.hairycles" && request.topic == 18))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = (permit, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, (permit, native)| Box::pin(async move {
            let permit = *permit;
            let native = *native;
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:npc-cosmetic:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            match replay(&mut tx, fence.character_id, &request).await {
                Ok(Some(outcome)) => { tx.commit(deadline).await?; return Ok(Ok(outcome)); },
                Err(error) => return Ok(Err(error)),
                Ok(None) => {},
            }
            match assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence.item_fence(), Some(request.command), native.source_digest()).await {
                Ok(_) => {},
                Err(EquipmentError::Rejected(_)) => return Ok(Err(CharacterProgressionError::AuthorityRejected)),
                Err(EquipmentError::Database(error)) => return Err(error.into()),
                Err(EquipmentError::Durability(error)) => return Err(error),
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root, Err(error) => return Ok(Err(error)),
            };
            let account: sqlx::types::Uuid = sqlx::query_scalar("SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            super::character_appearance::lock_account_grants(&mut tx, account).await?;
            let state = snapshot(&mut tx, fence.character_id, account, native).await?;
            let choice = rules::choose(&request.npc, &request.message, u8::try_from(request.topic).map_err(|_|DurabilityError::InvalidStoredState)?, &state);
            let mut outcome = NpcCosmeticOutcome { advance_notices:Vec::new(),reply_lines:Vec::new(),reply: choice.as_ref().map(|c| c.reply).unwrap_or("").into(),
                topic: choice.as_ref().map_or(0, |c| u16::from(c.topic)), inventory_changed: false, committed: false, close: false, magic_effect:None, fresh:false };
            let Some(action) = choice.and_then(|choice| choice.action) else { tx.commit(deadline).await?; return Ok(Ok(outcome)); };
            let Some(plan) = rules::plan(action, &state) else { return Ok(Err(CharacterProgressionError::InvalidInput)); };
            #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE plan_selected");
            let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            let progress = sqlx::query("SELECT character_revision::text,profile_revision,ruleset_revision,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if numeric_u64(&progress,"character_revision")? != root.revision || !state_matches_root(&progress,&root) {
                #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE progression_context_mismatch");
                return Err(DurabilityError::InvalidStoredState);
            }
            #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE progression_context_valid");
            let content = native.cosmetics().filter(|content| content.source_digest() == native.source_digest()).ok_or(DurabilityError::InvalidStoredState)?;
            let mut grant_keys = Vec::new();
            if let Some(grant) = &plan.grant {
                let outfit = grant.outfit.to_owned();
                let target=if grant.addon==0 {EarnedCosmetic::Outfit(&outfit)}else{EarnedCosmetic::Addon(&outfit,grant.addon)};
                if content.earned_unlock(grant.key, rules::PROVENANCE, target) != UnlockFact::Compatible {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let owned=if grant.addon==0 {state.bases.contains(&outfit)}else{state.addons.contains(&(outfit,grant.addon))};
                if !owned { grant_keys.push(grant.key.to_owned()); }
                else if !plan.costs.is_empty() {
                    outcome.reply = "You already own this outfit addon.".into();
                    tx.commit(deadline).await?; return Ok(Ok(outcome));
                }
            }
            let mut costs = Vec::with_capacity(plan.costs.len());
            for cost in &plan.costs {
                let policy = native.item_policy_for_source_id(u32::from(cost.item)).ok_or(DurabilityError::InvalidStoredState)?;
                let record = policy.record();
                if !record.admission.materializable { return Err(DurabilityError::InvalidStoredState); }
                let definition = super::item_mint::TypedDefinitionRef { family: record.production_definition.family.clone(),
                    production_key: record.production_definition.production_key.clone(), revision_ref: record.production_definition.revision_ref.clone() };
                if super::carried_cost::available(&mut tx, fence.item_fence(), &definition).await? < cost.count {
                    outcome.reply = cost.missing.into();
                    tx.commit(deadline).await?; return Ok(Ok(outcome));
                }
                costs.push(super::carried_cost::NpcItemCost { definition, count: cost.count });
            }
            let cost_transaction = if costs.is_empty() { None } else {
                let Some(transaction) = super::carried_cost::burn(&mut tx, permit, fence.item_fence(), request.command,
                    super::carried_cost::ClosedCostCause::NpcCosmetic, plan.source_key, &costs).await? else {
                    return Err(DurabilityError::InvalidStoredState);
                };
                Some(transaction)
            };
            #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE source_costs_valid");
            outcome.committed = true;
            outcome.magic_effect=plan.magic_effect;
            outcome.inventory_changed = !costs.is_empty();
            outcome.close = action == rules::Action::AmberStart;
            let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut **tx).await?;
            let changes = serde_json::to_value(&plan.changes).map_err(|_| DurabilityError::InvalidStoredState)?;
            let encoded = serde_json::to_value(&outcome).map_err(|_| DurabilityError::InvalidStoredState)?;
            let inserted = sqlx::query("INSERT INTO game_npc_cosmetic_receipts(transaction_id,character_id,account_id,game_session_id,command_id,npc_key,source_key,request_binding,outcome,track_changes,grant_keys,cost_transaction_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT $1::uuid,s.character_id,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9,$10,$11::uuid,s.character_revision,$12::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision FROM game_character_progression_state s WHERE s.character_id=encode($13,'hex')::uuid")
                .bind(&transaction).bind(account).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .bind(&request.npc).bind(plan.source_key).bind(binding(fence.character_id, &request)?).bind(encoded).bind(changes).bind(&grant_keys)
                .bind(cost_transaction.as_deref()).bind(next.to_string()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?.rows_affected();
            if inserted != 1 { return Err(DurabilityError::InvalidStoredState); }
            for query in ["UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric"] {
                if sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut **tx).await?.rows_affected() != 1 {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE source_receipt_roots_written");
            effects::apply(&mut tx, fence.character_id, account, &transaction, next, &plan, &grant_keys).await?;
            #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE source_effects_written");
            tx.commit(deadline).await?;
            #[cfg(any(test, feature = "test-support"))] eprintln!("SRV7_NPC_STAGE source_commit_known");
            outcome.fresh=true;
            Ok(Ok(outcome))
        })).await?
    }
}

async fn snapshot(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    account: sqlx::types::Uuid,
    native: &NativeGameplayState,
) -> Result<rules::Snapshot, DurabilityError> {
    let rows: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key LIMIT 13")
        .bind(character.as_bytes().as_slice()).bind(rules::TRACKS.as_slice()).fetch_all(&mut **tx).await?;
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
        .fetch_one(&mut **tx)
        .await?;
    let mut result = rules::Snapshot {
        tracks: rows.into_iter().collect(),
        now,
        ..Default::default()
    };
    let content = native
        .cosmetics()
        .filter(|content| content.source_digest() == native.source_digest())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let sex = super::character_appearance::item_action_sex(tx, character).await?;
    let account = AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts = super::character_appearance::read_account_appearance_unlocks(
        tx,
        account,
        &content.earned_grant_keys(),
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts: Vec<_> = facts
        .into_iter()
        .filter(|fact| {
            matches!(
                fact.source_kind.as_str(),
                "quest" | "npc" | "npc_wedding" | "mount_taming" | "task_board" | "item_action"
            )
        })
        .map(|fact| (fact.unlock_key, fact.content_revision))
        .collect();
    let earned = EarnedCosmetics {
        content,
        facts: &facts,
    }
    .with_sex(sex);
    for key in [
        rules::CITIZEN,
        rules::DRUID,
        rules::RIFT,
        rules::ROYAL,
        rules::SHAMAN,
    ] {
        let key = key.to_owned();
        if content.outfit_is_starter(&key, sex) || earned.outfit(&key) == UnlockFact::Compatible {
            result.bases.insert(key.clone());
        }
        for addon in [1, 2] {
            if content.addon_is_starter(&key, sex, addon)
                || earned.addon(&key, addon) == UnlockFact::Compatible
            {
                result.addons.insert((key.clone(), addon));
            }
        }
    }
    Ok(result)
}
