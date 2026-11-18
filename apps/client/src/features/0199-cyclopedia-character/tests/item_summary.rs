use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    cyclopedia_character::items::{Location, Page, Row},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_character::Intent;
#[test]
fn review_native_holdings_row_and_detail_use_display_names() {
    use crate::play::feature_0199_cyclopedia_character::view::native::{
        item_summary_detail_text, item_summary_row_text,
    };
    let mut items = oteryn_client_ui::cyclopedia_character::ItemSummary::default();
    let row = Row { definition: "oteryn:item/arrow".into(), quantities: [2, 3, 4, 5] };
    items.names.insert(row.definition.clone(), "Golden arrow".into());
    assert_eq!(item_summary_row_text(&items, &row), "Golden arrow\t2\t3\t4\t5");
    assert_eq!(item_summary_detail_text(&items, &row), "Golden arrow\r\nInventory: 2 · Depot: 3 · Stash: 4 · Inboxes: 5");
    items.names.clear();
    assert_eq!(item_summary_row_text(&items, &row), "oteryn:item/arrow\t2\t3\t4\t5");
    assert_eq!(item_summary_detail_text(&items, &row), "oteryn:item/arrow\r\nInventory: 2 · Depot: 3 · Stash: 4 · Inboxes: 5");
}
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

#[test]
fn store_inbox_opens_with_an_unfiltered_holdings_query() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_character::items::filtered;

    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    outbox.holdings_initialize(true);
    assert!(link.cyclopedia_holdings_filter("sword", Some(Location::Stash)));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing Stash query".into());
    };
    let stash = filtered::Query::decode(&command.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(stash.location, Some(Location::Stash));
    assert_eq!(stash.search, "sword");
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: filtered::Page {
            holdings: Page { offset: 0, total: 0, rows: Vec::new() },
            names: Vec::new(),
        }.encode().map_err(|error| format!("{error:?}"))?,
    });

    link.store_inbox_set_open(true);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing Store Inbox query".into());
    };
    assert_eq!(filtered::Query::decode(&command.payload).map_err(|error| format!("{error:?}"))?, filtered::Query {
        offset: 0, location: None, search: String::new(),
    });
    Ok(())
}
