//! The retained Wrath light and wall switches through ordinary map USE and Channel expiry.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{NativeSelector, Passage, Position, Selector};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const NAME: &str = "qengine-rewards-wrathm1-relay";
pub(super) const SOURCE: Position = Position {
    x: 33370,
    y: 31067,
    floor: -9,
};
const DETECTION: Position = Position { x: 33370, y: 31075, floor: -8 };
const APPROACH: Position = Position { y: 31074, ..DETECTION };
const PRISON: Position = Position { x: 33361, y: 31206, floor: -8 };
const LAMP_X: [u16; 3] = [33369, 33372, 33375];
const LAMP_Y: u16 = 31075;
const LAMP_FLOOR: i8 = -8;
const WALL_SWITCH: Position = Position {
    x: 33349,
    y: 31075,
    floor: -8,
};
const WALL_X: u16 = 33355;
const WALL_Y: u16 = 31068;
const WALL_ITEMS: [u16; 4] = [8348, 8291, 8293, 8348];

#[test]
fn wrathm1_relay_switch_reverses_without_extending_shared_light_timer() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, SOURCE, false)
    })
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    let patches = [
        (
            SOURCE.x - 1,
            SOURCE.y - 1,
            SOURCE.x + 1,
            SOURCE.y + 1,
            SOURCE.floor,
        ),
        (LAMP_X[0], APPROACH.y, LAMP_X[2], LAMP_Y, LAMP_FLOOR),
        (PRISON.x, PRISON.y, PRISON.x, PRISON.y, PRISON.floor),
        (
            WALL_SWITCH.x - 1,
            WALL_SWITCH.y - 1,
            WALL_SWITCH.x + 1,
            WALL_SWITCH.y + 1,
            WALL_SWITCH.floor,
        ),
        (WALL_X, WALL_Y, WALL_X, WALL_Y + 3, SOURCE.floor),
    ];
    for start in neighbours(SOURCE) {
        match super::questpc8_native_slice::world(native, &patches, &[9126, 10479], tile(start)) {
            Err(error) if error.to_string() == "StartNotEnterable" => continue,
            result => return result,
        }
    }
    Err("the retained relay switch has no enterable adjacent tile".into())
}

fn neighbours(source: Position) -> impl Iterator<Item = Position> {
    (source.y - 1..=source.y + 1)
        .flat_map(move |y| (source.x - 1..=source.x + 1).map(move |x| Position { x, y, ..source }))
}

async fn approach(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    source: Position,
) -> TestResult {
    use crate::content::{CollisionClass, LogicalCell};

    let cells = &owner.movement_cells;
    let at = neighbours(source)
        .find(|pos| {
            cells.index().lookup(
                cells.scope(),
                LogicalCell {
                    x: i32::from(pos.x),
                    y: i32::from(pos.y),
                    z: -i32::from(pos.floor),
                },
            ) == Ok(CollisionClass::Walkable)
        })
        .ok_or("no walkable approach to the relay")?;
    super::questpc8_mystery_geometry::arrive(owner, admitted, tile(at)).await
}

fn tile(at: Position) -> TilePos {
    TilePos {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let mut callback = super::authored_world_rewards::placeholder(SOURCE)?;
    callback.key = "oteryn:quest-callback/test/wrathm1-relay-first".into();
    callback.quest = "oteryn:quest.wrath_of_the_emperor_quest".into();
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_lights.lua:20-34".into()];
    callback.selector = Selector {
        position: None,
        position_aliases: Vec::new(),
        ordinal: None,
        unique: None,
        action: None,
        appearance: None,
        native: Some(NativeSelector {
            position: SOURCE,
            ordinal: 2,
            appearance: 9125,
        }),
    };
    callback.lever = Some([9125, 9126]);
    callback.reversible = true;
    callback.storage.clear();
    callback.requires_world_admission = false;
    callback.effects.passages_reset_after_ms = Some(20_000);
    let lamps = LAMP_X.map(|x| Position {
        x,
        y: LAMP_Y,
        floor: LAMP_FLOOR,
    });
    callback.passages = lamps
        .into_iter()
        .map(|position| Passage {
            position,
            appearance: 10491,
            if_present: true,
            replacement: Some(10479),
            replacement_passable: None,
            create_on_close: None,
            require_state: false,
        })
        .collect();
    let mut wall = callback.clone();
    wall.key = "oteryn:quest-callback/test/wrathm1-relay-wall".into();
    wall.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_lights.lua:45-73".into()];
    wall.selector.native = Some(NativeSelector {
        position: WALL_SWITCH,
        ordinal: 1,
        appearance: 9125,
    });
    let walls = [0, 1, 2, 3].map(|offset| Position {
        x: WALL_X,
        y: WALL_Y + offset,
        floor: SOURCE.floor,
    });
    wall.passages = walls
        .into_iter()
        .zip(WALL_ITEMS)
        .map(|(position, appearance)| Passage {
            position,
            appearance,
            if_present: true,
            replacement: Some(8290),
            replacement_passable: Some(true),
            create_on_close: None,
            require_state: false,
        })
        .collect();
    let mut guard = callback.clone();
    guard.key = "oteryn:quest-callback/test/wrathm1-relay-guard".into();
    guard.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/movements_crate.lua:268-273".into()];
    guard.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
    guard.selector.native = Some(NativeSelector { position: DETECTION, ordinal: 0, appearance: 10480 });
    guard.lever = None;
    guard.reversible = false;
    guard.effects = Default::default();
    guard.effects.guard_bypass = Some(
        oteryn_gameplay_quests::quest::world_callbacks::GuardBypass::ActiveRelay {
            position: SOURCE,
            ordinal: 2,
        },
    );
    guard.passages.clear();
    guard.teleport = Some(PRISON);
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback, guard, wall])
        .map_err(|_| "relay callback fixture already set")?;
    cross_guard(owner, admitted, PRISON).await?;
    approach(owner, admitted, SOURCE).await?;
    let map = owner.bundle_map().ok_or("relay map")?;
    let source = placement_key(tile(SOURCE), 2).ok_or("lever key")?;
    let target = |key: u64| {
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        placement
    };
    let placement = target(source);
    let wall_source = placement_key(tile(WALL_SWITCH), 1).ok_or("wall switch key")?;
    let wall_placement = target(wall_source);
    let wall_keys = walls
        .into_iter()
        .map(|wall| placement_key(tile(wall), 1).ok_or("wall key"))
        .collect::<Result<Vec<_>, _>>()?;
    let lamp_keys = lamps
        .into_iter()
        .map(|lamp| placement_key(tile(lamp), 1).ok_or("lamp key"))
        .collect::<Result<Vec<_>, _>>()?;

    use_switch(owner, admitted, &placement, 0).await?;
    let (due, lit_revisions) = {
        let map = owner.bundle_map().ok_or("relay map")?;
        let due = map
            .facts
            .next_movement_decay_time()
            .ok_or("shared lamp deadline")?;
        let revisions = lamp_keys
            .iter()
            .map(|key| map.facts.object_revision(*key))
            .collect::<Vec<_>>();
        assert_eq!(revisions, [1, 1, 1]);
        (due, revisions)
    };

    use_switch(owner, admitted, &placement, 1).await?;
    approach(owner, admitted, WALL_SWITCH).await?;
    use_switch(owner, admitted, &wall_placement, 0).await?;

    cross_guard(owner, admitted, DETECTION).await?;
    let map = owner.bundle_map().ok_or("relay map")?;
    assert_eq!(map.facts.object_revision(source), 2);
    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
    for (&key, revision) in lamp_keys.iter().zip(lit_revisions) {
        assert_eq!(map.facts.object_revision(key), revision);
    }
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 3);
    for key in lamp_keys {
        assert_eq!(map.facts.object_revision(key), 2);
    }
    assert_eq!(map.facts.object_revision(source), 2);
    let wall_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("shared wall deadline")?;
    assert!(wall_due > due);
    for &key in &wall_keys {
        assert_eq!(map.facts.object_revision(key), 1);
    }
    assert_eq!(map.facts.expire_movement_items(wall_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(wall_due), 4);
    for key in wall_keys {
        assert_eq!(map.facts.object_revision(key), 2);
    }
    assert_eq!(map.facts.object_revision(wall_source), 1);
    cross_guard(owner, admitted, PRISON).await?;
    Ok(())
}

async fn use_switch(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    placement: &[u8],
    expected_revision: u64,
) -> TestResult {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let payload = encode_use_intent(&WorldObjectTarget {
        placement: placement.to_vec(),
        expected_revision,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    Ok(())
}

async fn cross_guard(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    expected: Position,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
        decode_step_result, encode_step_intent,
    };
    super::questpc8_mystery_geometry::arrive(owner, admitted, tile(APPROACH)).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let reply = super::quest_world_callbacks::wire(
        owner, admitted, admitted.continuity.next_command_id, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::South)
    ).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime.read_actor_position(actor).map_err(debug)?.position(),
        super::quest_world_callbacks::local(expected),
        "relay guard landing while the switch is active or expired"
    );
    Ok(())
}
