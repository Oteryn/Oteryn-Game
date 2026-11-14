use super::*;
use oteryn_input_actions::KeyCode;
#[test]
fn negotiated_profile_exposes_only_native_keypad_diagonals() -> Result<(), String> {
    for (code, direction) in [
        (95, StepDir::NorthWest),
        (97, StepDir::NorthEast),
        (89, StepDir::SouthWest),
        (91, StepDir::SouthEast),
    ] {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(code).map_err(|error| format!("{error:?}"))?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(keypad_step(std::slice::from_ref(&event), false), None);
        assert_eq!(
            keypad_step(std::slice::from_ref(&event), true),
            Some(direction)
        );
    }
    assert_eq!(keypad_step(&[], true), None);
    let release = NormalizedInputEvent::Key {
        code: KeyCode::new(95).map_err(|error| format!("{error:?}"))?,
        state: ButtonState::Released,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    assert_eq!(keypad_step(&[release], true), None);
    Ok(())
}

fn arrow(code: KeyCode, state: ButtonState) -> NormalizedInputEvent {
    NormalizedInputEvent::Key {
        code,
        state,
        modifiers: Modifiers::NONE,
        repeat: false,
    }
}

#[test]
fn two_perpendicular_arrows_held_walk_diagonally() {
    let mut chord = ArrowChord::default();
    assert_eq!(
        chord.step(&[arrow(KeyCode::ARROW_UP, ButtonState::Pressed)]),
        Some(StepDir::North)
    );
    assert_eq!(
        chord.step(&[arrow(KeyCode::ARROW_LEFT, ButtonState::Pressed)]),
        Some(StepDir::NorthWest)
    );
    // An OS repeat of either held key keeps the diagonal.
    let repeat = NormalizedInputEvent::Key {
        code: KeyCode::ARROW_UP,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: true,
    };
    assert_eq!(chord.step(&[repeat]), Some(StepDir::NorthWest));
    // Releasing one key falls back to the other.
    assert_eq!(
        chord.step(&[arrow(KeyCode::ARROW_UP, ButtonState::Released)]),
        None
    );
    assert_eq!(
        chord.step(&[arrow(KeyCode::ARROW_LEFT, ButtonState::Pressed)]),
        Some(StepDir::West)
    );
}

#[test]
fn every_perpendicular_pair_maps_to_its_diagonal() {
    for (a, b, expected) in [
        (KeyCode::ARROW_UP, KeyCode::ARROW_RIGHT, StepDir::NorthEast),
        (KeyCode::ARROW_DOWN, KeyCode::ARROW_LEFT, StepDir::SouthWest),
        (
            KeyCode::ARROW_DOWN,
            KeyCode::ARROW_RIGHT,
            StepDir::SouthEast,
        ),
        (KeyCode::ARROW_RIGHT, KeyCode::ARROW_UP, StepDir::NorthEast),
    ] {
        let mut chord = ArrowChord::default();
        chord.step(&[arrow(a, ButtonState::Pressed)]);
        assert_eq!(
            chord.step(&[arrow(b, ButtonState::Pressed)]),
            Some(expected)
        );
    }
}

#[test]
fn opposing_arrows_do_not_cancel_into_a_diagonal_and_device_loss_releases_all() {
    let mut chord = ArrowChord::default();
    chord.step(&[arrow(KeyCode::ARROW_UP, ButtonState::Pressed)]);
    assert_eq!(
        chord.step(&[arrow(KeyCode::ARROW_DOWN, ButtonState::Pressed)]),
        Some(StepDir::South)
    );
    chord.step(&[NormalizedInputEvent::DeviceLost]);
    assert_eq!(
        chord.step(&[arrow(KeyCode::ARROW_LEFT, ButtonState::Pressed)]),
        Some(StepDir::West)
    );
}
