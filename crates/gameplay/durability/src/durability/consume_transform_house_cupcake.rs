//! Placed House Cupcakes retain the mature native receipt and exact named-House proof.
use super::*;
use crate::durability::item_use::cupcakes::{self, CupcakeTransformContext};
const CUPCAKE_SOURCE: &str = "SELECT * FROM game_seasonal_cupcake_house_source(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid)";
impl DurabilityRoot {
    pub async fn consume_cupcake_in_house(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        cupcake: &CupcakeTransformContext,
    ) -> Result<ConsumeTransformResult, DurabilityError> {
        if request.command.game_session_id() != context.fence.game_session_id
            || request.target.is_some()
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::Rejected));
        }
        let node = node.clone();
        let mut references = (native, cupcake);
        self.try_issue_semantic_pass()?.run_with_context(&mut references,move|holder,deadline,references|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            crate::durability::db::lock_admission_relations(&mut tx).await?;
            let fence=context.fence;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':' || $2,0))").bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let previous:Option<(Vec<u8>,Vec<u8>,Value,String)>=sqlx::query_as("SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric").bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((intent,character,effects,recipe))=previous {
                let result=if intent==request.intent && character==fence.character_id.as_bytes(){ConsumeTransformResult{disposition:UseDisposition::Committed,fresh:false,effects,recipe:Some(recipe)}}else{ConsumeTransformResult::refused(UseDisposition::Rejected)};
                tx.commit(deadline).await?;return Ok(result);
            }
            if !prove_current_incarnation(&mut tx,&node).await? || !fenced(CURRENT,&node,&context).fetch_one(&mut **tx).await?.try_get::<bool,_>(0)? {return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));}
            let source=fenced(CUPCAKE_SOURCE,&node,&context).bind(request.source.to_vec()).fetch_optional(&mut **tx).await?;
            let Some(source)=source else{return Ok(ConsumeTransformResult::refused(UseDisposition::NothingToUse));};
            let before:Value=source.try_get("before_item")?;
            let definition=TypedDefinitionRef{family:before["definition_family"].as_str().ok_or(DurabilityError::InvalidStoredState)?.into(),production_key:before["definition_production_key"].as_str().ok_or(DurabilityError::InvalidStoredState)?.into(),revision_ref:before["definition_revision_ref"].as_str().ok_or(DurabilityError::InvalidStoredState)?.into()};
            let Some(selected)=oteryn_gameplay_items::seasonal_cupcakes::plan(references.0,&definition)else{return Ok(ConsumeTransformResult::refused(UseDisposition::NothingToUse));};
            if cupcakes::cupcake_id(&definition.production_key)!=Some(references.1.item_id){return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));}
            let bytes:Vec<u8>=source.try_get("spatial_position")?;
            let at=HouseActionContext::decode_spatial_position(&bytes).ok_or(DurabilityError::InvalidStoredState)?;
            if at[2]!=context.position[2] || i64::from(at[0]).abs_diff(i64::from(context.position[0]))>1 || i64::from(at[1]).abs_diff(i64::from(context.position[1]))>1 {return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));}
            if !cupcakes::check_transform_cooldown(&mut tx,&fence,&request,references.1).await? {return Ok(ConsumeTransformResult::refused(UseDisposition::Exhausted));}
            let plan=ActionPlan{recipe:selected.recipe,outputs:Vec::new(),consume_source:true,source_count:1,effects:selected.effects,extra_inputs:Vec::new()};
            let mut location:Value=source.try_get("location")?;
            let provenance:Value=source.try_get("provenance")?;
            let map=location.as_object_mut().ok_or(DurabilityError::InvalidStoredState)?;
            map.insert("provenance".into(),provenance);map.insert("actor_position".into(),json!(context.position));map.insert("node".into(),json!(uuid_text(*node.fact().node_id().as_bytes())));map.insert("node_revision".into(),json!(node.fact().registration_revision().to_string()));
            let ordinal:String=source.try_get("stack_ordinal")?;
            let result=persist_action(None,&mut tx,fence,&request,references.0,&plan,&before,None,None,5,&ordinal,20,&[],&location,None).await?;
            cupcakes::insert_transform(&mut tx,&fence,&request,references.1).await?;
            tx.commit(deadline).await?;Ok(result)
        })).await
    }
}
