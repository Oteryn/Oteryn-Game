//! Crystal 201189f8 actions_mission01_lights.lua:8–12,17–29,47–68.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use serde_json::json;
use std::time::Duration;

pub(super) const NAME: &str = "qewrath-wall-recreation";
pub(super) const LAMP_NAME: &str = "qewrath-lamp-restoration";
const LEVER: Position = Position {
    x: 33349,
    y: 31075,
    floor: -8,
};
const WALL: Position = Position {
    x: 33355,
    y: 31068,
    floor: -9,
};
const LAMP_LEVER: Position = Position {
    x: 33370,
    y: 31067,
    floor: -9,
};
const LAMP: Position = Position {
    x: 33369,
    y: 31075,
    floor: -8,
};

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
#[test]
fn shrine_lamp_timer_transforms_and_restores_only_the_matching_lamp() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(LAMP_NAME, || {
        super::quest_world_callbacks::scenario(LAMP_NAME, LAMP_LEVER, false)
    })
}

fn fixture(lamp: bool) -> TestResult<Callback> {
    let (lever, passage, appearance, replacement, key, source) = if lamp {
        (LAMP_LEVER, LAMP, 10491, 10479, LAMP_NAME, "8-29")
    } else {
        (LEVER, WALL, 8348, 8290, NAME, "47-68")
    };
    let row = json!({
        "key":format!("oteryn:quest-callback/wrath/{key}"),
        "quest":"oteryn:quest.wrath_of_the_emperor_quest","trigger":"use",
        "selector":{"position":lever,"appearance":9125},
        "lever":[9125,9126],"reversible":true,"offerings":[],"return_on_failure":false,"storage":[],
        "passages":[{"position":passage,"appearance":appearance,"replacement":replacement,"if_present":true}],
        "effects":{"passages_reset_after_ms":20000,"retain_replaced_passages":!lamp,"passages_independent_toggle":true},
        "sources":[format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_lights.lua:{source}")]
    });
    Ok(serde_json::from_value(row)?)
}

pub(super) fn world(lamp: bool, trigger: Position) -> TestResult<crate::map::boot::BundleWorld> {
    let callback = fixture(lamp)?;
    if lamp {
        super::quest_world_callbacks::world_with_objects(&callback, trigger, &[(LAMP, 10479)], &[])
    } else {
        super::quest_world_callbacks::world(&callback, trigger)
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    world: &crate::map::boot::BundleWorld,
    lamp: bool,
) -> TestResult {
    let mut callback = fixture(lamp)?;
    let lever_at = callback.selector.position.ok_or("lever position")?;
    let [passage] = callback.passages.as_slice() else {
        return Err("timed passage fixture".into());
    };
    let before = passage.appearance;
    let toggled = passage.replacement.ok_or("replacement appearance")?;
    let remote = TilePos {
        x: passage.position.x,
        y: passage.position.y,
        floor: passage.position.floor,
    };
    if lamp {
        // Crystal 201189f8 actions_mission01_lights.lua:8–12,21–29
        // looks up each lamp independently and still toggles the source lever.
        let mut optional = passage.clone();
        optional.position.x = 33372;
        assert!(
            world
                .base()
                .tile(optional.position.x, optional.position.y, optional.position.floor)
                .is_none()
        );
        callback.passages.push(optional);
    }
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "passage fixture")?;
    let lever_at = TilePos {
        x: lever_at.x,
        y: lever_at.y,
        floor: lever_at.floor,
    };
    let lever = crate::map::view::placement_key(lever_at, 1).ok_or("lever")?;
    let appearance = |at: TilePos, ordinal: u8| -> TestResult<u16> {
        let raw = *world
            .base()
            .tile(at.x, at.y, at.floor)
            .and_then(|tile| tile.ids().get(usize::from(ordinal)))
            .ok_or("passage tile")?;
        Ok(world
            .facts()
            .base_entry(at, ordinal, raw)
            .ok_or("passage item")?
            .appearance_id)
    };
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&lever.to_be_bytes());
    let mut due = None;
    for (revision, expected) in [(0, 9126), (1, 9125)] {
        let command = admitted.continuity.next_command_id;
        let payload = encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(appearance(lever_at, 1)?, expected);
        assert_eq!(appearance(remote, 1)?, toggled);
        let current = world
            .facts()
            .next_movement_decay_time()
            .ok_or("passage deadline")?;
        assert_eq!(*due.get_or_insert(current), current);
    }
    assert!(world.facts().movement_items(remote).is_empty());
    let due = due.ok_or("passage window")?;
    assert_eq!(world.facts().expire_movement_items(due - 1), 0);
    owner.clock_origin -= Duration::from_secs(21);
    let shutdown = oteryn_foundation::CancellationToken::new();
    let ((), expired) = tokio::join!(owner.run_source_owner_cycles(&shutdown), async {
        let expired = tokio::time::timeout(Duration::from_secs(3), async {
            while world.facts().next_movement_decay_time().is_some() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        shutdown.cancel();
        expired
    });
    expired?;
    assert_eq!(appearance(remote, 1)?, before);
    assert_eq!(appearance(lever_at, 1)?, 9125);
    let retained = world.facts().movement_items(remote);
    if lamp {
        let unowned_lamp = crate::map::view::placement_key(remote, 2).ok_or("unowned lamp")?;
        assert_eq!(appearance(remote, 2)?, 10479);
        assert_eq!(world.facts().object_revision(unowned_lamp), 0);
        assert!(retained.is_empty());
    } else {
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].facts.appearance_id, toggled);
    }
    Ok(())
}
