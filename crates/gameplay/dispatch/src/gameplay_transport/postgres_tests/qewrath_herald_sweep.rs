//! Crystal 201189f8 movements_herald_red_floors.lua:31–39,59–83.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use serde_json::json;

pub(super) const NAME: &str = "qewrath-herald-sweep";
const HERALD: &str = "oteryn:creature.herald_of_fire";
const SUMMON: &str = "oteryn:creature.infernoid_blob";
const LAST_PLATE: Position = Position { x: 32496, y: 32663, floor: -15 };
const FIRST_PLATE: Position = Position { y: 32649, ..LAST_PLATE };
const SECOND_PLATE: Position = Position { x: 32503, y: 32656, ..LAST_PLATE };

#[test]
fn herald_last_plate_resets_the_room_monsters_and_preserves_the_herald() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, LAST_PLATE, false)
    })
}

pub(super) fn fixture() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/targuna/herald-room-sweep",
        "quest":"oteryn:quest.targuna_quest","trigger":"step_in",
        "selector":{"position":LAST_PLATE,"appearance":53096},
        "reversible":false,"offerings":[],"return_on_failure":false,"storage":[],
        "world_requirements":[
            {"position":FIRST_PLATE,"appearance":53095},
            {"position":SECOND_PLATE,"appearance":53095}
        ],
        "passages":[
            {"position":FIRST_PLATE,"appearance":53095,"replacement":53096},
            {"position":SECOND_PLATE,"appearance":53095,"replacement":53096}
        ],
        "effects":{"sweep_creatures":{
            "minimum":{"x":32484,"y":32644,"floor":-15},
            "maximum":{"x":32508,"y":32668,"floor":-15},
            "except":HERALD
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/targuna/movements_herald_red_floors.lua:31-83"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    owner.states.quest_callbacks.fixture.set(vec![fixture()?]).map_err(|_| "arena fixture")?;
    let targets = {
        let mut runtime = owner.runtime.lock().await;
        let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let mut spawn = |creature: &str, position| {
            let hp = runtime.companion_policy(creature)?.maximum_health;
            runtime.admit_encounter_actor(creature, hp, super::quest_world_callbacks::local(position))
        };
        [
            spawn(HERALD, Position { y: 32656, ..LAST_PLATE }).map_err(debug)?,
            spawn(SUMMON, Position { y: 32655, ..LAST_PLATE }).map_err(debug)?,
            spawn(SUMMON, Position { x: 32509, y: 32656, ..LAST_PLATE }).map_err(debug)?,
        ]
    };
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner, admitted, command, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    ).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    let runtime = owner.runtime.lock().await;
    assert!(runtime.contains_live_creature(targets[0]));
    assert!(!runtime.contains_live_creature(targets[1]));
    assert!(runtime.contains_live_creature(targets[2]));
    Ok(())
}
