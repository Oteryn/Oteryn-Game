//! Replay for SCENARIO_MATRIX C20b.
//!
//! The captured inventory snapshot is decoded and installed by
//! `snapshot_state.rs::SnapshotState::install_domains`; live replacements are applied by
//! `domain_dispatch.rs::Session::apply_state_delta`; callers observe it through
//! `session_access.rs::Session::inventory` (and `tools/dev-client/src/lib.rs::inventory`).

use super::*;

#[test]
fn c20b_inventory_loss_snapshot_replays_and_invalid_payload_is_atomic() -> Result<(), BoxError> {
    block_on(async {
        let (payload, expected) = fixture()?;
        let expected = parse_expected(&expected)?;
        let inventory = item_view::decode_character_inventory(&payload)
            .map_err(|error| format!("C20b inventory decode: {error:?}"))?;
        assert_eq!(usize::from(inventory.main_backpack.is_some()), expected.1);
        assert_eq!(inventory.entries.len(), expected.2);
        assert_eq!(inventory.equipment.len(), expected.3);

        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![(
                STATE_DOMAIN_CHARACTER_INVENTORY,
                expected.0,
                item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                payload.clone(),
            )],
            vec![],
        );
        let session = Session::admit(client, item_admission()?).await?;
        assert_eq!(session.inventory(), Some(&inventory));
        drop(session);
        peer.await??;

        let mut invalid = payload;
        invalid.push(0xff);
        let invalid_delta = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            expected.0,
            expected.0 + 1,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &invalid,
        )?;
        let (client, peer) = item_peer(
            item_snapshot(vec![])?,
            vec![(
                STATE_DOMAIN_CHARACTER_INVENTORY,
                expected.0,
                item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                item_view::encode_character_inventory(&inventory)
                    .map_err(|error| format!("C20b before-invalid encode: {error:?}"))?,
            )],
            vec![ItemStep::Send(Step::Send(invalid_delta))],
        );
        let mut session = Session::admit(client, item_admission()?).await?;
        let before = session.inventory().cloned();
        assert!(matches!(
            session.service_liveness(Duration::from_millis(50)).await,
            Err(SessionError::ItemView(_))
        ));
        assert_eq!(session.inventory(), before.as_ref());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

fn fixture() -> Result<(Vec<u8>, String), BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        return Ok((
            std::fs::read(directory.join("c20b-inventory.bin"))?,
            std::fs::read_to_string(directory.join("c20b-expected.txt"))?,
        ));
    }
    let inventory = CharacterInventory::default();
    Ok((
        item_view::encode_character_inventory(&inventory)
            .map_err(|error| format!("C20b fixture encode: {error:?}"))?,
        "revision=7\nbackpack=0\nentries=0\nequipment=0\n".into(),
    ))
}

fn parse_expected(text: &str) -> Result<(u64, usize, usize, usize), BoxError> {
    let mut values = [0_u64; 4];
    for (index, key) in ["revision", "backpack", "entries", "equipment"]
        .into_iter()
        .enumerate()
    {
        let line = text
            .lines()
            .find(|line| line.starts_with(key))
            .ok_or_else(|| format!("C20b expected missing {key}"))?;
        values[index] = line
            .strip_prefix(&format!("{key}="))
            .ok_or_else(|| format!("C20b expected malformed {key}"))?
            .parse()?;
    }
    Ok((
        values[0],
        usize::try_from(values[1])?,
        usize::try_from(values[2])?,
        usize::try_from(values[3])?,
    ))
}
