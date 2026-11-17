#[test]
fn host_reads_selected_section_without_losing_map_state() {
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    mailbox.properties.cyclopedia_map.summary = "retained map".into();
    mailbox
        .properties
        .cyclopedia_map
        .navigate(oteryn_client_ui::cyclopedia_map::Section::Charms);
    assert!(super::section_text(&mailbox).starts_with("Cyclopedia > Charms"));
    mailbox
        .properties
        .cyclopedia_map
        .navigate(oteryn_client_ui::cyclopedia_map::Section::Map);
    assert!(super::section_text(&mailbox).contains("retained map"));
}
#[test]
fn houses_section_projects_the_live_catalogue() {
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    mailbox
        .properties
        .cyclopedia_map
        .navigate(oteryn_client_ui::cyclopedia_map::Section::Houses);
    mailbox
        .properties
        .house
        .model
        .page
        .rows
        .push(oteryn_client_runtime::house::Row {
            house: "oteryn:house.harbour".into(),
            rent: 500,
            ends: Some(900),
            ..Default::default()
        });
    let text = super::section_text(&mailbox);
    assert!(text.contains("oteryn:house.harbour"));
    assert!(text.contains("rent 500"));
    assert!(text.contains("900"));
}
#[test]
fn analytics_selector_cycles_tools_without_resetting_confirmed_totals() {
    use oteryn_client_ui::analyser::Tool;
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    mailbox.analyser.selected = true;
    mailbox.analyser.kills = 9;
    super::select_analytics(&mut mailbox, false);
    assert_eq!(mailbox.analyser.tool, Tool::Hunting);
    assert!(super::section_text(&mailbox).contains("Kills: 9"));
    super::select_analytics(&mut mailbox, true);
    super::select_analytics(&mut mailbox, true);
    assert_eq!(mailbox.analyser.tool, Tool::Healing);
    assert_eq!(mailbox.analyser.kills, 9);
    assert!(!super::section_text(&mailbox).contains("Kills:"));
}
#[test]
fn shared_host_refresh_routes_to_selected_feature_and_recovers_closed_session()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{combat_input::PlayCommand, play::play_channel};
    use oteryn_client_ui::cyclopedia_map::Section;
    let (link, mut receiver, _) = play_channel();
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    mailbox.properties.charm_progression.selected = true;
    mailbox
        .properties
        .cyclopedia_map
        .navigate(Section::Bestiary);
    super::actions::refresh_section(&mut mailbox, &link.commands);
    assert!(
        matches!(receiver.try_recv()?, PlayCommand::Feature(command) if command.capability == 67)
    );
    assert!(mailbox.properties.charm_progression.pending);
    super::actions::refresh_section(&mut mailbox, &link.commands);
    assert!(receiver.try_recv().is_err());
    mailbox.properties.cyclopedia_map.navigate(Section::Bosses);
    mailbox.boss_encounter.selected = true;
    drop(receiver);
    super::actions::refresh_section(&mut mailbox, &link.commands);
    assert!(!mailbox.boss_encounter.pending);
    assert_eq!(
        mailbox.boss_encounter.feedback,
        Some("The session has ended")
    );
    assert_eq!(mailbox.properties.cyclopedia_map.section, Section::Bosses);
    Ok(())
}

#[test]
fn cyclopedia_host_navigation_round_trip_uses_the_live_map_protocol()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        combat_input::PlayCommand,
        play::{mailbox::lock, play_channel},
    };
    use oteryn_client_runtime::{
        cyclopedia_map as wire,
        raw_feature::{CommandStatus, GatedCommandResult},
    };
    use oteryn_client_ui::cyclopedia_map::Section;
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.properties.cyclopedia_map.selected = true;
        mailbox.properties.cyclopedia_map.summary = "retained exploration".into();
        for (index, section) in Section::ALL.into_iter().enumerate() {
            assert!(super::actions::navigate_section(&mut mailbox, index, &[]));
            assert!(
                super::section_text(&mailbox)
                    .starts_with(&format!("Cyclopedia > {}", section.title()))
            );
        }
        assert!(!super::actions::navigate_section(&mut mailbox, 99, &[]));
        assert!(super::actions::navigate_section(&mut mailbox, 2, &[]));
        assert!(super::section_text(&mailbox).contains("retained exploration"));
        super::actions::refresh_section(&mut mailbox, &link.commands);
    }
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("expected feature request".into());
    };
    assert_eq!((command.capability, command.kind), (57, 72));
    assert!(command.payload.is_empty());
    let reply = wire::MapResult {
        code: 0,
        revision: 5,
        discovered: false,
        found: 1,
        area_percent: 20,
        pool: 200,
        epoch: 3,
        improved: false,
        visible: vec![(100, 200, 0)],
    };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_result(&reply).map_err(|_| "map result")?,
        },
    );
    assert_eq!(link.cyclopedia_map_panel().map, Some(reply));
    assert!(super::section_text(&lock(&link.mailbox)).contains("100,200,0"));
    assert!(!link.cyclopedia_map_panel().pending);
    Ok(())
}
