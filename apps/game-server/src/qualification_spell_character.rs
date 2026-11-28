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

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn debug(error: impl std::fmt::Debug) -> String {
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
            exact_level: false,
            context,
            xp_occurrence,
            build_occurrence,
        },
    )
    .await
}

/// One source-derived, free-account qualification build. Base vocations only;
/// promotions and Premium need their own accepted owners.
pub(crate) struct QualificationBuildSpec<'a> {
    pub vocation: &'a str,
    pub level: u32,
    pub exact_level: bool,
    pub context: ProgressionRevisionContext<String>,
    pub xp_occurrence: [u8; 16],
    pub build_occurrence: [u8; 16],
}

#[cfg(test)]
const QUALIFICATION_LEVELS: usize = 1000;
pub(super) const PLAYABLE_VOCATIONS: [&str; 5] = ["sorcerer", "druid", "paladin", "knight", "monk"];

/// Canary Player::getExpForLevel polynomial, a finite qualification policy.
/// Bounded signed arithmetic avoids underflow in the early-level polynomial.
#[cfg(test)]
pub(super) fn source_experience(level: u32) -> TestResult<i64> {
    if !(1..=1001).contains(&level) {
        return Err("qualification level outside source fixture".into());
    }
    let n = i64::from(level);
    Ok(100 * (n - 1) * (n * n - 5 * n + 12) / 6)
}

#[cfg(test)]
pub(super) fn fixture_policy(
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
pub(crate) async fn prepare_character_with_policy<const N: usize>(
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
        exact_level,
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
    let experience = policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == level)
        .ok_or("qualification level absent from the served progression policy")?
        .minimum_experience;
    let classed = before.vocation() != NO_VOCATION;
    let requested_vocation_present = classed && before.vocation() == vocation;
    let xp_checkpoint = if level > 1
        && progression
            .as_ref()
            .is_some_and(|state| classed || state.level >= level)
    {
        retry_holder(Some(root), deadline, progression_busy, async || {
            root.reconcile_character_experience(authority, xp_occurrence)
                .await
        })
        .await
        .map_err(debug)?
        .filter(|receipt| {
            receipt.character_id == character
                && receipt.level_before == 1
                && receipt.level_after == level
                && receipt.experience_after == experience
                && receipt.experience_awarded == experience
        })
    } else {
        None
    };
    let checkpoint = xp_checkpoint.is_some();
    let fixture_build = if let Some(xp) = xp_checkpoint
        .as_ref()
        .filter(|_| requested_vocation_present)
    {
        retry_holder(Some(root), deadline, progression_busy, async || {
            root.reconcile_character_build(authority, build_occurrence)
                .await
        })
        .await
        .map_err(debug)?
        .is_some_and(|receipt| {
            receipt.character_id == character
                && receipt.after.vocation() == vocation
                && matches!(
                    receipt.cause,
                    BuildCause::VocationChoice | BuildCause::VocationTrial
                )
                && receipt.original_character_revision >= xp.committed_character_revision
        })
    } else {
        false
    };
    if classed
        && !fixture_build
        && !retry_holder(Some(root), deadline, progression_busy, async || {
            root.has_unchanged_bootstrap_vocation(authority, character, before.vocation())
                .await
        })
        .await
        .map_err(debug)?
    {
        return Err("spell fixture vocation is set".into());
    }
    let resume_items = checkpoint && requested_vocation_present;
    if !resume_items {
        let birth_seed =
            DurableBuildState::new(before.vocation(), (0, 0), [(10, 0); 7]).map_err(debug)?;
        if before != birth_seed || (classed && !PLAYABLE_VOCATIONS.contains(&before.vocation())) {
            return Err("spell fixture build is not initial".into());
        }
    }
    if let Some(state) = &progression {
        if !checkpoint
            && (state.level != 1
                || !policy.thresholds.first().is_some_and(|first| {
                    first.level == 1 && state.total_experience == first.minimum_experience
                }))
        {
            return Err("spell fixture progression level or XP is not initial".into());
        }
        if checkpoint
            && (exact_level || !resume_items)
            && (state.level != level || state.total_experience != experience)
        {
            return Err("qualification fixture XP checkpoint has changed".into());
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
        if !checkpoint
            && state.character_revision.get() != 1
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
    if resume_items {
        // The exact XP checkpoint and proven vocation permit resuming the item stage.
        // Preserve the current build and later play.
        return Ok(());
    }
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
    if level > 1 && !checkpoint {
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
    if before != after {
        retry_holder(Some(root), deadline, progression_busy, async || {
            slot.commit_build(
                root,
                authority,
                node,
                fence,
                BuildChangeRequest {
                    occurrence: build_occurrence,
                    cause: if classed {
                        BuildCause::VocationTrial
                    } else {
                        BuildCause::VocationChoice
                    },
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
    }
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
