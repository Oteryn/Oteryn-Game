use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    cyclopedia_character::items::{Location, Page, Row},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_character::Intent;
#[test]
fn t095_item_summary_caller_requests_all_locations_and_filters_confirmed_rows()
-> Result<(), &'static str> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::Items);
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("command");
    };
    assert_eq!(command.payload, [130, 0, 0]);
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: Page {
            offset: 0,
            total: 2,
            rows: vec![
                Row {
                    definition: "oteryn:item/arrow".into(),
                    quantities: [2, 3, 4, 5],
                },
                Row {
                    definition: "oteryn:item/sword".into(),
                    quantities: [1, 0, 0, 0],
                },
            ],
        }
        .encode()
        .map_err(|_| "encode")?,
    });
    let mut panel = link.cyclopedia_character_panel();
    assert_eq!(panel.items.rows().len(), 2);
    for location in [Location::Depot, Location::Stash, Location::Inbox] {
        panel.items.location = Some(location);
        let rows = panel.items.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].definition, "oteryn:item/arrow");
    }
    panel.items.location = Some(Location::Inventory);
    panel.items.search = "sword".into();
    assert_eq!(panel.items.rows()[0].quantities, [1, 0, 0, 0]);
    link.cyclopedia_character_request(Intent::ItemsNext);
    assert!(commands.try_recv().is_err());
    let confirmed = link.cyclopedia_character_panel().items.page;
    link.cyclopedia_character_request(Intent::Items);
    assert!(commands.try_recv().is_ok());
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: vec![130],
    });
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.items.page, confirmed);
    assert!(panel.feedback.is_some());
    Ok(())
}
