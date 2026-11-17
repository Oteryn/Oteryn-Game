use super::*;

#[test]
fn interface_scale_projects_panels_and_live_labels_without_zooming_the_map()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{fit_ui_scale, scale_ui_rect};
    let entity = WorldSpatialEntity {
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        kind: oteryn_session::EntityKind::Creature,
        display_name: Some(oteryn_session::EntityDisplayName::new("Rat").map_err(|_| "name")?),
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
    let mut play = view()?;
    play.set_presentations(vec![oteryn_session::VisibleAppearance {
        entity,
        look: None,
        light: None,
        forge_stacks: None,
    }])?;
    let tile_px = play.scene().view().tile_px();
    let mut preferences = play.preferences().clone();
    preferences.interface_scale = 150;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().view().tile_px(), tile_px);
    assert!(
        play.scene()
            .sprites()
            .instances()
            .iter()
            .any(|quad| quad.size == [12.0, 18.0])
    );
    assert_eq!(
        scale_ui_rect([10, 20, 300, 80], play.preferences().interface_scale),
        [15, 30, 450, 120]
    );
    let fit = fit_ui_scale(200, [1100, 630], [1920, 1040]);
    let fitted = scale_ui_rect([0, 0, 1100, 630], fit);
    assert!(fitted[2] <= 1920 && fitted[3] <= 1040);
    Ok(())
}

#[test]
fn tooltips_use_visible_server_names_delay_and_generation() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::HoverTooltip;
    use std::time::{Duration, Instant};
    let entity = WorldSpatialEntity {
        entity: EntityRef {
            identity: [7; 16],
            generation: 1,
        },
        kind: oteryn_session::EntityKind::Creature,
        display_name: Some(oteryn_session::EntityDisplayName::new("Rat").map_err(|_| "name")?),
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
    let mut play = view()?.with_entities(vec![entity])?;
    let [x, y] = play.scene().view().tile_to_screen(TileCoord::new(101, 200));
    let at = [x + 1.0, y + 1.0];
    let target = play.tooltip_target(at).ok_or("missing visible target")?;
    let mut hover = HoverTooltip::default();
    let now = Instant::now();
    let delay = Duration::from_millis(500);
    assert_eq!(hover.update(Some(target.clone()), now, delay), None);
    assert_eq!(
        hover.update(Some(target.clone()), now + delay, delay),
        Some("Rat")
    );
    play.entities[0].entity.generation += 1;
    play.rebuild()?;
    assert_eq!(
        hover.update(play.tooltip_target(at), now + delay, delay),
        None
    );
    let mut preferences = play.preferences().clone();
    preferences.tooltips_enabled = false;
    play.set_preferences(preferences)?;
    assert_eq!(play.tooltip_target(at), None);
    assert_eq!(hover.update(None, now + delay + delay, delay), None);
    Ok(())
}

#[test]
fn t050_cursor_and_full_ui_scale_ranges_persist_or_refuse() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, ReferencePreferences};
    let path = std::env::temp_dir().join(format!("t050-ui-scale-{}.json", std::process::id()));
    let mut store = PreferenceStore::defaults(path.clone());
    let mut preferences = ReferencePreferences {
        cursor_crosshair: true,
        interface_scale: 200,
        ui_scale: 80,
        ..Default::default()
    };
    store.apply(preferences.clone())?;
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &preferences
    );
    preferences.interface_scale = 80;
    preferences.ui_scale = 180;
    store.apply(preferences.clone())?;
    for (interface_scale, ui_scale) in [(79, 100), (201, 100), (100, 79), (100, 181)] {
        let rejected = ReferencePreferences {
            interface_scale,
            ui_scale,
            ..preferences.clone()
        };
        assert!(store.apply(rejected).is_err());
    }
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &preferences
    );
    let _ = std::fs::remove_file(path.with_extension("backup.json"));
    let _ = std::fs::remove_file(path);
    Ok(())
}
