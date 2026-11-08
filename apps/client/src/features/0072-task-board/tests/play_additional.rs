use super::*;

#[test]
fn talisman_purchase_uses_full_owner_and_displays_refusal() -> Result<(), Error> {
    use oteryn_client_ui::task_board::{TaskBoardPanel, talisman};
    let mut panel = TaskBoardPanel {
        selected: true,
        ..Default::default()
    };
    let command = talisman::request(&mut panel, talisman::Intent::Buy).ok_or("purchase command")?;
    assert_eq!((command.capability, command.kind), (72, 89));
    assert_eq!(
        task_board::full::decode_request(&command.payload)
            .map_err(|_| "purchase decode")?
            .action,
        17
    );
    assert!(talisman::request(&mut panel, talisman::Intent::Buy).is_none());
    let (link, mut incoming, outbox) = play_channel();
    outbox.task_board_initialize(true);
    let command = talisman::request(
        &mut crate::play::mailbox::lock(&link.mailbox).task_board,
        talisman::Intent::Buy,
    )
    .ok_or("native purchase")?;
    link.commands
        .send(crate::combat_input::PlayCommand::Feature(command))?;
    let command = match incoming.try_recv()? {
        crate::combat_input::PlayCommand::Feature(value) => value,
        _ => return Err("purchase did not reach feature command queue".into()),
    };
    assert_eq!(
        task_board::full::decode_request(&command.payload)
            .map_err(|_| "purchase decode")?
            .action,
        17
    );
    let mut reply = full_reply();
    reply.refusal = 14;
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: task_board::full::encode_reply(&reply).map_err(|_| "purchase reply")?,
    });
    assert!(!link.task_board_panel().pending);
    assert!(panel_text(&link.task_board_panel()).contains("server refused"));
    Ok(())
}

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
