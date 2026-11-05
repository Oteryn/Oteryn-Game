use super::*;
#[test]
fn live_outfit_uses_server_look_and_empty_outfit_suppresses_pixels() -> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let mut entry = VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Citizen")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: Some(oteryn_session::Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 0,
        }),
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.set_presentations_from(&[entry.clone()], 0, Some(root))?;
    assert!(
        scene.actor_draws[0]
            .1
            .iter()
            .any(|draw| draw.cell > MARKER_CELL)
    );
    assert_eq!(scene.name_count(), 1);
    entry.look = Some(oteryn_session::Look::EmptyOutfit);
    scene.set_presentations_from(&[entry.clone()], 0, None)?;
    assert!(scene.actor_draws[0].1.is_empty());
    entry.entity.position.floor = -1;
    scene.set_presentations_from(&[entry], 0, None)?;
    assert_eq!(scene.name_count(), 0);
    Ok(())
}
#[test]
fn other_actor_keeps_object_marker_and_local_player() -> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let mut scene = Scene::centered_on(
        Arc::new(World::builtin()?),
        None,
        tile,
        StepDir::South,
        &[tile],
    )?;
    let entity = oteryn_session::WorldSpatialEntity {
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        entity: oteryn_session::EntityRef {
            identity: [8; 16],
            generation: 1,
        },
        position: oteryn_session::ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut appearance = VisibleAppearance {
        entity,
        look: Some(oteryn_session::Look::EmptyOutfit),
    };
    scene.set_presentations_from(std::slice::from_ref(&appearance), 0, None)?;
    assert_eq!(
        scene.sprites().len(),
        2,
        "object marker and own actor must both survive"
    );
    appearance.look = None;
    scene.set_presentations_from(&[appearance], 0, None)?;
    assert_eq!(
        scene.sprites().len(),
        3,
        "object marker, other actor and local player must all survive"
    );
    Ok(())
}

#[test]
fn recenter_reuses_loaded_assets_outfits_glyphs_and_packed_atlas() -> Result<(), BatchError> {
    let world = Arc::new(World::builtin()?);
    let tile = TileCoord::new(100, 200);
    let mut scene = Scene::centered_on(world.clone(), None, tile, StepDir::South, &[])?;
    scene.set_own_identity([1; 16]);
    let entry = VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Citizen")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: Some(oteryn_session::Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 0,
        }),
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, Some(root.clone()))?;
    assert_eq!(scene.cache.counts(), [1, 1, 1]);
    assert!(
        scene
            .own_draws
            .as_ref()
            .is_some_and(|draws| draws.iter().any(|draw| draw.cell > MARKER_CELL))
    );
    let atlas = scene
        .presentation_atlas
        .clone()
        .ok_or(BatchError::InvalidAtlas)?;
    scene.recenter_with(
        world,
        None,
        TileCoord::new(101, 200),
        StepDir::South,
        &[],
        &[],
    )?;
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, Some(root.clone()))?;
    assert_eq!(
        scene.cache.counts(),
        [1, 1, 1],
        "rebuild must not reload, unpack or repack"
    );
    assert!(
        scene
            .presentation_atlas
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(&atlas, cached))
    );
    assert!(
        scene.actor_draws.is_empty(),
        "the own identity is drawn only in the player slot"
    );
    assert_eq!(scene.names[0].0, TileCoord::new(101, 200));
    // A new map atlas needs new slots but uses the already loaded appearance index/sheets.
    scene.recenter_with(
        Arc::new(World::builtin()?),
        None,
        tile,
        StepDir::South,
        &[],
        &[],
    )?;
    scene.set_presentations_from(&[entry], 0, Some(root))?;
    assert_eq!(scene.cache.counts(), [1, 2, 2]);
    Ok(())
}

#[test]
fn play_view_rebuild_retains_the_same_cached_image() -> Result<(), BatchError> {
    let mut view = crate::play::PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    view.set_own_identity([1; 16]);
    let entry = VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Player")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
    };
    view.set_presentations(vec![entry.clone()])?;
    let image = view
        .scene()
        .presentation_atlas
        .clone()
        .ok_or(BatchError::InvalidAtlas)?;
    let counts = view.scene().cache.counts();
    view.set_presentations(vec![entry])?;
    assert_eq!(view.scene().cache.counts(), counts);
    assert!(
        view.scene()
            .presentation_atlas
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(&image, cached))
    );
    Ok(())
}
