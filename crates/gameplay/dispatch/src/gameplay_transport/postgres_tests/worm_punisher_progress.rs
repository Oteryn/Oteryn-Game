//! Crystal 201189f8 actions_heated_worm_punisher.lua:13-56.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    isle_of_evil,
    world_callbacks::{self, Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use serde_json::{Value, json};

pub(super) const NAME: &str = "quest-worm-punisher-progress";
pub(super) const ROTATING: &str = "quest-worm-punisher-progress-rotating";
pub(super) const HEATING: &str = "quest-worm-punisher-progress-heating";
const CARRION: Position = Position {
    x: 32500,
    y: 32000,
    floor: -8,
};
const DRILLING: Position = Position {
    x: 32499,
    y: 32001,
    floor: -8,
};
const CRUCIBLE: Position = Position {
    x: 32499,
    y: 31999,
    floor: -8,
};
const COLD_HAMMER: &str = "oteryn:item.tibia.i9234";
const HAMMER: &str = "oteryn:item.tibia.i9201";
const COUNTER: &str = "oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/mission04-wormcount";
const DEBT: &str = "oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/mission04-mustpay";
const ACHIEVEMENT: &str = "oteryn:achievement/worm_whacker";

#[test]
fn worm_punisher_selected_hot_hammer_updates_debt_and_earns_worm_whacker_on_sixth_hit() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, CARRION, false)
    })
}

#[test]
fn wormholes_rotate_on_owner_clock_and_reject_a_stale_carrion_handle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(ROTATING, || {
        super::quest_world_callbacks::scenario(ROTATING, CARRION, false)
    })
}

#[test]
fn worm_punisher_heats_fences_expires_and_reheats_the_same_carried_item() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(HEATING, || {
        super::quest_world_callbacks::scenario(HEATING, CARRION, false)
    })
}

fn rows() -> [Value; 2] {
    let mission = format!("{}mission04", isle_of_evil::PREFIX);
    let row = |suffix: &str, position: Position, id: u16| {
        json!({
            "key":format!("oteryn:quest-callback/fixture-worm-punisher-{suffix}"),
            "quest":isle_of_evil::QUEST,"trigger":"use",
            "selector":{"position":position,"appearance":id,"native":{"position":position,"ordinal":1,"appearance":id}},
            "item_on_target":{"item":HAMMER,"count":1,"reusable":true,"selected_instance":true},
            "requires_world_admission":true,
            "lever":null,"reversible":false,"teleport":null,"offerings":[],"passages":[],
            "return_on_failure":false,"failure_message":null,
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_isle_of_evil_quest/actions_heated_worm_punisher.lua:13-56"]
        })
    };
    let mut carrion = row("last-carrion", CARRION, 9197);
    carrion["storage"] = json!([
        {"track":mission,"minimum":2,"maximum":2,"set":3},
        {"track":COUNTER,"minimum":5,"maximum":5,"set":0},
        {"track":DEBT,"minimum":0,"maximum":0,"set":0,"preserve":true}
    ]);
    carrion["effects"] = json!({"achievement":ACHIEVEMENT,"source_magic":32,
        "player_speech":"You've got him! That was the last one you had to scare off. Report to Scutty about the worms."});
    let mut drilling = row("drilling", DRILLING, 9196);
    drilling["storage"] = json!([
        {"track":mission,"minimum":2,"maximum":2,"set":2,"preserve":true},
        {"track":COUNTER,"minimum":0,"maximum":5,"set":0},
        {"track":DEBT,"minimum":0,"maximum":1,"set":1}
    ]);
    drilling["effects"] = json!({"source_magic":5,
        "player_speech":"Oh no! You have hit one of Scutty's precious drilling worms!"});
    [carrion, drilling]
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(rows()[0].clone())?)
}

pub(super) fn world(row: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_objects(
        row,
        CARRION,
        &[(DRILLING, 9196), (CRUCIBLE, 9242)],
        &[9194, 9195, 9243],
    )
}

pub(super) async fn exercise_heating(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use std::time::Duration;
    let mission = format!("{}mission04", isle_of_evil::PREFIX);
    let source_gate = json!({"track":mission,"minimum":2,"maximum":2,"set":2,"preserve":true});
    let [mut hit, _] = rows();
    hit["key"] = json!("oteryn:quest-callback/fixture-worm-punisher-early-carrion");
    hit["storage"] = json!([source_gate.clone(),
        {"track":COUNTER,"minimum":0,"maximum":4,"set":1,"increment":true},
        {"track":DEBT,"minimum":0,"maximum":0,"set":0,"preserve":true}]);
    hit["effects"] = json!({"carried_tool_window":{"kind":"require"},"source_magic":32,
        "player_speech":"You've got gim! That will tech the beast a lesson!"});
    // Crystal 201189f8 actions_heat_worm_punisher.lua:3-22 and items.xml:24803-24809,25010-25019.
    let heat = json!({
        "key":"oteryn:quest-callback/fixture-worm-punisher-heat",
        "quest":isle_of_evil::QUEST,"trigger":"use",
        "selector":{"position":CRUCIBLE,"appearance":9242,"native":{"position":CRUCIBLE,"ordinal":1,"appearance":9242}},
        "item_on_target":{"item":COLD_HAMMER,"count":1,"selected_instance":true,"replacement":HAMMER},
        "requires_world_admission":true,"lever":null,"reversible":false,"teleport":null,"offerings":[],"passages":[],
        "return_on_failure":false,"failure_message":null,"storage":[source_gate],
        "effects":{"carried_tool_window":{"kind":"start","duration_ms":10000},"source_magic":37,
            "player_speech":"You've heated up the hammer for a short while.",
            "object_cycle":{"initial":9242,"stages":[
                {"appearance":9242,"duration_ms":60000,"next":9243},
                {"appearance":9243,"duration_ms":60000,"next":9242}
            ]}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_isle_of_evil_quest/actions_heat_worm_punisher.lua:3-22"]
    });
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":[heat,hit]}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(mission.clone(), 1), (COUNTER.into(), 0), (DEBT.into(), 0)],
        121,
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: CARRION,
            item: COLD_HAMMER.into(),
            count: 1,
        },
        87,
        true,
    )
    .await?;
    let item: String = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(COLD_HAMMER).fetch_one(&h.pool).await?;
    let key = oteryn_world_bundle::bundle::placement_key(CRUCIBLE.floor, CRUCIBLE.x, CRUCIBLE.y, 1)
        .ok_or("crucible placement")?;
    let stop = oteryn_foundation::CancellationToken::new();
    let observe = async {
        let map = owner.bundle_map().ok_or("map")?;
        while !map.facts.quest_object_cycle_matches(key, 9242) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        stop.cancel();
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, ready) = tokio::time::timeout(
        Duration::from_secs(10),
        super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe),
    )
    .await?;
    ready?;
    assert_ne!(
        use_tool(owner, admitted, CRUCIBLE, 9242, None, COLD_HAMMER).await?,
        UseDisposition::Committed
    );
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(mission.clone(), 2)],
        122,
    )
    .await?;
    assert_eq!(
        use_tool(owner, admitted, CRUCIBLE, 9242, None, COLD_HAMMER).await?,
        UseDisposition::Committed
    );
    assert_eq!(definition(h, &item).await?, HAMMER);
    assert_eq!(
        strike(owner, admitted, CARRION, 9197).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, COUNTER).await?, Some(1));

    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_ne!(
        strike(owner, admitted, CARRION, 9197).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, COUNTER).await?, Some(1));
    owner.drain_source_item_deadlines().await;
    assert_eq!(definition(h, &item).await?, COLD_HAMMER);
    assert_eq!(
        use_tool(owner, admitted, CRUCIBLE, 9242, None, COLD_HAMMER).await?,
        UseDisposition::Committed
    );
    assert_eq!(definition(h, &item).await?, HAMMER);
    let windows: Vec<i32> = sqlx::query_scalar("SELECT duration_ms FROM game_quest_tool_windows WHERE item_instance_id=$1::uuid ORDER BY started_ms")
        .bind(&item).fetch_all(&h.pool).await?;
    assert_eq!(windows, [10_000, 10_000]);
    assert_eq!(super::m39_bone_journey::stage(h, &mission).await?, Some(2));
    Ok(())
}

async fn definition(h: &Harness, item: &str) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=$1::uuid AND lifecycle=1")
        .bind(item).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise_rotation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use std::time::Duration;
    let [mut carrion, mut drilling] = rows();
    // Crystal 201189f8 data-global/items/items.xml:24775-24786.
    let cycle = json!({"initial":9197,"stages":[
        {"appearance":9197,"duration_ms":2000,"next":9195},
        {"appearance":9195,"duration_ms":4000,"next":9196},
        {"appearance":9196,"duration_ms":2000,"next":9194},
        {"appearance":9194,"duration_ms":7000,"next":9197}
    ]});
    carrion["effects"]["object_cycle"] = cycle.clone();
    drilling["selector"]["position"] = json!(CARRION);
    drilling["selector"]["native"]["position"] = json!(CARRION);
    drilling["effects"]["object_cycle"] = cycle;
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":[carrion,drilling]}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    let mission = format!("{}mission04", isle_of_evil::PREFIX);
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(mission.clone(), 2), (COUNTER.into(), 5), (DEBT.into(), 0)],
        121,
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: CARRION,
            item: HAMMER.into(),
            count: 1,
        },
        87,
        true,
    )
    .await?;

    let key = oteryn_world_bundle::bundle::placement_key(CARRION.floor, CARRION.x, CARRION.y, 1)
        .ok_or("worm placement")?;
    let old_revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    let stop = oteryn_foundation::CancellationToken::new();
    let observe = async {
        let map = owner.bundle_map().ok_or("map")?;
        while !map.facts.quest_object_cycle_matches(key, 9195) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        stop.cancel();
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, advanced) = tokio::time::timeout(
        Duration::from_secs(10),
        super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe),
    )
    .await?;
    advanced?;
    assert_ne!(
        strike_at_revision(owner, admitted, CARRION, 9195, Some(old_revision)).await?,
        UseDisposition::Committed
    );
    assert_ne!(
        strike(owner, admitted, CARRION, 9195).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, COUNTER).await?, Some(5));
    {
        let map = owner.bundle_map().ok_or("map")?;
        let due = map
            .facts
            .next_movement_decay_time()
            .ok_or("next worm stage")?;
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
        assert_eq!(map.facts.expire_movement_items(due), 1);
        assert!(map.facts.quest_object_cycle_matches(key, 9196));
    }
    assert_eq!(
        strike(owner, admitted, CARRION, 9196).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, &mission).await?, Some(2));
    assert_eq!(super::m39_bone_journey::stage(h, COUNTER).await?, Some(0));
    assert_eq!(super::m39_bone_journey::stage(h, DEBT).await?, Some(1));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":rows()}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    let mission = format!("{}mission04", isle_of_evil::PREFIX);
    let historical = |debt| {
        [
            (mission.clone(), 2),
            (COUNTER.into(), 5),
            (DEBT.into(), debt),
        ]
    };
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &historical(0),
        121,
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: CARRION,
            item: HAMMER.into(),
            count: 1,
        },
        87,
        true,
    )
    .await?;
    assert_eq!(
        strike(owner, admitted, DRILLING, 9196).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, DEBT).await?, Some(1));
    assert_eq!(super::m39_bone_journey::stage(h, COUNTER).await?, Some(0));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &historical(1),
        122,
    )
    .await?;
    assert_ne!(
        strike(owner, admitted, CARRION, 9197).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, &mission).await?, Some(2));

    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &historical(0),
        123,
    )
    .await?;
    assert_eq!(
        strike(owner, admitted, CARRION, 9197).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::m39_bone_journey::stage(h, &mission).await?, Some(3));
    assert_eq!(super::m39_bone_journey::stage(h, COUNTER).await?, Some(0));
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(grants, 1);
    assert_eq!(super::m39_bone_journey::carried(h, 9201).await?, 1);
    Ok(())
}

async fn strike(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    appearance: u16,
) -> TestResult<UseDisposition> {
    strike_at_revision(owner, admitted, target, appearance, None).await
}

async fn strike_at_revision(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    appearance: u16,
    expected_revision: Option<u64>,
) -> TestResult<UseDisposition> {
    use_tool(
        owner,
        admitted,
        target,
        appearance,
        expected_revision,
        HAMMER,
    )
    .await
}

async fn use_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    target: Position,
    appearance: u16,
    expected_revision: Option<u64>,
    tool: &str,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("bundle map")?;
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1)
        .ok_or("worm placement")?;
    let tile = crate::map::overlay::TilePos {
        x: target.x,
        y: target.y,
        floor: target.floor,
    };
    let stack = crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), tile)
        .map_err(debug)?
        .ok_or("worm tile")?;
    assert!(stack.entries.iter().any(|entry| entry.source.key()
        == crate::map::view::MapEntryKey::Base(key)
        && entry.facts.appearance_id == appearance));
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: expected_revision.unwrap_or_else(|| map.facts.object_revision(key)),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let reference = {
        let runtime = owner.runtime.lock().await;
        crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .and_then(|index| index.definition_ref("Item", tool, "definition-r1"))
        .ok_or("worm punisher definition")?
    };
    let command = admitted.continuity.next_command_id;
    let (next, outcome, _) =
        super::item_craft::exchange(owner, *admitted, command, &intent, Some(reference)).await?;
    *admitted = next;
    Ok(outcome)
}
