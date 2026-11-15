use super::*;
use oteryn_client_ui::market::{MarketResponse, MarketView, market};
fn find(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(id, n)| (n.as_str() == name).then_some(id))
        .ok_or_else(|| name.to_owned().into())
}
fn click(app: &mut App, name: &str) -> TestResult {
    let entity = find(app, name)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn edit(app: &mut App, name: &str, value: &str) -> TestResult {
    let entity = find(app, name)?;
    app.world_mut()
        .entity_mut(entity)
        .insert(EditableText::new(value));
    app.update();
    Ok(())
}
fn command(
    commands: &mut CommandsReceiver,
) -> Result<market::MarketRequest, Box<dyn std::error::Error>> {
    let PlayCommand::Market(command) = commands.try_recv()? else {
        return Err("market command".into());
    };
    Ok(command.0)
}
fn row() -> market::MarketRow {
    let mut offer = [7; 16];
    offer[6] = 0x70;
    offer[8] = 0x80;
    market::MarketRow {
        offer,
        definition: 42,
        side: 2,
        amount: 5,
        price: 100,
        state: 1,
        anonymous: false,
    }
}
fn setup() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (mut app, commands, outbox, root) = app()?;
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
    super::super::offers::install(&mut app);
    app.world_mut().resource_mut::<State>().seen = None;
    app.update();
    Ok((app, commands, outbox, root))
}
fn reply(outbox: &Outbox, action: u8, rows: Vec<market::MarketRow>) {
    outbox.bevy_market_response(&MarketResponse {
        status: CommandStatus::Accepted,
        reply: Some(market::MarketReply {
            action,
            code: 1,
            offer: [0; 16],
            rows,
        }),
    });
}
#[test]
fn market_my_offers_tab_lists_own_offers() -> TestResult {
    let (mut app, mut commands, outbox, _) = setup()?;
    click(&mut app, "market.tab.own")?;
    assert_eq!(command(&mut commands)?.action, 3);
    reply(&outbox, 3, vec![row()]);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("amount 5"))
    );
    assert_eq!(
        app.world()
            .resource::<State>()
            .seen
            .as_ref()
            .ok_or("panel")?
            .view,
        MarketView::Own
    );
    Ok(())
}
#[test]
fn market_history_tab() -> TestResult {
    let (mut app, mut commands, outbox, _) = setup()?;
    click(&mut app, "market.tab.history")?;
    assert_eq!(command(&mut commands)?.action, 4);
    let mut history = row();
    history.state = 2;
    reply(&outbox, 4, vec![history]);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("amount 5"))
    );
    assert!(find(&mut app, "market.offer.cancel").is_err());
    assert!(find(&mut app, "market.offer.accept").is_err());
    Ok(())
}
#[test]
fn market_cancel_own_offer_sends_intent() -> TestResult {
    let (mut app, mut commands, outbox, _) = setup()?;
    click(&mut app, "market.tab.own")?;
    command(&mut commands)?;
    reply(&outbox, 3, vec![row()]);
    app.update();
    click(&mut app, "market.offer.select.0")?;
    click(&mut app, "market.offer.cancel")?;
    let request = command(&mut commands)?;
    assert_eq!(request.action, 7);
    assert_eq!(request.offer, row().offer);
    Ok(())
}
#[test]
fn market_accept_clamps_amount() -> TestResult {
    for (side, input, expected) in [(2, "999", 5), (2, "0", 1), (1, "999", 4)] {
        let (mut app, mut commands, outbox, root) = setup()?;
        let item = button(&mut app, root, Kind::Item(0))?;
        app.world_mut().trigger(Activate { entity: item });
        let book = feature(&mut commands)?;
        let mut offer = row();
        offer.side = side;
        outbox.bevy_market_reply(
            &book,
            &result(&wire::Reply::Book {
                definition: 42,
                page: 0,
                rows: vec![offer],
                statistics: wire::Statistics {
                    definition: 42,
                    as_of_us: 2592000000000,
                    ..default()
                },
                has_next: false,
            })?,
        );
        app.update();
        click(&mut app, "market.offer.select.0")?;
        edit(&mut app, "market.offer.quantity", input)?;
        click(&mut app, "market.offer.accept")?;
        let request = command(&mut commands)?;
        assert_eq!(request.action, 8);
        assert_eq!(request.amount, expected);
        assert_eq!(request.offer, row().offer);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn market_create_offer_validation() -> TestResult {
    let (mut app, mut commands, outbox, root) = setup()?;
    let confirm = find(&mut app, "market.create.confirm")?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(confirm)
            .is_some()
    );
    let item = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: item });
    let book = feature(&mut commands)?;
    outbox.bevy_market_reply(
        &book,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![],
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
            has_next: false,
        })?,
    );
    app.update();
    for (amount, price) in [
        ("0", "1"),
        ("1", "0"),
        ("64001", "1"),
        ("64000", "999999999999"),
        ("bad", "1"),
    ] {
        edit(&mut app, "market.create.amount", amount)?;
        edit(&mut app, "market.create.price", price)?;
        let confirm = find(&mut app, "market.create.confirm")?;
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(confirm)
                .is_some()
        );
        click(&mut app, "market.create.confirm")?;
        assert!(commands.try_recv().is_err());
    }
    edit(&mut app, "market.create.amount", "2")?;
    edit(&mut app, "market.create.price", "100")?;
    click(&mut app, "market.create.buy")?;
    click(&mut app, "market.create.confirm")?;
    let request = command(&mut commands)?;
    assert_eq!(
        (
            request.action,
            request.definition,
            request.amount,
            request.price
        ),
        (5, 42, 2, 100)
    );
    reply(&outbox, 5, vec![]);
    app.update();
    click(&mut app, "market.create.sell")?;
    click(&mut app, "market.create.confirm")?;
    assert_eq!(command(&mut commands)?.action, 6);
    Ok(())
}

#[test]
fn market_create_rechecks_modal_and_pending_text_edits() -> TestResult {
    use std::sync::{Arc, Mutex};
    let (mut app, mut commands, outbox, root) = setup()?;
    let item = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: item });
    let book = feature(&mut commands)?;
    outbox.bevy_market_reply(
        &book,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![],
            has_next: false,
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
        })?,
    );
    app.update();
    let price = find(&mut app, "market.create.price")?;
    let mut text = EditableText::new("100");
    text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::new(
        Mutex::new(None),
    )));
    app.world_mut().entity_mut(price).insert(text);
    click(&mut app, "market.create.confirm")?;
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .entity_mut(price)
        .insert(EditableText::new("0"));
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "a pending paste must not submit stale price"
    );
    edit(&mut app, "market.create.price", "100")?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    click(&mut app, "market.create.confirm")?;
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    let confirm = find(&mut app, "market.create.confirm")?;
    let owner = app.world().non_send::<HostOwner>();
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .market_set_open(false);
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn market_tabs_return_to_catalogue_from_history() -> TestResult {
    let (mut app, mut commands, outbox, _) = setup()?;
    click(&mut app, "market.tab.history")?;
    command(&mut commands)?;
    reply(&outbox, 4, vec![]);
    app.update();
    click(&mut app, "market.tab.browse")?;
    assert_eq!(command(&mut commands)?.action, 1);
    reply(&outbox, 1, vec![]);
    app.update();
    assert_eq!(
        app.world()
            .resource::<State>()
            .seen
            .as_ref()
            .ok_or("panel")?
            .view,
        MarketView::Browse
    );
    Ok(())
}

#[test]
fn market_accept_buy_offer_without_items_is_disabled() -> TestResult {
    let (mut app, mut commands, outbox, root) = setup()?;
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.market_query(MarketQuery::Catalogue {
        filter: Filter::default(),
        page: 0
    }));
    let query = feature(&mut commands)?;
    let mut catalogue = catalogue();
    let wire::Reply::Catalogue { items, .. } = &mut catalogue else {
        return Err("catalogue".into());
    };
    items[0].depot_units = 0;
    outbox.bevy_market_reply(&query, &result(&catalogue)?);
    app.update();
    let item = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: item });
    let book = feature(&mut commands)?;
    let mut offer = row();
    offer.side = 1;
    outbox.bevy_market_reply(
        &book,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![offer],
            has_next: false,
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
        })?,
    );
    app.update();
    click(&mut app, "market.offer.select.0")?;
    let accept = find(&mut app, "market.offer.accept")?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(accept)
            .is_some()
    );
    click(&mut app, "market.offer.accept")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn market_anonymous_checkbox_is_retained_in_buy_and_sell_requests() -> TestResult {
    let (mut app, mut commands, outbox, root) = setup()?;
    let item = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: item });
    let book = feature(&mut commands)?;
    outbox.bevy_market_reply(
        &book,
        &result(&wire::Reply::Book {
            definition: 42,
            page: 0,
            rows: vec![],
            statistics: wire::Statistics {
                definition: 42,
                as_of_us: 2592000000000,
                ..default()
            },
            has_next: false,
        })?,
    );
    app.update();
    edit(&mut app, "market.create.amount", "2")?;
    edit(&mut app, "market.create.price", "100")?;
    let checkbox = find(&mut app, "market.create.anonymous")?;
    app.world_mut().trigger(bevy::ui_widgets::ValueChange {
        source: checkbox,
        value: true,
        is_final: true,
    });
    app.update();
    for (side, action) in [("market.create.buy", 5), ("market.create.sell", 6)] {
        click(&mut app, side)?;
        click(&mut app, "market.create.confirm")?;
        let request = command(&mut commands)?;
        assert_eq!(request.action, action);
        assert!(request.anonymous);
        reply(&outbox, action, vec![]);
        app.update();
    }
    Ok(())
}

#[test]
fn market_catalogue_controls_submit_all_supported_filters() -> TestResult {
    let (mut app, mut commands, _, _) = setup()?;
    for (name, value) in [
        ("vocation", "2"),
        ("minimum", "10"),
        ("maximum", "100"),
        ("forge", "3"),
    ] {
        edit(&mut app, &format!("market.filter.{name}"), value)?;
    }
    let depot = find(&mut app, "market.filter.depot")?;
    app.world_mut().trigger(bevy::ui_widgets::ValueChange {
        source: depot,
        value: true,
        is_final: true,
    });
    app.update();
    click(&mut app, "market.filter.apply")?;
    let command = feature(&mut commands)?;
    let wire::Query::Catalogue { filter, page } =
        wire::decode_query(&command.payload).map_err(|e| format!("{e:?}"))?
    else {
        return Err("catalogue query".into());
    };
    assert_eq!(page, 0);
    assert_eq!(filter.vocation, Some(2));
    assert_eq!(filter.minimum_level, Some(10));
    assert_eq!(filter.maximum_level, Some(100));
    assert_eq!(filter.forge_class, Some(3));
    assert!(filter.depot_only);
    Ok(())
}

#[test]
fn market_catalogue_rejects_invalid_filter_fields() -> TestResult {
    let (mut app, mut commands, _, _) = setup()?;
    for value in ["0", "7", "bad"] {
        edit(&mut app, "market.filter.vocation", value)?;
        click(&mut app, "market.filter.apply")?;
        assert!(commands.try_recv().is_err());
    }
    edit(&mut app, "market.filter.vocation", "")?;
    edit(&mut app, "market.filter.minimum", "100")?;
    edit(&mut app, "market.filter.maximum", "10")?;
    click(&mut app, "market.filter.apply")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
