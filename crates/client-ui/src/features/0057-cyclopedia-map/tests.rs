use crate::cyclopedia_map::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc06_cyclopedia_map_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(24)?;
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
fn pc06_cyclopedia_map_view_keeps_confirmed_result_during_refusal() {
    let mut panel = Panel {
        selected: true,
        summary: "Confirmed server result".into(),
        ..Default::default()
    };
    let idle = panel_text(&panel);
    assert!(idle.starts_with("Cyclopedia map"));
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

#[test]
fn cyclopedia_sections_have_stable_navigation_and_distinct_titles() {
    assert_eq!(Section::ALL.len(), 8);
    let mut panel = Panel::default();
    for section in Section::ALL {
        panel.navigate(section);
        assert_eq!(panel.section, section);
        assert!(!section.title().is_empty());
    }
    assert_eq!(panel.section, Section::Analytics);
    assert!(!panel.pending);
}

#[test]
fn map_projects_only_visible_points_on_the_selected_floor() {
    let panel = Panel {
        map: Some(oteryn_client_runtime::cyclopedia_map::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 2,
            area_percent: 25,
            pool: 10,
            epoch: 1,
            improved: false,
            visible: vec![(65535, 65535, 0), (65534, 65535, 0), (1, 2, -1)],
        }),
        ..Default::default()
    };
    let cells = map_points(&panel);
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().all(|p| p.column < 32 && p.row < 12));
    assert!(map_text(&panel).contains("65535,65535"));
    assert!(!map_text(&panel).contains("1,2,-1"));
    assert!(map_text(&Panel::default()).contains("Query"));
}
