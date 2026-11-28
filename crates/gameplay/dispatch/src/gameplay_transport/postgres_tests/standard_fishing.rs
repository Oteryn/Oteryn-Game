//! M09: ordinary rod and bait reach the production wire and durable consume/mint owner.
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
fn cach1_final_marble_sculpture_grants_marblelous_and_marble_madness() -> TestResult {
    run_gathering(5908, 10426, None, Some(10429), true)
}
#[test]
fn cach1_final_ice_sculpture_grants_ice_sculptor_and_cold_as_ice() -> TestResult {
    run_gathering(5908, 7445, Some(7446), None, true)
}
#[test]
fn cach1_blessed_stake_finishes_ice_and_grants_both_achievements() -> TestResult {
    run_gathering(5942, 7445, Some(7446), None, true)
}
#[test]
fn cach1_blessed_stake_finishes_marble_and_grants_both_achievements() -> TestResult {
    run_gathering(5942, 10426, None, Some(10429), true)
}
#[test]
fn cach1_ice_sculpture_admits_the_third_stage_without_an_award() -> TestResult {
    run_gathering(5908, 7442, Some(7444), None, true)
}
#[test]
fn cach1_ice_sculpture_admits_the_fourth_stage_without_an_award() -> TestResult {
    run_gathering(5908, 7444, Some(7445), None, true)
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
fn m09_gathering_wire_mechanical_water_outside_sewers_returns_nothing_to_use() -> TestResult {
    run_gathering_at(9306, 4597, Some(4597), None, false, false, SculptureSetup::Base)
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

#[test]
fn review_carried_ice_consumes_the_material_without_minting() -> TestResult {
    for tool in [5942, 5908] {
        for (before, after) in [(7441, 7442), (7442, 7444), (7444, 7445), (7445, 7446)] {
            run_gathering_at(tool, before, Some(after), None, true, false, SculptureSetup::Carried)?;
        }
        run_gathering_at(tool, 10426, None, Some(10429), true, false, SculptureSetup::Carried)?;
    }
    Ok(())
}

#[test]
fn review_all_marble_tiers_keep_the_sculptor_inscription() -> TestResult {
    for tool in [5908, 5942] {
        for figurine in [10427, 10428, 10429] {
            run_gathering(tool, 10426, None, Some(figurine), true)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SculptureSetup {
    Base,
    Carried,
    NestedMaterial,
    FullBackpackNestedMaterial,
    FullBackpackNestedDropped,
    FullBackpackNestedTerrain,
    ObtainBag,
    ConfiguredObtain,
    FullBackpackQuiver,
    FullBackpackConfiguredQuiver,
    FullBackpackDirectMaterial,
    NestedTool,
    Dropped,
    DroppedIceLostAck,
    FullBackpack,
    Overweight,
    Offered,
}

#[test]
fn review_nested_sculpture_material_commits() -> TestResult {
    run_gathering_at(5908, 10426, None, Some(10429), true, false, SculptureSetup::NestedMaterial)
}

#[test]
fn review_sculpture_prefers_configured_obtain_container() -> TestResult {
    run_gathering_at(5908, 10426, None, Some(10429), true, false, SculptureSetup::ConfiguredObtain)
}

#[test]
fn review_sculpture_skips_configured_quiver_before_dropping() -> TestResult {
    run_gathering_at(5942, 10426, None, Some(10429), true, false, SculptureSetup::FullBackpackConfiguredQuiver)
}

#[test]
fn review_full_backpack_nested_marble_uses_nested_space() -> TestResult {
    for tool in [5908, 5942] {
        for setup in [
            SculptureSetup::FullBackpackNestedMaterial,
            SculptureSetup::FullBackpackNestedDropped,
            SculptureSetup::FullBackpackNestedTerrain,
        ] {
            run_gathering_at(tool, 10426, None, Some(10429), true, false, setup)?;
        }
    }
    Ok(())
}

#[test]
fn review_marble_prefers_assigned_obtain_bag() -> TestResult {
    run_gathering_at(5908, 10426, None, Some(10429), true, false, SculptureSetup::ObtainBag)
}

#[test]
fn review_marble_skips_ammunition_only_quiver() -> TestResult {
    run_gathering_at(5908, 10426, None, Some(10429), true, false, SculptureSetup::FullBackpackQuiver)
}

#[test]
fn review_full_backpack_direct_marble_drops_figurine() -> TestResult {
    for tool in [5908, 5942] {
        run_gathering_at(
            tool, 10426, None, Some(10429), true, false, SculptureSetup::FullBackpackDirectMaterial,
        )?;
    }
    Ok(())
}

#[test]
fn review_nested_sculpture_tool_commits() -> TestResult {
    run_gathering_at(5942, 10426, None, Some(10429), true, false, SculptureSetup::NestedTool)
}

#[test]
fn review_dropped_sculpture_material_commits() -> TestResult {
    run_gathering_at(5942, 10426, None, Some(10429), true, false, SculptureSetup::Dropped)
}

#[test]
fn dropped_ice_browse_reconciles_lost_sculpture_ack() -> TestResult {
    run_gathering_at(5908, 7441, Some(7442), None, true, false, SculptureSetup::DroppedIceLostAck)
}

#[test]
fn review_full_backpack_sculpture_drops_figurine() -> TestResult {
    run_gathering_at(5908, 10426, None, Some(10429), true, false, SculptureSetup::FullBackpack)
}

#[test]
fn review_overweight_sculpture_drops_figurine() -> TestResult {
    run_gathering_at(5942, 10426, None, Some(10429), true, false, SculptureSetup::Overweight)
}

#[test]
fn review_offered_sculpture_material_is_refused() -> TestResult {
    run_gathering_at(5942, 10426, None, Some(10429), true, false, SculptureSetup::Offered)
}

fn run_gathering(
    tool: u16,
    target: u16,
    after: Option<u16>,
    yield_item: Option<u16>,
    committed: bool,
) -> TestResult {
    run_gathering_at(tool, target, after, yield_item, committed, true, SculptureSetup::Base)
}

fn run_gathering_at(
    tool: u16,
    target: u16,
    after: Option<u16>,
    yield_item: Option<u16>,
    committed: bool,
    in_sewers: bool,
    setup: SculptureSetup,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            gathering_scenario(
                tool, target, after, yield_item, committed, in_sewers, setup,
            )
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

fn gathering_scenario(
    tool: u16,
    target_id: u16,
    after: Option<u16>,
    yield_item: Option<u16>,
    committed: bool,
    in_sewers: bool,
    setup: SculptureSetup,
) -> TestResult {
    let full_nested = matches!(
        setup,
        SculptureSetup::FullBackpackNestedMaterial
            | SculptureSetup::FullBackpackNestedDropped
            | SculptureSetup::FullBackpackNestedTerrain
    );
    let nested = matches!(
        setup,
        SculptureSetup::FullBackpackNestedMaterial
            | SculptureSetup::NestedMaterial
            | SculptureSetup::ConfiguredObtain
            | SculptureSetup::NestedTool
    );
    let dropped = matches!(
        setup,
        SculptureSetup::Dropped | SculptureSetup::FullBackpackNestedDropped | SculptureSetup::DroppedIceLostAck
    );
    let carried = nested
        || matches!(
            setup,
            SculptureSetup::Carried
                | SculptureSetup::Offered
                | SculptureSetup::FullBackpackDirectMaterial
        );
    let item_target = carried || dropped;
    let fallback = matches!(
        setup,
        SculptureSetup::FullBackpack
            | SculptureSetup::FullBackpackDirectMaterial
            | SculptureSetup::Overweight
            | SculptureSetup::FullBackpackQuiver
            | SculptureSetup::FullBackpackConfiguredQuiver
    );
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "m09_gathering", true, false).await?;
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
        let water_key = format!("oteryn:item.tibia.i{target_id}");
        let water = water_key.as_str();
        let bag = "oteryn:item.tibia.i2854";
        let tool_key = format!("oteryn:item.tibia.i{tool}");
        let rod = tool_key.as_str();
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
        let sewer = in_sewers
            && tool == 9306
            && oteryn_gameplay_items::fishing::MECHANICAL_FISHING_WATER.contains(&target_id);
        let bait_key = if sewer {
            oteryn_gameplay_items::fishing::NAIL
        } else {
            oteryn_gameplay_items::fishing::WORM
        };
        let start = crate::map::overlay::TilePos {
            x: if sewer { 32742 } else { 6 },
            y: if sewer { 31168 } else { 0 },
            floor: if sewer { -8 } else { -7 },
        };
        let (mut bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (water, reference(water).ok_or("water")?.get() - 1),
        );
        if sewer {
            // Place the fixture inside Crystal's Yalahar sewer rectangle. The original
            // room at (7, 0, -7) cannot reach the location-qualified mechanical branch.
            let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
            data.manifest.world.min_x += 32736;
            data.manifest.world.max_x += 32736;
            data.manifest.world.min_y += 31168;
            data.manifest.world.max_y += 31168;
            data.manifest.world.floors = vec![-8];
            for sector in &mut data.sectors {
                sector.floor = -8;
                sector.sx += 32736 / 32;
                sector.sy += 31168 / 32;
                for tile in &mut sector.tiles {
                    tile.x += 32736;
                    tile.y += 31168;
                }
            }
            bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
                &data.manifest,
                &data.sectors,
                &data.spawns,
                &data.npcs,
            )?;
            pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        }
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
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
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
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
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    floor: -i16::from(start.floor),
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
        let supplies = if item_target { (water, 1) } else { (bait_key, 3) };
        let mut entries = vec![(rod, 1), supplies];
        match setup {
            SculptureSetup::FullBackpackNestedMaterial | SculptureSetup::FullBackpackNestedDropped | SculptureSetup::FullBackpackNestedTerrain => {
                entries.push((bag, 1));
                // Moving an instance target out of the main backpack frees a slot.
                entries.resize(20 + usize::from(item_target), ("oteryn:item.tibia.i3031", 1));
            }
            SculptureSetup::ObtainBag => entries.extend([(bag, 1), (bag, 1)]),
            SculptureSetup::FullBackpackQuiver | SculptureSetup::FullBackpackConfiguredQuiver => {
                entries.push(("oteryn:item.tibia.i35562", 1));
                entries.resize(20, ("oteryn:item.tibia.i3031", 1));
            }
            SculptureSetup::FullBackpack | SculptureSetup::FullBackpackDirectMaterial => entries.resize(20, ("oteryn:item.tibia.i3031", 1)),
            SculptureSetup::Overweight => entries.resize(10, ("oteryn:item.tibia.i3578", 100)),
            SculptureSetup::ConfiguredObtain => entries.extend([(bag, 1), (bag, 1)]),
            SculptureSetup::NestedMaterial | SculptureSetup::NestedTool => entries.push((bag, 1)),
            _ => {}
        }
        let source_handle = if (fallback && !carried)
            || matches!(setup, SculptureSetup::FullBackpackNestedTerrain | SculptureSetup::FullBackpackDirectMaterial | SculptureSetup::ObtainBag)
        {
            u64::try_from(entries.len())? + 1
        } else {
            3
        };
        let target_handle = if setup == SculptureSetup::FullBackpackDirectMaterial {
            source_handle - 1
        } else {
            2
        };
        super::item_look_fixtures::seed_main_backpack(&harness, &entries).await?;
        if nested {
            let item = if setup == SculptureSetup::NestedTool { 71 } else { 72 };
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            sqlx::query("WITH moved AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid RETURNING *) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,placed_transaction_id FROM moved")
                .bind(id(item).as_slice()).bind(id(73).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if full_nested && !dropped {
            let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
                .fetch_one(&harness.pool).await?;
            assert_eq!(occupied, 20);
        }
        super::achievement_granters::seed_gathering(&harness).await?;
        let sculpture = match target_id {
            10426 => Some(("marblelous", "marble_madness", 5, yield_item == Some(10429))),
            7441 | 7442 | 7444 | 7445 => Some(("ice_sculptor", "cold_as_ice", 10, after == Some(7446))),
            _ => None,
        };
        if let Some((_, counted, total, _)) = sculpture {
            super::achievement_granters::seed_progress(
                &harness,
                &format!("oteryn:achievement/{counted}"),
                total - 1,
            )
            .await?;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        let premium = crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            consumer, Some(clock), Some(activation.clone()), Some(harness.root.clone()),
        );
        if matches!(setup, SculptureSetup::ObtainBag | SculptureSetup::ConfiguredObtain | SculptureSetup::FullBackpackConfiguredQuiver) {
            use oteryn_protocol_oteryn::quick_loot::{
                QuickLootAction, QuickLootCategory, QuickLootRecipient, QuickLootRequest, encode_quick_loot_request,
            };
            harness.root.latch_premium_activation(
                &activation.activation_id, activation.switch_over_us, activation.switch_over_us + 60_000_000,
            ).await.map_err(debug)?;
            owner.runtime.lock().await.bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                ).map_err(debug)?,
            ).map_err(debug)?;
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), admitted.game_session_id);
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18, 98, 607])
                    .ok_or("craft and Obtain offers selectable")?;
            let request = |action, recipient, category| QuickLootRequest {
                action,
                corpse: None,
                selected_item: None,
                review_filters: false,
                category,
                recipient: Some(QuickLootRecipient::Container),
                recipient_id: Some(id(recipient)),
                fallback_to_main: false,
                filter_mode: None,
                filter_definition_ref: None,
            };
            let mut requests = vec![request(
                QuickLootAction::AssignObtain, 73,
                (setup == SculptureSetup::ConfiguredObtain).then_some(QuickLootCategory::Decoration),
            )];
            if setup != SculptureSetup::FullBackpackConfiguredQuiver {
                requests.push(request(QuickLootAction::AssignLoot, 74, None));
            }
            let commands = requests.iter().map(|request| {
                Ok((request.action.command_type(), encode_quick_loot_request(request).map_err(debug)?))
            }).collect::<TestResult<Vec<_>>>()?;
            let (replies, _) = super::parameter_spell_fixtures::exchange(
                &owner, &mut admitted, &commands, false, false,
            ).await?;
            assert_eq!(replies.len(), commands.len());
            for ((kind, _), (status, _)) in commands.iter().zip(&replies) {
                assert_eq!(*status, crate::foundation::CommandStatus::Accepted, "quick-loot command {kind}");
            }
        }
        let mut placement = map.base().digest().to_vec();
        let target_pos = crate::map::overlay::TilePos {
            x: start.x + 1,
            y: start.y,
            floor: start.floor,
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
        if dropped {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            let ground = crate::durability::map_item_mint_audit::map_item_spatial_position(target_pos.x, target_pos.y, target_pos.floor);
            sqlx::query("WITH moved AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid RETURNING *) INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,$3::text::numeric,$4,convert_to('OTERYN_ITEM_LOOP_DROP/v1:fixture','UTF8'),$5,$6,$7,1 FROM moved")
                .bind(id(72).as_slice()).bind(id(43).as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                .bind(ground).bind(crate::map::overlay::map_revision(map.base()))
                .bind(format!("sha256:{}", native.source_digest().iter().map(|b| format!("{b:02x}")).collect::<String>()))
                .bind(pin.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            let ground = crate::gameplay_transport::loop_move::loop_ground(&owner, map.base(), Some(index))
                .await.ok_or("ground projection")?;
            assert!(ground.ground_entry(&id(72)).is_some());
            if full_nested {
                let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
                    .fetch_one(&harness.pool).await?;
                assert_eq!(occupied, 20);
            }
        }
        let observer = if setup == SculptureSetup::DroppedIceLostAck {
            let evidence = super::house_transfer_fixtures::RecoverySource::start(&harness).await?;
            let session = super::boss_rewards_fixtures::privacy::admit_gameplay_peer(
                &harness, &evidence.evidence, 1,
            ).await?;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                harness.root.clone(),
            ).current_session_at(session).await?.0;
            let peer_fence = crate::gameplay_transport::item_fence_of(current).ok_or("observer fence")?;
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
            let at = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
            runtime.initialize_pinned_test_position(peer, at).map_err(debug)?;
            Some((peer, peer_fence))
        } else {
            None
        };
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(if item_target {
                MapUseTarget::Item(std::num::NonZeroU64::new(2).ok_or("target handle")?)
            } else {
                MapUseTarget::Object(target.clone())
            }),
            use_with_position: None,
        };
        // A pickup may already be prepared when the tool changes/removes the map item.
        let mut stale_pickup = if target_id == 7441 && !item_target {
            let definition = crate::gameplay_transport::item_look::typed_definition(
                index,
                reference(water).ok_or("ice")?,
            )
            .ok_or("ice definition")?;
            let request = crate::durability::map_item_mint::MapItemMintRequest {
                flat_crate: None,
                key_number: None,
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(999).map_err(debug)?,
                ),
                base_bundle_digest: map.base().digest(),
                placement_key: crate::map::view::placement_key(target_pos, 1).ok_or("placement")?,
                reset_epoch: 0,
                item: crate::gameplay_transport::item_ref_admission::generation_item_facts(
                    native,
                    &definition,
                )
                .map_err(debug)?,
                quantity: 1,
                content_revision: format!(
                    "sha256:{}",
                    native
                        .source_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                    .into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            };
            Some(
                owner
                    .root
                    .freeze_map_item_mint(owner.character, owner.holder, fence, request)
                    .await
                    .map_err(debug)?,
            )
        } else {
            None
        };
        eprintln!(
            "FIXM09_PG_START tool={tool} target={target_id} after={after:?} yield={yield_item:?}"
        );
        let started = std::time::Instant::now();
        let session = admitted.game_session_id;
        let command_id = admitted.continuity.next_command_id;
        // Resume after a valid prior handle history. The next inventory snapshot issues the
        // carried tool at the third handle, as in the existing fishing fixture. The carried
        // sculpture material is issued the second handle, ahead of the tool.
        let counter = (admitted.continuity.item_view.handle_counter..).take(100_000)
            .find(|counter| {
                let mut candidate = intent.clone();
                candidate.target = UseTarget::Item(
                    std::num::NonZeroU64::new(counter + source_handle).unwrap_or(std::num::NonZeroU64::MAX),
                );
                if item_target {
                    candidate.use_with_map_item =
                        std::num::NonZeroU64::new(counter + target_handle).map(MapUseTarget::Item);
                }
                let Ok(actual) = encode_item_use_intent(&candidate) else {
                    return false;
                };
                use sha2::{Digest, Sha256};
                let digest = Sha256::new()
                    .chain_update(b"oteryn.fishing.catch.v1")
                    .chain_update(session.as_bytes())
                    .chain_update(command_id.to_be_bytes())
                    .chain_update(&actual)
                    .finalize();
                let mut occurrence = [0; 16];
                occurrence.copy_from_slice(&digest[..16]);
                let draw = oteryn_simulation_determinism::deterministic_decision_u64(
                    &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                        native.source_digest(),
                    ),
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),
                    "fishing.catch",
                    0,
                )
                .ok();
                let catch_roll = draw
                    .map(|draw| {
                        u32::try_from(crate::spell::uniform_draw(draw, 1, 100_000)).unwrap_or(0)
                    })
                    .unwrap_or(0);
                let roll = crate::gameplay_transport::item_craft::gathering_roll(
                    native.source_digest(),
                    session,
                    command_id,
                    &actual,
                )
                .unwrap_or(0);
                // Exercise the wire at a donor-qualified roll outside the old Crystal tiers.
                match (target_id, after, yield_item) {
                    (7442 | 7444 | 7445, Some(_), None) => return (10_000..=20_000).contains(&roll),
                    (10426, None, Some(10429)) => return (1_000..=10_000).contains(&roll),
                    (10426, None, Some(10428)) => return (10_001..=26_764).contains(&roll),
                    _ => {}
                }
                let attempt = oteryn_gameplay_items::fishing::gathering_attempt(
                    tool, target_id, 10, catch_roll, roll,
                );
                if !committed {
                    attempt.is_none()
                } else {
                    attempt.is_some_and(|p| p.after == after && p.yield_item == yield_item)
                }
            })
            .ok_or("deterministic handle history exists")?;
        let mut intent = intent;
        if item_target {
            intent.use_with_map_item =
                std::num::NonZeroU64::new(counter + target_handle).map(MapUseTarget::Item);
        }
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        admitted.continuity.item_view.handle_counter = counter;
        let (next, result, actual) = if nested || dropped || setup == SculptureSetup::Offered {
            use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
            if setup == SculptureSetup::Offered {
                let binding = owner.runtime.lock().await.binding();
                owner.states.player_trades.seed_offered_for_test(binding, session, crate::foundation::GameSessionId::decode(&id(84)).map_err(debug)?, id(72))
                    .ok_or("offered trade fixture")?;
            }
            intent.target = UseTarget::Item(std::num::NonZeroU64::new(counter + source_handle).ok_or("source handle")?);
            let actual = encode_item_use_intent(&intent).map_err(debug)?;
            let craft = owner.use_craft(
                deps.actor,
                UseCommand { game_session_id: session, command_id, item_fence: Some(fence) },
                intent.clone(),
                Some(ItemKey::Instance(id(71))),
                Some(ItemKey::Instance(id(72))),
            );
            let result = if let Some((peer, peer_fence)) = observer {
                use crate::gameplay_transport::container_view::ContainerObservation;
                let field = ItemKey::BrowseField {
                    x: i32::from(target_pos.x),
                    y: i32::from(target_pos.y),
                    floor: -i16::from(target_pos.floor),
                };
                let first = owner.observe_container(peer, Some(peer_fence), field).await;
                let Some(ContainerObservation::BrowseField { entries, .. }) = first else {
                    return Err("second session could not browse dropped ice".into());
                };
                assert!(entries.iter().any(|entry| entry.key == ItemKey::Instance(id(72))));
                assert!(crate::durability::item_craft::lose_ground_sculpture_ack(craft).await.is_none());
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                        .fetch_one(&harness.pool).await?,
                    1,
                    "only the acknowledgement was lost",
                );
                if owner.root.has_ready_demand() {
                    owner.root.maintain_ready_once().await.map_err(debug)?;
                }
                let observed = owner.observe_container(peer, Some(peer_fence), field).await;
                let Some(ContainerObservation::BrowseField { entries, .. }) = observed else {
                    return Err("second session could not browse after lost acknowledgement".into());
                };
                assert!(!entries.iter().any(|entry| entry.key == ItemKey::Instance(id(72))));
                assert!(entries.iter().any(|entry| entry.appearance_id == 7442));
                let (_, replay, _) = exchange(&owner, admitted, command_id, &actual, None).await?;
                assert_eq!(replay, UseDisposition::Committed);
                drop(owner);
                drop(authority);
                drop(seal);
                return harness.cleanup().await;
            } else {
                craft.await
            }.ok_or("sculpture dispatch returned no outcome");
            if setup == SculptureSetup::Offered {
                assert_eq!(result?.disposition, UseDisposition::Rejected);
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 0);
                assert!(!owner.states.player_trades.open(session));
                drop(owner);
                drop(authority);
                drop(seal);
                return harness.cleanup().await;
            }
            (admitted, result?.disposition, actual)
        } else if carried {
            super::item_craft_fixtures::exchange_carried(
                &owner,
                admitted,
                command_id,
                &payload,
                reference(rod),
                reference(water),
            )
            .await?
        } else {
            exchange(&owner, admitted, command_id, &payload, reference(rod)).await?
        };
        let actual_intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            &[4, 6, 11, 15, 18],
            &actual,
        )
        .map_err(debug)?;
        assert_eq!(
            actual_intent.target,
            UseTarget::Item(std::num::NonZeroU64::new(counter + source_handle).ok_or("issued source handle")?)
        );
        if carried {
            assert_eq!(actual_intent.use_with_map_item, intent.use_with_map_item);
        }
        assert_eq!(
            result,
            if committed {
                UseDisposition::Committed
            } else {
                UseDisposition::NothingToUse
            }
        );
        let state: Option<i64> =
            sqlx::query_scalar("SELECT state_item_id FROM game_terrain_tool_transforms")
                .fetch_optional(&harness.pool)
                .await?;
        assert_eq!(
            state,
            (committed && !item_target).then(|| after.map_or(0, i64::from))
        );
        let map = owner.bundle_map().ok_or("bundle map")?;
        let tile = crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos)
            .map_err(debug)?
            .ok_or("target tile")?;
        if let Some(appearance) = if item_target { Some(target_id) } else { after } {
            assert!(
                tile.entries
                    .iter()
                    .any(|e| e.facts.appearance_id == appearance)
            );
        } else {
            assert!(!tile.entries.iter().any(|e| e.source.key()
                == crate::map::view::MapEntryKey::Base(
                    crate::map::view::placement_key(target_pos, 1).unwrap_or(0)
                )));
        }
        let live_yield: Option<i64> = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(format!("oteryn:item.tibia.i{}", yield_item.unwrap_or(5878))).fetch_one(&harness.pool).await?;
        assert_eq!(live_yield, yield_item.map(|_| 1));
        if carried && let Some(next_stage) = after {
            let minted: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
            ).bind(format!("oteryn:item.tibia.i{next_stage}")).fetch_one(&harness.pool).await?;
            assert_eq!(minted, 0, "a carried position has no map tile for its next ice stage");
        }
        if full_nested || matches!(setup, SculptureSetup::ObtainBag | SculptureSetup::ConfiguredObtain) {
            let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(remaining, 0);
            let nested_outputs: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id)
                 WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key=$2",
            )
            .bind(id(73).as_slice())
            .bind(format!("oteryn:item.tibia.i{}", yield_item.ok_or("nested figurine")?))
            .fetch_one(&harness.pool).await?;
            assert_eq!(nested_outputs, 1, "use the selected nested bag");
        }
        if carried && after.is_some() {
            let state: i16 = sqlx::query_scalar(
                "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(72).as_slice())
            .fetch_one(&harness.pool).await?;
            assert_eq!(state, 2, "the carried ice stage is consumed");
        }
        if fallback {
            let ground_outputs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1 AND g.spatial_position=$2")
                .bind(format!("oteryn:item.tibia.i{}", yield_item.ok_or("fallback figurine")?))
                .bind(crate::durability::map_item_mint_audit::map_item_spatial_position(start.x, start.y, start.floor))
                .fetch_one(&harness.pool).await?;
            assert_eq!(ground_outputs, 1);
        }
        if let Some(10427..=10429) = yield_item {
            let text: Option<String> = sqlx::query_scalar("SELECT t.text FROM game_item_texts t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1")
                .bind(format!("oteryn:item.tibia.i{}", yield_item.unwrap_or_default())).fetch_optional(&harness.pool).await?;
            let sculptor: String = sqlx::query_scalar(
                "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert!(
                text.is_some_and(|text| text.contains(&sculptor)),
                "figurine has the sculptor's name"
            );
        }
        let tool_quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(rod).fetch_one(&harness.pool).await?;
        assert_eq!(tool_quantity, 1);
        let bait: i64 = sqlx::query_scalar(
            "SELECT quantity FROM game_item_instances WHERE definition_production_key=$1",
        )
        .bind(supplies.0)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(
            bait,
            if item_target {
                0
            } else if matches!(tool, 3483 | 9306) && committed {
                2
            } else {
                3
            }
        );
        let earned = match (tool, target_id, yield_item) {
            (5908, 5969, Some(5878)) => Some("oteryn:achievement/skin_deep"),
            (5942, 5995, Some(5906)) => Some("oteryn:achievement/ashes_to_dust"),
            (3483 | 9306, 7236, Some(7158 | 3580 | 7159)) => {
                Some("oteryn:achievement/exquisite_taste")
            }
            (3483 | 9306, 7236, Some(3578)) => Some("oteryn:achievement/here_fishy_fishy"),
            _ => None,
        };
        let achievement = super::achievement_granters::assert_gathering(&harness, earned).await?;
        if committed {
            let (_, replay, _) = exchange(&owner, next, command_id, &actual, None).await?;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            owner.refresh_terrain_tools().await;
        }
        assert_eq!(
            achievement,
            super::achievement_granters::assert_gathering(&harness, earned).await?
        );
        if let Some((first, counted, total, granted)) = sculpture {
            let receipt: String =
                sqlx::query_scalar("SELECT event_id::text FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?;
            let first = format!("oteryn:achievement/{first}");
            let first_event: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT source_event_id FROM game_account_achievement_grant_requests WHERE achievement_key=$1 AND source_kind='oteryn:item-craft'",
            )
            .bind(&first)
            .fetch_optional(&harness.pool)
            .await?;
            assert_eq!(
                first_event.as_deref(),
                granted.then_some(receipt.as_bytes()),
                "{first}"
            );
            let facts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
            )
            .bind(&first)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(facts, i64::from(granted), "{first}");
            let progress = super::achievement_granters::assert_progress(
                &harness,
                &format!("oteryn:achievement/{counted}"),
                total - i64::from(!granted),
                granted,
                "oteryn:item-craft",
            )
            .await?;
            assert_eq!(
                progress.as_ref().map(|(_, event, _)| event.as_slice()),
                granted.then_some(receipt.as_bytes())
            );
        }
        if let Some(candidate) = &mut stale_pickup {
            let permit = owner.spell_lane_permit().await.ok_or("pickup lane")?;
            let outcome = owner
                .root
                .commit_map_item_mint(&permit, owner.character, owner.holder, fence, candidate)
                .await;
            assert!(
                matches!(
                    outcome,
                    Err(crate::durability::map_item_mint::MapItemMintError::Refused(
                        crate::durability::map_item_mint::MapItemMintRefusal::AlreadyTaken
                    ))
                ),
                "stale pickup: {outcome:?}"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_item_mint_receipts")
                    .fetch_one(&harness.pool)
                    .await?,
                0
            );
            if after == Some(7442) {
                let mut request = candidate.request().clone();
                request.command = crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(1000).map_err(debug)?,
                );
                let definition = crate::gameplay_transport::item_look::typed_definition(
                    index,
                    reference("oteryn:item.tibia.i7442").ok_or("sculpture ref")?,
                )
                .ok_or("sculpture definition")?;
                request.item =
                    crate::gameplay_transport::item_ref_admission::generation_item_facts(
                        native,
                        &definition,
                    )
                    .map_err(debug)?;
                let mut fresh = owner
                    .root
                    .freeze_map_item_mint(owner.character, owner.holder, fence, request)
                    .await
                    .map_err(debug)?;
                assert!(matches!(
                    owner
                        .root
                        .commit_map_item_mint(
                            &permit,
                            owner.character,
                            owner.holder,
                            fence,
                            &mut fresh
                        )
                        .await
                        .map_err(debug)?,
                    crate::durability::map_item_mint::MapItemMintOutcome::Committed(_)
                ));
                let key: String = sqlx::query_scalar(
                    "SELECT definition_production_key FROM game_map_item_mint_receipts",
                )
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(key, "oteryn:item.tibia.i7442");
            }
        }
        if sewer && committed {
            let schedule: (Vec<i64>, Vec<i64>) = sqlx::query_as(
                "SELECT decay_states,decay_delays FROM game_terrain_tool_transforms",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(schedule, (vec![4597], vec![1_200_000]));
            // Receipt replay preserves the rod and consumes exactly one nail.
            let bait: i64 = sqlx::query_scalar(
                "SELECT quantity FROM game_item_instances WHERE definition_production_key=$1",
            )
            .bind(bait_key)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(bait, 2);
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                .execute(&harness.pool)
                .await?;
            owner.refresh_terrain_tools().await;
            let map = owner.bundle_map().ok_or("bundle map")?;
            let tile =
                crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos)
                    .map_err(debug)?
                    .ok_or("tile")?;
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 4597));
        }
        if target_id == 7236 && committed {
            let schedule: (Vec<i64>, Vec<i64>) = sqlx::query_as(
                "SELECT decay_states,decay_delays FROM game_terrain_tool_transforms",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(schedule, (vec![7200], vec![900_000])); // Crystal items.xml:18879-18880.
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                .execute(&harness.pool)
                .await?;
            owner.refresh_terrain_tools().await;
            let map = owner.bundle_map().ok_or("bundle map")?;
            let tile =
                crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos)
                    .map_err(debug)?
                    .ok_or("tile")?;
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 7200));
        }
        eprintln!(
            "FIXM09_PG_END tool={tool} target={target_id} duration={:?}",
            started.elapsed()
        );
        eprintln!(
            "POSTGRES_CASE_COMPLETED P11 gathering tool={tool} target={target_id} earned={earned:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
