//! W19E: real use/use-with routes, seeded rust and lottery outcomes, custody and replay.
use super::rust_lottery_fixtures::exchange;

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
    run_scenario("oteryn:item.tibia.i6558", false, true)?;
    run_scenario("oteryn:item.tibia.i6558", false, false)
}

// Crystal 201189f8 rust_remover.lua:45–65. In descending threshold order, each
// entry is the last eligible result at its own threshold.
const SHIELD_AND_HELMET_ROLLS: &[(&str, &[(u64, u32)])] = &[
    (
        "oteryn:item.tibia.i8902",
        &[
            (3137, 3410),
            (2887, 3432),
            (929, 7460),
            (23, 3419),
            (10, 3434),
        ],
    ),
    (
        "oteryn:item.tibia.i8907",
        &[
            (2200, 3354),
            (1870, 3376),
            (1490, 3353),
            (1010, 3351),
            (190, 3385),
            (10, 3391),
        ],
    ),
    (
        "oteryn:item.tibia.i8908",
        &[
            (3156, 3353),
            (2976, 3351),
            (963, 3385),
            (210, 3391),
            (7, 3392),
        ],
    ),
];

#[derive(Clone, Copy)]
struct RustRoll {
    chance: u64,
    result: Option<(u64, u32)>,
}

impl RustRoll {
    fn matches(self, chance: u64, selection: u64) -> bool {
        chance + 1 == self.chance
            && self
                .result
                .is_none_or(|(rank, _)| selection % (rank + 1) == rank)
    }
}

#[test]
fn w19e_rust_shield_and_helmets_wire_commit_exact_source_rolls() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    for &(key, outcomes) in SHIELD_AND_HELMET_ROLLS {
        for (rank, &(chance, output)) in (0_u64..).zip(outcomes) {
            run_seeded_scenario(
                key,
                true,
                true,
                Some(RustRoll {
                    chance,
                    result: Some((rank, output)),
                }),
            )?;
        }
    }
    Ok(())
}

#[test]
fn w19e_rust_shield_and_helmets_wire_break_and_replay_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    for &(key, outcomes) in SHIELD_AND_HELMET_ROLLS {
        run_seeded_scenario(
            key,
            true,
            false,
            Some(RustRoll {
                chance: outcomes[0].0 + 1,
                result: None,
            }),
        )?;
    }
    Ok(())
}

fn run_scenario(key: &'static str, rust: bool, success: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run_seeded_scenario(key, rust, success, None)
}

fn run_seeded_scenario(
    key: &'static str,
    rust: bool,
    success: bool,
    roll: Option<RustRoll>,
) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(key, rust, success, roll).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "rust lottery e2e panic")??;
    Ok(())
}
fn scenario(key: &str, rust: bool, success: bool, roll: Option<RustRoll>) -> TestResult {
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
        let tool_quantity = if success || roll.is_some() { 2 } else { 1 };
        let entries = if rust {
            vec![(tool_key, tool_quantity), (key, 1)]
        } else {
            vec![(key, 1)]
        };
        super::item_look_fixtures::seed_main_backpack(&harness, &entries).await?;
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
        let session = *fence.game_session_id.as_bytes();
        let rust_draws = |source| {
            Some((
                reward_bag_bounded_draw(session, command_id, source, 1, 10_000)?,
                reward_bag_bounded_draw(session, command_id, source, 2, 420)?,
            ))
        };
        let plan = |source| {
            if rust {
                let (chance, selection) = rust_draws(source)?;
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
        let source = (1_u32..if roll.is_some() { 1_000_000 } else { 100_000 })
            .map(|nonce| {
                let mut source = seeded_source;
                source[12..].copy_from_slice(&nonce.to_be_bytes());
                source
            })
            .find(|source| {
                if let Some(roll) = roll {
                    return rust_draws(*source)
                        .is_some_and(|(chance, selection)| roll.matches(chance, selection));
                }
                plan(*source).is_some_and(|p| {
                    if key == "oteryn:item.tibia.i6558" {
                        p.outputs.first().is_some_and(|(definition, _)| {
                            definition.production_key
                                == if success {
                                    "oteryn:item.tibia.i236"
                                } else {
                                    "oteryn:item.tibia.i237"
                                }
                        })
                    } else {
                        p.outputs.is_empty() != success
                    }
                })
            })
            .ok_or("seeded outcome")?;
        let (expected_recipe, expected_outputs) = if let Some(roll) = roll {
            let id = key
                .strip_prefix("oteryn:item.tibia.i")
                .ok_or("rusty source key")?;
            (
                format!("oteryn:rust-remover.i{id}.v1"),
                roll.result
                    .into_iter()
                    .map(|(_, output)| (format!("oteryn:item.tibia.i{output}"), 1))
                    .collect::<Vec<_>>(),
            )
        } else {
            let expected = plan(source).ok_or("plan")?;
            (
                expected.recipe,
                expected
                    .outputs
                    .into_iter()
                    .map(|(definition, quantity)| (definition.production_key, quantity))
                    .collect(),
            )
        };
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
            "{key} rust={rust} success={success} expected={expected_outputs:?}"
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
        let mut expected_keys: Vec<_> = expected_outputs
            .iter()
            .map(|(key, quantity)| (key.as_str(), *quantity))
            .collect();
        if rust && tool_quantity > 1 {
            expected_keys.push((tool_key, u32::try_from(tool_quantity - 1)?));
        }
        keys.sort_unstable();
        expected_keys.sort_unstable();
        assert_eq!(keys, expected_keys);
        let (recipe, outputs, before, hash_matches): (String, serde_json::Value, serde_json::Value, bool) = sqlx::query_as("SELECT r.recipe,r.outputs,r.before_secondary,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id)").fetch_one(&harness.pool).await?;
        assert_eq!(recipe, expected_recipe);
        let outputs = outputs.as_array().ok_or("outputs")?;
        assert_eq!(
            outputs.len(),
            usize::from(success || key == "oteryn:item.tibia.i6558")
        );
        for (output, (key, quantity)) in outputs.iter().zip(&expected_outputs) {
            assert_eq!(output["key"].as_str(), Some(key.as_str()));
            assert_eq!(output["count"].as_u64(), Some(u64::from(*quantity)));
        }
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
        // Re-enter the real connection with the post-use inventory, then replay the wire command.
        let (_, replayed, _) = exchange(&owner, end, command_id, &actual, None, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
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
        if key == "oteryn:item.tibia.i6558" {
            super::achievement_granters::assert_progress(
                &harness,
                "oteryn:achievement/demonic_barkeeper",
                1,
                false,
                "oteryn:consume-transform",
            )
            .await?;
            eprintln!(
                "SRX1_PG demonic blood wire/transform/replay/achievement elapsed={:?}",
                started.elapsed()
            );
        }
        eprintln!("POSTGRES_CASE_COMPLETED P11 Polisher rust={rust} success={success} replay");
        let elapsed = started.elapsed();
        assert!(elapsed > std::time::Duration::ZERO);
        eprintln!(
            "W19E_PG_EXECUTED rust={rust} success={success} command={command_id} elapsed={elapsed:?} item={key} outputs={expected_outputs:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
