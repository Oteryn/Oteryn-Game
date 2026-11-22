use super::*;

#[test]
fn coin_shop_refuses_foreign_checkout_expired_quote_and_wrong_character() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let opened = Arc::new(Mutex::new(0));
        let calls = opened.clone();
        let browser: Browser = Arc::new(move |_| {
            if let Ok(mut n) = calls.lock() {
                *n += 1;
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
            view.quote_until
                .values_mut()
                .for_each(|until| *until = Instant::now());
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
            assert!(view.order.is_none());
            assert!(!view.pending());
            controller::execute(
                &mut view,
                Action::Refresh,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            )
            .await;
            mock.authority
                .lock()
                .map_err(|_| "lock")?
                .coins
                .as_mut()
                .ok_or("provider")?
                .tamper_checkout = Some("https://untrusted.invalid/checkout".into());
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
            assert!(view.order.is_none());
            assert!(
                client
                    .coin_catalogue(&token, Some(PACKAGE), CancellationToken::new())
                    .await
                    .is_err()
            );
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
        assert_eq!(*opened.lock().map_err(|_| "browser")?, 0);
        Ok(())
    })
}

#[test]
fn coin_shop_money_format_is_integer_and_wallet_snapshot_cannot_regress() -> TestResult {
    assert_eq!(
        price_text(&Price {
            minor_units: 5,
            currency: "USD".into(),
            exponent: 2
        }),
        "USD 0.05"
    );
    assert_eq!(
        price_text(&Price {
            minor_units: 123,
            currency: "JPY".into(),
            exponent: 0
        }),
        "JPY 123"
    );
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let browser: Browser = Arc::new(|_| true);
        let update = |view: &mut View| {
            mock.runtime.block_on(controller::execute(
                view,
                Action::Refresh,
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            ));
        };
        let mut view = View::new(None);
        update(&mut view);
        assert!(view.can_purchase(PACKAGE));
        mock.with_coins(|coins| coins.fail_balance = true)?;
        update(&mut view);
        assert!(
            !view.can_purchase(PACKAGE),
            "quotes require a fresh validated wallet"
        );
        let original = view.balance.clone();
        mock.with_coins(|coins| {
            coins.fail_balance = false;
            coins.balance.total_coins = 999;
            coins.balance.available_coins = 999;
            coins.balance.transferable_coins = 999;
        })?;
        update(&mut view);
        assert_eq!(view.balance, original);
        assert!(view.feedback.contains("unavailable"));
        Ok(())
    })
}

#[test]
fn coin_post_refusals_settle_after_missing_status() -> TestResult {
    test_thread(|| {
        let mut settled = Vec::new();
        for refusal in [409, 403] {
            let mock = MockPlatform::start()?;
            let client = mock.client()?;
            let token = mock.token(&client)?;
            let browser: Browser = Arc::new(|_| false);
            let mut view = View::new(None);
            let execute = |view: &mut View, action| {
                mock.runtime.block_on(controller::execute(
                    view,
                    action,
                    &client,
                    &token,
                    &browser,
                    CancellationToken::new(),
                ))
            };
            execute(&mut view, Action::Refresh);
            mock.authority.lock().map_err(|_| "lock")?.coin_post_failure =
                Some(if refusal == 409 { 0 } else { refusal });
            execute(
                &mut view,
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: true,
                },
            );
            if refusal == 409 {
                let request = view.pending.clone().ok_or("ambiguous purchase")?;
                mock.authority.lock().map_err(|_| "lock")?.coin_post_failure = Some(refusal);
                execute(&mut view, Action::Retry);
                assert_eq!(view.pending.as_ref(), Some(&request));
                execute(&mut view, Action::Status);
            }
            settled.push(!view.pending());
            assert_eq!(mock.with_coins(|coins| coins.creations)?, 0);
        }
        assert_eq!(
            settled,
            [true, true],
            "coin retry 409 and pre-create 403 must both settle"
        );
        Ok(())
    })
}

#[test]
fn coin_reopen_preserves_wallet_and_order_revision_floors() -> TestResult {
    test_thread(|| {
        for stale_wallet in [true, false] {
            let mock = MockPlatform::start()?;
            let client = mock.client()?;
            let token = mock.token(&client)?;
            let browser: Browser = Arc::new(|_| false);
            let mut view = View::new(None);
            let show = |view: &mut View, presenter: &Presenter| {
                mock.runtime.block_on(run(
                    &client,
                    &token,
                    presenter,
                    &browser,
                    view,
                    CancellationToken::new(),
                ))
            };
            let first = std::sync::atomic::AtomicBool::new(true);
            let purchase: Presenter = Arc::new(move |_, _| {
                if first.swap(false, std::sync::atomic::Ordering::Relaxed) {
                    Action::Purchase {
                        package_id: PACKAGE.into(),
                        confirmed: true,
                    }
                } else {
                    Action::Back
                }
            });
            show(&mut view, &purchase)?;
            let back: Presenter = Arc::new(|_, _| Action::Back);
            // Learn both revision floors before the later shop reopen.
            mock.with_coins(|coins| {
                coins.balance.revision = 42;
                coins.balance.review_required = true;
                for (_, order) in coins.orders.values_mut() {
                    order.revision = 42;
                }
            })?;
            show(&mut view, &back)?;
            mock.with_coins(|coins| {
                if stale_wallet {
                    coins.balance.revision = 41;
                    coins.balance.review_required = false;
                } else {
                    for (_, order) in coins.orders.values_mut() {
                        order.revision = 41;
                    }
                }
            })?;
            show(&mut view, &back)?;
            assert_eq!(
                view.balance.as_ref().map(|w| w.revision),
                Some(42),
                "wallet floor"
            );
            assert_eq!(
                view.order.as_ref().map(|o| o.revision),
                Some(42),
                "order floor"
            );
            assert!(!view.can_purchase(PACKAGE));
        }
        Ok(())
    })
}
