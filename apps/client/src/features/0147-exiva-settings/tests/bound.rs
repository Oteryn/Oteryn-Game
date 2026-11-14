use super::*;
use crate::play::play_channel;
use oteryn_client_ui::exiva_settings::PlayerList;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn exiva_settings_owner_rejects_without_mutating_under_lock() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.exiva_initialize(true);
    let snapshot = link.exiva_snapshot();
    assert!(link.exiva_dispatch_bound(&snapshot, ExivaControl::Open, false));
    let _ = commands.try_recv()?;
    let list = PlayerList {
        modes: 2,
        players: vec![([3; 16], "Al".into())],
    };
    outbox.exiva_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: list.encode().ok_or("encode")?,
    });
    let snapshot = link.exiva_snapshot();
    for (action, confirmed) in [
        (ExivaControl::Remove([3; 16]), false),
        (ExivaControl::Remove([4; 16]), true),
        (ExivaControl::Toggle(5), false),
        (ExivaControl::AddName("?".into()), false),
    ] {
        assert!(!link.exiva_dispatch_bound(&snapshot, action, confirmed));
        assert_eq!(link.exiva_panel(), snapshot.panel);
        assert!(commands.try_recv().is_err());
    }
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.exiva_dispatch_bound(&snapshot, ExivaControl::Refresh, false));
    assert_eq!(link.exiva_panel(), snapshot.panel);
    drop(modal);
    assert!(link.exiva_dispatch_bound(&snapshot, ExivaControl::Refresh, false));
    assert!(!link.exiva_dispatch_bound(&snapshot, ExivaControl::Refresh, false));
    let _ = commands.try_recv()?;
    assert!(commands.try_recv().is_err());
    // Even a reply with identical data cannot revive a consumed observation.
    outbox.exiva_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: list.encode().ok_or("encode")?,
    });
    assert!(!link.exiva_dispatch_bound(&snapshot, ExivaControl::Refresh, false));
    assert!(commands.try_recv().is_err());
    assert!(link.exiva_dispatch_bound(&link.exiva_snapshot(), ExivaControl::Refresh, false));
    let _ = commands.try_recv()?;
    let pending = link.exiva_snapshot();
    assert!(link.exiva_dispatch_bound(&pending, ExivaControl::Close, false));
    let closed = link.exiva_snapshot();
    assert!(!link.exiva_dispatch_bound(&closed, ExivaControl::Refresh, false));
    assert_eq!(link.exiva_panel(), closed.panel);
    outbox.bevy_exiva_end();
    let ended = link.exiva_snapshot();
    assert!(!link.exiva_dispatch_bound(&ended, ExivaControl::Open, false));
    assert_eq!(link.exiva_panel(), ended.panel);
    assert!(commands.try_recv().is_err());
    Ok(())
}
