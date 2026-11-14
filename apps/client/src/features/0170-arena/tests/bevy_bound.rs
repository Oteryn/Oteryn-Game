#[cfg(feature = "bevy-shell")]
#[test]
fn arena_actions_are_bound_and_confirmed() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{combat_input::PlayCommand, play::play_channel};
    use oteryn_client_ui::arena::Intent;
    use oteryn_session::arena::ArenaAction;
    let (link, mut commands, outbox) = play_channel();
    outbox.arena_initialize(true);
    let snapshot = link.arena_snapshot();
    assert!(link.arena_toggle_bound(&snapshot));
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    outbox.arena_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 1,
        payload: oteryn_session::arena::encode_catalogue().map_err(|_| "catalogue")?,
    });
    let snapshot = link.arena_snapshot();
    let before = link.arena_panel();
    assert!(!link.arena_dispatch_bound(&snapshot, Intent::Action(ArenaAction::Enter), false));
    assert_eq!(link.arena_panel(), before);
    assert!(commands.try_recv().is_err());
    assert!(link.arena_dispatch_bound(&snapshot, Intent::Action(ArenaAction::Enter), true));
    assert_eq!(link.arena_panel().challenges, before.challenges);
    assert_eq!(link.arena_panel().result, before.result);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!(
        command.kind,
        oteryn_session::arena::COMMAND_TYPE_ARENA_MATCH
    );
    assert_eq!(
        command.payload,
        oteryn_session::arena::encode_match_request(ArenaAction::Enter).map_err(|_| "request")?
    );
    assert!(!link.arena_dispatch_bound(&snapshot, Intent::Action(ArenaAction::Enter), true));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[cfg(feature = "bevy-shell")]
#[test]
fn arena_rejected_when_session_replaced_or_target_changed() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::play_channel;
    use oteryn_client_ui::arena::Intent;
    let (link, mut commands, outbox) = play_channel();
    outbox.arena_initialize(true);
    let old = link.arena_snapshot();
    let (replacement, mut other_commands, other_outbox) = play_channel();
    other_outbox.arena_initialize(true);
    assert!(!replacement.arena_toggle_bound(&old));
    assert!(other_commands.try_recv().is_err());
    assert!(link.arena_toggle_bound(&old));
    let _ = commands.try_recv()?;
    let pending = link.arena_snapshot();
    outbox.arena_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 1,
        payload: oteryn_session::arena::encode_catalogue().map_err(|_| "catalogue")?,
    });
    let current = link.arena_snapshot();
    assert!(!link.arena_dispatch_bound(&pending, Intent::Catalogue, true));
    assert!(!replacement.arena_dispatch_bound(&current, Intent::Catalogue, true));
    assert!(replacement.arena_panel().challenges.is_empty());
    assert!(commands.try_recv().is_err());
    assert!(other_commands.try_recv().is_err());
    assert!(link.arena_toggle_bound(&current));
    let closed = link.arena_panel();
    assert!(!link.arena_dispatch_bound(&current, Intent::Catalogue, true));
    assert_eq!(link.arena_panel(), closed);
    assert!(commands.try_recv().is_err());
    outbox.end(crate::play::public_class(
        &oteryn_session::SessionError::SessionUnusable,
    ));
    let ended = link.arena_snapshot();
    assert!(!link.arena_toggle_bound(&ended));
    assert!(commands.try_recv().is_err());
    Ok(())
}
