//! Crystal 201189f8: the first and third seal callbacks spawn on personal progress.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

const FIRST: &str = "banshee-hidden-flame";
const THIRD: &str = "banshee-demonrage-warlocks";
const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const ENTRY: Position = Position { x: 32210, y: 31839, floor: -15 };
const FIRST_EXIT: Position = Position { x: 32266, y: 31849, floor: -15 };
const GHOST: &str = "oteryn:creature.ghost";
const SKELETON: &str = "oteryn:creature.demon_skeleton";
const WARLOCK: &str = "oteryn:creature.warlock";

#[test]
fn banshee_hidden_flame_spawns_the_ambush_once_and_teleports() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    run(FIRST)
}

#[test]
fn banshee_demonrage_spawns_two_warlocks_only_on_first_step() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    run(THIRD)
}

fn run(name: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, ENTRY, false)
    })
}

pub(super) fn matches(name: &str) -> bool {
    matches!(name, FIRST | THIRD)
}

fn document(name: &str) -> serde_json::Value {
    use serde_json::json;
    let first = name == FIRST;
    let (tracks, monsters, source) = if first {
        (
            vec!["first_seal", "first_seal_door"],
            vec![(GHOST, 32276, 31902, -13), (GHOST, 32274, 31902, -13), (SKELETON, 32276, 31904, -13)],
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-1-first_seal_flame.lua:3-25",
        )
    } else {
        (
            vec!["third_seal_warlocks"],
            vec![(WARLOCK, 32215, 31835, -15), (WARLOCK, 32215, 31840, -15)],
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-3-third_seal_warlock_tile.lua:3-20",
        )
    };
    let storage: Vec<_> = tracks.iter().enumerate().map(|(index, track)| json!({
        "track": format!("oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/{track}"),
        "minimum":-1, "maximum":if index == 0 { 0 } else { 1 }, "set":1
    })).collect();
    let ambush: Vec<_> = monsters.iter().map(|(creature,x,y,floor)| json!({
        "creature":creature,"position":{"x":x,"y":y,"floor":floor}
    })).collect();
    json!({"schema":1,"callbacks":[{
        "key":format!("oteryn:quest-callback/{name}"),"quest":QUEST,
        "trigger":"step_in","selector":{"position":ENTRY,"appearance":100},
        "reversible":false,"passages":[],"teleport":first.then_some(FIRST_EXIT),
        "requires_world_admission":true,"return_on_failure":first,
        "storage":storage,"offerings":[],"effects":{"ambush":ambush},"sources":[source]
    }]})
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    Ok(serde_json::from_value(document(name)["callbacks"][0].clone())?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let callback = callback(name)?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = Vec::new();
    for storage in &callback.storage {
        tracks.push(catalogue.track(&storage.track).cloned().unwrap_or(
            oteryn_gameplay_quests::quest::QuestTrack {
                key: storage.track.clone(), quest: QUEST.into(), initial: -1, min: -1, max: 1,
            },
        ));
    }
    let fixture = oteryn_gameplay_quests::quest::QuestStateCatalogue::new(
        catalogue.content_revision(), tracks, vec![],
    ).map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture, &document(name).to_string(),
    )?));
    owner.states.quest_callbacks.fixture.set(vec![callback.clone()])
        .map_err(|_| "callback fixture already installed")?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = {
        let mut runtime = owner.runtime.lock().await;
        runtime.install_companion_policies(native.policies().clone()).map_err(debug)?;
        runtime.read_actor_position(actor).map_err(debug)?.position()
    };
    let step = encode_step_intent(StepDirection::East);
    wire(owner, admitted, 1, 1, step.clone()).await?;
    let spawned = {
        let mut runtime = owner.runtime.lock().await;
        let landed = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(landed.position(), local(callback.teleport.unwrap_or(ENTRY)));
        let spawned = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(spawned.len(), callback.effects.ambush.len());
        for monster in &callback.effects.ambush {
            assert!(spawned.iter().any(|candidate| runtime.read_actor_position(*candidate)
                .is_ok_and(|position| position.position() == local(monster.position))));
        }
        let map = owner.bundle_map().ok_or("map")?;
        let doors = crate::map::floor::BundleDoors(map.facts);
        let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
            &runtime, owner.movement_cells(), actor, admitted.game_session_id, landed,
            origin, false, &doors, None,
        ).map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
        spawned
    };
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    wire(owner, admitted, 2, 1, step).await?;
    let runtime = owner.runtime.lock().await;
    assert_eq!(runtime.current_live_creature_census().map_err(debug)?, spawned);
    assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(),
        if name == FIRST { origin } else { local(ENTRY) });
    drop(runtime);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
        .bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    Ok(())
}
