use super::*;
use oteryn_client_ui::animus_mastery::Intent;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn animus_mastery_owner_rejects_foreign_closed_duplicate_and_modal_requests() -> Result {
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.animus_initialize(true);
    let closed = link.animus_binding();
    assert!(!link.animus_dispatch_bound(&closed, AnimusControl::Request(Intent::Refresh), true));
    assert_eq!(link.animus_panel(), closed.panel);
    assert!(!link.animus_dispatch_bound(&closed, AnimusControl::Toggle, false));
    assert!(commands.try_recv().is_err());
    assert!(link.animus_dispatch_bound(&closed, AnimusControl::Toggle, true));
    let _ = commands.try_recv()?;
    let pending = link.animus_binding();
    assert!(!link.animus_dispatch_bound(&pending, AnimusControl::Request(Intent::Refresh), true));
    assert_eq!(link.animus_panel(), pending.panel);
    let (other, mut other_commands, other_outbox) = crate::play::play_channel();
    other.use_shared_frontend();
    other_outbox.animus_initialize(true);
    assert!(other.animus_dispatch_bound(&other.animus_binding(), AnimusControl::Toggle, true));
    let _ = other_commands.try_recv()?;
    assert_eq!(other.animus_panel(), pending.panel);
    assert!(!other.animus_dispatch_bound(&pending, AnimusControl::Close, true));
    assert_eq!(other.animus_panel(), pending.panel);
    let modal = other.bevy_server_modal_fixture();
    let before = other.animus_binding();
    assert!(!other.animus_dispatch_bound(&before, AnimusControl::Close, true));
    assert_eq!(other.animus_panel(), before.panel);
    drop(modal);
    outbox.bevy_animus_end();
    let ended = link.animus_binding();
    assert!(!link.animus_dispatch_bound(&ended, AnimusControl::Toggle, true));
    assert_eq!(link.animus_panel(), ended.panel);
    assert!(commands.try_recv().is_err());
    assert!(other_commands.try_recv().is_err());
    Ok(())
}
#[test]
fn animus_mastery_owner_closed_queue_preserves_snapshot() {
    let (link, commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.animus_initialize(true);
    let before = link.animus_binding();
    drop(commands);
    assert!(!link.animus_dispatch_bound(&before, AnimusControl::Toggle, true));
    assert_eq!(link.animus_panel(), before.panel);
}
