//! Closed Ground entrypoint; the native transform receipt, cooldown and snapshot share one TX.
use super::*;
use crate::durability::item_use::cupcakes::CupcakeTransformContext;
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "independent native lane, actor, content, source and prepared successor proofs"
    )]
    pub async fn consume_transform_on_ground_with_cupcake<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        ground: &GroundActionContext,
        cupcake: &CupcakeTransformContext,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            false,
            None,
            ActionContext::Ground(ground),
            None,
            Some(cupcake),
        )
        .await
    }
}

impl DurabilityRoot {
    /// Serializes behind an unresolved native writer. Absence never executes the food action.
    pub async fn probe_cupcake_transform_receipt(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx,&record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':' || $2,0))").bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut *tx).await?;
            let row:Option<(Vec<u8>,Vec<u8>,Value,String)>=sqlx::query_as("SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric").bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let result=row.and_then(|(intent,character,effects,recipe)| {
                if !matches!(recipe.as_str(),"oteryn:seasonal.cupcake.28484"|"oteryn:seasonal.cupcake.28485"|"oteryn:seasonal.cupcake.28486") {return None;}
                Some(if fence.game_session_id==request.command.game_session_id() && character==fence.character_id.as_bytes() && intent==request.intent {ConsumeTransformResult{disposition:UseDisposition::Committed,fresh:false,effects,recipe:Some(recipe)}}else{ConsumeTransformResult::refused(UseDisposition::Rejected)})
            });
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }
}
