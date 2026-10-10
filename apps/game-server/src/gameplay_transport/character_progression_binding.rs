//! PROGRESSION-OWNER-1 (ARCH-PROGRESSION-SOURCE-0 §1.4-§1.5): the Character progression step
//! of one fresh admission. It decides, from current durable reads, whether the admitted
//! session holds a progression binding, and initializes the progression row of a new
//! Character (root revision 1) with the binding's exact request.
//!
//! Depends only on the durable owners, so the PostgreSQL targets include it as it is.

use crate::domain::progression::FiniteProgressionPolicy;
use crate::durability::DurabilityError;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CharacterProgressionState, CurrentCharacterGameplayFence,
    ProgressionInitializationOutcome, ProgressionInitializationRequest,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use std::future::Future;
use std::time::Duration;

/// Why an admitted session holds no progression binding (§1.5). Each is decided once, at
/// admission; the session's deaths then respawn non-durably and its kills award no XP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgressionUnbound {
    /// No row, and the Character root is past revision 1: there is no backfill.
    Uninitialized,
    /// The row's stored revisions are not the binding's.
    ContextMismatch,
    /// The row's `character_revision` is not the root revision of the session's fence.
    StaleRevision,
    /// The bounded rounds ended without a decision; the session is not input-eligible.
    InitializationUnavailable,
}

impl ProgressionUnbound {
    /// The `progression_unbound reason=` value of the admission log.
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            Self::Uninitialized => "progression_uninitialized",
            Self::ContextMismatch => "progression_context_mismatch",
            Self::StaleRevision => "progression_stale_revision",
            Self::InitializationUnavailable => "progression_initialization_unavailable",
        }
    }
}

/// The outcome of the step: the policy the session's binding is built from, or why it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgressionBinding<const N: usize> {
    Bound(Box<FiniteProgressionPolicy<String, N>>),
    Unbound(ProgressionUnbound),
}

/// The durable owners the step reads and writes through.
pub(crate) struct ProgressionDurable<'a, 'f, 's> {
    pub(crate) root: &'a DurabilityRoot,
    pub(crate) character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(crate) holder: &'a NodeIncarnationProof,
}

/// The bounded rounds of the step, like the first entry's (`RECONCILE_ATTEMPTS`,
/// `RECONCILE_BACKOFF`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProgressionRounds {
    pub(crate) attempts: u32,
    pub(crate) backoff: Duration,
}

enum Round<const N: usize> {
    Decided(ProgressionBinding<N>),
    Retry,
}

/// §1.5: bind the admitted session's progression.
///
/// `policy_for` forms the World's pinned policy with the Character root's profile, ruleset and
/// content revisions (`None`: the content forms no policy for them). `fence` reads the session's
/// current gameplay fence: `Ok(None)` when the session is proven terminal, `Err` when a read
/// failed. The request is formed once, from the first interpretation read, and every round sends
/// it unchanged, so a round whose initialization committed with its acknowledgement lost is
/// reconciled by the next round's `AlreadyInitialized`.
pub(crate) async fn bind_character_progression<const N: usize, P, F, Fut>(
    durable: ProgressionDurable<'_, '_, '_>,
    policy_for: P,
    mut fence: F,
    rounds: ProgressionRounds,
) -> ProgressionBinding<N>
where
    P: Fn(&str, &str, &str) -> Option<FiniteProgressionPolicy<String, N>>,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<CurrentCharacterGameplayFence>, ()>>,
{
    let mut request: Option<ProgressionInitializationRequest<N>> = None;
    for round in 1..=rounds.attempts.max(1) {
        if round > 1 {
            tokio::time::sleep(rounds.backoff).await;
        }
        if request.is_none() {
            let Ok(interpretation) = durable
                .root
                .read_current_character_interpretation(durable.character)
                .await
            else {
                continue;
            };
            let [profile, ruleset, content, _] = interpretation.revisions();
            let Some(policy) = policy_for(profile, ruleset, content) else {
                return ProgressionBinding::Unbound(ProgressionUnbound::ContextMismatch);
            };
            request = Some(ProgressionInitializationRequest {
                context: policy.context.clone(),
                policy_revision: policy.policy_revision.clone(),
                reward_revision: policy.reward_revision.clone(),
                policy,
            });
        }
        let Some(current) = request.as_ref() else {
            continue;
        };
        match bind_round(&durable, current, &mut fence).await {
            Round::Decided(binding) => return binding,
            Round::Retry => {}
        }
    }
    ProgressionBinding::Unbound(ProgressionUnbound::InitializationUnavailable)
}

/// One round: the fence and the row are read in the same round, and the initializer runs only
/// for a missing row at root revision 1.
async fn bind_round<const N: usize, F, Fut>(
    durable: &ProgressionDurable<'_, '_, '_>,
    request: &ProgressionInitializationRequest<N>,
    fence: &mut F,
) -> Round<N>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<CurrentCharacterGameplayFence>, ()>>,
{
    use ProgressionUnbound::{ContextMismatch, InitializationUnavailable, StaleRevision};
    let fence = match fence().await {
        Ok(Some(fence)) => fence,
        Ok(None) => return Round::Decided(ProgressionBinding::Unbound(InitializationUnavailable)),
        Err(()) => return Round::Retry,
    };
    let row = match durable
        .root
        .read_character_progression(durable.character, fence.character_id)
        .await
    {
        Ok(row) => row,
        Err(error) => return decide_error(&error),
    };
    let bound = || Round::Decided(ProgressionBinding::Bound(Box::new(request.policy.clone())));
    match row {
        Some(state) if !stored_matches(&state, request) => {
            Round::Decided(ProgressionBinding::Unbound(ContextMismatch))
        }
        Some(state) if state.character_revision != fence.expected_character_revision => {
            Round::Decided(ProgressionBinding::Unbound(StaleRevision))
        }
        Some(_) => bound(),
        None if fence.expected_character_revision.get() != 1 => Round::Decided(
            ProgressionBinding::Unbound(ProgressionUnbound::Uninitialized),
        ),
        None => match initialize(durable, fence, request.clone()).await {
            Ok(
                ProgressionInitializationOutcome::Initialized(_)
                | ProgressionInitializationOutcome::AlreadyInitialized(_),
            ) => bound(),
            Err(error) => decide_error(&error),
        },
    }
}

/// The eight stored revisions are the request's (`stored_context_matches`).
fn stored_matches<const N: usize>(
    state: &CharacterProgressionState,
    request: &ProgressionInitializationRequest<N>,
) -> bool {
    state.context == request.context
        && state.policy_revision == request.policy_revision
        && state.reward_revision == request.reward_revision
}

/// A context mismatch and an invalid stored state are decided at once; an unavailable holder,
/// a stale fence or a root that moved under the round is retried; anything else can never bind.
fn decide_error<const N: usize>(error: &CharacterProgressionError) -> Round<N> {
    use ProgressionUnbound::{ContextMismatch, InitializationUnavailable, Uninitialized};
    match error {
        CharacterProgressionError::ProgressionContextMismatch => {
            Round::Decided(ProgressionBinding::Unbound(ContextMismatch))
        }
        CharacterProgressionError::Unavailable(DurabilityError::InvalidStoredState) => {
            Round::Decided(ProgressionBinding::Unbound(Uninitialized))
        }
        CharacterProgressionError::Unavailable(_)
        | CharacterProgressionError::AuthorityRejected
        | CharacterProgressionError::CharacterRevisionMismatch => Round::Retry,
        _ => Round::Decided(ProgressionBinding::Unbound(InitializationUnavailable)),
    }
}

async fn initialize<const N: usize>(
    durable: &ProgressionDurable<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: ProgressionInitializationRequest<N>,
) -> Result<ProgressionInitializationOutcome, CharacterProgressionError> {
    #[cfg(test)]
    let fault = seam::enter();
    #[cfg(test)]
    if fault == Some(seam::InitializerFault::FailBeforeCommit) {
        return Err(CharacterProgressionError::Unavailable(
            DurabilityError::Unavailable,
        ));
    }
    let outcome = durable
        .root
        .initialize_character_progression(durable.character, durable.holder, fence, request)
        .await;
    #[cfg(test)]
    if fault == Some(seam::InitializerFault::LoseAcknowledgement) {
        drop(outcome);
        return Err(CharacterProgressionError::Unavailable(
            DurabilityError::CommitOutcomeUnknown,
        ));
    }
    outcome
}

/// The test fault seam around the initializer call (§2.2): each call takes the next scripted
/// fault and is counted. Outside [`seam::scoped`] no fault is injected.
#[cfg(test)]
pub(crate) mod seam {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::future::Future;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum InitializerFault {
        /// The round fails before the initializer runs.
        FailBeforeCommit,
        /// The initializer runs and commits; its acknowledgement is replaced by a retryable
        /// error.
        LoseAcknowledgement,
    }

    #[derive(Default)]
    struct Script {
        faults: VecDeque<Option<InitializerFault>>,
        calls: u32,
    }

    tokio::task_local! {
        static SCRIPT: RefCell<Script>;
    }

    /// Runs `future` with `faults` applied to its initializer calls in order (`None`: no fault
    /// for that call); returns its output and the number of initializer calls.
    #[allow(dead_code, reason = "the PostgreSQL admission cases drive the seam")]
    pub(crate) async fn scoped<T>(
        faults: impl IntoIterator<Item = Option<InitializerFault>>,
        future: impl Future<Output = T>,
    ) -> (T, u32) {
        let script = RefCell::new(Script {
            faults: faults.into_iter().collect(),
            calls: 0,
        });
        SCRIPT
            .scope(script, async move {
                let output = future.await;
                let calls = SCRIPT.with(|script| script.borrow().calls);
                (output, calls)
            })
            .await
    }

    pub(super) fn enter() -> Option<InitializerFault> {
        SCRIPT
            .try_with(|script| {
                let mut script = script.borrow_mut();
                script.calls += 1;
                script.faults.pop_front().flatten()
            })
            .ok()
            .flatten()
    }
}
