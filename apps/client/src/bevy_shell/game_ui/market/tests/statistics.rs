use super::*;

#[test]
fn production_market_charts_server_prices_and_clears_stale_statistics() -> TestResult {
    let (mut app, mut commands, outbox, root) = app()?;
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
    let select = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: select });
    let book = feature(&mut commands)?;
    let summary = wire::PriceSummary {
        trades: 2,
        units: 3,
        minimum: Some(100),
        maximum: Some(300),
        average: Some(200),
    };
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
                since_us: 0,
                trades: 4,
                units: 6,
                minimum: Some(100),
                maximum: Some(300),
                average: Some(200),
                buy: summary.clone(),
                sell: summary,
            },
        })?,
    );
    app.update();
    let ranges: Vec<Node> = app
        .world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "market.price.range")
        .map(|(_, node)| node.clone())
        .collect();
    assert_eq!(
        ranges.len(),
        2,
        "both server sides must have a graphical range"
    );
    assert_eq!(ranges[0].left, percent(100. / 3.));
    assert_eq!(ranges[0].width, percent(200. / 3.));
    let averages: Vec<Val> = app
        .world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "market.price.average")
        .map(|(_, node)| node.left)
        .collect();
    assert_eq!(averages, vec![percent(200. / 3.); 2]);
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(texts.iter().any(|text| text.contains("Required level: 30")));
    assert!(texts.iter().any(|text| text.contains("0–300 gold")));

    let host = app.world().non_send::<HostOwner>();
    assert!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .market_query(MarketQuery::Book {
                definition: 43,
                page: 0
            })
    );
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "market.price.range")
            .count(),
        0
    );
    Ok(())
}

#[test]
fn production_market_charts_flat_prices_and_preserves_empty_sides() -> TestResult {
    let (mut app, mut commands, outbox, root) = app()?;
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
    let select = button(&mut app, root, Kind::Item(0))?;
    app.world_mut().trigger(Activate { entity: select });
    let book = feature(&mut commands)?;
    let price = 999999999999;
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
                since_us: 0,
                trades: 1,
                units: 1,
                minimum: Some(price),
                maximum: Some(price),
                average: Some(price),
                buy: wire::PriceSummary {
                    trades: 1,
                    units: 1,
                    minimum: Some(price),
                    maximum: Some(price),
                    average: Some(price),
                },
                sell: wire::PriceSummary::default(),
            },
        })?,
    );
    app.update();
    let markers: Vec<Val> = app
        .world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "market.price.average")
        .map(|(_, node)| node.left)
        .collect();
    assert_eq!(markers, vec![percent(100.)]);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Sell offers: no completed trades")
    );
    let refresh = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.kind, Kind::Statistics).then_some(entity))
        .ok_or("refresh statistics button")?;
    app.world_mut().trigger(Activate { entity: refresh });
    let command = feature(&mut commands)?;
    outbox.bevy_market_reply(
        &command,
        &result(&wire::Reply::Statistics(wire::Statistics {
            definition: 42,
            as_of_us: 2592000000000,
            ..default()
        }))?,
    );
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "market.price.average")
            .count(),
        0
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "No completed prices to chart")
    );
    Ok(())
}
