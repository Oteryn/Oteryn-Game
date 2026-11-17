//! Party shortcuts and world context input, shared by the shell and synthetic scenarios.
use oteryn_input_actions::{ButtonState, Modifier, MouseButton, NormalizedInputEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Context(i32, i32),
}

#[derive(Debug, Default)]
pub struct PartyInput {
    pointer: Option<(i32, i32)>,
    secondary_down: bool,
}

impl PartyInput {
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<Action> {
        let mut actions = Vec::new();
        for event in events {
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    self.pointer = Some((position.x().get(), position.y().get()));
                }
                NormalizedInputEvent::MouseButton {
                    button,
                    state,
                    modifiers,
                } if *button == MouseButton::SECONDARY => {
                    let pressed = *state == ButtonState::Pressed;
                    if pressed
                        && !self.secondary_down
                        && *modifiers == Default::default()
                        && let Some((x, y)) = self.pointer
                    {
                        actions.push(Action::Context(x, y));
                    }
                    self.secondary_down = pressed;
                }
                event if open_key(std::slice::from_ref(event)) => actions.push(Action::Open),
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => {
                    self.pointer = None;
                    self.secondary_down = false;
                }
                _ => {}
            }
        }
        actions
    }
}

/// Ctrl+Shift+G opens Party without stealing Prey or Weapon Proficiency.
#[must_use]
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 10 && modifiers.contains(Modifier::Control)
            && modifiers.contains(Modifier::Shift)
            && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Super))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::{
        InputError, KeyCode, Modifiers, PointerCoordinate, PointerDelta, PointerMotion,
        PointerPosition,
    };

    #[test]
    fn shortcut_and_context_use_pressed_events_and_forget_lost_pointer() -> Result<(), InputError> {
        let mut input = PartyInput::default();
        let key = NormalizedInputEvent::Key {
            code: KeyCode::new(10)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
            repeat: false,
        };
        assert_eq!(input.route(&[key]), vec![Action::Open]);
        let pointer = NormalizedInputEvent::PointerMoved {
            position: PointerPosition::new(
                PointerCoordinate::new(10)?,
                PointerCoordinate::new(20)?,
            ),
            motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
        };
        let click = NormalizedInputEvent::MouseButton {
            button: MouseButton::SECONDARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
        };
        assert_eq!(
            input.route(&[pointer, click.clone(), click.clone()]),
            vec![Action::Context(10, 20)]
        );
        assert!(
            input
                .route(&[NormalizedInputEvent::DeviceLost, click])
                .is_empty()
        );
        Ok(())
    }
}
