//! Read only the same command's committed planting effects, never execute planting.
use super::*;
use crate::durability::consume_transform::ConsumeTransformResult;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "existing independent transaction authority proofs"
    )]
    pub async fn read_item_loop_planting_presentation<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: &LoopMoveRequest,
        native: &NativeGameplayState,
        content: &C,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        let intent = intent_bytes(fence.character_id, request)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if command.game_session_id() != fence.game_session_id
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(None);
        }
        permit.check_scope(fence.runtime_scope)?;
        self.require_current_node_registration(node)
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            let record = record.clone();
            let intent = intent.clone();
            Box::pin(async move {
                let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &record).await?;
                // The drop writer owns this lock before selecting either seed or pot.
                // Absence is conclusive only after an ambiguous former writer ends.
                lock_loop_command(&mut tx, command).await.map_err(|_| DurabilityError::Unavailable)?;
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':' || $2,0))")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .execute(&mut *tx).await?;
                let row: Option<(Vec<u8>, Vec<u8>, serde_json::Value, String)> = sqlx::query_as(
                    "SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string())
                    .fetch_optional(&mut *tx).await?;
                let result = row.filter(|(stored, character, _, recipe)| {
                    stored.as_slice() == intent.as_slice() && character.as_slice() == fence.character_id.as_bytes() && recipe.starts_with("oteryn:planting.")
                }).map(|(_, _, effects, recipe)| ConsumeTransformResult {
                    disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
                    fresh: false, effects, recipe: Some(recipe),
                });
                tx.commit().await?;
                Ok(result)
            })
        }).await
    }
}
