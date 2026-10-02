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
    let character =
        CharacterId::from_bytes(*current.current_character_lease().character_id().as_bytes())
            .map_err(debug)?;
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
    // Source reference XP thresholds through level 8. This finite fixture
    // deliberately makes no claim about later progression or production rewards.
    let thresholds = [
        (1, 0),
        (2, 100),
        (3, 200),
        (4, 400),
        (5, 800),
        (6, 1500),
        (7, 2600),
        (8, 4200),
    ];
    let policy = FiniteProgressionPolicy {
        context: context.clone(),
        policy_revision: "spell-qualification-levels-1-8-v1".into(),
        reward_revision: "spell-qualification-xp-v1".into(),
        death_policy_revision: "spell-qualification-death-v1".into(),
        declared_difference_revision: context.declaration.clone(),
        thresholds: thresholds.map(|(level, xp)| LevelThreshold {
            level,
            minimum_experience: ExactI64::new(xp),
        }),
        terminal_exclusive_experience: ExactI64::new(6400),
        death_loss_numerator: 1,
        death_loss_denominator: 1,
        death_loss_rounding: RoundingMode::Floor,
    };
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
    root.commit_character_experience(
        authority,
        node,
        fence,
        ExperienceAwardRequest {
            occurrence: ExperienceRewardOccurrence::from_bytes(xp_occurrence).map_err(debug)?,
            amount: ExactI64::new(4200),
            context: context.clone(),
            policy_revision: policy.policy_revision.clone(),
            reward_revision: policy.reward_revision.clone(),
            policy,
        },
    )
    .await
    .map_err(debug)?;
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
    let after = convert_vocation(&formula, &before, "sorcerer").map_err(debug)?;
    root.commit_character_build(
        authority,
        node,
        fence,
        BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(build_occurrence).map_err(debug)?,
            cause: BuildCause::VocationChoice,
            before,
            after,
            pruned_stance: None,
        },
        &formula,
    )
    .await
    .map_err(debug)?;
    let facts = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        root, authority, character,
    )
    .await;
    if !matches!(facts, crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } if facts.level == 8 && facts.vocation == crate::spell::Vocation::Sorcerer)
    {
        return Err("fenced spell fixture did not produce level-8 sorcerer facts".into());
    }
    Ok(())
}
