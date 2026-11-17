//! One original source expiry, admitted under current authority and the quest revision owner.
use super::*;
use oteryn_gameplay_quests::quest::timers::{self, Recipe};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestTimerOccurrence([u8; 16]);
impl QuestTimerOccurrence {
    pub(super) fn from_bytes(bytes: [u8; 16]) -> StoredResult<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Self(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}
#[derive(Clone, Debug)]
pub struct DueQuestTimer {
    pub occurrence: QuestTimerOccurrence,
    pub transition: String,
    pub actor_placement: [u8; 16],
}
impl DurabilityRoot {
    pub async fn read_due_quest_timers(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        actor_placement: [u8; 16],
    ) -> StoredResult<Vec<DueQuestTimer>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            assert_gameplay_fence(&mut tx,&fence,&node).await?.map_err(|_|DurabilityError::Unavailable)?;
            let crate::foundation::RuntimeScopeRefV1::Channel{world_id,channel_id}=fence.runtime_scope else {return Err(DurabilityError::Unavailable)};
            let rows=sqlx::query("SELECT uuid_send(l.transaction_id) AS occurrence,p.recipe,l.context->'actor_placement' AS actor FROM game_shared_switch_leases l JOIN game_quest_timer_rules p ON p.source_recipe=l.recipe WHERE l.character_id=encode($1,'hex')::uuid AND l.game_session_id=encode($2,'hex')::uuid AND l.context->'actor_placement'=$6::jsonb AND NOT EXISTS(SELECT 1 FROM game_quest_timer_observations o WHERE o.lease_id=l.transaction_id AND o.recipe=p.recipe) AND game_quest_timer_source_current(l.transaction_id,p.recipe,encode($1,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($2,'hex')::uuid,$5::text::numeric) ORDER BY l.expires_at_unix_ms,l.transaction_id,p.recipe LIMIT 8")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.character_lease_generation.to_string()).bind(serde_json::to_value(actor_placement).map_err(|_|DurabilityError::InvalidStoredState)?).fetch_all(&mut *tx).await?;
            let mut due=Vec::with_capacity(rows.len());
            for row in rows {
                let transition:String=row.try_get("recipe")?;
                if timers::recipe(&transition).is_none() { return Err(DurabilityError::InvalidStoredState); }
                let bytes:Vec<u8>=row.try_get("occurrence")?;
                due.push(DueQuestTimer {occurrence:QuestTimerOccurrence::from_bytes(bytes.try_into().map_err(|_|DurabilityError::InvalidStoredState)?)?,transition,actor_placement:serde_json::from_value(row.try_get("actor")?).map_err(|_|DurabilityError::InvalidStoredState)?});
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(due)
        })).await
    }
}
pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    recipe: &Recipe,
) -> StoredResult<bool> {
    let QuestCause::ServerTimer(occurrence) = request.cause else {
        return Ok(false);
    };
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let result:bool=sqlx::query_scalar("SELECT game_quest_timer_source_current(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric) AND NOT EXISTS(SELECT 1 FROM game_quest_timer_observations WHERE lease_id=encode($1,'hex')::uuid AND recipe=$2) AND EXISTS(SELECT 1 FROM game_quest_timer_rules WHERE recipe=$2 AND binding=$8 AND quest_key=$9 AND track_key=$10 AND maximum_before=$11 AND value_after=$12 AND source_recipe=$13)")
        .bind(occurrence.as_bytes().as_slice()).bind(recipe.key).bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_lease_generation.to_string()).bind(recipe.binding().as_slice()).bind(recipe.quest).bind(recipe.track).bind(recipe.maximum_before).bind(recipe.value_after).bind(recipe.source_lease_recipe).fetch_one(&mut **tx).await?;
    Ok(result)
}
pub(super) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    recipe: &Recipe,
    before: i64,
    committed: Option<u64>,
) -> StoredResult<()> {
    let QuestCause::ServerTimer(occurrence) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    sqlx::query("INSERT INTO game_quest_timer_observations(lease_id,recipe,character_id,world_id,channel_id,game_session_id,character_lease_generation,connection_generation,ownership_generation,value_before,value_after,committed_revision) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8::text::numeric,$9::text::numeric,$10,$11,$12::text::numeric)")
        .bind(occurrence.as_bytes().as_slice()).bind(recipe.key).bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_lease_generation.to_string()).bind(fence.connection_generation.get().to_string()).bind(fence.scope_ownership_generation.get().to_string()).bind(before).bind(if committed.is_some(){recipe.value_after}else{before}).bind(committed.map(|r|r.to_string())).execute(&mut **tx).await?;
    Ok(())
}
pub(super) async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &QuestTransitionRequest,
) -> StoredResult<()> {
    let QuestCause::ServerTimer(occurrence) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    sqlx::query("INSERT INTO game_quest_timer_audit(lease_id,recipe,envelope,envelope_sha256) SELECT lease_id,recipe,game_quest_timer_envelope(lease_id,recipe),sha256(game_quest_timer_envelope(lease_id,recipe)) FROM game_quest_timer_observations WHERE lease_id=encode($1,'hex')::uuid AND recipe=$2")
        .bind(occurrence.as_bytes().as_slice()).bind(&request.transition_key).execute(&mut **tx).await?;
    Ok(())
}
