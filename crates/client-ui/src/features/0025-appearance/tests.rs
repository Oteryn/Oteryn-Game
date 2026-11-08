use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc01_appearance_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(4)?;
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
fn gallery_navigation_uses_catalogue_and_never_changes_confirmed_selection()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::appearance::{Catalogue, Context, OutfitOption};
    let mut panel = Panel {
        selected: true,
        catalogue: Some(Catalogue {
            context: Context {
                content_generation: [1; 32],
                cosmetics_digest: [2; 32],
            },
            root_revision: 1,
            selected: None,
            palette: vec![[255, 0, 0]],
            outfits: vec![
                OutfitOption {
                    key: "knight".into(),
                    look_type: 128,
                    permitted_addons: 3,
                },
                OutfitOption {
                    key: "mage".into(),
                    look_type: 129,
                    permitted_addons: 1,
                },
            ],
            mounts: vec![],
        }),
        ..Default::default()
    };
    panel.fields[0] = "knight".into();
    panel.fields[1] = "3".into();
    assert!(panel.cycle_outfit(true));
    assert_eq!(panel.fields[0], "mage");
    assert_eq!(panel.fields[1], "1");
    assert!(panel_text(&panel).contains("Preview: mage"));
    assert!(panel_text(&panel).contains("look 129"));
    assert!(panel.confirmed.is_none());
    panel.pending = true;
    assert!(!panel.cycle_outfit(false));
    Ok(())
}
