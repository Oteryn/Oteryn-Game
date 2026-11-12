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

#[derive(Clone, Copy)]
enum ThawingSource {
    Daily { active: bool, stale_epoch: bool },
    Ordinary,
}

#[test]
fn worldchg_thawing_daily_added_use_yield_replay_restart() -> TestResult {
    run_case_with_source(
        13842,
        true,
        Some(ThawingSource::Daily {
            active: true,
            stale_epoch: false,
        }),
    )
}
#[test]
fn worldchg_thawing_inactive_daily_source_refuses_without_value() -> TestResult {
    run_case_with_source(
        13842,
        true,
        Some(ThawingSource::Daily {
            active: false,
            stale_epoch: false,
        }),
    )
}
#[test]
fn worldchg_thawing_stale_epoch_refuses_without_value() -> TestResult {
    run_case_with_source(
        13842,
        true,
        Some(ThawingSource::Daily {
            active: true,
            stale_epoch: true,
        }),
    )
}
#[test]
fn worldchg_thawing_ordinary_flower_at_daily_coordinate_is_not_day_gated() -> TestResult {
    run_case_with_source(13842, true, Some(ThawingSource::Ordinary))
}
fn run_case(before: u16, yields: bool) -> TestResult {
    run_case_with_source(before, yields, None)
}
fn run_case_with_source(before: u16, yields: bool, source: Option<ThawingSource>) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_SKIPPED no_admin_url map_harvest");
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Map harvest PG lane requires PostgreSQL"
        );
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(before, yields, source).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "map harvest e2e panic")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED map_harvest source={before} yields={yields} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(before: u16, yields: bool, source: Option<ThawingSource>) -> TestResult {
    run(async move |admin| {
        let started = std::time::Instant::now();
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
        // Ordinary flowers retain the base placement; only projected flowers omit it.
        let projected_flower = before == 13842 && !matches!(source, Some(ThawingSource::Ordinary));
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_states(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (water, reference(water).ok_or("water")?.get() - 1),
            if projected_flower { &[] } else { &states },
        );
        let (bytes, pins) = if source.is_some() {
            thawing_fixture(
                bytes,
                pins,
                matches!(source, Some(ThawingSource::Daily { .. })),
            )?
        } else {
            (bytes, pins)
        };
        let actor_cell = if source.is_some() {
            (32254, 31079, 5)
        } else {
            (6, 0, 7)
        };
        let target_pos = crate::map::overlay::TilePos {
            x: (actor_cell.0 + 1) as u16,
            y: actor_cell.1 as u16,
            floor: -(actor_cell.2 as i8),
        };
        let (bytes, pins) = if projected_flower {
            // The reset projection supplies the live flower, not the static fixture.
            let mut bundle = oteryn_world_bundle::bundle::read(&bytes)?;
            let flower = u32::try_from(
                bundle
                    .manifest
                    .palette
                    .iter()
                    .position(|entry| entry.key == water)
                    .ok_or("source palette")?,
            )?;
            bundle.manifest.palette.remove(usize::try_from(flower)?);
            for sector in &mut bundle.sectors {
                for tile in &mut sector.tiles {
                    tile.items.retain(|item| item.palette != flower);
                    for item in &mut tile.items {
                        if item.palette > flower {
                            item.palette -= 1;
                        }
                    }
                }
            }
            assert!(
                !bundle
                    .manifest
                    .palette
                    .iter()
                    .any(|entry| entry.key == water)
            );
            let bytes = oteryn_world_bundle::writer::write(
                &bundle.manifest,
                &bundle.sectors,
                &Default::default(),
            )?;
            let mut pins = pins;
            pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
            (bytes, pins)
        } else {
            (bytes, pins)
        };
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: actor_cell.0 as u16,
                y: actor_cell.1 as u16,
                floor: -(actor_cell.2 as i8),
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
        let world = deps.admitted.world_id;
        let channel = deps.admitted.channel_id;
        let boot = || {
            let checked =
                crate::map::boot::check(bytes.clone(), boot_pins.clone()).map_err(debug)?;
            let checked = if projected_flower {
                checked.with_daily_map(crate::map::daily::DailyMap {
                    seed: [0; 32],
                    fungus: false,
                    grimvale_full_moon: false,
                    items: vec![(13842, (target_pos.x, target_pos.y, -target_pos.floor))],
                    trader: None,
                })
            } else {
                checked
            };
            checked.boot(world, channel, definitions).map_err(debug)
        };
        let map = boot()?;
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
            actor_cell,
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
                    x: actor_cell.0,
                    y: actor_cell.1,
                    floor: actor_cell.2,
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
        if matches!(source, Some(ThawingSource::Daily { .. })) {
            assert!(map.facts().create_world_change_item(target_pos, 13842));
        }
        let key = source_key(&map, target_pos, source)?;
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The physical draw belongs to this source placement and reset epoch, not to a
        // player's command id or opaque view handle. Exercise both sourced flower outcomes.
        let epoch = (1_i64..2048)
            .find(|epoch| {
                if let Some(source) = source {
                    let expected = matches!(source, ThawingSource::Daily { active: true, .. });
                    if !crate::durability::world_event::changes::roll(
                        admitted.world_id,
                        *epoch as u64,
                        0,
                    )
                    .is_ok_and(|day| day.thawing == expected)
                    {
                        return false;
                    }
                }
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
        if source.is_some() {
            let current = if matches!(
                source,
                Some(ThawingSource::Daily {
                    stale_epoch: true,
                    ..
                })
            ) {
                epoch + 1
            } else {
                epoch
            };
            for e in [epoch, current] {
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01') ON CONFLICT DO NOTHING")
                    .bind(admitted.world_id.as_bytes().as_slice()).bind(e).bind(native.source_digest().as_slice()).execute(&harness.pool).await?;
            }
            sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
                .bind(admitted.world_id.as_bytes().as_slice()).bind(current).execute(&harness.pool).await?;
            assert!(
                harness
                    .root
                    .world_changes(admitted.world_id, current as u64)
                    .await?
                    .is_some()
            );
        }
        let intent = UseIntent {
            target: UseTarget::WorldObject(target.clone()),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        if yields && source.is_none() {
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
            owner.movement_cells = (&cells).into();
            owner.reset_epoch = u64::try_from(epoch)?;
            deps.initialize(&owner).await;
        }
        let (_, result, actual) = if matches!(source, Some(ThawingSource::Daily { .. })) {
            exchange_daily_flower(&owner, initial_admitted, 1, &payload).await?
        } else {
            exchange(&owner, initial_admitted, 1, &payload, None).await?
        };
        if matches!(
            source,
            Some(
                ThawingSource::Daily { active: false, .. }
                    | ThawingSource::Daily {
                        stale_epoch: true,
                        ..
                    }
            )
        ) {
            assert_eq!(result, UseDisposition::Rejected);
            let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_terrain_tool_receipts),(SELECT count(*) FROM game_terrain_tool_transforms),(SELECT count(*) FROM game_map_harvest_records)").fetch_one(&harness.pool).await?;
            assert_eq!(counts, (0, 0, 0));
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i13844'").fetch_one(&harness.pool).await?,0);
            assert!(started.elapsed().as_secs_f64() > 1.0);
            eprintln!(
                "WORLDCHG-Q PG EXECUTED thawing daily refusal active/epoch no mutation; elapsed={:?}",
                started.elapsed()
            );
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        assert_eq!(result, UseDisposition::Committed);
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
        let restored_map = boot()?;
        if matches!(source, Some(ThawingSource::Daily { .. })) {
            // A preceding unrelated callback must not change this daily object's identity.
            assert!(
                restored_map
                    .facts()
                    .create_movement_item(target_pos, 13842, 0, || true)
            );
            assert!(
                restored_map
                    .facts()
                    .create_world_change_item(target_pos, 13842)
            );
            assert_eq!(source_key(&restored_map, target_pos, source)?, key);
        }
        let restored_cells = restored_map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut restored = deps.owner(&harness, &authority);
        restored.active_generation = Some(active);
        restored.movement_cells = (&restored_cells).into();
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
        let (mut next, refused, _) = if matches!(source, Some(ThawingSource::Daily { .. })) {
            exchange_daily_flower(
                &restored,
                next,
                2,
                &encode_item_use_intent(&fresh).map_err(debug)?,
            )
            .await?
        } else {
            exchange(
                &restored,
                next,
                2,
                &encode_item_use_intent(&fresh).map_err(debug)?,
                None,
            )
            .await?
        };
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
        if before == 13842 && yields {
            // Every seed is earned by another real USE after four-hour regrowth.
            // Only the test clock is advanced; progress and rewards are never seeded.
            let mut seeds = 1_i64;
            for attempt in 1..=128_u64 {
                let revision = restored_map.facts().pinned().object_revision(key);
                let roll = oteryn_gameplay_items::event_planting::map_harvest_roll(
                    native.source_digest(),
                    *admitted.world_id.as_bytes(),
                    *admitted.channel_id.as_bytes(),
                    &target.placement,
                    epoch,
                    i64::try_from(revision)?,
                )
                .ok_or("harvest draw")?;
                let expected = oteryn_gameplay_items::event_planting::map_harvest(before, roll)
                    .ok_or("harvest plan")?;
                let mut intent = intent.clone();
                intent.target = UseTarget::WorldObject(WorldObjectTarget {
                    expected_revision: revision,
                    ..target.clone()
                });
                let command = next.continuity.next_command_id;
                let payload = encode_item_use_intent(&intent).map_err(debug)?;
                let (current, result, wire) = if matches!(source, Some(ThawingSource::Daily { .. }))
                {
                    exchange_daily_flower(&restored, next, command, &payload).await?
                } else {
                    exchange(&restored, next, command, &payload, None).await?
                };
                assert_eq!(result, UseDisposition::Committed, "harvest {attempt}");
                seeds += i64::from(expected.yield_item.is_some());
                assert_eq!(sqlx::query_scalar::<_, i64>(
                    "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                    .bind(output).fetch_one(&harness.pool).await?, seeds);
                assert_eq!(sqlx::query_scalar::<_, i64>(
                    "SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key=$2")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(achievement)
                    .fetch_one(&harness.pool).await?, seeds.min(10));
                let grants: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND a.achievement_key=$2")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(achievement)
                    .fetch_one(&harness.pool).await?;
                assert_eq!(grants, i64::from(seeds >= 10));
                // Replaying the exact successful command cannot add a seed or a grant.
                let (current, replayed, _) =
                    exchange(&restored, current, command, &wire, None).await?;
                assert_eq!(replayed, UseDisposition::Committed);
                next = current;
                sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=0")
                    .execute(&harness.pool)
                    .await?;
                restored.refresh_terrain_tools().await;
                assert_eq!(
                    restored_map.facts().pinned().object_revision(key),
                    revision + 2
                );
                if seeds >= 11 {
                    break;
                }
            }
            assert!(
                seeds >= 11,
                "ten-seed award and a subsequent seed were exercised"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_map_harvest_records")
                    .fetch_one(&harness.pool)
                    .await?,
                seeds
            );
            assert_eq!(sqlx::query_scalar::<_, i64>(
                "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                .bind(output).fetch_one(&harness.pool).await?, seeds);
            assert_eq!(sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND a.achievement_key=$2")
                .bind(fence.character_id.as_bytes().as_slice()).bind(achievement)
                .fetch_one(&harness.pool).await?, 1);
        }
        drop(restored);
        drop(authority);
        drop(seal);
        if source.is_some() {
            assert!(started.elapsed().as_secs_f64() > 1.0);
            eprintln!(
                "WORLDCHG-Q PG EXECUTED thawing native USE yield/replay/restart/regrowth; elapsed={:?}",
                started.elapsed()
            );
        }
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED W19_MAP_HARVEST_PG_EXECUTED source={before} yields={yields}"
        );
        harness.cleanup().await
    })
}

fn source_key(
    map: &crate::map::boot::BundleWorld,
    pos: crate::map::overlay::TilePos,
    source: Option<ThawingSource>,
) -> Result<u64, Box<dyn std::error::Error>> {
    if matches!(source, Some(ThawingSource::Daily { .. })) {
        map.facts()
            .pinned()
            .movement_items(pos)
            .into_iter()
            .rev()
            .find_map(|entry| {
                if let crate::map::view::EntrySource::Added(id) = entry.source {
                    Some(id.get())
                } else {
                    None
                }
            })
            .ok_or_else(|| "daily Added identity".into())
    } else {
        crate::map::view::placement_key(pos, 1).ok_or_else(|| "base identity".into())
    }
}

/// Explicit open fixture terrain at one authored daily source, not a production map substitute.
fn thawing_fixture(
    bytes: Vec<u8>,
    mut pins: crate::map::BundlePins,
    added: bool,
) -> Result<(Vec<u8>, crate::map::BundlePins), Box<dyn std::error::Error>> {
    let mut sectors = Vec::new();
    let visited = oteryn_world_bundle::bundle::visit(&bytes, |_, mut sector| {
        for tile in &mut sector.tiles {
            tile.x += 32248;
            tile.y += 31079;
            if added && tile.x == 32255 {
                tile.items.retain(|item| item.palette != 4);
            }
        }
        sector.floor = -5;
        sector.sx = sector.tiles[0].x / 32;
        sector.sy = sector.tiles[0].y / 32;
        sectors.push(sector);
        Ok(())
    })
    .map_err(debug)?;
    let mut manifest = visited.manifest;
    manifest.world.min_x += 32248;
    manifest.world.max_x += 32248;
    manifest.world.min_y += 31079;
    manifest.world.max_y += 31079;
    manifest.world.floors = vec![-5];
    let bytes = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())
        .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((bytes, pins))
}

/// Resolve the real Added-object handle from the ordinary map snapshot, then send literal USE.
async fn exchange_daily_flower(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    command_id: u64,
    payload: &[u8],
) -> TestResult<(
    crate::gameplay_transport::connection::AdmittedSession,
    UseDisposition,
    Vec<u8>,
)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::AsyncWriteExt;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut handle = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        handle = map
                            .tiles
                            .iter()
                            .flat_map(|tile| &tile.items)
                            .rev()
                            .find_map(|item| {
                                if matches!(item.appearance_id, 13842 | 13843)
                                    && let oteryn_protocol_oteryn::world_map::MapOrigin::Handle(
                                        handle,
                                    ) = item.origin
                                {
                                    return Some(handle);
                                }
                                None
                            })
                            .or(handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            admitted.continuity.selected_capabilities.as_slice(),
            payload,
        )
        .map_err(debug)?;
        intent.target = UseTarget::Item(handle.ok_or("daily source handle in map snapshot")?);
        let actual = encode_item_use_intent(&intent).map_err(debug)?;
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
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition =
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?;
                    return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
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
    // Preserve the client-side fixture failure instead of masking it with its later liveness loss.
    let (result, actual) = result.ok_or("client did not finish")??;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end.ok_or("server did not finish")?
    else {
        return Err("daily harvest disconnected unexpectedly".into());
    };
    Ok((admitted, result, actual))
}
