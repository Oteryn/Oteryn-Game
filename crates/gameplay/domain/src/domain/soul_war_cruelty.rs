//! Crystal 201189f soul_war_mechanics.lua:1091–1105. Each component is
//! multiplied as a Lua double, then converted to the engine's integer magnitude.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pinned Lua callback multiplies doubles and C++ truncates the nonnegative component"
)]
pub fn received_component(value: u32, drain: u64) -> u32 {
    let drain = u32::try_from(drain).unwrap_or(100).min(100);
    let multiplier = (1.0 - f64::from(drain) / 100.0).max(0.0);
    (f64::from(value) * multiplier) as u32
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_cruelty_health_event_truncates_components_individually() {
        assert_eq!((received_component(1, 2), received_component(1, 2)), (0, 0));
        assert_eq!(
            (received_component(10_000, 3), received_component(10_001, 3)),
            (9700, 9700)
        );
    }
    #[test]
    fn srx3_cruelty_health_event_retains_literal_zero_and_full_reduction_boundaries() {
        assert_eq!(received_component(12345, 0), 12345);
        assert_eq!(received_component(12345, 100), 0);
        assert_eq!(received_component(12345, 101), 0);
        assert_eq!(received_component(0, 2), 0);
    }
}
