use super::*;
use crate::play::play_channel;

type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn premium_overview_owner_rejects_modal_duplicate_closed_and_replaced_bindings() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.premium_game_initialize(true);
    let initial = link.premium_overview_binding();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.premium_overview_action_bound(&initial, false, true));
    assert!(initial.same_projection(&link.premium_overview_binding()));
    assert!(commands.try_recv().is_err());
    drop(modal);
    assert!(!link.premium_overview_action_bound(&initial, false, false));
    assert!(initial.same_projection(&link.premium_overview_binding()));
    assert!(link.premium_overview_action_bound(&initial, false, true));
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    assert!(!link.premium_overview_action_bound(&initial, false, true));
    assert!(commands.try_recv().is_err());
    let pending = link.premium_overview_binding();
    let (other, mut receiver, _) = play_channel();
    other.use_shared_frontend();
    assert!(!other.premium_overview_action_bound(&pending, false, true));
    assert!(receiver.try_recv().is_err());
    assert!(!link.premium_overview_action_bound(&pending, false, true));
    drop(commands);
    assert!(!link.premium_overview_action_bound(&pending, true, true));
    assert!(pending.same_projection(&link.premium_overview_binding()));
    Ok(())
}
#[test]
fn premium_overview_close_is_session_only_and_retains_settlement() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.premium_game_initialize(true);
    let initial = link.premium_overview_binding();
    assert!(link.premium_overview_action_bound(&initial, false, true));
    let request = match commands.try_recv()? {
        PlayCommand::Feature(c) => c,
        _ => return Err("request".into()),
    };
    let pending = link.premium_overview_binding();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.premium_game.selected = false;
        mailbox.death.dead = true;
    }
    let modal = link.bevy_server_modal_fixture();
    assert!(link.premium_overview_action_bound(&pending, true, false));
    assert!(!link.premium_overview_binding().open);
    assert!(link.premium_overview_binding().pending);
    // The result still settles the original operation while the view is closed.
    outbox.premium_game_result(
        &request,
        &GatedCommandResult {
            command_id: 1,
            status: oteryn_client_runtime::raw_feature::CommandStatus::Rejected,
            server_sequence: 1,
            payload: Vec::new(),
        },
    );
    assert!(!link.premium_overview_binding().pending);
    drop(modal);
    lock(&link.mailbox).death.dead = false;
    let closed = link.premium_overview_binding();
    assert!(link.premium_overview_action_bound(&closed, false, true));
    let unavailable = link.premium_overview_binding();
    assert!(unavailable.open);
    assert!(unavailable.text.contains("unavailable"));
    assert!(commands.try_recv().is_err());
    assert!(link.premium_overview_action_bound(&unavailable, true, false));
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let ended = link.premium_overview_binding();
    assert!(!link.premium_overview_action_bound(&ended, false, true));
    assert!(!link.premium_overview_action_bound(&ended, true, true));
    Ok(())
}
