#[test]
fn o021_overhead_queries_server_icons_without_opening_status_window()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    let (link, mut commands, outbox) = crate::play::play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .properties
        .party_skull
        .selected = true;
    let mut play = view()?;
    let mut preferences = play.preferences().clone();
    preferences.creature_bars = true;
    play.set_preferences(preferences)?;
    play.tick(&link).map_err(|_| "session ended")?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing icon query".into());
    };
    assert_eq!((command.capability, command.kind), (134, 155));
    assert!(!link.party_skull_panel().open);
    let entity = oteryn_session::EntityRef {
        identity: [8; 16],
        generation: 2,
    };
    let payload = oteryn_session::property_services::party_skull::encode_party_icons(&[
        oteryn_session::property_services::party_skull::PartyIcon {
            entity,
            shield: 7,
            skull: 4,
        },
    ])
    .map_err(|error| format!("{error:?}"))?;
    outbox.properties_result(
        &command,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 3,
            payload,
        },
    );
    outbox.entities(vec![oteryn_session::WorldSpatialEntity {
        entity,
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        position: oteryn_session::ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 50,
        },
    }]);
    play.tick(&link).map_err(|_| "session ended")?;
    assert_eq!(play.party_icons.len(), 1);
    let epoch = play.party_icon_epoch;
    play.project_presentation(epoch)?;
    let lit = play.scene().sprites().len();
    play.project_presentation(epoch + std::time::Duration::from_millis(500))?;
    assert!(play.scene().sprites().len() < lit);
    play.project_presentation(epoch + std::time::Duration::from_millis(1000))?;
    assert_eq!(play.scene().sprites().len(), lit);
    Ok(())
}
