//! Crystal 201189f8 actions_mission01_lights.lua:47–68.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use serde_json::json;
use std::time::Duration;

pub(super) const NAME: &str = "qewrath-wall-recreation";
const LEVER: Position = Position { x: 33349, y: 31075, floor: -8 };
const WALL: Position = Position { x: 33355, y: 31068, floor: -9 };

#[test]
fn shrine_lever_flips_while_the_wall_passage_keeps_its_original_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, LEVER, false)
    })
}
pub(super) fn fixture() -> TestResult<Callback> {
    let row = json!({
        "key":"oteryn:quest-callback/wrath/shrine-wall-recreation",
        "quest":"oteryn:quest.wrath_of_the_emperor_quest","trigger":"use",
        "selector":{"position":LEVER,"appearance":9125},
        "lever":[9125,9126],"reversible":true,"offerings":[],"return_on_failure":false,"storage":[],
        "passages":[{"position":WALL,"appearance":8348,"replacement":8290,"if_present":true}],
        "effects":{"passages_reset_after_ms":20000,"retain_replaced_passages":true,"passages_independent_toggle":true},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_lights.lua:47-68"]
    });
    Ok(serde_json::from_value(row)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    owner.states.quest_callbacks.fixture.set(vec![fixture()?]).map_err(|_| "wall fixture")?;
    let lever = crate::map::view::placement_key(TilePos { x: LEVER.x, y: LEVER.y, floor: LEVER.floor }, 1).ok_or("lever")?;
    let wall = TilePos { x: WALL.x, y: WALL.y, floor: WALL.floor };
    let raw = *world.base().tile(wall.x, wall.y, wall.floor).and_then(|tile| tile.ids().get(1)).ok_or("wall tile")?;
    let appearance = || -> TestResult<u16> {
        Ok(world.facts().base_entry(wall, 1, raw).ok_or("wall item")?.appearance_id)
    };
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&lever.to_be_bytes());
    let lever_appearance = || -> TestResult<u16> {
        let raw = *world.base().tile(LEVER.x, LEVER.y, LEVER.floor)
            .and_then(|tile| tile.ids().get(1)).ok_or("lever tile")?;
        Ok(world.facts().base_entry(TilePos { x: LEVER.x, y: LEVER.y, floor: LEVER.floor }, 1, raw)
            .ok_or("lever item")?.appearance_id)
    };
    let mut due = None;
    for (revision, expected) in [(0, 9126), (1, 9125)] {
        let command = admitted.continuity.next_command_id;
        let payload = encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        }).map_err(debug)?;
        let reply = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
        assert_eq!(lever_appearance()?, expected);
        assert_eq!(appearance()?, 8290);
        let current = world.facts().next_movement_decay_time().ok_or("wall deadline")?;
        assert_eq!(*due.get_or_insert(current), current);
    }
    assert!(world.facts().movement_items(wall).is_empty());
    let due = due.ok_or("passage window")?;
    assert_eq!(world.facts().expire_movement_items(due - 1), 0);
    owner.clock_origin -= Duration::from_secs(21);
    let shutdown = oteryn_foundation::CancellationToken::new();
    let ((), expired) = tokio::join!(
        owner.run_source_owner_cycles(&shutdown),
        async {
            let expired = tokio::time::timeout(Duration::from_secs(3), async {
                while world.facts().next_movement_decay_time().is_some() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }).await;
            shutdown.cancel();
            expired
        }
    );
    expired?;
    assert_eq!(appearance()?, 8348);
    assert_eq!(lever_appearance()?, 9125);
    let retained = world.facts().movement_items(wall);
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].facts.appearance_id, 8290);
    Ok(())
}
