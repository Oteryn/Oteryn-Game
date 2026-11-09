//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8, OTS_HYPOTHESIS_ONLY:
//! data-global/scripts/quests/children_of_the_revolution/actions_mission4_levers_puzzle.lua.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{children_puzzle, world_callbacks::Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const AT: Position = Position {
    x: 33349,
    y: 31124,
    floor: -5,
};

#[test]
fn m44_native_children_readset_reset_manual_close_reopen_and_stale_timer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m44-children-lifecycle", || {
        super::quest_world_callbacks::scenario("qn1-children-lever-1/ordinary", AT, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let mut expected = vec![ExpectedTile {
        position: (AT.x, AT.y),
        ids: vec![9929, 9125],
        counted_entry: None,
    }];
    expected.extend(
        children_puzzle::CELLS
            .iter()
            .zip(children_puzzle::PAIRS)
            .map(|(at, pair)| ExpectedTile {
                position: (at.x, at.y),
                ids: vec![u32::from(pair[0])],
                counted_entry: None,
            }),
    );
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z05-x130-y121.b3"
            )),
            region: (5, 130, 121),
            bounds: (33349, 31123, 33356, 31127),
            start: (33350, 31124),
            expected,
        }],
        &[],
        &[9126, 9933, 9934, 9937, 9938],
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("lever key")?;
    let payload = |revision| -> TestResult<Vec<u8>> {
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: revision,
        })
        .map_err(debug)
    };
    let read = |at: Position, ordinal: u8| -> TestResult<u16> {
        let base = map
            .overlay
            .base()
            .tile(at.x, at.y, at.floor)
            .ok_or("source tile")?;
        let id = *base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))
            .ok_or("source ordinal")?
            .0;
        Ok(map
            .facts
            .base_entry(
                TilePos {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
                ordinal,
                id,
            )
            .ok_or("source facts")?
            .appearance_id)
    };
    let cells = || -> TestResult<Vec<u16>> {
        children_puzzle::CELLS
            .iter()
            .map(|at| read(*at, 0))
            .collect()
    };
    let original = children_puzzle::PAIRS.map(|pair| pair[0]);
    let toggled = children_puzzle::toggle(1, original).ok_or("source toggle")?;
    assert_eq!(cells()?, original);
    let _clock = owner.freeze_owner_clock_for_test();
    let use_payload = payload(0)?;
    let (result, first_due) = super::m44_source_lifecycle::commit_through_barrier(
        owner,
        admitted,
        h,
        use_payload.clone(),
        5_000_000,
        4_444_002,
    )
    .await?;
    assert_eq!(read(AT, 1)?, 9126);
    assert_eq!(cells()?, toggled);
    let replay = super::quest_world_callbacks::wire(owner, admitted, 1, 2, use_payload).await?;
    assert_eq!(result, replay);
    assert_eq!(cells()?, toggled);
    for (command, revision, now, expected) in [(2, 1, 7_000_000, 9125), (3, 2, 8_000_000, 9126)] {
        owner
            .states
            .test_owner_time
            .store(now + 1, std::sync::atomic::Ordering::Relaxed);
        super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload(revision)?)
                .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(read(AT, 1)?, expected);
        assert_eq!(cells()?, if command == 2 { original } else { toggled });
    }
    // The first arm belongs to revision 1; close/reopen published revisions 2/3.
    assert_eq!(map.facts.expire_movement_items(first_due), 0);
    assert_eq!(map.facts.object_revision(key), 3);
    assert_eq!(read(AT, 1)?, 9126);
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("new source timer")?;
    assert_eq!(due, 13_000_000);
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(map.facts.object_revision(key), 4);
    assert_eq!(read(AT, 1)?, 9125);
    assert_eq!(
        cells()?,
        toggled,
        "lever expiry never reverts the puzzle cells"
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 3);
    eprintln!(
        "POSTGRES_CASE_COMPLETED M44 children_native_readset commit_barrier replay close reopen stale_timer exact_expiry cells_preserved"
    );
    Ok(())
}
