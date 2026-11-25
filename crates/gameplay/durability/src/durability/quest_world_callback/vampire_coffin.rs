//! Recover a Count encounter from the receipt that wrote the current hunt stage.
use crate::domain::CharacterId;
use crate::durability::{
    DurabilityError, DurabilityRoot,
    boss_encounter::reward_chest::scoped_death_id,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    character_progression::CharacterProgressionError,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_transfer::CurrentCharacterItemFence,
    runtime_scope_assignment::NodeIncarnationProof,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::world_callbacks::{VAMPIRE_COFFIN_QUESTLINE, VampireCoffinKind};

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
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(encounter)
                })
            })
            .await
            .map_err(Into::into)
    }

    /// Record the exact Count before publication. Its durable death settlement
    /// ends recovery. An unpublished or lost runtime actor can be replaced.
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
        publish: impl FnOnce() + Send,
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
        let ready = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
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
                    sqlx::query(
                        "INSERT INTO game_pending_vampire_coffin_payments \
                             (character_id, death_id) \
                             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid) \
                             ON CONFLICT (character_id) DO UPDATE SET death_id=EXCLUDED.death_id",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(death_id.as_slice())
                    .execute(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(true)
                })
            })
            .await?;
        if ready {
            publish();
        }
        Ok(ready)
    }
}

/// The hunt-stage writer determines the variant. For saves published before
/// the exact death ID was stored, the character's later Count reward chest
/// supplies the durable completion evidence.
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
