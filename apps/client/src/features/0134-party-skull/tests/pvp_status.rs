use super::*;
#[test]
fn own_pvp_status_crosses_session_caller_and_rejects_bad_replies() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    let status = wire::PvpStatus {
                        points_milli: [1500, 2500, 4500],
                        red_limits_milli: [3000, 5000, 10000],
                        black_limits_milli: [6000, 10000, 20000],
                        skull: 4,
                        remaining_ms: 60001,
                        situations: vec![wire::PvpSituation {
                            incoming: true,
                            remaining_ms: 9000,
                        }],
                    };
                    send(
                        &link,
                        &exchanges,
                        Intent::PvpStatus,
                        155,
                        vec![8, 1],
                        CommandStatus::Accepted,
                        status.encode().checked()?,
                    )
                    .await
                    .checked()?;
                    let panel = link.party_skull_panel();
                    let text = oteryn_client_ui::party_skull::panel_text(&panel);
                    assert!(text.contains("Daily: 1.500 / 3.000"));
                    assert!(text.contains("Weekly: 2.500 / 5.000"));
                    assert!(text.contains("Monthly: 4.500 / 10.000"));
                    assert!(text.contains("Red skull:"));
                    assert_eq!(
                        panel.skull_timer.map(|timer| timer.remaining_seconds(0)),
                        Some(Some(61))
                    );
                    assert!(text.contains("Incoming attack: 9s"));
                    assert!(text.contains("6.000 / 10.000 / 20.000"));
                    send(
                        &link,
                        &exchanges,
                        Intent::PvpStatus,
                        155,
                        vec![8, 1],
                        CommandStatus::Accepted,
                        vec![0],
                    )
                    .await
                    .checked()?;
                    assert!(link.party_skull_panel().unjustified.is_none());
                    assert!(link.party_skull_panel().skull_timer.is_none());
                    link.party_skull_request(Intent::Query);
                    task.await.checked()?;
                    Ok(())
                })
                .checked()?
        })?
        .join()
        .map_err(|_| "PvP status client thread")?
}

#[test]
fn frag_limits_and_open_situations_cross_caller_and_clear_on_refusal() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    let mut status = wire::PvpStatus {
                        points_milli: [500, 1500, 2500],
                        red_limits_milli: [3000, 5000, 10000],
                        black_limits_milli: [6000, 10000, 20000],
                        skull: 0,
                        remaining_ms: 0,
                        situations: vec![wire::PvpSituation {
                            incoming: false,
                            remaining_ms: 1000,
                        }],
                    };
                    send(
                        &link,
                        &exchanges,
                        Intent::PvpStatus,
                        155,
                        vec![8, 1],
                        CommandStatus::Accepted,
                        status.encode().checked()?,
                    )
                    .await
                    .checked()?;
                    let panel = link.party_skull_panel();
                    assert_eq!(panel.black_limits_milli, Some(status.black_limits_milli));
                    let snapshot = panel
                        .unjustified
                        .as_ref()
                        .ok_or("missing frag observation")?;
                    assert_eq!(
                        snapshot
                            .windows
                            .map(|window| (window.points_milli, window.limit_milli)),
                        [(500, 3000), (1500, 5000), (2500, 10000)]
                    );
                    assert!(
                        oteryn_client_ui::party_skull::panel_text(&panel)
                            .contains("Outgoing attack: 1s")
                    );
                    status.situations.clear();
                    send(
                        &link,
                        &exchanges,
                        Intent::PvpStatus,
                        155,
                        vec![8, 1],
                        CommandStatus::Accepted,
                        status.encode().checked()?,
                    )
                    .await
                    .checked()?;
                    assert_eq!(
                        link.party_skull_panel()
                            .unjustified
                            .and_then(|snapshot| snapshot.open_situations),
                        Some(vec![])
                    );
                    assert!(
                        oteryn_client_ui::party_skull::panel_text(&link.party_skull_panel())
                            .contains("No open PvP situations")
                    );
                    send(
                        &link,
                        &exchanges,
                        Intent::PvpStatus,
                        155,
                        vec![8, 1],
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    let refused = link.party_skull_panel();
                    assert!(refused.unjustified.is_none());
                    assert!(refused.skull_timer.is_none());
                    assert!(refused.black_limits_milli.is_none());
                    assert!(refused.feedback.contains("refused"));
                    link.party_skull_request(Intent::Query);
                    task.await.checked()?;
                    Ok(())
                })
                .checked()?
        })?
        .join()
        .map_err(|_| "frag limits client thread")?
}
