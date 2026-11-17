use crate::input::StepDir;
use oteryn_renderer::TileView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub visible: bool,
    pub mode: Option<usize>,
    pub joystick: Option<u64>,
    pub direction: Option<[StepDir; 2]>,
    pub alternate: bool,
    pub tap: Option<(u64, [f64; 2])>,
}

/// Shared drawing and hit geometry in the same scene pixels as mouse picking.
pub(crate) struct Layout {
    pub center: [f64; 2],
    pub radius: f64,
    pub scale: f64,
    pub bar: [f64; 2],
}
impl Layout {
    pub fn new(view: &TileView) -> Self {
        let (width, height) = view.viewport_px();
        let scale = (f64::from(width) / 480.0).min(f64::from(height) / 352.0);
        Self {
            center: [72.0 * scale, f64::from(height) - 72.0 * scale],
            radius: 64.0 * scale,
            scale,
            bar: [
                f64::from(width) - 288.0 * scale,
                f64::from(height) - 56.0 * scale,
            ],
        }
    }
    pub fn button_center(&self, index: usize) -> [f64; 2] {
        [
            self.bar[0] + (index as f64 * 70.0 + 34.0) * self.scale,
            self.bar[1] + 24.0 * self.scale,
        ]
    }
    pub fn button(&self, at: [f64; 2]) -> Option<usize> {
        (0..4).find(|&index| {
            let center = self.button_center(index);
            (at[0] - center[0]).abs() <= 33.0 * self.scale
                && (at[1] - center[1]).abs() <= 24.0 * self.scale
        })
    }
    pub fn distance(&self, at: [f64; 2]) -> f64 {
        (at[0] - self.center[0]).hypot(at[1] - self.center[1])
    }
    pub fn direction(&self, at: [f64; 2]) -> Option<[StepDir; 2]> {
        use StepDir::*;
        if !at.iter().all(|value| value.is_finite()) || self.distance(at) <= 16.0 * self.scale {
            return None;
        }
        let dx = at[0] - self.center[0];
        let dy = at[1] - self.center[1];
        let horizontal = if dx < 0.0 { West } else { East };
        let vertical = if dy < 0.0 { North } else { South };
        // tan(22.5 degrees) separates the eight equal angular sectors.
        Some(if dx.abs() < dy.abs() * 0.414_213_562_37 {
            [vertical; 2]
        } else if dy.abs() < dx.abs() * 0.414_213_562_37 {
            [horizontal; 2]
        } else {
            [vertical, horizontal]
        })
    }
}
