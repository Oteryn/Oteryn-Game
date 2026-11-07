//! Test-authored walkable corridor for near/far reward-chest checks.
use super::*;

pub(super) fn bundle_with_reward_corridor(
    chest: (&str, u32),
    gold: (&str, u32),
) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(chest, gold);
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let sector = bundle
        .sectors
        .iter_mut()
        .find(|sector| sector.floor == -7 && sector.sx == 0 && sector.sy == 0)
        .ok_or("W03 fixture corridor sector")?;
    let grass = sector
        .tiles
        .iter()
        .find(|tile| tile.x == 2 && tile.y == 0)
        .cloned()
        .ok_or("W03 fixture corridor grass")?;
    // The original row has a wall at x=5 and the static chest at x=6. Extend only
    // this test map with (7,1)/(7,2), so near/far assertions use actual cardinal
    // movement without walking through either obstruction or reinitializing an
    // existing actor. At (7,2) the chest at (6,0) is beyond Chebyshev reach one.
    for y in [1, 2] {
        assert!(!sector.tiles.iter().any(|tile| tile.x == 7 && tile.y == y));
        let mut tile = grass.clone();
        tile.x = 7;
        tile.y = y;
        sector.tiles.push(tile);
    }
    sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
    let bytes = oteryn_world_bundle::writer::write_with_npcs(
        &bundle.manifest,
        &bundle.sectors,
        &bundle.spawns,
        &bundle.npcs,
    )
    .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((bytes, pins))
}

pub(super) async fn walk_corridor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    ys: [i32; 3],
) -> TestResult {
    let _permit = owner
        .spell_lane_permit()
        .await
        .ok_or("W03 fixture movement lane")?;
    let actor = admitted.runtime_actor.ok_or("W03 fixture movement actor")?;
    let at = |y| MovementLocalPosition { x: 7, y, floor: 7 };
    let mut runtime = owner.runtime.lock().await;
    let mut current = runtime
        .borrow_movement_position()
        .read(actor)
        .map_err(debug)?;
    assert_eq!(current.position(), at(ys[0]));
    for y in ys.into_iter().skip(1) {
        let next = at(y);
        assert_eq!(
            owner.movement_cells.index().lookup(
                owner.movement_cells.scope(),
                crate::content::LogicalCell {
                    x: next.x,
                    y: next.y,
                    z: i32::from(next.floor)
                },
            ),
            Ok(crate::content::CollisionClass::Walkable),
            "every fixture step must use the actual map's collision facts"
        );
        assert!(
            !runtime
                .position_occupied_by_other(actor, next)
                .map_err(debug)?
        );
        current = runtime
            .borrow_movement_position()
            .commit_cardinal(current, next)
            .map_err(debug)?;
        assert_eq!(current.position(), next);
    }
    Ok(())
}
