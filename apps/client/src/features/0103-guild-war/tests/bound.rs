use super::*;
use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_ui::guild_war::Intent;
type Error = Box<dyn std::error::Error>;
fn values() -> [String; 3] {
    [
        "01010101010170018001010101010101".into(),
        "100".into(),
        "7".into(),
    ]
}
#[test]
fn guild_wars_owner_rejects_replaced_channel_changed_target_and_duplicates() -> Result<(), Error> {
    let (link, mut receiver, outbox) = play_channel();
    outbox.guild_wars_test_selected();
    link.guild_war_open();
    let binding = link.guild_wars_binding();
    let (sender, mut replacement_receiver) = tokio::sync::mpsc::unbounded_channel();
    let replacement = PlayLink {
        owns_session: false,
        commands: sender,
        mailbox: std::sync::Arc::clone(&link.mailbox),
    };
    assert!(!replacement.guild_wars_submit_bound(&binding, &values(), Intent::Declare));
    assert!(replacement_receiver.try_recv().is_err());
    assert_eq!(replacement.guild_war_panel(), binding.panel);
    link.guild_war_edit(0, "changed".into());
    let changed = link.guild_war_panel();
    assert!(!link.guild_wars_submit_bound(&binding, &values(), Intent::Declare));
    assert_eq!(link.guild_war_panel(), changed);
    assert!(receiver.try_recv().is_err());
    let binding = link.guild_wars_binding();
    assert!(link.guild_wars_submit_bound(&binding, &values(), Intent::Declare));
    let pending = link.guild_war_panel();
    assert!(!link.guild_wars_submit_bound(&binding, &values(), Intent::Declare));
    assert_eq!(link.guild_war_panel(), pending);
    assert!(matches!(receiver.try_recv()?, PlayCommand::Feature(_)));
    assert!(receiver.try_recv().is_err());
    Ok(())
}
#[test]
fn guild_wars_owner_rejects_closed_session_and_window_without_mutation() {
    let (link, receiver, outbox) = play_channel();
    outbox.guild_wars_test_selected();
    link.guild_war_open();
    let binding = link.guild_wars_binding();
    drop(receiver);
    assert!(!link.guild_wars_submit_bound(&binding, &values(), Intent::Accept));
    assert_eq!(link.guild_war_panel(), binding.panel);
    assert!(link.guild_wars_close_bound(&binding));
    let closed = link.guild_war_panel();
    assert!(!link.guild_wars_submit_bound(&binding, &values(), Intent::Accept));
    assert_eq!(link.guild_war_panel(), closed);
}

#[test]
fn guild_wars_binding_does_not_keep_session_alive() {
    let (link, receiver, _) = play_channel();
    let binding = link.guild_wars_binding();
    drop(link);
    assert!(receiver.is_closed());
    drop(binding);
}
