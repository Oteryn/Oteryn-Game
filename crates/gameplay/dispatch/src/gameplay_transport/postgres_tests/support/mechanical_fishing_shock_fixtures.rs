//! Crystal 201189f8 mechanical_fishing.lua:106-109,218-237.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ConditionValues;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::source_bindings;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

pub(super) fn is_shock_draw(digest: [u8; 32], occurrence: [u8; 16]) -> bool {
    deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(digest),
        DecisionOccurrenceId::from_bytes(occurrence),
        "fishing.sewer.shock",
        0,
    )
    .is_ok_and(|draw| (1..=10).contains(&crate::spell::uniform_draw(draw, 1, 100)))
}

pub(super) fn fixture() -> TestResult<source_bindings::FishingFailure> {
    let document = serde_json::json!({
        "schema":1, "callbacks":[],
        "fishing_failure":{
            "water_effect":2, "shock_effect":12, "shock_chance_percent":10,
            "damage":35, "look_type":33, "duration_ms":4000,
            "speech":"Yikes! That DID hurt!",
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/mechanical_fishing.lua:106-109,212-237,252-273"]
        }
    });
    Ok(source_bindings::fixture(&document.to_string())?
        .fishing_failure
        .ok_or("mechanical fishing binding")?)
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
