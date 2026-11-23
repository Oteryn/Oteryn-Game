//! Recover a Count encounter from the receipt that wrote the current hunt stage.
use crate::domain::CharacterId;
use crate::durability::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    character_progression::CharacterProgressionError,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_transfer::CurrentCharacterItemFence,
    runtime_scope_assignment::NodeIncarnationProof,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::CommandRef;
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

    /// Bind the already-admitted payment to the exact staged Count before
    /// publishing its actor. Only the canonical death transaction clears it.
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
        death: [u8; 16],
        publish: impl FnOnce() + Send,
    ) -> Result<bool, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let binding = self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_current_equipment_in_transaction(
                &mut tx, &recovery, &node, &fence, Some(command), content,
            ).await.map_err(|_| DurabilityError::Unavailable)?;
            if pending_encounter(&mut tx, fence.character_id).await? != Some(kind) {
                return Ok(false);
            }
            let bound = sqlx::query("UPDATE game_pending_vampire_coffin_payments SET death_id=encode($2,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(death.as_slice())
                .execute(&mut *tx).await?;
            if bound.rows_affected() != 1 {
                return Ok(false);
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(true)
        })).await?;
        if binding {
            publish();
        }
        Ok(binding)
    }
}

/// The latest hunt-stage writer determines which Count was admitted. The existing
/// recovery row covers either kind while the hunt still awaits its ring.
pub(super) async fn pending_encounter(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<Option<VampireCoffinKind>, DurabilityError> {
    let transition: Option<String> = sqlx::query_scalar(
        "SELECT r.transition_key FROM game_character_quest_tracks t \
         JOIN game_character_quest_receipts r USING (character_id, committed_character_revision) \
         JOIN game_pending_vampire_coffin_payments p USING (character_id) \
         WHERE t.character_id=encode($1,'hex')::uuid AND t.track_key=$2 \
         AND r.quest_key=t.quest_key",
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
