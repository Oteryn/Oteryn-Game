use super::*;

#[test]
fn qoptic_mirrors_committed_diagonal_without_mirroring_floor_changes() {
    let from = MovementLocalPosition { x: 20, y: 30, floor: 14 };
    let to = MovementLocalPosition { x: 21, y: 29, ..from };
    assert_eq!(
        observer_eye::mirrored_direction(from, to),
        Some(CardinalStep::NorthEast)
    );
    assert_eq!(
        observer_eye::mirrored_direction(from, MovementLocalPosition { floor: 15, ..to }),
        None
    );
    assert_eq!(observer_eye::mirrored_direction(from, from), None);
}
