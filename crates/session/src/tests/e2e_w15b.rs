//! W15b client replay: using a quest reward chest adds its reward to the CharacterInventory
//! (`Session::apply_state_delta` in `domain_dispatch.rs`, read through `Session::inventory`) and
//! advances the quest journal (`Session::read_gated_delta` for domain 16). The second use of the
//! chest changes nothing; a malformed inventory delta is refused and leaves the reward in place.
//!
//! The inventory views are session-owned views re-encoded by the row with the production encoder
//! (the C09/I11 practice); the quest payloads are the raw bytes the client received.
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::{encode_state_delta, item_view};

const SELECTED: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    quest_log::CAPABILITY_QUEST_LOG_V1,
];

struct Capture {
    inventory_before: Vec<u8>,
    inventory_after: Vec<u8>,
    quest_snapshot: Vec<u8>,
    quest_before: Vec<u8>,
    quest_after: Vec<u8>,
    expected: String,
}

fn entry(handle: u64, reference: u32, count: u32) -> Result<ItemEntry, BoxError> {
    Ok(ItemEntry {
        handle: ItemHandle::new(handle).ok_or("handle")?,
        item_definition_ref: std::num::NonZeroU32::new(reference).ok_or("reference")?,
        count: std::num::NonZeroU32::new(count).ok_or("count")?,
        sub_type: 0,
    })
}

/// `reference:count` of the backpack, loose entries and equipment, sorted.
fn units(inventory: Option<&CharacterInventory>) -> String {
    let Some(inventory) = inventory else {
        return String::new();
    };
    let mut held: Vec<(u32, u32)> = inventory
        .main_backpack
        .iter()
        .chain(inventory.entries.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .map(|item| (item.item_definition_ref.get(), item.count.get()))
        .collect();
    held.sort_unstable();
    held.iter()
        .map(|(reference, count)| format!("{reference}:{count}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn list_text(log: &quest_log::QuestLog) -> String {
    match &log.view {
        Some(quest_log::QuestLogView::List(entries)) => entries
            .iter()
            .map(|entry| format!("{}:{}", entry.quest_index, u8::from(entry.completed)))
            .collect::<Vec<_>>()
            .join(";"),
        _ => "no-list".into(),
    }
}

fn fixture() -> Result<Capture, BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok(Capture {
            inventory_before: std::fs::read(path.join("w15b-inventory-before.bin"))?,
            inventory_after: std::fs::read(path.join("w15b-inventory-after.bin"))?,
            quest_snapshot: std::fs::read(path.join("w15b-quest-snapshot.bin"))?,
            quest_before: std::fs::read(path.join("w15b-quest-before.bin"))?,
            quest_after: std::fs::read(path.join("w15b-quest-after.bin"))?,
            expected: std::fs::read_to_string(path.join("w15b-expected.txt"))?,
        });
    }
    let inventory = |extra: Option<ItemEntry>| -> Result<Vec<u8>, BoxError> {
        let mut entries = vec![entry(3, 3031, 100)?];
        entries.extend(extra);
        item_view::encode_character_inventory(&CharacterInventory {
            main_backpack: Some(entry(2, 2854, 1)?),
            entries,
            ..CharacterInventory::default()
        })
        .map_err(|error| format!("{error:?}").into())
    };
    let list = |entries: &[(u32, bool)]| -> Result<Vec<u8>, BoxError> {
        quest_log::encode_quest_log(&quest_log::QuestLog {
            map_marks: Vec::new(),
            view: Some(quest_log::QuestLogView::List(
                entries
                    .iter()
                    .map(|(quest_index, completed)| quest_log::QuestListEntry {
                        quest_index: *quest_index,
                        completed: *completed,
                    })
                    .collect(),
            )),
            tracked: vec![],
        })
        .map_err(|error| format!("{error:?}").into())
    };
    Ok(Capture {
        inventory_before: inventory(None)?,
        inventory_after: inventory(Some(entry(4, 3382, 1)?))?,
        quest_snapshot: quest_log::encode_quest_log(&quest_log::QuestLog::default())
            .map_err(|error| format!("{error:?}"))?,
        quest_before: list(&[(5, false)])?,
        quest_after: list(&[(5, false), (9, false)])?,
        expected: "claim=oteryn:claim.fixture\nsnapshot_revision=2\nbefore_units=2854:1;3031:100\n\
                   after_units=2854:1;3031:100;3382:1\nlist_before=5:0\nlist_before_base=2\n\
                   list_before_new=3\nlist_before_sequence=41\nlist_after=5:0;9:0\nlist_after_base=3\n\
                   list_after_new=4\nlist_after_sequence=42\nsecond_use=NothingToUse\n"
            .into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w15b expected lacks {key}").into())
}

#[test]
fn e2e_w15b_reward_chest_claim_replay() -> Result<(), BoxError> {
    crate::tests::block_on(async {
        let capture = fixture()?;
        let expected = capture.expected.as_str();
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let (before, after) = (
            capture.inventory_before.clone(),
            capture.inventory_after.clone(),
        );
        let snapshot = capture.quest_snapshot.clone();
        let revision: u64 = value(expected, "snapshot_revision")?.parse()?;
        let (quest_before, quest_after) =
            (capture.quest_before.clone(), capture.quest_after.clone());
        let numbers = |stage: &str| -> Result<(u64, u64, u64), BoxError> {
            Ok((
                value(expected, &format!("{stage}_sequence"))?.parse()?,
                value(expected, &format!("{stage}_base"))?.parse()?,
                value(expected, &format!("{stage}_new"))?.parse()?,
            ))
        };
        let (before_seq, before_base, before_new) = numbers("list_before")?;
        let (after_seq, after_base, after_new) = numbers("list_after")?;
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                entity_snapshot()?,
                &[
                    (
                        STATE_DOMAIN_CHARACTER_INVENTORY,
                        1,
                        item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                        before,
                    ),
                    (quest_log::STATE_DOMAIN_QUEST_LOG, revision, 1, snapshot),
                ],
            )
            .await?;
            for (sequence, base, new, payload) in [
                (before_seq, before_base, before_new, quest_before),
                (after_seq, after_base, after_new, quest_after),
            ] {
                write_frame(
                    &mut server,
                    &encode_state_delta(
                        1,
                        sequence,
                        quest_log::STATE_DOMAIN_QUEST_LOG,
                        base,
                        new,
                        1,
                        &payload,
                    )?,
                )
                .await?;
            }
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    after_seq + 1,
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
                return Err(format!("W15b admission: {error:?}; peer={peer_error:?}").into());
            }
        };
        assert_eq!(units(session.inventory()), value(expected, "before_units")?);
        assert_eq!(
            session
                .gated_snapshot(quest_log::STATE_DOMAIN_QUEST_LOG)
                .map(|snapshot| snapshot.revision),
            Some(revision)
        );
        let listed = session
            .read_gated_delta(quest_log::STATE_DOMAIN_QUEST_LOG, 1)
            .await?;
        assert_eq!(
            list_text(&quest_log::decode_quest_log(&listed.payload).map_err(|e| format!("{e:?}"))?),
            value(expected, "list_before")?
        );
        let advanced = session
            .read_gated_delta(quest_log::STATE_DOMAIN_QUEST_LOG, 1)
            .await?;
        assert_eq!(advanced.base_revision, listed.new_revision);
        assert_eq!(
            list_text(
                &quest_log::decode_quest_log(&advanced.payload).map_err(|e| format!("{e:?}"))?
            ),
            value(expected, "list_after")?
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while units(session.inventory()) != value(expected, "after_units")? {
            if tokio::time::Instant::now() >= deadline {
                return Err("W15b reward delta never reached the inventory".into());
            }
            session.service_liveness(Duration::from_millis(50)).await?;
        }
        assert_ne!(
            value(expected, "before_units")?,
            value(expected, "after_units")?
        );
        assert_eq!(value(expected, "second_use")?, "NothingToUse");

        // A malformed inventory delta is refused and leaves the claimed reward in place.
        let kept = session.inventory().cloned();
        let malformed = encode_state_delta(
            1,
            after_seq + 2,
            STATE_DOMAIN_CHARACTER_INVENTORY,
            2,
            3,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            &[0xff],
        )?;
        let envelope = decode_wire_envelope(&malformed)?;
        assert!(
            session
                .apply_state_delta(after_seq + 2, envelope.payload())
                .is_err()
        );
        assert_eq!(session.inventory(), kept.as_ref());
        assert_eq!(units(session.inventory()), value(expected, "after_units")?);
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
