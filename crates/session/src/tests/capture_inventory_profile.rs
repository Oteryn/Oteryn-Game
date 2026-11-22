//! Replay the inventory flags recorded by the same real capture; only synthetic fixtures
//! may use a legacy profile. Never infer a profile by trying permissive decoders.
use super::*;

pub(crate) fn selected(scenario: &str, synthetic: &[u32]) -> Result<Vec<u32>, BoxError> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(synthetic.to_vec());
    };
    let text = std::fs::read_to_string(
        std::path::Path::new(&root).join(format!("{scenario}-inventory-profile.txt")),
    )?;
    parse(&text, synthetic)
}

fn parse(text: &str, base: &[u32]) -> Result<Vec<u32>, BoxError> {
    let (mut equip_drop, mut appearances) = (None, None);
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("inventory profile line lacks '='")?;
        let target = match key {
            "equip_drop" => &mut equip_drop,
            "appearances" => &mut appearances,
            _ => return Err("unknown inventory profile flag".into()),
        };
        if target.is_some() {
            return Err("duplicate inventory profile flag".into());
        }
        *target = Some(match value {
            "0" => false,
            "1" => true,
            _ => return Err("invalid inventory profile flag".into()),
        });
    }
    let equip_drop = equip_drop.ok_or("missing inventory equipment profile")?;
    let appearances = appearances.ok_or("missing inventory appearance profile")?;
    if !base.contains(&item_view::CAPABILITY_ITEM_VIEW_MOVE_V1) {
        return Err("inventory replay lacks item-view selection".into());
    }
    let mut selected: Vec<u32> = base
        .iter()
        .copied()
        .filter(|id| {
            *id != item_view::CAPABILITY_ITEM_EQUIP_DROP_V1
                && *id != item_view::CAPABILITY_ITEM_APPEARANCE_V1
        })
        .collect();
    if equip_drop {
        selected.push(item_view::CAPABILITY_ITEM_EQUIP_DROP_V1);
    }
    if appearances {
        selected.push(item_view::CAPABILITY_ITEM_APPEARANCE_V1);
    }
    selected.sort_unstable();
    selected.dedup();
    Ok(selected)
}

pub(crate) fn decode(payload: &[u8], selected: &[u32]) -> Result<CharacterInventory, BoxError> {
    item_view::decode_character_inventory_with_appearances(
        payload,
        selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
        selected.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1),
    )
    .map_err(|error| format!("captured inventory: {error:?}").into())
}

pub(super) fn peer(
    selected: Vec<u32>,
    snapshot: Vec<u8>,
    script: Vec<Step>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            CLIENT_SUPPORTED_CAPABILITIES,
            &selected,
            &[],
            item_snapshot(vec![])?,
            &[(
                STATE_DOMAIN_CHARACTER_INVENTORY,
                3,
                item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                snapshot,
            )],
        )
        .await?;
        for step in script {
            play_step(&mut server, step).await?;
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok(())
    });
    (client, peer)
}

#[test]
fn capture_profile_requires_both_flags_and_preserves_exact_selection() -> Result<(), BoxError> {
    for (text, expected) in [
        ("equip_drop=0\nappearances=0\n", vec![4, 6]),
        ("equip_drop=1\nappearances=0\n", vec![4, 6, 12]),
        ("equip_drop=0\nappearances=1\n", vec![4, 6, 499]),
        ("equip_drop=1\nappearances=1\n", vec![4, 6, 12, 499]),
    ] {
        assert_eq!(parse(text, &[4, 6, 12, 499])?, expected);
    }
    for invalid in [
        "",
        "equip_drop=1\n",
        "appearances=1\n",
        "equip_drop=1\nequip_drop=0\nappearances=1\n",
        "equip_drop=1\nappearances=1\nunknown=1\n",
        "equip_drop=true\nappearances=1\n",
    ] {
        assert!(parse(invalid, &[4, 6]).is_err(), "accepted {invalid:?}");
    }
    Ok(())
}

#[test]
fn negotiated_capture_inventory_replays_equipment_and_appearances() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .name("capture-inventory-replay".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(replay_equipment_and_appearances)?
        .join()
        .map_err(|_| -> BoxError { "inventory replay thread panicked".into() })?
}

fn replay_equipment_and_appearances() -> Result<(), BoxError> {
    block_on(async {
        let selected = parse("equip_drop=1\nappearances=1\n", &[4, 6])?;
        let mut equipped = entry(3, 3364, 1);
        equipped.appearance_id = 733;
        let mut before = CharacterInventory {
            main_backpack: Some(entry(1, 2854, 1)),
            entries: vec![entry(2, 3031, 3)],
            equipment: vec![item_view::EquippedItem {
                slot: item_view::EquipmentSlot::Head,
                item: equipped,
            }],
        };
        before.entries[0].appearance_id = 732;
        let mut after = before.clone();
        after.entries[0].count = std::num::NonZeroU32::new(4).ok_or("count")?;
        let encode = |value: &CharacterInventory| {
            item_view::encode_character_inventory_with_appearances(value, true, true)
                .map_err(|error| format!("{error:?}"))
        };
        let payload = encode(&after)?;
        let (client, peer) = peer(selected.clone(), encode(&before)?, vec![]);
        let mut session = Session::admit(client, item_admission()?).await?;
        assert_eq!(session.inventory(), Some(&before));
        let frame = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            3,
            4,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &payload,
        )?;
        session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
        assert_eq!(session.inventory(), Some(&after));
        assert_eq!(decode(&payload, &selected)?, after);
        assert!(decode(&payload, &[4, 6]).is_err());
        let mut malformed = payload;
        malformed.push(0xff);
        let frame = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            4,
            5,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &malformed,
        )?;
        assert!(
            session
                .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                .is_err()
        );
        assert_eq!(session.inventory(), Some(&after));
        drop(session);
        peer.await??;
        Ok(())
    })?
}
