//! The durable PvP state of a Character (PVP-1 tables, PVP-RT-1b write-ahead path;
//! PARTY-PVP-0 §6.2, §8.1; migration 0081).
//!
//! `game_character_pvp_state` is a Character + World consequence row outside the
//! CharacterRevision chain (composition rule 1 amendment). This module holds:
//!
//! - the §8.1 deadline write-ahead ([`DurabilityRoot::write_ahead_pvp_deadlines`]): one
//!   transaction under the acting Character's session fence writes the database clock + 60 s +
//!   10 s for the attacker's and the target's affected columns (rows in CharacterId order,
//!   missing rows inserted, a deadline never lowered, the database enforces it too), and only
//!   then may the action take effect;
//! - the admission read ([`DurabilityRoot::read_pvp_state`]): the stored deadlines as time
//!   remaining on the database clock, so a node with a skewed clock cannot shorten a block;
//! - the World job ([`DurabilityRoot::drain_pvp_expiry`]): at most 100 rows per kind per pass.
//!
//! The kill block, skull, unjustified points and revenge marks are written by the death
//! transaction (PVP-DEATH-1), not here.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_session_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::RuntimeScopeRefV1;
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;

/// `PARTYPVP0-RL-12`: a PvP-sourced logout block, a PZ block and a white skull last 60 s.
pub const PVP_BLOCK_MS: u64 = 60_000;
/// `PARTYPVP0-RL-27`: a stored deadline is written this far beyond the runtime one.
pub const PVP_WRITE_AHEAD_SLACK_MS: u64 = 10_000;
/// A write-ahead names the attacker and the target at most.
pub const PVP_WRITE_AHEAD_MAX_TARGETS: usize = 2;
/// §6.2 World job: the most rows one pass deletes per kind.
pub const PVP_CLEANUP_BATCH: i64 = 100;

/// The deadline columns one write-ahead raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PvpDeadlineKinds {
    pub logout_block: bool,
    pub pz_block: bool,
    pub white_skull: bool,
}

impl PvpDeadlineKinds {
    fn any(self) -> bool {
        self.logout_block || self.pz_block || self.white_skull
    }
}

/// One Character of a write-ahead and the columns to raise for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvpWriteAhead {
    pub character_id: CharacterId,
    pub kinds: PvpDeadlineKinds,
}

/// The effective skull (§6.2): the stored one while `skull_until` is in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvpSkull {
    None,
    Red,
    Black,
}

/// Microseconds since the Unix epoch on PostgreSQL's clock, not an owner timer axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PvpDatabaseTimeMicros(u64);

impl PvpDatabaseTimeMicros {
    /// The database value, for diagnostics and database-clock comparisons only.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Lock deadlines and their observation time from the same fenced transaction/query.
/// These values cannot be installed onto an owner's monotonic clock by casting units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvpReadmissionClock {
    pub sampled_at: PvpDatabaseTimeMicros,
    pub logout_until: Option<PvpDatabaseTimeMicros>,
    pub pz_until: Option<PvpDatabaseTimeMicros>,
    pub kill_until: Option<PvpDatabaseTimeMicros>,
}

impl PvpReadmissionClock {
    /// Whether any stored lock still ran at the database observation boundary.
    #[must_use]
    pub fn has_running_lock(self) -> bool {
        [self.logout_until, self.pz_until, self.kill_until]
            .into_iter()
            .flatten()
            .any(|until| until > self.sampled_at)
    }
}

/// A nonnegative duration measured by one PostgreSQL read, not a database epoch timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvpRemainingMicros(u64);

impl PvpRemainingMicros {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[cfg(test)]
    pub const fn for_test(value: u64) -> Self {
        Self(value)
    }
}

/// Completed same-query database durations for the synchronous D1180 owner restore.
/// Zero means unset or elapsed, and must not create a runtime lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvpReadmissionRemaining {
    pub logout: PvpRemainingMicros,
    pub pz: PvpRemainingMicros,
    pub kill: PvpRemainingMicros,
}

/// The stored state at admission, every deadline as the time still remaining on the database
/// clock (`None`: unset or elapsed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredPvpState {
    pub readmission_remaining: PvpReadmissionRemaining,
    pub readmission_clock: PvpReadmissionClock,
    pub skull: PvpSkull,
    pub skull_remaining_ms: Option<u64>,
    pub kill_block_remaining_ms: Option<u64>,
    pub logout_block_remaining_ms: Option<u64>,
    pub pz_block_remaining_ms: Option<u64>,
    pub white_skull_remaining_ms: Option<u64>,
    pub adventurer_forfeited: bool,
    pub revision: u64,
}

/// What one World cleanup pass deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PvpCleanup {
    pub points: u64,
    pub marks: u64,
    pub ledger_rows: u64,
}

impl DurabilityRoot {
    /// §8.1 write-ahead under the session fence of `fence`: the acting Character must be one of
    /// `targets`, which hold at most [`PVP_WRITE_AHEAD_MAX_TARGETS`] distinct Characters of the
    /// fence's World. A refused write refuses the action.
    pub async fn write_ahead_pvp_deadlines(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        targets: Vec<PvpWriteAhead>,
    ) -> Result<()> {
        let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
            return Err(CharacterProgressionError::InvalidInput);
        };
        let mut targets = targets;
        targets.sort_by_key(|target| *target.character_id.as_bytes());
        let distinct = targets
            .windows(2)
            .all(|pair| pair[0].character_id != pair[1].character_id);
        if fence.character_lease_generation == 0
            || targets.is_empty()
            || targets.len() > PVP_WRITE_AHEAD_MAX_TARGETS
            || !distinct
            || !targets.iter().all(|target| target.kinds.any())
            || !targets
                .iter()
                .any(|target| target.character_id == fence.character_id)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_session_fence(&mut tx, &fence.item_fence(), &node).await? {
                        return Ok(Err(error));
                    }
                    for target in &targets {
                        let written = sqlx::query(
                            "INSERT INTO game_character_pvp_state AS s \
                                (character_id, world_id, logout_block_until, pz_block_until, \
                                 white_skull_until) \
                             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                                 CASE WHEN $3 THEN clock_timestamp() + $6 * interval '1 millisecond' END, \
                                 CASE WHEN $4 THEN clock_timestamp() + $6 * interval '1 millisecond' END, \
                                 CASE WHEN $5 THEN clock_timestamp() + $6 * interval '1 millisecond' END) \
                             ON CONFLICT (character_id) DO UPDATE SET \
                                 logout_block_until = GREATEST(s.logout_block_until, EXCLUDED.logout_block_until), \
                                 pz_block_until = GREATEST(s.pz_block_until, EXCLUDED.pz_block_until), \
                                 white_skull_until = GREATEST(s.white_skull_until, EXCLUDED.white_skull_until), \
                                 revision = s.revision + 1 \
                             WHERE s.world_id = EXCLUDED.world_id",
                        )
                        .bind(target.character_id.as_bytes().as_slice())
                        .bind(world_id.as_bytes().as_slice())
                        .bind(target.kinds.logout_block)
                        .bind(target.kinds.pz_block)
                        .bind(target.kinds.white_skull)
                        .bind((PVP_BLOCK_MS + PVP_WRITE_AHEAD_SLACK_MS) as i64)
                        .execute(&mut *tx)
                        .await?
                        .rows_affected();
                        if written != 1 {
                            // The stored row belongs to another World: nothing is written.
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    /// §8.1 admission: the Character's stored state under the session fence, `None` when no row
    /// exists (nothing to restore).
    pub async fn read_pvp_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<StoredPvpState>> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) =
                        assert_session_fence(&mut tx, &fence.item_fence(), &node).await?
                    {
                        return Ok(Err(error));
                    }
                    let stored = load_pvp_state(&mut tx, fence.character_id).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(stored))
                })
            })
            .await?
    }
}

async fn load_pvp_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
) -> std::result::Result<Option<StoredPvpState>, DurabilityError> {
    // Milliseconds still to run on the database clock; NULL when unset or elapsed.
    let row = sqlx::query(
        "WITH c AS MATERIALIZED (SELECT clock_timestamp() AS now) \
         SELECT skull, adventurer_forfeited, revision::text AS revision, \
            (extract(epoch FROM c.now) * 1000000)::bigint AS sampled_at_us, \
            GREATEST(COALESCE(extract(epoch FROM (logout_block_until - c.now)) * 1000000, 0), 0)::bigint AS logout_remaining_us, \
            GREATEST(COALESCE(extract(epoch FROM (pz_block_until - c.now)) * 1000000, 0), 0)::bigint AS pz_remaining_us, \
            GREATEST(COALESCE(extract(epoch FROM (kill_block_until - c.now)) * 1000000, 0), 0)::bigint AS kill_remaining_us, \
            CASE WHEN logout_block_until > c.now THEN \
                (extract(epoch FROM logout_block_until) * 1000000)::bigint END AS logout_until_us, \
            CASE WHEN pz_block_until > c.now THEN \
                (extract(epoch FROM pz_block_until) * 1000000)::bigint END AS pz_until_us, \
            CASE WHEN kill_block_until > c.now THEN \
                (extract(epoch FROM kill_block_until) * 1000000)::bigint END AS kill_until_us, \
            CASE WHEN skull_until > c.now THEN \
                ceil(extract(epoch FROM (skull_until - c.now)) * 1000)::bigint END AS skull_ms, \
            CASE WHEN kill_block_until > c.now THEN \
                ceil(extract(epoch FROM (kill_block_until - c.now)) * 1000)::bigint END AS kill_ms, \
            CASE WHEN logout_block_until > c.now THEN \
                ceil(extract(epoch FROM (logout_block_until - c.now)) * 1000)::bigint END AS logout_ms, \
            CASE WHEN pz_block_until > c.now THEN \
                ceil(extract(epoch FROM (pz_block_until - c.now)) * 1000)::bigint END AS pz_ms, \
            CASE WHEN white_skull_until > c.now THEN \
                ceil(extract(epoch FROM (white_skull_until - c.now)) * 1000)::bigint END AS white_ms \
           FROM c, game_character_pvp_state WHERE character_id = encode($1,'hex')::uuid",
    )
        .bind(character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let ms = |column: &str| -> std::result::Result<Option<u64>, DurabilityError> {
        row.try_get::<Option<i64>, _>(column)?
            .map(|value| u64::try_from(value).map_err(|_| DurabilityError::InvalidStoredState))
            .transpose()
    };
    let skull_remaining_ms = ms("skull_ms")?;
    let skull = match (row.try_get::<i16, _>("skull")?, skull_remaining_ms) {
        (1, Some(_)) => PvpSkull::Red,
        (2, Some(_)) => PvpSkull::Black,
        (0..=2, _) => PvpSkull::None,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let database_time = |value: i64| {
        u64::try_from(value)
            .map(PvpDatabaseTimeMicros)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let database_deadline = |column: &str| -> std::result::Result<_, DurabilityError> {
        row.try_get::<Option<i64>, _>(column)?
            .map(database_time)
            .transpose()
    };
    let remaining = |column: &str| -> std::result::Result<_, DurabilityError> {
        u64::try_from(row.try_get::<i64, _>(column)?)
            .map(PvpRemainingMicros)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(Some(StoredPvpState {
        readmission_remaining: PvpReadmissionRemaining {
            logout: remaining("logout_remaining_us")?,
            pz: remaining("pz_remaining_us")?,
            kill: remaining("kill_remaining_us")?,
        },
        readmission_clock: PvpReadmissionClock {
            sampled_at: database_time(row.try_get("sampled_at_us")?)?,
            logout_until: database_deadline("logout_until_us")?,
            pz_until: database_deadline("pz_until_us")?,
            kill_until: database_deadline("kill_until_us")?,
        },
        skull,
        skull_remaining_ms,
        kill_block_remaining_ms: ms("kill_ms")?,
        logout_block_remaining_ms: ms("logout_ms")?,
        pz_block_remaining_ms: ms("pz_ms")?,
        white_skull_remaining_ms: ms("white_ms")?,
        adventurer_forfeited: row.try_get("adventurer_forfeited")?,
        revision: row
            .try_get::<String, _>("revision")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    }))
}

/// §6.2 World job, inside a transaction that already holds the scope authority: at most
/// [`PVP_CLEANUP_BATCH`] point rows older than 45 days (`PARTYPVP0-RL-23`), expired revenge marks
/// and ledger rows whose `last_at` is older than 5 minutes (`PARTYPVP0-RL-18`) of the authority's
/// World. The authority must belong to this transaction, as for the World party cleanup.
#[allow(dead_code)] // The node's World job calls it in the PVP-RT-1b wiring slice.
pub async fn cleanup_pvp_rows_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &super::spell_item_transaction::SpellItemScopeAuthority,
) -> std::result::Result<PvpCleanup, super::spell_item_transaction::SpellItemError> {
    super::spell_item_transaction::check_scope_transaction(tx, authority).await?;
    let world = &authority.world_bytes();
    let points = sqlx::query(
        "DELETE FROM game_character_unjustified_points WHERE (character_id, death_occurrence_id) IN \
           (SELECT p.character_id, p.death_occurrence_id FROM game_character_unjustified_points p \
              JOIN game_character_roots r ON r.character_id = p.character_id \
             WHERE r.world_id = encode($1,'hex')::uuid \
               AND p.committed_at < clock_timestamp() - interval '45 days' \
             ORDER BY p.committed_at, p.character_id LIMIT $2 FOR UPDATE OF p)",
    )
    .bind(world.as_slice())
    .bind(PVP_CLEANUP_BATCH)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    let marks = sqlx::query(
        "DELETE FROM game_character_revenge_marks WHERE (victim_character_id, killer_character_id, \
            death_occurrence_id) IN \
           (SELECT m.victim_character_id, m.killer_character_id, m.death_occurrence_id \
              FROM game_character_revenge_marks m \
              JOIN game_character_roots r ON r.character_id = m.victim_character_id \
             WHERE r.world_id = encode($1,'hex')::uuid AND m.expires_at <= clock_timestamp() \
             ORDER BY m.expires_at, m.victim_character_id LIMIT $2 FOR UPDATE OF m)",
    )
    .bind(world.as_slice())
    .bind(PVP_CLEANUP_BATCH)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    let ledger_rows = sqlx::query(
        "DELETE FROM game_character_pvp_ledger WHERE (victim_character_id, contributor_character_id) IN \
           (SELECT l.victim_character_id, l.contributor_character_id FROM game_character_pvp_ledger l \
              JOIN game_character_pvp_state s ON s.character_id = l.victim_character_id \
             WHERE s.world_id = encode($1,'hex')::uuid \
               AND l.last_at < clock_timestamp() - interval '5 minutes' \
             ORDER BY l.last_at, l.victim_character_id LIMIT $2 FOR UPDATE OF l)",
    )
    .bind(world.as_slice())
    .bind(PVP_CLEANUP_BATCH)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    Ok(PvpCleanup {
        points,
        marks,
        ledger_rows,
    })
}

impl DurabilityRoot {
    /// One bounded World cleanup pass (§6.2) under the current scoped owner, the same source
    /// pass as the party invitation expiry: no command occurrence and no user payment exists.
    #[allow(dead_code)] // The node's World job calls it in the PVP-RT-1b wiring slice.
    pub async fn drain_pvp_expiry(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> std::result::Result<PvpCleanup, super::spell_item_transaction::SpellItemError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| super::DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx = super::spell_item_transaction::begin_spell_owner_transaction(
                        holder, deadline,
                    )
                    .await?;
                    let authority =
                        super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                            &mut tx, permit, &record, &node, scope, generation,
                        )
                        .await
                        .map_err(|_| super::DurabilityError::Unavailable)?;
                    let cleaned = cleanup_pvp_rows_in_transaction(&mut tx, &authority)
                        .await
                        .map_err(|_| super::DurabilityError::Unavailable)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(cleaned)
                })
            })
            .await
            .map_err(super::spell_item_transaction::SpellItemError::from)
    }
}
