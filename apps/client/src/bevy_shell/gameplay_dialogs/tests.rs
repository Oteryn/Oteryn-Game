use super::*;
use crate::play::{PlayView, play_channel};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn shared_logout_cancel_duplicate_and_stale_confirm_reach_existing_queue() -> Result {
    // Test-only play fixture; shipping admission still comes exclusively from Reentry.
    let mut host = HostOwner::new(None)?;
    let (link, mut commands, _events) = play_channel();
    host.play = Some(super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = Phase::Playing;
    host.request_logout_confirmation();
    let first = host.dialogs.pending.as_ref().ok_or("confirmation")?.id;
    assert!(host.choose_dialog(first, Choice::Cancel));
    assert!(!host.dialogs.active());
    assert!(commands.try_recv().is_err());
    host.request_logout_confirmation();
    let second = host
        .dialogs
        .pending
        .as_ref()
        .ok_or("second confirmation")?
        .id;
    assert!(!host.choose_dialog(first, Choice::Logout));
    assert!(
        host.dialogs.active(),
        "late click must not dismiss a newer dialog"
    );
    assert!(host.choose_dialog(second, Choice::Logout));
    assert!(!host.choose_dialog(second, Choice::Logout));
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("unexpected end {end:?}"))?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Logout
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn previous_generation_and_wrong_context_target_cannot_dispatch() -> Result {
    let mut host = HostOwner::new(None)?;
    let (link, mut commands, _events) = play_channel();
    host.play = Some(super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = Phase::Playing;
    host.request_logout_confirmation();
    let request = host.dialogs.pending.as_mut().ok_or("confirmation")?;
    request.generation = request.generation.saturating_add(1);
    let id = request.id;
    assert!(!host.choose_dialog(id, Choice::Logout));
    let menu = Menu {
        tile: Some(TileCoord::new(i32::MAX, i32::MAX)),
        binding: None,
        target: oteryn_client_runtime::world_interaction::Target::Position(
            oteryn_session::ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        ),
        entries: vec![(
            "Look",
            oteryn_client_runtime::world_interaction::Action::Look,
        )],
    };
    host.dialogs.open(
        host.generation(),
        Content::Context {
            menu: Some(Box::new(menu)),
            party: None,
            inspection: None,
            report: None,
            trade: None,
            browse: None,
        },
    );
    let id = host.dialogs.pending.as_ref().ok_or("context")?.id;
    assert!(!host.choose_dialog(id, Choice::Context(0)));
    assert!(commands.try_recv().is_err());
    Ok(())
}
