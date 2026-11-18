//! Crystal actions_dreamcatcher_curse.lua:1-8,39-46.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "nightmare-dreamcatcher-supply";
const KEY: &str = "oteryn:quest-callback/ground-supply/nightmare-dreamcatcher";
// Crystal registers this pole by item type. Mount it on a fixture cell in the arena.
const POLE: Position = Position {
    x: 32208,
    y: 32049,
    floor: -15,
};
const DROP: Position = Position {
    y: POLE.y + 1,
    ..POLE
};
// The starter generation has this qualified Ground item. The donor mask is i29276.
const ITEM: &str = "oteryn:item.tibia.i3031";

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":KEY, "quest":"oteryn:quest.the_dream_courts_quest", "trigger":"use",
        "selector":{"native":{"position":POLE,"ordinal":1,"appearance":29274}},
        "lever":[29274,29275], "requires_world_admission":true,
        "success_message":"You can rip off a dream catcher!",
        "effects":{
            "ground_supply_on_open":true, "reset_after_ms":10000,
            "ground_supply":[{"item":ITEM,"count":1,"position":DROP}],
            "magic":{"position":DROP,"effect":4}
        },
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_dreamcatcher_curse.lua:1-8",
                   "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_dreamcatcher_curse.lua:39-46"]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, POLE, 29274, 1, &[], &[29275], None, &[DROP])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let base = base
        .with_fixture_transition(world_callbacks::primary_transition(callback)?)
        .map_err(debug)?;
    let document = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        &base, &document,
    )?))
}

#[test]
fn nightmare_dreamcatcher_use_mints_once_until_the_native_pole_rearms() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, POLE, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let key = placement_key(
        TilePos {
            x: POLE.x,
            y: POLE.y,
            floor: POLE.floor,
        },
        1,
    )
    .ok_or("pole")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = |revision| {
        encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)
    };
    let start = owner.owner_now().get();
    let command = admitted.continuity.next_command_id;
    let first = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        payload(0)?,
    )
    .await?;
    assert_eq!(
        decode_use_result(&first).map_err(debug)?,
        UseDisposition::Committed
    );
    let first_items = supplies(h).await?;
    assert_eq!(first_items.len(), 1);
    let replay = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        payload(0)?,
    )
    .await?;
    assert_eq!(replay, first);
    let command = admitted.continuity.next_command_id;
    let spent = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        payload(map.facts.object_revision(key))?,
    )
    .await?;
    assert_ne!(
        decode_use_result(&spent).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(supplies(h).await?, first_items);
    let due = map.facts.next_movement_decay_time().ok_or("pole rearm")?;
    assert!(due >= start + 10_000_000);
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    let revision = map.facts.object_revision(key);
    let elapsed = due.saturating_sub(owner.owner_now().get());
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_micros(elapsed))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let rearmed = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        payload(revision)?,
    )
    .await?;
    assert_eq!(
        decode_use_result(&rearmed).map_err(debug)?,
        UseDisposition::Committed
    );
    let final_items = supplies(h).await?;
    assert_eq!(final_items.len(), 2);
    assert!(final_items.contains(&first_items[0]));
    Ok(())
}

async fn supplies(h: &Harness) -> TestResult<Vec<String>> {
    let mut spatial = i32::from(DROP.x).to_be_bytes().to_vec();
    spatial.extend_from_slice(&i32::from(DROP.y).to_be_bytes());
    spatial.extend_from_slice(&i16::from(DROP.floor).to_be_bytes());
    Ok(sqlx::query_scalar(
        "SELECT i.item_instance_id::text
           FROM game_native_map_item_receipts r JOIN game_item_instances i USING (item_instance_id)
           JOIN game_item_ground_locations g USING (item_instance_id)
          WHERE r.source_attributes #>> '{quest_supply,transition}'=$1
            AND i.definition_production_key=$2 AND i.quantity=1 AND g.spatial_position=$3
          ORDER BY i.item_instance_id",
    )
    .bind(KEY)
    .bind(ITEM)
    .bind(spatial)
    .fetch_all(&h.pool)
    .await?)
}
