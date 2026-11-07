//! Actual Native custody, source actor reservations and atomic station receipts.
use super::*;
use crate::durability::item_craft::{GatheringCraft, ItemCraftRequest};
use crate::durability::terrain_tool::{TerrainMutation, TerrainScope};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_items::fishing::GatheringAttempt;
use oteryn_gameplay_world_change::variants;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn wvar_thornfire_native_bucket_lifetime_atomic_credit_and_reserved_wolves() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Thornfire station PG thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h =
            Harness::create_with_admission_seed(admin, "wvar_thorn_native", true, false).await?;
        reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let weights = NativeItemWeights(native);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let world = deps.admitted.world_id;
        let certificate = [0xa7; 32];
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i11466", 1),
                ("oteryn:item.tibia.i2873", 1),
            ],
        )
        .await?;
        let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
            fence.runtime_scope,
        )
        .await
        .map_err(debug)?;
        let mut prep = ItemCraftRequest {
            command: command(deps.admitted.game_session_id, 900)?,
            source: id(71),
            tool: None,
            ingredient: Some(id(72)),
            station: Some(crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2873".into(),
                revision_ref: "definition-r1".into(),
            }),
            intent: vec![91, 0],
            gathering: None,
            enchanting: None,
        };
        assert_eq!(
            h.root
                .craft_item(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    prep.clone(),
                    native,
                    &weights,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(live_count(&h, "oteryn:item.tibia.i11466").await?, 0);
        assert_eq!(live_count(&h, "oteryn:item.tibia.i2873").await?, 0);
        assert_eq!(live_count(&h, "oteryn:item.tibia.i12819").await?, 1);
        let before = counts(&h).await?;
        assert_eq!(
            h.root
                .craft_item(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    prep.clone(),
                    native,
                    &weights,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(counts(&h).await?, before);
        // A source oil receipt cannot be reused under another command after its debit.
        prep.command = command(deps.admitted.game_session_id, 901)?;
        assert_ne!(
            h.root
                .craft_item(
                    &permit, &authority, &h.node, fence, prep, native, &weights, false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        drop(permit);
        let epoch = 992;
        seed_thorn_epoch(&h, world, epoch, certificate, native.source_digest(), true).await?;
        let definition = variants::definitions()?
            .iter()
            .find(|d| d.key == "thornfire")
            .ok_or("Thornfire")?;
        let selected = &definition.variants[0];
        let bog = definition
            .stations
            .iter()
            .find(|s| s.resource.is_none())
            .ok_or("source bog")?;
        let flames = definition
            .stations
            .iter()
            .filter(|s| s.resource.is_some())
            .take(3)
            .collect::<Vec<_>>();
        assert_eq!(flames.len(), 3);
        let mut permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
            fence.runtime_scope,
        )
        .await
        .map_err(debug)?;
        let origin = MovementLocalPosition {
            x: 33090,
            y: 32149,
            floor: 7,
        };
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            native.source_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            certificate,
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let crystal_policy = runtime
            .companion_policy("oteryn:creature.crystal_wolf")
            .map_err(debug)?;
        assert!(
            crystal_policy.maximum_health > 0,
            "source bog replacement has an admitted Native policy"
        );
        native
            .reward_table()
            .row("oteryn:creature.crystal_wolf")
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        // Historical personal progress is fixture state; the live writer owns both increments.
        sqlx::query("INSERT INTO game_character_achievement_progress(character_id,achievement_key,progress) VALUES(encode($1,'hex')::uuid,'oteryn:achievement/firefighter',499)").bind(id(41).as_slice()).execute(&h.pool).await?;
        for (iteration, flame) in flames.iter().enumerate() {
            let empty = live_item(&h, "oteryn:item.tibia.i12819").await?;
            let mut refill = station_request(
                native,
                deps.admitted.game_session_id,
                1000 + iteration as u64 * 100,
                empty,
                bog,
                selected,
                epoch,
                certificate,
                false,
            )?;
            refill
                .gathering
                .as_mut()
                .ok_or("refill gathering")?
                .mutation
                .expected_revision = i64::try_from(iteration)?;
            assert_eq!(
                h.root
                    .craft_item(
                        &permit,
                        &authority,
                        &h.node,
                        fence,
                        refill.clone(),
                        native,
                        &weights,
                        false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            let filled = live_item(&h, "oteryn:item.tibia.i12722").await?;
            let timer:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid AND duration_millis=3600000 AND target_production_key='oteryn:item.tibia.i12819' AND expires_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint)").bind(filled.as_slice()).fetch_one(&h.pool).await?;
            assert!(
                timer,
                "actual filled output owns its original one-hour timer"
            );
            let after_refill = counts(&h).await?;
            assert_eq!(
                h.root
                    .craft_item(
                        &permit, &authority, &h.node, fence, refill, native, &weights, false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            assert_eq!(counts(&h).await?, after_refill);
            let wolves = iteration == 1;
            let mut extinguish = station_request(
                native,
                deps.admitted.game_session_id,
                1010 + iteration as u64 * 100,
                filled,
                flame,
                selected,
                epoch,
                certificate,
                true,
            )?;
            let mut number = extinguish.command.command_id().get();
            while crate::durability::item_craft::source_station_wolves(&extinguish) != Some(wolves)
            {
                number += 1;
                extinguish.command = command(deps.admitted.game_session_id, number)?;
            }
            let before = counts(&h).await?;
            let mut wrong = extinguish.clone();
            wrong
                .gathering
                .as_mut()
                .ok_or("gathering")?
                .mutation
                .placement[39] = 63;
            assert_ne!(
                h.root
                    .craft_item(
                        &permit, &authority, &h.node, fence, wrong, native, &weights, false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            assert_eq!(counts(&h).await?, before);
            if wolves {
                // A bare craft call cannot manufacture two native source actors.
                assert_ne!(
                    h.root
                        .craft_item(
                            &permit,
                            &authority,
                            &h.node,
                            fence,
                            extinguish.clone(),
                            native,
                            &weights,
                            false
                        )
                        .await?,
                    Some(UseDisposition::Committed)
                );
                assert_eq!(counts(&h).await?, before);
                let binding =
                    crate::durability::item_craft::source_station_actor_binding(&extinguish)
                        .ok_or("binding")?;
                let pads = crate::durability::item_craft::WOLF_PADS;
                let actors = [
                    (crate::durability::item_craft::WOLF, pads[0]),
                    (crate::durability::item_craft::WOLF, pads[1]),
                ];
                let mut batch = runtime
                    .prepare_native_actor_batch(
                        deps.actor,
                        deps.admitted.game_session_id,
                        binding,
                        &actors,
                    )
                    .map_err(debug)?;
                runtime
                    .reserve_native_actor_batch(&mut batch)
                    .map_err(debug)?;
                let proof = batch.proof().map_err(debug)?;
                // Real caller cancellation after an actual SQL COMMIT: the typed
                // existing window must retain invisible reserved native slots.
                let prior_runtime =
                    std::mem::replace(&mut deps.runtime, tokio::sync::Mutex::new(runtime));
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(generation);
                owner.reset_epoch = epoch;
                drop(permit);
                let mut actual_permit = owner
                    .spell_lane_permit()
                    .await
                    .ok_or("actual native station lane")?;
                let pending =
                    crate::gameplay_transport::item_craft::world_changes::qualified_pending_station(
                        deps.actor,
                        deps.admitted.game_session_id,
                        fence,
                        extinguish.clone(),
                        batch,
                    );
                let mut window = actual_permit.open_commit_window(
                    pending,
                    crate::gameplay_transport::item_craft::world_changes::park_qualified_station,
                );
                assert_eq!(
                    h.root
                        .craft_item_with_source_actors(
                            &authority,
                            &h.node,
                            fence,
                            extinguish.clone(),
                            native,
                            &weights,
                            &mut window,
                            &proof
                        )
                        .await?,
                    Some(UseDisposition::Committed)
                );
                assert!(window.commit_called());
                assert!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .visible_entities()
                        .creatures
                        .is_empty(),
                    "committed but unresolved batch stays invisible"
                );
                drop(window);
                drop(actual_permit);
                let committed_counts = counts(&h).await?;
                let resolved = owner
                    .spell_lane_permit()
                    .await
                    .ok_or("normal lane resolves actual typed station receipt")?;
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .visible_entities()
                        .creatures
                        .len(),
                    2,
                    "known original receipt publishes exactly its reserved native pair"
                );
                assert_eq!(
                    counts(&h).await?,
                    committed_counts,
                    "resolution adds no craft/debit/credit/schedule/achievement"
                );
                drop(resolved);
                let replay = owner
                    .spell_lane_permit()
                    .await
                    .ok_or("resolved lane remains live")?;
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .visible_entities()
                        .creatures
                        .len(),
                    2,
                    "repeated resolution cannot spawn another pair"
                );
                drop(replay);
                drop(owner);
                runtime = std::mem::replace(&mut deps.runtime, prior_runtime).into_inner();
                permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
                    fence.runtime_scope,
                )
                .await
                .map_err(debug)?;
                assert_eq!(runtime.visible_entities().creatures.len(), 2);
            } else {
                assert_eq!(
                    h.root
                        .craft_item(
                            &permit,
                            &authority,
                            &h.node,
                            fence,
                            extinguish.clone(),
                            native,
                            &weights,
                            false
                        )
                        .await?,
                    Some(UseDisposition::Committed)
                );
            }
            assert_eq!(live_count(&h, "oteryn:item.tibia.i12722").await?, 0);
            assert_eq!(live_count(&h, "oteryn:item.tibia.i12819").await?, 1);
            assert_eq!(scalar(&h,"SELECT progress::bigint FROM game_character_achievement_progress WHERE achievement_key='oteryn:achievement/firefighter'").await?,500);
            assert_eq!(scalar(&h,"SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/firefighter'").await?,i64::from(iteration>=1));
            let after = counts(&h).await?;
            assert_eq!(
                h.root
                    .craft_item(
                        &permit, &authority, &h.node, fence, extinguish, native, &weights, false
                    )
                    .await?,
                Some(UseDisposition::Committed)
            );
            assert_eq!(counts(&h).await?, after);
        }
        assert_eq!(scalar(&h,"SELECT tally::bigint FROM game_world_change_tallies WHERE resource='thornfire_extinguished'").await?,3);
        assert_eq!(
            runtime.visible_entities().creatures.len(),
            2,
            "receipt replays never create another pair"
        );
        // Another source refill obtains a real immutable timer. Only its test clock
        // is moved overdue; the live writer must refuse the expired source bucket.
        let empty = live_item(&h, "oteryn:item.tibia.i12819").await?;
        let mut refill = station_request(
            native,
            deps.admitted.game_session_id,
            1500,
            empty,
            bog,
            selected,
            epoch,
            certificate,
            false,
        )?;
        refill
            .gathering
            .as_mut()
            .ok_or("refill")?
            .mutation
            .expected_revision = 3;
        assert_eq!(
            h.root
                .craft_item(
                    &permit, &authority, &h.node, fence, refill, native, &weights, false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        let filled = live_item(&h, "oteryn:item.tibia.i12722").await?;
        let mut expired = station_request(
            native,
            deps.admitted.game_session_id,
            1510,
            filled,
            flames[0],
            selected,
            epoch,
            certificate,
            true,
        )?;
        expired
            .gathering
            .as_mut()
            .ok_or("extinguish")?
            .mutation
            .expected_revision = 1;
        while crate::durability::item_craft::source_station_wolves(&expired) != Some(false) {
            let n = expired.command.command_id().get() + 1;
            expired.command = command(deps.admitted.game_session_id, n)?;
        }
        let before = counts(&h).await?;
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert_ne!(
            h.root
                .craft_item(
                    &permit,
                    &authority,
                    &h.node,
                    stale,
                    expired.clone(),
                    native,
                    &weights,
                    false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(counts(&h).await?, before);
        let mut clock = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *clock)
            .await?;
        sqlx::query("UPDATE game_spell_item_temporal_schedules SET expires_at_unix_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1 WHERE item_instance_id=encode($1,'hex')::uuid").bind(filled.as_slice()).execute(&mut *clock).await?;
        clock.commit().await?;
        assert_ne!(
            h.root
                .craft_item(
                    &permit, &authority, &h.node, fence, expired, native, &weights, false
                )
                .await?,
            Some(UseDisposition::Committed)
        );
        assert_eq!(counts(&h).await?, before);
        eprintln!(
            "WVAR_THORNFIRE_STATION_NATIVE POSTGRES_CASE_COMPLETED preparation1+1,refill-one-hour-timer,exact-source-ordinal,atomic-filled-debit,conditional500achievement,two-real-native-wolves,typed-window-unknown-COMMIT-resolution,replay-no-duplicate duration={:?}",
            started.elapsed()
        );
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
fn command(session: crate::foundation::GameSessionId, n: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(session, CommandId::new(n).map_err(debug)?))
}
#[allow(
    clippy::too_many_arguments,
    reason = "independent current source station fixture inputs"
)]
fn station_request(
    native: &crate::content::native_gameplay::NativeGameplayState,
    session: crate::foundation::GameSessionId,
    n: u64,
    source: [u8; 16],
    station: &variants::UseStation,
    variant: &variants::Variant,
    epoch: u64,
    certificate: [u8; 32],
    extinguish: bool,
) -> TestResult<ItemCraftRequest> {
    let tile = variant
        .tiles
        .iter()
        .find(|t| t.position == station.position)
        .ok_or("selected source tile")?;
    let ordinal = tile
        .entries
        .iter()
        .position(|e| e.0 == station.appearance)
        .ok_or("actual source ordinal")?;
    let (x, y, z) = station.position;
    let mut placement = certificate.to_vec();
    placement.extend_from_slice(
        &((u64::from(x) << 32)
            | (u64::from(y) << 16)
            | (u64::try_from(z)? << 8)
            | u64::try_from(ordinal)?)
        .to_be_bytes(),
    );
    let policy = native
        .item_policy(&station.material, "definition-r1")
        .ok_or("native source policy")?
        .record();
    Ok(ItemCraftRequest {
        command: command(session, n)?,
        source,
        tool: None,
        ingredient: None,
        station: None,
        intent: n.to_be_bytes().to_vec(),
        enchanting: None,
        gathering: Some(GatheringCraft {
            scope: TerrainScope {
                map_revision: format!(
                    "sha256:{}",
                    certificate
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                content_generation: native.source_digest(),
                reset_epoch: i64::try_from(epoch)?,
            },
            mutation: TerrainMutation {
                tool: source,
                tool_key: policy.production_definition.production_key.clone(),
                tool_revision: policy.production_definition.revision_ref.clone(),
                placement,
                expected_revision: 0,
                before: u32::from(station.appearance),
                after: station.after.map_or(0, u32::from),
                decay: if extinguish {
                    vec![(u32::from(station.appearance), 28_800_000)]
                } else {
                    vec![]
                },
            },
            attempt: GatheringAttempt {
                after: station.after,
                yield_item: station.yield_item,
                consumes_bait: true,
            },
        }),
    })
}
async fn scalar(h: &Harness, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)
}
async fn live_count(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}
async fn live_item(h: &Harness, key: &str) -> TestResult<[u8; 16]> {
    let bytes:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1").bind(key).fetch_one(&h.pool).await?;
    Ok(bytes.try_into().map_err(|_| "item UUID")?)
}
async fn counts(h: &Harness) -> TestResult<Vec<i64>> {
    let mut out = Vec::new();
    for q in [
        "SELECT count(*) FROM game_item_craft_records",
        "SELECT count(*) FROM game_terrain_tool_receipts",
        "SELECT count(*) FROM game_world_change_contributions",
        "SELECT count(*) FROM game_spell_item_temporal_schedules",
        "SELECT count(*) FROM game_account_achievements",
    ] {
        out.push(scalar(h, q).await?);
    }
    Ok(out)
}

// Paste into the station Native PG module (which imports super::*).
// The separate 57-native-death PG case proves the real counter transition;
// this fixture selects its already-established burning phase in the same reset TX.
async fn seed_thorn_epoch(
    h: &Harness,
    world: crate::foundation::WorldId,
    epoch: u64,
    certificate: [u8; 32],
    digest: [u8; 32],
    burning: bool,
) -> TestResult {
    use oteryn_gameplay_world_change::counter::{CounterKind, CounterState};
    let map_revision = format!(
        "sha256:{}",
        certificate
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let mut state = CounterState::new(CounterKind::Thornfire, epoch, false);
    if burning {
        state.stage = 2;
    }
    state.validate(CounterKind::Thornfire).map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp()-interval '60 seconds',to_char(clock_timestamp(),'YYYY-MM'))")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind(digest.as_slice())
        .bind(&map_revision)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_change_counters(world_id,change_key,reset_epoch,state) VALUES(encode($1,'hex')::uuid,'thornfire',$2::text::numeric,$3) ON CONFLICT(world_id,change_key) DO UPDATE SET reset_epoch=excluded.reset_epoch,state=excluded.state")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string())
        .bind(serde_json::to_value(state)?).execute(&mut *tx).await?;
    let day = crate::durability::world_event::changes::record_reset_fixture(&mut tx, world, epoch)
        .await?;
    assert!(day.world_counters);
    assert_eq!(
        day.counter_stages[CounterKind::Thornfire.index()],
        if burning { 2 } else { 1 }
    );
    assert_eq!(day.variant("thornfire"), Some(if burning { 0 } else { 1 }));
    tx.commit().await?;
    Ok(())
}
