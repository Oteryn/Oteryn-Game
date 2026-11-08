use super::*;

#[test]
fn full_board_bounds_pending_and_invalid_replies_preserve_server_state() -> Result<(), &'static str>
{
    use oteryn_client_ui::task_board::{
        FullIntent, TaskBoardPanel, apply_full_result, full_request, talisman,
    };
    let mut panel = TaskBoardPanel::default();
    assert!(full_request(&mut panel, FullIntent::Refresh).is_none());
    panel.selected = true;
    for intent in [
        FullIntent::StartWeekly(4),
        FullIntent::Deliver(9),
        FullIntent::SelectBounty(3),
        FullIntent::BuyShop(32),
        FullIntent::Preferred(5, String::new()),
        FullIntent::Unwanted(0, "rat".into()),
    ] {
        assert!(full_request(&mut panel, intent).is_none());
        assert!(!panel.pending);
    }
    for path in 0..4 {
        let command =
            talisman::request(&mut panel, talisman::Intent::Upgrade(path)).ok_or("upgrade")?;
        let decoded = task_board::full::decode_request(&command.payload).map_err(|_| "decode")?;
        assert_eq!((decoded.action, decoded.selection), (18, u16::from(path)));
        panel.pending = false;
    }
    assert!(talisman::request(&mut panel, talisman::Intent::Upgrade(4)).is_none());
    full_request(&mut panel, FullIntent::Refresh).ok_or("query")?;
    assert!(full_request(&mut panel, FullIntent::Deliver(0)).is_none());
    let reply = full_reply();
    let mut result = oteryn_session::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::DuplicateReplay,
        server_sequence: 1,
        payload: task_board::full::encode_reply(&reply).map_err(|_| "reply")?,
    };
    apply_full_result(&mut panel, &result);
    assert!(!panel.pending);
    assert_eq!(panel.weekly.as_ref(), Some(&reply));
    result.payload = vec![0];
    apply_full_result(&mut panel, &result);
    assert_eq!(panel.weekly.as_ref(), Some(&reply));
    assert!(panel.feedback.is_some_and(|text| text.contains("invalid")));
    result.status = CommandStatus::Rejected;
    apply_full_result(&mut panel, &result);
    assert!(panel.feedback.is_some_and(|text| text.contains("refused")));
    Ok(())
}

#[test]
fn native_task_board_shortcut_opens_and_queues_full_query() -> Result<(), &'static str> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(23).map_err(|_| "T key")?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    let (link, mut incoming, outbox) = play_channel();
    outbox.task_board_initialize(true);
    assert!(link.task_board_input(&[event]));
    assert!(link.task_board_panel().open);
    assert!(
        matches!(incoming.try_recv(), Ok(crate::combat_input::PlayCommand::Feature(command))
        if task_board::full::decode_request(&command.payload) == Ok(task_board::full::Request::query()))
    );
    Ok(())
}

mod input_tests {
    use oteryn_client_ui::task_board::*;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

    #[test]
    fn task_board_open_requires_exact_ctrl_o_press() -> Result<(), oteryn_input_actions::InputError>
    {
        let event = |code, state, modifiers, repeat| NormalizedInputEvent::Key {
            code,
            state,
            modifiers,
            repeat,
        };
        let key = KeyCode::new(18)?;
        let ctrl = Modifiers::one(Modifier::Control);
        assert!(open_key(&[event(key, ButtonState::Pressed, ctrl, false)]));
        for input in [
            event(key, ButtonState::Released, ctrl, false),
            event(key, ButtonState::Pressed, ctrl, true),
            event(key, ButtonState::Pressed, Modifiers::NONE, false),
            event(key, ButtonState::Pressed, ctrl.with(Modifier::Shift), false),
            event(key, ButtonState::Pressed, ctrl.with(Modifier::Alt), false),
            event(key, ButtonState::Pressed, ctrl.with(Modifier::Super), false),
            event(KeyCode::new(19)?, ButtonState::Pressed, ctrl, false),
        ] {
            assert!(!open_key(&[input]));
        }
        Ok(())
    }

    #[test]
    fn task_board_unavailable_projection_explains_capability() {
        let mut panel = TaskBoardPanel::default();
        assert!(request(&mut panel, Intent::Refresh).is_none());
        assert!(!panel.pending);
        assert!(panel_text(&panel).contains("unavailable in this session"));
    }
}
