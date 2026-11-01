//! Complete the bootstrap-only target through the existing D88 progression writer.
use super::*;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::character_progression::{
    ProgressionInitializationOutcome, ProgressionInitializationRequest,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

pub(super) async fn initialize(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    // historical_character deliberately seeds only the root/bootstrap receipt. The
    // shared Dependencies initializer installs Monk facts, whose real terminal save
    // requires a typed progression row. Use the canonical fenced D88 owner, not an
    // INSERT or a replacement logout path. These are the existing player_trade PG
    // fixture's finite policy/context, matching the bootstrap root's revisions.
    let fence = owner
        .current_monk_fence(admitted.game_session_id)
        .await
        .map_err(|()| "target progression fence unavailable")?
        .ok_or("target unexpectedly terminal before progression initialization")?;
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    let outcome = owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            ProgressionInitializationRequest {
                context: context.clone(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: FiniteProgressionPolicy {
                    context,
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    death_policy_revision: "death-1".into(),
                    declared_difference_revision: "declaration-1".into(),
                    thresholds: [
                        LevelThreshold {
                            level: 1,
                            minimum_experience: ExactI64::new(0),
                        },
                        LevelThreshold {
                            level: 2,
                            minimum_experience: ExactI64::new(100),
                        },
                    ],
                    terminal_exclusive_experience: ExactI64::new(200),
                    death_loss_numerator: 1,
                    death_loss_denominator: 10,
                    death_loss_rounding: RoundingMode::Floor,
                },
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, ProgressionInitializationOutcome::Initialized(_)),
        "bootstrap target must receive its first typed progression through D88");
    Ok(())
}
