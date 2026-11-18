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
pub(super) const VICTORY: &str = "dangerous-depths-warzone-four-defeat-exit";
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
    super::kill_reward_e2e::scenario_thread::run(VICTORY, || {
        super::quest_world_callbacks::scenario(VICTORY, LEVER, false)
    })
}

pub(super) fn terminal_callback() -> TestResult<Callback> {
    let document = json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/dangerous-depths/warzone-iv-defeat-exit",
        "quest":QUEST,"trigger":"use","selector":{"position":LEVER,"appearance":8911},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"encounter":{
            "minimum_level":0,"shared_staging":true,"players":[{"from":at(33650,32315),"to":ARRIVAL}],
            "monsters":[{"creature":BOSS,"position":at(33648,32303)}],
            "room_min":at(33638,32291),"room_max":at(33675,32313),
            "death_phases":{"terminal":BOSS,"replacements":[],"terminal_portal":{
                "position":EXIT,"original_appearance":1949,"open_appearance":22761,
                "destination":ALTERNATE,"return_destination":RETURN,"duration_ms":1200000
            }}
        }},
        "sources":[DEATH_SOURCE,"crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/actions_using_crystals.lua:371-376,437-449"]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into()
        .map_err(|_| "one Warzone exit binding")?;
    Ok(callback)
}

pub(super) fn portal_world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_scene_cells(
        callback,
        LEVER,
        &[(EXIT, 1949)],
        &[22761],
        &[ALTERNATE, RETURN],
    )
}

pub(super) async fn exercise_victory(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: Callback,
) -> TestResult {
    use crate::map::{overlay::TilePos, view::MapFacts};
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::South).await?;
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, LEVER, 8911).await?,
        UseDisposition::Committed
    );
    let boss = {
        let runtime = owner.runtime.lock().await;
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| runtime.matches_live_creature_identity(*actor, BOSS.as_bytes()))
            .ok_or("Baron")?
    };
    let tile = |p: Position| TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let map = owner.bundle_map().ok_or("map")?;
    let at = tile(EXIT);
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
    super::engenc2_cults::kill(owner, admitted, boss, BOSS, 101).await?;
    assert_eq!(appearance(), Some(22761));
    assert_eq!(
        map.facts
            .quest_portal_destination(at, owner.owner_now().get()),
        Some(tile(ALTERNATE))
    );
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
            .quest_callback_native_teleport(key, 1949, tile(RETURN))
    );
    Ok(())
}

#[test]
fn warzone_step_stamps_individual_cooldown_without_summoning_another_boss() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, PORTAL, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/dangerous-depths/warzone-iv-entry",
        "quest":QUEST,"trigger":"step_in",
        "selector":{"position":PORTAL,"action":57243,"appearance":1949},
        "reversible":false,"passages":[],"storage":[],"offerings":[],
        "requires_world_admission":true,"return_on_failure":true,
        "teleport":ARRIVAL,
        "state_operations":[{"track":TRACK,"from":{"kind":"any"},"effect":{"kind":"set_now"}}],
        "effects":{"boss_admission_cooldown":{"creature":BOSS,"seconds":36000}},
        "sources":[SOURCE]
    }))?)
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    use sha2::{Digest, Sha256};
    let digest: String = Sha256::digest(SOURCE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let fixture = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
        "tracks":[{"key":TRACK,"initial":-1,"minimum":-1,"maximum":i64::MAX}],
        "stages":[{"key":"warzone-iv-entry","completes":false,
            "trigger":{"kind":"step_in","callback":callback}}]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&fixture)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
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
            super::quest_world_callbacks::local(ARRIVAL)
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
    let cooldown: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM now()) * 1000)::bigint FROM game_character_boss_cooldowns WHERE boss_key=$1",
    ).bind(BOSS).fetch_one(&h.pool).await?;
    assert!((35_990_000..=36_000_000).contains(&cooldown));
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipts, 1);
    Ok(())
}
