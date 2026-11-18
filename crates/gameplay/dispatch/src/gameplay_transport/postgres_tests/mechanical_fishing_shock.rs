//! Crystal 201189f8 mechanical_fishing.lua:106-109,218-237.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ConditionValues;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

#[test]
fn mechanical_fishing_miss_can_shock_and_grant_temporary_outfit_without_burning_nail() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("mechanical-fishing-shock", || {
        super::standard_fishing::gathering_scenario(9306, 4597, Some(4597), None, false, true, true)
    })
}

pub(super) fn is_shock_draw(digest: [u8; 32], occurrence: [u8; 16]) -> bool {
    deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(digest),
        DecisionOccurrenceId::from_bytes(occurrence),
        "fishing.sewer.shock",
        0,
    )
    .is_ok_and(|draw| (1..=10).contains(&crate::spell::uniform_draw(draw, 1, 100)))
}

pub(super) async fn health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<u64> {
    let (_, vitals) = owner
        .observe_vitals(
            admitted.runtime_actor.ok_or("fishing actor")?,
            admitted.game_session_id,
        )
        .await
        .ok_or("fishing vitals")?;
    Ok(u64::try_from(vitals.health)?)
}

pub(super) async fn assert_shock(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: TilePos,
    before: u64,
) -> TestResult {
    assert_eq!(health(owner, admitted).await?, before - 35);
    let map = owner.bundle_map().ok_or("fishing map")?;
    let effects = map.facts.movement_effects_since(0);
    assert_eq!(
        effects
            .iter()
            .map(|(_, at, effect)| (*at, *effect))
            .collect::<Vec<_>>(),
        [(target, 2), (target, 12)],
    );
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("fishing actor")?,
            admitted.game_session_id,
        )
        .ok_or("fishing spell state")?;
    let outfit = state
        .owned_conditions()
        .instances()
        .iter()
        .find(|condition| {
            matches!(
                condition.definition().values(),
                ConditionValues::Outfit {
                    look_type: 33,
                    duration_ms: 4_000
                }
            )
        })
        .ok_or("temporary skeleton outfit")?;
    assert_eq!(
        outfit.remaining_duration_ms(outfit.started_at()),
        Some(4_000)
    );
    assert_eq!(
        outfit.remaining_duration_ms(outfit.started_at() + 4_000_000),
        Some(0)
    );
    Ok(())
}
