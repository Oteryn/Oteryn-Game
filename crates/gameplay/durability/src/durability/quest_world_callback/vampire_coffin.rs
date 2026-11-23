//! Recover a paid encounter from the receipt that wrote the current hunt stage.
use crate::domain::CharacterId;
use crate::durability::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::CharacterProgressionError,
    db::{begin_semantic_transaction, commit_semantic_transaction},
};
use oteryn_gameplay_quests::quest::world_callbacks::WEAKENED_COFFIN_KEYS;

impl DurabilityRoot {
    /// Selection advice for a fresh coffin use after the in-memory encounter was lost.
    pub async fn read_vampire_coffin_payment(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<bool, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let paid = current_payment(&mut tx, character).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(paid)
                })
            })
            .await
            .map_err(Into::into)
    }
}

/// The mission track points at its latest writer, including after a ring hand-in.
/// Both coffin pieces can recover the same payment while that stage remains current.
pub(super) async fn current_payment(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks t \
         JOIN game_character_quest_receipts r USING (character_id, committed_character_revision) \
         WHERE t.character_id=encode($1,'hex')::uuid AND t.track_key=$2 \
         AND r.quest_key=t.quest_key AND r.transition_key=ANY($3))",
    )
    .bind(character.as_bytes().as_slice())
    .bind("oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission03")
    .bind(&WEAKENED_COFFIN_KEYS[..])
    .fetch_one(&mut **tx)
    .await?)
}
