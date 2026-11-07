//! A source STEP's carried debit/absence and branch are one fenced physical fact.
use super::*;
use crate::durability::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::carried_step::recipe;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct Request {
    pub recipe: String,
    pub placement: Vec<u8>,
    pub object_revision: u64,
    pub context: QuestWorldContext,
    /// Prepared by the existing movement owner while holding the actor lane.
    pub success_prepared: bool,
    pub missing_prepared: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub transaction_id: [u8; 16],
    pub consumed: bool,
    pub replayed: bool,
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_carried_step(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        native: &NativeGameplayState,
        request: Request,
    ) -> Result<Option<Receipt>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, native);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit,native)=*context;
            let mut tx=crate::durability::db::begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            let Some(recipe)=recipe(&request.recipe) else { return Ok(None); };
            if request.placement.len()!=40 || command.game_session_id()!=fence.game_session_id {
                return Ok(None);
            }
            let crate::foundation::RuntimeScopeRefV1::Channel { world_id,channel_id }=fence.runtime_scope else { return Ok(None); };
            let character: String=sqlx::query_scalar("SELECT encode($1,'hex')::uuid::text")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let placement_hex=request.placement.iter().map(|b|format!("{b:02x}")).collect::<String>();
            let intent=serde_json::to_vec(&json!({"recipe":request.recipe,"placement":placement_hex,
                "object_revision":request.object_revision,"context":request.context,"character":character}))
                .map_err(|_|DurabilityError::InvalidStoredState)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:carried-step:'||encode($1,'hex')||':'||$2,0))")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let prior: Option<(String,Vec<u8>,bool)>=sqlx::query_as("SELECT transaction_id::text,intent,input_before IS NOT NULL FROM game_carried_step_receipts WHERE game_session_id=encode($1,'hex')::uuid AND cause_ordinal=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((transaction,before,consumed))=prior {
                if before!=intent { return Ok(None); }
                let result=Receipt { transaction_id:crate::durability::item_mint::uuid_text(&transaction)?,consumed,replayed:true };
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence.item_fence(),Some(command),native.source_digest())
                .await.map_err(|_|DurabilityError::Unavailable)?;
            if !super::context_is_current(&mut tx,permit,fence,&request.context).await? { return Ok(None); }
            let complete:bool=sqlx::query_scalar("SELECT game_carried_step_census_complete(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if !complete { return Ok(None); }
            let row=sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 AND i.definition_revision_ref='definition-r1' AND i.quantity>=$4 ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(recipe.item)
                .bind(i64::from(recipe.count)).fetch_optional(&mut **tx).await?;
            let input=if let Some(row)=row {
                if !request.success_prepared { return Ok(None); }
                let Some(policy)=native.item_policy(recipe.item,"definition-r1") else { return Ok(None); };
                let qualified=crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
                    .map_err(|_|DurabilityError::InvalidStoredState)?;
                if policy.record().semantics.charges != crate::content::ReferenceItemField::NotApplicable
                    || !qualified.movable || !qualified.inventory_destination || qualified.container_capacity.is_some()
                    { return Ok(None); }
                let input=Input { item:row.try_get("item")?,before:row.try_get("before_item")?,
                    location:row.try_get("location")?,kind:row.try_get("kind")?,quantity:row.try_get("quantity")?,
                    count:recipe.count,replacement:None };
                if !matches!(input.kind.as_str(),"container"|"nested") { return Ok(None); }
                super::lock_location(&mut tx,&input).await?;
                Some(input)
            } else {
                if !request.missing_prepared { return Ok(None); }
                None
            };
            let (transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
                .fetch_one(&mut **tx).await?;
            let tuple=tx.tuple();
            let envelope=input.as_ref().map(|input|crate::durability::item_craft::audit::encode_map_harvest(
                fence.item_fence(),command,json!({"recipe":recipe.key,"placement":request.placement,"context":request.context,
                    "input":input.before,"location":input.location,"count":input.count,"success":recipe.success}),
                &transaction,&event,occurred,tuple.tuple())).transpose()?;
            sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_carried_step_receipts(transaction_id,event_id,recipe,character_id,world_id,channel_id,game_session_id,cause_ordinal,character_lease_generation,connection_generation,ownership_generation,placement,object_revision,context,intent,intent_sha256,input_before,location_before,location_kind,occurred_at_unix_ms,item_envelope_sha256) VALUES($1::uuid,$2::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12,$13::text::numeric,$14,$15,sha256($15),$16,$17,$18,$19,CASE WHEN $20::bytea IS NULL THEN NULL ELSE sha256($20) END)")
                .bind(&transaction).bind(&event).bind(recipe.key).bind(fence.character_id.as_bytes().as_slice())
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).bind(fence.character_lease_generation.to_string())
                .bind(fence.connection_generation.get().to_string()).bind(fence.scope_ownership_generation.get().to_string())
                .bind(&request.placement).bind(request.object_revision.to_string()).bind(serde_json::to_value(&request.context).map_err(|_|DurabilityError::InvalidStoredState)?)
                .bind(&intent).bind(input.as_ref().map(|i|&i.before)).bind(input.as_ref().map(|i|&i.location))
                .bind(input.as_ref().map(|i|&i.kind)).bind(occurred).bind(envelope.as_deref()).execute(&mut **tx).await?;
            if let Some(input)=&input {
                super::apply_input(&mut tx,permit,fence,input,&transaction).await?;
                let envelope=envelope.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
                    .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
                    .bind(&input.item).bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS)
                    .bind(envelope).execute(&mut **tx).await?;
            }
            sqlx::query("INSERT INTO game_carried_step_audit(event_id,transaction_id,envelope,envelope_sha256) SELECT r.event_id,r.transaction_id,convert_to(jsonb_build_object('receipt',to_jsonb(r),'recipe',to_jsonb(p))::text,'UTF8'),sha256(convert_to(jsonb_build_object('receipt',to_jsonb(r),'recipe',to_jsonb(p))::text,'UTF8')) FROM game_carried_step_receipts r JOIN game_carried_step_recipes p USING(recipe) WHERE r.transaction_id=$1::uuid")
                .bind(&transaction).execute(&mut **tx).await?;
            let result=Receipt {transaction_id:crate::durability::item_mint::uuid_text(&transaction)?,consumed:input.is_some(),replayed:false};
            tx.commit(deadline).await?;
            Ok(Some(result))
        })).await
    }
}
