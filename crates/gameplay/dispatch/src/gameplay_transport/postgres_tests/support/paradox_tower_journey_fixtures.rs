//! One character through Paradox's native contacts, offerings, climb, riddle and reward.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
    l6_paradox_native_fixtures as tower, l6_paradox_story_fixtures as story,
    l6_paradox_upper_fixtures as upper, quest_world_callbacks_fixtures as wire,
};
use crate::{
    content::native_gameplay::NativeGameplayState,
    durability::character_authority::ReconciledCharacterAuthority,
    foundation::{MessageType, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements},
    gameplay_transport::connection::{
        ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
};
use oteryn_gameplay_quests::quest::{
    loader, log,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_wire_envelope, encode_client_command,
    item_view::{
        self, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
        WorldTilePosition,
    },
    npc_service::NpcIntentDisposition,
    quest_log::{QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query},
    world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
        encode_item_use_intent, encode_use_intent,
    },
    world_spatial_entities::EntityRef,
};
use serde::Deserialize;
use tokio::io::{AsyncWriteExt, DuplexStream};

pub(super) const NAME: &str = "qengine-rewards-questpc4-paradox-full-journey";
pub(super) const FORFEIT: &str = "qengine-rewards-questpc4-paradox-full-journey-forfeit";
const QUEST: &str = "oteryn:quest.the_paradox_tower_quest";
const RIDDLE: &str = "oteryn:quest-progress/quest/u7_24/the_paradox_tower/";
const TALONS: &str = "oteryn:quest-callback/paradox-tower/talons";
const SKULLS: &str = "oteryn:quest-callback/paradox-tower/native-skulls";
const KEY: Position = pos(32497, 31887, -7);
const ALTAR: Position = pos(32566, 31957, -1);
const CHEST: Position = pos(32479, 31900, -1);
const FOOD_DOOR: Position = pos(32479, 31903, -4);

const CONTACTS: &[&str] = &[
    "oteryn:npc.a_prisoner",
    "oteryn:npc.lubo",
    "oteryn:npc.oldrak",
    "oteryn:npc.padreia",
    "oteryn:npc.riddler",
    "oteryn:npc.zoltan",
];

const fn pos(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

/// Load the same generated NPC declarations consumed by the World compiler.
pub(super) fn placements() -> TestResult<NpcPlacementTable> {
    #[derive(Deserialize)]
    struct Shard {
        records: Vec<Record>,
    }
    #[derive(Deserialize)]
    struct Record {
        declaration: Declaration,
    }
    #[derive(Deserialize)]
    struct Declaration {
        npc: String,
        cell: Position,
        direction: String,
    }

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/world/npc-placements");
    let mut shards = Vec::new();
    for entry in std::fs::read_dir(&directory)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("placements-") && name.ends_with(".json"))
        {
            shards.push(path);
        }
    }
    shards.sort();
    let mut records = Vec::new();
    for path in shards {
        let shard: Shard = serde_json::from_slice(&std::fs::read(path)?)?;
        records.extend(shard.records);
    }
    let mut npcs = Vec::new();
    for key in CONTACTS {
        let declaration = records
            .iter()
            .map(|r| &r.declaration)
            .find(|r| r.npc == *key)
            .ok_or_else(|| format!("missing generated Paradox placement for {key}"))?;
        assert_eq!(declaration.direction, "north");
        npcs.push(NpcPlacements {
            key: declaration.npc.clone(),
            placements: vec![NpcPlacement {
                x: declaration.cell.x,
                y: declaration.cell.y,
                floor: -declaration.cell.floor,
                direction: MovementFacing::North,
            }],
        });
    }
    Ok(NpcPlacementTable { npcs })
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{
        ExpectedTile, NativeCrop, native_scene_with_appearances,
    };
    let region = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z01-x126-y124.b3"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z02-x126-y124.b3"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z03-x126-y124.b3"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z04-x126-y124.b3"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z05-x126-y124.b3"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z06-x126-y124.b3"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x126-y124.b3"
        ))
        .as_slice(),
    ];
    let mut crops = story::crops(false)?;
    let start = story::start();
    let expected = |p: Position, ids| ExpectedTile {
        position: (p.x, p.y),
        ids,
        counted_entry: None,
    };
    let crop = |z: u8, bounds, expected| NativeCrop {
        bytes: region[usize::from(z) - 1],
        region: (z, 126, 124),
        bounds,
        start: (start.x, start.y),
        expected,
    };
    for z in 1..=7 {
        let expected = match z {
            1 => vec![expected(CHEST, vec![409, 2321, 2472])],
            2 => vec![expected(pos(32479, 31901, -2), vec![409, 2576])],
            4 => vec![expected(FOOD_DOOR, vec![409, 1631])],
            _ => Vec::new(),
        };
        crops.push(crop(
            z,
            (32475, 31899, 32484, if z == 7 { 31924 } else { 31907 }),
            expected,
        ));
    }
    crops.push(NativeCrop {
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z01-x127-y124.b3"
        )),
        region: (1, 127, 124),
        bounds: (32562, 31957, 32570, 31960),
        start: (start.x, start.y),
        expected: vec![expected(ALTAR, vec![10145])],
    });
    crops.push(crop(
        7,
        (32496, 31887, 32499, 31889),
        vec![expected(KEY, vec![103, 4585, 3634])],
    ));
    native_scene_with_appearances(
        native,
        crops,
        &[],
        &[
            430, 431, 1632, 1633, 1791, 1947, 1948, 2772, 2773, 3695, 9125, 9126,
        ],
    )
}

async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    p: Position,
) -> TestResult {
    super::l6_native_signs_fixtures::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        p,
    )
    .await
}

async fn track(h: &Harness, key: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(key)
            .fetch_optional(&h.pool)
            .await?,
    )
}

pub(super) async fn carried(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<Vec<(String, i64)>> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    carried_for_character(h, fence.character_id, owner.world_id).await
}

async fn carried_for_character(
    h: &Harness,
    character_id: oteryn_gameplay_domain::domain::CharacterId,
    world_id: crate::foundation::WorldId,
) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as(
        "SELECT i.definition_production_key, sum(i.quantity)::bigint
         FROM game_quest_callback_carried(encode($1,'hex')::uuid, encode($2,'hex')::uuid) c
         JOIN game_item_instances i ON i.item_instance_id=c.item
         GROUP BY i.definition_production_key ORDER BY i.definition_production_key",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?)
}

async fn completed(h: &Harness) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_states
         WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?)
}

fn object_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    p: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("Paradox native map")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, ordinal)
        .ok_or("native placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_wire(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("use clock")?;
    let payload = encode_use_intent(&object_target(owner, p, ordinal)?).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result = wire::wire(owner, admitted, command, 2, payload).await?;
    decode_use_result(&result).map_err(debug)
}

async fn send(
    client: &mut DuplexStream,
    admitted: AdmittedSession,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(frame.len().try_into()?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

/// Read the published inventory handle and drop it through the real MOVE wire route.
async fn drop_offering(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    offering: &Offering,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native items")?;
    let definition = native
        .item_index()
        .ok_or("item index")?
        .definition_ref("Item", &offering.item, "definition-r1")
        .ok_or("offering definition")?;
    let initial = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let target = offering.position;
    let reader = tokio::spawn(async move {
        let result: TestResult = async {
            let mut handle = None;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::SnapshotChunk {
                    let (_, domains) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                            .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                            handle = item_view::decode_character_inventory_with_equip_drop(
                                domain.payload,
                                true,
                            )
                            .map_err(debug)?
                            .entries
                            .iter()
                            .find(|item| item.item_definition_ref == definition)
                            .map(|item| item.handle);
                        }
                    }
                } else if envelope.message_type() == MessageType::SnapshotCommit {
                    break;
                }
            }
            let payload = item_view::encode_item_move_intent_for(
                &ItemMoveIntent {
                    source: handle.ok_or("carried offering handle")?,
                    destination: ItemMoveDestination::Ground(WorldTilePosition {
                        x: target.x.into(),
                        y: target.y.into(),
                        floor: -i16::from(target.floor),
                    }),
                },
                ItemMoveSelection {
                    equip_drop: true,
                    world_interaction: true,
                    ..Default::default()
                },
            )
            .map_err(debug)?;
            send(&mut client, initial, 9, &payload).await?;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    assert_eq!(result.command_id, initial.continuity.next_command_id);
                    assert_eq!(
                        item_view::decode_item_move_result(result.payload).map_err(debug)?,
                        ItemMoveOutcome::Moved
                    );
                    break;
                }
            }
            client.shutdown().await?;
            Ok(())
        }
        .await;
        result.map_err(|e| e.to_string())
    });
    let end = serve_admitted(&mut server, initial, owner, IDLE_LIVENESS).await;
    drop(server);
    reader.await?.map_err(std::io::Error::other)?;
    let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
        return Err(format!("offering transport ended {end:?}").into());
    };
    *admitted = next;
    Ok(())
}

async fn offer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    offering: &Offering,
    number: u64,
) -> TestResult {
    let before = carried(owner, admitted, h).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("offering fence")?;
    // Fixture acquisition supplies common Skull/food inputs only. Ground placement is MOVE.
    let mut supply = offering.clone();
    supply.position = tower::at(owner, admitted).await?;
    wire::input_item(owner, fence.item_fence(), &supply, number, true).await?;
    drop_offering(owner, admitted, offering).await?;
    assert_eq!(
        carried(owner, admitted, h).await?,
        before,
        "MOVE placed the supplied item"
    );
    assert!(!completed(h).await?);
    Ok(())
}

async fn consumed(h: &Harness, callback: &Callback, count: i64) -> TestResult {
    let actual: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1",
    ).bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(actual, count, "consumptions for {}", callback.key);
    Ok(())
}

async fn skulls(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    relocate(owner, admitted, pos(32563, 31958, -1)).await?;
    let before = carried(owner, admitted, h).await?;
    for (idx, offering) in callback.offerings.iter().enumerate() {
        tower::walk(owner, admitted, pos(offering.position.x, 31958, -1)).await?;
        offer(owner, admitted, h, offering, 200 + u64::try_from(idx)?).await?;
        assert_eq!(
            track(h, "oteryn:quest-progress/first_paradox_access").await?,
            None
        );
    }
    tower::walk(owner, admitted, pos(32566, 31958, -1)).await?;
    tower::step(owner, admitted, 0, -1).await?;
    assert_eq!(tower::at(owner, admitted).await?, pos(32479, 31923, -7));
    assert_eq!(
        track(h, "oteryn:quest-progress/first_paradox_access").await?,
        Some(1)
    );
    consumed(h, callback, 4).await?;
    assert_eq!(carried(owner, admitted, h).await?, before);
    Ok(())
}

async fn ghoul(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let before = carried(owner, admitted, h).await?;
    tower::walk(owner, admitted, pos(32480, 31905, -5)).await?;
    tower::walk(owner, admitted, pos(32480, 31904, -5)).await?;
    let clock = owner.freeze_owner_clock_for_test();
    assert_eq!(
        use_wire(owner, admitted, pos(32481, 31904, -5), 1).await?,
        UseDisposition::Committed
    );
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("native ghoul deadline")?;
    tower::set_clock(owner, due)?;
    owner.drain_quest_encounter_deadlines().await;
    let ladder = pos(32478, 31904, -5);
    assert!(
        tower::ladder(owner, ladder)?.is_some(),
        "server ghoul opened the ladder"
    );
    drop(clock);
    tower::walk(owner, admitted, pos(32479, 31904, -5)).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("ladder clock")?;
    super::l6_world_puzzles_fixtures::use_added_ladder(
        owner,
        admitted,
        crate::map::overlay::TilePos {
            x: ladder.x,
            y: ladder.y,
            floor: ladder.floor,
        },
    )
    .await?;
    assert_eq!(tower::at(owner, admitted).await?, pos(32478, 31905, -4));
    assert_eq!(
        track(h, "oteryn:quest-progress/l6/paradox/floor-four-arrived").await?,
        Some(1)
    );
    assert_eq!(carried(owner, admitted, h).await?, before);
    Ok(())
}

async fn foods(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    z: i8,
) -> TestResult {
    let callback = upper::callback(z)?;
    let counter = &callback.storage.first().ok_or("food counter")?.track;
    assert_eq!(track(h, counter).await?, None);
    if z == 4 {
        open_food_room(owner, admitted, h).await?;
        tower::walk(owner, admitted, pos(32476, 31901, -4)).await?;
    } else {
        tower::walk(owner, admitted, pos(32477, 31905, -3)).await?;
        tower::walk(owner, admitted, pos(32477, 31904, -3)).await?;
    }
    for (idx, offering) in callback.offerings.iter().enumerate() {
        if z == 4 {
            tower::walk(owner, admitted, pos(offering.position.x, 31901, -4)).await?;
        } else if offering.position.x == 32479 {
            // The lever occupies the middle tile. Approach the east altar from the south.
            tower::walk(owner, admitted, pos(32477, 31905, -3)).await?;
            tower::walk(owner, admitted, pos(32479, 31905, -3)).await?;
            tower::walk(owner, admitted, pos(32479, 31904, -3)).await?;
        }
        offer(
            owner,
            admitted,
            h,
            offering,
            240 + u64::try_from(z)? * 10 + u64::try_from(idx)?,
        )
        .await?;
        assert_eq!(
            track(h, counter).await?,
            None,
            "offering does not solve the floor until USE"
        );
    }
    if z == 4 {
        tower::walk(owner, admitted, pos(32479, 31901, -4)).await?;
        tower::walk(owner, admitted, pos(32479, 31904, -4)).await?;
        tower::walk(owner, admitted, pos(32478, 31905, -4)).await?;
    } else {
        tower::walk(owner, admitted, pos(32479, 31905, -3)).await?;
        tower::walk(owner, admitted, pos(32477, 31905, -3)).await?;
        tower::walk(owner, admitted, pos(32477, 31904, -3)).await?;
    }
    let before = carried(owner, admitted, h).await?;
    let lever = callback.selector.position.ok_or("food lever")?;
    assert_eq!(
        use_wire(owner, admitted, lever, 1).await?,
        UseDisposition::Committed
    );
    assert_eq!(track(h, counter).await?, Some(0));
    consumed(h, &callback, i64::try_from(callback.offerings.len())?).await?;
    assert_eq!(carried(owner, admitted, h).await?, before);
    upper::ascend(owner, admitted, &callback, z).await
}

async fn open_food_room(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let inventory = carried(owner, admitted, h).await?;
    tower::walk(owner, admitted, pos(32478, 31904, -4)).await?;
    tower::walk(owner, admitted, pos(32479, 31904, -4)).await?;
    assert_eq!(
        use_wire(owner, admitted, FOOD_DOOR, 1).await?,
        UseDisposition::Locked
    );
    let reference = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i2970", "definition-r1"))
        .ok_or("Paradox key definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(object_target(owner, FOOD_DOOR, 1)?)),
        use_with_position: None,
    })
    .map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("key use clock")?;
    let (next, disposition, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    tower::walk(owner, admitted, pos(32479, 31901, -4)).await?;
    assert_eq!(
        carried(owner, admitted, h).await?,
        inventory,
        "door key is retained"
    );
    assert!(!completed(h).await?);
    Ok(())
}

async fn riddle(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    tower::walk(owner, admitted, pos(32480, 31905, -2)).await?;
    tower::walk(owner, admitted, pos(32480, 31902, -2)).await?;
    let actor = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| runtime.npc_key(n.actor) == Some("oteryn:npc.riddler"))
            .map(|n| EntityRef {
                identity: n.actor.placement_identity(),
                generation: n.generation,
            })
            .ok_or("placed Riddler")?
    };
    let items = carried(owner, admitted, h).await?;
    let math = format!("{RIDDLE}mathemagics");
    let colour = format!("{RIDDLE}favorite_colour");
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    let world = owner.world_id;
    let words = [
        "hi", "test", "yes", "goshnar", "demonbunny", "tha'kull", "yes", "breath", "silence",
        "old", "yes", "green", "none", "1",
    ];
    let replies = story::conversation_with_checkpoint(owner, admitted, actor, &words, async |count| {
        if matches!(count, 4 | 8 | 12 | 13) {
            assert_eq!(track(h, &math).await?, Some(1));
            assert_eq!(carried_for_character(h, fence.character_id, world).await?, items);
            assert!(!completed(h).await?);
        }
        if count == 13 {
            assert_eq!(track(h, &colour).await?, Some(1));
        }
        Ok(())
    })
    .await?;
    assert_eq!(replies.len(), words.len());
    assert!(
        replies
            .iter()
            .all(|reply| reply.disposition == NpcIntentDisposition::Ok)
    );
    assert_eq!(tower::at(owner, admitted).await?, pos(32478, 31905, -1));
    assert_eq!(track(h, &math).await?, Some(2));
    assert_eq!(track(h, &colour).await?, Some(2));
    assert_eq!(carried(owner, admitted, h).await?, items);
    assert!(!completed(h).await?);
    Ok(())
}

async fn journal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    index: u32,
) -> TestResult {
    let payload =
        encode_quest_log_query(&QuestLogQuery::Quest { quest_index: index }).map_err(debug)?;
    let command = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type: 22,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut framed = u32::try_from(command.len())?.to_be_bytes().to_vec();
    framed.extend(command);
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(owner, admitted, &framed).await?;
    let mut frames = output.as_slice();
    while !frames.is_empty() {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut frames).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 16
                && let Some(QuestLogView::Line(line)) =
                    decode_quest_log(delta.payload).map_err(debug)?.view
            {
                assert_eq!(line.quest_index, index);
                assert!(line.completed);
                for name in [
                    "The Feared Hugo",
                    "Favorite colour: Green",
                    "The Secret of Mathemagics",
                ] {
                    assert!(
                        line.missions
                            .iter()
                            .any(|mission| mission.name == name && mission.done),
                        "{name}"
                    );
                }
                return Ok(());
            }
        }
    }
    Err("no terminal Paradox journal on LOG 16/22".into())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    forfeit: bool,
) -> TestResult {
    let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load_for(loaded.catalogue())?;
    let skull_callback = callbacks
        .iter()
        .find(|callback| callback.key == SKULLS)
        .ok_or("native Paradox skull binding")?;
    assert!(
        callbacks
            .iter()
            .any(|callback| callback.key == TALONS && callback.completes)
    );
    let log = log::loader::load_embedded_quest_log(&loaded)?;
    let journal_index = log
        .content
        .active()
        .quests()
        .iter()
        .position(|quest| quest.key == QUEST)
        .ok_or("Paradox journal")?;
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(log.content)
            .ok_or("Paradox journal bounds")?,
    );
    let caps = [
        3, 4, 6, 11, 12, 15, 16, 18,
        oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
    ];
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &caps,
        )
        .ok_or("Paradox wire capabilities")?;
    assert!(!completed(h).await?);
    let initial = carried(owner, admitted, h).await?;
    story::exercise(owner, admitted, h, authority, NAME).await?;
    assert_eq!(
        track(h, &format!("{RIDDLE}the_feared_hugo")).await?,
        Some(4)
    );
    assert_eq!(track(h, &format!("{RIDDLE}mathemagics")).await?, Some(1));
    assert_eq!(carried(owner, admitted, h).await?, initial);

    relocate(owner, admitted, pos(32497, 31888, -7)).await?;
    assert_eq!(
        use_wire(owner, admitted, KEY, 2).await?,
        UseDisposition::Committed
    );
    let key_number: i32 = sqlx::query_scalar("SELECT key_number FROM game_item_keys")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(key_number, 3899);
    assert!(
        carried(owner, admitted, h)
            .await?
            .contains(&("oteryn:item.tibia.i2970".to_owned(), 1))
    );
    assert!(!completed(h).await?);

    skulls(owner, admitted, h, skull_callback).await?;
    // The crop harness positions the character at the tower's exterior approach
    // after verifying the actual skull teleport. Each interior ascent is physical.
    relocate(owner, admitted, tower::start("-first")).await?;
    tower::first(owner, admitted, h).await?;
    tower::walk(owner, admitted, pos(32476, 31901, -6)).await?;
    tower::walk(owner, admitted, pos(32476, 31905, -6)).await?;
    tower::walk(owner, admitted, tower::start("-second")).await?;
    tower::second(owner, admitted, h).await?;
    ghoul(owner, admitted, h).await?;
    foods(owner, admitted, h, 4).await?;
    foods(owner, admitted, h, 3).await?;
    assert_eq!(
        track(h, "oteryn:quest-progress/l6/paradox/climb-complete").await?,
        Some(1)
    );
    riddle(owner, admitted, h).await?;

    // The middle approach to the chests forfeits Talons. Approach from the
    // west before choosing whether to step on the Talon forfeiture tile.
    tower::walk(owner, admitted, pos(32476, 31905, -1)).await?;
    tower::walk(owner, admitted, pos(32476, 31901, -1)).await?;
    tower::walk(owner, admitted, pos(32478, 31901, -1)).await?;
    let marker = format!("{RIDDLE}reward/talon");
    assert!(matches!(track(h, &marker).await?, None | Some(-1 | 0)));
    if forfeit {
        let before = carried(owner, admitted, h).await?;
        tower::walk(owner, admitted, pos(32478, 31902, -1)).await?;
        assert_eq!(track(h, &marker).await?, Some(1), "AID 50310 forfeits talons");
        tower::walk(owner, admitted, pos(32478, 31901, -1)).await?;
        owner.clock_origin = owner.clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("reward clock")?;
        assert_eq!(use_wire(owner, admitted, CHEST, 2).await?, UseDisposition::Rejected);
        assert_eq!(carried(owner, admitted, h).await?, before);
        assert!(!completed(h).await?);
        return Ok(());
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("reward clock")?;
    let payload = encode_use_intent(&object_target(owner, CHEST, 2)?).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let response = wire::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        decode_use_result(&response).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(completed(h).await?);
    assert_eq!(
        track(h, "oteryn:quest-progress/authored/paradox-tower/talons").await?,
        Some(1)
    );
    let rewards = carried(owner, admitted, h).await?;
    assert!(rewards.contains(&("oteryn:item.tibia.i3034".to_owned(), 32)));
    // Each wire call reconnects the closed duplex with its retained session continuity.
    assert_eq!(
        wire::wire(owner, admitted, command, 2, payload).await?,
        response
    );
    assert_eq!(
        use_wire(owner, admitted, CHEST, 2).await?,
        UseDisposition::Rejected
    );
    assert_eq!(carried(owner, admitted, h).await?, rewards);
    assert!(completed(h).await?);
    journal(owner, *admitted, journal_index.try_into()?).await
}
