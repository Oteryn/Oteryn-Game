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

#[test]
fn live_names_reach_scene_and_disappear_with_visibility() -> Result<(), BatchError> {
    let mut view = view()?;
    let entity = oteryn_session::WorldSpatialEntity {
        display_name: Some(
            oteryn_session::EntityDisplayName::new("Rat").map_err(|_| BatchError::InvalidView)?,
        ),
        kind: oteryn_session::EntityKind::Creature,
        entity: oteryn_session::EntityRef {
            identity: [8; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    view.set_presentations(vec![oteryn_session::VisibleAppearance {
        light: None,
        entity,
        look: None,
    }])?;
    assert_eq!(view.scene().name_count(), 1);
    view.set_presentations(Vec::new())?;
    assert_eq!(view.scene().name_count(), 0);
    Ok(())
}

#[test]
fn own_identity_draws_once_between_step_and_appearance_publish() -> Result<(), BatchError> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    view.set_own_identity([1; 16]);
    let own = oteryn_session::VisibleAppearance {
        light: None,
        entity: oteryn_session::WorldSpatialEntity {
            display_name: None,
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
    };
    let mut other = own.clone();
    other.entity.entity.identity = [8; 16];
    other.entity.kind = oteryn_session::EntityKind::Creature;
    other.look = Some(oteryn_session::Look::EmptyOutfit);
    view.set_presentations(vec![own.clone(), other.clone()])?;
    assert_eq!(view.scene().sprites().len(), 1);
    view.apply(&moved_to(101, 200, 0))?;
    assert_eq!(
        view.scene().sprites().len(),
        1,
        "stale own position must not leave a ghost"
    );
    assert_eq!(
        view.scene().sprites().instances()[0].position,
        view.scene().view().tile_to_screen(TileCoord::new(101, 200))
    );
    let mut updated = own;
    updated.entity.position.x = 101;
    view.set_presentations(vec![updated, other])?;
    assert_eq!(view.scene().sprites().len(), 1);
    Ok(())
}

#[test]
fn live_presentations_keep_target_clock_and_light_through_rebuilds() -> Result<(), BatchError> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let entity = WorldSpatialEntity {
        display_name: Some(
            oteryn_session::EntityDisplayName::new("Rat").map_err(|_| BatchError::InvalidView)?,
        ),
        kind: oteryn_session::EntityKind::Creature,
        entity: EntityRef {
            identity: [8; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    view = view.with_entities(vec![entity])?;
    view.click(TileCoord::new(101, 200))?;
    let selected = view.scene().target();
    assert!(selected.is_some());
    let appearance = oteryn_session::VisibleAppearance {
        light: None,
        entity,
        look: None,
    };
    view.apply_pushed(&Pushed {
        appearances: Some(vec![appearance.clone()]),
        world_clock: Some(oteryn_session::world_clock::WorldClock {
            tibian_minute: 1439,
            light_level: 128,
            light_color: 180,
        }),
        ..Pushed::default()
    })?;
    let lit = view.scene().sprites().lighting.clone();
    let mut unlit = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    unlit.set_presentations(vec![appearance])?;
    assert_eq!(lit.ambient, [128, 0, 0]);
    assert_ne!(lit, unlit.scene().sprites().lighting);
    assert_eq!(view.scene().name_count(), 1);
    assert!(view.hud_status().contains("23:59"));
    assert_eq!(view.scene().target(), selected);
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().sprites().lighting, lit);
    assert_eq!(view.scene().target(), selected);
    Ok(())
}
