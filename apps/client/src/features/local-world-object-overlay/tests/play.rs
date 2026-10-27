use super::*;

#[test]
fn overlay_delta_adds_a_marker() -> Result<(), BatchError> {
    let mut view = view()?;
    let mut outcome = moved_to(100, 199, 0);
    outcome.world_object_overlay_delta = Some(AppliedDelta {
        server_sequence: 3,
        base_revision: 1,
        new_revision: 2,
        value: entry(key(101, 199, 0, 1)),
    });
    view.apply(&outcome)?;
    assert_eq!(view.marker_count(), 2);
    Ok(())
}

#[test]
fn markers_are_refiltered_when_the_floor_changes() -> Result<(), BatchError> {
    let mut view = view()?;
    assert_eq!(view.marker_count(), 1);
    view.apply(&moved_to(5, 5, -3))?;
    assert_eq!(view.marker_count(), 1);
    assert_eq!(view.scene().sprites().len(), 2);
    view.apply(&moved_to(100, 200, 0))?;
    assert_eq!(view.marker_count(), 1);
    view.apply(&moved_to(100, 200, -7))?;
    assert_eq!(view.marker_count(), 0);
    Ok(())
}

#[test]
fn the_entry_door_semantic_placement_draws_a_marker() -> Result<(), BatchError> {
    let door = entry(b"oteryn:cell/entry-door".to_vec());
    let view = PlayView::new(TileCoord::new(1, 0), 0, &[door])?;
    assert_eq!(view.marker_count(), 1);
    assert_eq!(view.scene().sprites().len(), 2);
    Ok(())
}
