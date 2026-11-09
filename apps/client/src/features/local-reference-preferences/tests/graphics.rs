use super::*;

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
