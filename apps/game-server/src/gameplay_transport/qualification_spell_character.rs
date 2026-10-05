//! Disposable qualification Character setup through the real fenced writers.
//! Compile only below the test-only qualification module. This does not install
//! actor vitals, issue entitlements, or bypass the ordinary client admission.

use crate::domain::CharacterId;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, NO_VOCATION, convert_vocation,
};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
    ProgressionInitializationRequest,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionAuthoritySnapshot};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

/// The caller must own the actual node proof and independently resolve the
/// current admitted session. Re-admission after setup loads normal full vitals.
/// A fresh disposable Character is required; existing builds are never reset.
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

const QUALIFICATION_LEVELS: usize = 1000;
const PLAYABLE_VOCATIONS: [&str; 5] = ["sorcerer", "druid", "paladin", "knight", "monk"];

/// Canary Player::getExpForLevel polynomial, a finite qualification policy.
/// Bounded signed arithmetic avoids underflow in the early-level polynomial.
fn source_experience(level: u32) -> TestResult<i64> {
    if !(1..=1001).contains(&level) {
        return Err("qualification level outside source fixture".into());
    }
    let n = i64::from(level);
    Ok(100 * (n - 1) * (n * n - 5 * n + 12) / 6)
}

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

pub(super) async fn prepare_free_character(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    spec: QualificationBuildSpec<'_>,
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
    let before = root
        .read_character_build_state(authority, character)
        .await
        .map_err(debug)?;
    if before.vocation() != NO_VOCATION
        || root
            .read_character_progression(authority, character)
            .await
            .map_err(debug)?
            .is_some()
    {
        return Err("spell fixture requires a fresh disposable Character".into());
    }
    let policy = fixture_policy(context.clone())?;
    let mut fence = CurrentCharacterGameplayFence {
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
    .map_err(debug)?;
    if level > 1 {
        slot.commit_experience(
            root,
            authority,
            node,
            fence,
            ExperienceAwardRequest {
                occurrence: xp_occurrence,
                amount: ExactI64::new(source_experience(level)?),
                context: context.clone(),
                policy_revision: policy.policy_revision.clone(),
                reward_revision: policy.reward_revision.clone(),
                policy,
            },
            None,
        )
        .await
        .map_err(debug)?;
    }
    fence.expected_character_revision = root
        .read_current_character(authority, character)
        .await
        .map_err(debug)?
        .revision;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
        &context.content,
    )
    .map_err(debug)?;
    let after = convert_vocation(&formula, &before, vocation).map_err(debug)?;
    slot.commit_build(
        root,
        authority,
        node,
        fence,
        BuildChangeRequest {
            occurrence: build_occurrence,
            cause: BuildCause::VocationChoice,
            before,
            after,
            pruned_stance: None,
        },
        &formula,
        None,
    )
    .await
    .map_err(debug)?;
    let facts = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        root, authority, character,
    )
    .await;
    if !matches!(facts, crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } if facts.level == level && Some(facts.vocation) == crate::spell::Vocation::from_key(vocation))
    {
        return Err("fenced spell fixture did not produce requested source build".into());
    }
    Ok(())
}

/// Advance an already classed disposable Character using its existing bound XP
/// policy. This is an award, not a replacement of the Character or actor facts.
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
