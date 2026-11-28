//! I12 replay of the captured admission inventory, committed result and original delta envelope.
use super::*;

#[test]
#[ignore = "requires fresh I12 live capture"]
fn pick_refining_replays_real_inventory_payload() -> Result<(), BoxError> {
    block_on(replay_refining_capture())?
}

async fn replay_refining_capture() -> Result<(), BoxError> {
    let root = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing fresh I12 capture directory")?,
    );
    let metadata = std::fs::read_to_string(root.join("i12-revisions.txt"))?;
    let fields = metadata.split_ascii_whitespace().collect::<Vec<_>>();
    let [
        session_id,
        baseline,
        snapshot_revision,
        base,
        new,
        sequence,
        selected,
        pick,
        stone,
        rewards,
    ] = fields.as_slice()
    else {
        return Err("invalid I12 capture metadata".into());
    };
    assert_eq!(*session_id, std::env::var("OTERYN_I12_SESSION")?);
    let selected = selected
        .split(',')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let mut replay_caps = vec![4, 6];
    for capability in [
        item_view::CAPABILITY_ITEM_EQUIP_DROP_V1,
        item_view::CAPABILITY_ITEM_APPEARANCE_V1,
    ] {
        if selected.contains(&capability) {
            replay_caps.push(capability);
        }
    }
    let result = std::fs::read(root.join("i12-result.bin"))?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result)
            .map_err(|error| format!("{error:?}"))?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
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
    let admission = std::fs::read(root.join("i12-snapshot.bin"))?;
    let revision = snapshot_revision.parse::<u64>()?;
    let (client, mut peer) = tokio::io::duplex(256 * 1024);
    // The peer supplies test admission transport. The inventory domain bytes and
    // the inventory delta envelope come from the live session.
    let join = tokio::spawn(async move {
        join_peer_with(
            &mut peer,
            CLIENT_SUPPORTED_CAPABILITIES,
            &replay_caps,
            &[],
            item_snapshot(vec![])?,
            &[(
                item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
                revision,
                item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                admission,
            )],
        )
        .await
    });
    let mut session = Session::admit(client, item_admission()?).await?;
    join.await??;
    let before = session
        .inventory()
        .ok_or("missing admitted inventory")?
        .clone();
    session.apply_state_delta(envelope.server_sequence(), envelope.payload())?;
    let after = session
        .inventory()
        .ok_or("missing applied inventory")?
        .clone();
    let pick = pick.parse::<u32>()?;
    let stone = stone.parse::<u32>()?;
    let rewards = rewards
        .split(',')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    assert!(refining_inventory_matches(
        &before, &after, pick, stone, &rewards
    ));

    let mut corrupt = delta.payload.to_vec();
    corrupt.push(0xff);
    let invalid = encode_state_delta(
        envelope.connection_generation(),
        envelope.server_sequence() + 1,
        delta.domain_id,
        delta.new_revision,
        delta.new_revision + 1,
        delta.delta_type,
        &corrupt,
    )?;
    assert!(
        session
            .apply_state_delta(
                envelope.server_sequence() + 1,
                decode_wire_envelope(&invalid)?.payload(),
            )
            .is_err()
    );
    assert_eq!(session.inventory(), Some(&after));
    Ok(())
}

fn refining_inventory_matches(
    before: &CharacterInventory,
    after: &CharacterInventory,
    pick: u32,
    stone: u32,
    rewards: &[u32],
) -> bool {
    let count = |inventory: &CharacterInventory| {
        let mut totals = std::collections::BTreeMap::<u32, u64>::new();
        for item in &inventory.entries {
            *totals.entry(item.item_definition_ref.get()).or_default() +=
                u64::from(item.count.get());
        }
        totals
    };
    let mut expected = count(before);
    let pick_entry = before
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref.get() == pick);
    if expected.get(&pick) != Some(&1)
        || expected.remove(&stone) != Some(1)
        || !pick_entry.is_some_and(|entry| after.entries.contains(entry))
    {
        return false;
    }
    let actual = count(after);
    rewards.iter().any(|reward| {
        let mut candidate = expected.clone();
        *candidate.entry(*reward).or_default() += 1;
        candidate == actual
    })
}

#[test]
fn refining_rejects_an_identical_replacement_pick_and_wrong_reward() {
    let before = CharacterInventory {
        entries: vec![entry(2, 3456, 1), entry(3, 10310, 1)],
        ..CharacterInventory::default()
    };
    let mut after = CharacterInventory {
        entries: vec![entry(2, 3456, 1), entry(4, 3031, 1)],
        ..CharacterInventory::default()
    };
    let matches = |after: &CharacterInventory| {
        refining_inventory_matches(&before, after, 3456, 10310, &[3031])
    };
    assert!(matches(&after));
    after.entries[0] = entry(5, 3456, 1);
    assert!(!matches(&after));
    after.entries[0] = entry(2, 3456, 1);
    after.entries[1] = entry(4, 9999, 1);
    assert!(!matches(&after));
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
