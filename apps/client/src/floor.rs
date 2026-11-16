//! Conversion at legacy chat, presentation and asset boundaries.

/// Convert a validated legacy depth (0..=15) to the native view floor.
pub(crate) const fn legacy_to_native(floor: i16) -> i16 {
    -floor
}
