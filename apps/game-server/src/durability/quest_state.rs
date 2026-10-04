//! Durable quest progress: tracks, quest states, receipts and reward-claim obligations
//! (QUEST-STATE-0 §3-§6 and §13, QUEST-STATE-1), over migration 0056.
//!
//! [`DurabilityRoot::commit_character_quest_transition`] commits one named transition (D35) as
//! one CharacterRevision with exactly one quest receipt, its track rows and its quest state, or
//! writes nothing and returns `STAGE_MISMATCH`, `REVISION_MISMATCH`, `OUT_OF_RANGE`,
//! `NOT_SUPPORTED` or `CAPACITY_EXCEEDED`. It is fenced exactly like
//! `commit_character_experience` (§5.3) and serializes with every Character writer on the
//! `character_root` row lock. Lock order: the recovery fence and admission relations, the
//! receipt key, the session checks, `character_root`, the obligation, the quest state, the
//! tracks by key, the Character's quest XP obligations.
//!
//! The receipt is keyed by (Character, cause occurrence, transition key) and binds the request
//! only (§5.1): the same key and binding replay the first outcome without reacquiring session
//! authority; the same key under another cause kind conflicts. The binding excludes the
//! revision, so a `CharacterRevisionMismatch` reloads the cursor and retries once in the
//! revision slot (§5.2). Runtime callers reach the writer only through a
//! [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
//!
//! An obligation cause (§5.4) consumes its `PENDING` row in the committing transaction. A
//! validation refusal sets it `REFUSED` (terminal, with its result code), or
//! `WAITING_MIGRATION` for `REVISION_MISMATCH`; nothing else is written and it is not retried.
//!
//! A transition with `experience` (QUEST-GATE-0 §5.5, QUEST-XP-1) also writes one quest XP
//! obligation (a fresh UUIDv7 occurrence, the amount and the quest's pin as provenance) in its
//! own transaction; with `QUESTGATE0-RL-10` obligations already pending it is refused whole as
//! `OUT_OF_RANGE`. [`RevisionSlot::commit_quest_transition_with_experience`](
//! super::character_revision_sequencer::RevisionSlot::commit_quest_transition_with_experience)
//! then submits the XP award in the same slot under the active progression policy; the XP writer
//! deletes the obligation, and [`request_pending_quest_experience`] requests the pending ones
//! again at admission. A refused award keeps its obligation and is reported as a defect.
//!
//! [`DurabilityRoot::read_character_quest_state`] is the admission load (§7, §12.3): the
//! tracks, quest states, pending obligations and pending quest XP obligations, bounded by
//! `QUESTSTATE0-RL-01`, `-05`, `-07`, `-08` and `QUESTGATE0-RL-10`; over a bound the load fails
//! closed.

#[path = "../quest/mod.rs"]
pub mod quest;

use std::collections::BTreeMap;
use std::sync::Arc;

use oteryn_simulation_determinism::ExactI64;
use quest::{
    QUESTGATE0_RL_10, QUESTSTATE0_RL_01, QUESTSTATE0_RL_05, QUESTSTATE0_RL_07,
    QUESTSTATE0_RL_08_STATES, QUESTSTATE0_RL_08_TRACKS, QuestRefusal, QuestStateCatalogue,
    QuestTrackChange, valid_quest_key,
};
use sha2::{Digest, Sha256};
use sqlx::Row;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::character_revision_sequencer::CharacterRevisionSequencer;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::progression::FiniteProgressionPolicy;
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::{CommandId, CommandRef, GameSessionId, RuntimeScopeRefV1};

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type StoredResult<T> = std::result::Result<T, DurabilityError>;
const REQUEST_BINDING_VERSION: u8 = 1;

/// The occurrence a transition request is bound to, 1:1 with its trigger (§4 "Request").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestCause {
    /// A player command, such as an NPC reply (NPC-0 §5.1): its CommandRef.
    Command(CommandRef),
    /// A USE interaction: its CommandRef.
    Use(CommandRef),
    /// A reward-claim obligation (§5.4): the claim's CommandRef.
    ClaimObligation(CommandRef),
    /// A creature-death reward occurrence.
    CreatureDeath(ExperienceRewardOccurrence),
}

impl QuestCause {
    const fn kind(self) -> &'static str {
        match self {
            Self::Command(_) => "command",
            Self::Use(_) => "use",
            Self::ClaimObligation(_) => "claim_obligation",
            Self::CreatureDeath(_) => "creature_death",
        }
    }

    /// The stored (id, ordinal): a CommandRef is (GameSessionId, CommandId), an occurrence is
    /// (occurrence, 0).
    fn key(self) -> ([u8; 16], u64) {
        match self {
            Self::Command(command) | Self::Use(command) | Self::ClaimObligation(command) => (
                *command.game_session_id().as_bytes(),
                command.command_id().get(),
            ),
            Self::CreatureDeath(occurrence) => (*occurrence.as_bytes(), 0),
        }
    }

    fn stored(kind: &str, id: [u8; 16], ordinal: u64) -> StoredResult<Self> {
        let invalid = |_| DurabilityError::InvalidStoredState;
        let command = || -> StoredResult<CommandRef> {
            Ok(CommandRef::new(
                GameSessionId::decode(&id).map_err(|_| DurabilityError::InvalidStoredState)?,
                CommandId::new(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?,
            ))
        };
        Ok(match kind {
            "command" => Self::Command(command()?),
            "use" => Self::Use(command()?),
            "claim_obligation" => Self::ClaimObligation(command()?),
            "creature_death" if ordinal == 0 => {
                Self::CreatureDeath(ExperienceRewardOccurrence::from_bytes(id).map_err(invalid)?)
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        })
    }
}

/// `request_transition(fence, character, transition_key, cause)` (§4): the fence names the
/// Character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestTransitionRequest {
    pub transition_key: String,
    pub cause: QuestCause,
}

/// One pending quest XP obligation (QUEST-GATE-0 §5.5): the award's occurrence and amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestXpObligation {
    pub occurrence: ExperienceRewardOccurrence,
    pub amount: i64,
}

impl QuestXpObligation {
    /// The award under the active progression policy: its context, `policy_revision` and
    /// `reward_revision` all come from `policy`, never from the quest content revision.
    #[must_use]
    pub fn award<const N: usize>(
        &self,
        policy: &FiniteProgressionPolicy<String, N>,
    ) -> ExperienceAwardRequest<N> {
        ExperienceAwardRequest {
            occurrence: self.occurrence,
            amount: ExactI64::new(self.amount),
            context: policy.context.clone(),
            policy_revision: policy.policy_revision.clone(),
            reward_revision: policy.reward_revision.clone(),
            policy: policy.clone(),
        }
    }
}

/// One committed transition: its receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedQuestTransition {
    pub character_id: CharacterId,
    pub cause: QuestCause,
    pub transition_key: String,
    pub quest_key: String,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    /// The quest state's pin (§6): its content revision and definition hash.
    pub pinned_content_revision: String,
    pub definition_hash: [u8; 32],
    pub completes: bool,
    pub changes: Vec<QuestTrackChange>,
    /// The quest XP obligation the transition wrote and that is still pending; `None` without
    /// `experience` or, on a replay, once the award consumed it.
    pub experience: Option<QuestXpObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestTransitionOutcome {
    Committed(CommittedQuestTransition),
    AlreadyCommitted(CommittedQuestTransition),
    /// Nothing was written to the quest relations; an obligation cause keeps the code.
    Refused(QuestRefusal),
    /// The obligation is `REFUSED` or `WAITING_MIGRATION`: it is not retried and nothing is
    /// written.
    ObligationClosed,
}

/// A transition and the XP award its slot submitted for it (QUEST-GATE-0 §5.5): `None` when the
/// transition committed no pending XP obligation.
#[derive(Debug)]
pub struct QuestTransitionAward {
    pub transition: QuestTransitionOutcome,
    pub experience: Option<std::result::Result<ExperienceCommitOutcome, CharacterProgressionError>>,
}

/// One quest state as loaded at admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStateRecord {
    pub pinned_content_revision: String,
    pub definition_hash: [u8; 32],
    pub completed: bool,
}

/// One open obligation as loaded at admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestObligationRecord {
    pub claim: CommandRef,
    pub transition_key: String,
    /// `WAITING_MIGRATION`: kept, counted toward RL-07, never re-requested here.
    pub waiting_migration: bool,
}

/// The session copy of one Character's quest progress (§7): loaded at admission and advanced by
/// each committed receipt. Predicates read it; the writer re-checks under lock.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestStateCopy {
    tracks: BTreeMap<String, i64>,
    states: BTreeMap<String, QuestStateRecord>,
    obligations: Vec<QuestObligationRecord>,
    xp_obligations: Vec<QuestXpObligation>,
}

impl QuestStateCopy {
    /// Stored track values; a track without a row reads as its declared initial value.
    #[must_use]
    pub fn tracks(&self) -> &BTreeMap<String, i64> {
        &self.tracks
    }

    #[must_use]
    pub fn states(&self) -> &BTreeMap<String, QuestStateRecord> {
        &self.states
    }

    /// Every open obligation, `PENDING` and `WAITING_MIGRATION`.
    #[must_use]
    pub fn obligations(&self) -> &[QuestObligationRecord] {
        &self.obligations
    }

    /// The obligations to request again (§5.4): `PENDING` only.
    pub fn pending_obligations(&self) -> impl Iterator<Item = &QuestObligationRecord> {
        self.obligations
            .iter()
            .filter(|obligation| !obligation.waiting_migration)
    }

    /// The pending quest XP obligations, requested again at admission (QUEST-GATE-0 §5.5).
    #[must_use]
    pub fn xp_obligations(&self) -> &[QuestXpObligation] {
        &self.xp_obligations
    }

    /// Apply one committed receipt of this Character. `false` (nothing applied) over
    /// `QUESTSTATE0-RL-08` or `QUESTGATE0-RL-10`: the caller then fails quest actions closed.
    #[must_use]
    pub fn apply(&mut self, committed: &CommittedQuestTransition) -> bool {
        let new_tracks = committed
            .changes
            .iter()
            .filter(|change| !self.tracks.contains_key(&change.track))
            .count();
        let new_state = usize::from(!self.states.contains_key(&committed.quest_key));
        let new_xp = committed
            .experience
            .filter(|xp| !self.xp_obligations.contains(xp));
        if self.tracks.len() + new_tracks > QUESTSTATE0_RL_08_TRACKS
            || self.states.len() + new_state > QUESTSTATE0_RL_08_STATES
            || self.xp_obligations.len() + usize::from(new_xp.is_some()) > QUESTGATE0_RL_10
        {
            return false;
        }
        self.xp_obligations.extend(new_xp);
        for change in &committed.changes {
            self.tracks.insert(change.track.clone(), change.after);
        }
        let state = self
            .states
            .entry(committed.quest_key.clone())
            .or_insert_with(|| QuestStateRecord {
                pinned_content_revision: committed.pinned_content_revision.clone(),
                definition_hash: committed.definition_hash,
                completed: false,
            });
        state.completed |= committed.completes;
        if let QuestCause::ClaimObligation(claim) = committed.cause {
            self.obligations
                .retain(|obligation| obligation.claim != claim);
        }
        true
    }

    /// Record the refusal of an obligation's attempt: `REVISION_MISMATCH` waits for a
    /// migration, any other refusal or a closed obligation leaves the copy.
    pub fn settle_obligation(&mut self, claim: CommandRef, outcome: &QuestTransitionOutcome) {
        match outcome {
            QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch) => {
                for obligation in &mut self.obligations {
                    if obligation.claim == claim {
                        obligation.waiting_migration = true;
                    }
                }
            }
            QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed => {
                self.obligations
                    .retain(|obligation| obligation.claim != claim);
            }
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_) => {}
        }
    }

    /// Record the outcome of a quest XP award: a receipt consumed the obligation; a refusal or
    /// an unknown outcome keeps it for the next admission.
    pub fn settle_experience(
        &mut self,
        occurrence: ExperienceRewardOccurrence,
        outcome: &std::result::Result<ExperienceCommitOutcome, CharacterProgressionError>,
    ) {
        if outcome.is_ok() {
            self.xp_obligations
                .retain(|obligation| obligation.occurrence != occurrence);
        }
    }
}

impl DurabilityRoot {
    /// Commit one quest transition (§4-§5). Exact replay of the receipt key returns the receipt
    /// under the current recovery fence without session authority; another cause kind on the
    /// same key conflicts. A new request is fenced like an XP award, refused while a death's
    /// respawn is pending, and validated against the locked values: a refusal writes nothing to
    /// the quest relations or the revision chain. `catalogue` must be the Character's current
    /// content revision; an unknown transition is `InvalidInput`.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_quest_transition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || !valid_quest_key(&request.transition_key)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        // §5.3: a command cause belongs to the fenced GameSession; a still-pending one after a
        // same-GameSession reconnect commits with the current generation, and a replaced
        // session's command is refused. An obligation may come from an earlier session.
        if let QuestCause::Command(command) | QuestCause::Use(command) = request.cause
            && command.game_session_id() != fence.game_session_id
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let binding = request_binding(fence.character_id, &request);
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
                    lock_receipt_key(&mut tx, character, &request).await?;
                    if let Some(row) = load_receipt(&mut tx, character, &request).await? {
                        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(QuestTransitionOutcome::AlreadyCommitted(committed)));
                    }
                    let Some(transition) = catalogue.transition(&request.transition_key) else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let id = character.as_bytes().as_slice();
                    let state = sqlx::query(
                        "SELECT character_revision::text, profile_revision, ruleset_revision, \
                                content_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let content_revision: String = state.try_get("content_revision")?;
                    if !state_matches_root(&state, &root)
                        || content_revision != catalogue.content_revision()
                    {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let pending: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid)",
                    )
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }
                    let obligation = match request.cause {
                        QuestCause::ClaimObligation(claim) => Some(claim),
                        _ => None,
                    };
                    if let Some(claim) = obligation {
                        let row = sqlx::query(
                            "SELECT character_id::text, transition_key, state \
                               FROM game_character_quest_obligations \
                              WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                AND claim_command_id = $2::text::numeric(20,0) FOR UPDATE",
                        )
                        .bind(claim.game_session_id().as_bytes().as_slice())
                        .bind(claim.command_id().get().to_string())
                        .fetch_optional(&mut *tx)
                        .await?;
                        let Some(row) = row else {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        };
                        if uuid_text(row.try_get("character_id")?)? != *character.as_bytes()
                            || row.try_get::<String, _>("transition_key")?
                                != request.transition_key
                        {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        if row.try_get::<String, _>("state")? != "PENDING" {
                            return Ok(Ok(QuestTransitionOutcome::ObligationClosed));
                        }
                    }

                    let quest = transition.quest.as_str();
                    let current_hash = catalogue
                        .definition_hash(quest)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let stored_state = sqlx::query(
                        "SELECT pinned_content_revision, definition_hash, \
                                completed_character_revision IS NOT NULL AS completed \
                           FROM game_character_quest_states \
                          WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2 \
                          FOR UPDATE",
                    )
                    .bind(id)
                    .bind(quest)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let (pin, hash, refusal) = match &stored_state {
                        Some(row) => {
                            let hash: [u8; 32] = row
                                .try_get::<Vec<u8>, _>("definition_hash")?
                                .try_into()
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                            // §6: a completed quest keeps its pin and no longer blocks.
                            let blocked =
                                hash != current_hash && !row.try_get::<bool, _>("completed")?;
                            (
                                row.try_get::<String, _>("pinned_content_revision")?,
                                hash,
                                blocked.then_some(QuestRefusal::RevisionMismatch),
                            )
                        }
                        None => {
                            let states: i64 = sqlx::query_scalar(
                                "SELECT count(*) FROM game_character_quest_states \
                                  WHERE character_id = encode($1,'hex')::uuid",
                            )
                            .bind(id)
                            .fetch_one(&mut *tx)
                            .await?;
                            let full = usize::try_from(states).unwrap_or(usize::MAX)
                                >= QUESTSTATE0_RL_05;
                            (
                                content_revision.clone(),
                                current_hash,
                                full.then_some(QuestRefusal::CapacityExceeded),
                            )
                        }
                    };
                    let keys: Vec<&str> = transition
                        .effects
                        .iter()
                        .map(|effect| effect.track.as_str())
                        .collect();
                    let rows = sqlx::query(
                        "SELECT track_key, quest_key, value FROM game_character_quest_tracks \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND track_key = ANY($2::text[]) \
                          ORDER BY track_key FOR UPDATE",
                    )
                    .bind(id)
                    .bind(&keys)
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut stored = BTreeMap::new();
                    for row in &rows {
                        if row.try_get::<String, _>("quest_key")? != quest {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        stored.insert(row.try_get::<String, _>("track_key")?, row.try_get("value")?);
                    }
                    let refusal = match refusal {
                        Some(refusal) => Some(refusal),
                        None if rows.len() < keys.len() => {
                            let tracks: i64 = sqlx::query_scalar(
                                "SELECT count(*) FROM game_character_quest_tracks \
                                  WHERE character_id = encode($1,'hex')::uuid",
                            )
                            .bind(id)
                            .fetch_one(&mut *tx)
                            .await?;
                            let after = usize::try_from(tracks)
                                .unwrap_or(usize::MAX)
                                .saturating_add(keys.len() - rows.len());
                            (after > QUESTSTATE0_RL_01).then_some(QuestRefusal::CapacityExceeded)
                        }
                        None => None,
                    };
                    // QUEST-GATE-0 §5.5: count the locked pending XP obligations before any
                    // write; at RL-10 an XP-bearing transition is refused whole.
                    let xp_full = match (refusal, transition.experience) {
                        (None, Some(_)) => {
                            let pending = sqlx::query(
                                "SELECT reward_occurrence_id::text \
                                   FROM game_character_quest_xp_obligations \
                                  WHERE character_id = encode($1,'hex')::uuid \
                                  ORDER BY reward_occurrence_id FOR UPDATE",
                            )
                            .bind(id)
                            .fetch_all(&mut *tx)
                            .await?;
                            pending.len() >= QUESTGATE0_RL_10
                        }
                        _ => false,
                    };
                    let now: i64 =
                        sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
                            .fetch_one(&mut *tx)
                            .await?;
                    let changes = match refusal
                        .map_or_else(|| catalogue.evaluate(transition, &stored, now), Err)
                        .and_then(|changes| {
                            if xp_full {
                                Err(QuestRefusal::OutOfRange)
                            } else {
                                Ok(changes)
                            }
                        }) {
                        Ok(changes) => changes,
                        Err(refusal) => {
                            let Some(claim) = obligation else {
                                // Dropping the transaction rolls back: nothing is written.
                                return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                            };
                            let next = if refusal == QuestRefusal::RevisionMismatch {
                                "WAITING_MIGRATION"
                            } else {
                                "REFUSED"
                            };
                            let updated = sqlx::query(
                                "UPDATE game_character_quest_obligations \
                                    SET state = $3, result_code = $4, \
                                        updated_at = greatest(updated_at, \
                                          floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
                                  WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                    AND claim_command_id = $2::text::numeric(20,0) \
                                    AND state = 'PENDING'",
                            )
                            .bind(claim.game_session_id().as_bytes().as_slice())
                            .bind(claim.command_id().get().to_string())
                            .bind(next)
                            .bind(refusal.code())
                            .execute(&mut *tx)
                            .await?;
                            if updated.rows_affected() != 1 {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                        }
                    };

                    let committed_revision = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let (original, committed) =
                        (root.revision.to_string(), committed_revision.to_string());
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
                            .bind(id)
                            .bind(&committed)
                            .bind(&original)
                            .execute(&mut *tx)
                            .await?;
                        if updated.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let (cause_id, ordinal) = request.cause.key();
                    let track_keys: Vec<&str> =
                        changes.iter().map(|change| change.track.as_str()).collect();
                    let before: Vec<i64> = changes.iter().map(|change| change.before).collect();
                    let after: Vec<i64> = changes.iter().map(|change| change.after).collect();
                    let receipt = sqlx::query(
                        "INSERT INTO game_character_quest_receipts(\
                           character_id, cause_kind, cause_id, cause_ordinal, transition_key, \
                           quest_key, request_binding, pinned_content_revision, definition_hash, \
                           completes, track_keys, values_before, values_after, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           profile_revision, ruleset_revision, content_revision, \
                           simulation_revision, evidence_revision, declaration_revision, \
                           policy_revision, reward_revision, committed_at) \
                         SELECT s.character_id, $2, encode($3,'hex')::uuid, \
                           $4::text::numeric(20,0), $5, $6, $7, $8, $9, $10, $11::text[], \
                           $12::bigint[], $13::bigint[], $14::text::numeric(20,0), \
                           $15::text::numeric(20,0), s.level, s.level, s.total_experience, \
                           s.total_experience, s.profile_revision, s.ruleset_revision, \
                           s.content_revision, s.simulation_revision, s.evidence_revision, \
                           s.declaration_revision, s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($1,'hex')::uuid",
                    )
                    .bind(id)
                    .bind(request.cause.kind())
                    .bind(cause_id.as_slice())
                    .bind(ordinal.to_string())
                    .bind(&request.transition_key)
                    .bind(quest)
                    .bind(binding.as_slice())
                    .bind(&pin)
                    .bind(hash.as_slice())
                    .bind(transition.completes)
                    .bind(&track_keys)
                    .bind(&before)
                    .bind(&after)
                    .bind(&original)
                    .bind(&committed)
                    .execute(&mut *tx)
                    .await?;
                    if receipt.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    for change in &changes {
                        let written = sqlx::query(
                            "INSERT INTO game_character_quest_tracks(character_id, track_key, \
                               quest_key, value, committed_character_revision) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                               $5::text::numeric(20,0)) \
                             ON CONFLICT (character_id, track_key) DO UPDATE \
                               SET value = EXCLUDED.value, \
                                   committed_character_revision = \
                                     EXCLUDED.committed_character_revision",
                        )
                        .bind(id)
                        .bind(&change.track)
                        .bind(quest)
                        .bind(change.after)
                        .bind(&committed)
                        .execute(&mut *tx)
                        .await?;
                        if written.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let completed = transition.completes.then_some(committed.as_str());
                    let written = if stored_state.is_some() {
                        sqlx::query(
                            "UPDATE game_character_quest_states \
                                SET committed_character_revision = $3::text::numeric(20,0), \
                                    completed_character_revision = coalesce(\
                                      completed_character_revision, $4::text::numeric(20,0)) \
                              WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2",
                        )
                        .bind(id)
                        .bind(quest)
                        .bind(&committed)
                        .bind(completed)
                        .execute(&mut *tx)
                        .await?
                    } else {
                        sqlx::query(
                            "INSERT INTO game_character_quest_states(character_id, quest_key, \
                               pinned_content_revision, definition_hash, \
                               started_character_revision, completed_character_revision, \
                               committed_character_revision) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                               $5::text::numeric(20,0), $6::text::numeric(20,0), \
                               $5::text::numeric(20,0))",
                        )
                        .bind(id)
                        .bind(quest)
                        .bind(&pin)
                        .bind(hash.as_slice())
                        .bind(&committed)
                        .bind(completed)
                        .execute(&mut *tx)
                        .await?
                    };
                    if written.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let experience = match transition.experience {
                        Some(amount) => Some(
                            insert_xp_obligation(
                                &mut tx,
                                character,
                                &request,
                                amount,
                                &pin,
                            )
                            .await?,
                        ),
                        None => None,
                    };
                    if let Some(claim) = obligation {
                        let consumed = sqlx::query(
                            "UPDATE game_character_quest_obligations \
                                SET state = 'CONSUMED', \
                                    updated_at = greatest(updated_at, \
                                      floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
                              WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                AND claim_command_id = $2::text::numeric(20,0) \
                                AND state = 'PENDING'",
                        )
                        .bind(claim.game_session_id().as_bytes().as_slice())
                        .bind(claim.command_id().get().to_string())
                        .execute(&mut *tx)
                        .await?;
                        if consumed.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let committed = CommittedQuestTransition {
                        character_id: character,
                        cause: request.cause,
                        transition_key: request.transition_key,
                        quest_key: quest.to_owned(),
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        pinned_content_revision: pin,
                        definition_hash: hash,
                        completes: transition.completes,
                        changes,
                        experience,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(QuestTransitionOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an ambiguous quest commit by its receipt key. This proves only what committed and
    /// never reacquires gameplay authority.
    pub async fn reconcile_character_quest_transition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        request: QuestTransitionRequest,
    ) -> Result<Option<CommittedQuestTransition>> {
        let binding = request_binding(character, &request);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_receipt_key(&mut tx, character, &request).await?;
                    let Some(row) = load_receipt(&mut tx, character, &request).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// The admission load (§7, §12.3): tracks, quest states and open obligations of one
    /// Character. It writes nothing. Over `QUESTSTATE0-RL-01`, `-05`, `-07` or `-08`, or with a
    /// stored value out of shape, it fails closed with `Unavailable(InvalidStoredState)`.
    pub async fn read_character_quest_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<QuestStateCopy> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let id = character.as_bytes().as_slice();
                    // One coherent copy: every quest writer and every claim that inserts an
                    // obligation holds `character_root` FOR UPDATE until it commits, so with the
                    // root held FOR SHARE no quest relation of this Character can change between
                    // the reads below.
                    sqlx::query(
                        "SELECT 1 FROM game_character_roots \
                          WHERE character_id = encode($1,'hex')::uuid FOR SHARE",
                    )
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let bound = |limit: usize| i64::try_from(limit + 1).unwrap_or(i64::MAX);
                    let over = |rows: &[sqlx::postgres::PgRow], limit: usize| rows.len() > limit;
                    let tracks = sqlx::query(
                        "SELECT track_key, value FROM game_character_quest_tracks \
                          WHERE character_id = encode($1,'hex')::uuid \
                          ORDER BY track_key LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS)))
                    .fetch_all(&mut *tx)
                    .await?;
                    let states = sqlx::query(
                        "SELECT quest_key, pinned_content_revision, definition_hash, \
                                completed_character_revision IS NOT NULL AS completed \
                           FROM game_character_quest_states \
                          WHERE character_id = encode($1,'hex')::uuid \
                          ORDER BY quest_key LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES)))
                    .fetch_all(&mut *tx)
                    .await?;
                    let obligations = sqlx::query(
                        "SELECT claim_game_session_id::text, claim_command_id::text, \
                                transition_key, state \
                           FROM game_character_quest_obligations \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND state IN ('PENDING', 'WAITING_MIGRATION') \
                          ORDER BY created_at, claim_game_session_id, claim_command_id \
                          LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTSTATE0_RL_07))
                    .fetch_all(&mut *tx)
                    .await?;
                    let xp_obligations = sqlx::query(
                        "SELECT reward_occurrence_id::text, amount \
                           FROM game_character_quest_xp_obligations \
                          WHERE character_id = encode($1,'hex')::uuid \
                          ORDER BY created_at, reward_occurrence_id LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTGATE0_RL_10))
                    .fetch_all(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    if over(&tracks, QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS))
                        || over(&states, QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES))
                        || over(&obligations, QUESTSTATE0_RL_07)
                        || over(&xp_obligations, QUESTGATE0_RL_10)
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let mut copy = QuestStateCopy::default();
                    for row in &tracks {
                        copy.tracks
                            .insert(row.try_get("track_key")?, row.try_get("value")?);
                    }
                    for row in &states {
                        copy.states.insert(
                            row.try_get("quest_key")?,
                            QuestStateRecord {
                                pinned_content_revision: row.try_get("pinned_content_revision")?,
                                definition_hash: row
                                    .try_get::<Vec<u8>, _>("definition_hash")?
                                    .try_into()
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                completed: row.try_get("completed")?,
                            },
                        );
                    }
                    for row in &obligations {
                        copy.obligations.push(QuestObligationRecord {
                            claim: CommandRef::new(
                                GameSessionId::decode(&uuid_text(
                                    row.try_get("claim_game_session_id")?,
                                )?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                                CommandId::new(numeric_u64(row, "claim_command_id")?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                            ),
                            transition_key: row.try_get("transition_key")?,
                            waiting_migration: row.try_get::<String, _>("state")?
                                == "WAITING_MIGRATION",
                        });
                    }
                    for row in &xp_obligations {
                        copy.xp_obligations.push(QuestXpObligation {
                            occurrence: ExperienceRewardOccurrence::from_bytes(uuid_text(
                                row.try_get("reward_occurrence_id")?,
                            )?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                            amount: row.try_get("amount")?,
                        });
                    }
                    Ok(Ok(copy))
                })
            })
            .await?
    }
}

/// What one admission keeps for its session (§7, §12.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestAdmission {
    /// The session copy; `None` when the load failed or a bound was exceeded: quest actions of
    /// the session then fail closed, and login does not.
    pub copy: Option<QuestStateCopy>,
    /// An obligation attempt failed without an outcome: request the pending obligations again
    /// after the backoff (at most once a minute, §5.4).
    pub retry: bool,
}

/// The admission step of fresh admission and resume: load the Character's tracks, quest states
/// and open obligations, then request each `PENDING` obligation again through the Character's
/// revision slot. Without a catalogue (no quest content loaded) the obligations stay pending.
pub async fn admit_character_quest_state(
    sequencer: &CharacterRevisionSequencer,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    catalogue: Option<&Arc<QuestStateCatalogue>>,
) -> QuestAdmission {
    let Ok(mut copy) = root
        .read_character_quest_state(authority, fence.character_id)
        .await
    else {
        return QuestAdmission {
            copy: None,
            retry: false,
        };
    };
    match request_pending_obligations(
        sequencer, root, authority, node, fence, catalogue, &mut copy,
    )
    .await
    {
        Some(retry) => QuestAdmission {
            copy: Some(copy),
            retry,
        },
        None => QuestAdmission {
            copy: None,
            retry: false,
        },
    }
}

/// Request every `PENDING` obligation of `copy` once, each in the Character's revision slot,
/// and apply each outcome to `copy`. `Some(true)` when an attempt failed without an outcome
/// (retry after the backoff); `None` when a committed receipt would take `copy` over
/// `QUESTSTATE0-RL-08` (the copy is then unusable). A refusal is final (§5.4).
pub async fn request_pending_obligations(
    sequencer: &CharacterRevisionSequencer,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    catalogue: Option<&Arc<QuestStateCatalogue>>,
    copy: &mut QuestStateCopy,
) -> Option<bool> {
    let Some(catalogue) = catalogue else {
        return Some(false);
    };
    let pending: Vec<QuestObligationRecord> = copy.pending_obligations().cloned().collect();
    let mut retry = false;
    for obligation in pending {
        let mut slot = sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                root,
                authority,
                node,
                fence,
                QuestTransitionRequest {
                    transition_key: obligation.transition_key.clone(),
                    cause: QuestCause::ClaimObligation(obligation.claim),
                },
                Arc::clone(catalogue),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(committed)
                | QuestTransitionOutcome::AlreadyCommitted(committed),
            ) => {
                if !copy.apply(&committed) {
                    return None;
                }
            }
            Ok(outcome) => copy.settle_obligation(obligation.claim, &outcome),
            Err(_) => retry = true,
        }
    }
    Some(retry)
}

/// Request every pending quest XP obligation of `copy` once (QUEST-GATE-0 §5.5), each in the
/// Character's revision slot under the active progression `policy`, and apply each outcome to
/// `copy`. `true` when an attempt failed without an outcome (retry after the backoff). A refused
/// award keeps its obligation for the next admission and is reported as a defect.
pub async fn request_pending_quest_experience<const N: usize>(
    sequencer: &CharacterRevisionSequencer,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    policy: &FiniteProgressionPolicy<String, N>,
    copy: &mut QuestStateCopy,
) -> bool {
    let pending = copy.xp_obligations.clone();
    let mut retry = false;
    for obligation in pending {
        let mut slot = sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_experience(root, authority, node, fence, &obligation, policy)
            .await;
        drop(slot);
        retry |= matches!(outcome, Err(CharacterProgressionError::Unavailable(_)));
        report_refused_experience(&outcome);
        copy.settle_experience(obligation.occurrence, &outcome);
    }
    retry
}

/// QUEST-GATE-0 §5.5: an XP refusal keeps its obligation and is a defect (a policy or revision
/// mismatch fails closed, QUEST-STATE-0 §5.2). An unknown outcome is not a refusal.
pub(super) fn report_refused_experience(
    outcome: &std::result::Result<ExperienceCommitOutcome, CharacterProgressionError>,
) {
    match outcome {
        Ok(_) | Err(CharacterProgressionError::Unavailable(_)) => {}
        Err(error) => eprintln!(
            "oteryn-game-server defect: a quest XP award was refused ({error}); its obligation \
             stays pending"
        ),
    }
}

/// Insert the quest XP obligation of a committing transition: a fresh UUIDv7 occurrence (the
/// transaction's statement time and database randomness), the amount and the quest's pin.
async fn insert_xp_obligation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
    amount: i64,
    pin: &str,
) -> StoredResult<QuestXpObligation> {
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT substring(int8send(\
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint) FROM 3) \
         || substring(uuid_send(gen_random_uuid()) FROM 7)",
    )
    .fetch_one(&mut **tx)
    .await?;
    let mut occurrence: [u8; 16] = bytes
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    occurrence[6] = 0x70 | (occurrence[6] & 0x0f);
    occurrence[8] = 0x80 | (occurrence[8] & 0x3f);
    let occurrence = ExperienceRewardOccurrence::from_bytes(occurrence)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let (cause_id, ordinal) = request.cause.key();
    let written = sqlx::query(
        "INSERT INTO game_character_quest_xp_obligations(reward_occurrence_id, character_id, \
           cause_id, cause_ordinal, transition_key, amount, pinned_content_revision, created_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), $5, $6, $7, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(occurrence.as_bytes().as_slice())
    .bind(character.as_bytes().as_slice())
    .bind(cause_id.as_slice())
    .bind(ordinal.to_string())
    .bind(&request.transition_key)
    .bind(amount)
    .bind(pin)
    .execute(&mut **tx)
    .await?;
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(QuestXpObligation { occurrence, amount })
}

/// Version byte plus SHA-256 over the request only (§5.1): the Character, the cause (kind, id,
/// ordinal) and the transition key. The fence, the revision and the definition are not inputs,
/// so a retry after a reconnect, a revision move or a content change replays.
fn request_binding(character: CharacterId, request: &QuestTransitionRequest) -> [u8; 33] {
    let (id, ordinal) = request.cause.key();
    let mut semantic = b"oteryn:quest-transition:request-binding\0".to_vec();
    semantic.push(REQUEST_BINDING_VERSION);
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(request.cause.kind().as_bytes());
    semantic.push(0);
    semantic.extend_from_slice(&id);
    semantic.extend_from_slice(&ordinal.to_be_bytes());
    // Checked keys cannot contain NUL.
    semantic.extend_from_slice(request.transition_key.as_bytes());
    semantic.push(0);
    let mut binding = [REQUEST_BINDING_VERSION; 33];
    binding[1..].copy_from_slice(&Sha256::digest(&semantic));
    binding
}

async fn lock_receipt_key(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
) -> StoredResult<()> {
    let (id, ordinal) = request.cause.key();
    let mut key = character.as_bytes().to_vec();
    key.extend_from_slice(&id);
    key.extend_from_slice(&ordinal.to_be_bytes());
    key.extend_from_slice(request.transition_key.as_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:character-quest-receipt:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
) -> StoredResult<Option<sqlx::postgres::PgRow>> {
    let (id, ordinal) = request.cause.key();
    // The receipt and, while pending, the quest XP obligation it wrote (QUEST-GATE-0 §5.5).
    Ok(sqlx::query(
        "SELECT r.character_id::text, r.cause_kind, r.cause_id::text, \
                r.cause_ordinal::text, r.transition_key, r.quest_key, r.request_binding, \
                r.pinned_content_revision, r.definition_hash, r.completes, r.track_keys, \
                r.values_before, r.values_after, r.original_character_revision::text, \
                r.committed_character_revision::text, \
                x.reward_occurrence_id::text AS xp_occurrence, x.amount AS xp_amount \
           FROM game_character_quest_receipts r \
           LEFT JOIN game_character_quest_xp_obligations x \
             ON x.character_id = r.character_id AND x.cause_id = r.cause_id \
            AND x.cause_ordinal = r.cause_ordinal AND x.transition_key = r.transition_key \
          WHERE r.character_id = encode($1,'hex')::uuid \
            AND r.cause_id = encode($2,'hex')::uuid \
            AND r.cause_ordinal = $3::text::numeric(20,0) AND r.transition_key = $4",
    )
    .bind(character.as_bytes().as_slice())
    .bind(id.as_slice())
    .bind(ordinal.to_string())
    .bind(&request.transition_key)
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(row: &sqlx::postgres::PgRow) -> StoredResult<CommittedQuestTransition> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let keys: Vec<String> = row.try_get("track_keys")?;
    let before: Vec<i64> = row.try_get("values_before")?;
    let after: Vec<i64> = row.try_get("values_after")?;
    if before.len() != keys.len() || after.len() != keys.len() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
        .map_err(invalid)?;
    let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
        .map_err(invalid)?;
    if original.get().checked_add(1) != Some(committed.get()) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let experience = match (
        row.try_get::<Option<&str>, _>("xp_occurrence")?,
        row.try_get::<Option<i64>, _>("xp_amount")?,
    ) {
        (Some(occurrence), Some(amount)) => Some(QuestXpObligation {
            occurrence: ExperienceRewardOccurrence::from_bytes(uuid_text(occurrence)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            amount,
        }),
        (None, None) => None,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(CommittedQuestTransition {
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(invalid)?,
        cause: QuestCause::stored(
            row.try_get("cause_kind")?,
            uuid_text(row.try_get("cause_id")?)?,
            numeric_u64(row, "cause_ordinal")?,
        )?,
        transition_key: row.try_get("transition_key")?,
        quest_key: row.try_get("quest_key")?,
        original_character_revision: original,
        committed_character_revision: committed,
        pinned_content_revision: row.try_get("pinned_content_revision")?,
        definition_hash: row
            .try_get::<Vec<u8>, _>("definition_hash")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        completes: row.try_get("completes")?,
        changes: keys
            .into_iter()
            .zip(before)
            .zip(after)
            .map(|((track, before), after)| QuestTrackChange {
                track,
                before,
                after,
            })
            .collect(),
        experience,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn id(tag: u8) -> [u8; 16] {
        [tag, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    fn command(tag: u8, command_id: u64) -> CommandRef {
        CommandRef::new(
            GameSessionId::decode(&id(tag)).expect("session"),
            CommandId::new(command_id).expect("command"),
        )
    }

    fn request(cause: QuestCause, key: &str) -> QuestTransitionRequest {
        QuestTransitionRequest {
            transition_key: key.into(),
            cause,
        }
    }

    #[test]
    fn the_quest_state_api_is_linked() {
        // Standalone durability suites path-load this module without quest cases.
        let _ = DurabilityRoot::commit_character_quest_transition;
        let _ = DurabilityRoot::reconcile_character_quest_transition;
        let _ = DurabilityRoot::read_character_quest_state;
        let _ = quest::QuestStateCatalogue::track;
        let _ = admit_character_quest_state;
        let _ = request_pending_obligations;
        let _ = request_pending_quest_experience::<2>;
        let _ = QuestStateCopy::xp_obligations;
        let _ = |award: QuestTransitionAward| (award.transition, award.experience);
    }

    #[test]
    fn the_binding_covers_the_request_only() {
        let character = CharacterId::from_bytes(id(1)).expect("character");
        let base = request(QuestCause::Command(command(2, 7)), "oteryn:t/1");
        let binding = request_binding(character, &base);
        assert_eq!(binding[0], 1);
        assert!(binding.len() <= quest::QUESTSTATE0_RL_04, "RL-04");
        assert_eq!(binding, request_binding(character, &base.clone()));
        let death = ExperienceRewardOccurrence::from_bytes(id(2)).expect("occurrence");
        for changed in [
            request(QuestCause::Use(command(2, 7)), "oteryn:t/1"),
            request(QuestCause::ClaimObligation(command(2, 7)), "oteryn:t/1"),
            request(QuestCause::CreatureDeath(death), "oteryn:t/1"),
            request(QuestCause::Command(command(2, 8)), "oteryn:t/1"),
            request(QuestCause::Command(command(3, 7)), "oteryn:t/1"),
            request(QuestCause::Command(command(2, 7)), "oteryn:t/2"),
        ] {
            assert_ne!(request_binding(character, &changed), binding, "{changed:?}");
        }
        let other = CharacterId::from_bytes(id(9)).expect("character");
        assert_ne!(request_binding(other, &base), binding);
    }

    #[test]
    fn a_cause_round_trips_through_its_stored_shape() {
        let death = ExperienceRewardOccurrence::from_bytes(id(4)).expect("occurrence");
        for cause in [
            QuestCause::Command(command(2, 7)),
            QuestCause::Use(command(2, u64::MAX)),
            QuestCause::ClaimObligation(command(3, 1)),
            QuestCause::CreatureDeath(death),
        ] {
            let (stored_id, ordinal) = cause.key();
            assert_eq!(
                QuestCause::stored(cause.kind(), stored_id, ordinal).expect("stored"),
                cause
            );
        }
        assert!(QuestCause::stored("command", id(2), 0).is_err());
        assert!(QuestCause::stored("creature_death", id(2), 1).is_err());
        assert!(QuestCause::stored("npc", id(2), 1).is_err());
    }

    #[test]
    fn the_copy_applies_receipts_and_settles_obligations() {
        let claim = command(5, 1);
        let mut copy = QuestStateCopy {
            obligations: vec![QuestObligationRecord {
                claim,
                transition_key: "oteryn:t/1".into(),
                waiting_migration: false,
            }],
            ..QuestStateCopy::default()
        };
        assert_eq!(copy.pending_obligations().count(), 1);
        let committed = CommittedQuestTransition {
            character_id: CharacterId::from_bytes(id(1)).expect("character"),
            cause: QuestCause::ClaimObligation(claim),
            transition_key: "oteryn:t/1".into(),
            quest_key: "oteryn:quest/q".into(),
            original_character_revision: CharacterRevision::new(1).expect("revision"),
            committed_character_revision: CharacterRevision::new(2).expect("revision"),
            pinned_content_revision: "content-1".into(),
            definition_hash: [7; 32],
            completes: true,
            changes: vec![QuestTrackChange {
                track: "oteryn:quest-progress/q".into(),
                before: 0,
                after: 3,
            }],
            experience: None,
        };
        assert!(copy.apply(&committed), "within RL-08");
        assert_eq!(copy.tracks().get("oteryn:quest-progress/q"), Some(&3));
        assert!(copy.states()["oteryn:quest/q"].completed);
        assert!(copy.obligations().is_empty(), "the receipt consumed it");

        copy.obligations.push(QuestObligationRecord {
            claim,
            transition_key: "oteryn:t/1".into(),
            waiting_migration: false,
        });
        copy.settle_obligation(
            claim,
            &QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch),
        );
        assert_eq!(copy.obligations().len(), 1);
        assert_eq!(copy.pending_obligations().count(), 0, "not retried");
        copy.settle_obligation(claim, &QuestTransitionOutcome::ObligationClosed);
        assert!(copy.obligations().is_empty());

        let mut full = QuestStateCopy::default();
        for index in 0..QUESTSTATE0_RL_08_TRACKS {
            full.tracks.insert(format!("oteryn:t/{index}"), 0);
        }
        assert!(!full.apply(&committed), "over RL-08");
    }
}
