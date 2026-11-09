#[test]
fn t124_market_details_use_the_existing_selected_market_route() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(128 * 1024);
                    let query = market_details::Query::Catalogue {
                        filter: market_details::Filter::default(),
                        page: 0,
                    };
                    let expected = query.clone();
                    let peer = tokio::spawn(async move {
                        let raw = if selected {
                            vec![
                                (
                                    9,
                                    0,
                                    1,
                                    item_view::encode_character_inventory(
                                        &CharacterInventory::default(),
                                    )
                                    .map_err(|e| format!("{e:?}"))?,
                                ),
                                (
                                    11,
                                    0,
                                    1,
                                    item_view::encode_open_container(&OpenContainer::default())
                                        .map_err(|e| format!("{e:?}"))?,
                                ),
                            ]
                        } else {
                            vec![]
                        };
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            if selected { &[4, 6, 37, 63] } else { &[] },
                            &[],
                            if selected {
                                item_snapshot(vec![])?
                            } else {
                                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0))
                            },
                            &raw,
                        )
                        .await?;
                        if selected {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!(
                                command.command_type,
                                market_details::COMMAND_TYPE_MARKET_DETAILS
                            );
                            assert_eq!(
                                market_details::decode_query(command.payload)
                                    .map_err(|_| "details request")?,
                                expected
                            );
                            let reply = market_details::Reply::Catalogue {
                                items: Vec::new(),
                                categories: vec!["Soul cores".into()],
                                page: 0,
                                has_next: false,
                            };
                            let payload =
                                market_details::encode_reply(&reply).map_err(|_| "reply")?;
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    1,
                                    41,
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
                    let mut session = match Session::admit(client, admission).await {
                        Ok(value) => value,
                        Err(error) => {
                            let peer_result = peer.await?;
                            return Err(format!(
                                "selected={selected}: admission={error}; peer={peer_result:?}"
                            )
                            .into());
                        }
                    };
                    let payload = market_details::encode_query(&query).map_err(|_| "query")?;
                    let result = session
                        .gated_command(63, market_details::COMMAND_TYPE_MARKET_DETAILS, &payload)
                        .await;
                    peer.await??;
                    if selected {
                        assert!(matches!(
                            market_details::decode_reply(&result?.payload)
                                .map_err(|_| "decoded reply")?,
                            market_details::Reply::Catalogue { page: 0, .. }
                        ));
                    } else {
                        assert!(matches!(
                            result,
                            Err(SessionError::CapabilityNotSelected { capability: 63 })
                        ));
                    }
                }
                Ok::<(), BoxError>(())
            })
        })?
        .join()
        .map_err(|_| "market session test thread")??
}
