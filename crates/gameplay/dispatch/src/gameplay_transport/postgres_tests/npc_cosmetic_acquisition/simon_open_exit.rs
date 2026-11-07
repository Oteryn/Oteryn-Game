//! Exact existing Hugo shop door USE, followed by mature ordinary floor movement.
use super::*;
pub(super) async fn open_hugo_exit(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: &mut u64,
) -> TestResult {
    walk::walk(
        owner,
        admitted,
        crate::foundation::MovementLocalPosition {
            x: 32952,
            y: 32105,
            floor: 5,
        },
    )
    .await?;
    let map = owner.bundle_map().ok_or("actual Hugo source map")?;
    let tile = crate::map::overlay::TilePos {
        x: 32952,
        y: 32106,
        floor: -5,
    };
    let key = crate::map::view::placement_key(tile, 1).ok_or("actual shop door ordinal")?;
    assert!(
        map.facts.doors().contains(key),
        "source17561 is an existing native door"
    );
    let before = map.facts.doors().revision(key);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
        placement,
        expected_revision: before,
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let (next, result, _) =
        super::super::super::item_craft::exchange(owner, *admitted, *command, &payload, None)
            .await?;
    *admitted = next;
    *command = command.checked_add(1).ok_or("door command overflow")?;
    assert_eq!(
        result,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
        "actual closed17561 opens through the ordinary native USE owner"
    );
    assert_eq!(map.facts.doors().revision(key), before + 1);
    Ok(())
}
