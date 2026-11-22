//! M09: ordinary rod and bait reach the production wire and durable consume/mint owner.
use super::fishing_wire::{GatheringCase, gathering_scenario};

use super::fixture::{Harness, debug, id, run};
use super::item_craft_fixtures::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn m09_standard_fishing_wire_consumes_bait_and_replays_without_duplicate_fish() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "fishing e2e panic")??;
    eprintln!(
        "M09_PG_CASE_EXECUTED standard_fishing elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "standard_fishing", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
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
        let water = "oteryn:item.tibia.i4597";
        let bag = "oteryn:item.tibia.i2854";
        let rod = "oteryn:item.tibia.i3483";
        for key in [
            rod,
            water,
            "oteryn:item.tibia.i3492",
            "oteryn:item.tibia.i3578",
        ] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native fishing profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (water, reference(water).ok_or("water")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
        };
        // The production boot policy: an unknown fact blocks and is not pickupable.
        let definitions = |key: &str| {
            let policy = native.item_policy(key, "definition-r1").map(|p| p.record());
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.and_then(|p| p.attributes.blocks_movement),
                blocks_projectile: policy.and_then(|p| p.attributes.blocks_projectile).unwrap_or(true),
                pickupable: policy.is_some_and(|p| matches!(&p.semantics.physical, crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true))),
                door: false,
            })
        };
        let map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
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
        super::item_look_fixtures::seed_main_backpack(&harness, &[(rod, 1), ("oteryn:item.tibia.i3492", 3)])
            .await?;
        super::achievement_granters::seed_gathering(&harness).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let mut placement = map.base().digest().to_vec();
        let target_pos = crate::map::overlay::TilePos {
            x: 7,
            y: 0,
            floor: -7,
        };
        placement.extend_from_slice(
            &crate::map::view::placement_key(target_pos, 1)
                .ok_or("target placement")?
                .to_be_bytes(),
        );
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The wire helper resolves the current rod handle from the inventory snapshot.
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target.clone())),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let mut caught = None;
        for command_id in 1..=128 {
            let (next, result, actual) =
                exchange(&owner, admitted, command_id, &payload, reference(rod)).await?;
            admitted = next;
            match result {
                UseDisposition::Committed => {
                    caught = Some((command_id, actual));
                    break;
                }
                UseDisposition::NothingToUse => {
                    assert_eq!(
                        sqlx::query_scalar::<_, i64>(
                            "SELECT count(*) FROM game_item_craft_records"
                        )
                        .fetch_one(&harness.pool)
                        .await?,
                        0
                    );
                    let bait: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3492'").fetch_one(&harness.pool).await?;
                    assert_eq!(bait, 3);
                    super::achievement_granters::assert_gathering(&harness, None).await?;
                }
                other => return Err(format!("unexpected fishing refusal {other:?}").into()),
            }
        }
        let (command_id, actual) =
            caught.ok_or("no catch in the bounded deterministic command sequence")?;
        for (key, quantity) in [
            (rod, 1_i64),
            ("oteryn:item.tibia.i3492", 2),
            ("oteryn:item.tibia.i3578", 1),
        ] {
            let actual: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&harness.pool).await?;
            assert_eq!(actual, quantity, "{key}");
        }
        let achievement = super::achievement_granters::assert_gathering(
            &harness,
            Some("oteryn:achievement/here_fishy_fishy"),
        )
        .await?;
        let (_, replay, _) = exchange(&owner, admitted, command_id, &actual, None).await?;
        assert_eq!(replay, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        let fish: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3578' AND lifecycle=1").fetch_one(&harness.pool).await?;
        assert_eq!(fish, 1);
        assert_eq!(
            achievement,
            super::achievement_granters::assert_gathering(
                &harness,
                Some("oteryn:achievement/here_fishy_fishy"),
            )
            .await?
        );
        eprintln!("POSTGRES_CASE_COMPLETED P11 standard fishing grant and replay");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn m09_gathering_wire_mutates_corpse_and_grants_inventory_once() -> TestResult {
    run_gathering(5908, 5969, Some(4012), Some(5878), true)
}
#[test]
fn m09_gathering_wire_failed_skinning_marks_corpse() -> TestResult {
    run_gathering(5908, 5969, Some(4012), None, true)
}
#[test]
fn m09_gathering_wire_marble_tier_removes_rock_and_grants_figurine() -> TestResult {
    run_gathering(5908, 10426, None, Some(10428), true)
}
#[test]
fn m09_gathering_wire_dusting_grants_demon_dust() -> TestResult {
    run_gathering(5942, 5995, Some(4098), Some(5906), true)
}
#[test]
fn m09_gathering_wire_ice_sculpture_stays_on_map() -> TestResult {
    run_gathering(5908, 7441, Some(7442), None, true)
}
#[test]
fn m09_gathering_wire_failed_ice_removes_block() -> TestResult {
    run_gathering(5942, 7441, None, None, true)
}
#[test]
fn m09_gathering_wire_ice_hole_catches_rare_fish() -> TestResult {
    run_gathering(3483, 7236, Some(7237), Some(7158), true)
}

#[test]
fn m09_gathering_wire_mechanical_sewer_water_reaches_complete_classifier() -> TestResult {
    run_gathering(9306, 4597, Some(4609), Some(9307), true)
}
#[test]
fn m09_gathering_wire_mechanical_water_outside_sewers_uses_a_worm() -> TestResult {
    run_gathering_at(9306, 4597, Some(4609), Some(3578), true, false)
}
#[test]
fn m09_gathering_wire_failed_catch_keeps_hole_and_bait() -> TestResult {
    run_gathering(3483, 7236, Some(7236), None, false)
}

#[test]
fn p11_ice_fishing_second_rare_species_grants_exquisite_taste() -> TestResult {
    run_gathering(3483, 7236, Some(7237), Some(3580), true)
}
#[test]
fn p11_ice_fishing_third_rare_species_grants_exquisite_taste() -> TestResult {
    run_gathering(3483, 7236, Some(7237), Some(7159), true)
}
#[test]
fn p11_ice_fishing_common_fish_counts_here_fishy_fishy() -> TestResult {
    run_gathering(3483, 7236, Some(7237), Some(3578), true)
}
#[test]
fn p11_mechanical_ice_fishing_grants_exquisite_taste() -> TestResult {
    run_gathering(9306, 7236, Some(7237), Some(7158), true)
}
#[test]
fn p11_failed_dusting_marks_corpse_without_advancing_ashes_to_dust() -> TestResult {
    run_gathering(5942, 5995, Some(4098), None, true)
}

fn run_gathering(
    tool: u16,
    target: u16,
    after: Option<u16>,
    yield_item: Option<u16>,
    committed: bool,
) -> TestResult {
    run_gathering_at(tool, target, after, yield_item, committed, true)
}

fn run_gathering_at(
    tool: u16,
    target: u16,
    after: Option<u16>,
    yield_item: Option<u16>,
    committed: bool,
    in_sewers: bool,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            gathering_scenario(GatheringCase {
                tool,
                target_id: target,
                after,
                yield_item,
                committed,
                in_sewers,
                select_roll: if tool == 9306 && !in_sewers {
                    Some(|catch, _| catch <= 10_000)
                } else {
                    None
                },
                consumes_bait: matches!(tool, 3483 | 9306) && committed,
            })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "gathering e2e panic")??;
    eprintln!(
        "M09_PG_CASE_EXECUTED tool={tool} target={target} yield={yield_item:?} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
