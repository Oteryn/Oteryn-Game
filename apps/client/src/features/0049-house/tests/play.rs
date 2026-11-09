use crate::combat_input::PlayCommand;
use crate::play::{feature_0049_house::input::submit, play_channel};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

#[test]
fn house_production_queue_and_result_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    link.house_world("01010101-0101-7101-8101-010101010101");
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.properties.house.initialize(true);
        assert_eq!(
            mailbox.properties.house.fields[0],
            "01010101-0101-7101-8101-010101010101"
        );
        let fields = &mut mailbox.properties.house.fields;
        fields[0] = "01010101-0101-7101-8101-010101010101".into();
        fields[1] = "oteryn:content.area.city.thais".into();
        submit(&mut mailbox, &link.commands, 0);
    }
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("house did not enqueue its protocol command".into());
    };
    assert_eq!((command.capability, command.kind), (49, 57));
    assert!(link.house_panel().model.pending.is_some());
    link.house_request(0);
    assert!(receiver.try_recv().is_err());
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![8, 1],
        },
    );
    assert!(link.house_panel().model.pending.is_none());
    assert_eq!(link.house_panel().model.feedback, "House updated");
    link.house_request(0);
    assert!(receiver.try_recv().is_ok());
    crate::play::mailbox::lock(&link.mailbox).properties.end();
    assert!(!link.house_panel().model.selected);
    assert!(link.house_panel().fields.iter().all(String::is_empty));
    link.house_request(0);
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn house_changes_require_confirmation_and_current_catalogue()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0049_house::{input::prepare, state::Panel};
    use oteryn_client_runtime::house::Row;
    let mut panel = Panel::default();
    panel.model.selected = true;
    panel.fields[0] = "01010101-0101-7101-8101-010101010101".into();
    panel.fields[1] = "oteryn:content.area.city.thais".into();
    prepare(&mut panel, 0)?;
    panel.model.pending = None;
    panel.model.page.rows.push(Row {
        house: "oteryn:house.test".into(),
        ..Default::default()
    });
    panel.fields[2] = "1".into();
    panel.fields[3] = "100".into();
    assert!(prepare(&mut panel, 2).is_err());
    panel.fields[6] = "CONFIRM".into();
    assert_eq!(prepare(&mut panel, 2)?.kind, 56);
    assert!(panel.fields[6].is_empty());
    panel.model.pending = None;
    panel.fields[1] = "oteryn:content.area.city.carlin".into();
    panel.fields[6] = "CONFIRM".into();
    assert!(prepare(&mut panel, 2).is_err());
    assert!(panel.model.page.rows.is_empty());
    Ok(())
}

#[test]
fn door_editor_uses_catalogue_revision_without_manual_revision() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0049_house::{input::prepare, state::Panel};
    use oteryn_client_runtime::house::{Action, Row};
    let mut panel = Panel::default();
    panel.model.selected = true;
    panel.fields[0] = "01010101-0101-7101-8101-010101010101".into();
    panel.fields[1] = "oteryn:content.area.city.thais".into();
    prepare(&mut panel, 0)?;
    panel.model.pending = None;
    panel.model.page.rows.push(Row { house: "oteryn:house.test".into(), acl_revision: 37, ..Row::default() });
    panel.fields[2] = "1".into();
    panel.fields[5] = "100 200 -7".into();
    panel.fields[6] = "CONFIRM".into();
    let command = prepare(&mut panel, 5)?;
    assert_eq!(command, oteryn_client_runtime::house::mutation(panel.model.world, "oteryn:house.test", 37, &Action::Door { x: 100, y: 200, floor: -7, revision: 37, members: vec![] })?);
    Ok(())
}

#[test]
fn house_catalogue_details_reach_production_panel() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.properties.house.initialize(true);
    }
    link.house_edit(0, "01010101-0101-7101-8101-010101010101".into());
    link.house_edit(1, "oteryn:content.area.city.thais".into());
    link.house_request(0);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing house query".into());
    };
    assert!(command.payload.ends_with(&[32, 1]), "request map details explicitly");
    let mut row = vec![10, 1, b'h', 16, 3, 24, 50, 32, 1, 40, 0, 56, 100, 80, 1];
    row.extend([64, 200, 1, 72, 150, 1]);
    // Detail metadata: name, area, beds and entrance x/y/floor from the server.
    row.extend([98, 4, b'H', b'o', b'm', b'e', 104, 25, 112, 2, 120, 100, 128, 1, 110, 136, 1, 13]);
    let mut payload = vec![8, 1, 26, u8::try_from(row.len())?];
    payload.extend(row);
    outbox.properties_result(&command, &GatedCommandResult {
        command_id: 1, server_sequence: 1, status: CommandStatus::Accepted, payload,
    });
    let panel = link.house_panel();
    assert!(panel.model.feedback.contains("updated"));
    let text = oteryn_client_ui::house::panel_text(&panel.model);
    assert!(text.contains("Auction ends (Unix ms): 200; your maximum bid: 150"));
    assert!(text.contains("Home"));
    assert!(text.contains("25 m²"));
    assert!(text.contains("100,110,-7"));
    let map = oteryn_client_ui::house::map_houses(&panel.model);
    assert_eq!(map.len(), 1);
    assert_eq!(map[0].position, (100, 110, -7));
    assert!(map[0].selected);
    assert_eq!(map[0].label, "1: Home");
    Ok(())
}
