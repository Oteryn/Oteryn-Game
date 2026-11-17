#[test]
fn store_coin_history_uses_existing_gated_route_and_refuses_without_selection()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                use oteryn_protocol_oteryn::store;
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(128 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            if selected { &[154] } else { &[] },
                            &[],
                            (1, spatial(0)),
                            &[],
                        )
                        .await?;
                        if selected {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!(command.command_type, 184);
                            assert!(
                                store::decode_coin_history_request(command.payload)
                                    .map_err(|_| "ledger request")?
                                    .page_cursor
                                    .is_empty()
                            );
                            let payload =
                                store::encode_coin_history_page(&store::CoinHistoryPage {
                                    entries: vec![],
                                    next_page_cursor: String::new(),
                                })
                                .map_err(|_| "ledger page")?;
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    1,
                                    34 + command.command_id,
                                    command.command_id,
                                    CommandStatus::Accepted,
                                    &payload,
                                )?,
                            )
                            .await?;
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut admission = admission()?;
                    admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                    let mut session = Session::admit(client, admission).await?;
                    let request = store::encode_coin_history_request(&store::StoreHistoryRequest {
                        page_cursor: String::new(),
                    })
                    .map_err(|_| "request")?;
                    let result = session.gated_command(154, 184, &request).await;
                    if selected {
                        assert!(
                            store::decode_coin_history_page(&result?.payload)
                                .map_err(|_| "page")?
                                .entries
                                .is_empty()
                        );
                    } else {
                        assert!(matches!(
                            result,
                            Err(SessionError::CapabilityNotSelected { capability: 154 })
                        ));
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "coin history selection thread failed")?
}
