use super::*;
use oteryn_protocol_oteryn::pvp_expert as wire;

#[test]
fn pc01_expert_session_validates_snapshot_and_delivers_confirmed_modes() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for malformed in [true, false] {
                    for result_first in [true, false] {
                        let (client, mut server) = tokio::io::duplex(128 * 1024);
                        let initial = wire::ExpertFightModes::default();
                        let requested = wire::ExpertFightModes {
                            expert: wire::ExpertMode::RedFist,
                            ..initial
                        };
                        // Optional PvP overrides the requested expert mode to Dove.
                        let confirmed = initial;
                        let peer = tokio::spawn(async move {
                            let combat =
                                attack::encode_actor_combat_state(&combat_state(None, false))
                                    .map_err(|e| format!("{e:?}"))?;
                            let modes = if malformed {
                                vec![255]
                            } else {
                                wire::encode_expert_fight_modes(&initial)
                            };
                            join_peer_with(
                                &mut server,
                                CLIENT_SUPPORTED_CAPABILITIES,
                                &[6, 17, 87],
                                &[],
                                entity_snapshot(vec![]),
                                &[(10, 1, 1, combat), (52, 1, 1, modes)],
                            )
                            .await?;
                            if !malformed {
                                let command = read_frame(&mut server).await?;
                                let command = decode_wire_envelope(&command)?.client_command(1)?;
                                assert_eq!(command.command_type, 103);
                                assert_eq!(
                                    wire::decode_expert_fight_modes(command.payload)
                                        .map_err(|e| format!("{e:?}"))?,
                                    requested
                                );
                                let result = encode_command_result(
                                    1,
                                    if result_first { 41 } else { 42 },
                                    7,
                                    CommandStatus::Accepted,
                                    &attack::encode_attack_intent_result(
                                        attack::AttackIntentDisposition::Ok,
                                    ),
                                )?;
                                let push = encode_state_delta(
                                    1,
                                    if result_first { 42 } else { 41 },
                                    52,
                                    1,
                                    2,
                                    1,
                                    &wire::encode_expert_fight_modes(&confirmed),
                                )?;
                                if result_first {
                                    write_frame(&mut server, &result).await?;
                                    write_frame(&mut server, &push).await?;
                                } else {
                                    write_frame(&mut server, &push).await?;
                                    write_frame(&mut server, &result).await?;
                                }
                            }
                            let mut rest = Vec::new();
                            server.read_to_end(&mut rest).await?;
                            Ok::<(), BoxError>(())
                        });
                        let admitted =
                            Session::admit(client, admission_with(CLIENT_SUPPORTED_CAPABILITIES)?)
                                .await;
                        if malformed {
                            assert!(admitted.is_err(), "malformed expert snapshot was admitted");
                            drop(admitted);
                        } else {
                            let mut session = admitted?;
                            let result = session
                                .gated_command(
                                    87,
                                    103,
                                    &wire::encode_expert_fight_modes(&requested),
                                )
                                .await?;
                            assert_eq!(result.status, CommandStatus::Accepted);
                            tokio::time::timeout(Duration::from_secs(3), async {
                                loop {
                                    if let Some(event) = session
                                        .take_events()
                                        .into_iter()
                                        .find(|e| matches!(e, SessionEvent::ExpertFightModes(_)))
                                    {
                                        let SessionEvent::ExpertFightModes(delta) = event else {
                                            return Err("event".into());
                                        };
                                        assert_eq!(delta.value, confirmed);
                                        assert_eq!(delta.new_revision, 2);
                                        return Ok::<(), BoxError>(());
                                    }
                                    session.service_liveness(Duration::from_millis(10)).await?;
                                }
                            })
                            .await??;
                            drop(session);
                        }
                        peer.await??;
                    }
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "expert session test thread failed")?
}

#[test]
fn pc01_panel_capabilities_have_registered_session_routes() {
    for (capability, command) in [(25, 25), (67, 84), (87, 103)] {
        assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
        assert!(
            crate::capability_support::gated_routes()
                .any(|(id, commands, _)| id == capability && commands.contains(&command))
        );
    }
}

#[test]
fn party_icons_are_typed_and_require_current_visible_player_incarnations() -> Result<(), BoxError> {
    block_on(async {
        use oteryn_protocol_oteryn::party_skull::{PartyIcon, encode_party_icons};
        for (entity, valid) in [
            (own(at(0, 0)).entity, true),
            (actor(EntityKind::Player, 9, 1, at(0, 1)).entity, false),
            (actor(EntityKind::Creature, 1, 1, at(0, 1)).entity, false),
            (
                EntityRef {
                    identity: OWN,
                    generation: 2,
                },
                false,
            ),
        ] {
            let icons = vec![PartyIcon {
                entity,
                shield: 3,
                skull: 2,
            }];
            let mut payload = encode_world_spatial_entities_delta(&entities_delta(
                at(0, 0),
                vec![],
                vec![],
                vec![],
            ))
            .map_err(|error| format!("{error:?}"))?;
            let encoded = encode_party_icons(&icons).map_err(|error| format!("{error:?}"))?;
            payload.push(0x32);
            payload.push(u8::try_from(encoded.len())?);
            payload.extend(encoded);
            let frame = encode_state_delta(1, 41, 1, 5, 6, 2, &payload)?;
            let (client, peer) = entity_peer(
                entity_snapshot(vec![actor(EntityKind::Creature, 1, 1, at(0, 1))]),
                &[6, 42, 134],
                vec![Step::Send(frame)],
            );
            let mut session = Session::admit(client, entity_admission()?).await?;
            let result = session.service_liveness(Duration::from_millis(100)).await;
            if valid {
                result?;
                assert_eq!(
                    session.take_events(),
                    vec![SessionEvent::PartyIcons(AppliedDelta {
                        server_sequence: 41,
                        base_revision: 5,
                        new_revision: 6,
                        value: icons,
                    })]
                );
            } else {
                assert!(result.is_err());
                assert_eq!(session.spatial_revision, 5);
                assert!(session.take_events().is_empty());
            }
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}
