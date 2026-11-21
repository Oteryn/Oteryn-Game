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

pub use crate::domain::character_build::{
    MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL, NO_VOCATION, SKILLS, cumulative_progress,
    relevel, skill_tries_required,
};

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

/// The persisted progression and revision context for a complete skills display.
#[derive(Debug, Clone)]
pub struct SkillDisplayProgression {
    pub level: u32,
    pub experience: u64,
    pub profile_revision: String,
    pub ruleset_revision: String,
    pub content_revision: String,
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
    pub(super) fn stored(
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

    /// Decode an ARRAY projection in family column order, retaining nullable death seeds.
    pub(super) fn stored_array(
        row: &sqlx::postgres::PgRow,
        vocation: &str,
        column: &str,
    ) -> std::result::Result<Option<Self>, DurabilityError> {
        let vocation: Option<String> = row.try_get(vocation)?;
        let values: Vec<Option<i64>> = row.try_get(column)?;
        if values.len() != 16 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let Some(vocation) = vocation else {
            return if values.iter().all(Option::is_none) {
                Ok(None)
            } else {
                Err(DurabilityError::InvalidStoredState)
            };
        };
        let mut families = [(0, 0); 8];
        for (family, pair) in families.iter_mut().zip(values.as_chunks::<2>().0) {
            *family = (
                u16::try_from(pair[0].ok_or(DurabilityError::InvalidStoredState)?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                u64::try_from(pair[1].ok_or(DurabilityError::InvalidStoredState)?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            );
        }
        let mut skills = [(0, 0); 7];
        skills.copy_from_slice(&families[1..]);
        Self::new(vocation, families[0], skills)
            .map(Some)
            .map_err(|_| DurabilityError::InvalidStoredState)
    }

    pub(super) fn family_values(&self) -> Vec<i64> {
        self.families()
            .into_iter()
            .flat_map(|(level, progress)| {
                [i64::from(level), i64::try_from(progress).unwrap_or(i64::MAX)]
            })
            .collect()
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

impl crate::domain::character_build::DeathBuildState for DurableBuildState {
    fn vocation(&self) -> &str {
        self.vocation()
    }

    fn magic(&self) -> (u16, u64) {
        self.magic()
    }

    fn skills(&self) -> [(u16, u64); 7] {
        self.skills()
    }

    fn from_death_families(
        vocation: &str,
        magic: (u16, u64),
        skills: [(u16, u64); 7],
    ) -> Option<Self> {
        Self::new(vocation, magic, skills).ok()
    }
}

/// The cause of a build change (A13 §4.2). There is no death cause: a death carries its loss in
/// the death receipt (A13 §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildCause {
    /// Vocation equal; no family lower, at least one higher.
    Training,
    /// RESTSTAM3: resting-area stamina restoration; build families stay equal.
    RestingStamina,
    /// `none` to a vocation key. The owner re-levels cumulative progress under the new vocation
    /// (SKILLS-0 §3.3) and binds the formula table in the policy digest.
    VocationChoice,
    /// S16c: a Dawnport trial tile switches to another vocation key below level 20, re-levelled
    /// like [`Self::VocationChoice`] (Crystal 201189f8 data-global/lib/others/dawnport.lua:26-120).
    VocationTrial,
    /// One vocation key to another; every family equal.
    Promotion,
    /// OFFLINE0: fenced terminal logout marker.
    Logout,
    /// OFFLINE0: once-per-lease admission settlement.
    OfflineSettlement,
}

impl BuildCause {
    const fn key(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::RestingStamina => "resting_stamina",
            Self::Logout => "logout",
            Self::OfflineSettlement => "offline_settlement",
            Self::VocationChoice => "vocation_choice",
            Self::VocationTrial => "vocation_trial",
            Self::Promotion => "promotion",
        }
    }

    fn parse(key: &str) -> Option<Self> {
        [
            Self::Training,
            Self::RestingStamina,
            Self::VocationChoice,
            Self::VocationTrial,
            Self::Promotion,
            Self::Logout,
            Self::OfflineSettlement,
        ]
        .into_iter()
        .find(|cause| cause.key() == key)
    }

    /// The direction of the 0030 cause CHECK, checked before any write.
    /// The causes whose `after` must be exactly [`convert_vocation`] of `before`.
    fn converts(self) -> bool {
        matches!(self, Self::VocationChoice | Self::VocationTrial)
    }

    /// Pending native training follows its Character into an activity Instance.
    /// Other build causes retain their Channel admission contract. This only
    /// admits the input shape; every new write still proves the complete current
    /// session, lease, runtime assignment and Character revision fence below.
    fn admits_runtime_scope(self, scope: RuntimeScopeRefV1) -> bool {
        match scope {
            RuntimeScopeRefV1::Channel { .. } => true,
            RuntimeScopeRefV1::Instance { .. } => matches!(self, Self::Training),
        }
    }

    fn admits(self, before: &DurableBuildState, after: &DurableBuildState) -> bool {
        let (from, to) = (before.families(), after.families());
        match self {
            Self::Logout => before == after,
            // A resting receipt is valid only with its increase-only OfflineChange.
            Self::RestingStamina => false,
            Self::OfflineSettlement => {
                before.vocation == after.vocation
                    && from.iter().zip(&to).all(|(from, to)| to >= from)
            }
            Self::Training => {
                before.vocation == after.vocation
                    && from.iter().zip(&to).all(|(from, to)| to >= from)
                    && from != to
            }
            Self::VocationChoice => before.vocation == NO_VOCATION && after.vocation != NO_VOCATION,
            Self::VocationTrial => {
                after.vocation != NO_VOCATION && before.vocation != after.vocation
            }
            Self::Promotion => {
                before.vocation != NO_VOCATION
                    && after.vocation != NO_VOCATION
                    && before.vocation != after.vocation
                    && from == to
            }
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

pub use crate::domain::character_build::BuildFormula;

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

include!("character_build/transaction.rs");

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

include!("character_build/persistence.rs");

include!("character_build/tests.rs");
