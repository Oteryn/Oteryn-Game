//! Crystal 201189f8 kilmaresh/movements_firetp.lua:2-39,43-104.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks_fixtures::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

const QUEST: &str = "oteryn:quest.fixture_revenge_of_the_ogres";
const TRACK: &str = "oteryn:quest-progress/fixture-ogres/mission";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/kilmaresh/movements_firetp.lua:2-39,43-104";
const FIRES: [Position; 3] = [at(33830, 31628), at(33832, 31628), at(33834, 31628)];
const SUCCESS: Position = at(33826, 31620);
const FAILURE: Position = at(33822, 31645);
const ACTIONS: [u16; 4] = [57539, 57540, 57541, 57542];
// The fixture assigns the four script AIDs to the pressure plates on the native
// puzzle floor. The observation cells and both destinations are from the script.
const PADS: [Position; 4] = [
    at(33829, 31630),
    at(33831, 31630),
    at(33833, 31630),
    at(33835, 31630),
];
const LIT: u16 = 171;
const UNLIT: u16 = 2114;
const TELEPORT_EFFECT: u16 = 11;

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -9 }
}

fn tile(position: Position) -> TilePos {
    TilePos {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}

#[test]
fn ogre_step_routes_check_current_fires_and_teleport_to_either_occupied_landing() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(exercise)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let stages: Vec<_> = ACTIONS
        .into_iter()
        .enumerate()
        .map(|(lit, action)| {
            let world_requirements: Vec<_> = FIRES
                .into_iter()
                .enumerate()
                .map(|(index, position)| {
                    let appearance = if index < lit { LIT } else { UNLIT };
                    json!({"position": position, "appearance": appearance})
                })
                .collect();
            json!({
                "key": format!("oteryn:quest-stage/fixture-ogres/fire-{action}"),
                "completes": false, "trigger": {"kind": "step_in", "callback": {
                    "key": format!("oteryn:quest-callback/fixture-ogres/fire-{action}"),
                    "quest": QUEST, "trigger": "step_in", "selector": {"action": action},
                    "teleport": SUCCESS, "failure_teleport": FAILURE, "return_on_failure": true,
                    "world_requirements": world_requirements,
                    "effects": {"scripted_teleport": true, "teleport_effect": TELEPORT_EFFECT},
                    "reversible": false, "passages": [], "offerings": [], "sources": [SOURCE]
                }}
            })
        })
        .collect();
    let document = json!({"schema": 1, "quests": [{
        "quest": QUEST, "sources": [SOURCE],
        "recipe_sha256": Sha256::digest(SOURCE).iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "tracks": [{"key": TRACK, "initial": -1, "minimum": -1, "maximum": 1}],
        "stages": stages
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&document)?)?.attach(&base)?)
}

async fn exercise(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "ogre_fire_routes", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active generation")?;
    let objects: Vec<_> = PADS
        .into_iter()
        .zip(ACTIONS)
        .map(|(p, action)| (p.x, p.y, p.floor, action, 1949))
        .chain(
            FIRES
                .into_iter()
                .zip([LIT, UNLIT, UNLIT])
                .map(|(p, appearance)| (p.x, p.y, p.floor, 0, appearance)),
        )
        .chain([SUCCESS, FAILURE].map(|p| (p.x, p.y, p.floor, 0, 422)))
        .collect();
    let first = PADS[0];
    let world = super::quest_callbacks_fixtures::source_world_step_actions(
        (first.x, first.y, first.floor),
        &objects,
    )?;
    super::arena_match_fixtures::fixture::runtime_at(
        &mut deps,
        generation,
        &pin,
        &world,
        local(at(first.x - 1, first.y)),
    )?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let landings = {
        let mut runtime = owner.runtime.lock().await;
        let creature = "oteryn:creature.fire_devil";
        let health = runtime
            .companion_policy(creature)
            .map_err(debug)?
            .maximum_health;
        runtime
            .commit_quest_creature_spawn(&[SUCCESS, FAILURE].map(|p| (creature, health, local(p))))
            .map_err(debug)?
    };
    let mut expected_cues = Vec::new();
    for (index, pad) in PADS.into_iter().enumerate() {
        for solved in [false, true] {
            if solved {
                let fire = FIRES[index.saturating_sub(1)];
                let key = crate::map::view::placement_key(tile(fire), 1).ok_or("fire placement")?;
                let appearance = if index == 0 { UNLIT } else { LIT };
                let map = owner.bundle_map().ok_or("map")?;
                assert!(
                    map.facts.transform_movement_item(key, appearance),
                    "publish the live fire appearance"
                );
            }
            super::l6_native_signs_fixtures::relocate(&owner, actor, session, at(pad.x - 1, pad.y)).await?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(2))
                .ok_or("step clock")?;
            let command = u64::try_from(index * 2)? + u64::from(solved) + 1;
            let reply = wire(
                &owner,
                &mut admitted,
                command,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                encode_step_intent(StepDirection::East),
            )
            .await?;
            assert_eq!(
                decode_step_result(&reply).map_err(debug)?,
                StepDisposition::Moved
            );
            let expected = if solved { SUCCESS } else { FAILURE };
            {
                let runtime = owner.runtime.lock().await;
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    local(expected)
                );
                for (occupant, landing) in landings.iter().zip([SUCCESS, FAILURE]) {
                    assert_eq!(
                        runtime
                            .read_actor_position(*occupant)
                            .map_err(debug)?
                            .position(),
                        local(landing)
                    );
                }
            }
            expected_cues.push(tile(expected));
            let map = owner.bundle_map().ok_or("map")?;
            let cues: Vec<_> = map
                .facts
                .movement_effects_since(0)
                .into_iter()
                .filter_map(|(_, at, effect)| (effect == TELEPORT_EFFECT).then_some(at))
                .collect();
            assert_eq!(cues, expected_cues);
        }
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-callback/fixture-ogres/%'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, 0,
        "the fire puzzle moves the player without changing quest progress"
    );
    Ok(())
}
