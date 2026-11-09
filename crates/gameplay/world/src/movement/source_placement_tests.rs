use super::MovementLocalPosition;
use super::source_placement::{SourcePlacementCell, SourcePlacementError, SourcePlacementPlan};

fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 15 }
}
fn tile(query_add: bool, protection_zone: bool, teleport: bool) -> SourcePlacementCell {
    SourcePlacementCell {
        exists: true,
        query_add,
        protection_zone,
        teleport,
    }
}

#[test]
fn srx3_source_placement_center_force_fallback_and_current_occupancy()
-> Result<(), SourcePlacementError> {
    let center = at(33558, 32759);
    let plan = SourcePlacementPlan::prepare(center, false, false, |_| 0)?;
    assert_eq!(plan.candidates().len(), 9);
    let first = plan.candidates()[1];
    let second = plan.candidates()[2];
    let mut occupied = vec![center];
    let select = || {
        plan.select(
            |p| Some(tile(!occupied.contains(&p), false, false)),
            |_, _| Some(true),
        )
    };
    assert_eq!(select()?, Some(first));
    occupied.push(first);
    assert_eq!(
        plan.select(
            |p| Some(tile(!occupied.contains(&p), false, false)),
            |_, _| Some(true)
        )?,
        Some(second)
    );
    let forced = SourcePlacementPlan::prepare(center, false, true, |_| 0)?;
    assert_eq!(
        forced.select(|_| Some(tile(false, true, true)), |_, _| Some(false))?,
        Some(center)
    );
    assert_ne!(
        forced.select(
            |p| Some(if p == center {
                SourcePlacementCell {
                    exists: false,
                    ..tile(false, false, false)
                }
            } else {
                tile(true, false, false)
            }),
            |_, _| Some(true)
        )?,
        Some(center)
    );
    Ok(())
}

#[test]
fn srx3_source_placement_pz_teleport_extended_sight_and_unknown_are_distinct()
-> Result<(), SourcePlacementError> {
    let center = at(33558, 32759);
    let plan = SourcePlacementPlan::prepare(center, true, false, |_| 0)?;
    assert_eq!(plan.candidates().len(), 13);
    let candidate = plan.candidates()[1];
    let choose = |center_pz, candidate_pz, teleport, sight| {
        plan.select(
            |p| {
                Some(if p == center {
                    tile(false, center_pz, false)
                } else if p == candidate {
                    tile(true, candidate_pz, teleport)
                } else {
                    tile(false, false, false)
                })
            },
            |_, _| Some(sight),
        )
    };
    assert_eq!(choose(false, true, false, true)?, Some(candidate));
    assert_eq!(choose(true, false, false, true)?, None);
    assert_eq!(choose(false, false, true, true)?, None);
    assert_eq!(choose(false, false, false, false)?, None);
    assert!(plan.select(|_| None, |_, _| Some(true)).is_err());
    assert!(
        plan.select(|p| Some(tile(p != center, false, false)), |_, _| None)
            .is_err()
    );
    assert!(SourcePlacementPlan::prepare(center, true, false, |bound| bound).is_err());
    Ok(())
}
