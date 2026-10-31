//! PAR-P12c Cyclopedia character tabs: a read-only projection of the native owners. A foreign
//! target is disclosed under the same VIP consent rule as `read_character_info`.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::domain::CharacterId;
use oteryn_protocol_oteryn::character_info::{HISTORY_WINDOW_MS, PageRequest};
use oteryn_protocol_oteryn::cyclopedia_character::{
    CharacterTab, EquippedItem, GeneralStats, KillStatus, StoreSummary, TabView, death_cause,
    pvp_kill_description,
};
use sqlx::Row;

const VISIBLE_SQL: &str = "SELECT $1=$2 OR EXISTS(SELECT 1 FROM game_vip_consents c JOIN game_character_roots v ON v.account_id=c.account_id AND v.world_id=c.world_id WHERE v.character_id=encode($1,'hex')::uuid AND c.character_id=encode($2,'hex')::uuid AND c.consent=2)";

const GENERAL_STATS_SQL: &str = "SELECT p.level,p.total_experience,b.magic_level,b.fist_level,b.club_level,b.sword_level,b.axe_level,b.distance_level,b.shielding_level,b.fishing_level,b.stamina_minutes,b.offline_pool_s,l.health,l.mana \
 FROM game_character_progression_state p JOIN game_character_build_state b ON b.character_id=p.character_id \
 LEFT JOIN game_character_last_positions l ON l.character_id=p.character_id WHERE p.character_id=encode($1,'hex')::uuid";

/// Deaths in the history window, newest first, with the Character killers the PvP owners
/// recorded for the same death (unjustified points and guild war frags).
const DEATHS_SQL: &str = "SELECT d.committed_at,d.level_before,ARRAY(SELECT r.name FROM (SELECT u.character_id AS killer FROM game_character_unjustified_points u WHERE u.death_occurrence_id=d.death_occurrence_id \
   UNION SELECT f.killer_character_id FROM game_guild_war_frags f WHERE f.death_occurrence_id=d.death_occurrence_id) k JOIN game_character_roots r ON r.character_id=k.killer ORDER BY r.name) AS killers \
 FROM game_character_death_receipts d WHERE d.character_id=encode($1,'hex')::uuid AND d.committed_at BETWEEN $2 AND $3 ORDER BY d.committed_at DESC,d.death_occurrence_id DESC OFFSET $4 LIMIT $5";

/// PvP kills in the history window, newest first. A guild war frag wins over unjustified
/// points for the same death (Crystal reports one status per kill).
const KILLS_SQL: &str = "SELECT k.at,k.status,r.name FROM (SELECT DISTINCT ON (death) death,at,status,victim FROM ( \
   SELECT f.death_occurrence_id AS death,f.occurred_at AS at,2::smallint AS status,f.victim_character_id AS victim FROM game_guild_war_frags f WHERE f.killer_character_id=encode($1,'hex')::uuid \
   UNION ALL SELECT u.death_occurrence_id,floor(extract(epoch FROM u.committed_at)*1000)::bigint,1::smallint,u.victim_character_id FROM game_character_unjustified_points u WHERE u.character_id=encode($1,'hex')::uuid \
 ) a ORDER BY death,status DESC) k JOIN game_character_roots r ON r.character_id=k.victim \
 WHERE k.at BETWEEN $2 AND $3 ORDER BY k.at DESC,k.death DESC OFFSET $4 LIMIT $5";

const ACHIEVEMENTS_SQL: &str = "SELECT (SELECT count(*) FROM game_account_achievements a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid) AS total, \
 ARRAY(SELECT a.achievement_key FROM game_account_achievements a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid ORDER BY a.achievement_key OFFSET $2 LIMIT $3) AS keys";

const APPEARANCES_SQL: &str = "SELECT (SELECT count(*) FROM game_account_appearance_unlocks a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid) AS total, \
 ARRAY(SELECT a.unlock_key FROM game_account_appearance_unlocks a JOIN game_character_roots r ON r.account_id=a.account_id WHERE r.character_id=encode($1,'hex')::uuid ORDER BY a.unlock_key OFFSET $2 LIMIT $3) AS keys";

const STORE_SUMMARY_SQL: &str = "WITH me AS (SELECT account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) SELECT \
 ARRAY(SELECT blessing_key FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY blessing_key) AS blessings, \
 (SELECT count(*) FROM game_account_prey_unlocks u JOIN me ON me.account_id=u.account_id AND me.world_id=u.world_id) AS prey_slots, \
 COALESCE((SELECT w.balance FROM game_account_prey_wildcards w JOIN me ON me.account_id=w.account_id AND me.world_id=w.world_id),0)::bigint AS wildcards, \
 (SELECT count(*) FROM game_house_hirelings WHERE owner_character_id=encode($1,'hex')::uuid) AS hirelings";

const INSPECTION_SQL: &str = "SELECT s.slot,i.definition_production_key,i.quantity FROM game_character_equipment_slots s JOIN game_item_instances i ON i.item_instance_id=s.item_instance_id WHERE s.character_id=encode($1,'hex')::uuid ORDER BY s.slot";

fn narrow<T: TryFrom<i64>>(value: i64) -> Result<T, sqlx::Error> {
    T::try_from(value)
        .map_err(|_| sqlx::Error::Decode("cyclopedia value out of range".into()))
}

impl DurabilityRoot {
    /// PAR-P12c (Crystal `ProtocolGame::sendCyclopediaCharacter*`, `protocolgame.cpp:4372-4817`):
    /// one tab of `target`'s character window as `viewer` may see it.
    pub async fn read_cyclopedia_character_tab(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        viewer: CharacterId,
        target: CharacterId,
        tab: CharacterTab,
        page: PageRequest,
        now_unix_ms: i64,
    ) -> Result<TabView, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let allowed: bool = sqlx::query_scalar(VISIBLE_SQL)
                .bind(viewer.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !allowed { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let id = target.as_bytes().to_vec();
            let window = now_unix_ms.saturating_sub(HISTORY_WINDOW_MS);
            let (offset, limit) = (i64::from(page.offset), i64::from(page.limit));
            let view = match tab {
                CharacterTab::GeneralStats => {
                    let Some(row) = sqlx::query(GENERAL_STATS_SQL).bind(&id).fetch_optional(&mut *tx).await? else {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    };
                    let mut skills = [0; 7];
                    for (slot, field) in skills.iter_mut().zip(["fist_level", "club_level", "sword_level", "axe_level", "distance_level", "shielding_level", "fishing_level"]) {
                        *slot = narrow(i64::from(row.try_get::<i32, _>(field)?))?;
                    }
                    let vitals = match (row.try_get::<Option<i32>, _>("health")?, row.try_get::<Option<i32>, _>("mana")?) {
                        (Some(health), Some(mana)) => Some((narrow(i64::from(health))?, narrow(i64::from(mana))?)),
                        _ => None,
                    };
                    TabView::GeneralStats(GeneralStats {
                        experience: narrow(row.try_get::<i64, _>("total_experience")?)?,
                        level: narrow(row.try_get::<i64, _>("level")?)?,
                        magic_level: narrow(i64::from(row.try_get::<i32, _>("magic_level")?))?,
                        skills,
                        stamina_minutes: narrow(i64::from(row.try_get::<i32, _>("stamina_minutes")?))?,
                        offline_training_minutes: narrow(i64::from(row.try_get::<i32, _>("offline_pool_s")?) / 60)?,
                        last_vitals: vitals,
                    })
                }
                CharacterTab::RecentDeaths => {
                    let rows = sqlx::query(DEATHS_SQL).bind(&id).bind(window).bind(now_unix_ms).bind(offset).bind(limit).fetch_all(&mut *tx).await?;
                    let mut entries = Vec::with_capacity(rows.len());
                    for row in rows {
                        let level: i64 = row.try_get("level_before")?;
                        let killers: Vec<String> = row.try_get("killers")?;
                        entries.push((row.try_get("committed_at")?, death_cause(narrow(level)?, &killers)));
                    }
                    TabView::RecentDeaths(entries)
                }
                CharacterTab::RecentPvpKills => {
                    let rows = sqlx::query(KILLS_SQL).bind(&id).bind(window).bind(now_unix_ms).bind(offset).bind(limit).fetch_all(&mut *tx).await?;
                    let mut entries = Vec::with_capacity(rows.len());
                    for row in rows {
                        let status = if row.try_get::<i16, _>("status")? == 2 { KillStatus::GuildWar } else { KillStatus::Unjustified };
                        let victim: String = row.try_get("name")?;
                        entries.push((row.try_get("at")?, status, pvp_kill_description(&victim)));
                    }
                    TabView::RecentPvpKills(entries)
                }
                CharacterTab::Achievements | CharacterTab::Appearances => {
                    let sql = if tab == CharacterTab::Achievements { ACHIEVEMENTS_SQL } else { APPEARANCES_SQL };
                    let row = sqlx::query(sql).bind(&id).bind(offset).bind(limit).fetch_one(&mut *tx).await?;
                    let total = narrow(row.try_get::<i64, _>("total")?)?;
                    let keys = row.try_get("keys")?;
                    if tab == CharacterTab::Achievements { TabView::Achievements { total, keys } } else { TabView::Appearances { total, keys } }
                }
                CharacterTab::StoreSummary => {
                    let row = sqlx::query(STORE_SUMMARY_SQL).bind(&id).fetch_one(&mut *tx).await?;
                    TabView::StoreSummary(StoreSummary {
                        blessings: row.try_get("blessings")?,
                        permanent_prey_slots: narrow(row.try_get::<i64, _>("prey_slots")?)?,
                        prey_wildcards: narrow(row.try_get::<i64, _>("wildcards")?)?,
                        hirelings: narrow(row.try_get::<i64, _>("hirelings")?)?,
                    })
                }
                CharacterTab::Inspection => {
                    let rows = sqlx::query(INSPECTION_SQL).bind(&id).fetch_all(&mut *tx).await?;
                    let mut items = Vec::with_capacity(rows.len());
                    for row in rows {
                        items.push(EquippedItem {
                            slot: narrow(i64::from(row.try_get::<i16, _>("slot")?))?,
                            definition: row.try_get("definition_production_key")?,
                            quantity: narrow(row.try_get::<i64, _>("quantity")?)?,
                        });
                    }
                    TabView::Inspection(items)
                }
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(view))
        })).await?
    }
}
