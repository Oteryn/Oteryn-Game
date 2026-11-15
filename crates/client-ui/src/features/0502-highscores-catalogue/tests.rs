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
    assert_eq!((command.capability, command.command_type), (502, 6242));
    let (query, kind) =
        oteryn_client_runtime::highscores::catalogue::decode_query(&command.payload)
            .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.world, kind),
        (Some(world), Some(WorldType::Optional))
    );
    Ok(())
}
