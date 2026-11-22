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
pub(super) const RESUME: &str = "qc-twenty-miles-resume";
pub(super) const START: Position = Position {
    x: 33550,
    y: 31865,
    floor: -7,
};
const QUEST: &str = "oteryn:quest.twenty_miles_beneath_the_sea_quest";
const STAGE: &str = "oteryn:quest-progress/qc/twenty_miles/stage";
const LEGACY_HASH: &str = "955724dfc1c9b0392d23eaa22a2b771f739e367e9b7a07f2a7d75e524bfe3f3c";
const NPCS: [(&str, Position); 3] = [
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
];

pub(super) fn matches(name: &str) -> bool {
    matches!(name, CHAIN | RESUME)
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
    assert_eq!(callbacks.len(), 44);
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
    for (key, at) in NPCS {
        let npc = world
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or_else(|| format!("missing shipped NPC {key}"))?;
        assert_eq!(npc.placements.len(), 1);
        let p = npc.placements[0];
        assert_eq!((p.x, p.y, p.floor), (at.x, at.y, at.floor));
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
    for step in [
        StepDirection::North,
        StepDirection::South,
        StepDirection::West,
        StepDirection::East,
        StepDirection::NorthWest,
        StepDirection::NorthEast,
        StepDirection::SouthWest,
        StepDirection::SouthEast,
    ] {
        let (dx, dy) = step.delta();
        let adjacent = crate::foundation::MovementLocalPosition {
            x: local.x - dx,
            y: local.y - dy,
            ..local
        };
        if current.position() == adjacent {
            return Ok((adjacent, step));
        }
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
    let mut placement = base.digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
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
    Ok(())
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

async fn install_npcs(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<[EntityRef; 3]> {
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
    let visible = runtime.visible_entities();
    let find = |key| {
        visible
            .npcs
            .iter()
            .find_map(|npc| {
                (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                    identity: npc.actor.placement_identity(),
                    generation: npc.generation,
                })
            })
            .ok_or_else(|| format!("missing live NPC {key}"))
    };
    Ok([find(keys[0])?, find(keys[1])?, find(keys[2])?])
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
    let npcs = install_npcs(owner, admitted).await?;
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
    if name == RESUME {
        super::qc_pinned_world::seed_saved_progress(
            h,
            owner,
            admitted,
            QUEST,
            LEGACY_HASH,
            STAGE,
            19,
        )
        .await?;
        use_object(owner, admitted, "sea-ear-shell", None, Committed).await?;
        say!(1, "guardian-sea-ear", ["mission"], true);
        stage(h, 20).await?;
        return Ok(());
    }

    stand_near(owner, admitted, NPCS[1].1).await?;
    let energy = oteryn_gameplay_quests::quest::fire_feathered_native::TRACKS[0].to_owned();
    super::classic_quests_fixtures::talk(owner, admitted, npcs[1], 21_001, "hi").await?;
    for (command, word) in [(21_002, "orbs"), (21_003, "energies")] {
        let lines =
            super::classic_quests_fixtures::talk(owner, admitted, npcs[1], command, word).await?;
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("Bring the darkness, life and light energies."))
        );
    }
    say!(1, "guardian-yes-start", ["assist", "yes"], false);
    use_object(owner, admitted, "pile-1", None, Rejected).await?;
    enter(owner, admitted, "west-entrance", false).await?;
    enter(owner, admitted, "sunken-city-entrance", false).await?;
    say!(0, "brandon-yes", ["assist", "yes"], true);
    enter(owner, admitted, "maelstrom-to-grounds", true).await?;
    say!(1, "guardian-yes-start", ["assist", "yes"], true);
    // A loose Seacrest Pearl is already present in the shipped map.
    pickup_pearl(owner, admitted).await?;
    use_object(owner, admitted, "pile-1", None, Committed).await?;
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
    use_object(owner, admitted, "altar-1", Some(21770), Committed).await?;
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
    say!(1, "guardian-skulls-laid", ["yes"], true);
    use_object(owner, admitted, "hoard-1", Some(21746), Committed).await?;
    use_object(owner, admitted, "ring-chest", None, Committed).await?;
    say!(0, "brandon-ring", ["wedding ring"], true);
    enter(owner, admitted, "west-entrance", true).await?;
    say!(0, "brandon-mission-slabs", ["mission"], true);
    say!(1, "guardian-mission-slabs", ["mission"], true);
    use_object(owner, admitted, "shipwreck-chest", None, Committed).await?;
    say!(1, "guardian-first-slab", ["mission"], true);
    enter(owner, admitted, "shrine-entrance", true).await?;
    use_object(owner, admitted, "shrine-chest", None, Committed).await?;
    say!(1, "guardian-second-slab", ["mission"], true);
    say!(0, "brandon-yeo", ["yeo"], true);
    say!(2, "yeo-slab", ["nacre slab", "yes"], true);
    say!(1, "guardian-third-slab", ["mission"], true);
    say!(1, "guardian-second-slab-back", ["mission"], true);
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
    }
    say!(1, "guardian-compass-place", ["mission"], true);
    use_object(owner, admitted, "sundial", Some(21754), Committed).await?;
    say!(1, "guardian-compass", ["compass"], true);
    stage(h, 23).await?;
    let done: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)")
        .bind(QUEST).fetch_one(&h.pool).await?;
    assert!(done);
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(energy.clone(), 1)],
        120,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let lines =
        super::classic_quests_fixtures::talk(owner, admitted, npcs[1], 21_004, "yes").await?;
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("Bring the darkness, life and light energies."))
    );
    assert_eq!(track(h, &energy).await?, 1);
    stage(h, 23).await?;
    Ok(())
}
