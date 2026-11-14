use crate::play::play_channel;
use oteryn_client_ui::{depot, depot_search, stash};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn depot_owner_dispatches_move() -> Result {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_depot_fixture();
    let bound = link.depot_binding(7);
    assert_eq!(
        bound.panel.reply.as_ref().ok_or("reply")?.entries[0].definition,
        42
    );
    assert!(link.depot_dispatch_bound(&bound, 7, Some(depot::Intent::Withdraw(0))));
    let crate::combat_input::PlayCommand::Depot(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    assert_eq!(command.action, depot::DepotAction::Withdraw(91));
    assert!(!link.depot_dispatch_bound(&bound, 7, Some(depot::Intent::Withdraw(0))));
    assert_eq!(link.depot_panel().reply, bound.panel.reply);
    Ok(())
}
#[test]
fn stash_owner_uses_quantity_and_rejects_stale_session() -> Result {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_depot_fixture();
    let bound = link.stash_binding(7);
    assert!(link.stash_dispatch_bound(
        &bound,
        7,
        crate::play::feature_0062_stash::view::StashControl::Request(stash::Intent::Withdraw {
            row: 0,
            amount: 13
        })
    ));
    let crate::combat_input::PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    assert_eq!(
        command.intent.action,
        stash::stash::StashAction::Withdraw {
            row: [3; 16],
            amount: 13
        }
    );
    let (other, mut other_commands, _) = play_channel();
    assert!(!other.stash_dispatch_bound(
        &bound,
        7,
        crate::play::feature_0062_stash::view::StashControl::Request(stash::Intent::Withdraw {
            row: 0,
            amount: 13
        })
    ));
    assert!(other_commands.try_recv().is_err());
    Ok(())
}
#[test]
fn depot_search_owner_uses_server_results_only() -> Result {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_depot_fixture();
    let bound = link.depot_search_binding(7);
    assert!(bound.panel.reply.is_none());
    assert!(link.depot_search_dispatch_bound(&bound, 7, Some(42)));
    assert!(link.depot_search_panel().reply.is_none());
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ServerMechanic(_)
    ));
    outbox.bevy_depot_search_reply()?;
    assert!(depot_search::panel_text(&link.depot_search_panel()).contains("depot 5"));
    Ok(())
}
#[test]
fn depot_owner_rejects_replaced_session() -> Result {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_depot_fixture();
    let bound = link.depot_binding(7);
    assert!(!link.depot_dispatch_bound(&bound, 8, Some(depot::Intent::Withdraw(0))));
    let (other, _, other_outbox) = play_channel();
    other_outbox.bevy_depot_fixture();
    assert!(!other.depot_dispatch_bound(&bound, 7, Some(depot::Intent::Withdraw(0))));
    assert_eq!(other.depot_panel(), bound.panel);
    commands.close();
    assert!(!link.depot_dispatch_bound(&bound, 7, Some(depot::Intent::Withdraw(0))));
    assert_eq!(link.depot_panel(), bound.panel);
    Ok(())
}
