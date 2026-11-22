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
        edit_commerce_field(&mut app, "", "1000000000000")?;
        click_named(&mut app, &format!("account.auction.{AUCTION}"))?;
        assert_no_confirmation(&mut app);
        edit_commerce_field(&mut app, "1000000000000", "invalid")?;
        click_named(&mut app, &format!("account.auction.{AUCTION}"))?;
        assert_no_confirmation(&mut app);
        edit_commerce_field(&mut app, "invalid", "17")?;
        click_named(&mut app, &format!("account.auction.{AUCTION}"))?;
        cancel_confirmation(&mut app, page)?;
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
                matches!(&bid.command, BazaarCommand::Bid { auction_id, coins: 17, .. } if auction_id == AUCTION)
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
        edit_commerce_field(&mut app, "1", "1000000000000")?;
        click_named(&mut app, "account.sell")?;
        assert_no_confirmation(&mut app);
        edit_commerce_field(&mut app, "1000000000000", "invalid")?;
        click_named(&mut app, "account.sell")?;
        assert_no_confirmation(&mut app);
        edit_commerce_field(&mut app, "invalid", "19")?;
        edit_commerce_field(&mut app, "24", "48")?;
        click_named(&mut app, "account.sell")?;
        cancel_confirmation(&mut app, receipt)?;
        click_named(&mut app, "account.sell")?;
        click_named(&mut app, "commerce.confirm")?;
        let listed = wait(&mut app, Some(receipt))?;
        click_named(&mut app, &format!("account.character.{CHARACTER}"))?;
        click_named(&mut app, &format!("account.transfer.{DESTINATION}"))?;
        let quote = wait(&mut app, Some(listed))?;
        click_named(&mut app, "account.transfer.confirm")?;
        cancel_confirmation(&mut app, quote)?;
        click_named(&mut app, "account.transfer.confirm")?;
        click_named(&mut app, "commerce.confirm")?;
        wait(&mut app, Some(quote))?;
        click(&mut app, Action::Back)?;
        finish_worker(worker, "return from Bazaar")??;

        let state = authority.lock().map_err(|_| "authority")?;
        assert_eq!(state.effects, 3);
        assert!(state.commands.values().any(|(request, _)| matches!(
            &request.command,
            BazaarCommand::Sell { character_id, minimum_bid: 19, duration_hours: 48 } if character_id == CHARACTER
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
        let action = finish_worker(
            worker,
            if listing {
                "confirm listing"
            } else {
                "request transfer quote"
            },
        )?;
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
        let _ = finish_worker(worker, "cancel confirmation")?;
        result
    })
}

#[test]
fn expired_transfer_quote_disables_review_and_prepared_confirmation() -> TestResult {
    let (mut app, config) = commerce_app()?;
    let worker = thread::spawn(move || {
        (config.account_services)(
            &crate::account_services::View::default(),
            CancellationToken::new(),
        )
    });
    let request = wait(&mut app, None)?;
    super::commerce::assert_expired_transfer_controls(&mut app, request)?;
    click(&mut app, Action::Back)?;
    assert!(matches!(
        finish_worker(worker, "close expired quote")?,
        crate::account_services::Action::Back
    ));
    Ok(())
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
