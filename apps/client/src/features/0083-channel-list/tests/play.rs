use crate::play::{PlayEvent, PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{AppliedDelta, GatedSnapshot, SessionEvent};
fn payload(premium: bool) -> Vec<u8> {
    let mut bytes = vec![10, 38, 10, 16];
    bytes.extend([1; 16]);
    bytes.extend([18, 16]);
    bytes.extend([2; 16]);
    bytes.extend([24, u8::from(premium)]);
    bytes
}
#[test]
fn gap_world_channel_list() -> Result<(), oteryn_renderer::BatchError> {
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    for premium in [false, true] {
        outbox.channel_list_initialize(Some(payload(premium)));
        if let Some(PlayEvent::Pushed(pushed)) = link.poll() {
            view.apply_pushed(&pushed)?;
        }
        assert!(
            view.hud_status()
                .contains(&format!("Premium-only: {premium}"))
        );
    }
    outbox.push(vec![SessionEvent::GatedSnapshotReplaced(AppliedDelta {
        server_sequence: 41,
        base_revision: 0,
        new_revision: 1,
        value: GatedSnapshot {
            domain_id: 48,
            snapshot_type: 1,
            revision: 1,
            payload: payload(false),
        },
    })]);
    if let Some(PlayEvent::Pushed(pushed)) = link.poll() {
        view.apply_pushed(&pushed)?;
    }
    assert!(view.hud_status().contains("Premium-only: false"));
    outbox.channel_list_initialize(None);
    if let Some(PlayEvent::Pushed(pushed)) = link.poll() {
        view.apply_pushed(&pushed)?;
    }
    assert_eq!(view.served_channel_text(), None);
    Ok(())
}

#[test]
fn social_party_uses_admitted_channel_without_channel_list_snapshot() {
    let (link, _commands, outbox) = play_channel();
    let current = "01900000-0000-7000-8000-000000000001";
    link.world_channels("", current, &[]);
    outbox.channel_list_initialize(None);
    assert_eq!(link.social_channels().0, oteryn_platform_client::native_login::parse_uuid_v7(current));
    assert!(link.social_channels().0.is_some());
}
