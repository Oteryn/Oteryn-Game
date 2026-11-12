//! Replay coverage for SCENARIO_MATRIX I14.
//!
//! The production consumer under test is `Session::apply_state_delta` in
//! `crates/session/src/domain_dispatch.rs`; its Character Inventory arm calls
//! `decode_character_inventory_with_equip_drop`, updates `Session::inventory`, and emits
//! `SessionEvent::Inventory`. The client-visible accessor is `Session::inventory` in
//! `crates/session/src/session_access.rs`.
use super::*;

#[test]
fn i14_real_consumable_inventory_payload_replays_and_invalid_delta_is_atomic()
-> Result<(), BoxError> {
    block_on(async {
        let (before, after, metadata) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("i14-inventory-before.bin"))?,
                std::fs::read(path.join("i14-inventory-after.bin"))?,
                std::fs::read_to_string(path.join("i14-expected.txt"))?,
            )
        } else {
            let before = CharacterInventory {
                main_backpack: Some(entry(70, 1988, 1)),
                entries: vec![entry(71, 3577, 2)],
                ..CharacterInventory::default()
            };
            let after = CharacterInventory {
                main_backpack: Some(entry(70, 1988, 1)),
                entries: vec![entry(71, 3577, 1)],
                ..CharacterInventory::default()
            };
            (
                item_view::encode_character_inventory_with_equip_drop(&before, true)
                    .map_err(|error| format!("before fixture: {error:?}"))?,
                item_view::encode_character_inventory_with_equip_drop(&after, true)
                    .map_err(|error| format!("after fixture: {error:?}"))?,
                "41 1 2 1\n".into(),
            )
        };
        let numbers = metadata
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        let &[sequence, base, new, expected_meat] = numbers.as_slice() else {
            return Err("invalid I14 capture metadata".into());
        };
        let selected: &'static [u32] = &[4, 6, 12];
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let before_for_peer = before.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                selected,
                selected,
                &[],
                item_snapshot(vec![])?,
                &[(9, base, 1, before_for_peer)],
            )
            .await?;
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<(), BoxError>(())
        });
        let mut admission = item_admission()?;
        admission.supported_capabilities = selected;
        let mut session = Session::admit(client, admission).await?;
        let frame = encode_state_delta(1, sequence, 9, base, new, 1, &after)?;
        let payload = decode_wire_envelope(&frame)?.payload().to_vec();
        assert!(matches!(
            session.apply_state_delta(sequence, &payload)?,
            SessionEvent::Inventory(_)
        ));
        let meat = session
            .inventory()
            .ok_or("inventory not stored")?
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref.get() == 3577)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        assert_eq!(meat, expected_meat);

        let retained = session.inventory().cloned();
        let invalid = encode_state_delta(1, sequence + 1, 9, new, new + 1, 1, &[0xff])?;
        assert!(
            session
                .apply_state_delta(sequence + 1, decode_wire_envelope(&invalid)?.payload())
                .is_err()
        );
        assert_eq!(session.inventory(), retained.as_ref());
        drop(session);
        peer.await??;
        Ok::<_, BoxError>(())
    })?
}
