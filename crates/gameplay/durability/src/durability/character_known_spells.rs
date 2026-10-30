//! Durable, automatically acquired player-spell identities (news 8675).
//!
//! This projection is monotonic: reaching an eligible level/vocation adds a spell, while a
//! later vocation transition never silently removes knowledge already earned. Definitions that
//! still require an explicit learning owner are deliberately excluded by the dispatch planner.

use std::collections::BTreeSet;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};

/// The quest writer calls this only after its gameplay fence and root lock, inside
/// the same transaction as the earned shrine flag. Existing automatic knowledge is kept.
pub(super) async fn grant_quest_spell(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    key: &str,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_known_spells(character_id,spell_key) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING")
        .bind(character.as_bytes().as_slice()).bind(key).execute(&mut **tx).await?;
    Ok(())
}

impl DurabilityRoot {
    /// Add dispatch-qualified keys under the current session and catalogue snapshot revision.
    pub async fn acquire_automatic_spells(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        eligible: &BTreeSet<String>,
    ) -> Result<(), CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let eligible = eligible.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    for key in eligible {
                        let inserted = sqlx::query(
                            "INSERT INTO game_character_known_spells(character_id, spell_key) \
                             SELECT encode($1, 'hex')::uuid, $2 \
                             WHERE EXISTS (SELECT 1 FROM game_character_roots \
                                           WHERE character_id=encode($1, 'hex')::uuid) \
                             ON CONFLICT DO NOTHING",
                        )
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(key)
                        .execute(&mut *tx)
                        .await?;
                        if inserted.rows_affected() > 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness as harness;
    use harness::{Harness, TestResult, debug};

    #[test]
    fn known_spell_postgres_replay_restart_and_atomic_rejection() -> TestResult {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, "known_spell_ledger", true).await?;
            let result: TestResult = async {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let keys = BTreeSet::from(["oteryn:spell.light".to_owned()]);
                let fence = harness::fence(1)?;
                {
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    h.root.acquire_automatic_spells(&authority, &h.node, fence, &keys).await.map_err(debug)?;
                }
                let before: Vec<(String, String)> = sqlx::query_as("SELECT spell_key, learned_at::text FROM game_character_known_spells ORDER BY spell_key").fetch_all(&h.pool).await?;
                let reopened = DurabilityRoot::connect_test_runtime(&h.database.url)?;
                assert!(reopened.maintain_ready_once().await?);
                let authority = reopened.open_character_authority(&seal).await.map_err(debug)?;
                reopened.acquire_automatic_spells(&authority, &h.node, fence, &keys).await.map_err(debug)?;
                reopened.acquire_automatic_spells(&authority, &h.node, fence, &BTreeSet::new()).await.map_err(debug)?;
                let invalid = BTreeSet::from(["new-grant".to_owned(), "trailing-space ".to_owned()]);
                assert!(reopened.acquire_automatic_spells(&authority, &h.node, fence, &invalid).await.is_err());
                let after: Vec<(String, String)> = sqlx::query_as("SELECT spell_key, learned_at::text FROM game_character_known_spells ORDER BY spell_key").fetch_all(&h.pool).await?;
                assert_eq!(after, before);
                let privileges: (bool, bool, bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime', 'game_character_known_spells', 'INSERT'), has_table_privilege('oteryn_game_runtime', 'game_character_known_spells', 'SELECT'), has_table_privilege('oteryn_game_runtime', 'game_character_known_spells', 'DELETE')").fetch_one(&h.pool).await?;
                assert_eq!(privileges, (true, true, false));
                eprintln!("AUTOSPELL PostgreSQL restart, rollback and runtime privileges verified");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    }
}
