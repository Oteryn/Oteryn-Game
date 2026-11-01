//! ANIMUS-1 durable, monotonic per-species mastery owner.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use sqlx::Row;

pub const MAX_ANIMUS_MASTERIES: usize = 2_000;
const MAX_ANIMUS_MASTERIES_SQL: i64 = 2_000;

fn valid_species_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 160
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b':' | b'_' | b'.' | b'-')
        })
}

impl DurabilityRoot {
    /// Records a Soulpit-qualified completion. Replays are idempotent and completion is monotonic.
    pub async fn complete_animus_mastery(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        species_key: &str,
    ) -> Result<bool, CharacterProgressionError> {
        if !valid_species_key(species_key) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let species_key = species_key.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let inserted =
                        match complete_in_transaction(&mut tx, fence.character_id, &species_key)
                            .await
                        {
                            Ok(inserted) => inserted,
                            Err(error) => return Ok(Err(error)),
                        };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(inserted))
                })
            })
            .await?
    }

    /// Reads the character-owned completion set under the reconciled recovery authority.
    pub async fn read_animus_masteries(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<Vec<String>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let rows = sqlx::query(
                    "SELECT species_key FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid ORDER BY species_key LIMIT $2",
                )
                .bind(character.as_bytes().as_slice())
                .bind(MAX_ANIMUS_MASTERIES_SQL)
                .fetch_all(&mut *tx)
                .await?;
                let keys = rows
                    .into_iter()
                    .map(|row| row.try_get::<String, _>("species_key"))
                    .collect::<Result<Vec<_>, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(keys))
            }))
            .await?
    }
}

/// The existing monotonic owner, composed in Soulpit's completion transaction. The caller
/// has already proved the current gameplay fence and holds the character root lock.
pub(super) async fn complete_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    species: &str,
) -> Result<bool, CharacterProgressionError> {
    if !valid_species_key(species) {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid AND species_key=$2)")
        .bind(character.as_bytes().as_slice()).bind(species).fetch_one(&mut **tx).await
        .map_err(DurabilityError::from)?;
    if exists {
        return Ok(false);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid LIMIT $2) bounded")
        .bind(character.as_bytes().as_slice()).bind(MAX_ANIMUS_MASTERIES_SQL)
        .fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    if count >= MAX_ANIMUS_MASTERIES_SQL {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let inserted = sqlx::query("INSERT INTO game_character_animus_masteries(character_id,species_key) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING")
        .bind(character.as_bytes().as_slice()).bind(species).execute(&mut **tx).await
        .map_err(DurabilityError::from)?.rows_affected() == 1;
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_keys_are_canonical_and_bounded() {
        assert!(valid_species_key("oteryn:creature.dragon"));
        assert!(!valid_species_key("Dragon"));
        assert!(!valid_species_key("dragon lord"));
        assert!(!valid_species_key(""));
    }
}
