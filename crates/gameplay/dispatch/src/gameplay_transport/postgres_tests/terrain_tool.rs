//! TOOLS-2: ordinary USE → durable terrain transform → replay/reload → database deadline.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn terrain_tool_wire_durable_replay_and_deadline_revert() -> TestResult {
    run_case("oteryn:item.tibia.i3457", 867, 868)
}

#[test]
fn m08_machete_cuts_jungle_grass_through_wire_and_recovers_regrowth() -> TestResult {
    run_case("oteryn:item.tibia.i3308", 3696, 3695)
}

#[test]
fn m08_machete_cuts_grass_through_wire_and_recovers_regrowth() -> TestResult {
    run_case("oteryn:item.tibia.i3308", 3702, 3701)
}

#[test]
fn m08_scythe_harvests_wheat_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i3453", 3653, 3651, vec![3652, 3653])
}

#[test]
fn m08_multi_scythe_harvests_wheat_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i9596", 3653, 3651, vec![3652, 3653])
}

#[test]
fn m08_scythe_harvests_burning_cane_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i3453", 5464, 5463, vec![5470, 5465])
}

#[test]
fn m08_multi_scythe_harvests_burning_cane_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i9596", 5464, 5463, vec![5470, 5465])
}

#[test]
fn m08_sickle_harvests_cane_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i3293", 5463, 5462, vec![5470, 5465])
}

#[test]
fn m08_multi_crowbar_opens_ape_city_cask_through_the_wire() -> TestResult {
    run_case("oteryn:item.tibia.i9598", 4848, 3134)
}

#[test]
fn m08_crowbar_opens_ape_city_cask_through_the_wire() -> TestResult {
    run_case("oteryn:item.tibia.i3304", 4848, 3134)
}

#[test]
fn m08b_pick_opens_a_stone_pile_through_the_wire_and_it_decays_back() -> TestResult {
    run_case("oteryn:item.tibia.i3456", 593, 594)
}

#[test]
fn m08b_pick_opens_a_loose_stone_pile_through_the_wire_and_it_decays_back() -> TestResult {
    run_case("oteryn:item.tibia.i3456", 606, 615)
}

#[test]
fn m08b_pick_opens_a_loose_ice_pile_through_the_wire_and_it_decays_back() -> TestResult {
    run_case("oteryn:item.tibia.i3456", 608, 609)
}

#[test]
fn m08b_pick_opens_the_pits_of_inferno_hole_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i3456", 7200, 7236, Vec::new())
}

#[test]
fn m08b_pick_opens_the_spike_hole_through_the_wire() -> TestResult {
    run_case_with_decay("oteryn:item.tibia.i3456", 21341, 21342, Vec::new())
}

fn run_case(tool: &'static str, before: u16, after: u16) -> TestResult {
    run_case_with_decay(tool, before, after, vec![before])
}

fn run_case_with_decay(tool: &'static str, before: u16, after: u16, decay: Vec<u16>) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(tool, before, after, &decay).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "terrain tool e2e panic")??;
    eprintln!(
        "M08B_PG_CASE_EXECUTED tool={tool} target={before} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn scenario(tool: &str, before: u16, after: u16, decay: &[u16]) -> TestResult {
    scenario_with_voice(tool, before, after, decay, None)
}

#[test]
fn worldchg_voice_active_gobbler_wire_and_replay() -> TestResult {
    worldchg_voice_case(true)
}

#[test]
fn worldchg_voice_inactive_gobbler_refuses_without_writes() -> TestResult {
    worldchg_voice_case(false)
}

fn worldchg_voice_case(active: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario_with_voice("oteryn:item.tibia.i12077", 12059, 12065, &[], Some(active))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "voice wire panic")??;
    assert!(started.elapsed() > std::time::Duration::from_secs(1));
    eprintln!(
        "WORLDCHG-Q PG EXECUTED voice active={active} elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario_with_voice(
    tool: &str,
    before: u16,
    after: u16,
    decay: &[u16],
    voice: Option<bool>,
) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "terrain_tool", voice.is_none(), false)
                .await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (dx, dy, floor, ordinal) = if voice.is_some() {
            (33312, 31872, 9, 0)
        } else {
            (0, 0, 7, 1)
        };
        let epoch = if let Some(wanted) = voice {
            let world = deps.admitted.world_id;
            (1..1000)
                .find(|e| {
                    crate::durability::world_event::changes::roll(world, *e, 0)
                        .is_ok_and(|day| day.their_masters_voice == wanted)
                })
                .ok_or("voice day")?
        } else {
            0
        };
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let target_key = format!("oteryn:item.tibia.i{before}");
        let closed_hole = target_key.as_str();
        let bag = "oteryn:item.tibia.i2854";
        let shovel = tool;
        for key in [shovel, closed_hole, &format!("oteryn:item.tibia.i{after}")] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native terrain profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (mut bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (
                closed_hole,
                reference(closed_hole).ok_or("closed_hole")?.get() - 1,
            ),
        );
        if voice.is_some() {
            let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
            data.manifest.world.min_x += dx;
            data.manifest.world.max_x += dx;
            data.manifest.world.min_y += dy;
            data.manifest.world.max_y += dy;
            data.manifest.world.floors = vec![-floor];
            data.manifest.palette[4].terrain = data.manifest.palette[0].terrain;
            let mut second = data.sectors[0]
                .tiles
                .iter()
                .find(|t| t.x == 7)
                .ok_or("fungus tile")?
                .clone();
            second.y = 1;
            data.sectors[0].tiles.push(second);
            for sector in &mut data.sectors {
                sector.floor = -floor;
                sector.sx += dx / 32;
                sector.sy += dy / 32;
                for tile in &mut sector.tiles {
                    if tile.x == 7 {
                        tile.items.remove(0);
                    }
                    tile.x += dx;
                    tile.y += dy;
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
        if before == 4848 {
            // The Ape City cask is explicitly bound to source action 40041.
            let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
            let entry = data
                .sectors
                .iter_mut()
                .flat_map(|s| &mut s.tiles)
                .find(|tile| tile.x == 7)
                .ok_or("cask tile")?
                .items
                .last_mut()
                .ok_or("cask entry")?;
            entry.attrs.action = Some(40041);
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
            start: crate::map::overlay::TilePos {
                x: 6 + dx,
                y: dy,
                floor: -floor,
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
            (i32::from(6 + dx), i32::from(dy), i16::from(floor)),
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
                    x: i32::from(6 + dx),
                    y: i32::from(dy),
                    floor: i16::from(floor),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.reset_epoch = epoch;
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
        // Initial native content activation precedes the first committed World epoch.
        // Once reset owns this World, subsequent cutovers must use its real protocol.
        if voice.is_some() {
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
                .bind(deps.admitted.world_id.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)")
                .bind(deps.admitted.world_id.as_bytes().as_slice()).bind(epoch.to_string()).execute(&harness.pool).await?;
            let day = harness
                .root
                .world_changes(deps.admitted.world_id, epoch)
                .await?
                .ok_or("committed voice day")?;
            assert_eq!(
                Some(day.their_masters_voice),
                voice,
                "real current World selection"
            );
        }
        super::item_look::seed_main_backpack(
            &harness,
            &[(shovel, 1), ("oteryn:item.tibia.i3577", 1)],
        )
        .await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        if voice.is_some() {
            owner.admit_progression_session(&mut admitted).await;
            assert!(
                owner
                    .progression_binding(admitted.game_session_id)
                    .is_some(),
                "native XP binding"
            );
        }
        let mut placement = map.base().digest().to_vec();
        let target_pos = crate::map::overlay::TilePos {
            x: 7 + dx,
            y: dy,
            floor: -floor,
        };
        placement.extend_from_slice(
            &crate::map::view::placement_key(target_pos, ordinal)
                .ok_or("target placement")?
                .to_be_bytes(),
        );
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The wire helper resolves the current shovel handle from the inventory snapshot.
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target.clone())),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (mut next, result, payload) =
            exchange(&owner, admitted, 1, &payload, reference(shovel))
                .await
                .map_err(|e| format!("initial use: {e}"))?;
        if voice == Some(false) {
            assert_eq!(result, UseDisposition::Rejected);
            for query in [
                "SELECT count(*) FROM game_terrain_tool_receipts",
                "SELECT count(*) FROM game_terrain_tool_transforms",
                "SELECT count(*) FROM game_world_change_credit_receipts",
                "SELECT count(*) FROM game_world_change_personal_counters",
                "SELECT count(*) FROM game_character_xp_receipts",
            ] {
                let count: i64 = sqlx::query_scalar(query).fetch_one(&harness.pool).await?;
                assert_eq!(count, 0);
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        assert_eq!(result, UseDisposition::Committed);
        let state: (i64,i64,Option<bool>) = sqlx::query_as("SELECT state_item_id,object_revision,decay_at_unix_ms > floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_terrain_tool_transforms").fetch_one(&harness.pool).await?;
        assert_eq!(
            state,
            (
                i64::from(after),
                1,
                (!decay.is_empty() || voice.is_some()).then_some(true)
            )
        );
        let key = crate::map::view::placement_key(target_pos, ordinal).ok_or("key")?;
        assert_eq!(map.facts().pinned().object_revision(key), 1);
        if voice == Some(true) {
            let second_pos = crate::map::overlay::TilePos {
                y: dy + 1,
                ..target_pos
            };
            let mut second_placement = map.base().digest().to_vec();
            second_placement.extend_from_slice(
                &crate::map::view::placement_key(second_pos, 0)
                    .ok_or("second placement")?
                    .to_be_bytes(),
            );
            let second_intent = UseIntent {
                use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                    placement: second_placement,
                    expected_revision: 0,
                })),
                ..intent.clone()
            };
            let (_, result, _) = exchange(
                &owner,
                next,
                2,
                &encode_item_use_intent(&second_intent).map_err(debug)?,
                reference(shovel),
            )
            .await?;
            assert_eq!(
                result,
                UseDisposition::Rejected,
                "five-second per-character exhaustion"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
        }
        if voice == Some(true) {
            let xp: (i64, i64) = sqlx::query_as("SELECT count(*),COALESCE(sum(experience_awarded),0)::bigint FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(xp, (1, 20), "closed gobbler XP joins the terrain receipt");
            let shared: (bool, String, String) = sqlx::query_as("SELECT t.decay_at_unix_ms=(c.state->>'deadline_ms')::bigint,(c.state->'counts'->>0),p.counter::text FROM game_terrain_tool_transforms t JOIN game_world_change_counters c USING(world_id) JOIN game_world_change_personal_counters p ON p.world_id=c.world_id AND p.change_key=c.change_key WHERE c.change_key='their_masters_voice' AND p.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                shared,
                (true, "1".into(), "1".into()),
                "same-transaction gobble/tally/shared restore deadline"
            );
        }
        // A new owner recovers the committed state and original receipt without current handles.
        drop(owner);
        let restored_map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
        )
        .map_err(debug)?;
        assert_eq!(restored_map.facts().pinned().object_revision(key), 0);
        let restored_cells = restored_map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.reset_epoch = epoch;
        restarted.movement_cells = (&restored_cells).into();

        deps.initialize(&restarted).await;
        let (_, result, _) = exchange(&restarted, admitted, 1, &payload, None)
            .await
            .map_err(|e| format!("replay: {e}"))?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT object_revision FROM game_terrain_tool_transforms"
            )
            .fetch_one(&harness.pool)
            .await?,
            1
        );
        if voice == Some(true) {
            assert_eq!(
                restored_map
                    .facts()
                    .pinned()
                    .base_entry(
                        target_pos,
                        ordinal,
                        reference(closed_hole).ok_or("fungus reference")?.get() - 1
                    )
                    .ok_or("fungus entry")?
                    .appearance_id,
                12065
            );
            sqlx::query(
                "DELETE FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid",
            )
            .bind(deps.admitted.world_id.as_bytes().as_slice())
            .execute(&harness.pool)
            .await?;
            let second_pos = crate::map::overlay::TilePos {
                y: dy + 1,
                ..target_pos
            };
            let mut second_placement = restored_map.base().digest().to_vec();
            second_placement.extend_from_slice(
                &crate::map::view::placement_key(second_pos, 0)
                    .ok_or("second source")?
                    .to_be_bytes(),
            );
            let second_intent = UseIntent {
                use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                    placement: second_placement,
                    expected_revision: 0,
                })),
                ..intent.clone()
            };
            let mut probe = admitted;
            probe.continuity.next_command_id = 3;
            restarted.admit_progression_session(&mut probe).await;
            // Removing the active epoch closes snapshot admission before USE can
            // dispatch. Assert that earlier authority boundary, rather than
            // expecting a command result from a connection that cannot enter.
            let refusal = exchange(
                &restarted,
                probe,
                3,
                &encode_item_use_intent(&second_intent).map_err(debug)?,
                reference(tool),
            )
            .await;
            let error = refusal.err().ok_or("missing epoch admitted a connection")?;
            assert_eq!(error.to_string(), "broken pipe");
            let xp_after: (i64, i64) = sqlx::query_as("SELECT count(*),COALESCE(sum(experience_awarded),0)::bigint FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(xp_after, (1, 20), "closed admission cannot award XP");
            let gobbles_after: String = sqlx::query_scalar("SELECT counter::text FROM game_world_change_personal_counters WHERE character_id=encode($1,'hex')::uuid AND change_key='their_masters_voice'")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(gobbles_after, "1", "closed admission cannot credit gobbles");
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            drop(restarted);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let yield_item = match before {
            3653 => Some("oteryn:item.tibia.i3605"),
            5463 | 5464 => Some("oteryn:item.tibia.i5466"),
            _ => None,
        };
        if let Some(yield_item) = yield_item {
            let quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                .bind(yield_item).fetch_one(&harness.pool).await?;
            assert_eq!(quantity, 1, "replay must not mint a second harvest yield");
            let tool_quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
                .bind(shovel).fetch_one(&harness.pool).await?;
            assert_eq!(tool_quantity, 1, "harvesting preserves the carried tool");
            let crafts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(crafts, 1);
        }
        // Still before the durable deadline: a server clock drain cannot revert early.
        restarted.refresh_terrain_tools().await;
        assert_eq!(restored_map.facts().pinned().object_revision(key), 1);
        assert_eq!(
            restored_map
                .facts()
                .pinned()
                .base_entry(
                    target_pos,
                    1,
                    reference(closed_hole).ok_or("closed reference")?.get() - 1
                )
                .ok_or("recovered entry")?
                .appearance_id,
            after
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT state_item_id FROM game_terrain_tool_transforms")
                .fetch_one(&harness.pool)
                .await?,
            i64::from(after)
        );
        // Advance each persisted deadline through the production clock path.
        // Harvest has two source-defined stages; check both, including the
        // retained deadline until the final stage and every recovered revision.
        for (step, expected) in decay.iter().enumerate() {
            sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1").execute(&harness.pool).await?;
            restarted.refresh_terrain_tools().await;
            let revision = i64::try_from(step)? + 2;
            let state: (i64,i64,bool) = sqlx::query_as("SELECT state_item_id,object_revision,decay_at_unix_ms IS NULL FROM game_terrain_tool_transforms").fetch_one(&harness.pool).await?;
            assert_eq!(
                state,
                (i64::from(*expected), revision, step + 1 == decay.len())
            );
            assert_eq!(
                restored_map.facts().pinned().object_revision(key),
                u64::try_from(revision)?
            );
            assert_eq!(
                restored_map
                    .facts()
                    .pinned()
                    .base_entry(
                        target_pos,
                        1,
                        reference(closed_hole).ok_or("closed reference")?.get() - 1
                    )
                    .ok_or("reverted entry")?
                    .appearance_id,
                *expected
            );
        }
        let final_revision = i64::try_from(decay.len())? + 1;
        let final_state = decay.last().copied().unwrap_or(after);
        restarted.refresh_terrain_tools().await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT object_revision FROM game_terrain_tool_transforms"
            )
            .fetch_one(&harness.pool)
            .await?,
            final_revision
        );
        // A stale map revision cannot start another transform or create another receipt.
        next.continuity.next_command_id = 2;
        let (_, result, _) = exchange(&restarted, next, 2, &payload, reference(shovel))
            .await
            .map_err(|e| format!("stale target: {e}"))?;
        assert_eq!(result, UseDisposition::StaleState);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        // A changed payload cannot use the historical receipt's CommandRef.
        let mut changed = intent.clone();
        changed.use_with_map_item = Some(MapUseTarget::Object(WorldObjectTarget {
            expected_revision: u64::try_from(final_revision)?,
            ..target.clone()
        }));
        // Transport closes on altered duplicate CommandRef bytes before item dispatch. Probe
        // the durable boundary directly as well, so its own receipt binding is qualified.
        let permit = restarted.spell_lane_permit().await.ok_or("permit")?;
        let scope = crate::durability::terrain_tool::TerrainScope {
            map_revision: crate::map::overlay::map_revision(restored_map.base()),
            content_generation: native.source_digest(),
            reset_epoch: 0,
        };
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                fence,
                crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(1).map_err(debug)?,
                ),
                encode_item_use_intent(&changed).map_err(debug)?,
                scope.clone(),
                None,
            )
            .await?
            .ok_or("changed replay outcome")?;
        assert_eq!(result, UseDisposition::Rejected);
        // Valid map and tool facts, but a stale lease: the durable owner must refuse.
        let mut stale_fence = fence;
        stale_fence.character_lease_generation += 1;
        let command = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(3).map_err(debug)?,
        );
        let mutation = crate::durability::terrain_tool::TerrainMutation {
            tool: id(71),
            tool_key: shovel.into(),
            tool_revision: "definition-r1".into(),
            placement: target.placement.clone(),
            expected_revision: final_revision,
            before: u32::from(final_state),
            after: u32::from(after),
            decay: vec![(
                u32::from(before),
                if before == 867 { 600_000 } else { 300_000 },
            )],
        };
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                stale_fence,
                command,
                encode_item_use_intent(&changed).map_err(debug)?,
                scope.clone(),
                Some(mutation.clone()),
            )
            .await?
            .ok_or("stale fence outcome")?;
        assert_eq!(result, UseDisposition::Rejected);
        let absent_tool = crate::durability::terrain_tool::TerrainMutation {
            tool: id(99),
            ..mutation.clone()
        };
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                fence,
                command,
                encode_item_use_intent(&changed).map_err(debug)?,
                scope.clone(),
                Some(absent_tool),
            )
            .await?
            .ok_or("absent tool outcome")?;
        assert_eq!(result, UseDisposition::StaleState);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        // The same request with only current authority and real carried custody succeeds.
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                fence,
                command,
                encode_item_use_intent(&changed).map_err(debug)?,
                scope,
                Some(mutation),
            )
            .await?
            .ok_or("current fence outcome")?;
        drop(permit);
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            2
        );
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
