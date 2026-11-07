//! Shared physical switch activation is one fenced, immutable World fact.
use super::*;
use crate::durability::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::shared_switches::recipe;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct Request {
    pub recipe: String,
    pub placement: Vec<u8>,
    pub object_revision: u64,
    pub context: QuestWorldContext,
    #[cfg(feature = "test-support")]
    occurred_for_test: Option<i64>,
}
impl Request {
    pub fn new(
        recipe: String,
        placement: Vec<u8>,
        object_revision: u64,
        context: QuestWorldContext,
    ) -> Self {
        Self {
            recipe,
            placement,
            object_revision,
            context,
            #[cfg(feature = "test-support")]
            occurred_for_test: None,
        }
    }
    #[cfg(feature = "test-support")]
    pub fn for_test_occurrence(mut self, occurred: i64) -> Self {
        self.occurred_for_test = Some(occurred);
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub transaction_id: [u8; 16],
    pub expires_at_unix_ms: i64,
    /// Current PostgreSQL observation, including when an original receipt replays.
    pub observed_at_unix_ms: i64,
    pub active_count: u16,
    pub replayed: bool,
}
impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn activate_shared_switch(
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
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move|holder,deadline,context|Box::pin(async move {
            let (permit,native)=*context;
            let mut tx=crate::durability::db::begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            let Some(recipe)=recipe(&request.recipe) else{return Ok(None)};
            if request.placement.len()!=40 || command.game_session_id()!=fence.game_session_id {return Ok(None)}
            let crate::foundation::RuntimeScopeRefV1::Channel {world_id,channel_id}=fence.runtime_scope else{return Ok(None)};
            let character:String=sqlx::query_scalar("SELECT encode($1,'hex')::uuid::text").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let placement_hex=request.placement.iter().map(|b|format!("{b:02x}")).collect::<String>();
            let intent=serde_json::to_vec(&json!({"recipe":request.recipe,"placement":placement_hex,"object_revision":request.object_revision,"context":request.context,"character":character})).map_err(|_|DurabilityError::InvalidStoredState)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:shared-switch-command:'||encode($1,'hex')||':'||$2,0))")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
            let prior:Option<(String,Vec<u8>,i64,i32,i64)>=sqlx::query_as("SELECT transaction_id::text,intent,expires_at_unix_ms,active_after,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_shared_switch_leases WHERE game_session_id=encode($1,'hex')::uuid AND cause_ordinal=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((transaction,before,expires,count,observed_at_unix_ms))=prior {
                if before!=intent {return Ok(None)}
                let result=Receipt {transaction_id:crate::durability::item_mint::uuid_text(&transaction)?,expires_at_unix_ms:expires,observed_at_unix_ms,active_count:count.try_into().map_err(|_|DurabilityError::InvalidStoredState)?,replayed:true};
                tx.commit(deadline).await?;return Ok(Some(result));
            }
            assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence.item_fence(),Some(command),native.source_digest()).await.map_err(|_|DurabilityError::Unavailable)?;
            if !super::context_is_current(&mut tx,permit,fence,&request.context).await? {return Ok(None)}
            sqlx::query("SELECT game_shared_switch_lock(encode($1,'hex')::uuid,$2,$3::text::numeric)")
                .bind(world_id.as_bytes().as_slice()).bind(&request.context.map_revision).bind(request.context.reset_epoch.to_string()).execute(&mut **tx).await?;
            let (transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM statement_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
            let observed_at_unix_ms=occurred;
            #[cfg(feature="test-support")]
            let occurred=request.occurred_for_test.unwrap_or(occurred);
            // Refuse invalid time before a trigger abort makes the command outcome uncertain.
            // The immutable SQL constraint remains the final independent backstop.
            let admitted:bool=sqlx::query_scalar("SELECT $1::bigint<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint").bind(occurred).fetch_one(&mut **tx).await?;
            if !admitted {return Ok(None)}
            let expires=occurred.checked_add(recipe.lifetime_millis).ok_or(DurabilityError::InvalidStoredState)?;
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_shared_switch_leases WHERE world_id=encode($1,'hex')::uuid AND context->>'map_revision'=$2 AND (context->>'reset_epoch')::numeric=$3::text::numeric AND placement=$4 AND expires_at_unix_ms>$5)")
                .bind(world_id.as_bytes().as_slice()).bind(&request.context.map_revision).bind(request.context.reset_epoch.to_string()).bind(&request.placement).bind(occurred).fetch_one(&mut **tx).await?;
            if active {return Ok(None)}
            let before:i64=sqlx::query_scalar("SELECT game_shared_switch_count(encode($1,'hex')::uuid,$2,$3::text::numeric,$4,$5)")
                .bind(world_id.as_bytes().as_slice()).bind(&request.context.map_revision).bind(request.context.reset_epoch.to_string()).bind(recipe.group).bind(occurred).fetch_one(&mut **tx).await?;
            if !(0..500).contains(&before) {return Ok(None)}
            let before=i32::try_from(before).map_err(|_|DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_shared_switch_leases(transaction_id,event_id,recipe,character_id,world_id,channel_id,game_session_id,cause_ordinal,character_lease_generation,connection_generation,ownership_generation,placement,object_revision,context,intent,intent_sha256,occurred_at_unix_ms,expires_at_unix_ms,active_before,active_after) VALUES($1::uuid,$2::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12,$13::text::numeric,$14,$15,sha256($15),$16,$17,$18,$18+1)")
                .bind(&transaction).bind(&event).bind(recipe.key).bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).bind(fence.character_lease_generation.to_string()).bind(fence.connection_generation.get().to_string()).bind(fence.scope_ownership_generation.get().to_string())
                .bind(&request.placement).bind(request.object_revision.to_string()).bind(serde_json::to_value(&request.context).map_err(|_|DurabilityError::InvalidStoredState)?).bind(&intent).bind(occurred).bind(expires).bind(before).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_shared_switch_audit(event_id,transaction_id,envelope,envelope_sha256) SELECT r.event_id,r.transaction_id,convert_to(jsonb_build_object('lease',to_jsonb(r),'recipe',to_jsonb(p))::text,'UTF8'),sha256(convert_to(jsonb_build_object('lease',to_jsonb(r),'recipe',to_jsonb(p))::text,'UTF8')) FROM game_shared_switch_leases r JOIN game_shared_switch_recipes p USING(recipe) WHERE r.transaction_id=$1::uuid")
                .bind(&transaction).execute(&mut **tx).await?;
            let result=Receipt {transaction_id:crate::durability::item_mint::uuid_text(&transaction)?,expires_at_unix_ms:expires,observed_at_unix_ms,active_count:(before+1).try_into().map_err(|_|DurabilityError::InvalidStoredState)?,replayed:false};
            tx.commit(deadline).await?;Ok(Some(result))
        })).await
    }
}
