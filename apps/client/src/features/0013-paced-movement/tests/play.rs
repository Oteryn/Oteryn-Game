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
    view.apply(&moved_to(88, 200, 1))?;
    assert!(view.walk_started().is_some());
    let mut blocked = moved_to(88, 200, 1);
    blocked.disposition = StepDisposition::Blocked;
    blocked.world_spatial_delta = None;
    view.apply(&blocked)?;
    assert_eq!(view.walk_started(), None);
    Ok(())
}

#[test]
fn queued_own_delta_preserves_glide_but_teleport_cancels_it()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    view.apply(&moved_to(101, 200, 0))?;
    let started = view.walk_started().ok_or("a glide begins")?;
    view.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        }),
        ..Default::default()
    })?;
    assert_eq!(view.walk_started(), Some(started));
    view.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 105,
            y: 200,
            floor: 0,
        }),
        ..Default::default()
    })?;
    assert_eq!(view.walk_started(), None);
    Ok(())
}

#[test]
fn replacing_a_glide_keeps_its_remaining_visual_displacement()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    view.apply(&moved_to(101, 200, 0))?;
    let duration = view.walk_duration().ok_or("duration")?;
    view.walk.as_mut().ok_or("walk")?.1 = std::time::Instant::now() - duration / 2;
    view.apply(&moved_to(102, 200, 0))?;
    let offset = view
        .camera_offset(view.walk_started().ok_or("start")?)
        .ok_or("offset")?;
    assert!((offset[0] - tile_px(&view) * 1.5).abs() < 1.0);
    Ok(())
}

#[test]
fn world_sprite_preparation_retains_the_scrolling_edge_margin() -> Result<(), BatchError> {
    let view = view()?;
    let origin = view.scene().view().origin();
    let edges = [
        (origin.x - 1, 200),
        (100, origin.y - 1),
        (origin.x + crate::scene::SCENE_COLUMNS as i32, 200),
        (100, origin.y + crate::scene::SCENE_ROWS as i32),
    ];
    let view = view.with_entities(
        edges
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| WorldSpatialEntity {
                motion: None,
                entity: oteryn_session::EntityRef {
                    identity: [i as u8 + 1; 16],
                    generation: 1,
                },
                kind: oteryn_session::EntityKind::Creature,
                position: ActorPosition { x, y, floor: 0 },
                display_name: None,
                detail: oteryn_session::EntityDetail::Actor {
                    direction: oteryn_session::StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            })
            .collect(),
    )?;
    let quads = view.scene().sprites().instances();
    for axis in 0..2 {
        assert!(
            quads
                .iter()
                .any(|q| q.position[axis] < 0.0 && q.position[axis] + q.size[axis] <= 0.0)
        );
        let edge = if axis == 0 {
            view.scene().view().viewport_px().0
        } else {
            view.scene().view().viewport_px().1
        } as f32;
        assert!(quads.iter().any(|q| q.position[axis] >= edge));
    }
    Ok(())
}

#[test]
fn tooltip_picks_the_displayed_creature_during_a_walk() -> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    let entity = oteryn_session::EntityRef {
        identity: [3; 16],
        generation: 1,
    };
    view = view.with_entities(vec![WorldSpatialEntity {
        motion: None,
        entity,
        kind: oteryn_session::EntityKind::Creature,
        position: ActorPosition {
            x: 103,
            y: 200,
            floor: 0,
        },
        display_name: Some(
            oteryn_session::EntityDisplayName::new("Target").map_err(|e| format!("{e:?}"))?,
        ),
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }])?;
    view.apply(&moved_to(101, 200, 0))?;
    let start = view.walk_started().ok_or("walk")?;
    let camera = view.camera_offset(start).ok_or("camera")?;
    let [x, y] = view.scene().view().tile_to_screen(TileCoord::new(103, 200));
    assert_eq!(
        view.tooltip_target([x + 4., y + 4.]).map(|t| t.entity),
        Some(entity)
    );
    view.use_shared_frontend();
    assert_eq!(
        view.tooltip_target([x + camera[0] + 4., y + camera[1] + 4.])
            .map(|t| t.entity),
        Some(entity)
    );
    Ok(())
}

#[test]
fn committed_step_timing_controls_first_and_continuous_visual_glides()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = view()?;
    for (x, y, duration) in [(101, 200, 150), (102, 200, 900), (103, 201, 1350)] {
        let mut outcome = moved_to(x, y, 0);
        outcome.step_wait_ms = duration;
        view.apply(&outcome)?;
        assert_eq!(view.walk_duration(), Some(std::time::Duration::from_millis(u64::from(duration))));
    }
    Ok(())
}
