//! Shared source duration for ordinary movement and House custody.
use super::*;
use crate::movement::CardinalStep;
pub fn player_directional_step_duration(
    source: &impl GroundSpeedSource,
    onto: LogicalCell,
    speed: u16,
    direction: CardinalStep,
) -> Option<Duration> {
    player_step_duration(source, onto, speed)?.checked_mul(direction.duration_multiplier())
}
