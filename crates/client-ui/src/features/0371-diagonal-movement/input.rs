//! Native keypad diagonals; the caller gates these bindings on the negotiated profile.
use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
pub fn default_diagonal_binding() {}
pub fn keypad_step(events: &[NormalizedInputEvent], selected: bool) -> Option<StepDir> {
    if !selected {
        return None;
    }
    events.iter().find_map(|event| match event {
        NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            ..
        } if *modifiers == Modifiers::NONE => match code.get() {
            // Stable HID mappings already owned by input-platform::winit_adapter.
            95 => Some(StepDir::NorthWest),
            97 => Some(StepDir::NorthEast),
            89 => Some(StepDir::SouthWest),
            91 => Some(StepDir::SouthEast),
            _ => None,
        },
        _ => None,
    })
}

/// Held arrow keys (up, right, down, left) and when each was pressed, so that two perpendicular
/// arrows held together walk diagonally, as OTClient's game_walk does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArrowChord {
    pressed_at: [Option<u64>; 4],
    clock: u64,
}

impl ArrowChord {
    const KEYS: [KeyCode; 4] = [
        KeyCode::ARROW_UP,
        KeyCode::ARROW_RIGHT,
        KeyCode::ARROW_DOWN,
        KeyCode::ARROW_LEFT,
    ];

    /// Tracks the arrow transitions of `events` (a lost device releases every key) and returns
    /// the step for the last arrow press among them, including a platform repeat.
    pub fn step(&mut self, events: &[NormalizedInputEvent]) -> Option<StepDir> {
        let mut pressed = None;
        for event in events {
            match event {
                NormalizedInputEvent::DeviceLost => self.pressed_at = [None; 4],
                NormalizedInputEvent::Key { code, state, .. } => {
                    let Some(key) = Self::KEYS.iter().position(|arrow| arrow == code) else {
                        continue;
                    };
                    if *state == ButtonState::Pressed {
                        self.clock += 1;
                        self.pressed_at[key] = Some(self.clock);
                        pressed = Some(key);
                    } else {
                        self.pressed_at[key] = None;
                    }
                }
                _ => {}
            }
        }
        let key = pressed?;
        // The newest held arrow perpendicular to the one just pressed joins it.
        let partner = [(key + 1) % 4, (key + 3) % 4]
            .into_iter()
            .filter_map(|other| Some((self.pressed_at[other]?, other)))
            .max()
            .map(|(_, other)| other);
        Some(match (key, partner) {
            (0, Some(1)) | (1, Some(0)) => StepDir::NorthEast,
            (0, Some(3)) | (3, Some(0)) => StepDir::NorthWest,
            (2, Some(1)) | (1, Some(2)) => StepDir::SouthEast,
            (2, Some(3)) | (3, Some(2)) => StepDir::SouthWest,
            (0, _) => StepDir::North,
            (1, _) => StepDir::East,
            (2, _) => StepDir::South,
            _ => StepDir::West,
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
