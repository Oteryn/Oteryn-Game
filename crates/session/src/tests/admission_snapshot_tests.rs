use super::*;

#[test]
fn admission_snapshot_preserves_wire_evidence_through_replacement() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let selection = &[
                    CAPABILITY_ITEM_VIEW_MOVE_V1,
                    CAPABILITY_WORLD_SPATIAL_ENTITIES,
                ];
                let inventory = CharacterInventory {
                    main_backpack: Some(entry(1, 2854, 1)),
                    entries: vec![entry(7, 3031, 1)],
                    ..CharacterInventory::default()
                };
                let (inventory_id, revision, inventory_type, payload) =
                    inventory_domain(8, &inventory)?;
                let (spatial_type, spatial_payload) = item_snapshot(vec![])?;
                let expected_inventory = GatedSnapshot {
                    domain_id: inventory_id,
                    revision,
                    snapshot_type: inventory_type,
                    payload: payload.clone(),
                };
                let expected_spatial = GatedSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: spatial_type,
                    payload: spatial_payload.clone(),
                };
                let updated_inventory = CharacterInventory {
                    main_backpack: inventory.main_backpack,
                    entries: vec![entry(7, 3031, 2)],
                    ..CharacterInventory::default()
                };
                let (_, new_revision, _, replacement) = inventory_domain(9, &updated_inventory)?;
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        selection,
                        selection,
                        &[],
                        (spatial_type, spatial_payload),
                        &[(inventory_id, revision, inventory_type, payload)],
                    )
                    .await?;
                    for frame in encode_single_chunk_snapshot(
                        1,
                        2,
                        41,
                        &[DomainSnapshot {
                            domain_id: inventory_id,
                            revision: new_revision,
                            snapshot_type: inventory_type,
                            payload: &replacement,
                        }],
                    )? {
                        write_frame(&mut server, &frame).await?;
                    }
                    server.read_to_end(&mut Vec::new()).await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(selection)?).await?;
                assert_eq!(session.inventory(), Some(&inventory));
                assert_eq!(
                    session.admission_snapshot(inventory_id),
                    Some(&expected_inventory)
                );
                assert_eq!(
                    session.admission_snapshot(expected_spatial.domain_id),
                    Some(&expected_spatial)
                );
                assert!(session.gated_snapshot(inventory_id).is_none());
                assert!(session.gated_snapshot(expected_spatial.domain_id).is_none());

                session.wait_for_events(DEADLINE).await?;
                assert_eq!(session.inventory(), Some(&updated_inventory));
                assert_eq!(
                    session.admission_snapshot(inventory_id),
                    Some(&expected_inventory)
                );
                assert_eq!(
                    session.admission_snapshot(expected_spatial.domain_id),
                    Some(&expected_spatial)
                );
                drop(session);
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "admission snapshot test thread failed")?
}
