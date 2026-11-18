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
    carrion["effects"] = json!({"achievement":ACHIEVEMENT});
    let mut drilling = row("drilling", DRILLING, 9196);
    drilling["storage"] = json!([
        {"track":mission,"minimum":2,"maximum":2,"set":2,"preserve":true},
        {"track":COUNTER,"minimum":0,"maximum":5,"set":0},
        {"track":DEBT,"minimum":0,"maximum":1,"set":1}
    ]);
    [carrion, drilling]
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(rows()[0].clone())?)
}

pub(super) fn world(row: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_objects(row, CARRION, &[(DRILLING, 9196)], &[])
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
            expected_revision: map.facts.object_revision(key),
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
        .and_then(|index| index.definition_ref("Item", HAMMER, "definition-r1"))
        .ok_or("heated hammer reference")?
    };
    let command = admitted.continuity.next_command_id;
    let (next, outcome, _) =
        super::item_craft::exchange(owner, *admitted, command, &intent, Some(reference)).await?;
    *admitted = next;
    Ok(outcome)
}
