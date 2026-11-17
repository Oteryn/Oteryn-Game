//! W19b: the wire uses a materialized movable pot under current Ground custody.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[derive(Clone, Copy)]
enum Case {
    GroundPlant,
    GroundPreserve,
    NestedCarriedPlant,
    CarriedPreserve,
}
impl Case {
    fn preserve(self) -> bool {
        matches!(self, Self::GroundPreserve | Self::CarriedPreserve)
    }
    fn carried(self) -> bool {
        matches!(self, Self::NestedCarriedPlant | Self::CarriedPreserve)
    }
    fn nested(self) -> bool {
        matches!(self, Self::NestedCarriedPlant)
    }
}
#[test]
fn ice_seed_use_materialized_ground_pot_wire_receipt_and_absolute_chain() -> TestResult {
    run_case(Case::GroundPlant)
}
#[test]
fn ice_seed_preserves_materialized_ground_plant_and_grants_once() -> TestResult {
    run_case(Case::GroundPreserve)
}
#[test]
fn ice_seed_from_open_nested_container_plants_carried_pot() -> TestResult {
    run_case(Case::NestedCarriedPlant)
}
#[test]
fn ice_seed_preserves_carried_plant_without_mutating_its_identity() -> TestResult {
    run_case(Case::CarriedPreserve)
}
fn run_case(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url item_seed_planting");
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Item planting PG lane requires PostgreSQL"
        );
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Item planting wire panic")??;
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "item_seed_planting", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let seed = 13844;
        let before: u16 = if case.preserve() { 14029 } else { 306 };
        let after: u16 = if case.preserve() { 14029 } else { 14031 };
        let mature: u16 = if case.preserve() { 14029 } else { 14032 };
        let carried: u8 = 2;
        let seed_key = format!("oteryn:item.tibia.i{seed}");
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let water_key = format!("oteryn:item.tibia.i{}", before);
        let water = water_key.as_str();
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let state_keys: Vec<_> = [after, mature]
            .into_iter()
            .filter(|id| *id != before)
            .map(|id| format!("oteryn:item.tibia.i{id}"))
            .collect();
        let states = state_keys
            .iter()
            .map(|key| Ok((key.as_str(), reference(key).ok_or("plant state")?.get() - 1)))
            .collect::<Result<Vec<_>, &str>>()?;
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_states(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (water, reference(water).ok_or("water")?.get() - 1),
            &states,
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
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            map.frame_binding_digest(deps.admitted.world_id),
            map.map_revision_digest(),
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
        owner.rooms = crate::gameplay_transport::seam_rooms::SeamRooms::new(None, Some(&room));
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
        let mut carried_entries = vec![(seed_key.as_str(), i64::from(carried))];
        if case.nested() {
            carried_entries.push((bag, 1));
        }
        super::item_look::seed_main_backpack(&harness, &carried_entries).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 12, 14, 15, 18, CAPABILITY_ITEM_STACK_COUNT_V1],
        )
        .ok_or("craft offer selectable")?;
        assert_eq!(admitted.continuity.next_command_id, 1);
        let pos = crate::map::overlay::TilePos {
            x: 7,
            y: 0,
            floor: -7,
        };
        let key = view::placement_key(pos, 1).ok_or("pot placement")?;
        let definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            reference(water).ok_or("empty or preserved pot")?,
        )
        .ok_or("pot definition")?;
        let request = crate::durability::map_item_mint::MapItemMintRequest {
            flat_crate: None,
            key_number: None,
            command: crate::foundation::CommandRef::new(
                admitted.game_session_id,
                crate::foundation::CommandId::new(900).map_err(debug)?,
            ),
            base_bundle_digest: map.base().digest(),
            placement_key: key,
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
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        };
        let mut candidate = owner
            .root
            .freeze_map_item_mint(owner.character, owner.holder, fence, request)
            .await
            .map_err(debug)?;
        let permit = owner.spell_lane_permit().await.ok_or("pot lane")?;
        let minted = owner
            .root
            .commit_map_item_mint(
                &permit,
                owner.character,
                owner.holder,
                fence,
                &mut candidate,
            )
            .await
            .map_err(debug)?;
        let crate::durability::map_item_mint::MapItemMintOutcome::Committed(minted) = minted else {
            return Err("fresh empty pot mint expected".into());
        };
        if case.nested() {
            let move_seed = crate::durability::item_loop_move::LoopMoveRequest {
                item: id(71),
                count: None,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Container(id(72)),
            };
            assert!(
                owner
                    .root
                    .commit_item_loop_move(
                        &permit,
                        owner.character,
                        owner.holder,
                        &fence,
                        crate::foundation::CommandRef::new(
                            admitted.game_session_id,
                            crate::foundation::CommandId::new(901).map_err(debug)?
                        ),
                        &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                        move_seed
                    )
                    .await
                    .map_err(debug)?
                    .is_ok()
            );
        }
        if case.carried() {
            let pickup = crate::durability::item_loop_move::LoopMoveRequest {
                item: minted.item_instance_id,
                count: None,
                pickup_cell: Some(
                    crate::durability::map_item_mint_audit::map_item_spatial_position(
                        pos.x, pos.y, pos.floor,
                    ),
                ),
                destination: crate::durability::item_loop_move::LoopDestination::MainBackpack,
            };
            assert!(
                owner
                    .root
                    .commit_item_loop_move(
                        &permit,
                        owner.character,
                        owner.holder,
                        &fence,
                        crate::foundation::CommandRef::new(
                            admitted.game_session_id,
                            crate::foundation::CommandId::new(902).map_err(debug)?
                        ),
                        &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                        pickup
                    )
                    .await
                    .map_err(debug)?
                    .is_ok()
            );
        }
        let before_item:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(minted.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
        drop(permit);
        owner.states.loop_ground.invalidate();
        if !case.carried() {
            let source = owner
                .root
                .read_ground_action_source(owner.character, fence, minted.item_instance_id)
                .await
                .map_err(debug)?
                .ok_or("materialized pot has current Ground custody")?;
            let runtime = owner.runtime.lock().await;
            crate::durability::consume_transform::GroundActionContext::from_native_owner(
                &room,
                &runtime,
                deps.actor,
                fence.game_session_id,
                source,
            )
            .map_err(debug)?;
        }
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("seed handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("pot handle")?,
            )),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let command = if case.nested() { 2 } else { 1 };
        let (end, result, actual) = super::rust_lottery::exchange(
            &owner,
            admitted,
            command,
            &payload,
            reference(&seed_key),
            reference(water),
        )
        .await?;
        assert_eq!(result, UseDisposition::Committed);
        let retired:(i64,i16)=sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(minted.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, if case.preserve() { (1, 1) } else { (0, 2) });
        if case.preserve() {
            assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(minted.item_instance_id.as_slice()).fetch_one(&harness.pool).await?,before_item);
        }
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&harness.pool).await?,1);
        let chain:Vec<(i32,i64,i64,String)>=sqlx::query_as("SELECT s.source_stage,s.duration_millis,s.expires_at_unix_ms-r.occurred_at,s.target_production_key FROM game_spell_field_temporal_schedules s JOIN game_item_consume_transform_records r ON r.transaction_id=s.event_source_transaction_id JOIN game_item_instances i ON i.item_instance_id=s.item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i14031' ORDER BY s.source_stage")
            .fetch_all(&harness.pool).await?;
        assert_eq!(
            chain,
            if case.preserve() {
                vec![]
            } else {
                vec![
                    (1, 72_000_000, 72_000_000, "oteryn:item.tibia.i14032".into()),
                    (2, 72_000_000, 144_000_000, "oteryn:item.tibia.i306".into()),
                ]
            }
        );
        let proof:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(r.recipe=$1 AND r.source_kind=$2 AND jsonb_array_length(r.extra_inputs)=1 AND (r.extra_inputs->0->>'source_kind')::integer=$3 AND r.envelope_sha256=a.envelope_sha256 AND a.envelope_sha256=sha256(a.envelope)) FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id)")
            .bind(if case.preserve() { "oteryn:planting.preserve" } else { "oteryn:planting.ice-flower" })
            .bind(if case.carried() { 1_i32 } else { 3_i32 }).bind(if case.nested() { 4_i32 } else { 1_i32 })
            .fetch_one(&harness.pool).await?;
        assert_eq!(proof, (1, true));
        if matches!(case, Case::GroundPlant) {
            // This real output has a new UUID but retains its source map's base
            // context. A later temporal stage changes definition/revision only;
            // the existing Ground-action mint proof must keep it usable.
            let (planted, map_mint, action_output): (Vec<u8>, bool, bool) = sqlx::query_as(
                "SELECT uuid_send(i.item_instance_id),game_item_loop_map_mint_source(i.item_instance_id),game_item_action_ground_source(i.item_instance_id) FROM game_item_instances i JOIN game_item_consume_transform_records r ON r.transaction_id=i.minted_transaction_id WHERE r.source_kind=3 AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i14031'"
            ).fetch_one(&harness.pool).await?;
            assert!(!map_mint, "a new action output is not its source map mint");
            assert!(
                action_output,
                "the immutable Ground action proves this output"
            );
            let planted: [u8; 16] = planted.try_into().map_err(|_| "planted UUID")?;
            let source = owner
                .root
                .read_ground_action_source(owner.character, fence, planted)
                .await
                .map_err(debug)?
                .ok_or("planted output has current Ground custody")?;
            let runtime = owner.runtime.lock().await;
            crate::durability::consume_transform::GroundActionContext::from_native_owner(
                &room,
                &runtime,
                deps.actor,
                fence.game_session_id,
                source,
            )
            .map_err(debug)?;
        }
        let (end, replayed, _) =
            super::rust_lottery::exchange(&owner, end, command, &actual, None, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_consume_transform_records"
            )
            .fetch_one(&harness.pool)
            .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_spell_field_temporal_schedules"
            )
            .fetch_one(&harness.pool)
            .await?,
            if case.preserve() { 0 } else { 2 }
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&harness.pool).await?,1);
        if case.preserve() {
            let grants_before:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/preservationist'")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(grants_before, 1);
            let (_, second, _) = super::rust_lottery::exchange(
                &owner,
                end,
                command + 1,
                &payload,
                reference(&seed_key),
                reference(water),
            )
            .await?;
            assert_eq!(second, UseDisposition::Committed);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                2
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/preservationist'")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?,1);
            assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(minted.item_instance_id.as_slice()).fetch_one(&harness.pool).await?,before_item);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&harness.pool).await?,0);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED ITEM_SEED_PLANTING_PG_EXECUTED ground={} nested={} preserve={} exact_seed=true absolute_chain=true replay=true",
            !case.carried(),
            case.nested(),
            case.preserve()
        );
        harness.cleanup().await
    })
}
