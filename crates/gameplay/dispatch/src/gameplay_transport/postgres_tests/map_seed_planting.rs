//! W19: seed USE and AddItem enter existing wire, custody, terrain and audit owners atomically.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::map::view::{self, MapFacts};
use oteryn_protocol_oteryn::item_view::*;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use tokio::io::AsyncWriteExt;

// Direct connections have no node maintenance task. Service demand from a
// bounded pass that retired its holder before the next fixture pass.
async fn maintain_ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}

async fn use_exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    maintain_ready(owner).await?;
    let result =
        super::item_craft::exchange(owner, admitted, command_id, payload, current_source).await;
    maintain_ready(owner).await?;
    result
}

#[derive(Clone, Copy)]
enum Case {
    OrdinaryDrop,
    IceDrop,
    IceUse,
    Preserve,
    IceUsePickup,
    IceUseGroundFirst,
}
#[test]
fn seed_drop_partial_stack_wire_is_atomic_replayable_and_grows() -> TestResult {
    run_case(Case::OrdinaryDrop)
}
#[test]
fn ice_seed_drop_last_unit_wire_retires_only_that_seed() -> TestResult {
    run_case(Case::IceDrop)
}
#[test]
fn ice_seed_use_wire_consumes_one_with_map_cas_and_regrowth() -> TestResult {
    run_case(Case::IceUse)
}
#[test]
fn ice_seed_preservation_wire_grants_once_without_minting_a_seed() -> TestResult {
    run_case(Case::Preserve)
}
#[test]
fn planted_pot_pickup_keeps_deadlines_through_inventory_and_catches_up_once() -> TestResult {
    run_case(Case::IceUsePickup)
}
#[test]
fn planted_pot_ground_stage_remains_visible_and_pickable() -> TestResult {
    run_case(Case::IceUseGroundFirst)
}

fn run_case(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url map_seed_planting");
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Seed planting PG lane requires PostgreSQL"
        );
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "seed planting wire panic")??;
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "map_seed_planting", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dropping = matches!(case, Case::OrdinaryDrop | Case::IceDrop);
        let seed = if matches!(case, Case::OrdinaryDrop) {
            647
        } else {
            13844
        };
        let before: u16 = if matches!(case, Case::Preserve) {
            14029
        } else {
            306
        };
        let after: u16 = if seed == 647 {
            324
        } else if before == 14029 {
            14029
        } else {
            14031
        };
        let mature: u16 = if after == 324 {
            330
        } else if after == 14031 {
            14032
        } else {
            14029
        };
        let carried: u8 = if matches!(case, Case::OrdinaryDrop) {
            5
        } else {
            1
        };
        let count: u8 = if matches!(case, Case::OrdinaryDrop) {
            3
        } else {
            1
        };
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
        super::item_look::seed_main_backpack(&harness, &[(seed_key.as_str(), i64::from(carried))])
            .await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 12, 14, 15, 18, CAPABILITY_ITEM_STACK_COUNT_V1],
        )
        .ok_or("craft offer selectable")?;
        let initial_admitted = admitted;
        assert_eq!(initial_admitted.continuity.next_command_id, 1);
        let pos = crate::map::overlay::TilePos {
            x: 7,
            y: 0,
            floor: -7,
        };
        let key = view::placement_key(pos, 1).ok_or("pot placement")?;
        let mut placement = map.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let intent = UseIntent {
            target: UseTarget::ItemByDefinition(reference(&seed_key).ok_or("seed")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: 0,
            })),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (create_trigger, drop_trigger, count_receipts) = if dropping {
            (
                "CREATE TRIGGER test_seed_fail BEFORE INSERT ON game_map_seed_drop_records FOR EACH ROW EXECUTE FUNCTION test_seed_fail()",
                "DROP TRIGGER test_seed_fail ON game_map_seed_drop_records",
                "SELECT count(*) FROM game_map_seed_drop_records",
            )
        } else {
            (
                "CREATE TRIGGER test_seed_fail BEFORE INSERT ON game_item_craft_records FOR EACH ROW EXECUTE FUNCTION test_seed_fail()",
                "DROP TRIGGER test_seed_fail ON game_item_craft_records",
                "SELECT count(*) FROM game_item_craft_records",
            )
        };
        sqlx::query("CREATE FUNCTION test_seed_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'seed rollback probe'; END $$")
            .execute(&harness.pool).await?;
        sqlx::query(create_trigger).execute(&harness.pool).await?;
        maintain_ready(&owner).await?;
        if dropping {
            let (_, result, _) = drop_exchange(
                &owner,
                admitted,
                1,
                None,
                reference(&seed_key).ok_or("seed")?,
                count,
                true,
            )
            .await?;
            assert!(
                result.is_none(),
                "an unacknowledged durable drop must disconnect before command sequencing"
            );
        } else {
            super::event_object_uncertainty::exchange_item_uncertain(
                &owner,
                admitted,
                &payload,
                reference(&seed_key).ok_or("seed")?,
            )
            .await?;
        }
        maintain_ready(&owner).await?;
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&harness.pool).await?,i64::from(carried));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                .fetch_one(&harness.pool)
                .await?,
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_ground_locations")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_loop_move_receipts")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_transforms")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        assert_eq!(map.facts().pinned().object_revision(key), 0);
        sqlx::query(drop_trigger).execute(&harness.pool).await?;
        sqlx::query("DROP FUNCTION test_seed_fail()")
            .execute(&harness.pool)
            .await?;
        // The failed operation retains an outstanding ingress command. Reconstruct a real
        // fresh stream at command 1; never invent its high-water mark from a counter.
        drop(owner);
        owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        // A bounded pass can retire the sole holder before the reconnect snapshot.
        // Exercise that load-dependent state deterministically in every seed case.
        owner.root.request_ready();
        assert!(owner.root.maintain_ready_once().await.map_err(debug)?);
        let holder = owner.root.try_acquire_ready().map_err(debug)?;
        holder.close().await?;
        owner.root.request_ready();
        assert!(!owner.root.is_ready());
        let actual = if dropping {
            let (_, result, actual) = drop_exchange(
                &owner,
                initial_admitted,
                1,
                None,
                reference(&seed_key).ok_or("seed")?,
                count,
                false,
            )
            .await?;
            assert_eq!(result, Some(ItemMoveOutcome::Moved));
            actual
        } else {
            let (_, result, actual) =
                use_exchange(&owner, initial_admitted, 1, &payload, reference(&seed_key)).await?;
            assert_eq!(result, UseDisposition::Committed);
            actual
        };
        if !dropping {
            let receipt: (String, serde_json::Value, bool, String) = sqlx::query_as(
                "SELECT recipe,outputs,consume_source,before_item->>'definition_production_key' FROM game_item_craft_records",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                receipt,
                (
                    "oteryn:planting.crystal".into(),
                    serde_json::json!([]),
                    true,
                    seed_key.clone(),
                )
            );
        }
        let live:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(&seed_key).fetch_one(&harness.pool).await?;
        assert_eq!(live, i64::from(carried) - 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(count_receipts)
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        let(state,revision,decay_states,delays):(i64,i64,Vec<i64>,Vec<i64>)=sqlx::query_as("SELECT state_item_id,object_revision,decay_states,decay_delays FROM game_terrain_tool_transforms")
            .fetch_one(&harness.pool).await?;
        assert_eq!((state, revision), (i64::from(after), 1));
        assert_eq!(
            decay_states,
            if before == 14029 {
                vec![]
            } else {
                vec![i64::from(mature), 306]
            }
        );
        assert_eq!(
            delays,
            if before == 14029 {
                vec![]
            } else {
                vec![72_000_000, 72_000_000]
            }
        );
        assert_eq!(map.facts().pinned().object_revision(key), 1);
        assert!(
            view::compose_tile(map.overlay(), &map.facts().pinned(), pos)
                .map_err(debug)?
                .ok_or("plant")?
                .entries
                .iter()
                .any(|entry| entry.facts.appearance_id == after)
        );
        if dropping {
            let ground:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1")
                .bind(&seed_key).fetch_one(&harness.pool).await?;
            assert_eq!(ground, i64::from(count) - 1);
            let carried_now:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1")
                .bind(&seed_key).fetch_one(&harness.pool).await?;
            assert_eq!(carried_now, i64::from(carried) - i64::from(count));
            let audit:(i64,bool,bool)=sqlx::query_as("SELECT (d.before_item->>'quantity')::bigint,a.envelope_sha256=d.envelope_sha256 AND a.envelope_sha256=sha256(a.envelope),d.move_transaction_id<>d.transaction_id FROM game_map_seed_drop_records d JOIN game_item_audit_outbox a USING(event_id)")
                .fetch_one(&harness.pool).await?;
            assert_eq!(audit, (i64::from(count), true, true));
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_loop_move_receipts")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_terrain_tool_receipts WHERE tool_item_instance_id IS NULL").fetch_one(&harness.pool).await?,1);
        }
        let preservation:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND a.achievement_key='oteryn:achievement/preservationist')")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(preservation, matches!(case, Case::Preserve));
        if matches!(case, Case::IceUsePickup | Case::IceUseGroundFirst) {
            // Simulate the original map clock becoming overdue while the owner was offline.
            // Only the existing terrain source is moved in time; immutable Item schedules
            // are then created by the real materialization transaction without alteration.
            let ground_first = matches!(case, Case::IceUseGroundFirst);
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-$1")
                .bind(if ground_first { 1_i64 } else { 72_000_001_i64 }).execute(&harness.pool).await?;
            let original_deadline: i64 =
                sqlx::query_scalar("SELECT decay_at_unix_ms FROM game_terrain_tool_transforms")
                    .fetch_one(&harness.pool)
                    .await?;
            let definition = crate::gameplay_transport::item_look::typed_definition(
                index,
                reference("oteryn:item.tibia.i14031").ok_or("planted pot")?,
            )
            .ok_or("pot definition")?;
            let request = crate::durability::map_item_mint::MapItemMintRequest {
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(901).map_err(debug)?,
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
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                    .into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            };
            maintain_ready(&owner).await?;
            let mut candidate = owner
                .root
                .freeze_map_item_mint(owner.character, owner.holder, fence, request)
                .await
                .map_err(debug)?;
            let permit = owner.spell_lane_permit().await.ok_or("pot lane")?;
            maintain_ready(&owner).await?;
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
            let crate::durability::map_item_mint::MapItemMintOutcome::Committed(minted) = minted
            else {
                return Err("fresh pot mint expected".into());
            };
            let deadlines:Vec<(i32,i64)>=sqlx::query_as("SELECT source_stage,expires_at_unix_ms FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid ORDER BY source_stage")
                .bind(minted.item_instance_id.as_slice()).fetch_all(&harness.pool).await?;
            assert_eq!(
                deadlines,
                vec![(1, original_deadline), (2, original_deadline + 72000000)]
            );
            if ground_first {
                maintain_ready(&owner).await?;
                assert_eq!(
                    owner
                        .root
                        .drain_spell_item_deadlines(
                            &permit,
                            owner.character,
                            owner.holder,
                            fence.runtime_scope,
                            fence.scope_ownership_generation.get()
                        )
                        .await
                        .map_err(debug)?,
                    1
                );
                maintain_ready(&owner).await?;
                let actual_ground = owner
                    .root
                    .read_item_loop_ground_for_scope(fence.runtime_scope)
                    .await
                    .map_err(debug)?;
                let visible = actual_ground
                    .iter()
                    .find(|entry| entry.item.item_instance_id == minted.item_instance_id)
                    .ok_or("map-created pot lost Ground projection after its first timer stage")?;
                assert_eq!(
                    visible.item.definition.production_key,
                    "oteryn:item.tibia.i14032"
                );
                assert_eq!(visible.item.quantity, 1);
                assert!(
                    visible.field_collision.is_none(),
                    "a movable plant cannot become an immovable spell field"
                );
            }
            maintain_ready(&owner).await?;
            let movement = crate::durability::item_loop_move::LoopMoveRequest {
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
                        movement
                    )
                    .await
                    .map_err(debug)?
                    .is_ok()
            );
            maintain_ready(&owner).await?;
            assert_eq!(
                owner
                    .root
                    .drain_spell_item_deadlines(
                        &permit,
                        owner.character,
                        owner.holder,
                        fence.runtime_scope,
                        fence.scope_ownership_generation.get()
                    )
                    .await
                    .map_err(debug)?,
                if ground_first { 0 } else { 2 }
            );
            maintain_ready(&owner).await?;
            assert_eq!(
                owner
                    .root
                    .drain_spell_item_deadlines(
                        &permit,
                        owner.character,
                        owner.holder,
                        fence.runtime_scope,
                        fence.scope_ownership_generation.get()
                    )
                    .await
                    .map_err(debug)?,
                0
            );
            let final_item:(String,i64,bool)=sqlx::query_as("SELECT definition_production_key,quantity,EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(minted.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                final_item,
                (
                    (if ground_first {
                        "oteryn:item.tibia.i14032"
                    } else {
                        "oteryn:item.tibia.i306"
                    })
                    .into(),
                    1,
                    true
                )
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_spell_field_temporal_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(minted.item_instance_id.as_slice()).fetch_one(&harness.pool).await?,if ground_first { 1 } else { 2 });
            drop(permit);
            drop(owner);
            drop(authority);
            drop(seal);
            eprintln!(
                "W19 POSTGRES_CASE_COMPLETED MAP_SEED_PLANTING_PG_EXECUTED pickup=true original_deadlines=true carried=true catch_up=true"
            );
            return harness.cleanup().await;
        }
        drop(owner);
        let restored_map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
        )
        .map_err(debug)?;
        let restored_cells = restored_map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut restored = deps.owner(&harness, &authority);
        restored.active_generation = Some(active);
        restored.movement_cells = (&restored_cells).into();
        deps.initialize(&restored).await;
        maintain_ready(&restored).await?;
        restored.refresh_terrain_tools().await;
        assert_eq!(restored_map.facts().pinned().object_revision(key), 1);
        if dropping {
            let (_, result, _) = drop_exchange(
                &restored,
                initial_admitted,
                1,
                Some(actual),
                reference(&seed_key).ok_or("seed")?,
                count,
                false,
            )
            .await?;
            assert_eq!(result, Some(ItemMoveOutcome::Moved));
        } else {
            let (_, result, _) =
                use_exchange(&restored, initial_admitted, 1, &actual, None).await?;
            assert_eq!(result, UseDisposition::Committed);
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>(count_receipts)
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(&seed_key).fetch_one(&harness.pool).await?,live);
        if before != 14029 {
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                .execute(&harness.pool)
                .await?;
            maintain_ready(&restored).await?;
            restored.refresh_terrain_tools().await;
            assert_eq!(restored_map.facts().pinned().object_revision(key), 2);
            assert!(
                view::compose_tile(restored_map.overlay(), &restored_map.facts().pinned(), pos)
                    .map_err(debug)?
                    .ok_or("mature plant")?
                    .entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == mature)
            );
            // The second immutable deadline restores the real empty pot after restart.
            maintain_ready(&restored).await?;
            restored.refresh_terrain_tools().await;
            assert_eq!(restored_map.facts().pinned().object_revision(key), 3);
            assert!(
                view::compose_tile(restored_map.overlay(), &restored_map.facts().pinned(), pos)
                    .map_err(debug)?
                    .ok_or("empty pot")?
                    .entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == 306)
            );
        }
        drop(restored);
        drop(authority);
        drop(seal);
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED MAP_SEED_PLANTING_PG_EXECUTED seed={seed} target={before} dropping={dropping} rollback=true replay=true"
        );
        harness.cleanup().await
    })
}

#[allow(clippy::too_many_arguments)]
async fn drop_exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    replay: Option<Vec<u8>>,
    seed: std::num::NonZeroU32,
    count: u8,
    expect_disconnect: bool,
) -> TestResult<(AdmittedSession, Option<ItemMoveOutcome>, Vec<u8>)> {
    drop_exchange_at(
        owner,
        admitted,
        command_id,
        replay,
        seed,
        count,
        expect_disconnect,
        WorldTilePosition { x: 7, y: 0, floor: -7 },
    ).await
}

// Preserve the existing wire probe controls while varying only its target tile.
#[allow(clippy::too_many_arguments)]
pub(super) async fn drop_exchange_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    replay: Option<Vec<u8>>,
    seed: std::num::NonZeroU32,
    count: u8,
    expect_disconnect: bool,
    destination: WorldTilePosition,
) -> TestResult<(AdmittedSession, Option<ItemMoveOutcome>, Vec<u8>)> {
    maintain_ready(owner).await?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            decode_character_inventory_with_equip_drop(domain.payload, true)
                                .map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == seed)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if let Some(replay) = replay {
            replay
        } else {
            let intent = ItemMoveIntent {
                source: source.ok_or("current seed handle")?,
                destination: ItemMoveDestination::Ground(destination),
            };
            let mut payload = encode_item_move_intent_for(
                &intent,
                ItemMoveSelection {
                    equip_drop: true,
                    container_tree: true,
                    ..ItemMoveSelection::default()
                },
            )
            .map_err(debug)?;
            payload.extend([0x30, count]);
            payload
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_ITEM_MOVE_INTENT,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = match super::item_look::read_frame(&mut client).await {
                Ok(raw) => raw,
                Err(error)
                    if expect_disconnect
                        && error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                            error.kind() == std::io::ErrorKind::UnexpectedEof
                        }) =>
                {
                    return Ok::<_, Box<dyn std::error::Error>>((None, actual));
                }
                Err(error) => return Err(error),
            };
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let outcome = decode_item_move_result_with_equip_drop(result.payload, true)
                        .map_err(debug)?;
                    return Ok::<_, Box<dyn std::error::Error>>((Some(outcome), actual));
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
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
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
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    maintain_ready(owner).await?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) =
        end.ok_or("drop server did not finish")?
    else {
        return Err("drop server ended before admission".into());
    };
    let (result, actual) = result.ok_or("drop client did not finish")??;
    Ok((admitted, result, actual))
}
