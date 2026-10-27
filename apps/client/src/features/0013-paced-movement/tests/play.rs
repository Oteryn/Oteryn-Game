use super::*;

#[test]
fn arrow_key_steps_without_a_goal() -> Result<(), BatchError> {
    let mut view = view()?;
    assert_eq!(view.facing(), StepDir::South);
    view.arrow(StepDir::North);
    assert_eq!(view.next_step(), Some(StepDir::North));
    assert_eq!(view.next_step(), None);
    // The player turns to the step sent, moved or not.
    assert_eq!(view.facing(), StepDir::North);
    Ok(())
}

#[test]
fn sending_a_step_redraws_the_new_facing() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(view.own())?;
    let target = view.scene().target();
    assert!(target.is_some());
    view.arrow(StepDir::North);
    assert_eq!(view.send_step()?, Some(StepDir::North));
    assert_eq!(view.scene().facing(), StepDir::North);
    // Turning keeps the selection.
    assert_eq!(view.scene().target(), target);
    Ok(())
}

#[test]
fn step_outcome_moves_the_view_and_a_refusal_ends_the_walk() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(view.next_step(), Some(StepDir::West));
    view.apply(&moved_to(99, 200, 0))?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    assert_eq!(view.scene().view().origin(), TileCoord::new(92, 195));
    assert_eq!(view.next_step(), Some(StepDir::West));
    let mut blocked = moved_to(99, 200, 0);
    blocked.disposition = StepDisposition::Blocked;
    blocked.world_spatial_delta = None;
    view.apply(&blocked)?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    assert_eq!(view.next_step(), None);
    Ok(())
}

#[test]
fn one_step_is_in_flight_until_its_outcome() -> Result<(), BatchError> {
    let mut view = view()?;
    view.arrow(StepDir::North);
    assert_eq!(view.next_step(), Some(StepDir::North));
    view.arrow(StepDir::East);
    assert_eq!(view.next_step(), None);
    view.apply(&moved_to(100, 199, 0))?;
    assert_eq!(view.next_step(), Some(StepDir::East));
    Ok(())
}
