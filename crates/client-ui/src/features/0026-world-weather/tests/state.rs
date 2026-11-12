use super::*;
use oteryn_client_runtime::world_weather::WorldWeather;
#[test]
fn weather_replacement_intensity_and_floors() {
    let rain = |intensity| WorldWeather::Active {
        binding_token: [1; 32],
        absolute_slot: 0,
        kind: 3,
        intensity,
    };
    let mut state = None;
    protocol::apply(&mut state, rain(1));
    assert_eq!(overlay(state, -7), Some(([65, 105, 155], 24)));
    protocol::apply(&mut state, rain(3));
    assert_eq!(overlay(state, 0), Some(([65, 105, 155], 72)));
    assert_eq!(overlay(state, -8), None);
    assert_eq!(overlay(state, 1), None);
    protocol::apply(&mut state, WorldWeather::Unavailable);
    assert_eq!(overlay(state, -7), None);
    assert_eq!(overlay(None, -7), None);
    for kind in 1..=9 {
        let weather = WorldWeather::Active {
            binding_token: [1; 32],
            absolute_slot: 0,
            kind,
            intensity: 2,
        };
        assert_eq!(overlay(Some(weather), -7).is_some(), kind != 1);
    }
}
