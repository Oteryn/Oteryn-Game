use crate::play::feature_0445_house_furniture_rotation::view::BoundAction;
use oteryn_client_ui::house_furniture_rotation::Intent;
#[test]
fn furniture_rotation_owner_rejects_closed_pending_and_replaced_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.furniture_rotation_selected();
    let closed = link.house_rotation_binding();
    assert!(!link.house_rotation_dispatch_bound(&closed, BoundAction::Request(Intent::Refresh)));
    assert!(commands.try_recv().is_err());
    assert!(link.house_rotation_dispatch_bound(&closed, BoundAction::Toggle));
    let pending = link.house_rotation_binding();
    assert!(!link.house_rotation_dispatch_bound(&closed, BoundAction::Toggle));
    assert!(!link.house_rotation_dispatch_bound(&pending, BoundAction::Request(Intent::Refresh)));
    let _ = commands.try_recv()?;
    assert!(commands.try_recv().is_err());
    assert!(link.house_rotation_dispatch_bound(&pending, BoundAction::Close));
    let after = link.house_rotation_panel();
    // Closing must not let an older same-page reply consume a replacement request.
    assert!(
        link.house_rotation_dispatch_bound(&link.house_rotation_binding(), BoundAction::Toggle)
    );
    let mut reopened = after;
    reopened.open = true;
    assert_eq!(link.house_rotation_panel(), reopened);
    let after = reopened;
    assert!(!link.house_rotation_dispatch_bound(&pending, BoundAction::Request(Intent::Refresh)));
    assert_eq!(link.house_rotation_panel(), after);
    drop(commands);
    let binding = link.house_rotation_binding();
    assert!(!link.house_rotation_dispatch_bound(&binding, BoundAction::Toggle));
    assert_eq!(link.house_rotation_panel(), after);
    Ok(())
}
