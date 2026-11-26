//! Use the Serpentine Tower lever through production dispatch.
use super::fixture::{Harness, debug};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks_fixtures as fixture,
};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::map::{
    boot::BundleWorld,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const LEVER: Position = Position {
    x: 33152,
    y: 32866,
    floor: -8,
};
pub(super) const NAME: &str = "serpentine-tower-lever";

fn walls() -> [Position; 4] {
    [
        Position {
            x: 33148,
            y: 32867,
            floor: -9,
        },
        Position {
            x: 33149,
            y: 32867,
            floor: -9,
        },
        Position {
            x: 33148,
            y: 32868,
            floor: -9,
        },
        Position {
            x: 33149,
            y: 32868,
            floor: -9,
        },
    ]
}

pub(super) fn world(callback: &Callback) -> TestResult<BundleWorld> {
    let objects = walls().map(|cell| (cell, 2129));
    let landings = [33148, 33149].map(|x| Position {
        x,
        y: 32869,
        floor: -9,
    });
    fixture::world_cells(callback, LEVER, 2772, 1, &objects, &[], None, &landings)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    world: &BundleWorld,
    h: &Harness,
    variant: &str,
) -> TestResult {
    let lever = native(LEVER);
    let key = placement_key(lever, 1).ok_or("lever")?;
    let appearance_id = *world
        .base()
        .tile(lever.x, lever.y, lever.floor)
        .and_then(|tile| tile.ids().get(1))
        .ok_or("lever base item")?;
    let occupied = walls()[0];
    let landing = Position {
        y: 32869,
        ..occupied
    };
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("scene item fence")?;
    let movable = fixture::callback("inquisition-weakened-coffin")?
        .offerings
        .into_iter()
        .next()
        .ok_or("movable item fixture")?;
    for (command, appearance, walls_hidden) in [(1, 2773, true), (2, 2772, false), (3, 2773, true)]
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        if command == 2 && variant == "exhausted" {
            let actor = admitted.runtime_actor.ok_or("scene actor")?;
            // Use a future gate timestamp so database or scheduling latency cannot
            // drain the cooldown during the request.
            let now = owner.clock_origin.elapsed() + std::time::Duration::from_secs(60);
            assert!(owner.states.world_gate.lock().map_err(debug)?.admit(actor, now));
        }
        if command == 2 && variant == "deadline" {
            owner.states.quest_callbacks.delay_scene_commit_ack.store(
                true, std::sync::atomic::Ordering::Release,
            );
        }
        let target = WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: world.facts().object_revision(key),
        };
        let use_lever = fixture::wire(
            owner,
            admitted,
            command,
            2,
            encode_use_intent(&target).map_err(debug)?,
        );
        let result = if command == 2 && matches!(variant, "unknown_commit" | "superseded") {
            let started = std::sync::Arc::new(tokio::sync::Notify::new());
            let allow = std::sync::Arc::new(tokio::sync::Notify::new());
            let check_ownership = async {
                tokio::time::timeout(std::time::Duration::from_secs(15), started.notified())
                    .await
                    .map_err(|_| "close never attempted receipt reconciliation")?;
                let runtime_available = owner.runtime.try_lock().is_ok();
                let spells_available = owner.states.spell_states.try_lock().is_ok();
                let lane_available = tokio::time::timeout(
                    std::time::Duration::ZERO,
                    owner.spell_lane.acquire(),
                )
                .await
                .is_ok_and(|permit| permit.is_ok());
                let advanced = if variant == "superseded" {
                    admit_successor_during_receipt_read(owner, h).await
                } else {
                    Ok(())
                };
                allow.notify_one();
                assert_eq!(
                    (runtime_available, spells_available, lane_available),
                    (true, true, true),
                    "receipt retry retained channel ownership",
                );
                advanced
            };
            let (result, checked) = tokio::join!(
                crate::durability::item_loop_move::with_lost_source_scene_commit_ack(
                    use_lever, started.clone(), allow.clone(),
                ),
                check_ownership,
            );
            checked?;
            if variant == "superseded" {
                let Err(error) = result else {
                    return Err("superseded scene emitted a terminal USE result".into());
                };
                assert_eq!(error.to_string(), "no command result");
                Vec::new()
            } else {
                result?
            }
        } else {
            use_lever.await?
        };
        if command == 2 && variant == "superseded" {
            assert!(owner.has_pending_quest_callback(admitted.game_session_id));
            assert_eq!(world.facts().object_revision(key), target.expected_revision);
            let drops = owner.root.read_item_loop_ground_for_scope(fence.runtime_scope).await.map_err(debug)?;
            assert_eq!(drops.len(), 4, "the acknowledged database commit moved every root once");
            for wall in walls() {
                assert!(world.facts().base_hidden(placement_key(native(wall), 1).ok_or("wall")?));
                let landing = crate::durability::map_item_mint_audit::map_item_spatial_position(
                    wall.x, landing.y, wall.floor,
                );
                assert_eq!(drops.iter().filter(|drop| drop.item.ground.spatial_position == landing).count(), 2);
            }
            return Ok(());
        }
        if command == 2 && matches!(variant, "exhausted" | "capacity") {
            let drops = owner.root.read_item_loop_ground_for_scope(fence.runtime_scope)
                .await.map_err(debug)?;
            for wall in walls() {
                assert_eq!(drops.iter().filter(|drop| drop.item.ground.spatial_position ==
                    crate::durability::map_item_mint_audit::map_item_spatial_position(wall.x, wall.y, wall.floor)).count(),
                    1, "ground custody changed at {wall:?}");
                assert!(world.facts().base_hidden(placement_key(native(wall), 1).ok_or("wall")?));
            }
            // This fixture negotiates legacy USE results, which encode exhaustion
            // as Rejected. Custody and wall invariants apply to either refusal.
            assert_eq!(decode_use_result(&result).map_err(debug)?, UseDisposition::Rejected);
            assert_eq!(world.facts().object_revision(key), target.expected_revision);
            return Ok(());
        }
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed,
            "lever operation {command}",
        );
        assert_eq!(
            world
                .facts()
                .base_entry(lever, 1, appearance_id)
                .ok_or("lever facts")?
                .appearance_id,
            appearance
        );
        for wall in walls() {
            let wall_key = placement_key(native(wall), 1).ok_or("forcefield")?;
            assert_eq!(
                world.facts().base_hidden(wall_key),
                walls_hidden,
                "{wall:?}"
            );
        }
        if command == 1 {
            for (index, position) in walls().into_iter().enumerate() {
                fixture::offering(
                    owner,
                    fence.item_fence(),
                    &Offering {
                        position,
                        ..movable.clone()
                    },
                    100 + u64::try_from(index)?,
                )
                .await?;
            }
            if variant == "large" {
                for index in 0..29 {
                    fixture::offering(
                        owner,
                        fence.item_fence(),
                        &Offering { position: occupied, ..movable.clone() },
                        300 + index,
                    ).await?;
                }
            }
            if variant == "capacity" {
                for index in 0..crate::durability::ground_cell_limit::GROUND_CELL_ROOT_LIMIT - 1 {
                    fixture::offering(
                        owner,
                        fence.item_fence(),
                        &Offering { position: landing, ..movable.clone() },
                        200 + u64::try_from(index)?,
                    ).await?;
                }
            }
            if variant == "repin" {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx).await?;
                // Preserve map revision, frame and custody across the content-only repin.
                let changed = sqlx::query("UPDATE game_item_ground_locations SET content_revision='fixture-predecessor' WHERE game_item_loop_drop_ref(corpse_ref)")
                    .execute(&mut *tx).await?;
                assert_eq!(changed.rows_affected(), 4);
                tx.commit().await?;
            }
            owner.states.loop_ground.invalidate();
            owner
                .runtime
                .lock()
                .await
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.telas".into(),
                            placements: vec![NpcPlacement {
                                x: occupied.x,
                                y: occupied.y,
                                floor: occupied.floor,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
        } else {
            let drops = owner
                .root
                .read_item_loop_ground_for_scope(fence.runtime_scope)
                .await
                .map_err(debug)?;
            for wall in walls() {
                let spatial = |p: Position| {
                    crate::durability::map_item_mint_audit::map_item_spatial_position(
                        p.x, p.y, p.floor,
                    )
                };
                let count_at = |p| {
                    drops
                        .iter()
                        .filter(|drop| drop.item.ground.spatial_position == spatial(p))
                        .count()
                };
                assert_eq!(count_at(wall), 0, "wall cell {wall:?}");
                assert_eq!(
                    count_at(Position {
                        y: landing.y,
                        ..wall
                    }),
                    if variant == "large" && wall.x == occupied.x { 31 } else { 2 }
                );
            }
            let runtime = owner.runtime.lock().await;
            let occupant = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.telas"))
                .ok_or("occupant")?;
            assert_eq!(
                runtime
                    .read_scene_actor_position(occupant.actor)
                    .map_err(debug)?
                    .position(),
                fixture::local(landing)
            );
        }
    }
    Ok(())
}

async fn admit_successor_during_receipt_read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    // Simulate the database being advanced while this owner retains its seal.
    // The old seal cannot authorize the successor or publish a staged scene.
    let prior = owner.character.record_for(owner.root).map_err(debug)?;
    let digest = crate::character_recovery_fence::recovery_record_digest(&prior).map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-recovery-admission', 0))")
        .execute(&mut *tx).await?;
    let admission = sqlx::query(
        "INSERT INTO game_character_recovery_admissions
           (authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation,
            predecessor_digest, issued_at, issuer_identity, reconciled_at)
         SELECT authority_scope_id, recovery_generation + 1, game_character_uuid_v7(),
                recovery_generation, $1, issued_at + 1, issuer_identity, reconciled_at + 1
         FROM game_character_recovery_admissions WHERE recovery_generation = $2::numeric",
    )
    .bind(digest.as_slice())
    .bind(prior.recovery_generation.to_string())
    .execute(&mut *tx).await?;
    assert_eq!(admission.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

fn native(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
