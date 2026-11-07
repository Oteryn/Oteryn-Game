//! Real durable raid claim -> current Bundle/Item read -> live native actors.
//! The terrain fixture is explicitly open; catalogue coordinates, counts,
//! delays, creatures and end policy are the unmodified source selection.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::raid_runner::{CommittedRaidAction, RaidCreatureIdentity};
use crate::durability::raid_schedule::RaidWrite;
use crate::foundation::{ChannelRuntimeV1, NodeId, ScopeOwnershipGeneration};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidCatalogue, RaidPosition, SelectedRaid};
use oteryn_gameplay_entities::carrier::RaidSpawnOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn wevents_devovorga_native_incarnation_actor() -> TestResult {
    run_case(
        "wevents_devovorga_incarnation",
        "raid:world-events.rise-anmothra",
        1,
        false,
    )
}
#[test]
fn wevents_devovorga_native_final_actor() -> TestResult {
    run_case(
        "wevents_devovorga_final",
        "raid:world-events.rise-devovorga",
        1,
        false,
    )
}

#[test]
fn wevents_postgres_insect_native_population() -> TestResult {
    run_case(
        "wevents_insectoid_invasion",
        "raid:world-events.greenshore-invasion",
        2,
        false,
    )
}

#[test]
fn wevents_postgres_bank_native_robber() -> TestResult {
    run_case(
        "wevents_bank_robber",
        "raid:world-events.bank-robbery-thais",
        1,
        false,
    )
}

#[test]
fn raids_global_postgres_badger_map_runtime_e2e() -> TestResult {
    run_case("raids_global_badger_e2e", "raid:badger", 100, false)
}

#[test]
fn worldchg_postgres_stampede_map_runtime_e2e() -> TestResult {
    run_case("worldchg_stampede_map", "raid:porthope.stampede", 16, false)
}

#[test]
fn wevents_postgres_flower_native_wave() -> TestResult {
    run_case(
        "wevents_flower_map",
        "oteryn:raid.raid.dryad_invasion",
        25,
        false,
    )
}

#[test]
fn wevents_postgres_kingsday_native_arena() -> TestResult {
    run_case(
        "wevents_kingsday_arena",
        "raid:world-events.kingsday-arena",
        2,
        false,
    )
}

#[test]
fn wevents_postgres_spring_native_hatchling() -> TestResult {
    run_case(
        "wevents_spring_map",
        "raid:world-events.spring-nest-1",
        1,
        false,
    )
}

#[test]
fn wevents_postgres_horestis_single_native_boss() -> TestResult {
    run_case("wevents_mummy_map", "raid:world-events.horestis", 1, true)
}

#[test]
fn raids_global_postgres_sugar_mommy_map_runtime_boss_death_e2e() -> TestResult {
    run_case("raids_global_boss_e2e", "raid:candia.sugarmommy", 1, true)
}

#[test]
fn raids_global_postgres_dracola_death_before_later_wave_e2e() -> TestResult {
    run_case(
        "raids_global_dracola_before_wave_e2e",
        "raid:venore.dracola",
        1,
        true,
    )
}

fn run_case(name: &'static str, key: &'static str, expected: usize, boss: bool) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {name} route_less=true");
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                crate::bestiary_postgres_harness::runtime()?.block_on(async {
                    let harness =
                        Harness::create_with_admission_seed(admin, name, false, false).await?;
                    let result = scenario(&harness, key, expected, boss).await;
                    let cleanup = harness.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "raid live case thread failed")?;
    result.map_err(Box::<dyn std::error::Error>::from)?;
    eprintln!(
        "RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn seed(
    harness: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    key: &ProductionKey,
    selected: &SelectedRaid,
    firing: [u8; 16],
    digest: [u8; 32],
) -> TestResult {
    let origin = if key.as_str() == "oteryn:raid.raid.dryad_invasion" {
        1_939_118_400_000_i64
    } else {
        0
    };
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000),to_char(to_timestamp($3::double precision/1000) AT TIME ZONE 'Europe/Berlin','YYYY-MM'))")
        .bind(owner.world_id.as_bytes().as_slice()).bind(digest.as_slice()).bind(origin).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&harness.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,$3::bigint+60000,$3,NULL,false)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).bind(origin).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,$6,$4,$5,1)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).bind(firing.as_slice())
        .bind(i64::from(selected.definition.waves.iter().map(|wave| wave.delay_ms).max().ok_or("wave")?)
            + i64::from(selected.definition.despawn_after_ms.unwrap_or(0))+origin).bind(digest.as_slice()).bind(origin).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    Ok(())
}

async fn scenario(harness: &Harness, key: &str, expected: usize, boss: bool) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
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
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    if key == "raid:world-events.bank-robbery-thais" {
        let reward = native
            .reward_table()
            .row("oteryn:creature.jesse_the_wicked")
            .map_err(debug)?;
        let bag = reward
            .loot_table
            .entries
            .iter()
            .find(|entry| entry.item.production_key == "oteryn:item.tibia.i13429")
            .ok_or("guaranteed stolen-gold loot")?;
        assert_eq!(
            (bag.min_count, bag.max_count, bag.probability_ppm),
            (1, 1, Some(1_000_000))
        );
    }
    let digest = active.identity().server_artifact_digest();
    let firing = id(91);
    let key = ProductionKey::new(key).map_err(debug)?;
    let phased_boss = key.as_str() == "raid:venore.dracola";
    let flower = key.as_str() == "oteryn:raid.raid.dryad_invasion";
    let mummy = key.as_str() == "raid:world-events.horestis";
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let selected = catalogue
        .get(&key)
        .ok_or("catalogue raid")?
        .select(digest, firing)
        .map_err(debug)?;
    let first_anchor = &selected.definition.waves[0].entries[0].area_anchor;
    let mut area = *selected.areas.get(first_anchor).ok_or("source area")?;
    let declared_areas: std::collections::BTreeSet<_> = selected
        .definition
        .waves
        .iter()
        .flat_map(|wave| wave.entries.iter().map(|entry| &entry.area_anchor))
        .collect();
    assert!(
        declared_areas
            .iter()
            .all(|anchor| selected.areas.contains_key(*anchor))
    );
    for (_, source_area) in selected
        .areas
        .iter()
        .filter(|(key, _)| !flower || *key == first_anchor)
    {
        assert_eq!(source_area.from.floor, area.from.floor);
        assert_eq!(source_area.to.floor, area.to.floor);
        area.from.x = area.from.x.min(source_area.from.x);
        area.from.y = area.from.y.min(source_area.from.y);
        area.to.x = area.to.x.max(source_area.to.x);
        area.to.y = area.to.y.max(source_area.to.y);
    }
    assert_eq!(area.from.floor, area.to.floor);
    if key.as_str() == "raid:porthope.stampede" {
        assert_eq!(selected.definition.announcements.len(), 1);
        assert_eq!(selected.definition.announcements[0].delay_ms, 1_000);
        assert_eq!(
            selected.definition.announcements[0].text,
            "Stampede! The Ape God has stirred up Tiquanda's elephants again!"
        );
    } else if flower {
        assert_eq!(selected.definition.announcements.len(), 4);
        assert_eq!(selected.shared_wave_attempt_budget, Some(250));
    } else {
        assert!(selected.definition.announcements.is_empty());
    }
    let single = selected.areas.values().any(|area| area.from == area.to);
    let entry_x = if mummy || single {
        area.from.x - 1
    } else {
        area.from.x
    };
    let from = (u16::try_from(entry_x)?, u16::try_from(area.from.y)?);
    let to = (u16::try_from(area.to.x)?, u16::try_from(area.to.y)?);
    let native_floor = i8::try_from(area.from.floor.checked_neg().ok_or("native floor")?)?;
    let world = crate::map::spell_tiles::tests::movement_arena::rectangle(from, to, native_floor);
    deps.room = activated.qualified_room().clone();
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err("bundle collision index".into());
    };
    let bundle_digest = index.map().overlay.base().digest();
    let selected = catalogue
        .get(&key)
        .ok_or("catalogue raid")?
        .select(bundle_digest, firing)
        .map_err(debug)?;

    let room = deps
        .room
        .bundle_view(
            cells.clone(),
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: entry_x,
                y: area.from.y,
                floor: area.from.floor,
            },
        )
        .ok_or("bundle combat view")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        digest,
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (entry_x, area.from.y, area.from.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        expected + 1,
        pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = &cells;
    owner.reset_epoch = 1;
    seed(harness, &owner, &key, &selected, firing, bundle_digest).await?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let origin = if flower { 1_939_118_400_000_i64 } else { 0 };
    let now = origin + i64::from(selected.definition.waves.first().ok_or("wave")?.delay_ms);
    if key.as_str() == "raid:porthope.stampede" || flower {
        let notice_at = if flower { origin } else { 1_000 };
        let announcement = harness
            .root
            .advance_raid_run(
                &harness.node,
                firing,
                owner.channel_id,
                generation,
                selected.definition.clone(),
                notice_at,
            )
            .await?
            .ok_or("Stampede announcement")?;
        assert!(matches!(
            announcement.action,
            CommittedRaidAction::Announcement { .. }
        ));
        harness
            .root
            .acknowledge_raid_effect(&harness.node, announcement.identity, vec![], notice_at)
            .await?;
    }
    let wave = harness
        .root
        .advance_raid_run(
            &harness.node,
            firing,
            owner.channel_id,
            generation,
            selected.definition.clone(),
            now,
        )
        .await?
        .ok_or("committed wave")?;
    let outcomes = owner
        .apply_raid_wave(&wave, &selected, bundle_digest)
        .await
        .map_err(|_| "physical raid wave")?;
    assert_eq!(outcomes.len(), expected);
    assert!(outcomes.iter().all(|outcome| outcome.placed));
    assert_eq!(
        owner
            .apply_raid_wave(&wave, &selected, bundle_digest)
            .await
            .map_err(|_| "physical raid replay")?,
        outcomes
    );
    let statuses = {
        let runtime = deps.runtime.lock().await;
        let statuses = runtime.raid_creatures(firing).map_err(debug)?;
        assert_eq!(statuses.len(), expected);
        let mut occupied = std::collections::BTreeSet::new();
        for status in &statuses {
            assert!(status.alive);
            assert_eq!(
                status.boss,
                selected
                    .boss_entries
                    .contains(&(status.identity.wave, status.identity.entry))
            );
            let RaidSpawnOutcome::Placed(actor) = status.outcome else {
                return Err("missing actor".into());
            };
            let at = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            let entry = selected
                .definition
                .waves
                .get(usize::from(status.identity.wave))
                .and_then(|wave| wave.entries.get(usize::from(status.identity.entry)))
                .ok_or("source entry")?;
            let source_area = selected
                .areas
                .get(&entry.area_anchor)
                .ok_or("entry source area")?;
            assert!(source_area.contains(RaidPosition {
                x: at.x,
                y: at.y,
                floor: at.floor
            }));
            assert!(occupied.insert((at.x, at.y, at.floor)));
            let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
            assert!(snapshot.maximum_health > 0);
            assert_eq!(
                snapshot.state.policy.definition_key,
                entry.creature.as_str()
            );
        }
        statuses
    };
    if flower {
        // This fixture covers the first source rectangle; the owner PG case drives
        // all four timed waves and repeat admission without altering the definition.
        assert_eq!(
            harness
                .root
                .acknowledge_raid_effect(&harness.node, wave.identity, outcomes, now)
                .await?,
            RaidWrite::Applied
        );
        return Ok(());
    }
    if !boss || mummy {
        assert_eq!(
            harness
                .root
                .acknowledge_raid_effect(&harness.node, wave.identity, outcomes, now)
                .await?,
            RaidWrite::Applied
        );
        let end = harness
            .root
            .advance_raid_run(
                &harness.node,
                firing,
                owner.channel_id,
                generation,
                selected.definition.clone(),
                now,
            )
            .await?
            .ok_or("persistent schedule end")?;
        assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: false });
        let mut runtime = deps.runtime.lock().await;
        let stamp = runtime.issue_owner_work().map_err(debug)?;
        runtime
            .finish_raid_run(stamp, firing, false)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .raid_creatures(firing)
                .map_err(debug)?
                .iter()
                .filter(|status| status.alive)
                .count(),
            expected
        );
        drop(runtime);
        harness
            .root
            .acknowledge_raid_effect(&harness.node, end.identity, Vec::new(), now)
            .await?;
    } else if phased_boss {
        assert_eq!(
            harness
                .root
                .acknowledge_raid_effect(&harness.node, wave.identity, outcomes, now)
                .await?,
            RaidWrite::Applied
        );
    }
    // The existing test-only damage writer commits through real HP/actor state;
    // no fabricated database death or administrative despawn substitutes for it.
    let mut deaths = Vec::with_capacity(statuses.len());
    {
        let mut runtime = deps.runtime.lock().await;
        for status in &statuses {
            let RaidSpawnOutcome::Placed(actor) = status.outcome else {
                return Err("placed actor".into());
            };
            let health = runtime
                .companion_snapshot(actor)
                .map_err(debug)?
                .maximum_health;
            // The carrier rejects an unsequenced occurrence containing a NUL byte, so the raw
            // big-endian index (e.g. [0, 0]) cannot name the hit; use a NUL-free text identity.
            let occurrence = format!(
                "raid-live-death:{}:{}:{}",
                status.identity.wave, status.identity.entry, status.identity.index
            );
            runtime
                .commit_monster_lab_damage(actor, occurrence.as_bytes(), health)
                .map_err(debug)?;
            assert!(!runtime.contains_live_creature(actor));
            deaths.push(RaidCreatureIdentity {
                wave: status.identity.wave,
                entry: status.identity.entry,
                index: status.identity.index,
            });
        }
    }
    if boss && !mummy {
        // Sugar dies before its wave ACK; Dracola dies after ACK, while a
        // delayed poll makes the later sourced Lich wave already due. Both
        // must end through the actual driver before any further new spawn.
        let drive_at = if phased_boss {
            i64::from(
                selected
                    .definition
                    .waves
                    .last()
                    .ok_or("last wave")?
                    .delay_ms,
            ) + 1
        } else {
            now + 1
        };
        let mut progress = crate::gameplay_transport::raid_live::RaidEffectProgress::default();
        owner
            .drive_raid_firing(
                firing,
                &key,
                &catalogue,
                bundle_digest,
                drive_at,
                &mut progress,
            )
            .await
            .map_err(|_| "raid effect driver after uncertain wave ACK")?;
        if phased_boss {
            let later: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_world_raid_effects WHERE action_kind=2 AND wave>0",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(later, 0, "boss death precedes every newly due later wave");
            let mut retry = crate::gameplay_transport::raid_live::RaidEffectProgress::default();
            owner
                .drive_raid_firing(
                    firing,
                    &key,
                    &catalogue,
                    bundle_digest,
                    drive_at + 1,
                    &mut retry,
                )
                .await
                .map_err(|_| "raid effect driver after settled boss end")?;
        }
        let placed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_world_raid_creature_placements WHERE placed",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(placed, 1);
        let cleaned: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE action_kind=3 AND cleanup AND state=2")
            .fetch_one(&harness.pool).await?;
        assert_eq!(cleaned, 1);
    } else {
        assert_eq!(
            harness
                .root
                .record_raid_creature_deaths(&harness.node, wave.identity, deaths)
                .await?,
            RaidWrite::Applied
        );
    }
    assert!(
        harness
            .root
            .due_raid_firings(owner.channel_id)
            .await?
            .is_empty()
    );
    let result = harness.root.query_raid_schedule(owner.channel_id).await?;
    assert!(!result.running);
    assert_eq!(result.active_creatures, 0);
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_world_raid_effects WHERE state=1")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(pending, 0);
    Ok(())
}
