//! Framework-neutral normalized input and semantic action contracts.
//!
//! This crate owns no platform adapter, UI widget, gameplay command, settings
//! persistence, default product keymap or application composition.

mod error;
mod physical;
mod router;
mod semantic;
mod text;

pub use error::InputError;
pub use physical::{
    ButtonState, InputAtom, KeyCode, MAX_KEY_CODE, MAX_MOUSE_BUTTON, MAX_POINTER_COORDINATE,
    MAX_POINTER_DELTA, MAX_WHEEL_DELTA, Modifier, Modifiers, MouseButton, NormalizedInputEvent,
    PointerCoordinate, PointerDelta, PointerMotion, PointerPosition, WheelDelta, WheelDirection,
};
pub use router::{ActionEvent, ActionPhase, InputRouter};
pub use semantic::{
    ActionId, Binding, BindingMap, ContextDefinition, ContextId, ContextKind, InputChord,
    MAX_CHORD_INPUTS, MAX_IDENTIFIER_BYTES, RepeatPolicy,
};
pub use text::{MAX_TEXT_BYTES, TextCommit};

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn action(value: &str) -> Result<ActionId, InputError> {
        ActionId::new(value.to_owned())
    }

    fn context(value: &str) -> Result<ContextId, InputError> {
        ContextId::new(value.to_owned())
    }

    fn key_chord(code: KeyCode) -> Result<InputChord, InputError> {
        InputChord::new(Modifiers::NONE, vec![InputAtom::Key(code)])
    }

    fn event_key(
        code: KeyCode,
        state: ButtonState,
        modifiers: Modifiers,
        repeat: bool,
    ) -> NormalizedInputEvent {
        NormalizedInputEvent::Key {
            code,
            state,
            modifiers,
            repeat,
        }
    }

    fn context_map() -> Result<BindingMap, InputError> {
        let global = context("global")?;
        let gameplay = context("gameplay")?;
        let text = context("text")?;
        let modal = context("modal")?;
        let chord = key_chord(KeyCode::KEY_A)?;
        BindingMap::new(
            vec![
                ContextDefinition::new(global.clone(), ContextKind::Global, 0),
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 10),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
                ContextDefinition::new(modal.clone(), ContextKind::Modal, 10),
            ],
            vec![
                Binding::new(
                    global,
                    chord.clone(),
                    action("global.action")?,
                    RepeatPolicy::Ignore,
                ),
                Binding::new(
                    gameplay,
                    chord.clone(),
                    action("gameplay.action")?,
                    RepeatPolicy::Ignore,
                ),
                Binding::new(
                    text,
                    chord.clone(),
                    action("text.action")?,
                    RepeatPolicy::Ignore,
                ),
                Binding::new(modal, chord, action("modal.action")?, RepeatPolicy::Ignore),
            ],
            &[],
        )
    }

    #[test]
    fn modifiers_and_chords_normalize_deterministically() -> Result<(), InputError> {
        let modifiers = Modifiers::one(Modifier::Alt)
            .with(Modifier::Control)
            .with(Modifier::Shift);
        let ordered: Vec<Modifier> = modifiers.iter().collect();
        assert_eq!(
            ordered,
            vec![Modifier::Shift, Modifier::Control, Modifier::Alt]
        );

        let chord = InputChord::new(
            modifiers,
            vec![
                InputAtom::Key(KeyCode::KEY_B),
                InputAtom::Mouse(MouseButton::PRIMARY),
                InputAtom::Key(KeyCode::KEY_A),
            ],
        )?;
        assert_eq!(
            chord.inputs(),
            &[
                InputAtom::Key(KeyCode::KEY_A),
                InputAtom::Key(KeyCode::KEY_B),
                InputAtom::Mouse(MouseButton::PRIMARY),
            ]
        );
        assert_eq!(
            InputChord::new(
                Modifiers::NONE,
                vec![
                    InputAtom::Key(KeyCode::KEY_A),
                    InputAtom::Key(KeyCode::KEY_A),
                ],
            ),
            Err(InputError::DuplicateChordInput)
        );
        Ok(())
    }

    #[test]
    fn conflicts_reserved_chords_and_unknown_contexts_fail_explicitly() -> Result<(), InputError> {
        let gameplay = context("gameplay")?;
        let chord = key_chord(KeyCode::KEY_C)?;
        let definition = ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0);
        let first = Binding::new(
            gameplay.clone(),
            chord.clone(),
            action("first")?,
            RepeatPolicy::Ignore,
        );
        let second = Binding::new(
            gameplay.clone(),
            chord.clone(),
            action("second")?,
            RepeatPolicy::Ignore,
        );
        assert_eq!(
            BindingMap::new(vec![definition.clone()], vec![first.clone(), second], &[]),
            Err(InputError::ConflictingBinding)
        );
        assert_eq!(
            BindingMap::new(vec![definition], vec![first], std::slice::from_ref(&chord)),
            Err(InputError::ReservedBinding)
        );
        assert_eq!(
            BindingMap::new(
                Vec::new(),
                vec![Binding::new(
                    gameplay,
                    chord,
                    action("unknown")?,
                    RepeatPolicy::Ignore,
                )],
                &[],
            ),
            Err(InputError::UnknownBindingContext)
        );
        Ok(())
    }

    #[test]
    fn modal_and_text_contexts_suppress_gameplay_deterministically() -> Result<(), Box<dyn Error>> {
        let mut router = InputRouter::new(context_map()?);
        let gameplay = context("gameplay")?;
        let text = context("text")?;
        let modal = context("modal")?;
        router.set_context_active(&gameplay, true)?;

        let started = router.process(&event_key(
            KeyCode::KEY_A,
            ButtonState::Pressed,
            Modifiers::NONE,
            false,
        ));
        assert_eq!(started[0].action().as_str(), "gameplay.action");

        let cancelled = router.set_context_active(&text, true)?;
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
        assert_eq!(cancelled[0].action().as_str(), "gameplay.action");
        router.process(&event_key(
            KeyCode::KEY_A,
            ButtonState::Released,
            Modifiers::NONE,
            false,
        ));

        let text_started = router.process(&event_key(
            KeyCode::KEY_A,
            ButtonState::Pressed,
            Modifiers::NONE,
            false,
        ));
        assert_eq!(text_started[0].action().as_str(), "text.action");

        let text_cancelled = router.set_context_active(&modal, true)?;
        assert_eq!(text_cancelled[0].action().as_str(), "text.action");
        router.process(&event_key(
            KeyCode::KEY_A,
            ButtonState::Released,
            Modifiers::NONE,
            false,
        ));

        let modal_started = router.process(&event_key(
            KeyCode::KEY_A,
            ButtonState::Pressed,
            Modifiers::NONE,
            false,
        ));
        assert_eq!(modal_started[0].action().as_str(), "modal.action");
        Ok(())
    }

    #[test]
    fn priority_and_identifier_order_break_context_ties() -> Result<(), Box<dyn Error>> {
        let low = context("gameplay.low")?;
        let high = context("gameplay.high")?;
        let chord = key_chord(KeyCode::KEY_D)?;
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(low.clone(), ContextKind::Gameplay, 1),
                ContextDefinition::new(high.clone(), ContextKind::Gameplay, 2),
            ],
            vec![
                Binding::new(
                    low.clone(),
                    chord.clone(),
                    action("low")?,
                    RepeatPolicy::Ignore,
                ),
                Binding::new(high.clone(), chord, action("high")?, RepeatPolicy::Ignore),
            ],
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&low, true)?;
        router.set_context_active(&high, true)?;
        let output = router.process(&event_key(
            KeyCode::KEY_D,
            ButtonState::Pressed,
            Modifiers::NONE,
            false,
        ));
        assert_eq!(output[0].action().as_str(), "high");
        Ok(())
    }

    #[test]
    fn repeat_and_release_lifecycle_is_explicit() -> Result<(), Box<dyn Error>> {
        let gameplay = context("gameplay")?;
        let map = BindingMap::new(
            vec![ContextDefinition::new(
                gameplay.clone(),
                ContextKind::Gameplay,
                0,
            )],
            vec![Binding::new(
                gameplay.clone(),
                key_chord(KeyCode::KEY_W)?,
                action("move.forward")?,
                RepeatPolicy::Allow,
            )],
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        assert_eq!(
            router.process(&event_key(
                KeyCode::KEY_W,
                ButtonState::Pressed,
                Modifiers::NONE,
                false,
            ))[0]
                .phase(),
            ActionPhase::Started
        );
        assert_eq!(
            router.process(&event_key(
                KeyCode::KEY_W,
                ButtonState::Pressed,
                Modifiers::NONE,
                true,
            ))[0]
                .phase(),
            ActionPhase::Repeated
        );
        assert_eq!(
            router.process(&event_key(
                KeyCode::KEY_W,
                ButtonState::Released,
                Modifiers::NONE,
                false,
            ))[0]
                .phase(),
            ActionPhase::Ended
        );
        assert!(router.held_inputs().is_empty());
        Ok(())
    }

    #[test]
    fn focus_loss_cancels_actions_and_clears_held_state() -> Result<(), Box<dyn Error>> {
        let gameplay = context("gameplay")?;
        let map = BindingMap::new(
            vec![ContextDefinition::new(
                gameplay.clone(),
                ContextKind::Gameplay,
                0,
            )],
            vec![Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::one(Modifier::Shift),
                    vec![InputAtom::Key(KeyCode::KEY_S)],
                )?,
                action("move.backward")?,
                RepeatPolicy::Ignore,
            )],
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        router.process(&event_key(
            KeyCode::KEY_S,
            ButtonState::Pressed,
            Modifiers::one(Modifier::Shift),
            false,
        ));
        let cancelled = router.process(&NormalizedInputEvent::FocusChanged { focused: false });
        assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
        assert!(router.held_inputs().is_empty());
        assert_eq!(router.modifiers(), Modifiers::NONE);
        assert!(!router.focused());
        assert!(
            router
                .process(&event_key(
                    KeyCode::KEY_S,
                    ButtonState::Pressed,
                    Modifiers::NONE,
                    false,
                ))
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn capture_loss_cancels_mouse_actions_and_device_loss_clears_everything()
    -> Result<(), Box<dyn Error>> {
        let gameplay = context("gameplay")?;
        let mouse_chord = InputChord::new(
            Modifiers::NONE,
            vec![InputAtom::Mouse(MouseButton::PRIMARY)],
        )?;
        let map = BindingMap::new(
            vec![ContextDefinition::new(
                gameplay.clone(),
                ContextKind::Gameplay,
                0,
            )],
            vec![Binding::new(
                gameplay.clone(),
                mouse_chord,
                action("pointer.primary")?,
                RepeatPolicy::Ignore,
            )],
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        router.process(&NormalizedInputEvent::CaptureChanged { captured: true });
        router.process(&NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
        });
        let cancelled = router.process(&NormalizedInputEvent::CaptureChanged { captured: false });
        assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
        assert!(router.held_inputs().is_empty());
        assert!(!router.captured());

        router.process(&NormalizedInputEvent::CaptureChanged { captured: true });
        router.process(&NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
        });
        let device_cancelled = router.process(&NormalizedInputEvent::DeviceLost);
        assert_eq!(device_cancelled[0].phase(), ActionPhase::Cancelled);
        assert!(router.held_inputs().is_empty());
        assert!(!router.captured());
        Ok(())
    }

    #[test]
    fn wheel_binding_emits_one_bounded_impulse() -> Result<(), Box<dyn Error>> {
        let global = context("global")?;
        let chord = InputChord::new(
            Modifiers::one(Modifier::Control),
            vec![InputAtom::Wheel(WheelDirection::Up)],
        )?;
        let map = BindingMap::new(
            vec![ContextDefinition::new(
                global.clone(),
                ContextKind::Global,
                0,
            )],
            vec![Binding::new(
                global,
                chord,
                action("zoom.in")?,
                RepeatPolicy::Ignore,
            )],
            &[],
        )?;
        let mut router = InputRouter::new(map);
        let output = router.process(&NormalizedInputEvent::Wheel {
            delta: WheelDelta::new(0, 120)?,
            modifiers: Modifiers::one(Modifier::Control),
        });
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].phase(), ActionPhase::Started);
        assert_eq!(output[1].phase(), ActionPhase::Ended);
        assert_eq!(output[0].action().as_str(), "zoom.in");
        Ok(())
    }

    #[test]
    fn bounded_text_pointer_and_physical_values_fail_closed() -> Result<(), InputError> {
        assert_eq!(KeyCode::new(0), Err(InputError::ZeroKeyCode));
        assert_eq!(
            KeyCode::new(MAX_KEY_CODE + 1),
            Err(InputError::KeyCodeOutOfRange)
        );
        assert_eq!(MouseButton::new(0), Err(InputError::ZeroMouseButton));
        assert_eq!(
            PointerCoordinate::new(MAX_POINTER_COORDINATE + 1),
            Err(InputError::CoordinateOutOfRange)
        );
        assert_eq!(
            PointerDelta::new(MAX_POINTER_DELTA + 1),
            Err(InputError::DeltaOutOfRange)
        );
        assert_eq!(WheelDelta::new(0, 0), Err(InputError::ZeroWheelDelta));
        assert_eq!(TextCommit::new(String::new()), Err(InputError::EmptyText));
        assert_eq!(
            TextCommit::new("x".repeat(MAX_TEXT_BYTES + 1)),
            Err(InputError::TextTooLong {
                max: MAX_TEXT_BYTES,
                actual: MAX_TEXT_BYTES + 1,
            })
        );
        assert_eq!(
            ActionId::new("contains space".to_owned()),
            Err(InputError::InvalidIdentifier)
        );
        Ok(())
    }

    #[test]
    fn text_debug_is_redacted() -> Result<(), InputError> {
        let text = TextCommit::new("private committed text".to_owned())?;
        let rendered = format!("{text:?}");
        assert!(!rendered.contains("private committed text"));
        assert!(rendered.contains("bytes"));
        Ok(())
    }

    #[test]
    fn original_synthetic_event_stream_is_deterministic() -> Result<(), Box<dyn Error>> {
        let gameplay = context("gameplay")?;
        let global = context("global")?;
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 10),
                ContextDefinition::new(global.clone(), ContextKind::Global, 0),
            ],
            vec![
                Binding::new(
                    gameplay.clone(),
                    key_chord(KeyCode::KEY_W)?,
                    action("move.forward")?,
                    RepeatPolicy::Allow,
                ),
                Binding::new(
                    global,
                    InputChord::new(
                        Modifiers::NONE,
                        vec![InputAtom::Wheel(WheelDirection::Down)],
                    )?,
                    action("scale.down")?,
                    RepeatPolicy::Ignore,
                ),
            ],
            &[],
        )?;
        let mut first = InputRouter::new(map.clone());
        let mut second = InputRouter::new(map);
        first.set_context_active(&gameplay, true)?;
        second.set_context_active(&gameplay, true)?;

        let stream = [
            event_key(KeyCode::KEY_W, ButtonState::Pressed, Modifiers::NONE, false),
            event_key(KeyCode::KEY_W, ButtonState::Pressed, Modifiers::NONE, true),
            event_key(
                KeyCode::KEY_W,
                ButtonState::Released,
                Modifiers::NONE,
                false,
            ),
            NormalizedInputEvent::Wheel {
                delta: WheelDelta::new(0, -120)?,
                modifiers: Modifiers::NONE,
            },
            NormalizedInputEvent::TextCommitted(TextCommit::new("hello".to_owned())?),
            NormalizedInputEvent::DeviceLost,
        ];

        let first_output: Vec<Vec<ActionEvent>> =
            stream.iter().map(|event| first.process(event)).collect();
        let second_output: Vec<Vec<ActionEvent>> =
            stream.iter().map(|event| second.process(event)).collect();
        assert_eq!(first_output, second_output);
        assert_eq!(first, second);
        assert!(first.held_inputs().is_empty());
        Ok(())
    }
}
