//! Replay coverage for SCENARIO_MATRIX I12. The owning client state is
//! `crates/session/src/snapshot_state.rs::SnapshotState::from_join` for the initial inventory and
//! `crates/session/src/domain_dispatch.rs::Session::apply_state_delta` for refining deltas;
//! `crates/session/src/session_access.rs::Session::inventory` is the client-visible projection.
use super::*;

#[test]
fn pick_refining_replays_real_inventory_payload() -> Result<(), BoxError> {
    block_on(async {
        let (snapshot_payload, delta_payload, revisions, expected) = fixture()?;
        let revisions = revisions
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        let [
            snapshot_revision,
            base_revision,
            new_revision,
            server_sequence,
        ] = revisions.as_slice()
        else {
            return Err("invalid I12 revision metadata".into());
        };
        let snapshot_revision = *snapshot_revision;
        let base_revision = *base_revision;
        let new_revision = *new_revision;
        let server_sequence = *server_sequence;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let snapshot_payload_for_peer = snapshot_payload.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[4, 6],
                &[],
                item_snapshot(vec![])?,
                &[(
                    item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                    snapshot_revision,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                    snapshot_payload_for_peer,
                )],
            )
            .await
        });
        let mut session = Session::admit(client, item_admission()?).await?;
        let frame = encode_state_delta(
            1,
            server_sequence,
            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
            base_revision,
            new_revision,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &delta_payload,
        )?;
        session.apply_state_delta(server_sequence, decode_wire_envelope(&frame)?.payload())?;
        assert_eq!(visible(session.inventory()), expected);

        let before = session.inventory().cloned();
        let mut invalid = delta_payload;
        invalid.push(0xff);
        let bad = encode_state_delta(
            1,
            server_sequence + 1,
            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
            new_revision,
            new_revision + 1,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &invalid,
        )?;
        assert!(
            session
                .apply_state_delta(server_sequence + 1, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        assert_eq!(session.inventory(), before.as_ref());
        drop(session);
        peer.await??;
        Ok::<(), BoxError>(())
    })?
}

fn fixture() -> Result<(Vec<u8>, Vec<u8>, String, String), BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("i12-snapshot.bin"))?,
            std::fs::read(root.join("i12-inventory.bin"))?,
            std::fs::read_to_string(root.join("i12-revisions.txt"))?,
            std::fs::read_to_string(root.join("i12-expected.txt"))?,
        ));
    }
    let before = CharacterInventory {
        main_backpack: Some(entry(1, 2854, 1)),
        entries: vec![entry(2, 10310, 1)],
        ..CharacterInventory::default()
    };
    let after = CharacterInventory {
        main_backpack: Some(entry(1, 2854, 1)),
        entries: vec![entry(3, 3031, 1)],
        ..CharacterInventory::default()
    };
    Ok((
        item_view::encode_character_inventory(&before).map_err(|error| format!("{error:?}"))?,
        item_view::encode_character_inventory(&after).map_err(|error| format!("{error:?}"))?,
        "3 3 4 41".into(),
        visible(Some(&after)),
    ))
}

fn visible(inventory: Option<&CharacterInventory>) -> String {
    inventory
        .into_iter()
        .flat_map(|inventory| &inventory.entries)
        .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
        .collect()
}
