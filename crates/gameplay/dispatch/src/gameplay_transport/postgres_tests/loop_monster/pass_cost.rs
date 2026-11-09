//! MAINRED-NB: measure the real held pass over a channel-sized idle census.
use super::*;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::carrier::{NativeRaidSpawnSpec, RaidSpawnIdentity, RaidSpawnOutcome};

#[test]
fn monster_pass_cost_idle_channel_does_not_query_every_empty_tile() -> TestResult {
    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let (deps, rat) = super::fixture::dependencies_with_capacity(active, &pin, 1024)?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        let permit = owner.spell_lane_permit().await.ok_or("lane")?;
        let mut runtime = deps.runtime.lock().await;
        let objects = deps.door.lock().await;
        let policy = runtime.companion_snapshot(rat).map_err(debug)?.state.policy;
        let spec = NativeRaidSpawnSpec::qualified(
            &policy.definition_key,
            &policy.definition_revision,
            native.source_digest(),
            false,
        )
        .map_err(debug)?;
        let mut actors = vec![rat];
        'cells: for x in -14..18 {
            for y in -15..17 {
                let at = MovementLocalPosition { x, y, floor: -7 };
                let admits = |runtime: &crate::foundation::ChannelRuntimeV1, at| {
                    Some(oteryn_gameplay_durability::durability::native_world_read::qualified_combat_tile(
                        &deps.room, runtime, &*objects, at,
                    ).is_ok_and(|tile| tile.ground_present() && !tile.block_solid() && !tile.floor_change()))
                };
                if admits(&runtime, at) != Some(true)
                    || runtime.position_occupied_by_other(rat, at).map_err(debug)?
                    || runtime.read_actor_position(rat).map_err(debug)?.position() == at
                {
                    continue;
                }
                let stamp = runtime.issue_owner_work().map_err(debug)?;
                let receipt = runtime
                    .realize_raid_spawn(
                        stamp,
                        RaidSpawnIdentity {
                            firing_id: *world.as_bytes(),
                            wave: 0,
                            entry: 0,
                            index: u16::try_from(actors.len())?,
                        },
                        &spec,
                        Some(at),
                        admits,
                    )
                    .map_err(debug)?;
                let RaidSpawnOutcome::Placed(actor) = receipt.outcome else {
                    return Err("qualified creature placement refused".into());
                };
                actors.push(actor);
                if actors.len() == 512 {
                    break 'cells;
                }
            }
        }
        assert_eq!(
            actors.len(),
            512,
            "real source-qualified positioned creatures"
        );
        drop(objects);
        let mut states = owner.states.spell_states.lock().await;
        let creature = crate::content::ProjectV2DefinitionRef {
            family: crate::content::ProjectV2Family::Creature,
            key: policy.definition_key.clone(),
            revision: policy.definition_revision.clone(),
        };
        for actor in &actors {
            states
                .monster_melee
                .creature_ai
                .admit_native_creature(&runtime, native, *actor, &creature, 0)
                .map_err(debug)?;
        }
        let mut attack = owner.states.attack.lock().await;
        let revisions = crate::ability::RevisionSet::new(
            "rules-1",
            "content-1",
            "world-1",
            "formula-1",
            "sim-1",
        )
        .map_err(debug)?;
        for now in [10_000_000, 10_050_000] {
            let before = ground_scans(owner.root).await?;
            let started = std::time::Instant::now();
            owner
                .drive_monster_profiles(
                    &mut runtime,
                    &mut states,
                    &mut attack,
                    &deps.room,
                    native,
                    &permit,
                    &revisions,
                    &[],
                    now,
                )
                .await
                .map_err(debug)?;
            let elapsed = started.elapsed();
            let scans = ground_scans(owner.root).await? - before;
            eprintln!(
                "MONSTER-PASS-COST creatures={} now={now} elapsed_ms={} ground_scans={scans}",
                actors.len(),
                elapsed.as_millis()
            );
            assert!(
                scans <= 64,
                "held monster pass scanned Ground {scans} times in {elapsed:?}"
            );
        }
        assert!(
            actors
                .iter()
                .all(|actor| runtime.contains_live_creature(*actor))
        );
        Ok(())
    })
}

async fn ground_scans(root: &crate::durability::DurabilityRoot) -> TestResult<i64> {
    root.try_issue_semantic_pass().map_err(debug)?.run(|holder, _| {
        Box::pin(async move {
            // The root has one semantic connection. Flush its previous pass's counters,
            // then read in a separate autocommit statement (no cached stats snapshot).
            sqlx::query("SELECT pg_stat_force_next_flush()")
                .execute(&mut **holder).await?;
            let scans = sqlx::query_scalar::<_, i64>(
                "SELECT seq_scan + COALESCE(idx_scan,0) FROM pg_stat_user_tables WHERE relname='game_item_ground_locations'",
            ).fetch_one(&mut **holder).await?;
            Ok(scans)
        })
    }).await.map_err(debug)
}
