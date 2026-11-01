//! M09: ordinary rod and bait reach the production wire and durable consume/mint owner.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
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
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "fishing e2e panic")??;
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
        owner.movement_cells = &cells;
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
        super::item_look::seed_main_backpack(&harness, &[(rod, 1), ("oteryn:item.tibia.i3492", 3)])
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
            &harness, Some("oteryn:achievement/here_fishy_fishy"),
        ).await?;
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
        assert_eq!(achievement, super::achievement_granters::assert_gathering(
            &harness, Some("oteryn:achievement/here_fishy_fishy"),
        ).await?);
        eprintln!("POSTGRES_CASE_COMPLETED P11 standard fishing grant and replay");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn m09_gathering_wire_mutates_corpse_and_grants_inventory_once() -> TestResult { run_gathering(5908,5969,Some(4012),Some(5878),true) }
#[test]
fn m09_gathering_wire_failed_skinning_marks_corpse() -> TestResult { run_gathering(5908,5969,Some(4012),None,true) }
#[test]
fn m09_gathering_wire_dusting_grants_demon_dust() -> TestResult { run_gathering(5942,5995,Some(4098),Some(5906),true) }
#[test]
fn m09_gathering_wire_ice_sculpture_stays_on_map() -> TestResult { run_gathering(5908,7441,Some(7442),None,true) }
#[test]
fn m09_gathering_wire_failed_ice_removes_block() -> TestResult { run_gathering(5942,7441,None,None,true) }
#[test]
fn m09_gathering_wire_ice_hole_catches_rare_fish() -> TestResult { run_gathering(3483,7236,Some(7237),Some(7158),true) }
#[test]
fn m09_gathering_wire_failed_catch_keeps_hole_and_bait() -> TestResult { run_gathering(3483,7236,Some(7236),None,false) }

#[test]
fn p11_ice_fishing_second_rare_species_grants_exquisite_taste() -> TestResult { run_gathering(3483,7236,Some(7237),Some(3580),true) }
#[test]
fn p11_ice_fishing_third_rare_species_grants_exquisite_taste() -> TestResult { run_gathering(3483,7236,Some(7237),Some(7159),true) }
#[test]
fn p11_ice_fishing_common_fish_counts_here_fishy_fishy() -> TestResult { run_gathering(3483,7236,Some(7237),Some(3578),true) }
#[test]
fn p11_mechanical_ice_fishing_grants_exquisite_taste() -> TestResult { run_gathering(9306,7236,Some(7237),Some(7158),true) }
#[test]
fn p11_failed_dusting_marks_corpse_without_advancing_ashes_to_dust() -> TestResult { run_gathering(5942,5995,Some(4098),None,true) }

fn run_gathering(tool: u16, target: u16, after: Option<u16>, yield_item: Option<u16>, committed: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || gathering_scenario(tool,target,after,yield_item,committed).map_err(|e| e.to_string()))?
        .join().map_err(|_| "gathering e2e panic")??;
    Ok(())
}

fn gathering_scenario(tool: u16, target_id: u16, after: Option<u16>, yield_item: Option<u16>, committed: bool) -> TestResult {
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
        owner.movement_cells = &cells;
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
        super::item_look::seed_main_backpack(&harness, &[(rod, 1), ("oteryn:item.tibia.i3492", 3)])
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

        // A pickup may already be prepared when the tool changes/removes the map item.
        let mut stale_pickup = if target_id == 7441 {
            let definition = crate::gameplay_transport::item_look::typed_definition(index, reference(water).ok_or("ice")?).ok_or("ice definition")?;
            let request = crate::durability::map_item_mint::MapItemMintRequest {
                command: crate::foundation::CommandRef::new(admitted.game_session_id, crate::foundation::CommandId::new(999).map_err(debug)?),
                base_bundle_digest: map.base().digest(),
                placement_key: crate::map::view::placement_key(target_pos,1).ok_or("placement")?,
                reset_epoch: 0,
                item: crate::gameplay_transport::item_ref_admission::generation_item_facts(native,&definition).map_err(debug)?,
                quantity: 1,
                content_revision: format!("sha256:{}", native.source_digest().iter().map(|b| format!("{b:02x}")).collect::<String>()),
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            };
            Some(owner.root.freeze_map_item_mint(owner.character,owner.holder,fence,request).await.map_err(debug)?)
        } else { None };
        eprintln!("FIXM09_PG_START tool={tool} target={target_id} after={after:?} yield={yield_item:?}");
        let started = std::time::Instant::now();
        let session = admitted.game_session_id;
        // Resume after a valid prior handle history. The next inventory snapshot issues the
        // carried tool at the third handle, as in the existing fishing fixture. Command 1
        // remains the first command; only the server-issued view handles vary.
        let counter = (0_u64..100_000).find(|counter| {
            let mut candidate = intent.clone();
            candidate.target = UseTarget::Item(std::num::NonZeroU64::new(counter + 3).unwrap_or(std::num::NonZeroU64::MAX));
            let Ok(actual) = encode_item_use_intent(&candidate) else { return false; };
            use sha2::{Digest, Sha256};
            let digest = Sha256::new().chain_update(b"oteryn.fishing.catch.v1").chain_update(session.as_bytes()).chain_update(1_u64.to_be_bytes()).chain_update(&actual).finalize();
            let mut occurrence = [0;16]; occurrence.copy_from_slice(&digest[..16]);
            let draw = oteryn_simulation_determinism::deterministic_decision_u64(&oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(native.source_digest()),oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),"fishing.catch",0).ok();
            let catch_roll = draw.map(|draw| u32::try_from(crate::spell::uniform_draw(draw,1,100_000)).unwrap_or(0)).unwrap_or(0);
            let roll = crate::gameplay_transport::item_craft::gathering_roll(native.source_digest(),session,1,&actual).unwrap_or(0);
            let attempt = oteryn_gameplay_items::fishing::gathering_attempt(tool,target_id,10,catch_roll,roll);
            if !committed { attempt.is_none() } else { attempt.is_some_and(|p| p.after == after && p.yield_item == yield_item) }
        }).ok_or("deterministic handle history exists")?;
        admitted.continuity.item_view.handle_counter = counter;
        let command_id = 1;
        let (next, result, actual) = exchange(&owner, admitted, command_id, &payload, reference(rod)).await?;
        let actual_intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(&[4,6,11,15,18], &actual).map_err(debug)?;
        assert_eq!(actual_intent.target, UseTarget::Item(std::num::NonZeroU64::new(counter+3).ok_or("issued source handle")?));
        assert_eq!(result, if committed { UseDisposition::Committed } else { UseDisposition::NothingToUse });
        let state: Option<i64> = sqlx::query_scalar("SELECT state_item_id FROM game_terrain_tool_transforms").fetch_optional(&harness.pool).await?;
        assert_eq!(state, committed.then(|| after.map_or(0,i64::from)));
        let map = owner.bundle_map().ok_or("bundle map")?;
        let tile = crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), target_pos).map_err(debug)?.ok_or("target tile")?;
        if let Some(after) = after { assert!(tile.entries.iter().any(|e| e.facts.appearance_id == after)); }
        else { assert!(!tile.entries.iter().any(|e| e.source.key() == crate::map::view::MapEntryKey::Base(crate::map::view::placement_key(target_pos,1).unwrap_or(0)))); }
        let inventory_yield: Option<i64> = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(format!("oteryn:item.tibia.i{}", yield_item.unwrap_or(5878))).fetch_one(&harness.pool).await?;
        assert_eq!(inventory_yield, yield_item.map(|_| 1));
        let tool_quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(rod).fetch_one(&harness.pool).await?;
        assert_eq!(tool_quantity,1);
        let bait: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3492'").fetch_one(&harness.pool).await?;
        assert_eq!(bait,if matches!(tool, 3483 | 9306) && committed {2} else {3});
        let earned = match (tool, target_id, yield_item) {
            (5908, 5969, Some(5878)) => Some("oteryn:achievement/skin_deep"),
            (5942, 5995, Some(5906)) => Some("oteryn:achievement/ashes_to_dust"),
            (3483 | 9306, 7236, Some(7158 | 3580 | 7159)) => Some("oteryn:achievement/exquisite_taste"),
            (3483 | 9306, 7236, Some(3578)) => Some("oteryn:achievement/here_fishy_fishy"),
            _ => None,
        };
        let achievement = super::achievement_granters::assert_gathering(&harness, earned).await?;
        if committed {
            let (_, replay, _) = exchange(&owner, next, command_id, &actual, None).await?;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?,1);
            owner.refresh_terrain_tools().await;
        }
        assert_eq!(achievement, super::achievement_granters::assert_gathering(&harness, earned).await?);
        if let Some(candidate) = &mut stale_pickup {
            let permit = owner.spell_lane_permit().await.ok_or("pickup lane")?;
            let outcome = owner.root.commit_map_item_mint(&permit,owner.character,owner.holder,fence,candidate).await;
            assert!(matches!(outcome, Err(crate::durability::map_item_mint::MapItemMintError::Refused(crate::durability::map_item_mint::MapItemMintRefusal::AlreadyTaken))), "stale pickup: {outcome:?}");
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_item_mint_receipts").fetch_one(&harness.pool).await?,0);
            if after == Some(7442) {
                let mut request = candidate.request().clone();
                request.command = crate::foundation::CommandRef::new(session,crate::foundation::CommandId::new(1000).map_err(debug)?);
                let definition = crate::gameplay_transport::item_look::typed_definition(index,reference("oteryn:item.tibia.i7442").ok_or("sculpture ref")?).ok_or("sculpture definition")?;
                request.item = crate::gameplay_transport::item_ref_admission::generation_item_facts(native,&definition).map_err(debug)?;
                let mut fresh = owner.root.freeze_map_item_mint(owner.character,owner.holder,fence,request).await.map_err(debug)?;
                assert!(matches!(owner.root.commit_map_item_mint(&permit,owner.character,owner.holder,fence,&mut fresh).await.map_err(debug)?,crate::durability::map_item_mint::MapItemMintOutcome::Committed(_)));
                let key: String = sqlx::query_scalar("SELECT definition_production_key FROM game_map_item_mint_receipts").fetch_one(&harness.pool).await?;
                assert_eq!(key,"oteryn:item.tibia.i7442");
            }

        }
        if target_id == 7236 && committed {
            let schedule: (Vec<i64>,Vec<i64>) = sqlx::query_as("SELECT decay_states,decay_delays FROM game_terrain_tool_transforms").fetch_one(&harness.pool).await?;
            assert_eq!(schedule,(vec![7200],vec![900_000])); // Crystal items.xml:18879-18880.
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0").execute(&harness.pool).await?;
            owner.refresh_terrain_tools().await;
            let map = owner.bundle_map().ok_or("bundle map")?;
            let tile = crate::map::view::compose_tile(&map.overlay,&map.facts.pinned(),target_pos).map_err(debug)?.ok_or("tile")?;
            assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 7200));
        }
        eprintln!("FIXM09_PG_END tool={tool} target={target_id} duration={:?}", started.elapsed());
        eprintln!("POSTGRES_CASE_COMPLETED P11 gathering tool={tool} target={target_id} earned={earned:?}");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
