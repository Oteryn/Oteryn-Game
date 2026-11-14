use crate::play::play_channel;
#[test]
fn hotkey_equip_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.hotkey_equip_initialize(true);
    link.hotkey_equip_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

#[test]
fn assigned_item_key_sends_the_definition_and_waits_for_the_server()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.hotkey_equip_initialize(true);
    outbox.hotkey_equip_items(vec![256, 512]);
    assert!(link.hotkey_equip_assign(0, 256));
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(58)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    assert!(link.hotkey_equip_input(std::slice::from_ref(&event)));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("missing assigned hotkey".into());
    };
    assert_eq!((command.capability, command.command_type), (179, 348));
    assert_eq!(command.payload, [0, 0, 1, 0]);
    // Pending equipment results lock repeated input without inventing a local outcome.
    assert!(link.hotkey_equip_input(std::slice::from_ref(&event)));
    assert!(commands.try_recv().is_err());
    assert!(link.hotkey_equip_panel().outcome.is_none());
    outbox.hotkey_equip_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    assert!(link.hotkey_equip_input(std::slice::from_ref(&event)));
    let crate::combat_input::PlayCommand::ServerMechanic(toggle) = commands.try_recv()? else {
        return Err("missing toggle".into());
    };
    assert_eq!(toggle.payload, command.payload);
    outbox.hotkey_equip_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 2,
            payload: vec![8, 9],
        },
    );
    assert_eq!(
        link.hotkey_equip_panel().outcome,
        Some(oteryn_session::ItemMoveOutcome::Rejected)
    );
    assert!(link.hotkey_equip_assign(0, 0));
    assert!(!link.hotkey_equip_input(&[event]));
    assert!(!link.hotkey_equip_assign(12, 256));
    assert!(!link.hotkey_equip_assign(0, 999));
    Ok(())
}

use crate::play::feature_0179_hotkey_equip::view::native;

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

#[test]
fn item_hotkeys_persist_per_character_and_negotiate_smart_sources()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0179_hotkey_equip::view::supported_capabilities;
    use crate::play::{PreferenceStore, SettingsState};
    let root = std::env::temp_dir().join(format!("item-hotkeys-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let base = root.join("preferences.json");
    let mut settings = SettingsState::new(Some(base.clone()));
    settings.select_character(Some([41; 16]));
    let (link, _, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.hotkey_equip_items(vec![256]);
    settings.sync_item_hotkeys(&link);
    assert!(link.hotkey_equip_assign(11, 256));
    link.set_smart_equip(false);
    settings.sync_item_hotkeys(&link);
    let saved = PreferenceStore::load(PreferenceStore::character_path(&base, [41; 16]))?;
    assert_eq!(
        saved.preferences().item_hotkeys.bindings[11].map(|id| id.get()),
        Some(256)
    );
    assert!(!saved.preferences().item_hotkeys.smart);
    assert!(!supported_capabilities([41; 16], Some(&base)).contains(&180));
    assert!(supported_capabilities([42; 16], Some(&base)).contains(&180));
    settings.select_character(Some([42; 16]));
    settings.sync_item_hotkeys(&link);
    assert!(
        link.hotkey_equip_panel()
            .bindings
            .iter()
            .all(Option::is_none)
    );
    settings.select_character(Some([41; 16]));
    settings.sync_item_hotkeys(&link);
    assert_eq!(
        link.hotkey_equip_panel().bindings[11].map(|id| id.get()),
        Some(256)
    );
    assert!(!link.hotkey_equip_panel().smart);
    let capabilities = supported_capabilities([41; 16], Some(&base));
    assert!(capabilities.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        capabilities.len() + 1,
        oteryn_session::CLIENT_SUPPORTED_CAPABILITIES.len()
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn t041_object_selector_and_smart_option_persist() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.hotkey_equip_initialize(true);
    outbox.hotkey_equip_items(vec![256, 512]);
    // The selector only offers carried items and a slot clears with 0.
    assert!(link.hotkey_equip_assign(3, 512));
    assert_eq!(
        link.hotkey_equip_panel().bindings[3].map(std::num::NonZeroU32::get),
        Some(512)
    );
    assert!(!link.hotkey_equip_assign(4, 999));
    link.set_smart_equip(false);
    let path = std::env::temp_dir().join(format!("t041-item-hotkeys-{}.json", std::process::id()));
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.sync_item_hotkeys(&link);
    let saved = crate::play::SettingsState::new(Some(path.clone()));
    assert!(!saved.preferences.item_hotkeys.smart);
    assert_eq!(
        saved.preferences.item_hotkeys.bindings[3].map(std::num::NonZeroU32::get),
        Some(512)
    );
    assert!(link.hotkey_equip_assign(3, 0));
    assert!(link.hotkey_equip_panel().bindings[3].is_none());
    let _ = std::fs::remove_file(path);
    Ok(())
}
