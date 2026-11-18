//! Crystal 201189f dreamers_challenge_quest/actions_tic_tac.lua:3-12.
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

pub(super) const NAME: &str = "dreamers-board-supply";
const KEY: &str = "oteryn:quest-callback/ground-supply/dreamers-challenge-tokens";
const LEVER: Position = Position {
    x: 32838,
    y: 32263,
    floor: -14,
};
const WHITE: Position = Position { y: 32264, ..LEVER };
const BLACK: Position = Position { x: 32839, ..LEVER };
// Portable stack policies already admitted in this generation. Content
// authoring substitutes the donor token Items i3547 and i3548.
const WHITE_ITEM: &str = "oteryn:item.tibia.i3031";
const BLACK_ITEM: &str = "oteryn:item.tibia.i3035";

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":KEY,"quest":"oteryn:quest.dreamer_s_challenge_quest","trigger":"use",
        "selector":{"native":{"position":LEVER,"ordinal":2,"appearance":2772}},
        "lever":[2772,2773],"reversible":true,"requires_world_admission":true,
        "passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"ground_supply":[
            {"item":WHITE_ITEM,"count":8,"position":WHITE},
            {"item":BLACK_ITEM,"count":12,"position":BLACK}
        ]},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/actions_tic_tac.lua:3-12"]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(
        callback,
        LEVER,
        2772,
        2,
        &[],
        &[2773],
        None,
        &[WHITE, BLACK],
    )
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
fn dreamers_board_supply_mints_exact_ground_stacks_only_on_the_return_lever_face_and_once_on_replay()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, LEVER, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let key = placement_key(
        TilePos {
            x: LEVER.x,
            y: LEVER.y,
            floor: LEVER.floor,
        },
        2,
    )
    .ok_or("lever")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = |revision| {
        encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)
    };

    let initial = admitted.continuity.next_command_id;
    let reply = wire(
        owner,
        admitted,
        initial,
        COMMAND_TYPE_USE_INTENT,
        payload(0)?,
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(supplies(h).await?.is_empty());
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(1))
        .ok_or("clock")?;
    let revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    let command = admitted.continuity.next_command_id;
    let reply = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        payload(revision)?,
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let minted = supplies(h).await?;
    assert_eq!(
        minted
            .iter()
            .map(|(item, count, position, _)| (item.as_str(), *count, position.as_slice()))
            .collect::<Vec<_>>(),
        [
            (WHITE_ITEM, 8, spatial(WHITE).as_slice()),
            (BLACK_ITEM, 12, spatial(BLACK).as_slice())
        ]
    );
    let replay = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        payload(revision)?,
    )
    .await?;
    assert_eq!(replay, reply);
    assert_eq!(supplies(h).await?, minted);
    Ok(())
}

async fn supplies(h: &Harness) -> TestResult<Vec<(String, i64, Vec<u8>, String)>> {
    Ok(sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity,g.spatial_position,i.item_instance_id::text
         FROM game_native_map_item_receipts r JOIN game_item_instances i USING(item_instance_id)
         JOIN game_item_ground_locations g USING(item_instance_id)
         WHERE r.source_attributes #>> '{quest_supply,transition}'=$1 ORDER BY r.placement_key",
    )
    .bind(KEY)
    .fetch_all(&h.pool)
    .await?)
}

fn spatial(p: Position) -> Vec<u8> {
    let mut bytes = i32::from(p.x).to_be_bytes().to_vec();
    bytes.extend_from_slice(&i32::from(p.y).to_be_bytes());
    bytes.extend_from_slice(&i16::from(p.floor).to_be_bytes());
    bytes
}
