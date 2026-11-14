use super::*;
use crate::play::{Outbox, play_channel};
use oteryn_client_ui::hireling::{Intent, hireling};
fn ready() -> (
    PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    Outbox,
) {
    let (link, commands, outbox) = play_channel();
    outbox.hireling_services_test_initialize();
    link.hireling_target(7, 9, -1);
    lock(&link.mailbox).hireling.open = true;
    (link, commands, outbox)
}
#[test]
fn hireling_services_owner_rejects_unconfirmed_duplicate_closed_and_foreign_bindings() {
    let (link, mut commands, outbox) = ready();
    let expected = link.hireling_services_snapshot();
    let summon = HirelingServicesAction::Request {
        intent: Intent::Summon,
        confirmed: false,
    };
    assert!(!link.hireling_services_dispatch_bound(&expected, summon));
    assert_eq!(link.hireling_panel(), expected.panel);
    assert!(commands.try_recv().is_err());
    let (replacement, mut replacement_commands, _) = ready();
    assert!(!replacement.hireling_services_dispatch_bound(
        &expected,
        HirelingServicesAction::Request {
            intent: Intent::Refresh,
            confirmed: false
        }
    ));
    assert!(replacement_commands.try_recv().is_err());
    let confirmed = HirelingServicesAction::Request {
        intent: Intent::Summon,
        confirmed: true,
    };
    assert!(link.hireling_services_dispatch_bound(&expected, confirmed));
    assert!(!link.hireling_services_dispatch_bound(&expected, confirmed));
    assert!(!link.hireling_services_dispatch_bound(&link.hireling_services_snapshot(), confirmed));
    assert!(
        matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::Hireling(command)) if command.0 == hireling::Request::Summon {x:7,y:9,floor:-1})
    );
    assert!(commands.try_recv().is_err());
    assert!(link.hireling_panel().active.is_none());
    outbox.hireling_services_test_response(true);
    let expected = link.hireling_services_snapshot();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.hireling_services_dispatch_bound(&expected, confirmed));
    assert_eq!(link.hireling_panel(), expected.panel);
    drop(modal);
    outbox.hireling_services_test_end();
    assert!(!link.hireling_services_dispatch_bound(&expected, confirmed));
    assert_eq!(link.hireling_panel(), expected.panel);
    assert!(commands.try_recv().is_err());
}
#[test]
fn hireling_services_owner_rejects_changed_target_and_closed_panel_without_mutation() {
    let (link, mut commands, _) = ready();
    let expected = link.hireling_services_snapshot();
    link.hireling_target(8, 9, -1);
    let before = link.hireling_panel();
    let confirmed = HirelingServicesAction::Request {
        intent: Intent::Summon,
        confirmed: true,
    };
    assert!(!link.hireling_services_dispatch_bound(&expected, confirmed));
    assert_eq!(link.hireling_panel(), before);
    assert!(link.hireling_services_dispatch_bound(
        &link.hireling_services_snapshot(),
        HirelingServicesAction::Close
    ));
    let closed = link.hireling_services_snapshot();
    assert!(!link.hireling_services_dispatch_bound(&closed, confirmed));
    assert_eq!(link.hireling_panel(), closed.panel);
    assert!(commands.try_recv().is_err());
}
