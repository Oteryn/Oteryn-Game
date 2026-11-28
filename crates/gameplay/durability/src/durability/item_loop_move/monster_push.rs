//! Creature-initiated Ground-to-Ground custody within the existing Item owner.
use super::super::{
    item_mint::ground_scope_columns,
    runtime_scope_assignment::{NodeIncarnationProof, assignment_scope_key},
    spell_item_transaction::{SpellItemError, SpellItemScopeAuthority, check_scope_transaction},
    spell_items_abi::{DurableTileItem, SpellGroundTarget},
};
use crate::foundation::ExactActorRef;
use sqlx::{Postgres, Transaction};

pub struct MonsterPushOccurrence<'a> {
    pub actor: ExactActorRef,
    pub due: u64,
    pub node: &'a NodeIncarnationProof,
    pub reset_epoch: u64,
}

pub async fn push_ground_blocker(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    occurrence: &MonsterPushOccurrence<'_>,
    source: &SpellGroundTarget,
    destination: &SpellGroundTarget,
    item: &DurableTileItem,
) -> Result<(), SpellItemError> {
    check_scope_transaction(tx, authority).await?;
    let actor = occurrence.actor;
    if actor.runtime_scope() != authority.runtime_scope()
        || actor.scope_generation().get() != authority.generation()
        || source.content_revision != destination.content_revision
        || source.map_revision != destination.map_revision
        || source.placement_context != destination.placement_context
        || !item.blocks_movement
        || item.immovable_block_solid
        || item.expires_at_unix_ms.is_some()
    {
        return Err(SpellItemError::Rejected("invalid monster push custody"));
    }
    let ordinal = i64::try_from(item.top_down_ordinal)
        .map_err(|_| SpellItemError::Rejected("Ground ordinal"))?;
    let historical: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_monster_item_push_receipts WHERE source_ordinal=$1)",
    )
    .bind(ordinal)
    .fetch_one(&mut **tx)
    .await?;
    if !historical {
        let remaining = super::super::ground_cell_limit::check_ground_cell_root_admission_in_transaction(
            tx, authority, destination, &[],
        )
        .await
        .map_err(|error| error.into_spell_error())?;
        if remaining.len() >= super::super::ground_cell_limit::GROUND_CELL_ROOT_LIMIT {
            return Err(SpellItemError::Rejected("monster push landing full"));
        }
    }
    let scope = authority.runtime_scope();
    let (kind, channel, instance) = ground_scope_columns(scope);
    let moved: bool = sqlx::query_scalar(
        "SELECT game_push_monster_ground_item(
            encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3,
            encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6::text::numeric, $7, $8,
            $9::text::numeric, $10::text::numeric, $11, $12, $13, $14, $15, $16,
            encode($17,'hex')::uuid, $18::text::numeric, $19, $20::text::numeric)",
    )
    .bind(item.item_instance_id.as_slice())
    .bind(authority.world_bytes().as_slice())
    .bind(kind)
    .bind(channel.as_ref().map(|id| id.as_slice()))
    .bind(instance.as_ref().map(|id| id.as_slice()))
    .bind(authority.generation().to_string())
    .bind(assignment_scope_key(scope).as_slice())
    .bind(actor.placement_identity().as_slice())
    .bind(occurrence.due.to_string())
    .bind(item.state_revision.to_string())
    .bind(ordinal)
    .bind(&source.spatial_position)
    .bind(&destination.spatial_position)
    .bind(&source.map_revision)
    .bind(&source.content_revision)
    .bind(&source.placement_context)
    .bind(occurrence.node.fact().node_id().as_bytes().as_slice())
    .bind(occurrence.node.fact().registration_revision().to_string())
    .bind(occurrence.node.sql_possession_secret())
    .bind(occurrence.reset_epoch.to_string())
    .fetch_one(&mut **tx)
    .await?;
    if !moved {
        return Err(SpellItemError::Rejected("monster push source no longer current"));
    }
    Ok(())
}

/// Script tags survive current custody. Native-map rows use the original
/// source attributes even after a later relocation.
pub async fn unprotected_blocker(
    tx: &mut Transaction<'_, Postgres>,
    item: &DurableTileItem,
) -> Result<bool, SpellItemError> {
    let attributes: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT source_attributes FROM game_native_map_item_receipts
         WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.item_instance_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .flatten();
    attributes.as_ref().map_or(Ok(true), |tags| {
        Ok(super::super::spell_item_transaction::source_map_protection_tags(tags)? == (false, false))
    })
}
