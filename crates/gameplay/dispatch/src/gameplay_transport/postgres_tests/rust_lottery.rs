//! W19E: real use/use-with routes, seeded rust and lottery outcomes, custody and replay.
use super::fixture::{Harness, debug, id, run};

use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw;
use oteryn_gameplay_items::rust_lottery::{lottery_conversion, rust_conversion};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use tokio::io::AsyncWriteExt;
#[test]
fn w19e_rust_success_wire_receipts() -> TestResult {
    run_scenario("oteryn:item.tibia.i8894", true, true)
}
#[test]
fn w19e_rust_break_wire_receipts() -> TestResult {
    run_scenario("oteryn:item.tibia.i8894", true, false)
}
#[test]
fn w19e_lottery_win_wire_receipts() -> TestResult {
    run_scenario("oteryn:item.tibia.i5957", false, true)
}
#[test]
fn w19e_lottery_lose_wire_receipts() -> TestResult {
    run_scenario("oteryn:item.tibia.i5957", false, false)
}

#[test]
fn srx1_demonic_blood_wire_transform_and_replay_postgres() -> TestResult {
    run_scenario("oteryn:item.tibia.i6558",false,true)?;
    run_scenario("oteryn:item.tibia.i6558",false,false)
}

fn run_scenario(key: &'static str, rust: bool, success: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(key, rust, success).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "rust lottery e2e panic")??;
    Ok(())
}
fn scenario(key: &str, rust: bool, success: bool) -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let harness =
            Harness::create_with_admission_seed(admin, "rust_lottery", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        eprintln!("W19E_PG_BEGIN rust={rust} success={success}");
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let tool_key = "oteryn:item.tibia.i9016";
        let tool_quantity = if success { 2 } else { 1 };
        let entries = if rust {
            vec![(tool_key, tool_quantity), (key, 1)]
        } else {
            vec![(key, 1)]
        };
        super::item_look::seed_main_backpack(&harness, &entries).await?;
        let achievement_key = "oteryn:achievement/polisher";
        super::achievement_granters::seed_progress(&harness, achievement_key, 999).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("offers")?;
        let typed = |key: &str| {
            crate::gameplay_transport::item_look::typed_definition(index, reference(key)?)
        };
        let used = typed(key).ok_or("used definition")?;
        let tool = typed(tool_key).ok_or("tool definition")?;
        let seeded_source = if rust { id(72) } else { id(71) };
        let command_id = 1;
        let plan = |source| {
            let session = *fence.game_session_id.as_bytes();
            if rust {
                let chance = reward_bag_bounded_draw(session, command_id, source, 1, 10_000)?;
                let selection = reward_bag_bounded_draw(session, command_id, source, 2, 420)?;
                rust_conversion(native, &tool, &used, selection * 10_000 + chance)
            } else {
                lottery_conversion(
                    native,
                    &used,
                    reward_bag_bounded_draw(session, command_id, source, 3, 98)?,
                )
            }
        };
        // A fresh wire session starts at CommandId 1. Seed the input identity instead of
        // inventing a command high-water mark without retained ingress history.
        let source = (1_u32..100_000)
            .map(|nonce| {
                let mut source = seeded_source;
                source[12..].copy_from_slice(&nonce.to_be_bytes());
                source
            })
            .find(|source| plan(*source).is_some_and(|p| {
                if key=="oteryn:item.tibia.i6558" {
                    p.outputs.first().is_some_and(|(definition,_)| definition.production_key==
                        if success {"oteryn:item.tibia.i236"} else {"oteryn:item.tibia.i237"})
                } else {p.outputs.is_empty()!=success}
            }))
            .ok_or("seeded outcome")?;
        let expected = plan(source).ok_or("plan")?;
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(source.as_slice()).bind(seeded_source.as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(source.as_slice()).bind(seeded_source.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source")?),
            use_with: None,
            use_with_map_item: rust.then_some(MapUseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("target")?,
            )),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (end, result, actual) = exchange(
            &owner,
            admitted,
            command_id,
            &payload,
            reference(if rust { tool_key } else { key }),
            if rust { reference(key) } else { None },
        )
        .await?;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "rust={rust} success={success}"
        );
        let retired: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, (0, 2));
        let durable = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("backpack")?;
        let mut keys: Vec<_> = durable
            .entries
            .iter()
            .map(|e| (e.item.definition.production_key.as_str(), e.item.quantity))
            .collect();
        let mut expected_keys: Vec<_> = expected
            .outputs
            .iter()
            .map(|(d, n)| (d.production_key.as_str(), *n))
            .collect();
        if rust && success {
            expected_keys.push((tool_key, 1));
        }
        keys.sort_unstable();
        expected_keys.sort_unstable();
        assert_eq!(keys, expected_keys);
        let (recipe, outputs, before, hash_matches): (String, serde_json::Value, serde_json::Value, bool) = sqlx::query_as("SELECT r.recipe,r.outputs,r.before_secondary,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id)").fetch_one(&harness.pool).await?;
        assert_eq!(recipe, expected.recipe);
        assert_eq!(
            outputs.as_array().ok_or("outputs")?.len(),
            usize::from(success || key=="oteryn:item.tibia.i6558")
        );
        assert!(hash_matches);
        let polished = rust && success;
        let achievement = super::achievement_granters::assert_progress(
            &harness,
            achievement_key,
            999 + i64::from(polished),
            polished,
            "oteryn:consume-transform",
        )
        .await?;
        if let Some((_, event, _)) = &achievement {
            let receipt_event: String = sqlx::query_scalar(
                "SELECT event_id::text FROM game_item_consume_transform_records",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(event.as_slice(), receipt_event.as_bytes());
        }
        if rust {
            let proof = before;
            assert_eq!(proof["definition_production_key"], tool_key);
            assert_eq!(proof["quantity"], tool_quantity);
        }
        // Receipt replay with the exact original request must not consume another flask.
        let replay = if rust {
            owner
                .use_consume_transform(
                    deps.actor,
                    UseCommand {
                        game_session_id: fence.game_session_id,
                        command_id,
                        item_fence: Some(fence),
                    },
                    oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                        end.continuity.selected_capabilities.as_slice(),
                        &actual,
                    )
                    .map_err(debug)?,
                    None,
                    None,
                )
                .await
        } else {
            owner
                .use_item_completion(
                    deps.actor,
                    UseCommand {
                        game_session_id: fence.game_session_id,
                        command_id,
                        item_fence: Some(fence),
                    },
                    ItemKey::Instance(source),
                )
                .await
        }
        .ok_or("replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_consume_transform_records"
            )
            .fetch_one(&harness.pool)
            .await?,
            1
        );
        if rust {
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?, tool_quantity - 1);
        }
        // Re-enter the real connection with the post-use inventory, then replay the wire command.
        let (_, replayed, _) = exchange(&owner, end, command_id, &actual, None, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(
            achievement,
            super::achievement_granters::assert_progress(
                &harness,
                achievement_key,
                999 + i64::from(polished),
                polished,
                "oteryn:consume-transform",
            )
            .await?
        );
        if key=="oteryn:item.tibia.i6558" {
            super::achievement_granters::assert_progress(&harness,"oteryn:achievement/demonic_barkeeper",1,false,
                "oteryn:consume-transform").await?;
            eprintln!("SRX1_PG demonic blood wire/transform/replay/achievement elapsed={:?}",started.elapsed());
        }
        eprintln!("POSTGRES_CASE_COMPLETED P11 Polisher rust={rust} success={success} replay");
        let elapsed = started.elapsed();
        assert!(elapsed > std::time::Duration::ZERO);
        eprintln!(
            "W19E_PG_EXECUTED rust={rust} success={success} command={command_id} elapsed={elapsed:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
pub(super) fn bundle_action_room(
    room: &crate::content::QualifiedNativeEntryRoom,
    map: &crate::map::boot::BundleWorld,
    world: crate::foundation::WorldId,
    cells: &crate::content::NativeEntryMovementCells,
) -> TestResult<crate::content::QualifiedNativeEntryRoom> {
    let start = map.start();
    let floor = start
        .floor
        .checked_neg()
        .ok_or("bundle fixture start floor")?;
    Ok(room
        .bundle_view(
            cells.clone(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: i16::from(floor),
            },
        )
        .ok_or("qualified bundle fixture room")?)
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected_source(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        current_target,
        false,
        None,
    )
    .await
}

/// Select the last matching entry from the real current inventory snapshot.
/// Dice-streak fixtures keep earlier dice in their valid carried custody.
pub(super) async fn exchange_latest_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected_source(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        None,
        true,
        None,
    )
    .await
}

/// Drain presentation deltas after the result, through the real production publisher.
pub(super) async fn exchange_with_visuals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(
    AdmittedSession,
    UseDisposition,
    Vec<u8>,
    Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>,
)> {
    let visuals = std::cell::RefCell::new(Vec::new());
    let (end, result, actual) = exchange_selected_source(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        None,
        false,
        Some(&visuals),
    )
    .await?;
    Ok((end, result, actual, visuals.into_inner()))
}

#[allow(
    clippy::too_many_arguments,
    reason = "test helper preserves existing exchange inputs and adds an optional wire recorder"
)]
async fn exchange_selected_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
    latest_source: bool,
    visuals: Option<
        &std::cell::RefCell<Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>>,
    >,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut target = None;
        let mut container = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(
                                domain.payload,
                                admitted.continuity.selected_capabilities.contains(12),
                            )
                            .map_err(debug)?;
                        // A picked-up target may precede the real bag in the
                        // inventory. OPEN must name a published, native-qualified
                        // container rather than merely any non-source item.
                        container = inventory
                            .entries
                            .iter()
                            .find_map(|entry| {
                                let native = owner.active_generation?.native_gameplay()?;
                                let index = native.item_index()?;
                                let definition = crate::gameplay_transport::item_look::typed_definition(
                                    index, entry.item_definition_ref,
                                )?;
                                let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
                                    native, &definition,
                                ).ok()?;
                                facts.container_capacity.filter(|capacity| *capacity > 0)
                                    .map(|_| entry.handle)
                            })
                            .or(container);
                        target = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_target)
                            .map(|entry| entry.handle);
                        let mut candidates = inventory
                            .entries
                            .iter()
                            .chain(inventory.equipment.iter().map(|entry| &entry.item))
                            .filter(|entry| Some(entry.item_definition_ref) == current_source);
                        source = (if latest_source {
                            candidates.next_back()
                        } else {
                            candidates.next()
                        })
                        .map(|entry| entry.handle)
                        .or(source);
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        target = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| {
                                if let (MapDefinition::Item(reference), MapOrigin::Handle(handle)) =
                                    (item.definition, item.origin)
                                    && Some(reference) == current_target
                                {
                                    Some(handle)
                                } else {
                                    None
                                }
                            })
                            .or(target);
                        source = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| {
                                if let (MapDefinition::Item(reference), MapOrigin::Handle(handle)) =
                                    (item.definition, item.origin)
                                    && Some(reference) == current_source
                                {
                                    Some(handle)
                                } else {
                                    None
                                }
                            })
                            .or(source);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        if current_source.is_some()
            && source.is_none()
            && admitted.continuity.selected_capabilities.contains(14)
            && command_id > 1
        {
            use oteryn_protocol_oteryn::container_tree as containers;
            let open =
                containers::encode_container_view_intent(&containers::ContainerViewIntent::Open {
                    handle: container.ok_or("nested source container handle")?,
                    replace_view: None,
                })
                .map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: command_id - 1,
                    command_type: containers::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
                    payload: &open,
                },
            )?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            let mut opened = false;
            while source.is_none() || !opened {
                let raw = super::item_look::read_frame(&mut client).await?;
                let envelope = crate::foundation::decode_wire_envelope(&raw)?;
                if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                    assert_eq!(result.command_id, command_id - 1);
                    assert_eq!(
                        containers::decode_container_view_result(result.payload).map_err(debug)?,
                        containers::ContainerViewOutcome::Opened
                    );
                    opened = true;
                }
                if envelope.message_type() == crate::foundation::MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                    if delta.domain_id == containers::STATE_DOMAIN_CONTAINER_VIEWS {
                        let views =
                            containers::decode_container_views(delta.payload).map_err(debug)?;
                        source = views
                            .views
                            .iter()
                            .flat_map(|view| view.entries.iter())
                            .find(|entry| Some(entry.item_definition_ref) == current_source)
                            .map(|entry| entry.handle);
                    }
                }
            }
        }
        let actual = if current_source.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            if current_target.is_some() {
                intent.use_with_map_item =
                    Some(MapUseTarget::Item(target.ok_or("current target handle")?));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        let mut result = None;
        loop {
            let raw = match super::item_look::read_frame(&mut client).await {
                Ok(raw) => raw,
                Err(error)
                    if visuals.is_some()
                        && result.is_some()
                        && error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                            error.kind() == std::io::ErrorKind::UnexpectedEof
                        }) =>
                {
                    return Ok::<_, Box<dyn std::error::Error>>((
                        result.ok_or("missing result")?,
                        actual,
                    ));
                }
                Err(error) => return Err(error),
            };
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::StateDelta
                && let Some(visuals) = visuals
            {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id
                    == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS
                {
                    visuals.borrow_mut().extend(
                        oteryn_protocol_oteryn::presentation::decode_item_effect_batch(
                            delta.payload,
                        )
                        .map_err(debug)?,
                    );
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let reply = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if reply.command_id == command_id {
                    let disposition =
                        oteryn_protocol_oteryn::world_object::decode_use_result(reply.payload)
                            .map_err(debug)?;
                    if visuals.is_none() {
                        return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
                    }
                    result = Some(disposition);
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    let mut maintenance = std::pin::pin!(super::item_craft::maintain_ready(owner));
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if let Poll::Ready(result) = maintenance.as_mut().poll(context) {
                return Poll::Ready(result);
            }
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }),
    )
    .await??;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}; client_result={result:?}").into());
    };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}
