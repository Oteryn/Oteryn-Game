//! Baker hand-in: carried substance cost and World counter have one physical commit.
use super::{advance_world, finish_world_change, lock_world};
use crate::durability::carried_cost::{self, ClosedCostCause, NpcItemCost};
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_equipment::{
    EquipmentError, assert_current_equipment_in_transaction,
};
use crate::durability::db::begin_type2_transaction;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;

pub const BAKERS: [&str; 5] = [
    "oteryn:npc.balduin_the_baker",
    "oteryn:npc.benny_the_baker",
    "oteryn:npc.biff_the_baker",
    "oteryn:npc.billy_the_baker",
    "oteryn:npc.brian_the_baker",
];

#[derive(Debug, Clone)]
pub struct CakeDeliveryRequest {
    pub command: CommandRef,
    pub npc_key: String,
    pub substance: TypedDefinitionRef,
    pub content_digest: [u8; 32],
    pub intent: Vec<u8>,
    pub now_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CakeDelivery {
    pub accepted: u16,
    pub substances: u16,
    pub golems: u16,
    pub stage: u8,
}

fn world_error(error: super::BossClockError) -> DurabilityError {
    match error {
        super::BossClockError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "one physical cost/World transaction with exact source and current authority"
    )]
    pub async fn deliver_cake_substance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        request: CakeDeliveryRequest,
    ) -> Result<Option<CakeDelivery>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || !BAKERS.contains(&request.npc_key.as_str())
            || request.now_seconds < 0
            || request.substance.family != "Item"
            || request.substance.production_key != "oteryn:item.tibia.i12143"
            || request.content_digest == [0; 32]
            || request.intent.is_empty()
            || request.intent.len() > 4096
        {
            return Ok(None);
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        let binding: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:cake-delivery:v1")
            .chain_update(fence.character_id.as_bytes())
            .chain_update(fence.runtime_scope.world_id().as_bytes())
            .chain_update(request.npc_key.as_bytes())
            .chain_update(request.content_digest)
            .chain_update(request.substance.revision_ref.as_bytes())
            .chain_update(&request.intent)
            .finalize()
            .into();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, permit| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cake-delivery:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            if let Some(row) = sqlx::query("SELECT character_id=encode($3,'hex')::uuid AS same_character,binding,result FROM game_seasonal_cake_deliveries WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await? {
                if !row.try_get::<bool,_>("same_character")? || row.try_get::<Vec<u8>,_>("binding")? != binding { return Ok(None); }
                let result = serde_json::from_value(row.try_get("result")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            match assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence,
                Some(request.command), request.content_digest).await {
                Ok(_) => {},
                Err(EquipmentError::Rejected(_)) => return Ok(None),
                Err(EquipmentError::Database(error)) => return Err(error.into()),
                Err(EquipmentError::Durability(error)) => return Err(error),
            }
            let (world_id,channel_id)=crate::durability::item_transfer::scope_of(&fence).map_err(|_|DurabilityError::InvalidStoredState)?;
            super::authority::arm(&mut tx,&node,world_id,channel_id,fence.scope_ownership_generation,request.now_seconds).await.map_err(world_error)?;
            let Some(mut world) = lock_world(&mut tx, fence.runtime_scope.world_id(), request.now_seconds).await.map_err(world_error)? else { return Ok(None); };
            advance_world(&mut tx, &mut world, request.now_seconds).await.map_err(world_error)?;
            if world.state.stage != 1 || request.now_seconds >= world.event_end { return Ok(None); }
            let available = carried_cost::available(&mut tx, fence, &request.substance).await?;
            let accepted = available.min(u32::from(200 - world.state.substances));
            let mut result = CakeDelivery { accepted: u16::try_from(accepted).map_err(|_| DurabilityError::InvalidStoredState)?,
                substances: world.state.substances, golems: world.state.golems, stage: world.state.stage };
            if accepted == 0 { tx.commit(deadline).await?; return Ok(Some(result)); }
            let source = format!("cake:{}:{}", world.event_start, request.npc_key);
            let Some(cost) = carried_cost::burn(&mut tx, permit, fence, request.command, ClosedCostCause::CakeSubstance,
                &source, &[NpcItemCost { definition: request.substance, count: accepted }]).await? else { return Ok(None); };
            let previous = world.state.clone();
            world.state.substances += result.accepted;
            finish_world_change(&mut tx, &mut world, request.now_seconds, previous).await.map_err(world_error)?;
            result.substances = world.state.substances; result.stage = world.state.stage;
            sqlx::query("INSERT INTO game_seasonal_cake_deliveries(game_session_id,command_id,character_id,world_id,event_start,cost_transaction_id,binding,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6::uuid,$7,$8)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.world.as_bytes().as_slice()).bind(world.event_start)
                .bind(cost).bind(binding.as_slice()).bind(serde_json::to_value(&result).map_err(|_| DurabilityError::InvalidStoredState)?)
                .execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Some(result))
        })).await
    }
}
