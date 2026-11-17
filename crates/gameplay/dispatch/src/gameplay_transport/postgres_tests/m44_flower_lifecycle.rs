//! Crystal 201189f actions_lions_rock.lua:82-96, OTS_HYPOTHESIS_ONLY.
//! Real USE couples each native regrowth revision to one flower MINT.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    self,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const AT: Position = Position {
    x: 33138,
    y: 32352,
    floor: -5,
};
const CELL: TilePos = TilePos {
    x: 33138,
    y: 32352,
    floor: -5,
};
#[test]
fn m44_native_flower_initial_harvest_and_reharvest_after_exact_regrowth() -> TestResult {
    case("initial")
}
#[test]
fn m44_native_flower_reharvest_preserves_higher_quest_progress() -> TestResult {
    case("later")
}
#[test]
fn m44_native_flower_unknown_commit_reserves_source_and_reconciles_one_grant() -> TestResult {
    case("uncertain")
}
fn case(phase: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m44-flower", move || {
        super::quest_world_callbacks::scenario(&format!("m44-lions-flower-0/{phase}"), AT, false)
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
                "/../../../content/world/placements/region-z05-x129-y126.b3"
            )),
            region: (5, 129, 126),
            bounds: (33138, 32352, 33140, 32353),
            start: (33139, 32352),
            expected: vec![ExpectedTile {
                position: (AT.x, AT.y),
                ids: vec![4559, 21388],
                counted_entry: None,
            }],
        }],
        &[],
        &[21935],
    )
}
pub(super) fn catalogue(
    original: &quest::QuestStateCatalogue,
    callback: &Callback,
    later: bool,
) -> TestResult<quest::QuestStateCatalogue> {
    let tracks = callback
        .storage
        .iter()
        .map(|state| {
            let mut track = original
                .track(&state.track)
                .cloned()
                .ok_or("flower track")?;
            track.initial = if later && state.track.ends_with("/lions_rock/questline") {
                7
            } else {
                0
            };
            Ok(track)
        })
        .collect::<Result<Vec<_>, &str>>()?;
    quest::QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .ok_or("flower transition")?
                .clone(),
        ],
    )
    .map_err(debug)
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    later: bool,
    uncertain: bool,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("flower key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = |expected_revision| {
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision,
        })
        .map_err(debug)
    };
    let appearance = || -> TestResult<u16> {
        let tile = map
            .overlay
            .base()
            .tile(AT.x, AT.y, AT.floor)
            .ok_or("flower tile")?;
        let id = *tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(1)
            .ok_or("flower ordinal")?
            .0;
        Ok(map
            .facts
            .base_entry(CELL, 1, id)
            .ok_or("flower facts")?
            .appearance_id)
    };
    let _clock = owner.freeze_owner_clock_for_test();
    owner
        .states
        .test_owner_time
        .store(5_000_001, std::sync::atomic::Ordering::Relaxed);
    if uncertain {
        use crate::durability::quest_state::QuestCause;
        use crate::foundation::{CommandId, CommandRef};
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(
            super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload(0)?)
                .await
                .is_err()
        );
        assert!(owner.has_pending_quest_callback(admitted.game_session_id));
        assert_eq!(state(h).await?, (1, 1, 1, 1));
        assert_eq!(appearance()?, 21388);
        assert_eq!(map.facts.object_revision(key), 0);
        assert_eq!(map.facts.next_movement_decay_time(), None);
        let actor = admitted.runtime_actor.ok_or("flower actor")?;
        let cause = |n| -> TestResult<_> {
            Ok(QuestCause::Use(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(n).map_err(debug)?,
            )))
        };
        assert!(owner.states.quest_callbacks.source_target_available(
            actor,
            admitted.game_session_id,
            &cause(1)?,
            key,
            key
        ));
        assert!(!owner.states.quest_callbacks.source_target_available(
            actor,
            admitted.game_session_id,
            &cause(2)?,
            key,
            key
        ));
        owner
            .states
            .test_owner_time
            .store(6_000_001, std::sync::atomic::Ordering::Relaxed);
        assert!(
            owner
                .reconcile_quest_callbacks(admitted.game_session_id)
                .await
        );
        assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
        assert_eq!(state(h).await?, (1, 1, 1, 1));
        assert_eq!(appearance()?, 21935);
        assert_eq!(map.facts.object_revision(key), 1);
        assert_eq!(map.facts.next_movement_decay_time(), Some(66_000_000));
        assert_eq!(map.facts.expire_movement_items(65_999_999), 0);
        assert_eq!(map.facts.expire_movement_items(66_000_000), 1);
        assert_eq!(appearance()?, 21388);
        eprintln!(
            "POSTGRES_CASE_COMPLETED M44 native_flower unknown_commit source_reserved reconciled_one_grant"
        );
        return Ok(());
    }
    let (reply, due) = super::m44_source_lifecycle::commit_through_barrier(
        owner,
        admitted,
        h,
        payload(0)?,
        60_000_000,
        if later { 4_444_006 } else { 4_444_005 },
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(appearance()?, 21935);
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload(0)?).await?,
        reply
    );
    super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
    let busy = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(1)?).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&busy).map_err(debug)?,
        UseDisposition::NothingToUse
    );
    assert_eq!(state(h).await?, (1, if later { 7 } else { 1 }, 1, 1));
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(appearance()?, 21388);
    assert_eq!(map.facts.object_revision(key), 2);
    owner
        .states
        .test_owner_time
        .store(due + 1_000_001, std::sync::atomic::Ordering::Relaxed);
    super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
    let again = super::quest_world_callbacks::wire(owner, admitted, 3, 2, payload(2)?).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&again).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(appearance()?, 21935);
    assert_eq!(map.facts.object_revision(key), 3);
    assert_eq!(map.facts.next_movement_decay_time(), Some(due + 61_000_000));
    assert_eq!(state(h).await?, (2, if later { 7 } else { 1 }, 1, 2));
    eprintln!(
        "POSTGRES_CASE_COMPLETED M44 native_flower USE exact_regrowth reharvest progress_preserved later={later}"
    );
    Ok(())
}
async fn state(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u10_70/lions_rock/questline'),(SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u8_1/tibia_tales/default_start'),(SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i21389' AND lifecycle=1)").fetch_one(&h.pool).await?)
}
