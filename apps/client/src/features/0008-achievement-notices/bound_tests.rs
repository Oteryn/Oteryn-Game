use crate::play::play_channel;

#[test]
fn achievement_notices_bound_visibility_rejects_closed_duplicate_and_other_session() {
    let (link, receiver, outbox) = play_channel();
    link.use_shared_frontend();
    let initial = link.achievement_notices_binding();
    assert!(link.achievement_notices_set_open_bound(&initial, true));
    let open = link.achievement_notices_binding();
    assert!(!link.achievement_notices_set_open_bound(&initial, true));
    let (other, _commands, _outbox) = play_channel();
    other.use_shared_frontend();
    let before = other.achievement_notices_panel();
    assert!(!other.achievement_notices_set_open_bound(&open, false));
    assert_eq!(other.achievement_notices_panel(), before);
    drop(receiver);
    assert!(!link.achievement_notices_set_open_bound(&open, false));
    assert_eq!(link.achievement_notices_panel(), open.panel);
    outbox.bevy_achievement_notices_end();
    let ended = link.achievement_notices_binding();
    assert!(!link.achievement_notices_set_open_bound(&ended, true));
    assert_eq!(link.achievement_notices_panel(), ended.panel);
}

#[test]
fn achievement_notices_bound_visibility_rejects_owner_modal_under_lock() {
    let (link, _commands, _outbox) = play_channel();
    link.use_shared_frontend();
    let initial = link.achievement_notices_binding();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.achievement_notices_set_open_bound(&initial, true));
    assert_eq!(link.achievement_notices_panel(), initial.panel);
    drop(modal);
    assert!(link.achievement_notices_set_open_bound(&initial, true));
    let open = link.achievement_notices_binding();
    let _modal = link.bevy_server_modal_fixture();
    assert!(!link.achievement_notices_set_open_bound(&open, false));
    assert_eq!(link.achievement_notices_panel(), open.panel);
}
