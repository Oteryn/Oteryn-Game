use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc03_raid_schedule_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(14)?;
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
fn pc03_raid_schedule_view_keeps_confirmed_result_during_refusal() {
    let mut panel = Panel {
        selected: true,
        summary: "Confirmed server result".into(),
        ..Default::default()
    };
    let idle = panel_text(&panel);
    assert!(idle.starts_with("Raid schedule"));
    assert!(idle.contains("Confirmed server result"));
    assert!(request(&mut panel, 1));
    assert!(panel_text(&panel).contains("Waiting"));
    panel.failed("The server refused the request");
    let refused = panel_text(&panel);
    assert!(refused.contains("refused"));
    assert!(refused.contains("Confirmed server result"));
    assert!(!panel.pending);
    let fresh = Panel::default();
    assert!(fresh.fields.iter().all(String::is_empty));
    assert!(fresh.summary.is_empty());
    assert!(panel_text(&fresh).contains("Unavailable"));
}
