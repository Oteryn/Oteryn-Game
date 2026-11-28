//! Recover a Count encounter from the receipt that wrote the current hunt stage.
use crate::domain::CharacterId;
use crate::durability::db::commit_semantic_read;
use crate::durability::{
    DurabilityError, DurabilityRoot,
    boss_encounter::reward_chest::scoped_death_id,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    character_progression::CharacterProgressionError,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_transfer::CurrentCharacterItemFence,
    runtime_scope_assignment::{
        AssignmentState, NodeIncarnationProof, assignment_scope_key, load_assignment,
    },
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{ActorLocalGeneration, ActorLocalId, ActorRef, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::world_callbacks::{VAMPIRE_COFFIN_QUESTLINE, VampireCoffinKind};
use sqlx::Row;

impl DurabilityRoot {
    /// Selection advice for a fresh coffin use after the in-memory encounter was lost.
    pub async fn read_pending_vampire_coffin(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<Option<VampireCoffinKind>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let encounter = pending_encounter(&mut tx, character).await?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(encounter)
                })
            })
            .await
            .map_err(Into::into)
    }

    /// Record the exact Count before publication. Its durable death settlement
    /// ends recovery. Replacing a published actor requires the original owner
    /// to prove retirement or the persisted assignment to prove supersession.
    #[allow(
        clippy::too_many_arguments,
        reason = "the publication uses the same current item fence as payment"
    )]
    pub async fn deliver_vampire_coffin(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: [u8; 32],
        kind: VampireCoffinKind,
        count: ExactActorRef,
        mut retains_previous: impl FnMut(ExactActorRef) -> bool + Send,
    ) -> Result<bool, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        if count.runtime_scope() != fence.runtime_scope
            || count.scope_generation() != fence.scope_ownership_generation
        {
            return Ok(false);
        }
        let death_id = scoped_death_id(
            count.runtime_scope(),
            count.scope_generation().get(),
            count.actor_local_id(),
            count.actor_local_generation(),
        );
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run_with_context(
                &mut retains_previous,
                move |holder, deadline, retains_previous| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        assert_current_equipment_in_transaction(
                            &mut tx,
                            &recovery,
                            &node,
                            &fence,
                            Some(command),
                            content,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                        if pending_encounter(&mut tx, fence.character_id).await? != Some(kind) {
                            return Ok(false);
                        }
                        let scope_key = assignment_scope_key(fence.runtime_scope);
                        if !previous_spawn_ended(&mut tx, &fence, &scope_key, retains_previous)
                            .await?
                        {
                            return Ok(false);
                        }
                        sqlx::query(
                            "INSERT INTO game_pending_vampire_coffin_payments \
                             (character_id, death_id, spawn_scope, spawn_scope_generation, \
                              spawn_actor_id, spawn_actor_generation) \
                             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, \
                                     $4::text::numeric, $5, $6::text::numeric) \
                             ON CONFLICT (character_id) DO UPDATE SET death_id=EXCLUDED.death_id, \
                               spawn_scope=EXCLUDED.spawn_scope, \
                               spawn_scope_generation=EXCLUDED.spawn_scope_generation, \
                               spawn_actor_id=EXCLUDED.spawn_actor_id, \
                               spawn_actor_generation=EXCLUDED.spawn_actor_generation",
                        )
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(death_id.as_slice())
                        .bind(scope_key.as_slice())
                        .bind(count.scope_generation().get().to_string())
                        .bind(i64::from(count.actor_local_id()))
                        .bind(count.actor_local_generation().to_string())
                        .execute(&mut *tx)
                        .await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(true)
                    })
                },
            )
            .await
    }
}

async fn previous_spawn_ended(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    current_scope: &[u8; 33],
    retains_previous: impl FnOnce(ExactActorRef) -> bool,
) -> Result<bool, DurabilityError> {
    // The current Character fence locks the root before reading or replacing
    // this row. A prior assignment's generation can only advance.
    let row = sqlx::query(
        "SELECT spawn_scope, spawn_scope_generation::text, spawn_actor_id, \
                spawn_actor_generation::text FROM game_pending_vampire_coffin_payments \
         WHERE character_id=encode($1,'hex')::uuid AND death_id IS NOT NULL",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else { return Ok(true) };
    let Some(scope) = row.try_get::<Option<Vec<u8>>, _>("spawn_scope")? else {
        // Older records cannot prove a still-live spawn has been retired.
        return Ok(false);
    };
    let generation = row
        .try_get::<String, _>("spawn_scope_generation")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if scope.as_slice() == current_scope && generation == fence.scope_ownership_generation.get() {
        let id = u32::try_from(row.try_get::<i64, _>("spawn_actor_id")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let actor_generation = row
            .try_get::<String, _>("spawn_actor_generation")?
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let previous = ExactActorRef(ActorRef::from_scoped_owner_parts(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            ActorLocalId(id),
            ActorLocalGeneration(actor_generation),
        ));
        return Ok(!retains_previous(previous));
    }
    let previous = load_assignment(tx, &scope, false).await?;
    Ok(previous.is_some_and(|assignment| {
        assignment.ownership_generation > generation
            || (assignment.ownership_generation == generation
                && assignment.state == AssignmentState::Revoked)
    }))
}

/// The hunt-stage writer determines the variant. For saves published before
/// the exact death ID was stored, the character's later Count reward chest
/// supplies the durable completion evidence. The ordinary Count uses the
/// immutable corpse MINT receipt, whose definition key is its corpse item.
pub(super) async fn pending_encounter(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<Option<VampireCoffinKind>, DurabilityError> {
    let transition: Option<String> = sqlx::query_scalar(
        "SELECT r.transition_key FROM game_character_quest_tracks t \
         JOIN game_character_quest_receipts r USING (character_id, committed_character_revision) \
         JOIN game_character_roots c USING (character_id) \
         LEFT JOIN game_pending_vampire_coffin_payments p USING (character_id) \
         WHERE t.character_id=encode($1,'hex')::uuid AND t.track_key=$2 \
         AND r.quest_key=t.quest_key AND t.value=9 \
         AND NOT EXISTS (SELECT 1 FROM game_boss_death_settlements d \
                         WHERE d.world_id=c.world_id AND d.death_id=p.death_id) \
         AND NOT EXISTS (SELECT 1 FROM game_item_mint_receipts m \
                         WHERE m.death_world_id=c.world_id AND m.death_scope_key=p.spawn_scope \
                           AND m.death_scope_ownership_generation=p.spawn_scope_generation \
                           AND m.death_actor_local_id=p.spawn_actor_id \
                           AND m.death_actor_local_generation=p.spawn_actor_generation \
                           AND m.loot_table_production_key='oteryn:item.tibia.i8109' \
                           AND m.loot_purpose_key='CORPSE_MATERIALIZATION') \
         AND (p.death_id IS NOT NULL OR NOT EXISTS ( \
             SELECT 1 FROM game_character_boss_reward_chests b \
             WHERE b.character_id=c.character_id AND b.world_id=c.world_id \
               AND b.boss_key IN ('oteryn:creature.the_count','oteryn:creature.the_weakened_count') \
               AND b.minted_at >= ( \
                   SELECT q.committed_at FROM game_character_quest_receipts q \
                   WHERE q.character_id=c.character_id AND q.quest_key=t.quest_key \
                     AND q.values_before[array_position(q.track_keys,t.track_key)]=8 \
                     AND q.values_after[array_position(q.track_keys,t.track_key)]=9 \
                   ORDER BY q.committed_character_revision DESC LIMIT 1)))",
    )
    .bind(character.as_bytes().as_slice())
    .bind(VAMPIRE_COFFIN_QUESTLINE)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(transition
        .as_deref()
        .and_then(VampireCoffinKind::for_callback))
}

pub(super) async fn record_encounter(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query("INSERT INTO game_pending_vampire_coffin_payments (character_id) VALUES (encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(character.as_bytes().as_slice())
        .execute(&mut **tx).await?.rows_affected() == 1)
}
