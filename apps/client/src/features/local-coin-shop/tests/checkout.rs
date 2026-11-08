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
