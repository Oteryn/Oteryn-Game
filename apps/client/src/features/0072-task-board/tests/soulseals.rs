use super::*;
use crate::combat_input::PlayCommand;

#[cfg(not(windows))]
#[test]
fn soulseal_dialog_refreshes_confirmed_balance_without_inventing_fight_offers() -> Result<(), Error>
{
    let (link, mut incoming, outbox) = play_channel();
    outbox.task_board_initialize(true);
    link.open_soulseals();
    assert!(link.task_board_panel().soulseals_open);
    assert!(!link.task_board_panel().open);
    let Ok(PlayCommand::Feature(command)) = incoming.try_recv() else {
        return Err("balance refresh command".into());
    };
    assert_eq!((command.capability, command.kind), (72, 89));
    assert_eq!(
        task_board::full::decode_request(&command.payload)
            .map_err(|_| "request")?
            .action,
        4
    );
    let text = oteryn_client_ui::task_board::soulseals_text(&link.task_board_panel());
    assert!(text.contains("Refresh to load"));
    let mut reply = full_reply();
    reply.soulseals = 37;
    reply.weekly.pending_soulseals = 5;
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: task_board::full::encode_reply(&reply).map_err(|_| "reply")?,
    });
    let text = oteryn_client_ui::task_board::soulseals_text(&link.task_board_panel());
    assert!(text.contains("Balance: 37 Soulseals"));
    assert!(text.contains("Pending weekly rewards: 5 Soulseals"));
    assert!(text.contains("Encounter selection is unavailable"));
    assert!(
        incoming.try_recv().is_err(),
        "a balance reply must never initiate a fight"
    );
    Ok(())
}
