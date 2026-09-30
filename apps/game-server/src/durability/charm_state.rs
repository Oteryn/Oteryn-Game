//! Typed, fenced Character Charm commands (CHARM-3, migration 0019).
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
use super::character_progression::CurrentCharacterGameplayFence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmKey, CharmRuleError,
    CharmSlotEntitlement, CharmStage, derive_balance, plan_assign, plan_unlock,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgConnection;
use std::collections::BTreeMap;
use std::future::Future;

type Result<T> = std::result::Result<T, CharmStateError>;
const COMMAND_BINDING_VERSION: u8 = 1;
const CATALOGUE_DIGEST_VERSION: u8 = 1;
const MAX_REVISION_BYTES: usize = 128;
const KIND_UNLOCK: i16 = 1;
const KIND_ASSIGN: i16 = 2;
const CATEGORY_MAJOR: i16 = 1;
const CATEGORY_MINOR: i16 = 2;

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
/// CHARM-2 implements the Bestiary methods from its kill counters. No durable promotion or slot
/// entitlement exists yet, so production reports `false` and `Free` until one does.
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
    /// Commit one Charm command. Exact occurrence replay returns its retained result without
    /// reacquiring session authority; changed semantic reuse conflicts. A new occurrence is
    /// fenced like an XP award and validated against the stored unlocks and assignments and the
    /// earning facts, all read after the Character root is locked in the same transaction.
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

                    let root = match assert_charm_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    if request.catalogue_revision != root.content_revision {
                        return Ok(Err(CharmStateError::CharmContextMismatch));
                    }

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
                    if !state_matches_root(&state, &root) {
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

/// Current Character root facts proven under the gameplay fence.
struct FencedCharacterRoot {
    revision: u64,
    profile_revision: String,
    ruleset_revision: String,
    content_revision: String,
}

/// The current gameplay fence of a Charm command. It is the fence of
/// `character_progression::commit_character_experience` (that module's private
/// `assert_gameplay_fence`), kept identical: the caller has asserted the recovery fence and
/// taken the admission relation locks; this proves the live FND-04 session/lease/scope, the scope
/// assignment held by the current node incarnation, the admission guards, the live root
/// (row-locked) at the expected CharacterRevision and the current Game-owned interpretation.
async fn assert_charm_gameplay_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
) -> std::result::Result<std::result::Result<FencedCharacterRoot, CharmStateError>, DurabilityError>
{
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(Err(CharmStateError::AuthorityRejected));
    };
    let session = sqlx::query(
        "SELECT account_id::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id = encode($1,'hex')::uuid \
           AND character_id = encode($2,'hex')::uuid \
           AND world_id = encode($3,'hex')::uuid \
           AND runtime_scope_kind = 1 \
           AND runtime_scope_world_id = encode($3,'hex')::uuid \
           AND runtime_scope_channel_id = encode($4,'hex')::uuid \
           AND runtime_scope_instance_id IS NULL \
           AND current_generation = $5::text::numeric(20,0) \
           AND character_lease_generation = $6::text::numeric(20,0) \
           AND scope_ownership_generation = $7::text::numeric(20,0) \
           AND session_state IN (1,2) FOR SHARE",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.connection_generation.get().to_string())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(session) = session else {
        return Ok(Err(CharmStateError::AuthorityRejected));
    };
    let account_text: String = session.try_get("account_id")?;

    let key = scope_key(world_id, channel_id);
    let fact = node.fact();
    let assignment = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
           AND channel_id = encode($3,'hex')::uuid AND state = 1 \
           AND ownership_generation = $4::text::numeric(20,0) \
           AND holder_node_id = encode($5,'hex')::uuid \
           AND holder_registration_revision = $6::text::numeric(20,0) \
         FOR SHARE",
    )
    .bind(key.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    if assignment.is_none() || !prove_current_incarnation(tx, node).await? {
        return Ok(Err(CharmStateError::AuthorityRejected));
    }

    let guards_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 \
           FROM game_durability_admission_character_guards c \
           JOIN game_durability_admission_account_guards a \
             ON a.account_id = c.account_id \
           JOIN game_durability_admission_runtime_guards g \
             ON g.scope_key = $1 \
          WHERE c.character_id = encode($2,'hex')::uuid \
            AND c.account_id = $3::uuid \
            AND c.world_id = encode($4,'hex')::uuid \
            AND c.eligible \
            AND c.lease_generation = $5::text::numeric(20,0) \
            AND c.holder_game_session_id = encode($6,'hex')::uuid \
            AND a.presence_character_id = c.character_id \
            AND a.holder_game_session_id = c.holder_game_session_id \
            AND g.ready \
            AND g.ownership_generation = $7::text::numeric(20,0))",
    )
    .bind(key.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&account_text)
    .bind(world_id.as_bytes().as_slice())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_one(&mut **tx)
    .await?;
    if !guards_ok {
        return Ok(Err(CharmStateError::AuthorityRejected));
    }

    let root = sqlx::query(
        "SELECT account_id::text, world_id::text, character_revision::text, \
                profile_revision, ruleset_revision, content_revision, \
                starter_template_revision \
           FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 \
          FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(root) = root else {
        return Ok(Err(CharmStateError::AuthorityRejected));
    };
    if root.try_get::<String, _>("account_id")? != account_text
        || uuid_text(root.try_get("world_id")?)? != *world_id.as_bytes()
    {
        return Ok(Err(CharmStateError::AuthorityRejected));
    }
    let root_revision = numeric_u64(&root, "character_revision")?;
    if root_revision != fence.expected_character_revision.get() {
        return Ok(Err(CharmStateError::CharacterRevisionMismatch));
    }

    let current = sqlx::query(
        "SELECT profile_revision, ruleset_revision, content_revision, \
                starter_template_revision \
           FROM game_character_interpretations \
          ORDER BY interpretation_revision DESC LIMIT 1 FOR SHARE",
    )
    .fetch_optional(&mut **tx)
    .await?;
    let Some(current) = current else {
        return Ok(Err(CharmStateError::CharmContextMismatch));
    };
    for column in [
        "profile_revision",
        "ruleset_revision",
        "content_revision",
        "starter_template_revision",
    ] {
        if root.try_get::<String, _>(column)? != current.try_get::<String, _>(column)? {
            return Ok(Err(CharmStateError::CharmContextMismatch));
        }
    }
    Ok(Ok(FencedCharacterRoot {
        revision: root_revision,
        profile_revision: root.try_get("profile_revision")?,
        ruleset_revision: root.try_get("ruleset_revision")?,
        content_revision: root.try_get("content_revision")?,
    }))
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

fn valid_revision(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= MAX_REVISION_BYTES
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
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
          WHERE character_id = encode($1,'hex')::uuid ORDER BY charm_key",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
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
          WHERE character_id = encode($1,'hex')::uuid ORDER BY charm_key",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    for row in assignments {
        let key = CharmKey::new(row.try_get::<String, _>("charm_key")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let race = BestiaryRaceKey::new(row.try_get::<String, _>("race_key")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        state.assignments.insert(key, race);
    }
    Ok(state)
}

fn state_matches_root(row: &sqlx::postgres::PgRow, root: &FencedCharacterRoot) -> bool {
    row.try_get::<String, _>("profile_revision").ok().as_deref()
        == Some(root.profile_revision.as_str())
        && row.try_get::<String, _>("ruleset_revision").ok().as_deref()
            == Some(root.ruleset_revision.as_str())
        && row.try_get::<String, _>("content_revision").ok().as_deref()
            == Some(root.content_revision.as_str())
}

fn numeric_u64(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> std::result::Result<u64, DurabilityError> {
    row.try_get::<String, _>(column)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

fn uuid_text(value: &str) -> std::result::Result<[u8; 16], DurabilityError> {
    let hex: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if hex.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut out = [0_u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::charm::CharmDefinition;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
    };

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
