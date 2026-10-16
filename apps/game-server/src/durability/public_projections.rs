//! Game side of the public projections (migration 0091): `character_profile`
//! and `highscores` outboxes, the `world_online` scan and its stored
//! snapshots.
//!
//! Every read runs under the reconciled Character authority, so a restored or
//! unreconciled store never publishes. Each call is one semantic transaction.

use super::DurabilityRoot;
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use oteryn_game_server::native_admission_source::public_projections::{
    FILTERS, HighscoreRow, HighscoresBatch, MAX_HIGHSCORE_ROWS, MAX_ONLINE, NAME_HANDOFF_HOLD,
    OnlineCut, OnlineEntry, OnlineState, OnlineWorld, OutboxFacts, ProfileEntry, ProfileSnapshot,
    PublicStore, VOCATION_IDS, VOCATIONS, projection_name_valid, vocation_id,
};
use sqlx::Row;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, CharacterAuthorityError>;

// The Characters with the oldest undelivered changes ($3 at most, oldest
// first), skipping held Characters and any Character holding a name whose
// vacating publication is still undelivered (§7.3: a reused name never joins
// the new holder before the old one's `[]`). The hold ends once the vacating
// row is $2 ms old (`PUBPROJ-NAME-HANDOFF-HOLD`), so a rename cycle, where
// each holder waits on the other's vacating row, cannot hold delivery forever.
// One statement reads the epoch, the revisions and the state. `observed_at` is
// when the revision was assigned, never the read time, so a retry of the same
// (epoch, revision) is byte-identical.
pub(crate) const PROFILE: &str = "\
WITH head AS (SELECT o.character_id, min(o.created_at) AS oldest \
                FROM game_public_character_profile_outbox o \
               WHERE NOT (o.character_id::text = ANY($1::text[])) \
                 AND NOT EXISTS (SELECT 1 FROM game_character_roots n \
                                   JOIN game_public_character_profile_outbox v \
                                     ON v.vacated_name_key = n.name_key \
                                    AND v.character_id <> n.character_id \
                                  WHERE n.character_id = o.character_id \
                                    AND v.created_at > floor(extract(epoch FROM \
                                        statement_timestamp()) * 1000)::bigint - $2) \
               GROUP BY o.character_id \
               ORDER BY oldest, o.character_id LIMIT $3) \
SELECT h.character_id::text AS character_id, e.projection_epoch, p.projection_revision, \
       p.revised_at / 1000 AS observed_at, r.world_id::text AS world_id, r.name, r.lifecycle, \
       coalesce(g.level, 1)::text AS level, coalesce(b.vocation, 'none') AS vocation \
  FROM head h \
  JOIN game_public_character_profiles p USING (character_id) \
 CROSS JOIN game_public_projection_epoch e \
  LEFT JOIN game_character_roots r ON r.character_id = h.character_id \
  LEFT JOIN game_character_progression_state g ON g.character_id = h.character_id \
  LEFT JOIN game_character_build_state b ON b.character_id = h.character_id \
 WHERE e.epoch_scope = 1 \
 ORDER BY h.oldest, h.character_id";

pub(crate) const CLEAR_PROFILES: &str = "DELETE FROM game_public_character_profile_outbox o \
     USING unnest($1::text[], $2::bigint[], $3::bigint[]) AS t(c, e, r) \
     WHERE o.character_id = t.c::uuid AND o.projection_epoch <= t.e \
       AND o.projection_revision <= t.r";

const SOURCE: &str = "SELECT to_regclass('game_world_highscore_heads') IS NOT NULL \
     AND to_regclass('game_world_highscore_snapshots') IS NOT NULL \
     AND to_regclass('game_world_highscore_rows') IS NOT NULL";

const FACTS: &str = "SELECT e.projection_epoch, \
     (SELECT min(created_at) FROM game_public_character_profile_outbox) AS profile_oldest, \
     (SELECT min(created_at) FROM game_public_highscore_outbox) AS highscores_oldest, \
     (to_regclass('game_world_highscore_heads') IS NOT NULL \
      AND to_regclass('game_world_highscore_snapshots') IS NOT NULL \
      AND to_regclass('game_world_highscore_rows') IS NOT NULL) AS highscores_source, \
     floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint AS now_ms \
     FROM game_public_projection_epoch e WHERE e.epoch_scope = 1";

const HIGHSCORES_HEAD: &str = "\
SELECT o.world_id::text AS world_id, o.category, e.projection_epoch, h.projection_revision, \
       h.revised_at / 1000 AS observed_at \
  FROM game_public_highscore_outbox o \
  JOIN game_public_highscores h USING (world_id, category) \
 CROSS JOIN game_public_projection_epoch e \
 WHERE e.epoch_scope = 1 \
   AND NOT ((o.world_id::text || ':' || o.category::text) = ANY($1::text[])) \
 ORDER BY o.created_at, o.world_id, o.category LIMIT 1";

// The HIGHSCORES-0 tables (RANK-1, HS-1). Read only once `SOURCE` holds, in
// the same transaction. One row more than all six lists' bound is read, so an
// oversized snapshot refuses instead of being truncated.
const HIGHSCORES_ROWS: &str = "\
SELECT s.computed_at_ms, r.filter::int AS filter, r.rank::text AS rank, \
       r.character_id::text AS character_id, r.name, r.vocation::text AS vocation, \
       r.level::text AS level, r.value_primary::text AS value \
  FROM game_world_highscore_heads h \
  JOIN game_world_highscore_snapshots s ON s.snapshot_id = h.current_snapshot_id \
  LEFT JOIN game_world_highscore_rows r ON r.snapshot_id = s.snapshot_id \
 WHERE h.world_id = $1::uuid AND h.category = $2 \
 ORDER BY r.filter, r.position LIMIT $3";

const CLEAR_HIGHSCORES: &str = "DELETE FROM game_public_highscore_outbox \
     WHERE world_id = $1::uuid AND category = $2 \
       AND projection_epoch <= $3 AND projection_revision <= $4";

// One statement, so one snapshot: the epoch and the cut (kind 0, the cut is
// its statement time on the database clock), every qualifying online
// Character (kind 1) and every World with a Character or a stored snapshot,
// with that snapshot (kind 2). A Character is online while it holds a live
// game session on its own World, exactly the predicate of the Game's own
// presence lookup (`spell_character_lookup`): its admission guard is eligible
// on that World and held by the session, and the session has
// `session_state = 2`, a current transport and no control loss. One with an
// unclassifiable vocation, a level below 1 or a name outside the wire grammar
// is not listed (§4.6). One entry more than the bound is read per World, so an
// oversized World refuses instead of being truncated.
pub(crate) const ONLINE: &str = "\
WITH cut AS (SELECT e.projection_epoch, \
                    floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint AS cut_ms \
               FROM game_public_projection_epoch e WHERE e.epoch_scope = 1), \
     online AS (SELECT r.world_id, r.name, coalesce(b.vocation, 'none') AS vocation, \
                       coalesce(g.level, 1) AS level, \
                       row_number() OVER (PARTITION BY r.world_id \
                                          ORDER BY r.name COLLATE \"C\") AS n \
                  FROM game_character_roots r \
                  LEFT JOIN game_character_progression_state g \
                    ON g.character_id = r.character_id \
                  LEFT JOIN game_character_build_state b ON b.character_id = r.character_id \
                 WHERE r.lifecycle = 1 \
                   AND EXISTS (SELECT 1 FROM game_durability_admission_character_guards a \
                                  JOIN game_durability_reconnect_sessions s \
                                    ON s.game_session_id = a.holder_game_session_id \
                                WHERE a.character_id = r.character_id \
                                  AND a.eligible AND a.world_id = r.world_id \
                                  AND s.character_id = r.character_id \
                                  AND s.world_id = r.world_id \
                                  AND s.session_state = 2 \
                                  AND s.current_transport_ref IS NOT NULL \
                                  AND s.control_loss_epoch IS NULL) \
                   AND coalesce(b.vocation, 'none') = ANY($2::text[]) \
                   AND coalesce(g.level, 1) >= 1 \
                   AND octet_length(r.name) BETWEEN 1 AND 64 \
                   AND r.name COLLATE \"C\" ~ '^[ -~]+$' \
                   AND strpos(r.name, chr(34)) = 0 AND strpos(r.name, chr(92)) = 0 \
                   AND strpos(r.name, chr(47)) = 0 AND r.name NOT IN ('.', '..')), \
     worlds AS (SELECT world_id FROM game_character_roots \
                UNION SELECT world_id FROM game_public_world_online) \
SELECT 0 AS kind, c.projection_epoch, c.cut_ms, NULL::text AS world_id, NULL::text AS name, \
       NULL::text AS vocation, NULL::text AS level, NULL::bigint AS projection_revision, \
       NULL::bytea AS characters_digest, NULL::bigint AS source_observed_at, \
       NULL::boolean AS acked \
  FROM cut c \
UNION ALL \
SELECT 1, NULL, NULL, o.world_id::text, o.name, o.vocation, o.level::text, NULL, NULL, NULL, \
       NULL \
  FROM online o WHERE o.n <= $1 \
UNION ALL \
SELECT 2, s.projection_epoch, NULL, w.world_id::text, NULL, NULL, NULL, s.projection_revision, \
       s.characters_digest, s.source_observed_at, s.acked \
  FROM worlds w LEFT JOIN game_public_world_online s USING (world_id)";

const PERSIST_ONLINE: &str = "\
INSERT INTO game_public_world_online \
       (world_id, projection_epoch, projection_revision, characters_digest, \
        source_observed_at, acked) \
SELECT w::uuid, e, r, d, o, false \
  FROM unnest($1::text[], $2::bigint[], $3::bigint[], $4::bytea[], $5::bigint[]) AS t(w, e, r, d, o) \
    ON CONFLICT (world_id) DO UPDATE \
   SET projection_epoch = EXCLUDED.projection_epoch, \
       projection_revision = EXCLUDED.projection_revision, \
       characters_digest = EXCLUDED.characters_digest, \
       source_observed_at = EXCLUDED.source_observed_at, acked = false";

const ACK_ONLINE: &str = "\
UPDATE game_public_world_online s SET acked = true \
  FROM unnest($1::text[], $2::bigint[], $3::bigint[]) AS t(w, e, r) \
 WHERE s.world_id = t.w::uuid AND s.projection_epoch = t.e \
   AND s.projection_revision = t.r AND NOT s.acked";

fn positive(value: i64) -> Result<u64> {
    u64::try_from(value)
        .ok()
        .filter(|v| *v > 0)
        .ok_or(CharacterAuthorityError::Rejected)
}

fn wide(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| CharacterAuthorityError::Rejected)
}

fn decimal(value: &str) -> Result<u64> {
    value.parse().map_err(|_| CharacterAuthorityError::Rejected)
}

/// A stored HIGHSCORES-0 vocation: a native key (a promoted vocation counts
/// as its base) or its wire id 1..6. Anything else is unclassifiable and
/// refuses the snapshot.
fn stored_vocation(value: &str) -> Option<u8> {
    vocation_id(value).or_else(|| value.parse().ok().filter(|id| VOCATION_IDS.contains(id)))
}

fn known_vocations() -> Vec<String> {
    VOCATIONS.iter().map(|(key, _)| (*key).to_owned()).collect()
}

impl DurabilityRoot {
    /// The current profiles of up to `limit` Characters with the oldest
    /// undelivered changes, oldest first. A Character without a root, or
    /// with an unclassifiable vocation, a level below 1 or a name outside the
    /// wire grammar, publishes `profile: []` (§4.2).
    pub async fn next_public_profiles(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        held: &[String],
        limit: usize,
    ) -> Result<Vec<ProfileSnapshot>> {
        let recovery = authority.record_for(self)?;
        let held = held.to_vec();
        let hold = i64::try_from(NAME_HANDOFF_HOLD.as_millis()).unwrap_or(i64::MAX);
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(PROFILE)
                        .bind(held)
                        .bind(hold)
                        .bind(limit)
                        .fetch_all(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    let mut snapshots = Vec::with_capacity(rows.len());
                    for row in &rows {
                        match profile_snapshot(row)? {
                            Ok(snapshot) => snapshots.push(snapshot),
                            Err(error) => return Ok(Err(error)),
                        }
                    }
                    Ok(Ok(snapshots))
                })
            })
            .await?
    }

    /// Clear the outbox rows at or below each acknowledged
    /// (`character_id`, epoch, revision), in one statement.
    pub async fn clear_public_profiles(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        acknowledged: &[(String, u64, u64)],
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        let mut columns = (vec![], vec![], vec![]);
        for (character, epoch, revision) in acknowledged {
            columns.0.push(character.clone());
            columns.1.push(wide(*epoch)?);
            columns.2.push(wide(*revision)?);
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query(CLEAR_PROFILES)
                        .bind(columns.0)
                        .bind(columns.1)
                        .bind(columns.2)
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    /// Both outbox families' §5.1 watermark facts on the database clock.
    pub async fn public_outbox_facts(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<OutboxFacts> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(FACTS).fetch_one(&mut *tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    let projection_epoch = match positive(row.try_get("projection_epoch")?) {
                        Ok(v) => v,
                        Err(e) => return Ok(Err(e)),
                    };
                    let source: bool = row.try_get("highscores_source")?;
                    Ok(Ok(OutboxFacts {
                        projection_epoch,
                        profile_oldest_ms: row.try_get("profile_oldest")?,
                        highscores: if source {
                            Some(row.try_get("highscores_oldest")?)
                        } else {
                            None
                        },
                        now_ms: row.try_get("now_ms")?,
                    }))
                })
            })
            .await?
    }

    /// The current HIGHSCORES-0 snapshot of the (World, category) with the
    /// oldest undelivered revision; `None` while the source tables do not
    /// exist. A category without a computed snapshot publishes six empty
    /// lists computed at the revision time.
    pub async fn next_public_highscores(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        held: &[String],
    ) -> Result<Option<HighscoresBatch>> {
        let recovery = authority.record_for(self)?;
        let held = held.to_vec();
        let limit = i64::try_from(FILTERS * MAX_HIGHSCORE_ROWS + 1).unwrap_or(i64::MAX);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let source: bool = sqlx::query_scalar(SOURCE).fetch_one(&mut *tx).await?;
                    let head = if source {
                        sqlx::query(HIGHSCORES_HEAD)
                            .bind(held)
                            .fetch_optional(&mut *tx)
                            .await?
                    } else {
                        None
                    };
                    let Some(head) = head else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let world_id: String = head.try_get("world_id")?;
                    let category: i16 = head.try_get("category")?;
                    let rows = sqlx::query(HIGHSCORES_ROWS)
                        .bind(&world_id)
                        .bind(category)
                        .bind(limit)
                        .fetch_all(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    highscores_batch(world_id, category, &head, &rows)
                })
            })
            .await?
    }

    pub async fn clear_public_highscores(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world_id: &str,
        category: u8,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        let (epoch, revision) = (wide(projection_epoch)?, wide(projection_revision)?);
        let world = world_id.to_owned();
        let category = i16::from(category);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query(CLEAR_HIGHSCORES)
                        .bind(world)
                        .bind(category)
                        .bind(epoch)
                        .bind(revision)
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    /// One `world_online` cut: every World's qualifying online list and the
    /// publisher's stored snapshot of it, read by one statement, so at one
    /// database snapshot.
    pub async fn public_online_cut(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<OnlineCut> {
        let recovery = authority.record_for(self)?;
        let limit = i64::try_from(MAX_ONLINE + 1).unwrap_or(i64::MAX);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(ONLINE)
                        .bind(limit)
                        .bind(known_vocations())
                        .fetch_all(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    online_cut(&rows)
                })
            })
            .await?
    }

    pub async fn persist_public_online(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        states: &[OnlineState],
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        let mut columns = (vec![], vec![], vec![], vec![], vec![]);
        for state in states {
            columns.0.push(state.world_id.clone());
            columns.1.push(wide(state.projection_epoch)?);
            columns.2.push(wide(state.projection_revision)?);
            columns.3.push(state.characters_digest.to_vec());
            columns.4.push(state.source_observed_at);
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query(PERSIST_ONLINE)
                        .bind(columns.0)
                        .bind(columns.1)
                        .bind(columns.2)
                        .bind(columns.3)
                        .bind(columns.4)
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    pub async fn ack_public_online(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        states: &[OnlineState],
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        let mut columns = (vec![], vec![], vec![]);
        for state in states {
            columns.0.push(state.world_id.clone());
            columns.1.push(wide(state.projection_epoch)?);
            columns.2.push(wide(state.projection_revision)?);
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query(ACK_ONLINE)
                        .bind(columns.0)
                        .bind(columns.1)
                        .bind(columns.2)
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }
}

type Decoded<T> = std::result::Result<Result<T>, super::DurabilityError>;

fn profile_snapshot(row: &sqlx::postgres::PgRow) -> Decoded<ProfileSnapshot> {
    let (Ok(projection_epoch), Ok(projection_revision)) = (
        positive(row.try_get("projection_epoch")?),
        positive(row.try_get("projection_revision")?),
    ) else {
        return Ok(Err(CharacterAuthorityError::Rejected));
    };
    let world_id: Option<String> = row.try_get("world_id")?;
    let name: Option<String> = row.try_get("name")?;
    let lifecycle: Option<i16> = row.try_get("lifecycle")?;
    let level: String = row.try_get("level")?;
    let vocation: String = row.try_get("vocation")?;
    let profile = match (world_id, name, lifecycle) {
        (Some(world_id), Some(name), Some(1)) => match (vocation_id(&vocation), decimal(&level)) {
            (Some(vocation), Ok(level)) if level != 0 && projection_name_valid(&name) => {
                Some(ProfileEntry {
                    world_id,
                    name,
                    vocation,
                    level,
                    // Guild membership defaults to public (§4.2, the Tibia
                    // reference); no Game-side setting lowers it yet.
                    guild_visible: true,
                    // No guild exists natively yet (§4.2): always `[]`.
                    guild: None,
                })
            }
            // Unclassifiable: published as absent, never held (§4.2).
            _ => None,
        },
        _ => None,
    };
    Ok(Ok(ProfileSnapshot {
        character_id: row.try_get("character_id")?,
        projection_epoch,
        projection_revision,
        source_observed_at: row.try_get("observed_at")?,
        profile,
    }))
}

fn highscores_batch(
    world_id: String,
    category: i16,
    head: &sqlx::postgres::PgRow,
    rows: &[sqlx::postgres::PgRow],
) -> Decoded<Option<HighscoresBatch>> {
    let (Ok(projection_epoch), Ok(projection_revision), Ok(category)) = (
        positive(head.try_get("projection_epoch")?),
        positive(head.try_get("projection_revision")?),
        u8::try_from(category),
    ) else {
        return Ok(Err(CharacterAuthorityError::Rejected));
    };
    let source_observed_at: i64 = head.try_get("observed_at")?;
    let mut computed_at = source_observed_at;
    let mut lists: [Vec<HighscoreRow>; FILTERS] = Default::default();
    for row in rows {
        computed_at = row.try_get::<i64, _>("computed_at_ms")?.div_euclid(1000);
        let Some(filter) = row.try_get::<Option<i32>, _>("filter")? else {
            continue;
        };
        let Some(list) = usize::try_from(filter).ok().and_then(|f| lists.get_mut(f)) else {
            return Ok(Err(CharacterAuthorityError::Rejected));
        };
        let field =
            |name: &str| -> std::result::Result<Option<String>, sqlx::Error> { row.try_get(name) };
        let (Some(rank), Some(character_id), Some(name), Some(vocation), Some(level), Some(value)) = (
            field("rank")?,
            field("character_id")?,
            field("name")?,
            field("vocation")?,
            field("level")?,
            field("value")?,
        ) else {
            return Ok(Err(CharacterAuthorityError::Rejected));
        };
        let (Ok(rank), Ok(level), Ok(value)) = (decimal(&rank), decimal(&level), decimal(&value))
        else {
            return Ok(Err(CharacterAuthorityError::Rejected));
        };
        list.push(HighscoreRow {
            rank,
            character_id,
            name,
            vocation: stored_vocation(&vocation),
            level,
            value,
        });
    }
    Ok(Ok(Some(HighscoresBatch {
        world_id,
        category,
        projection_epoch,
        projection_revision,
        source_observed_at,
        computed_at,
        lists,
    })))
}

fn online_state(world_id: &str, row: &sqlx::postgres::PgRow) -> Decoded<Option<OnlineState>> {
    let Some(epoch) = row.try_get::<Option<i64>, _>("projection_epoch")? else {
        return Ok(Ok(None));
    };
    let digest: Vec<u8> = row.try_get("characters_digest")?;
    let (Ok(projection_epoch), Ok(projection_revision), Ok(characters_digest)) = (
        positive(epoch),
        positive(row.try_get("projection_revision")?),
        <[u8; 32]>::try_from(digest),
    ) else {
        return Ok(Err(CharacterAuthorityError::Rejected));
    };
    Ok(Ok(Some(OnlineState {
        world_id: world_id.to_owned(),
        projection_epoch,
        projection_revision,
        characters_digest,
        source_observed_at: row.try_get("source_observed_at")?,
        acked: row.try_get("acked")?,
    })))
}

fn online_cut(rows: &[sqlx::postgres::PgRow]) -> Decoded<OnlineCut> {
    let mut cut = None;
    let mut read: BTreeMap<String, usize> = BTreeMap::new();
    let mut lists: BTreeMap<String, Vec<OnlineEntry>> = BTreeMap::new();
    let mut states: BTreeMap<String, Option<OnlineState>> = BTreeMap::new();
    for row in rows {
        match row.try_get::<i32, _>("kind")? {
            0 => {
                let Ok(projection_epoch) = positive(row.try_get("projection_epoch")?) else {
                    return Ok(Err(CharacterAuthorityError::Rejected));
                };
                cut = Some((projection_epoch, row.try_get::<i64, _>("cut_ms")?));
            }
            1 => {
                let world_id: String = row.try_get("world_id")?;
                *read.entry(world_id.clone()).or_default() += 1;
                let name: String = row.try_get("name")?;
                let vocation: String = row.try_get("vocation")?;
                let level: String = row.try_get("level")?;
                // The query lists only classifiable entries; anything else is
                // left out, never a reason to withhold the World (§4.6).
                let (Some(vocation), Ok(level)) = (vocation_id(&vocation), decimal(&level)) else {
                    continue;
                };
                if level == 0 || !projection_name_valid(&name) {
                    continue;
                }
                lists.entry(world_id).or_default().push(OnlineEntry {
                    name,
                    vocation,
                    level,
                });
            }
            2 => {
                let world_id: String = row.try_get("world_id")?;
                let state = match online_state(&world_id, row)? {
                    Ok(state) => state,
                    Err(error) => return Ok(Err(error)),
                };
                states.insert(world_id, state);
            }
            _ => return Ok(Err(CharacterAuthorityError::Rejected)),
        }
    }
    let Some((projection_epoch, cut_ms)) = cut else {
        return Ok(Err(CharacterAuthorityError::Rejected));
    };
    for world_id in read.keys() {
        states.entry(world_id.clone()).or_insert(None);
    }
    let worlds = states
        .into_iter()
        .map(|(world_id, state)| {
            let mut characters = lists.remove(&world_id).unwrap_or_default();
            characters.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
            let over = read.get(&world_id).copied().unwrap_or_default() > MAX_ONLINE;
            OnlineWorld {
                characters: (!over).then_some(characters),
                world_id,
                state,
            }
        })
        .collect();
    Ok(Ok(OnlineCut {
        projection_epoch,
        cut_ms,
        worlds,
    }))
}

/// The publisher's store over one reconciled Character authority.
pub struct PublicProjectionsStore<'r, 'a, 'f, 's> {
    pub root: &'r DurabilityRoot,
    pub authority: &'a ReconciledCharacterAuthority<'f, 's>,
}

impl PublicStore for PublicProjectionsStore<'_, '_, '_, '_> {
    /// A failed checkout of the single holder (gameplay or the publisher
    /// itself) records a demand until maintenance serves it; the publisher
    /// yields meanwhile.
    fn gameplay_waiting(&self) -> bool {
        self.root.has_ready_demand()
    }
    async fn outbox_facts(&self) -> std::result::Result<OutboxFacts, ()> {
        self.root
            .public_outbox_facts(self.authority)
            .await
            .map_err(|_| ())
    }
    async fn next_profiles(
        &self,
        held: &[String],
        limit: usize,
    ) -> std::result::Result<Vec<ProfileSnapshot>, ()> {
        self.root
            .next_public_profiles(self.authority, held, limit)
            .await
            .map_err(|_| ())
    }
    async fn clear_profiles(
        &self,
        acknowledged: &[(String, u64, u64)],
    ) -> std::result::Result<(), ()> {
        self.root
            .clear_public_profiles(self.authority, acknowledged)
            .await
            .map_err(|_| ())
    }
    async fn next_highscores(
        &self,
        held: &[String],
    ) -> std::result::Result<Option<HighscoresBatch>, ()> {
        self.root
            .next_public_highscores(self.authority, held)
            .await
            .map_err(|_| ())
    }
    async fn clear_highscores(
        &self,
        world_id: &str,
        category: u8,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> std::result::Result<(), ()> {
        self.root
            .clear_public_highscores(
                self.authority,
                world_id,
                category,
                projection_epoch,
                projection_revision,
            )
            .await
            .map_err(|_| ())
    }
    async fn online_cut(&self) -> std::result::Result<OnlineCut, ()> {
        self.root
            .public_online_cut(self.authority)
            .await
            .map_err(|_| ())
    }
    async fn persist_online(&self, states: &[OnlineState]) -> std::result::Result<(), ()> {
        self.root
            .persist_public_online(self.authority, states)
            .await
            .map_err(|_| ())
    }
    async fn ack_online(&self, states: &[OnlineState]) -> std::result::Result<(), ()> {
        self.root
            .ack_public_online(self.authority, states)
            .await
            .map_err(|_| ())
    }
}
