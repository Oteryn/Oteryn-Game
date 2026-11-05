use crate::play::play_channel;
#[test]
fn hotkey_equip_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.hotkey_equip_initialize(true);
    link.hotkey_equip_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

#[path = "../native.rs"]
mod native;

#[test]
fn native_hotkey_configuration_and_visible_refusal() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let _open: fn(&crate::play::PlayLink) = native::open_window;
    let mut p = oteryn_client_ui::hotkey_equip::HotkeyEquipPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native::submit(&mut p, "bad").is_none());
    let c = native::submit(&mut p, "256").ok_or("missing equip")?;
    assert_eq!(c.payload, [0, 0, 1, 0]);
    assert!(native::submit(&mut p, "512").is_none());
    assert_eq!(p.item_definition, 256);
    oteryn_client_ui::hotkey_equip::apply_result(
        &mut p,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    assert!(native::panel_text(&p).contains("Equip result:"));
    assert!(!p.pending);
    assert!(native::submit(&mut p, "0").is_none());
    oteryn_client_ui::hotkey_equip::apply_result(
        &mut p,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Rejected,
            server_sequence: 1,
            payload: vec![],
        },
    );
    assert!(native::panel_text(&p).contains("refused"));
    Ok(())
}
