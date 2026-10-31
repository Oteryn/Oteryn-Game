//! PAR-P12b durable account badges. Global badges belong to the account and are permanent:
//! the owner only inserts. Writes are fenced by the requesting Character's current session.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::CharacterId;
use oteryn_protocol_oteryn::cyclopedia_character::{Badge, BadgeFacts, newly_earned};
use sqlx::Row;

/// The account facts of `character`'s account. The account age is the time in its UUIDv7 id
/// (minted no earlier than the account), so a badge is never granted early.
const BADGE_FACTS_SQL: &str = "WITH me AS (SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid), \
 chars AS (SELECT p.level, CASE b.vocation WHEN 'elder_druid' THEN 'druid' WHEN 'elite_knight' THEN 'knight' WHEN 'exalted_monk' THEN 'monk' \
   WHEN 'royal_paladin' THEN 'paladin' WHEN 'master_sorcerer' THEN 'sorcerer' ELSE b.vocation END AS base \
   FROM game_character_roots r JOIN me ON me.account_id=r.account_id \
   JOIN game_character_progression_state p ON p.character_id=r.character_id \
   LEFT JOIN game_character_build_state b ON b.character_id=r.character_id) \
 SELECT \
 COALESCE((SELECT extract(year FROM age(to_timestamp($2/1000.0), to_timestamp((('x'||lpad(substr(replace(me.account_id::text,'-',''),1,12),16,'0'))::bit(64)::bigint)/1000.0)))::bigint FROM me),0) AS account_years, \
 COALESCE((SELECT l.loyalty_points FROM game_account_loyalty_projections l JOIN me ON me.account_id=l.account_id),0)::bigint AS loyalty_points, \
 COALESCE((SELECT sum(level) FROM chars),0)::bigint AS level_sum, \
 ARRAY(SELECT COALESCE((SELECT max(level) FROM chars WHERE base=v.key),0)::bigint FROM unnest($3::text[]) WITH ORDINALITY v(key,n) ORDER BY v.n) AS vocation_levels";

fn decode_badge(raw: i16) -> Result<Badge, CharacterAuthorityError> {
    u8::try_from(raw)
        .ok()
        .and_then(Badge::from_id)
        .ok_or(CharacterAuthorityError::Rejected)
}

impl DurabilityRoot {
    /// PAR-P12b (Crystal `PlayerBadge::checkAndUpdateNewBadges`, `player_badge.cpp:63`): in one
    /// fenced transaction, grant every badge the account's native facts meet and return the
    /// account's earned badges in id order.
    pub async fn refresh_account_badges(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        now_unix_ms: i64,
    ) -> Result<Vec<Badge>, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let character: CharacterId = fence.character_id;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let row = sqlx::query(BADGE_FACTS_SQL)
                .bind(character.as_bytes().as_slice())
                .bind(now_unix_ms)
                .bind(oteryn_protocol_oteryn::cyclopedia_character::BaseVocation::KEYS.as_slice())
                .fetch_one(&mut *tx).await?;
            let count = |value: i64| u32::try_from(value.max(0)).unwrap_or(u32::MAX);
            let levels: Vec<i64> = row.try_get("vocation_levels")?;
            let mut vocation_levels = [0; 5];
            for (slot, value) in vocation_levels.iter_mut().zip(levels) {
                *slot = count(value);
            }
            let facts = BadgeFacts {
                account_years: count(row.try_get("account_years")?),
                loyalty_points: row.try_get("loyalty_points")?,
                level_sum: row.try_get("level_sum")?,
                vocation_levels,
            };
            let stored: Vec<i16> = sqlx::query_scalar("SELECT badge FROM game_account_badges WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) ORDER BY badge FOR UPDATE")
                .bind(character.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let mut earned = Vec::with_capacity(stored.len());
            for raw in stored {
                match decode_badge(raw) { Ok(badge) => earned.push(badge), Err(error) => return Ok(Err(error)) }
            }
            for badge in newly_earned(facts, &earned) {
                sqlx::query("INSERT INTO game_account_badges(account_id,badge,earned_at_unix_ms) SELECT account_id,$2,$3 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
                    .bind(character.as_bytes().as_slice()).bind(i16::from(badge.id())).bind(now_unix_ms)
                    .execute(&mut *tx).await?;
                earned.push(badge);
            }
            earned.sort_unstable();
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(earned))
        })).await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p12b_stored_badges_decode_only_admitted_ids() {
        assert!(matches!(decode_badge(1), Ok(Badge::FledgelingHero)));
        assert!(matches!(decode_badge(27), Ok(Badge::EnlightenedStudentGrade3)));
        for invalid in [0, 15, 21, 28, -1, 256] {
            assert!(matches!(
                decode_badge(invalid),
                Err(CharacterAuthorityError::Rejected)
            ));
        }
    }
}
