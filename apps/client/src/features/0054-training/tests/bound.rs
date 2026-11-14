use super::*;
use crate::play::play_channel;
use oteryn_client_ui::training::Intent;
#[test]
fn training_bound_owner_rejects_death_shutdown_and_duplicate()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.training_test_selected(true);
    link.training_open();
    let fields = ["7".into(), "9".into(), "11".into()];
    let binding = link.training_binding();
    lock(&link.mailbox).death_reset_commands = true;
    assert!(!link.training_dispatch_bound(&binding, fields.clone(), Some(Intent::Start), true));
    lock(&link.mailbox).death_reset_commands = false;
    lock(&link.mailbox).inventory_shutdown = true;
    assert!(!link.training_dispatch_bound(&binding, fields.clone(), Some(Intent::Start), true));
    lock(&link.mailbox).inventory_shutdown = false;
    assert_eq!(link.training_panel(), binding.panel);
    assert!(commands.try_recv().is_err());
    assert!(link.training_dispatch_bound(&binding, fields.clone(), Some(Intent::Start), true));
    let pending = link.training_panel();
    assert!(!link.training_dispatch_bound(&binding, fields, Some(Intent::Start), true));
    assert_eq!(link.training_panel(), pending);
    assert!(commands.try_recv().is_ok());
    assert!(commands.try_recv().is_err());
    Ok(())
}
