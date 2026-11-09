//! Crystal 201189f actions_fire_bug.lua:110-128; source entrance lifecycle.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
const AT: Position = Position {
    x: 32849,
    y: 32233,
    floor: -9,
};
#[test]
fn m44_native_dreamer_retains_firebug_enters_portal_and_restores_ashes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m44-dreamer", || {
        super::quest_world_callbacks::scenario("m44-dreamer-entrance", AT, false)
    })
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z09-x128-y125.b3"
                )),
                region: (9, 128, 125),
                bounds: (32849, 32232, 32851, 32234),
                start: (32849, 32232),
                expected: vec![ExpectedTile {
                    position: (32849, 32233),
                    ids: vec![353, 3134],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z11-x128-y125.b3"
                )),
                region: (11, 128, 125),
                bounds: (32857, 32234, 32858, 32235),
                start: (32857, 32234),
                expected: vec![ExpectedTile {
                    position: (32857, 32234),
                    ids: vec![354],
                    counted_entry: None,
                }],
            },
        ],
        &[],
        &[1949],
    )
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("ashes key")?;
    let appearance = || -> TestResult<u16> {
        let pos = TilePos {
            x: AT.x,
            y: AT.y,
            floor: AT.floor,
        };
        let tile = map
            .overlay
            .base()
            .tile(AT.x, AT.y, AT.floor)
            .ok_or("ashes tile")?;
        let id = *tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, d)| **d == 0)
            .nth(1)
            .ok_or("ashes ordinal")?
            .0;
        Ok(map
            .facts
            .base_entry(pos, 1, id)
            .ok_or("ashes facts")?
            .appearance_id)
    };
    assert_eq!(appearance()?, 3134);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(map.facts.object_revision(key), 0);
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: AT,
            item: "oteryn:item.tibia.i5467".into(),
            count: 1,
        },
        70,
        true,
    )
    .await?;
    let _clock = owner.freeze_owner_clock_for_test();
    owner
        .states
        .test_owner_time
        .store(5_000_001, std::sync::atomic::Ordering::Relaxed);
    super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
    let mut blocker = h.pool.begin().await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i5467' AND lifecycle=1 FOR UPDATE").fetch_all(&mut *blocker).await?;
    let use_ = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload.clone());
    let release = async {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND query LIKE '%game_quest_callback_carried%')").bind(blocker_pid).fetch_one(&h.pool).await?;
                if waiting { break; }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            Ok::<(), sqlx::Error>(())
        }).await.map_err(debug)??;
        assert_eq!(map.facts.object_revision(key), 0);
        assert!(
            !map.facts
                .quest_portal_group_live("oteryn:quest-source/dreamer-entrance", 5_000_000)
        );
        owner
            .states
            .test_owner_time
            .store(6_000_001, std::sync::atomic::Ordering::Relaxed);
        blocker.rollback().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (reply, released) = tokio::join!(use_, release);
    released?;
    let reply = reply?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(appearance()?, 1949);
    let due = 306_000_000;
    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload).await?,
        reply
    );
    let south = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::South,
    );
    let step = super::quest_world_callbacks::wire(owner, admitted, 3, 1, south).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&step).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    let destination = Position {
        x: 32857,
        y: 32234,
        floor: -11,
    };
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(destination)
    );
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(map.facts.object_revision(key), 2);
    assert_eq!(appearance()?, 3134);
    assert!(
        !map.facts
            .quest_portal_group_live("oteryn:quest-source/dreamer-entrance", due)
    );
    let state:(i64,i64,i64) = sqlx::query_as("SELECT (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i5467' AND lifecycle=1),(SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&h.pool).await?;
    assert_eq!(state, (1, 0, 0));
    eprintln!(
        "POSTGRES_CASE_COMPLETED M44 native_dreamer retained_firebug portal_STEP exact_restore"
    );
    Ok(())
}
