use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_ui::house_transfer::Intent;

#[test]
fn house_transfer_owner_rejects_closed_stream_and_identical_replacement() {
    let (link, mut commands, outbox) = play_channel();
    let (replacement, _receiver, replacement_outbox) = play_channel();
    for (link, outbox) in [(&link, &outbox), (&replacement, &replacement_outbox)] {
        outbox.bevy_house_transfer_selected(true);
        link.house_transfer_open();
        link.house_transfer_edit(0, "oteryn:content.house.test".into());
    }
    let expected = link.house_transfer_snapshot();
    assert_eq!(replacement.house_transfer_panel(), expected.panel);
    assert!(!replacement.house_transfer_dispatch_bound(
        &expected,
        Some(Intent::Enter),
        &expected.panel.fields[0]
    ));
    assert_eq!(replacement.house_transfer_panel(), expected.panel);
    commands.close();
    assert!(!link.house_transfer_dispatch_bound(
        &expected,
        Some(Intent::Enter),
        &expected.panel.fields[0]
    ));
    assert!(!link.house_transfer_set_open_bound(&expected, false));
    assert_eq!(link.house_transfer_panel(), expected.panel);
    assert!(commands.try_recv().is_err());
}

#[test]
fn house_transfer_owner_rejects_duplicate_closed_window_and_late_reply()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_house_transfer_selected(true);
    link.house_transfer_edit(0, "oteryn:content.house.test".into());
    let closed = link.house_transfer_snapshot();
    assert!(!link.house_transfer_dispatch_bound(
        &closed,
        Some(Intent::Enter),
        &closed.panel.fields[0]
    ));
    assert_eq!(link.house_transfer_panel(), closed.panel);
    assert!(link.house_transfer_set_open_bound(&closed, true));
    let expected = link.house_transfer_snapshot();
    assert!(link.house_transfer_dispatch_bound(
        &expected,
        Some(Intent::Enter),
        &expected.panel.fields[0]
    ));
    let pending = link.house_transfer_snapshot();
    assert!(pending.panel.pending && pending.panel.summary.is_empty());
    assert!(!link.house_transfer_dispatch_bound(
        &pending,
        Some(Intent::Exit),
        &pending.panel.fields[0]
    ));
    assert_eq!(link.house_transfer_panel(), pending.panel);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("expected feature command".into());
    };
    assert!(commands.try_recv().is_err());
    outbox.bevy_house_transfer_end();
    let ended = link.house_transfer_panel();
    outbox.bevy_house_transfer_response(
        &command,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: vec![255],
        },
    );
    assert_eq!(link.house_transfer_panel(), ended);
    assert!(!link.house_transfer_dispatch_bound(
        &pending,
        Some(Intent::Enter),
        &pending.panel.fields[0]
    ));
    assert_eq!(link.house_transfer_panel(), ended);
    Ok(())
}
