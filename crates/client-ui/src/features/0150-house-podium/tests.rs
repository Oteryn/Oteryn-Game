use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn cp01_house_podium_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(19)?;
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, modifiers, true)]));
    assert!(!open_key(&[event(ButtonState::Released, modifiers, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        modifiers.with(Modifier::Shift),
        false
    )]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    let mut panel = Panel::default();
    assert!(!request(&mut panel, 1));
    assert!(panel_text(&panel).contains("Unavailable"));
    panel.selected = true;
    assert!(request(&mut panel, 1));
    assert!(!request(&mut panel, 2));
    assert_eq!(panel.pending_kind, 1);
    assert!(panel_text(&panel).contains("Waiting"));
    panel.failed("The session has ended");
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("ended"));
    Ok(())
}

#[test]
fn podium_previews_the_draft_separately_from_committed_projection() {
    let panel = Panel {
        selected: true,
        fields: [
            "item".into(),
            "2".into(),
            "1".into(),
            "oteryn:mount.horse".into(),
        ],
        summary: "Committed: empty podium".into(),
        ..Panel::default()
    };
    let text = panel_text(&panel);
    assert!(text.contains("Preview: mount oteryn:mount.horse facing east"));
    assert!(text.contains("Committed: empty podium"));
}
