use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    charm_progression::catalogue as wire,
    raw_feature::{CommandStatus, GatedCommandResult},
};

#[test]
fn bestiary_catalogue_queue_categories_and_detail_use_confirmed_server_replies()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .properties
        .charm_progression
        .selected = true;
    link.bestiary_catalogue_request(0);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing catalogue query".into());
    };
    assert_eq!((command.capability, command.kind), (67, 84));
    assert_eq!(
        wire::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .race,
        0
    );
    link.bestiary_catalogue_request(0);
    assert!(receiver.try_recv().is_err());
    let stale = oteryn_client_runtime::raw_feature::FeatureCommand {
        capability: 67,
        kind: 84,
        payload: vec![8, 1],
    };
    outbox.properties_result(
        &stale,
        &GatedCommandResult {
            command_id: 99,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![],
        },
    );
    assert!(link.charm_progression_panel().pending);
    let page = wire::Page {
        next: Some(9),
        categories: vec!["Forest dwellers".into()],
        cards: vec![
            wire::Card {
                race: 7,
                name: "Grove watcher".into(),
                category: "Forest dwellers".into(),
                detail: String::new(),
            },
            wire::Card {
                race: 9,
                name: "Moss keeper".into(),
                category: "Forest dwellers".into(),
                detail: String::new(),
            },
        ],
    };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_page(&page).map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(
        oteryn_client_ui::charm_progression::bestiary_text(&link.charm_progression_panel())
            .contains("Grove watcher")
    );
    link.bestiary_catalogue_request(7);
    assert_eq!(link.charm_progression_panel().fields[1], "9");
    link.bestiary_catalogue_request(6);
    assert_eq!(link.charm_progression_panel().fields[1], "7");
    assert!(receiver.try_recv().is_err());
    link.charm_progression_edit(1, "7".into());
    link.bestiary_catalogue_request(2);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing creature detail query".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .race,
        7
    );
    let page = wire::Page { next: None, categories: page.categories, cards: vec![wire::Card {
        race: 7, name: "Grove watcher".into(), category: "Forest dwellers".into(),
        detail: "Health: 90\r\nExperience: 25\r\nResistance fire: 20/1%\r\nLoot:\r\nTwig: 1-2, 250000 ppm".into(),
    }] };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_page(&page).map_err(|e| format!("{e:?}"))?,
        },
    );
    let text = oteryn_client_ui::charm_progression::bestiary_text(&link.charm_progression_panel());
    assert!(text.contains("Health: 90"));
    assert!(text.contains("Resistance fire: 20/1%"));
    assert!(text.contains("Twig: 1-2"));
    outbox.logged_out();
    link.bestiary_catalogue_request(0);
    assert!(receiver.try_recv().is_err());
    assert!(
        link.charm_progression_panel()
            .catalogue
            .page
            .cards
            .is_empty()
    );
    Ok(())
}
