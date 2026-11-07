//! Read the existing craft receipt before a W19 USE handle is resolved again.
use super::*;

impl DurabilityRoot {
    /// A historical source remains addressable after its item and view handle retire.
    /// The normal craft probe still compares the original wire intent and owns replay.
    pub async fn read_event_container_command_source(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Result<Option<[u8; 16]>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Ok(None);
        }
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let source: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT uuid_send(source_item_instance_id) FROM game_item_craft_records \
                 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric \
                   AND character_id=encode($3,'hex')::uuid AND recipe LIKE 'oteryn:event-container.%'",
            )
            .bind(command.game_session_id().as_bytes().as_slice())
            .bind(command.command_id().get().to_string())
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_optional(&mut **tx).await?;
            tx.commit(deadline).await?;
            source.map(|value| value.try_into().map_err(|_| DurabilityError::InvalidStoredState)).transpose()
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
