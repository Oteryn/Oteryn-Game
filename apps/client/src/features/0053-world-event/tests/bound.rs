use super::*;
use crate::play::play_channel;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn world_event_window_owner_rejects_closed_duplicate_and_changed_snapshot() -> Result {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.world_event_initialize(true);
    let binding = link.world_event_binding().ok_or("binding")?;
    assert!(link.world_event_dispatch_bound(&binding, WorldEventControl::Toggle));
    assert!(commands.try_recv().is_ok());
    let pending = link.world_event_binding().ok_or("pending")?;
    let before = link.world_event_panel();
    assert!(!link.world_event_dispatch_bound(&pending, WorldEventControl::Refresh));
    assert!(!link.world_event_dispatch_bound(&binding, WorldEventControl::Toggle));
    assert_eq!(link.world_event_panel(), before);
    assert!(commands.try_recv().is_err());
    assert!(link.world_event_dispatch_bound(&pending, WorldEventControl::Close));
    let closed = link.world_event_binding().ok_or("closed")?;
    assert!(!link.world_event_dispatch_bound(&closed, WorldEventControl::Toggle));
    assert_eq!(link.world_event_panel(), closed.panel);
    let result = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![],
    };
    outbox.world_event_result(&result);
    assert!(!link.world_event_panel().pending);
    assert!(link.world_event_panel().state.is_none());
    let closed = link.world_event_binding().ok_or("settled")?;
    drop(commands);
    assert!(!link.world_event_dispatch_bound(&closed, WorldEventControl::Toggle));
    assert_eq!(link.world_event_panel(), closed.panel);
    assert!(link.world_event_binding().is_none());
    Ok(())
}
#[test]
fn world_event_window_owner_drops_shutdown_and_unsolicited_replies() -> Result {
    let (link, _commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.world_event_initialize(true);
    let result = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![],
    };
    let before = link.world_event_panel();
    outbox.world_event_result(&result);
    assert_eq!(link.world_event_panel(), before);
    let binding = link.world_event_binding().ok_or("binding")?;
    outbox.bevy_world_event_end();
    let before = link.world_event_panel();
    assert!(!link.world_event_dispatch_bound(&binding, WorldEventControl::Toggle));
    outbox.world_event_result(&result);
    assert_eq!(link.world_event_panel(), before);
    Ok(())
}

#[test]
fn world_event_window_unavailable_status_opens_without_a_request() -> Result {
    let (link, mut commands, _outbox) = play_channel();
    link.use_shared_frontend();
    let binding = link.world_event_binding().ok_or("binding")?;
    assert!(link.world_event_dispatch_bound(&binding, WorldEventControl::Toggle));
    let opened = link.world_event_binding().ok_or("opened")?;
    assert!(opened.panel.open);
    assert!(!opened.panel.selected);
    assert!(opened.panel.state.is_none());
    assert!(commands.try_recv().is_err());
    assert!(!link.world_event_dispatch_bound(&opened, WorldEventControl::Refresh));
    assert_eq!(link.world_event_panel(), opened.panel);
    Ok(())
}
#[test]
fn world_event_window_default_presenter_keeps_existing_result_semantics() {
    let (link, _commands, outbox) = play_channel();
    outbox.world_event_initialize(true);
    let result = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![],
    };
    let mut expected = link.world_event_panel();
    oteryn_client_ui::world_event::apply_result(&mut expected, &result);
    outbox.world_event_result(&result);
    assert_eq!(link.world_event_panel(), expected);
}
