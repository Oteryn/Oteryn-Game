//! Crystal 201189f8 data-global/scripts/quests/blood_brothers_quest/boreth_death.lua:3-20.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const NAME: &str = "boreth-death-cleanup";
pub(super) const UNBOUND: &str = "boreth-death-without-cleanup-binding";
const BORETH: &str = "oteryn:creature.boreth";
const ADD: &str = "oteryn:creature.plaguethrower";

#[test]
fn boreth_projected_death_removes_only_its_adds_inside_the_source_rectangle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(NAME)
}

#[test]
fn boreth_projected_death_leaves_adds_alone_without_a_binding() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(UNBOUND)
}

fn run(name: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
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
    enabled: bool,
) -> TestResult {
    let rows = if enabled {
        vec![serde_json::json!({
            "boss":BORETH, "add":ADD,
            "minimum":{"x":32936,"y":31474,"floor":-1},
            "maximum":{"x":32944,"y":31482,"floor":-1},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/boreth_death.lua:3-20"]
        })]
    } else {
        Vec::new()
    };
    let binding = serde_json::json!({"schema":1, "callbacks":[], "death_cleanups":rows});
    owner
        .states
        .spell_states
        .lock()
        .await
        .encounter_phases
        .bind_fixture_death_cleanups(&binding.to_string())?;
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
    assert_eq!(runtime.contains_live_creature(inside), !enabled);
    assert!(runtime.contains_live_creature(outside));
    assert!(runtime.contains_live_creature(vampire));
    Ok(())
}
