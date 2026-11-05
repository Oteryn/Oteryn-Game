use super::*;
use oteryn_client_runtime::tutorial_notices::Notice;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn notices_are_bounded_and_server_save_is_separate() {
    let mut panel = NoticePanel::default();
    assert!(panel_text(&panel).contains("unavailable"));
    panel.tutorial_selected = true;
    panel.save_selected = true;
    for id in 1..=40 {
        panel.apply(Notice::Tutorial(format!("Hint {id}")));
    }
    panel.apply(Notice::ServerSave { minutes: 3 });
    panel.apply(Notice::ServerSave { minutes: 1 });
    assert_eq!(panel.tutorial.len(), 32);
    assert_eq!(panel.tutorial.front().map(String::as_str), Some("Hint 9"));
    let text = panel_text(&panel);
    assert!(text.contains("Hint 40"));
    assert!(text.contains("1 minute"));
    assert!(!text.contains("3 minutes"));
    assert!(!text.contains("unavailable"));
}
#[test]
fn notice_shortcut_requires_exact_modifiers_and_fresh_press()
-> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(17)?;
    let key = |state, repeat, modifiers| NormalizedInputEvent::Key {
        code,
        state,
        repeat,
        modifiers,
    };
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    assert!(open_key(&[key(ButtonState::Pressed, false, modifiers)]));
    assert!(!open_key(&[key(ButtonState::Released, false, modifiers)]));
    assert!(!open_key(&[key(ButtonState::Pressed, true, modifiers)]));
    assert!(!open_key(&[key(
        ButtonState::Pressed,
        false,
        Modifiers::NONE
    )]));
    assert!(!open_key(&[key(
        ButtonState::Pressed,
        false,
        modifiers.with(Modifier::Alt)
    )]));
    Ok(())
}
