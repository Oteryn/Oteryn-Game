//! CHARINFO durable title facts. Writes are fenced by the existing reconciled
//! Character authority and never copy the native facts used for qualification.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::CharacterId;
use oteryn_protocol_oteryn::character_info::PageRequest;
use oteryn_protocol_oteryn::character_info::{
    CharacterTitle, TRACKED_CREATURES, TitleFacts, TitleLifetime, newly_eligible,
    qualified_selection, qualify, valid_at,
};
use sqlx::Row;

#[path = "character_title/charm.rs"]
mod charm;
#[path = "character_title/completion.rs"]
mod completion;
#[path = "character_title/cosmetics.rs"]
mod cosmetics;

struct Sources {
    charm: Option<&'static charm::Source>,
    completion: Option<&'static completion::Source>,
    cosmetics: Option<cosmetics::Source>,
}

impl Sources {
    fn new(cosmetics: Option<NativeCosmeticsView>) -> Self {
        Self {
            charm: charm::source(),
            completion: completion::source(),
            cosmetics: cosmetics.map(cosmetics::Source::new),
        }
    }
}

/// Only the fenced quest transaction calls this after persisting the Eternity flag.
pub(super) async fn grant_pensive_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<(), super::DurabilityError> {
    sqlx::query("INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms) SELECT encode($1,'hex')::uuid,$2,floor(extract(epoch FROM statement_timestamp())*1000)::bigint WHERE EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$3 AND value=1) ON CONFLICT DO NOTHING")
        .bind(character.as_bytes().as_slice()).bind(i16::from(CharacterTitle::PensiveWanderer.id()))
        .bind(format!("{}eternity", oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX))
        .execute(&mut **tx).await?;
    Ok(())
}

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

/// One read of every native owner a title rule names (`TitleRule`). Absent owners count as
/// zero/false. Highscore leads come from the published World snapshot (0129), filter 0, rank 1.
async fn read_title_facts(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    now_unix_ms: i64,
    sources: &Sources,
) -> Result<TitleFacts, super::DurabilityError> {
    // Bestiary/Charm writes serialize on this root through assert_gameplay_fence. Hold a
    // shared lock until the caller's grant commits, so spending cannot invalidate Beastly's
    // observed available balance between qualification and insertion.
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let row = sqlx::query(TITLE_FACTS_SQL)
        .bind(character.as_bytes().as_slice())
        .bind(TRACKED_CREATURES.as_slice())
        .bind(now_unix_ms)
        .bind(format!(
            "{}eternity",
            oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX
        ))
        .fetch_one(&mut **tx)
        .await?;
    let count = |value: i64| u32::try_from(value.max(0)).unwrap_or(u32::MAX);
    let leads: Vec<i32> = row.try_get("world_leads")?;
    let bosses: Vec<i64> = row.try_get("bosses_unlocked")?;
    let kills: Vec<i64> = row.try_get("tracked_kills")?;
    let mut tracked_kills = [0; TRACKED_CREATURES.len()];
    for (slot, value) in tracked_kills.iter_mut().zip(kills) {
        *slot = count(value);
    }
    let mut bosses_unlocked = [0; 3];
    for (slot, value) in bosses_unlocked.iter_mut().zip(bosses) {
        *slot = count(value);
    }
    let available_charm_points = match sources.charm {
        Some(source) => source.available(tx, character).await?,
        None => 0,
    };
    let cosmetics = match &sources.cosmetics {
        Some(source) => source.read(tx, character).await?,
        None => cosmetics::Facts::default(),
    };
    let completion = match sources.completion {
        Some(source) => source.read(tx, character).await?,
        None => completion::Facts::default(),
    };
    Ok(TitleFacts {
        monk_pilgrimage: row.try_get("monk_pilgrimage")?,
        weapon_mastery: row.try_get("weapon_mastery")?,
        all_map_areas_discovered: row.try_get("map_discovered")?,
        bank_balance: row.try_get("bank_balance")?,
        level: count(row.try_get("level")?),
        world_leads: leads
            .into_iter()
            .filter(|category| (0..16).contains(category))
            .fold(0, |bits, category| bits | (1 << category)),
        login_streak: count(row.try_get::<i32, _>("login_streak")?.into()),
        guild_leader: row.try_get("guild_leader")?,
        boss_points: row.try_get("boss_points")?,
        bosses_unlocked,
        tracked_kills,
        character_decade: row.try_get("character_decade")?,
        available_charm_points,
        mounts_unlocked: cosmetics.mounts,
        full_outfits: cosmetics.full_outfits,
        outfits_unlocked: cosmetics.outfits,
        bestiary_classes_complete: completion.bestiary_classes,
        all_bestiary_complete: completion.all_bestiary,
        boss_categories_complete: completion.boss_categories,
        all_bosses_complete: completion.all_bosses,
        ..TitleFacts::default()
    })
}

// Collapse immutable kill history once per read. A correlated LIMIT 1 per boss/category
// rescanned receipts without a (Character, boss) index and exhausted the semantic pass.
const TITLE_FACTS_SQL: &str = "WITH me AS (SELECT character_id,account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid), \
 boss_categories AS MATERIALIZED (SELECT DISTINCT ON (boss_key) boss_key,category FROM game_character_bosstiary_kill_receipts WHERE character_id=encode($1,'hex')::uuid) SELECT \
 COALESCE((SELECT greatest(sword_level,axe_level,club_level,distance_level)>=100 FROM game_character_build_state b WHERE b.character_id=me.character_id),false) AS weapon_mastery, \
 COALESCE((SELECT (progress->>'discovered')::boolean FROM game_character_cyclopedia c WHERE c.character_id=me.character_id),false) AS map_discovered, \
 EXISTS(SELECT 1 FROM game_character_quest_tracks q WHERE q.character_id=me.character_id AND q.track_key=$4 AND q.value=1) AS monk_pilgrimage, \
 COALESCE((SELECT balance FROM game_account_bank_balances b WHERE b.account_id=me.account_id AND b.world_id=me.world_id),0)::bigint AS bank_balance, \
 COALESCE((SELECT level FROM game_character_progression_state p WHERE p.character_id=me.character_id),0)::bigint AS level, \
 ARRAY(SELECT h.category FROM game_world_highscore_heads h JOIN game_world_highscore_rows w ON w.snapshot_id=h.current_snapshot_id AND w.filter=0 AND w.position=1 WHERE h.world_id=me.world_id AND w.character_id=me.character_id ORDER BY h.category) AS world_leads, \
 COALESCE((SELECT max(streak) FROM game_character_daily_rewards d WHERE d.character_id=me.character_id),0)::integer AS login_streak, \
 EXISTS(SELECT 1 FROM game_guild_members g WHERE g.character_id=me.character_id AND g.rank_level=1) AS guild_leader, \
 COALESCE((SELECT sum(boss_points) FROM game_character_bosstiary_progress p WHERE p.character_id=me.character_id),0)::bigint AS boss_points, \
 ARRAY(SELECT (SELECT count(*) FROM game_character_bosstiary_progress p JOIN boss_categories k USING(boss_key) WHERE p.character_id=me.character_id AND p.boss_points>0 AND k.category=c.category) \
   FROM unnest(ARRAY['bane','archfoe','nemesis']) WITH ORDINALITY c(category,n) ORDER BY c.n) AS bosses_unlocked, \
 ARRAY(SELECT greatest(COALESCE((SELECT kill_count FROM game_character_bestiary_progress b WHERE b.character_id=me.character_id AND b.race_key=t.key),0), \
   COALESCE((SELECT kill_count FROM game_character_bosstiary_progress p WHERE p.character_id=me.character_id AND p.boss_key=t.key),0)) \
   FROM unnest($2::text[]) WITH ORDINALITY t(key,n) ORDER BY t.n) AS tracked_kills, \
 COALESCE((SELECT to_timestamp(min(occurred_at)/1000.0)+interval '10 years'<=to_timestamp($3/1000.0) FROM game_character_creation_receipts r WHERE r.character_id=me.character_id),false) AS character_decade \
 FROM me";

impl DurabilityRoot {
    /// PAR-P12a automatic unlock (Crystal `PlayerTitle::checkAndUpdateNewTitles`,
    /// `player_title.cpp:123`): in one fenced transaction, grant every newly eligible title
    /// from the native facts, clear a selection that no longer qualifies and revoke the
    /// non-permanent titles whose predicate fails. Returns the titles granted now.
    pub async fn refresh_character_titles(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        now_unix_ms: i64,
        cosmetics: Option<NativeCosmeticsView>,
    ) -> Result<Vec<CharacterTitle>, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let character = fence.character_id;
        let sources = Sources::new(cosmetics);
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let facts = read_title_facts(&mut tx, character, now_unix_ms, &sources).await?;
            let rows: Vec<i16> = sqlx::query_scalar("SELECT title FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND (expires_at_unix_ms IS NULL OR expires_at_unix_ms>$2) ORDER BY title FOR UPDATE")
                .bind(character.as_bytes().as_slice()).bind(now_unix_ms).fetch_all(&mut *tx).await?;
            let mut earned = Vec::with_capacity(rows.len());
            for raw in rows {
                match decode_title(raw) { Ok(title) => earned.push(title), Err(error) => return Ok(Err(error)) }
            }
            let granted = newly_eligible(facts, &earned, now_unix_ms);
            for title in &granted {
                let expiry = match qualify(*title, facts).lifetime { TitleLifetime::Until(value) => Some(value), _ => None };
                sqlx::query("INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms,expires_at_unix_ms) VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING")
                    .bind(character.as_bytes().as_slice()).bind(i16::from(title.id())).bind(now_unix_ms).bind(expiry)
                    .execute(&mut *tx).await?;
            }
            let revoked = revoked_conditional_titles(facts);
            if !revoked.is_empty() {
                // The selected reference is cleared first, so the revocation keeps the FK.
                sqlx::query("UPDATE game_character_title_selection SET title=NULL,revision=revision+1 WHERE character_id=encode($1,'hex')::uuid AND title=ANY($2)")
                    .bind(character.as_bytes().as_slice()).bind(&revoked).execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=ANY($2)")
                    .bind(character.as_bytes().as_slice()).bind(&revoked).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(granted))
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
    /// Reads native qualification and earned state in the same fenced transaction,
    /// then clears a selection and conditional award which no longer qualify.
    /// A page read cannot revoke a newer grant using an earlier facts snapshot.
    pub async fn read_and_requalify_character_titles(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        character: CharacterId,
        now_unix_ms: i64,
        cosmetics: Option<NativeCosmeticsView>,
    ) -> Result<CharacterTitleState, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let sources = Sources::new(cosmetics);
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let facts = read_title_facts(&mut tx, character, now_unix_ms, &sources).await?;
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

    /// Live command seam: qualification and its durable grant share the current session fence
    /// and transaction. In particular, Charm Points cannot be spent between a separate
    /// qualification read and this write. Content is prepared before entering the transaction.
    pub async fn qualify_and_unlock_character_title(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        title: CharacterTitle,
        now_unix_ms: i64,
        cosmetics: Option<NativeCosmeticsView>,
    ) -> Result<bool, CharacterAuthorityError> {
        if now_unix_ms < 0 {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let sources = Sources::new(cosmetics);
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let facts = read_title_facts(&mut tx, fence.character_id, now_unix_ms, &sources).await?;
            let qualification = qualify(title, facts);
            if !valid_at(qualification, now_unix_ms) {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let expiry = match qualification.lifetime {
                TitleLifetime::Until(value) => Some(value),
                _ => None,
            };
            let changed = sqlx::query("INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms,expires_at_unix_ms) VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING")
                .bind(fence.character_id.as_bytes().as_slice()).bind(i16::from(title.id()))
                .bind(now_unix_ms).bind(expiry).execute(&mut *tx).await?.rows_affected() == 1;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(changed))
        })).await?
    }

    /// Trusted internal grant seam. Live client commands use
    /// [`Self::qualify_and_unlock_character_title`] so a request cannot supply qualification.
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
            (5, CharacterTitle::PensiveWanderer),
        ] {
            assert!(matches!(decode_title(stored), Ok(decoded) if decoded == title));
        }
        // P12a appended titles widen the valid catalogue; the first unknown id follows it.
        for invalid in [
            0,
            oteryn_protocol_oteryn::character_info::TITLE_COUNT as i16 + 1,
            -1,
            256,
        ] {
            assert!(matches!(
                decode_title(invalid),
                Err(CharacterAuthorityError::Rejected)
            ));
        }
        // Q2=B appends the 26 class/category completion titles as conditional IDs 59–84.
        let mut revoked = vec![
            2, 3, 7, 19, 20, 21, 23, 24, 25, 26, 27, 29, 30, 31, 32, 33, 34, 35, 36, 38, 39, 41, 42,
        ];
        revoked.extend(59..=84);
        assert_eq!(revoked_conditional_titles(TitleFacts::default()), revoked);
        let all = TitleFacts {
            monk_pilgrimage: true,
            weapon_mastery: true,
            all_grade_three_mods: true,
            all_map_areas_discovered: true,
            drome_rota_top_five: true,
            next_rotation_unix_ms: Some(1),
            bank_balance: 100_000_000,
            level: 1000,
            world_leads: u16::MAX,
            guild_leader: true,
            bestiary_classes_complete: [true;
                oteryn_protocol_oteryn::character_info::BestiaryClass::COUNT],
            all_bestiary_complete: true,
            boss_categories_complete: [true; 3],
            all_bosses_complete: true,
            ..TitleFacts::default()
        };
        assert!(revoked_conditional_titles(all).is_empty());
        // A level-50 character keeps Trolltrasher and loses every higher level title.
        let low = TitleFacts {
            level: 50,
            ..TitleFacts::default()
        };
        assert!(!revoked_conditional_titles(low).contains(&42));
        assert!(revoked_conditional_titles(low).contains(&20));
    }
}
