//! Real command routing, fenced custody, replay and durable equipment reload.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::{self as protocol, item_view::ItemMoveOutcome};
use tokio::io::AsyncWriteExt;

#[test]
fn hotkey_equip_postgres_dispatch_toggle_replay_and_durable_reload() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip", || scenario(false))
}

#[test]
fn hotkey_equip_postgres_worn_item_toggles_with_spare_carried() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-spare", || scenario(true))
}

fn scenario(spare: bool) -> TestResult {
    scenario_options(spare, false, false, false, None, false)
}

#[test]
fn hotkey_equip_postgres_feared_refuses_without_mutation() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-feared", || {
        scenario_options(false, true, false, false, None, false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_source_is_atomic_and_durable() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-nested", || {
        scenario_options(false, false, true, false, None, false)
    })
}

#[test]
fn hotkey_equip_postgres_first_started_copy_beats_unused_and_later_started() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-started", || {
        scenario_options(false, false, false, true, None, false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_exchange_has_one_receipt_and_exact_edges() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-nested-swap", || {
        scenario_options(false, false, true, false, Some(false), false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_exchange_full_main_refuses_atomically() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-nested-full", || {
        scenario_options(false, false, true, false, Some(true), false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_search_with_equipped_extra_quiver() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-extra-quiver", || {
        scenario_options(false, false, true, false, None, true)
    })
}

fn scenario_options(
    spare: bool,
    feared: bool,
    nested: bool,
    started: bool,
    occupied: Option<bool>,
    extra_quiver: bool,
) -> TestResult {
    scenario_options_impl(
        spare,
        feared,
        nested,
        started,
        occupied,
        extra_quiver,
        ScenarioMode::Normal,
    )
}

pub(super) fn coalescing_scenario() -> TestResult {
    scenario_options_impl(
        false,
        false,
        false,
        false,
        None,
        false,
        ScenarioMode::Coalescing,
    )
}

#[derive(Clone, Copy)]
pub(super) enum ScenarioMode {
    Normal,
    Coalescing,
    Pouch,
    ForeignPouch,
    Managed,
    Hands {
        incoming: &'static str,
        both: bool,
        full: bool,
    },
    Ammo {
        full: bool,
        absent: bool,
    },
}

#[test]
fn hotkey_equip_postgres_pouch_source_replay_and_foreign_refusal() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-pouch", || {
        scenario_options_impl(false, false, true, false, None, false, ScenarioMode::Pouch)?;
        scenario_options_impl(
            false,
            false,
            true,
            false,
            None,
            false,
            ScenarioMode::ForeignPouch,
        )
    })
}

#[test]
fn hotkey_equip_postgres_managed_destination_replay() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-managed", || {
        scenario_options_impl(
            false,
            false,
            true,
            false,
            None,
            false,
            ScenarioMode::Managed,
        )
    })
}

pub(super) fn swap_scenario(mode: ScenarioMode) -> TestResult {
    scenario_options_impl(false, false, false, false, None, false, mode)
}

fn scenario_options_impl(
    spare: bool,
    feared: bool,
    nested: bool,
    started: bool,
    occupied: Option<bool>,
    extra_quiver: bool,
    mode: ScenarioMode,
) -> TestResult {
    let coalescing = matches!(mode, ScenarioMode::Coalescing);
    let pouch = matches!(mode, ScenarioMode::Pouch | ScenarioMode::ForeignPouch);
    let foreign = matches!(mode, ScenarioMode::ForeignPouch);
    let managed = matches!(mode, ScenarioMode::Managed);
    run(async |admin| {
        let started_at = std::time::Instant::now();
        let harness =
            Harness::create_with_admission_seed(admin, "hotkey_equip", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let item_key = if let ScenarioMode::Hands { incoming, .. } = mode {
            incoming
        } else if matches!(mode, ScenarioMode::Ammo { .. }) {
            "oteryn:item.tibia.i3447"
        } else if started {
            "oteryn:item.tibia.i6529"
        } else {
            "oteryn:item.tibia.i3264"
        };
        let reference = native
            .item_index()
            .ok_or("index")?
            .definition_ref("Item", item_key, "definition-r1")
            .ok_or("sword ref")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        if feared {
            crate::gameplay_transport::hotkey_equip::tests::apply_status(
                &mut *deps.runtime.lock().await,
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::StatusKind::Feared,
                owner.owner_now(),
            )?;
        }
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        let mut entries = vec![(item_key, 1)];
        if matches!(mode, ScenarioMode::Hands { full: true, .. }) {
            entries.resize(20, ("oteryn:item.tibia.i3031", 1));
        }
        if spare {
            entries.push(("oteryn:item.tibia.i3264", 1));
        }
        if nested {
            entries = vec![("oteryn:item.tibia.i2854", 1)];
        }
        if started {
            entries = vec![(item_key, 1); 3];
        }
        if occupied == Some(true) {
            entries.resize(20, ("oteryn:item.tibia.i3031", 1));
        }
        item_look::seed_main_backpack(&harness, &entries).await?;
        if matches!(mode, ScenarioMode::Hands { .. } | ScenarioMode::Ammo { .. }) {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            if matches!(mode, ScenarioMode::Hands { .. }) {
                // The pinned gameplay profile qualifies this two-hander for level-50 monks.
                sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'monk',0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(id(112).as_slice()).execute(&mut *tx).await?;
            }
            let worn: Vec<(u8, &str, i16)> = match mode {
                ScenarioMode::Hands { both: true, .. } => vec![
                    (110, "oteryn:item.tibia.i3277", 5),
                    (111, "oteryn:item.tibia.i3412", 6),
                ],
                ScenarioMode::Hands { .. } => vec![(110, "oteryn:item.tibia.i50273", 5)],
                ScenarioMode::Ammo { absent: false, .. } => {
                    vec![(110, "oteryn:item.tibia.i35562", 6)]
                }
                _ => vec![],
            };
            for (seed, key, slot) in worn {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        seed,
                        key,
                        "definition-r1",
                        1,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($3,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(slot).bind(id(seed).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            }
            if matches!(mode, ScenarioMode::Ammo { full: true, .. }) {
                // Crystal items.xml:66305: ordinary quiver has six entries.
                for seed in 120..126 {
                    sqlx::raw_sql(sqlx::AssertSqlSafe(
                        crate::durability::item_fee_burn::test_support::item(
                            seed,
                            "oteryn:item.tibia.i3447",
                            "definition-r1",
                            100,
                        ),
                    ))
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($1,'hex')::uuid)")
                        .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(110).as_slice()).bind(i64::from(seed)-119).execute(&mut *tx).await?;
                }
            }
            tx.commit().await?;
        }
        if occupied.is_some() {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(
                crate::durability::item_fee_burn::test_support::item(
                    110,
                    "oteryn:item.tibia.i3308",
                    "definition-r1",
                    1,
                ),
            ))
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,5,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(110).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if started {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            // Newest copy 73 has a stored FULL budget; 72 and 71 are started.
            // Search order, rather than the smallest remaining budget, chooses 72.
            for (seed, remaining) in [(71_u8, 13_000_000_i64), (72, 14_000_000), (73, 14_400_000)] {
                sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,remaining_ms,state_revision) VALUES(encode($1,'hex')::uuid,$2,1)")
                    .bind(id(seed).as_slice()).bind(remaining).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        if nested {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(
                crate::durability::item_fee_burn::test_support::item(
                    100,
                    "oteryn:item.tibia.i3264",
                    "definition-r1",
                    1,
                ),
            ))
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)")
                .bind(id(100).as_slice()).bind(id(42).as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            if extra_quiver {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        111,
                        "oteryn:item.tibia.i35562",
                        "definition-r1",
                        1,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,10,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(id(111).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        if pouch || managed {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            if pouch {
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(71).as_slice()).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i23721' WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(71).as_slice()).execute(&mut *tx).await?;
                if foreign {
                    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Hotkey Foreign' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
                        .bind(id(90).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
                }
                sqlx::query("INSERT INTO game_item_character_inbox_locations(item_instance_id,character_id,world_id,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1)")
                    .bind(id(71).as_slice()).bind(id(if foreign {90} else {41}).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO game_character_quick_loot_settings(character_id,unassigned_container_id,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
                    .bind(id(41).as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            if nested
                || started
                || matches!(mode, ScenarioMode::Hands { .. } | ScenarioMode::Ammo { .. })
            {
                &[4, 6, 12, 14, 179, 180]
            } else {
                &[4, 6, 12, 14, 179]
            },
        )
        .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(179));
        let request = protocol::hotkey_equip::Request {
            definition: reference,
        }
        .encode();
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serve = async {
            crate::gameplay_transport::connection::serve_admitted(
                &mut server,
                admitted,
                &owner,
                crate::gameplay_transport::connection::IDLE_LIVENESS,
            )
            .await
        };
        let pool = &harness.pool;
        let exercise = async move {
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                    break;
                }
            }
            if matches!(mode, ScenarioMode::Hands { .. } | ScenarioMode::Ammo { .. }) {
                let expected = match mode {
                    ScenarioMode::Hands { full: true, .. }
                    | ScenarioMode::Ammo { full: true, .. } => ItemMoveOutcome::NoRoom,
                    ScenarioMode::Ammo { absent: true, .. } => ItemMoveOutcome::SlotMismatch,
                    _ => ItemMoveOutcome::Moved,
                };
                assert_eq!(
                    send(&mut client, 1, &request, protocol::CommandStatus::Accepted).await?,
                    expected
                );
                assert_eq!(
                    send(
                        &mut client,
                        1,
                        &request,
                        protocol::CommandStatus::DuplicateReplay
                    )
                    .await?,
                    expected
                );
                client.shutdown().await?;
                return Ok(());
            }
            if let Some(full) = occupied {
                assert_eq!(
                    send(&mut client, 1, &request, protocol::CommandStatus::Accepted).await?,
                    if full {
                        ItemMoveOutcome::NoRoom
                    } else {
                        ItemMoveOutcome::Moved
                    }
                );
                let receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts")
                        .fetch_one(pool)
                        .await?;
                assert_eq!(receipts, if full { 0 } else { 1 });
                let source_edges:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(100).as_slice()).fetch_one(pool).await?;
                assert_eq!(source_edges, i64::from(full));
                let old_main:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(110).as_slice()).fetch_one(pool).await?;
                assert_eq!(old_main, i64::from(!full));
                if !full {
                    assert_eq!(
                        send(
                            &mut client,
                            1,
                            &request,
                            protocol::CommandStatus::DuplicateReplay
                        )
                        .await?,
                        ItemMoveOutcome::Moved
                    );
                }
                client.shutdown().await?;
                return Ok(());
            }
            if feared || foreign {
                assert_eq!(
                    send(&mut client, 1, &request, protocol::CommandStatus::Accepted).await?,
                    if foreign {
                        ItemMoveOutcome::Stale
                    } else {
                        ItemMoveOutcome::Rejected
                    }
                );
                assert_eq!(equipped_count(pool).await?, 0);
                let receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts")
                        .fetch_one(pool)
                        .await?;
                assert_eq!(receipts, 0);
                client.shutdown().await?;
                return Ok(());
            }
            assert_eq!(
                send(
                    &mut client,
                    1,
                    &[0, 0, 0, 0],
                    protocol::CommandStatus::Accepted
                )
                .await?,
                ItemMoveOutcome::Rejected
            );
            assert_eq!(
                send(&mut client, 2, &request, protocol::CommandStatus::Accepted).await?,
                ItemMoveOutcome::Moved
            );
            if extra_quiver {
                assert_eq!(equipped_count(pool).await?, 2);
                client.shutdown().await?;
                return Ok(());
            }
            assert_eq!(equipped_count(pool).await?, 1);
            if coalescing {
                // All three presses arrive during the cooldown of command 2. Only 5
                // may toggle the sword; 3 and 4 must have no durable equipment receipt.
                for command_id in 3..=5 {
                    let frame = protocol::encode_client_command(
                        1,
                        &protocol::ClientCommandValue {
                            command_id,
                            command_type: protocol::hotkey_equip::COMMAND_TYPE_HOTKEY_EQUIP,
                            payload: &request,
                        },
                    )
                    .map_err(debug)?;
                    client.write_u32(u32::try_from(frame.len())?).await?;
                    client.write_all(&frame).await?;
                }
                for (command_id, expected) in [
                    (3, ItemMoveOutcome::Rejected),
                    (4, ItemMoveOutcome::Rejected),
                    (5, ItemMoveOutcome::Moved),
                ] {
                    loop {
                        let frame = item_look::read_frame(&mut client).await?;
                        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                        if envelope.message_type() == protocol::MessageType::CommandResult {
                            let result = protocol::decode_command_result(envelope.payload())
                                .map_err(debug)?;
                            assert_eq!(result.command_id, command_id);
                            assert_eq!(result.status, protocol::CommandStatus::Accepted);
                            assert_eq!(
                                protocol::hotkey_equip::decode_reply(result.payload)
                                    .map_err(debug)?,
                                expected
                            );
                            break;
                        }
                    }
                }
                assert_eq!(equipped_count(pool).await?, 0);
                let cancelled_receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts WHERE command_id IN (3,4)").fetch_one(pool).await?;
                assert_eq!(cancelled_receipts, 0);
                assert_eq!(
                    send(&mut client, 6, &request, protocol::CommandStatus::Accepted).await?,
                    ItemMoveOutcome::Moved
                );
                assert_eq!(
                    send(
                        &mut client,
                        6,
                        &request,
                        protocol::CommandStatus::DuplicateReplay
                    )
                    .await?,
                    ItemMoveOutcome::Moved
                );
                assert_eq!(equipped_count(pool).await?, 1);
                let receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts")
                        .fetch_one(pool)
                        .await?;
                assert_eq!(receipts, 3);
                client.shutdown().await?;
                return Ok(());
            }
            if started {
                let chosen:String=sqlx::query_scalar("SELECT replace(item_instance_id::text,'-','') FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(pool).await?;
                assert_eq!(
                    chosen,
                    id(72)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
            }
            // Foundation replay must not select the now-worn item and toggle it back.
            assert_eq!(
                send(
                    &mut client,
                    2,
                    &request,
                    protocol::CommandStatus::DuplicateReplay
                )
                .await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(equipped_count(pool).await?, 1);
            assert_eq!(
                send(&mut client, 3, &request, protocol::CommandStatus::Accepted).await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(equipped_count(pool).await?, 0);
            if managed || pouch {
                let query = if managed {
                    "SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid"
                } else {
                    "SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid"
                };
                let count: i64 = sqlx::query_scalar(query)
                    .bind(id(100).as_slice())
                    .bind(id(if managed { 71 } else { 41 }).as_slice())
                    .fetch_one(pool)
                    .await?;
                assert_eq!(count, 1);
                assert_eq!(
                    send(
                        &mut client,
                        3,
                        &request,
                        protocol::CommandStatus::DuplicateReplay
                    )
                    .await?,
                    ItemMoveOutcome::Moved
                );
                assert_eq!(equipped_count(pool).await?, 0);
            }
            assert_eq!(
                send(&mut client, 4, &request, protocol::CommandStatus::Accepted).await?,
                ItemMoveOutcome::Moved
            );
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(pool).await?;
            assert_eq!(receipts, 3);
            client.shutdown().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise)).await??;
        // A newly composed owner reads the stored item by identity, independent of the wire view.
        let mut reloaded = deps.owner(&harness, &authority);
        reloaded.active_generation = Some(generation);
        let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(
            &reloaded,
            admitted.game_session_id,
        )
        .await
        .ok_or("reloaded equipment")?;
        if matches!(mode, ScenarioMode::Hands { .. } | ScenarioMode::Ammo { .. }) {
            let refused = matches!(
                mode,
                ScenarioMode::Hands { full: true, .. }
                    | ScenarioMode::Ammo { full: true, .. }
                    | ScenarioMode::Ammo { absent: true, .. }
            );
            if !refused {
                // Bypass the connection's retained reply: a new owner must answer from
                // the durable receipt before selecting the now-moved source again.
                assert_eq!(
                    crate::gameplay_transport::hotkey_equip::execute(
                        &reloaded,
                        deps.actor,
                        crate::gameplay_transport::connection::UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: 1,
                            item_fence: Some(fence),
                        },
                        protocol::hotkey_equip::Request { definition: reference },
                        true,
                    )
                    .await,
                    Some(ItemMoveOutcome::Moved),
                );
            }
            let source_main: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(source_main, i64::from(refused));
            let source_revision: String = sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(source_revision, if refused { "1" } else { "2" });
            let receipts: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_equipment_receipts)+(SELECT count(*) FROM game_item_loop_move_receipts)").fetch_one(&harness.pool).await?;
            assert_eq!(receipts, i64::from(!refused));
            match mode {
                ScenarioMode::Hands {
                    both,
                    full,
                    incoming,
                } => {
                    assert_eq!(equipment.items.len(), if full { 2 } else { 1 });
                    if !full {
                        assert_eq!(equipment.items[0].item_instance_id, id(71));
                        assert_eq!(
                            equipment.items[0].slot,
                            if incoming == "oteryn:item.tibia.i3412" {
                                6
                            } else {
                                5
                            }
                        );
                    }
                    for seed in if both { vec![110, 111] } else { vec![110] } {
                        let old_main: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid")
                            .bind(id(seed).as_slice()).bind(id(70).as_slice()).fetch_one(&harness.pool).await?;
                        assert_eq!(old_main, i64::from(!full));
                        let revision: String = sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(seed).as_slice()).fetch_one(&harness.pool).await?;
                        assert_eq!(revision, if full { "1" } else { "2" });
                    }
                }
                ScenarioMode::Ammo { absent, .. } => {
                    assert_eq!(equipment.items.len(), usize::from(!absent));
                    let in_quiver: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid")
                        .bind(id(71).as_slice()).bind(id(110).as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(in_quiver, i64::from(!refused));
                }
                _ => return Err("swap mode".into()),
            }
            eprintln!("HOTKEYC_POSTGRES_EXECUTED elapsed={:?}", started_at.elapsed());
            return Ok(());
        }
        if extra_quiver {
            assert_eq!(equipment.items.len(), 2);
            assert!(
                equipment
                    .items
                    .iter()
                    .any(|item| item.slot == 5 && item.item_instance_id == id(100))
            );
            assert!(
                equipment
                    .items
                    .iter()
                    .any(|item| item.slot == 10 && item.item_instance_id == id(111))
            );
        } else if let Some(full) = occupied {
            assert_eq!(equipment.items.len(), 1);
            assert_eq!(
                equipment.items[0].item_instance_id,
                if full { id(110) } else { id(100) }
            );
            let revision:String=sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(100).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(revision, if full { "1" } else { "2" });
        } else if started {
            assert_eq!(equipment.items.len(), 1);
            assert_eq!(equipment.items[0].item_instance_id, id(72));
            assert_eq!(equipment.items[0].slot, 4);
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(receipts, 3);
        } else if feared || foreign {
            assert!(equipment.items.is_empty());
        } else {
            assert_eq!(equipment.items.len(), 1);
            assert_eq!(
                equipment.items[0].item_instance_id,
                if nested {
                    id(100)
                } else if spare {
                    id(72)
                } else {
                    id(71)
                }
            );
            assert_eq!(equipment.items[0].slot, 5);
        }
        let quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(item_key).fetch_one(&harness.pool).await?;
        assert_eq!(
            quantity,
            if started {
                3
            } else if spare {
                2
            } else {
                1
            }
        );
        if spare {
            let untouched: String = sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(untouched, "1");
            let receipts_for_worn: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND swap_item_instance_id IS NULL")
                .bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(receipts_for_worn, 3);
        }
        if nested {
            let evidence: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_equipment_nested_removals")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                evidence,
                if occupied == Some(true) || foreign {
                    0
                } else if managed {
                    2
                } else {
                    1
                }
            );
            if occupied != Some(true) && !foreign {
                let mut tx = harness.pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                    .execute(&mut *tx)
                    .await?;
                assert!(sqlx::query("UPDATE game_equipment_nested_removals SET placement_ordinal=placement_ordinal").execute(&mut *tx).await.is_err());
                tx.rollback().await?;
            }
        }
        if foreign {
            let unchanged: (String, i64) = sqlx::query_as("SELECT i.state_revision::text,(SELECT count(*) FROM game_item_nested_entries e WHERE e.item_instance_id=i.item_instance_id AND e.parent_item_instance_id=encode($2,'hex')::uuid) FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid")
                .bind(id(100).as_slice()).bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(unchanged, ("1".into(), 1));
        }
        eprintln!("PG_MARKER hotkey-real-route pouch={pouch} foreign={foreign} managed={managed} elapsed={:?}", started_at.elapsed());
        if coalescing {
            eprintln!(
                "PKG7_POSTGRES_EXECUTED: burst 3,4,5; zero cancelled receipts; replay 6; durable reload; PostgreSQL 18.6"
            );
        }
        drop(reloaded);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn equipped_count(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(pool).await?)
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    payload: &[u8],
    expected_status: protocol::CommandStatus,
) -> TestResult<ItemMoveOutcome> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: protocol::hotkey_equip::COMMAND_TYPE_HOTKEY_EQUIP,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = item_look::read_frame(client).await?;
        let e = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if e.message_type() == protocol::MessageType::CommandResult {
            let r = protocol::decode_command_result(e.payload()).map_err(debug)?;
            let outcome =
                protocol::item_view::decode_item_move_result_with_equip_drop(r.payload, true)
                    .map_err(debug)?;
            assert_eq!(r.status, expected_status);
            return Ok(outcome);
        }
    }
}

async fn drive<A, B>(
    serve: A,
    exercise: B,
) -> TestResult<crate::gameplay_transport::connection::ConnectionEnd>
where
    A: std::future::Future<Output = crate::gameplay_transport::connection::ConnectionEnd>,
    B: std::future::Future<Output = TestResult>,
{
    use std::task::Poll;
    let mut serve = std::pin::pin!(serve);
    let mut exercise = std::pin::pin!(exercise);
    let mut result = None;
    std::future::poll_fn(|cx| {
        if result.is_none()
            && let Poll::Ready(value) = exercise.as_mut().poll(cx)
        {
            if let Err(error) = value {
                return Poll::Ready(Err(error));
            }
            result = Some(());
        }
        if let Poll::Ready(end) = serve.as_mut().poll(cx) {
            return Poll::Ready(if result.is_some() {
                Ok(end)
            } else {
                Err(format!("connection ended before hotkey exercise: {end:?}").into())
            });
        }
        Poll::Pending
    })
    .await
}
