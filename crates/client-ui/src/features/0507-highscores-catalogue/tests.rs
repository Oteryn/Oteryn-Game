#[test]
fn catalogue_continuation_runs_once_and_does_not_retry_a_refused_query()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::highscores::{
        Action, HighscoresPanel, apply_result, catalogue_continuation, request,
    };
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    let mut panel = HighscoresPanel {
        selected: true,
        catalogue_selected: true,
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Action::Refresh)
            .ok_or("catalogue")?
            .command_type,
        6244
    );
    assert!(catalogue_continuation(&mut panel).is_none());
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: oteryn_client_runtime::highscores::catalogue::encode_catalogue(&[])
                .map_err(|error| format!("{error:?}"))?,
        },
    );
    assert!(panel.reply.is_none());
    assert_eq!(
        catalogue_continuation(&mut panel)
            .ok_or("query")?
            .command_type,
        6245
    );
    assert!(catalogue_continuation(&mut panel).is_none());
    apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Rejected,
            payload: Vec::new(),
        },
    );
    assert!(panel.feedback.is_some());
    assert!(catalogue_continuation(&mut panel).is_none());
    panel.catalogue_selected = false;
    panel.feedback = None;
    assert!(catalogue_continuation(&mut panel).is_none());
    Ok(())
}

#[test]
fn world_type_selector_uses_the_server_catalogue() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::highscores::catalogue::{Entry, WorldId, WorldType};
    let world = WorldId::decode(&[1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1])
        .map_err(|e| format!("{e:?}"))?;
    let mut panel = crate::highscores::HighscoresPanel::default();
    panel.selected = true;
    panel.catalogue_selected = true;
    panel.catalogue_loaded = true;
    panel.catalogue = vec![Entry {
        world,
        world_type: WorldType::Optional,
    }];
    assert_eq!(super::world_choices(&panel).count(), 1);
    panel.world_type = Some(WorldType::Open);
    assert_eq!(super::world_choices(&panel).count(), 0);
    let command = crate::highscores::request(
        &mut panel,
        crate::highscores::Action::WorldType(Some(WorldType::Optional)),
    )
    .ok_or("typed query")?;
    assert_eq!((command.capability, command.command_type), (507, 6245));
    let (query, kind) =
        oteryn_client_runtime::highscores::catalogue::decode_query(&command.payload)
            .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.world, kind),
        (Some(world), Some(WorldType::Optional))
    );
    Ok(())
}
