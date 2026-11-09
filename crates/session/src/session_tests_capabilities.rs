use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
};

#[test]
fn prey_native_capability_routes_tier_and_confirmed_pick_and_refuses_without_selection()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(128 * 1024);
                    let requests = [
                        prey::Request::SetTier { slot: 1, tier: 3 },
                        prey::Request::OfferedToday {
                            slot: 1,
                            after: String::new(),
                        },
                        prey::Request::PickAgain {
                            slot: 1,
                            creature: "oteryn:creature.dragon".into(),
                            expected_price_gold: 45000,
                        },
                    ];
                    let expected = requests.clone();
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            if selected { &[140] } else { &[] },
                            &[],
                            (1, spatial(0)),
                            &[],
                        )
                        .await?;
                        if selected {
                            for (index, expected) in expected.into_iter().enumerate() {
                                let frame = read_frame(&mut server).await?;
                                let envelope = decode_wire_envelope(&frame)?;
                                let command = envelope.client_command(1)?;
                                let id = 7 + index as u64;
                                assert_eq!((command.command_type, command.command_id), (160, id));
                                assert_eq!(
                                    prey::decode_request(command.payload).map_err(|_| "request")?,
                                    expected
                                );
                                let reply = prey::Reply {
                                    status: 0,
                                    wildcard_balance: 10,
                                    slots: Vec::new(),
                                    list_size: 12,
                                    expansion_owned: true,
                                    offered_today: vec![prey::OfferedCreature {
                                        creature: "oteryn:creature.dragon".into(),
                                        price_gold: 45000,
                                        occupied_slot: 0,
                                    }],
                                    offered_next: String::new(),
                                };
                                let payload = prey::encode_reply(&reply).map_err(|_| "reply")?;
                                write_frame(
                                    &mut server,
                                    &encode_command_result(
                                        1,
                                        34 + id,
                                        id,
                                        CommandStatus::Accepted,
                                        &payload,
                                    )?,
                                )
                                .await?;
                            }
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut admission = admission()?;
                    admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                    let mut session = Session::admit(client, admission).await?;
                    for request in requests {
                        let payload = prey::encode_request(&request).map_err(|_| "request")?;
                        let result = session.gated_command(140, 160, &payload).await;
                        if selected {
                            let reply =
                                prey::decode_reply(&result?.payload).map_err(|_| "reply")?;
                            assert_eq!(reply.list_size, 12);
                            assert_eq!(reply.offered_today[0].price_gold, 45000);
                        } else {
                            assert!(matches!(
                                result,
                                Err(SessionError::CapabilityNotSelected { capability: 140 })
                            ));
                        }
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "Prey selection thread panicked")?
}

#[path="session_tests_capabilities/gated_routes.rs"]
mod gated_route_tests;
