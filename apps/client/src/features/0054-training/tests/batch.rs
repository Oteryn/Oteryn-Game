#[test]
fn training_batch_completion_requires_the_offline_server_receipt_and_survives_session_end()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().map_err(|e| e.to_string())?;
            runtime
                .block_on(async {
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, commands, outbox) = play_channel();
                    let completion = link.training_completion();
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: exchanges.clone(),
                        },
                        commands,
                        outbox,
                    ));
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while !link.training_panel().selected {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                    link.training_edit(0, "7".into());
                    let payload = oteryn_session::property_services::offline_payload(
                        7_u64.try_into().map_err(|e| format!("{e:?}"))?,
                    );
                    send(
                        &link,
                        &exchanges,
                        Intent::Offline,
                        66,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    assert!(!completion());
                    send(
                        &link,
                        &exchanges,
                        Intent::Offline,
                        66,
                        payload,
                        CommandStatus::Accepted,
                        wire::encode_training_result(wire::TrainingResult {
                            disposition: 3,
                            revision: 1,
                            pool_seconds: 3600,
                            charges: 0,
                        })
                        .map_err(|e| format!("{e:?}"))?,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    assert!(completion());
                    drop(link);
                    task.await.map_err(|e| e.to_string())?;
                    assert!(completion());
                    Ok::<(), String>(())
                })
                .map_err(|e| e.to_string())??;
            runtime.shutdown(Duration::from_secs(1));
            Ok(())
        })?
        .join()
        .map_err(|_| "training batch thread failed")?
        .map_err(Into::into)
}
