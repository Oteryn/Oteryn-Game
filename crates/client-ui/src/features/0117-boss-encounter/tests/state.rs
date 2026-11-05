use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn boss_encounter_gates_pending_and_projects_only_valid_server_data() {
    let mut panel = BossEncounterPanel::default();
    assert!(request(&mut panel, Intent::Refresh).is_none());
    panel.selected = true;
    let command = request(&mut panel, Intent::Refresh);
    assert_eq!(
        command.map(|c| (c.capability, c.kind, c.payload)),
        Some((117, 133, vec![]))
    );
    assert!(request(&mut panel, Intent::Refresh).is_none());
    let response = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![
            8, 100, 18, 7, 10, 3, b'a', b':', b'b', 16, 120, 34, 7, 10, 3, b'a', b':', b'b', 16, 3,
            40, 5,
        ],
    };
    assert!(apply_result(&mut panel, &response));
    let text = panel_text(&panel);
    assert!(text.contains("Boss points: 5"));
    assert!(text.contains("a:b: 3 kills"));
    assert!(text.contains("20 ms remaining"));
    let before = panel.snapshot.clone();
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![8]
        }
    ));
    assert_eq!(panel.snapshot, before);
    assert!(!panel.pending);
}

#[test]
fn boss_encounter_next_uses_earlier_cursor_to_avoid_skipping_either_list() {
    use oteryn_client_runtime::boss_encounter::Snapshot;
    let mut panel = BossEncounterPanel {
        selected: true,
        snapshot: Some(Snapshot {
            cooldown_next: Some("a:z".into()),
            progress_next: Some("a:m".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Intent::Next).map(|c| c.payload),
        Some(vec![10, 3, b'a', b':', b'm'])
    );
    panel.pending = false;
    panel.snapshot = Some(Snapshot::default());
    assert!(request(&mut panel, Intent::Next).is_none());
    assert!(panel_text(&panel).contains("No boss records"));
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Rejected,
        payload: vec![],
    };
    assert!(!apply_result(&mut panel, &result));
    assert!(panel.feedback.is_some());
}

#[test]
fn boss_encounter_open_requires_exact_chord_and_ignores_repeat_release()
-> Result<(), oteryn_input_actions::InputError> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(13)?;
    let modifiers = Modifiers::NONE
        .with(Modifier::Control)
        .with(Modifier::Shift);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    for e in [
        event(ButtonState::Released, modifiers, false),
        event(ButtonState::Pressed, modifiers, true),
        event(ButtonState::Pressed, modifiers.with(Modifier::Alt), false),
        event(
            ButtonState::Pressed,
            Modifiers::NONE.with(Modifier::Control),
            false,
        ),
    ] {
        assert!(!open_key(&[e]));
    }
    Ok(())
}
