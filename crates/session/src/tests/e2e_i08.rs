//! SCENARIO_MATRIX I08 client replay.
//!
//! The Primal Bag USE result is decoded by
//! `crates/client-runtime/src/features/0011-world-interaction/protocol.rs::use_target`, while
//! its client-visible reward is the character-inventory delta consumed by
//! `crates/session/src/domain_dispatch.rs::Session::apply_delta`; callers observe the applied
//! state through `crates/session/src/session_access.rs::Session::inventory` (the same accessor
//! exposed by `tools/dev-client/src/lib.rs::DevClientSession::inventory`).
use super::*;

fn captured_or_fixture() -> Result<(Vec<u8>, Vec<u8>, String), BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok((
            std::fs::read(path.join("i08-before.bin"))?,
            std::fs::read(path.join("i08-after.bin"))?,
            std::fs::read_to_string(path.join("i08-expected.txt"))?,
        ));
    }
    let before = CharacterInventory {
        main_backpack: Some(entry(1, 2_854, 1)),
        entries: vec![entry(8, 39_546, 1)],
        equipment: vec![],
    };
    let after = CharacterInventory {
        main_backpack: before.main_backpack,
        entries: vec![entry(9, 39_150, 1)],
        equipment: vec![],
    };
    Ok((
        item_view::encode_character_inventory_with_equip_drop(&before, true)
            .map_err(|error| format!("{error:?}"))?,
        item_view::encode_character_inventory_with_equip_drop(&after, true)
            .map_err(|error| format!("{error:?}"))?,
        "39150 1\n".into(),
    ))
}

fn visible(inventory: &CharacterInventory) -> String {
    inventory
        .entries
        .iter()
        .map(|item| format!("{} {}\n", item.item_definition_ref, item.count))
        .collect()
}

#[test]
fn primal_bag_inventory_payload_replays_and_invalid_delta_is_atomic() -> Result<(), BoxError> {
    block_on(async {
        let (before_payload, after_payload, expected) = captured_or_fixture()?;
        let before = item_view::decode_character_inventory_with_equip_drop(&before_payload, true)
            .map_err(|error| format!("before inventory: {error:?}"))?;
        let _after = item_view::decode_character_inventory_with_equip_drop(&after_payload, true)
            .map_err(|error| format!("after inventory: {error:?}"))?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let (valid_sent, valid_received) = tokio::sync::oneshot::channel();
        let (continue_invalid, continue_invalid_received) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[4, 6, 12],
                &[],
                item_snapshot(vec![])?,
                &[inventory_domain(3, &before)?],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    3,
                    4,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &after_payload,
                )?,
            )
            .await?;
            let _ = valid_sent.send(());
            continue_invalid_received.await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    42,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    4,
                    5,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &[0xff],
                )?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, item_admission()?).await?;
        valid_received.await?;
        session.service_liveness(Duration::from_millis(20)).await?;
        assert_eq!(
            visible(session.inventory().ok_or("inventory absent after replay")?),
            expected
        );
        let applied = session.inventory().cloned();
        let _ = continue_invalid.send(());
        assert!(
            session
                .service_liveness(Duration::from_millis(20))
                .await
                .is_err()
        );
        assert_eq!(session.inventory(), applied.as_ref());
        drop(session);
        peer.await??;
        Ok::<(), BoxError>(())
    })?
}
