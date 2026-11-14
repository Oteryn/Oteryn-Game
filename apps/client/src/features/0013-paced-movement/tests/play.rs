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
fn arrow_step_outcome_moves_the_view_and_refusal_keeps_the_position() -> Result<(), BatchError> {
    let mut view = view()?;
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    view.apply(&moved_to(99, 200, 0))?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    assert_eq!(view.scene().view().origin(), TileCoord::new(92, 195));
    view.arrow(StepDir::West);
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

fn tile_px(view: &PlayView) -> f32 {
    view.scene().view().tile_px() as f32
}

#[test]
fn a_moved_step_scrolls_the_camera_from_the_old_tile_to_rest() -> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    view.apply(&moved_to(99, 200, 0))?;
    let start = view.walk_started().ok_or("a glide begins")?;
    // The view is already centred on the new tile; the map still sits one tile to the left.
    assert_eq!(view.camera_offset(start), Some([-tile_px(&view), 0.0]));
    let halfway = start + view.walk_duration().ok_or("a duration")? / 2;
    assert_eq!(view.camera_offset(halfway), Some([-tile_px(&view) / 2.0, 0.0]));
    let done = start + view.walk_duration().ok_or("a duration")?;
    assert_eq!(view.camera_offset(done), None);
    Ok(())
}

#[test]
fn a_diagonal_step_scrolls_the_camera_diagonally() -> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    view.arrow(StepDir::NorthWest);
    assert_eq!(view.next_step(), Some(StepDir::NorthWest));
    view.apply(&moved_to(99, 199, 0))?;
    let start = view.walk_started().ok_or("a glide begins")?;
    let tile = tile_px(&view);
    assert_eq!(view.camera_offset(start), Some([-tile, -tile]));
    Ok(())
}

#[test]
fn a_jump_a_floor_change_and_a_refusal_snap_with_no_scroll() -> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    view.apply(&moved_to(90, 200, 0))?;
    assert_eq!(view.walk_started(), None);
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    view.apply(&moved_to(89, 200, 1))?;
    assert_eq!(view.walk_started(), None);
    view.arrow(StepDir::West);
    assert_eq!(view.next_step(), Some(StepDir::West));
    let mut blocked = moved_to(89, 200, 1);
    blocked.disposition = StepDisposition::Blocked;
    blocked.world_spatial_delta = None;
    view.apply(&blocked)?;
    assert_eq!(view.walk_started(), None);
    Ok(())
}

#[test]
fn only_the_player_quads_stay_put_while_the_camera_scrolls() -> Result<(), Box<dyn std::error::Error>> {
    let view = view()?;
    let own = view.scene().player_quads();
    assert!(own.iter().any(|range| !range.is_empty()));
    let quads = view.scene().sprites().instances().len();
    assert!(own.iter().all(|range| range.end <= quads));
    Ok(())
}
