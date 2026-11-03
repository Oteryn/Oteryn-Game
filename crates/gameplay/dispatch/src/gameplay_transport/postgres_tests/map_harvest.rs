//! W19: map harvesting uses the production object USE path and one durable map/value commit.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn w19_blueberry_harvest_wire_atomic_replay_and_regrowth() -> TestResult {
    run_case(3699, true)
}
#[test]
fn w19_ice_flower_harvest_wire_yield_replay_and_restart() -> TestResult {
    run_case(13842, true)
}
#[test]
fn w19_ice_flower_harvest_wire_no_seed_cannot_reroll() -> TestResult {
    run_case(13842, false)
}

fn run_case(before: u16, yields: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url map_harvest");
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Map harvest PG lane requires PostgreSQL"
        );
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(before, yields).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "map harvest e2e panic")??;
    Ok(())
}

fn scenario(before: u16, yields: bool) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "map_harvest", true, false).await?;
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
        let water_key = format!("oteryn:item.tibia.i{before}");
        let water = water_key.as_str();
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let plan =
            oteryn_gameplay_items::event_planting::map_harvest(before, 1).ok_or("source plan")?;
        let state_keys: Vec<_> = std::iter::once(plan.after)
            .chain(plan.decay.and_then(|(id, _)| u16::try_from(id).ok()))
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
        super::item_look::seed_main_backpack(&harness, &[]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let initial_admitted = admitted;
        assert_eq!(initial_admitted.continuity.next_command_id, 1);
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
        // The physical draw belongs to this source placement and reset epoch, not to a
        // player's command id or opaque view handle. Exercise both sourced flower outcomes.
        let epoch = (0_i64..128)
            .find(|epoch| {
                let roll = oteryn_gameplay_items::event_planting::map_harvest_roll(
                    native.source_digest(),
                    *admitted.world_id.as_bytes(),
                    *admitted.channel_id.as_bytes(),
                    &target.placement,
                    *epoch,
                    0,
                );
                roll.and_then(|roll| {
                    oteryn_gameplay_items::event_planting::map_harvest(before, roll)
                })
                .is_some_and(|plan| plan.yield_item.is_some() == yields)
            })
            .ok_or("bounded source draw fixture")?;
        owner.reset_epoch = u64::try_from(epoch)?;
        let intent = UseIntent {
            target: UseTarget::WorldObject(target.clone()),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        if yields {
            // Fail after the real map CAS, before value rows. The transaction must restore
            // both map and value state, and retrying cannot select a different flower draw.
            sqlx::query("CREATE FUNCTION test_map_harvest_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'harvest rollback probe'; END $$")
                .execute(&harness.pool).await?;
            sqlx::query("CREATE TRIGGER test_map_harvest_fail BEFORE INSERT ON game_map_harvest_records FOR EACH ROW EXECUTE FUNCTION test_map_harvest_fail()")
                .execute(&harness.pool).await?;
            super::event_object_uncertainty::exchange_uncertain(&owner, admitted, &payload).await?;
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
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_harvest_records")
                    .fetch_one(&harness.pool)
                    .await?,
                0
            );
            assert_eq!(
                map.facts()
                    .pinned()
                    .object_revision(crate::map::view::placement_key(target_pos, 1).ok_or("key")?),
                0
            );
            sqlx::query("DROP TRIGGER test_map_harvest_fail ON game_map_harvest_records")
                .execute(&harness.pool)
                .await?;
            sqlx::query("DROP FUNCTION test_map_harvest_fail()")
                .execute(&harness.pool)
                .await?;
            // A reconstructed owner starts with an empty session command directory.
            // Exercise the real first command again, without synthesizing a high-water mark.
            drop(owner);
            owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(active);
            owner.movement_cells = &cells;
            owner.reset_epoch = u64::try_from(epoch)?;
            deps.initialize(&owner).await;
        }
        let (_, result, actual) = exchange(&owner, initial_admitted, 1, &payload, None).await?;
        assert_eq!(result, UseDisposition::Committed);
        let key = crate::map::view::placement_key(target_pos, 1).ok_or("key")?;
        assert_eq!(map.facts().pinned().object_revision(key), 1);
        let (state, revision, decay_states, decay_delays): (i64, i64, Vec<i64>, Vec<i64>) = sqlx::query_as(
            "SELECT state_item_id,object_revision,decay_states,decay_delays FROM game_terrain_tool_transforms")
            .fetch_one(&harness.pool).await?;
        assert_eq!((state, revision), (i64::from(plan.after), 1));
        assert_eq!(
            decay_states,
            plan.decay.map(|p| vec![i64::from(p.0)]).unwrap_or_default()
        );
        assert_eq!(
            decay_delays,
            plan.decay.map(|p| vec![p.1]).unwrap_or_default()
        );
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts WHERE tool_item_instance_id IS NULL").fetch_one(&harness.pool).await?, 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_harvest_records")
                .fetch_one(&harness.pool)
                .await?,
            i64::from(yields)
        );
        let output = if before == 3699 {
            "oteryn:item.tibia.i3588"
        } else {
            "oteryn:item.tibia.i13844"
        };
        let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(output).fetch_one(&harness.pool).await?;
        assert_eq!(
            quantity,
            if yields {
                if before == 3699 { 3 } else { 1 }
            } else {
                0
            }
        );
        if yields {
            let proof: (i64, bool, bool) = sqlx::query_as("SELECT r.output_quantity,a.envelope_sha256=r.envelope_sha256 AND a.envelope_sha256=sha256(a.envelope),r.parent_item_instance_id IS NOT NULL FROM game_map_harvest_records r JOIN game_item_audit_outbox a USING(event_id)")
                .fetch_one(&harness.pool).await?;
            assert_eq!(proof, (quantity, true, before != 3699));
            if before == 3699 {
                let position: Vec<u8> = sqlx::query_scalar("SELECT g.spatial_position FROM game_map_harvest_records r JOIN game_item_ground_locations g ON g.item_instance_id=r.output_item_instance_id")
                    .fetch_one(&harness.pool).await?;
                assert_eq!(
                    position,
                    crate::durability::map_item_mint_audit::map_item_spatial_position(7, 0, -7)
                );
            }
        }
        let achievement = if before == 3699 {
            "oteryn:achievement/bluebarian"
        } else {
            "oteryn:achievement/ice_harvester"
        };
        let progress: Option<i64> = sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key=$2")
            .bind(fence.character_id.as_bytes().as_slice()).bind(achievement).fetch_optional(&harness.pool).await?;
        assert_eq!(progress, yields.then_some(1));
        // Reboot projection and replay the original bytes after the visible source changed.
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
        restored.movement_cells = &restored_cells;
        restored.reset_epoch = u64::try_from(epoch)?;
        deps.initialize(&restored).await;
        restored.refresh_terrain_tools().await;
        assert_eq!(restored_map.facts().pinned().object_revision(key), 1);
        let (next, replayed, _) = exchange(&restored, initial_admitted, 1, &actual, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_harvest_records")
                .fetch_one(&harness.pool)
                .await?,
            i64::from(yields)
        );
        let mut fresh = intent.clone();
        fresh.target = UseTarget::WorldObject(WorldObjectTarget {
            expected_revision: 1,
            ..target.clone()
        });
        let (_, refused, _) = exchange(
            &restored,
            next,
            2,
            &encode_item_use_intent(&fresh).map_err(debug)?,
            None,
        )
        .await?;
        assert_ne!(refused, UseDisposition::Committed); // transformed flower/bush cannot be reharvested
        if plan.decay.is_some() {
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                .execute(&harness.pool)
                .await?;
            restored.refresh_terrain_tools().await;
            assert_eq!(restored_map.facts().pinned().object_revision(key), 2);
            let tile = crate::map::view::compose_tile(
                restored_map.overlay(),
                &restored_map.facts().pinned(),
                target_pos,
            )
            .map_err(debug)?
            .ok_or("regrown source")?;
            assert!(
                tile.entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == before)
            );
        }
        drop(restored);
        drop(authority);
        drop(seal);
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED W19_MAP_HARVEST_PG_EXECUTED source={before} yields={yields}"
        );
        harness.cleanup().await
    })
}
