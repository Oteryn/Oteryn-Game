//! Input compatibility facade and session-specific world-selection glue.

pub use oteryn_client_domain::movement::{StepDir, StepResult};
pub use oteryn_client_simulation::movement::{
    ClickWalk, DEFAULT_GROUND_SPEED, SPEED_MIN, StepSpeedTable, step_toward,
};
pub use oteryn_client_ui::logout::logout_key;
pub use oteryn_client_ui::paced_movement::arrow_step;
pub use oteryn_client_ui::world_interaction::{
    CLICK_ACTION, ClickAction, MouseActions, click_tile,
};

use oteryn_renderer::TileCoord;
use oteryn_session::EntityRef;

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
    /// A server entity or object glyph is drawn there.
    pub glyph: bool,
    /// The server entity drawn there, when it is one a click may attack.
    pub entity: Option<EntityRef>,
}
/// Selects an entity before an object; the last candidate of a kind wins, matching draw order.
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
    use oteryn_input_actions::{
        ButtonState, InputError, KeyCode, Modifier, Modifiers, MouseButton, NormalizedInputEvent,
    };
    use oteryn_renderer::{TileCoord, TileView};
    use std::time::Duration;

    const KEY_L: u16 = 15;
    const KEY_Q: u16 = 20;
    const STEP_SPEED_TABLE_JSON: &str =
        include_str!("../../../content/movement/step_speed_v1.json");

    fn t(x: i32, y: i32) -> TileCoord {
        TileCoord::new(x, y)
    }

    fn view() -> Result<TileView, oteryn_renderer::BatchError> {
        TileView::new(t(-7, -5), 48, 15, 11)
    }

    #[test]
    fn a_step_moves_one_tile_north_is_y_minus_one() {
        assert_eq!(StepDir::North.from(t(5, 5)), t(5, 4));
        assert_eq!(StepDir::East.from(t(5, 5)), t(6, 5));
        assert_eq!(StepDir::South.from(t(5, 5)), t(5, 6));
        assert_eq!(StepDir::West.from(t(i32::MIN, 0)), t(i32::MIN, 0));
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
                glyph: false,
                entity: None,
            },
            Targetable {
                tile: t(1, 1),
                kind: TargetKind::Entity,
                glyph: false,
                entity: None,
            },
            Targetable {
                tile: t(2, 2),
                kind: TargetKind::Object,
                glyph: false,
                entity: None,
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

    #[test]
    fn arrow_keys_map_to_steps_and_other_keys_to_none() {
        let key = |code, state| NormalizedInputEvent::Key {
            code,
            state,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(
            arrow_step(&[key(KeyCode::ARROW_LEFT, ButtonState::Pressed)]),
            Some(StepDir::West)
        );
        assert_eq!(
            arrow_step(&[key(KeyCode::ARROW_UP, ButtonState::Pressed)]),
            Some(StepDir::North)
        );
        assert_eq!(
            arrow_step(&[key(KeyCode::ARROW_UP, ButtonState::Released)]),
            None
        );
        assert_eq!(
            arrow_step(&[key(KeyCode::KEY_W, ButtonState::Pressed)]),
            None
        );
    }

    #[test]
    fn ctrl_l_and_ctrl_q_ask_for_logout_once() -> Result<(), InputError> {
        let key = |code, modifiers, repeat| NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            repeat,
        };
        let ctrl = Modifiers::one(Modifier::Control);
        assert!(logout_key(&[key(KeyCode::new(KEY_L)?, ctrl, false)]));
        assert!(logout_key(&[key(KeyCode::new(KEY_Q)?, ctrl, false)]));
        assert!(!logout_key(&[key(
            KeyCode::new(KEY_L)?,
            Modifiers::NONE,
            false
        )]));
        assert!(!logout_key(&[key(KeyCode::new(KEY_Q)?, ctrl, true)]));
        assert!(!logout_key(&[key(KeyCode::KEY_W, ctrl, false)]));
        Ok(())
    }
}
