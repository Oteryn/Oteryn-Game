//! Walk smoothing: the sub-tile offset of a creature that has just stepped.

use oteryn_client_domain::TileCoord;
use std::time::Duration;

/// The longest gap between two steps still read as one continuous walk.
const MAX_GLIDE: Duration = Duration::from_millis(2_000);
/// The shortest glide, one server beat.
const MIN_GLIDE: Duration = Duration::from_millis(50);

/// One adjacent step, drawn at its destination with an offset that shrinks to zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glide {
    from: TileCoord,
    to: TileCoord,
    duration: Duration,
    carry: [f32; 2],
}

impl Glide {
    /// `None` for anything but a step to an adjacent tile (a teleport or a standstill snaps)
    /// or a zero duration.
    #[must_use]
    pub fn new(from: TileCoord, to: TileCoord, duration: Duration) -> Option<Self> {
        let dx = i64::from(to.x) - i64::from(from.x);
        let dy = i64::from(to.y) - i64::from(from.y);
        let adjacent = dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0);
        (adjacent && !duration.is_zero()).then_some(Self {
            from,
            to,
            duration,
            carry: [0.; 2],
        })
    }

    /// Start this adjacent step from the previous glide's current visual position.
    #[must_use]
    pub fn continuing(mut self, previous: Self, elapsed: Duration) -> Self {
        if previous.to == self.from {
            self.carry = previous.offset(elapsed, 1.0);
        }
        self
    }

    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    #[must_use]
    pub fn finished(&self, elapsed: Duration) -> bool {
        elapsed >= self.duration
    }

    /// Screen pixels to add to the destination tile `elapsed` into the step: the old tile at
    /// zero, the destination at the end.
    #[must_use]
    pub fn offset(&self, elapsed: Duration, tile_px: f32) -> [f32; 2] {
        let remaining = 1.0 - (elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);
        [
            ((self.from.x - self.to.x) as f32 + self.carry[0]) * tile_px * remaining,
            ((self.from.y - self.to.y) as f32 + self.carry[1]) * tile_px * remaining,
        ]
    }

    /// What the rest of the map shifts by while the camera follows this glide: the player stays
    /// centred and the world slides under it.
    #[must_use]
    pub fn camera_offset(&self, elapsed: Duration, tile_px: f32) -> [f32; 2] {
        let [x, y] = self.offset(elapsed, tile_px);
        [-x, -y]
    }
}

/// How long to glide a step: the gap since the previous one landed while the walk is
/// continuous (the server paces steps), else the speed-table `default`.
#[must_use]
pub fn glide_duration(since_last_step: Option<Duration>, default: Duration) -> Duration {
    since_last_step
        .filter(|gap| *gap <= MAX_GLIDE)
        .map_or(default, |gap| gap.max(MIN_GLIDE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(x: i32, y: i32) -> TileCoord {
        TileCoord::new(x, y)
    }
    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn the_offset_glides_linearly_from_the_old_tile_to_zero() -> Result<(), &'static str> {
        let glide = Glide::new(t(5, 5), t(6, 5), ms(500)).ok_or("adjacent")?;
        assert_eq!(glide.offset(ms(0), 48.0), [-48.0, 0.0]);
        assert_eq!(glide.offset(ms(250), 48.0), [-24.0, 0.0]);
        assert_eq!(glide.offset(ms(500), 48.0), [0.0, 0.0]);
        assert_eq!(glide.offset(ms(900), 48.0), [0.0, 0.0]);
        assert!(!glide.finished(ms(499)));
        assert!(glide.finished(ms(500)));
        Ok(())
    }

    #[test]
    fn a_diagonal_step_glides_diagonally() -> Result<(), &'static str> {
        let glide = Glide::new(t(5, 5), t(4, 6), ms(400)).ok_or("diagonal")?;
        assert_eq!(glide.offset(ms(100), 32.0), [24.0, -24.0]);
        Ok(())
    }

    #[test]
    fn a_jump_a_standstill_or_a_zero_duration_snaps_with_no_glide() {
        assert!(Glide::new(t(5, 5), t(7, 5), ms(500)).is_none());
        assert!(Glide::new(t(5, 5), t(5, 8), ms(500)).is_none());
        assert!(Glide::new(t(5, 5), t(5, 5), ms(500)).is_none());
        assert!(Glide::new(t(5, 5), t(6, 5), ms(0)).is_none());
    }

    #[test]
    fn the_camera_follows_by_the_opposite_offset() -> Result<(), &'static str> {
        let glide = Glide::new(t(5, 5), t(6, 5), ms(500)).ok_or("adjacent")?;
        assert_eq!(glide.camera_offset(ms(250), 48.0), [24.0, 0.0]);
        Ok(())
    }

    #[test]
    fn a_continuous_walk_glides_over_the_step_gap_and_a_cold_start_uses_the_default() {
        assert_eq!(glide_duration(Some(ms(550)), ms(400)), ms(550));
        assert_eq!(glide_duration(Some(ms(10)), ms(400)), ms(50));
        assert_eq!(glide_duration(Some(ms(5_000)), ms(400)), ms(400));
        assert_eq!(glide_duration(None, ms(400)), ms(400));
    }
}
