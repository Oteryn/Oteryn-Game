//! Seed real low-level progression through the character authority.
use super::*;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_transfer::CurrentCharacterItemFence;

pub(in super::super) async fn initialize_level_one(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    item: CurrentCharacterItemFence,
) -> TestResult {
    use crate::domain::progression::{
        FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
    };
    use crate::durability::character_progression::{
        ProgressionInitializationOutcome, ProgressionInitializationRequest,
    };
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    let fence = CurrentCharacterGameplayFence {
        character_id: item.character_id,
        game_session_id: item.game_session_id,
        connection_generation: item.connection_generation,
        character_lease_generation: item.character_lease_generation,
        runtime_scope: item.runtime_scope,
        scope_ownership_generation: item.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
    };
    let initialized = h
        .root
        .initialize_character_progression(
            authority,
            &h.node,
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
    assert!(matches!(
        initialized,
        ProgressionInitializationOutcome::Initialized(_)
    ));
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(level, 1);
    Ok(())
}
