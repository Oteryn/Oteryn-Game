use super::*;
use crate::quest::ground_stack::dense_pattern_tests::pattern;

fn line(
    axis: ShiftAxis,
) -> (
    Position,
    Vec<GroundStackRequirement>,
    Vec<GroundStackSnapshot>,
) {
    let (required, observed) = pattern();
    let keep = |p: Position| match axis {
        ShiftAxis::Row => p.y == 32334,
        ShiftAxis::Column => p.x == 32818,
    };
    let control = match axis {
        ShiftAxis::Row => Position {
            x: 32816,
            y: 32334,
            floor: -9,
        },
        ShiftAxis::Column => Position {
            x: 32818,
            y: 32332,
            floor: -9,
        },
    };
    (
        control,
        required.into_iter().filter(|r| keep(r.position)).collect(),
        observed.into_iter().filter(|s| keep(s.position)).collect(),
    )
}

#[test]
fn m07_ground_rotation_preserves_six_original_identities_and_wraps_each_axis()
-> Result<(), GroundStackError> {
    for axis in [ShiftAxis::Row, ShiftAxis::Column] {
        let (control, mut required, mut observed) = line(axis);
        let plan = plan_rotation(axis, control, &required, &observed)?
            .ok_or(GroundStackError::InvalidObservation)?;
        assert_eq!(plan.moves.len(), 6);
        assert_eq!(plan.moves[0].source, required[5].position);
        assert_eq!(plan.moves[0].destination, required[0].position);
        assert_eq!(plan.moves[5].destination, required[1].position);
        assert!(std::ptr::eq(plan.moves[0].item, &observed[5].entries[0]));
        let ids: std::collections::BTreeSet<_> =
            plan.moves.iter().map(|m| m.item.instance).collect();
        assert_eq!(ids.len(), 6);
        required.reverse();
        observed.reverse();
        let reordered = plan_rotation(axis, control, &required, &observed)?
            .ok_or(GroundStackError::InvalidObservation)?;
        assert_eq!(
            reordered.moves[0].item.instance,
            observed[0].entries[0].instance
        );
    }
    Ok(())
}

#[test]
fn m07_ground_rotation_refuses_wrong_control_partial_lines_and_reused_roots()
-> Result<(), GroundStackError> {
    let (control, required, mut observed) = line(ShiftAxis::Row);
    let mut wrong = control;
    wrong.x += 1;
    assert!(plan_rotation(ShiftAxis::Row, wrong, &required, &observed).is_err());
    wrong.x = u16::MAX;
    assert!(plan_rotation(ShiftAxis::Row, wrong, &required, &observed).is_err());
    assert!(plan_rotation(ShiftAxis::Column, control, &required, &observed).is_err());
    assert!(plan_rotation(ShiftAxis::Row, control, &required[..5], &observed).is_err());
    assert!(plan_rotation(ShiftAxis::Row, control, &required, &observed[..5])?.is_none());
    let mut stacked = required.clone();
    stacked[0].items[0].quantity = 2;
    observed[0].entries[0].quantity = 2;
    assert!(plan_rotation(ShiftAxis::Row, control, &stacked, &observed).is_err());
    let (_, _, mut observed) = line(ShiftAxis::Row);
    observed[0].entries[0].instance = observed[1].entries[0].instance;
    assert_eq!(
        plan_rotation(ShiftAxis::Row, control, &required, &observed),
        Err(GroundStackError::InvalidObservation)
    );
    Ok(())
}
