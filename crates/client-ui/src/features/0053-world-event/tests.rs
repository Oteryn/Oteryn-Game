use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn w05d_seasonal_calendar_names_are_decoded_and_presented_natively()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = WorldEventPanel {
        selected: true,
        ..WorldEventPanel::default()
    };
    for name in ["Halloween", "Christmas", "A Piece of Cake"] {
        let _ = request(&mut panel).ok_or("calendar query")?;
        let announcement = format!("{name} is active.");
        // Existing world-event protobuf fields: epoch3, active, UTF-8 announcement.
        let mut payload = vec![8, 3, 16, 1, 26, u8::try_from(announcement.len())?];
        payload.extend_from_slice(announcement.as_bytes());
        apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload,
            },
        );
        assert!(panel_text(&panel).contains(&announcement));
        assert!(!panel.pending);
    }
    Ok(())
}

#[test]
fn world_event_input_ignores_repeats_releases_and_other_modifiers()
-> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(17)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let ctrl = Modifiers::one(Modifier::Control);
    assert!(open_key(&[event(ButtonState::Pressed, ctrl, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, ctrl, true)]));
    assert!(!open_key(&[event(ButtonState::Released, ctrl, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        ctrl.with(Modifier::Shift),
        false
    )]));
    Ok(())
}

#[test]
fn world_event_query_and_projection_follow_server_results() -> Result<(), Box<dyn std::error::Error>>
{
    let mut panel = WorldEventPanel::default();
    assert!(request(&mut panel).is_none());
    assert!(panel_text(&panel).contains("unavailable"));
    panel.selected = true;
    let command = request(&mut panel).ok_or("missing query")?;
    assert_eq!(
        (command.capability, command.kind, command.payload),
        (53, 64, vec![])
    );
    assert!(panel_text(&panel).contains("Waiting"));
    assert!(request(&mut panel).is_none());
    let mut result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![8, 9, 16, 1, 26, 4, b'F', b'a', b'i', b'r'],
    };
    apply_result(&mut panel, &result);
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("Fair"));
    assert!(panel_text(&panel).contains("9"));
    assert!(request(&mut panel).is_some());
    result.status = CommandStatus::DuplicateReplay;
    result.payload = vec![8, 10];
    apply_result(&mut panel, &result);
    assert!(panel_text(&panel).contains("No active world event"));
    assert!(!panel_text(&panel).contains("Fair"));
    assert!(request(&mut panel).is_some());
    result.payload = vec![255];
    apply_result(&mut panel, &result);
    assert!(panel.state.is_none());
    assert!(panel_text(&panel).contains("invalid"));
    assert!(request(&mut panel).is_some());
    result.status = CommandStatus::Rejected;
    apply_result(&mut panel, &result);
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("refused"));
    Ok(())
}
