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
        let mut view = View::new(None);
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        let original = view.balance.clone();
        {
            let mut authority = mock.authority.lock().map_err(|_| "lock")?;
            let coins = authority.coins.as_mut().ok_or("provider")?;
            coins.balance.total_coins = 999;
            coins.balance.available_coins = 999;
            coins.balance.transferable_coins = 999;
        }
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        assert_eq!(view.balance, original);
        assert!(view.feedback.contains("unavailable"));
        Ok(())
    })
}
