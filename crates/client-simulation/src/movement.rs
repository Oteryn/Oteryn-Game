//! Deterministic click-walk state and server-compatible movement pacing.

use oteryn_client_domain::{
    TileCoord,
    movement::{StepDir, StepResult},
};
use std::sync::OnceLock;
use std::time::Duration;

const STEP_SPEED_TABLE_JSON: &str = include_str!("../../../content/movement/step_speed_v1.json");
const STEP_SPEED_TABLE_SHA256_U16LE: &str =
    "323b70ceb76edc53ed341a31d67fea5543149689ead9ad51856286e792bfe436";
pub const SPEED_MIN: u16 = 10;
pub const DEFAULT_GROUND_SPEED: u16 = 150;
const SERVER_BEAT_MS: u64 = 50;

#[derive(Debug)]
pub struct StepSpeedTable {
    step_speed: Vec<u16>,
}
impl StepSpeedTable {
    #[doc(hidden)]
    pub fn parse(text: &str) -> Option<Self> {
        let digest = format!("\"sha256_u16le\": \"{STEP_SPEED_TABLE_SHA256_U16LE}\"");
        if !text.contains(&digest)
            || !text.contains("\"speed_min\": 10,")
            || !text.contains("\"speed_max\": 65535,")
        {
            return None;
        }
        let (_, rest) = text.split_once("\"step_speed\": [")?;
        let (values, _) = rest.split_once(']')?;
        let step_speed = values
            .split(',')
            .map(|v| v.trim().parse::<u16>().ok().filter(|v| *v > 0))
            .collect::<Option<Vec<_>>>()?;
        (step_speed.len() == usize::from(u16::MAX - SPEED_MIN) + 1).then_some(Self { step_speed })
    }
    #[must_use]
    pub fn embedded() -> Option<&'static Self> {
        static TABLE: OnceLock<Option<StepSpeedTable>> = OnceLock::new();
        TABLE
            .get_or_init(|| Self::parse(STEP_SPEED_TABLE_JSON))
            .as_ref()
    }
    #[must_use]
    pub fn step_duration(&self, speed: u16, ground_speed: u16) -> Option<Duration> {
        if ground_speed == 0 {
            return None;
        }
        let step_speed = u64::from(
            *self
                .step_speed
                .get(usize::from(speed.checked_sub(SPEED_MIN)?))?,
        );
        let raw = 1000 * u64::from(ground_speed) / step_speed;
        Some(Duration::from_millis(
            raw.div_ceil(SERVER_BEAT_MS) * SERVER_BEAT_MS,
        ))
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickWalk {
    goal: Option<TileCoord>,
    pending: bool,
    ready_at: Option<Duration>,
}
impl ClickWalk {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            goal: None,
            pending: false,
            ready_at: None,
        }
    }
    #[must_use]
    pub const fn goal(&self) -> Option<TileCoord> {
        self.goal
    }
    pub const fn set_goal(&mut self, goal: TileCoord) {
        self.goal = Some(goal);
    }
    pub const fn cancel(&mut self) {
        self.goal = None;
    }
    pub fn next_step(&mut self, own: TileCoord) -> Option<StepDir> {
        if self.pending {
            return None;
        }
        let direction = step_toward(own, self.goal?);
        if direction.is_none() {
            self.goal = None;
        }
        self.pending = direction.is_some();
        direction
    }
    pub const fn on_result(&mut self, result: StepResult) {
        self.pending = false;
        if matches!(result, StepResult::Refused) {
            self.goal = None;
        }
    }
    pub fn next_paced_step(&mut self, own: TileCoord, now: Duration) -> Option<StepDir> {
        if self.ready_at.is_some_and(|ready| now < ready) {
            None
        } else {
            self.next_step(own)
        }
    }
    pub fn on_paced_result(&mut self, result: StepResult, now: Duration, duration: Duration) {
        if matches!(result, StepResult::Moved) {
            self.ready_at = now.checked_add(duration);
        }
        self.on_result(result);
    }
}
