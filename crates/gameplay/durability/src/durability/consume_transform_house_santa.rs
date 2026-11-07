//! Closed Santa gift USE under the existing named-House source and actor fences.
use super::*;
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "Node, House, character authority and observed item state are independent admission proofs"
    )]
    pub async fn use_santa_gift_in_house<C: ItemLoopContent>(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        expected: TypedDefinitionRef,
        expected_revision: u64,
    ) -> Result<ConsumeTransformResult, DurabilityError> {
        if request.command.game_session_id() != context.fence.game_session_id
            || request.target.is_some()
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::Rejected));
        }
        let node = node.clone();
        let mut generation = (native, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut generation,move|holder,deadline,generation|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            crate::durability::db::lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await?
                ||!fenced(CURRENT,&node,&context).fetch_one(&mut **tx).await?.try_get::<bool,_>(0)? {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            }
            let fence=context.fence;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let previous:Option<(Vec<u8>,Vec<u8>,Value,String)>=sqlx::query_as("SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((intent,character,effects,recipe))=previous {
                let result=if intent==request.intent&&character==fence.character_id.as_bytes() {
                    ConsumeTransformResult {disposition:UseDisposition::Committed,fresh:false,effects,recipe:Some(recipe)}
                }else {ConsumeTransformResult::refused(UseDisposition::Rejected)};
                tx.commit(deadline).await?;return Ok(result);
            }
            let source=fenced(SOURCE,&node,&context).bind(request.source.to_vec()).fetch_optional(&mut **tx).await?;
            let Some(source)=source else{return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));};
            let before:Value=source.try_get("before_item")?;
            if before["definition_family"].as_str()!=Some(expected.family.as_str())
                ||before["definition_production_key"].as_str()!=Some(expected.production_key.as_str())
                ||before["definition_revision_ref"].as_str()!=Some(expected.revision_ref.as_str())
                ||before["state_revision"].as_u64()!=Some(expected_revision) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            }

            let Some(id)=before["definition_production_key"].as_str().and_then(|k|k.strip_prefix("oteryn:item.tibia.i")).and_then(|id|id.parse::<u32>().ok())
                .filter(|id|oteryn_gameplay_items::novelty_items::santa_gifts::recognizes(*id)) else {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NothingToUse));
            };
            let _=id;
            let mut location:Value=source.try_get("location")?;
            let object=location.as_object_mut().ok_or(DurabilityError::InvalidStoredState)?;
            object.insert("provenance".into(),source.try_get("provenance")?);
            object.insert("actor_position".into(),json!(context.position));
            object.insert("node".into(),json!(uuid_text(*node.fact().node_id().as_bytes())));
            object.insert("node_revision".into(),json!(node.fact().registration_revision().to_string()));
            let position=if location.get("custody_kind").and_then(Value::as_str).unwrap_or("house")=="house" {
                let bytes:Vec<u8>=source.try_get("spatial_position")?;
                HouseActionContext::decode_spatial_position(&bytes).ok_or(DurabilityError::InvalidStoredState)?
            } else {context.position};
            if position[2]!=context.position[2]
                || (i64::from(position[0])-i64::from(context.position[0])).abs()>1
                || (i64::from(position[1])-i64::from(context.position[1])).abs()>1 {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            sqlx::query("SELECT game_santa_arm_house_source(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9::text::numeric,encode($10,'hex')::uuid,$11,encode($12,'hex')::uuid,$13,$14)")
                .bind(node.fact().node_id().as_bytes().as_slice()).bind(node.fact().registration_revision().to_string()).bind(node.sql_possession_secret())
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
                .bind(context.house.world_id().as_bytes().as_slice()).bind(context.house.house_key()).bind(request.source.as_slice())
                .bind(json!(context.position)).bind(json!(position)).execute(&mut **tx).await?;
            let ordinal:String=source.try_get("stack_ordinal")?;
            let result=super::super::santa_leaves::write(&mut tx,&node,None,fence,&request,generation.0,generation.1,&before,5,None,&ordinal,&location,
                Some(GroundEffect {x:position[0],y:position[1],floor:i16::try_from(position[2]).map_err(|_|DurabilityError::InvalidStoredState)?})).await?;
            if result.disposition==UseDisposition::Committed {tx.commit(deadline).await?;}
            Ok(result)
        })).await
    }
}
