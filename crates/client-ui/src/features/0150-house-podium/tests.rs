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

#[test]
fn t136_creature_podium_switch_set_cycles_displays_and_facings_without_touching_the_projection() {
    let mut panel = Panel {
        selected: true,
        summary: "Committed: outfit podium".into(),
        revision: Some(4),
        ..Panel::default()
    };
    panel.fields[3] = "oteryn:creature.wolf".into();
    for (kind, label) in [(1, "outfit"), (2, "mount"), (3, "creature")] {
        for (direction, facing) in [(0, "north"), (1, "east"), (2, "south"), (3, "west")] {
            assert!(panel.choose_display(kind, direction));
            assert!(
                panel_text(&panel).contains(&format!(
                    "Preview: {label} oteryn:creature.wolf facing {facing} (unsaved)"
                )),
                "{label} {facing}"
            );
        }
    }
    assert!(panel_text(&panel).contains("Committed: outfit podium"));
    assert_eq!(panel.revision, Some(4));
}

#[test]
fn t136_switch_refuses_unknown_displays_and_pending_requests_and_drops_stale_artwork() {
    use oteryn_client_runtime::cui3::house_podium::preview::Artwork;
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    assert!(!panel.choose_display(0, 0));
    assert!(!panel.choose_display(4, 0));
    assert!(!panel.choose_display(1, 4));
    assert_eq!(panel.fields[1], "");
    assert!(panel.choose_display(2, 1));
    panel.artwork = Some(Artwork {
        object: false,
        direction: 1,
        looks: [7, 0],
    });
    assert!(panel.choose_display(2, 1), "unchanged switch keeps artwork");
    assert!(panel.artwork.is_some());
    assert!(panel.choose_display(3, 1));
    assert!(
        panel.artwork.is_none(),
        "a different display invalidates the preview"
    );
    panel.pending = true;
    assert!(!panel.choose_display(1, 0));
    assert_eq!(panel.fields[1], "3");
}
