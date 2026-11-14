use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::character_info::Intent;
use oteryn_client_ui::highscores::Action;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn highscores_query_is_bound_and_drops_stale_replies() -> TestResult {
    use oteryn_client_runtime::{highscores::wire, server_mechanic::ServerMechanicResponse};
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.highscores_initialize(true, false);
    let initial = link.highscores_panel();
    assert!(link.highscores_open_bound(&initial));
    let expected = link.highscores_panel();
    assert!(link.highscores_request_bound(
        &expected,
        Action::Filter {
            category: 2,
            vocation: 3
        }
    ));
    assert!(!link.highscores_request_bound(&expected, Action::Refresh));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.category, query.vocation, query.page), (2, 3, 1));
    assert!(link.highscores_panel().reply.is_none());
    let mut reply = wire::HighscoresResult {
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
    };
    let before = link.highscores_panel();
    outbox.highscores_result(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: wire::encode_highscores_result(&reply).map_err(|e| format!("{e:?}"))?,
    });
    assert_eq!(link.highscores_panel(), before);
    reply.category = 2;
    reply.own_page = Some(2);
    reply.own_row = reply.rows.first().cloned();
    outbox.highscores_result(&ServerMechanicResponse {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: wire::encode_highscores_result(&reply).map_err(|e| format!("{e:?}"))?,
    });
    assert_eq!(link.highscores_panel().reply, Some(reply.clone()));
    let current = link.highscores_panel();
    assert!(link.highscores_request_bound(&current, Action::ShowMe));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("page command".into());
    };
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.page, query.snapshot_id), (2, 7));
    reply.page = 2;
    outbox.highscores_result(&ServerMechanicResponse {
        command_id: 3,
        server_sequence: 3,
        status: oteryn_session::CommandStatus::Accepted,
        payload: wire::encode_highscores_result(&reply).map_err(|e| format!("{e:?}"))?,
    });
    let world = highscores_world()?;
    assert!(link.highscores_request_bound(&link.highscores_panel(), Action::World(Some(world))));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("world command".into());
    };
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.world, query.page, query.snapshot_id),
        (Some(world), 1, 0)
    );
    assert!(link.highscores_panel().reply.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn character_reply(
    outbox: &crate::play::Outbox,
    command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    deaths: u16,
    selected: u8,
) {
    let mut payload = vec![0];
    for value in [12_i64, 99, 1, 2, 3, 4, 5, 6] {
        payload.extend(value.to_le_bytes());
    }
    payload.extend(deaths.to_le_bytes());
    payload.extend(0_u16.to_le_bytes());
    payload.extend([1, 1, selected]);
    super::super::super::feature_0097_character_info::protocol::apply(
        &mut lock(&outbox.0).properties.character_info,
        command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload,
        },
    );
}
fn character(outbox: &crate::play::Outbox) {
    lock(&outbox.0).properties.character_info.selected = true;
}
fn next(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
) -> Result<oteryn_client_runtime::raw_feature::FeatureCommand, Box<dyn std::error::Error>> {
    match commands.try_recv()? {
        crate::combat_input::PlayCommand::Feature(c) => Ok(c),
        _ => Err("feature command".into()),
    }
}
#[test]
fn character_info_shows_server_pages_only() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    character(&outbox);
    assert!(link.character_info_open_bound(&link.character_info_panel()));
    let panel = link.character_info_panel();
    let fields = ["0".into(), "3".into(), String::new(), String::new()];
    assert!(link.character_info_request_bound(&panel, fields.clone(), Intent::Query));
    assert!(link.character_info_panel().summary.is_empty());
    let command = next(&mut commands)?;
    assert_eq!(command.payload, [0, 0, 0, 3]);
    character_reply(&outbox, &command, 3, 1);
    let confirmed = link.character_info_panel();
    assert!(confirmed.summary.contains("Page deaths 3"));
    assert!(confirmed.own_titles.is_some());
    let mut page = fields;
    page[0] = "1".into();
    page[1] = "1".into();
    assert!(link.character_info_request_bound(&confirmed, page, Intent::Query));
    assert_eq!(link.character_info_panel().summary, confirmed.summary);
    let command = next(&mut commands)?;
    assert_eq!(command.payload, [0, 1, 0, 1]);
    character_reply(&outbox, &command, 1, 1);
    assert!(
        link.character_info_panel()
            .summary
            .contains("Page deaths 1")
    );
    Ok(())
}
#[test]
fn character_title_select_is_bound_without_optimistic_change() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    character(&outbox);
    assert!(link.character_info_open_bound(&link.character_info_panel()));
    let panel = link.character_info_panel();
    assert!(link.character_info_request_bound(
        &panel,
        [String::new(), String::new(), String::new(), String::new()],
        Intent::Query
    ));
    let command = next(&mut commands)?;
    character_reply(&outbox, &command, 0, 1);
    let panel = link.character_info_panel();
    let mut fields = panel.fields.clone();
    fields[3] = "0".into();
    assert!(link.character_info_request_bound(&panel, fields.clone(), Intent::Select));
    assert!(!link.character_info_request_bound(&panel, fields, Intent::Select));
    assert_eq!(link.character_info_panel().own_titles, panel.own_titles);
    let command = next(&mut commands)?;
    assert_eq!(command.payload, [3, 0]);
    super::super::super::feature_0097_character_info::protocol::apply(
        &mut lock(&outbox.0).properties.character_info,
        &command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: [vec![3], 0_u64.to_le_bytes().to_vec()].concat(),
        },
    );
    assert_eq!(
        link.character_info_panel()
            .own_titles
            .ok_or("titles")?
            .selected,
        None
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn highscores_and_character_info_rejected_when_session_replaced() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    outbox.highscores_initialize(true, false);
    character(&outbox);
    assert!(link.highscores_open_bound(&link.highscores_panel()));
    assert!(link.character_info_open_bound(&link.character_info_panel()));
    let scores = link.highscores_panel();
    let info = link.character_info_panel();
    link.use_shared_frontend();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.highscores_request_bound(&scores, Action::Refresh));
    assert!(!link.character_info_request_bound(&info, info.fields.clone(), Intent::Query));
    assert_eq!(link.highscores_panel(), scores);
    assert_eq!(link.character_info_panel(), info);
    drop(modal);
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let ended_scores = link.highscores_panel();
    let ended_info = link.character_info_panel();
    assert!(!link.highscores_request_bound(&scores, Action::Refresh));
    assert!(!link.character_info_request_bound(&info, info.fields.clone(), Intent::Query));
    assert!(!link.highscores_open_bound(&scores));
    assert!(!link.character_info_open_bound(&info));
    assert_eq!(link.highscores_panel(), ended_scores);
    assert_eq!(link.character_info_panel(), ended_info);
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn highscores_world()
-> Result<oteryn_client_runtime::highscores::wire::WorldId, Box<dyn std::error::Error>> {
    oteryn_client_ui::highscores::parse_world("01020304-0506-7008-800a-0b0c0d0e0f01")
        .flatten()
        .ok_or("world".into())
}
