use crate::play::{PlayView, Pushed};
use oteryn_renderer::TileCoord;
use oteryn_session::world_clock::WorldClock;
#[test]
fn clxa_character_light_reaches_native_scene_and_expires_on_update()
-> Result<(), oteryn_renderer::BatchError> {
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, VisibleAppearance,
    };
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    let entry = VisibleAppearance {
        forge_stacks: None,
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            display_name: None,
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
        light: Some((8, 215)),
    };
    let clock = WorldClock {
        tibian_minute: 0,
        light_level: 0,
        light_color: 215,
    };
    view.apply_pushed(&Pushed {
        world_clock: Some(clock),
        appearances: Some(vec![entry.clone()]),
        ..Default::default()
    })?;
    assert_eq!(view.scene().sprites().lighting.sources.len(), 1);
    let source = view.scene().sprites().lighting.sources[0];
    assert_eq!(
        view.scene().sprites().lighting.sample(source.position),
        [1.0; 3]
    );
    assert_eq!(
        view.scene()
            .sprites()
            .lighting
            .sample([source.position[0] + source.radius, source.position[1]]),
        [0.0; 3]
    );
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().sprites().lighting.sources.len(), 1);
    let other_floor = VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            position: ActorPosition {
                floor: 6,
                ..entry.entity.position
            },
            ..entry.entity
        },
        ..entry.clone()
    };
    view.apply_pushed(&Pushed {
        appearances: Some(vec![other_floor]),
        ..Default::default()
    })?;
    assert!(view.scene().sprites().lighting.sources.is_empty());
    view.apply_pushed(&Pushed {
        appearances: Some(vec![VisibleAppearance {
            forge_stacks: None,
            light: None,
            ..entry
        }]),
        ..Default::default()
    })?;
    assert!(view.scene().sprites().lighting.sources.is_empty());
    Ok(())
}
#[test]
fn clock_and_light_survive_scene_rebuilds() -> Result<(), oteryn_renderer::BatchError> {
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    let original = view.scene().atlas().rgba().to_vec();
    view.apply_pushed(&Pushed {
        world_clock: Some(WorldClock {
            tibian_minute: 1439,
            light_level: 128,
            light_color: 180,
        }),
        ..Pushed::default()
    })?;
    assert_eq!(view.world_clock_text().as_deref(), Some("23:59"));
    assert_eq!(view.scene().atlas().rgba(), original);
    let lit = view.scene().sprites().lighting.clone();
    assert_eq!(lit.ambient, [128, 0, 0]);
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().sprites().lighting, lit);
    Ok(())
}

#[test]
fn ambient_preference_changes_confirmed_clock_lighting() -> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    view.apply_pushed(&Pushed {
        world_clock: Some(WorldClock { tibian_minute: 0, light_level: 0, light_color: 215 }),
        ..Pushed::default()
    })?;
    assert_eq!(view.scene().sprites().lighting.ambient, [0; 3]);
    let mut preferences = view.preferences().clone();
    preferences.ambient_brightness = 40;
    view.set_preferences(preferences.clone())?;
    assert_eq!(view.scene().sprites().lighting.ambient, [102; 3]);
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().sprites().lighting.ambient, [102; 3]);
    preferences.ambient_brightness = 101;
    assert!(view.set_preferences(preferences).is_err());
    Ok(())
}

#[test]
fn local_player_gets_minimum_light_without_reported_personal_light() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, VisibleAppearance, WorldSpatialEntity};
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    view.scene.set_own_identity([1;16]);
    let own = VisibleAppearance {
        entity: WorldSpatialEntity {
            motion: None,
            entity: EntityRef { identity: [1;16], generation: 1 },
            position: ActorPosition { x: 0, y: 0, floor: 7 },
            kind: EntityKind::Player, display_name: None,
            detail: EntityDetail::Actor { direction: StepDirection::South, appearance_ref: 1, health_percent: 100 },
        },
        light: None, look: None, forge_stacks: None,
    };
    for (reported, radius, color) in [(None, 3.0, 215), (Some((1,0)), 3.0, 215), (Some((8,180)), 8.0, 180)] {
        view.appearances = vec![VisibleAppearance { light: reported, ..own.clone() }];
        view.world_clock = Some(WorldClock { tibian_minute: 0, light_level: 0, light_color: 215 });
        view.light_scene()?;
        let sources = &view.scene().sprites().lighting.sources;
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].radius, radius * view.scene().view().tile_px() as f32);
        assert_eq!(sources[0].color, oteryn_renderer::lighting::light_color(color));
    }
    Ok(())
}
