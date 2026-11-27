use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const LEVER: Position = Position {
    x: 32182,
    y: 32145,
    floor: -11,
};
pub(super) const START: Position = Position {
    x: LEVER.x,
    y: LEVER.y + 1,
    floor: LEVER.floor,
};
const DOOR: TilePos = TilePos {
    x: 32177,
    y: 32148,
    floor: -11,
};

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z11-x125-y125.b3"
            )),
            region: (11, 125, 125),
            bounds: (32176, 32144, 32183, 32150),
            start: (START.x, START.y),
            expected: Vec::new(),
        }],
        &[],
        &[5107, 5108, 2773],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    let key = crate::map::view::placement_key(
        TilePos {
            x: LEVER.x,
            y: LEVER.y,
            floor: LEVER.floor,
        },
        1,
    )
    .ok_or("Katana lever placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    assert!(!world.enterable(DOOR));
    for (revision, open) in [(0, true), (1, false), (2, true)] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let command = admitted.continuity.next_command_id;
        let result = super::quest_world_callbacks_fixtures::wire(
            owner,
            admitted,
            command,
            2,
            encode_use_intent(&WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: revision,
            })
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(world.enterable(DOOR), open);
    }
    Ok(())
}
