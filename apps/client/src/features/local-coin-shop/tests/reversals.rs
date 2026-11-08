use super::*;

#[test]
fn coin_refund_reverses_once_and_late_paid_event_cannot_mint_again() -> TestResult {
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
        let (paid, sig) = mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(mock.webhook(&paid, &sig)?, 200);
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Status,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1100));
        let (refund, sig) = mock.signed_event(&order, EventKind::Refund)?;
        assert_eq!(mock.webhook(&refund, &sig)?, 200);
        assert_eq!(mock.webhook(&refund, &sig)?, 200);
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
            Some(PurchaseState::Refunded)
        );
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1000));
        let (late, sig) = mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(mock.webhook(&late, &sig)?, 200);
        let (chargeback, sig) = mock.signed_event(&order, EventKind::Chargeback)?;
        assert_eq!(mock.webhook(&chargeback, &sig)?, 200);
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
            Some(PurchaseState::Chargeback)
        );
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1000));
        let state = mock.authority.lock().map_err(|_| "lock")?;
        let coins = state.coins.as_ref().ok_or("provider")?;
        assert_eq!(coins.credits, 1);
        assert_eq!(coins.reversals, 1);
        Ok(())
    })
}

#[test]
fn coin_chargeback_after_spend_flags_review_without_negative_balance() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let browser: Browser = Arc::new(|_| true);
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
        let order = view.order.clone().ok_or("order")?;
        let (paid, sig) = mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(mock.webhook(&paid, &sig)?, 200);
        {
            let mut state = mock.authority.lock().map_err(|_| "lock")?;
            let coins = state.coins.as_mut().ok_or("provider")?;
            coins.balance.total_coins = 0;
            coins.balance.available_coins = 0;
            coins.balance.transferable_coins = 0;
            coins.balance.revision += 1;
        }
        let (chargeback, sig) = mock.signed_event(&order, EventKind::Chargeback)?;
        assert_eq!(mock.webhook(&chargeback, &sig)?, 200);
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
            Some(PurchaseState::Chargeback)
        );
        assert_eq!(
            view.balance
                .as_ref()
                .map(|b| (b.total_coins, b.review_required)),
            Some((0, true))
        );
        assert!(!view.can_purchase(PACKAGE));
        assert!(view.status_text().contains("review required"));
        Ok(())
    })
}
