use oteryn_client_runtime::highscores::wire::{self, HighscoreRow, HighscoresResult};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use oteryn_client_ui::highscores::{self, Action, HighscoresPanel};

#[test]
fn live_page_projection_and_snapshot_navigation() -> Result<(), Box<dyn std::error::Error>> {
    let mut panel = HighscoresPanel::default();
    panel.selected = true;
    let command = highscores::request(&mut panel, Action::Refresh).ok_or("missing query")?;
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((command.capability, command.command_type), (9, 14));
    assert_eq!(
        (
            query.category,
            query.vocation,
            query.page,
            query.snapshot_id
        ),
        (1, 0, 1, 0)
    );
    assert!(highscores::request(&mut panel, Action::Refresh).is_none());
    let row = HighscoreRow {
        rank: 1,
        name: "Ada Knight".into(),
        vocation: "knight".into(),
        level: 90,
        value: 123456,
    };
    let result = HighscoresResult {
        category: 1,
        snapshot_id: 73,
        computed_at: 9000,
        page: 1,
        has_more: true,
        rows: vec![row.clone(); 50],
        own_row: Some(row),
    };
    highscores::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: wire::encode_highscores_result(&result).map_err(|e| format!("{e:?}"))?,
        },
    );
    let text = highscores::panel_text(&panel);
    assert!(
        text.contains("Ada Knight") && text.contains("123456") && text.contains("Your rank: 1")
    );
    assert!(text.contains("9000"));
    let command = highscores::request(&mut panel, Action::Next).ok_or("missing next")?;
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.page, query.snapshot_id), (2, 73));
    let second = HighscoresResult {
        page: 2,
        has_more: false,
        rows: vec![],
        ..result.clone()
    };
    highscores::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 3,
            payload: wire::encode_highscores_result(&second).map_err(|e| format!("{e:?}"))?,
        },
    );
    let previous = highscores::request(&mut panel, Action::Previous).ok_or("missing previous")?;
    let previous =
        wire::decode_highscores_query(&previous.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((previous.page, previous.snapshot_id), (1, 73));
    highscores::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Rejected,
            server_sequence: 3,
            payload: vec![],
        },
    );
    assert!(!panel.pending);
    assert!(highscores::panel_text(&panel).contains("refused"));
    let command = highscores::request(
        &mut panel,
        Action::Filter {
            category: 11,
            vocation: 5,
        },
    )
    .ok_or("missing filter")?;
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            query.category,
            query.vocation,
            query.page,
            query.snapshot_id
        ),
        (11, 5, 1, 0)
    );
    Ok(())
}

#[test]
fn unavailable_invalid_and_unsolicited_results_do_not_change_rows() {
    let mut panel = HighscoresPanel::default();
    assert!(highscores::request(&mut panel, Action::Refresh).is_none());
    assert!(highscores::panel_text(&panel).contains("unavailable"));
    panel.selected = true;
    assert!(highscores::request(&mut panel, Action::Previous).is_none());
    assert!(highscores::request(&mut panel, Action::Next).is_none());
    assert!(
        highscores::request(
            &mut panel,
            Action::Filter {
                category: 0,
                vocation: 6
            }
        )
        .is_none()
    );
    assert!(highscores::request(&mut panel, Action::Refresh).is_some());
    highscores::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: vec![255],
        },
    );
    assert!(!panel.pending && panel.reply.is_none());
    assert!(highscores::panel_text(&panel).contains("invalid"));
}

#[test]
fn open_key_ignores_repeat_release_and_other_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(11)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let ctrl = Modifiers::one(Modifier::Control);
    assert!(highscores::open_key(&[event(
        ButtonState::Pressed,
        ctrl,
        false
    )]));
    assert!(!highscores::open_key(&[event(
        ButtonState::Released,
        ctrl,
        false
    )]));
    assert!(!highscores::open_key(&[event(
        ButtonState::Pressed,
        ctrl,
        true
    )]));
    assert!(!highscores::open_key(&[event(
        ButtonState::Pressed,
        ctrl.with(Modifier::Shift),
        false
    )]));
    assert!(!highscores::open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    Ok(())
}
