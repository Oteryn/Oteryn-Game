//! Actual Fibula tunnel: ordinary floor steps plus three pinned source ladder USEs.
use super::*;
pub(super) async fn traverse(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: &mut u64,
) -> TestResult {
    // Crystal201189 ladder_up.lua5–20 and position.lua22–49. Each independently
    // qualified default SOUTH destination is walkable in the untouched source B3.
    for (approach, ladder, destination) in [
        ((32209, 32370, 10), (32208, 32370, 10), (32208, 32371, 9)),
        ((32199, 32368, 9), (32198, 32367, 9), (32198, 32368, 8)),
        ((32180, 32368, 8), (32179, 32367, 8), (32179, 32368, 7)),
    ] {
        walk::walk(
            owner,
            admitted,
            crate::foundation::MovementLocalPosition {
                x: approach.0,
                y: approach.1,
                floor: approach.2,
            },
        )
        .await?;
        let map = owner.bundle_map().ok_or("actual Fibula tunnel map")?;
        let at = crate::map::overlay::TilePos {
            x: ladder.0,
            y: ladder.1,
            floor: -i8::try_from(ladder.2)?,
        };
        let key = crate::map::view::placement_key(at, 1).ok_or("actual ladder ordinal")?;
        let tile = map
            .overlay
            .base()
            .tile(at.x, at.y, at.floor)
            .ok_or("actual ladder source cell")?;
        assert_eq!(
            tile.depths(),
            &[0, 0],
            "exported ladder is exact ground+1948"
        );
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement,
                expected_revision: 0,
            },
        )
        .map_err(debug)?;
        let (next, result, _) =
            super::super::super::item_craft::exchange(owner, *admitted, *command, &payload, None)
                .await?;
        *admitted = next;
        *command = command.checked_add(1).ok_or("ladder command overflow")?;
        assert_eq!(
            result,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
            "actual source USE1948 at {ladder:?} must ascend through the production movement owner"
        );
        let actual = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actual tunnel actor")?)
            .map_err(debug)?
            .position();
        assert_eq!(
            (actual.x, actual.y, actual.floor),
            destination,
            "source preferred SOUTH landing, no test relocation"
        );
    }
    walk::walk(
        owner,
        admitted,
        crate::foundation::MovementLocalPosition {
            x: 32186,
            y: 32472,
            floor: 7,
        },
    )
    .await
}
