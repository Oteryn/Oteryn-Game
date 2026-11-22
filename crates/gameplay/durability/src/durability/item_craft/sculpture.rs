use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use super::super::ground_cell_limit::{GroundCellAdmissionError, GroundCellLimitError};
use super::super::item_mint::{GroundPlacement, uuid_text};
use super::super::item_mint_audit::Type2Transaction;
use super::super::spell_item_transaction::SpellItemError;
use super::super::spell_owner_commit::SpellLanePermit;

/// Current map custody for an on-ground sculpture material.
#[derive(Debug)]
pub struct SculptureGroundTarget {
    pub position: Vec<u8>,
    pub ordinal: u64,
}

/// Actor tile for a marble yield that cannot be carried, and an optional dropped target.
#[derive(Debug)]
pub struct GroundSculpture {
    pub actor: GroundPlacement,
    pub target: Option<SculptureGroundTarget>,
}

pub(super) struct GroundContext<'a> {
    pub operand: &'a GroundSculpture,
    pub recovery: &'a CharacterRecoveryFenceV1,
    pub node: &'a NodeIncarnationProof,
}

impl GroundContext<'_> {
    pub(super) fn placement(&self, ice: bool) -> GroundPlacement {
        let mut placement = self.operand.actor.clone();
        if let Some(target) = self.operand.target.as_ref().filter(|_| ice) {
            placement.spatial_position.clone_from(&target.position);
        }
        placement
    }

    pub(super) async fn check_room(
        &self,
        tx: &mut Type2Transaction<'_>,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        placement: &GroundPlacement,
        output: &Value,
    ) -> Result<Result<(), UseDisposition>, DurabilityError> {
        let id = uuid_text(output["item"].as_str().ok_or(DurabilityError::InvalidStoredState)?)?;
        match super::super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
            tx, permit, self.recovery, self.node,
            (fence.runtime_scope, fence.scope_ownership_generation.get()),
            placement, &id,
        ).await {
            Ok(()) => Ok(Ok(())),
            Err(GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom)) =>
                Ok(Err(UseDisposition::NoRoom)),
            Err(GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput)) =>
                Ok(Err(UseDisposition::NotPossible)),
            Err(GroundCellAdmissionError::Source(SpellItemError::Rejected(_) | SpellItemError::FieldCapabilityRefused)) =>
                Ok(Err(UseDisposition::Rejected)),
            Err(GroundCellAdmissionError::Source(SpellItemError::Database(error))) => Err(error.into()),
            Err(GroundCellAdmissionError::Source(SpellItemError::Durability(error))) => Err(error),
            Err(GroundCellAdmissionError::Source(SpellItemError::Training(_))) => Err(DurabilityError::Unavailable),
        }
    }
}

pub(super) async fn lock_ground_material(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    material: [u8; 16],
    target: &SculptureGroundTarget,
    context: &GroundSculpture,
) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(sqlx::query(
        "SELECT i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,to_jsonb(g) AS ground_source
           FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id)
          WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid
            AND i.lifecycle=1 AND i.quantity=1 AND g.channel_id=encode($3,'hex')::uuid
            AND g.runtime_scope_ownership_generation=$4::text::numeric AND g.spatial_position=$5
            AND g.stack_ordinal=$6 AND g.map_revision=$7 AND g.content_revision=$8
            AND g.native_room_placement_context=$9
            AND (game_item_loop_drop_ref(g.corpse_ref) OR game_item_action_ground_source(i.item_instance_id))
          FOR UPDATE OF i,g",
    )
    .bind(material.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string()).bind(&target.position)
    .bind(i64::try_from(target.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(&context.actor.map_revision).bind(&context.actor.content_revision)
    .bind(&context.actor.native_room_placement_context)
    .fetch_optional(&mut ***tx).await?)
}

pub(super) fn ground_proof(
    fence: CurrentCharacterItemFence,
    placement: &GroundPlacement,
) -> Result<Value, DurabilityError> {
    let (_, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let hex = |bytes: &[u8]| bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    Ok(json!({
        "channel": hex(channel.as_bytes()),
        "generation": fence.scope_ownership_generation.get().to_string(),
        "position": hex(&placement.spatial_position),
        "map_revision": placement.map_revision,
        "content_revision": placement.content_revision,
        "frame": hex(&placement.native_room_placement_context),
    }))
}

pub(super) async fn place_ground(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    output: &Value,
    placement: &GroundPlacement,
) -> Result<(), DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
            runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context)
         VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)",
    )
    .bind(output).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string()).bind(&placement.spatial_position)
    .bind(&placement.corpse_ref).bind(&placement.map_revision).bind(&placement.content_revision)
    .bind(&placement.native_room_placement_context)
    .execute(&mut ***tx).await?;
    Ok(())
}
