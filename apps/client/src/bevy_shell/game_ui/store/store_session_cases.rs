#[test]
fn store_history_page() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    click(&mut app, Kind::History(false))?;
    let c = command(&mut rx)?;
    assert_eq!((c.capability, c.command_type), (285, 501));
    outbox.bevy_purchase_history_reply(&ServerMechanicResponse {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::discovery::store::encode_history_page(
            &oteryn_client_runtime::discovery::store::StoreHistoryPage {
                entries: vec![oteryn_client_runtime::discovery::store::StoreHistoryEntry {
                    order_id: [1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1],
                    offer_key: "store:test".into(),
                    catalog_revision: "server-revision".into(),
                    price: 100,
                    currency: "TIBIA_COIN".into(),
                    purchased_at_unix_ms: 1234,
                }],
                next_page_cursor: String::new(),
            },
        )
        .map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    assert!(text(&mut app).contains("store:test"));
    assert!(text(&mut app).contains("1234 ms"));
    assert!(text(&mut app).contains("End of purchase history"));
    click(&mut app, Kind::Ledger(false))?;
    assert_eq!(command(&mut rx)?.command_type, 184);
    Ok(())
}
#[test]
fn store_stale_confirmation_and_closed_window_cannot_purchase() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    click(&mut app, Kind::Buy)?;
    let confirm = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == Kind::Confirm).then_some(e))
        .ok_or("confirm")?;
    // An unsolicited catalogue/balance change invalidates the exact displayed consent.
    reply(
        &outbox,
        page(200).encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(rx.try_recv().is_err());
    assert!(app.world().resource::<State>().confirmation.is_none());
    click(&mut app, Kind::Close)?;
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
#[test]
fn store_shortcut_requests_catalogue() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_store_initialize();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(22)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        }],
        false,
    );
    assert!(owner.play.as_ref().ok_or("play")?.link.store_panel().open);
    assert_eq!(command(&mut rx)?.command_type, browse::COMMAND_TYPE);
    Ok(())
}

#[test]
fn store_shortcut_rejects_extra_modifiers() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_store_initialize();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    for extra in [Modifier::Alt, Modifier::Super] {
        owner.route_input(
            &[NormalizedInputEvent::Key {
                code: KeyCode::new(22)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control)
                    .with(Modifier::Shift)
                    .with(extra),
                repeat: false,
            }],
            false,
        );
        assert!(!owner.play.as_ref().ok_or("play")?.link.store_panel().open);
        assert!(rx.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn purchase_history_shortcut_opens_visible_history_page() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, _rx, _) = fixture(250)?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_set_open(false);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(11)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .store_panel()
            .open
    );
    app.update();
    assert!(matches!(
        app.world().resource::<State>().page,
        Page::History
    ));
    assert!(
        app.world_mut()
            .query_filtered::<&Node, With<Overlay>>()
            .iter(app.world())
            .any(|node| node.display != Display::None)
    );
    Ok(())
}

#[test]
fn team_finder_shortcut_presents_real_browse_controls() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut rx, outbox) = fixture(250)?;
    outbox.bevy_team_finder_supported();
    // The shortcut opens the retained Team Finder window over the discovery panel.
    super::super::team_finder::install(&mut app);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .use_shared_frontend();
    }
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_set_open(false);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(23)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Team Finder: Refresh").then_some(entity))
        .ok_or("team browse control")?;
    app.world_mut().trigger(Activate { entity });
    assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|command| matches!(command, PlayCommand::ServerMechanic(command) if command.capability == 232)));
    Ok(())
}

#[test]
fn store_search_and_sort_controls_send_catalogue_queries() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    assert!(text(&mut app).contains("Search products"));
    for (field, mut value) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 3 {
            *value = EditableText::new("outfit");
        }
    }
    let search = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| {
            let children = app.world().get::<Children>(e)?;
            children
                .iter()
                .any(|child| {
                    app.world()
                        .get::<Text>(child)
                        .is_some_and(|t| t.0 == "Search")
                })
                .then_some(*a)
        })
        .ok_or("search button")?;
    click(&mut app, search.kind)?;
    let query = browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.category.as_str(), query.search.as_str(), query.sort),
        ("featured", "outfit", 0)
    );
    reply(
        &outbox,
        page(250).encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.update();
    let sort = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| {
            let children = app.world().get::<Children>(e)?;
            children
                .iter()
                .any(|child| {
                    app.world()
                        .get::<Text>(child)
                        .is_some_and(|t| t.0 == "Price low–high")
                })
                .then_some(*a)
        })
        .ok_or("sort button")?;
    click(&mut app, sort.kind)?;
    let query = browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((query.search.as_str(), query.sort), ("outfit", 3));
    Ok(())
}

#[test]
fn store_filters_survive_paging_and_reset_cursors_when_changed() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    for (field, mut value) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 3 {
            *value = EditableText::new("outfit");
        }
    }
    click(&mut app, Kind::Search)?;
    let _ = command(&mut rx)?;
    // Pending requests cannot be replaced by a second search or sort.
    click(&mut app, Kind::Sort(4))?;
    assert!(rx.try_recv().is_err());
    let mut next = page(250);
    next.next = "server-next-page".into();
    reply(
        &outbox,
        next.encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.update();
    click(&mut app, Kind::Page(1))?;
    let query = browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (query.search.as_str(), query.sort, query.cursor.as_str()),
        ("outfit", 0, "server-next-page")
    );
    reply(
        &outbox,
        next.encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.update();
    for sort in [1, 2, 3, 4, 0] {
        click(&mut app, Kind::Sort(sort))?;
        let query =
            browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (query.search.as_str(), query.sort, query.cursor.as_str()),
            ("outfit", sort, "")
        );
        reply(
            &outbox,
            next.encode().map_err(|e| format!("{e:?}"))?,
            CommandStatus::Accepted,
        );
        app.update();
    }
    click(&mut app, Kind::Category(2))?;
    let query = browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            query.category.as_str(),
            query.search.as_str(),
            query.cursor.as_str()
        ),
        ("cosmetics.outfits", "outfit", "")
    );
    reply(
        &outbox,
        next.encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.update();
    click(&mut app, Kind::ClearSearch)?;
    let query = browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            query.category.as_str(),
            query.search.as_str(),
            query.sort,
            query.cursor.as_str()
        ),
        ("cosmetics.outfits", "", 0, "")
    );
    let mut empty = page(250);
    empty.products.clear();
    reply(
        &outbox,
        empty.encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.update();
    assert!(text(&mut app).contains("No products on this page"));
    assert!(
        !app.world_mut()
            .query::<&Action>()
            .iter(app.world())
            .any(|a| a.kind == Kind::Buy)
    );
    assert!(rx.try_recv().is_err());
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(field, text)| field.0 == 3 && text.value().to_string().is_empty())
    );
    Ok(())
}

#[test]
fn store_exposes_coin_checkout_with_an_admitted_account() -> ResultTest {
    let (mut app, _, _) = fixture(250)?;
    let config = crate::NativeLoginConfig::from_env(|key| match key {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some("https://localhost:1".into()),
        "OTERYN_OAUTH_CLIENT_ID" => Some("test-client".into()),
        "OTERYN_DEV_ROOT" => Some("/tmp/unused-store-checkout-root".into()),
        _ => None,
    })?.ok_or("config")?;
    app.world().non_send::<HostOwner>().play.as_ref().ok_or("play")?
        .link.configure_coin_shop(Some(config), "hero");
    app.update();
    let world = app.world_mut();
    let button = world.query::<(Entity, &Text, &ChildOf)>().iter(world)
        .find_map(|(_, text, parent)| (text.0 == "Get Oteryn coins").then_some(parent.parent()))
        .ok_or("coin checkout button")?;
    assert!(world.get::<InteractionDisabled>(button).is_none());
    Ok(())
}

#[test]
fn store_get_coins_opens_the_account_catalogue_for_the_admitted_character() -> ResultTest {
    use crate::{
        account_services::test_server::{CHARACTER, MockPlatform, coins::PACKAGE, test_thread},
        native_entry::test_support::Stack,
    };
    use std::time::{Duration, Instant};

    test_thread(|| {
        let stack = Stack::start_with_frames(vec![], vec![])?;
        let mock = MockPlatform::start()?;
        let mut app = App::new();
        crate::bevy_shell::account_services::install(&mut app);
        let (mut app, _commands, _) = fixture_in(app, 250)?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let owner = app.world().non_send::<HostOwner>();
        owner.services.configure(&mut config);
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .configure_coin_shop(Some(config), CHARACTER);
        app.update();
        click(&mut app, Kind::CoinCheckout)?;

        let deadline = Instant::now() + Duration::from_secs(5);
        let package = format!("coin.package.{PACKAGE}");
        loop {
            app.update();
            let world = app.world_mut();
            if world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == package)
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err("Store did not open the authenticated coin packages".into());
            }
            std::thread::yield_now();
        }
        let request = format!(
            "GET /api/v1/native-account/coin-purchases/catalogue?character_id={CHARACTER} "
        );
        assert!(
            mock.authority
                .lock()
                .map_err(|_| "authority")?
                .requests
                .iter()
                .any(|line| line.starts_with(&request))
        );
        drop(app);
        stack.finish();
        Ok(())
    })
}

#[test]
fn native_admission_wires_store_checkout_to_the_selected_character() -> ResultTest {
    use crate::{
        account_services::test_server::{CHARACTER, MockPlatform, coins::PACKAGE, test_thread},
        native_entry::test_support::Stack,
    };
    use std::time::{Duration, Instant};

    test_thread(|| {
        let stack = Stack::start_two_channels()?;
        let mock = MockPlatform::start()?;
        let mut app = App::new();
        crate::bevy_shell::account_services::install(&mut app);
        let (mut app, _, _) = fixture_in(app, 250)?;
        let mut config = stack.config();
        config.platform_url.clone_from(&mock.base);
        app.world().non_send::<HostOwner>().services.configure(&mut config);

        let client = crate::ClientBootstrap::new()?.with_native_login(config);
        let admitted = client.request_gameplay_entry()?;
        assert_eq!(admitted.character_id(), CHARACTER);
        let (view, link) = client.start_play(admitted)?;
        assert!(Snapshot::read(&link).coin_checkout);
        assert!(link.open_coin_checkout());
        app.world_mut().non_send_mut::<HostOwner>().play = Some(crate::bevy_shell::owner::ActivePlay { view, link });

        let deadline = Instant::now() + Duration::from_secs(5);
        let package = format!("coin.package.{PACKAGE}");
        loop {
            app.update();
            let world = app.world_mut();
            if world.query::<&Name>().iter(world).any(|name| name.as_str() == package) {
                break;
            }
            if Instant::now() >= deadline {
                return Err("admitted Store launcher did not open the coin catalogue".into());
            }
            std::thread::yield_now();
        }
        let request = format!("GET /api/v1/native-account/coin-purchases/catalogue?character_id={CHARACTER} ");
        assert!(mock.authority.lock().map_err(|_| "authority")?.requests.iter().any(|line| line.starts_with(&request)));
        drop(app);
        client.shutdown();
        stack.finish();
        Ok(())
    })
}
