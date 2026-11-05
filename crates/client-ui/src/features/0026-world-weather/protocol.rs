use oteryn_client_runtime::world_weather::WorldWeather;
pub fn apply(state: &mut Option<WorldWeather>, weather: WorldWeather) {
    *state = Some(weather);
}
