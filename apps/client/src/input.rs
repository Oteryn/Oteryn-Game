//! Mouse input logic (ADR-0020 section 5, N3): screen to tile, click to tile, click to target.
//!
//! Everything here is pure and transport-free. `StepDir` mirrors the four `step` directions
//! (north is `y - 1`); N4 maps it 1:1 onto the session's `step` command. No command, wire
//! message or server behaviour is added.

use oteryn_renderer::{TileCoord, TileView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDir {
    North,
    East,
    South,
    West,
}

/// Tile under a physical mouse position, or `None` outside the drawn view.
#[must_use]
pub fn click_tile(view: &TileView, x: f64, y: f64) -> Option<TileCoord> {
    // Precision loss is irrelevant at window-pixel scale.
    #[allow(clippy::cast_possible_truncation)]
    view.screen_to_tile(x as f32, y as f32)
}

/// One step from `from` toward `to`, along the axis with the larger remaining distance
/// (horizontal on a tie). `None` on arrival.
#[must_use]
pub fn step_toward(from: TileCoord, to: TileCoord) -> Option<StepDir> {
    let dx = i64::from(to.x) - i64::from(from.x);
    let dy = i64::from(to.y) - i64::from(from.y);
    if dx == 0 && dy == 0 {
        None
    } else if dx.abs() >= dy.abs() {
        Some(if dx > 0 { StepDir::East } else { StepDir::West })
    } else {
        Some(if dy > 0 {
            StepDir::South
        } else {
            StepDir::North
        })
    }
}

/// Server answer to one step, reduced to what click-to-tile needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Moved,
    Refused,
}

/// Click-to-tile state: one goal, one outstanding step. No pathfinding over unknown terrain:
/// the walk stops on the first refusal or on arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickWalk {
    goal: Option<TileCoord>,
    pending: bool,
}

impl ClickWalk {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            goal: None,
            pending: false,
        }
    }

    #[must_use]
    pub const fn goal(&self) -> Option<TileCoord> {
        self.goal
    }

    /// Sets a new goal, replacing any earlier one.
    pub const fn set_goal(&mut self, goal: TileCoord) {
        self.goal = Some(goal);
    }

    pub const fn cancel(&mut self) {
        self.goal = None;
    }

    /// The step to send now, if any. Returns `None` while a step awaits its answer, without a
    /// goal, and on arrival (which clears the goal). A returned step is marked outstanding until
    /// `on_result`.
    pub fn next_step(&mut self, own: TileCoord) -> Option<StepDir> {
        if self.pending {
            return None;
        }
        let goal = self.goal?;
        let direction = step_toward(own, goal);
        if direction.is_none() {
            self.goal = None;
        }
        self.pending = direction.is_some();
        direction
    }

    /// Records the server's answer to the outstanding step. A refusal ends the walk.
    pub const fn on_result(&mut self, result: StepResult) {
        self.pending = false;
        if matches!(result, StepResult::Refused) {
            self.goal = None;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Object,
    Entity,
}

/// Something visible that a click may select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Targetable {
    pub tile: TileCoord,
    pub kind: TargetKind,
}

/// What a click on `tile` selects: an entity wins over an object on the same tile; empty tiles
/// select nothing (they are a walk goal instead). The last candidate of a kind wins, matching
/// draw order.
#[must_use]
pub fn pick_target(tile: TileCoord, visible: &[Targetable]) -> Option<Targetable> {
    let on_tile = || visible.iter().rev().filter(move |t| t.tile == tile);
    on_tile()
        .find(|t| t.kind == TargetKind::Entity)
        .or_else(|| on_tile().find(|t| t.kind == TargetKind::Object))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(x: i32, y: i32) -> TileCoord {
        TileCoord::new(x, y)
    }

    fn view() -> Result<TileView, oteryn_renderer::BatchError> {
        TileView::new(t(-7, -5), 48, 15, 11)
    }

    #[test]
    fn click_maps_pixels_to_tiles_and_rejects_outside() -> Result<(), oteryn_renderer::BatchError> {
        let view = view()?;
        assert_eq!(click_tile(&view, 0.0, 0.0), Some(t(-7, -5)));
        assert_eq!(click_tile(&view, 47.9, 47.9), Some(t(-7, -5)));
        assert_eq!(click_tile(&view, 48.0, 96.0), Some(t(-6, -3)));
        assert_eq!(click_tile(&view, 719.0, 527.0), Some(t(7, 5)));
        assert_eq!(click_tile(&view, 720.0, 10.0), None);
        assert_eq!(click_tile(&view, -1.0, 10.0), None);
        assert_eq!(click_tile(&view, f64::NAN, 10.0), None);
        Ok(())
    }

    #[test]
    fn step_toward_prefers_the_longer_axis_and_stops_on_arrival() {
        assert_eq!(step_toward(t(0, 0), t(0, 0)), None);
        assert_eq!(step_toward(t(0, 0), t(3, 1)), Some(StepDir::East));
        assert_eq!(step_toward(t(0, 0), t(-3, 1)), Some(StepDir::West));
        assert_eq!(step_toward(t(0, 0), t(1, 3)), Some(StepDir::South));
        assert_eq!(step_toward(t(0, 0), t(1, -3)), Some(StepDir::North));
        assert_eq!(step_toward(t(0, 0), t(2, 2)), Some(StepDir::East));
    }

    #[test]
    fn walk_steps_one_tile_at_a_time_until_arrival() {
        let mut walk = ClickWalk::new();
        walk.set_goal(t(2, 0));
        assert_eq!(walk.next_step(t(0, 0)), Some(StepDir::East));
        // Waits for the server's answer before the next step.
        assert_eq!(walk.next_step(t(0, 0)), None);
        walk.on_result(StepResult::Moved);
        assert_eq!(walk.next_step(t(1, 0)), Some(StepDir::East));
        walk.on_result(StepResult::Moved);
        assert_eq!(walk.next_step(t(2, 0)), None);
        assert_eq!(walk.goal(), None);
    }

    #[test]
    fn walk_stops_on_refusal() {
        let mut walk = ClickWalk::new();
        walk.set_goal(t(0, 5));
        assert_eq!(walk.next_step(t(0, 0)), Some(StepDir::South));
        walk.on_result(StepResult::Refused);
        assert_eq!(walk.goal(), None);
        assert_eq!(walk.next_step(t(0, 0)), None);
    }

    #[test]
    fn a_new_click_replaces_the_goal_and_cancel_clears_it() {
        let mut walk = ClickWalk::new();
        walk.set_goal(t(5, 0));
        walk.set_goal(t(0, -4));
        assert_eq!(walk.next_step(t(0, 0)), Some(StepDir::North));
        walk.cancel();
        walk.on_result(StepResult::Moved);
        assert_eq!(walk.next_step(t(0, -1)), None);
    }

    #[test]
    fn pick_prefers_entities_then_objects_then_nothing() {
        let visible = [
            Targetable {
                tile: t(1, 1),
                kind: TargetKind::Object,
            },
            Targetable {
                tile: t(1, 1),
                kind: TargetKind::Entity,
            },
            Targetable {
                tile: t(2, 2),
                kind: TargetKind::Object,
            },
        ];
        assert_eq!(
            pick_target(t(1, 1), &visible).map(|p| p.kind),
            Some(TargetKind::Entity)
        );
        assert_eq!(
            pick_target(t(2, 2), &visible).map(|p| p.kind),
            Some(TargetKind::Object)
        );
        assert_eq!(pick_target(t(3, 3), &visible), None);
    }
}
