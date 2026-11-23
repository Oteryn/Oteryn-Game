use super::*;

#[test]
fn character_vocation_while_inspecting_another_tab_uses_own_hud_stats() -> TestResult {
    use oteryn_client_runtime::cyclopedia_character::{GeneralStats, TabExtension, TabView};
    let (mut app, mut commands, outbox, _) = setup()?;
    outbox.bevy_cyclopedia_initialize(true, true, false);
    outbox.hud_stamina_initialize(true);
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.cyclopedia_character_request(
        oteryn_client_ui::cyclopedia_character::Intent::SelectTab(
            oteryn_client_runtime::cyclopedia_character::CharacterTab::Offence
        )
    ));
    assert_eq!(feature(&mut commands)?.capability, 199);
    link.bevy_item_details_selected(true);
    click(&mut app, "market.filter.two_handed")?;
    let inspection = feature(&mut commands)?;
    click(&mut app, "market.filter.my_vocation")?;
    assert_eq!(commands.try_recv()?, PlayCommand::HudStamina);
    outbox.trade_inspection_result(
        &inspection,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    app.update();
    let payload = TabView::GeneralStats(GeneralStats {
        level: 42,
        experience: 987654,
        magic_level: 9,
        skills: [11; 7],
        stamina_minutes: 125,
        offline_training_minutes: 60,
        last_vitals: None,
    })
    .encode_extended(&TabExtension {
        vocation_id: Some(7),
        deaths: None,
    })
    .map_err(|_| "encode")?;
    outbox.hud_stamina_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
    app.update();
    let query = wire::decode_query(&feature(&mut commands)?.payload).map_err(|_| "query")?;
    assert_eq!(
        query,
        wire::Query::Catalogue {
            filter: Filter {
                vocation: Some(2),
                ..default()
            },
            page: 0
        }
    );
    let character = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cyclopedia_character_panel();
    assert!(character.pending);
    assert_eq!(
        character.tab,
        oteryn_client_runtime::cyclopedia_character::CharacterTab::Offence
    );
    Ok(())
}
