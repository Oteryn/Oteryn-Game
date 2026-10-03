//! Weapon Proficiency perk modification writer (PROFICIENCY-1B §4-§10, PROF-SHAPE-1a).
//!
//! One command changes one slot of one track under one `perk_modification` receipt: the
//! receipt advances the CharacterRevision, its one track line only advances the track's
//! committed revision, and its one modification line carries the slot's before and after values
//! and the bound cost (migration 0055). A command whose revision set moved between reservation
//! and commit is refused `REVISION_CHANGED` and kept as a terminal record outside the revision
//! chain. Replay by occurrence runs before any other check and never redraws.
//!
//! No value ledger exists before FORGE-1, so this slice admits only operations whose bound dust
//! and orb costs are 0; ORB_RANK also needs the orb stack lookup and stays closed. PROF-SHAPE-1b
//! adds the dust SPEND and orb BURN shapes. The capability check and the rate cap belong to the
//! command path: it calls [`DurabilityRoot::reconcile_character_proficiency_modification`]
//! first, charges [`ProficiencyModificationRateCap`](crate::domain::weapon_proficiency::ProficiencyModificationRateCap) only for an unseen occurrence, then
//! commits through a [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot).

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_proficiency::{
    ProficiencyCause, ProficiencyDefinitions, ProficiencyOccurrence, read::decode_state,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::weapon_proficiency::{
    ProficiencyModificationCommandKind, ProficiencyModificationCost, ProficiencyModificationInput,
    ProficiencyModificationOperation, ProficiencyModificationResult, ProficiencyModificationTrack,
    ProficiencyModifiedPerk, ProficiencyShapingRevision, plan_proficiency_modification,
    proficiency_modification_active, proficiency_shaping_seed,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type StoredResult<T> = std::result::Result<T, DurabilityError>;

/// Before FORGE-1 no dust ledger or orb BURN shape exists (migration 0055 pins both to 0).
const VALUE_LEDGER_AVAILABLE: bool = false;

/// The shaping definition of a Proficiency definition (PROFICIENCY-1B §3.1):
/// `oteryn:proficiency.tibia.p<Id>` is shaped by `oteryn:proficiency-shaping.tibia.p<Id>`.
pub fn proficiency_shaping_key(definition_key: &str) -> Option<String> {
    definition_key
        .strip_prefix("oteryn:proficiency.")
        .map(|rest| format!("oteryn:proficiency-shaping.{rest}"))
}

/// Semantic shaping content, as [`ProficiencyDefinitions`]: one immutable content generation.
pub trait ProficiencyShapingSource: Send + Sync {
    /// The active revision of a shaping key, or `None` when the key has no shaping content.
    fn active(&self, shaping_key: &str) -> Option<ProficiencyShapingRevision>;
    /// The canonical non-value identity of a selectable perk, comparable with pool entries.
    fn perk_identity(
        &self,
        definition_key: &str,
        definition_revision: &str,
        level: u8,
        perk_index: u8,
    ) -> Option<String>;
}

/// The revision set an occurrence binds at reservation (PROFICIENCY-1B §6.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProficiencyModificationBoundRevisions {
    pub definition_revision: String,
    pub shaping_revision: String,
    pub simulation_revision: String,
}

/// One modification command with its reserved occurrence and bound revisions. The occurrence
/// derives from the CommandId (PROFICIENCY-0 §4.3) and is reused by every retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyModificationCommand {
    occurrence: ProficiencyOccurrence,
    item_key: String,
    slot: u8,
    kind: ProficiencyModificationCommandKind,
    expected_track_revision: CharacterRevision,
    bound: ProficiencyModificationBoundRevisions,
}

fn reference(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

impl ProficiencyModificationCommand {
    pub fn new(
        occurrence: ProficiencyOccurrence,
        item_key: &str,
        slot: u8,
        kind: ProficiencyModificationCommandKind,
        expected_track_revision: CharacterRevision,
        bound: ProficiencyModificationBoundRevisions,
    ) -> Result<Self> {
        let valid_payload = match kind {
            ProficiencyModificationCommandKind::Modify { level } => level < 7,
            ProficiencyModificationCommandKind::ReshapeChoose { choice } => {
                choice.is_none_or(|choice| choice < 3)
            }
            _ => true,
        };
        if !reference(item_key)
            || !item_key.starts_with("oteryn:item.")
            || item_key.len() <= 12
            || !(1..=2).contains(&slot)
            || !valid_payload
            || ![
                &bound.definition_revision,
                &bound.shaping_revision,
                &bound.simulation_revision,
            ]
            .into_iter()
            .all(|value| reference(value))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            occurrence,
            item_key: item_key.to_owned(),
            slot,
            kind,
            expected_track_revision,
            bound,
        })
    }

    pub const fn occurrence(&self) -> ProficiencyOccurrence {
        self.occurrence
    }

    pub fn item_key(&self) -> &str {
        &self.item_key
    }

    pub const fn slot(&self) -> u8 {
        self.slot
    }

    pub const fn kind(&self) -> ProficiencyModificationCommandKind {
        self.kind
    }

    pub const fn expected_track_revision(&self) -> CharacterRevision {
        self.expected_track_revision
    }

    pub const fn bound(&self) -> &ProficiencyModificationBoundRevisions {
        &self.bound
    }

    /// Replay-or-conflict binding (PROFICIENCY-1B §6.1): operation, item, slot, level, choice,
    /// expected track revision and the bound revision set. The global CharacterRevision is not
    /// an input, so a losing writer retries under the same occurrence (§6.4).
    pub fn command_binding(&self, character_id: CharacterId) -> [u8; 33] {
        const VERSION: u8 = 1;
        let mut semantic = b"oteryn:character-proficiency-modification:command-binding\0".to_vec();
        semantic.push(VERSION);
        semantic.extend_from_slice(self.occurrence.as_bytes());
        semantic.extend_from_slice(character_id.as_bytes());
        for text in [
            self.item_key.as_str(),
            self.kind.operation().key(),
            &self.bound.definition_revision,
            &self.bound.shaping_revision,
            &self.bound.simulation_revision,
        ] {
            semantic.extend_from_slice(text.as_bytes());
            semantic.push(0);
        }
        semantic.push(self.slot);
        match self.kind {
            ProficiencyModificationCommandKind::Modify { level } => semantic.extend([1, level]),
            ProficiencyModificationCommandKind::ReshapeChoose { choice } => {
                semantic.extend([2, choice.unwrap_or(3)]);
            }
            _ => semantic.extend([0, 0]),
        }
        semantic.extend_from_slice(&self.expected_track_revision.get().to_be_bytes());
        let mut binding = [VERSION; 33];
        binding[1..].copy_from_slice(&Sha256::digest(&semantic));
        binding
    }
}

/// Runtime facts of the command that are not durable state: whether the actor stands in a
/// protection zone, and the server seed of `proficiency_shaping` draws.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProficiencyModificationContext {
    pub in_protection_zone: bool,
    pub server_seed: [u8; 32],
}

impl std::fmt::Debug for ProficiencyModificationContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProficiencyModificationContext")
            .field("in_protection_zone", &self.in_protection_zone)
            .finish_non_exhaustive()
    }
}

/// Immutable committed evidence of one `perk_modification` receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedProficiencyModification {
    pub character_id: CharacterId,
    pub occurrence_id: [u8; 16],
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub item_key: String,
    pub slot: u8,
    pub operation: ProficiencyModificationOperation,
    pub before: Option<ProficiencyModifiedPerk>,
    pub after: Option<ProficiencyModifiedPerk>,
    pub cost: ProficiencyModificationCost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProficiencyModificationOutcome {
    Committed(CommittedProficiencyModification),
    AlreadyCommitted(CommittedProficiencyModification),
    /// The occurrence's terminal `REVISION_CHANGED` record, first written or replayed.
    RevisionChanged,
    /// A §5 refusal; nothing was written.
    Refused(ProficiencyModificationResult),
}

impl ProficiencyModificationOutcome {
    /// The command result of this outcome (PROFICIENCY-1B §11.2).
    pub const fn result(&self) -> ProficiencyModificationResult {
        match self {
            Self::Committed(_) | Self::AlreadyCommitted(_) => {
                ProficiencyModificationResult::Accepted
            }
            Self::RevisionChanged => ProficiencyModificationResult::RevisionChanged,
            Self::Refused(result) => *result,
        }
    }
}

const OCCURRENCE_LOCK: &str = "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-proficiency:' || encode($1,'hex'),0))";

macro_rules! line_columns {
    () => {
        "m.item_key, m.slot, m.operation, m.bound_shaping_revision, \
    m.level_before, m.shaping_key_before, m.shaping_revision_before, m.entry_index_before, \
    m.rank_before, m.pending_offer_before, m.level_after, m.shaping_key_after, \
    m.shaping_revision_after, m.entry_index_after, m.rank_after, m.pending_offer_after, \
    m.dust_cost, m.dust_spent, m.orb_cost, m.orbs_spent, \
    coalesce(m.pending_offer_before IS NULL OR (array_ndims(m.pending_offer_before)=1 \
        AND array_lower(m.pending_offer_before,1)=1),false) AS canonical_offer_before, \
    coalesce(m.pending_offer_after IS NULL OR (array_ndims(m.pending_offer_after)=1 \
        AND array_lower(m.pending_offer_after,1)=1),false) AS canonical_offer_after"
    };
}

fn small(value: i16) -> StoredResult<u8> {
    u8::try_from(value).map_err(|_| DurabilityError::InvalidStoredState)
}

/// One stored side of a row or line: all value columns NULL is the cleared state.
pub(in crate::durability) fn decode_perk(
    row: &sqlx::postgres::PgRow,
    suffix: &str,
    canonical_offer: &str,
) -> StoredResult<Option<ProficiencyModifiedPerk>> {
    let column = |name: &str| format!("{name}{suffix}");
    let level: Option<i16> = row.try_get(column("level").as_str())?;
    let shaping_key: Option<String> = row.try_get(column("shaping_key").as_str())?;
    let shaping_revision: Option<String> = row.try_get(column("shaping_revision").as_str())?;
    let entry_index: Option<i16> = row.try_get(column("entry_index").as_str())?;
    let rank: Option<i16> = row.try_get(column("rank").as_str())?;
    let offer: Option<Vec<Option<i16>>> = row.try_get(column("pending_offer").as_str())?;
    if !row.try_get::<bool, _>(canonical_offer)? {
        return Err(DurabilityError::InvalidStoredState);
    }
    let (Some(level), Some(shaping_key), Some(shaping_revision), Some(entry_index), Some(rank)) =
        (level, shaping_key, shaping_revision, entry_index, rank)
    else {
        return if offer.is_none() {
            Ok(None)
        } else {
            Err(DurabilityError::InvalidStoredState)
        };
    };
    let pending_offer = offer
        .map(|offer| {
            let values = offer
                .into_iter()
                .map(|value| {
                    value
                        .ok_or(DurabilityError::InvalidStoredState)
                        .and_then(small)
                })
                .collect::<StoredResult<Vec<_>>>()?;
            <[u8; 3]>::try_from(values).map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()?;
    let perk = ProficiencyModifiedPerk {
        level: small(level)?,
        shaping_key,
        shaping_revision,
        entry_index: small(entry_index)?,
        rank: small(rank)?,
        pending_offer,
    };
    if !perk.is_well_formed() || !reference(&perk.shaping_revision) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(perk))
}

/// Columns of one side, in `level, shaping_key, shaping_revision, entry_index, rank,
/// pending_offer` order.
type SideColumns = (
    Option<i16>,
    Option<String>,
    Option<String>,
    Option<i16>,
    Option<i16>,
    Option<Vec<i16>>,
);

fn side(perk: Option<&ProficiencyModifiedPerk>) -> SideColumns {
    match perk {
        None => (None, None, None, None, None, None),
        Some(perk) => (
            Some(i16::from(perk.level)),
            Some(perk.shaping_key.clone()),
            Some(perk.shaping_revision.clone()),
            Some(i16::from(perk.entry_index)),
            Some(i16::from(perk.rank)),
            perk.pending_offer
                .map(|offer| offer.iter().map(|entry| i16::from(*entry)).collect()),
        ),
    }
}

struct LineWrite<'a> {
    character: CharacterId,
    occurrence: &'a [u8; 16],
    revision: &'a str,
    cause: ProficiencyCause,
    item_key: &'a str,
    slot: u8,
    operation: ProficiencyModificationOperation,
    bound_shaping_revision: Option<&'a str>,
    before: Option<&'a ProficiencyModifiedPerk>,
    after: Option<&'a ProficiencyModifiedPerk>,
    cost: ProficiencyModificationCost,
}

/// Write one modification line and set its row to the line's after values.
async fn write_modification(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    line: LineWrite<'_>,
) -> StoredResult<()> {
    let before = side(line.before);
    let after = side(line.after);
    let dust = i64::try_from(line.cost.dust).map_err(|_| DurabilityError::InvalidStoredState)?;
    let orbs = i16::from(line.cost.orbs);
    let inserted = sqlx::query(
        "INSERT INTO game_character_proficiency_modification_lines(proficiency_occurrence_id,\
         character_id,committed_character_revision,cause,item_key,slot,operation,bound_shaping_revision,\
         level_before,shaping_key_before,shaping_revision_before,entry_index_before,rank_before,\
         pending_offer_before,level_after,shaping_key_after,shaping_revision_after,entry_index_after,\
         rank_after,pending_offer_after,dust_cost,dust_spent,orb_cost,orbs_spent) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4,$5,$6,$7,$8,\
         $9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$21,$22,$22)",
    )
    .bind(line.occurrence.as_slice())
    .bind(line.character.as_bytes().as_slice())
    .bind(line.revision)
    .bind(line.cause.key())
    .bind(line.item_key)
    .bind(i16::from(line.slot))
    .bind(line.operation.key())
    .bind(line.bound_shaping_revision)
    .bind(before.0)
    .bind(before.1)
    .bind(before.2)
    .bind(before.3)
    .bind(before.4)
    .bind(before.5)
    .bind(after.0)
    .bind(after.1.clone())
    .bind(after.2.clone())
    .bind(after.3)
    .bind(after.4)
    .bind(after.5.clone())
    .bind(dust)
    .bind(orbs)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let updated = sqlx::query(
        "INSERT INTO game_character_proficiency_modifications(character_id,item_key,slot,level,\
         shaping_key,shaping_revision,entry_index,rank,pending_offer,committed_character_revision,\
         last_proficiency_occurrence_id) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,$8,$9,\
         $10::text::numeric(20,0),encode($11,'hex')::uuid) \
         ON CONFLICT(character_id,item_key,slot) DO UPDATE SET (level,shaping_key,shaping_revision,\
         entry_index,rank,pending_offer,committed_character_revision,last_proficiency_occurrence_id)=\
         (EXCLUDED.level,EXCLUDED.shaping_key,EXCLUDED.shaping_revision,EXCLUDED.entry_index,\
         EXCLUDED.rank,EXCLUDED.pending_offer,EXCLUDED.committed_character_revision,\
         EXCLUDED.last_proficiency_occurrence_id)",
    )
    .bind(line.character.as_bytes().as_slice())
    .bind(line.item_key)
    .bind(i16::from(line.slot))
    .bind(after.0)
    .bind(after.1)
    .bind(after.2)
    .bind(after.3)
    .bind(after.4)
    .bind(after.5)
    .bind(line.revision)
    .bind(line.occurrence.as_slice())
    .execute(&mut **tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

/// The stored rows of one track, by slot (index 0 is slot 1), locked for update.
async fn load_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    item_key: &str,
) -> StoredResult<[Option<ProficiencyModifiedPerk>; 2]> {
    let rows = sqlx::query(
        "SELECT slot, level, shaping_key, shaping_revision, entry_index, rank, pending_offer, \
         coalesce(pending_offer IS NULL OR (array_ndims(pending_offer)=1 \
             AND array_lower(pending_offer,1)=1),false) AS canonical_offer \
         FROM game_character_proficiency_modifications \
         WHERE character_id=encode($1,'hex')::uuid AND item_key=$2 ORDER BY slot FOR UPDATE",
    )
    .bind(character.as_bytes().as_slice())
    .bind(item_key)
    .fetch_all(&mut **tx)
    .await?;
    let mut slots = [None, None];
    for row in rows {
        let slot = usize::from(small(row.try_get("slot")?)?);
        let target = slots
            .get_mut(slot.wrapping_sub(1))
            .ok_or(DurabilityError::InvalidStoredState)?;
        *target = decode_perk(&row, "", "canonical_offer")?;
    }
    Ok(slots)
}

/// §8: hold the key's shared retention lock until commit and find the revision retained.
async fn retained(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    shaping_key: &str,
    revision: &str,
) -> StoredResult<bool> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock_shared(hashtextextended('proficiency_shaping:' || $1, 0))",
    )
    .bind(shaping_key)
    .execute(&mut **tx)
    .await?;
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_proficiency_shaping_revisions \
         WHERE shaping_key=$1 AND shaping_revision=$2)",
    )
    .bind(shaping_key)
    .bind(revision)
    .fetch_one(&mut **tx)
    .await?)
}

enum Replay {
    Unseen,
    Conflict,
    Found(Box<ProficiencyModificationOutcome>),
}

/// §6.1: a receipt, then a terminal record, by occurrence, before any write.
async fn replay(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    command: &ProficiencyModificationCommand,
) -> StoredResult<Replay> {
    let binding = command.command_binding(character);
    let reserved = command.occurrence();
    let occurrence = reserved.as_bytes();
    let receipt = sqlx::query(
        "SELECT character_id::text, cause, command_binding, original_character_revision::text, \
         committed_character_revision::text FROM game_character_proficiency_receipts \
         WHERE proficiency_occurrence_id=encode($1,'hex')::uuid",
    )
    .bind(occurrence.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = receipt {
        if row.try_get::<&str, _>("cause")? != ProficiencyCause::PerkModification.key()
            || row.try_get::<Vec<u8>, _>("command_binding")? != binding
            || uuid_text(row.try_get("character_id")?)? != *character.as_bytes()
        {
            return Ok(Replay::Conflict);
        }
        let line = sqlx::query(concat!(
            "SELECT ",
            line_columns!(),
            " FROM game_character_proficiency_modification_lines m \
             WHERE m.proficiency_occurrence_id=encode($1,'hex')::uuid"
        ))
        .bind(occurrence.as_slice())
        .fetch_one(&mut **tx)
        .await?;
        let committed = decode_committed(
            &line,
            character,
            *occurrence,
            numeric_u64(&row, "original_character_revision")?,
            numeric_u64(&row, "committed_character_revision")?,
        )?;
        return Ok(Replay::Found(Box::new(
            ProficiencyModificationOutcome::AlreadyCommitted(committed),
        )));
    }
    let terminal: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT command_binding FROM game_character_proficiency_modification_terminals \
         WHERE proficiency_occurrence_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid",
    )
    .bind(occurrence.as_slice())
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let foreign: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_proficiency_modification_terminals \
         WHERE proficiency_occurrence_id=encode($1,'hex')::uuid AND character_id<>encode($2,'hex')::uuid)",
    )
    .bind(occurrence.as_slice())
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    Ok(match terminal {
        Some(stored) if stored == binding => {
            Replay::Found(Box::new(ProficiencyModificationOutcome::RevisionChanged))
        }
        Some(_) => Replay::Conflict,
        None if foreign => Replay::Conflict,
        None => Replay::Unseen,
    })
}

fn decode_committed(
    line: &sqlx::postgres::PgRow,
    character: CharacterId,
    occurrence: [u8; 16],
    original: u64,
    committed: u64,
) -> StoredResult<CommittedProficiencyModification> {
    let revision =
        |value| CharacterRevision::new(value).map_err(|_| DurabilityError::InvalidStoredState);
    let unsigned =
        |value: i64| u64::try_from(value).map_err(|_| DurabilityError::InvalidStoredState);
    Ok(CommittedProficiencyModification {
        character_id: character,
        occurrence_id: occurrence,
        original_character_revision: revision(original)?,
        committed_character_revision: revision(committed)?,
        item_key: line.try_get("item_key")?,
        slot: small(line.try_get("slot")?)?,
        operation: ProficiencyModificationOperation::from_key(line.try_get("operation")?)
            .ok_or(DurabilityError::InvalidStoredState)?,
        before: decode_perk(line, "_before", "canonical_offer_before")?,
        after: decode_perk(line, "_after", "canonical_offer_after")?,
        cost: ProficiencyModificationCost {
            dust: unsigned(line.try_get("dust_spent")?)?,
            orbs: small(line.try_get("orbs_spent")?)?,
        },
    })
}

impl DurabilityRoot {
    /// One modification command of one track and slot, or nothing (PROFICIENCY-1B §5-§7).
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot), after the replay
    /// lookup and the rate cap.
    #[allow(
        clippy::too_many_arguments,
        reason = "the fences, the command, its runtime context and the two content sources"
    )]
    pub async fn commit_character_proficiency_modification(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: ProficiencyModificationCommand,
        context: ProficiencyModificationContext,
        definitions: std::sync::Arc<dyn ProficiencyDefinitions>,
        shaping: std::sync::Arc<dyn ProficiencyShapingSource>,
    ) -> Result<ProficiencyModificationOutcome> {
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
                    let character = fence.character_id;
                    sqlx::query(OCCURRENCE_LOCK)
                        .bind(command.occurrence().as_bytes().as_slice())
                        .execute(&mut *tx)
                        .await?;
                    match replay(&mut tx, character, &command).await? {
                        Replay::Found(outcome) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(*outcome));
                        }
                        Replay::Conflict => {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        Replay::Unseen => {}
                    }
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let state = sqlx::query(
                        "SELECT character_revision::text,profile_revision,ruleset_revision,\
                         content_revision,simulation_revision FROM game_character_progression_state \
                         WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(character.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root)
                        || definitions.content_revision()
                            != state.try_get::<String, _>("content_revision")?
                    {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let pending: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns \
                         WHERE character_id=encode($1,'hex')::uuid)",
                    )
                    .bind(character.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }
                    let refuse = |result| Ok(Ok(ProficiencyModificationOutcome::Refused(result)));
                    let resolved = definitions.resolve_weapon(command.item_key());
                    let active = resolved
                        .as_ref()
                        .and_then(|resolved| proficiency_shaping_key(&resolved.definition_key))
                        .and_then(|key| {
                            shaping.active(&key).filter(|active| {
                                active.shaping_key == key && active.is_well_formed()
                            })
                        });
                    let simulation: String = state.try_get("simulation_revision")?;
                    // §6.2: compare the bound set before admission; a moved set is terminal. A
                    // bound member that no longer resolves has moved too, so a later reappearance
                    // can never commit this occurrence.
                    if resolved.as_ref().map(|resolved| resolved.definition_revision.as_str())
                        != Some(command.bound().definition_revision.as_str())
                        || active.as_ref().map(|active| active.revision.as_str())
                            != Some(command.bound().shaping_revision.as_str())
                        || simulation != command.bound().simulation_revision
                    {
                        insert_terminal(&mut tx, character, &command).await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(ProficiencyModificationOutcome::RevisionChanged));
                    }
                    let (Some(resolved), Some(active)) = (resolved, active) else {
                        return refuse(ProficiencyModificationResult::NotAdmitted);
                    };
                    if resolved.canonical_item_key != command.item_key()
                        || !retained(&mut tx, &active.shaping_key, &active.revision).await?
                    {
                        return refuse(ProficiencyModificationResult::NotAdmitted);
                    }
                    let track = sqlx::query(
                        "SELECT *, committed_character_revision::text AS track_revision, \
                         coalesce(array_ndims(selections)=1 AND array_lower(selections,1)=1,false) \
                             AS canonical \
                         FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid \
                         AND item_key=$2 FOR UPDATE",
                    )
                    .bind(character.as_bytes().as_slice())
                    .bind(command.item_key())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let rows = load_rows(&mut tx, character, command.item_key()).await?;
                    let row = rows[usize::from(command.slot()) - 1].as_ref().filter(|row| {
                        row.shaping_key == active.shaping_key && row.shaping_revision == active.revision
                    });
                    // §3.3 plus this slice's gate: any cost needs the ledger, ORB_RANK the orb lookup.
                    if let Some(cost) = active.admitted(command.kind(), command.slot(), row)
                        && !VALUE_LEDGER_AVAILABLE
                        && (cost.dust > 0
                            || cost.orbs > 0
                            || matches!(command.kind(), ProficiencyModificationCommandKind::OrbRank))
                    {
                        return refuse(ProficiencyModificationResult::NotAdmitted);
                    }
                    let stored = track.as_ref().map(|row| decode_state(row, "")).transpose()?;
                    let track_revision = track
                        .as_ref()
                        .map(|row| numeric_u64(row, "track_revision"))
                        .transpose()?
                        .map(CharacterRevision::new)
                        .transpose()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if let Some(stored) = &stored
                        && (stored.definition_key() != resolved.definition_key
                            || stored.definition_revision() != resolved.definition_revision)
                    {
                        // The track awaits a migration; its committed revision will move.
                        return refuse(ProficiencyModificationResult::StaleRevision);
                    }
                    if stored
                        .as_ref()
                        .is_some_and(|stored| stored.validate_shape(resolved.shape).is_err())
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let progress = stored
                        .as_ref()
                        .map_or(0, |stored| u32::try_from(stored.progress()).unwrap_or(u32::MAX));
                    let selected_identity = match (command.kind(), &stored) {
                        (ProficiencyModificationCommandKind::Modify { level }, Some(stored)) => stored
                            .selections()
                            .get(usize::from(level))
                            .copied()
                            .flatten()
                            .and_then(|index| {
                                shaping.perk_identity(
                                    &resolved.definition_key,
                                    &resolved.definition_revision,
                                    level,
                                    index,
                                )
                            }),
                        _ => None,
                    };
                    let seed = proficiency_shaping_seed(
                        &context.server_seed,
                        &simulation,
                        &active.revision,
                        command.occurrence().as_bytes(),
                    );
                    let input = ProficiencyModificationInput {
                        slot: command.slot(),
                        command: command.kind(),
                        expected_track_revision: command.expected_track_revision(),
                        in_protection_zone: context.in_protection_zone,
                        active: &active,
                        track: stored.as_ref().zip(track_revision).map(|(stored, revision)| {
                            ProficiencyModificationTrack {
                                shape: resolved.shape,
                                progress,
                                selections: stored.selections(),
                                committed_track_revision: revision,
                                slots: [rows[0].as_ref(), rows[1].as_ref()],
                            }
                        }),
                        selected_identity: selected_identity.as_deref(),
                        seed,
                    };
                    let plan = match plan_proficiency_modification(&input) {
                        Ok(plan) => plan,
                        Err(result) => return refuse(result),
                    };
                    if plan.cost.dust > 0 || plan.cost.orbs > 0 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    // The row's own revision is retained under the same shared lock (§8).
                    if let Some(after) = &plan.after
                        && after.shaping_revision != active.revision
                        && !retained(&mut tx, &after.shaping_key, &after.shaping_revision).await?
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let next = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let (original, committed) = (root.revision.to_string(), next.to_string());
                    for sql in [
                        "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) \
                         WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
                        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) \
                         WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
                    ] {
                        if sqlx::query(sql)
                            .bind(character.as_bytes().as_slice())
                            .bind(&committed)
                            .bind(&original)
                            .execute(&mut *tx)
                            .await?
                            .rows_affected()
                            != 1
                        {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let reserved = command.occurrence();
                    let occurrence = reserved.as_bytes();
                    let binding = command.command_binding(character);
                    let header = sqlx::query(
                        "INSERT INTO game_character_proficiency_receipts(proficiency_occurrence_id,\
                         command_binding,policy_digest,character_id,original_character_revision,\
                         committed_character_revision,cause,level_before,level_after,experience_before,\
                         experience_after,profile_revision,ruleset_revision,content_revision,\
                         simulation_revision,evidence_revision,declaration_revision,policy_revision,\
                         reward_revision,committed_at) \
                         SELECT encode($1,'hex')::uuid,$2,$3,character_id,$5::text::numeric(20,0),\
                         $6::text::numeric(20,0),'perk_modification',level,level,total_experience,\
                         total_experience,profile_revision,ruleset_revision,content_revision,\
                         simulation_revision,evidence_revision,declaration_revision,policy_revision,\
                         reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                         FROM game_character_progression_state WHERE character_id=encode($4,'hex')::uuid",
                    )
                    .bind(occurrence.as_slice())
                    .bind(binding.as_slice())
                    .bind(definitions.digest().as_slice())
                    .bind(character.as_bytes().as_slice())
                    .bind(&original)
                    .bind(&committed)
                    .execute(&mut *tx)
                    .await?;
                    let track_line = sqlx::query(
                        "INSERT INTO game_character_proficiency_receipt_lines(proficiency_occurrence_id,\
                         character_id,committed_character_revision,cause,item_key,definition_key_before,\
                         definition_revision_before,progress_before,selections_before,definition_key_after,\
                         definition_revision_after,progress_after,selections_after) \
                         SELECT encode($1,'hex')::uuid,character_id,$2::text::numeric(20,0),\
                         'perk_modification',item_key,definition_key,definition_revision,progress,selections,\
                         definition_key,definition_revision,progress,selections \
                         FROM game_character_proficiency WHERE character_id=encode($3,'hex')::uuid AND item_key=$4",
                    )
                    .bind(occurrence.as_slice())
                    .bind(&committed)
                    .bind(character.as_bytes().as_slice())
                    .bind(command.item_key())
                    .execute(&mut *tx)
                    .await?;
                    let track_row = sqlx::query(
                        "UPDATE game_character_proficiency SET committed_character_revision=\
                         $3::text::numeric(20,0),last_proficiency_occurrence_id=encode($4,'hex')::uuid \
                         WHERE character_id=encode($1,'hex')::uuid AND item_key=$2",
                    )
                    .bind(character.as_bytes().as_slice())
                    .bind(command.item_key())
                    .bind(&committed)
                    .bind(occurrence.as_slice())
                    .execute(&mut *tx)
                    .await?;
                    if [header, track_line, track_row]
                        .iter()
                        .any(|written| written.rows_affected() != 1)
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    write_modification(
                        &mut tx,
                        LineWrite {
                            character,
                            occurrence,
                            revision: &committed,
                            cause: ProficiencyCause::PerkModification,
                            item_key: command.item_key(),
                            slot: plan.slot,
                            operation: plan.operation,
                            bound_shaping_revision: Some(&command.bound().shaping_revision),
                            before: plan.before.as_ref(),
                            after: plan.after.as_ref(),
                            cost: plan.cost,
                        },
                    )
                    .await?;
                    let result = CommittedProficiencyModification {
                        character_id: character,
                        occurrence_id: *occurrence,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(next)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        item_key: command.item_key().to_owned(),
                        slot: plan.slot,
                        operation: plan.operation,
                        before: plan.before,
                        after: plan.after,
                        cost: plan.cost,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(ProficiencyModificationOutcome::Committed(result)))
                })
            })
            .await?
    }

    /// Replay lookup and reconciliation (PROFICIENCY-1B §6.1, §6.3, §9): the committed receipt
    /// or the terminal record of the command's occurrence, or `None` for an unseen occurrence.
    /// A known occurrence with a different binding is `ConflictingOccurrence`. It needs no
    /// gameplay session, reads no current content and never restores controller authority.
    pub async fn reconcile_character_proficiency_modification(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        command: ProficiencyModificationCommand,
    ) -> Result<Option<ProficiencyModificationOutcome>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let result = match replay(&mut tx, character, &command).await? {
                        Replay::Found(outcome) => Some(*outcome),
                        Replay::Conflict => {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        Replay::Unseen => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }
}

async fn insert_terminal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    command: &ProficiencyModificationCommand,
) -> StoredResult<()> {
    let inserted = sqlx::query(
        "INSERT INTO game_character_proficiency_modification_terminals(proficiency_occurrence_id,\
         character_id,command_binding,item_key,slot,operation,bound_definition_revision,\
         bound_shaping_revision,bound_simulation_revision,expected_track_revision,level,choice,\
         refusal,recorded_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8,$9,\
         $10::text::numeric(20,0),$11,$12,'REVISION_CHANGED',\
         floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(command.occurrence().as_bytes().as_slice())
    .bind(character.as_bytes().as_slice())
    .bind(command.command_binding(character).as_slice())
    .bind(command.item_key())
    .bind(i16::from(command.slot()))
    .bind(command.kind().operation().key())
    .bind(&command.bound().definition_revision)
    .bind(&command.bound().shaping_revision)
    .bind(&command.bound().simulation_revision)
    .bind(command.expected_track_revision().get().to_string())
    .bind(match command.kind() {
        ProficiencyModificationCommandKind::Modify { level } => Some(i16::from(level)),
        _ => None,
    })
    .bind(match command.kind() {
        ProficiencyModificationCommandKind::ReshapeChoose { choice } => {
            Some(i16::from(choice.unwrap_or(3)))
        }
        _ => None,
    })
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

/// §9: every terminal record of one Character recomputes its stored binding from its own
/// persisted command fields, so a terminal whose binding disagrees with them fails.
async fn verify_terminals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> StoredResult<()> {
    let rows = sqlx::query(
        "SELECT proficiency_occurrence_id::text AS occurrence, command_binding, item_key, slot, \
         operation, bound_definition_revision, bound_shaping_revision, bound_simulation_revision, \
         expected_track_revision::text AS expected, level, choice \
         FROM game_character_proficiency_modification_terminals \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let invalid = || DurabilityError::InvalidStoredState;
    for row in rows {
        let level: Option<i16> = row.try_get("level")?;
        let choice: Option<i16> = row.try_get("choice")?;
        let kind = match (row.try_get::<&str, _>("operation")?, level, choice) {
            ("MODIFY", Some(level), None) => ProficiencyModificationCommandKind::Modify {
                level: small(level)?,
            },
            ("RANK_UP", None, None) => ProficiencyModificationCommandKind::RankUp,
            ("ORB_RANK", None, None) => ProficiencyModificationCommandKind::OrbRank,
            ("RESHAPE_OFFER", None, None) => ProficiencyModificationCommandKind::ReshapeOffer,
            ("RESHAPE_CHOOSE", None, Some(choice)) => {
                ProficiencyModificationCommandKind::ReshapeChoose {
                    choice: Some(small(choice)?).filter(|choice| *choice < 3),
                }
            }
            ("CLEAR", None, None) => ProficiencyModificationCommandKind::Clear,
            _ => return Err(invalid()),
        };
        let expected = row
            .try_get::<&str, _>("expected")?
            .parse::<u64>()
            .ok()
            .and_then(|value| CharacterRevision::new(value).ok())
            .ok_or_else(invalid)?;
        let command = ProficiencyModificationCommand::new(
            ProficiencyOccurrence::from_bytes(uuid_text(row.try_get("occurrence")?)?)
                .map_err(|_| invalid())?,
            row.try_get("item_key")?,
            small(row.try_get("slot")?)?,
            kind,
            expected,
            ProficiencyModificationBoundRevisions {
                definition_revision: row.try_get("bound_definition_revision")?,
                shaping_revision: row.try_get("bound_shaping_revision")?,
                simulation_revision: row.try_get("bound_simulation_revision")?,
            },
        )
        .map_err(|_| invalid())?;
        if command.command_binding(character).as_slice()
            != row.try_get::<Vec<u8>, _>("command_binding")?.as_slice()
        {
            return Err(invalid());
        }
    }
    Ok(())
}

/// PROFICIENCY-1B §10: whether a selection change or clear at `level` would replace the perk
/// under an active modification (`MODIFIED_LEVEL`). Called by the PROF-1 writer under its locks.
pub(in crate::durability) async fn level_has_active_modification(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    track: &super::character_proficiency::DurableProficiencyState,
    shape: crate::domain::weapon_proficiency::ProficiencySelectionShape,
    level: usize,
) -> StoredResult<bool> {
    let rows = load_rows(tx, character, track.item_key()).await?;
    let progress = u32::try_from(track.progress()).unwrap_or(u32::MAX);
    Ok(rows.iter().zip(1u8..).any(|(row, slot)| {
        row.as_ref().is_some_and(|row| {
            usize::from(row.level) == level
                && proficiency_modification_active(shape, progress, track.selections(), slot, row)
        })
    }))
}

/// PROFICIENCY-1B §4.3: a migration receipt clears, with no refund, every modified row whose
/// level the migration changes (its selection changes or the level no longer exists), with one
/// `MIGRATION_CLEAR` line each. Called by the PROF-1 writer after it wrote the track line.
pub(in crate::durability) async fn clear_migrated_modifications(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: &[u8; 16],
    revision: &str,
    line: &super::character_proficiency::ProficiencyLineCandidate,
) -> StoredResult<()> {
    let rows = load_rows(tx, character, line.before().item_key()).await?;
    for (row, slot) in rows.iter().zip(1u8..) {
        let Some(row) = row else { continue };
        let level = usize::from(row.level);
        let before = line.before().selections().get(level).copied().flatten();
        let after = line.after().selections().get(level).copied().flatten();
        if level < line.after().selections().len() && before == after {
            continue;
        }
        write_modification(
            tx,
            LineWrite {
                character,
                occurrence,
                revision,
                cause: ProficiencyCause::Migration,
                item_key: line.before().item_key(),
                slot,
                operation: ProficiencyModificationOperation::MigrationClear,
                bound_shaping_revision: None,
                before: Some(row),
                after: None,
                cost: ProficiencyModificationCost::default(),
            },
        )
        .await?;
    }
    Ok(())
}

/// §9 retained verification of one Character's modification rows, lines and terminal records:
/// every `perk_modification` receipt has one modification line of its one track line's track;
/// each (track, slot) chain starts cleared and follows its previous line; each row equals its
/// latest line; costs are spent exactly; each binding, terminal records' included, recomputes;
/// and no terminal record shares an occurrence with a receipt. Lines carry their own costs and draws, so no content is read.
pub(in crate::durability) async fn verify_modification_history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> StoredResult<()> {
    let invalid = sqlx::query(
        "SELECT 1 FROM game_character_proficiency_receipts h \
         WHERE h.character_id=encode($1,'hex')::uuid AND h.cause='perk_modification' AND ( \
            (SELECT count(*) FROM game_character_proficiency_modification_lines m \
              WHERE m.proficiency_occurrence_id=h.proficiency_occurrence_id)<>1 \
            OR NOT EXISTS (SELECT 1 FROM game_character_proficiency_modification_lines m \
              JOIN game_character_proficiency_receipt_lines l USING (proficiency_occurrence_id, item_key) \
              WHERE m.proficiency_occurrence_id=h.proficiency_occurrence_id)) \
         UNION ALL SELECT 1 FROM game_character_proficiency_modification_lines m \
          LEFT JOIN game_character_proficiency_receipts h USING (proficiency_occurrence_id) \
          WHERE m.character_id=encode($1,'hex')::uuid AND (h.proficiency_occurrence_id IS NULL \
            OR h.character_id<>m.character_id OR h.cause<>m.cause \
            OR h.committed_character_revision<>m.committed_character_revision \
            OR m.dust_spent<>m.dust_cost OR m.orbs_spent<>m.orb_cost \
            OR (m.cause='migration')<>(m.operation='MIGRATION_CLEAR')) \
         UNION ALL SELECT 1 FROM game_character_proficiency_modification_terminals t \
          JOIN game_character_proficiency_receipts h USING (proficiency_occurrence_id) \
          WHERE t.character_id=encode($1,'hex')::uuid OR h.character_id=encode($1,'hex')::uuid \
         LIMIT 1",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if invalid.is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    verify_terminals(tx, character).await?;
    let lines = sqlx::query(concat!(
        "SELECT ",
        line_columns!(),
        ", m.cause, m.proficiency_occurrence_id::text AS occurrence, \
         m.committed_character_revision::text AS revision, h.simulation_revision, \
         l.definition_revision_before, \
         (SELECT p.committed_character_revision::text FROM game_character_proficiency_receipt_lines p \
           WHERE p.character_id=m.character_id AND p.item_key=m.item_key \
             AND p.committed_character_revision<m.committed_character_revision \
           ORDER BY p.committed_character_revision DESC LIMIT 1) AS previous_track_revision, \
         h.command_binding \
         FROM game_character_proficiency_modification_lines m \
         JOIN game_character_proficiency_receipts h USING (proficiency_occurrence_id) \
         JOIN game_character_proficiency_receipt_lines l USING (proficiency_occurrence_id, item_key) \
         WHERE m.character_id=encode($1,'hex')::uuid \
         ORDER BY m.item_key, m.slot, m.committed_character_revision"
    ))
    .bind(character.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let mut latest = std::collections::BTreeMap::<
        (String, u8),
        (Option<ProficiencyModifiedPerk>, u64, [u8; 16]),
    >::new();
    for line in &lines {
        let occurrence = uuid_text(line.try_get("occurrence")?)?;
        let revision = numeric_u64(line, "revision")?;
        let original = revision
            .checked_sub(1)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let committed = decode_committed(line, character, occurrence, original, revision)?;
        let key = (committed.item_key.clone(), committed.slot);
        let previous = latest
            .get(&key)
            .map(|(after, _, _)| after.clone())
            .unwrap_or(None);
        if previous != committed.before {
            return Err(DurabilityError::InvalidStoredState);
        }
        if line.try_get::<&str, _>("cause")? == ProficiencyCause::PerkModification.key() {
            let choice = match committed.operation {
                ProficiencyModificationOperation::ReshapeChoose => {
                    let offer = committed
                        .before
                        .as_ref()
                        .and_then(|before| before.pending_offer)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let entry = committed
                        .after
                        .as_ref()
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .entry_index;
                    Some(Some(
                        u8::try_from(
                            offer
                                .iter()
                                .position(|offered| *offered == entry)
                                .ok_or(DurabilityError::InvalidStoredState)?,
                        )
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    ))
                }
                ProficiencyModificationOperation::ReshapeDecline => Some(None),
                _ => None,
            };
            let kind = match committed.operation {
                ProficiencyModificationOperation::Modify => {
                    ProficiencyModificationCommandKind::Modify {
                        level: committed
                            .after
                            .as_ref()
                            .ok_or(DurabilityError::InvalidStoredState)?
                            .level,
                    }
                }
                ProficiencyModificationOperation::RankUp => {
                    ProficiencyModificationCommandKind::RankUp
                }
                ProficiencyModificationOperation::OrbRank => {
                    ProficiencyModificationCommandKind::OrbRank
                }
                ProficiencyModificationOperation::ReshapeOffer => {
                    ProficiencyModificationCommandKind::ReshapeOffer
                }
                ProficiencyModificationOperation::ReshapeChoose
                | ProficiencyModificationOperation::ReshapeDecline => {
                    ProficiencyModificationCommandKind::ReshapeChoose {
                        choice: choice.ok_or(DurabilityError::InvalidStoredState)?,
                    }
                }
                ProficiencyModificationOperation::Clear => {
                    ProficiencyModificationCommandKind::Clear
                }
                ProficiencyModificationOperation::MigrationClear => {
                    return Err(DurabilityError::InvalidStoredState);
                }
            };
            let expected = line
                .try_get::<Option<String>, _>("previous_track_revision")?
                .and_then(|text| text.parse::<u64>().ok())
                .and_then(|value| CharacterRevision::new(value).ok())
                .ok_or(DurabilityError::InvalidStoredState)?;
            let bound_shaping: Option<String> = line.try_get("bound_shaping_revision")?;
            let command = ProficiencyModificationCommand::new(
                ProficiencyOccurrence::from_bytes(occurrence)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                &committed.item_key,
                committed.slot,
                kind,
                expected,
                ProficiencyModificationBoundRevisions {
                    definition_revision: line.try_get("definition_revision_before")?,
                    shaping_revision: bound_shaping.ok_or(DurabilityError::InvalidStoredState)?,
                    simulation_revision: line.try_get("simulation_revision")?,
                },
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            if command.command_binding(character).as_slice()
                != line.try_get::<Vec<u8>, _>("command_binding")?.as_slice()
            {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        latest.insert(key, (committed.after, revision, occurrence));
    }
    let rows = sqlx::query(
        "SELECT item_key, slot, level, shaping_key, shaping_revision, entry_index, rank, \
         pending_offer, committed_character_revision::text AS revision, \
         last_proficiency_occurrence_id::text AS occurrence, \
         coalesce(pending_offer IS NULL OR (array_ndims(pending_offer)=1 \
             AND array_lower(pending_offer,1)=1),false) AS canonical_offer \
         FROM game_character_proficiency_modifications WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    for row in rows {
        let key = (
            row.try_get::<String, _>("item_key")?,
            small(row.try_get("slot")?)?,
        );
        let stored = (
            decode_perk(&row, "", "canonical_offer")?,
            numeric_u64(&row, "revision")?,
            uuid_text(row.try_get("occurrence")?)?,
        );
        if latest.remove(&key) != Some(stored) {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    if !latest.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
