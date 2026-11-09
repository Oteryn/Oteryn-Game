#[test]
fn pc05_boss_activity_refusal_and_reconnect_scenario() -> Result<(), Error> {
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
                    let (link, mut incoming, outbox) = play_channel();
                    link.boss_encounter_request(Intent::Enter);
                    assert!(!link.boss_encounter_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    link.boss_encounter_edit(0, "boss:test".into());
                    let key = wire::encode_boss_key("boss:test").checked()?;
                    let mut handoff = [1; 16];
                    handoff[6] = 0x70;
                    handoff[8] = 0x80;
                    let accepted =
                        wire::encode_boss_activity_reply(&wire::BossActivityReply::Accepted {
                            handoff,
                            deadline_unix_ms: Some(123),
                        })
                        .checked()?;
                    for (intent, kind) in [
                        (Intent::Enter, 178),
                        (Intent::Defeated, 179),
                        (Intent::Timeout, 180),
                    ] {
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Accepted,
                            accepted.clone(),
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().summary.contains("accepted"));
                        let summary = link.boss_encounter_panel().summary;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Rejected,
                            vec![],
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().feedback.is_some());
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                        let refused = wire::encode_boss_activity_reply(
                            &wire::BossActivityReply::Refused(wire::BossActivityRefusal::Cooldown),
                        )
                        .checked()?;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Rejected,
                            refused,
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().summary.contains("Cooldown"));
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::DuplicateReplay,
                            accepted.clone(),
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Accepted,
                            vec![255],
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                    }
                    link.boss_encounter_edit(0, "oteryn:creature.test".into());
                    link.boss_encounter_edit(1, "2".into());
                    link.boss_encounter_edit(2, "3".into());
                    let chest = wire::encode_reward_chest(
                        100,
                        &[wire::BossRewardChestEntry {
                            death: [7; 16],
                            boss_key: "boss:test".into(),
                            minted_at_unix_ms: 1,
                            expires_at_unix_ms: 1 + wire::BOSS_REWARD_EXPIRY_MS,
                            item_count: 2,
                        }],
                    )
                    .checked()?;
                    for (intent, kind, payload, reply, text) in [
                        (Intent::HazardQuery, 169, vec![], vec![5, 2], "Hazard: 2"),
                        (Intent::HazardSelect, 169, vec![3], vec![5, 3], "Hazard: 3"),
                        (
                            Intent::SlotsQuery,
                            491,
                            vec![0],
                            vec![0; 26],
                            "Boss slots: Updated",
                        ),
                        (
                            Intent::SlotEquip,
                            491,
                            [vec![1, 2], b"oteryn:creature.test".to_vec()].concat(),
                            vec![0; 26],
                            "Boss slots: Updated",
                        ),
                        (
                            Intent::SlotRemove,
                            491,
                            vec![2, 2],
                            vec![0; 26],
                            "Boss slots: Updated",
                        ),
                        (Intent::Rewards, 340, vec![], chest, "boss:test"),
                    ] {
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload.clone(),
                            CommandStatus::Accepted,
                            reply,
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().summary.contains(text));
                        let summary = link.boss_encounter_panel().summary;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload.clone(),
                            CommandStatus::Rejected,
                            vec![],
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload,
                            CommandStatus::Accepted,
                            vec![255],
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                    }
                    // Hold the transport failure until the pending state has been observed.
                    exchanges
                        .entries
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push_back((
                            178,
                            wire::encode_boss_key("oteryn:creature.test").checked()?,
                            None,
                            vec![],
                        ));
                    link.boss_encounter_request(Intent::Enter);
                    assert!(link.boss_encounter_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.boss_encounter_panel();
                    assert!(!ended.pending && !ended.open && !ended.selected);
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(
                        ended.summary.is_empty()
                            && ended.snapshot.is_none()
                            && ended.rewards.is_none()
                    );
                    assert!(ended.feedback.is_some_and(|v| v.contains("ended")));
                    let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnected = PlayLink {
                        commands,
                        mailbox: Arc::clone(&link.mailbox),
                    };
                    let reconnect = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        receiver,
                        outbox,
                    ));
                    ready(&reconnected).await.checked()?;
                    assert!(reconnected.boss_encounter_panel().summary.is_empty());
                    reconnected.boss_encounter_edit(0, "boss:test".into());
                    send(
                        &reconnected,
                        &exchanges,
                        Intent::Enter,
                        178,
                        key,
                        CommandStatus::Accepted,
                        accepted,
                    )
                    .await
                    .checked()?;
                    assert!(
                        reconnected
                            .boss_encounter_panel()
                            .summary
                            .contains("accepted")
                    );
                    drop(reconnected);
                    reconnect.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })
        .checked()?
        .join()
        .map_err(|_| "boss activity test thread failed")
        .checked()?
}
