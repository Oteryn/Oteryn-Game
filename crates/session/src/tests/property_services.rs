use super::*;

#[test]
fn cp01_property_commands_use_admitted_session_routes() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        block_on(async {
            for selected in [false, true] {
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let cases = [(54,66),(54,67),(55,68),(55,69),(115,124),(150,174),(150,175),(150,176)];
                let peer = tokio::spawn(async move {
                    if selected {
                        use oteryn_protocol_oteryn::world_map::{MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot};
                        let map = encode_world_map_snapshot(&WorldMapViewSnapshot {
                            header: MapViewHeader {content_generation:CONTENT_GENERATION,bundle_digest:[0x22;32],reset_epoch:0,origin:at(0,0),first_visible_floor:0},
                            tiles:vec![],
                        }).map_err(|error|format!("{error:?}"))?;
                        let inventory = item_view::encode_character_inventory(&CharacterInventory::default()).map_err(|error|format!("{error:?}"))?;
                        let container = item_view::encode_open_container(&OpenContainer::default()).map_err(|error|format!("{error:?}"))?;
                        join_peer_with(&mut server,CLIENT_SUPPORTED_CAPABILITIES,
                            &[4,6,18,21,49,54,55,115,150],&[],item_snapshot(vec![])?,
                            &[(9,0,1,inventory),(11,0,1,container),(17,0,1,map)]).await?;
                    } else {
                        join_peer(&mut server,CLIENT_SUPPORTED_CAPABILITIES,&[],&[]).await?;
                    }
                    if selected {
                        for (index, (_,kind)) in cases.iter().enumerate() {
                            let frame = read_frame(&mut server).await?;
                            let command = decode_wire_envelope(&frame)?.client_command(1)?;
                            assert_eq!(command.command_type,*kind);
                            assert_eq!(command.payload,b"feature-owned payload");
                            write_frame(&mut server,&encode_command_result(1,41+index as u64,7+index as u64,
                                CommandStatus::Accepted,b"server reply")?).await?;
                        }
                    }
                    Ok::<(),BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(CLIENT_SUPPORTED_CAPABILITIES)?).await.map_err(|error| format!("property admission selected={selected}: {error:?}"))?;
                for (capability,kind) in cases {
                    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
                    let result = session.gated_command(capability,kind,b"feature-owned payload").await;
                    if selected { assert_eq!(result.map_err(|error| format!("property selected={selected} command={kind}: {error:?}"))?.payload,b"server reply"); }
                    else { assert!(matches!(result,Err(SessionError::CapabilityNotSelected {capability: id}) if id == capability)); }
                }
                peer.await??;
            }
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&49));
            Ok::<(),BoxError>(())
        })?
    })?.join().map_err(|_| "property service session thread failed")?
}

#[test]
fn cp01_training_adapter_round_trips_boundaries_and_rejects_malformed_replies()
-> Result<(), BoxError> {
    use crate::property_services::*;
    use loop_training::{ExerciseIntent, TrainingResult};
    for value in [1, 127, 128, u64::MAX] {
        let handle = ItemHandle::new(value).ok_or("handle")?;
        assert_eq!(
            loop_training::decode_offline_training(&offline_payload(handle))
                .map_err(|e| format!("{e:?}"))?,
            handle
        );
        let intent = ExerciseIntent::Start {
            weapon: handle,
            dummy: handle,
        };
        assert_eq!(
            loop_training::decode_exercise_training(&exercise_payload(intent))
                .map_err(|e| format!("{e:?}"))?,
            intent
        );
    }
    for intent in [ExerciseIntent::Status, ExerciseIntent::Stop] {
        assert_eq!(
            loop_training::decode_exercise_training(&exercise_payload(intent))
                .map_err(|e| format!("{e:?}"))?,
            intent
        );
    }
    for reply in [
        TrainingResult::REFUSED,
        TrainingResult {
            disposition: 1,
            revision: u64::MAX,
            pool_seconds: 43200,
            charges: u32::MAX,
        },
    ] {
        let bytes = loop_training::encode_training_result(reply).map_err(|e| format!("{e:?}"))?;
        assert_eq!(training_result(&bytes), Some(reply));
        for n in 0..bytes.len() {
            assert!(training_result(&bytes[..n]).is_none());
        }
        let mut extra = bytes.clone();
        extra.extend([8, 1]);
        assert!(training_result(&extra).is_none());
        let mut unknown = bytes;
        unknown.extend([40, 0]);
        assert!(training_result(&unknown).is_none());
    }
    assert!(training_result(&[8, 1, 16, 0, 24, 0, 32, 0]).is_none());
    assert!(training_result(&[8, 1, 16, 0x80]).is_none());
    Ok(())
}
