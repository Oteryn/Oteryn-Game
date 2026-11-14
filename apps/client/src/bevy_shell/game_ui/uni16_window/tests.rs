use super::*;
use crate::{
    bevy_shell::owner::ActivePlay,
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture(character: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_uni16_highscores_initialize();
    outbox.bevy_uni16_character_initialize();
    if character {
        assert!(link.character_info_open_bound(&link.character_info_panel()));
    } else {
        assert!(link.highscores_open_bound(&link.highscores_panel()));
    }
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn find(
    app: &mut App,
    predicate: impl Fn(&Kind) -> bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| predicate(&c.kind).then_some(entity))
        .ok_or("control absent".into())
}
fn text_contains(app: &mut App, value: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0.contains(value))
}
#[test]
fn highscores_window_projects_reply_and_blocks_stale_modal_and_closed_controls() -> TestResult {
    use oteryn_client_runtime::{highscores::wire, server_mechanic::ServerMechanicResponse};
    let (mut app, mut commands, outbox) = fixture(false)?;
    let refresh = find(&mut app, |kind| {
        matches!(kind, Kind::Highscores(Action::Refresh))
    })?;
    app.world_mut().trigger(Activate { entity: refresh });
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(_)
    ));
    assert!(commands.try_recv().is_err());
    assert!(!text_contains(&mut app, "Server Hero"));
    outbox.bevy_uni16_highscores_reply(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: wire::encode_highscores_result(&wire::HighscoresResult {
            world: None,
            own_page: None,
            category: 1,
            snapshot_id: 7,
            computed_at: 1,
            page: 1,
            has_more: false,
            rows: vec![wire::HighscoreRow {
                rank: 1,
                name: "Server Hero".into(),
                vocation: "knight".into(),
                level: 12,
                value: 99,
            }],
            own_row: None,
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    assert!(text_contains(&mut app, "Server Hero"));
    let refresh = find(&mut app, |kind| {
        matches!(kind, Kind::Highscores(Action::Refresh))
    })?;
    let modal = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_server_modal_fixture();
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    drop(modal);
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    let stale = app
        .world()
        .get::<Control>(refresh)
        .ok_or("control")?
        .clone();
    let mut stale_generation = stale.clone();
    stale_generation.generation += 1;
    assert!(!perform(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        &stale_generation,
        std::array::from_fn(|_| String::new())
    ));
    app.world_mut().non_send_mut::<HostOwner>().close();
    assert!(!perform(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        &stale,
        std::array::from_fn(|_| String::new())
    ));
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
#[test]
fn highscores_character_window_renders_server_history_and_confirmed_titles() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    let query = find(&mut app, |kind| {
        matches!(kind, Kind::Character(Intent::Query))
    })?;
    app.world_mut().trigger(Activate { entity: query });
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("query command".into());
    };
    assert!(!text_contains(&mut app, "Page deaths 3"));
    let mut payload = vec![0];
    for value in [12_i64, 99, 1, 2, 3, 4, 5, 6] {
        payload.extend(value.to_le_bytes());
    }
    payload.extend(3_u16.to_le_bytes());
    payload.extend(0_u16.to_le_bytes());
    payload.extend([1, 1, 1]);
    outbox.bevy_uni16_character_reply(
        &command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload,
        },
    );
    app.update();
    assert!(text_contains(&mut app, "Page deaths 3"));
    let clear = find(&mut app, |kind| matches!(kind, Kind::Title(0)))?;
    app.world_mut().trigger(Activate { entity: clear });
    app.world_mut().trigger(Activate { entity: clear });
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("select command".into());
    };
    assert!(commands.try_recv().is_err());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .character_info_panel()
            .own_titles
            .ok_or("titles")?
            .selected
            .is_some()
    );
    outbox.bevy_uni16_character_reply(
        &command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: [vec![3], 0_u64.to_le_bytes().to_vec()].concat(),
        },
    );
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .character_info_panel()
            .own_titles
            .ok_or("titles")?
            .selected
            .is_none()
    );
    Ok(())
}

#[test]
fn highscores_and_character_info_controls_reject_actual_session_replacement() -> TestResult {
    for character in [false, true] {
        let (mut app, mut old_commands, old_outbox) = fixture(character)?;
        let button = find(&mut app, |kind| {
            matches!(
                (character, kind),
                (true, Kind::Character(Intent::Query)) | (false, Kind::Highscores(Action::Refresh))
            )
        })?;
        let old_generation = app.world().non_send::<HostOwner>().generation();
        old_outbox.bevy_uni16_end();
        app.world_mut().non_send_mut::<HostOwner>().poll([800, 600]);
        assert_ne!(
            app.world().non_send::<HostOwner>().generation(),
            old_generation
        );
        let (link, mut new_commands, outbox) = play_channel();
        link.use_shared_frontend();
        outbox.bevy_uni16_highscores_initialize();
        outbox.bevy_uni16_character_initialize();
        if character {
            assert!(link.character_info_open_bound(&link.character_info_panel()));
        } else {
            assert!(link.highscores_open_bound(&link.highscores_panel()));
        }
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.play = Some(ActivePlay { view, link });
            owner.phase = Phase::Playing;
        }
        let before = snapshot(app.world().non_send::<HostOwner>());
        app.world_mut().trigger(Activate { entity: button });
        assert!(snapshot(app.world().non_send::<HostOwner>()) == before);
        assert!(old_commands.try_recv().is_err());
        assert!(new_commands.try_recv().is_err());
        app.update();
        assert_eq!(
            app.world().resource::<State>().generation,
            app.world().non_send::<HostOwner>().generation()
        );
    }
    Ok(())
}
