//! Crystal 201189f8, dangerous_depth/movements_boss_entrance.lua:10-47.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use serde_json::json;

pub(super) const CASE: &str = "dangerous-depths-warzone-four-portal";
const COUNT_ENTRY: &str = "dangerous-depths-warzone-five-portal";
const DUKE_ENTRY: &str = "dangerous-depths-warzone-six-portal";
pub(super) const VICTORY: &str = "dangerous-depths-warzone-four-defeat-exit";
const COUNT_VICTORY: &str = "dangerous-depths-warzone-five-defeat-exit";
const DUKE_VICTORY: &str = "dangerous-depths-warzone-six-defeat-exit";
const PORTAL: Position = Position {
    x: 33460,
    y: 32267,
    floor: -15,
};
const ARRIVAL: Position = Position {
    x: 33650,
    y: 32310,
    floor: -15,
};
const BOSS: &str = "oteryn:creature.the_baron_from_below";
const QUEST: &str = "oteryn:quest.dangerous_depths_quest";
const TRACK: &str = "oteryn:quest-progress/dangerous-depths/warzone-iv/admitted-at";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/movements_boss_entrance.lua:10-47";
const DEATH_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/creaturescripts_bosses_mission_depths.lua:1-26";
const EXIT: Position = at(33650, 32312);
const ALTERNATE: Position = at(33668, 32301);
const RETURN: Position = at(33462, 32267);
// The mounted encounter lever supplies the same native boss admission owner.
const LEVER: Position = at(33650, 32314);
const TELEPORT_EFFECT: u16 = 11; // Crystal items.xml:45547-45551, CONST_ME_TELEPORT.

struct Entry {
    portal: Position,
    arrival: Position,
    boss: &'static str,
    track: &'static str,
}

fn entry(name: &str) -> Option<Entry> {
    // Crystal movements_boss_entrance.lua:10-35.
    Some(match name {
        CASE => Entry {
            portal: PORTAL,
            arrival: ARRIVAL,
            boss: BOSS,
            track: TRACK,
        },
        COUNT_ENTRY => Entry {
            portal: at(33324, 32109),
            arrival: at(33681, 32338),
            boss: "oteryn:creature.the_count_of_the_core",
            track: "oteryn:quest-progress/dangerous-depths/warzone-v/admitted-at",
        },
        DUKE_ENTRY => Entry {
            portal: at(33275, 32316),
            arrival: at(33717, 32302),
            boss: "oteryn:creature.the_duke_of_the_depths",
            track: "oteryn:quest-progress/dangerous-depths/warzone-vi/admitted-at",
        },
        _ => return None,
    })
}

pub(super) fn is_entry(name: &str) -> bool {
    entry(name).is_some()
}

fn run_entry(name: &'static str) -> TestResult {
    let spec = entry(name).ok_or("Warzone entrance")?;
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, spec.portal, false)
    })
}

struct VictoryExit {
    boss: &'static str,
    boss_position: Position,
    exit: Position,
    alternate: Position,
    returns_to: Position,
    arrival: Position,
    room_min: Position,
    room_max: Position,
    direction: StepDirection,
}

fn victory_exit(name: &str) -> Option<VictoryExit> {
    // Crystal creaturescripts_bosses_mission_depths.lua:1-5 and actions_using_crystals.lua:371-398,437-519.
    Some(match name {
        VICTORY => VictoryExit {
            boss: BOSS,
            boss_position: at(33648, 32303),
            exit: EXIT,
            alternate: ALTERNATE,
            returns_to: RETURN,
            arrival: ARRIVAL,
            room_min: at(33638, 32291),
            room_max: at(33675, 32313),
            direction: StepDirection::South,
        },
        COUNT_VICTORY => VictoryExit {
            boss: "oteryn:creature.the_count_of_the_core",
            boss_position: at(33681, 32334),
            exit: at(33681, 32340),
            alternate: at(33682, 32315),
            returns_to: at(33324, 32111),
            arrival: at(33681, 32338),
            room_min: at(33668, 32310),
            room_max: at(33695, 32343),
            direction: StepDirection::South,
        },
        DUKE_VICTORY => VictoryExit {
            boss: "oteryn:creature.the_duke_of_the_depths",
            boss_position: at(33712, 32303),
            exit: at(33719, 32302),
            alternate: at(33691, 32301),
            returns_to: at(33275, 32318),
            arrival: at(33717, 32302),
            room_min: at(33684, 32293),
            room_max: at(33724, 32314),
            direction: StepDirection::East,
        },
        _ => return None,
    })
}

pub(super) fn is_victory(name: &str) -> bool {
    victory_exit(name).is_some()
}

fn run_victory(name: &'static str) -> TestResult {
    // Each fixture uses the native USE stage to supply the roster before the sealed death.
    let lever = match name {
        VICTORY => LEVER,
        COUNT_VICTORY => at(33681, 32344),
        DUKE_VICTORY => at(33719, 32315),
        _ => return Err("Warzone exit test case".into()),
    };
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, lever, false)
    })
}

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -15 }
}

#[test]
fn warzone_baron_death_redirects_the_exact_exit_and_restores_it_after_twenty_minutes() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_victory(VICTORY)
}

#[test]
fn warzone_count_exit_teleports_two_players_to_the_same_landing_and_restores() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_victory(COUNT_VICTORY)
}

#[test]
fn warzone_duke_exit_teleports_two_players_facing_west_and_restores() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_victory(DUKE_VICTORY)
}

pub(super) fn terminal_callback(name: &str, lever: Position) -> TestResult<Callback> {
    let spec = victory_exit(name).ok_or("Warzone victory exit")?;
    let document = json!({"schema":1,"callbacks":[{
        "key":format!("oteryn:quest-callback/{name}"),
        "quest":QUEST,"trigger":"use","selector":{"position":lever,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"shared_staging":true,"players":[
                {"from":at(lever.x,lever.y+1),"to":spec.arrival},
                {"from":at(lever.x+1,lever.y+1),"to":spec.arrival}
            ],
            "monsters":[{"creature":spec.boss,"position":spec.boss_position}],
            "room_min":spec.room_min,"room_max":spec.room_max,
            "death_phases":{"terminal":spec.boss,"replacements":[],"terminal_portal":{
                "position":spec.exit,"original_appearance":1949,"open_appearance":22761,
                "destination":spec.alternate,"return_destination":spec.returns_to,"duration_ms":1200000,
                "traversal_effect":TELEPORT_EFFECT
            }}
        }},
        "sources":[DEATH_SOURCE,"crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/actions_using_crystals.lua:371-398,437-519"]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into()
        .map_err(|_| "one Warzone exit binding")?;
    Ok(callback)
}

pub(super) fn portal_world(
    callback: &Callback,
    name: &str,
    lever: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    let spec = victory_exit(name).ok_or("Warzone victory exit")?;
    super::quest_world_callbacks::world_with_scene_cells(
        callback,
        lever,
        &[(spec.exit, 1949)],
        &[22761],
        &[spec.alternate, spec.returns_to],
    )
}

pub(super) async fn exercise_victory(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: Callback,
    name: &str,
    lever: Position,
) -> TestResult {
    use crate::map::{overlay::TilePos, view::MapFacts};
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use oteryn_protocol_oteryn::world_spatial::StepDisposition;
    let spec = victory_exit(name).ok_or("Warzone victory exit")?;
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    let primary = admitted.runtime_actor.ok_or("primary player")?;
    let peer =
        super::ferumbras_crystal_reset::admit_peer(owner, h, at(lever.x + 1, lever.y + 1)).await?;
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::South).await?;
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, lever, 8911).await?,
        UseDisposition::Committed
    );
    let (boss, peer_session) = {
        let runtime = owner.runtime.lock().await;
        for actor in [primary, peer] {
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                super::quest_world_callbacks::local(spec.arrival)
            );
        }
        let boss = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| runtime.matches_live_creature_identity(*actor, spec.boss.as_bytes()))
            .ok_or("Warzone boss")?;
        let peer_session = runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, _, session)| (actor == peer).then_some(session).flatten())
            .ok_or("peer session")?;
        (boss, peer_session)
    };
    let tile = |p: Position| TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let map = owner.bundle_map().ok_or("map")?;
    let at = tile(spec.exit);
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("exit tile")?;
    let id = *base.ids().get(1).ok_or("exit placement")?;
    let appearance = || {
        map.facts
            .base_entry(at, 1, id)
            .map(|facts| facts.appearance_id)
    };
    assert_eq!(appearance(), Some(1949));
    super::engenc2_cults::kill(owner, admitted, boss, spec.boss, 101).await?;
    assert_eq!(appearance(), Some(22761));
    assert_eq!(
        map.facts
            .quest_portal_destination(at, owner.owner_now().get()),
        Some(tile(spec.alternate))
    );
    let effect_cursor = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |(id, _, _)| *id);
    let facing = if spec.exit.x > spec.alternate.x {
        crate::foundation::MovementFacing::West
    } else {
        crate::foundation::MovementFacing::East
    };
    for (actor, session) in [(primary, admitted.game_session_id), (peer, peer_session)] {
        for _ in 0..2 {
            let (outcome, pace) = owner
                .ordinary_paced_step(actor, session, spec.direction, None, None)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
            if let Some(pace) = pace {
                tokio::time::sleep(pace).await;
            }
        }
        let landed = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            landed.position(),
            super::quest_world_callbacks::local(spec.alternate)
        );
        assert_eq!(landed.facing(), Some(facing));
    }
    let effects = map.facts.movement_effects_since(effect_cursor);
    for position in [at, tile(spec.alternate)] {
        assert_eq!(
            effects
                .iter()
                .filter(|(_, where_at, effect)| *where_at == position && *effect == TELEPORT_EFFECT)
                .count(),
            2
        );
    }
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("exit deadline")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(appearance(), Some(22761));
    assert!(map.facts.expire_movement_items(due) > 0);
    assert_eq!(appearance(), Some(1949));
    assert_eq!(map.facts.quest_portal_destination(at, due), None);
    let key = crate::map::view::placement_key(at, 1).ok_or("exit key")?;
    assert!(
        map.facts
            .quest_callback_native_teleport(key, 1949, tile(spec.returns_to))
    );
    Ok(())
}

#[test]
fn warzone_step_stamps_each_entrant_and_refuses_a_retry_without_summoning_another_boss()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_entry(CASE)
}

#[test]
fn warzone_count_entrance_stamps_each_players_count_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_entry(COUNT_ENTRY)
}

#[test]
fn warzone_duke_entrance_stamps_each_players_duke_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_entry(DUKE_ENTRY)
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let spec = entry(name).ok_or("Warzone entrance")?;
    Ok(serde_json::from_value(json!({
        "key":format!("oteryn:quest-callback/{name}"),
        "quest":QUEST,"trigger":"step_in",
        "selector":{"position":spec.portal,"action":57243,"appearance":1949},
        "reversible":false,"passages":[],"storage":[],"offerings":[],
        "requires_world_admission":true,"return_on_failure":true,
        "teleport":spec.arrival,
        "state_operations":[{"track":spec.track,"from":{"kind":"any"},"effect":{"kind":"set_now"}}],
        "effects":{"boss_admission_cooldown":{"creature":spec.boss,"seconds":36000},"teleport_effect":TELEPORT_EFFECT},
        "sources":[SOURCE]
    }))?)
}

pub(super) fn catalogue(callback: &Callback, name: &str) -> TestResult<QuestStateCatalogue> {
    let spec = entry(name).ok_or("Warzone entrance")?;
    use sha2::{Digest, Sha256};
    let digest: String = Sha256::digest(SOURCE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let fixture = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
        "tracks":[{"key":spec.track,"initial":-1,"minimum":-1,"maximum":i64::MAX}],
        "stages":[{"key":"admit-entrant","completes":false,
            "trigger":{"kind":"step_in","callback":callback}}]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&fixture)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let spec = entry(name).ok_or("Warzone entrance")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let entered = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(
            entered.position(),
            super::quest_world_callbacks::local(spec.arrival)
        );
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        let map = owner.bundle_map().ok_or("map")?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let return_to_approach = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            entered,
            origin,
            false,
            &closed,
        )
        .map_err(debug)?;
        runtime
            .commit_source_step(return_to_approach)
            .map_err(debug)?;
    }
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    let approach = Position {
        y: spec.portal.y - 1,
        ..spec.portal
    };
    let peer = super::ferumbras_crystal_reset::admit_peer(owner, h, approach).await?;
    let peer_session = {
        let runtime = owner.runtime.lock().await;
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, _, session)| (actor == peer).then_some(session).flatten())
            .ok_or("peer session")?
    };
    let (outcome, _) = owner
        .ordinary_paced_step(peer, peer_session, StepDirection::South, None, None)
        .await;
    assert_eq!(
        outcome.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime.read_actor_position(peer).map_err(debug)?.position(),
        super::quest_world_callbacks::local(spec.arrival)
    );
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    assert!(
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .is_empty()
    );
    drop(runtime);
    let map = owner.bundle_map().ok_or("Warzone entrance map")?;
    assert_eq!(
        map.facts
            .movement_effects_since(0)
            .iter()
            .filter(|(_, pos, effect)| {
                (pos.x, pos.y, pos.floor) == (spec.arrival.x, spec.arrival.y, spec.arrival.floor)
                    && *effect == TELEPORT_EFFECT
            })
            .count(),
        2,
        "only the two admitted steps publish the arrival effect",
    );
    let cooldowns: Vec<i64> = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM now()) * 1000)::bigint FROM game_character_boss_cooldowns WHERE boss_key=$1",
    ).bind(spec.boss).fetch_all(&h.pool).await?;
    assert_eq!(cooldowns.len(), 2);
    assert!(
        cooldowns
            .iter()
            .all(|cooldown| (35_970_000..=36_000_000).contains(cooldown))
    );
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipts, 2);
    Ok(())
}
