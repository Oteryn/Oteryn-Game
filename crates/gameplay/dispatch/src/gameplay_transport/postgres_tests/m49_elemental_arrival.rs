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
fn m49_energy_durable_fields_summon_once() -> TestResult {
    run("m49-energy-fields")
}
#[test]
fn m49_missing_durable_energy_field_keeps_corpse_and_boss_absent() -> TestResult {
    run("m49-energy-fields-missing")
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
    if name.contains("-fields") {
        // This fixture creates actual player fields, so declare their real boot
        // capability closure; the writer still checks the current persisted guard.
        owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[11,30], actor.runtime_scope(), actor.scope_generation().get(), "policy-1",
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        ).map_err(debug)?;
    }
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
        if name.contains("-fields") {
            break;
        }
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
    if name.contains("-fields") {
        seed_fields(
            owner,
            h,
            admitted,
            actor,
            r,
            &source,
            name.ends_with("missing"),
        )
        .await?;
    }
    if name.contains("-fields") {
        let p = r.positions[0];
        let mut placement = source.clone();
        placement.spatial_position = [
            i32::from(p.x).to_be_bytes().as_slice(),
            i32::from(p.y).to_be_bytes().as_slice(),
            (-i16::from(p.floor)).to_be_bytes().as_slice(),
        ]
        .concat();
        // Ordinary loot has no spell collision row. Its presence beside the
        // charge must neither hide the field nor qualify the coin as a charge.
        let mut coin = h
            .root
            .freeze_item_mint(
                owner.character,
                owner.holder,
                ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        owner.world_id,
                        owner.channel_id,
                        ScopeOwnershipGeneration::new(1).map_err(debug)?,
                        101,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:m49.loot".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:m49.field-neighbor".into(),
                        1,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i3031".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 1,
                    ground: placement,
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(debug)?;
        let permit = coin.fresh_death_lane_permit().await;
        assert!(matches!(
            h.root
                .commit_item_mint(&permit, owner.character, owner.holder, &mut coin)
                .await
                .map_err(debug)?,
            ItemMintOutcome::Committed(_)
        ));
        drop(permit);
    }
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
        let state:(i16,i64,i64)=sqlx::query_as("SELECT lifecycle,quantity,(SELECT count(*) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(corpse.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state, (1, 1, 1));
        let runtime = owner.runtime.lock().await;
        assert!(
            !runtime
                .current_live_creature_candidates(4096)
                .map_err(debug)?
                .iter()
                .any(|actor| runtime.companion_snapshot(*actor).is_ok_and(|s| s
                    .state
                    .policy
                    .definition_key
                    == r.creature))
        );
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
            let boss = {
                let mut runtime = owner.runtime.lock().await;
                let boss = runtime
                    .current_live_creature_candidates(4096)
                    .map_err(debug)?
                    .into_iter()
                    .find(|actor| {
                        runtime.matches_live_creature_identity(*actor, r.creature.as_bytes())
                    })
                    .ok_or("unacknowledged boss")?;
                assert!(runtime.actor_spell_reserved(boss));
                assert!(
                    runtime.remove_encounter_actor(boss).is_err(),
                    "unknown ACK must prevent death/removal before the latch records its exact actor"
                );
                boss
            };
            // Lose the volatile memo while the ACK is still failing. The real
            // owner reconstructs only the matching installed source reservation.
            owner
                .states
                .quest_callbacks
                .elemental
                .lock()
                .map_err(|_| "actor cache")?
                .clear();
            owner.drain_elemental_altar_bosses().await;
            sqlx::query("DROP TRIGGER m49_ack_fail ON game_elemental_boss_realizations")
                .execute(&h.pool)
                .await?;
            // Persist ACK but omit its physical projection, simulating a lost
            // successful response. The next owner pass must read it afresh.
            let arrival: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(transaction_id) FROM game_elemental_altar_receipts LIMIT 1",
            )
            .fetch_one(&h.pool)
            .await?;
            let permit = owner.spell_lane_permit().await.ok_or("realization lane")?;
            assert!(
                owner
                    .root
                    .record_altar_boss_realization(
                        &permit,
                        owner.character,
                        owner.holder,
                        arrival.try_into().map_err(|_| "arrival UUID")?,
                        boss
                    )
                    .await
                    .map_err(debug)?
            );
            drop(permit);
            assert!(owner.runtime.lock().await.actor_spell_reserved(boss));
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
        assert!(!owner.runtime.lock().await.actor_spell_reserved(bosses[0]));
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_elemental_boss_realizations")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);
    }
    if name.contains("-fields") {
        let fields:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i2126' AND lifecycle=1 AND quantity=1").fetch_one(&h.pool).await?;
        assert_eq!(fields, if name.ends_with("missing") { 7 } else { 8 });
    }
    for (key, rev) in charged {
        assert_eq!(map.facts.object_revision(key), rev);
    }
    Ok(())
}

/// Fixture cast/cost, but real native qualification, current guard and Item writer.
async fn seed_fields(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    rule: &SummonRule,
    at: &crate::durability::item_mint::GroundPlacement,
    missing: bool,
) -> TestResult {
    use crate::durability::spell_item_transaction::{
        apply_spell_items_in_transaction, assert_spell_item_authority_in_transaction,
    };
    use crate::durability::spell_items_abi::*;
    use crate::foundation::{CommandId, CommandRef};
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native fields")?;
    let policy = native
        .item_policy_for_source_id(2126)
        .ok_or("native energy field")?;
    let definition = crate::durability::native_world_items::qualify_source_field_chain(
        native,
        QualifiedItemDefinition::from_native_policy(policy).map_err(debug)?,
    )
    .map_err(debug)?;
    assert_eq!(definition.field_class, SpellFieldClass::Field);
    let blocks_movement = policy
        .record()
        .attributes
        .blocks_movement
        .ok_or("field movement")?;
    let blocks_projectile = policy
        .record()
        .attributes
        .blocks_projectile
        .ok_or("field projectile")?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("field fence")?;
    let command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(4900).map_err(debug)?,
    );
    let mut operations = Vec::new();
    for (ordinal, p) in rule.positions.iter().enumerate() {
        if missing && ordinal == 7 {
            continue;
        }
        let mut placement = at.clone();
        placement.spatial_position = [
            i32::from(p.x).to_be_bytes().as_slice(),
            i32::from(p.y).to_be_bytes().as_slice(),
            (-i16::from(p.floor)).to_be_bytes().as_slice(),
        ]
        .concat();
        operations.push(SpellItemOperation::MintGround {
            item_instance_id: id(180 + u8::try_from(ordinal)?),
            definition: definition.clone(),
            quantity: 1,
            placement,
            lifetime_millis: None,
            blocks_movement,
            blocks_projectile,
            description: None,
        });
    }
    let request = SpellItemTransactionRequest {
        swing_ordinal: None,
        command,
        spell: crate::durability::item_mint::TypedDefinitionRef {
            family: "Spell".into(),
            production_key: "fixture:m49.energy-charge".into(),
            revision_ref: "definition-r1".into(),
        },
        catalog_digest: native.source_digest(),
        transaction_id: id(188),
        event_id: id(189),
        cost: CastCostBinding {
            vitals_revision_before: 1,
            vitals_revision_after: 2,
            mana_before: 200,
            mana_after: 180,
            soul_before: 100,
            soul_after: 100,
            cooldowns_before: Vec::new(),
            cooldowns_after: vec![("spell:fixture_m49".into(), 100_000)],
            caster_digest_before: [11; 32],
            caster_digest_after: [12; 32],
        },
        caster_origin: Some(SourceCasterOrigin {
            actor,
            character_lease_generation: fence.character_lease_generation,
        }),
        operations,
        companion: None,
        direct_companion: None,
    };
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let mut tx = h.pool.begin().await?;
    let authority = assert_spell_item_authority_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        &fence,
        command,
        native.source_digest(),
    )
    .await
    .map_err(debug)?;
    let outcome = apply_spell_items_in_transaction(
        &mut tx,
        &authority,
        &request,
        &crate::gameplay_transport::world_required_field_guard::player_origin(
            &owner.required_capabilities,
        ),
    )
    .await
    .map_err(debug)?;
    assert!(matches!(outcome, SpellItemTransactionOutcome::Applied(_)));
    tx.commit().await?;
    Ok(())
}
