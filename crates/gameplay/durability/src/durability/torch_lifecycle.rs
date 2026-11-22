//! The physical torch clock is written under the current owner, independently of custody moves.

use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, check_scope_transaction,
};
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, MovementLocalPosition};
use oteryn_protocol_oteryn::world_object::{MAX_USE_INTENT_BYTES, UseDisposition};

pub struct TorchUse {
    pub command: CommandRef,
    pub item: Option<[u8; 16]>,
    pub intent: Vec<u8>,
    pub position: MovementLocalPosition,
    pub map_digest: [u8; 32],
    pub frame_digest: [u8; 32],
    pub content_digest: [u8; 32],
}

impl DurabilityRoot {
    pub async fn use_torch(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: TorchUse,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > MAX_USE_INTENT_BYTES
        {
            return Ok(Some(UseDisposition::Rejected));
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let session = fence.game_session_id.as_bytes().as_slice();
            let command = request.command.command_id().get().to_string();
            // Keep historical replies independent of the current handle/custody.
            let previous: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT intent FROM game_torch_transitions WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
            ).bind(session).bind(&command).fetch_optional(&mut *tx).await?;
            if let Some(previous) = previous {
                return Ok(Some(if previous == request.intent { UseDisposition::Committed } else { UseDisposition::Rejected }));
            }
            let Some(item) = request.item else { return Ok(None); };
            let torch: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_item_instances i
                 CROSS JOIN LATERAL game_torch_stage(i.definition_production_key) stage
                 WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid
                  AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1')",
            ).bind(item.as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !torch { return Ok(None); }
            match assert_current_equipment_in_transaction(
                &mut tx, &record, &node, &fence, Some(request.command), request.content_digest,
            ).await {
                Ok(_) => {}
                Err(EquipmentError::Rejected(_)) => return Ok(Some(UseDisposition::StaleState)),
                Err(EquipmentError::Database(error)) => return Err(error.into()),
                Err(EquipmentError::Durability(error)) => return Err(error),
            }
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut *tx).await?;
            let pos = [request.position.x, request.position.y, i32::from(request.position.floor)];
            let result: i16 = sqlx::query_scalar(
                "SELECT game_torch_use(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,
                    $4,$5::text::numeric,$6::text::numeric,$7,$8,$9,$10)",
            ).bind(item.as_slice()).bind(session).bind(&command).bind(request.intent)
                .bind(fence.scope_ownership_generation.get().to_string()).bind(fence.connection_generation.get().to_string())
                .bind(pos.as_slice()).bind(hex_revision(request.map_digest)).bind(request.frame_digest.as_slice())
                .bind(hex_revision(request.content_digest)).fetch_one(&mut *tx).await?;
            let result = match result {
                0 => None,
                1 => Some(UseDisposition::Committed),
                2 => Some(UseDisposition::Rejected),
                3 => Some(UseDisposition::TooFar),
                4 => Some(UseDisposition::NotPossible),
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }
}

fn hex_revision(digest: [u8; 32]) -> String {
    super::map_item_mint_audit::map_revision_of(&digest)
}

pub(super) async fn drain_due(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &SpellItemScopeAuthority,
    limit: usize,
) -> Result<usize, SpellItemError> {
    if limit == 0 {
        return Ok(0);
    }
    check_scope_transaction(tx, authority).await?;
    let world = authority.world_bytes();
    let channel = authority
        .channel_bytes()
        .ok_or(SpellItemError::Rejected("torch Channel scope"))?;
    let limit =
        i32::try_from(limit).map_err(|_| SpellItemError::Rejected("torch deadline batch bound"))?;
    let changed: i32 = sqlx::query_scalar(
        "SELECT game_torch_drain(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4)",
    ).bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string())
     .bind(limit).fetch_one(&mut **tx).await?;
    usize::try_from(changed).map_err(|_| SpellItemError::Rejected("torch deadline count"))
}
