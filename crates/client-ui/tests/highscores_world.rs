use oteryn_client_runtime::highscores::wire::{self, HighscoreRow, HighscoresResult};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use oteryn_client_ui::highscores::{self, Action, HighscoresPanel};

#[test]
fn show_me_uses_stored_filter_position_instead_of_global_rank()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = HighscoresPanel::default();
    panel.selected = true;
    highscores::request(
        &mut panel,
        Action::Filter {
            category: 1,
            vocation: 1,
        },
    )
    .ok_or("request")?;
    let result = HighscoresResult {
        world: None,
        own_page: Some(2),
        category: 1,
        snapshot_id: 9,
        computed_at: 100,
        page: 1,
        has_more: false,
        rows: vec![],
        own_row: Some(HighscoreRow {
            rank: 1500,
            name: "Me".into(),
            vocation: "knight".into(),
            level: 10,
            value: 20,
        }),
    };
    highscores::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_highscores_result(&result).map_err(|e| format!("{e:?}"))?,
        },
    );
    let command = highscores::request(&mut panel, Action::ShowMe).ok_or("show me")?;
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.page, query.snapshot_id), (2, 9));
    Ok(())
}

#[test]
fn world_selection_resets_the_snapshot_and_rejects_a_foreign_reply()
-> Result<(), Box<dyn std::error::Error>> {
    let world = highscores::parse_world("5a020304-0506-7008-800a-0b0c0d0e0f5a")
        .ok_or("world")?
        .ok_or("world ID")?;
    assert!(highscores::parse_world("not a world").is_none());
    assert_eq!(highscores::parse_world(" "), Some(None));
    assert_eq!(
        highscores::parse_world(&highscores::world_text(world)),
        Some(Some(world))
    );
    let mut panel = HighscoresPanel::default();
    panel.selected = true;
    let command =
        highscores::request(&mut panel, Action::World(Some(world))).ok_or("world query")?;
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.world, query.page, query.snapshot_id, query.details),
        (Some(world), 1, 0, true)
    );
    let mut reply = HighscoresResult {
        category: 1,
        snapshot_id: 4,
        computed_at: 100,
        page: 1,
        has_more: false,
        rows: vec![],
        own_row: None,
        world: None,
        own_page: None,
    };
    let response = |reply: &HighscoresResult| -> Result<ServerMechanicResponse, String> {
        Ok(ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_highscores_result(reply).map_err(|e| format!("{e:?}"))?,
        })
    };
    highscores::apply_result(&mut panel, &response(&reply)?);
    assert!(panel.reply.is_none());
    highscores::request(&mut panel, Action::Refresh).ok_or("refresh")?;
    reply.world = Some(world);
    highscores::apply_result(&mut panel, &response(&reply)?);
    assert!(panel.reply.is_some());
    assert!(highscores::request(&mut panel, Action::ShowMe).is_none());
    let command = highscores::request(&mut panel, Action::World(None)).ok_or("home")?;
    let query = wire::decode_highscores_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.world, query.page, query.snapshot_id), (None, 1, 0));
    assert!(panel.reply.is_none());
    Ok(())
}
