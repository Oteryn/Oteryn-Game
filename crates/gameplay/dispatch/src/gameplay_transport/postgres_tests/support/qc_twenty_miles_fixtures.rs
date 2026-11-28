use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::BTreeSet;
use std::num::NonZeroU64;

pub(super) const CHAIN: &str = "qc-twenty-miles";
pub(super) const GUARDIAN: &str = "qc-twenty-miles-wandering-guardian";
pub(super) const TRADE: &str = "qc-twenty-miles-open-trade";
pub(super) const GOLD: &str = "qc-twenty-miles-merged-gold";
pub(super) const SPLIT_GOLD: &str = "qc-twenty-miles-split-gold";
pub(super) const EARLIER_SPLIT_GOLD: &str = "qc-twenty-miles-earlier-split-gold";
pub(super) const RESUME: &str = "qc-twenty-miles-resume";
pub(super) const BORROWED_REPORT: &str = "qc-twenty-miles-borrowed-report";
pub(super) const BORROWED_SONGS: &str = "qc-twenty-miles-borrowed-songs";
pub(super) const START: Position = Position {
    x: 33550,
    y: 31865,
    floor: -7,
};
const QUEST: &str = "oteryn:quest.twenty_miles_beneath_the_sea_quest";
const STAGE: &str = "oteryn:quest-progress/qc/twenty_miles/stage";
const LEGACY_HASH: &str = "955724dfc1c9b0392d23eaa22a2b771f739e367e9b7a07f2a7d75e524bfe3f3c";
const PRE_REVIEW_HASH: &str = "e02b51e20a74c18d4f7bb4705331d1545f0180f0709b669bcdc474e26560cb3c";
const NPCS: [(&str, Position); 5] = [
    (
        "oteryn:npc.mr_brandon",
        Position {
            x: 33548,
            y: 31864,
            floor: -7,
        },
    ),
    (
        "oteryn:npc.guardian_of_the_deep_seas",
        Position {
            x: 33547,
            y: 31632,
            floor: -14,
        },
    ),
    (
        "oteryn:npc.yeo",
        Position {
            x: 33660,
            y: 31989,
            floor: -7,
        },
    ),
    (
        "oteryn:npc.guardian_of_the_deep_seas",
        Position { x: 33553, y: 31624, floor: -14 },
    ),
    (
        "oteryn:npc.yeo",
        Position { x: 33554, y: 31624, floor: -14 },
    ),
];

pub(super) fn matches(name: &str) -> bool {
    matches!(name, CHAIN | RESUME | GUARDIAN | TRADE | GOLD | SPLIT_GOLD | EARLIER_SPLIT_GOLD | BORROWED_REPORT | BORROWED_SONGS)
}

fn source(callback: &Callback) -> Option<Position> {
    callback
        .selector
        .position
        .or_else(|| callback.selector.native.as_ref().map(|s| s.position))
}

pub(super) fn pinned_placements() -> TestResult<NpcPlacementTable> {
    use oteryn_world_bundle::{bundle, npc::Direction};
    let (bytes, pin) = super::qc_pinned_world::artifact()?;
    let callbacks: Vec<_> = world_callbacks::load()?
        .into_iter()
        .filter(|c| c.quest == QUEST)
        .collect();
    assert_eq!(callbacks.len(), 74);
    let mut matched = BTreeSet::new();
    let mut landings = BTreeSet::new();
    let world = bundle::visit(&bytes, |manifest, sector| {
        for callback in &callbacks {
            if let Some(at) = callback.teleport
                && sector.floor == at.floor
                && sector.sx == at.x / 32
                && sector.sy == at.y / 32
                && sector
                    .tiles
                    .iter()
                    .any(|tile| tile.x == at.x && tile.y == at.y)
            {
                landings.insert(&callback.key);
            }
            let Some(at) = source(callback) else { continue };
            if sector.floor != at.floor || sector.sx != at.x / 32 || sector.sy != at.y / 32 {
                continue;
            }
            let selector = &callback.selector;
            if sector.tiles.iter().any(|tile| {
                tile.x == at.x
                    && tile.y == at.y
                    && tile
                        .items
                        .iter()
                        .filter(|item| item.depth == 0)
                        .enumerate()
                        .any(|(ord, item)| {
                            crate::map::palette_appearance(
                                &manifest.palette[item.palette as usize].key,
                            ) == selector.appearance
                                && selector
                                    .native
                                    .as_ref()
                                    .is_none_or(|s| usize::from(s.ordinal) == ord)
                                && selector
                                    .unique
                                    .is_none_or(|uid| item.attrs.unique == Some(uid))
                                && selector
                                    .action
                                    .is_none_or(|aid| item.attrs.action == Some(aid))
                        })
            }) {
                matched.insert(&callback.key);
            }
        }
        Ok(())
    })
    .map_err(debug)?;
    let digest = world
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(pin["digest"], digest);
    for callback in &callbacks {
        assert!(
            matched.contains(&callback.key),
            "missing source {}",
            callback.key
        );
        assert!(
            callback.teleport.is_none() || landings.contains(&callback.key),
            "missing landing {}",
            callback.key
        );
    }
    let mut table = NpcPlacementTable { npcs: Vec::new() };
    for (key, at) in NPCS.into_iter().take(3) {
        let npc = world
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or_else(|| format!("missing shipped NPC {key}"))?;
        let route_cell = (at.x, at.y, at.floor);
        let mut expected_cells = vec![route_cell];
        if key == "oteryn:npc.mr_brandon" {
            // The regional population also admits Brandon's second Gray Beach contact.
            expected_cells.push((33552, 31871, -7));
        }
        let mut cells: Vec<_> = npc.placements.iter().map(|p| (p.x, p.y, p.floor)).collect();
        cells.sort_unstable();
        assert_eq!(cells, expected_cells, "shipped placements for {key}");
        let p = npc
            .placements
            .iter()
            .find(|p| (p.x, p.y, p.floor) == route_cell)
            .ok_or_else(|| format!("missing Twenty Miles route placement for {key}"))?;
        let direction = match p.direction {
            Direction::North => MovementFacing::North,
            Direction::South => MovementFacing::South,
            Direction::East => MovementFacing::East,
            Direction::West => MovementFacing::West,
        };
        table.npcs.push(NpcPlacements {
            key: npc.key.clone(),
            placements: vec![NpcPlacement {
                x: p.x,
                y: p.y,
                floor: p.floor,
                direction,
            }],
        });
    }
    table.npcs.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(table)
}

async fn stand_near(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
) -> TestResult<(crate::foundation::MovementLocalPosition, StepDirection)> {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let mut runtime = owner.runtime.lock().await;
    let current = runtime.read_actor_position(actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let local = super::quest_world_callbacks_fixtures::local(at);
    let directions = [
        StepDirection::North,
        StepDirection::South,
        StepDirection::West,
        StepDirection::East,
        StepDirection::NorthWest,
        StepDirection::NorthEast,
        StepDirection::SouthWest,
        StepDirection::SouthEast,
    ];
    let current_position = current.position();
    if current_position.floor == local.floor
        && let Some(direction) = directions.iter().copied().find(|step| {
            let (dx, dy) = step.delta();
            current_position.x + dx == local.x && current_position.y + dy == local.y
        })
    {
        return Ok((current_position, direction));
    }
    for step in directions {
        let (dx, dy) = step.delta();
        let adjacent = crate::foundation::MovementLocalPosition {
            x: local.x - dx,
            y: local.y - dy,
            ..local
        };
        if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            current,
            adjacent,
            false,
            &doors,
        ) {
            runtime.commit_source_step(proof).map_err(debug)?;
            return Ok((adjacent, step));
        }
    }
    Err(format!("no adjacent walkable cell at {at:?}").into())
}

async fn enter(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    name: &str,
    open: bool,
) -> TestResult {
    let cb = callback(name)?;
    let (origin, step) = stand_near(owner, admitted, source(&cb).ok_or("portal source")?).await?;
    super::questpc7_ferumbras_fixtures::step(owner, admitted, step)
        .await
        .map_err(|error| format!("{name} STEP failed: {error}"))?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let actual = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let expected = if open {
        super::quest_world_callbacks_fixtures::local(cb.teleport.ok_or("portal destination")?)
    } else {
        origin
    };
    assert_eq!(actual, expected, "{name}");
    Ok(())
}

fn callback(name: &str) -> TestResult<Callback> {
    super::quest_world_callbacks_fixtures::callback(&format!("qc-twenty-miles-{name}"))
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    name: &str,
    item: Option<u32>,
    expected: UseDisposition,
) -> TestResult {
    let cb = callback(name)?;
    let at = source(&cb).ok_or("object position")?;
    stand_near(owner, admitted, at).await?;
    let map = owner.bundle_map().ok_or("map")?;
    let base = map.overlay.base();
    let tile = base.tile(at.x, at.y, at.floor).ok_or("object tile")?;
    let ord = super::qc_pinned_world::ordinal(
        tile,
        cb.selector.appearance.ok_or("appearance")?,
        cb.selector.unique,
    )?;
    let key =
        oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ord).ok_or("placement")?;
    let base_id = tile.ids().iter().zip(tile.depths())
        .filter(|(_, depth)| **depth == 0).nth(usize::from(ord))
        .map(|(id, _)| *id).ok_or("base palette entry")?;
    let mut placement = base.digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let original_revision = map.facts.object_revision(key);
    let target = WorldObjectTarget {
        placement,
        expected_revision: original_revision,
    };
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let actual = if let Some(item) = item {
        let definition = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.item_index())
            .and_then(|index| {
                index.definition_ref(
                    "Item",
                    &format!("oteryn:item.tibia.i{item}"),
                    "definition-r1",
                )
            })
            .ok_or("selected item definition")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::MIN),
            use_with: None,
            use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(target)),
        })
        .map_err(debug)?;
        let (next, result, _) = super::rust_lottery_fixtures::exchange(
            owner,
            *admitted,
            command,
            &payload,
            Some(definition),
            None,
        )
        .await
        .map_err(|error| format!("{name} selected USE failed: {error}"))?;
        *admitted = next;
        result
    } else {
        let payload = encode_use_intent(&target).map_err(debug)?;
        let bytes =
            super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload)
                .await
                .map_err(|error| format!("{name} USE failed: {error}"))?;
        decode_use_result(&bytes).map_err(debug)?
    };
    assert_eq!(actual, expected, "{name} with {item:?}");
    if actual == UseDisposition::Committed && name.starts_with("broken-altar-") {
        let map = owner.bundle_map().ok_or("map")?;
        let (pos, ord) = crate::map::view::placement_of(key).ok_or("repaired altar placement")?;
        assert!(!map.facts.base_hidden(key), "{name} visibility");
        let restored = map.facts.base_entry(pos, ord, base_id).ok_or("repaired altar facts")?;
        assert_eq!(restored.appearance_id, 21796, "{name} projection");
        assert_eq!(map.facts.object_revision(key), original_revision + 1, "{name} revision");
    }
    Ok(())
}

async fn only_live_item(h: &Harness, key: &str) -> TestResult<[u8; 16]> {
    let rows: Vec<sqlx::types::Uuid> = sqlx::query_scalar(
        "SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1
         AND definition_production_key=$1 ORDER BY item_instance_id LIMIT 2",
    )
    .bind(key)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(rows.len(), 1, "one live issuance for {key}");
    rows.first().map(|id| *id.as_bytes()).ok_or_else(|| "item missing".into())
}

async fn move_recovery_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: [u8; 16],
    command_id: u64,
    drop: bool,
    count: Option<u32>,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::WorldTilePosition;
    let at = Position { x: 33709, y: 31804, floor: -8 };
    let (origin, _) = stand_near(owner, admitted, at).await?;
    let fence = owner.current_quest_fence(admitted.game_session_id)
        .await.map_err(|()| "fence")?.ok_or("fence")?;
    let result = owner.move_loop_item(
        admitted.runtime_actor.ok_or("actor")?,
        UseCommand { game_session_id: admitted.game_session_id, command_id, item_fence: Some(fence.item_fence()) },
        LoopMove {
            item: ItemKey::Instance(item),
            count,
            target: if drop {
                LoopTarget::Ground(WorldTilePosition { x: origin.x, y: origin.y, floor: -origin.floor })
            } else {
                LoopTarget::MainBackpack
            },
            slot: None,
        },
    ).await;
    assert_eq!(result, LoopMoveResult::Committed, "recovery item movement with drop={drop}");
    Ok(())
}

async fn expire_dropped_item(h: &Harness, item: [u8; 16]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    // Supply the post-reset fixture state. The drop above goes through Item Loop.
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let item = sqlx::types::Uuid::from_bytes(item);
    let ground = sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1")
        .bind(item).execute(&mut *tx).await?;
    assert_eq!(ground.rows_affected(), 1);
    let retired = sqlx::query("UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=uuidv7()
        WHERE item_instance_id=$1 AND lifecycle=1")
        .bind(item).execute(&mut *tx).await?;
    assert_eq!(retired.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn prove_use_commit_releases_guards(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let pause = std::sync::Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    let states = owner.states.clone();
    let runtime = owner.runtime.clone();
    assert!(states.quest_callbacks.before_commit.lock().map_err(|_| "commit pause")?
        .replace(pause.clone()).is_none());
    let use_pile = use_object(owner, admitted, "pile-1", None, UseDisposition::Committed);
    super::quest_world_callbacks_fixtures::use_with_commit_probe(use_pile, &pause, || {
        let free = runtime.try_lock().is_ok() && states.spell_states.try_lock().is_ok();
        assert!(free, "the wire USE is in the commit window with both Channel guards free");
    }).await
}

async fn pickup_pearl(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;

    let at = Position {
        x: 33414,
        y: 31620,
        floor: -14,
    };
    stand_near(owner, admitted, at).await?;
    let base = owner.bundle_map().ok_or("map")?.overlay.base();
    let tile = base.tile(at.x, at.y, at.floor).ok_or("pearl tile")?;
    let ordinal = super::qc_pinned_world::ordinal(tile, 21747, None)?;
    let placement_key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ordinal)
        .ok_or("pearl placement")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .move_loop_item(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 22_001,
                item_fence: Some(fence.item_fence()),
            },
            LoopMove {
                item: ItemKey::MapBase {
                    bundle_digest: base.digest(),
                    placement_key,
                    reset_epoch: 0,
                },
                count: None,
                target: LoopTarget::MainBackpack,
                slot: None,
            },
        )
        .await;
    assert_eq!(result, LoopMoveResult::Committed);
    Ok(())
}

async fn track(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
    )
    .bind(key)
    .fetch_optional(&h.pool)
    .await?
    .unwrap_or(-1))
}

async fn stage(h: &Harness, expected: i64) -> TestResult {
    assert_eq!(track(h, STAGE).await?, expected);
    Ok(())
}

async fn awaken_fire_feathered(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness) -> TestResult {
    let map = owner.bundle_map().ok_or("Fire-Feathered map")?;
    let pin = owner.runtime.lock().await.content_pin().clone();
    let world = owner.world_id.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world).bind(map.overlay.base().digest().as_slice())
        .bind(crate::map::overlay::map_revision(map.overlay.base()))
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world).execute(&h.pool).await?;
    h.root.world_changes(owner.world_id, 0).await?.ok_or("Fire-Feathered day")?;
    let awake = sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(state,'{stage}','3') WHERE world_id=encode($1,'hex')::uuid AND change_key='the_fire_feathered_serpent'")
        .bind(world).execute(&h.pool).await?;
    assert_eq!(awake.rows_affected(), 1);
    Ok(())
}

async fn install_npcs(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    wander: bool,
) -> TestResult<[EntityRef; 5]> {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    let table = pinned_placements()?;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let keys = NPCS.map(|(key, _)| key);
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model_with_native_quests(
            &model, &keys,
        ),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let mut runtime = owner.runtime.lock().await;
    runtime
        .place_npcs(&table, |key| keys.contains(&key), true)
        .map_err(debug)?;
    let mut found = Vec::new();
    for (key, at) in NPCS {
        let actor = if (at.x, at.y) == (33553, 31624) || (at.x, at.y) == (33554, 31624) {
            runtime.place_dynamic_npc(key, NpcPlacement {
                x: at.x, y: at.y, floor: at.floor, direction: MovementFacing::North,
            }).map_err(debug)?
        } else {
            runtime.visible_entities().npcs.into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some(key))
                .ok_or_else(|| format!("missing live NPC {key}"))?.actor
        };
        // Move both exact Guardian actors through the same cardinal owner used by NPC cadence.
        if wander && key == NPCS[1].0 {
            let expected = runtime.borrow_movement_position().read(actor).map_err(debug)?;
            let position = expected.position();
            runtime
                .borrow_movement_position()
                .commit_cardinal(
                    expected,
                    crate::foundation::MovementLocalPosition {
                        y: position.y + 1,
                        ..position
                    },
                )
                .map_err(debug)?;
        }
        found.push(EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        });
    }
    found.try_into().map_err(|_| "exact NPC refs".into())

}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use UseDisposition::{Committed, Rejected};
    let mut caps = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    caps.extend([
        3,
        4,
        6,
        11,
        15,
        18,
        25,
        oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
    ]);
    caps.sort_unstable();
    caps.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &caps)
            .ok_or("capabilities")?;
    let npcs = install_npcs(owner, admitted, name == GUARDIAN).await?;
    let callbacks = reward_callbacks::load()?;
    macro_rules! say {
        ($npc:expr, $name:literal, [$($word:literal),+], $expected:expr) => {{
            let key = concat!("oteryn:quest-reward/qc-twenty-miles-", $name);
            let cb = callbacks.iter().find(|c| c.key == key).ok_or(key)?;
            stand_near(owner, admitted, NPCS[$npc].1).await?;
            let (result, _, _) = super::qengine_rewards_fixtures::wire_conversation(
                owner, admitted, npcs[$npc], cb, false, vec!["hi", $($word),+],
            ).await.map_err(|error| format!("{key} NPC exchange failed: {error}"))?;
            assert_eq!(result == NpcIntentDisposition::Ok, $expected, "{key}");
        }};
    }
    if matches!(name, BORROWED_REPORT | BORROWED_SONGS) {
        let (pin, reported_stage) = if name == BORROWED_REPORT {
            (LEGACY_HASH, 20)
        } else {
            (PRE_REVIEW_HASH, 21)
        };
        super::qc_pinned_world::seed_saved_progress(
            h, owner, admitted, QUEST, pin, STAGE, reported_stage,
        ).await?;
        say!(1, "recover-sea-ear-after-report", ["lost sea ear"], false);
        enter(owner, admitted, "maelstrom-to-shells", true).await?;
        use_object(owner, admitted, "sea-ear-shell", None, Committed).await?;
        stage(h, reported_stage).await?;
        assert_eq!(track(h, "oteryn:quest-progress/qc/twenty_miles/ear_found").await?, 1);
        only_live_item(h, world_callbacks::phase::SEA_EAR_ITEM).await?;
        return Ok(());
    }
    if name == RESUME {
        super::qc_pinned_world::seed_saved_progress(
            h,
            owner,
            admitted,
            QUEST,
            LEGACY_HASH,
            STAGE,
            20,
        )
        .await?;
        // Stage 20 is reachable by showing a borrowed Sea Ear. After it is
        // returned, the old save has progress but no issue to replace.
        let recovery_key = "oteryn:quest-reward/qc-twenty-miles-recover-sea-ear-after-report";
        let callback = callbacks
            .iter()
            .find(|c| c.key == recovery_key)
            .ok_or(recovery_key)?;
        stand_near(owner, admitted, NPCS[1].1).await?;
        let (disposition, _, _) = super::qengine_rewards_fixtures::wire_conversation(
            owner,
            admitted,
            npcs[1],
            callback,
            false,
            vec!["hi", "lost sea ear"],
        )
        .await?;
        assert_eq!(disposition, NpcIntentDisposition::Rejected);
        let replacements: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(recovery_key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(replacements, 0);
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h, owner, admitted, &[(STAGE.to_owned(), 19)], 121,
        ).await?;
        owner.refresh_quest_session(admitted.game_session_id, false).await;
        owner.states.quest_callbacks.phase_read_unavailable.store(
            true, std::sync::atomic::Ordering::Release,
        );
        enter(owner, admitted, "maelstrom-to-shells", false).await?;
        enter(owner, admitted, "maelstrom-to-shells", true).await?;
        use_object(owner, admitted, "sea-ear-shell", None, Committed).await?;
        let item_key = world_callbacks::phase::SEA_EAR_ITEM;
        let ear = only_live_item(h, item_key).await?;
        move_recovery_item(owner, admitted, ear, 22_002, true, None).await?;
        enter(owner, admitted, "shell-cave-exit", true).await?;
        enter(owner, admitted, "maelstrom-to-shells", true).await?;
        move_recovery_item(owner, admitted, ear, 22_003, false, None).await?;
        enter(owner, admitted, "shell-cave-exit", true).await?;
        enter(owner, admitted, "maelstrom-to-grounds-with-ear", true).await?;
        say!(1, "guardian-sea-ear", ["mission"], true);
        say!(1, "recover-sea-ear-after-report", ["lost sea ear"], false);
        move_recovery_item(owner, admitted, ear, 22_004, true, None).await?;
        say!(1, "recover-sea-ear-after-report", ["lost sea ear"], false);
        assert_eq!(only_live_item(h, item_key).await?, ear);
        expire_dropped_item(h, ear).await?;
        say!(1, "recover-sea-ear-after-report", ["lost sea ear"], true);
        assert_ne!(only_live_item(h, item_key).await?, ear);
        say!(1, "recover-sea-ear-after-report", ["lost sea ear"], false);
        assert_eq!(track(h, "oteryn:quest-progress/qc/twenty_miles/replacements").await?, 1);
        stage(h, 20).await?;
        return Ok(());
    }

    stand_near(owner, admitted, NPCS[3].1).await?;
    super::classic_quests_fixtures::talk(owner, admitted, npcs[3], 21_001, "hi").await?;
    for (command, word) in [(21_002, "orbs"), (21_003, "energies")] {
        let lines =
            super::classic_quests_fixtures::talk(owner, admitted, npcs[3], command, word).await?;
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("Bring the darkness, life and light energies.")),
            "event Guardian {word}: {lines:?}"
        );
    }
    say!(1, "guardian-yes-start", ["assist", "yes"], false);
    use_object(owner, admitted, "pile-1", None, Rejected).await?;
    enter(owner, admitted, "west-entrance", false).await?;
    enter(owner, admitted, "sunken-city-entrance", false).await?;
    // The sourced main grounds admit a visitor before the quest agreement, bareheaded.
    enter(owner, admitted, "maelstrom-to-grounds", true).await?;
    say!(0, "brandon-yes", ["assist", "yes"], true);
    enter(owner, admitted, "maelstrom-to-grounds", true).await?;
    say!(1, "guardian-yes-start", ["wedding ring", "something", "will"], true);
    if name == GUARDIAN {
        stage(h, 2).await?;
        return Ok(());
    }
    // A loose Seacrest Pearl is already present in the shipped map.
    pickup_pearl(owner, admitted).await?;
    if name == TRADE {
        let pearl = only_live_item(h, "oteryn:item.tibia.i21747").await?;
        for (name, tool) in [("pile-1", None), ("stone-shrine-01", Some(21770))] {
            {
                let runtime = owner.runtime.lock().await;
                owner.states.player_trades.fixture_open_offer(
                    &runtime, admitted.game_session_id, pearl, owner.owner_now().get() / 1000,
                ).ok_or("open trade offer")?;
            }
            use_object(owner, admitted, name, tool, Rejected).await?;
            assert!(!owner.states.player_trades.open(admitted.game_session_id));
            if tool.is_none() {
                Box::pin(prove_use_commit_releases_guards(owner, admitted)).await?;
            }
        }
        return Ok(());
    }
    Box::pin(prove_use_commit_releases_guards(owner, admitted)).await?;
    use_object(owner, admitted, "pile-1", None, Rejected).await?;
    use_object(owner, admitted, "altar-1", None, Rejected).await?;
    use_object(
        owner,
        admitted,
        "altar-1",
        Some(21747),
        UseDisposition::NothingToUse,
    )
    .await?;
    use_object(owner, admitted, "stone-shrine-01", Some(21770), Committed).await?;
    for i in 2..=5 {
        use_object(owner, admitted, &format!("pile-{i}"), None, Committed).await?;
        use_object(
            owner,
            admitted,
            &format!("altar-{i}"),
            Some(21770),
            Committed,
        )
        .await?;
    }
    awaken_fire_feathered(owner, h).await?;
    say!(1, "guardian-skulls-laid", ["mission", "yes"], false);
    stage(h, 2).await?;
    let energies = oteryn_gameplay_quests::quest::fire_feathered_native::TRACKS
        .map(|key| (key.to_owned(), 1));
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, admitted, &energies, 120,
    ).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    say!(3, "guardian-skulls-laid", ["yes"], true);
    stage(h, 2).await?;
    for (energy, _) in energies {
        assert_eq!(track(h, &energy).await?, 0);
    }
    say!(1, "guardian-skulls-laid", ["mission", "yes"], true);
    if matches!(name, GOLD | SPLIT_GOLD | EARLIER_SPLIT_GOLD | BORROWED_REPORT | BORROWED_SONGS) {
        let gold = "oteryn:item.tibia.i21746";
        let issued = only_live_item(h, gold).await?;
        move_recovery_item(owner, admitted, issued, 22_010, true, None).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        let receiver = super::fixture::id(216);
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(216, gold, "definition-r1", if name == EARLIER_SPLIT_GOLD { 2 } else { 1 }),
        )).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id)
             SELECT character_id,parent_item_instance_id,encode($1,'hex')::uuid,world_id,max(placement_ordinal)+1,encode($1,'hex')::uuid
             FROM game_item_container_entries GROUP BY character_id,parent_item_instance_id,world_id",
        ).bind(receiver.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        if name == EARLIER_SPLIT_GOLD {
            move_recovery_item(owner, admitted, receiver, 22_011, true, Some(1)).await?;
            move_recovery_item(owner, admitted, issued, 22_012, false, None).await?;
            move_recovery_item(owner, admitted, receiver, 22_013, true, None).await?;
            expire_dropped_item(h, receiver).await?;
            let unrelated = only_live_item(h, gold).await?;
            assert_ne!(unrelated, issued);
            assert_ne!(unrelated, receiver);
            say!(1, "recover-cursed-gold", ["lost cursed gold"], true);
            stage(h, 3).await?;
            return Ok(());
        }
        move_recovery_item(owner, admitted, issued, 22_011, false, None).await?;
        assert_eq!(only_live_item(h, gold).await?, receiver);
        if name == SPLIT_GOLD {
            // Split a fresh coin onto Ground, then lose the original stack. With
            // zero carried coins the recovery must reach its issuance check.
            move_recovery_item(owner, admitted, receiver, 22_012, true, Some(1)).await?;
            let split: sqlx::types::Uuid = sqlx::query_scalar(
                "SELECT item_instance_id FROM game_item_instances
                 WHERE definition_production_key=$1 AND lifecycle=1 AND item_instance_id<>$2",
            ).bind(gold).bind(sqlx::types::Uuid::from_bytes(receiver)).fetch_one(&h.pool).await?;
            move_recovery_item(owner, admitted, receiver, 22_013, true, None).await?;
            expire_dropped_item(h, receiver).await?;
            assert_eq!(only_live_item(h, gold).await?, *split.as_bytes());
            say!(1, "recover-cursed-gold", ["lost cursed gold"], false);
            expire_dropped_item(h, *split.as_bytes()).await?;
            say!(1, "recover-cursed-gold", ["lost cursed gold"], true);
            stage(h, 3).await?;
            return Ok(());
        }
        say!(1, "recover-cursed-gold", ["lost cursed gold"], false);
        move_recovery_item(owner, admitted, receiver, 22_012, true, None).await?;
        expire_dropped_item(h, receiver).await?;
        say!(1, "recover-cursed-gold", ["lost cursed gold"], true);
        assert_ne!(only_live_item(h, gold).await?, receiver);
        stage(h, 3).await?;
        return Ok(());
    }
    use_object(owner, admitted, "hoard-1-reentry", None, Rejected).await?;
    let hoard = source(&callback("hoard-1")?).ok_or("hoard source")?;
    let (origin, _) = stand_near(owner, admitted, hoard).await?;
    owner.states.quest_callbacks.interrupt_after_commit.store(true, std::sync::atomic::Ordering::Release);
    assert!(use_object(owner, admitted, "hoard-1", Some(21746), Committed).await.is_err());
    stage(h, 4).await?;
    assert_eq!(
        owner.runtime.lock().await.read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?.position(),
        origin,
    );
    assert!(owner.reconcile_quest_callback_receipts(admitted.game_session_id).await);
    assert!(owner.states.quest_callbacks.discard_session_projection(admitted.game_session_id));
    // Preserve the durable command high-water while discarding the map projection.
    owner.states.command_ingress.recover_trade_continuity(admitted)
        .ok_or("recover interrupted hoard command")?;
    for i in 1..=4 {
        use_object(owner, admitted, &format!("hoard-{i}-reentry"), None, Committed).await?;
    }
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_quest_callback_consumptions
         WHERE transition_key LIKE 'oteryn:quest-callback/qc-twenty-miles-hoard-%'",
    ).fetch_one(&h.pool).await?, 1);
    use_object(owner, admitted, "ring-chest", None, Committed).await?;
    say!(0, "brandon-ring", ["wedding ring"], true);
    enter(owner, admitted, "west-entrance", true).await?;
    say!(0, "brandon-mission-slabs", ["mission"], true);
    say!(1, "guardian-mission-slabs", ["mission"], true);
    use_object(owner, admitted, "shipwreck-chest", None, Committed).await?;
    say!(1, "guardian-first-slab", ["mission"], true);
    enter(owner, admitted, "shrine-entrance", true).await?;
    for whirl in ["shrine-whirl-south", "shrine-whirl-middle", "shrine-whirl-centre"] {
        enter(owner, admitted, whirl, true).await?;
    }
    use_object(owner, admitted, "shrine-chest", None, Committed).await?;
    enter(owner, admitted, "shrine-whirl-return", true).await?;
    enter(owner, admitted, "shrine-exit", true).await?;
    say!(1, "guardian-second-slab", ["mission"], true);
    say!(0, "brandon-yeo", ["mission", "yeo"], true);
    // A live Yeo in the event placement must leave the pearl and stage untouched.
    let pearl_key = "oteryn:item.tibia.i21747";
    let pearl = only_live_item(h, pearl_key).await?;
    say!(4, "yeo-slab", ["seacrest pearl", "yes"], true);
    stage(h, 13).await?;
    assert_eq!(only_live_item(h, pearl_key).await?, pearl);
    say!(2, "yeo-slab", ["fishing", "interesting", "use"], true);
    say!(2, "yeo-slab", ["nacre slab", "use"], true);
    stage(h, 13).await?;
    assert_eq!(only_live_item(h, pearl_key).await?, pearl);
    say!(2, "yeo-slab", ["seacrest pearl", "yes"], true);
    say!(1, "guardian-third-slab", ["mission"], true);
    stage(h, 16).await?;
    only_live_item(h, "oteryn:item.tibia.i21753").await?;
    only_live_item(h, "oteryn:item.tibia.i21861").await?;
    for (altar, slab) in [21753, 21861, 21862].into_iter().enumerate() {
        use_object(
            owner,
            admitted,
            &format!("broken-altar-{}", altar + 1),
            Some(slab),
            Committed,
        )
        .await?;
    }
    say!(1, "guardian-altars-mended", ["mission"], true);
    enter(owner, admitted, "sunken-city-entrance", true).await?;
    say!(1, "guardian-quest-sea-ear", ["quest"], true);
    say!(0, "brandon-sea-ear", ["mission"], true);
    enter(owner, admitted, "maelstrom-to-shells", true).await?;
    use_object(owner, admitted, "sea-ear-shell", None, Committed).await?;
    enter(owner, admitted, "shell-cave-exit", true).await?;
    enter(owner, admitted, "maelstrom-to-grounds-with-ear", true).await?;
    say!(1, "guardian-sea-ear", ["mission"], true);
    say!(1, "guardian-compass-place", ["mission"], false);
    for i in 1..=5 {
        use_object(
            owner,
            admitted,
            &format!("statue-{i}"),
            Some(21754),
            Committed,
        )
        .await?;
        let song = format!("oteryn:quest-progress/qc/twenty_miles/song{i}");
        assert_eq!(track(h, &song).await?, 0);
        use_object(
            owner, admitted, &format!("statue-{i}"), Some(21754), Rejected,
        ).await?;
        assert_eq!(track(h, &song).await?, 0);
        let mut clock = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *clock).await?;
        let advanced = sqlx::query(
            "UPDATE game_character_quest_tracks SET value=value-120 WHERE track_key=$1",
        ).bind(format!("{song}_at")).execute(&mut *clock).await?;
        assert_eq!(advanced.rows_affected(), 1);
        clock.commit().await?;
        use_object(
            owner,
            admitted,
            &format!("statue-{i}"),
            Some(21754),
            Committed,
        )
        .await?;
        assert_eq!(track(h, &song).await?, 1);

    }
    say!(1, "guardian-compass-place", ["mission"], true);
    use_object(owner, admitted, "sundial", Some(21754), Committed).await?;
    enter(owner, admitted, "birthing-entrance", false).await?;
    say!(1, "guardian-compass", ["compass"], true);
    enter(owner, admitted, "birthing-entrance", true).await?;
    stage(h, 23).await?;
    let done: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)")
        .bind(QUEST).fetch_one(&h.pool).await?;
    assert!(done);
    Ok(())
}
