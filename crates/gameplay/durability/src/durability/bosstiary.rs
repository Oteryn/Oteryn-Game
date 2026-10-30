//! PKG-P4/P02: durable character boss counters, independent of XP and Bestiary caps.
use super::bestiary_progress::BestiaryKillOccurrence;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::bosstiary::{BosstiaryBoss, progress};

impl DurabilityRoot {
    /// Credits a captured kill under the caller's held revision slot. This counter, like
    /// Animus completion, does not advance CharacterRevision. Current session/generation,
    /// root, lease, scope and recovery are re-proved even for a replay. The locked root
    /// serializes concurrent occurrences; the receipt and counter commit atomically.
    pub async fn credit_bosstiary_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: BestiaryKillOccurrence,
        boss: &BosstiaryBoss,
    ) -> Result<bool, CharacterProgressionError> {
        if boss.key.len() > 128
            || !boss
                .key
                .split_once(':')
                .is_some_and(|(ns, key)| !ns.is_empty() && !key.is_empty())
            || !boss
                .key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b":._/-".contains(&c))
            || progress(&boss.category, 0).is_none()
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let boss = boss.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                return Ok(Err(error));
            }
            let stored: Option<(String, String)> = sqlx::query_as(
                "SELECT boss_key,category FROM game_character_bosstiary_kill_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if let Some((key, category)) = stored {
                if key != boss.key || category != boss.category {
                    return Ok(Err(CharacterProgressionError::InvalidInput));
                }
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(false));
            }
            let before: Option<i64> = sqlx::query_scalar(
                "SELECT kill_count FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&boss.key).fetch_optional(&mut *tx).await?;
            let count = before.unwrap_or(0).checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            let points = progress(&boss.category, u64::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?)
                .ok_or(DurabilityError::InvalidStoredState)?.points;
            sqlx::query("INSERT INTO game_character_bosstiary_progress(character_id,boss_key,kill_count,boss_points) VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT(character_id,boss_key) DO UPDATE SET kill_count=EXCLUDED.kill_count,boss_points=EXCLUDED.boss_points")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&boss.key).bind(count).bind(i64::from(points)).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_bosstiary_kill_receipts(character_id,occurrence_id,boss_key,category) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice()).bind(&boss.key).bind(&boss.category).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(true))
        })).await?
    }
}
