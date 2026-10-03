//! One revision-advancing write in flight per Character (CHAR-REV-SEQ-1, QUEST-STATE-0 §5.2).
//!
//! The owning Channel runtime holds one [`CharacterRevisionSequencer`]. Every write that advances
//! a CharacterRevision (XP, death, Bestiary, charm with its in-transaction fee burn, monk state
//! save, build, proficiency, quest transition) runs through a [`RevisionSlot`] of that Character: the slot is an asynchronous FIFO
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
//! quest transition) reloads the cursor and is retried once; a write whose binding includes it (XP, death, charm,
//! monk, build, proficiency) is not retried: it fails closed and is reported as a defect.
//!
//! The slot is not the runtime lock and adds no fence: callers never hold the runtime lock while
//! they wait for a slot or across the durable write, and every write is still refused by the
//! existing session-generation gameplay fence in its own transaction.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex as SlotQueue, OwnedMutexGuard};

use super::DurabilityError;
use super::DurabilityRoot;
use super::bestiary_progress::{BestiaryKillOutcome, BestiaryKillRequest, BestiaryProgressError};
use super::character_authority::{CharacterAuthorityError, ReconciledCharacterAuthority};
use super::character_build::{BuildChangeRequest, BuildCommitOutcome, BuildFormula};
use super::character_death::{CharacterDeathOutcome, CharacterDeathRequest};
use super::character_proficiency::{
    ProficiencyChangeRequest, ProficiencyCommitOutcome, ProficiencyDefinitions,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome,
};
use super::charm_state::{CharmCommandOutcome, CharmCommandRequest, CharmFacts, CharmStateError};
use super::monk_state::{MonkStateSaveOutcome, MonkStateSaveRequest};
use super::quest_state::quest::QuestStateCatalogue;
use super::quest_state::{QuestTransitionOutcome, QuestTransitionRequest};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::{CharacterId, CharacterRevision};

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
        let queue = {
            let mut slots = self
                .slots
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // A slot nobody holds or waits for keeps only a cursor the Character root restores.
            slots.retain(|_, queue| Arc::strong_count(queue) > 1);
            Arc::clone(slots.entry(character_id).or_default())
        };
        RevisionSlot {
            character_id,
            cursor: queue.lock_owned().await,
        }
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

    /// One proficiency change at the cursor, or the exact replay of a retained receipt at its
    /// `original` revision. A mismatch fails closed; a refusal writes nothing.
    #[allow(
        clippy::too_many_arguments,
        dead_code,
        reason = "the proficiency writer's own arguments plus the replay revision; standalone durability \
                  suites path-load this module without proficiency cases"
    )]
    pub async fn commit_proficiency(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ProficiencyChangeRequest,
        policy: std::sync::Arc<dyn ProficiencyDefinitions>,
        original: Option<CharacterRevision>,
    ) -> Result<ProficiencyCommitOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| {
                root.commit_character_proficiency(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&policy),
                )
            },
        )
        .await
    }

    /// One quest transition at the cursor (QUEST-STATE-0 §5.2). Its binding excludes the
    /// revision, so a mismatch reloads the cursor and retries once (replay or one commit); a
    /// refusal writes nothing.
    #[allow(
        dead_code,
        reason = "standalone durability suites path-load this module without quest cases"
    )]
    pub async fn commit_quest_transition(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_quest_transition(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                )
            },
        )
        .await
    }

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
                if let Some(committed) = outcome.committed_revision() {
                    let cursor = self
                        .cursor
                        .map_or(committed, |cursor| cursor.max(committed));
                    *self.cursor = Some(cursor);
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

/// A sequenced write's success: the revision it committed or replayed, if any.
trait SequencedOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision>;
}

impl SequencedOutcome for ExperienceCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for CharacterDeathOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for BestiaryKillOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Saturated { .. } => None,
        }
    }
}

impl SequencedOutcome for CharmCommandOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for BuildCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for ProficiencyCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Refused(_) => None,
        }
    }
}

impl SequencedOutcome for QuestTransitionOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Refused(_) | Self::ObligationClosed => None,
        }
    }
}

impl SequencedOutcome for MonkStateSaveOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Unchanged => None,
        }
    }
}

/// A sequenced write's error, as the slot classifies it.
trait SequencedError {
    fn is_mismatch(&self) -> bool;
    /// The write may or may not have committed.
    fn is_unknown(&self) -> bool;
    fn rejected() -> Self;
    fn unavailable(error: DurabilityError) -> Self;

    fn from_authority(error: CharacterAuthorityError) -> Self
    where
        Self: Sized,
    {
        match error {
            CharacterAuthorityError::Unavailable(error) => Self::unavailable(error),
            CharacterAuthorityError::Rejected
            | CharacterAuthorityError::Conflict
            | CharacterAuthorityError::NameUnavailable => Self::rejected(),
        }
    }
}

impl SequencedError for CharacterProgressionError {
    fn is_mismatch(&self) -> bool {
        matches!(self, Self::CharacterRevisionMismatch)
    }
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
    fn rejected() -> Self {
        Self::AuthorityRejected
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl SequencedError for BestiaryProgressError {
    fn is_mismatch(&self) -> bool {
        matches!(self, Self::CharacterRevisionMismatch)
    }
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
    fn rejected() -> Self {
        Self::AuthorityRejected
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl SequencedError for CharmStateError {
    fn is_mismatch(&self) -> bool {
        matches!(self, Self::CharacterRevisionMismatch)
    }
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
    fn rejected() -> Self {
        Self::AuthorityRejected
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Receipt(Option<CharacterRevision>);

    impl SequencedOutcome for Receipt {
        fn committed_revision(&self) -> Option<CharacterRevision> {
            self.0
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Failure {
        Mismatch,
        Unknown,
        Refused,
    }

    impl SequencedError for Failure {
        fn is_mismatch(&self) -> bool {
            *self == Self::Mismatch
        }
        fn is_unknown(&self) -> bool {
            *self == Self::Unknown
        }
        fn rejected() -> Self {
            Self::Refused
        }
        fn unavailable(_: DurabilityError) -> Self {
            Self::Unknown
        }
    }

    fn character(seed: u8) -> CharacterId {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        CharacterId::from_bytes(bytes).expect("UUIDv7 bytes")
    }

    fn revision(value: u64) -> CharacterRevision {
        CharacterRevision::new(value).expect("non-zero revision")
    }

    fn fence(character_id: CharacterId) -> CurrentCharacterGameplayFence {
        use crate::foundation::{
            ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
            ScopeOwnershipGeneration, WorldId,
        };
        let bytes = character_id.as_bytes();
        CurrentCharacterGameplayFence {
            character_id,
            game_session_id: GameSessionId::decode(bytes).expect("session"),
            connection_generation: ConnectionGeneration::new(1).expect("generation"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(bytes).expect("world"),
                ChannelId::decode(bytes).expect("channel"),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("generation"),
            expected_character_revision: revision(1),
        }
    }

    /// Both futures to completion, polled in turn on one task (no `tokio::join!` here).
    async fn join<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
        use std::task::Poll;
        let (mut a, mut b) = (std::pin::pin!(a), std::pin::pin!(b));
        let (mut left, mut right) = (None, None);
        std::future::poll_fn(|context| {
            if left.is_none()
                && let Poll::Ready(output) = a.as_mut().poll(context)
            {
                left = Some(output);
            }
            if right.is_none()
                && let Poll::Ready(output) = b.as_mut().poll(context)
            {
                right = Some(output);
            }
            match (left.take(), right.take()) {
                (Some(l), Some(r)) => Poll::Ready((l, r)),
                (l, r) => {
                    (left, right) = (l, r);
                    Poll::Pending
                }
            }
        })
        .await
    }

    /// The Character root of one test: the durable revision a write is fenced against.
    struct Durable(AtomicU64);

    impl Durable {
        fn load(
            &self,
        ) -> impl FnMut(
            CharacterId,
        )
            -> std::future::Ready<Result<CharacterRevision, CharacterAuthorityError>>
        + '_ {
            |_| std::future::ready(Ok(revision(self.0.load(Ordering::SeqCst))))
        }

        /// A revision-advancing write: commits one revision when `expected` is current, as the
        /// root row lock does.
        async fn write(&self, expected: CharacterRevision) -> Result<Receipt, Failure> {
            tokio::task::yield_now().await;
            let next = expected.get() + 1;
            self.0
                .compare_exchange(expected.get(), next, Ordering::SeqCst, Ordering::SeqCst)
                .map_err(|_| Failure::Mismatch)?;
            tokio::task::yield_now().await;
            Ok(Receipt(Some(revision(next))))
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime")
            .block_on(future)
    }

    #[test]
    fn concurrent_writers_of_one_character_commit_in_sequence_without_mismatch() {
        let durable = Durable(AtomicU64::new(7));
        let sequencer = CharacterRevisionSequencer::new();
        let id = character(1);
        let writer = || async {
            let mut slot = sequencer.acquire(id).await;
            slot.sequenced(
                durable.load(),
                fence(id),
                Expect::Cursor(OnMismatch::FailClosed),
                |fence| durable.write(fence.expected_character_revision),
            )
            .await
        };
        // An XP award and a charm command arriving together, interleaved at every await.
        let (xp, charm) = block_on(join(writer(), writer()));
        let mut committed = [xp.expect("XP commits"), charm.expect("charm commits")]
            .map(|receipt| receipt.0.expect("revision").get());
        committed.sort_unstable();
        assert_eq!(committed, [8, 9]);
        assert_eq!(durable.0.load(Ordering::SeqCst), 9);
    }

    #[test]
    fn unsequenced_writers_race_into_a_mismatch() {
        // The defect class the sequencer removes: both read revision 7, one loses.
        let durable = Durable(AtomicU64::new(7));
        let (first, second) =
            block_on(join(durable.write(revision(7)), durable.write(revision(7))));
        assert!(first.is_ok() != second.is_ok());
    }

    #[test]
    fn a_composition_holds_the_slot_for_its_chain() {
        let durable = Durable(AtomicU64::new(3));
        let sequencer = CharacterRevisionSequencer::new();
        let id = character(2);
        let interloper_saw = Rc::new(Cell::new(0));
        block_on(async {
            let chain = async {
                let mut slot = sequencer.acquire(id).await;
                let xp = slot
                    .sequenced(
                        durable.load(),
                        fence(id),
                        Expect::Cursor(OnMismatch::FailClosed),
                        |fence| durable.write(fence.expected_character_revision),
                    )
                    .await
                    .expect("XP commits");
                tokio::task::yield_now().await;
                let bestiary_expected = Cell::new(None);
                slot.sequenced(
                    durable.load(),
                    fence(id),
                    Expect::Cursor(OnMismatch::RetryOnce),
                    |fence| {
                        bestiary_expected.set(Some(fence.expected_character_revision));
                        durable.write(fence.expected_character_revision)
                    },
                )
                .await
                .expect("Bestiary commits");
                // Bestiary takes the revision XP committed.
                assert_eq!(bestiary_expected.get(), xp.0);
            };
            let interloper = async {
                tokio::task::yield_now().await;
                let _slot = sequencer.acquire(id).await;
                interloper_saw.set(durable.0.load(Ordering::SeqCst));
            };
            join(chain, interloper).await;
        });
        // The waiting request ran only after the whole chain.
        assert_eq!(interloper_saw.get(), 5);
    }

    #[test]
    fn a_bestiary_mismatch_reloads_the_cursor_and_retries_once() {
        let id = character(3);
        let sequencer = CharacterRevisionSequencer::new();
        let durable = Durable(AtomicU64::new(10));
        let attempts = Cell::new(0);
        let result = block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(9)); // a bypass moved the root under the cursor
            slot.sequenced(
                durable.load(),
                fence(id),
                Expect::Cursor(OnMismatch::RetryOnce),
                |fence| {
                    attempts.set(attempts.get() + 1);
                    durable.write(fence.expected_character_revision)
                },
            )
            .await
        });
        assert_eq!(attempts.get(), 2);
        assert_eq!(result, Ok(Receipt(Some(revision(11)))));

        // A second mismatch is not retried again.
        let attempts = Cell::new(0);
        let result = block_on(async {
            let mut slot = sequencer.acquire(id).await;
            slot.sequenced(
                durable.load(),
                fence(id),
                Expect::Cursor(OnMismatch::RetryOnce),
                |_| {
                    attempts.set(attempts.get() + 1);
                    std::future::ready(Err::<Receipt, _>(Failure::Mismatch))
                },
            )
            .await
        });
        assert_eq!((attempts.get(), result), (2, Err(Failure::Mismatch)));
    }

    #[test]
    fn a_revision_bound_mismatch_fails_closed_without_retry() {
        let id = character(4);
        let sequencer = CharacterRevisionSequencer::new();
        let durable = Durable(AtomicU64::new(10));
        let attempts = Cell::new(0);
        block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(9));
            let result = slot
                .sequenced(
                    durable.load(),
                    fence(id),
                    Expect::Cursor(OnMismatch::FailClosed),
                    |fence| {
                        attempts.set(attempts.get() + 1);
                        durable.write(fence.expected_character_revision)
                    },
                )
                .await;
            assert_eq!(result, Err(Failure::Mismatch));
            // The cursor is dropped, so the next write reloads it from the root.
            assert_eq!(*slot.cursor, None);
        });
        assert_eq!(attempts.get(), 1);
        assert_eq!(durable.0.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn a_replay_uses_its_original_revision_once() {
        let id = character(11);
        let sequencer = CharacterRevisionSequencer::new();
        let seen = Cell::new(None);
        let attempts = Cell::new(0);
        let result = block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(9));
            slot.sequenced(
                |_| std::future::ready(Ok(revision(9))),
                fence(id),
                Expect::Replay(revision(4)),
                |fence| {
                    attempts.set(attempts.get() + 1);
                    seen.set(Some(fence.expected_character_revision));
                    std::future::ready(Ok::<_, Failure>(Receipt(Some(revision(5)))))
                },
            )
            .await
        });
        assert_eq!(result, Ok(Receipt(Some(revision(5)))));
        assert_eq!((attempts.get(), seen.get()), (1, Some(revision(4))));
    }

    #[test]
    fn the_cursor_moves_only_on_receipts_and_never_back() {
        let id = character(5);
        let sequencer = CharacterRevisionSequencer::new();
        block_on(async {
            let mut slot = sequencer.acquire(id).await;
            *slot.cursor = Some(revision(6));
            slot.settle::<Receipt, Failure>(&Ok(Receipt(Some(revision(4)))));
            assert_eq!(*slot.cursor, Some(revision(6)), "a replayed older receipt");
            slot.settle::<Receipt, Failure>(&Ok(Receipt(None)));
            assert_eq!(*slot.cursor, Some(revision(6)), "nothing written");
            slot.settle::<Receipt, Failure>(&Err(Failure::Refused));
            assert_eq!(*slot.cursor, Some(revision(6)), "a refusal wrote nothing");
            slot.settle::<Receipt, Failure>(&Ok(Receipt(Some(revision(7)))));
            assert_eq!(*slot.cursor, Some(revision(7)));
            slot.settle::<Receipt, Failure>(&Err(Failure::Unknown));
            assert_eq!(*slot.cursor, None, "an unknown outcome");
        });
    }

    #[test]
    fn a_slot_refuses_another_characters_fence() {
        let sequencer = CharacterRevisionSequencer::new();
        let result = block_on(async {
            let mut slot = sequencer.acquire(character(6)).await;
            slot.sequenced(
                |_| std::future::ready(Ok(revision(1))),
                fence(character(7)),
                Expect::Cursor(OnMismatch::FailClosed),
                |_| std::future::ready(Ok::<_, Failure>(Receipt(None))),
            )
            .await
        });
        assert_eq!(result, Err(Failure::Refused));
    }

    #[test]
    fn idle_slots_are_dropped() {
        let sequencer = CharacterRevisionSequencer::new();
        block_on(async {
            drop(sequencer.acquire(character(8)).await);
            let _held = sequencer.acquire(character(9)).await;
            drop(sequencer.acquire(character(10)).await);
        });
        let slots = sequencer.slots.lock().expect("slots");
        assert!(slots.contains_key(&character(9)) && slots.len() <= 2);
    }

    /// Revision-advancing durable writers and the only non-test source files allowed to call
    /// them. Every other writer reaches them through a [`RevisionSlot`].
    const SEQUENCED_WRITERS: [(&str, &str); 9] = [
        (
            ".commit_character_experience(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_character_death(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_bestiary_kill(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_charm_command(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_character_monk_state_save(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_character_build(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_character_proficiency(",
            "durability/character_revision_sequencer.rs",
        ),
        (
            ".commit_character_quest_transition(",
            "durability/character_revision_sequencer.rs",
        ),
        // The fee burn runs only inside its source's sequenced Character transaction.
        ("burn_fee_in_transaction(", "durability/charm_state.rs"),
    ];

    /// The non-test part of a source file: from the first `#[cfg(test)]` that, after any other
    /// attributes, opens a `mod`, the rest is cut off.
    fn production_source(source: &str) -> &str {
        let mut offset = 0;
        let mut lines = source.lines();
        while let Some(line) = lines.next() {
            if line.trim() == "#[cfg(test)]" {
                let mut ahead = lines.clone().map(str::trim);
                if ahead
                    .find(|next| !next.starts_with("#["))
                    .is_some_and(|next| next.starts_with("mod "))
                {
                    return &source[..offset];
                }
            }
            offset += line.len() + 1;
        }
        source
    }

    fn bypasses(path: &str, source: &str) -> Vec<String> {
        let source = production_source(source);
        SEQUENCED_WRITERS
            .iter()
            .filter(|(call, allowed)| {
                !path.ends_with(allowed)
                    && source.lines().any(|line| {
                        let code = line.split("//").next().unwrap_or_default();
                        code.contains(call) && !code.contains("fn ")
                    })
            })
            .map(|(call, _)| format!("{path}: {call}"))
            .collect()
    }

    fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("source directory") {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "tests.rs" || name.ends_with("_tests.rs"))
            {
                out.push(path);
            }
        }
    }

    #[test]
    fn no_production_writer_advances_a_revision_outside_the_sequencer() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        sources(&root, &mut files);
        assert!(
            files.len() > 100,
            "the scan must see the game-server sources"
        );
        let mut found = Vec::new();
        for file in files {
            let source = std::fs::read_to_string(&file).expect("source file");
            let path = file.to_string_lossy().replace('\\', "/");
            found.extend(bypasses(&path, &source));
        }
        assert!(found.is_empty(), "unsequenced revision writers: {found:?}");
    }

    #[test]
    fn the_structural_gate_catches_a_bypass_writer() {
        let bypass = "async fn award(root: &DurabilityRoot) {\n    \
                      let _ = root\n        .commit_character_experience(a, n, f, r)\n        .await;\n}\n";
        assert_eq!(
            bypasses("src/combat/new_writer.rs", bypass),
            ["src/combat/new_writer.rs: .commit_character_experience("]
        );
        let fee = "burn_fee_in_transaction(&mut tx, &fence, &request).await?;";
        assert_eq!(bypasses("src/durability/item_transfer.rs", fee).len(), 1);
        assert!(bypasses("src/durability/charm_state.rs", fee).is_empty());
        // Test modules and definitions are not writers.
        let tested = "pub async fn commit_charm_command(\n#[cfg(test)]\n#[allow(clippy::expect_used)]\nmod tests { x.commit_charm_command(); }";
        assert!(bypasses("src/durability/charm_state.rs", tested).is_empty());
        assert!(bypasses("src/gameplay_transport/charm_native.rs", tested).is_empty());
    }
}
