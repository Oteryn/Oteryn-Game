use super::*;
#[test]
fn t060_selected_engine_and_aa_survive_restart_and_live_input_queue()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{SettingsState, play_channel};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    use oteryn_renderer::{Antialiasing, GraphicsEngine};
    let path = std::env::temp_dir().join(format!("t060-select-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut preferences = settings.preferences.clone();
    preferences.graphics.engine = GraphicsEngine::Vulkan;
    preferences.graphics.antialiasing = Antialiasing::Msaa4;
    settings.apply(preferences);
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut play = view()?;
    settings.restore(&mut play, [960, 540])?;
    assert_eq!(play.preferences().graphics.engine, GraphicsEngine::Vulkan);
    assert_eq!(play.preferences().graphics.antialiasing.samples(), 4);
    let (link, mut commands, _) = play_channel();
    let (events, opened) = settings.input(&[NormalizedInputEvent::Key {
        code: KeyCode::ARROW_UP,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    }]);
    assert!(!opened);
    play.arrow(crate::input::arrow_step(&events).ok_or("missing movement")?);
    play.tick(&link).map_err(|_| "tick failed")?;
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Step(crate::input::StepDir::North)
    );
    assert!(commands.try_recv().is_err());
    let legacy: crate::play::GraphicsPreferences = serde_json::from_str("{}")?;
    assert_eq!(legacy.engine, GraphicsEngine::Direct3D12);
    assert_eq!(legacy.antialiasing, Antialiasing::Off);
    assert!(
        serde_json::from_str::<crate::play::GraphicsPreferences>(r#"{"antialiasing":"Unknown"}"#)
            .is_err()
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn t060_graphics_settings_restore_into_real_scene_and_presentation_request()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    let path = std::env::temp_dir().join(format!("t060-graphics-{}.json", std::process::id()));
    let mut state = SettingsState::new(Some(path.clone()));
    let mut prefs = state.preferences.clone();
    prefs.ui_scale = 100;
    prefs.graphics.fullscreen = true;
    prefs.graphics.vsync = false;
    prefs.graphics.integer_scaling = true;
    state.apply(prefs);
    let mut state = SettingsState::new(Some(path.clone()));
    let mut play = view()?;
    state.restore(&mut play, [4096, 4096])?;
    assert_eq!(play.scene().view().tile_px(), 64);
    assert!(play.preferences().graphics.fullscreen);
    assert_eq!(
        play.preferences().graphics.presentation(),
        oteryn_renderer::PresentationMode::Unlocked
    );
    let mut changed = play.preferences().clone();
    changed.graphics.integer_scaling = false;
    play.set_preferences(changed)?;
    assert_eq!(play.scene().view().tile_px(), 48);
    assert_eq!(oteryn_renderer::GRAPHICS_ENGINE.label(), "Direct3D 12");
    assert_eq!(oteryn_renderer::ANTIALIASING.samples(), 1);
    let mut integral = play.preferences().clone();
    integral.graphics.integer_scaling = true;
    for zoom in [80, 100, 125, 150, 180] {
        integral.ui_scale = zoom;
        play.set_preferences(integral.clone())?;
        assert_eq!(
            play.scene().view().tile_px() % oteryn_renderer::PAGE_CELL_PX,
            0
        );
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn graphics_preferences_reach_live_scene_lighting_and_gpu_floor_separation()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{Pushed, SettingsState};
    use oteryn_session::{world_clock::WorldClock, world_weather::WorldWeather};
    let path = std::env::temp_dir().join(format!("t062-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut play = view()?;
    let mut map = crate::play::tests::feature_local_world_view::live_snapshot(100);
    let mut below = map.tiles[0].clone();
    below.position = ActorPosition {
        x: 99,
        y: 199,
        floor: -1,
    };
    map.tiles.push(below);
    play.set_live_map(&map)?;
    play.apply_pushed(&Pushed {
        world_clock: Some(WorldClock {
            tibian_minute: 0,
            light_level: 0,
            light_color: 215,
        }),
        ..Default::default()
    })?;
    let mut preferences = settings.preferences.clone();
    preferences.graphics.ambient = 80;
    preferences.graphics.separator = 50;
    preferences.graphics.indoor = 25;
    preferences.graphics.cloud = 50;
    preferences.visible_panels.retain(|id| id != "minimap");
    settings.apply(preferences);
    let mut restored = SettingsState::new(Some(path.clone()));
    restored.restore(&mut play, [960, 540])?;
    assert_eq!(play.scene().sprites().lighting.ambient, [204; 3]);
    let gpu_rgb = |play: &crate::play::PlayView| {
        let batch = play.scene().sprites();
        batch
            .lighting
            .instance_bytes(batch.instances())
            .as_chunks::<{ oteryn_renderer::lighting::LIT_INSTANCE_BYTES }>()
            .0
            .iter()
            .map(|row| f32::from_ne_bytes([row[32], row[33], row[34], row[35]]))
            .collect::<Vec<_>>()
    };
    let rgb = gpu_rgb(&play);
    assert!(
        rgb[0] < rgb[1],
        "the lower floor must be darker in the GPU upload"
    );
    play.apply_pushed(&Pushed {
        world_weather: Some(WorldWeather::Active {
            binding_token: [1; 32],
            absolute_slot: 0,
            kind: 3,
            intensity: 3,
        }),
        ..Default::default()
    })?;
    assert_eq!(play.scene().sprites().lighting.ambient, [102; 3]);
    play.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 100,
            y: 200,
            floor: -8,
        }),
        ..Default::default()
    })?;
    assert_eq!(play.scene().sprites().lighting.ambient, [153; 3]);
    let mut invalid = play.preferences().clone();
    invalid.graphics.separator = 101;
    assert!(play.set_preferences(invalid).is_err());
    let mut disabled = play.preferences().clone();
    disabled.show_lights = false;
    play.set_preferences(disabled)?;
    assert_eq!(play.scene().sprites().lighting.ambient, [255; 3]);
    std::fs::remove_file(path)?;
    Ok(())
}
