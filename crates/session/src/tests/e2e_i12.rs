//! I12 replay of the original inventory admission payload and received delta envelope.
use super::*;

#[test]
fn pick_refining_replays_real_inventory_payload() -> Result<(), BoxError> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = std::path::Path::new(&root);
    let metadata = std::fs::read_to_string(root.join("i12-revisions.txt"))?;
    let fields = metadata.split_ascii_whitespace().collect::<Vec<_>>();
    let [
        session,
        baseline,
        snapshot_revision,
        base,
        new,
        sequence,
        selected,
    ] = fields.as_slice()
    else {
        return Err("invalid I12 capture metadata".into());
    };
    assert_eq!(*session, std::env::var("OTERYN_I12_SESSION")?);
    let selected = selected
        .split(',')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let equip = selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1);
    let appearance = selected.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1);
    let admission = std::fs::read(root.join("i12-snapshot.bin"))?;
    let before =
        item_view::decode_character_inventory_with_appearances(&admission, equip, appearance)
            .map_err(|error| format!("{error:?}"))?;
    let frame = std::fs::read(root.join("i12-inventory.bin"))?;
    let envelope = decode_wire_envelope(&frame)?;
    envelope.validate(Direction::ServerToClient, true)?;
    assert_eq!(envelope.message_type(), MessageType::StateDelta);
    assert_eq!(envelope.server_sequence(), sequence.parse::<u64>()?);
    assert!(envelope.server_sequence() > baseline.parse::<u64>()?);
    let delta = decode_state_delta(envelope.payload())?;
    assert_eq!(delta.domain_id, item_view::STATE_DOMAIN_CHARACTER_INVENTORY);
    assert_eq!(delta.base_revision, base.parse::<u64>()?);
    assert_eq!(delta.new_revision, new.parse::<u64>()?);
    check_delta(
        &delta,
        item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
        snapshot_revision.parse()?,
    )?;
    let after =
        item_view::decode_character_inventory_with_appearances(delta.payload, equip, appearance)
            .map_err(|error| format!("{error:?}"))?;
    assert_ne!(before, after);
    let visible = after
        .entries
        .iter()
        .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
        .collect::<String>();
    assert_eq!(
        visible,
        std::fs::read_to_string(root.join("i12-expected.txt"))?
    );
    Ok(())
}

#[test]
fn idle_inventory_delta_retains_original_envelope() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let frame = encode_state_delta(
            1,
            41,
            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
            3,
            4,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &inventory_domain(3, &CharacterInventory::default())?.3,
        )?;
        let sent = frame.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[4, 6],
                &[],
                item_snapshot(vec![])?,
                &[inventory_domain(3, &CharacterInventory::default())?],
            )
            .await?;
            write_frame(&mut server, &sent).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, item_admission()?).await?;
        let before_receive = std::time::SystemTime::now();
        session.service_liveness(Duration::from_millis(30)).await?;
        let (received, received_at) = session.take_inventory_frame().ok_or("missing frame")?;
        assert_eq!(received, frame);
        assert!(received_at >= before_receive && received_at <= std::time::SystemTime::now());
        peer.await??;
        Ok::<(), BoxError>(())
    })?
}
