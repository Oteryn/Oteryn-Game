use oteryn_client_runtime::world_weather::WorldWeather;
/// Visual-only weather tint. Native floors -7..=0 are outdoors; no roof data is sent.
pub fn overlay(weather: Option<WorldWeather>, floor: i16) -> Option<([u8; 3], u8)> {
    if !(-7..=0).contains(&floor) {
        return None;
    }
    let WorldWeather::Active {
        kind, intensity, ..
    } = weather?
    else {
        return None;
    };
    let color = match kind {
        2 => [100, 110, 125],
        3 => [65, 105, 155],
        4 => [45, 55, 90],
        5 => [195, 200, 205],
        6 => [220, 235, 255],
        7 => [190, 215, 245],
        8 => [235, 175, 100],
        9 => [190, 145, 75],
        _ => return None,
    };
    Some((color, intensity.clamp(1, 3) * 24))
}
