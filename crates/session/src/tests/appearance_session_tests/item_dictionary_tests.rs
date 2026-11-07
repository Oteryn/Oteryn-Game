//! Item dictionary checks exercised through actual admission and domain dispatch.
use super::*;

#[test]
fn appearance_item_dictionary_validates_snapshot_and_delta_atomically() -> Result<(), BoxError> {
    block_on(async {
        for case in 0..7 {
            let (mut snapshot, entity, spatial) = appearance_fixture::snapshot()?;
            snapshot.bindings[0].look = appearance::Look::Item { definition_ref: 1 };
            let domain = (21, 1, 1, snapshot.encode().map_err(|e| format!("{e:?}"))?);
            let mut binding = snapshot.bindings[0].clone();
            binding.root_revision += 1;
            binding.look = appearance::Look::Item {
                definition_ref: if case == 0 { 2 } else { 3 },
            };
            let mappings = appearance::Mappings {
                context: snapshot.catalogue.context,
                upserts: vec![binding],
                forgets: Vec::new(),
            }
            .encode()
            .map_err(|e| format!("{e:?}"))?;
            let (client, mut server) = tokio::io::duplex(256 * 1024);
            let peer = tokio::spawn(async move {
                join_peer_with(
                    &mut server,
                    CLIENT_SUPPORTED_CAPABILITIES,
                    &[4, 6, 25],
                    &[],
                    spatial,
                    &[domain],
                )
                .await?;
                if case < 2 {
                    write_frame(
                        &mut server,
                        &encode_state_delta(1, 41, 21, 1, 2, 1, &mappings)?,
                    )
                    .await?;
                }
                let mut rest = Vec::new();
                server.read_to_end(&mut rest).await?;
                Ok::<_, BoxError>(())
            });
            let mut admission = entity_admission()?;
            admission.appearance_items = match case {
                0 | 1 => Some(AppearanceItemDictionary {
                    content_generation: CONTENT_GENERATION,
                    definition_refs: &[1, 2],
                }),
                2 => None,
                3 => Some(AppearanceItemDictionary {
                    content_generation: [9; 32],
                    definition_refs: &[1, 2],
                }),
                4 => Some(AppearanceItemDictionary {
                    content_generation: CONTENT_GENERATION,
                    definition_refs: &[2],
                }),
                5 => Some(AppearanceItemDictionary {
                    content_generation: CONTENT_GENERATION,
                    definition_refs: &[0, 1],
                }),
                _ => Some(AppearanceItemDictionary {
                    content_generation: CONTENT_GENERATION,
                    definition_refs: &[1, 1],
                }),
            };
            let admitted = Session::admit(client, admission).await;
            if case < 2 {
                let mut session = admitted?;
                let result = session.service_liveness(Duration::from_millis(30)).await;
                let state = session.appearance.as_ref().ok_or("appearance")?;
                if case == 0 {
                    result?;
                    assert_eq!(
                        state.resolve(entity.entity, 1),
                        Some(&appearance::Look::Item { definition_ref: 2 })
                    );
                    assert_eq!(session.take_events().len(), 1);
                } else {
                    assert!(result.is_err());
                    assert_eq!(
                        state.resolve(entity.entity, 1),
                        Some(&appearance::Look::Item { definition_ref: 1 })
                    );
                    assert!(session.take_events().is_empty());
                }
                drop(session);
            } else {
                assert!(admitted.is_err(), "invalid dictionary case {case}");
            }
            peer.await??;
        }
        Ok(())
    })?
}
