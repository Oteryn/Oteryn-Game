use super::*;

pub(super) fn pattern() -> (Vec<GroundStackRequirement>, Vec<GroundStackSnapshot>) {
    let mut requirements = Vec::new();
    let mut snapshots = Vec::new();
    for row in 0_u8..6 {
        for column in 0_u8..6 {
            let position = Position {
                x: 32818 + u16::from(column),
                y: 32334 + u16::from(row),
                floor: -9,
            };
            let colors = [2394_u16, 2396, 2395, 900];
            let item = format!(
                "oteryn:item.tibia.i{}",
                colors[usize::from((row / 3) * 2 + column / 3)]
            );
            requirements.push(GroundStackRequirement {
                position,
                items: vec![StackItemRequirement {
                    item: item.clone(),
                    revision: "definition-r1".into(),
                    quantity: 1,
                }],
            });
            snapshots.push(GroundStackSnapshot {
                position,
                entries: vec![GroundStackEntry {
                    instance: [1 + row * 6 + column; 16],
                    item,
                    revision: "definition-r1".into(),
                    quantity: 1,
                    ordinal: 1,
                }],
            });
        }
    }
    (requirements, snapshots)
}

#[test]
fn m07_dense_ground_pattern_matches_all_36_typed_instance_witnesses() -> Result<(), GroundStackError>
{
    let (requirements, mut snapshots) = pattern();
    let witnesses =
        match_readset(&requirements, &snapshots)?.ok_or(GroundStackError::InvalidObservation)?;
    assert_eq!(witnesses.len(), 36);
    assert_eq!(witnesses[35].entries[0].instance, [36; 16]);
    snapshots.reverse();
    assert!(
        match_readset(&requirements, &snapshots)?.is_some(),
        "tile enumeration order does not replace custody identity"
    );
    snapshots[0].entries[0].item = "oteryn:item.wrong_pillow".into();
    assert!(match_readset(&requirements, &snapshots)?.is_none());
    Ok(())
}

#[test]
fn m07_dense_ground_pattern_refuses_holes_extra_items_and_reused_identity()
-> Result<(), GroundStackError> {
    let (requirements, mut snapshots) = pattern();
    snapshots.pop();
    assert!(
        match_readset(&requirements, &snapshots)?.is_none(),
        "a missing observation never completes the puzzle"
    );
    let (requirements, mut snapshots) = pattern();
    snapshots[0].entries[0].instance = snapshots[1].entries[0].instance;
    assert_eq!(
        match_readset(&requirements, &snapshots),
        Err(GroundStackError::InvalidObservation)
    );
    let (mut requirements, mut snapshots) = pattern();
    requirements[35].position.x += 1;
    snapshots[35].position.x += 1;
    assert!(
        match_readset(&requirements, &snapshots).is_err(),
        "large sparse readsets remain outside the bound"
    );
    let (requirements, mut snapshots) = pattern();
    let extra = snapshots[1].entries[0].clone();
    snapshots[0].entries.push(extra);
    assert!(
        match_readset(&requirements, &snapshots).is_err(),
        "the larger tile bound does not authorize larger stacks"
    );
    Ok(())
}
