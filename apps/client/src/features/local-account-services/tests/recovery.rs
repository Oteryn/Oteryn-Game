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
            mock.authority.lock().map_err(|_| "lock")?.pending_first = false;
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
            mock.authority.lock().map_err(|_| "lock")?.pending_first = false;
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

#[test]
fn refreshed_tracked_receipt_reconciles_pending_and_preserves_terminal_identity() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        mock.runtime.block_on(async {
            let mut view = View::default();
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
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
            let request = view.pending.clone().ok_or("ambiguous request")?;
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(!view.pending());
            assert!(!view.can_retry());
            let receipt = view.receipt.clone().ok_or("refreshed receipt")?;
            assert_eq!(receipt.request_id, request.request_id);
            assert_eq!(receipt.state, OperationState::Committed);
            controller::execute(
                &mut view,
                Action::Quote {
                    character_id: CHARACTER.into(),
                    world_id: DESTINATION.into(),
                },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(view.quote.is_some());
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(
                view.quote.is_some(),
                "unchanged receipts must preserve a fresh quote"
            );
            mock.authority
                .lock()
                .map_err(|_| "lock")?
                .commands
                .get_mut(&request.request_id)
                .ok_or("command")?
                .1
                .balance = 999;
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(view.receipt, Some(receipt));
            assert!(view.feedback.contains("Conflicting terminal receipt"));
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}

#[test]
fn bazaar_retry_and_precreate_refusals_settle_after_missing_status() -> TestResult {
    test_thread(|| {
        let mut settled = Vec::new();
        for (first, retry) in [(0, Some(409)), (403, None), (0, Some(403))] {
            let mock = MockPlatform::start()?;
            let client = mock.client()?;
            let token = mock.token(&client)?;
            let mut view = View::default();
            let execute = |view: &mut View, action| {
                mock.runtime.block_on(controller::execute(
                    view,
                    action,
                    &client,
                    &token,
                    CancellationToken::new(),
                ))
            };
            execute(&mut view, Action::Refresh(AuctionView::All));
            mock.authority
                .lock()
                .map_err(|_| "lock")?
                .bazaar_post_failure = Some(first);
            execute(
                &mut view,
                Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: true,
                },
            );
            if let Some(refusal) = retry {
                let request = view.pending.clone().ok_or("ambiguous bid")?;
                mock.authority
                    .lock()
                    .map_err(|_| "lock")?
                    .bazaar_post_failure = Some(refusal);
                execute(&mut view, Action::Retry);
                assert_eq!(view.pending.as_ref(), Some(&request));
                assert_eq!(view.can_retry(), refusal == 409);
                execute(&mut view, Action::Status);
            }
            settled.push(!view.pending());
            assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 0);
            if retry.is_some() {
                let pending = view.pending.clone().ok_or("missing unresolved request")?;
                assert!(view.can_retry());
                mock.authority
                    .lock()
                    .map_err(|_| "lock")?
                    .bazaar_post_failure = Some(403);
                execute(&mut view, Action::Retry);
                assert!(!view.can_retry());
                execute(&mut view, Action::Refresh(AuctionView::All));
                assert!(view.can_retry());
                execute(&mut view, Action::Retry);
                assert_eq!(
                    view.receipt.as_ref().map(|r| &r.request_id),
                    Some(&pending.request_id)
                );
                assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 1);
            }
        }
        assert_eq!(
            settled,
            [false, true, false],
            "only the initial definitive refusal settles the request"
        );
        Ok(())
    })
}

#[test]
fn bazaar_reopen_with_same_authorization_keeps_retry_before_first_receipt() -> TestResult {
    use std::sync::atomic::{AtomicUsize, Ordering};
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let stack = crate::native_entry::test_support::Stack::start_two_channels()?;
        let mut config = stack.config();
        let step = AtomicUsize::new(0);
        config.account_services =
            Arc::new(move |view, _| match step.fetch_add(1, Ordering::SeqCst) {
                0 => Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: true,
                },
                1 => {
                    assert!(view.can_retry());
                    Action::Back
                }
                2 => {
                    assert!(view.can_retry(), "reopen lost same-authorization Retry");
                    Action::Retry
                }
                _ => Action::Back,
            });
        mock.authority
            .lock()
            .map_err(|_| "lock")?
            .bazaar_post_failure = Some(0);
        let mut session = Session::default();
        for _ in 0..2 {
            mock.runtime.block_on(run(
                &client,
                &token,
                &config,
                &mut session,
                CancellationToken::new(),
            ))?;
        }
        assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 1);
        Ok(())
    })
}

#[test]
fn fresh_grant_filters_previously_accepted_bazaar_receipt() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let next = mock.token(&client)?;
        let execute = |view: &mut View, action, grant| {
            mock.runtime.block_on(controller::execute(
                view,
                action,
                &client,
                grant,
                CancellationToken::new(),
            ));
        };
        let mut first = View::default();
        execute(&mut first, Action::Refresh(AuctionView::All), &token);
        execute(
            &mut first,
            Action::Bid {
                auction_id: AUCTION.into(),
                coins: 50,
                confirmed: true,
            },
            &token,
        );
        let id = first.receipt.ok_or("receipt")?.request_id;
        {
            let mut state = mock.authority.lock().map_err(|_| "lock")?;
            let listed = state.commands.remove(&id).ok_or("listed receipt")?;
            state.commands.insert("stale-index".into(), listed);
        }
        let mut fresh = View {
            prior: first.prior,
            ..Default::default()
        };
        execute(&mut fresh, Action::Refresh(AuctionView::Own), &next);
        assert!(fresh.page.is_some());
        assert!(!format!("{fresh:?}").contains(&id));
        assert!(!fresh.pending());
        Ok(())
    })
}

fn seed_bids(
    mock: &MockPlatform,
    client: &NativeLoginClient,
    token: &AccessToken,
    count: usize,
) -> TestResult {
    for _ in 0..count {
        let request = BazaarRequest {
            version: 1,
            request_id: AttemptRef::generate()?.as_str().into(),
            command: BazaarCommand::Bid {
                auction_id: AUCTION.into(),
                expected_revision: 1,
                coins: 50,
            },
        };
        mock.runtime
            .block_on(client.bazaar_command(token, &request, CancellationToken::new()))?;
    }
    Ok(())
}

#[test]
fn transient_status_failure_fences_until_status_retry() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        seed_bids(&mock, &client, &token, 1)?;
        mock.authority.lock().map_err(|_| "lock")?.fail_status_once = true;
        let mut view = View::default();
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh(AuctionView::All),
            &client,
            &token,
            CancellationToken::new(),
        ));
        assert!(view.pending());
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Status,
            &client,
            &token,
            CancellationToken::new(),
        ));
        assert!(!view.pending());
        Ok(())
    })
}

#[test]
fn bazaar_refresh_looks_up_both_rows_concurrently() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        seed_bids(&mock, &client, &token, 2)?;
        mock.authority.lock().map_err(|_| "lock")?.status_barrier =
            Some(Arc::new(tokio::sync::Barrier::new(2)));
        let mut view = View::default();
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh(AuctionView::All),
            &client,
            &token,
            CancellationToken::new(),
        ));
        assert_eq!(view.page.ok_or("page")?.recent_operations.len(), 2);
        Ok(())
    })
}
