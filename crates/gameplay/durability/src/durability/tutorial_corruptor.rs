//! Claim one Corruptor reward using the existing death identity and tutorial reducer.
use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::character_tutorial::{TutorialProgressOutcome, tutorial_progress_in_transaction};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::CreatureDeathOccurrenceKey;
use oteryn_gameplay_quests::tutorial::{TutorialEvent, corruptor_items, tutorial_transition};

impl DurabilityRoot {
    /// A same-death retry gets exactly the first selection; another death gets nothing
    /// after the track is cleared. No client input constructs a creature-death key.
    pub async fn claim_tutorial_corruptor(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        death: CreatureDeathOccurrenceKey,
        defined: Vec<u32>,
    ) -> Result<Vec<u32>, CharacterProgressionError> {
        if defined.len() > 3
            || death.world_id() != fence.runtime_scope.world_id()
            || fence.runtime_scope
                != crate::foundation::RuntimeScopeRefV1::channel(
                    death.world_id(),
                    death.channel_id(),
                )
            || death.scope_ownership_generation() != fence.scope_ownership_generation
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(error));}
            let previous:Option<Vec<i32>>=sqlx::query_scalar("SELECT item_ids FROM game_tutorial_corruptor_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND scope_generation=$4::text::numeric(20,0) AND actor_id=$5 AND actor_generation=$6::text::numeric(20,0)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(death.world_id().as_bytes().as_slice()).bind(death.channel_id().as_bytes().as_slice())
                .bind(death.scope_ownership_generation().get().to_string()).bind(i64::from(death.actor_local_id())).bind(death.actor_local_generation().to_string())
                .fetch_optional(&mut *tx).await?;
            if let Some(previous)=previous {
                let previous=previous.into_iter().map(|id|u32::try_from(id).map_err(|_|super::DurabilityError::InvalidStoredState)).collect::<Result<Vec<_>,_>>()?;
                commit_semantic_transaction(tx,deadline).await?;
                return Ok(Ok(previous));
            }
            let TutorialProgressOutcome::Read(Some(stored))=tutorial_progress_in_transaction(&mut tx,fence.character_id,None).await? else {return Ok(Ok(Vec::new()));};
            let Some(next)=tutorial_transition(true,Some(stored.progress),TutorialEvent::Corruptor,"none",false) else {return Ok(Ok(Vec::new()));};
            let build=super::character_death::load_death_build(&mut tx,fence.character_id).await?;
            if !defined.is_empty() && defined!=corruptor_items(build.vocation()) {return Ok(Err(CharacterProgressionError::InvalidInput));}
            let ids=defined.iter().map(|id|i32::try_from(*id)).collect::<Result<Vec<_>,_>>().map_err(|_|super::DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_tutorial_corruptor_claims(character_id,world_id,channel_id,scope_generation,actor_id,actor_generation,item_ids) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5,$6::text::numeric(20,0),$7)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(death.world_id().as_bytes().as_slice()).bind(death.channel_id().as_bytes().as_slice())
                .bind(death.scope_ownership_generation().get().to_string()).bind(i64::from(death.actor_local_id())).bind(death.actor_local_generation().to_string()).bind(ids).execute(&mut *tx).await?;
            tutorial_progress_in_transaction(&mut tx,fence.character_id,Some((stored.version,next.progress))).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(defined))
        })).await?
    }
}
