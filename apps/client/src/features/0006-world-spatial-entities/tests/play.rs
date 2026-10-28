use super::*;

#[test]
fn snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers()
-> Result<(), BatchError> {
    let view = view()?;
    // Only the marker on the own floor and with a decodable placement is drawn.
    assert_eq!(view.marker_count(), 1);
    assert_eq!(view.markers.len(), 2);
    let scene = view.scene();
    assert_eq!(scene.sprites().len(), 2);
    let [x, y] = scene.view().tile_to_screen(TileCoord::new(100, 200));
    assert_eq!(
        scene.view().screen_to_tile(x + 1.0, y + 1.0),
        Some(TileCoord::new(100, 200))
    );
    assert_eq!(scene.view().origin(), TileCoord::new(93, 195));
    Ok(())
}

#[test]
fn generated_dispatch_keeps_spatial_event_order_and_ignores_undrawn_domains() -> Result<(), String>
{
    let position = ActorPosition {
        x: 103,
        y: 201,
        floor: 0,
    };
    let entities = SessionEvent::WorldSpatialEntities(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::WorldSpatialEntitiesDelta {
            content_generation: [0; 32],
            actor_position: position,
            enter: Vec::new(),
            update: Vec::new(),
            leave: Vec::new(),
        },
    });
    let ignored = SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: 3,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::ActorVitals::default(),
    });
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![spatial_event(1, 50), entities, ignored]);
    let Some(PlayEvent::Pushed(pushed)) = link.poll() else {
        return Err("missing pushed event".to_owned());
    };
    assert_eq!(pushed.own, Some(position));
    let mut view = view().map_err(|e| e.to_string())?;
    view.apply_pushed(&pushed).map_err(|e| e.to_string())?;
    assert_eq!(view.own(), TileCoord::new(103, 201));
    outbox.push(vec![spatial_event(4, 51)]);
    let Some(PlayEvent::Pushed(pushed)) = link.poll() else {
        return Err("missing pushed event".to_owned());
    };
    assert_eq!(pushed.own.map(|p| p.x), Some(51));
    Ok(())
}
