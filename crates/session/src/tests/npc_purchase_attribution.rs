use super::*;

#[test]
fn session_applies_npc_purchase_attribution_only_when_selected() -> Result<(), BoxError> {
    use npc_service::CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1;
    use oteryn_protocol_oteryn::encode_npc_purchase_inventory_delta;

    block_on(async {
        const SUPPORTED: &[u32] = &[3, 4, 6, CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1];
        for selected in [false, true] {
            let caps = if selected { SUPPORTED } else { &[3, 4, 6] };
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer_with(
                    &mut server,
                    SUPPORTED,
                    caps,
                    &[],
                    item_snapshot(vec![])?,
                    &[inventory_domain(1, &CharacterInventory::default())?],
                )
                .await
            });
            let mut session = Session::admit(client, admission_with(SUPPORTED)?).await?;
            peer.await??;
            let payload = item_view::encode_character_inventory(&CharacterInventory::default())
                .map_err(|error| format!("{error:?}"))?;
            let frame = encode_npc_purchase_inventory_delta(1, 41, 1, 2, &payload, &[7, 8])?;
            let outcome = session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload());
            if selected {
                assert!(matches!(outcome?, SessionEvent::Inventory(_)));
                assert_eq!(session.inventory_npc_purchase_command_ids(), &[7, 8]);
                let next =
                    encode_state_delta(1, 42, STATE_DOMAIN_CHARACTER_INVENTORY, 2, 3, 1, &payload)?;
                session.apply_state_delta(42, decode_wire_envelope(&next)?.payload())?;
                assert!(session.inventory_npc_purchase_command_ids().is_empty());
            } else {
                assert!(matches!(
                    outcome,
                    Err(SessionError::UnselectedDomain {
                        domain_id: STATE_DOMAIN_CHARACTER_INVENTORY
                    })
                ));
                assert!(session.inventory_npc_purchase_command_ids().is_empty());
                assert_eq!(session.inventory_revision, 1);
            }
        }
        Ok(())
    })?
}
