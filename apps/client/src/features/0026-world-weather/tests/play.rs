use crate::play::{PlayView, Pushed};
use oteryn_renderer::TileCoord;
use oteryn_session::{ActorPosition, world_clock::WorldClock, world_weather::WorldWeather};
#[test]
fn weather_replaces_clears_and_tracks_floor_without_losing_light()
-> Result<(), oteryn_renderer::BatchError> {
    let mut view = PlayView::new(TileCoord::new(0, 0), -7, &[])?;
    view.apply_pushed(&Pushed {
        world_clock: Some(WorldClock {
            tibian_minute: 0,
            light_level: 128,
            light_color: 215,
        }),
        ..Pushed::default()
    })?;
    let lit = view.scene().atlas().clone();
    let rain = |intensity| WorldWeather::Active {
        binding_token: [1; 32],
        absolute_slot: 0,
        kind: 3,
        intensity,
    };
    view.apply_pushed(&Pushed {
        world_weather: Some(rain(1)),
        ..Pushed::default()
    })?;
    let light_rain = view.scene().atlas().clone();
    assert_ne!(light_rain, lit);
    view.apply_pushed(&Pushed {
        world_weather: Some(rain(3)),
        ..Pushed::default()
    })?;
    let heavy_rain = view.scene().atlas().clone();
    assert_ne!(heavy_rain, light_rain);
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().atlas(), &heavy_rain);
    view.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 0,
            y: 0,
            floor: -8,
        }),
        ..Pushed::default()
    })?;
    assert_eq!(view.scene().atlas(), &lit);
    view.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 0,
            y: 0,
            floor: -7,
        }),
        ..Pushed::default()
    })?;
    assert_eq!(view.scene().atlas(), &heavy_rain);
    view.apply_pushed(&Pushed {
        world_weather: Some(WorldWeather::Unavailable),
        ..Pushed::default()
    })?;
    assert_eq!(view.scene().atlas(), &lit);
    assert_eq!(view.world_clock_text().as_deref(), Some("00:00"));
    Ok(())
}
