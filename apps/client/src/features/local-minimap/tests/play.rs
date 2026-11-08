use super::*;

#[test]
fn minimap_click_is_centered_consumed_and_queued_on_current_floor() -> Result<(), BatchError> {
    let mut play = minimap_view()?;
    play.set_live_map(&snapshot(100))?;
    assert!(play.minimap_click(62.0, 50.0));
    assert!(play.inputs.is_empty());
    assert!(!play.minimap_click(124.0, 0.0));
    play.apply(&moved_to(101, 202, -6))?;
    assert!(play.minimap_click(66.0, 50.0));
    assert_eq!(
        play.inputs.pop_front(),
        Some(Input::TargetWalk(ActorPosition {
            x: 102,
            y: 202,
            floor: -6
        }))
    );
    let mut prefs = play.preferences().clone();
    prefs.visible_panels.retain(|p| p != "minimap");
    assert!(play.set_preferences(prefs).is_ok());
    assert!(!play.minimap_click(62.0, 50.0));
    assert!(mini(&play).is_empty());
    Ok(())
}

fn mini(play: &PlayView) -> Vec<oteryn_renderer::QuadInstance> {
    play.scene()
        .sprites()
        .instances()
        .iter()
        .filter(|q| q.size == [4.0; 2] || q.size == [12.0; 2])
        .copied()
        .collect()
}

#[test]
fn minimap_projects_live_tiles_creatures_floor_changes_and_reset() -> Result<(), BatchError> {
    let mut play = minimap_view()?;
    assert!(mini(&play).is_empty());
    let map = snapshot(101);
    play.set_live_map(&map)?;
    let base = mini(&play);
    assert_eq!(base.len(), 31 * 25 + 2);
    assert!(
        base.iter()
            .any(|q| q.size == [12.0; 2] && q.position == [56.0, 44.0])
    ); // Background, one received tile, own marker.
    let creature = WorldSpatialEntity {
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        entity: EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 102,
            y: 200,
            floor: -7,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut off_floor = creature;
    off_floor.position.floor = -6;
    off_floor.entity.identity = [2; 16];
    play.apply_pushed(&Pushed {
        entities: Some(vec![creature, off_floor]),
        ..Pushed::default()
    })?;
    let projected = mini(&play);
    assert_eq!(projected.len(), base.len() + 1);
    assert!(projected.iter().any(|q| {
        q.position == [68.0, 48.0]
            && q.uv
                == play
                    .scene()
                    .atlas()
                    .uv_rect(crate::map::MARKER_CELL)
                    .unwrap_or_default()
    }));
    play.apply(&moved_to(101, 200, -7))?;
    assert!(mini(&play).iter().any(|q| {
        q.position == [64.0, 48.0]
            && q.uv
                == play
                    .scene()
                    .atlas()
                    .uv_rect(crate::map::MARKER_CELL)
                    .unwrap_or_default()
    }));
    play.apply(&moved_to(101, 200, -6))?;
    assert_eq!(mini(&play).len(), 31 * 25 + 2); // Above-floor creature, no surface terrain.
    play.apply(&moved_to(101, 200, -7))?;
    let mut empty = map;
    empty.tiles.clear();
    play.set_live_map(&empty)?;
    assert_eq!(mini(&play).len(), 31 * 25 + 2); // Creature remains; the authoritative clear removes terrain.
    Ok(())
}

#[test]
fn minimap_autowalk_obeys_capability_gate() -> Result<(), BatchError> {
    for selected in [false, true] {
        let mut play = minimap_view()?;
        play.set_live_map(&snapshot(100))?;
        let (link, mut commands, outbox) = play_channel();
        outbox.target_walk_initialize(selected);
        assert!(play.minimap_click(66.0, 50.0));
        assert_eq!(play.tick(&link), Ok(()));
        if selected {
            assert_eq!(
                commands.try_recv(),
                Ok(crate::combat_input::PlayCommand::TargetWalk(
                    ActorPosition {
                        x: 101,
                        y: 200,
                        floor: -7
                    }
                ))
            );
        } else {
            assert!(commands.try_recv().is_err());
            assert_eq!(play.tick(&link), Ok(()));
            assert!(play.hud_status().contains("Server walking unavailable"));
        }
        assert_eq!(play.own(), TileCoord::new(100, 200));
    }
    Ok(())
}

fn minimap_view() -> Result<PlayView, BatchError> {
    PlayView::new(TileCoord::new(100, 200), -7, &[])
}

fn snapshot(x: i32) -> oteryn_session::world_map::WorldMapViewSnapshot {
    let mut map = super::feature_local_world_view::live_snapshot(x);
    map.header.origin.floor = -7;
    map.header.first_visible_floor = -7;
    for tile in &mut map.tiles {
        tile.position.floor = -7;
    }
    map
}

#[test]
fn minimap_rectangle_captures_clicks_before_world_selection() -> Result<(), BatchError> {
    let mut play = minimap_view()?;
    play.set_live_map(&snapshot(100))?;
    let beneath =
        crate::input::click_tile(play.scene().view(), 4.0, 4.0).ok_or(BatchError::InvalidView)?;
    let creature = WorldSpatialEntity {
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        entity: EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: beneath.x,
            y: beneath.y,
            floor: -7,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    play.apply_pushed(&Pushed {
        entities: Some(vec![creature]),
        ..Pushed::default()
    })?;
    play.click_screen(4.0, 4.0)?;
    assert_eq!(play.scene().target(), None);
    assert_eq!(
        play.inputs.pop_front(),
        Some(Input::TargetWalk(ActorPosition {
            x: 86,
            y: 189,
            floor: -7
        }))
    );
    assert!(play.inputs.is_empty());
    let mut hidden = play.preferences().clone();
    hidden.visible_panels.retain(|p| p != "minimap");
    assert!(play.set_preferences(hidden).is_ok());
    play.click_screen(4.0, 4.0)?;
    assert_eq!(
        play.scene().target().and_then(|t| t.entity),
        Some(creature.entity)
    );
    assert_eq!(
        play.inputs.pop_front(),
        Some(Input::Attack(creature.entity))
    );
    Ok(())
}

#[test]
fn persistent_minimap_pan_zoom_and_floor_selection_share_the_click_projection()
-> Result<(), BatchError> {
    let mut play = minimap_view()?;
    let mut ground = snapshot(100);
    ground.tiles[0].ground_speed = 100;
    play.set_live_map(&ground)?;
    play.set_live_map(&snapshot(1000))?;
    {
        let mut map = crate::play::feature_local_minimap::state::lock(&play.minimap);
        assert!(map.cells.contains_key(&(-7, 100, 200)));
        map.tracking = false;
        map.x = 100;
        map.y = 200;
        map.zoom = 4;
        map.floor = -6;
    }
    play.rebuild()?;
    assert!(play.minimap_click(66.0, 50.0));
    assert!(play.inputs.is_empty()); // Browsing another floor never sends a walking command.
    crate::play::feature_local_minimap::state::lock(&play.minimap).floor = -7;
    play.rebuild()?;
    assert!(play.minimap_click(66.0, 50.0));
    assert_eq!(
        play.inputs.pop_front(),
        Some(Input::TargetWalk(ActorPosition {
            x: 104,
            y: 200,
            floor: -7
        }))
    );
    Ok(())
}

#[test]
fn minimap_world_switch_reanchors_camera_without_losing_same_world_pan() -> Result<(), BatchError> {
    let mut play = minimap_view()?;
    play.bind_minimap_world("11111111-1111-4111-8111-111111111111");
    play.set_live_map(&snapshot(100))?;
    {
        let mut map = crate::play::feature_local_minimap::state::lock(&play.minimap);
        map.pan(50, 50);
        map.zoom = 4;
    }
    play.bind_minimap_world("22222222-2222-4222-8222-222222222222");
    play.apply(&moved_to(110, 202, -7))?;
    play.rebuild()?;
    {
        let mut map = crate::play::feature_local_minimap::state::lock(&play.minimap);
        assert_eq!((map.x, map.y, map.floor, map.zoom), (110, 202, -7, 1));
        assert!(map.tracking);
        map.pan(2, 0);
    }
    play.bind_minimap_world("22222222-2222-4222-8222-222222222222");
    assert!(!crate::play::feature_local_minimap::state::lock(&play.minimap).tracking);
    Ok(())
}

#[test]
fn minimap_transfer_round_trips_scoped_memory_and_markers_without_changing_world()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{MinimapMark, MinimapMemory, PreferenceStore, SettingsState};
    let mut play = minimap_view()?;
    let mut map = snapshot(100);
    play.set_live_map(&map)?;
    let mut memory = play.preferences().minimap.clone();
    assert_eq!(memory.tiles.len(), map.tiles.len());
    memory.add_mark(MinimapMark {
        x: 101,
        y: 200,
        floor: -7,
        label: "Meeting place".into(),
    })?;
    let root = std::env::temp_dir().join(format!("clientgap-map-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let file = root.join("map.json");
    memory.export_new(&file)?;
    assert!(memory.export_new(&file).is_err());
    let loaded = MinimapMemory::import(&file)?;
    assert_eq!(loaded, memory);
    let mut mismatched = play.preferences().clone();
    mismatched.minimap = loaded.clone();
    if let Some(binding) = &mut mismatched.minimap.binding {
        binding.epoch += 99;
    }
    play.set_preferences(mismatched)?;
    assert!(play.scene.minimap_memory.tiles.is_empty());
    let old_world = play.local.world();
    let old_own = play.own();
    let mut prefs = play.preferences().clone();
    prefs.minimap = loaded;
    play.set_preferences(prefs.clone())?;
    assert!(std::sync::Arc::ptr_eq(&old_world, &play.local.world()));
    assert_eq!(play.own(), old_own);
    assert!(play.inputs.is_empty());
    let path = root.join("settings.json");
    let mut store = PreferenceStore::load(path.clone())?;
    store.apply(prefs.clone())?;
    assert_eq!(PreferenceStore::load(path)?.preferences().minimap, memory);
    let mut settings = SettingsState::new(None);
    settings.restore(&mut play, [960, 540])?;
    assert_eq!(settings.preferences.minimap, memory);
    let mut reset = settings.preferences.clone();
    reset.minimap = MinimapMemory::default();
    settings.apply(reset);
    settings.restore(&mut play, [960, 540])?;
    assert!(play.preferences().minimap.tiles.is_empty());
    // A new server observation repopulates memory; a content/reset binding change
    // invalidates old observations and markers instead of mixing map scopes.
    map.header.reset_epoch += 1;
    play.set_live_map(&map)?;
    settings.restore(&mut play, [960, 540])?;
    assert_eq!(
        settings
            .preferences
            .minimap
            .binding
            .as_ref()
            .map(|b| b.epoch),
        Some(map.header.reset_epoch)
    );
    assert!(settings.preferences.minimap.marks.is_empty());
    let mut invalid = memory.clone();
    invalid.tiles.push(invalid.tiles[0]);
    std::fs::write(&file, serde_json::to_vec(&invalid)?)?;
    assert!(MinimapMemory::import(&file).is_err());
    std::fs::write(&file, vec![b' '; 1024 * 1024 + 1])?;
    assert!(MinimapMemory::import(&file).is_err());
    let scoped_path = root.join("character-settings.json");
    let mut scoped = SettingsState::new(Some(scoped_path.clone()));
    scoped.select_character(Some([1; 16]));
    let mut observed = minimap_view()?;
    observed.set_live_map(&map)?;
    scoped.restore(&mut observed, [960, 540])?;
    let remembered = scoped.preferences.minimap.clone();
    assert!(!remembered.tiles.is_empty());
    scoped.select_character(Some([2; 16]));
    scoped.restore(&mut observed, [960, 540])?;
    assert!(observed.preferences().minimap.tiles.is_empty());
    let mut restarted = SettingsState::new(Some(scoped_path));
    restarted.select_character(Some([1; 16]));
    assert_eq!(restarted.preferences.minimap, remembered);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
