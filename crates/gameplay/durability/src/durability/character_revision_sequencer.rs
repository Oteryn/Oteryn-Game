//! One revision-advancing write in flight per Character (CHAR-REV-SEQ-1, QUEST-STATE-0 §5.2).
//!
//! The owning Channel runtime holds one [`CharacterRevisionSequencer`]. Every write that advances
//! a CharacterRevision (XP, death, Bestiary, charm with its in-transaction fee burn, monk state
//! save, build, proficiency, quest transition, quest XP award, Wheel change and Wheel reset) runs through a [`RevisionSlot`] of that Character: the slot is an asynchronous FIFO
//! queue per Character, and the holder is the only writer of that Character's revision until it
//! drops the slot. A composition (a creature death's XP, then Bestiary) holds one slot for its
//! whole chain, so each step takes the revision the previous one committed and no other request
//! can commit between them.
//!
//! The slot holds the revision cursor: loaded from the Character root when the slot has none,
//! advanced only to the committed revision of a receipt, and dropped (reloaded on next use) when
//! an outcome is unknown or a mismatch proves another writer bypassed the sequencer.
//!
//! On a `CharacterRevisionMismatch`, a write whose binding excludes the revision (Bestiary,
//! quest transition, Wheel change and reset) reloads the cursor and is retried once; a write whose binding includes it (XP, death, charm,
//! monk, build, proficiency) is not retried: it fails closed and is reported as a defect.
//!
//! The slot is not the runtime lock and adds no fence: callers never hold the runtime lock while
//! they wait for a slot or across the durable write, and every write is still refused by the
//! existing session-generation gameplay fence in its own transaction.

mod appearance_revision;
mod arena_match;
mod atelier;
mod npc_teleport;

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex as SlotQueue, OwnedMutexGuard};
mod cyclopedia;
mod death_promotion;
mod grizzly;
mod native_doors;
mod native_kills;
mod native_hits;
mod npc_marriage;
mod npc_program;
mod npc_purchase;
mod npc_travel;
mod proficiency_catalyst;
mod world_access;
mod rotten_area;
mod rotten_flame;
#[cfg(test)]
mod writer_registration_tests;

use super::DurabilityError;
use super::DurabilityRoot;
use super::bestiary_progress::{BestiaryKillOutcome, BestiaryKillRequest, BestiaryProgressError};
use super::character_authority::{CharacterAuthorityError, ReconciledCharacterAuthority};
use super::character_build::{BuildChangeRequest, BuildCommitOutcome, BuildFormula};
use super::character_death::{CharacterDeathOutcome, CharacterDeathRequest};
use super::character_proficiency::{
    ProficiencyChangeRequest, ProficiencyCommitOutcome, ProficiencyDefinitions,
};
use super::character_proficiency_modification::{
    ProficiencyModificationCommand, ProficiencyModificationContext, ProficiencyModificationOutcome,
    ProficiencyShapingSource,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome,
};
use super::character_wheel::{
    WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelResetOutcome, WheelRuleset,
};
use super::charm_state::{CharmCommandOutcome, CharmCommandRequest, CharmFacts, CharmStateError};
use super::monk_state::{MonkStateSaveOutcome, MonkStateSaveRequest};
use super::quest_state::{
    QuestTransitionAward, QuestTransitionOutcome, QuestTransitionRequest, QuestXpObligation,
    report_refused_experience,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::progression::FiniteProgressionPolicy;
use crate::domain::{CharacterId, CharacterRevision};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;

/// Formula and starter item facts for the atomic Newhaven build.
pub struct NewhavenBuildContext<'a> {
    pub formula: &'a dyn BuildFormula,
    pub kit: super::starter_grant::NewhavenKit<'a>,
}

/// The per-Character slots of one Channel runtime. Idle slots are dropped, so the map holds at
/// most the Characters with a write queued or in flight, plus the one being acquired.
#[derive(Debug, Default)]
pub struct CharacterRevisionSequencer {
    slots: std::sync::Mutex<HashMap<CharacterId, Arc<SlotQueue<Option<CharacterRevision>>>>>,
}

impl CharacterRevisionSequencer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Wait for the slot of `character_id`. Requests are granted in arrival order. The caller
    /// must not hold the runtime lock while it waits.
    pub async fn acquire(&self, character_id: CharacterId) -> RevisionSlot {
        RevisionSlot {
            character_id,
            cursor: self.queue(character_id).lock_owned().await,
        }
    }

    /// A background projection may defer while an advancing writer owns the slot.
    pub fn try_acquire(&self, character_id: CharacterId) -> Option<RevisionSlot> {
        Some(RevisionSlot {
            character_id,
            cursor: self.queue(character_id).try_lock_owned().ok()?,
        })
    }

    fn queue(&self, character_id: CharacterId) -> Arc<SlotQueue<Option<CharacterRevision>>> {
        let mut slots = self.slots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        slots.retain(|_, queue| Arc::strong_count(queue) > 1);
        Arc::clone(slots.entry(character_id).or_default())
    }
}

/// The exclusive right to advance one Character's revision, with its cursor.
#[derive(Debug)]
pub struct RevisionSlot {
    character_id: CharacterId,
    cursor: OwnedMutexGuard<Option<CharacterRevision>>,
}

/// How a mismatch is handled (§5.2): by whether the request's binding includes the revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnMismatch {
    FailClosed,
    RetryOnce,
}

/// The revision a write expects: the cursor for a new request, or the original revision of a
/// retained receipt for its exact replay, which returns that receipt and is never retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Cursor(OnMismatch),
    Replay(CharacterRevision),
}

impl Expect {
    fn from(original: Option<CharacterRevision>, on_mismatch: OnMismatch) -> Self {
        original.map_or(Self::Cursor(on_mismatch), Self::Replay)
    }
}

impl RevisionSlot {
    /// Quest/Hazard descendants can follow XP in one transaction. Reload the
    /// live root after that commit or an unknown outcome, never a receipt cursor.
    pub fn observe_endgame_progression(&mut self) {
        *self.cursor = None;
    }

    /// A citizenship choice may advance the root. Reload after success or an unknown outcome.
    pub fn observe_home_town(&mut self) {
        *self.cursor = None;
    }

    /// A composed loop-training transaction (or an unknown outcome) changed the root.
    /// The next queued writer reloads it; no historical receipt grants a current cursor.
    pub fn observe_loop_training(&mut self) {
        *self.cursor = None;
    }

    #[must_use]
    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// The revision the next write of this slot expects, read from the Character root when the
    /// slot holds none.
    pub async fn cursor(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<CharacterRevision, CharacterAuthorityError> {
        self.load_cursor(root_revision(root, authority)).await
    }

    async fn load_cursor<L, LFut>(
        &mut self,
        mut load: L,
    ) -> Result<CharacterRevision, CharacterAuthorityError>
    where
        L: FnMut(CharacterId) -> LFut,
        LFut: Future<Output = Result<CharacterRevision, CharacterAuthorityError>>,
    {
        if let Some(revision) = *self.cursor {
            return Ok(revision);
        }
        let revision = load(self.character_id).await?;
        *self.cursor = Some(revision);
        Ok(revision)
    }

    /// An atomic multi-Character XP transaction has ended. Unknown outcomes invalidate the
    /// cursor, and replayed older receipts cannot move it backwards.
    pub fn observe_party_experience(
        &mut self,
        outcome: &Result<ExperienceCommitOutcome, CharacterProgressionError>,
    ) {
        self.settle(outcome);
    }

    /// One XP award at the cursor, or the exact replay of a retained receipt at its `original`
    /// revision. A mismatch fails closed.
    pub async fn commit_experience<const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ExperienceAwardRequest<N>,
        original: Option<CharacterRevision>,
    ) -> Result<ExperienceCommitOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| root.commit_character_experience(authority, node, fence, request.clone()),
        )
        .await
    }

    /// One player death at the cursor, or the exact replay of a retained receipt at its
    /// `original` revision. A mismatch fails closed.
    pub async fn commit_death<const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        original: Option<CharacterRevision>,
    ) -> Result<CharacterDeathOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| root.commit_character_death(authority, node, fence, request.clone()),
        )
        .await
    }

    /// Build-aware death uses the same sequencer cursor and retained-receipt
    /// original revision as the compatibility death method. The writer resolves
    /// current fences and build before-state in its transaction.
    #[allow(
        clippy::too_many_arguments,
        reason = "existing sequenced death inputs plus the current build formula"
    )]
    pub async fn commit_death_with_build<const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        formula: &dyn BuildFormula,
        original: Option<CharacterRevision>,
    ) -> Result<CharacterDeathOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| {
                root.commit_character_death_with_build(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    formula,
                )
            },
        )
        .await
    }

    /// One Bestiary kill at the cursor. Its binding excludes the revision, so a mismatch reloads
    /// the cursor and retries once.
    pub async fn commit_bestiary(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BestiaryKillRequest,
    ) -> Result<BestiaryKillOutcome, BestiaryProgressError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| root.commit_bestiary_kill(authority, node, fence, request.clone()),
        )
        .await
    }

    /// A retained, owner-qualified death keeps its original talisman decision
    /// while a Character revision mismatch reloads only the sequencing cursor.
    pub async fn commit_bestiary_with_talisman(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BestiaryKillRequest,
        captured: Option<super::bestiary_progress::CapturedBountyTalismanKill>,
    ) -> Result<BestiaryKillOutcome, BestiaryProgressError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_bestiary_kill_with_talisman(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    captured.clone(),
                )
            },
        )
        .await
    }

    /// One Charm command (with any fee burn it runs in its transaction) at the cursor, or the
    /// exact replay of a retained receipt at its `original` revision. A mismatch fails closed.
    #[allow(
        clippy::too_many_arguments,
        reason = "the Charm writer's own arguments plus the replay revision"
    )]
    pub async fn commit_charm<F: CharmFacts + Clone>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharmCommandRequest,
        facts: F,
        original: Option<CharacterRevision>,
    ) -> Result<CharmCommandOutcome, CharmStateError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| {
                root.commit_charm_command(authority, node, fence, request.clone(), facts.clone())
            },
        )
        .await
    }

    /// One monk state save at the cursor. A mismatch fails closed.
    pub async fn commit_monk_state_save(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: MonkStateSaveRequest,
    ) -> Result<MonkStateSaveOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::FailClosed),
            |fence| root.commit_character_monk_state_save(authority, node, fence, request),
        )
        .await
    }

    /// One build change at the cursor, or the exact replay of a retained receipt at its
    /// `original` revision. A mismatch fails closed.
    #[allow(
        clippy::too_many_arguments,
        dead_code,
        reason = "the build writer's own arguments plus the replay revision; standalone durability \
                  suites path-load this module without build cases"
    )]
    pub async fn commit_build(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BuildChangeRequest,
        formula: &dyn BuildFormula,
        original: Option<CharacterRevision>,
    ) -> Result<BuildCommitOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| root.commit_character_build(authority, node, fence, request.clone(), formula),
        )
        .await
    }

    pub async fn commit_newhaven_build(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BuildChangeRequest,
        context: NewhavenBuildContext<'_>,
    ) -> Result<BuildCommitOutcome, CharacterProgressionError> {
        let NewhavenBuildContext { formula, kit } = context;
        let super::starter_grant::NewhavenKit { content, definitions } = kit;
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Replay(fence.expected_character_revision),
            |fence| {
                root.commit_character_build_with_kit(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    formula,
                    Some(super::starter_grant::NewhavenKit { content, definitions }),
                )
            },
        )
        .await
    }
}

include!("character_revision_sequencer/progression_writers.rs");

include!("character_revision_sequencer/world_writers.rs");

impl RevisionSlot {
    async fn sequenced<T, E, L, LFut, W, Fut>(
        &mut self,
        mut load: L,
        fence: CurrentCharacterGameplayFence,
        expect: Expect,
        mut write: W,
    ) -> Result<T, E>
    where
        T: SequencedOutcome,
        E: SequencedError,
        L: FnMut(CharacterId) -> LFut,
        LFut: Future<Output = Result<CharacterRevision, CharacterAuthorityError>>,
        W: FnMut(CurrentCharacterGameplayFence) -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        if fence.character_id != self.character_id {
            return Err(E::rejected());
        }
        let mut retry = expect == Expect::Cursor(OnMismatch::RetryOnce);
        loop {
            let expected = match expect {
                Expect::Cursor(_) => self
                    .load_cursor(&mut load)
                    .await
                    .map_err(E::from_authority)?,
                Expect::Replay(original) => original,
            };
            let result = write(CurrentCharacterGameplayFence {
                expected_character_revision: expected,
                ..fence
            })
            .await;
            self.settle(&result);
            if retry && result.as_ref().is_err_and(SequencedError::is_mismatch) {
                retry = false;
                continue;
            }
            return result;
        }
    }

    /// Move the cursor by one write's result: to a receipt's committed revision (never back),
    /// or unknown after an unknown outcome or a mismatch. A refusal that wrote nothing keeps it.
    fn settle<T: SequencedOutcome, E: SequencedError>(&mut self, result: &Result<T, E>) {
        match result {
            Ok(outcome) => {
                // A receipt may be historical. Only a loaded root revision can seed the cursor.
                if let (Some(committed), Some(cursor)) = (outcome.committed_revision(), *self.cursor) {
                    *self.cursor = Some(cursor.max(committed));
                }
            }
            Err(error) if error.is_mismatch() => {
                // With the sequencer in place only a writer that bypassed it can move the
                // revision under a held slot.
                eprintln!(
                    "oteryn-game-server defect: CharacterRevisionMismatch under a held revision \
                     slot; a revision-advancing writer bypassed the sequencer"
                );
                *self.cursor = None;
            }
            Err(error) if error.is_unknown() => *self.cursor = None,
            Err(_) => {}
        }
    }
}

/// The current revision of the Character root, the cursor's source.
fn root_revision<'r>(
    root: &'r DurabilityRoot,
    authority: &'r ReconciledCharacterAuthority<'_, '_>,
) -> impl FnMut(
    CharacterId,
) -> std::pin::Pin<
    Box<dyn Future<Output = Result<CharacterRevision, CharacterAuthorityError>> + Send + 'r>,
> + 'r {
    move |character_id| {
        Box::pin(async move {
            Ok(root
                .read_current_character(authority, character_id)
                .await?
                .revision)
        })
    }
}

include!("character_revision_sequencer/outcomes.rs");

include!("character_revision_sequencer/slot_tests.rs");
