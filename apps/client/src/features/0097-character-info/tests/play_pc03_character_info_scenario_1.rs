#[test]
fn pc03_character_info_scenario() -> Result<(), Error> {
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
                    link.character_info_request(Intent::Query);
                    assert!(!link.character_info_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(13).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.character_info_input(&[event]));
                    assert!(link.character_info_panel().open);
                    let fields = ["0", "50", "", ""];
                    for (i, value) in fields.iter().enumerate() {
                        link.character_info_edit(i, (*value).into());
                    }
                    let intent = Intent::Query;
                    let payload = vec![0, 0, 0, 50];
                    let reply = {
                        let mut bytes = vec![0];
                        bytes.extend_from_slice(&42i64.to_le_bytes());
                        bytes.resize(69, 0);
                        bytes.extend_from_slice(&[0, 0]);
                        bytes
                    };
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    assert!(!link.character_info_panel().summary.is_empty());
                    link.character_info_edit(2, "11111111-1111-1111-1111-111111111111".into());
                    let mut visible = vec![1];
                    visible.extend_from_slice(&[0x11; 16]);
                    visible.extend_from_slice(&[0, 0, 50]);
                    send(
                        &link,
                        &exchanges,
                        Intent::Visible,
                        113,
                        visible,
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    let mut look = vec![4];
                    look.extend_from_slice(&[0x11; 16]);
                    send(
                        &link,
                        &exchanges,
                        Intent::Look,
                        113,
                        look,
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    link.character_info_edit(3, "1".into());
                    send(
                        &link,
                        &exchanges,
                        Intent::Unlock,
                        113,
                        vec![2, 1],
                        CommandStatus::Accepted,
                        vec![2, 1],
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.character_info_panel()
                            .summary
                            .contains("Adept Armorer")
                    );
                    let mut selected = vec![3];
                    selected.extend_from_slice(&1u64.to_le_bytes());
                    send(
                        &link,
                        &exchanges,
                        Intent::Select,
                        113,
                        vec![3, 1],
                        CommandStatus::Accepted,
                        selected.clone(),
                    )
                    .await
                    .checked()?;
                    link.character_info_edit(3, "0".into());
                    send(
                        &link,
                        &exchanges,
                        Intent::Select,
                        113,
                        vec![3, 0],
                        CommandStatus::Accepted,
                        selected,
                    )
                    .await
                    .checked()?;
                    assert!(link.character_info_panel().summary.contains("None"));
                    link.character_info_edit(1, "51".into());
                    link.character_info_request(Intent::Query);
                    assert!(!link.character_info_panel().pending);
                    assert!(link.character_info_panel().feedback.contains("1-50"));
                    link.character_info_edit(1, "50".into());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    let reconnect_payload = payload.clone();
                    let reconnect_reply = reply.clone();
                    let summary = link.character_info_panel().summary;
                    assert!(!summary.is_empty());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.character_info_panel().feedback.contains("refused"));
                    assert_eq!(link.character_info_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.character_info_panel().feedback.contains("Invalid"));
                    assert_eq!(link.character_info_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        113,
                        payload,
                        CommandStatus::DuplicateReplay,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.character_info_panel().summary, summary);
                    // An unexpected transport end while a request is in flight resets every session field.
                    link.character_info_request(intent);
                    assert!(link.character_info_panel().pending);
                    task.await.checked()?;
                    let ended = link.character_info_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    link.character_info_request(intent);
                    assert!(!link.character_info_panel().pending);
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
                    let fresh = reconnected.character_info_panel();
                    assert!(!fresh.open && !fresh.pending);
                    assert!(fresh.summary.is_empty() && fresh.fields.iter().all(String::is_empty));
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.character_info_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        113,
                        reconnect_payload,
                        CommandStatus::Accepted,
                        reconnect_reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.character_info_panel().summary.is_empty());
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
        .map_err(|_| "scenario thread failed")
        .checked()?
}
