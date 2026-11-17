use super::*;

#[test]
fn bazaar_ambiguous_retry_keeps_identity_and_history_recovers_receipt() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let mut view = View::default();
        mock.runtime.block_on(async {
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(view.page.is_some());
            controller::execute(
                &mut view,
                Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: false,
                },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(!view.pending());
            mock.authority.lock().map_err(|_| "lock")?.fail_after_commit = true;
            controller::execute(
                &mut view,
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
            assert!(view.pending());
            let request = view.pending.clone().ok_or("pending request")?;
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
            assert_eq!(view.pending.as_ref(), Some(&request));
            controller::execute(
                &mut view,
                Action::Retry,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(!view.pending());
            assert_eq!(
                view.receipt.as_ref().map(|r| &r.state),
                Some(&OperationState::Committed)
            );
            assert!(view.status_text().contains("receipt"));
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::History),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(
                view.page.as_ref().map(|p| &p.auctions[0].state),
                Some(&AuctionState::Sold)
            );
            assert_eq!(
                view.page.as_ref().map(|p| p.recent_operations.len()),
                Some(1)
            );
            controller::execute(
                &mut view,
                Action::Receipt(request.request_id.clone()),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            let mut changed = request.clone();
            changed.command = BazaarCommand::Bid {
                auction_id: AUCTION.into(),
                expected_revision: 1,
                coins: 99,
            };
            assert!(
                client
                    .bazaar_command(&token, &changed, CancellationToken::new())
                    .await
                    .is_err()
            );
            let state = mock.authority.lock().map_err(|_| "lock")?;
            assert_eq!(state.effects, 1);
            assert_eq!(state.commands.len(), 1);
            let posts: Vec<_> = state
                .requests
                .iter()
                .filter(|r| r.starts_with("POST /api/v1/native-account/bazaar/commands "))
                .collect();
            assert_eq!(posts.len(), 3);
            assert_eq!(
                posts[0].split_once("\r\n\r\n").map(|(_, body)| body),
                posts[1].split_once("\r\n\r\n").map(|(_, body)| body)
            );
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}

#[test]
fn bazaar_pending_status_and_conflicting_terminal_receipt_are_fenced() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        mock.authority.lock().map_err(|_| "lock")?.pending_first = true;
        let mut view = View::default();
        mock.runtime.block_on(async {
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            controller::execute(
                &mut view,
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
            assert!(view.pending());
            assert_eq!(
                view.receipt.as_ref().map(|r| &r.state),
                Some(&OperationState::Pending)
            );
            controller::execute(
                &mut view,
                Action::Retry,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(view.pending());
            controller::execute(
                &mut view,
                Action::Status,
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(!view.pending());
            let original = view.receipt.clone().ok_or("receipt")?;
            {
                let mut state = mock.authority.lock().map_err(|_| "lock")?;
                assert_eq!(state.effects, 1);
                state
                    .commands
                    .get_mut(&original.request_id)
                    .ok_or("command")?
                    .1
                    .balance = 999;
            }
            controller::execute(
                &mut view,
                Action::Receipt(original.request_id.clone()),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(view.receipt, Some(original));
            assert!(view.feedback.contains("Conflicting terminal receipt"));
            assert_eq!(view.page.as_ref().map(|p| p.balance), Some(950));
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}
