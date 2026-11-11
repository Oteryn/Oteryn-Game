//! I13 replay owner: `Session::apply_state_delta` in `domain_dispatch.rs`
//! decodes `item_view::CharacterInventory` and replaces `Session::inventory`;
//! `Session::inventory` in `session_access.rs` exposes that client-visible state.
use super::*;

fn replay_peer(
    revision: u64,
    payload: Vec<u8>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            CLIENT_SUPPORTED_CAPABILITIES,
            &[4, 6, 12],
            &[],
            item_snapshot(vec![])?,
            &[(
                item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                revision,
                item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                payload,
            )],
        )
        .await?;
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok(())
    });
    (client, peer)
}

fn visible(inventory: &CharacterInventory) -> String {
    let mut rows = inventory
        .entries
        .iter()
        .map(|entry| (entry.item_definition_ref.get(), entry.count.get()))
        .collect::<Vec<_>>();
    rows.sort_unstable();
    rows.into_iter()
        .map(|(item, count)| format!("{item} {count}\n"))
        .collect()
}

#[test]
fn random_reward_inventory_payload_replays() -> Result<(), BoxError> {
    block_on(async {
        let (before_payload, after_payload, revisions, expected) = if let Some(path) =
            std::env::var_os("E2E_ALL_CAPTURE_DIR").map(std::path::PathBuf::from)
        {
            (
                std::fs::read(path.join("i13-before.bin"))?,
                std::fs::read(path.join("i13-after.bin"))?,
                std::fs::read_to_string(path.join("i13-revisions.txt"))?,
                std::fs::read_to_string(path.join("i13-expected.txt"))?,
            )
        } else {
            let before = CharacterInventory {
                main_backpack: Some(entry(1, 2854, 1)),
                entries: vec![entry(2, 43895, 1)],
                equipment: vec![],
            };
            let after = CharacterInventory {
                main_backpack: before.main_backpack,
                entries: vec![entry(3, 43864, 1)],
                equipment: vec![],
            };
            (
                item_view::encode_character_inventory_with_equip_drop(&before, true)
                    .map_err(|error| format!("{error:?}"))?,
                item_view::encode_character_inventory_with_equip_drop(&after, true)
                    .map_err(|error| format!("{error:?}"))?,
                "3 3 4 41".into(),
                visible(&after),
            )
        };
        let numbers = revisions
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        let [
            snapshot_revision,
            base_revision,
            new_revision,
            server_sequence,
        ] = numbers.as_slice()
        else {
            return Err("invalid I13 capture revision metadata".into());
        };
        let (client, peer) = replay_peer(*snapshot_revision, before_payload);
        let mut admission = item_admission()?;
        admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
        let mut session = Session::admit(client, admission).await?;
        let frame = encode_state_delta(
            1,
            *server_sequence,
            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
            *base_revision,
            *new_revision,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &after_payload,
        )?;
        let envelope = decode_wire_envelope(&frame)?;
        session.apply_state_delta(*server_sequence, envelope.payload())?;
        assert_eq!(
            visible(session.inventory().ok_or("I13 inventory absent")?),
            expected
        );

        let unchanged = session.inventory().cloned();
        let mut invalid = after_payload;
        invalid.truncate(invalid.len().saturating_sub(1));
        let frame = encode_state_delta(
            1,
            server_sequence + 1,
            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
            *new_revision,
            new_revision + 1,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &invalid,
        )?;
        let envelope = decode_wire_envelope(&frame)?;
        assert!(
            session
                .apply_state_delta(server_sequence + 1, envelope.payload())
                .is_err()
        );
        assert_eq!(session.inventory(), unchanged.as_ref());
        drop(session);
        peer.await??;
        Ok(())
    })?
}
