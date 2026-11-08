use super::*;

#[test]
fn bazaar_sell_and_world_transfer_recheck_mock_fences() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let mut view = View::default();
        mock.runtime.block_on(async {
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::Own),
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            controller::execute(
                &mut view,
                Action::Sell {
                    character_id: CHARACTER.into(),
                    minimum_bid: 10,
                    duration_hours: 24,
                    confirmed: true,
                },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(
                view.receipt.as_ref().map(|r| &r.state),
                Some(&OperationState::Committed)
            );
            for guard in 0..6 {
                {
                    let mut state = mock.authority.lock().map_err(|_| "lock")?;
                    state.offline = guard != 0;
                    state.lease_active = guard == 1;
                    state.in_guild = guard == 2;
                    state.has_house = guard == 3;
                    state.market = guard == 4;
                    state.cooldown = guard == 5;
                }
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
                assert!(!view.can_transfer());
                controller::execute(
                    &mut view,
                    Action::Transfer { confirmed: true },
                    &client,
                    &token,
                    CancellationToken::new(),
                )
                .await;
            }
            {
                let mut s = mock.authority.lock().map_err(|_| "lock")?;
                s.offline = true;
                s.lease_active = false;
                s.in_guild = false;
                s.has_house = false;
                s.market = false;
                s.cooldown = false;
            }
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
            assert!(view.can_transfer());
            assert!(view.status_text().contains("Ari → South"));
            view.quote_until = Some(Instant::now());
            assert!(!view.can_transfer());
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
            mock.authority.lock().map_err(|_| "lock")?.lease_active = true;
            controller::execute(
                &mut view,
                Action::Transfer { confirmed: true },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(
                view.receipt.as_ref().map(|r| &r.state),
                Some(&OperationState::Rejected)
            );
            mock.authority.lock().map_err(|_| "lock")?.lease_active = false;
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
            controller::execute(
                &mut view,
                Action::Transfer { confirmed: false },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert!(view.can_transfer());
            controller::execute(
                &mut view,
                Action::Transfer { confirmed: true },
                &client,
                &token,
                CancellationToken::new(),
            )
            .await;
            assert_eq!(
                view.receipt.as_ref().map(|r| &r.state),
                Some(&OperationState::Committed)
            );
            assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 2);
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}
