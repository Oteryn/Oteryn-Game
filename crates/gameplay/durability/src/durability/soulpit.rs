//! W07 durable fights below the shared BossActivityHandoff owner (#2769).
//! No function here admits a session, assigns a scope or teleports a participant.
//! Entry composes in that owner's Type2 transaction; progress uses its real Instance fence.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{CurrentCharacterGameplayFence, assert_runtime_scope_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint_audit::Type2Transaction;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::domain::soulpit::{EncounterEffect, SoulpitEncounter, SoulpitError};
use crate::foundation::{CommandRef, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};

mod entry;
mod seal_entry;
pub use seal_entry::SoulsealReceipt;
mod progress;
mod scope_read;
mod terminal_read;
pub use scope_read::{SoulpitScopeBinding, SoulpitScopeMember};
pub use terminal_read::SoulpitTerminalCleanup;

/// Shared returns take the same encounter lock before session/root locks, matching
/// progress and its survivor writes. No return can race a final survival credit.
pub(super) async fn lock_shared_return(
    tx: &mut Transaction<'_, Postgres>,
    handoff: &[u8; 16],
) -> Result<bool, DurabilityError> {
    let source: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_soulpit_encounters f USING(handoff_id) WHERE g.handoff_id=encode($1,'hex')::uuid AND g.activity_key=$2)")
        .bind(handoff.as_slice()).bind(SOULPIT_ACTIVITY_KEY).fetch_one(&mut **tx).await?;
    if source {
        lock_handoff(tx, handoff).await?;
    }
    Ok(source)
}

/// The shared owner has already independently proven the current participant's
/// return or loss of its old owner. Close membership in that same transaction.
/// Crystal 201189f soulpit.lua:132-140: process loss never resumes a fight.
pub(super) async fn shared_returning(
    tx: &mut Transaction<'_, Postgres>,
    request: &super::activity_handoff::ActivityReturnRequest,
    recovering: bool,
) -> Result<(), DurabilityError> {
    let RuntimeScopeRefV1::Instance { instance_id, .. } = request.source.runtime_scope else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let row = sqlx::query("SELECT f.revision,f.checkpoint,f.terminal,s.staging_slot FROM game_soulpit_encounters f JOIN game_activity_handoff_groups g USING(handoff_id) JOIN game_soulpit_participants s USING(handoff_id) JOIN game_activity_runtime_returns r ON r.handoff_id=s.handoff_id AND r.character_id=s.character_id WHERE r.return_id=encode($1,'hex')::uuid AND r.handoff_id=encode($2,'hex')::uuid AND r.character_id=encode($3,'hex')::uuid AND r.source_game_session_id=encode($4,'hex')::uuid AND g.world_id=encode($5,'hex')::uuid AND g.instance_id=encode($6,'hex')::uuid AND g.activity_key=$7 AND g.state=2 AND g.ownership_generation=$8::text::numeric AND r.reason=$9 FOR UPDATE OF f")
        .bind(request.return_id.as_slice()).bind(request.handoff_id.as_slice())
        .bind(request.source.character_id.as_bytes().as_slice()).bind(request.source.game_session_id.as_bytes().as_slice())
        .bind(request.source.runtime_scope.world_id().as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(SOULPIT_ACTIVITY_KEY).bind(request.source.scope_ownership_generation.get().to_string())
        .bind(request.reason.stored()).fetch_optional(&mut **tx).await?;
    let row = row.ok_or(DurabilityError::InvalidStoredState)?;
    if row.try_get::<bool, _>("terminal")? {
        return Ok(());
    }
    let slot = u8::try_from(row.try_get::<i16, _>("staging_slot")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let revision = row.try_get::<i64, _>("revision")?;
    let next = revision
        .checked_add(1)
        .filter(|value| *value > 0)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut encounter = decode_checkpoint(row.try_get("checkpoint")?)?;
    let snapshot = encounter.snapshot();
    let mut now = snapshot.last_observed_at_micros;
    let action = if recovering
        || request.reason == super::activity_handoff::ActivityReturnReason::OwnerLost
    {
        SoulpitAction::Interrupt
    } else {
        if request.reason == super::activity_handoff::ActivityReturnReason::Timeout {
            // The authenticated shared Timeout receipt proves this wall-clock
            // deadline. Translate that fact to the retained encounter's clock;
            // never compare process monotonic origins across ownership recovery.
            now = now.max(
                snapshot
                    .started_at_micros
                    .checked_add(crate::domain::soulpit::TIME_LIMIT_MICROS)
                    .ok_or(DurabilityError::InvalidStoredState)?,
            );
        } else if !snapshot.surviving_slots.contains(&slot) {
            return Ok(());
        }
        SoulpitAction::Leave { slot }
    };
    let effect = progress::apply(&mut encounter, &action, now)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let state = checkpoint(&encounter)?;
    let terminal = encounter.snapshot().phase.is_terminal();
    let intent = json!({"version":1,"cause":"shared-runtime-return","return":hex(&request.return_id),"source_session":hex(request.source.game_session_id.as_bytes()),"reason":request.reason.stored(),"expected_revision":revision,"now":now,"action":action});
    sqlx::query("INSERT INTO game_soulpit_events(handoff_id,occurrence_id,revision,intent,checkpoint,effect,terminal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7)")
        .bind(request.handoff_id.as_slice()).bind(request.return_id.as_slice()).bind(next).bind(intent).bind(&state)
        .bind(serde_json::to_value(effect).map_err(|_|DurabilityError::InvalidStoredState)?).bind(terminal).execute(&mut **tx).await?;
    let changed=sqlx::query("UPDATE game_soulpit_encounters SET revision=$2,checkpoint=$3,terminal=$4,last_occurrence=encode($5,'hex')::uuid WHERE handoff_id=encode($1,'hex')::uuid AND revision=$6")
        .bind(request.handoff_id.as_slice()).bind(next).bind(state).bind(terminal).bind(request.return_id.as_slice()).bind(revision).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

pub(super) async fn write_shared_entry(
    root: &DurabilityRoot,
    tx: &mut Type2Transaction<'_>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &SoulpitEntryRequest,
    native: &NativeGameplayState,
) -> Result<SoulpitEntryWritten, SoulpitWriteError> {
    assert_recovery_fence(tx, recovery).await?;
    entry::write(root, tx, recovery, node, fence, request, native).await
}

pub const SOULPIT_ACTIVITY_KEY: &str = "oteryn:activity.soulpit";

/// Level, position and current trusted staff group are independently read inside the shared
/// transaction. The compatibility hint is not authority for a level exemption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulpitParticipantFact {
    pub character: CharacterId,
    /// Non-authoritative compatibility hint; the entry writer always resolves current staff.
    pub game_master: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoulpitFunding {
    Core { source: [u8;16], definition: crate::durability::item_mint::TypedDefinitionRef },
    Soulseals { species: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulpitEntryRequest {
    pub handoff_id: [u8; 16],
    pub command: CommandRef,
    pub funding: SoulpitFunding,
    pub intent: Vec<u8>,
    pub started_at_micros: u64,
    pub participants: Vec<SoulpitParticipantFact>,
}

#[derive(Debug, Clone)]
pub struct SoulpitEntryWritten {
    pub handoff_id: [u8; 16],
    pub species: String,
    pub source_transaction_id: [u8; 16],
    pub replayed: bool,
}

/// Only server actor/timer owners produce these inputs. Actor IDs are stable occurrence
/// identities within this handoff, including summons; the wire has no lifecycle command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoulpitAction {
    Advance,
    Spawned {
        wave: u8,
        monster_ids: [Option<u64>; 7],
    },
    Summoned {
        wave: u8,
        monster_id: u64,
        stack: u8,
    },
    Defeated {
        wave: u8,
        monster_id: u64,
    },
    Leave {
        slot: u8,
    },
    Rejoin {
        slot: u8,
    },
    Interrupt,
}

#[derive(Debug, Clone)]
pub struct SoulpitStepRequest {
    pub handoff_id: [u8; 16],
    pub occurrence: [u8; 16],
    pub runtime_scope: RuntimeScopeRefV1,
    pub scope_ownership_generation: ScopeOwnershipGeneration,
    pub expected_revision: u64,
    pub now_micros: u64,
    pub action: SoulpitAction,
    /// Complete current survivor set when this action can finish wave four. The caller holds
    /// each character's revision slot. A disconnected/dead/returned actor must leave first.
    pub survivors: Vec<CurrentCharacterGameplayFence>,
}

#[derive(Debug, Clone)]
pub struct SoulpitStepOutcome {
    pub revision: u64,
    pub encounter: SoulpitEncounter,
    pub effect: EncounterEffect,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct SoulpitStoredEncounter {
    pub revision: u64,
    pub species: String,
    pub encounter: SoulpitEncounter,
}

#[derive(Debug)]
pub enum SoulpitWriteError {
    Rule(SoulpitError),
    AuthorityRejected,
    StaleState,
    InsufficientSoulseals,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for SoulpitWriteError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for SoulpitWriteError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl From<SoulpitError> for SoulpitWriteError {
    fn from(error: SoulpitError) -> Self {
        Self::Rule(error)
    }
}
impl std::fmt::Display for SoulpitWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Soulpit write refused: {self:?}")
    }
}
impl std::error::Error for SoulpitWriteError {}

impl DurabilityRoot {
    /// Called while #2769's source item fence is still live, after it prepares the group,
    /// before it terminalizes sources. It never commits. Any later handoff failure rolls
    /// the entire debit, fight and audit back; the deferred SQL proof refuses partial work.
    #[allow(
        clippy::too_many_arguments,
        reason = "independent shared transaction, character, node, spell lane and native content authorities"
    )]
    pub async fn write_soulpit_entry_in_transaction(
        &self,
        tx: &mut Type2Transaction<'_>,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: &SoulpitEntryRequest,
        native: &NativeGameplayState,
    ) -> Result<SoulpitEntryWritten, SoulpitWriteError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| SoulpitWriteError::AuthorityRejected)?;
        self.write_soulpit_entry_with_recovery_in_transaction(
            tx, permit, &recovery, node, fence, request, native,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the shared handoff retains the same independent item and recovery authorities"
    )]
    pub(crate) async fn write_soulpit_entry_with_recovery_in_transaction(
        &self,
        tx: &mut Type2Transaction<'_>,
        permit: &SpellLanePermit,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: &SoulpitEntryRequest,
        native: &NativeGameplayState,
    ) -> Result<SoulpitEntryWritten, SoulpitWriteError> {
        assert_recovery_fence(tx, recovery).await?;
        permit.check_scope(fence.runtime_scope)?;
        entry::write(self, tx, recovery, node, fence, request, native).await
    }

    /// A retry returns its exact retained transition before live actor/fence checks. A new
    /// occurrence proves current Instance ownership and CAS revision and writes once.
    pub async fn advance_soulpit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: SoulpitStepRequest,
    ) -> Result<SoulpitStepOutcome, SoulpitWriteError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| SoulpitWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result = progress::write(&mut tx, &node, request).await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }

    /// Recovery reads never restart a stored fight. The caller submits Interrupt and asks
    /// BossActivityHandoff for its recovery/return disposition; this owner grants no placement.
    pub async fn read_soulpit_encounter(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        handoff_id: [u8; 16],
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<SoulpitStoredEncounter>, SoulpitWriteError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| SoulpitWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else {
                return Ok(Err(SoulpitWriteError::AuthorityRejected));
            };
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(Err(SoulpitWriteError::AuthorityRejected));
            }
            let row = sqlx::query("SELECT f.revision,f.species_key,f.checkpoint FROM game_soulpit_encounters f JOIN game_activity_handoff_groups g USING(handoff_id) WHERE f.handoff_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.instance_id=encode($3,'hex')::uuid AND g.state=2 AND g.ownership_generation<=$4::text::numeric")
                .bind(handoff_id.as_slice()).bind(scope.world_id().as_bytes().as_slice()).bind(instance_id.as_slice())
                .bind(generation.get().to_string()).fetch_optional(&mut *tx).await?;
            let result = row.map(|row| Ok(SoulpitStoredEncounter {
                revision: u64::try_from(row.try_get::<i64,_>("revision")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                species: row.try_get("species_key")?, encounter: decode_checkpoint(row.try_get("checkpoint")?)?,
            })).transpose();
            commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await?
    }
}

fn decode_checkpoint(value: Value) -> Result<SoulpitEncounter, DurabilityError> {
    let checkpoint =
        serde_json::from_value(value).map_err(|_| DurabilityError::InvalidStoredState)?;
    SoulpitEncounter::restore(
        checkpoint,
        oteryn_gameplay_entities::carrier::CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)
}

fn checkpoint(encounter: &SoulpitEncounter) -> Result<Value, DurabilityError> {
    serde_json::to_value(encounter.checkpoint()).map_err(|_| DurabilityError::InvalidStoredState)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_uuid_v7(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}

async fn lock_handoff(
    tx: &mut Transaction<'_, Postgres>,
    handoff: &[u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:soulpit:'||encode($1,'hex'),0))",
    )
    .bind(handoff.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
