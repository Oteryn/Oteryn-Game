//! Current shared-counter observation; caller clocks/counts never enter authority.
use super::*;
use crate::durability::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    runtime_scope_assignment::NodeIncarnationProof,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;

#[derive(Clone, Debug)]
pub struct Request {
    pub gate: String,
    pub context: QuestWorldContext,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub count: u16,
    pub personal_completion: bool,
    pub observed_at_unix_ms: i64,
    pub next_change_unix_ms: Option<i64>,
}
impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn read_shared_switch_count(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        native: &NativeGameplayState,
        request: Request,
    ) -> Result<Option<Observation>, DurabilityError> {
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
            let Some(gate)=oteryn_gameplay_quests::quest::shared_switches::gate(&request.gate) else{return Ok(None)};
            let Some(recipe)=oteryn_gameplay_quests::quest::shared_switches::recipe(gate.counter_recipe) else{return Ok(None)};
            let crate::foundation::RuntimeScopeRefV1::Channel {world_id,..}=fence.runtime_scope else{return Ok(None)};
            assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence.item_fence(),None,native.source_digest()).await.map_err(|_|DurabilityError::Unavailable)?;
            if !super::context_is_current(&mut tx,permit,fence,&request.context).await? {return Ok(None)}
            let (now,count,next,personal_completion):(i64,i64,Option<i64>,bool)=sqlx::query_as("WITH clock AS (SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS now) SELECT clock.now,game_shared_switch_count(encode($1,'hex')::uuid,$2,$3::text::numeric,$4,clock.now),(SELECT min(r.expires_at_unix_ms) FROM game_shared_switch_leases r JOIN game_shared_switch_recipes p USING(recipe) WHERE r.world_id=encode($1,'hex')::uuid AND r.context->>'map_revision'=$2 AND (r.context->>'reset_epoch')::numeric=$3::text::numeric AND p.group_key=$4 AND r.expires_at_unix_ms>clock.now),EXISTS(SELECT 1 FROM game_character_quest_tracks t WHERE t.character_id=encode($5,'hex')::uuid AND t.track_key=$6 AND t.quest_key=$7 AND t.value=1) FROM clock")
                .bind(world_id.as_bytes().as_slice()).bind(&request.context.map_revision).bind(request.context.reset_epoch.to_string()).bind(recipe.group).bind(fence.character_id.as_bytes().as_slice()).bind(gate.personal_completion).bind(gate.quest).fetch_one(&mut **tx).await?;
            if !(0..=500).contains(&count) {return Err(DurabilityError::InvalidStoredState)}
            if !super::context_is_current(&mut tx,permit,fence,&request.context).await? {return Ok(None)}
            let result=Observation {personal_completion,count:count.try_into().map_err(|_|DurabilityError::InvalidStoredState)?,observed_at_unix_ms:now,next_change_unix_ms:next};
            tx.commit(deadline).await?;Ok(Some(result))
        })).await
    }
}
