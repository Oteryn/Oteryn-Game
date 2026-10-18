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
//! - the death-transaction adapter ([`apply_pvp_death_in_transaction`], PVP-1b, §9): inside the
//!   victim's death transaction, the kill block of every contributor and, for each unjustified
//!   one, its point row, revenge mark, white skull (PVP-0 §6.1, D971) and skull evaluation
//!   ([`skull::evaluate_skull`], D1443 windows), then the deletion of the victim's ledger;
//! - the World job ([`DurabilityRoot::drain_pvp_expiry`]): at most 100 rows per kind per pass,
//!   including the normalization of expired skulls.
//!
//! The runtime classification of each contributor (justified or not, damage or assist) is the
//! caller's (PVP-RT-1 / PVP-DEATH-1); avenging a revenge mark belongs to PVP-DEATH-1.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_session_fence, uuid_text,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::{RuntimeScopeRefV1, WorldId};
use sqlx::Row;

/// The skull arithmetic lives with the PvP combat sources; it is mounted here because the death
/// transaction is compiled into targets that hold the durability tree without the combat tree.
#[path = "../combat/pvp/skull.rs"]
pub(crate) mod skull;

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
    pub(crate) const fn for_test(value: u64) -> Self {
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

/// What one World cleanup pass deleted, and how many expired skulls it cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PvpCleanup {
    pub points: u64,
    pub marks: u64,
    pub ledger_rows: u64,
    pub skulls: u64,
}

/// One contributor of a PvP death (§9) as the runtime classified it: every full and compact
/// ledger row of the victim within the five-minute window is one contributor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvpDeathContributor {
    pub character_id: CharacterId,
    /// A damage contributor; `false` for an assist-only one.
    pub damage: bool,
    /// Justified (marked victim, war or Hardcore); `false` for an unjustified Open-PvP kill.
    pub justified: bool,
}

/// The PvP consequences of one death. Empty for a death without PvP contributors: the death
/// transaction then writes no PvP row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PvpDeathConsequences {
    pub contributors: Vec<PvpDeathContributor>,
}

impl PvpDeathConsequences {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.contributors.is_empty()
    }

    /// The contributors in CharacterId order, after the §9 bounds: at most 80, distinct, none
    /// the victim.
    pub(crate) fn validated(&self, victim: CharacterId) -> Result<Vec<PvpDeathContributor>> {
        let mut contributors = self.contributors.clone();
        contributors.sort_by_key(|contributor| *contributor.character_id.as_bytes());
        if contributors.len() > skull::MAX_DEATH_CONTRIBUTORS
            || contributors
                .windows(2)
                .any(|pair| pair[0].character_id == pair[1].character_id)
            || contributors
                .iter()
                .any(|contributor| contributor.character_id == victim)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(contributors)
    }

    /// The death command binding's PvP part, empty for no contributors so an earlier receipt's
    /// binding is unchanged.
    pub(crate) fn binding_bytes(&self, victim: CharacterId) -> Result<Vec<u8>> {
        let contributors = self.validated(victim)?;
        if contributors.is_empty() {
            return Ok(Vec::new());
        }
        let mut bytes = b"pvp".to_vec();
        bytes.push(
            u8::try_from(contributors.len())
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        );
        for contributor in contributors {
            bytes.extend_from_slice(contributor.character_id.as_bytes());
            bytes.push(u8::from(contributor.damage) | (u8::from(contributor.justified) << 1));
        }
        Ok(bytes)
    }
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
/// World, and clears at most as many skulls whose `skull_until` has passed (the effective skull
/// is already none, §6.2). The authority must belong to this transaction, as for the World party
/// cleanup.
pub(crate) async fn cleanup_pvp_rows_in_transaction(
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
    let skulls = sqlx::query(
        "UPDATE game_character_pvp_state AS s SET skull = 0, skull_until = NULL, \
                revision = s.revision + 1 \
          WHERE s.character_id IN \
           (SELECT e.character_id FROM game_character_pvp_state e \
             WHERE e.world_id = encode($1,'hex')::uuid AND e.skull > 0 \
               AND e.skull_until <= clock_timestamp() \
             ORDER BY e.skull_until, e.character_id LIMIT $2 FOR UPDATE OF e)",
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
        skulls,
    })
}

impl DurabilityRoot {
    /// One bounded World cleanup pass (§6.2) under the current scoped owner, the same source
    /// pass as the party invitation expiry: no command occurrence and no user payment exists.
    pub(crate) async fn drain_pvp_expiry(
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

/// §9 inside the victim's death transaction, after its receipt is inserted (the 0081 and 0123
/// guards require that receipt and the victim's ledger): the contributors must be exactly the
/// victim's ledger rows within five minutes. State rows of the victim and every contributor are
/// inserted when missing and locked in CharacterId order, then each contributor gets its kill
/// block and each unjustified one its point row, revenge mark, white skull and skull
/// evaluation; finally the victim's ledger rows are deleted. No CharacterRevision moves.
/// A refusal (`Ok(Err(_))`) leaves the caller to roll the death transaction back.
pub(crate) async fn apply_pvp_death_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    victim: CharacterId,
    world: WorldId,
    occurrence: &[u8; 16],
    consequences: &PvpDeathConsequences,
) -> std::result::Result<Result<()>, DurabilityError> {
    let contributors = match consequences.validated(victim) {
        Ok(contributors) if !contributors.is_empty() => contributors,
        Ok(_) => return Ok(Ok(())),
        Err(error) => return Ok(Err(error)),
    };
    let mut ids: Vec<Vec<u8>> = contributors
        .iter()
        .map(|contributor| contributor.character_id.as_bytes().to_vec())
        .collect();
    ids.push(victim.as_bytes().to_vec());
    ids.sort();
    let in_world: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_roots \
          WHERE world_id = encode($2,'hex')::uuid \
            AND character_id IN (SELECT encode(id,'hex')::uuid FROM unnest($1::bytea[]) id)",
    )
    .bind(&ids)
    .bind(world.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if usize::try_from(in_world).ok() != Some(ids.len()) {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    for id in &ids {
        sqlx::query(
            "INSERT INTO game_character_pvp_state (character_id, world_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid) \
             ON CONFLICT (character_id) DO NOTHING",
        )
        .bind(id.as_slice())
        .bind(world.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    }
    let locked = sqlx::query(
        "SELECT character_id::text, world_id::text, skull, \
                floor(extract(epoch FROM skull_until) * 1000)::bigint AS skull_until_ms \
           FROM game_character_pvp_state \
          WHERE character_id IN (SELECT encode(id,'hex')::uuid FROM unnest($1::bytea[]) id) \
          ORDER BY character_id FOR UPDATE",
    )
    .bind(&ids)
    .fetch_all(&mut **tx)
    .await?;
    if locked.len() != ids.len()
        || locked
            .iter()
            .map(|row| row.try_get::<String, _>("world_id"))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .iter()
            .any(|stored| uuid_text(stored).ok() != Some(*world.as_bytes()))
    {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    // Contribution evidence: the victim's ledger names exactly these contributors in the window.
    let ledger = sqlx::query(
        "SELECT contributor_character_id::text AS contributor, compact, compact_kind \
           FROM game_character_pvp_ledger \
          WHERE victim_character_id = encode($1,'hex')::uuid \
            AND last_at >= clock_timestamp() - interval '5 minutes' \
          ORDER BY contributor_character_id FOR UPDATE",
    )
    .bind(victim.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    if ledger.len() != contributors.len() {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    for (row, contributor) in ledger.iter().zip(&contributors) {
        let stored = uuid_text(&row.try_get::<String, _>("contributor")?)?;
        let kind: Option<i16> = row.try_get("compact_kind")?;
        let compact: bool = row.try_get("compact")?;
        if stored != *contributor.character_id.as_bytes()
            || (compact && kind != Some(if contributor.damage { 1 } else { 2 }))
        {
            return Ok(Err(CharacterProgressionError::InvalidInput));
        }
    }
    let damage_contributors = contributors.iter().filter(|c| c.damage).count();
    for contributor in &contributors {
        let id = contributor.character_id.as_bytes().as_slice();
        let mut skull_write: Option<(i16, i64)> = None;
        if !contributor.justified {
            let Some(points) =
                skull::unjustified_points_milli(contributor.damage, damage_contributors)
            else {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            };
            let state = locked
                .iter()
                .find(|row| {
                    row.try_get::<String, _>("character_id")
                        .ok()
                        .and_then(|text| uuid_text(&text).ok())
                        == Some(*contributor.character_id.as_bytes())
                })
                .ok_or(DurabilityError::InvalidStoredState)?;
            let current = skull::SkullState {
                skull: match state.try_get::<i16, _>("skull")? {
                    0 => PvpSkull::None,
                    1 => PvpSkull::Red,
                    2 => PvpSkull::Black,
                    _ => return Err(DurabilityError::InvalidStoredState),
                },
                until_ms: state
                    .try_get::<Option<i64>, _>("skull_until_ms")?
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            };
            let now_ms: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint",
            )
            .fetch_one(&mut **tx)
            .await?;
            let now_ms = u64::try_from(now_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
            let history: Vec<(i64, i32)> = sqlx::query_as(
                "SELECT floor(extract(epoch FROM committed_at) * 1000)::bigint, points_milli \
                   FROM game_character_unjustified_points \
                  WHERE character_id = encode($1,'hex')::uuid \
                    AND death_occurrence_id <> encode($2,'hex')::uuid \
                    AND committed_at <= to_timestamp($3::bigint / 1000.0) \
                    AND committed_at > to_timestamp($3::bigint / 1000.0) - $4 * interval '1 millisecond'",
            )
            .bind(id)
            .bind(occurrence.as_slice())
            .bind(i64::try_from(now_ms).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(i64::try_from(skull::HISTORY_WINDOW_MS).map_err(|_| DurabilityError::InvalidStoredState)?)
            .fetch_all(&mut **tx)
            .await?;
            let history = history
                .into_iter()
                .map(|(at, milli)| {
                    Ok(skull::SkullPoint {
                        committed_at_ms: u64::try_from(at)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        points_milli: u16::try_from(milli)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                    })
                })
                .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
            let evaluation = skull::evaluate_skull(now_ms, current, &history, points)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if evaluation.points_milli[0] < u64::from(points) {
                return Err(DurabilityError::InvalidStoredState);
            }
            if let (skull @ (PvpSkull::Red | PvpSkull::Black), Some(until)) =
                (evaluation.state.skull, evaluation.state.until_ms)
            {
                skull_write = Some((
                    if skull == PvpSkull::Red { 1 } else { 2 },
                    i64::try_from(until).map_err(|_| DurabilityError::InvalidStoredState)?,
                ));
            }
            sqlx::query(
                "INSERT INTO game_character_unjustified_points \
                    (character_id, death_occurrence_id, victim_character_id, points_milli) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, $4)",
            )
            .bind(id)
            .bind(occurrence.as_slice())
            .bind(victim.as_bytes().as_slice())
            .bind(i32::from(points))
            .execute(&mut **tx)
            .await?;
            sqlx::query(
                "INSERT INTO game_character_revenge_marks \
                    (victim_character_id, killer_character_id, death_occurrence_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid)",
            )
            .bind(victim.as_bytes().as_slice())
            .bind(id)
            .bind(occurrence.as_slice())
            .execute(&mut **tx)
            .await?;
        }
        // Every contributor: the kill block (RL-13); an unjustified one also keeps its white
        // skull at least as long (PVP-0 §6.1); a new or refreshed skull never shortens an
        // unexpired one (the 0081 guard enforces it too).
        let updated = sqlx::query(
            "UPDATE game_character_pvp_state AS s SET \
                kill_block_until = GREATEST(s.kill_block_until, c.kill_until), \
                white_skull_until = CASE WHEN $2 \
                    THEN GREATEST(s.white_skull_until, s.kill_block_until, c.kill_until) \
                    ELSE s.white_skull_until END, \
                skull = COALESCE($3::smallint, s.skull), \
                skull_until = CASE WHEN $3::smallint IS NULL THEN s.skull_until \
                    ELSE GREATEST(CASE WHEN s.skull_until > clock_timestamp() THEN s.skull_until END, \
                                  to_timestamp($4::bigint / 1000.0)) END, \
                revision = s.revision + 1 \
               FROM (SELECT clock_timestamp() + $5 * interval '1 millisecond' AS kill_until) c \
              WHERE s.character_id = encode($1,'hex')::uuid",
        )
        .bind(id)
        .bind(!contributor.justified)
        .bind(skull_write.map(|(skull, _)| skull))
        .bind(skull_write.map(|(_, until)| until))
        .bind(i64::try_from(skull::KILL_BLOCK_MS).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if updated != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    // §8.3: the death deletes the victim's ledger (the 0081 delete guard admits it in this
    // transaction).
    sqlx::query(
        "DELETE FROM game_character_pvp_ledger WHERE victim_character_id = encode($1,'hex')::uuid",
    )
    .bind(victim.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(Ok(()))
}
