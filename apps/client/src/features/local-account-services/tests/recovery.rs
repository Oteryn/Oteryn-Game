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

#[test]
fn bazaar_resolving_one_pending_with_failed_refresh_keeps_other_mutations_fenced() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        mock.authority.lock().map_err(|_| "lock")?.pending_first = true;
        mock.runtime.block_on(async {
            for _ in 0..2 {
                let request = BazaarRequest {
                    version: 1,
                    request_id: AttemptRef::generate()?.as_str().into(),
                    command: BazaarCommand::Bid {
                        auction_id: AUCTION.into(),
                        expected_revision: 1,
                        coins: 50,
                    },
                };
                assert_eq!(
                    client
                        .bazaar_command(&token, &request, CancellationToken::new())
                        .await?
                        .state,
                    OperationState::Pending
                );
            }
            let mut view = View::default();
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::Own),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(view.pending());
            mock.authority.lock().map_err(|_| "lock")?.fail_pages = true;
            controller::execute(
                &mut view,
                Action::Status,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(
                view.pending(),
                "the second known pending operation must retain the mutation gate"
            );
            controller::execute(
                &mut view,
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
            assert_eq!(mock.authority.lock().map_err(|_| "lock")?.commands.len(), 2);
            controller::execute(
                &mut view,
                Action::Status,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(!view.pending());
            assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 2);
            mock.authority.lock().map_err(|_| "lock")?.fail_pages = false;
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::Own),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(view.page.as_ref().map(|p| p.balance), Some(900));
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}
