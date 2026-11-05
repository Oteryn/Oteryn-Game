use crate::play::play_channel;
#[test]
fn browse_field_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    link.browse_field_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

#[cfg(not(windows))]
#[path = "../native.rs"]
mod native;

#[cfg(not(windows))]
#[test]
fn browse_field_native_controls_and_results() {
    use oteryn_client_runtime::cui3::browse_field::{
        BrowseFieldIntent, decode_browse_field_intent,
    };
    use oteryn_client_runtime::cui3::item_view::WorldTilePosition;
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    use oteryn_client_ui::browse_field::{BrowseFieldPanel, apply_result};
    let mut p = BrowseFieldPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native::submit(&mut p, false, ["bad", "200", "-7", "0", "0"]).is_none());
    assert!(!p.pending);
    let command = native::submit(&mut p, false, ["100", "200", "-7", "0", "0"]);
    assert!(
        matches!(command, Some(c) if c.capability == 230 && c.command_type == 440 && decode_browse_field_intent(&c.payload) == Ok(BrowseFieldIntent::Open { position: WorldTilePosition { x: 100, y: 200, floor: -7 } }))
    );
    assert!(!native::enabled(&p));
    assert!(native::result_text(&p).contains("Waiting"));
    assert!(native::submit(&mut p, true, ["", "", "", "1", "30"]).is_none());
    apply_result(
        &mut p,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    assert!(native::enabled(&p));
    assert!(native::result_text(&p).contains("Opened"));
    let command = native::submit(&mut p, true, ["", "", "", "1", "30"]);
    assert!(
        matches!(command, Some(c) if decode_browse_field_intent(&c.payload) == Ok(BrowseFieldIntent::Seek { view_id: 1, index: 30 }))
    );
    apply_result(
        &mut p,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Rejected,
            server_sequence: 2,
            payload: vec![],
        },
    );
    assert!(native::enabled(&p));
    assert!(native::result_text(&p).contains("refused"));
    p.selected = false;
    assert!(!native::enabled(&p));
}
