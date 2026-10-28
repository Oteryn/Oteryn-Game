//! CHARINFO durable title facts. Writes are fenced by the existing reconciled
//! Character authority and never copy the native facts used for qualification.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::CharacterId;
use oteryn_protocol_oteryn::character_info::PageRequest;
use oteryn_protocol_oteryn::character_info::{
    CharacterTitle, TitleFacts, TitleLifetime, qualified_selection, qualify,
};
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTitleState {
    pub earned: Vec<(CharacterTitle, Option<i64>)>,
    pub selected: Option<CharacterTitle>,
    pub revision: u64,
}

fn decode_title(value: i16) -> Result<CharacterTitle, CharacterAuthorityError> {
    u8::try_from(value)
        .ok()
        .and_then(CharacterTitle::from_id)
        .ok_or(CharacterAuthorityError::Rejected)
}

/// Stored ids of the conditional titles whose predicate no longer holds. Global revokes these
/// titles (Adept Enhancer, Globetrotter) instead of keeping them earned.
fn revoked_conditional_titles(facts: TitleFacts) -> Vec<i16> {
    CharacterTitle::ALL
        .into_iter()
        .filter(|title| {
            let qualification = qualify(*title, facts);
            qualification.lifetime == TitleLifetime::Conditional && !qualification.eligible
        })
        .map(|title| i16::from(title.id()))
        .collect()
}

impl DurabilityRoot {
    /// Derives title predicates from their existing durable owners. No title
    /// qualification is copied into the title tables.
    pub async fn character_title_facts(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<TitleFacts, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT COALESCE((SELECT greatest(sword_level,axe_level,club_level,distance_level)>=100 FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid),false) AS weapon_mastery, COALESCE((SELECT (progress->>'discovered')::boolean FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid),false) AS map_discovered")
                .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(TitleFacts { weapon_mastery: row.try_get("weapon_mastery")?, all_map_areas_discovered: row.try_get("map_discovered")?, ..TitleFacts::default() }))
        })).await?
    }

    /// Bounded native projection. A foreign target is disclosed only when the
    /// existing VIP/Friends owner records accepted consent for this viewer.
    pub async fn read_character_info(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        viewer: CharacterId,
        target: CharacterId,
        page: PageRequest,
        now_unix_ms: i64,
    ) -> Result<Vec<u8>, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let allowed: bool = sqlx::query_scalar("SELECT $1=$2 OR EXISTS(SELECT 1 FROM game_vip_consents c JOIN game_character_roots v ON v.account_id=c.account_id AND v.world_id=c.world_id WHERE v.character_id=encode($1,'hex')::uuid AND c.character_id=encode($2,'hex')::uuid AND c.consent=2)")
                .bind(viewer.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !allowed { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let row = sqlx::query("SELECT p.level,p.total_experience,(SELECT count(*) FROM game_account_achievements a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid) AS achievements,(SELECT count(*) FROM game_account_appearance_unlocks a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid) AS appearances,(SELECT count(*) FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid) AS inventory,COALESCE((SELECT sum(committed)::bigint FROM game_character_depot_box_counters WHERE character_id=encode($1,'hex')::uuid),0) AS depot,COALESCE((SELECT committed::bigint FROM game_character_inbox_counters WHERE character_id=encode($1,'hex')::uuid),0) AS inboxes,COALESCE((SELECT sum(quantity)::bigint FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid),0) AS stash FROM game_character_progression_state p WHERE p.character_id=encode($1,'hex')::uuid")
                .bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let deaths = sqlx::query("SELECT committed_at FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid AND committed_at BETWEEN $2 AND $3 ORDER BY committed_at DESC OFFSET $4 LIMIT $5")
                .bind(target.as_bytes().as_slice()).bind(now_unix_ms.saturating_sub(oteryn_protocol_oteryn::character_info::HISTORY_WINDOW_MS)).bind(now_unix_ms).bind(i64::from(page.offset)).bind(i64::from(page.limit)).fetch_all(&mut *tx).await?;
            let kills = sqlx::query("SELECT floor(extract(epoch FROM committed_at)*1000)::bigint AS committed_at FROM game_character_unjustified_points WHERE character_id=encode($1,'hex')::uuid AND committed_at BETWEEN to_timestamp($2::double precision/1000) AND to_timestamp($3::double precision/1000) ORDER BY committed_at DESC OFFSET $4 LIMIT $5")
                .bind(target.as_bytes().as_slice()).bind(now_unix_ms.saturating_sub(oteryn_protocol_oteryn::character_info::HISTORY_WINDOW_MS)).bind(now_unix_ms).bind(i64::from(page.offset)).bind(i64::from(page.limit)).fetch_all(&mut *tx).await?;
            use oteryn_protocol_oteryn::character_info::{CharacterInfoPage, CharacterSummary, HistoryEntry, ItemSummary};
            let count = |field: &str| -> Result<u32, sqlx::Error> {
                u32::try_from(row.try_get::<i64, _>(field)?)
                    .map_err(|error| sqlx::Error::Decode(Box::new(error)))
            };
            let history = |rows: &[sqlx::postgres::PgRow]| -> Result<Vec<HistoryEntry>, sqlx::Error> {
                rows.iter().map(|row| Ok(HistoryEntry {
                    occurred_at_unix_ms: row.try_get("committed_at")?,
                    counterpart: None,
                })).collect()
            };
            let projection = CharacterInfoPage {
                summary: CharacterSummary {
                    level: count("level")?,
                    experience: u64::try_from(row.try_get::<i64, _>("total_experience")?)
                        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?,
                    achievement_points: count("achievements")?,
                    appearance_count: u16::try_from(count("appearances")?)
                        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?,
                    items: ItemSummary {
                        inventory: count("inventory")?, depot: count("depot")?,
                        stash: count("stash")?, inboxes: count("inboxes")?,
                    },
                },
                deaths: history(&deaths)?, pvp_kills: history(&kills)?,
                earned_titles: Vec::new(), selected_title: None, next_offset: None,
            };
            if projection.bounded(page, now_unix_ms).is_none() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let mut bytes = vec![0];
            bytes.extend_from_slice(&row.try_get::<i64,_>("level")?.to_le_bytes());
            bytes.extend_from_slice(&row.try_get::<i64,_>("total_experience")?.to_le_bytes());
            for field in ["achievements", "appearances", "inventory", "depot", "stash", "inboxes"] { bytes.extend_from_slice(&row.try_get::<i64,_>(field)?.to_le_bytes()); }
            bytes.extend_from_slice(&(deaths.len() as u16).to_le_bytes());
            bytes.extend_from_slice(&(kills.len() as u16).to_le_bytes());
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(bytes))
        })).await?
    }
    /// Reads earned state and atomically clears a selection which no longer
    /// qualifies. The supplied facts are observations from the native owners,
    /// not persisted copies of their truth.
    pub async fn read_and_requalify_character_titles(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        character: CharacterId,
        facts: TitleFacts,
        now_unix_ms: i64,
    ) -> Result<CharacterTitleState, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let rows = sqlx::query("SELECT title,expires_at_unix_ms FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND (expires_at_unix_ms IS NULL OR expires_at_unix_ms>$2) ORDER BY title FOR UPDATE")
                .bind(character.as_bytes().as_slice()).bind(now_unix_ms).fetch_all(&mut *tx).await?;
            let revoked = revoked_conditional_titles(facts);
            let mut earned = Vec::with_capacity(rows.len());
            for row in rows {
                let title = match decode_title(row.try_get("title")?) { Ok(title) => title, Err(error) => return Ok(Err(error)) };
                if !revoked.contains(&i16::from(title.id())) {
                    earned.push((title, row.try_get("expires_at_unix_ms")?));
                }
            }
            let selection = sqlx::query("SELECT title,revision FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let (selected, mut revision) = match selection {
                Some(row) => {
                    let selected = match row.try_get::<Option<i16>, _>("title")? { Some(raw) => match decode_title(raw) { Ok(title) => Some(title), Err(error) => return Ok(Err(error)) }, None => None };
                    let revision = match u64::try_from(row.try_get::<i64, _>("revision")?) { Ok(value) => value, Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)) };
                    (selected, revision)
                }
                None => (None, 0),
            };
            let qualified = qualified_selection(selected, facts, now_unix_ms)
                .filter(|title| earned.iter().any(|(earned, _)| earned == title));
            if selected.is_some() && qualified.is_none() {
                let next: i64 = sqlx::query_scalar("UPDATE game_character_title_selection SET title=NULL,revision=revision+1 WHERE character_id=encode($1,'hex')::uuid RETURNING revision")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                revision = match u64::try_from(next) { Ok(value) => value, Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)) };
            }
            // The selected reference is cleared first, then conditional earned state
            // is revoked in the same locked transaction (Global's manage(false)).
            if !revoked.is_empty() {
                sqlx::query("DELETE FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=ANY($2)")
                    .bind(character.as_bytes().as_slice()).bind(&revoked).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(CharacterTitleState { earned, selected: qualified, revision }))
        })).await?
    }

    pub async fn unlock_character_title(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        title: CharacterTitle,
        earned_at_unix_ms: i64,
        expires_at_unix_ms: Option<i64>,
    ) -> Result<bool, CharacterAuthorityError> {
        if earned_at_unix_ms < 0
            || expires_at_unix_ms.is_some_and(|expiry| expiry <= earned_at_unix_ms)
        {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let character = fence.character_id;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
                let changed = sqlx::query(
                    "INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms,expires_at_unix_ms) \
                     VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING",
                )
                .bind(character.as_bytes().as_slice())
                .bind(i16::from(title.id()))
                .bind(earned_at_unix_ms)
                .bind(expires_at_unix_ms)
                .execute(&mut *tx)
                .await?
                .rows_affected() == 1;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(changed))
            }))
            .await?
    }

    pub async fn select_character_title(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        title: Option<CharacterTitle>,
    ) -> Result<u64, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let character = fence.character_id;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    if super::character_progression::assert_session_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    }
                    let revision: i64 = sqlx::query_scalar(
                        "INSERT INTO game_character_title_selection(character_id,title,revision) \
                     VALUES(encode($1,'hex')::uuid,$2,1) ON CONFLICT(character_id) DO UPDATE \
                     SET title=EXCLUDED.title,revision=game_character_title_selection.revision+1 \
                     RETURNING revision",
                    )
                    .bind(character.as_bytes().as_slice())
                    .bind(title.map(|value| i16::from(value.id())))
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    let revision = match u64::try_from(revision) {
                        Ok(revision) => revision,
                        Err(_) => return Ok(Err(CharacterAuthorityError::Rejected)),
                    };
                    Ok(Ok(revision))
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PAR-P12: stored ids stay readable after the Global rename, and both Global
    /// non-permanent conditional titles are revoked when their predicate fails.
    #[test]
    fn p12_stored_ids_decode_and_conditional_titles_revoke() {
        for (stored, title) in [
            (1, CharacterTitle::AdeptArmorer),
            (2, CharacterTitle::AdeptEnhancer),
            (3, CharacterTitle::Globetrotter),
            (4, CharacterTitle::ReigningDromeChampion),
        ] {
            assert!(matches!(decode_title(stored), Ok(decoded) if decoded == title));
        }
        for invalid in [0, 5, -1, 256] {
            assert!(matches!(
                decode_title(invalid),
                Err(CharacterAuthorityError::Rejected)
            ));
        }
        assert_eq!(revoked_conditional_titles(TitleFacts::default()), [2, 3]);
        let all = TitleFacts {
            weapon_mastery: true,
            all_grade_three_mods: true,
            all_map_areas_discovered: true,
            drome_rota_top_five: true,
            next_rotation_unix_ms: Some(1),
        };
        assert!(revoked_conditional_titles(all).is_empty());
    }
}
