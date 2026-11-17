//! Disposable qualification Character setup through the real fenced writers.
//! Shared by qualification tests and the explicitly opted-in node-boot fixture. This does not install
//! actor vitals, issue entitlements, or bypass the ordinary client admission.

use crate::domain::CharacterId;
#[cfg(test)]
use crate::domain::progression::LevelThreshold;
use crate::domain::progression::{FiniteProgressionPolicy, ProgressionRevisionContext};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, NO_VOCATION,
    convert_vocation,
};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
    ProgressionInitializationRequest,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionAuthoritySnapshot};
#[cfg(test)]
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

/// Retry only holder acquisition failure, which occurs before the semantic pass runs.
/// Fenced writes retain their request/occurrence across attempts; ambiguous writes are final.
pub(super) async fn retry_holder<T, E>(
    root: Option<&DurabilityRoot>,
    deadline: tokio::time::Instant,
    busy: impl Fn(&E) -> bool,
    mut step: impl AsyncFnMut() -> Result<T, E>,
) -> Result<T, E> {
    loop {
        let outcome = step().await;
        if !outcome.as_ref().is_err_and(&busy) || tokio::time::Instant::now() >= deadline {
            return outcome;
        }
        let now = tokio::time::Instant::now();
        let latest = (now + std::time::Duration::from_millis(250)).min(deadline);
        let earliest = (now + std::time::Duration::from_millis(50)).min(latest);
        if let Some(root) = root {
            root.wait_holder_retry(earliest, latest).await;
        } else {
            tokio::time::sleep_until(earliest).await;
        }
        if tokio::time::Instant::now() >= deadline {
            return outcome;
        }
    }
}

pub(super) fn progression_busy(
    error: &crate::durability::character_progression::CharacterProgressionError,
) -> bool {
    matches!(
        error,
        crate::durability::character_progression::CharacterProgressionError::Unavailable(
            crate::durability::DurabilityError::RootUnavailable
        )
    )
}

pub(super) fn authority_busy(
    error: &crate::durability::character_authority::CharacterAuthorityError,
) -> bool {
    matches!(
        error,
        crate::durability::character_authority::CharacterAuthorityError::Unavailable(
            crate::durability::DurabilityError::RootUnavailable
        )
    )
}

/// The caller must own the actual node proof and independently resolve the
/// current admitted session. Re-admission after setup loads normal full vitals.
/// A fresh disposable Character is required; existing builds are never reset.
#[cfg(test)]
pub(super) async fn prepare_free_sorcerer(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    context: ProgressionRevisionContext<String>,
    xp_occurrence: [u8; 16],
    build_occurrence: [u8; 16],
) -> TestResult {
    prepare_free_character(
        root,
        authority,
        node,
        current,
        QualificationBuildSpec {
            vocation: "sorcerer",
            level: 8,
            context,
            xp_occurrence,
            build_occurrence,
        },
    )
    .await
}

/// One source-derived, free-account qualification build. Base vocations only;
/// promotions and Premium need their own accepted owners.
pub(super) struct QualificationBuildSpec<'a> {
    pub vocation: &'a str,
    pub level: u32,
    pub context: ProgressionRevisionContext<String>,
    pub xp_occurrence: [u8; 16],
    pub build_occurrence: [u8; 16],
}

#[cfg(test)]
const QUALIFICATION_LEVELS: usize = 1000;
const PLAYABLE_VOCATIONS: [&str; 5] = ["sorcerer", "druid", "paladin", "knight", "monk"];

/// Canary Player::getExpForLevel polynomial, a finite qualification policy.
/// Bounded signed arithmetic avoids underflow in the early-level polynomial.
#[cfg(test)]
fn source_experience(level: u32) -> TestResult<i64> {
    if !(1..=1001).contains(&level) {
        return Err("qualification level outside source fixture".into());
    }
    let n = i64::from(level);
    Ok(100 * (n - 1) * (n * n - 5 * n + 12) / 6)
}

#[cfg(test)]
fn fixture_policy(
    context: ProgressionRevisionContext<String>,
) -> TestResult<FiniteProgressionPolicy<String, QUALIFICATION_LEVELS>> {
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; QUALIFICATION_LEVELS];
    for (slot, level) in thresholds.iter_mut().zip(1..=1000) {
        *slot = LevelThreshold {
            level,
            minimum_experience: ExactI64::new(source_experience(level)?),
        };
    }
    Ok(FiniteProgressionPolicy {
        declared_difference_revision: context.declaration.clone(),
        context,
        policy_revision: "spell-qualification-source-levels-1-1000-v1".into(),
        reward_revision: "spell-qualification-xp-v1".into(),
        death_policy_revision: "spell-qualification-death-v1".into(),
        thresholds,
        terminal_exclusive_experience: ExactI64::new(source_experience(1001)?),
        death_loss_numerator: 1,
        death_loss_denominator: 1,
        death_loss_rounding: RoundingMode::Floor,
    })
}

#[cfg(test)]
pub(super) async fn prepare_free_character(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    spec: QualificationBuildSpec<'_>,
) -> TestResult {
    let policy = fixture_policy(spec.context.clone())?;
    prepare_character_with_policy(root, authority, node, current, spec, policy).await
}

/// Use the served policy so later kill awards and death writes remain compatible.
#[cfg(test)]
pub(super) async fn prepare_character_with_policy<const N: usize>(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    spec: QualificationBuildSpec<'_>,
    policy: FiniteProgressionPolicy<String, N>,
) -> TestResult {
    prepare_character_with_policy_until(
        root,
        authority,
        node,
        current,
        spec,
        policy,
        tokio::time::Instant::now() + std::time::Duration::from_secs(15),
    )
    .await
}

pub(super) async fn prepare_character_with_policy_until<const N: usize>(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    spec: QualificationBuildSpec<'_>,
    policy: FiniteProgressionPolicy<String, N>,
    deadline: tokio::time::Instant,
) -> TestResult {
    if !PLAYABLE_VOCATIONS.contains(&spec.vocation) || !(1..=1000).contains(&spec.level) {
        return Err("qualification requires a base vocation and source level 1..1000".into());
    }
    let xp_occurrence =
        ExperienceRewardOccurrence::from_bytes(spec.xp_occurrence).map_err(debug)?;
    let build_occurrence = BuildOccurrence::from_bytes(spec.build_occurrence).map_err(debug)?;
    let QualificationBuildSpec {
        vocation,
        level,
        context,
        xp_occurrence: _,
        build_occurrence: _,
    } = spec;
    let character =
        CharacterId::from_bytes(*current.current_character_lease().character_id().as_bytes())
            .map_err(debug)?;
    // CHAR-REV-SEQ-1: the fixture's revision writes run through one slot of this Character.
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(character).await;
    let before = retry_holder(Some(root), deadline, progression_busy, async || {
        root.read_character_build_state(authority, character).await
    })
    .await
    .map_err(debug)?;
    let progression = retry_holder(Some(root), deadline, progression_busy, async || {
        root.read_character_progression(authority, character).await
    })
    .await
    .map_err(debug)?;
    if before.vocation() != NO_VOCATION {
        return Err("spell fixture vocation is set".into());
    }
    if before != DurableBuildState::default() {
        return Err("spell fixture build is not initial".into());
    }
    if let Some(state) = &progression {
        if state.level != 1
            || !policy.thresholds.first().is_some_and(|first| {
                first.level == 1 && state.total_experience == first.minimum_experience
            })
        {
            return Err("spell fixture progression level or XP is not initial".into());
        }
        if state.context != policy.context {
            return Err("spell fixture progression context mismatch".into());
        }
        if state.policy_revision != policy.policy_revision {
            return Err("spell fixture progression policy revision mismatch".into());
        }
        if state.reward_revision != policy.reward_revision {
            return Err("spell fixture progression reward revision mismatch".into());
        }
        // Admission/offline settlement and logout advance revisions even with an
        // unchanged build. Retained XP/death receipts distinguish earned history
        // from those checkpoints, including XP that later returned to zero.
        if state.character_revision.get() != 1
            && retry_holder(Some(root), deadline, progression_busy, async || {
                root.has_character_experience_or_death_receipts(authority, character)
                    .await
            })
            .await
            .map_err(debug)?
        {
            return Err(
                "spell fixture noninitial progression revision has XP or death history".into(),
            );
        }
    }
    if policy.context != context {
        return Err("qualification policy context mismatch".into());
    }
    let experience = policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == level)
        .ok_or("qualification level absent from the served progression policy")?
        .minimum_experience;
    let mut fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: retry_holder(
            Some(root),
            deadline,
            authority_busy,
            async || root.read_current_character(authority, character).await,
        )
        .await
        .map_err(debug)?
        .revision,
    };
    if progression
        .as_ref()
        .is_some_and(|state| state.character_revision != fence.expected_character_revision)
    {
        return Err("spell fixture progression revision differs from current root revision".into());
    }
    if progression.is_none() {
        retry_holder(Some(root), deadline, progression_busy, async || {
            root.initialize_character_progression(
                authority,
                node,
                fence,
                ProgressionInitializationRequest {
                    context: context.clone(),
                    policy_revision: policy.policy_revision.clone(),
                    reward_revision: policy.reward_revision.clone(),
                    policy: policy.clone(),
                },
            )
            .await
        })
        .await
        .map_err(debug)?;
    }
    if level > 1 {
        retry_holder(Some(root), deadline, progression_busy, async || {
            slot.commit_experience(
                root,
                authority,
                node,
                fence,
                ExperienceAwardRequest {
                    occurrence: xp_occurrence,
                    amount: experience,
                    context: context.clone(),
                    policy_revision: policy.policy_revision.clone(),
                    reward_revision: policy.reward_revision.clone(),
                    policy: policy.clone(),
                },
                None,
            )
            .await
        })
        .await
        .map_err(debug)?;
    }
    fence.expected_character_revision =
        retry_holder(Some(root), deadline, authority_busy, async || {
            root.read_current_character(authority, character).await
        })
        .await
        .map_err(debug)?
        .revision;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../tools/content-schema/native-gameplay/build-training.json"),
        &context.content,
    )
    .map_err(debug)?;
    let after = convert_vocation(&formula, &before, vocation).map_err(debug)?;
    retry_holder(Some(root), deadline, progression_busy, async || {
        slot.commit_build(
            root,
            authority,
            node,
            fence,
            BuildChangeRequest {
                occurrence: build_occurrence,
                cause: BuildCause::VocationChoice,
                before: before.clone(),
                after: after.clone(),
                pruned_stance: None,
            },
            &formula,
            None,
        )
        .await
    })
    .await
    .map_err(debug)?;
    // Verify the requested owned state with typed errors; the cast-facts loader
    // erases holder-busy into Unavailable, which cannot safely be retried.
    let verification_revision = retry_holder(Some(root), deadline, authority_busy, async || {
        root.read_current_character(authority, character).await
    })
    .await
    .map_err(debug)?
    .revision;
    let final_build = retry_holder(Some(root), deadline, progression_busy, async || {
        root.read_character_build_state(authority, character).await
    })
    .await
    .map_err(debug)?;
    let final_progression = retry_holder(Some(root), deadline, progression_busy, async || {
        root.read_character_progression(authority, character).await
    })
    .await
    .map_err(debug)?;
    let verified_revision = retry_holder(Some(root), deadline, authority_busy, async || {
        root.read_current_character(authority, character).await
    })
    .await
    .map_err(debug)?
    .revision;
    if verification_revision != verified_revision
        || final_build != after
        || !final_progression.is_some_and(|state| {
            state.character_id == character
                && state.character_revision <= verified_revision
                && state.level == level
                && state.total_experience == experience
        })
    {
        return Err("fenced spell fixture did not produce requested source build".into());
    }
    Ok(())
}

/// Advance an already classed disposable Character using its existing bound XP
/// policy. This is an award, not a replacement of the Character or actor facts.
#[cfg(test)]
pub(super) async fn advance_source_level(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    level: u32,
    occurrence: [u8; 16],
) -> TestResult {
    let occurrence = ExperienceRewardOccurrence::from_bytes(occurrence).map_err(debug)?;
    let character =
        CharacterId::from_bytes(*current.current_character_lease().character_id().as_bytes())
            .map_err(debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(character).await;
    let before = root
        .read_character_progression(authority, character)
        .await
        .map_err(debug)?
        .ok_or("classed fixture has no progression")?;
    let target = source_experience(level)?;
    if level > 1000 || level <= before.level || target <= before.total_experience.get() {
        return Err("qualification level award must advance within existing source policy".into());
    }
    let policy = fixture_policy(before.context.clone())?;
    if before.policy_revision != policy.policy_revision
        || before.reward_revision != policy.reward_revision
    {
        return Err("qualification level award cannot replace another progression policy".into());
    }
    let fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: root
            .read_current_character(authority, character)
            .await
            .map_err(debug)?
            .revision,
    };
    slot.commit_experience(
        root,
        authority,
        node,
        fence,
        ExperienceAwardRequest {
            occurrence,
            amount: ExactI64::new(target - before.total_experience.get()),
            context: before.context,
            policy_revision: policy.policy_revision.clone(),
            reward_revision: policy.reward_revision.clone(),
            policy,
        },
        None,
    )
    .await
    .map_err(debug)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn qualification_holder_busy_retries_then_succeeds() {
        use crate::durability::DurabilityError;
        let mut attempts = 0;
        let result = retry_holder(
            None,
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            |error| matches!(error, DurabilityError::RootUnavailable),
            async || {
                attempts += 1;
                if attempts == 1 {
                    Err(DurabilityError::RootUnavailable)
                } else {
                    Ok(42)
                }
            },
        )
        .await;
        assert!(matches!(result, Ok(42)), "{result:?}");
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn qualification_holder_busy_stops_at_deadline() {
        use crate::durability::DurabilityError;
        let mut attempts = 0;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(20);
        let result: Result<(), _> = retry_holder(
            None,
            deadline,
            |error| matches!(error, DurabilityError::RootUnavailable),
            async || {
                attempts += 1;
                Err(DurabilityError::RootUnavailable)
            },
        )
        .await;
        assert!(matches!(result, Err(DurabilityError::RootUnavailable)));
        assert!(tokio::time::Instant::now() >= deadline);
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn qualification_holder_nonbusy_error_is_final() {
        use crate::durability::{
            DurabilityError, character_progression::CharacterProgressionError,
        };
        for unknown_commit in [false, true] {
            let mut attempts = 0;
            let result: Result<(), _> = retry_holder(
                None,
                tokio::time::Instant::now() + std::time::Duration::from_secs(1),
                progression_busy,
                async || {
                    attempts += 1;
                    if unknown_commit {
                        Err(CharacterProgressionError::Unavailable(
                            DurabilityError::CommitOutcomeUnknown,
                        ))
                    } else {
                        Err(CharacterProgressionError::AuthorityRejected)
                    }
                },
            )
            .await;
            if unknown_commit {
                assert!(matches!(
                    result,
                    Err(CharacterProgressionError::Unavailable(
                        DurabilityError::CommitOutcomeUnknown
                    ))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(CharacterProgressionError::AuthorityRejected)
                ));
            }
            assert_eq!(attempts, 1);
        }
    }

    #[test]
    fn source_fixture_matches_known_level_thresholds() -> TestResult {
        for (level, xp) in [(1, 0), (2, 100), (8, 4200), (20, 98800), (100, 15694800)] {
            assert_eq!(source_experience(level)?, xp);
        }
        assert!(source_experience(0).is_err());
        assert!(source_experience(1002).is_err());
        Ok(())
    }

    #[test]
    fn finite_policy_covers_every_base_vocation_level_without_extrapolation() -> TestResult {
        let context = ProgressionRevisionContext {
            profile: "fixture-profile".into(),
            ruleset: "fixture-ruleset".into(),
            content: "fixture-content".into(),
            simulation: "fixture-simulation".into(),
            evidence: "fixture-evidence".into(),
            declaration: "fixture-declaration".into(),
        };
        let policy = fixture_policy(context.clone())?;
        crate::domain::progression::validate_policy(&policy).map_err(debug)?;
        assert_eq!(policy.context, context);
        assert_eq!(policy.thresholds[999].level, 1000);
        for vocation in PLAYABLE_VOCATIONS {
            assert!(crate::spell::Vocation::from_key(vocation).is_some());
        }
        assert!(!PLAYABLE_VOCATIONS.contains(&"master_sorcerer"));
        Ok(())
    }
}
