//! All four source routes drive the production command-9 owner on native map crops.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_move::{LoopMove, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    elemental_spheres::{self, SummonRule},
    world_callbacks::Position,
};
use sha2::{Digest, Sha256};

fn revision(hash: [u8; 32]) -> String {
    format!(
        "sha256:{}",
        hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}
fn rule(name: &str) -> TestResult<&'static SummonRule> {
    elemental_spheres::rules()
        .iter()
        .find(|r| {
            name.contains(
                r.key
                    .rsplit('/')
                    .next()
                    .unwrap_or("unbound")
                    .trim_end_matches("_overlord"),
            )
        })
        .ok_or_else(|| "Elemental rule".into())
}
pub(super) fn start(name: &str) -> TestResult<Position> {
    let r = rule(name)?;
    Ok(Position {
        x: r.spawn.x + 1,
        ..r.spawn
    })
}
pub(super) fn world(
    native: &NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop, native_scene_with_appearances};
    let r = rule(name)?;
    let rx = r.spawn.x / 256;
    let bytes: &[u8] = if rx == 129 {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x129-y125.b3"
        ))
    } else {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y125.b3"
        ))
    };
    let minx = r.positions.iter().map(|p| p.x).min().ok_or("minimumx")? - 1;
    let miny = r.positions.iter().map(|p| p.y).min().ok_or("minimumy")? - 1;
    let maxx = r.positions.iter().map(|p| p.x).max().ok_or("maximumx")? + 2;
    let maxy = r.positions.iter().map(|p| p.y).max().ok_or("maximumy")? + 2;
    let appearance = r
        .charged
        .rsplit('i')
        .next()
        .ok_or("charge appearance")?
        .parse()?;
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes,
            region: (13, rx, 125),
            bounds: (minx, miny, maxx, maxy),
            start: (r.spawn.x + 1, r.spawn.y),
            expected: vec![],
        }],
        &[],
        &[appearance],
    )
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let trigger = rule(name)?.spawn;
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M49 scenario thread")?
        .map_err(Into::into)
}
#[test]
fn m49_energy_native_arrival_summons_once() -> TestResult {
    run("m49-energy")
}
#[test]
fn m49_fire_native_arrival_summons_once() -> TestResult {
    run("m49-fire")
}
#[test]
fn m49_ice_native_arrival_summons_once() -> TestResult {
    run("m49-ice")
}
#[test]
fn m49_earth_native_arrival_summons_once() -> TestResult {
    run("m49-earth")
}
#[test]
fn m49_missing_charge_keeps_corpse_and_boss_absent() -> TestResult {
    run("m49-fire-missing")
}

#[test]
fn m49_blocked_spawn_recovers_after_charge_revision_changes() -> TestResult {
    run("m49-fire-recover")
}
#[test]
fn m49_unknown_realization_ack_never_duplicates_the_actor() -> TestResult {
    run("m49-fire-ack")
}
#[test]
fn m49_arrival_retires_real_corpse_loot_with_the_root() -> TestResult {
    run("m49-fire-loot")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
        ItemMintRequest, TypedDefinitionRef,
    };
    use crate::foundation::ScopeOwnershipGeneration;
    let r = rule(name)?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?;
    let index = native.item_index().ok_or("Item index")?;
    let map = owner.bundle_map().ok_or("native map")?;
    let appearance: u16 = r
        .charged
        .rsplit('i')
        .next()
        .ok_or("charge appearance")?
        .parse()?;
    let after = map
        .facts
        .movement_item_facts(appearance)
        .ok_or("charge projection facts")?;
    let mut charged = Vec::new();
    // Seed the source charging operation's exact map projection. Arrivals still
    // read/fence these real native placements through the production owner.
    for (n, p) in r.positions.iter().enumerate() {
        let pos = TilePos {
            x: p.x,
            y: p.y,
            floor: p.floor,
        };
        let facts = map.facts.pinned();
        let tile = crate::map::view::compose_tile(&map.overlay, &facts, pos)
            .map_err(debug)?
            .ok_or("charge tile")?;
        let entry = tile.entries.last().ok_or("charge root")?;
        let crate::map::view::MapEntryKey::Base(key) = entry.source.key() else {
            return Err("native charge root".into());
        };
        let rev = map.facts.object_revision(key);
        if !(name.ends_with("missing") && n == 7) {
            let projection = (r.charged == "oteryn:item.tibia.i2126").then_some(
                crate::map::facts::QuestCallbackProjection::ItemPresence {
                    position: pos,
                    entry: after,
                },
            );
            let root_after = if projection.is_some() {
                entry.facts
            } else {
                after
            };
            assert!(map.facts.apply_quest_callback(
                key,
                rev,
                root_after,
                &[],
                &[],
                false,
                projection.as_ref()
            ));
            charged.push((key, rev + 1));
        }
    }
    let (frame, content) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().frame_binding_digest().to_vec(),
            runtime.content_pin().server_artifact_digest(),
        )
    };
    let physical_map = revision(
        Sha256::digest(crate::map::overlay::map_revision(map.overlay.base()).as_bytes()).into(),
    );
    let physical_content = revision(content);
    let from = start(name)?;
    let source = GroundPlacement {
        spatial_position: [
            i32::from(from.x).to_be_bytes().as_slice(),
            i32::from(from.y).to_be_bytes().as_slice(),
            (-i16::from(from.floor)).to_be_bytes().as_slice(),
        ]
        .concat(),
        corpse_ref: id(77).to_vec(),
        map_revision: physical_map,
        content_revision: physical_content,
        native_room_placement_context: frame,
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: r.corpse.into(),
        revision_ref: index.revision(r.corpse).ok_or("corpse revision")?.into(),
    };
    let mut candidate = h
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    ScopeOwnershipGeneration::new(1).map_err(debug)?,
                    100,
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:m49.loot".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                    0,
                ),
                item: definition,
                quantity: 1,
                ground: source,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(corpse) = h
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut candidate,
            id(41),
        )
        .await
        .map_err(debug)?
    else {
        return Err("native corpse MINT".into());
    };
    drop(permit);
    if name.ends_with("loot") {
        use crate::durability::item_mint::{CorpseContainerPlacement, CorpseLootMintRequest};
        let mut loot = h
            .root
            .freeze_corpse_loot_mint(
                owner.character,
                owner.holder,
                CorpseLootMintRequest {
                    cause: ItemMintCause::for_test(
                        owner.world_id,
                        owner.channel_id,
                        ScopeOwnershipGeneration::new(1).map_err(debug)?,
                        100,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:m49.loot".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:m49.loot-output".into(),
                        1,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i3031".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 7,
                    placement: CorpseContainerPlacement {
                        corpse_item_instance_id: corpse.item_instance_id,
                        placement_ordinal: 1,
                        fluid_subtype: None,
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(debug)?;
        let permit = loot.fresh_death_lane_permit().await;
        assert!(matches!(
            h.root
                .commit_corpse_loot_mint(&permit, owner.character, owner.holder, &mut loot)
                .await
                .map_err(debug)?,
            ItemMintOutcome::Committed(_)
        ));
        drop(permit);
    }
    let blocker = if name.ends_with("recover") {
        Some(
            owner
                .runtime
                .lock()
                .await
                .admit_encounter_actor(
                    "fixture:m49.blocker",
                    1,
                    crate::foundation::MovementLocalPosition {
                        x: i32::from(r.spawn.x),
                        y: i32::from(r.spawn.y),
                        floor: -i16::from(r.spawn.floor),
                    },
                )
                .map_err(debug)?,
        )
    } else {
        None
    };
    if name.ends_with("ack") {
        sqlx::query("CREATE FUNCTION m49_ack_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'M49 injected unknown ACK'; END$$").execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER m49_ack_fail BEFORE INSERT ON game_elemental_boss_realizations FOR EACH ROW EXECUTE FUNCTION m49_ack_fail()").execute(&h.pool).await?;
    }
    let request = LoopMove {
        item: ItemKey::Instance(corpse.item_instance_id),
        count: None,
        target: LoopTarget::Ground(oteryn_protocol_oteryn::item_view::WorldTilePosition {
            x: i32::from(r.spawn.x),
            y: i32::from(r.spawn.y),
            floor: i16::from(r.spawn.floor),
        }),
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 4901,
        item_fence: admitted.item_fence,
    };
    use crate::gameplay_transport::item_view::{ItemViewContinuity, SessionItemView};
    use oteryn_protocol_oteryn::item_view::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
        encode_item_move_intent_for,
    };
    let mut view = SessionItemView::resume(ItemViewContinuity::default())
        .with_map_view()
        .with_equip_drop();
    let handle = view
        .map_view(&[request.item], |table| {
            table
                .handle(&request.item)
                .ok_or(crate::gameplay_transport::item_view::ItemViewError::Exhausted)
        })
        .map_err(debug)?;
    let selection = ItemMoveSelection {
        equip_drop: true,
        container_tree: false,
        world_interaction: true,
    };
    let LoopTarget::Ground(to) = request.target else {
        return Err("native destination".into());
    };
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: handle,
            destination: ItemMoveDestination::Ground(to),
        },
        selection,
    )
    .map_err(debug)?;
    let result = crate::gameplay_transport::item_move::item_move(
        owner, actor, command, &payload, selection, false, &mut view,
    )
    .await;
    assert!(
        matches!(
            result,
            crate::gameplay_transport::item_move::ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
        ),
        "native arrival {name}: {result:?}"
    );
    // The real command dispatcher classifies a committed occurrence before
    // resolving the now-retired corpse's old map handle.
    let replay = crate::gameplay_transport::item_move::item_move(
        owner, actor, command, &payload, selection, false, &mut view,
    )
    .await;
    assert!(matches!(
        replay,
        crate::gameplay_transport::item_move::ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_elemental_altar_receipts")
        .fetch_one(&h.pool)
        .await?;
    if name.ends_with("missing") {
        assert_eq!(count, 0);
    } else {
        assert_eq!(count, 1);
        if let Some(blocker) = blocker {
            let realizations: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_elemental_boss_realizations")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(realizations, 0);
            let arrival: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(transaction_id) FROM game_elemental_altar_receipts LIMIT 1",
            )
            .fetch_one(&h.pool)
            .await?;
            let mut stale = blocker;
            stale.0.set_scope_generation(
                crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?,
            );
            owner
                .states
                .quest_callbacks
                .elemental
                .lock()
                .map_err(|_| "actor cache")?
                .insert(arrival.try_into().map_err(|_| "arrival UUID")?, stale);
            owner
                .runtime
                .lock()
                .await
                .remove_encounter_actor(blocker)
                .map_err(debug)?;
            let (key, rev) = charged.first_mut().ok_or("charged readset")?;
            assert!(
                map.facts
                    .apply_quest_callback(*key, *rev, after, &[], &[], false, None)
            );
            *rev += 1;
            owner.drain_elemental_altar_bosses().await;
        }
        if name.ends_with("ack") {
            let realizations: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_elemental_boss_realizations")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(realizations, 0);
            sqlx::query("DROP TRIGGER m49_ack_fail ON game_elemental_boss_realizations")
                .execute(&h.pool)
                .await?;
        }
        if name.ends_with("loot") {
            let retired:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_elemental_altar_receipts r ON r.transaction_id=i.last_transaction_id WHERE i.lifecycle=2 AND i.quantity=0").fetch_one(&h.pool).await?;
            assert_eq!(retired, 2);
        }
        let runtime = owner.runtime.lock().await;
        let census = runtime
            .current_live_creature_candidates(4096)
            .map_err(debug)?;
        let bosses = census
            .into_iter()
            .filter(|a| {
                runtime
                    .companion_snapshot(*a)
                    .is_ok_and(|s| s.state.policy.definition_key == r.creature)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            bosses.len(),
            1,
            "boss must be physically realized by native ingress"
        );
        assert_eq!(
            runtime
                .read_actor_position(bosses[0])
                .map_err(debug)?
                .position(),
            crate::foundation::MovementLocalPosition {
                x: i32::from(r.spawn.x),
                y: i32::from(r.spawn.y),
                floor: -i16::from(r.spawn.floor)
            }
        );
        drop(runtime);
        owner.drain_elemental_altar_bosses().await;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_elemental_boss_realizations")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);
    }
    for (key, rev) in charged {
        assert_eq!(map.facts.object_revision(key), rev);
    }
    Ok(())
}
