//! PROGRESSION-OWNER-1 (ARCH-PROGRESSION-SOURCE-0 §1.4-§1.5): the Character progression step
//! of one fresh admission. It decides, from current durable reads, whether the admitted
//! session holds a progression binding, and initializes the progression row of a new
//! Character (root revision 1) with the binding's exact request.
//!
//! Depends only on the durable owners, so the PostgreSQL targets include it as it is.

use crate::domain::CharacterRevision;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelUpVitalMaxima, calculate_level_up_vital_change,
};
use crate::durability::DurabilityError;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CharacterProgressionState, CommittedExperienceAward,
    CurrentCharacterGameplayFence, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    ProgressionInitializationOutcome, ProgressionInitializationRequest,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use std::future::Future;
use std::time::Duration;

/// Prepared maxima replacement, never a vitals write or permission to install an actor.
/// Capacity has no admitted native source/unit yet and is deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "effect installation awaits the LOOP-PROGRESS-1 CP slot"
)]
pub struct LevelUpEffectPlan {
    pub occurrence: ExperienceRewardOccurrence,
    pub expected_before: CurrentCharacterGameplayFence,
    pub expected_after: CurrentCharacterGameplayFence,
    pub maxima_before: LevelUpVitalMaxima,
    pub maxima_after: LevelUpVitalMaxima,
    pub health_maximum_gain: u32,
    pub mana_maximum_gain: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "effect installation awaits the LOOP-PROGRESS-1 CP slot"
)]
pub enum LevelUpPlanError {
    InvalidReceipt,
    BindingMismatch,
    FactsUnavailable,
    FactsMismatch,
}

/// Prepare only a newly acknowledged XP commit. Replay never emits another effect.
///
/// Both maxima inputs are data, not qualified sources or authority. The caller must retain
/// `before_maxima` and `before_fence` from the independently admitted actor immediately before
/// this award, and independently refresh `current_fence`. `after` must project the existing
/// native `load_character_cast_facts` result at its returned root revision, with unchanged
/// vocation/magic/soul. Never reconstruct these inputs from the receipt or candidate curves.
/// That private loader hookup belongs to the CP slot: this standalone-compatible preparation
/// neither loads nor independently qualifies vocation maxima.
///
/// Even a successful plan does not prove current liveness. The future installer must atomically
/// recheck session, lease, scope and Character revision against the expected binding. Current
/// pools, durable vitals, capacity, training and revision advancement retain their owners.
#[allow(
    dead_code,
    reason = "effect installation awaits the LOOP-PROGRESS-1 CP slot"
)]
pub fn plan_committed_experience_level_up(
    outcome: &ExperienceCommitOutcome,
    before_fence: CurrentCharacterGameplayFence,
    before_maxima: LevelUpVitalMaxima,
    current_fence: CurrentCharacterGameplayFence,
    after: Option<(CharacterRevision, LevelUpVitalMaxima)>,
) -> Result<Option<LevelUpEffectPlan>, LevelUpPlanError> {
    let Some(receipt) = fresh_level_up(outcome, before_fence, before_maxima, current_fence)? else {
        return Ok(None);
    };
    let (character_revision, maxima_after) = after.ok_or(LevelUpPlanError::FactsUnavailable)?;
    if character_revision != receipt.committed_character_revision
        || maxima_after.level != receipt.level_after
    {
        return Err(LevelUpPlanError::FactsMismatch);
    }
    let gains = calculate_level_up_vital_change(before_maxima, maxima_after)
        .map_err(|_| LevelUpPlanError::FactsMismatch)?;
    Ok(Some(LevelUpEffectPlan {
        occurrence: receipt.occurrence,
        expected_before: before_fence,
        expected_after: current_fence,
        maxima_before: before_maxima,
        maxima_after,
        health_maximum_gain: gains.health_maximum_gain,
        mana_maximum_gain: gains.mana_maximum_gain,
    }))
}

#[allow(
    dead_code,
    reason = "effect installation awaits the LOOP-PROGRESS-1 CP slot"
)]
fn fresh_level_up(
    outcome: &ExperienceCommitOutcome,
    before_fence: CurrentCharacterGameplayFence,
    before_maxima: LevelUpVitalMaxima,
    current_fence: CurrentCharacterGameplayFence,
) -> Result<Option<&CommittedExperienceAward>, LevelUpPlanError> {
    let ExperienceCommitOutcome::Committed(receipt) = outcome else {
        return Ok(None);
    };
    if receipt.level_before == 0
        || receipt.level_after < receipt.level_before
        || receipt.experience_before.get() < 0
        || receipt.experience_awarded.get() <= 0
        || receipt
            .experience_before
            .checked_add(receipt.experience_awarded)
            .ok()
            != Some(receipt.experience_after)
        || receipt.original_character_revision.get().checked_add(1)
            != Some(receipt.committed_character_revision.get())
    {
        return Err(LevelUpPlanError::InvalidReceipt);
    }
    let mut expected_after = before_fence;
    expected_after.expected_character_revision = receipt.committed_character_revision;
    if before_fence.character_id != receipt.character_id
        || before_fence.expected_character_revision != receipt.original_character_revision
        || current_fence != expected_after
        || before_maxima.level != receipt.level_before
    {
        return Err(LevelUpPlanError::BindingMismatch);
    }
    Ok((receipt.level_after > receipt.level_before).then_some(receipt))
}

/// Why an admitted session holds no progression binding (§1.5). Each is decided once, at
/// admission; the session's deaths then respawn non-durably and its kills award no XP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressionUnbound {
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
    pub const fn reason(self) -> &'static str {
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
pub enum ProgressionBinding<const N: usize> {
    Bound(Box<FiniteProgressionPolicy<String, N>>),
    Unbound(ProgressionUnbound),
}

/// The durable owners the step reads and writes through.
pub struct ProgressionDurable<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub holder: &'a NodeIncarnationProof,
}

/// The bounded rounds of the step, like the first entry's (`RECONCILE_ATTEMPTS`,
/// `RECONCILE_BACKOFF`).
#[derive(Debug, Clone, Copy)]
pub struct ProgressionRounds {
    pub attempts: u32,
    pub backoff: Duration,
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
/// failed. The request is formed once, from the profile, ruleset and content revisions stored on
/// the fenced Character's own root (never the newest global interpretation, which may have
/// advanced since the Character was created), and every round sends it unchanged, so a round
/// whose initialization committed with its acknowledgement lost is reconciled by the next
/// round's `AlreadyInitialized`. A root whose interpretation is no longer the newest global one
/// is decided `ContextMismatch` in the round that reads it: the gameplay fence of every durable
/// progression write refuses it (§0.2), so a binding would leave each death of the session
/// retrying a write that can never commit.
pub async fn bind_character_progression<const N: usize, P, F, Fut>(
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
        let current_fence = match fence().await {
            Ok(Some(current_fence)) => current_fence,
            Ok(None) => {
                return ProgressionBinding::Unbound(ProgressionUnbound::InitializationUnavailable);
            }
            Err(()) => continue,
        };
        let Ok(interpretation) = durable
            .root
            .read_character_root_interpretation(durable.character, current_fence.character_id)
            .await
        else {
            continue;
        };
        let Some(interpretation) = interpretation else {
            return ProgressionBinding::Unbound(ProgressionUnbound::ContextMismatch);
        };
        if request.is_none() {
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
        match bind_round(&durable, current, current_fence).await {
            Round::Decided(binding) => return binding,
            Round::Retry => {}
        }
    }
    ProgressionBinding::Unbound(ProgressionUnbound::InitializationUnavailable)
}

/// One round: `fence` was read in this round, before the row, and the initializer runs only
/// for a missing row at root revision 1.
async fn bind_round<const N: usize>(
    durable: &ProgressionDurable<'_, '_, '_>,
    request: &ProgressionInitializationRequest<N>,
    fence: CurrentCharacterGameplayFence,
) -> Round<N> {
    use ProgressionUnbound::{ContextMismatch, StaleRevision};
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
pub mod seam {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::future::Future;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InitializerFault {
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
    pub async fn scoped<T>(
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod level_up_tests {
    use super::*;
    use crate::domain::CharacterId;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };
    use oteryn_simulation_determinism::ExactI64;

    fn id(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    fn revision(value: u64) -> CharacterRevision {
        CharacterRevision::new(value).expect("revision")
    }

    fn receipt() -> CommittedExperienceAward {
        CommittedExperienceAward {
            occurrence: ExperienceRewardOccurrence::from_bytes(id(1)).expect("occurrence"),
            character_id: CharacterId::from_bytes(id(2)).expect("character"),
            original_character_revision: revision(4),
            committed_character_revision: revision(5),
            level_before: 8,
            level_after: 10,
            experience_before: ExactI64::new(4200),
            experience_after: ExactI64::new(9300),
            experience_awarded: ExactI64::new(5100),
        }
    }

    // Independent admitted/current fixtures, never reconstructed from a receipt.
    fn admitted() -> CurrentCharacterGameplayFence {
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
            expected_character_revision: revision(4),
        }
    }

    fn current() -> CurrentCharacterGameplayFence {
        CurrentCharacterGameplayFence {
            expected_character_revision: revision(5),
            ..admitted()
        }
    }

    // Arithmetic fixtures only: these do not qualify the native vocation curves/loader.
    fn before() -> LevelUpVitalMaxima {
        LevelUpVitalMaxima {
            level: 8,
            max_health: 185,
            max_mana: 90,
        }
    }

    fn after() -> LevelUpVitalMaxima {
        LevelUpVitalMaxima {
            level: 10,
            max_health: 215,
            max_mana: 100,
        }
    }

    fn plan(
        outcome: ExperienceCommitOutcome,
        before_fence: CurrentCharacterGameplayFence,
        before_maxima: LevelUpVitalMaxima,
        current_fence: CurrentCharacterGameplayFence,
        facts: Option<(CharacterRevision, LevelUpVitalMaxima)>,
    ) -> Result<Option<LevelUpEffectPlan>, LevelUpPlanError> {
        plan_committed_experience_level_up(
            &outcome,
            before_fence,
            before_maxima,
            current_fence,
            facts,
        )
    }

    #[test]
    fn level_up_preserves_actual_receipt_and_all_expected_binding_fields() {
        let effect = plan(
            ExperienceCommitOutcome::Committed(receipt()),
            admitted(),
            before(),
            current(),
            Some((revision(5), after())),
        )
        .expect("valid plan")
        .expect("level-up");
        assert_eq!(effect.expected_before, admitted());
        assert_eq!(effect.expected_after, current());
        assert_eq!(effect.occurrence, receipt().occurrence);
        assert_eq!(
            (effect.maxima_before, effect.maxima_after),
            (before(), after())
        );
        assert_eq!(
            (effect.health_maximum_gain, effect.mana_maximum_gain),
            (30, 10)
        );
    }

    #[test]
    fn level_up_replay_never_reissues_an_effect_even_without_owner_data() {
        assert_eq!(
            plan(
                ExperienceCommitOutcome::AlreadyCommitted(receipt()),
                admitted(),
                before(),
                current(),
                None
            ),
            Ok(None)
        );
    }

    #[test]
    fn level_up_same_level_award_needs_no_maxima_reload() {
        let mut award = receipt();
        award.level_after = 8;
        award.experience_awarded = ExactI64::new(1);
        award.experience_after = ExactI64::new(4201);
        assert_eq!(
            plan(
                ExperienceCommitOutcome::Committed(award),
                admitted(),
                before(),
                current(),
                None
            ),
            Ok(None)
        );
    }

    #[test]
    fn level_up_rejects_each_replaced_session_scope_or_revision_binding() {
        let mut cases = [current(); 8];
        cases[0].character_id = CharacterId::from_bytes(id(9)).expect("other character");
        cases[1].game_session_id = GameSessionId::decode(&id(9)).expect("other session");
        cases[2].connection_generation = ConnectionGeneration::new(2).expect("generation");
        cases[3].character_lease_generation = 2;
        cases[4].runtime_scope = RuntimeScopeRefV1::channel(
            WorldId::decode(&id(9)).expect("world"),
            ChannelId::decode(&id(5)).expect("channel"),
        );
        cases[5].runtime_scope = RuntimeScopeRefV1::channel(
            WorldId::decode(&id(4)).expect("world"),
            ChannelId::decode(&id(9)).expect("channel"),
        );
        cases[6].scope_ownership_generation = ScopeOwnershipGeneration::new(2).expect("scope");
        cases[7].expected_character_revision = revision(6);
        for fence in cases {
            assert_eq!(
                plan(
                    ExperienceCommitOutcome::Committed(receipt()),
                    admitted(),
                    before(),
                    fence,
                    Some((revision(5), after()))
                ),
                Err(LevelUpPlanError::BindingMismatch)
            );
        }
    }

    #[test]
    fn level_up_rejects_stale_pre_award_revision_or_level() {
        let stale = CurrentCharacterGameplayFence {
            expected_character_revision: revision(3),
            ..admitted()
        };
        assert_eq!(
            plan(
                ExperienceCommitOutcome::Committed(receipt()),
                stale,
                before(),
                current(),
                Some((revision(5), after()))
            ),
            Err(LevelUpPlanError::BindingMismatch)
        );
        assert_eq!(
            plan(
                ExperienceCommitOutcome::Committed(receipt()),
                admitted(),
                LevelUpVitalMaxima {
                    level: 7,
                    ..before()
                },
                current(),
                Some((revision(5), after()))
            ),
            Err(LevelUpPlanError::BindingMismatch)
        );
    }

    #[test]
    fn level_up_refuses_missing_or_wrong_revision_owner_projection() {
        assert_eq!(
            plan(
                ExperienceCommitOutcome::Committed(receipt()),
                admitted(),
                before(),
                current(),
                None
            ),
            Err(LevelUpPlanError::FactsUnavailable)
        );
        for root_revision in [revision(4), revision(6)] {
            assert_eq!(
                plan(
                    ExperienceCommitOutcome::Committed(receipt()),
                    admitted(),
                    before(),
                    current(),
                    Some((root_revision, after()))
                ),
                Err(LevelUpPlanError::FactsMismatch)
            );
        }
    }

    #[test]
    fn level_up_refuses_wrong_after_level_or_regressing_maxima() {
        for changed in [
            LevelUpVitalMaxima {
                level: 11,
                ..after()
            },
            LevelUpVitalMaxima {
                max_health: 184,
                ..after()
            },
            LevelUpVitalMaxima {
                max_mana: 89,
                ..after()
            },
        ] {
            assert_eq!(
                plan(
                    ExperienceCommitOutcome::Committed(receipt()),
                    admitted(),
                    before(),
                    current(),
                    Some((revision(5), changed))
                ),
                Err(LevelUpPlanError::FactsMismatch)
            );
        }
    }

    #[test]
    fn level_up_rejects_malformed_receipt_without_reprojecting_xp_policy() {
        let base = receipt();
        let mut awards = [
            base.clone(),
            base.clone(),
            base.clone(),
            base.clone(),
            base.clone(),
            base.clone(),
            base.clone(),
        ];
        awards[0].level_before = 0;
        awards[1].level_after = 7;
        awards[2].experience_before = ExactI64::new(-1);
        awards[3].experience_awarded = ExactI64::new(0);
        awards[4].experience_after = ExactI64::new(9301);
        awards[5].committed_character_revision = revision(6);
        awards[6].experience_before = ExactI64::new(i64::MAX);
        for award in awards {
            assert_eq!(
                plan(
                    ExperienceCommitOutcome::Committed(award),
                    admitted(),
                    before(),
                    current(),
                    Some((revision(5), after()))
                ),
                Err(LevelUpPlanError::InvalidReceipt)
            );
        }
    }
}
