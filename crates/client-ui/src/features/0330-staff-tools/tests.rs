use super::*;
use oteryn_client_runtime::raw_feature::staff_tools as wire;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn staff_tools_request_keeps_signed_bytes_and_uses_negotiated_commands()
-> Result<(), wire::StaffToolsWireError> {
    let envelope = wire::SignedEnvelope {
        principal: "operator".into(),
        canonical: b" exact signed text ".to_vec(),
        signature: vec![1, 2, 3],
    };
    for (receipt, kind) in [(false, 560), (true, 561)] {
        let command = request(&envelope, receipt)?;
        assert_eq!((command.capability, command.kind), (330, kind));
        assert_eq!(wire::decode_envelope(&command.payload)?, envelope);
    }
    Ok(())
}
#[test]
fn staff_tools_shortcut_is_exact_and_does_not_repeat() -> Result<(), &'static str> {
    let chord = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    for (modifiers, repeat, state, expected) in [
        (chord, false, ButtonState::Pressed, true),
        (chord, true, ButtonState::Pressed, false),
        (chord, false, ButtonState::Released, false),
        (
            chord.with(Modifier::Shift),
            false,
            ButtonState::Pressed,
            false,
        ),
        (
            Modifiers::one(Modifier::Control),
            false,
            ButtonState::Pressed,
            false,
        ),
    ] {
        assert_eq!(
            open_key(&[NormalizedInputEvent::Key {
                code: KeyCode::new(39).map_err(|_| "key code")?,
                state,
                modifiers,
                repeat,
            }]),
            expected
        );
    }
    assert!(
        panel_text("Character: Example", "Audited action confirmed").contains("Character: Example")
    );
    Ok(())
}
