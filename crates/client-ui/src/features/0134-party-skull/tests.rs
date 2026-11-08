use crate::party_skull::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc06_party_skull_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(25)?;
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
fn pc06_party_skull_view_keeps_confirmed_result_during_refusal() {
    let mut panel = Panel {
        selected: true,
        summary: "Confirmed server result".into(),
        ..Default::default()
    };
    let idle = panel_text(&panel);
    assert!(idle.starts_with("Party shields and skulls"));
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
fn skull_timer_projects_observed_remaining_time_without_inventing_an_expiry() {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    panel.skull_timer = Some(SkullTimer {
        skull: 4,
        remaining_ms: Some(1501),
    });
    assert!(panel_text(&panel).contains("Red skull: 2s at observation"));
    let timer = SkullTimer {
        skull: 4,
        remaining_ms: Some(1501),
    };
    assert_eq!(timer.remaining_seconds(1500), Some(1));
    assert_eq!(timer.remaining_seconds(9999), Some(0));
    panel.skull_timer = Some(SkullTimer {
        skull: 4,
        remaining_ms: None,
    });
    assert!(panel_text(&panel).contains("expiry unavailable"));
}

#[test]
fn unjustified_widget_keeps_fractional_points_and_server_limits() {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    panel.unjustified = Some(UnjustifiedSnapshot {
        windows: [
            FragWindow {
                points_milli: 1500,
                limit_milli: 3000,
            },
            FragWindow {
                points_milli: 3500,
                limit_milli: 5000,
            },
            FragWindow {
                points_milli: 10500,
                limit_milli: 10000,
            },
        ],
        open_situations: Some(vec!["Aggression: rival".into()]),
    });
    let text = panel_text(&panel);
    assert!(text.contains("Daily: 1.500 / 3.000"));
    assert!(text.contains("Weekly: 3.500 / 5.000"));
    assert!(text.contains("Monthly: 10.500 / 10.000"));
    assert!(text.contains("Aggression: rival"));
    if let Some(state) = &mut panel.unjustified {
        state.open_situations = None;
    }
    assert!(panel_text(&panel).contains("Open PvP situations unavailable"));
}
