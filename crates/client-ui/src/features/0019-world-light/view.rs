use oteryn_client_runtime::world_light::WorldClock;
/// Server-owned minute of day, displayed without extrapolating the clock locally.
pub fn clock_text(clock: WorldClock) -> String {
    format!(
        "{:02}:{:02}",
        clock.tibian_minute / 60,
        clock.tibian_minute % 60
    )
}
/// Tibia's indexed colour cube, scaled by the server's ambient light level.
pub fn ambient_rgb(clock: WorldClock) -> [u8; 3] {
    let color = u16::from(clock.light_color);
    let level = u16::from(clock.light_level);
    [color / 36 % 6, color / 6 % 6, color % 6].map(|component| (component * 51 * level / 255) as u8)
}
