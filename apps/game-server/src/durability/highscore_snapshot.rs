//! HIGHSCORES0 §§3–6: rebuildable committed-state projections, never gameplay authority.
//!
//! RANK-1: migration 0097 installs the tables; the node's serve loop runs one
//! [`job::HighscoreWorldJob`] per World and the HS-WIRE-1 handler reads pages through
//! [`DurabilityRoot::read_highscore_page`]. A Root pass supplies its real connection and deadline;
//! it does not attest a caller's World or job permission.
#[path = "../highscores/ranking.rs"]
pub(crate) mod ranking;

#[path = "../highscores/job.rs"]
pub(crate) mod job;

#[path = "../highscores/reply.rs"]
pub(crate) mod reply;

use super::account_achievement::EarnedAchievement;
use super::character_build::{DurableBuildState, SKILLS};
use super::charm_state::{BestiaryCharmEntry, BestiaryCharmFacts};
use super::db::commit_semantic_transaction;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CharacterId, WorldId};
use ranking::{RankingRow, ranked_list};
use serde::Serialize;
use sqlx::{Connection, Postgres, Row, Transaction, pool::PoolConnection, postgres::PgRow};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Private persisted filter order, not a new protocol-number allocation.
pub(crate) const FILTERS: [Option<&str>; 6] = [
    None,
    Some("knight"),
    Some("paladin"),
    Some("sorcerer"),
    Some("druid"),
    Some("monk"),
];
const PAGE_ROWS: u64 = 50;
const SNAPSHOT_INTERVAL_MS: i64 = 30 * 60 * 1000;

/// The account achievement point owner (category 10). The composition root implements it with
/// the activated achievement catalogue; this module stays independent of that content owner.
pub(crate) trait AchievementPoints: Send + Sync {
    /// Total points of every earned fact, or `None` when a fact names an unknown key or the sum
    /// overflows. A retired entry counts its owner's value (0).
    fn total_points(&self, earned: &[EarnedAchievement]) -> Option<u32>;
}

/// HIGHSCORES0 §3 category 11 source: the activated per-race Charm point catalogue. CHARM-6 has
/// activated none yet, so this server withholds category 11: the job publishes no category 11
/// snapshot and a query for it is `NOT_SUPPORTED`, never a ranking of fabricated zero points.
pub(crate) fn activated_charm_points() -> Option<Vec<BestiaryCharmEntry>> {
    None
}

/// The highest category this server publishes and answers: 11 with an activated Charm point
/// source, otherwise 10.
pub(crate) fn served_categories() -> u8 {
    if activated_charm_points().is_some() {
        11
    } else {
        10
    }
}

/// Actual owning catalogues from the activated content generation, never caller-provided scores.
#[derive(Clone)]
pub(crate) struct SnapshotCatalogues {
    achievements: Arc<dyn AchievementPoints>,
    pub(crate) bestiary: BTreeMap<String, BestiaryCharmEntry>,
    /// Whether a Charm point source backs category 11; without one it is not published.
    charm_points: bool,
}
impl SnapshotCatalogues {
    /// The serve job's catalogues: `achievements` and the activated Charm point source, if any.
    pub(crate) fn activated(achievements: Arc<dyn AchievementPoints>) -> Result<Self> {
        match activated_charm_points() {
            Some(entries) => Self::new(achievements, entries),
            None => Ok(Self {
                achievements,
                bestiary: BTreeMap::new(),
                charm_points: false,
            }),
        }
    }

    /// Catalogues with the Charm point source `entries`, so category 11 is published.
    pub(crate) fn new(
        achievements: Arc<dyn AchievementPoints>,
        entries: impl IntoIterator<Item = BestiaryCharmEntry>,
    ) -> Result<Self> {
        let entries: Vec<_> = entries.into_iter().collect();
        // The actual Charm source owner validates race keys, duplicate entries and its bound.
        BestiaryCharmFacts::new(entries.clone()).map_err(|_| corrupt())?;
        Ok(Self {
            achievements,
            bestiary: entries
                .into_iter()
                .map(|entry| (entry.race.key().to_owned(), entry))
                .collect(),
            charm_points: true,
        })
    }

    /// The categories one publication writes: 1..=11, or 1..=10 without a Charm point source.
    fn published_categories(&self) -> i32 {
        if self.charm_points { 11 } else { 10 }
    }
}

#[derive(Debug)]
pub(crate) enum SnapshotError {
    NotSupported,
    InvalidQuery,
    SnapshotExpired,
    /// A concurrent publisher won the repeatable-read race. No automatic retry.
    RetryableConflict,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for SnapshotError {
    fn from(value: DurabilityError) -> Self {
        Self::Unavailable(value)
    }
}
impl From<sqlx::Error> for SnapshotError {
    fn from(value: sqlx::Error) -> Self {
        if value
            .as_database_error()
            .and_then(|e| e.code())
            .is_some_and(|c| c == "40001" || c == "40P01")
        {
            Self::RetryableConflict
        } else {
            Self::Unavailable(value.into())
        }
    }
}
type Result<T> = std::result::Result<T, SnapshotError>;
fn corrupt() -> SnapshotError {
    DurabilityError::InvalidStoredState.into()
}

#[derive(Debug)]
pub(crate) struct SnapshotRefresh {
    pub snapshots: Vec<SnapshotInfo>,
    /// Remaining time from this transaction's database clock to the earliest durable expiry.
    pub next_check_after: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotInfo {
    pub snapshot_id: u64,
    pub category: u8,
    pub computed_at_ms: i64,
    /// PostgreSQL MVCC cut, retained as diagnostic DATA, not a current fence.
    pub source_cut: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotRow {
    pub rank: u64,
    pub character_id: CharacterId,
    pub name: String,
    pub vocation: String,
    pub level: u32,
    /// Experience: (total XP,0). Magic/skills: the complete (level,progress) tuple.
    /// The future wire shows the category's primary value; progress breaks ties only.
    pub value: (u64, u64),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotPage {
    pub snapshot: SnapshotInfo,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<SnapshotRow>,
    /// Only when present in this snapshot's capped selected list (§6).
    pub own_row: Option<SnapshotRow>,
}

/// Actual native Root budget and commit handling, with no Character/value write or session
/// authority grant. This cannot use begin_semantic_transaction: its first SELECT has already
/// fixed the transaction's isolation. Only this derived projection starts RR directly.
pub(crate) async fn begin_projection<'a>(
    holder: &'a mut PoolConnection<Postgres>,
    deadline: Instant,
) -> std::result::Result<Transaction<'a, Postgres>, DurabilityError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    let mut tx = tokio::time::timeout(
        remaining,
        (**holder).begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ"),
    )
    .await
    .map_err(|_| DurabilityError::RootPassDeadlineExceeded)??;
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    let millis = u64::try_from(remaining.as_millis().max(1))
        .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?;
    tokio::time::timeout(remaining, sqlx::query(
        "SELECT set_config('transaction_timeout',$1,true),set_config('statement_timeout',$1,true),set_config('lock_timeout',$1,true)"
    ).bind(format!("{millis}ms")).execute(&mut *tx))
        .await.map_err(|_| DurabilityError::RootPassDeadlineExceeded)??;
    Ok(tx)
}

impl DurabilityRoot {
    /// Legacy nine-source projection, retained for the isolated native fixtures.
    /// Production scheduling uses `refresh_highscore_schedule` with both owning catalogues.
    /// Deployment must bind this to the dedicated projection role. No raw admin pool is used.
    #[allow(
        dead_code,
        reason = "the PostgreSQL highscore suites call it; the serve job uses refresh_highscore_schedule"
    )]
    pub(crate) async fn publish_highscore_world(
        &self,
        world: WorldId,
    ) -> Result<Vec<SnapshotInfo>> {
        self.publish_highscore_cut(world, None, false)
            .await
            .map(|refresh| refresh.snapshots)
    }

    /// Periodic publication is due after 30 minutes; a planned reset forces one complete cut.
    /// The composition root supplies the actual reset notification, not client command DATA.
    #[allow(
        dead_code,
        reason = "the PostgreSQL highscore suites call it; the serve job uses refresh_highscore_schedule"
    )]
    pub(crate) async fn refresh_highscore_world(
        &self,
        world: WorldId,
        catalogues: SnapshotCatalogues,
        planned_reset: bool,
    ) -> Result<Vec<SnapshotInfo>> {
        self.refresh_highscore_schedule(world, catalogues, planned_reset)
            .await
            .map(|refresh| refresh.snapshots)
    }

    pub(crate) async fn refresh_highscore_schedule(
        &self,
        world: WorldId,
        catalogues: SnapshotCatalogues,
        planned_reset: bool,
    ) -> Result<SnapshotRefresh> {
        self.publish_highscore_cut(world, Some(catalogues), !planned_reset)
            .await
    }

    async fn publish_highscore_cut(
        &self,
        world: WorldId,
        catalogues: Option<SnapshotCatalogues>,
        only_if_due: bool,
    ) -> Result<SnapshotRefresh> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_projection(holder, deadline).await?;
                    match publish_cut(&mut tx, world, catalogues.as_ref(), only_if_due).await {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(SnapshotError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Caller supplies the session's independently resolved World/Character. These arguments
    /// are query DATA, not permission proofs; transport admission belongs to HS-WIRE-1.
    pub(crate) async fn read_highscore_page(
        &self,
        world: WorldId,
        category: u8,
        vocation: Option<&str>,
        page: u32,
        snapshot_id: u64,
        character: CharacterId,
    ) -> Result<SnapshotPage> {
        if !(1..=11).contains(&category) {
            return Err(SnapshotError::NotSupported);
        }
        if page == 0 || (page > 1 && snapshot_id == 0) {
            return Err(SnapshotError::InvalidQuery);
        }
        let filter = FILTERS
            .iter()
            .position(|f| *f == vocation)
            .ok_or(SnapshotError::InvalidQuery)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_projection(holder, deadline).await?;
                    match read_in_transaction(
                        &mut tx,
                        world,
                        category,
                        filter,
                        page,
                        snapshot_id,
                        character,
                    )
                    .await
                    {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(SnapshotError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

pub(crate) struct Source {
    character_id: CharacterId,
    name: String,
    vocation: String,
    level: u32,
    values: [(u64, u64); 9],
}
fn nonnegative(row: &PgRow, column: &str) -> Result<u64> {
    u64::try_from(row.try_get::<i64, _>(column)?).map_err(|_| corrupt())
}
fn character(row: &PgRow) -> Result<CharacterId> {
    let bytes: Vec<u8> = row.try_get("character_id")?;
    CharacterId::decode(&bytes).map_err(|_| corrupt())
}

pub(crate) async fn read_source(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Vec<Source>> {
    // No row means native vocation `none`, excluded. Do not silently drop an eligible
    // non-none build whose required XP row is missing: fail and retain the prior generation.
    let rows = sqlx::query("SELECT uuid_send(r.character_id) AS character_id,r.name,p.level,p.total_experience,b.vocation,b.magic_level,b.mana_spent,b.fist_level,b.fist_tries,b.club_level,b.club_tries,b.sword_level,b.sword_tries,b.axe_level,b.axe_tries,b.distance_level,b.distance_tries,b.shielding_level,b.shielding_tries,b.fishing_level,b.fishing_tries FROM game_character_roots r JOIN game_character_build_state b USING(character_id) LEFT JOIN game_character_progression_state p USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND b.vocation<>'none'")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    rows.into_iter()
        .map(|row| {
            let mut skills = [(0, 0); 7];
            for (index, skill) in SKILLS.iter().enumerate() {
                let level: i32 = row.try_get(format!("{skill}_level").as_str())?;
                skills[index] = (
                    u16::try_from(level).map_err(|_| corrupt())?,
                    nonnegative(&row, format!("{skill}_tries").as_str())?,
                );
            }
            let build = DurableBuildState::new(
                row.try_get::<String, _>("vocation")?,
                (
                    u16::try_from(row.try_get::<i32, _>("magic_level")?).map_err(|_| corrupt())?,
                    nonnegative(&row, "mana_spent")?,
                ),
                skills,
            )
            .map_err(|_| corrupt())?;
            let mut values = [(0, 0); 9];
            values[0] = (nonnegative(&row, "total_experience")?, 0);
            let magic = build.magic();
            values[1] = (u64::from(magic.0), magic.1);
            for (slot, (level, tries)) in values[2..].iter_mut().zip(build.skills()) {
                *slot = (u64::from(level), tries);
            }
            let level = u32::try_from(nonnegative(&row, "level")?).map_err(|_| corrupt())?;
            if level == 0 {
                return Err(corrupt());
            }
            Ok(Source {
                character_id: character(&row)?,
                name: row.try_get("name")?,
                vocation: build.vocation().to_owned(),
                level,
                values,
            })
        })
        .collect()
}

/// Both additional categories use their actual source owners inside the same MVCC cut.
/// Category 10 selects its representative before the vocation filter, including vocation none.
async fn read_extra_sources(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    characters: &[Source],
    catalogues: &SnapshotCatalogues,
) -> Result<[Vec<Source>; 2]> {
    // Migration 0009 keeps a bootstrap-only Character (revision one, no progression row) valid:
    // it has no level to rank, so it never represents its account and never fails the cut.
    let representatives = sqlx::query("SELECT DISTINCT ON(r.account_id) r.account_id::text AS account_id,uuid_send(r.character_id) AS character_id,r.name,p.level,COALESCE(b.vocation,'none') AS vocation FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) LEFT JOIN game_character_build_state b USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 ORDER BY r.account_id,p.level DESC,r.character_id")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    // All account facts, not only achievements earned in this World or by its representative.
    let rows = sqlx::query("SELECT f.account_id::text AS account_id,f.achievement_key,g.earned_at FROM game_account_achievements f JOIN game_account_achievement_grant_requests g USING(account_id,achievement_key,source_kind,source_event_id) WHERE EXISTS(SELECT 1 FROM game_character_roots r WHERE r.account_id=f.account_id AND r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1) ORDER BY f.account_id,f.achievement_key")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    let mut facts: BTreeMap<String, Vec<EarnedAchievement>> = BTreeMap::new();
    for row in rows {
        facts
            .entry(row.try_get("account_id")?)
            .or_default()
            .push(EarnedAchievement {
                achievement_key: row.try_get("achievement_key")?,
                earned_at_unix_ms: row.try_get("earned_at")?,
            });
    }
    let mut achievements = Vec::with_capacity(representatives.len());
    for row in representatives {
        let account: String = row.try_get("account_id")?;
        let earned = facts.remove(&account).unwrap_or_default();
        // The owner handles retired/unknown entries and checked point addition over ALL facts.
        let points = catalogues
            .achievements
            .total_points(&earned)
            .ok_or_else(corrupt)?;
        let level = u32::try_from(nonnegative(&row, "level")?).map_err(|_| corrupt())?;
        if level == 0 {
            return Err(corrupt());
        }
        let mut values = [(0, 0); 9];
        values[0] = (u64::from(points), 0);
        achievements.push(Source {
            character_id: character(&row)?,
            name: row.try_get("name")?,
            vocation: row.try_get("vocation")?,
            level,
            values,
        });
    }
    // Without a Charm point source category 11 is not published, so its counters are not read.
    if !catalogues.charm_points {
        return Ok([achievements, Vec::new()]);
    }
    // One set-based read of the native kill counters, avoiding a network call per Character.
    // Both consumers use the owning BestiaryRace::is_complete predicate and content points;
    // spent unlock currency is deliberately irrelevant to the earned-points category.
    let rows = sqlx::query("SELECT uuid_send(p.character_id) AS character_id,p.race_key,p.kill_count FROM game_character_bestiary_progress p JOIN game_character_roots r USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 ORDER BY p.character_id,p.race_key")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    let mut earned: BTreeMap<CharacterId, u64> = BTreeMap::new();
    for row in rows {
        let key: String = row.try_get("race_key")?;
        let count = u32::try_from(nonnegative(&row, "kill_count")?).map_err(|_| corrupt())?;
        if let Some(entry) = catalogues.bestiary.get(&key)
            && entry.race.is_complete(count)
        {
            let sum = earned.entry(character(&row)?).or_default();
            *sum = sum
                .checked_add(u64::from(entry.charm_points))
                .ok_or_else(corrupt)?;
        }
    }
    let mut charms = Vec::with_capacity(characters.len());
    for source in characters {
        let mut values = [(0, 0); 9];
        values[0] = (earned.remove(&source.character_id).unwrap_or(0), 0);
        charms.push(Source {
            character_id: source.character_id,
            name: source.name.clone(),
            vocation: source.vocation.clone(),
            level: source.level,
            values,
        });
    }
    Ok([achievements, charms])
}

#[derive(Serialize)]
struct StoredRow {
    filter: i16,
    position: i32,
    rank: i64,
    character_id: String,
    name: String,
    vocation: String,
    level: i64,
    value_primary: i64,
    value_progress: i64,
}
fn materialize(source: &[Source], category: usize) -> Result<Vec<StoredRow>> {
    let levels: BTreeMap<_, _> = source.iter().map(|r| (r.character_id, r.level)).collect();
    let input: Vec<_> = source
        .iter()
        .map(|r| RankingRow {
            character_id: r.character_id,
            name: r.name.as_str(),
            vocation: r.vocation.as_str(),
            value: r.values[category],
        })
        .collect();
    let mut result = Vec::new();
    for (filter, vocation) in FILTERS.iter().enumerate() {
        for (position, ranked) in ranked_list(&input, *vocation)
            .map_err(|_| corrupt())?
            .into_iter()
            .enumerate()
        {
            let row = ranked.row;
            result.push(StoredRow {
                filter: i16::try_from(filter).map_err(|_| corrupt())?,
                position: i32::try_from(position + 1).map_err(|_| corrupt())?,
                rank: i64::try_from(ranked.global_rank).map_err(|_| corrupt())?,
                character_id: row
                    .character_id
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                name: row.name.to_owned(),
                vocation: row.vocation.to_owned(),
                level: i64::from(*levels.get(&row.character_id).ok_or_else(corrupt)?),
                value_primary: i64::try_from(row.value.0).map_err(|_| corrupt())?,
                value_progress: i64::try_from(row.value.1).map_err(|_| corrupt())?,
            });
        }
    }
    Ok(result)
}

#[allow(
    dead_code,
    reason = "the PostgreSQL stale-publication case calls it inside its own transaction"
)]
pub(crate) async fn publish_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Vec<SnapshotInfo>> {
    publish_cut(tx, world, None, false)
        .await
        .map(|refresh| refresh.snapshots)
}

async fn publish_cut(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    catalogues: Option<&SnapshotCatalogues>,
    only_if_due: bool,
) -> Result<SnapshotRefresh> {
    // HIGHSCORES0 §4: only the job's role writes the snapshot tables. The runtime login is its
    // member without inheritance, so these privileges last for this transaction only.
    sqlx::query("SET LOCAL ROLE oteryn_game_highscore_job")
        .execute(&mut **tx)
        .await?;
    let categories: i32 = catalogues.map_or(9, SnapshotCatalogues::published_categories);
    // A real locked publication row, not an advisory lock acquired after a stale MVCC cut.
    // Concurrent RR publishers either see this generation or lose with 40001. Initial
    // concurrent head insertion also conflicts; an invisible head never becomes an empty cut.
    sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category) SELECT encode($1,'hex')::uuid,category FROM generate_series(1,$2) category ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(categories).execute(&mut **tx).await?;
    let heads = sqlx::query("SELECT category,current_snapshot_id FROM game_world_highscore_heads WHERE world_id=encode($1,'hex')::uuid AND category<=$2 ORDER BY category FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(categories).fetch_all(&mut **tx).await?;
    if heads.len() != usize::try_from(categories).map_err(|_| corrupt())? {
        return Err(SnapshotError::RetryableConflict);
    }
    let cut = sqlx::query("SELECT pg_current_snapshot()::text AS source_cut,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS computed_at_ms")
        .fetch_one(&mut **tx).await?;
    let source_cut: String = cut.try_get("source_cut")?;
    let computed_at_ms: i64 = cut.try_get("computed_at_ms")?;
    // Read due state while holding the same per-category locks used by publication. A stale
    // concurrent RR cut loses with a typed conflict rather than publishing a second generation.
    if only_if_due {
        let recent = sqlx::query("SELECT count(*) AS recent,min(s.computed_at_ms) AS oldest FROM game_world_highscore_heads h JOIN game_world_highscore_snapshots s ON s.snapshot_id=h.current_snapshot_id WHERE h.world_id=encode($1,'hex')::uuid AND h.category<=$2 AND s.computed_at_ms>$3")
            .bind(world.as_bytes().as_slice()).bind(categories)
            .bind(computed_at_ms.saturating_sub(SNAPSHOT_INTERVAL_MS)).fetch_one(&mut **tx).await?;
        if recent.try_get::<i64, _>("recent")? == i64::from(categories) {
            let oldest: i64 = recent.try_get("oldest")?;
            let remaining = oldest
                .saturating_add(SNAPSHOT_INTERVAL_MS)
                .saturating_sub(computed_at_ms);
            return Ok(SnapshotRefresh {
                snapshots: Vec::new(),
                next_check_after: Duration::from_millis(
                    u64::try_from(remaining).map_err(|_| corrupt())?,
                ),
            });
        }
    }
    let source = read_source(tx, world).await?;
    let extra = match catalogues {
        Some(catalogues) => Some(read_extra_sources(tx, world, &source, catalogues).await?),
        None => None,
    };
    let mut published = Vec::new();
    for (index, head) in heads.into_iter().enumerate() {
        let category = u8::try_from(head.try_get::<i32, _>("category")?).map_err(|_| corrupt())?;
        if usize::from(category) != index + 1 {
            return Err(corrupt());
        }
        let old: Option<i64> = head.try_get("current_snapshot_id")?;
        let rows = match index {
            0..=8 => materialize(&source, index)?,
            9 | 10 => materialize(&extra.as_ref().ok_or_else(corrupt)?[index - 9], 0)?,
            _ => return Err(corrupt()),
        };
        let count = i32::try_from(rows.len()).map_err(|_| corrupt())?;
        let id: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5) RETURNING snapshot_id")
            .bind(world.as_bytes().as_slice()).bind(i32::from(category)).bind(computed_at_ms)
            .bind(&source_cut).bind(count).fetch_one(&mut **tx).await?;
        // At most 6000 rows/category, one bounded bulk statement instead of per-row calls.
        let json = serde_json::to_string(&rows).map_err(|_| corrupt())?;
        let inserted = sqlx::query("INSERT INTO game_world_highscore_rows(snapshot_id,filter,position,rank,character_id,name,vocation,level,value_primary,value_progress) SELECT $1,x.filter,x.position,x.rank,x.character_id::uuid,x.name,x.vocation,x.level,x.value_primary,x.value_progress FROM jsonb_to_recordset($2::jsonb) AS x(filter SMALLINT,position INTEGER,rank BIGINT,character_id TEXT,name TEXT,vocation TEXT,level BIGINT,value_primary BIGINT,value_progress BIGINT)")
            .bind(id).bind(&json).execute(&mut **tx).await?.rows_affected();
        if inserted != u64::try_from(count).map_err(|_| corrupt())? {
            return Err(corrupt());
        }
        sqlx::query("UPDATE game_world_highscore_heads SET previous_snapshot_id=current_snapshot_id,current_snapshot_id=$3 WHERE world_id=encode($1,'hex')::uuid AND category=$2")
            .bind(world.as_bytes().as_slice()).bind(i32::from(category)).bind(id).execute(&mut **tx).await?;
        sqlx::query("DELETE FROM game_world_highscore_snapshots WHERE world_id=encode($1,'hex')::uuid AND category=$2 AND snapshot_id<>$3 AND ($4::bigint IS NULL OR snapshot_id<>$4)")
            .bind(world.as_bytes().as_slice()).bind(i32::from(category)).bind(id).bind(old).execute(&mut **tx).await?;
        published.push(SnapshotInfo {
            snapshot_id: u64::try_from(id).map_err(|_| corrupt())?,
            category,
            computed_at_ms,
            source_cut: source_cut.clone(),
        });
    }
    Ok(SnapshotRefresh {
        snapshots: published,
        next_check_after: Duration::from_millis(
            u64::try_from(SNAPSHOT_INTERVAL_MS).map_err(|_| corrupt())?,
        ),
    })
}

fn stored_row(row: PgRow) -> Result<SnapshotRow> {
    Ok(SnapshotRow {
        rank: nonnegative(&row, "rank")?,
        character_id: character(&row)?,
        name: row.try_get("name")?,
        vocation: row.try_get("vocation")?,
        level: u32::try_from(nonnegative(&row, "level")?).map_err(|_| corrupt())?,
        value: (
            nonnegative(&row, "value_primary")?,
            nonnegative(&row, "value_progress")?,
        ),
    })
}
async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    category: u8,
    filter: usize,
    page: u32,
    requested: u64,
    character: CharacterId,
) -> Result<SnapshotPage> {
    let wanted = i64::try_from(requested).map_err(|_| SnapshotError::SnapshotExpired)?;
    let row = sqlx::query("SELECT s.snapshot_id,s.computed_at_ms,s.source_cut FROM game_world_highscore_snapshots s JOIN game_world_highscore_heads h USING(world_id,category) WHERE s.world_id=encode($1,'hex')::uuid AND s.category=$2 AND s.snapshot_id=CASE WHEN $3=0 THEN h.current_snapshot_id ELSE $3 END AND s.snapshot_id IN(h.current_snapshot_id,h.previous_snapshot_id)")
        .bind(world.as_bytes().as_slice()).bind(i32::from(category)).bind(wanted).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Err(if requested == 0 {
            DurabilityError::Unavailable.into()
        } else {
            SnapshotError::SnapshotExpired
        });
    };
    let id: i64 = row.try_get("snapshot_id")?;
    let snapshot = SnapshotInfo {
        snapshot_id: u64::try_from(id).map_err(|_| corrupt())?,
        category,
        computed_at_ms: row.try_get("computed_at_ms")?,
        source_cut: row.try_get("source_cut")?,
    };
    let start = i64::from(page - 1) * i64::try_from(PAGE_ROWS).map_err(|_| corrupt())?;
    let end = start + i64::try_from(PAGE_ROWS).map_err(|_| corrupt())?;
    let filter = i16::try_from(filter).map_err(|_| corrupt())?;
    let rows = sqlx::query("SELECT uuid_send(character_id) AS character_id,rank,name,vocation,level,value_primary,value_progress FROM game_world_highscore_rows WHERE snapshot_id=$1 AND filter=$2 AND position>$3 AND position<=$4 ORDER BY position LIMIT 50")
        .bind(id).bind(filter).bind(start).bind(end).fetch_all(&mut **tx).await?
        .into_iter().map(stored_row).collect::<Result<Vec<_>>>()?;
    let has_more = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_highscore_rows WHERE snapshot_id=$1 AND filter=$2 AND position>$3)")
        .bind(id).bind(filter).bind(end).fetch_one(&mut **tx).await?;
    let own_row = sqlx::query("SELECT uuid_send(character_id) AS character_id,rank,name,vocation,level,value_primary,value_progress FROM game_world_highscore_rows WHERE snapshot_id=$1 AND filter=$2 AND character_id=encode($3,'hex')::uuid")
        .bind(id).bind(filter).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
        .map(stored_row).transpose()?;
    Ok(SnapshotPage {
        snapshot,
        page,
        has_more,
        rows,
        own_row,
    })
}
