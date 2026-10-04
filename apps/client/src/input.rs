//! Mouse input logic (ADR-0020 section 5, N3): screen to tile, click to tile, click to target.
//!
//! Everything here is pure and transport-free. `StepDir` mirrors the four `step` directions
//! (north is `y - 1`); N4 maps it 1:1 onto the session's `step` command. No command, wire
//! message or server behaviour is added.
//!
//! SPEED-1 step pacing (CONDITIONS-0 §4.2, §4.3): the client reads the same checked-in step-speed
//! table as the server and waits a moved step's duration before sending the next one, so a paced
//! walk never sends a step the server would hold or refuse.

use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ContextDefinition, ContextId, ContextKind,
    InputAtom, InputChord, InputError, InputRouter, Modifiers, MouseButton, NormalizedInputEvent,
    RepeatPolicy,
};
use oteryn_renderer::{TileCoord, TileView};
use std::sync::OnceLock;
use std::time::Duration;

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

const STEP_SPEED_TABLE_JSON: &str = include_str!("../../../content/movement/step_speed_v1.json");
/// The digest the server verifies the same file against (`movement/speed.rs`).
const STEP_SPEED_TABLE_SHA256_U16LE: &str =
    "323b70ceb76edc53ed341a31d67fea5543149689ead9ad51856286e792bfe436";
/// The effective speed range the table covers (CONDITIONS-0 §4.1 clamp).
pub const SPEED_MIN: u16 = 10;
/// The ground speed of a tile whose ground item names none; every tile the client knows today
/// (MAP-WIRE-2 adds per-tile ground speed).
pub const DEFAULT_GROUND_SPEED: u16 = 150;
const SERVER_BEAT_MS: u64 = 50;

/// The checked-in step-speed table (`content/movement/step_speed_v1.json`).
#[derive(Debug)]
pub struct StepSpeedTable {
    step_speed: Vec<u16>,
}

impl StepSpeedTable {
    /// Read the table file: the pinned digest, the range and one positive u16 per speed.
    fn parse(text: &str) -> Option<Self> {
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
            .map(|value| value.trim().parse::<u16>().ok().filter(|v| *v > 0))
            .collect::<Option<Vec<u16>>>()?;
        (step_speed.len() == usize::from(u16::MAX - SPEED_MIN) + 1).then_some(Self { step_speed })
    }

    /// The embedded table, read once. `None` only if the file is malformed, which the tests rule
    /// out.
    #[must_use]
    pub fn embedded() -> Option<&'static Self> {
        static TABLE: OnceLock<Option<StepSpeedTable>> = OnceLock::new();
        TABLE
            .get_or_init(|| Self::parse(STEP_SPEED_TABLE_JSON))
            .as_ref()
    }

    /// Step duration (CONDITIONS-0 §4.2), exactly as the server computes it:
    /// `floor(1000 × ground speed / step speed)` ms rounded up to 50 ms. `None` below the speed
    /// clamp or on ground speed 0, where nothing paces.
    #[must_use]
    pub fn step_duration(&self, speed: u16, ground_speed: u16) -> Option<Duration> {
        if ground_speed == 0 {
            return None;
        }
        let index = usize::from(speed.checked_sub(SPEED_MIN)?);
        let step_speed = u64::from(*self.step_speed.get(index)?);
        let raw = 1000 * u64::from(ground_speed) / step_speed;
        Some(Duration::from_millis(
            raw.div_ceil(SERVER_BEAT_MS) * SERVER_BEAT_MS,
        ))
    }
}

/// Click-to-tile state: one goal, one outstanding step. No pathfinding over unknown terrain:
/// the walk stops on the first refusal or on arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickWalk {
    goal: Option<TileCoord>,
    pending: bool,
    /// SPEED-1: the client clock time before which the next step is not sent.
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

    /// SPEED-1: [`Self::next_step`] at client clock time `now`, holding the step until the
    /// previous moved step's duration has passed.
    pub fn next_paced_step(&mut self, own: TileCoord, now: Duration) -> Option<StepDir> {
        if self.ready_at.is_some_and(|ready| now < ready) {
            return None;
        }
        self.next_step(own)
    }

    /// SPEED-1: [`Self::on_result`] received at `now`. A moved step's `duration` (from
    /// [`StepSpeedTable::step_duration`]) passes before the next step is sent; the server's clock
    /// started when it sent the result, so a step sent after it is never early.
    pub fn on_paced_result(&mut self, result: StepResult, now: Duration, duration: Duration) {
        if matches!(result, StepResult::Moved) {
            self.ready_at = now.checked_add(duration);
        }
        self.on_result(result);
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

/// Action id of the primary-button click in the gameplay context.
pub const CLICK_ACTION: &str = "client.click";

/// A gameplay click at a window pixel, produced only by the routed `client.click` action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClickAction {
    pub x: f64,
    pub y: f64,
}

/// Routes normalized platform events (from `InputPlatformAdapter`) through `InputRouter`, so a
/// click reaches select or walk only as the gameplay `client.click` action. An active text or
/// modal context suppresses it; focus loss and device loss cancel held input in the router.
#[derive(Debug)]
pub struct MouseActions {
    router: InputRouter,
    gameplay: ContextId,
    text: ContextId,
    modal: ContextId,
    /// Last pointer position; `None` at startup and after focus or device loss.
    pointer: Option<(f64, f64)>,
}

impl MouseActions {
    pub fn new() -> Result<Self, InputError> {
        let gameplay = ContextId::new("gameplay".to_owned())?;
        let text = ContextId::new("text".to_owned())?;
        let modal = ContextId::new("modal".to_owned())?;
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
                ContextDefinition::new(modal.clone(), ContextKind::Modal, 20),
            ],
            vec![Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::NONE,
                    vec![InputAtom::Mouse(MouseButton::PRIMARY)],
                )?,
                ActionId::new(CLICK_ACTION.to_owned())?,
                RepeatPolicy::Ignore,
            )],
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        Ok(Self {
            router,
            gameplay,
            text,
            modal,
            pointer: None,
        })
    }

    /// Text entry (chat, later) suppresses gameplay clicks while active.
    pub fn set_text_active(&mut self, active: bool) -> Result<(), InputError> {
        self.router.set_context_active(&self.text, active).map(drop)
    }

    /// A modal (menu, dialog) suppresses gameplay clicks while active.
    pub fn set_modal_active(&mut self, active: bool) -> Result<(), InputError> {
        self.router
            .set_context_active(&self.modal, active)
            .map(drop)
    }

    #[must_use]
    pub const fn gameplay_context(&self) -> &ContextId {
        &self.gameplay
    }

    /// Feeds normalized events and returns the gameplay clicks they produced.
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<ClickAction> {
        let mut clicks = Vec::new();
        for event in events {
            // Only pointer, button and lifecycle events reach the click router: a held
            // keyboard key or wheel movement is irrelevant to the click chord and must not
            // block it. Text and modal contexts still suppress it inside the router.
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    self.pointer =
                        Some((f64::from(position.x().get()), f64::from(position.y().get())));
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => self.pointer = None,
                NormalizedInputEvent::Key { .. }
                | NormalizedInputEvent::Wheel { .. }
                | NormalizedInputEvent::TextCommitted(_) => continue,
                _ => {}
            }
            for action in self.router.process(event) {
                if action.action().as_str() == CLICK_ACTION
                    && action.context() == &self.gameplay
                    && action.phase() == ActionPhase::Started
                    && let Some((x, y)) = self.pointer
                {
                    clicks.push(ClickAction { x, y });
                }
            }
        }
        clicks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::ButtonState;

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
    fn the_embedded_step_speed_table_matches_the_server_and_canary() -> Result<(), &'static str> {
        let table = StepSpeedTable::embedded().ok_or("the checked-in table reads")?;
        let ms = Duration::from_millis;
        // Canary `04b83b51` samples: speed 110 → 278, 220 → 500, 10 → 9.
        assert_eq!(
            table.step_duration(110, DEFAULT_GROUND_SPEED),
            Some(ms(550))
        );
        assert_eq!(
            table.step_duration(220, DEFAULT_GROUND_SPEED),
            Some(ms(300))
        );
        assert_eq!(table.step_duration(110, 200), Some(ms(750)));
        assert_eq!(table.step_duration(SPEED_MIN, 1_000), Some(ms(111_150)));
        assert_eq!(table.step_duration(SPEED_MIN - 1, 150), None);
        assert_eq!(table.step_duration(110, 0), None);
        assert!(
            StepSpeedTable::parse(&STEP_SPEED_TABLE_JSON.replacen("323b", "423b", 1)).is_none()
        );
        assert!(
            StepSpeedTable::parse(&STEP_SPEED_TABLE_JSON.replacen("    9,", "    0,", 1)).is_none()
        );
        Ok(())
    }

    #[test]
    fn a_paced_walk_waits_each_moved_step_duration() {
        let ms = Duration::from_millis;
        let mut walk = ClickWalk::new();
        walk.set_goal(t(3, 0));
        assert_eq!(walk.next_paced_step(t(0, 0), ms(0)), Some(StepDir::East));
        walk.on_paced_result(StepResult::Moved, ms(40), ms(550));
        assert_eq!(walk.next_paced_step(t(1, 0), ms(589)), None);
        assert_eq!(walk.next_paced_step(t(1, 0), ms(590)), Some(StepDir::East));
        walk.on_paced_result(StepResult::Moved, ms(630), ms(750));
        assert_eq!(walk.next_paced_step(t(2, 0), ms(1_000)), None);
        assert_eq!(
            walk.next_paced_step(t(2, 0), ms(1_380)),
            Some(StepDir::East)
        );
        // A refusal ends the walk and leaves the clock where the last moved step put it.
        walk.on_paced_result(StepResult::Refused, ms(1_400), ms(550));
        assert_eq!(walk.next_paced_step(t(2, 0), ms(5_000)), None);
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

    fn moved(x: i32, y: i32) -> Result<NormalizedInputEvent, InputError> {
        use oteryn_input_actions::{
            PointerCoordinate, PointerDelta, PointerMotion, PointerPosition,
        };
        Ok(NormalizedInputEvent::PointerMoved {
            position: PointerPosition::new(PointerCoordinate::new(x)?, PointerCoordinate::new(y)?),
            motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
        })
    }

    fn primary(state: ButtonState) -> NormalizedInputEvent {
        NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn primary_click_becomes_a_gameplay_action_at_the_pointer() -> Result<(), InputError> {
        let mut actions = MouseActions::new()?;
        let clicks = actions.route(&[moved(100, 60)?, primary(ButtonState::Pressed)]);
        assert_eq!(clicks, vec![ClickAction { x: 100.0, y: 60.0 }]);
        assert!(actions.route(&[primary(ButtonState::Released)]).is_empty());
        Ok(())
    }

    #[test]
    fn text_and_modal_contexts_suppress_the_click_action() -> Result<(), InputError> {
        let mut actions = MouseActions::new()?;
        actions.route(&[moved(1, 1)?]);
        actions.set_text_active(true)?;
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        actions.route(&[primary(ButtonState::Released)]);
        actions.set_text_active(false)?;
        actions.set_modal_active(true)?;
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        actions.route(&[primary(ButtonState::Released)]);
        actions.set_modal_active(false)?;
        assert_eq!(actions.route(&[primary(ButtonState::Pressed)]).len(), 1);
        Ok(())
    }

    #[test]
    fn focus_loss_drops_a_held_click() -> Result<(), InputError> {
        let mut actions = MouseActions::new()?;
        assert!(
            actions
                .route(&[NormalizedInputEvent::FocusChanged { focused: false }])
                .is_empty()
        );
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        Ok(())
    }

    #[test]
    fn click_needs_a_fresh_pointer_after_startup_and_after_loss() -> Result<(), InputError> {
        let mut actions = MouseActions::new()?;
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        actions.route(&[primary(ButtonState::Released), moved(10, 20)?]);
        assert_eq!(actions.route(&[primary(ButtonState::Pressed)]).len(), 1);
        actions.route(&[primary(ButtonState::Released)]);
        actions.route(&[NormalizedInputEvent::FocusChanged { focused: false }]);
        actions.route(&[NormalizedInputEvent::FocusChanged { focused: true }]);
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        actions.route(&[primary(ButtonState::Released), moved(30, 40)?]);
        assert_eq!(
            actions.route(&[primary(ButtonState::Pressed)]),
            vec![ClickAction { x: 30.0, y: 40.0 }]
        );
        actions.route(&[
            primary(ButtonState::Released),
            NormalizedInputEvent::DeviceLost,
        ]);
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        Ok(())
    }

    fn key_w(state: ButtonState) -> NormalizedInputEvent {
        NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::KEY_W,
            state,
            modifiers: Modifiers::NONE,
            repeat: false,
        }
    }

    #[test]
    fn a_held_unrelated_key_does_not_block_the_click_but_contexts_still_suppress()
    -> Result<(), InputError> {
        let mut actions = MouseActions::new()?;
        actions.route(&[moved(5, 6)?, key_w(ButtonState::Pressed)]);
        assert_eq!(actions.route(&[primary(ButtonState::Pressed)]).len(), 1);
        actions.route(&[primary(ButtonState::Released)]);
        actions.set_modal_active(true)?;
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        actions.route(&[primary(ButtonState::Released)]);
        actions.set_modal_active(false)?;
        actions.set_text_active(true)?;
        assert!(actions.route(&[primary(ButtonState::Pressed)]).is_empty());
        Ok(())
    }
}
