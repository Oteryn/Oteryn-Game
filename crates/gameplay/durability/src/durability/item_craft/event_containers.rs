//! Read the existing craft receipt before a W19 USE handle is resolved again.
use super::*;

pub enum EventContainerReplaySource {
    Wire([u8; 16]),
    InstanceTagged,
}

pub const REWARD_BAG_INSTANCE_INTENT_TAG: &[u8] = b"oteryn:reward-bag-use:v1";

impl DurabilityRoot {
    /// A historical source remains addressable after its item and view handle retire.
    /// The normal craft probe still compares the original wire intent and owns replay.
    pub async fn read_event_container_command_source(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Result<Option<EventContainerReplaySource>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Ok(None);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let source: Option<(Vec<u8>, bool)> = sqlx::query_as(
                "SELECT uuid_send(source_item_instance_id), \
                        intent = $5::bytea || uuid_send(source_item_instance_id) \
                 FROM game_item_craft_records \
                 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric \
                   AND character_id=encode($3,'hex')::uuid \
                   AND (recipe LIKE 'oteryn:event-container.%' OR recipe=$4)",
            )
            .bind(command.game_session_id().as_bytes().as_slice())
            .bind(command.command_id().get().to_string())
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(BLUE_PRESENT_RECIPE)
            .bind(REWARD_BAG_INSTANCE_INTENT_TAG)
            .fetch_optional(&mut **tx).await?;
            tx.commit(deadline).await?;
            match source {
                Some((_, true)) => Ok(Some(EventContainerReplaySource::InstanceTagged)),
                Some((value, false)) => Ok(Some(EventContainerReplaySource::Wire(
                    value.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                ))),
                None => Ok(None),
            }
        })).await
    }
}

/// Resolve a nested event source through the existing carried-tree owner. Rewards
/// still enter the main backpack, so retiring this leaf never frees a root slot.
pub(super) async fn nested_source(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    source: [u8; 16],
    world: [u8; 16],
) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,\
         i.definition_revision_ref,i.quantity,uuid_send(b.item_instance_id) AS parent,\
         e.placement_ordinal::text AS ordinal,to_jsonb(e) AS nested_source \
         FROM game_item_instances i JOIN game_item_nested_entries e USING(item_instance_id) \
         JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) owner ON true \
         JOIN game_item_container_slots b ON b.character_id=owner.owner_character \
         WHERE i.item_instance_id=encode($1,'hex')::uuid AND owner.owner_character=encode($2,'hex')::uuid \
           AND owner.depth BETWEEN 1 AND 7 AND i.world_id=encode($3,'hex')::uuid \
           AND e.world_id=i.world_id AND b.world_id=i.world_id AND i.lifecycle=1 \
         FOR UPDATE OF i,e",
    ).bind(source.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_slice())
        .fetch_optional(&mut ***tx).await?)
}

pub const BLUE_PRESENT: &str = "oteryn:item.tibia.i6570";
pub(super) const BLUE_PRESENT_RECIPE: &str = "oteryn:reward-bag.event_6570";

pub(super) struct DropAdmission<'a> {
    pub(super) target: &'a LoopGround,
    pub(super) recovery: &'a crate::character_recovery_fence::CharacterRecoveryFenceV1,
    pub(super) node: &'a NodeIncarnationProof,
}

impl DropAdmission<'_> {
    pub(super) async fn admit(
        &self,
        tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterItemFence,
        transaction: &str,
        item: &str,
        parent_capacity: u32,
    ) -> Result<Result<Value, UseDisposition>, DurabilityError> {
        use super::super::ground_cell_limit::{
            GroundCellAdmissionError, GroundCellLimitError,
            check_ground_cell_root_insert_with_recovery,
        };
        use super::super::item_loop_move::{
            ITEM_LOOP_CHANNEL_DROPS_MAX, ITEM_LOOP_DROP_REF_PREFIX,
        };
        use super::super::item_mint::{GroundPlacement, uuid_text};
        use super::super::spell_item_transaction::SpellItemError;

        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        let placement = GroundPlacement {
            spatial_position: self.target.spatial_position.clone(),
            corpse_ref: [
                ITEM_LOOP_DROP_REF_PREFIX,
                uuid_text(transaction)?.as_slice(),
            ]
            .concat(),
            map_revision: self.target.map_revision.clone(),
            content_revision: self.target.content_revision.clone(),
            native_room_placement_context: self.target.placement_context.clone(),
        };
        match check_ground_cell_root_insert_with_recovery(
            tx,
            permit,
            self.recovery,
            self.node,
            (fence.runtime_scope, fence.scope_ownership_generation.get()),
            &placement,
            &uuid_text(item)?,
        )
        .await
        {
            Ok(()) => {}
            Err(GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom)) => {
                return Ok(Err(UseDisposition::NoRoom));
            }
            Err(GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput)) => {
                return Ok(Err(UseDisposition::NotPossible));
            }
            Err(GroundCellAdmissionError::Source(
                SpellItemError::Rejected(_) | SpellItemError::FieldCapabilityRefused,
            )) => return Ok(Err(UseDisposition::Rejected)),
            Err(GroundCellAdmissionError::Source(SpellItemError::Database(error))) => {
                return Err(error.into());
            }
            Err(GroundCellAdmissionError::Source(SpellItemError::Durability(error))) => {
                return Err(error);
            }
            Err(GroundCellAdmissionError::Source(SpellItemError::Training(_))) => {
                return Err(DurabilityError::Unavailable);
            }
        }
        // The key-33 owner lock also serializes ordinary Ground drops.
        let has_room: bool = sqlx::query_scalar(
            "SELECT count(*) < $3 FROM (SELECT 1 FROM game_item_ground_locations
             WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
              AND game_item_loop_drop_ref(corpse_ref) LIMIT $3) drops",
        )
        .bind(world.as_bytes().as_slice())
        .bind(channel.as_bytes().as_slice())
        .bind(ITEM_LOOP_CHANNEL_DROPS_MAX)
        .fetch_one(&mut ***tx)
        .await?;
        if !has_room {
            return Ok(Err(UseDisposition::NoRoom));
        }
        let location = sqlx::query_scalar(
            "SELECT jsonb_build_object('world_id',encode($1,'hex')::uuid,
             'channel_id',encode($2,'hex')::uuid,'scope_kind',1,'instance_id',NULL::uuid,
             'runtime_scope_ownership_generation',$3::text::numeric,'spatial_position',$4::bytea,
             'corpse_ref',$5::bytea,'map_revision',$6::text,'content_revision',$7::text,
             'native_room_placement_context',$8::bytea,'parent_capacity',$9::integer)",
        )
        .bind(world.as_bytes().as_slice())
        .bind(channel.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(placement.spatial_position)
        .bind(placement.corpse_ref)
        .bind(placement.map_revision)
        .bind(placement.content_revision)
        .bind(placement.native_room_placement_context)
        .bind(i32::try_from(parent_capacity).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_one(&mut ***tx)
        .await?;
        Ok(Ok(location))
    }
}

/// Place an output after admission has acquired the key-33 lock in this transaction.
pub(super) async fn place_drop(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    _permit: &super::super::spell_owner_commit::SpellLanePermit,
    output: &Value,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
          runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,
          content_revision,native_room_placement_context,stack_ordinal)
         SELECT ($1->>'item')::uuid,g.world_id,g.channel_id,g.runtime_scope_ownership_generation,
          g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,
          g.native_room_placement_context,($1->>'ordinal')::bigint
         FROM jsonb_populate_record(NULL::game_item_ground_locations,$1->'ground') g",
    )
    .bind(output)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}
