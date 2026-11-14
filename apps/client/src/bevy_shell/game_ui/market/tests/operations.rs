use super::*;

fn find(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, value)| (value.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing {name}").into())
}
fn offer() -> oteryn_client_runtime::market::market::MarketRow {
    let mut id = [7; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    oteryn_client_runtime::market::market::MarketRow {
        offer: id,
        definition: 42,
        side: 2,
        amount: 5,
        price: 100,
        state: 1,
        anonymous: true,
    }
}
fn book(app: &mut App, commands: &mut CommandsReceiver, outbox: &Outbox) -> TestResult {
    app.world_mut().non_send_mut::<HostOwner>().phase = crate::bevy_shell::owner::Phase::Playing;
    app.init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        })
        .add_systems(Update, project);
    app.world_mut().resource_mut::<State>().seen = None;
    app.update();
    let select = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.kind, Kind::Item(0)).then_some(entity))
        .ok_or("named catalogue row")?;
    app.world_mut().trigger(Activate { entity: select });
    let command = feature(commands)?;
    outbox.bevy_market_reply(
        &command,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![offer()],
            has_next: false,
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
        })?,
    );
    super::super::purchase::install(app);
    app.update();
    Ok(())
}

fn panel(app: &App) -> Result<MarketPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .market_panel())
}

fn ready() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (mut app, mut commands, outbox, root) = app()?;
    book(&mut app, &mut commands, &outbox)?;
    super::super::operations::install(&mut app);
    app.update();
    let open = find(&mut app, "market.operations.open")?;
    app.world_mut().trigger(Activate { entity: open });
    app.update();
    Ok((app, commands, outbox, root))
}
fn click(app: &mut App, name: &str) -> TestResult {
    let entity = find(app, name)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn reply(outbox: &Outbox, action: u8, rows: Vec<oteryn_client_runtime::market::market::MarketRow>) {
    outbox.bevy_market_response(&oteryn_client_runtime::market::MarketResponse {
        status: CommandStatus::Accepted,
        reply: Some(oteryn_client_runtime::market::market::MarketReply {
            action,
            code: 1,
            offer: [0; 16],
            rows,
        }),
    });
}
#[test]
fn market_create_offer_confirms_and_dispatches_bound() -> TestResult {
    for sell in [true, false] {
        let (mut app, mut commands, _, _) = ready()?;
        for (name, value) in [("amount", "3"), ("price", "100")] {
            let field = find(&mut app, &format!("market.operations.{name}"))?;
            app.world_mut()
                .entity_mut(field)
                .insert(EditableText::new(value));
        }
        click(&mut app, "market.operations.anonymous")?;
        click(
            &mut app,
            if sell {
                "market.operations.sell"
            } else {
                "market.operations.buy"
            },
        )?;
        assert!(commands.try_recv().is_err());
        assert!(panel(&app)?.pending.is_none());
        let confirm = find(&mut app, "market.operations.confirm")?;
        app.world_mut().trigger(Activate { entity: confirm });
        let PlayCommand::Market(command) = commands.try_recv()? else {
            return Err("create command".into());
        };
        assert_eq!(
            (
                command.0.action,
                command.0.definition,
                command.0.amount,
                command.0.price,
                command.0.anonymous
            ),
            (if sell { 6 } else { 5 }, 42, 3, 100, true)
        );
        app.world_mut().trigger(Activate { entity: confirm });
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn market_cancel_own_offer_dispatches_and_waits_for_server() -> TestResult {
    let (mut app, mut commands, outbox, _) = ready()?;
    click(&mut app, "market.operations.own")?;
    let PlayCommand::Market(query) = commands.try_recv()? else {
        return Err("own query".into());
    };
    assert_eq!(query.0.action, 3);
    reply(&outbox, 3, vec![offer()]);
    app.update();
    assert!(find(&mut app, "market.purchase.row").is_err());
    click(&mut app, "market.operations.cancel.0")?;
    assert!(commands.try_recv().is_err());
    click(&mut app, "market.operations.confirm")?;
    let PlayCommand::Market(command) = commands.try_recv()? else {
        return Err("cancel command".into());
    };
    assert_eq!((command.0.action, command.0.offer), (7, offer().offer));
    assert_eq!(panel(&app)?.rows, vec![offer()]);
    assert!(panel(&app)?.pending.is_some());
    reply(&outbox, 7, vec![]);
    assert_eq!(panel(&app)?.rows, vec![offer()]);
    Ok(())
}
#[test]
fn market_history_shows_server_rows_only() -> TestResult {
    let (mut app, mut commands, outbox, _) = ready()?;
    click(&mut app, "market.operations.history")?;
    let PlayCommand::Market(query) = commands.try_recv()? else {
        return Err("history query".into());
    };
    assert_eq!(query.0.action, 4);
    assert!(find(&mut app, "market.operations.history.0").is_err());
    let mut transaction = offer();
    transaction.amount = 123;
    transaction.state = 4;
    reply(&outbox, 4, vec![transaction]);
    app.update();
    assert!(find(&mut app, "market.purchase.row").is_err());
    let row = find(&mut app, "market.operations.history.0")?;
    assert!(
        app.world()
            .get::<Text>(row)
            .ok_or("history text")?
            .0
            .contains("123")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn market_operations_rejected_when_session_replaced() -> TestResult {
    for reason in ["replaced", "closed", "modal", "ended"] {
        let (mut app, mut commands, _, _) = ready()?;
        click(&mut app, "market.operations.sell")?;
        let confirm = find(&mut app, "market.operations.confirm")?;
        if reason == "replaced" {
            let (link, _, outbox) = play_channel();
            outbox.bevy_market_initialize(true);
            link.market_set_open(true);
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .play
                .as_mut()
                .ok_or("play")?
                .link = link;
        } else if reason == "closed" {
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .market_set_open(false);
        } else if reason == "modal" {
            app.world_mut()
                .spawn((GameModal, GlobalZIndex(999), Node::default()));
        } else {
            app.world_mut().non_send_mut::<HostOwner>().phase =
                crate::bevy_shell::owner::Phase::Closed;
        }
        let before = panel(&app)?;
        app.world_mut().trigger(Activate { entity: confirm });
        assert_eq!(before, panel(&app)?);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn coin_market_owner_confirms_once_and_retries_bound_session() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    outbox.bevy_market_initialize(true);
    let binding = link.market_session_binding();
    assert!(link.coin_market_open_bound(binding));
    let PlayCommand::ServerMechanic(_) = commands.try_recv()? else {
        return Err("book request".into());
    };
    outbox.bevy_coin_market_response(&GatedCommandResult {
        command_id: 1, server_sequence: 1, status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::coin_market::encode_reply(r#"{"result":"ok","offers":[{"offer":"07070707-0707-7007-8007-070707070707","remaining":5,"price":100,"side":"sell","state":1}]}"#).map_err(|e| format!("{e:?}"))?,
    });
    let fields = ["2".into(), "100".into(), "1".into()];
    let panel = link.market_panel();
    assert!(!link.coin_market_bound(binding, &panel.coins, 6, fields.clone()));
    assert!(commands.try_recv().is_err());
    assert!(link.coin_market_bound(binding, &panel.coins, 4, fields.clone()));
    assert!(commands.try_recv().is_err());
    let reviewed = link.market_panel();
    assert!(reviewed.coins.reviewed.is_some());
    assert!(!link.coin_market_bound(binding.wrapping_add(1), &reviewed.coins, 6, fields.clone()));
    assert_eq!(link.market_panel(), reviewed);
    assert!(link.coin_market_bound(binding, &reviewed.coins, 6, fields.clone()));
    let PlayCommand::ServerMechanic(first) = commands.try_recv()? else {
        return Err("accept command".into());
    };
    let request =
        oteryn_client_runtime::coin_market::decode(&first.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.action, request.amount), (5, 2));
    assert!(!link.coin_market_bound(binding, &reviewed.coins, 6, fields.clone()));
    outbox.bevy_coin_market_response(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::coin_market::encode_reply(r#"{"result":"unavailable"}"#)
            .map_err(|e| format!("{e:?}"))?,
    });
    let retry = link.market_panel();
    assert!(link.coin_market_bound(binding, &retry.coins, 7, fields));
    let PlayCommand::ServerMechanic(second) = commands.try_recv()? else {
        return Err("retry".into());
    };
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn coin_market_accept_requires_confirm_and_bound_session() -> TestResult {
    let (mut app, mut commands, outbox, _) = ready()?;
    crate::bevy_shell::game_ui::coin_market::install(&mut app);
    click(&mut app, "market.operations.coins")?;
    let PlayCommand::ServerMechanic(book) = commands.try_recv()? else {
        return Err("coin book command".into());
    };
    assert_eq!(
        oteryn_client_runtime::coin_market::decode(&book.payload)
            .map_err(|e| format!("{e:?}"))?
            .action,
        1
    );
    outbox.bevy_coin_market_response(&GatedCommandResult {
        command_id: 1, server_sequence: 1, status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::coin_market::encode_reply(r#"{"result":"ok","offers":[{"offer":"07070707-0707-7007-8007-070707070707","remaining":5,"price":100,"side":"sell","state":1}]}"#).map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    for (index, value) in ["2", "100", "1"].into_iter().enumerate() {
        let field = find(&mut app, &format!("coin_market.field.{index}"))?;
        app.world_mut()
            .entity_mut(field)
            .insert(EditableText::new(value));
    }
    click(&mut app, "coin_market.action.4")?;
    assert!(commands.try_recv().is_err());
    let label = find(&mut app, "coin_market.server")?;
    assert!(
        app.world()
            .get::<Text>(label)
            .ok_or("coin review")?
            .0
            .contains("REVIEW Buy from sell offer")
    );
    let confirm = find(&mut app, "coin_market.action.6")?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    let before = panel(&app)?;
    app.world_mut().trigger(Activate { entity: confirm });
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.world_mut().trigger(Activate { entity: confirm });
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("coin take command".into());
    };
    let request = oteryn_client_runtime::coin_market::decode(&command.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.action, request.amount), (5, 2));
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?.coins.view, before.coins.view);
    Ok(())
}
