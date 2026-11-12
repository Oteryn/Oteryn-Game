//! Newhaven's one-shot entitlement is consumed by the atomic corpse/loot MINT owner.
use super::DurabilityError;
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

/// Trusted preparation only. Preparing this value never consumes an entitlement.
#[derive(Debug, Clone)]
pub struct TutorialCorruptorClaim {
    pub(crate) fence: CurrentCharacterGameplayFence,
    pub(crate) death: CreatureDeathOccurrenceKey,
    items: Vec<u32>,
    pub(crate) draw_ordinals: Vec<u32>,
}

impl TutorialCorruptorClaim {
    /// The exact vocation gear selected under the current Character fence.
    pub fn items(&self) -> &[u32] {
        &self.items
    }

    /// Bind guaranteed outputs to distinct first-roll planner source ordinals.
    pub fn bind_draw_ordinals(mut self, first: u32) -> Option<Self> {
        self.draw_ordinals = (0..self.items.len())
            .map(|offset| first.checked_add(u32::try_from(offset).ok()?))
            .collect::<Option<Vec<_>>>()?;
        Some(self)
    }

    /// Selected Item IDs paired with their distinct first-roll planner sources.
    pub fn source_outputs(&self) -> impl ExactSizeIterator<Item = (u32, u32)> + '_ {
        self.items
            .iter()
            .copied()
            .zip(self.draw_ordinals.iter().copied())
    }
}

/// The one-shot branch has a stable MINT cause separate from ordinary drops.
pub fn guarantee_purpose(item: u32) -> String {
    // Shorter than the original canonical Item purpose, so remapping cannot
    // exceed the planner's already validated byte-footprint budget.
    format!("oteryn:nhc.i{item}")
}

impl DurabilityRoot {
    /// Replays retain the original item selection; an uncommitted preparation leaves
    /// the arm available after a process loss. Only the atomic batch writer consumes it.
    pub async fn prepare_tutorial_corruptor(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        death: CreatureDeathOccurrenceKey,
    ) -> Result<Option<TutorialCorruptorClaim>, CharacterProgressionError> {
        if death.channel_id().is_none()
            || fence.runtime_scope != death.runtime_scope()
            || death.scope_ownership_generation() != fence.scope_ownership_generation
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let prior = prior_claim(&mut tx, fence, death).await?;
                    let items = if let Some(items) = prior {
                        items
                    } else {
                        let TutorialProgressOutcome::Read(Some(stored)) =
                            tutorial_progress_in_transaction(&mut tx, fence.character_id, None)
                                .await?
                        else {
                            return Ok(Ok(None));
                        };
                        if !stored.progress.corruptor {
                            return Ok(Ok(None));
                        }
                        let build =
                            super::character_death::load_death_build(&mut tx, fence.character_id)
                                .await?;
                        corruptor_items(build.vocation()).to_vec()
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok((!items.is_empty()).then_some(TutorialCorruptorClaim {
                        fence,
                        death,
                        items,
                        draw_ordinals: Vec::new(),
                    })))
                })
            })
            .await?
    }
}

async fn prior_claim(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    death: CreatureDeathOccurrenceKey,
) -> Result<Option<Vec<u32>>, DurabilityError> {
    let channel = death
        .channel_id()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let prior: Option<Vec<i32>> = sqlx::query_scalar("SELECT item_ids FROM game_tutorial_corruptor_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND scope_generation=$4::text::numeric(20,0) AND actor_id=$5 AND actor_generation=$6::text::numeric(20,0)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(death.world_id().as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(death.scope_ownership_generation().get().to_string()).bind(i64::from(death.actor_local_id())).bind(death.actor_local_generation().to_string())
        .fetch_optional(&mut **tx).await?;
    prior
        .map(|ids| {
            ids.into_iter()
                .map(|id| u32::try_from(id).map_err(|_| DurabilityError::InvalidStoredState))
                .collect()
        })
        .transpose()
}

/// Called only inside the same root-locked transaction that inserted the corpse
/// and its entire frozen gear set. Any subsequent MINT failure rolls this back too.
pub(super) async fn consume_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    claim: &TutorialCorruptorClaim,
) -> Result<(), DurabilityError> {
    let fence = claim.fence;
    assert_gameplay_fence(tx, &fence, node)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if let Some(prior) = prior_claim(tx, fence, claim.death).await? {
        if prior != claim.items {
            return Err(DurabilityError::InvalidStoredState);
        }
        return Ok(());
    }
    let build = super::character_death::load_death_build(tx, fence.character_id).await?;
    if claim.items.is_empty() || claim.items != corruptor_items(build.vocation()) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let TutorialProgressOutcome::Read(Some(stored)) =
        tutorial_progress_in_transaction(tx, fence.character_id, None).await?
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let next = tutorial_transition(
        true,
        Some(stored.progress),
        TutorialEvent::Corruptor,
        "none",
        false,
    )
    .ok_or(DurabilityError::InvalidStoredState)?;
    let death = claim.death;
    let channel = death
        .channel_id()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let ids: Vec<i32> = claim
        .items
        .iter()
        .map(|id| i32::try_from(*id).map_err(|_| DurabilityError::InvalidStoredState))
        .collect::<Result<_, _>>()?;
    sqlx::query("INSERT INTO game_tutorial_corruptor_claims(character_id,world_id,channel_id,scope_generation,actor_id,actor_generation,item_ids) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5,$6::text::numeric(20,0),$7)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(death.world_id().as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(death.scope_ownership_generation().get().to_string()).bind(i64::from(death.actor_local_id())).bind(death.actor_local_generation().to_string())
        .bind(ids).execute(&mut **tx).await?;
    tutorial_progress_in_transaction(
        tx,
        fence.character_id,
        Some((stored.version, next.progress)),
    )
    .await?;
    Ok(())
}
