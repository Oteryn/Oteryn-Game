use super::*;

#[test]
fn bazaar_reopened_window_recovers_pending_before_allowing_another_mutation() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        mock.authority.lock().map_err(|_| "lock")?.pending_first = true;
        mock.runtime.block_on(async {
            let mut first = View::default();
            controller::execute(
                &mut first,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            controller::execute(
                &mut first,
                Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: true,
                },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(first.pending());
            let mut reopened = View::default();
            controller::execute(
                &mut reopened,
                Action::Refresh(AuctionView::Own),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(reopened.pending());
            assert!(!reopened.can_retry());
            controller::execute(
                &mut reopened,
                Action::Sell {
                    character_id: CHARACTER.into(),
                    minimum_bid: 1,
                    duration_hours: 24,
                    confirmed: true,
                },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(mock.authority.lock().map_err(|_| "lock")?.commands.len(), 1);
            controller::execute(
                &mut reopened,
                Action::Status,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(!reopened.pending());
            assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 1);
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}
