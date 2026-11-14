use super::*;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::cyclopedia_tracker::Intent;
#[test]
fn cyclopedia_trackers_owner_rejects_stale_modal_duplicate_and_closed_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_tracker_initialize(true);
    let closed = link.cyclopedia_trackers_binding();
    let action = || TrackerViewAction::Request {
        intent: Intent::Add,
        boss: false,
        creature_key: "oteryn:rat".into(),
        confirmed: false,
    };
    assert!(!link.cyclopedia_trackers_dispatch_bound(&closed, action(), false));
    assert!(link.cyclopedia_trackers_dispatch_bound(&closed, TrackerViewAction::Toggle, false));
    let current = link.cyclopedia_trackers_binding();
    let unchanged = link.cyclopedia_tracker_panel();
    assert!(!link.cyclopedia_trackers_dispatch_bound(&current, action(), true));
    assert_eq!(link.cyclopedia_tracker_panel(), unchanged);
    assert!(!link.cyclopedia_trackers_dispatch_bound(
        &current,
        TrackerViewAction::Request {
            intent: Intent::Remove,
            boss: false,
            creature_key: "oteryn:rat".into(),
            confirmed: false
        },
        false
    ));
    assert_eq!(link.cyclopedia_tracker_panel(), unchanged);
    let fixture = link.bevy_server_modal_fixture();
    assert!(!link.cyclopedia_trackers_dispatch_bound(&current, action(), false));
    assert_eq!(link.cyclopedia_tracker_panel(), unchanged);
    drop(fixture);
    // Release the synthetic modal through its existing owner test driver.
    lock(&outbox.0).server_modal = Default::default();
    lock(&outbox.0).cyclopedia_tracker.creature_key = "oteryn:dragon".into();
    let changed = link.cyclopedia_tracker_panel();
    assert!(!link.cyclopedia_trackers_dispatch_bound(&current, action(), false));
    assert_eq!(link.cyclopedia_tracker_panel(), changed);
    let current = link.cyclopedia_trackers_binding();
    assert!(link.cyclopedia_trackers_dispatch_bound(&current, action(), false));
    assert!(!link.cyclopedia_trackers_dispatch_bound(&current, action(), false));
    assert!(commands.try_recv().is_ok());
    assert!(commands.try_recv().is_err());
    let before = link.cyclopedia_tracker_panel();
    assert!(!link.cyclopedia_trackers_dispatch_bound(
        &link.cyclopedia_trackers_binding(),
        action(),
        false
    ));
    assert_eq!(link.cyclopedia_tracker_panel(), before);
    outbox.bevy_cyclopedia_trackers_end();
    let before = link.cyclopedia_tracker_panel();
    assert!(!link.cyclopedia_trackers_dispatch_bound(
        &link.cyclopedia_trackers_binding(),
        action(),
        false
    ));
    assert_eq!(link.cyclopedia_tracker_panel(), before);
    Ok(())
}
