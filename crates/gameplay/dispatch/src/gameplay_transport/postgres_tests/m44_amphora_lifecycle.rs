//! Crystal 201189f movements_lions_rock.lua:54-108, OTS_HYPOTHESIS_ONLY.
//! Literal STEP must couple the scroll and questline to the separate amphora.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const AT: Position = Position {
    x: 33119,
    y: 32246,
    floor: -9,
};
const AMPHORA: TilePos = TilePos {
    x: 33119,
    y: 32247,
    floor: -9,
};

#[test]
fn m44_native_amphora_step_mints_once_and_restores_only_its_fenced_revision() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m44-amphora", || {
        super::quest_world_callbacks::scenario("m44-lions-amphora-1", AT, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z09-x129-y125.b3"
            )),
            region: (9, 129, 125),
            bounds: (33117, 32244, 33122, 32249),
            start: (33119, 32245),
            expected: vec![
                ExpectedTile {
                    position: (AT.x, AT.y),
                    ids: vec![521, 6373],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (AMPHORA.x, AMPHORA.y),
                    ids: vec![499, 6385, 21945],
                    counted_entry: None,
                },
            ],
        }],
        &[],
        &[21946],
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let source =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 0).ok_or("ring key")?;
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(
        owner
            .callback_content()
            .iter()
            .filter(|callback| callback.trigger
                == oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn
                && crate::gameplay_transport::quest_callbacks::matches(map, callback, source))
            .count(),
        1,
        "the real native STEP tile must select exactly one source callback"
    );
    let _clock = owner.freeze_owner_clock_for_test();
    owner
        .states
        .test_owner_time
        .store(5_000_001, std::sync::atomic::Ordering::Relaxed);
    let south = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::South);
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1, south.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(owner.runtime.lock().await.read_actor_position(admitted.runtime_actor.ok_or("actor")?).map_err(debug)?.position(), super::quest_world_callbacks::local(AT));
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(AMPHORA.floor, AMPHORA.x, AMPHORA.y, 2)
        .ok_or("amphora key")?;
    let appearance = || -> TestResult<u16> {
        let base = map
            .overlay
            .base()
            .tile(AMPHORA.x, AMPHORA.y, AMPHORA.floor)
            .ok_or("amphora tile")?;
        let id = *base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(2)
            .ok_or("amphora ordinal")?
            .0;
        Ok(map
            .facts
            .base_entry(AMPHORA, 2, id)
            .ok_or("amphora facts")?
            .appearance_id)
    };
    assert_eq!(appearance()?, 21946);
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.next_movement_decay_time(), Some(20_000_000));
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, 1, 1, south.clone()).await?,
        reply
    );
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.expire_movement_items(19_999_999), 0);
    assert_eq!(map.facts.expire_movement_items(20_000_000), 1);
    assert_eq!(appearance()?, 21945);
    assert_eq!(map.facts.object_revision(key), 2);
    for (command, at, direction) in [
        (2, 21_000_000, StepDirection::North),
        (3, 31_000_000, StepDirection::South),
    ] {
        owner
            .states
            .test_owner_time
            .store(at + 1, std::sync::atomic::Ordering::Relaxed);
        let result = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&result).map_err(debug)?,
            StepDisposition::Moved
        );
    }
    assert_eq!(
        appearance()?,
        21945,
        "completed personal trial cannot break another shared amphora"
    );
    assert_eq!(map.facts.object_revision(key), 2);
    let state: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u10_70/lions_rock/questline'),(SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i21467' AND lifecycle=1)")
        .fetch_one(&h.pool).await?;
    assert_eq!(state, (1, 5, 1));
    eprintln!(
        "POSTGRES_CASE_COMPLETED M44 native_amphora STEP scroll_atomic replay exact_reset no_repeat"
    );
    Ok(())
}
