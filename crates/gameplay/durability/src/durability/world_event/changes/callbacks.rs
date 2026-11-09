//! Native quest callbacks observe the existing World owner; no quest state is invented.
use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use crate::durability::quest_world_callback::{QuestWorldContext, context_facts_are_current};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_quests::quest::world_callbacks::world_change_requirement;

/// Joins an already-fenced quest transaction, before consumption or rewards.
/// A retained World snapshot never grants permission; load resolves the current save.
pub async fn require(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    callback: &str,
) -> Result<bool, DurabilityError> {
    let Some((key, stage)) = world_change_requirement(callback) else {
        return Ok(true);
    };
    let Some(day) = super::load(tx, world, epoch).await? else {
        return Ok(false);
    };
    if !super::super::change_active(tx, world, key).await? {
        return Ok(false);
    }
    Ok(match stage {
        None => true,
        Some(stage) => oteryn_gameplay_world_change::counter::CounterKind::from_key(key)
            .is_some_and(|kind| day.world_counters && day.counter_stages[kind.index()] == stage),
    })
}

impl DurabilityRoot {
    /// Stateless entry callbacks use the same current Character/node/map/reset proof
    /// as a value-bearing quest callback. Closed Ground puzzles also retain their
    /// exact Item/custody read-set in this pass without writing personal progress.
    /// Hive gates additionally observe the
    /// QuestState-owned scent timestamp; this creates no quest receipt. The caller retains its permit and rechecks the local
    /// actor/object CAS before applying the prepared movement.
    #[allow(
        clippy::too_many_arguments,
        reason = "the independent current fence and native map context remain explicit"
    )]
    pub async fn observe_world_callback_gate(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        context: QuestWorldContext,
        callback: String,
    ) -> Result<bool, DurabilityError> {
        self.observe_world_callback_gate_input(authority,node,permit,fence,context,callback,None).await
    }

    /// Source portals retain their exact reusable tool through the fenced read pass.
    #[allow(clippy::too_many_arguments, reason="source admission binds the same fence, World snapshot and command")]
    pub async fn observe_source_portal_use(
        &self, authority:&ReconciledCharacterAuthority<'_, '_>, node:&NodeIncarnationProof,
        permit:&SpellLanePermit, fence:CurrentCharacterGameplayFence, context:QuestWorldContext,
        callback:String, command:crate::foundation::CommandRef,
    ) -> Result<bool,DurabilityError> {
        self.observe_world_callback_gate_input(authority,node,permit,fence,context,callback,Some(command)).await
    }

    #[allow(clippy::too_many_arguments, reason="shared observation keeps source custody and current authority in one pass")]
    async fn observe_world_callback_gate_input(
        &self, authority:&ReconciledCharacterAuthority<'_, '_>, node:&NodeIncarnationProof,
        permit:&SpellLanePermit, fence:CurrentCharacterGameplayFence, context:QuestWorldContext,
        callback:String, source_use:Option<crate::foundation::CommandRef>,
    ) -> Result<bool,DurabilityError> {
        let source = if let Some(command)=source_use {
            if command.game_session_id() != fence.game_session_id { return Ok(false); }
            let Some(source)=oteryn_gameplay_quests::quest::world_callbacks::load()
                .map_err(|_|DurabilityError::InvalidStoredState)?.into_iter()
                .find(|c| c.key == callback && c.effects.source_portal.is_some()) else { return Ok(false); };
            Some((source,command))
        } else { None };
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(false);
        };
        permit.check_channel(world_id, channel_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let ground = oteryn_gameplay_quests::quest::world_callbacks::stateless_ground_requirements(
            &callback,
        );
        let mut lane = (permit,source);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut lane, move |holder, deadline, (lane,source)| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_gameplay_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                        || !context_facts_are_current(&mut tx, fence, &context).await?
                        || !require(&mut tx, world_id, context.reset_epoch, &callback).await?
                        || !hive_gate_permitted(
                            &mut tx,
                            world_id,
                            context.reset_epoch,
                            fence.character_id,
                            &callback,
                        )
                        .await?
                    {
                        return Ok(false);
                    }
                    if let Some((source,command)) = source.as_ref() {
                        let request=crate::durability::quest_state::QuestTransitionRequest {
                            transition_key:source.key.clone(),
                            cause:crate::durability::quest_state::QuestCause::Use(*command),
                        };
                        // SourcePortal validation permits exactly one retained input,
                        // no offering, replacement, personal effect or value write.
                        if !crate::durability::quest_world_callback::consume(&mut tx,lane,fence,
                            &request,source,&[],&context.clone().into(),(&recovery,&node)).await? { return Ok(false); }
                    }
                    if let Some(requirements) = &ground
                        && !crate::durability::quest_world_callback::ground_stack::matches_requirements(
                            &mut tx, lane, fence, &context, requirements,
                        ).await? {
                        return Ok(false);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(true)
                })
            })
            .await
    }
}

/// The existing QuestState timestamp is a Character permit, never a World-stage counter.
pub async fn hive_gate_permitted(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    character: crate::domain::CharacterId,
    callback: &str,
) -> Result<bool, DurabilityError> {
    use oteryn_gameplay_quests::quest::hive_access;
    if hive_access::exit_gate(callback).is_some() {
        // The enclosing observer still proves Character/node/map/content/reset.
        // Loading the current World here also refuses a missing/retired epoch.
        return Ok(super::load(tx, world, epoch).await?.is_some());
    }
    let Some(gate) = hive_access::gate(callback) else {
        return Ok(true);
    };
    let Some(day) = super::load(tx, world, epoch).await? else {
        return Ok(false);
    };
    if day.reset_epoch == 0 {
        return Ok(false);
    }
    if day.counter_stages[oteryn_gameplay_world_change::counter::CounterKind::HiveBorn.index()]
        < gate.minimum_stage
    {
        return Ok(false);
    }
    let purchased:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND quest_key=$3")
        .bind(character.as_bytes().as_slice()).bind(hive_access::PURCHASED).bind(oteryn_gameplay_quests::quest::shared_world_progress::HIVE_QUEST)
        .fetch_optional(&mut **tx).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    Ok(purchased.is_some_and(|purchased| hive_access::permit_valid(purchased, now)))
}
