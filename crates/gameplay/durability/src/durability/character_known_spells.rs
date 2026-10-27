//! Durable, automatically acquired player-spell identities (news 8675).
//!
//! This projection is monotonic: reaching an eligible level/vocation adds a spell, while a
//! later vocation transition never silently removes knowledge already earned. Definitions that
//! still require an explicit learning owner are deliberately excluded by the dispatch planner.

use std::collections::BTreeSet;

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;

impl DurabilityRoot {
    /// Add the dispatch-qualified spell keys and return the complete durable projection.
    pub async fn acquire_automatic_spells(
        &self,
        character: CharacterId,
        eligible: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, DurabilityError> {
        let eligible = eligible.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    for key in eligible {
                        let inserted = sqlx::query(
                            "INSERT INTO game_character_known_spells(character_id, spell_key) \
                             SELECT encode($1, 'hex')::uuid, $2 \
                             WHERE EXISTS (SELECT 1 FROM game_character_roots \
                                           WHERE character_id=encode($1, 'hex')::uuid) \
                             ON CONFLICT DO NOTHING",
                        )
                        .bind(character.as_bytes().as_slice())
                        .bind(key)
                        .execute(&mut *tx)
                        .await?;
                        if inserted.rows_affected() > 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let keys: Vec<String> = sqlx::query_scalar(
                        "SELECT spell_key FROM game_character_known_spells \
                         WHERE character_id=encode($1, 'hex')::uuid ORDER BY spell_key",
                    )
                    .bind(character.as_bytes().as_slice())
                    .fetch_all(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(keys.into_iter().collect())
                })
            })
            .await
    }
}
