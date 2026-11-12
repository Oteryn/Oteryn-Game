//! HIGHSCORES0 §§3–6: rebuildable committed-state projections, never gameplay authority.
//!
//! RANK-1: migration 0129 installs the tables; the node's serve loop runs one
//! [`job::HighscoreWorldJob`] per World and the HS-WIRE-1 handler reads pages through
//! [`DurabilityRoot::read_highscore_page`]. A Root pass supplies its real connection and deadline;
//! it does not attest a caller's World or job permission.
pub mod ranking;

pub mod job;

pub mod reply;

use super::account_achievement::EarnedAchievement;
use super::character_build::{DurableBuildState, SKILLS};
use super::charm_state::{BestiaryCharmEntry, BestiaryCharmFacts};
use super::db::commit_semantic_transaction;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CharacterId, WorldId};
use ranking::{RankingRow, ranked_lists};
use sqlx::{Connection, Postgres, Row, Transaction, pool::PoolConnection, postgres::PgRow};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The stored `filter` of `game_world_highscore_rows`: 0 all, otherwise the base vocation by its
/// public §4.0 id (druid 1, knight 2, monk 3, paladin 4, sorcerer 5), the numbering the public
/// projection producer reads (`OTERYN_GAME_PUBLIC_PROJECTIONS_V1` §11, "Highscores ids"). The
/// HS-WIRE-1 query numbers its vocations differently; [`reply::QUERY_VOCATIONS`] maps it, and the
/// store resolves a list by its vocation key, never by a query number.
pub const FILTERS: [Option<&str>; 6] = [
    None,
    Some("druid"),
    Some("knight"),
    Some("monk"),
    Some("paladin"),
    Some("sorcerer"),
];
const PAGE_ROWS: u64 = 50;
const SNAPSHOT_INTERVAL_MS: i64 = 30 * 60 * 1000;
/// HIGHSCORES0-RL-04: eligible Characters, and separately ranked accounts, of one World cut.
/// A larger World fails its cut closed with [`SnapshotError::CapacityExceeded`] and keeps the
/// previous snapshot; the cut never truncates its sources, which would falsify global ranks.
pub const CUT_CHARACTERS_MAX: usize = 50_000;
/// HIGHSCORES0-RL-04: rows of one keyset-paged source read inside the cut.
pub const CUT_SOURCE_PAGE_ROWS: usize = 1_000;
/// One keyset page of the cut's eligible Characters in CharacterId order. Its World range and
/// order are an index range scan of `game_character_roots_highscore_world_character` (0129).
pub const CHARACTER_SOURCE_PAGE_SQL: &str = "SELECT uuid_send(r.character_id) AS character_id,r.name,p.level,p.total_experience,b.vocation,b.magic_level,b.mana_spent,b.fist_level,b.fist_tries,b.club_level,b.club_tries,b.sword_level,b.sword_tries,b.axe_level,b.axe_tries,b.distance_level,b.distance_tries,b.shielding_level,b.shielding_tries,b.fishing_level,b.fishing_tries FROM game_character_roots r JOIN game_character_build_state b USING(character_id) LEFT JOIN game_character_progression_state p USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id) AND b.vocation<>'none' AND ($2::text IS NULL OR r.character_id>$2::text::uuid) ORDER BY r.character_id LIMIT $3";
/// One keyset page of the cut's account representatives in AccountId order. Its World range and
/// order are an index range scan of `game_character_roots_highscore_world_account` (0129).
pub const ACCOUNT_REPRESENTATIVE_PAGE_SQL: &str = "SELECT DISTINCT ON(r.account_id) r.account_id::text AS account_id,uuid_send(r.character_id) AS character_id,r.name,p.level,COALESCE(b.vocation,'none') AS vocation FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) LEFT JOIN game_character_build_state b USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id) AND ($2::text IS NULL OR r.account_id>$2::text::uuid) ORDER BY r.account_id,p.level DESC,r.character_id LIMIT $3";

/// The bounds of one World cut. Production uses [`CutLimits::REGISTERED`]; a fixture may only
/// tighten them, never pass the registered hard maximums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutLimits {
    characters_max: usize,
    page_rows: usize,
}
impl CutLimits {
    pub const REGISTERED: Self = Self {
        characters_max: CUT_CHARACTERS_MAX,
        page_rows: CUT_SOURCE_PAGE_ROWS,
    };
    /// Fixture-only tighter bounds, or `None` above a registered maximum or for an empty page.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(
        dead_code,
        reason = "fixture entry point of the PostgreSQL integration targets that path-load this module"
    )]
    pub fn tighter(characters_max: usize, page_rows: usize) -> Option<Self> {
        (characters_max <= CUT_CHARACTERS_MAX && (1..=CUT_SOURCE_PAGE_ROWS).contains(&page_rows))
            .then_some(Self {
                characters_max,
                page_rows,
            })
    }
    /// The next page's row limit: at most one row past the bound, so the refusal is exact and
    /// a cut never holds more than `characters_max + 1` rows of one source.
    fn next_page(self, held: usize) -> Result<i64> {
        let rows = self
            .page_rows
            .min(self.characters_max.saturating_add(1).saturating_sub(held));
        i64::try_from(rows).map_err(|_| corrupt())
    }
    fn admit(self, held: usize) -> Result<()> {
        if held > self.characters_max {
            Err(SnapshotError::CapacityExceeded)
        } else {
            Ok(())
        }
    }
}

/// The account achievement point owner (category 10). The composition root implements it with
/// the activated achievement catalogue; this module stays independent of that content owner.
pub trait AchievementPoints: Send + Sync {
    /// Total points of every earned fact, or `None` when a fact names an unknown key or the sum
    /// overflows. A retired entry counts its owner's value (0).
    fn total_points(&self, earned: &[EarnedAchievement]) -> Option<u32>;
}

/// HIGHSCORES0 §3 category 11 source: the activated per-race Charm point catalogue. CHARM-6 has
/// activated none yet, so this server withholds category 11: the job publishes no category 11
/// snapshot and a query for it is `NOT_SUPPORTED`, never a ranking of fabricated zero points.
pub fn activated_charm_points() -> Option<Vec<BestiaryCharmEntry>> {
    None
}

/// HIGHSCORES0 §3 row 12 and Crystal `game.cpp:434`: Boss Points.
pub const BOSS_POINTS_CATEGORY: u8 = 12;
/// N014 (`highscores-loyalty-v1.md`): Loyalty points from Platform's account projection (0406).
/// Published with every cut; the dispatch route serves it only to sessions that selected
/// capability 498, and it is never queued to the public projection (migration 2801).
pub const LOYALTY_CATEGORY: u8 = 13;
/// docs/reference/tibia-manual/combat.md:262: cumulative levels and the current two-week rotation.
pub const TIBIADROME_LIFETIME_CATEGORY: u8 = 14;
pub const TIBIADROME_ROTATION_CATEGORY: u8 = 17;
pub const PHOSPHORUS_RECORD_CATEGORY: u8 = 16;
const DROME_ROTATION_MS: i64 = super::tibiadrome::ROTATION_SECONDS as i64 * 1000;

/// Categories 1-10, 12, 13, 14, 16 and 17 are always served; category 11 requires an activated
/// Charm point source.
pub fn served_category(category: u8) -> bool {
    (1..=10).contains(&category)
        || category == BOSS_POINTS_CATEGORY
        || matches!(
            category,
            LOYALTY_CATEGORY
                | TIBIADROME_LIFETIME_CATEGORY
                | PHOSPHORUS_RECORD_CATEGORY
                | TIBIADROME_ROTATION_CATEGORY
        )
        || (category == 11 && activated_charm_points().is_some())
}

/// Actual owning catalogues from the activated content generation, never caller-provided scores.
#[derive(Clone)]
pub struct SnapshotCatalogues {
    achievements: Arc<dyn AchievementPoints>,
    pub bestiary: BTreeMap<String, BestiaryCharmEntry>,
    /// Whether a Charm point source backs category 11; without one it is not published.
    charm_points: bool,
}
impl SnapshotCatalogues {
    /// The serve job's catalogues: `achievements` and the activated Charm point source, if any.
    pub fn activated(achievements: Arc<dyn AchievementPoints>) -> Result<Self> {
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
    pub fn new(
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

    /// The categories one publication writes, ascending: 1..=10, 12, 13, 14, 16 and 17, plus 11
    /// with a Charm source.
    fn published_categories(&self) -> Vec<i32> {
        let mut categories: Vec<i32> = (1..=10).collect();
        if self.charm_points {
            categories.push(11);
        }
        categories.extend([
            i32::from(BOSS_POINTS_CATEGORY),
            i32::from(LOYALTY_CATEGORY),
            i32::from(TIBIADROME_LIFETIME_CATEGORY),
            i32::from(PHOSPHORUS_RECORD_CATEGORY),
            i32::from(TIBIADROME_ROTATION_CATEGORY),
        ]);
        categories
    }
}

#[derive(Debug)]
pub enum SnapshotError {
    NotSupported,
    InvalidQuery,
    SnapshotExpired,
    /// A concurrent publisher won the repeatable-read race. No automatic retry.
    RetryableConflict,
    /// The World's cut exceeds HIGHSCORES0-RL-04; nothing is published and the previous
    /// snapshot stays current.
    CapacityExceeded,
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
pub struct SnapshotRefresh {
    pub snapshots: Vec<SnapshotInfo>,
    /// Remaining time from this transaction's database clock to the earliest durable expiry.
    pub next_check_after: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotInfo {
    pub snapshot_id: u64,
    pub category: u8,
    pub computed_at_ms: i64,
    /// PostgreSQL MVCC cut, retained as diagnostic DATA, not a current fence.
    pub source_cut: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRow {
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
pub struct SnapshotPage {
    pub own_page: Option<u32>,
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
pub async fn begin_projection<'a>(
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
    /// Periodic publication is due after 30 minutes; a planned reset forces one complete cut.
    /// The composition root supplies the actual reset notification, not client command DATA.
    pub async fn refresh_highscore_schedule(
        &self,
        world: WorldId,
        catalogues: SnapshotCatalogues,
        planned_reset: bool,
    ) -> Result<SnapshotRefresh> {
        self.publish_highscore_cut(world, catalogues, !planned_reset, CutLimits::REGISTERED)
            .await
    }

    /// Fixture-only planned-reset cut under tighter `limits`, crossing page boundaries and
    /// the World bound with a handful of Characters.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(
        dead_code,
        reason = "fixture entry point of the PostgreSQL integration targets that path-load this module"
    )]
    pub async fn refresh_highscore_world_within(
        &self,
        world: WorldId,
        catalogues: SnapshotCatalogues,
        limits: CutLimits,
    ) -> Result<Vec<SnapshotInfo>> {
        self.publish_highscore_cut(world, catalogues, false, limits)
            .await
            .map(|refresh| refresh.snapshots)
    }

    async fn publish_highscore_cut(
        &self,
        world: WorldId,
        catalogues: SnapshotCatalogues,
        only_if_due: bool,
        limits: CutLimits,
    ) -> Result<SnapshotRefresh> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_projection(holder, deadline).await?;
                    match publish_cut(&mut tx, world, &catalogues, only_if_due, limits).await {
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
    pub async fn read_highscore_page(
        &self,
        world: WorldId,
        category: u8,
        vocation: Option<&str>,
        page: u32,
        snapshot_id: u64,
        character: CharacterId,
    ) -> Result<SnapshotPage> {
        if !(1..=12).contains(&category)
            && !matches!(
                category,
                LOYALTY_CATEGORY
                    | TIBIADROME_LIFETIME_CATEGORY
                    | PHOSPHORUS_RECORD_CATEGORY
                    | TIBIADROME_ROTATION_CATEGORY
            )
        {
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

pub struct Source {
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

fn uuid_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The eligible Characters of categories 1-9, 11 and 12 at the cut: keyset pages in CharacterId
/// order, all inside the caller's repeatable-read cut, so the pages are one consistent set. More
/// than `limits` eligible Characters refuse the whole cut.
pub async fn read_source_within(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    limits: CutLimits,
) -> Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let wanted = limits.next_page(sources.len())?;
        // No row means native vocation `none`, excluded. Do not silently drop an eligible
        // non-none build whose required XP row is missing: fail and retain the prior generation.
        let rows = sqlx::query(CHARACTER_SOURCE_PAGE_SQL)
            .bind(world.as_bytes().as_slice())
            .bind(after.as_deref())
            .bind(wanted)
            .fetch_all(&mut **tx)
            .await?;
        let fetched = rows.len();
        for row in rows {
            sources.push(source_row(&row)?);
        }
        limits.admit(sources.len())?;
        if i64::try_from(fetched).map_err(|_| corrupt())? < wanted {
            return Ok(sources);
        }
        after = sources
            .last()
            .map(|last| uuid_hex(last.character_id.as_bytes()));
    }
}

fn source_row(row: &PgRow) -> Result<Source> {
    let mut skills = [(0, 0); 7];
    for (index, skill) in SKILLS.iter().enumerate() {
        let level: i32 = row.try_get(format!("{skill}_level").as_str())?;
        skills[index] = (
            u16::try_from(level).map_err(|_| corrupt())?,
            nonnegative(row, format!("{skill}_tries").as_str())?,
        );
    }
    let build = DurableBuildState::new(
        row.try_get::<String, _>("vocation")?,
        (
            u16::try_from(row.try_get::<i32, _>("magic_level")?).map_err(|_| corrupt())?,
            nonnegative(row, "mana_spent")?,
        ),
        skills,
    )
    .map_err(|_| corrupt())?;
    let mut values = [(0, 0); 9];
    values[0] = (nonnegative(row, "total_experience")?, 0);
    let magic = build.magic();
    values[1] = (u64::from(magic.0), magic.1);
    for (slot, (level, tries)) in values[2..].iter_mut().zip(build.skills()) {
        *slot = (u64::from(level), tries);
    }
    let level = u32::try_from(nonnegative(row, "level")?).map_err(|_| corrupt())?;
    if level == 0 {
        return Err(corrupt());
    }
    Ok(Source {
        character_id: character(row)?,
        name: row.try_get("name")?,
        vocation: build.vocation().to_owned(),
        level,
        values,
    })
}

/// Both additional categories use their actual source owners inside the same MVCC cut.
/// Category 10 selects its representative before the vocation filter, including vocation none.
/// Every read is keyset paged in the cut; more than `limits` ranked accounts refuse the cut.
async fn read_extra_sources(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    characters: &[Source],
    catalogues: &SnapshotCatalogues,
    limits: CutLimits,
) -> Result<[Vec<Source>; 2]> {
    let mut achievements = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let wanted = limits.next_page(achievements.len())?;
        // Migration 0009 keeps a bootstrap-only Character (revision one, no progression row)
        // valid: it has no level to rank, so it never represents its account and never fails
        // the cut.
        let representatives = sqlx::query(ACCOUNT_REPRESENTATIVE_PAGE_SQL)
            .bind(world.as_bytes().as_slice())
            .bind(after.as_deref())
            .bind(wanted)
            .fetch_all(&mut **tx)
            .await?;
        let fetched = representatives.len();
        let accounts = representatives
            .iter()
            .map(|row| row.try_get::<String, _>("account_id"))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        // All facts of this page's accounts, not only achievements earned in this World or by
        // the representative. The page's accounts are exactly those with a ranked root here.
        // Keyset pages on the (account, key) primary key bound every fetch to `page_rows`.
        let mut facts: BTreeMap<String, Vec<EarnedAchievement>> = BTreeMap::new();
        let fact_page = i64::try_from(limits.page_rows).map_err(|_| corrupt())?;
        let mut fact_after: Option<(String, String)> = None;
        loop {
            let (after_account, after_key) = fact_after.clone().unzip();
            let rows = sqlx::query("SELECT f.account_id::text AS account_id,f.achievement_key,g.earned_at FROM game_account_achievements f JOIN game_account_achievement_grant_requests g USING(account_id,achievement_key,source_kind,source_event_id) WHERE f.account_id=ANY($1::text[]::uuid[]) AND ($2::text IS NULL OR (f.account_id,f.achievement_key)>($2::text::uuid,$3::text)) ORDER BY f.account_id,f.achievement_key LIMIT $4")
                .bind(&accounts).bind(after_account).bind(after_key).bind(fact_page)
                .fetch_all(&mut **tx).await?;
            let fetched = rows.len();
            for row in rows {
                let account: String = row.try_get("account_id")?;
                let achievement_key: String = row.try_get("achievement_key")?;
                fact_after = Some((account.clone(), achievement_key.clone()));
                facts.entry(account).or_default().push(EarnedAchievement {
                    achievement_key,
                    earned_at_unix_ms: row.try_get("earned_at")?,
                });
            }
            if i64::try_from(fetched).map_err(|_| corrupt())? < fact_page {
                break;
            }
        }
        for (row, account) in representatives.iter().zip(&accounts) {
            let earned = facts.remove(account).unwrap_or_default();
            // The owner handles retired/unknown entries and checked point addition over ALL
            // facts.
            let points = catalogues
                .achievements
                .total_points(&earned)
                .ok_or_else(corrupt)?;
            let level = u32::try_from(nonnegative(row, "level")?).map_err(|_| corrupt())?;
            if level == 0 {
                return Err(corrupt());
            }
            let mut values = [(0, 0); 9];
            values[0] = (u64::from(points), 0);
            achievements.push(Source {
                character_id: character(row)?,
                name: row.try_get("name")?,
                vocation: row.try_get("vocation")?,
                level,
                values,
            });
        }
        limits.admit(achievements.len())?;
        if i64::try_from(fetched).map_err(|_| corrupt())? < wanted {
            break;
        }
        after = accounts.last().cloned();
    }
    // Without a Charm point source category 11 is not published, so its counters are not read.
    if !catalogues.charm_points {
        return Ok([achievements, Vec::new()]);
    }
    // One set-based read of the native kill counters per page of the already bounded Characters,
    // avoiding a network call per Character. Both consumers use the owning
    // BestiaryRace::is_complete predicate and content points; spent unlock currency is
    // deliberately irrelevant to the earned-points category.
    let mut charms = Vec::with_capacity(characters.len());
    for page in characters.chunks(limits.page_rows) {
        let ids: Vec<String> = page
            .iter()
            .map(|source| uuid_hex(source.character_id.as_bytes()))
            .collect();
        let rows = sqlx::query("SELECT uuid_send(character_id) AS character_id,race_key,kill_count FROM game_character_bestiary_progress WHERE character_id=ANY($1::text[]::uuid[]) ORDER BY character_id,race_key")
            .bind(&ids).fetch_all(&mut **tx).await?;
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
        for source in page {
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
    }
    Ok([achievements, charms])
}

/// HIGHSCORES0 §3 category 12: Bosstiary boss points, Crystal `game.cpp:9897` `boss_points`;
/// same eligibility as categories 1-9 and 11.
async fn read_boss_points(
    tx: &mut Transaction<'_, Postgres>,
    characters: &[Source],
    limits: CutLimits,
) -> Result<Vec<Source>> {
    let mut bosses = Vec::with_capacity(characters.len());
    for page in characters.chunks(limits.page_rows) {
        let ids: Vec<String> = page
            .iter()
            .map(|source| uuid_hex(source.character_id.as_bytes()))
            .collect();
        let rows = sqlx::query("SELECT uuid_send(character_id) AS character_id,sum(boss_points)::bigint AS boss_points FROM game_character_bosstiary_progress WHERE character_id=ANY($1::text[]::uuid[]) GROUP BY character_id ORDER BY character_id")
            .bind(&ids).fetch_all(&mut **tx).await?;
        let mut earned: BTreeMap<CharacterId, u64> = BTreeMap::new();
        for row in rows {
            earned.insert(character(&row)?, nonnegative(&row, "boss_points")?);
        }
        for source in page {
            let mut values = [(0, 0); 9];
            values[0] = (earned.remove(&source.character_id).unwrap_or(0), 0);
            bosses.push(Source {
                character_id: source.character_id,
                name: source.name.clone(),
                vocation: source.vocation.clone(),
                level: source.level,
                values,
            });
        }
    }
    Ok(bosses)
}

/// Category 13 rows of one page of eligible Characters: each Character whose account holds
/// positive projected loyalty points in `points`, ranked by those points. A Character without
/// a projection or with zero points gets no row, never a fabricated zero.
fn loyalty_sources(page: &[Source], points: &BTreeMap<CharacterId, u64>) -> Vec<Source> {
    page.iter()
        .filter_map(|source| {
            let earned = *points.get(&source.character_id)?;
            (earned > 0).then(|| {
                let mut values = [(0, 0); 9];
                values[0] = (earned, 0);
                Source {
                    character_id: source.character_id,
                    name: source.name.clone(),
                    vocation: source.vocation.clone(),
                    level: source.level,
                    values,
                }
            })
        })
        .collect()
}

/// N014 category 13: the account loyalty projection (0406), Platform's points as Game pulled them;
/// same eligibility as categories 1-9, one set-based read per page inside the cut.
async fn read_loyalty(
    tx: &mut Transaction<'_, Postgres>,
    characters: &[Source],
    limits: CutLimits,
) -> Result<Vec<Source>> {
    let mut loyalty = Vec::new();
    for page in characters.chunks(limits.page_rows) {
        let ids: Vec<String> = page
            .iter()
            .map(|source| uuid_hex(source.character_id.as_bytes()))
            .collect();
        let rows = sqlx::query("SELECT uuid_send(r.character_id) AS character_id,l.loyalty_points FROM game_character_roots r JOIN game_account_loyalty_projections l USING(account_id) WHERE r.character_id=ANY($1::text[]::uuid[]) AND l.loyalty_points>0 ORDER BY r.character_id")
            .bind(&ids).fetch_all(&mut **tx).await?;
        let mut points = BTreeMap::new();
        for row in rows {
            points.insert(character(&row)?, nonnegative(&row, "loyalty_points")?);
        }
        loyalty.extend(loyalty_sources(page, &points));
    }
    Ok(loyalty)
}

/// Only victorious characters enter the Phosphorus Record; no zero-score rows.
async fn read_phosphorus(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    limits: CutLimits,
) -> Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let wanted = limits.next_page(sources.len())?;
        let rows = sqlx::query("SELECT uuid_send(r.character_id) AS character_id,r.name,p.level,COALESCE(b.vocation,'none') AS vocation,max(d.highest_defeated)::bigint AS highest FROM game_character_roots r JOIN game_character_boss_difficulty_records d USING(character_id) JOIN game_boss_room_catalogue c ON c.boss_key=d.boss_key LEFT JOIN game_character_progression_state p USING(character_id) LEFT JOIN game_character_build_state b USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id) AND (c.boss_key='oteryn:boss.phosphorus' OR c.boss_creature_key='oteryn:creature.phosphorus') AND ($2::text IS NULL OR r.character_id>$2::text::uuid) GROUP BY r.character_id,r.name,p.level,b.vocation ORDER BY r.character_id LIMIT $3")
            .bind(world.as_bytes().as_slice()).bind(after.as_deref()).bind(wanted).fetch_all(&mut **tx).await?;
        let fetched = rows.len();
        for row in rows {
            let level = u32::try_from(nonnegative(&row, "level")?).map_err(|_| corrupt())?;
            if level == 0 {
                return Err(corrupt());
            }
            let mut values = [(0, 0); 9];
            values[0] = (nonnegative(&row, "highest")?, 0);
            sources.push(Source {
                character_id: character(&row)?,
                name: row.try_get("name")?,
                vocation: row.try_get("vocation")?,
                level,
                values,
            });
        }
        limits.admit(sources.len())?;
        if i64::try_from(fetched).map_err(|_| corrupt())? < wanted {
            return Ok(sources);
        }
        after = sources
            .last()
            .map(|last| uuid_hex(last.character_id.as_bytes()));
    }
}

/// HIGHSCORES0 §5's vocation-none exclusion belongs to classic categories 1-9/11.
/// docs/reference/tibia-manual/combat.md:252,262: the Drome's positive score is its
/// qualification; current level/vocation are presentation, not new entry prerequisites.
async fn read_tibiadrome_characters(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    limits: CutLimits,
    computed_at_ms: i64,
) -> Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let wanted = limits.next_page(sources.len())?;
        let rows = sqlx::query("SELECT uuid_send(r.character_id) AS character_id,r.name,p.level,COALESCE(b.vocation,'none') AS vocation FROM game_character_roots r LEFT JOIN game_character_progression_state p USING(character_id) LEFT JOIN game_character_build_state b USING(character_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id) AND EXISTS(SELECT 1 FROM game_tibiadrome_scores d WHERE d.character_id=r.character_id AND d.rotation<=$4) AND ($2::text IS NULL OR r.character_id>$2::text::uuid) ORDER BY r.character_id LIMIT $3")
            .bind(world.as_bytes().as_slice()).bind(after.as_deref()).bind(wanted)
            .bind(computed_at_ms / DROME_ROTATION_MS).fetch_all(&mut **tx).await?;
        let fetched = rows.len();
        for row in rows {
            let level = u32::try_from(nonnegative(&row, "level")?).map_err(|_| corrupt())?;
            if level == 0 {
                return Err(corrupt());
            }
            sources.push(Source {
                character_id: character(&row)?,
                name: row.try_get("name")?,
                vocation: row.try_get("vocation")?,
                level,
                values: [(0, 0); 9],
            });
        }
        limits.admit(sources.len())?;
        if i64::try_from(fetched).map_err(|_| corrupt())? < wanted {
            return Ok(sources);
        }
        after = sources
            .last()
            .map(|last| uuid_hex(last.character_id.as_bytes()));
    }
}

/// docs/reference/tibia-manual/combat.md:252,262: lifetime cumulative levels follow the Character;
/// rotation standings use this World and the same clock division as the Drome owner.
/// Character-id pages bound the aggregated result, inside the existing repeatable-read cut.
async fn read_tibiadrome(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    characters: &[Source],
    limits: CutLimits,
    computed_at_ms: i64,
) -> Result<[Vec<Source>; 2]> {
    let mut sources = [Vec::new(), Vec::new()];
    for page in characters.chunks(limits.page_rows) {
        let ids: Vec<String> = page
            .iter()
            .map(|source| uuid_hex(source.character_id.as_bytes()))
            .collect();
        let rows = sqlx::query("SELECT uuid_send(character_id) AS character_id,sum(level)::bigint AS lifetime,max(level) FILTER(WHERE world_id=encode($2,'hex')::uuid AND rotation=$3) AS current_level,min(experience) FILTER(WHERE world_id=encode($2,'hex')::uuid AND rotation=$3) AS current_experience FROM game_tibiadrome_scores WHERE character_id=ANY($1::text[]::uuid[]) AND rotation<=$3 GROUP BY character_id ORDER BY character_id")
            .bind(&ids).bind(world.as_bytes().as_slice()).bind(computed_at_ms / DROME_ROTATION_MS)
            .fetch_all(&mut **tx).await?;
        let mut earned = BTreeMap::new();
        for row in rows {
            earned.insert(
                character(&row)?,
                (
                    nonnegative(&row, "lifetime")?,
                    row.try_get::<Option<i64>, _>("current_level")?,
                    row.try_get::<Option<i64>, _>("current_experience")?,
                ),
            );
        }
        for source in page {
            let Some((lifetime, level, experience)) = earned.remove(&source.character_id) else {
                continue;
            };
            let values = [
                Some((lifetime, 0)),
                match (level, experience) {
                    (Some(level), Some(experience)) => Some((
                        u64::try_from(level).map_err(|_| corrupt())?,
                        // Drome ranks equal levels by lower captured XP (0973's rank index).
                        u64::try_from(
                            i64::MAX
                                .checked_sub(experience)
                                .filter(|_| experience >= 0)
                                .ok_or_else(corrupt)?,
                        )
                        .map_err(|_| corrupt())?,
                    )),
                    (None, None) => None,
                    _ => return Err(corrupt()),
                },
            ];
            for (index, value) in values.into_iter().enumerate() {
                if let Some(value) = value {
                    let mut values = [(0, 0); 9];
                    values[0] = value;
                    sources[index].push(Source {
                        character_id: source.character_id,
                        name: source.name.clone(),
                        vocation: source.vocation.clone(),
                        level: source.level,
                        values,
                    });
                }
            }
        }
    }
    Ok(sources)
}

fn next_refresh_delay(computed_at_ms: i64, interval_ms: i64) -> Result<Duration> {
    let rotation_remaining = DROME_ROTATION_MS - computed_at_ms.rem_euclid(DROME_ROTATION_MS);
    Ok(Duration::from_millis(
        u64::try_from(interval_ms.min(rotation_remaining)).map_err(|_| corrupt())?,
    ))
}

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
    materialize_ordered(source, category, false)
}

fn materialize_ordered(
    source: &[Source],
    category: usize,
    drome_order: bool,
) -> Result<Vec<StoredRow>> {
    let levels: BTreeMap<_, _> = source.iter().map(|r| (r.character_id, r.level)).collect();
    let input: Vec<_> = source
        .iter()
        .map(|r| RankingRow {
            character_id: r.character_id,
            name: r.name.as_str(),
            vocation: r.vocation.as_str(),
            // Drome's existing standing owner uses row_number by level, lower XP,
            // then CharacterId; other Highscores retain shared ranks/name ties.
            value: (
                r.values[category],
                drome_order.then_some(std::cmp::Reverse(r.character_id)),
            ),
        })
        .collect();
    let mut result = Vec::new();
    // One sort of the cut serves all six filter lists.
    let lists = ranked_lists(&input, &FILTERS).map_err(|_| corrupt())?;
    for (filter, list) in lists.into_iter().enumerate() {
        for (position, ranked) in list.into_iter().enumerate() {
            let row = ranked.row;
            result.push(StoredRow {
                filter: i16::try_from(filter).map_err(|_| corrupt())?,
                position: i32::try_from(position + 1).map_err(|_| corrupt())?,
                rank: i64::try_from(ranked.global_rank).map_err(|_| corrupt())?,
                character_id: uuid_hex(row.character_id.as_bytes()),
                name: row.name.to_owned(),
                vocation: row.vocation.to_owned(),
                level: i64::from(*levels.get(&row.character_id).ok_or_else(corrupt)?),
                value_primary: i64::try_from(row.value.0.0).map_err(|_| corrupt())?,
                value_progress: i64::try_from(row.value.0.1).map_err(|_| corrupt())?,
            });
        }
    }
    Ok(result)
}

/// Fixture-only publication with owning catalogues inside the caller's transaction, so the
/// PostgreSQL suite can exercise a stale cut's publication conflict.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    dead_code,
    reason = "fixture entry point of the PostgreSQL integration targets that path-load this module"
)]
pub async fn publish_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    catalogues: &SnapshotCatalogues,
) -> Result<Vec<SnapshotInfo>> {
    publish_cut(tx, world, catalogues, false, CutLimits::REGISTERED)
        .await
        .map(|refresh| refresh.snapshots)
}

async fn publish_cut(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    catalogues: &SnapshotCatalogues,
    only_if_due: bool,
    limits: CutLimits,
) -> Result<SnapshotRefresh> {
    // HIGHSCORES0 §4: only the job's role writes the snapshot tables. The runtime login is its
    // member without inheritance, so these privileges last for this transaction only.
    sqlx::query("SET LOCAL ROLE oteryn_game_highscore_job")
        .execute(&mut **tx)
        .await?;
    let categories: Vec<i32> = catalogues.published_categories();
    // A real locked publication row, not an advisory lock acquired after a stale MVCC cut.
    // Concurrent RR publishers either see this generation or lose with 40001. Initial
    // concurrent head insertion also conflicts; an invisible head never becomes an empty cut.
    sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category) SELECT encode($1,'hex')::uuid,category FROM unnest($2::int[]) category ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(&categories).execute(&mut **tx).await?;
    let heads = sqlx::query("SELECT category,current_snapshot_id FROM game_world_highscore_heads WHERE world_id=encode($1,'hex')::uuid AND category=ANY($2::int[]) ORDER BY category FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(&categories).fetch_all(&mut **tx).await?;
    if heads.len() != categories.len() {
        return Err(SnapshotError::RetryableConflict);
    }
    let cut = sqlx::query("SELECT pg_current_snapshot()::text AS source_cut,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS computed_at_ms")
        .fetch_one(&mut **tx).await?;
    let source_cut: String = cut.try_get("source_cut")?;
    let computed_at_ms: i64 = cut.try_get("computed_at_ms")?;
    // Read due state while holding the same per-category locks used by publication. A stale
    // concurrent RR cut loses with a typed conflict rather than publishing a second generation.
    if only_if_due {
        let recent = sqlx::query("SELECT count(*) AS recent,min(s.computed_at_ms) AS oldest FROM game_world_highscore_heads h JOIN game_world_highscore_snapshots s ON s.snapshot_id=h.current_snapshot_id WHERE h.world_id=encode($1,'hex')::uuid AND h.category=ANY($2::int[]) AND s.computed_at_ms>$3 AND (h.category<>17 OR s.computed_at_ms/$4=$5/$4)")
            .bind(world.as_bytes().as_slice()).bind(&categories)
            .bind(computed_at_ms.saturating_sub(SNAPSHOT_INTERVAL_MS)).bind(DROME_ROTATION_MS).bind(computed_at_ms).fetch_one(&mut **tx).await?;
        if recent.try_get::<i64, _>("recent")?
            == i64::try_from(categories.len()).map_err(|_| corrupt())?
        {
            let oldest: i64 = recent.try_get("oldest")?;
            let remaining = oldest
                .saturating_add(SNAPSHOT_INTERVAL_MS)
                .saturating_sub(computed_at_ms);
            return Ok(SnapshotRefresh {
                snapshots: Vec::new(),
                next_check_after: next_refresh_delay(computed_at_ms, remaining)?,
            });
        }
    }
    let source = read_source_within(tx, world, limits).await?;
    let extra = read_extra_sources(tx, world, &source, catalogues, limits).await?;
    let bosses = read_boss_points(tx, &source, limits).await?;
    let loyalty = read_loyalty(tx, &source, limits).await?;
    let drome_characters = read_tibiadrome_characters(tx, world, limits, computed_at_ms).await?;
    let drome = read_tibiadrome(tx, world, &drome_characters, limits, computed_at_ms).await?;
    let phosphorus = read_phosphorus(tx, world, limits).await?;
    let mut published = Vec::new();
    for (index, head) in heads.into_iter().enumerate() {
        let category = u8::try_from(head.try_get::<i32, _>("category")?).map_err(|_| corrupt())?;
        if i32::from(category) != categories[index] {
            return Err(corrupt());
        }
        let old: Option<i64> = head.try_get("current_snapshot_id")?;
        let rows = match category {
            1..=9 => materialize(&source, usize::from(category) - 1)?,
            10 => materialize(&extra[0], 0)?,
            11 => materialize(&extra[1], 0)?,
            BOSS_POINTS_CATEGORY => materialize(&bosses, 0)?,
            LOYALTY_CATEGORY => materialize(&loyalty, 0)?,
            PHOSPHORUS_RECORD_CATEGORY => materialize(&phosphorus, 0)?,
            TIBIADROME_LIFETIME_CATEGORY => materialize(&drome[0], 0)?,
            TIBIADROME_ROTATION_CATEGORY => materialize_ordered(&drome[1], 0, true)?,
            _ => return Err(corrupt()),
        };
        let count = i32::try_from(rows.len()).map_err(|_| corrupt())?;
        let id: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5) RETURNING snapshot_id")
            .bind(world.as_bytes().as_slice()).bind(i32::from(category)).bind(computed_at_ms)
            .bind(&source_cut).bind(count).fetch_one(&mut **tx).await?;
        // At most 6000 rows/category, one bounded bulk statement instead of per-row calls.
        // Typed arrays avoid serializing and reparsing every field as JSON while the
        // atomic publication transaction is holding its bounded Root pass.
        let filters: Vec<_> = rows.iter().map(|r| r.filter).collect();
        let positions: Vec<_> = rows.iter().map(|r| r.position).collect();
        let ranks: Vec<_> = rows.iter().map(|r| r.rank).collect();
        let characters: Vec<_> = rows.iter().map(|r| r.character_id.as_str()).collect();
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        let vocations: Vec<_> = rows.iter().map(|r| r.vocation.as_str()).collect();
        let levels: Vec<_> = rows.iter().map(|r| r.level).collect();
        let primary: Vec<_> = rows.iter().map(|r| r.value_primary).collect();
        let progress: Vec<_> = rows.iter().map(|r| r.value_progress).collect();
        let inserted = sqlx::query("INSERT INTO game_world_highscore_rows(snapshot_id,filter,position,rank,character_id,name,vocation,level,value_primary,value_progress) SELECT $1,x.filter,x.position,x.rank,x.character_id::uuid,x.name,x.vocation,x.level,x.value_primary,x.value_progress FROM unnest($2::smallint[],$3::integer[],$4::bigint[],$5::text[],$6::text[],$7::text[],$8::bigint[],$9::bigint[],$10::bigint[]) AS x(filter,position,rank,character_id,name,vocation,level,value_primary,value_progress)")
            .bind(id).bind(&filters).bind(&positions).bind(&ranks).bind(&characters)
            .bind(&names).bind(&vocations).bind(&levels).bind(&primary).bind(&progress)
            .execute(&mut **tx).await?.rows_affected();
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
        next_check_after: next_refresh_delay(computed_at_ms, SNAPSHOT_INTERVAL_MS)?,
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
    // docs/reference/tibia-manual/combat.md:263: auctioned Characters leave Drome
    // leaderboards immediately. Like account detachment, this withdraws captured rows;
    // it never reranks or fills gaps in a retained snapshot, or changes other categories.
    let drome = matches!(
        category,
        TIBIADROME_LIFETIME_CATEGORY | PHOSPHORUS_RECORD_CATEGORY | TIBIADROME_ROTATION_CATEGORY
    );
    let rows = sqlx::query("SELECT uuid_send(character_id) AS character_id,rank,name,vocation,level,value_primary,value_progress FROM game_world_highscore_rows hs WHERE snapshot_id=$1 AND filter=$2 AND NOT game_account_closure_detached(character_id) AND (NOT $5::boolean OR EXISTS(SELECT 1 FROM game_character_roots live WHERE live.character_id=hs.character_id AND live.lifecycle=1)) AND position>$3 AND position<=$4 ORDER BY position LIMIT 50")
        .bind(id).bind(filter).bind(start).bind(end).bind(drome).fetch_all(&mut **tx).await?
        .into_iter().map(stored_row).collect::<Result<Vec<_>>>()?;
    let has_more = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_highscore_rows hs WHERE snapshot_id=$1 AND filter=$2 AND NOT game_account_closure_detached(character_id) AND (NOT $4::boolean OR EXISTS(SELECT 1 FROM game_character_roots live WHERE live.character_id=hs.character_id AND live.lifecycle=1)) AND position>$3)")
        .bind(id).bind(filter).bind(end).bind(drome).fetch_one(&mut **tx).await?;
    let own = sqlx::query("SELECT position,uuid_send(character_id) AS character_id,rank,name,vocation,level,value_primary,value_progress FROM game_world_highscore_rows hs WHERE snapshot_id=$1 AND filter=$2 AND NOT game_account_closure_detached(character_id) AND (NOT $4::boolean OR EXISTS(SELECT 1 FROM game_character_roots live WHERE live.character_id=hs.character_id AND live.lifecycle=1)) AND character_id=encode($3,'hex')::uuid")
        .bind(id).bind(filter).bind(character.as_bytes().as_slice()).bind(drome).fetch_optional(&mut **tx).await?
        ;
    let own_page = own
        .as_ref()
        .map(|row| {
            let position: i32 = row.try_get("position")?;
            if !(1..=1000).contains(&position) {
                return Err(corrupt());
            }
            Ok((position as u32 - 1) / PAGE_ROWS as u32 + 1)
        })
        .transpose()?;
    let own_row = own.map(stored_row).transpose()?;
    Ok(SnapshotPage {
        own_page,
        snapshot,
        page,
        has_more,
        rows,
        own_row,
    })
}

#[cfg(test)]
mod category_set_tests {
    use super::{
        BOSS_POINTS_CATEGORY, DROME_ROTATION_MS, LOYALTY_CATEGORY, SNAPSHOT_INTERVAL_MS, Source,
        loyalty_sources, materialize, next_refresh_delay, served_category,
    };
    use crate::foundation::CharacterId;
    use std::collections::BTreeMap;

    #[test]
    fn drome_rotation_boundary_wakes_before_the_highscore_refresh_interval() -> Result<(), String> {
        assert_eq!(
            next_refresh_delay(DROME_ROTATION_MS - 5, SNAPSHOT_INTERVAL_MS)
                .map_err(|e| format!("{e:?}"))?,
            std::time::Duration::from_millis(5),
        );
        assert_eq!(
            next_refresh_delay(DROME_ROTATION_MS, SNAPSHOT_INTERVAL_MS)
                .map_err(|e| format!("{e:?}"))?,
            std::time::Duration::from_secs(30 * 60),
        );
        Ok(())
    }

    #[test]
    fn served_category_admits_boss_points_and_withholds_unsourced_charm_points() {
        assert!(served_category(12));
        assert!(served_category(1));
        assert!(served_category(10));
        assert!(!served_category(11));
        assert!(!served_category(0));
        assert!(served_category(13));
        assert!(served_category(14));
        assert!(!served_category(15));
        assert!(served_category(17));
        assert!(!served_category(18));
        assert!(served_category(16));
        assert_eq!(BOSS_POINTS_CATEGORY, 12);
        assert_eq!(LOYALTY_CATEGORY, 13);
    }

    fn character(tag: u8) -> Result<CharacterId, String> {
        CharacterId::decode(&[1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag])
            .map_err(|e| format!("{e:?}"))
    }

    #[test]
    fn loyalty_category_ranks_characters_by_account_points_without_zero_rows() -> Result<(), String>
    {
        let eligible = [(1, "knight"), (2, "druid"), (3, "knight"), (4, "sorcerer")]
            .into_iter()
            .map(|(tag, vocation)| {
                Ok(Source {
                    character_id: character(tag)?,
                    name: format!("Player {tag}"),
                    vocation: vocation.into(),
                    level: 8,
                    values: [(7, 7); 9],
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        // Characters 1 and 3 share one account (50), 2 has 900, 4's account has zero points.
        let points = BTreeMap::from([
            (character(1)?, 50),
            (character(2)?, 900),
            (character(3)?, 50),
            (character(4)?, 0),
        ]);
        let loyalty = loyalty_sources(&eligible, &points);
        assert_eq!(loyalty.len(), 3);
        assert!(loyalty.iter().all(|row| row.values[0].1 == 0));
        let rows = materialize(&loyalty, 0).map_err(|e| format!("{e:?}"))?;
        let all: Vec<_> = rows.iter().filter(|row| row.filter == 0).collect();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].rank, 1);
        assert_eq!(all[0].value_primary, 900);
        assert_eq!(all[0].name, "Player 2");
        assert!(all[1..].iter().all(|row| row.value_primary == 50 && row.rank > 1));
        assert!(!rows.iter().any(|row| row.name == "Player 4"));
        // The knight list (public filter 2) holds only the account's two knights.
        let knights: Vec<_> = rows.iter().filter(|row| row.filter == 2).collect();
        assert_eq!(knights.len(), 2);
        assert!(knights.iter().all(|row| row.vocation == "knight"));
        Ok(())
    }
}
