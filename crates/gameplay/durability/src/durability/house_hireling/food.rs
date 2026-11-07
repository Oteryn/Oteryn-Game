//! Cook delivery composes native Inbox MINT and the existing coin/bank fee owner.
use super::super::{DurabilityError, DurabilityRoot};
use super::super::character_authority::{ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::character_inbox::{RewardClaimInboxError, RewardClaimInboxMintRequest,
    RewardClaimInboxMintOutput, RewardClaimInboxOutcome, RewardClaimInboxPlan};
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_fee_burn::{FeeBurnCause, FeeBurnError, FeeBurnRequest, HirelingFeeChange, burn_hireling_fee_in_transaction};
use super::super::item_mint::TypedDefinitionRef;
use super::super::item_transfer::{CurrentCharacterItemFence, ItemDefinitionFacts, ItemWeightLookup};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::{CharacterId, hireling::food::FoodPlan};
use crate::foundation::{ChannelId, CommandRef};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone)]
pub struct HirelingFoodRequest {
    pub command: CommandRef,
    pub hireling_id: [u8;16],
    pub intent_binding: [u8;32],
    pub food: FoodPlan,
    pub item: ItemDefinitionFacts,
    pub output_id: [u8;16],
    pub mint_transaction: [u8;16],
    pub fee_transaction: [u8;16],
    pub fee_event: [u8;16],
    pub fee_occurrence: [u8;16],
    pub change: HirelingFeeChange,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirelingFoodOutcome { Committed, AlreadyCommitted }

#[derive(Debug)]
pub enum HirelingFoodError {
    InvalidInput,
    AuthorityRejected,
    InsufficientFunds,
    Capacity,
    InboxFull,
    Conflict,
    Unavailable(DurabilityError),
}

/// Same active House identity, current effective owner and character-wide entitlement.
/// Runs after the caller holds recovery/admission and the exact House item fence.
pub(crate) async fn service_in_transaction(
    tx: &mut Transaction<'_, Postgres>, fence: &CurrentCharacterItemFence, hireling: [u8;16], skill: Option<u16>,
) -> Result<bool, DurabilityError> {
    let row = sqlx::query("SELECT h.owner_character_id::text AS owner FROM game_house_hirelings h \
        JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($2,'hex')::uuid \
        WHERE h.hireling_id=encode($1,'hex')::uuid AND h.active \
        AND h.world_id=s.world_id AND h.house_key=s.runtime_scope_house_key \
        AND s.runtime_scope_kind=2 AND s.runtime_scope_house_key IS NOT NULL \
        AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id \
        FOR SHARE OF h")
        .bind(hireling.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(false); };
    let Some(skill) = skill else { return Ok(true); };
    let owner: String = row.try_get("owner")?;
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_entitlements \
        WHERE character_id=$1::uuid AND entitlement_id=$2 FOR SHARE)")
        .bind(owner).bind(i16::try_from(skill).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_one(&mut **tx).await?)
}

fn valid_food(plan: FoodPlan) -> bool {
    match plan.gold {
        15_000 => matches!(plan.item_id, 29408..=29416),
        90_000 => (29408..=29415).contains(&plan.item_id),
        _ => false,
    }
}

fn source_binding(request: &HirelingFoodRequest) -> [u8;32] {
    let mut hash = Sha256::new(); hash.update(b"oteryn:hireling-food:v1\0");
    hash.update(request.intent_binding); hash.update(request.hireling_id);
    hash.update(request.food.item_id.to_be_bytes()); hash.update(request.food.gold.to_be_bytes());
    hash.update(request.fee_transaction); hash.update(request.fee_event); hash.update(request.fee_occurrence);
    hash.finalize().into()
}

impl DurabilityRoot {
    /// Historical replay never reacquires live NPC or House access.
    pub async fn read_hireling_food_receipt(&self, authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId, command: CommandRef, intent_binding: [u8;32],
    ) -> Result<bool, HirelingFoodError> {
        let recovery = authority.record_for(self).map_err(|_| HirelingFoodError::AuthorityRejected)?;
        self.try_issue_semantic_pass().map_err(HirelingFoodError::Unavailable)?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let binding: Option<Vec<u8>> = sqlx::query_scalar("SELECT intent_binding FROM game_hireling_food_receipts \
                WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid \
                AND command_id=$3::text::numeric")
                .bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let result = match binding { None => Ok(false), Some(bound) if bound == intent_binding => Ok(true), Some(_) => Err(HirelingFoodError::Conflict) };
            commit_semantic_transaction(tx,deadline).await?; Ok(result)
        })).await.map_err(HirelingFoodError::Unavailable)?
    }

    pub async fn commit_hireling_food(&self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence, origin_channel: ChannelId, request: HirelingFoodRequest,
        weights: &dyn ItemWeightLookup,
    ) -> Result<HirelingFoodOutcome, HirelingFoodError> {
        if !valid_food(request.food) || request.item.definition.production_key != format!("oteryn:item.tibia.i{}",request.food.item_id)
            || request.command.game_session_id() != fence.game_session_id {
            return Err(HirelingFoodError::InvalidInput);
        }
        if self.read_hireling_food_receipt(authority,fence.character_id,request.command,request.intent_binding).await? {
            return Ok(HirelingFoodOutcome::AlreadyCommitted);
        }
        let plan = RewardClaimInboxPlan { command:request.command, character_id:fence.character_id,
            world_id:fence.runtime_scope.world_id(),channel_id:origin_channel,transaction_id:request.mint_transaction,
            source_binding:source_binding(&request),item_instance_ids:vec![request.output_id] };
        let mint = RewardClaimInboxMintRequest { claim:TypedDefinitionRef {family:"RewardClaim".into(),
            production_key:format!("oteryn:reward.hireling.food.{}",request.food.item_id),revision_ref:"crystal-201189f".into()},
            content_revision:request.content_revision.clone(),ruleset_revision:request.ruleset_revision.clone(),sim_revision:request.sim_revision.clone(),
            outputs:vec![RewardClaimInboxMintOutput {source_line:2190,item:request.item.clone(),quantity:1,key_number:None}] };
        let mut candidate = self.prepare_hireling_inbox_mint(authority,node,fence.item_fence(),&plan,mint).await.map_err(map_inbox)?;
        struct Owner<'a> { request:HirelingFoodRequest, node:NodeIncarnationProof, weights:&'a dyn ItemWeightLookup,
            failure:Option<HirelingFoodError>, at:i64 }
        let mut owner = Owner { request,node:node.clone(),weights,failure:None,at:candidate.occurred_at_unix_ms() };
        let result = self.commit_hireling_inbox_mint(authority,node,fence.item_fence(),&mut candidate,&mut owner,
            move |context,owner| Box::pin(async move {
                if !service_in_transaction(context.transaction(),&fence.item_fence(),owner.request.hireling_id,Some(1002)).await? {
                    owner.failure=Some(HirelingFoodError::AuthorityRejected); return Err(RewardClaimInboxError::SourceRefused);
                }
                // Native bridge has placed the new output, so <=20 is the donor preflight <20.
                // Crystal data/items/items.xml:46144-46148 Store Inbox capacity.
                let inbox_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **context.transaction()).await?;
                if inbox_count>20 {owner.failure=Some(HirelingFoodError::InboxFull);return Err(RewardClaimInboxError::SourceRefused);}
                let weight = owner.request.item.weight.ok_or(RewardClaimInboxError::InvalidPlan)?;
                let carried = super::super::item_loop_move::carried_weight(context.transaction(),fence.character_id,
                    *fence.runtime_scope.world_id().as_bytes(),owner.weights).await.map_err(|_|RewardClaimInboxError::InvalidPlan)?;
                let capacity = super::super::item_transfer::read_derived_transfer_capacity_centi_oz(context.transaction(),fence.character_id,owner.weights).await?;
                if carried.checked_add(u64::from(weight)).is_none_or(|total|total>capacity) {
                    owner.failure=Some(HirelingFoodError::Capacity); return Err(RewardClaimInboxError::SourceRefused);
                }
                let fee = FeeBurnRequest { cause:FeeBurnCause::HirelingService {
                    service:format!("oteryn:service.hireling.food.{}",owner.request.food.item_id),occurrence:owner.request.fee_occurrence },
                    fee_gold_units:owner.request.food.gold,transaction_id:owner.request.fee_transaction,event_id:owner.request.fee_event,
                    occurred_at_unix_ms:owner.at,server_build_id:SERVER_BUILD_ID.into(),change:owner.request.change.clone() };
                if let Err(error)=burn_hireling_fee_in_transaction(context.type2_transaction(),&owner.node,&fence,origin_channel,&fee).await {
                    owner.failure=Some(match error { FeeBurnError::InsufficientFunds=>HirelingFoodError::InsufficientFunds,
                        FeeBurnError::Unavailable(error)=>HirelingFoodError::Unavailable(error),_=>HirelingFoodError::Capacity });
                    return Err(RewardClaimInboxError::SourceRefused);
                }
                sqlx::query("INSERT INTO game_hireling_food_receipts(character_id,game_session_id,command_id,hireling_id, \
                    intent_binding,item_id,price_gold,mint_transaction_id,food_item_instance_id,fee_transaction_id, \
                    fee_occurrence_id,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
                    $3::text::numeric,encode($4,'hex')::uuid,$5,$6,$7::text::numeric,encode($8,'hex')::uuid, \
                    encode($9,'hex')::uuid,encode($10,'hex')::uuid,encode($11,'hex')::uuid,$12::text::numeric)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(owner.request.command.game_session_id().as_bytes().as_slice())
                    .bind(owner.request.command.command_id().get().to_string()).bind(owner.request.hireling_id.as_slice())
                    .bind(owner.request.intent_binding.as_slice()).bind(i64::from(owner.request.food.item_id)).bind(owner.request.food.gold.to_string())
                    .bind(owner.request.mint_transaction.as_slice()).bind(owner.request.output_id.as_slice()).bind(owner.request.fee_transaction.as_slice())
                    .bind(owner.request.fee_occurrence.as_slice()).bind(fence.expected_character_revision.get().to_string())
                    .execute(&mut **context.transaction()).await?;
                Ok(())
            })).await;
        match result {
            Ok(RewardClaimInboxOutcome::Committed {..})=>Ok(HirelingFoodOutcome::Committed),
            Ok(RewardClaimInboxOutcome::AlreadyCommitted(_))=>Ok(HirelingFoodOutcome::AlreadyCommitted),
            Err(error)=>Err(owner.failure.unwrap_or_else(||map_inbox(error))),
        }
    }
}

fn map_inbox(error:RewardClaimInboxError)->HirelingFoodError {
    match error {
        RewardClaimInboxError::Unavailable(error)=>HirelingFoodError::Unavailable(error),
        RewardClaimInboxError::AuthorityRejected=>HirelingFoodError::AuthorityRejected,
        RewardClaimInboxError::ConflictingDelivery=>HirelingFoodError::Conflict,
        RewardClaimInboxError::CapacityExceeded|RewardClaimInboxError::Delivery(_)=>HirelingFoodError::InboxFull,
        _=>HirelingFoodError::InvalidInput,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_donor_dish_price_pairs_are_accepted() {
        for id in 29408..=29415 {assert!(valid_food(FoodPlan{item_id:id,gold:90_000}));}
        assert!(!valid_food(FoodPlan{item_id:29416,gold:90_000}));
        assert!(valid_food(FoodPlan{item_id:29409,gold:15_000}));
        assert!(!valid_food(FoodPlan{item_id:35173,gold:15_000}));
        assert!(!valid_food(FoodPlan{item_id:29412,gold:1}));
    }
}
