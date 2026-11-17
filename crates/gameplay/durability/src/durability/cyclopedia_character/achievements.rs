//! Read rich achievement facts under the existing Character visibility boundary.
use super::*;
use crate::durability::account_achievement::EarnedAchievement;
const FACTS_SQL: &str = "SELECT a.achievement_key,q.earned_at FROM game_character_roots r JOIN game_account_achievements a ON a.account_id=r.account_id JOIN game_account_achievement_grant_requests q ON q.account_id=a.account_id AND q.achievement_key=a.achievement_key AND q.source_kind=a.source_kind AND q.source_event_id=a.source_event_id WHERE r.character_id=encode($1,'hex')::uuid ORDER BY a.achievement_key LIMIT 16385";
impl DurabilityRoot {
    pub async fn read_cyclopedia_achievement_facts(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        viewer_fence: CurrentCharacterItemFence,
        target: CharacterId,
    ) -> Result<(String, Vec<EarnedAchievement>), CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::super::character_progression::assert_item_only_fence(&mut tx, &viewer_fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let allowed: bool = sqlx::query_scalar(VISIBLE_SQL)
                .bind(viewer_fence.character_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !allowed { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let Some(name) = sqlx::query_scalar::<_, String>("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(target.as_bytes().as_slice()).fetch_optional(&mut *tx).await? else { return Ok(Err(CharacterAuthorityError::Rejected)); };
            let rows = sqlx::query(FACTS_SQL).bind(target.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            if rows.len() > 16_384 { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let facts = rows.iter().map(|row| Ok(EarnedAchievement {
                achievement_key: row.try_get("achievement_key")?, earned_at_unix_ms: row.try_get("earned_at")?,
            })).collect::<Result<Vec<_>, sqlx::Error>>()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok((name, facts)))
        })).await?
    }
}
