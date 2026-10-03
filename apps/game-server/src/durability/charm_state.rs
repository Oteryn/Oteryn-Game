//! Typed, fenced Character Charm commands (CHARM-3, migration 0020).
//!
//! Two commands exist: unlock the next stage of a charm, and assign an unlocked charm to a
//! Bestiary race. Each is one Character transaction under the same current gameplay fence as an
//! XP award (recovery fence, FND-04 session/lease/scope, current scope assignment and node
//! incarnation, the root row-locked at the expected CharacterRevision), validated against the
//! balance derived in that transaction, and advancing the CharacterRevision exactly once. There is
//! no unassign (owner answer 3c). Charm Points and Minor Charm Echoes are never stored (2a).
//!
//! Bestiary progress (CHARM-2), promotion and the slot entitlement are read through
//! [`CharmFacts`] inside the command's transaction. Earned currency only grows, and every writer
//! of the unlocks takes the same root lock and revision, so a double spend is impossible.

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
use crate::domain::bestiary::BestiaryRace;
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmBalance, CharmCatalogue, CharmCategory, CharmKey,
    CharmRuleError, CharmSlotEntitlement, CharmStage, derive_balance, plan_assign, plan_unlock,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgConnection;
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

type Result<T> = std::result::Result<T, CharmStateError>;
const COMMAND_BINDING_VERSION: u8 = 1;
const CATALOGUE_DIGEST_VERSION: u8 = 1;
const KIND_UNLOCK: i16 = 1;
const KIND_ASSIGN: i16 = 2;
const CATEGORY_MAJOR: i16 = 1;
const CATEGORY_MINOR: i16 = 2;
const MAX_VIEW_RACES: usize = oteryn_protocol_oteryn::bestiary::MAX_BESTIARY_VIEW_ENTRIES;
const MAX_VIEW_CHARMS: usize = oteryn_protocol_oteryn::charm::MAX_CHARMS;

/// The idempotency identity of one Charm command (UUIDv7), issued once per player command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharmCommandOccurrence([u8; 16]);

impl CharmCommandOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(CharmStateError::InvalidInput);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// The Character facts a Charm command depends on and that other slices own. Every method reads
/// within the Charm command's transaction (`connection`), after the Character root is locked.
/// The Bestiary methods must never report less progress than before for the same Character and
/// definition revision: kill counters saturate and never fall (§4.1).
///
/// [`BestiaryCharmFacts`] is the production implementation over the CHARM-2 kill counters.
pub trait CharmFacts: Send + Sync + 'static {
    /// The completed Bestiary stage of `race` for `character` (0 when the race is unknown).
    fn completed_stage<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        character: CharacterId,
        race: &'a BestiaryRaceKey,
    ) -> impl Future<Output = std::result::Result<BestiaryStage, DurabilityError>> + Send + 'a;

    /// The `charm_points` of each of `character`'s completed Bestiary entries.
    fn completed_entry_charm_points<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        character: CharacterId,
    ) -> impl Future<Output = std::result::Result<Vec<u32>, DurabilityError>> + Send + 'a;

    /// Whether `character` is promoted (100 Minor Charm Echoes).
    fn promoted<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        character: CharacterId,
    ) -> impl Future<Output = std::result::Result<bool, DurabilityError>> + Send + 'a;

    /// What bounds `character`'s number of assigned charms at this command.
    fn slot_entitlement<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        character: CharacterId,
    ) -> impl Future<Output = std::result::Result<CharmSlotEntitlement, DurabilityError>> + Send + 'a;
}

/// One Bestiary entry as the Charm rules see it: the race bound from its Creature definition
/// and the entry's `charm_points`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestiaryCharmEntry {
    pub race: BestiaryRace,
    pub charm_points: u32,
}

/// Production [`CharmFacts`] over the CHARM-2 kill counters (`game_character_bestiary_progress`),
/// read in the Charm command's transaction. Every counter writer advances the CharacterRevision
/// under the same root lock, so the counters read here are current. A counter of a race missing
/// from `entries` earns nothing and admits no assignment. No durable promotion or slot
/// entitlement exists yet: `promoted` is `false` and the entitlement `Free` until one does.
#[derive(Debug, Clone)]
pub struct BestiaryCharmFacts {
    entries: Arc<BTreeMap<String, BestiaryCharmEntry>>,
}

impl BestiaryCharmFacts {
    /// Rejects a duplicate race and a race key that is not a Bestiary race key.
    pub fn new(entries: impl IntoIterator<Item = BestiaryCharmEntry>) -> Result<Self> {
        let mut map = BTreeMap::new();
        for entry in entries {
            let key = BestiaryRaceKey::new(entry.race.key())
                .map_err(|_| CharmStateError::InvalidInput)?;
            if map.insert(key.as_str().to_owned(), entry).is_some() {
                return Err(CharmStateError::InvalidInput);
            }
            if map.len() > MAX_VIEW_RACES {
                return Err(CharmStateError::InvalidInput);
            }
        }
        Ok(Self {
            entries: Arc::new(map),
        })
    }
}

/// The completed stage of an entry: the final stage at the last threshold, otherwise the
/// thresholds reached, capped below the final stage.
fn bestiary_stage(race: &BestiaryRace, kill_count: u32) -> BestiaryStage {
    if race.is_complete(kill_count) {
        return BestiaryStage::FINAL;
    }
    let reached = u8::try_from(race.stages_unlocked(kill_count)).unwrap_or(u8::MAX);
    BestiaryStage::new(reached.min(BestiaryStage::FINAL.get() - 1)).unwrap_or(BestiaryStage::NONE)
}

impl CharmFacts for BestiaryCharmFacts {
    async fn completed_stage(
        &self,
        connection: &mut PgConnection,
        character: CharacterId,
        race: &BestiaryRaceKey,
    ) -> std::result::Result<BestiaryStage, DurabilityError> {
        let Some(entry) = self.entries.get(race.as_str()) else {
            return Ok(BestiaryStage::NONE);
        };
        let kill_count: Option<i64> = sqlx::query_scalar(
            "SELECT kill_count FROM game_character_bestiary_progress \
              WHERE character_id = encode($1,'hex')::uuid AND race_key = $2",
        )
        .bind(character.as_bytes().as_slice())
        .bind(race.as_str())
        .fetch_optional(&mut *connection)
        .await?;
        let kill_count = u32::try_from(kill_count.unwrap_or(0))
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(bestiary_stage(&entry.race, kill_count))
    }

    async fn completed_entry_charm_points(
        &self,
        connection: &mut PgConnection,
        character: CharacterId,
    ) -> std::result::Result<Vec<u32>, DurabilityError> {
        let keys: Vec<_> = self.entries.keys().cloned().collect();
        let rows = sqlx::query(
            "SELECT wanted.race_key, p.kill_count FROM unnest($2::text[]) wanted(race_key) \
               JOIN LATERAL (SELECT kill_count FROM game_character_bestiary_progress \
                  WHERE character_id = encode($1,'hex')::uuid AND race_key = wanted.race_key \
                  LIMIT 1) p ON true ORDER BY wanted.race_key",
        )
        .bind(character.as_bytes().as_slice())
        .bind(&keys)
        .fetch_all(&mut *connection)
        .await?;
        let mut points = Vec::new();
        for row in rows {
            let race_key: String = row.try_get("race_key")?;
            let kill_count = u32::try_from(row.try_get::<i64, _>("kill_count")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if let Some(entry) = self.entries.get(&race_key)
                && entry.race.is_complete(kill_count)
            {
                points.push(entry.charm_points);
            }
        }
        Ok(points)
    }

    async fn promoted(
        &self,
        _connection: &mut PgConnection,
        _character: CharacterId,
    ) -> std::result::Result<bool, DurabilityError> {
        Ok(false)
    }

    async fn slot_entitlement(
        &self,
        _connection: &mut PgConnection,
        _character: CharacterId,
    ) -> std::result::Result<CharmSlotEntitlement, DurabilityError> {
        Ok(CharmSlotEntitlement::Free)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharmCommand {
    UnlockNextStage {
        charm: CharmKey,
    },
    Assign {
        charm: CharmKey,
        race: BestiaryRaceKey,
    },
}

impl CharmCommand {
    #[must_use]
    pub const fn charm(&self) -> &CharmKey {
        match self {
            Self::UnlockNextStage { charm } | Self::Assign { charm, .. } => charm,
        }
    }
}

/// One Charm command. `catalogue` is the static charm catalogue of content revision
/// `catalogue_revision`, which must be the Character root's current content revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmCommandRequest {
    pub occurrence: CharmCommandOccurrence,
    pub command: CharmCommand,
    pub catalogue_revision: String,
    pub catalogue: CharmCatalogue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharmCommandEffect {
    Unlocked {
        /// 0 when the charm was locked.
        stage_before: u8,
        stage_after: CharmStage,
        cost: u32,
    },
    Assigned {
        race: BestiaryRaceKey,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedCharmCommand {
    pub occurrence: CharmCommandOccurrence,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub charm: CharmKey,
    pub category: CharmCategory,
    pub effect: CharmCommandEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharmCommandOutcome {
    Committed(CommittedCharmCommand),
    AlreadyCommitted(CommittedCharmCommand),
}

/// The stored Charm state of one Character. The balance is derived from it with
/// [`derive_balance`] and the Character's [`CharmFacts`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterCharmState {
    pub unlocks: BTreeMap<CharmKey, CharmStage>,
    pub assignments: BTreeMap<CharmKey, BestiaryRaceKey>,
}

/// The admitted content generation's bounded keys and progression catalogue.
#[allow(
    dead_code,
    reason = "standalone durability suites path-load these production types without Charm cases"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmProgressionReadRequest {
    pub catalogue_revision: String,
    pub catalogue: CharmCatalogue,
    pub races: Vec<BestiaryRaceKey>,
}

/// Both views' durable inputs, read together under one current gameplay fence and root lock.
#[allow(
    dead_code,
    reason = "standalone durability suites path-load these production types without Charm cases"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterCharmProgressionSnapshot {
    pub character_revision: CharacterRevision,
    pub state: CharacterCharmState,
    /// Only races of the admitted generation. Historical counters remain in storage.
    pub bestiary_counts: BTreeMap<BestiaryRaceKey, u32>,
    pub balance: CharmBalance,
    pub slot_entitlement: CharmSlotEntitlement,
}

#[derive(Debug)]
pub enum CharmStateError {
    InvalidInput,
    AuthorityRejected,
    MissingProgressionState,
    CharacterRevisionMismatch,
    CharmContextMismatch,
    ConflictingOccurrence,
    Rule(CharmRuleError),
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for CharmStateError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl std::fmt::Display for CharmStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid Charm command input"),
            Self::AuthorityRejected => formatter.write_str("Charm command authority rejected"),
            Self::MissingProgressionState => {
                formatter.write_str("Character progression state is missing")
            }
            Self::CharacterRevisionMismatch => {
                formatter.write_str("Character revision does not match")
            }
            Self::CharmContextMismatch => {
                formatter.write_str("Charm catalogue or Character context does not match")
            }
            Self::ConflictingOccurrence => {
                formatter.write_str("Charm command occurrence was reused with different semantics")
            }
            Self::Rule(error) => write!(formatter, "Charm rule rejected the command: {error:?}"),
            Self::Unavailable(error) => {
                write!(formatter, "Charm storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for CharmStateError {}

impl DurabilityRoot {
    /// Read Bestiary counters, charms, balances and entitlement at the same CharacterRevision.
    /// The revision supplied by the caller is expected evidence; all live authority is checked
    /// by the existing gameplay fence. Current-generation key probes bound historical reads.
    #[allow(
        dead_code,
        reason = "standalone durability suites path-load these production types without Charm cases"
    )]
    pub async fn read_character_charm_progression<F: CharmFacts>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        mut request: CharmProgressionReadRequest,
        facts: F,
    ) -> Result<CharacterCharmProgressionSnapshot> {
        if !valid_revision(&request.catalogue_revision)
            || request.races.len() > MAX_VIEW_RACES
            || request
                .catalogue
                .definitions()
                .take(MAX_VIEW_CHARMS + 1)
                .count()
                > MAX_VIEW_CHARMS
        {
            return Err(CharmStateError::InvalidInput);
        }
        request.races.sort_unstable();
        if request.races.windows(2).any(|keys| keys[0] == keys[1]) {
            return Err(CharmStateError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharmStateError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(fence_error(error))),
                    };
                    let row = sqlx::query(
                        "SELECT character_revision::text, profile_revision, ruleset_revision, \
                                content_revision FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(row) = row else {
                        return Ok(Err(CharmStateError::MissingProgressionState));
                    };
                    if numeric_u64(&row, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&row, &root)
                        || row.try_get::<String, _>("content_revision")?
                            != request.catalogue_revision
                    {
                        return Ok(Err(CharmStateError::CharmContextMismatch));
                    }
                    let state = load_charm_state(&mut tx, fence.character_id).await?;
                    let keys: Vec<_> = request.races.iter().map(BestiaryRaceKey::as_str).collect();
                    let rows = sqlx::query(
                        "SELECT wanted.race_key, p.kill_count \
                           FROM unnest($2::text[]) wanted(race_key) \
                           JOIN LATERAL (SELECT kill_count FROM game_character_bestiary_progress \
                             WHERE character_id = encode($1,'hex')::uuid \
                               AND race_key = wanted.race_key LIMIT 1) p ON true \
                          ORDER BY wanted.race_key",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(&keys)
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut bestiary_counts = BTreeMap::new();
                    for row in rows {
                        let key = BestiaryRaceKey::new(row.try_get::<String, _>("race_key")?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        let count = u32::try_from(row.try_get::<i64, _>("kill_count")?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        bestiary_counts.insert(key, count);
                    }
                    let points = facts
                        .completed_entry_charm_points(&mut tx, fence.character_id)
                        .await?;
                    let promoted = facts.promoted(&mut tx, fence.character_id).await?;
                    let slot_entitlement =
                        facts.slot_entitlement(&mut tx, fence.character_id).await?;
                    let balance =
                        derive_balance(&request.catalogue, &state.unlocks, points, promoted)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let result = CharacterCharmProgressionSnapshot {
                        character_revision: fence.expected_character_revision,
                        state,
                        bestiary_counts,
                        balance,
                        slot_entitlement,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }

    /// Commit one Charm command. Exact occurrence replay returns its retained result without
    /// reacquiring session authority; changed semantic reuse conflicts. A new occurrence is
    /// fenced like an XP award and validated against the stored unlocks and assignments and the
    /// earning facts, all read after the Character root is locked in the same transaction.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_charm_command<F: CharmFacts>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharmCommandRequest,
        facts: F,
    ) -> Result<CharmCommandOutcome> {
        validate_request(&fence, &request)?;
        let digest = catalogue_digest(&request.catalogue)?;
        let binding = command_binding(&fence, &request, &digest)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharmStateError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;

                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:character-charm:' || encode($1, 'hex'), 0))",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .execute(&mut *tx)
                    .await?;

                    if let Some(row) = load_receipt(&mut tx, request.occurrence).await? {
                        let stored: Vec<u8> = row.try_get("command_binding")?;
                        if stored != binding {
                            return Ok(Err(CharmStateError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(CharmCommandOutcome::AlreadyCommitted(committed)));
                    }

                    // The XP writer's own fence, so every Character write is fenced identically.
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(fence_error(error))),
                    };

                    let state = sqlx::query(
                        "SELECT character_revision::text, level, total_experience, \
                                profile_revision, ruleset_revision, content_revision, \
                                simulation_revision, evidence_revision, declaration_revision, \
                                policy_revision, reward_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharmStateError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    // The state's content revision equals the fenced root's, so the catalogue
                    // must belong to the Character's current content.
                    if !state_matches_root(&state, &root)
                        || state.try_get::<String, _>("content_revision")?
                            != request.catalogue_revision
                    {
                        return Ok(Err(CharmStateError::CharmContextMismatch));
                    }

                    let current = load_charm_state(&mut tx, fence.character_id).await?;
                    let effect = match &request.command {
                        CharmCommand::UnlockNextStage { charm } => {
                            let points = facts
                                .completed_entry_charm_points(&mut tx, fence.character_id)
                                .await?;
                            let promoted = facts.promoted(&mut tx, fence.character_id).await?;
                            let planned = derive_balance(
                                &request.catalogue,
                                &current.unlocks,
                                points,
                                promoted,
                            )
                            .and_then(|balance| {
                                plan_unlock(&request.catalogue, &current.unlocks, &balance, charm)
                            });
                            match planned {
                                Ok(plan) => (
                                    plan.category,
                                    CharmCommandEffect::Unlocked {
                                        stage_before: plan.stage_before,
                                        stage_after: plan.stage_after,
                                        cost: plan.cost,
                                    },
                                ),
                                Err(error) => return Ok(Err(CharmStateError::Rule(error))),
                            }
                        }
                        CharmCommand::Assign { charm, race } => {
                            let stage = facts
                                .completed_stage(&mut tx, fence.character_id, race)
                                .await?;
                            let entitlement =
                                facts.slot_entitlement(&mut tx, fence.character_id).await?;
                            match plan_assign(
                                &request.catalogue,
                                &current.unlocks,
                                &current.assignments,
                                charm,
                                race,
                                stage,
                                entitlement,
                            ) {
                                Ok(plan) => (
                                    plan.category,
                                    CharmCommandEffect::Assigned { race: plan.race },
                                ),
                                Err(error) => return Ok(Err(CharmStateError::Rule(error))),
                            }
                        }
                    };
                    let (category, effect) = effect;

                    let committed_revision = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let root_update = sqlx::query(
                        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root.revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if root_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let state_update = sqlx::query(
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root.revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if state_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }

                    let (kind, stage_before, stage_after, cost, race) = match &effect {
                        CharmCommandEffect::Unlocked {
                            stage_before,
                            stage_after,
                            cost,
                        } => (
                            KIND_UNLOCK,
                            Some(i16::from(*stage_before)),
                            Some(i16::from(stage_after.get())),
                            Some(i64::from(*cost)),
                            None,
                        ),
                        CharmCommandEffect::Assigned { race } => {
                            (KIND_ASSIGN, None, None, None, Some(race.as_str()))
                        }
                    };
                    let charm = request.command.charm();
                    let receipt = sqlx::query(
                        "INSERT INTO game_character_charm_receipts(\
                           charm_occurrence_id, command_binding, catalogue_digest, \
                           catalogue_revision, character_id, original_character_revision, \
                           committed_character_revision, level_before, level_after, \
                           experience_before, experience_after, command_kind, charm_key, \
                           charm_category, stage_before, stage_after, stage_cost, race_key, \
                           profile_revision, ruleset_revision, content_revision, \
                           simulation_revision, evidence_revision, declaration_revision, \
                           policy_revision, reward_revision, committed_at) \
                         SELECT encode($1,'hex')::uuid, $2, $3, $4, s.character_id, \
                           $5::text::numeric(20,0), $6::text::numeric(20,0), s.level, s.level, \
                           s.total_experience, s.total_experience, $7, $8, $9, $10, $11, $12, \
                           $13, s.profile_revision, s.ruleset_revision, s.content_revision, \
                           s.simulation_revision, s.evidence_revision, s.declaration_revision, \
                           s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($14,'hex')::uuid",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .bind(&binding)
                    .bind(digest.as_slice())
                    .bind(&request.catalogue_revision)
                    .bind(root.revision.to_string())
                    .bind(committed_revision.to_string())
                    .bind(kind)
                    .bind(charm.as_str())
                    .bind(category_code(category))
                    .bind(stage_before)
                    .bind(stage_after)
                    .bind(cost)
                    .bind(race)
                    .bind(fence.character_id.as_bytes().as_slice())
                    .execute(&mut *tx)
                    .await?;
                    if receipt.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }

                    match &effect {
                        CharmCommandEffect::Unlocked { stage_after, .. } => {
                            sqlx::query(
                                "INSERT INTO game_character_charm_unlocks(\
                                   character_id, charm_key, unlocked_stage, \
                                   committed_character_revision, last_charm_occurrence_id) \
                                 VALUES (encode($1,'hex')::uuid, $2, $3, \
                                   $4::text::numeric(20,0), encode($5,'hex')::uuid) \
                                 ON CONFLICT (character_id, charm_key) DO UPDATE \
                                   SET unlocked_stage = EXCLUDED.unlocked_stage, \
                                       committed_character_revision = \
                                         EXCLUDED.committed_character_revision, \
                                       last_charm_occurrence_id = \
                                         EXCLUDED.last_charm_occurrence_id",
                            )
                            .bind(fence.character_id.as_bytes().as_slice())
                            .bind(charm.as_str())
                            .bind(i16::from(stage_after.get()))
                            .bind(committed_revision.to_string())
                            .bind(request.occurrence.0.as_slice())
                            .execute(&mut *tx)
                            .await?;
                        }
                        CharmCommandEffect::Assigned { race } => {
                            sqlx::query(
                                "INSERT INTO game_character_charm_assignments(\
                                   character_id, charm_key, race_key, charm_category, \
                                   committed_character_revision, charm_occurrence_id) \
                                 VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                                   $5::text::numeric(20,0), encode($6,'hex')::uuid)",
                            )
                            .bind(fence.character_id.as_bytes().as_slice())
                            .bind(charm.as_str())
                            .bind(race.as_str())
                            .bind(category_code(category))
                            .bind(committed_revision.to_string())
                            .bind(request.occurrence.0.as_slice())
                            .execute(&mut *tx)
                            .await?;
                        }
                    }

                    let committed = CommittedCharmCommand {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        charm: charm.clone(),
                        category,
                        effect,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(CharmCommandOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Read a retained outcome after a lost response. This proves only what committed for the
    /// occurrence and never reacquires gameplay authority.
    pub async fn reconcile_charm_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: CharmCommandOccurrence,
    ) -> Result<Option<CommittedCharmCommand>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharmStateError::AuthorityRejected)?;
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

    /// The stored unlocks and assignments of one Character.
    pub async fn read_character_charm_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<CharacterCharmState> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharmStateError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let state = load_charm_state(&mut tx, character_id).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }
}

/// The shared fence refuses with `AuthorityRejected`, `CharacterRevisionMismatch` or
/// `ProgressionContextMismatch`; any other refusal it might add later still fails closed as an
/// authority rejection.
fn fence_error(error: CharacterProgressionError) -> CharmStateError {
    match error {
        CharacterProgressionError::CharacterRevisionMismatch => {
            CharmStateError::CharacterRevisionMismatch
        }
        CharacterProgressionError::ProgressionContextMismatch => {
            CharmStateError::CharmContextMismatch
        }
        CharacterProgressionError::Unavailable(error) => CharmStateError::Unavailable(error),
        _ => CharmStateError::AuthorityRejected,
    }
}

fn validate_request(
    fence: &CurrentCharacterGameplayFence,
    request: &CharmCommandRequest,
) -> Result<()> {
    if fence.character_lease_generation == 0
        || !valid_revision(&request.catalogue_revision)
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Err(CharmStateError::InvalidInput);
    }
    Ok(())
}

const fn category_code(category: CharmCategory) -> i16 {
    match category {
        CharmCategory::Major => CATEGORY_MAJOR,
        CharmCategory::Minor => CATEGORY_MINOR,
    }
}

fn push_text(encoded: &mut Vec<u8>, value: &str) -> Result<()> {
    let length = u16::try_from(value.len()).map_err(|_| CharmStateError::InvalidInput)?;
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
    Ok(())
}

/// Every definition in key order: key, category and the three stage costs.
fn catalogue_digest(catalogue: &CharmCatalogue) -> Result<[u8; 32]> {
    let mut encoded = vec![CATALOGUE_DIGEST_VERSION];
    for definition in catalogue.definitions() {
        push_text(&mut encoded, definition.key.as_str())?;
        encoded.extend_from_slice(&category_code(definition.category).to_be_bytes());
        for cost in definition.stage_costs {
            encoded.extend_from_slice(&cost.to_be_bytes());
        }
    }
    Ok(Sha256::digest(encoded).into())
}

/// The semantic identity of a command. Retry-local authority (session, connection, lease and
/// scope generations) is excluded so a reconnected replay of the same command matches.
fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &CharmCommandRequest,
    catalogue_digest: &[u8; 32],
) -> Result<Vec<u8>> {
    let mut semantic = vec![COMMAND_BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    match &request.command {
        CharmCommand::UnlockNextStage { charm } => {
            semantic.extend_from_slice(&KIND_UNLOCK.to_be_bytes());
            push_text(&mut semantic, charm.as_str())?;
        }
        CharmCommand::Assign { charm, race } => {
            semantic.extend_from_slice(&KIND_ASSIGN.to_be_bytes());
            push_text(&mut semantic, charm.as_str())?;
            push_text(&mut semantic, race.as_str())?;
        }
    }
    push_text(&mut semantic, &request.catalogue_revision)?;
    semantic.extend_from_slice(catalogue_digest);
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    Ok(binding)
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: CharmCommandOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT charm_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                command_kind, charm_key, charm_category, stage_before, stage_after, \
                stage_cost, race_key \
           FROM game_character_charm_receipts \
          WHERE charm_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedCharmCommand, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let occurrence = CharmCommandOccurrence(uuid_text(row.try_get("charm_occurrence_id")?)?);
    let character =
        CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?).map_err(invalid)?;
    let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
        .map_err(invalid)?;
    let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
        .map_err(invalid)?;
    let charm = CharmKey::new(row.try_get::<String, _>("charm_key")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let category = match row.try_get::<i16, _>("charm_category")? {
        CATEGORY_MAJOR => CharmCategory::Major,
        CATEGORY_MINOR => CharmCategory::Minor,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let effect = match row.try_get::<i16, _>("command_kind")? {
        KIND_UNLOCK => {
            let small = |column: &str| -> std::result::Result<u8, DurabilityError> {
                row.try_get::<Option<i16>, _>(column)?
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or(DurabilityError::InvalidStoredState)
            };
            CharmCommandEffect::Unlocked {
                stage_before: small("stage_before")?,
                stage_after: CharmStage::new(small("stage_after")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                cost: row
                    .try_get::<Option<i64>, _>("stage_cost")?
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(DurabilityError::InvalidStoredState)?,
            }
        }
        KIND_ASSIGN => CharmCommandEffect::Assigned {
            race: row
                .try_get::<Option<String>, _>("race_key")?
                .and_then(|value| BestiaryRaceKey::new(value).ok())
                .ok_or(DurabilityError::InvalidStoredState)?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(CommittedCharmCommand {
        occurrence,
        character_id: character,
        original_character_revision: original,
        committed_character_revision: committed,
        charm,
        category,
        effect,
    })
}

async fn load_charm_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
) -> std::result::Result<CharacterCharmState, DurabilityError> {
    let mut state = CharacterCharmState::default();
    let unlocks = sqlx::query(
        "SELECT charm_key, unlocked_stage FROM game_character_charm_unlocks \
          WHERE character_id = encode($1,'hex')::uuid ORDER BY charm_key LIMIT $2",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(i64::try_from(MAX_VIEW_CHARMS + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
    .fetch_all(&mut **tx)
    .await?;
    if unlocks.len() > MAX_VIEW_CHARMS {
        return Err(DurabilityError::InvalidStoredState);
    }
    for row in unlocks {
        let key = CharmKey::new(row.try_get::<String, _>("charm_key")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let stage = u8::try_from(row.try_get::<i16, _>("unlocked_stage")?)
            .ok()
            .and_then(|value| CharmStage::new(value).ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        state.unlocks.insert(key, stage);
    }
    let assignments = sqlx::query(
        "SELECT charm_key, race_key FROM game_character_charm_assignments \
          WHERE character_id = encode($1,'hex')::uuid ORDER BY charm_key LIMIT $2",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(i64::try_from(MAX_VIEW_CHARMS + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
    .fetch_all(&mut **tx)
    .await?;
    if assignments.len() > MAX_VIEW_CHARMS {
        return Err(DurabilityError::InvalidStoredState);
    }
    for row in assignments {
        let key = CharmKey::new(row.try_get::<String, _>("charm_key")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let race = BestiaryRaceKey::new(row.try_get::<String, _>("race_key")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        state.assignments.insert(key, race);
    }
    Ok(state)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::charm::CharmDefinition;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
    };

    #[test]
    fn production_bestiary_facts_bound_current_generation_keys() {
        let entries = |count| {
            (0..count).map(|i| BestiaryCharmEntry {
                race: BestiaryRace::new(
                    format!("oteryn:creature.race{i}"),
                    "definition-r1",
                    vec![1, 2, 3],
                )
                .expect("bounded race"),
                charm_points: 1,
            })
        };
        let maximal = BestiaryCharmFacts::new(entries(MAX_VIEW_RACES)).expect("maximum");
        assert_eq!(maximal.entries.len(), MAX_VIEW_RACES);
        assert!(matches!(
            BestiaryCharmFacts::new(entries(MAX_VIEW_RACES + 1)),
            Err(CharmStateError::InvalidInput)
        ));
    }

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn catalogue() -> CharmCatalogue {
        CharmCatalogue::new([
            CharmDefinition {
                key: CharmKey::new("oteryn:charm.wound").expect("key"),
                category: CharmCategory::Major,
                stage_costs: [240, 360, 1200],
            },
            CharmDefinition {
                key: CharmKey::new("oteryn:charm.gut").expect("key"),
                category: CharmCategory::Minor,
                stage_costs: [100, 150, 225],
            },
        ])
        .expect("catalogue")
    }

    fn request() -> CharmCommandRequest {
        CharmCommandRequest {
            occurrence: CharmCommandOccurrence::from_bytes(id(1)).expect("occurrence"),
            command: CharmCommand::Assign {
                charm: CharmKey::new("oteryn:charm.wound").expect("key"),
                race: BestiaryRaceKey::new("oteryn:creature.rat").expect("race"),
            },
            catalogue_revision: "content-1".into(),
            catalogue: catalogue(),
        }
    }

    fn fence() -> CurrentCharacterGameplayFence {
        CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(2)).expect("character"),
            game_session_id: GameSessionId::decode(&id(3)).expect("session"),
            connection_generation: ConnectionGeneration::new(1).expect("connection"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(4)).expect("world"),
                ChannelId::decode(&id(5)).expect("channel"),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("scope"),
            expected_character_revision: CharacterRevision::new(1).expect("revision"),
        }
    }

    fn binding(fence: &CurrentCharacterGameplayFence, request: &CharmCommandRequest) -> Vec<u8> {
        let digest = catalogue_digest(&request.catalogue).expect("digest");
        command_binding(fence, request, &digest).expect("binding")
    }

    #[test]
    fn binding_excludes_retry_local_authority_but_includes_every_semantic_input() {
        let original = binding(&fence(), &request());
        assert_eq!(original.len(), 33);

        let mut reconnected = fence();
        reconnected.game_session_id = GameSessionId::decode(&id(9)).expect("session");
        reconnected.connection_generation = ConnectionGeneration::new(7).expect("connection");
        reconnected.character_lease_generation = 8;
        reconnected.scope_ownership_generation = ScopeOwnershipGeneration::new(9).expect("scope");
        assert_eq!(binding(&reconnected, &request()), original);

        let mut revision = fence();
        revision.expected_character_revision = CharacterRevision::new(2).expect("revision");
        assert_ne!(binding(&revision, &request()), original);

        let mut race = request();
        race.command = CharmCommand::Assign {
            charm: CharmKey::new("oteryn:charm.wound").expect("key"),
            race: BestiaryRaceKey::new("oteryn:creature.wolf").expect("race"),
        };
        assert_ne!(binding(&fence(), &race), original);

        let mut unlock = request();
        unlock.command = CharmCommand::UnlockNextStage {
            charm: CharmKey::new("oteryn:charm.wound").expect("key"),
        };
        assert_ne!(binding(&fence(), &unlock), original);

        let mut revision_text = request();
        revision_text.catalogue_revision = "content-2".into();
        assert_ne!(binding(&fence(), &revision_text), original);

        let mut cost = request();
        cost.catalogue = CharmCatalogue::new([CharmDefinition {
            key: CharmKey::new("oteryn:charm.wound").expect("key"),
            category: CharmCategory::Major,
            stage_costs: [240, 360, 1201],
        }])
        .expect("catalogue");
        assert_ne!(binding(&fence(), &cost), original);
    }

    #[test]
    fn bestiary_stage_is_final_only_at_the_last_threshold() {
        let race = BestiaryRace::new("oteryn:creature.rat", "definition-r1", vec![25, 250, 500])
            .expect("race");
        for (kills, stage) in [
            (0, 0),
            (24, 0),
            (25, 1),
            (249, 1),
            (250, 2),
            (499, 2),
            (500, 3),
        ] {
            assert_eq!(bestiary_stage(&race, kills).get(), stage, "{kills} kills");
        }
        // A one-threshold entry is complete or nothing.
        let single =
            BestiaryRace::new("oteryn:creature.boss", "definition-r1", vec![5]).expect("race");
        assert_eq!(bestiary_stage(&single, 4), BestiaryStage::NONE);
        assert_eq!(bestiary_stage(&single, 5), BestiaryStage::FINAL);
    }

    #[test]
    fn bestiary_facts_reject_duplicate_and_non_bestiary_races() {
        let entry = |key: &str| BestiaryCharmEntry {
            race: BestiaryRace::new(key, "definition-r1", vec![1, 2, 3]).expect("race"),
            charm_points: 5,
        };
        assert!(BestiaryCharmFacts::new([entry("oteryn:creature.rat")]).is_ok());
        assert!(matches!(
            BestiaryCharmFacts::new([entry("oteryn:creature.rat"), entry("oteryn:creature.rat")]),
            Err(CharmStateError::InvalidInput)
        ));
        assert!(matches!(
            BestiaryCharmFacts::new([entry("oteryn:item.rat")]),
            Err(CharmStateError::InvalidInput)
        ));
    }

    #[test]
    fn catalogue_digest_covers_category_and_every_cost() {
        let base = catalogue_digest(&catalogue()).expect("digest");
        let recategorised = CharmCatalogue::new([
            CharmDefinition {
                key: CharmKey::new("oteryn:charm.wound").expect("key"),
                category: CharmCategory::Minor,
                stage_costs: [240, 360, 1200],
            },
            CharmDefinition {
                key: CharmKey::new("oteryn:charm.gut").expect("key"),
                category: CharmCategory::Minor,
                stage_costs: [100, 150, 225],
            },
        ])
        .expect("catalogue");
        assert_ne!(catalogue_digest(&recategorised).expect("digest"), base);
    }

    #[test]
    fn invalid_occurrence_revision_and_scope_fail_before_database_work() {
        assert!(matches!(
            CharmCommandOccurrence::from_bytes([0; 16]),
            Err(CharmStateError::InvalidInput)
        ));
        validate_request(&fence(), &request()).expect("valid");
        let mut revision = request();
        revision.catalogue_revision = "-bad".into();
        assert!(matches!(
            validate_request(&fence(), &revision),
            Err(CharmStateError::InvalidInput)
        ));
        let mut lease = fence();
        lease.character_lease_generation = 0;
        assert!(matches!(
            validate_request(&lease, &request()),
            Err(CharmStateError::InvalidInput)
        ));
        let mut instance = fence();
        instance.runtime_scope =
            RuntimeScopeRefV1::instance(WorldId::decode(&id(4)).expect("world"), id(6))
                .expect("instance scope");
        assert!(matches!(
            validate_request(&instance, &request()),
            Err(CharmStateError::InvalidInput)
        ));
    }
}
