use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use oteryn_client_ui::quick_loot::{
    Action, Panel, QuickLootCategory, QuickLootRecipient, QuickLootResult, apply_result,
    encode_quick_loot_result, request,
};

#[test]
fn managed_destinations_replace_only_after_server_confirmation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let destination = |id| Action::Destination {
        category: Some(QuickLootCategory::Gold),
        recipient: QuickLootRecipient::Container,
        id: Some([id; 16]),
        fallback: false,
    };
    let reply = ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_quick_loot_result(&QuickLootResult { items: vec![] })
            .map_err(|e| format!("{e:?}"))?,
    };
    assert!(request(&mut panel, destination(1)).is_some());
    assert!(panel.destinations.is_empty());
    apply_result(&mut panel, &reply);
    assert_eq!(panel.destinations, vec![destination(1)]);
    assert!(request(&mut panel, destination(2)).is_some());
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    assert_eq!(panel.destinations, vec![destination(1)]);
    assert!(request(&mut panel, destination(2)).is_some());
    apply_result(&mut panel, &reply);
    assert_eq!(panel.destinations, vec![destination(2)]);
    Ok(())
}
