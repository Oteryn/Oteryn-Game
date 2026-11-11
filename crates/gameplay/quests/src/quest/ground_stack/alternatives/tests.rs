use super::*;
use crate::quest::ground_stack::dense_pattern_tests::pattern;

fn variants() -> (
    Vec<GroundStackRequirement>,
    Vec<GroundStackRequirement>,
    Vec<GroundStackSnapshot>,
) {
    let (mut primary, observations) = pattern();
    for cell in &mut primary {
        if cell.items[0].item == "oteryn:item.tibia.i900" {
            cell.items[0].item = "oteryn:item.tibia.i2397".into();
        }
    }
    let (alternative, _) = pattern();
    (primary, alternative, observations)
}

#[test]
fn m07_ground_patterns_require_one_complete_pillow_variant() -> Result<(), GroundStackError> {
    let (primary, alternative, mut observed) = variants();
    let matched = match_patterns(&primary, &alternative, &observed)?
        .ok_or(GroundStackError::InvalidObservation)?;
    assert_eq!(matched.variant, PatternVariant::Alternative);
    assert_eq!(matched.cells.len(), 36);
    assert!(std::ptr::eq(
        matched.cells[35].entries,
        observed[35].entries.as_slice()
    ));
    for cell in &mut observed {
        if cell.entries[0].item == "oteryn:item.tibia.i900" {
            cell.entries[0].item = "oteryn:item.tibia.i2397".into();
        }
    }
    assert_eq!(
        match_patterns(&primary, &alternative, &observed)?
            .ok_or(GroundStackError::InvalidObservation)?
            .variant,
        PatternVariant::Primary
    );
    observed[35].entries[0].item = "oteryn:item.tibia.i900".into();
    assert!(
        match_patterns(&primary, &alternative, &observed)?.is_none(),
        "the nine yellow cells must agree on one source branch"
    );
    Ok(())
}

#[test]
fn m07_ground_patterns_validate_both_plans_before_accepting_either() -> Result<(), GroundStackError>
{
    let (primary, alternative, observed) = variants();
    let mut invalid = primary.clone();
    invalid[35].position = invalid[34].position;
    assert!(match_patterns(&alternative, &invalid, &observed).is_err());
    let mut invalid = primary.clone();
    invalid[35].items[0].revision.clear();
    assert!(match_patterns(&alternative, &invalid, &observed).is_err());
    assert!(
        match_patterns(&alternative, &alternative, &observed).is_err(),
        "indistinguishable alternatives cannot identify a branch"
    );
    let mut reordered = alternative.clone();
    reordered.reverse();
    assert!(match_patterns(&alternative, &reordered, &observed).is_err());
    let mut wrong_floor = primary.clone();
    for cell in &mut wrong_floor {
        cell.position.floor = -8;
    }
    assert!(match_patterns(&alternative, &wrong_floor, &observed).is_err());
    let mut reused = observed.clone();
    reused[35].entries[0].instance = reused[34].entries[0].instance;
    assert_eq!(
        match_patterns(&primary, &alternative, &reused),
        Err(GroundStackError::InvalidObservation)
    );
    assert!(match_patterns(&primary, &alternative, &observed[..35])?.is_none());
    Ok(())
}
