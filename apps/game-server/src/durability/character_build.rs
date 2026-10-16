//! Durable Character build state: vocation, magic level and the seven skills (CHAR-BUILD-1b).
//!
//! A13 (`reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md`) §4.1, §4.2 and
//! §4.6, as amended by SKILLS-0 (`reviews/OTERYN_GAME_SKILLS0_WEAPON_SKILLS_DECISION_2026-09-30.md`)
//! §3.1-§3.3, over migration 0030. [`DurabilityRoot::commit_character_build`] commits one build
//! change as one CharacterRevision with exactly one build receipt, the build row and, for a
//! vocation change that prunes the stance, the stance row. It is fenced exactly like
//! `commit_character_experience` and serializes with every Character writer on the
//! `character_root` row lock; a stale fence writes nothing.
//!
//! The binding covers the expected CharacterRevision, so a revision mismatch fails closed and is
//! never retried with the same occurrence (QUEST-STATE-0 §5.2, CHAR-REV-SEQ-1). The owner submits
//! every revision-advancing write of one Character through one ordered queue (A13 §4.2
//! "Writer"), each taking the revision the previous one committed. A training checkpoint that
//! still loses keeps its pending progress as a delta, applies it to the committed values and
//! commits under a new occurrence.
//!
//! [`DurabilityRoot::read_character_build_state`] is the admission load (A13 §4.1): the row, or
//! the seed when there is none. `verify_character_integrity` checks the chain at admission.
//!
//! The caller passes the content formula table as a [`BuildFormula`], as the XP writer takes its
//! policy. The writer rejects an `after` whose progress already pays for a next level, computes a
//! vocation choice itself ([`convert_vocation`], SKILLS-0 §3.3) and binds the table digest.
//!
//! [`skill_tries_required`], [`cumulative_progress`] and [`relevel`] are the SKILLS-0 §3.1
//! arithmetic for the training, vocation-choice and death-loss callers: `req(L)` as Canary
//! computes it (f64 `pow`, truncated), and cumulative sums in checked 128-bit arithmetic that
//! saturate at the storage bound and never wrap.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text, valid_revision,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::{PgArguments, Postgres};
use sqlx::query::Query;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
const COMMAND_BINDING_VERSION: u8 = 1;

/// Highest magic level or skill level stored (the 0030 CHECK).
pub const MAX_BUILD_LEVEL: u16 = 1000;
/// Lowest skill level, and the seed of every skill (SKILLS-0 §3.1).
pub const MIN_SKILL_LEVEL: u16 = 10;
/// Most `mana_spent` or tries stored: 2^63 - 1 (SKILLS-0 §3.1).
pub const MAX_BUILD_PROGRESS: u64 = i64::MAX.unsigned_abs();
/// The vocation of a Character that has not chosen one (A13 §4.1).
pub const NO_VOCATION: &str = "none";
/// The seven skills in the column order of 0030.
pub const SKILLS: [&str; 7] = [
    "fist",
    "club",
    "sword",
    "axe",
    "distance",
    "shielding",
    "fishing",
];

/// The eight families of 0030 in column order, as `<prefix><column><suffix>`.
macro_rules! family_columns {
    ($prefix:literal, $suffix:literal) => {
        family_columns!(@ $prefix, $suffix; magic_level, mana_spent, fist_level, fist_tries,
            club_level, club_tries, sword_level, sword_tries, axe_level, axe_tries,
            distance_level, distance_tries, shielding_level, shielding_tries, fishing_level,
            fishing_tries)
    };
    (@ $prefix:literal, $suffix:literal; $first:ident $(, $rest:ident)*) => {
        concat!($prefix, stringify!($first), $suffix $(, ", ", $prefix, stringify!($rest), $suffix)*)
    };
}

/// One Character's build state. A family is (`level`, progress toward the next level), compared
/// in that order: magic level is (`magic_level`, `mana_spent`), a skill is (`level`, `tries`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableBuildState {
    vocation: String,
    magic: (u16, u64),
    skills: [(u16, u64); 7],
}

impl Default for DurableBuildState {
    /// The chain seed, which a Character without a build row has.
    fn default() -> Self {
        Self {
            vocation: NO_VOCATION.to_owned(),
            magic: (0, 0),
            skills: [(MIN_SKILL_LEVEL, 0); 7],
        }
    }
}

impl DurableBuildState {
    /// Rejects a vocation key outside the 0030 key syntax and any family outside its bounds.
    pub fn new(
        vocation: impl Into<String>,
        magic: (u16, u64),
        skills: [(u16, u64); 7],
    ) -> Result<Self> {
        let vocation = vocation.into();
        let bounded = |(level, progress): (u16, u64), floor: u16| {
            (floor..=MAX_BUILD_LEVEL).contains(&level) && progress <= MAX_BUILD_PROGRESS
        };
        if !valid_revision(&vocation)
            || !bounded(magic, 0)
            || !skills.iter().all(|skill| bounded(*skill, MIN_SKILL_LEVEL))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            vocation,
            magic,
            skills,
        })
    }

    #[must_use]
    pub fn vocation(&self) -> &str {
        &self.vocation
    }

    #[must_use]
    pub const fn magic(&self) -> (u16, u64) {
        self.magic
    }

    /// In the order of [`SKILLS`].
    #[must_use]
    pub const fn skills(&self) -> [(u16, u64); 7] {
        self.skills
    }

    fn families(&self) -> [(u16, u64); 8] {
        let mut families = [self.magic; 8];
        families[1..].copy_from_slice(&self.skills);
        families
    }

    /// A stored state; a value outside the bounds is corrupt and fails closed.
    fn stored(
        row: &sqlx::postgres::PgRow,
        vocation: &str,
        suffix: &str,
    ) -> std::result::Result<Self, DurabilityError> {
        let family = |level: String, progress: String| {
            let level = u16::try_from(row.try_get::<i32, _>(level.as_str())?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let progress = u64::try_from(row.try_get::<i64, _>(progress.as_str())?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            Ok::<_, DurabilityError>((level, progress))
        };
        let magic = family(
            format!("magic_level{suffix}"),
            format!("mana_spent{suffix}"),
        )?;
        let mut skills = [(0, 0); 7];
        for (slot, skill) in skills.iter_mut().zip(SKILLS) {
            *slot = family(
                format!("{skill}_level{suffix}"),
                format!("{skill}_tries{suffix}"),
            )?;
        }
        Self::new(row.try_get::<String, _>(vocation)?, magic, skills)
            .map_err(|_| DurabilityError::InvalidStoredState)
    }

    /// Binds the vocation and the sixteen family columns, in the order of `family_columns!`.
    fn bind<'q>(
        &self,
        query: Query<'q, Postgres, PgArguments>,
    ) -> Query<'q, Postgres, PgArguments> {
        let mut query = query.bind(self.vocation.clone());
        for (level, progress) in self.families() {
            // Both bounds were checked on construction, so the conversion never saturates.
            query = query
                .bind(i32::from(level))
                .bind(i64::try_from(progress).unwrap_or(i64::MAX));
        }
        query
    }

    fn encode(&self, out: &mut Vec<u8>) {
        // At most 128 bytes (the key syntax).
        out.push(u8::try_from(self.vocation.len()).unwrap_or(u8::MAX));
        out.extend_from_slice(self.vocation.as_bytes());
        for (level, progress) in self.families() {
            out.extend_from_slice(&level.to_be_bytes());
            out.extend_from_slice(&progress.to_be_bytes());
        }
    }
}

/// The cause of a build change (A13 §4.2). There is no death cause: a death carries its loss in
/// the death receipt (A13 §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildCause {
    /// Vocation equal; no family lower, at least one higher.
    Training,
    /// `none` to a vocation key. The owner re-levels cumulative progress under the new vocation
    /// (SKILLS-0 §3.3) and binds the formula table in the policy digest.
    VocationChoice,
    /// One vocation key to another; every family equal.
    Promotion,
    /// OFFLINE-0 marker; ordinary BuildChangeRequest cannot admit it.
    Logout,
    OfflineSettlement,
}

impl BuildCause {
    const fn key(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::VocationChoice => "vocation_choice",
            Self::Promotion => "promotion",
            Self::Logout => "logout",
            Self::OfflineSettlement => "offline_settlement",
        }
    }

    fn parse(key: &str) -> Option<Self> {
        [
            Self::Training,
            Self::VocationChoice,
            Self::Promotion,
            Self::Logout,
            Self::OfflineSettlement,
        ]
        .into_iter()
        .find(|cause| cause.key() == key)
    }

    /// The direction of the 0030 cause CHECK, checked before any write.
    fn admits(self, before: &DurableBuildState, after: &DurableBuildState) -> bool {
        let (from, to) = (before.families(), after.families());
        match self {
            Self::Training => {
                before.vocation == after.vocation
                    && from.iter().zip(&to).all(|(from, to)| to >= from)
                    && from != to
            }
            Self::VocationChoice => before.vocation == NO_VOCATION && after.vocation != NO_VOCATION,
            Self::Promotion => {
                before.vocation != NO_VOCATION
                    && after.vocation != NO_VOCATION
                    && before.vocation != after.vocation
                    && from == to
            }
            Self::Logout | Self::OfflineSettlement => false,
        }
    }
}

/// The idempotency identity of one build commit attempt (UUIDv7), minted by the owner and kept
/// until the outcome is known (A13 §4.2 "Occurrence key").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildOccurrence([u8; 16]);

impl BuildOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildChangeRequest {
    pub occurrence: BuildOccurrence,
    pub cause: BuildCause,
    /// The committed values the change starts from: the owner's copy of the build row.
    pub before: DurableBuildState,
    pub after: DurableBuildState,
    /// The stored stance key a vocation change prunes (A13 §4.2 "Stance fields", §4.4). `None`
    /// leaves the stance row unchanged.
    pub pruned_stance: Option<String>,
}

/// The content formula table (SKILLS-0 §3.5) the writer checks a change against: `req(L)` per
/// vocation and family, and the digest of the table revision, which the receipt binds as its
/// policy digest.
pub trait BuildFormula: Send + Sync {
    /// Progress needed to go from `level - 1` to `level` in `family` (0 is magic level, 1..=7 the
    /// skills in the order of [`SKILLS`]) under `vocation`. `None` means unreachable.
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64>;
    fn digest(&self) -> [u8; 32];
    /// The content revision this table belongs to. The writer refuses a table whose revision is
    /// not the Character's stored content revision.
    fn content_revision(&self) -> &str;
}

/// SKILLS-0 §3.3: keep each family's cumulative progress under the old vocation and re-level it
/// under `vocation`. The magic level is not capped here (DAWNPORT-1 owns the Dawnport cap).
pub fn convert_vocation(
    formula: &dyn BuildFormula,
    before: &DurableBuildState,
    vocation: &str,
) -> Result<DurableBuildState> {
    let mut families = before.families();
    for (family, (level, progress)) in families.iter_mut().enumerate() {
        let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
        let total = cumulative_progress(*level, *progress, floor, |next| {
            formula.required(&before.vocation, family, next)
        });
        (*level, *progress) = relevel(total, floor, |next| {
            formula.required(vocation, family, next)
        });
    }
    let mut skills = [(0, 0); 7];
    skills.copy_from_slice(&families[1..]);
    DurableBuildState::new(vocation, families[0], skills)
}

/// Every level up to the current one is reachable, and progress is held within the current level
/// (SKILLS-0 §3.1): no family has paid for its next reachable level without advancing.
///
/// An uncommitted build successor. It grants no durable or gameplay authority.
/// Only a proof of COMMIT of this same physical transaction exposes its receipt.
pub(crate) struct PendingBuildChange {
    outcome: BuildCommitOutcome,
    physical_transaction: String,
    source_transaction: Option<String>,
}
impl PendingBuildChange {
    pub(super) fn historical_outcome(&self) -> Option<BuildCommitOutcome> {
        match &self.outcome {
            BuildCommitOutcome::AlreadyCommitted(_) => Some(self.outcome.clone()),
            BuildCommitOutcome::Committed(_) => None,
        }
    }
    pub(crate) fn after_commit(
        self,
        proof: &super::spell_owner_commit::CommittedSpellOwnerTransaction,
    ) -> Result<BuildCommitOutcome> {
        if self.source_transaction.as_deref() != Some(proof.physical_transaction())
            || (matches!(self.outcome, BuildCommitOutcome::Committed(_))
                && self.physical_transaction != proof.physical_transaction())
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(self.outcome)
    }
}

/// Join a genuine Character build write to the caster's existing transaction.
/// Current recovery, session, lease, root revision and node are verified here;
/// a preceding write in this transaction cannot substitute for current authority.
pub(crate) async fn prepare_character_build_in_transaction(
    root: &DurabilityRoot,
    tx: &mut sqlx::Transaction<'_, Postgres>,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    formula: &dyn BuildFormula,
) -> Result<PendingBuildChange> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    prepare_character_build_with_recovery(tx, &recovery, node, fence, request, formula).await
}

/// Sibling writers may carry an actual pre-qualified recovery record into the
/// SQL-only semantic-pass callback. This never reconstructs a current token:
/// the record and every independent current DB fence are verified in this TX.
pub(super) async fn prepare_character_build_with_recovery(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    formula: &dyn BuildFormula,
) -> Result<PendingBuildChange> {
    if fence.character_lease_generation == 0
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        || !request.cause.admits(&request.before, &request.after)
        || !normalized(formula, &request.after)
        || request
            .pruned_stance
            .as_deref()
            .is_some_and(|stance| request.cause == BuildCause::Training || !valid_revision(stance))
        || (request.cause == BuildCause::VocationChoice
            && convert_vocation(formula, &request.before, &request.after.vocation)?
                != request.after)
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let policy_digest = formula.digest();
    let binding = command_binding(&fence, &request, &policy_digest);
    let outcome = apply_build_in_transaction(
        tx,
        node,
        &fence,
        request,
        policy_digest,
        formula.content_revision().to_owned(),
        binding,
    )
    .await??;
    let physical_transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
    let occurrence = match &outcome {
        BuildCommitOutcome::Committed(receipt) | BuildCommitOutcome::AlreadyCommitted(receipt) => {
            receipt.occurrence
        }
    };
    let source_transaction:Option<String>=sqlx::query_scalar(
        "SELECT created_xact_id::text FROM game_character_build_receipts WHERE build_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    Ok(PendingBuildChange {
        outcome,
        physical_transaction,
        source_transaction,
    })
}

async fn apply_build_in_transaction(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    policy_digest: [u8; 32],
    content_revision: String,
    binding: Vec<u8>,
) -> std::result::Result<Result<BuildCommitOutcome>, DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:character-build:' || encode($1, 'hex'), 0))",
    )
    .bind(request.occurrence.0.as_slice())
    .execute(&mut **tx)
    .await?;

    if let Some(row) = load_receipt(tx, request.occurrence).await? {
        let stored: Vec<u8> = row.try_get("command_binding")?;
        if stored != binding {
            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
        }
        let committed = decode_receipt(&row)?;
        return Ok(Ok(BuildCommitOutcome::AlreadyCommitted(committed)));
    }

    let root = match assert_gameplay_fence(tx, fence, node).await? {
        Ok(root) => root,
        Err(error) => return Ok(Err(error)),
    };
    let character = fence.character_id.as_bytes().as_slice();
    let state = sqlx::query(
        "SELECT character_revision::text, profile_revision, ruleset_revision, \
                                content_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(state) = state else {
        return Ok(Err(CharacterProgressionError::MissingProgressionState));
    };
    if numeric_u64(&state, "character_revision")? != root.revision {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !state_matches_root(&state, &root)
        || state.try_get::<String, _>("content_revision")? != content_revision
    {
        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid)",
    )
    .bind(character)
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Ok(Err(CharacterProgressionError::RespawnPending));
    }
    let stored_row = sqlx::query(concat!(
        "SELECT vocation, ",
        family_columns!("", ""),
        ", stamina_minutes, offline_pool_s, offline_skill FROM game_character_build_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE"
    ))
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?;
    let offline_columns = stored_row
        .as_ref()
        .map(|row| decode_offline_columns(row, ""))
        .transpose()?
        .unwrap_or_default();
    let stored = stored_row
        .map(|row| DurableBuildState::stored(&row, "vocation", ""))
        .transpose()?
        .unwrap_or_default();
    if stored != request.before {
        return Ok(Err(CharacterProgressionError::BuildStateMismatch));
    }
    if let Some(pruned) = &request.pruned_stance {
        let stance: Option<Option<String>> = sqlx::query_scalar(
            "SELECT stance_key FROM game_character_stance \
                              WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(character)
        .fetch_optional(&mut **tx)
        .await?;
        if stance.flatten().as_ref() != Some(pruned) {
            return Ok(Err(CharacterProgressionError::BuildStateMismatch));
        }
    }

    let committed_revision = root
        .revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (original, committed) = (root.revision.to_string(), committed_revision.to_string());
    for successor in [
        "UPDATE game_character_roots \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
    ] {
        let updated = sqlx::query(successor)
            .bind(character)
            .bind(&committed)
            .bind(&original)
            .execute(&mut **tx)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let receipt = sqlx::query(concat!(
        "INSERT INTO game_character_build_receipts(\
                           build_occurrence_id, command_binding, policy_digest, character_id, \
                           original_character_revision, committed_character_revision, cause, \
                           level_before, level_after, experience_before, experience_after, \
                           vocation_before, ",
        family_columns!("", "_before"),
        ", vocation_after, ",
        family_columns!("", "_after"),
        ", stance_before, stance_after, profile_revision, ruleset_revision, \
                           content_revision, simulation_revision, evidence_revision, \
                           declaration_revision, policy_revision, reward_revision, committed_at, stamina_minutes_before, offline_pool_s_before, offline_skill_before, stamina_minutes_after, offline_pool_s_after, offline_skill_after) \
                         SELECT encode($1,'hex')::uuid, $2, $3, s.character_id, \
                           $4::text::numeric(20,0), $5::text::numeric(20,0), $6, s.level, \
                           s.level, s.total_experience, s.total_experience, \
                           $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, \
                           $21, $22, $23, \
                           $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, \
                           $37, $38, $39, $40, \
                           $41, NULL, s.profile_revision, s.ruleset_revision, \
                           s.content_revision, s.simulation_revision, s.evidence_revision, \
                           s.declaration_revision, s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint, $43, $44, $45, $46, $47, $48 \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($42,'hex')::uuid"
    ))
    .bind(request.occurrence.0.as_slice())
    .bind(&binding)
    .bind(policy_digest.as_slice())
    .bind(&original)
    .bind(&committed)
    .bind(request.cause.key());
    let receipt = request.after.bind(request.before.bind(receipt));
    let receipt = receipt
        .bind(request.pruned_stance.clone())
        .bind(character)
        .bind(i32::from(offline_columns.stamina_minutes))
        .bind(
            i32::try_from(offline_columns.pool_s)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .bind(offline_columns.skill.map_or("none", OfflineSkill::sql_key))
        .bind(i32::from(offline_columns.stamina_minutes))
        .bind(
            i32::try_from(offline_columns.pool_s)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .bind(offline_columns.skill.map_or("none", OfflineSkill::sql_key))
        .execute(&mut **tx)
        .await?;
    if receipt.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let row = sqlx::query(concat!(
        "INSERT INTO game_character_build_state(character_id, vocation, ",
        family_columns!("", ""),
        ", committed_character_revision, last_build_occurrence_id) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, \
                           $11, $12, $13, $14, $15, $16, $17, $18, $19::text::numeric(20,0), \
                           encode($20,'hex')::uuid) \
                         ON CONFLICT (character_id) DO UPDATE SET (vocation, ",
        family_columns!("", ""),
        ", committed_character_revision, last_build_occurrence_id) = \
                           (EXCLUDED.vocation, ",
        family_columns!("EXCLUDED.", ""),
        ", EXCLUDED.committed_character_revision, \
                           EXCLUDED.last_build_occurrence_id)"
    ))
    .bind(character);
    let row = request
        .after
        .bind(row)
        .bind(&committed)
        .bind(request.occurrence.0.as_slice())
        .execute(&mut **tx)
        .await?;
    if row.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if request.pruned_stance.is_some() {
        let pruned = sqlx::query(
            "UPDATE game_character_stance \
                                SET stance_key = NULL, \
                                    committed_character_revision = $2::text::numeric(20,0), \
                                    last_stance_occurrence_id = encode($3,'hex')::uuid \
                              WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(character)
        .bind(&committed)
        .bind(request.occurrence.0.as_slice())
        .execute(&mut **tx)
        .await?;
        if pruned.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }

    let committed = CommittedBuildChange {
        occurrence: request.occurrence,
        character_id: fence.character_id,
        original_character_revision: fence.expected_character_revision,
        committed_character_revision: CharacterRevision::new(committed_revision)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        cause: request.cause,
        before: request.before,
        after: request.after,
        pruned_stance: request.pruned_stance,
    };
    Ok(Ok(BuildCommitOutcome::Committed(committed)))
}

fn normalized(formula: &dyn BuildFormula, state: &DurableBuildState) -> bool {
    state
        .families()
        .into_iter()
        .enumerate()
        .all(|(family, (level, progress))| {
            let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
            let required = |level| formula.required(&state.vocation, family, level);
            (floor.saturating_add(1)..=level).all(|level| required(level).is_some())
                && (level >= MAX_BUILD_LEVEL
                    || required(level + 1).is_none_or(|required| progress < required))
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedBuildChange {
    pub occurrence: BuildOccurrence,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub cause: BuildCause,
    pub before: DurableBuildState,
    pub after: DurableBuildState,
    pub pruned_stance: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildCommitOutcome {
    Committed(CommittedBuildChange),
    AlreadyCommitted(CommittedBuildChange),
}

impl DurabilityRoot {
    /// Commit one build change. Exact occurrence replay returns the retained receipt without
    /// reacquiring session authority; the same occurrence with another binding conflicts, and the
    /// binding is compared before replay. A new occurrence is fenced like an XP award, must start
    /// from the stored build (and stance, for a prune), and is refused while a death's respawn is
    /// pending. `after` must hold its progress within its levels under `formula`, and a vocation
    /// choice must be exactly the writer's own conversion of `before` (SKILLS-0 §3.3).
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_build(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BuildChangeRequest,
        formula: &dyn BuildFormula,
    ) -> Result<BuildCommitOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || !request.cause.admits(&request.before, &request.after)
            || !normalized(formula, &request.after)
            || request.pruned_stance.as_deref().is_some_and(|stance| {
                request.cause == BuildCause::Training || !valid_revision(stance)
            })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if request.cause == BuildCause::VocationChoice
            && convert_vocation(formula, &request.before, &request.after.vocation)? != request.after
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let policy_digest = formula.digest();
        let content_revision = formula.content_revision().to_owned();
        let binding = command_binding(&fence, &request, &policy_digest);
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

                    let outcome = apply_build_in_transaction(
                        &mut tx,
                        &node,
                        &fence,
                        request,
                        policy_digest,
                        content_revision,
                        binding,
                    )
                    .await?;
                    if outcome.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }

    /// Resolve an ambiguous build commit by its occurrence (A13 §4.2). This proves only what
    /// committed and never reacquires gameplay authority.
    pub async fn reconcile_character_build(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: BuildOccurrence,
    ) -> Result<Option<CommittedBuildChange>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let record = load_receipt(&mut tx, occurrence)
                        .await?
                        .as_ref()
                        .map(decode_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(record))
                })
            })
            .await?
    }

    /// The admission load (A13 §4.1): the build row a new runtime actor starts from, or the seed
    /// when the Character has none. It writes nothing. A stored value outside the bounds fails
    /// closed with `Unavailable(InvalidStoredState)`.
    pub async fn read_character_build_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<DurableBuildState> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let state = sqlx::query(concat!(
                        "SELECT vocation, ",
                        family_columns!("", ""),
                        " FROM game_character_build_state \
                          WHERE character_id = encode($1,'hex')::uuid"
                    ))
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?
                    .map(|row| DurableBuildState::stored(&row, "vocation", ""))
                    .transpose()?
                    .unwrap_or_default();
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }
}

/// The complete intent (A13 §4.2 "Occurrence key"): character, expected revision, occurrence,
/// cause, before and after values, the stance fields and the formula table digest. Retry-local
/// authority (session, connection, lease and scope generations) is excluded so a replay after a
/// lost response matches.
fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &BuildChangeRequest,
    policy_digest: &[u8; 32],
) -> Vec<u8> {
    let mut semantic = vec![COMMAND_BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.extend_from_slice(request.cause.key().as_bytes());
    semantic.push(0);
    request.before.encode(&mut semantic);
    request.after.encode(&mut semantic);
    match &request.pruned_stance {
        None => semantic.push(0),
        Some(stance) => {
            semantic.push(1);
            semantic.push(u8::try_from(stance.len()).unwrap_or(u8::MAX));
            semantic.extend_from_slice(stance.as_bytes());
        }
    }
    semantic.extend_from_slice(policy_digest);
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    binding
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: BuildOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(concat!(
        "SELECT build_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, cause, \
                vocation_before, ",
        family_columns!("", "_before"),
        ", vocation_after, ",
        family_columns!("", "_after"),
        ", stance_before FROM game_character_build_receipts \
          WHERE build_occurrence_id = encode($1,'hex')::uuid"
    ))
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedBuildChange, DurabilityError> {
    Ok(CommittedBuildChange {
        occurrence: BuildOccurrence(uuid_text(row.try_get("build_occurrence_id")?)?),
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "original_character_revision",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "committed_character_revision",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        cause: BuildCause::parse(row.try_get("cause")?)
            .ok_or(DurabilityError::InvalidStoredState)?,
        before: DurableBuildState::stored(row, "vocation_before", "_before")?,
        after: DurableBuildState::stored(row, "vocation_after", "_after")?,
        pruned_stance: row.try_get("stance_before")?,
    })
}

/// `req(L)`, the progress needed to go from level `L - 1` to `L`: `base x multiplier^(L - 11)`
/// in f64, truncated to an integer, as Canary computes it (SKILLS-0 §2, §3.1). `None` for a level
/// outside 11..=1000, a multiplier that is not finite and positive, or a requirement above
/// [`MAX_BUILD_PROGRESS`]: that level is unreachable.
#[must_use]
pub fn skill_tries_required(base: u16, multiplier: f64, level: u16) -> Option<u64> {
    if !(MIN_SKILL_LEVEL + 1..=MAX_BUILD_LEVEL).contains(&level)
        || !multiplier.is_finite()
        || multiplier <= 0.0
    {
        return None;
    }
    let required = f64::from(base) * multiplier.powf(f64::from(level - (MIN_SKILL_LEVEL + 1)));
    // Below 2^63 every f64 truncates into 0..=2^63 - 1024.
    (required.is_finite() && required < 9_223_372_036_854_775_808.0).then_some(required as u64)
}

/// A family's cumulative progress from its `floor` level: `req` of every level from `floor + 1`
/// to `level`, plus `progress`. Summed in checked 128-bit arithmetic, saturating at
/// [`MAX_BUILD_PROGRESS`]; an unreachable level on the way also saturates.
#[must_use]
pub fn cumulative_progress(
    level: u16,
    progress: u64,
    floor: u16,
    req: impl Fn(u16) -> Option<u64>,
) -> u64 {
    let total = (floor.saturating_add(1)..=level).try_fold(u128::from(progress), |total, level| {
        total.checked_add(u128::from(req(level)?))
    });
    total.map_or(MAX_BUILD_PROGRESS, |total| {
        u64::try_from(total.min(u128::from(MAX_BUILD_PROGRESS))).unwrap_or(MAX_BUILD_PROGRESS)
    })
}

/// Re-level cumulative progress from (`floor`, 0) (SKILLS-0 §3.3, §3.6): advance while the next
/// level is reachable and paid for, up to [`MAX_BUILD_LEVEL`]; the rest is progress.
#[must_use]
pub fn relevel(cumulative: u64, floor: u16, req: impl Fn(u16) -> Option<u64>) -> (u16, u64) {
    let (mut level, mut rest) = (floor, cumulative.min(MAX_BUILD_PROGRESS));
    while level < MAX_BUILD_LEVEL {
        match req(level + 1) {
            Some(required) if rest >= required => {
                rest -= required;
                level += 1;
            }
            _ => break,
        }
    }
    (level, rest)
}

#[cfg(test)]
mod tests {
    use super::{
        BuildCause, BuildFormula, DurableBuildState, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL,
        convert_vocation, cumulative_progress, normalized, relevel, skill_tries_required,
    };

    /// Skills: base 50, multiplier 2.0 without a vocation and 1.1 with one. Magic: 100 x L.
    struct Table;

    impl BuildFormula for Table {
        fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
            match (family, vocation) {
                (0, _) => Some(100 * u64::from(level)),
                (_, "none") => skill_tries_required(50, 2.0, level),
                _ => skill_tries_required(50, 1.1, level),
            }
        }

        fn digest(&self) -> [u8; 32] {
            [1; 32]
        }

        fn content_revision(&self) -> &str {
            "content-1"
        }
    }

    #[test]
    fn vocation_choice_keeps_cumulative_progress_under_the_new_multipliers() {
        let mut skills = [(10, 0); 7];
        // Sword 50 + 100 + 3 = 153 tries without a vocation.
        skills[2] = (12, 3);
        let before = DurableBuildState::new("none", (2, 40), skills).unwrap_or_default();
        let after = convert_vocation(&Table, &before, "knight").unwrap_or_default();
        // As a knight 153 = 50 + 55 + 48, below req(13) = 60; magic keeps (2, 40).
        assert_eq!(after.vocation(), "knight");
        assert_eq!(after.skills()[2], (12, 48));
        assert_eq!(after.magic(), (2, 40));
        assert!(normalized(&Table, &after));
        skills[2] = (10, 50);
        let unpaid = DurableBuildState::new("knight", (0, 0), skills).unwrap_or_default();
        assert!(!normalized(&Table, &unpaid), "50 tries pay for level 11");
        skills[2] = (1000, 0);
        let unreachable = DurableBuildState::new("none", (0, 0), skills).unwrap_or_default();
        assert!(!normalized(&Table, &unreachable), "req(69) is unreachable");
    }

    #[test]
    fn req_truncates_the_f64_power_and_stops_below_the_bound() {
        let sword = |level| skill_tries_required(50, 1.1, level);
        assert_eq!(sword(10), None);
        assert_eq!(sword(11), Some(50));
        assert_eq!(sword(12), Some(55));
        // 50 x 1.21 = 60.500000000000014 truncates to 60.
        assert_eq!(sword(13), Some(60));
        assert_eq!(sword(1001), None);
        // With 2.0, level 68 is the last reachable one (SKILLS-0 §3.1 "around level 67").
        assert_eq!(skill_tries_required(50, 2.0, 68), Some(50 << 57));
        assert_eq!(skill_tries_required(50, 2.0, 69), None);
        assert_eq!(skill_tries_required(50, f64::NAN, 12), None);
        assert_eq!(skill_tries_required(50, 0.0, 12), None);
    }

    #[test]
    fn cumulative_sums_saturate_and_relevel_round_trips() {
        let sword = |level| skill_tries_required(50, 1.1, level);
        assert_eq!(cumulative_progress(12, 3, MIN_SKILL_LEVEL, sword), 108);
        assert_eq!(relevel(108, MIN_SKILL_LEVEL, sword), (12, 3));
        assert_eq!(cumulative_progress(10, 7, MIN_SKILL_LEVEL, sword), 7);
        let doubling = |level| skill_tries_required(50, 2.0, level);
        // Levels 11..=68 total 50 x (2^58 - 1), above the bound: the sum saturates.
        assert_eq!(
            cumulative_progress(68, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL, doubling),
            MAX_BUILD_PROGRESS
        );
        assert_eq!(
            cumulative_progress(69, 0, MIN_SKILL_LEVEL, doubling),
            MAX_BUILD_PROGRESS
        );
        // The bound re-levels to 67 (levels 11..=67 total 50 x (2^57 - 1)).
        assert_eq!(
            relevel(MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL, doubling),
            (67, MAX_BUILD_PROGRESS - 50 * ((1 << 57) - 1))
        );
        assert_eq!(
            relevel(u64::MAX, 0, |_| Some(1)),
            (1000, MAX_BUILD_PROGRESS - 1000)
        );
    }

    #[test]
    fn state_bounds_and_cause_directions() {
        let seed = DurableBuildState::default();
        assert!(DurableBuildState::new("knight", (0, 0), [(9, 0); 7]).is_err());
        assert!(DurableBuildState::new("knight", (1001, 0), [(10, 0); 7]).is_err());
        assert!(
            DurableBuildState::new("knight", (0, MAX_BUILD_PROGRESS + 1), [(10, 0); 7]).is_err()
        );
        assert!(DurableBuildState::new("", (0, 0), [(10, 0); 7]).is_err());
        let knight = DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).unwrap_or_default();
        let trained = DurableBuildState::new("knight", (1, 0), [(10, 0); 7]).unwrap_or_default();
        let elite =
            DurableBuildState::new("elite_knight", (1, 0), [(10, 0); 7]).unwrap_or_default();
        assert!(BuildCause::VocationChoice.admits(&seed, &knight));
        assert!(!BuildCause::VocationChoice.admits(&knight, &elite));
        assert!(BuildCause::Training.admits(&knight, &trained));
        assert!(!BuildCause::Training.admits(&trained, &knight));
        assert!(!BuildCause::Training.admits(&knight, &knight));
        assert!(BuildCause::Promotion.admits(&trained, &elite));
        assert!(!BuildCause::Promotion.admits(&knight, &elite));
        assert_eq!(BuildCause::parse("promotion"), Some(BuildCause::Promotion));
        assert_eq!(BuildCause::parse("death"), None);
    }
}

// D1108 native owner extension. Existing v1 ordinary build bindings/replay stay
// unchanged; marker receipts have the v2 binding and their own lease/cause replay key.
use crate::training::offline::{
    LatestBuildReceipt, MarkerKey, MarkerKind, OfflineSkill, ProposedLogoutMarker,
    ProposedOfflineColumns,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeOfflineMarkerReceipt {
    pub(crate) key: MarkerKey,
    pub(crate) occurrence: BuildOccurrence,
    pub(crate) original_character_revision: CharacterRevision,
    pub(crate) committed_character_revision: CharacterRevision,
    pub(crate) session: crate::foundation::GameSessionId,
    pub(crate) scope: RuntimeScopeRefV1,
    pub(crate) as_of_us: i64,
    pub(crate) before_build: DurableBuildState,
    pub(crate) after_build: DurableBuildState,
    pub(crate) before_columns: ProposedOfflineColumns,
    pub(crate) after_columns: ProposedOfflineColumns,
}
pub(crate) struct NativeOfflineMarkerValues {
    pub(crate) key: MarkerKey,
    pub(crate) as_of_us: i64,
    pub(crate) before_build: DurableBuildState,
    pub(crate) after_build: DurableBuildState,
    pub(crate) before_columns: ProposedOfflineColumns,
    pub(crate) after_columns: ProposedOfflineColumns,
}
pub(crate) struct LockedOfflineBuild {
    pub(crate) build: DurableBuildState,
    pub(crate) columns: ProposedOfflineColumns,
    pub(crate) respawn_pending: bool,
    pub(crate) progression_present: bool,
}

fn decode_offline_columns(
    row: &sqlx::postgres::PgRow,
    suffix: &str,
) -> std::result::Result<ProposedOfflineColumns, DurabilityError> {
    let stamina = row.try_get::<i32, _>(format!("stamina_minutes{suffix}").as_str())?;
    let pool = row.try_get::<i32, _>(format!("offline_pool_s{suffix}").as_str())?;
    let skill = row.try_get::<String, _>(format!("offline_skill{suffix}").as_str())?;
    let columns = ProposedOfflineColumns {
        stamina_minutes: u16::try_from(stamina).map_err(|_| DurabilityError::InvalidStoredState)?,
        pool_s: u32::try_from(pool).map_err(|_| DurabilityError::InvalidStoredState)?,
        skill: if skill == "none" {
            None
        } else {
            Some(OfflineSkill::from_sql_key(&skill).ok_or(DurabilityError::InvalidStoredState)?)
        },
    };
    if columns.stamina_minutes > 2520 || columns.pool_s > 43200 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(columns)
}

pub(crate) async fn load_offline_marker(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    key: MarkerKey,
) -> std::result::Result<Option<NativeOfflineMarkerReceipt>, DurabilityError> {
    let row = sqlx::query(
        "SELECT r.*, build_occurrence_id::text AS occurrence_text, \
         marker_lease_generation::text AS marker_lease_text, \
         marker_game_session_id::text AS marker_session_text, \
         marker_world_id::text AS marker_world_text, \
         marker_channel_id::text AS marker_channel_text, \
         original_character_revision::text AS original_revision_text, \
         committed_character_revision::text AS committed_revision_text \
         FROM game_character_build_receipts r WHERE character_id=encode($1,'hex')::uuid \
         AND marker_lease_generation=$2::text::numeric(20,0) AND cause=$3",
    )
    .bind(key.character.as_bytes().as_slice())
    .bind(key.lease.to_string())
    .bind(key.kind.sql_key())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let binding: Vec<u8> = row.try_get("command_binding")?;
    let as_of_us: i64 = row.try_get("marker_as_of_us")?;
    let committed_ms: i64 = row.try_get("committed_at")?;
    if binding.len() != 33 || binding[0] != 2 || as_of_us < 0 || as_of_us / 1000 != committed_ms {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(NativeOfflineMarkerReceipt {
        key,
        occurrence: BuildOccurrence::from_bytes(uuid_text(row.try_get("occurrence_text")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            &row,
            "original_revision_text",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            &row,
            "committed_revision_text",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        session: crate::foundation::GameSessionId::decode(&uuid_text(
            row.try_get("marker_session_text")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        scope: RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&uuid_text(row.try_get("marker_world_text")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            crate::foundation::ChannelId::decode(&uuid_text(row.try_get("marker_channel_text")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
        as_of_us,
        before_build: DurableBuildState::stored(&row, "vocation_before", "_before")?,
        after_build: DurableBuildState::stored(&row, "vocation_after", "_after")?,
        before_columns: decode_offline_columns(&row, "_before")?,
        after_columns: decode_offline_columns(&row, "_after")?,
    }))
}

pub(crate) async fn lock_offline_build(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterGameplayFence,
) -> Result<LockedOfflineBuild> {
    let root = assert_gameplay_fence(tx, fence, node).await??;
    let character = fence.character_id.as_bytes().as_slice();
    let progression = sqlx::query(
        "SELECT character_revision::text, profile_revision, ruleset_revision, content_revision \
         FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE"
    ).bind(character).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    if let Some(row) = &progression
        && (numeric_u64(row, "character_revision")? != root.revision
            || !state_matches_root(row, &root))
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let respawn_pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)"
    ).bind(character).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    let build = sqlx::query(
        "SELECT * FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE"
    ).bind(character).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    Ok(LockedOfflineBuild {
        build: build
            .as_ref()
            .map(|row| DurableBuildState::stored(row, "vocation", ""))
            .transpose()?
            .unwrap_or_default(),
        columns: build
            .as_ref()
            .map(|row| decode_offline_columns(row, ""))
            .transpose()?
            .unwrap_or_default(),
        respawn_pending,
        progression_present: progression.is_some(),
    })
}

pub(crate) async fn latest_offline_build_receipt(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
) -> std::result::Result<LatestBuildReceipt, DurabilityError> {
    // A later death carrying build state invalidates an older logout. Do not search
    // for the latest *logout* independently of the whole build-carrying chain.
    let row = sqlx::query(
        "SELECT cause, marker_lease_generation::text AS lease FROM (\
           SELECT cause, marker_lease_generation, committed_character_revision \
           FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid \
           UNION ALL SELECT 'death', NULL, committed_character_revision \
           FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid AND vocation IS NOT NULL\
         ) chain ORDER BY committed_character_revision DESC LIMIT 1"
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(LatestBuildReceipt::Absent);
    };
    if row.try_get::<String, _>("cause")? != "logout" {
        return Ok(LatestBuildReceipt::OtherBuildCarryingReceipt);
    }
    let lease = row
        .try_get::<Option<String>, _>("lease")?
        .ok_or(DurabilityError::InvalidStoredState)?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let receipt = load_offline_marker(
        tx,
        MarkerKey {
            character: fence.character_id,
            lease,
            kind: MarkerKind::Logout,
        },
    )
    .await?
    .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(LatestBuildReceipt::Logout(ProposedLogoutMarker {
        key: receipt.key,
        session: receipt.session,
        scope: receipt.scope,
        // The native ms anchor is checked above against the exact DB-read us anchor.
        // Do not reinterpret the old committed_at millisecond field as microseconds.
        as_of_us: receipt.as_of_us,
        committed_at_us: receipt.as_of_us,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn write_offline_marker(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterGameplayFence,
    occurrence: BuildOccurrence,
    formula: &dyn BuildFormula,
    values: NativeOfflineMarkerValues,
) -> Result<NativeOfflineMarkerReceipt> {
    let NativeOfflineMarkerValues {
        key,
        as_of_us,
        before_build,
        after_build,
        before_columns,
        after_columns,
    } = values;
    if key.character != fence.character_id
        || key.lease != fence.character_lease_generation
        || as_of_us < 0
        || before_build.vocation != after_build.vocation
        || !normalized(formula, &after_build)
        || after_columns.stamina_minutes > 2520
        || after_columns.pool_s > 43200
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let (from, to) = (before_build.families(), after_build.families());
    let valid_direction = match key.kind {
        MarkerKind::Logout => {
            from.iter().zip(to).all(|(a, b)| b >= *a)
                && after_columns.stamina_minutes <= before_columns.stamina_minutes
                && after_columns.pool_s >= before_columns.pool_s
                && after_columns.skill == before_columns.skill
        }
        // The dedicated settlement source applies the source-qualified selected
        // family and conditional shielding before reaching this same-row writer.
        MarkerKind::Settlement => {
            from.iter().zip(to).all(|(a, b)| b >= *a)
                && after_columns.stamina_minutes >= before_columns.stamina_minutes
                && after_columns.skill.is_none()
        }
    };
    if !valid_direction {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let locked = lock_offline_build(tx, node, fence).await?;
    if locked.respawn_pending {
        return Err(CharacterProgressionError::RespawnPending);
    }
    if !locked.progression_present {
        return Err(CharacterProgressionError::MissingProgressionState);
    }
    if locked.build != before_build || locked.columns != before_columns {
        return Err(CharacterProgressionError::BuildStateMismatch);
    }
    let current = assert_gameplay_fence(tx, fence, node).await??;
    if current.content_revision != formula.content_revision() {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let next = current
        .revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut semantic = vec![2];
    semantic.extend_from_slice(occurrence.as_bytes());
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&key.lease.to_be_bytes());
    semantic.extend_from_slice(key.kind.sql_key().as_bytes());
    before_build.encode(&mut semantic);
    after_build.encode(&mut semantic);
    for c in [before_columns, after_columns] {
        semantic.extend_from_slice(&c.stamina_minutes.to_be_bytes());
        semantic.extend_from_slice(&c.pool_s.to_be_bytes());
        semantic.extend_from_slice(c.skill.map_or("none", OfflineSkill::sql_key).as_bytes());
        semantic.push(0);
    }
    let policy_digest = formula.digest();
    semantic.extend_from_slice(&policy_digest);
    let mut binding = vec![2];
    binding.extend_from_slice(&Sha256::digest(semantic));
    for sql in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
    ] {
        if sqlx::query(sql)
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(next.to_string())
            .bind(current.revision.to_string())
            .execute(&mut **tx)
            .await
            .map_err(DurabilityError::from)?
            .rows_affected()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
    }
    let sql = concat!(
        "INSERT INTO game_character_build_receipts(build_occurrence_id,command_binding,policy_digest,character_id,original_character_revision,committed_character_revision,cause,level_before,level_after,experience_before,experience_after,vocation_before,",
        family_columns!("", "_before"),
        ",vocation_after,",
        family_columns!("", "_after"),
        ",stance_before,stance_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at,stamina_minutes_before,offline_pool_s_before,offline_skill_before,stamina_minutes_after,offline_pool_s_after,offline_skill_after,marker_lease_generation,marker_game_session_id,marker_world_id,marker_channel_id,marker_as_of_us) ",
        "SELECT encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5::text::numeric(20,0),$6::text::numeric(20,0),$7,s.level,s.level,s.total_experience,s.total_experience,",
        "$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,",
        "$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38,$39,$40,$41,$42,",
        "NULL,NULL,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,$8,",
        "$43,$44,$45,$46,$47,$48,$49::text::numeric(20,0),encode($50,'hex')::uuid,encode($51,'hex')::uuid,encode($52,'hex')::uuid,$53 ",
        "FROM game_character_progression_state s WHERE s.character_id=encode($4,'hex')::uuid"
    );
    let query = sqlx::query(sql)
        .bind(occurrence.as_bytes().as_slice())
        .bind(binding)
        .bind(policy_digest.as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(current.revision.to_string())
        .bind(next.to_string())
        .bind(key.kind.sql_key())
        .bind(as_of_us / 1000);
    let query = after_build.bind(before_build.bind(query));
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(CharacterProgressionError::AuthorityRejected);
    };
    let query = query
        .bind(i32::from(before_columns.stamina_minutes))
        .bind(
            i32::try_from(before_columns.pool_s)
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        )
        .bind(before_columns.skill.map_or("none", OfflineSkill::sql_key))
        .bind(i32::from(after_columns.stamina_minutes))
        .bind(
            i32::try_from(after_columns.pool_s)
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        )
        .bind(after_columns.skill.map_or("none", OfflineSkill::sql_key))
        .bind(key.lease.to_string())
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .bind(as_of_us);
    if query
        .execute(&mut **tx)
        .await
        .map_err(DurabilityError::from)?
        .rows_affected()
        != 1
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let query = sqlx::query(concat!(
        "INSERT INTO game_character_build_state(character_id,vocation,",family_columns!("", ""),
        ",committed_character_revision,last_build_occurrence_id,stamina_minutes,offline_pool_s,offline_skill) VALUES (encode($1,'hex')::uuid,",
        "$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19::text::numeric(20,0),encode($20,'hex')::uuid,$21,$22,$23) ",
        "ON CONFLICT(character_id) DO UPDATE SET (vocation,",family_columns!("", ""),
        ",committed_character_revision,last_build_occurrence_id,stamina_minutes,offline_pool_s,offline_skill)=(EXCLUDED.vocation,",family_columns!("EXCLUDED.", ""),
        ",EXCLUDED.committed_character_revision,EXCLUDED.last_build_occurrence_id,EXCLUDED.stamina_minutes,EXCLUDED.offline_pool_s,EXCLUDED.offline_skill)"
    )).bind(fence.character_id.as_bytes().as_slice());
    let query = after_build
        .bind(query)
        .bind(next.to_string())
        .bind(occurrence.as_bytes().as_slice())
        .bind(i32::from(after_columns.stamina_minutes))
        .bind(
            i32::try_from(after_columns.pool_s)
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        )
        .bind(after_columns.skill.map_or("none", OfflineSkill::sql_key));
    if query
        .execute(&mut **tx)
        .await
        .map_err(DurabilityError::from)?
        .rows_affected()
        != 1
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    load_offline_marker(tx, key)
        .await?
        .ok_or(DurabilityError::InvalidStoredState.into())
}
