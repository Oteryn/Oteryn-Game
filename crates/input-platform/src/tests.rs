use super::adapter::{PlatformButtonState, PlatformEvent, PlatformWheelUnit};
use super::{InputPlatformAdapter, InputPlatformError};
use oteryn_input_actions::{
    ActionEvent, ActionId, ActionPhase, Binding, BindingMap, ButtonState, ContextDefinition,
    ContextId, ContextKind, InputAtom, InputChord, InputError, InputRouter, KeyCode,
    MAX_TEXT_BYTES, Modifier, Modifiers, MouseButton, NormalizedInputEvent, RepeatPolicy,
    WheelDirection,
};
use std::error::Error;

fn action(value: &str) -> Result<ActionId, InputError> {
    ActionId::new(value.to_owned())
}

fn context(value: &str) -> Result<ContextId, InputError> {
    ContextId::new(value.to_owned())
}

fn process_all(router: &mut InputRouter, events: &[NormalizedInputEvent]) -> Vec<ActionEvent> {
    let mut output = Vec::new();
    for event in events {
        output.extend(router.process(event));
    }
    output
}

fn key_event(code: u16, state: PlatformButtonState) -> PlatformEvent<'static> {
    PlatformEvent::Key {
        code: Some(code),
        state,
        repeat: false,
        text: None,
        synthetic: false,
    }
}

fn router_with_key_mouse_and_wheel() -> Result<InputRouter, InputError> {
    let gameplay = context("gameplay")?;
    let map = BindingMap::new(
        vec![ContextDefinition::new(
            gameplay.clone(),
            ContextKind::Gameplay,
            0,
        )],
        vec![
            Binding::new(
                gameplay.clone(),
                InputChord::new(Modifiers::NONE, vec![InputAtom::Key(KeyCode::KEY_W)])?,
                action("move.forward")?,
                RepeatPolicy::Allow,
            ),
            Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::NONE,
                    vec![InputAtom::Mouse(MouseButton::PRIMARY)],
                )?,
                action("pointer.primary")?,
                RepeatPolicy::Ignore,
            ),
            Binding::new(
                gameplay.clone(),
                InputChord::new(Modifiers::NONE, vec![InputAtom::Wheel(WheelDirection::Up)])?,
                action("wheel.up")?,
                RepeatPolicy::Ignore,
            ),
        ],
        &[],
    )?;
    let mut router = InputRouter::new(map);
    router.set_context_active(&gameplay, true)?;
    Ok(router)
}

#[test]
fn keyboard_and_non_ime_text_are_distinct_events() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let events = adapter.process_platform_event(PlatformEvent::Key {
        code: Some(26),
        state: PlatformButtonState::Pressed,
        repeat: false,
        text: Some("private-marker"),
        synthetic: false,
    })?;

    assert_eq!(events.len(), 2);
    match &events[0] {
        NormalizedInputEvent::Key {
            code,
            state,
            modifiers,
            repeat,
        } => {
            assert_eq!(*code, KeyCode::KEY_W);
            assert_eq!(*state, ButtonState::Pressed);
            assert_eq!(*modifiers, Modifiers::NONE);
            assert!(!repeat);
        }
        other => assert!(matches!(other, NormalizedInputEvent::Key { .. })),
    }
    assert!(matches!(&events[1], NormalizedInputEvent::TextCommitted(_)));
    let debug = format!("{:?}", events[1]);
    assert!(!debug.contains("private-marker"));
    assert!(debug.contains("bytes"));
    Ok(())
}

#[test]
fn synthetic_and_background_input_are_ignored_before_validation() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let synthetic = adapter.process_platform_event(PlatformEvent::Key {
        code: None,
        state: PlatformButtonState::Pressed,
        repeat: false,
        text: None,
        synthetic: true,
    })?;
    assert!(synthetic.is_empty());

    adapter.process_platform_event(PlatformEvent::Focus(false))?;
    assert!(
        adapter
            .process_platform_event(PlatformEvent::Key {
                code: None,
                state: PlatformButtonState::Pressed,
                repeat: false,
                text: Some("ignored"),
                synthetic: false,
            })?
            .is_empty()
    );
    assert!(
        adapter
            .process_platform_event(PlatformEvent::MouseButton {
                button: None,
                state: PlatformButtonState::Pressed,
            })?
            .is_empty()
    );
    assert!(
        adapter
            .process_platform_event(PlatformEvent::PointerPosition {
                x: f64::NAN,
                y: 0.0,
            })?
            .is_empty()
    );
    assert!(
        adapter
            .process_platform_event(PlatformEvent::Wheel {
                horizontal: f64::INFINITY,
                vertical: 0.0,
                unit: PlatformWheelUnit::Pixels,
            })?
            .is_empty()
    );
    let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
    assert!(
        adapter
            .process_platform_event(PlatformEvent::TextCommit(&oversized))?
            .is_empty()
    );
    Ok(())
}

#[test]
fn unsupported_controls_reset_router_state_without_aliasing() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router_with_key_mouse_and_wheel()?;

    let pressed = adapter.process_platform_event(key_event(
        KeyCode::KEY_W.get(),
        PlatformButtonState::Pressed,
    ))?;
    let started = process_all(&mut router, &pressed);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].phase(), ActionPhase::Started);

    for unsupported in [
        PlatformEvent::Key {
            code: None,
            state: PlatformButtonState::Released,
            repeat: false,
            text: None,
            synthetic: false,
        },
        PlatformEvent::Key {
            code: Some(4_096),
            state: PlatformButtonState::Pressed,
            repeat: false,
            text: None,
            synthetic: false,
        },
        PlatformEvent::MouseButton {
            button: None,
            state: PlatformButtonState::Pressed,
        },
        PlatformEvent::MouseButton {
            button: Some(17),
            state: PlatformButtonState::Pressed,
        },
    ] {
        let reset = adapter.process_platform_event(unsupported)?;
        assert_eq!(reset, vec![NormalizedInputEvent::DeviceLost]);
        process_all(&mut router, &reset);
        assert!(router.held_inputs().is_empty());
        assert_eq!(router.modifiers(), Modifiers::NONE);
        assert!(!router.captured());
    }
    Ok(())
}

#[test]
fn side_specific_modifier_transitions_override_stale_snapshots() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    adapter.process_platform_event(PlatformEvent::Modifiers { bits: 0 })?;

    let left_pressed =
        adapter.process_platform_event(key_event(225, PlatformButtonState::Pressed))?;
    match &left_pressed[0] {
        NormalizedInputEvent::Key { modifiers, .. } => {
            assert_eq!(*modifiers, Modifiers::one(Modifier::Shift));
        }
        other => assert!(matches!(other, NormalizedInputEvent::Key { .. })),
    }

    adapter.process_platform_event(key_event(229, PlatformButtonState::Pressed))?;
    let left_released =
        adapter.process_platform_event(key_event(225, PlatformButtonState::Released))?;
    match &left_released[0] {
        NormalizedInputEvent::Key { modifiers, .. } => {
            assert_eq!(*modifiers, Modifiers::one(Modifier::Shift));
        }
        other => assert!(matches!(other, NormalizedInputEvent::Key { .. })),
    }

    let right_released =
        adapter.process_platform_event(key_event(229, PlatformButtonState::Released))?;
    match &right_released[0] {
        NormalizedInputEvent::Key { modifiers, .. } => {
            assert_eq!(*modifiers, Modifiers::NONE);
        }
        other => assert!(matches!(other, NormalizedInputEvent::Key { .. })),
    }
    assert_eq!(adapter.modifiers()?, Modifiers::NONE);
    Ok(())
}

#[test]
fn pointer_values_are_bounded_and_large_jumps_rebase() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let first =
        adapter.process_platform_event(PlatformEvent::PointerPosition { x: 10.4, y: 20.6 })?;
    match &first[0] {
        NormalizedInputEvent::PointerMoved { position, motion } => {
            assert_eq!(position.x().get(), 10);
            assert_eq!(position.y().get(), 21);
            assert_eq!(motion.x().get(), 0);
            assert_eq!(motion.y().get(), 0);
        }
        other => assert!(matches!(other, NormalizedInputEvent::PointerMoved { .. })),
    }

    let second =
        adapter.process_platform_event(PlatformEvent::PointerPosition { x: 15.0, y: 19.0 })?;
    match &second[0] {
        NormalizedInputEvent::PointerMoved { motion, .. } => {
            assert_eq!(motion.x().get(), 5);
            assert_eq!(motion.y().get(), -2);
        }
        other => assert!(matches!(other, NormalizedInputEvent::PointerMoved { .. })),
    }

    let rebased = adapter.process_platform_event(PlatformEvent::PointerPosition {
        x: 200_000.0,
        y: 19.0,
    })?;
    match &rebased[0] {
        NormalizedInputEvent::PointerMoved { position, motion } => {
            assert_eq!(position.x().get(), 200_000);
            assert_eq!(motion.x().get(), 0);
            assert_eq!(motion.y().get(), 0);
        }
        other => assert!(matches!(other, NormalizedInputEvent::PointerMoved { .. })),
    }

    adapter.set_capture_state(true)?;
    let relative =
        adapter.process_platform_event(PlatformEvent::PointerMotion { x: 4.4, y: -3.6 })?;
    match &relative[0] {
        NormalizedInputEvent::PointerMoved { position, motion } => {
            assert_eq!(position.x().get(), 200_000);
            assert_eq!(motion.x().get(), 4);
            assert_eq!(motion.y().get(), -4);
        }
        other => assert!(matches!(other, NormalizedInputEvent::PointerMoved { .. })),
    }

    adapter.set_capture_state(false)?;
    adapter.set_capture_state(true)?;
    assert_eq!(
        adapter.process_platform_event(PlatformEvent::PointerMotion { x: 1.0, y: 1.0 }),
        Err(InputPlatformError::RelativeMotionUnavailable)
    );
    assert_eq!(
        adapter.process_platform_event(PlatformEvent::PointerPosition {
            x: f64::NAN,
            y: 0.0,
        }),
        Err(InputPlatformError::NonFiniteValue)
    );
    assert_eq!(
        adapter.process_platform_event(PlatformEvent::PointerPosition {
            x: 1_000_001.0,
            y: 0.0,
        }),
        Err(InputPlatformError::Contract(
            InputError::CoordinateOutOfRange
        ))
    );
    Ok(())
}

#[test]
fn wheel_units_are_deterministic_bounded_impulses() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let lines = adapter.process_platform_event(PlatformEvent::Wheel {
        horizontal: 1.0,
        vertical: -2.0,
        unit: PlatformWheelUnit::Lines,
    })?;
    match &lines[0] {
        NormalizedInputEvent::Wheel { delta, modifiers } => {
            assert_eq!(delta.horizontal(), 120);
            assert_eq!(delta.vertical(), -240);
            assert_eq!(*modifiers, Modifiers::NONE);
        }
        other => assert!(matches!(other, NormalizedInputEvent::Wheel { .. })),
    }
    assert!(
        adapter
            .process_platform_event(PlatformEvent::Wheel {
                horizontal: 0.001,
                vertical: 0.0,
                unit: PlatformWheelUnit::Lines,
            })?
            .is_empty()
    );
    assert_eq!(
        adapter.process_platform_event(PlatformEvent::Wheel {
            horizontal: 1_001.0,
            vertical: 0.0,
            unit: PlatformWheelUnit::Lines,
        }),
        Err(InputPlatformError::Contract(
            InputError::WheelDeltaOutOfRange
        ))
    );
    Ok(())
}

#[test]
fn ime_commit_is_bounded_and_never_reclassified_as_a_key() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    adapter.process_platform_event(PlatformEvent::ImeEnabled(true))?;
    adapter.process_platform_event(PlatformEvent::ImePreedit)?;
    let key = adapter.process_platform_event(PlatformEvent::Key {
        code: Some(4),
        state: PlatformButtonState::Pressed,
        repeat: false,
        text: Some("suppressed"),
        synthetic: false,
    })?;
    assert_eq!(key.len(), 1);
    assert!(matches!(&key[0], NormalizedInputEvent::Key { .. }));

    let commit = adapter.process_platform_event(PlatformEvent::TextCommit("private-ime"))?;
    assert_eq!(commit.len(), 1);
    assert!(matches!(&commit[0], NormalizedInputEvent::TextCommitted(_)));
    assert!(!format!("{:?}", commit[0]).contains("private-ime"));

    let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
    assert_eq!(
        adapter.process_platform_event(PlatformEvent::TextCommit(&oversized)),
        Err(InputPlatformError::Contract(InputError::TextTooLong {
            max: MAX_TEXT_BYTES,
            actual: MAX_TEXT_BYTES + 1,
        }))
    );

    adapter.process_platform_event(PlatformEvent::ImeEnabled(false))?;
    let non_ime = adapter.process_platform_event(PlatformEvent::Key {
        code: Some(5),
        state: PlatformButtonState::Pressed,
        repeat: false,
        text: Some("b"),
        synthetic: false,
    })?;
    assert_eq!(non_ime.len(), 2);
    Ok(())
}

#[test]
fn focus_loss_clears_router_state_and_is_duplicate_safe() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router_with_key_mouse_and_wheel()?;

    let pressed = adapter.process_platform_event(key_event(
        KeyCode::KEY_W.get(),
        PlatformButtonState::Pressed,
    ))?;
    let started = process_all(&mut router, &pressed);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].phase(), ActionPhase::Started);

    let lost = adapter.process_platform_event(PlatformEvent::Focus(false))?;
    let cancelled = process_all(&mut router, &lost);
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
    assert!(router.held_inputs().is_empty());
    assert_eq!(router.modifiers(), Modifiers::NONE);
    assert!(!router.focused());
    assert!(
        adapter
            .process_platform_event(PlatformEvent::Focus(false))?
            .is_empty()
    );
    Ok(())
}

#[test]
fn capture_loss_clears_mouse_actions_and_lifecycle_order_is_explicit() -> Result<(), Box<dyn Error>>
{
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router_with_key_mouse_and_wheel()?;

    let captured = adapter.set_capture_state(true)?;
    process_all(&mut router, &captured);
    assert!(router.captured());
    assert!(adapter.set_capture_state(true)?.is_empty());

    let mouse = adapter.process_platform_event(PlatformEvent::MouseButton {
        button: Some(MouseButton::PRIMARY.get()),
        state: PlatformButtonState::Pressed,
    })?;
    let started = process_all(&mut router, &mouse);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].phase(), ActionPhase::Started);

    let released = adapter.set_capture_state(false)?;
    let cancelled = process_all(&mut router, &released);
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
    assert!(router.held_inputs().is_empty());
    assert!(!router.captured());

    adapter.set_capture_state(true)?;
    let focus_loss = adapter.process_platform_event(PlatformEvent::Focus(false))?;
    assert_eq!(
        focus_loss,
        vec![
            NormalizedInputEvent::CaptureChanged { captured: false },
            NormalizedInputEvent::FocusChanged { focused: false },
        ]
    );
    assert_eq!(
        adapter.set_capture_state(true),
        Err(InputPlatformError::CaptureWhileUnfocused)
    );
    adapter.process_platform_event(PlatformEvent::Focus(true))?;
    assert!(
        adapter
            .process_platform_event(PlatformEvent::Focus(true))?
            .is_empty()
    );
    Ok(())
}

#[test]
fn device_loss_cancels_all_router_state_without_a_device_identifier() -> Result<(), Box<dyn Error>>
{
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router_with_key_mouse_and_wheel()?;
    let key = adapter.process_platform_event(key_event(
        KeyCode::KEY_W.get(),
        PlatformButtonState::Pressed,
    ))?;
    process_all(&mut router, &key);
    assert!(!router.held_inputs().is_empty());

    let device_lost = adapter.notify_device_lost();
    assert_eq!(device_lost, vec![NormalizedInputEvent::DeviceLost]);
    let cancelled = process_all(&mut router, &device_lost);
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
    assert!(router.held_inputs().is_empty());
    assert!(!router.captured());
    assert_eq!(router.modifiers(), Modifiers::NONE);
    assert_eq!(
        adapter.notify_device_lost(),
        vec![NormalizedInputEvent::DeviceLost]
    );
    Ok(())
}

#[test]
fn wheel_output_routes_as_reachable_start_end_impulse() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router_with_key_mouse_and_wheel()?;
    let wheel = adapter.process_platform_event(PlatformEvent::Wheel {
        horizontal: 0.0,
        vertical: 1.0,
        unit: PlatformWheelUnit::Lines,
    })?;
    let output = process_all(&mut router, &wheel);
    assert_eq!(output.len(), 2);
    assert_eq!(output[0].phase(), ActionPhase::Started);
    assert_eq!(output[1].phase(), ActionPhase::Ended);
    assert!(router.held_inputs().is_empty());
    Ok(())
}
