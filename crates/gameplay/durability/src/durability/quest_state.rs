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
//! tracks by key. The Character root lock also serializes its quest XP obligations.
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

use std::collections::BTreeMap;
use std::sync::Arc;

use oteryn_gameplay_quests::quest::{
    QUESTGATE0_RL_10, QUESTSTATE0_RL_01, QUESTSTATE0_RL_05, QUESTSTATE0_RL_07,
    QUESTSTATE0_RL_08_STATES, QUESTSTATE0_RL_08_TRACKS, QuestRefusal, QuestStateCatalogue,
    QuestTrackChange, valid_quest_key,
};
use oteryn_simulation_determinism::ExactI64;
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
    /// Server-owned committed movement revision, in its own cause namespace.
    Step(CommandRef),
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
            Self::Step(_) => "step",
            Self::ClaimObligation(_) => "claim_obligation",
            Self::CreatureDeath(_) => "creature_death",
        }
    }

    /// The stored (id, ordinal): a CommandRef is (GameSessionId, CommandId), an occurrence is
    /// (occurrence, 0).
    pub(super) fn key(self) -> ([u8; 16], u64) {
        match self {
            Self::Command(command)
            | Self::Use(command)
            | Self::Step(command)
            | Self::ClaimObligation(command) => (
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
            "step" => Self::Step(command()?),
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
    /// Global quest-door storage semantics: absent storage reads as `-1`; any
    /// durable value for the source storage action id grants the door.
    #[must_use]
    pub fn meets_door_storage(&self, action: u16) -> bool {
        // The donor's numeric action names the same storage that quest authoring
        // normalizes to a named track. Resolve that identity before reading the copy.
        #[derive(serde::Deserialize)]
        struct Binding {
            action: u16,
            track: String,
        }
        #[derive(serde::Deserialize)]
        struct Bindings {
            bindings: Vec<Binding>,
        }
        static BINDINGS: std::sync::OnceLock<Vec<Binding>> = std::sync::OnceLock::new();
        let bindings = BINDINGS.get_or_init(|| {
            serde_json::from_str::<Bindings>(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/world-authoring/samples/quest-door-bindings.json"
            )))
            .map(|document| document.bindings)
            .unwrap_or_default()
        });
        bindings
            .iter()
            .find(|binding| binding.action == action)
            .and_then(|binding| self.tracks.get(&binding.track))
            .or_else(|| {
                self.tracks
                    .get(&format!("oteryn:quest-progress/storage/{action}"))
            })
            .is_some_and(|value| *value != -1)
    }
    /// Stored track values with supported read projections (including the Monk shrine count).
    /// A stored track without a row reads as its declared initial value.
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
            .count()
            + usize::from(
                !self
                    .tracks
                    .contains_key(oteryn_gameplay_quests::quest::monk_shrines::COUNT_TRACK)
                    && committed.changes.iter().any(|change| {
                        change
                            .track
                            .starts_with(oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX)
                    }),
            );
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
        oteryn_gameplay_quests::quest::monk_shrines::derive_count(&mut self.tracks);
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
mod storage;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
mod writer;
use storage::*;

/// The caller holds the Character root lock, as do all QuestState writers.
pub(super) async fn inquisition_blessing_eligible(
    connection: &mut sqlx::PgConnection,
    character: CharacterId,
) -> std::result::Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_inquisition_quest' AND track_key='oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline' AND value=25)")
        .bind(character.as_bytes().as_slice()).fetch_one(connection).await?)
}
