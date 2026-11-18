//! Crystal 201189f8 data-global/scripts/quests/blood_brothers_quest/boreth_death.lua:3-20.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const NAME: &str = "boreth-death-cleanup";
const BORETH: &str = "oteryn:creature.boreth";
const ADD: &str = "oteryn:creature.plaguethrower";

#[test]
fn boreth_projected_death_removes_only_its_adds_inside_the_source_rectangle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 32944,
                y: 31479,
                floor: -1,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let (boreth, inside, outside, vampire) = {
        let mut runtime = owner.runtime.lock().await;
        let mut place = |definition: &str, x, y| -> TestResult<_> {
            let policy = runtime.companion_policy(definition).map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(
                    definition,
                    policy.maximum_health,
                    MovementLocalPosition { x, y, floor: 1 },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, definition)
                .map_err(debug)?;
            Ok(actor)
        };
        (
            place(BORETH, 32946, 31479)?,
            place(ADD, 32944, 31479)?,
            place(ADD, 32945, 31479)?,
            place("oteryn:creature.vampire", 32944, 31480)?,
        )
    };
    super::engenc2_cults::kill(owner, admitted, boreth, BORETH, 9100).await?;
    let runtime = owner.runtime.lock().await;
    assert!(!runtime.contains_live_creature(inside));
    assert!(runtime.contains_live_creature(outside));
    assert!(runtime.contains_live_creature(vampire));
    Ok(())
}
