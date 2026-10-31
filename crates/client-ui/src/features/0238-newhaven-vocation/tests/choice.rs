use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};

#[test]
fn newhaven_chooser_waits_for_hint_and_server_confirmation() -> Result<(), &'static str> {
    let mut panel = VocationPanel::default();
    assert_eq!(request(&mut panel, 1), None);
    panel.selected = true;
    assert_eq!(request(&mut panel, 1), None);
    panel.hint(2);
    for vocation in 1..=5 {
        assert_eq!(request(&mut panel, vocation), Some(vocation));
        assert!(panel.open);
        assert_eq!(request(&mut panel, vocation), None);
        panel.result(false);
        assert!(panel.open);
    }
    assert_eq!(request(&mut panel, 0), None);
    assert_eq!(request(&mut panel, 6), None);
    assert!(panel_text(&panel).contains("5 Monk"));
    assert_eq!(request(&mut panel, 3), Some(3));
    panel.result(true);
    assert!(!panel.open);
    panel.hint(2);
    assert!(!panel.open);
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(34).map_err(|_| "key")?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    assert_eq!(choice_key(&[key]), Some(5));
    Ok(())
}
