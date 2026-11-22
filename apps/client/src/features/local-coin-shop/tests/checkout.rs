use super::*;

#[test]
fn coin_checkout_never_credits_before_signed_bound_event_and_replays_once() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let opened = Arc::new(Mutex::new(Vec::new()));
        let record = opened.clone();
        let browser: Browser = Arc::new(move |url| {
            if let Ok(mut log) = record.lock() {
                log.push(url.to_owned());
            }
            true
        });
        let mut view = View::new(Some(CHARACTER.into()));
        mock.runtime.block_on(async {
            controller::execute(
                &mut view,
                Action::Refresh,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            controller::execute(
                &mut view,
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: true,
                },
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
        });
        assert_eq!(opened.lock().map_err(|_| "browser lock")?.len(), 1);
        let order = view.order.clone().ok_or("order")?;
        assert_eq!(order.state, PurchaseState::AwaitingPayment);
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1000));
        assert!(!format!("{order:?}").contains(&mock.base));
        let (bad_body, signature) = mock.signed_event(&order, EventKind::Paid)?;
        let mut bad: Event = serde_json::from_slice(&bad_body)?;
        bad.price.minor_units += 1;
        let changed = serde_json::to_vec(&bad)?;
        assert_eq!(mock.webhook(&changed, &signature)?, 400);
        let binding_sig = mock
            .authority
            .lock()
            .map_err(|_| "lock")?
            .coins
            .as_ref()
            .ok_or("provider")?
            .provider
            .sign(&changed);
        assert_eq!(mock.webhook(&changed, &binding_sig)?, 400);
        let (body, signature) = mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(mock.webhook(&body, &signature)?, 200);
        assert_eq!(mock.webhook(&body, &signature)?, 200);
        let mut replay: Event = serde_json::from_slice(&body)?;
        replay.price.minor_units += 1;
        let changed = serde_json::to_vec(&replay)?;
        let changed_signature = mock
            .authority
            .lock()
            .map_err(|_| "lock")?
            .coins
            .as_ref()
            .ok_or("provider")?
            .provider
            .sign(&changed);
        assert_eq!(mock.webhook(&changed, &changed_signature)?, 400);
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Status,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        assert_eq!(
            view.order.as_ref().map(|o| o.state),
            Some(PurchaseState::Paid)
        );
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1100));
        assert!(view.status_text().contains("Receipt"));
        let state = mock.authority.lock().map_err(|_| "lock")?;
        let coins = state.coins.as_ref().ok_or("provider")?;
        assert_eq!(coins.credits, 1);
        assert_eq!(coins.creations, 1);
        Ok(())
    })
}

#[test]
fn coin_order_ambiguous_retry_preserves_quote_identity_and_browser_retry_creates_no_order()
-> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let opened = Arc::new(Mutex::new(0));
        let attempts = opened.clone();
        let browser: Browser = Arc::new(move |_| {
            if let Ok(mut n) = attempts.lock() {
                *n += 1;
            }
            false
        });
        let mut view = View::new(None);
        mock.runtime.block_on(async {
            controller::execute(
                &mut view,
                Action::Refresh,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            controller::execute(
                &mut view,
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: false,
                },
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            assert!(view.order.is_none());
            mock.authority
                .lock()
                .map_err(|_| "lock")?
                .coins
                .as_mut()
                .ok_or("provider")?
                .fail_after_create = true;
            controller::execute(
                &mut view,
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: true,
                },
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            assert!(view.pending());
            let request = view.pending.clone().ok_or("request")?;
            controller::execute(
                &mut view,
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: true,
                },
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(view.pending.as_ref(), Some(&request));
            controller::execute(
                &mut view,
                Action::Retry,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            assert!(!view.pending());
            assert!(view.feedback.contains("Could not open"));
            assert_eq!(
                view.order.as_ref().map(|o| &o.request_id),
                Some(&request.request_id)
            );
            controller::execute(
                &mut view,
                Action::OpenCheckout,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            let mut changed = request.clone();
            changed.package_id = CHARACTER.into();
            assert!(
                client
                    .create_coin_order(&token, &changed, CancellationToken::new())
                    .await
                    .is_err()
            );
            let state = mock.authority.lock().map_err(|_| "lock")?;
            let coins = state.coins.as_ref().ok_or("provider")?;
            assert_eq!(coins.creations, 1);
            assert_eq!(coins.credits, 0);
            let posts: Vec<_> = state
                .requests
                .iter()
                .filter(|r| r.starts_with("POST /api/v1/native-account/coin-purchases/orders "))
                .collect();
            assert_eq!(posts.len(), 3);
            assert_eq!(
                posts[0].split_once("\r\n\r\n").map(|(_, b)| b),
                posts[1].split_once("\r\n\r\n").map(|(_, b)| b)
            );
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
        assert_eq!(*opened.lock().map_err(|_| "browser lock")?, 2);
        Ok(())
    })
}

#[test]
fn refreshed_tracked_order_reconciles_paid_and_validates_immutable_binding() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let browser: Browser = Arc::new(|_| true);
        let mut view = View::new(None);
        mock.runtime.block_on(async {
            controller::execute(
                &mut view,
                Action::Refresh,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            controller::execute(
                &mut view,
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: true,
                },
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
        });
        let order = view.order.clone().ok_or("order")?;
        assert!(view.can_checkout());
        let (body, signature) = mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(mock.webhook(&body, &signature)?, 200);
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        let paid = view.order.clone().ok_or("paid order")?;
        assert_eq!(paid.state, PurchaseState::Paid);
        assert!(!view.can_checkout());
        {
            let mut state = mock.authority.lock().map_err(|_| "lock")?;
            let stored = &mut state
                .coins
                .as_mut()
                .ok_or("coins")?
                .orders
                .get_mut(&order.request_id)
                .ok_or("stored order")?
                .1;
            stored.coins += 1;
            stored.revision += 1;
        }
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        assert_eq!(view.order, Some(paid));
        assert!(view.feedback.contains("Coin service unavailable"));
        Ok(())
    })
}

#[test]
fn overlapping_purchases_send_one_post_until_response() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let other = mock.token(&client)?;
        let browser: Browser = Arc::new(|_| true);
        let cancel = CancellationToken::new();
        let mut first = View::new(Some(CHARACTER.into()));
        let purchase = Action::Purchase {
            package_id: PACKAGE.into(),
            confirmed: true,
        };
        let (ready, received) = tokio::sync::oneshot::channel();
        let (release, held) = tokio::sync::oneshot::channel();
        mock.authority
            .lock()
            .map_err(|_| "lock")?
            .hold_coin_response = Some((ready, held));
        mock.runtime.block_on(async {
            controller::execute(
                &mut first,
                Action::Refresh,
                &client,
                &token,
                &browser,
                cancel.clone(),
            )
            .await;
            let mut second = first.clone();
            let submit = controller::execute(
                &mut first,
                purchase.clone(),
                &client,
                &token,
                &browser,
                cancel.clone(),
            );
            let overlap = async {
                received.await?;
                controller::execute(
                    &mut second,
                    purchase.clone(),
                    &client,
                    &other,
                    &browser,
                    cancel.clone(),
                )
                .await;
                release.send(()).map_err(|_| "response")?;
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            let (_, result) = tokio::join!(submit, overlap);
            result?;
            assert!(second.can_purchase(PACKAGE));
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
        let authority = mock.authority.lock().map_err(|_| "lock")?;
        assert_eq!(
            authority
                .requests
                .iter()
                .filter(|r| r.starts_with("POST /api/v1/native-account/coin-purchases/orders "))
                .count(),
            1
        );
        Ok(())
    })
}

#[test]
fn fresh_grant_filters_previously_accepted_order() -> TestResult {
    checkout_refresh_case(true)
}

#[test]
fn earlier_order_cannot_replace_just_purchased_checkout() -> TestResult {
    checkout_refresh_case(false)
}

#[test]
fn coin_status_recovers_request_outside_recent_history() -> TestResult {
    test_thread(|| {
        use crate::account_services::test_server::coins::CoinAuthority;

        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let next = mock.token(&client)?;
        let other = mock.token(&client)?;
        mock.authority
            .lock()
            .map_err(|_| "lock")?
            .coin_wallets_by_grant
            .insert(3, CoinAuthority::new(mock.base.clone())?);
        let browser: Browser = Arc::new(|_| false);
        let execute = |view: &mut View, action, grant| {
            mock.runtime.block_on(controller::execute(
                view,
                action,
                &client,
                grant,
                &browser,
                CancellationToken::new(),
            ));
        };
        let mut first = View::new(None);
        execute(&mut first, Action::Refresh, &token);
        mock.with_coins(|coins| {
            coins.drop_after_create = true;
            coins.hide_recent_orders = true;
        })?;
        execute(
            &mut first,
            Action::Purchase {
                package_id: PACKAGE.into(),
                confirmed: true,
            },
            &token,
        );
        let request_id = first
            .pending
            .as_ref()
            .ok_or("pending request")?
            .request_id
            .clone();
        let mut other_account = View::with_prior(None, first.prior.clone());
        execute(&mut other_account, Action::Status, &other);
        assert!(other_account.order.is_none());
        assert!(!format!("{other_account:?}").contains(&request_id));

        let mut reopened = View::with_prior(None, first.prior);
        execute(&mut reopened, Action::Refresh, &next);
        assert!(reopened.order.is_none());
        assert!(!reopened.can_retry());
        execute(&mut reopened, Action::Status, &next);
        let order = reopened.order.ok_or("status failed to recover the order")?;
        assert_eq!(order.request_id, request_id);
        assert_eq!(mock.with_coins(|coins| coins.creations)?, 1);
        Ok(())
    })
}

fn checkout_refresh_case(deny_lookup: bool) -> TestResult {
    test_thread(move || {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let next = mock.token(&client)?;
        let browser: Browser = Arc::new(|_| true);
        let execute = |view: &mut View, action, grant| {
            mock.runtime.block_on(controller::execute(
                view,
                action,
                &client,
                grant,
                &browser,
                CancellationToken::new(),
            ));
        };
        let purchase = || Action::Purchase {
            package_id: PACKAGE.into(),
            confirmed: true,
        };
        let mut first = View::new(None);
        execute(&mut first, Action::Refresh, &token);
        mock.with_coins(|c| {
            c.drop_after_create = !deny_lookup;
            c.hide_recent_orders = !deny_lookup;
        })?;
        execute(&mut first, purchase(), &token);
        let original = mock
            .with_coins(|c| c.orders.keys().next().cloned())?
            .ok_or("order")?;
        let mut fresh = View::with_prior(None, first.prior.clone());
        if deny_lookup {
            let listed = mock
                .with_coins(|c| c.orders.remove(&original))?
                .ok_or("listed order")?;
            mock.with_coins(|c| c.orders.insert("stale-index".into(), listed))?;
            execute(&mut fresh, Action::Refresh, &next);
            assert!(!format!("{fresh:?}").contains(&original));
            assert!(fresh.can_purchase(PACKAGE));
        } else {
            execute(&mut fresh, Action::Refresh, &next);
            mock.with_coins(|c| {
                c.hide_recent_orders = false;
                c.drop_after_create = false;
            })?;
            execute(&mut fresh, purchase(), &next);
            assert_eq!(mock.with_coins(|c| c.orders.len())?, 2);
            assert_ne!(
                fresh.order.as_ref().ok_or("active order")?.request_id,
                original
            );
        }
        Ok(())
    })
}
