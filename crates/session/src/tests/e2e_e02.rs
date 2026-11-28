//! E02 client replay: NPC gold-to-platinum exchange changes the backpack coins. The owning client state is the CharacterInventory domain, decoded
//! by `SnapshotState::decode` and applied by `Session::apply_state_delta`
//! (`crates/session/src/domain_dispatch.rs`), read through `Session::inventory`.
//!
//! The live case admits from the captured server frames, then passes the original inventory
//! deltas through the session reducer. The standalone synthetic fixture checks codec negatives.
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::{encode_state_delta, item_view};

const SELECTED: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
];

struct Capture {
    before: Vec<u8>,
    after: Vec<u8>,
    /// (definition reference, count) of every carried entry, before then after.
    before_units: Vec<(u32, u32)>,
    after_units: Vec<(u32, u32)>,
}

fn units(inventory: Option<&CharacterInventory>) -> Vec<(u32, u32)> {
    inventory.map_or_else(Vec::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| (item.item_definition_ref.get(), item.count.get()))
            .collect()
    })
}

fn entry(handle: u64, reference: u32, count: u32) -> Result<ItemEntry, BoxError> {
    Ok(ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::new(handle).ok_or("handle")?,
        item_definition_ref: std::num::NonZeroU32::new(reference).ok_or("reference")?,
        count: std::num::NonZeroU32::new(count).ok_or("count")?,
        sub_type: 0,
    })
}

fn fixture() -> Result<Capture, BoxError> {
    let encode = |entries: Vec<ItemEntry>| -> Result<Vec<u8>, BoxError> {
        let inventory = CharacterInventory {
            main_backpack: Some(entry(2, 2854, 1)?),
            entries,
            ..CharacterInventory::default()
        };
        item_view::encode_character_inventory(&inventory).map_err(|e| format!("{e:?}").into())
    };
    Ok(Capture {
        before: encode(vec![entry(3, 31, 100)?])?,
        after: encode(vec![entry(4, 35, 1)?])?,
        before_units: vec![(31, 100)],
        after_units: vec![(35, 1)],
    })
}

fn coins(inventory: &CharacterInventory, refs: [u32; 3]) -> [u64; 3] {
    refs.map(|reference| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref.get() == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

fn check_transition(
    before: &CharacterInventory,
    after: &CharacterInventory,
    refs: [u32; 3],
    expected: [u64; 3],
    authorized: bool,
) -> Result<(), BoxError> {
    if before == after {
        return Ok(());
    }
    if !authorized || coins(after, refs) != expected || coins(before, refs) == expected {
        return Err("E02 inventory changed outside its exact confirmed conversion".into());
    }
    let unrelated = |inventory: &CharacterInventory| {
        let mut result = inventory.clone();
        result
            .entries
            .retain(|item| !refs.contains(&item.item_definition_ref.get()));
        result
    };
    if unrelated(before) != unrelated(after) {
        return Err("E02 replay changed unrelated inventory".into());
    }
    Ok(())
}

fn check_replay_kind(kind: MessageType) -> Result<(), BoxError> {
    match kind {
        MessageType::CommandResult | MessageType::StateDelta | MessageType::LivenessProbe => Ok(()),
        _ => Err(format!("E02 unexpected post-admission frame {kind:?}").into()),
    }
}

async fn replay_live(path: &std::path::Path) -> Result<(), BoxError> {
    let variant = std::env::var("E02_LIVE_VARIANT").unwrap_or_else(|_| "client".to_owned());
    if !matches!(variant.as_str(), "client" | "full") {
        return Err("E02 unknown capture variant".into());
    }
    let expected = std::fs::read_to_string(path.join(format!("e02-{variant}.txt")))?;
    let value = |key: &str| -> Result<&str, BoxError> {
        expected
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .ok_or_else(|| format!("E02 expected lacks {key}").into())
    };
    let session_id = value("session=")?;
    let after_sequence: u64 = value("after_sequence=")?.parse()?;
    let binding = format!("session={session_id} sequence={after_sequence}");
    if let Ok(live_binding) = std::env::var("E02_LIVE_CAPTURE_BINDING")
        && binding != live_binding
    {
        return Err("E02 replay binding differs from live verdict".into());
    }
    let refs: [u32; 3] = value("refs=")?
        .split(',')
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "E02 expected three coin refs")?;
    let frames = std::fs::read(path.join(format!("e02-{variant}.frames")))?;
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        server.write_all(&frames).await?;
        server.shutdown().await?;
        tokio::io::copy(&mut server, &mut tokio::io::sink()).await?;
        Ok::<(), std::io::Error>(())
    });
    let mut session =
        Session::admit(client, admission_with(CLIENT_SUPPORTED_CAPABILITIES)?).await?;
    let live_session: String = session
        .join_snapshot()
        .game_session_id
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(live_session, session_id.replace('-', ""));

    let mut first = true;
    let mut conversions = 0;
    for checkpoint in expected
        .lines()
        .filter_map(|line| line.strip_prefix("checkpoint="))
    {
        let [end, confirmation, gold, platinum, crystal]: [u64; 5] = checkpoint
            .split(',')
            .map(str::parse)
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| "E02 malformed checkpoint")?;
        let target = [gold, platinum, crystal];
        let prior = session
            .inventory()
            .cloned()
            .ok_or("E02 inventory missing")?;
        if first {
            assert_eq!(end, session.last_server_sequence());
            assert_eq!(confirmation, 0);
            assert_eq!(
                coins(&prior, refs),
                if variant == "client" {
                    [100, 0, 0]
                } else {
                    [100, 0, 1]
                }
            );
            first = false;
        } else {
            assert!(end > session.last_server_sequence());
        }
        let mut authorized = false;
        while session.last_server_sequence() < end {
            // read_post_admission would silently install a replacement snapshot.
            let frame = bounded(
                DEADLINE,
                "E02 captured envelope",
                read_frame(&mut session.stream),
            )
            .await?;
            let envelope = decode_wire_envelope(&frame)?;
            envelope.validate(Direction::ServerToClient, true)?;
            session.check_generation(&envelope)?;
            let kind = envelope.message_type();
            check_replay_kind(kind)?;
            if kind == MessageType::LivenessProbe {
                continue;
            }
            let sequence = session.next_sequence(&envelope)?;
            if kind == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if confirmation != 0 && result.command_id == confirmation {
                    assert!(!authorized);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    authorized = true;
                }
            } else if decode_state_delta(envelope.payload())?.domain_id
                == STATE_DOMAIN_CHARACTER_INVENTORY
            {
                let previous = session
                    .inventory()
                    .cloned()
                    .ok_or("E02 inventory missing")?;
                session.apply_state_delta(sequence, envelope.payload())?;
                let current = session
                    .inventory()
                    .ok_or("E02 inventory missing after delta")?;
                check_transition(&previous, current, refs, target, authorized)?;
            }
        }
        assert_eq!(session.last_server_sequence(), end);
        assert_eq!(authorized, confirmation != 0);
        assert_eq!(
            coins(
                session
                    .inventory()
                    .ok_or("E02 inventory missing at checkpoint")?,
                refs
            ),
            target
        );
        conversions += usize::from(coins(&prior, refs) != target);
    }
    assert_eq!(conversions, if variant == "client" { 1 } else { 9 });
    assert_eq!(session.last_server_sequence(), after_sequence);
    assert_eq!(
        coins(
            session.inventory().ok_or("E02 final inventory missing")?,
            refs
        ),
        if variant == "client" {
            [0, 1, 0]
        } else {
            [0, 1, 1]
        },
    );
    let mut trailing = Vec::new();
    tokio::time::timeout(
        Duration::from_secs(2),
        session.stream.read_to_end(&mut trailing),
    )
    .await??;
    assert!(
        trailing.is_empty(),
        "frames remain after the live sequence boundary"
    );
    drop(session);
    peer.await??;
    Ok(())
}

#[test]
fn e02_rejects_premature_collateral_and_snapshot_substitution() -> Result<(), BoxError> {
    let refs = [31, 35, 37];
    let before = CharacterInventory {
        entries: vec![entry(1, 31, 100)?, entry(2, 99, 1)?],
        ..CharacterInventory::default()
    };
    let after = CharacterInventory {
        entries: vec![entry(3, 35, 1)?, entry(2, 99, 1)?],
        ..CharacterInventory::default()
    };
    assert!(check_transition(&before, &after, refs, [0, 1, 0], true).is_ok());
    assert!(check_transition(&before, &after, refs, [0, 1, 0], false).is_err());
    let mut collateral = after.clone();
    collateral.entries.pop();
    assert!(check_transition(&before, &collateral, refs, [0, 1, 0], true).is_err());
    assert!(check_replay_kind(MessageType::SnapshotBegin).is_err());
    assert!(check_replay_kind(MessageType::SnapshotChunk).is_err());
    Ok(())
}

#[test]
fn e02_coin_inventory_replay() -> Result<(), BoxError> {
    crate::tests::block_on(async {
        if let Some(path) = std::env::var_os("E02_CAPTURE_DIR") {
            return replay_live(std::path::Path::new(&path)).await;
        }
        if std::env::var_os("E02_LIVE_CAPTURE_BINDING").is_some() {
            return Err("E02 live replay requires a capture directory".into());
        }
        let capture = fixture()?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let before = capture.before.clone();
        let after = capture.after.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                entity_snapshot()?,
                &[(
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    1,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                    before,
                )],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    1,
                    2,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    &after,
                )?,
            )
            .await?;
            std::future::pending::<()>().await;
            Ok::<(), BoxError>(())
        });
        let mut session = match Session::admit(client, admission_with(SELECTED)?).await {
            Ok(session) => session,
            Err(error) => {
                peer.abort();
                let peer_error = peer.await;
                return Err(format!("E02 admission: {error:?}; peer={peer_error:?}").into());
            }
        };
        assert_eq!(units(session.inventory()), capture.before_units);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while units(session.inventory()) != capture.after_units {
            if tokio::time::Instant::now() >= deadline {
                return Err("E02 coin delta never reached the inventory".into());
            }
            session.service_liveness(Duration::from_millis(50)).await?;
        }
        assert_eq!(units(session.inventory()), capture.after_units);
        assert_ne!(capture.before_units, capture.after_units);

        // A malformed inventory delta is rejected and leaves the converted coins unchanged.
        let kept = session.inventory().cloned();
        let malformed = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            2,
            3,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &[0xff],
        )?;
        let envelope = decode_wire_envelope(&malformed)?;
        assert!(session.apply_state_delta(42, envelope.payload()).is_err());
        assert_eq!(session.inventory(), kept.as_ref());
        peer.abort();
        assert!(peer.await.is_err());
        Ok(())
    })?
}

fn entity_snapshot() -> Result<(u32, Vec<u8>), BoxError> {
    let position = ActorPosition {
        x: 0,
        y: 0,
        floor: 0,
    };
    let own_identity = [0xa0; 16];
    let own = WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: own_identity,
            generation: 1,
        },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let payload = world_spatial_entities::encode_world_spatial_entities_snapshot_with_item_handles(
        &world_spatial_entities::WorldSpatialEntitiesSnapshot {
            content_generation: [0x11; 32],
            actor_position: position,
            own_identity,
            entities: vec![own],
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok((
        world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        payload,
    ))
}
