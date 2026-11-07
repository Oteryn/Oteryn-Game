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

#[test]
fn overlay_snapshot_clears_stale_markers_and_keeps_later_deltas() -> Result<(), BatchError> {
    for step_between in [false, true] {
        let mut view = view()?;
        let (link, _commands, outbox) = play_channel();
        outbox.push(vec![SessionEvent::WorldObjectOverlay(AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: entry(key(103, 200, 0, 0)),
        })]);
        if step_between {
            outbox.stepped(moved_to(100, 200, 0));
        }
        outbox.push(vec![
            SessionEvent::WorldObjectOverlaySnapshot(AppliedDelta {
                server_sequence: 2,
                base_revision: 2,
                new_revision: 9,
                value: vec![],
            }),
            SessionEvent::WorldObjectOverlay(AppliedDelta {
                server_sequence: 3,
                base_revision: 9,
                new_revision: 10,
                value: entry(key(104, 200, 0, 0)),
            }),
        ]);
        view.tick(&link).map_err(|_| BatchError::InvalidView)?;
        assert_eq!(view.marker_count(), 1);
        assert!(view.markers.contains_key(&key(104, 200, 0, 0)));
        assert!(!view.markers.contains_key(&key(102, 199, 0, 0)));
        assert!(!view.markers.contains_key(&key(103, 200, 0, 0)));
    }
    Ok(())
}
