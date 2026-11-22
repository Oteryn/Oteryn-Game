use super::*;
use crate::account_services::test_server::{MockPlatform, TestResult, test_thread};
use oteryn_foundation::CancellationToken;
use std::{
    thread,
    time::{Duration, Instant},
};

fn wait(app: &mut App, after: Option<u64>) -> Result<u64, &'static str> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.update();
        if let Some(id) = app
            .world()
            .non_send::<HostOwner>()
            .services
            .pending()
            .map(|r| r.id)
            && Some(id) != after
        {
            return Ok(id);
        }
        thread::yield_now();
    }
    Err("account presenter did not reach Bevy")
}
fn click(app: &mut App, action: Action) -> TestResult {
    let id = app
        .world()
        .non_send::<HostOwner>()
        .services
        .pending()
        .ok_or("request")?
        .id;
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, c)| c.request == id && c.action == action)
        .map(|(e, _)| e)
        .ok_or("button")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn premium_bevy_buttons_fetch_refresh_report_errors_and_return_to_account_menu() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let authority = mock.authority.clone();
        let owner = HostOwner::new(None)?;
        let mut config = crate::NativeLoginConfig::from_env(|key| match key {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(mock.base.clone()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("test-client".into()),
            "OTERYN_DEV_ROOT" => Some("/tmp/unused-premium-test-root".into()),
            _ => None,
        })?
        .ok_or("config")?;
        owner.services.configure(&mut config);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<InputFocus>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .insert_non_send(owner)
            .add_systems(Startup, super::super::ui::setup);
        install(&mut app);
        super::super::game_ui::modal::install(&mut app);
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            mock.runtime
                .block_on(crate::account_services::run(
                    &client,
                    &token,
                    &config,
                    &mut Default::default(),
                    CancellationToken::new(),
                ))
                .map_err(|e| format!("{e:?}"))
        })?;
        let menu = wait(&mut app, None)?;
        click(&mut app, Action::Premium)?;
        let premium = wait(&mut app, Some(menu))?;
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .single(app.world())?;
        assert!(text.0.contains("Private channels"));
        assert!(text.0.contains("Expiry:"));
        assert!(text.0.contains("[available]"));
        authority.lock().map_err(|_| "lock")?.premium_http_status = Some(503);
        click(&mut app, Action::Refresh)?;
        let failed = wait(&mut app, Some(premium))?;
        let text = app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .single(app.world())?;
        assert!(text.0.contains("Premium service unavailable"));
        assert!(!text.0.contains("[available]"));
        authority.lock().map_err(|_| "lock")?.premium_http_status = None;
        click(&mut app, Action::Refresh)?;
        let refreshed = wait(&mut app, Some(failed))?;
        // A stale button's request must not dismiss a refreshed view.
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .services
            .respond(premium, Action::Back);
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .map(|r| r.id),
            Some(refreshed)
        );
        click(&mut app, Action::Back)?;
        wait(&mut app, Some(refreshed))?;
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::Escape,
                logical_key: bevy::input::keyboard::Key::Escape,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        worker.join().map_err(|_| "worker")??;
        app.update();
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .is_none()
        );
        let state = authority.lock().map_err(|_| "lock")?;
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|r| r.starts_with("GET /api/v1/native-account/premium "))
                .count(),
            3
        );
        assert_eq!(state.effects, 0);
        Ok(())
    })
}

fn commerce_app() -> Result<(App, crate::NativeLoginConfig), Box<dyn std::error::Error>> {
    let owner = HostOwner::new(None)?;
    let mut config = crate::NativeLoginConfig::from_env(|key| match key {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some("https://localhost:1".into()),
        "OTERYN_OAUTH_CLIENT_ID" => Some("test-client".into()),
        "OTERYN_DEV_ROOT" => Some("/tmp/unused-commerce-test-root".into()),
        _ => None,
    })?
    .ok_or("config")?;
    owner.services.configure(&mut config);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, super::super::ui::setup);
    install(&mut app);
    super::super::game_ui::modal::install(&mut app);
    Ok((app, config))
}

fn click_named(app: &mut App, name: &str) -> TestResult {
    let world = app.world_mut();
    let entity = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing {name}"))?;
    world.trigger(Activate { entity });
    app.update();
    Ok(())
}

#[test]
fn account_services_routes_bazaar_and_transfer_controls_to_the_presenter() -> TestResult {
    use crate::account_services::{Action, View};
    use oteryn_platform_client::native_login::bazaar::{
        AuctionView, BazaarPage, OperationReceipt, OperationState, OwnedCharacter, TransferWorld,
    };
    let (mut app, config) = commerce_app()?;
    let present = config.account_services.clone();
    let worker = thread::spawn(move || present(&View::default(), CancellationToken::new()));
    let menu = wait(&mut app, None)?;
    click_named(&mut app, "account.bazaar.own")?;
    assert!(matches!(
        worker.join().map_err(|_| "Bazaar presenter")?,
        Action::Refresh(AuctionView::Own)
    ));

    let mut view = View::default();
    view.page = Some(BazaarPage {
        version: 1,
        view: AuctionView::Own,
        auctions: vec![],
        balance: 10,
        owned_characters: vec![OwnedCharacter {
            id: "hero".into(),
            name: "Hero".into(),
            world_id: "home".into(),
        }],
        worlds: vec![TransferWorld {
            id: "other".into(),
            name: "Other".into(),
        }],
        next_cursor: None,
        recent_operations: vec![OperationReceipt {
            version: 1,
            request_id: "prior".into(),
            state: OperationState::Committed,
            receipt_id: Some("receipt".into()),
            code: "transfer".into(),
            charged_coins: 1,
            balance: 10,
        }],
    });
    let presenter = config.account_services.clone();
    let receipt_view = view.clone();
    let worker = thread::spawn(move || presenter(&receipt_view, CancellationToken::new()));
    let receipt = wait(&mut app, Some(menu))?;
    click_named(&mut app, "account.receipt.prior")?;
    assert!(matches!(worker.join().map_err(|_| "receipt presenter")?,
        Action::Receipt(id) if id == "prior"));

    let worker = thread::spawn(move || (config.account_services)(&view, CancellationToken::new()));
    wait(&mut app, Some(receipt))?;
    click_named(&mut app, "account.character.hero")?;
    click_named(&mut app, "account.transfer.other")?;
    assert!(
        matches!(worker.join().map_err(|_| "transfer presenter")?, Action::Quote { character_id, world_id } if character_id == "hero" && world_id == "other")
    );
    Ok(())
}

#[test]
fn coin_shop_checkout_control_routes_to_the_authenticated_presenter() -> TestResult {
    use crate::coin_shop::{Action, View};
    let (mut app, config) = commerce_app()?;
    let worker = thread::spawn(move || {
        let mut view = View::default();
        view.order = Some(serde_json::from_value(serde_json::json!({
            "version": 1, "order_id":"order", "package_id":"small", "coins":10,
            "request_id":"request", "quote_id":"quote", "revision":1, "character_id":null, "sandbox":true,
            "state":"awaiting_payment", "checkout_url":"https://checkout.example/order",
            "price":{"currency":"USD","minor_units":100,"exponent":2},
            "code":"awaiting_payment", "receipt_id":null, "review_required":false
        }))?);
        Ok::<_, serde_json::Error>((config.coin_shop)(&view, CancellationToken::new()))
    });
    wait(&mut app, None)?;
    click_named(&mut app, "coin.checkout")?;
    assert!(matches!(
        worker.join().map_err(|_| "coin presenter")??,
        Action::OpenCheckout
    ));
    Ok(())
}

#[test]
fn get_coins_reopens_and_inspects_an_awaiting_payment_order_by_request_id() -> TestResult {
    use crate::account_services::test_server::coins::{EventKind, PACKAGE};
    use std::sync::{Arc, Mutex};

    test_thread(|| {
        let mock = Arc::new(MockPlatform::start()?);
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let authority = mock.authority.clone();
        authority
            .lock()
            .map_err(|_| "authority")?
            .coins
            .as_mut()
            .ok_or("coin authority")?
            .fail_after_create = true;
        let (mut app, mut config) = commerce_app()?;
        let opens = Arc::new(Mutex::new(Vec::new()));
        let browser = opens.clone();
        config.open_browser = Arc::new(move |url| {
            if let Ok(mut opened) = browser.lock() {
                opened.push(url.to_owned());
            }
            true
        });
        let runner = Arc::clone(&mock);
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            runner
                .runtime
                .block_on(crate::account_services::run(
                    &client,
                    &token,
                    &config,
                    &mut Default::default(),
                    CancellationToken::new(),
                ))
                .map_err(|e| format!("{e:?}"))
        })?;

        let menu = wait(&mut app, None)?;
        click_named(&mut app, "account.coins")?;
        let packages = wait(&mut app, Some(menu))?;
        click_named(&mut app, &format!("coin.package.{PACKAGE}"))?;
        click_named(&mut app, "commerce.confirm")?;
        let pending = wait(&mut app, Some(packages))?;
        click_named(&mut app, "coin.retry")?;
        let checkout = wait(&mut app, Some(pending))?;
        let order = authority
            .lock()
            .map_err(|_| "authority")?
            .coins
            .as_ref()
            .and_then(|coin| coin.orders.values().next())
            .map(|(_, order)| order.clone())
            .ok_or("order")?;
        assert_ne!(order.order_id, order.request_id);
        click(&mut app, Action::Back)?;
        let menu = wait(&mut app, Some(checkout))?;
        click_named(&mut app, "account.coins")?;
        let recent = wait(&mut app, Some(menu))?;
        click_named(&mut app, &format!("coin.order.{}", order.order_id))?;
        let inspected = wait(&mut app, Some(recent))?;
        click_named(&mut app, "coin.checkout")?;
        let reopened = wait(&mut app, Some(inspected))?;
        let (body, signature) = mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(mock.webhook(&body, &signature)?, 200);
        click_named(&mut app, "coin.status")?;
        let paid = wait(&mut app, Some(reopened))?;
        let status = &app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .single(app.world())?
            .0;
        assert!(status.contains("Paid, 100 coins"), "{status}");
        assert!(status.contains("Balance: 1100 coins."), "{status}");
        click(&mut app, Action::Back)?;
        wait(&mut app, Some(paid))?;
        let status = &app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .single(app.world())?
            .0;
        assert!(
            status.contains("Last refreshed balance: 1100 Oteryn coins"),
            "{status}"
        );
        click(&mut app, Action::Back)?;
        worker.join().map_err(|_| "account services worker")??;

        let state = authority.lock().map_err(|_| "authority")?;
        assert!(
            state
                .requests
                .iter()
                .any(|request| request.starts_with(&format!(
                    "GET /api/v1/native-account/coin-purchases/requests/{} ",
                    order.request_id
                )))
        );
        let coins = state.coins.as_ref().ok_or("coin authority")?;
        assert_eq!(coins.creations, 1);
        assert_eq!(coins.credits, 1);
        assert_eq!(opens.lock().map_err(|_| "browser")?.len(), 2);
        Ok(())
    })
}

#[test]
fn bazaar_page_keeps_auction_details_during_bid_retry_and_status_recovery() -> TestResult {
    use crate::account_services::test_server::{AUCTION, CHARACTER, DESTINATION};
    use oteryn_platform_client::native_login::bazaar::BazaarCommand;

    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let authority = mock.authority.clone();
        {
            let mut state = authority.lock().map_err(|_| "authority")?;
            state.paginate = true;
            state.pending_first = true;
        }
        let (mut app, config) = commerce_app()?;
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            mock.runtime
                .block_on(crate::account_services::run(
                    &client,
                    &token,
                    &config,
                    &mut Default::default(),
                    CancellationToken::new(),
                ))
                .map_err(|e| format!("{e:?}"))
        })?;

        let menu = wait(&mut app, None)?;
        click_named(&mut app, "account.bazaar.all")?;
        let browse = wait(&mut app, Some(menu))?;
        click_named(&mut app, "account.bazaar.next")?;
        let page = wait(&mut app, Some(browse))?;
        click_named(&mut app, &format!("account.auction.{AUCTION}"))?;
        click_named(&mut app, "commerce.confirm")?;
        let pending = wait(&mut app, Some(page))?;
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| {
                    text.0.contains("Next character")
                        && text.0.contains("highest bid 0")
                        && text.0.contains("2000000000 UTC")
                }),
            "keep the auction summary visible while a bid is pending"
        );
        let id = authority
            .lock()
            .map_err(|_| "authority")?
            .commands
            .keys()
            .next()
            .cloned()
            .ok_or("bid request")?;
        assert!(
            app.world_mut()
                .query_filtered::<&Text, With<Body>>()
                .single(app.world())?
                .0
                .contains(&id)
        );
        click_named(&mut app, "account.retry")?;
        let retried = wait(&mut app, Some(pending))?;
        click_named(&mut app, "account.status")?;
        let resolved = wait(&mut app, Some(retried))?;
        {
            let mut state = authority.lock().map_err(|_| "authority")?;
            let (bid, _) = state.commands.get(&id).ok_or("same bid")?;
            assert!(
                matches!(&bid.command, BazaarCommand::Bid { auction_id, coins: 10, .. } if auction_id == AUCTION)
            );
            assert_eq!(state.commands.len(), 1);
            assert_eq!(state.effects, 1);
            let bid_submissions = state.requests.iter().filter(|request| {
                request.starts_with("POST /api/v1/native-account/bazaar/commands ")
            });
            assert_eq!(
                bid_submissions
                    .filter(|request| request.contains(&id))
                    .count(),
                2
            );
            assert!(state.requests.iter().any(|request| {
                request.starts_with(&format!("GET /api/v1/native-account/bazaar/commands/{id} "))
            }));
            state.pending_first = false;
        }
        click_named(&mut app, &format!("account.receipt.{id}"))?;
        let receipt = wait(&mut app, Some(resolved))?;
        click_named(&mut app, &format!("account.character.{CHARACTER}"))?;
        click_named(&mut app, "account.sell")?;
        click_named(&mut app, "commerce.confirm")?;
        let listed = wait(&mut app, Some(receipt))?;
        click_named(&mut app, &format!("account.character.{CHARACTER}"))?;
        click_named(&mut app, &format!("account.transfer.{DESTINATION}"))?;
        let quote = wait(&mut app, Some(listed))?;
        click_named(&mut app, "account.transfer.confirm")?;
        click_named(&mut app, "commerce.confirm")?;
        wait(&mut app, Some(quote))?;
        click(&mut app, Action::Back)?;
        worker.join().map_err(|_| "account services worker")??;

        let state = authority.lock().map_err(|_| "authority")?;
        assert_eq!(state.effects, 3);
        assert!(state.commands.values().any(|(request, _)| matches!(
            &request.command,
            BazaarCommand::Sell { character_id, minimum_bid: 1, duration_hours: 24 } if character_id == CHARACTER
        )));
        assert!(
            state
                .commands
                .values()
                .any(|(request, _)| matches!(&request.command, BazaarCommand::Transfer { .. }))
        );
        Ok(())
    })
}

#[test]
fn commerce_uses_a_character_selected_in_the_same_frame_as_the_action() -> TestResult {
    use crate::account_services::{Action, View};
    use oteryn_platform_client::native_login::bazaar::{
        AuctionView, BazaarPage, OwnedCharacter, TransferWorld,
    };

    for listing in [false, true] {
        let (mut app, config) = commerce_app()?;
        let mut view = View::default();
        view.page = Some(BazaarPage {
            version: 1,
            view: AuctionView::Own,
            auctions: vec![],
            balance: 10,
            owned_characters: ["Alice", "Bob"]
                .map(|name| OwnedCharacter {
                    id: name.into(),
                    name: name.into(),
                    world_id: "home".into(),
                })
                .into(),
            worlds: vec![TransferWorld {
                id: "other".into(),
                name: "Other".into(),
            }],
            next_cursor: None,
            recent_operations: vec![],
        });
        let worker =
            thread::spawn(move || (config.account_services)(&view, CancellationToken::new()));
        wait(&mut app, None)?;
        click_named(&mut app, "account.character.Alice")?;
        for name in [
            "account.character.Bob",
            if listing {
                "account.sell"
            } else {
                "account.transfer.other"
            },
        ] {
            let world = app.world_mut();
            let entity = world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
                .ok_or("commerce control")?;
            world.trigger(Activate { entity });
        }
        app.update();
        if listing {
            click_named(&mut app, "commerce.confirm")?;
        }
        let action = worker.join().map_err(|_| "commerce presenter")?;
        if listing {
            assert!(matches!(action, Action::Sell { character_id, .. } if character_id == "Bob"));
        } else {
            assert!(matches!(action, Action::Quote { character_id, .. } if character_id == "Bob"));
        }
    }
    Ok(())
}

#[test]
fn commerce_cancel_invalidates_queued_confirmation() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let (mut app, config) = commerce_app()?;
        let cancellation = CancellationToken::new();
        let worker_token = cancellation.clone();
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            mock.runtime
                .block_on(crate::account_services::run(
                    &client,
                    &token,
                    &config,
                    &mut Default::default(),
                    worker_token,
                ))
                .map_err(|err| format!("{err:?}"))
        })?;
        let result = (|| -> TestResult {
            let menu = wait(&mut app, None)?;
            click_named(&mut app, "account.coins")?;
            let packages = wait(&mut app, Some(menu))?;
            let prepare = format!(
                "coin.package.{}",
                crate::account_services::test_server::coins::PACKAGE
            );
            click_named(&mut app, &prepare)?;
            let mut entities = Vec::new();
            for name in ["commerce.cancel", &prepare, "commerce.confirm"] {
                let world = app.world_mut();
                entities.push(
                    world
                        .query::<(Entity, &Name)>()
                        .iter(world)
                        .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
                        .ok_or("commerce button")?,
                );
            }
            for entity in entities {
                app.world_mut().trigger(Activate { entity });
            }
            app.update();
            assert_eq!(
                app.world()
                    .non_send::<HostOwner>()
                    .services
                    .pending()
                    .map(|r| r.id),
                Some(packages)
            );
            Ok(())
        })();
        cancellation.cancel();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !worker.is_finished() {
            if Instant::now() >= deadline {
                return Err("account presenter shutdown timed out".into());
            }
            thread::yield_now();
        }
        let _ = worker.join().map_err(|_| "account presenter")?;
        result
    })
}

#[test]
fn unavailable_coin_shop_offers_back_without_refresh() -> TestResult {
    let (mut app, config) = commerce_app()?;
    let worker = thread::spawn(move || {
        let mut view = crate::coin_shop::View::new(None);
        view.unavailable = true;
        (config.coin_shop)(&view, CancellationToken::new())
    });
    wait(&mut app, None)?;
    assert!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .all(|name| name.as_str() != "coin.refresh")
    );
    click(&mut app, Action::Back)?;
    assert!(matches!(
        worker.join().map_err(|_| "coin presenter")?,
        crate::coin_shop::Action::Back
    ));
    Ok(())
}

fn with_coin_offer(
    ambiguous: bool,
    check: impl FnOnce(&mut App, u64, &MockPlatform) -> TestResult + Send + 'static,
) -> TestResult {
    use crate::account_services::test_server::coins::PACKAGE;
    test_thread(move || {
        let mock = std::sync::Arc::new(MockPlatform::start()?);
        let client = mock.client()?;
        let token = mock.token(&client)?;
        if ambiguous {
            mock.with_coins(|coins| {
                coins.drop_after_create = true;
                coins.hide_recent_orders = true;
            })?;
        }
        let (mut app, config) = commerce_app()?;
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let runner = mock.clone();
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            runner.runtime.block_on(crate::account_services::run(
                &client,
                &token,
                &config,
                &mut Default::default(),
                worker_cancel,
            ))
        })?;
        let result = (|| {
            let account = wait(&mut app, None)?;
            click_named(&mut app, "account.coins")?;
            let shop = wait(&mut app, Some(account))?;
            click_named(&mut app, &format!("coin.package.{PACKAGE}"))?;
            check(&mut app, shop, &mock)
        })();
        cancel.cancel();
        worker.join().map_err(|_| "account presenter")??;
        result
    })
}

#[test]
fn ambiguous_coin_order_keeps_identity_when_shop_reopens() -> TestResult {
    with_coin_offer(true, |app, shop, mock| {
        click_named(app, "commerce.confirm")?;
        let pending = wait(app, Some(shop))?;
        let id = mock
            .with_coins(|coins| coins.orders.keys().next().cloned())?
            .ok_or("order")?;
        click(app, Action::Back)?;
        let account = wait(app, Some(pending))?;
        click_named(app, "account.coins")?;
        let reopened = wait(app, Some(account))?;
        let world = app.world_mut();
        let body = world.query_filtered::<&Text, With<Body>>().single(world)?;
        assert!(
            body.0.contains(&id),
            "reopened shop lost unresolved request {id}"
        );
        click_named(app, "coin.status")?;
        wait(app, Some(reopened))?;
        let state = mock.authority.lock().map_err(|_| "lock")?;
        let target = format!("GET /api/v1/native-account/coin-purchases/requests/{id} ");
        assert!(state.requests.iter().any(|r| r.starts_with(&target)));
        assert_eq!(state.coins.as_ref().ok_or("coins")?.creations, 1);
        Ok(())
    })
}

#[test]
fn rejected_expired_order_can_refresh_and_confirm_again() -> TestResult {
    use crate::account_services::test_server::coins::PACKAGE;
    with_coin_offer(false, |app, shop, mock| {
        mock.with_coins(|coins| coins.expire_quotes())?;
        click_named(app, "commerce.confirm")?;
        let pending = wait(app, Some(shop))?;
        click_named(app, "coin.status")?;
        let resolved = wait(app, Some(pending))?;
        let world = app.world_mut();
        let body = world.query_filtered::<&Text, With<Body>>().single(world)?;
        assert!(
            !body.0.contains("Unresolved order request"),
            "a rejected POST and missing status have no effect"
        );
        click_named(app, "coin.refresh")?;
        let refreshed = wait(app, Some(resolved))?;
        click_named(app, &format!("coin.package.{PACKAGE}"))?;
        click_named(app, "commerce.confirm")?;
        wait(app, Some(refreshed))?;
        assert_eq!(mock.with_coins(|coins| coins.creations)?, 1);
        Ok(())
    })
}

#[test]
fn expired_coin_offer_disables_review_and_closes_confirmation() -> TestResult {
    with_coin_offer(false, |app, _, _| {
        use crate::account_services::test_server::coins::PACKAGE;
        let offer = format!("coin.package.{PACKAGE}");
        let controls = |world: &mut World| {
            let mut names = world.query_filtered::<&Name, Without<bevy::ui::InteractionDisabled>>();
            let mut present = |wanted| names.iter(world).any(|name| name.as_str() == wanted);
            (present(offer.as_str()), present("commerce.confirm"))
        };
        assert_eq!(controls(app.world_mut()), (true, true));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualInstant(
            Instant::now() + Duration::from_secs(121),
        ));
        app.update();
        assert_eq!(
            controls(app.world_mut()),
            (false, false),
            "expired offer and confirmation must be disabled"
        );
        Ok(())
    })
}

#[test]
fn expired_transfer_offer_disables_review_and_closes_confirmation() -> TestResult {
    use crate::account_services::test_server::{CHARACTER, DESTINATION};
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let (mut app, config) = commerce_app()?;
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            mock.runtime.block_on(crate::account_services::run(
                &client,
                &token,
                &config,
                &mut Default::default(),
                worker_cancel,
            ))
        })?;
        let result = (|| {
            let account = wait(&mut app, None)?;
            click_named(&mut app, &format!("account.character.{CHARACTER}"))?;
            click_named(&mut app, &format!("account.transfer.{DESTINATION}"))?;
            wait(&mut app, Some(account))?;
            click_named(&mut app, "account.transfer.confirm")?;
            let controls = |world: &mut World| {
                let mut names =
                    world.query_filtered::<&Name, Without<bevy::ui::InteractionDisabled>>();
                let mut active = |wanted| names.iter(world).any(|name| name.as_str() == wanted);
                (
                    active("account.transfer.confirm"),
                    active("commerce.confirm"),
                )
            };
            assert_eq!(controls(app.world_mut()), (true, true));
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualInstant(
                Instant::now() + Duration::from_secs(121),
            ));
            app.update();
            assert_eq!(controls(app.world_mut()), (false, false));
            Ok(())
        })();
        cancel.cancel();
        worker.join().map_err(|_| "account presenter")??;
        result
    })
}
