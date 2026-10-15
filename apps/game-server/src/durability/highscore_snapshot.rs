//! HIGHSCORES0 §§3–6: rebuildable committed-state projections, never gameplay authority.
//!
//! UNINSTALLED candidate: the CP owns job/reset scheduling, the dedicated writer role,
//! source SELECT grants, runtime SELECT-only grants and registration. A Root pass supplies
//! its real connection/deadline; it does not attest a caller's World or job permission.
#![allow(
    dead_code,
    reason = "Snapshot job/reader registration awaits the CP slot"
)]

#[path = "../highscores/ranking.rs"]
mod ranking;

use super::character_build::{DurableBuildState, SKILLS};
use super::db::commit_semantic_transaction;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CharacterId, WorldId};
use ranking::{RankingRow, ranked_list};
use serde::Serialize;
use sqlx::{Connection, Postgres, Row, Transaction, pool::PoolConnection, postgres::PgRow};
use std::collections::BTreeMap;
use std::time::Instant;

/// Private persisted filter order, not a new protocol-number allocation.
const FILTERS: [Option<&str>; 6] = [
    None,
    Some("knight"),
    Some("paladin"),
    Some("sorcerer"),
    Some("druid"),
    Some("monk"),
];
const PAGE_ROWS: u64 = 50;

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
async fn begin_projection<'a>(
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
    /// One complete World cut, all nine supported categories, atomically published.
    /// Deployment must bind this to the dedicated projection role. No raw admin pool is used.
    pub(crate) async fn publish_highscore_world(
        &self,
        world: WorldId,
    ) -> Result<Vec<SnapshotInfo>> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_projection(holder, deadline).await?;
                    match publish_in_transaction(&mut tx, world).await {
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
        if !(1..=9).contains(&category) {
            return Err(SnapshotError::NotSupported);
        }
        if page == 0 {
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

struct Source {
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

async fn read_source(tx: &mut Transaction<'_, Postgres>, world: WorldId) -> Result<Vec<Source>> {
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

async fn publish_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Vec<SnapshotInfo>> {
    // A real locked publication row, not an advisory lock acquired after a stale MVCC cut.
    // Concurrent RR publishers either see this generation or lose with 40001. Initial
    // concurrent head insertion also conflicts; an invisible head never becomes an empty cut.
    sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category) SELECT encode($1,'hex')::uuid,category FROM generate_series(1,9) category ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    let heads = sqlx::query("SELECT category,current_snapshot_id FROM game_world_highscore_heads WHERE world_id=encode($1,'hex')::uuid ORDER BY category FOR UPDATE")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if heads.len() != 9 {
        return Err(SnapshotError::RetryableConflict);
    }
    let cut = sqlx::query("SELECT pg_current_snapshot()::text AS source_cut,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS computed_at_ms")
        .fetch_one(&mut **tx).await?;
    let source_cut: String = cut.try_get("source_cut")?;
    let computed_at_ms: i64 = cut.try_get("computed_at_ms")?;
    let source = read_source(tx, world).await?;
    let mut published = Vec::new();
    for (index, head) in heads.into_iter().enumerate() {
        let category = u8::try_from(head.try_get::<i32, _>("category")?).map_err(|_| corrupt())?;
        if usize::from(category) != index + 1 {
            return Err(corrupt());
        }
        let old: Option<i64> = head.try_get("current_snapshot_id")?;
        let rows = materialize(&source, index)?;
        let count = i32::try_from(rows.len()).map_err(|_| corrupt())?;
        let id: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5) RETURNING snapshot_id")
            .bind(world.as_bytes().as_slice()).bind(i32::from(category)).bind(computed_at_ms)
            .bind(&source_cut).bind(count).fetch_one(&mut **tx).await?;
        // At most6000 rows/category, one bounded bulk statement instead of per-row calls.
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
    Ok(published)
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

/// Private UNINSTALLED design/fixture only. The eventual job role gets source SELECT and
/// projection DML; runtime gets projection SELECT only. No role/grant/migration is installed.
pub(crate) const SNAPSHOT_SCHEMA_PROPOSAL: &str = r#"
CREATE TABLE game_world_highscore_snapshots(
 snapshot_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 world_id UUID NOT NULL,category INTEGER NOT NULL CHECK(category BETWEEN 1 AND 9),
 computed_at_ms BIGINT NOT NULL CHECK(computed_at_ms>=0),source_cut TEXT NOT NULL,
 row_count INTEGER NOT NULL CHECK(row_count BETWEEN 0 AND 6000),
 UNIQUE(world_id,category,snapshot_id));
CREATE TABLE game_world_highscore_rows(
 snapshot_id BIGINT NOT NULL REFERENCES game_world_highscore_snapshots ON DELETE CASCADE,
 filter SMALLINT NOT NULL CHECK(filter BETWEEN 0 AND 5),
 position INTEGER NOT NULL CHECK(position BETWEEN 1 AND 1000),rank BIGINT NOT NULL CHECK(rank>=1),
 character_id UUID NOT NULL,name TEXT NOT NULL,vocation TEXT NOT NULL,
 level BIGINT NOT NULL CHECK(level BETWEEN 1 AND 4294967295),
 value_primary BIGINT NOT NULL CHECK(value_primary>=0),value_progress BIGINT NOT NULL CHECK(value_progress>=0),
 PRIMARY KEY(snapshot_id,filter,position),UNIQUE(snapshot_id,filter,character_id));
CREATE TABLE game_world_highscore_heads(
 world_id UUID NOT NULL,category INTEGER NOT NULL CHECK(category BETWEEN 1 AND 9),
 current_snapshot_id BIGINT,previous_snapshot_id BIGINT,PRIMARY KEY(world_id,category),
 CHECK(previous_snapshot_id IS NULL OR current_snapshot_id<>previous_snapshot_id),
 FOREIGN KEY(world_id,category,current_snapshot_id) REFERENCES game_world_highscore_snapshots(world_id,category,snapshot_id),
 FOREIGN KEY(world_id,category,previous_snapshot_id) REFERENCES game_world_highscore_snapshots(world_id,category,snapshot_id));
CREATE FUNCTION game_highscore_immutable() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN
 RAISE EXCEPTION 'immutable highscore projection row' USING ERRCODE='23514'; END;$$;
CREATE TRIGGER game_highscore_header_immutable BEFORE UPDATE ON game_world_highscore_snapshots
 FOR EACH ROW EXECUTE FUNCTION game_highscore_immutable();
CREATE TRIGGER game_highscore_row_immutable BEFORE UPDATE ON game_world_highscore_rows
 FOR EACH ROW EXECUTE FUNCTION game_highscore_immutable();
CREATE FUNCTION game_highscore_row_membership_guard() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN
 IF TG_OP='INSERT' THEN
  IF EXISTS(SELECT 1 FROM game_world_highscore_heads WHERE current_snapshot_id=NEW.snapshot_id OR previous_snapshot_id=NEW.snapshot_id) THEN
   RAISE EXCEPTION 'published snapshot cannot acquire rows' USING ERRCODE='23514';
  END IF;
  RETURN NEW;
 END IF;
 IF EXISTS(SELECT 1 FROM game_world_highscore_snapshots WHERE snapshot_id=OLD.snapshot_id) THEN
  RAISE EXCEPTION 'snapshot rows only delete with their header' USING ERRCODE='23514';
 END IF;
 RETURN OLD;
END;$$;
CREATE TRIGGER game_highscore_row_membership_guard BEFORE INSERT OR DELETE ON game_world_highscore_rows
 FOR EACH ROW EXECUTE FUNCTION game_highscore_row_membership_guard();
REVOKE ALL ON game_world_highscore_snapshots,game_world_highscore_rows,game_world_highscore_heads FROM PUBLIC;
REVOKE ALL ON FUNCTION game_highscore_immutable(),game_highscore_row_membership_guard() FROM PUBLIC;
"#;

#[cfg(test)]
#[path = "highscore_snapshot_tests.rs"]
mod tests;
