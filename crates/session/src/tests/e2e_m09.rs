//! Replay coverage for SCENARIO_MATRIX M09 fishing and skinning/gathering yields.
//!
//! The owning client seam is `Session::apply_delta` in `domain_dispatch.rs`, which decodes
//! domain-9 payloads and replaces `Session::inventory`; `Session::inventory` in
//! `session_access.rs` exposes that client-visible state. The search also covered
//! `crates/client-ui/src/features/0004-item-view-move/InventoryPanel::apply` and
//! `crates/client-runtime/src/features/0011-world-interaction/state.rs`; those consume the
//! already-decoded session projection rather than the server payload.
use super::*;

#[test]
fn e2e_m09_replays_yield_inventory_and_rejects_invalid_payload() -> Result<(), BoxError> {
    block_on(async {
        let (payload, expected) = if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let root = std::path::Path::new(&root);
            (
                std::fs::read(root.join("m09-inventory.bin"))?,
                std::fs::read_to_string(root.join("m09-expected.txt"))?,
            )
        } else {
            let inventory = CharacterInventory {
                main_backpack: Some(entry(1, 2854, 1)),
                entries: vec![entry(2, 3492, 39), entry(3, 3578, 1)],
                equipment: vec![item_view::EquippedItem {
                    slot: item_view::EquipmentSlot::Head,
                    item: entry(99, 3351, 1),
                }],
            };
            (
                item_view::encode_character_inventory_with_equip_drop(&inventory, true)
                    .map_err(|error| format!("{error:?}"))?,
                "3492 39\n3578 1\n".into(),
            )
        };
        let decoded = item_view::decode_character_inventory_with_equip_drop(&payload, true)
            .map_err(|error| format!("{error:?}"))?;
        let before = CharacterInventory::default();
        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![inventory_domain(3, &before)?],
            vec![
                ItemStep::Send(Step::ReadCommand),
                ItemStep::Send(use_result(41, 7, UseDisposition::Committed)?),
                ItemStep::Send(Step::Send(encode_state_delta(
                    1,
                    42,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    3,
                    4,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &payload,
                )?)),
            ],
        );
        let mut session = Session::admit(client, item_admission()?).await?;
        let outcome = session.use_object(b"water", 1).await?;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        session
            .service_liveness(Duration::from_millis(200))
            .await
            .ok();
        assert_eq!(session.inventory(), Some(&decoded));
        let visible = session
            .inventory()
            .into_iter()
            .flat_map(|inventory| inventory.entries.iter())
            .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
            .collect::<String>();
        assert_eq!(visible, expected);
        drop(session);
        peer.await??;

        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![inventory_domain(3, &decoded)?],
            vec![
                ItemStep::Send(Step::ReadCommand),
                ItemStep::Send(use_result(41, 7, UseDisposition::NothingToUse)?),
                ItemStep::Send(Step::Send(encode_state_delta(
                    1,
                    42,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    3,
                    4,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &[0xff],
                )?)),
            ],
        );
        let mut session = Session::admit(client, item_admission()?).await?;
        let outcome = session.use_object(b"water", 1).await?;
        assert_eq!(outcome.disposition, UseDisposition::NothingToUse);
        assert!(
            session
                .service_liveness(Duration::from_millis(200))
                .await
                .is_err()
        );
        assert_eq!(session.inventory(), Some(&decoded));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

fn item_peer(
    snapshot: (u32, Vec<u8>),
    raw: Vec<(u32, u64, u32, Vec<u8>)>,
    script: Vec<ItemStep>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            CLIENT_SUPPORTED_CAPABILITIES,
            &[4, 6, 12],
            &[],
            snapshot,
            &raw,
        )
        .await?;
        for step in script {
            match step {
                ItemStep::Send(step) => play_step(&mut server, step).await?,
                ItemStep::Expect(command_type, payload) => {
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_type, command_type);
                    assert_eq!(command.payload, payload.as_slice());
                }
            }
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    (client, peer)
}

fn inventory_domain(
    revision: u64,
    view: &CharacterInventory,
) -> Result<(u32, u64, u32, Vec<u8>), BoxError> {
    Ok((
        STATE_DOMAIN_CHARACTER_INVENTORY,
        revision,
        item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
        item_view::encode_character_inventory_with_equip_drop(view, true)
            .map_err(|error| format!("{error:?}"))?,
    ))
}
