use super::*;
#[test]
fn wasd_preset_preserves_arrows_through_both_input_stages() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_input_actions::ButtonState;
    for modern_wasd in [false, true] {
        let mut state = super::super::SettingsState::new(None);
        state.preferences.controls.movement = if modern_wasd {
            super::super::MovementKeys::ArrowsAndWasd
        } else {
            super::super::MovementKeys::Arrows
        };
        state.apply_chat_preset(false)?;
        for (physical, expected) in [
            (26, 82),
            (7, 79),
            (22, 81),
            (4, 80),
            (82, 82),
            (79, 79),
            (81, 81),
            (80, 80),
        ] {
            let code = KeyCode::new(physical)?;
            let gesture = [ButtonState::Pressed, ButtonState::Released].map(|state| {
                NormalizedInputEvent::Key {
                    code,
                    modifiers: Modifiers::NONE,
                    state,
                    repeat: false,
                }
            });
            let controls = state.control_events_at(&gesture, false, std::time::Duration::ZERO);
            let routed = state.input_in_context(&controls, false).0;
            assert!(
                matches!(routed.as_slice(), [NormalizedInputEvent::Key { code, state: ButtonState::Pressed, .. }, NormalizedInputEvent::Key { state: ButtonState::Released, .. }] if code.get() == expected),
                "physical key {physical}, modern WASD {modern_wasd}"
            );
        }
    }
    Ok(())
}
#[test]
fn executable_hotkeys_repeat_at_the_control_interval_and_release_cleanly()
-> Result<(), Box<dyn std::error::Error>> {
    use super::super::{CustomAction, ObjectTarget, SettingsState};
    use oteryn_input_actions::{ButtonState, Modifier};
    for legacy_delay in [false, true] {
        for (code, action) in [
            (
                68,
                CustomAction::Object {
                    definition: 900,
                    sub_type: 0,
                    target: ObjectTarget::Use,
                },
            ),
            (67, CustomAction::ActionBar { slot: 1 }),
        ] {
            let mut state = SettingsState::new(None);
            if legacy_delay {
                state.preferences.keyboard_delay_ms = 100;
            } else {
                state.preferences.controls.keyboard_delay_ms = 100;
            }
            if matches!(action, CustomAction::ActionBar { .. }) {
                state
                    .preferences
                    .action_keys
                    .0
                    .insert(0, "SUPER+F10".into());
            } else {
                state
                    .preferences
                    .assign_custom_hotkey("SUPER+F11", action.clone())?;
            }
            let code = KeyCode::new(code)?;
            let key = |state, repeat, modifiers| NormalizedInputEvent::Key {
                code,
                state,
                repeat,
                modifiers,
            };
            let mods = Modifiers::one(Modifier::Super);
            for (millis, repeat, expected) in [
                (0, false, 1),
                (99, true, 0),
                (100, true, 1),
                (101, false, 0),
                (201, true, 1),
            ] {
                let controls = state.control_events_at(
                    &[key(ButtonState::Pressed, repeat, mods)],
                    false,
                    std::time::Duration::from_millis(millis),
                );
                assert!(state.input_in_context(&controls, false).0.is_empty());
                let actions = state.take_custom_actions();
                assert_eq!(actions.len(), expected, "at {millis} ms for {action:?}");
                assert!(actions.iter().all(|value| value == &action));
            }
            state.preferences.custom_hotkeys.clear();
            state.preferences.action_keys.0.clear();
            let controls = state.control_events_at(
                &[key(ButtonState::Released, false, Modifiers::NONE)],
                false,
                std::time::Duration::from_millis(202),
            );
            assert!(state.input_in_context(&controls, false).0.is_empty());
            assert!(state.take_custom_actions().is_empty());
            let controls = state.control_events_at(
                &[key(ButtonState::Pressed, false, mods)],
                false,
                std::time::Duration::from_millis(203),
            );
            assert_eq!(state.input_in_context(&controls, false).0.len(), 1);
            assert!(state.take_custom_actions().is_empty());
        }
    }
    Ok(())
}
