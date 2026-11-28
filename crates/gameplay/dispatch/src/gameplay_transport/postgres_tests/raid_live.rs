//! Real durable raid claim -> current Bundle/Item read -> live native actors.
//! The terrain fixture is explicitly open; catalogue coordinates, counts,
//! delays, creatures and end policy are the unmodified source selection.
#[path = "raid_live/cake.rs"]
mod cake;

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

const HALLOWEEN_HARE: &str = "oteryn:raid.kazordoon_raids.the_halloween_hare";

#[test]
fn w05d_hare_dispatch_announces_and_places_one_kazordoon_actor_on_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case_with_admin(admin, "w05d_hare_dispatch", HALLOWEEN_HARE, 1, false)
}

#[test]
fn world_src_postgres_flower_month_places_canonical_dryads_and_replays_once() -> TestResult {
    run_case(
        "world_src_flower_physical",
        "oteryn:raid.raid.dryad_invasion",
        25,
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

#[test]
fn questpc7_hunting_for_tokens_exchanges_native_raid_loot() -> TestResult {
    run_case("questpc7_earned_token", "raid:svargrond.zushuka", 1, true)
}

#[test]
fn nbhold_raid_discovery_and_execution_survive_changing_publication_postgres() -> TestResult {
    run_case("nbhold_raid_publication", "raid:badger", 100, false)
}

fn run_case(name: &'static str, key: &'static str, expected: usize, boss: bool) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {name} route_less=true");
        return Ok(());
    };
    run_case_with_admin(admin, name, key, expected, boss)
}

fn run_case_with_admin(
    admin: String,
    name: &'static str,
    key: &'static str,
    expected: usize,
    boss: bool,
) -> TestResult {
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                crate::bestiary_postgres_harness::runtime()?.block_on(async {
                    let harness = if name == "questpc7_earned_token" {
                        Harness::create_quest_reward(admin, name, true).await?
                    } else {
                        Harness::create_with_admission_seed(admin, name, false, false).await?
                    };
                    let result = scenario(
                        &harness,
                        key,
                        expected,
                        boss,
                        name == "nbhold_raid_publication",
                    )
                    .await;
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
    eprintln!("RAIDS_GLOBAL POSTGRES_CASE_COMPLETED {name}");
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
    let content = owner.runtime.lock().await.content_pin().clone();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(digest.as_slice())
        .bind(content.server_artifact_digest().as_slice())
        .bind(content.client_artifact_digest().as_slice())
        .bind(content.frame_binding_digest().as_slice())
        .execute(&harness.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&harness.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).bind(firing.as_slice())
        .bind(i64::from(selected.definition.waves.iter().map(|wave| wave.delay_ms).max().ok_or("wave")?)
            + i64::from(selected.definition.despawn_after_ms.unwrap_or(0))).bind(digest.as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    Ok(())
}

async fn scenario(
    harness: &Harness,
    key: &str,
    expected: usize,
    boss: bool,
    contention: bool,
) -> TestResult {
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
    let digest = active.identity().server_artifact_digest();
    let firing = id(91);
    let key = ProductionKey::new(key).map_err(debug)?;
    let phased_boss = key.as_str() == "raid:venore.dracola";
    let token_raid = key.as_str() == "raid:svargrond.zushuka";
    let hare = key.as_str() == HALLOWEEN_HARE;
    let flower = key.as_str() == "oteryn:raid.raid.dryad_invasion";
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let selected = catalogue
        .get(&key)
        .ok_or("catalogue raid")?
        .select(digest, firing)
        .map_err(debug)?;
    let mut area = if flower {
        *selected
            .areas
            .get(&selected.definition.waves[0].entries[0].area_anchor)
            .ok_or("Flower first source area")?
    } else {
        *selected.areas.values().next().ok_or("source area")?
    };
    assert_eq!(
        selected.areas.len(),
        if flower {
            4
        } else if phased_boss {
            2
        } else {
            1
        }
    );
    for source_area in selected.areas.values().filter(|_| !flower) {
        assert_eq!(source_area.from.floor, area.from.floor);
        assert_eq!(source_area.to.floor, area.to.floor);
        area.from.x = area.from.x.min(source_area.from.x);
        area.from.y = area.from.y.min(source_area.from.y);
        area.to.x = area.to.x.max(source_area.to.x);
        area.to.y = area.to.y.max(source_area.to.y);
    }
    assert_eq!(area.from.floor, area.to.floor);
    if flower { assert_eq!(selected.definition.announcements.len(), 4); }
    else if key.as_str() == "raid:porthope.stampede" {
        assert_eq!(selected.definition.announcements.len(), 1);
        assert_eq!(selected.definition.announcements[0].delay_ms, 1_000);
        assert_eq!(selected.definition.announcements[0].text,
            "Stampede! The Ape God has stirred up Tiquanda's elephants again!");
    } else if !hare {
        assert!(selected.definition.announcements.is_empty());
    }
    let margin = i32::from(token_raid || hare);
    let from = (u16::try_from(area.from.x - margin)?, u16::try_from(area.from.y - margin)?);
    let to = (u16::try_from(area.to.x + margin)?, u16::try_from(area.to.y + margin)?);
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
    let first_area = selected
        .areas
        .get(&selected.definition.waves[0].entries[0].area_anchor)
        .ok_or("first source spawn area")?;
    let room = deps
        .room
        .bundle_view(
            cells.clone(),
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(from.0),
                y: i32::from(from.1),
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
        (i32::from(from.0), i32::from(from.1), area.from.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        expected + 1 + usize::from(token_raid),
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
    owner.movement_cells = (&cells).into();
    if token_raid {
        super::questpc7_tokens_fixtures::prepare_earned_journey(&owner, harness).await?;
    }
    owner.reset_epoch = 1;
    seed(harness, &owner, &key, &selected, firing, bundle_digest).await?;
    if hare {
        let session = deps.admitted.game_session_id;
        let messages = &owner.states.server_messages;
        assert!(messages.join(session, 1));
        let mut progress = crate::gameplay_transport::raid_live::RaidEffectProgress::default();
        owner
            .drive_raid_firing(firing, &key, &catalogue, bundle_digest, 0, &mut progress)
            .await
            .map_err(|_| "Hare effects at the save tick")?;
        assert_eq!(
            messages.take(session, 1),
            Some(oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(
                "Beware, beware the halloween hare.".into(),
            )),
        );
        let mut retry = crate::gameplay_transport::raid_live::RaidEffectProgress::default();
        owner
            .drive_raid_firing(firing, &key, &catalogue, bundle_digest, 60_000, &mut retry)
            .await
            .map_err(|_| "Hare effects on replay")?;
        assert_eq!(messages.take(session, 1), None);
        let runtime = deps.runtime.lock().await;
        let actors = runtime.raid_creatures(firing).map_err(debug)?;
        let [status] = actors.as_slice() else {
            return Err("expected exactly one Hare spawn".into());
        };
        let RaidSpawnOutcome::Placed(actor) = status.outcome else {
            return Err("Hare placement refused".into());
        };
        assert!(status.alive);
        let at = runtime.read_actor_position(actor).map_err(debug)?.position();
        assert_eq!(
            RaidPosition { x: at.x, y: at.y, floor: at.floor },
            first_area.from,
        );
        assert_eq!(
            runtime.companion_snapshot(actor).map_err(debug)?.state.policy.definition_key,
            selected.definition.waves[0].entries[0].creature.as_str(),
        );
        drop(runtime);
        let placed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_world_raid_creature_placements WHERE placed",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(placed, 1);
        return Ok(());
    }
    if contention {
        use crate::durability::DurabilityRoot;
        let foreground = harness.root.foreground_demand()?;
        let shutdown = oteryn_foundation::CancellationToken::new();
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        let probe = async {
            let deadline = tokio::time::Instant::now()
                + crate::durability::ROOT_RECOVERY_WINDOW * 2
                + std::time::Duration::from_secs(2);
            let mut present = false;
            let mut observed = None;
            loop {
                let publication: Option<i64> =
                    sqlx::query_scalar("SELECT max(observed_at_ms) FROM game_world_raid_activity")
                        .fetch_one(&harness.pool)
                        .await?;
                // Change the source after each successful publication, rather
                // than toggling on a timer that can alias the owner's cadence.
                if publication != observed {
                    observed = publication;
                    if present {
                        owner
                            .states
                            .gameplay_idle_presence
                            .forget(admitted.game_session_id);
                    } else {
                        owner.states.gameplay_idle_presence.observe(admitted);
                    }
                    present = !present;
                }
                let effects: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_world_raid_effects WHERE state=2",
                )
                .fetch_one(&harness.pool)
                .await?;
                if effects > 0 {
                    return Ok::<(), Box<dyn std::error::Error>>(());
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "changing publication consumed every fairness turn before discovery/execution"
                );
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        };
        let result = crate::gameplay_transport::first(probe, async {
            DurabilityRoot::with_background_holder(owner.run_raid_effects(&shutdown)).await;
        })
        .await;
        result.ok_or("raid owner stopped before execution")??;
        assert!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_world_raid_creature_placements WHERE placed"
            )
            .fetch_one(&harness.pool)
            .await?
                > 0
        );
        drop(foreground);
        return Ok(());
    }
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let now = i64::from(selected.definition.waves.first().ok_or("wave")?.delay_ms);
    if key.as_str() == "raid:porthope.stampede" {
        let announcement = harness.root.advance_raid_run(
            &harness.node, firing, owner.channel_id, generation, selected.definition.clone(), 1_000,
        ).await?.ok_or("Stampede announcement")?;
        assert!(matches!(announcement.action, CommittedRaidAction::Announcement { .. }));
        harness.root.acknowledge_raid_effect(&harness.node, announcement.identity, vec![], 1_000).await?;
    }
    let mut wave = harness.root.advance_raid_run(
        &harness.node, firing, owner.channel_id, generation, selected.definition.clone(), now,
    ).await?.ok_or("committed wave")?;
    if flower {
        assert!(matches!(
            wave.action,
            CommittedRaidAction::Announcement { .. }
        ));
        assert_eq!(
            harness
                .root
                .acknowledge_raid_effect(&harness.node, wave.identity, Vec::new(), now)
                .await?,
            RaidWrite::Applied
        );
        wave = harness
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
            .ok_or("Flower first committed wave")?;
    }
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
            assert_eq!(status.boss, boss);
            let RaidSpawnOutcome::Placed(actor) = status.outcome else {
                return Err("missing actor".into());
            };
            let at = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            assert!(first_area.contains(RaidPosition {
                x: at.x,
                y: at.y,
                floor: at.floor
            }));
            assert!(occupied.insert((at.x, at.y, at.floor)));
            let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
            assert!(snapshot.maximum_health > 0);
            assert_eq!(
                snapshot.state.policy.definition_key,
                selected.definition.waves[0].entries[0].creature.as_str()
            );
        }
        statuses
    };
    if token_raid {
        assert_eq!(
            harness.root.acknowledge_raid_effect(&harness.node, wave.identity, outcomes, now).await?,
            RaidWrite::Applied,
        );
        let RaidSpawnOutcome::Placed(zushuka) = statuses[0].outcome else {
            return Err("source Zushuka actor".into());
        };
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        return super::questpc7_tokens_fixtures::earned_journey(
            &mut owner, &mut admitted, harness, zushuka,
        ).await;
    }
    if flower {
        assert_eq!(
            selected.definition.waves[0].entries[0].creature.as_str(),
            "oteryn:creature.dryad"
        );
        assert_eq!(
            harness
                .root
                .acknowledge_raid_effect(&harness.node, wave.identity, outcomes, now)
                .await?,
            RaidWrite::Applied
        );
        assert!(
            harness
                .root
                .advance_raid_run(
                    &harness.node,
                    firing,
                    owner.channel_id,
                    generation,
                    selected.definition.clone(),
                    now + 899_999
                )
                .await?
                .is_none()
        );
        let placed: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_world_raid_creature_placements WHERE placed",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(placed, 25, "replay keeps exactly one native wave");
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .raid_creatures(firing)
                .map_err(debug)?
                .iter()
                .filter(|status| status.alive)
                .count(),
            25
        );
        eprintln!("WORLD_SRC POSTGRES_CASE_COMPLETED Flower physical native25 replay25");
        return Ok(());
    }
    if !boss {
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
    let credited = if key.as_str() == "raid:porthope.stampede" {
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner.progression_binding(admitted.game_session_id).is_some(),
            "the credited raider has a bound progression profile"
        );
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("Stampede reward fence")?;
        let character =
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                fence.game_session_id,
                crate::foundation::CharacterLease::new(character, fence.character_lease_generation)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let frame_binding = deps.runtime.lock().await.content_pin().frame_binding_digest();
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(owner.world_id.as_bytes().as_slice())
            .bind(owner.channel_id.as_bytes().as_slice())
            .execute(&harness.pool)
            .await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(owner.world_id.as_bytes().as_slice())
            .bind(owner.channel_id.as_bytes().as_slice())
            .bind(digest.as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(frame_binding.as_slice())
            .execute(&harness.pool)
            .await?;
        let queue = owner.states.attack.lock().await.kills().clone();
        Some((fence, character, queue))
    } else {
        None
    };
    // The existing test-only damage writer commits through real HP/actor state;
    // no fabricated database death or administrative despawn substitutes for it.
    let mut deaths = Vec::with_capacity(statuses.len());
    for (index, status) in statuses.iter().enumerate() {
        let RaidSpawnOutcome::Placed(actor) = status.outcome else {
            return Err("placed actor".into());
        };
        let mut runtime = deps.runtime.lock().await;
        let health = runtime
            .companion_snapshot(actor)
            .map_err(debug)?
            .maximum_health;
        if let Some((fence, character, queue)) = credited.as_ref().filter(|_| index < 6) {
            use crate::gameplay_transport::kill_reward::{
                DrainOutcome, KillSink, record_creature_kill,
            };
            let command = crate::foundation::CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(100 + u64::try_from(index)?).map_err(debug)?,
            );
            let observed = owner.owner_now();
            let at = observed.get() / 1000;
            runtime
                .borrow_exact_actor_commit_at(at)
                .commit_damage_for_bound_attacker(
                    actor,
                    deps.actor,
                    command,
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: b"oteryn:creature.terrified_elephant",
                        occurrence: b"stampede",
                        binding: b"stampede-credit",
                        damage: health,
                    },
                )
                .map_err(debug)?;
            record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                actor,
                crate::combat::CapturedRewardPrincipal {
                    character: *character,
                    lease_generation: fence.character_lease_generation,
                    session: fence.game_session_id,
                    actor: deps.actor,
                },
                at,
                Some(observed),
            );
            assert!(queue.has_session(fence.game_session_id));
            assert!(!runtime.contains_live_creature(actor));
            drop(runtime);
            assert_eq!(
                owner.drain_kill_rewards(fence.game_session_id).await,
                DrainOutcome::Drained
            );
            let (kills, awards): (i64, i64) = sqlx::query_as(
                "SELECT progress.kill_count,
                        (SELECT count(*) FROM game_account_achievements
                         WHERE account_id=root.account_id
                           AND achievement_key='oteryn:achievement/trail_of_the_ape_god')
                 FROM game_character_roots root
                 JOIN game_character_bestiary_progress progress USING (character_id)
                 WHERE root.character_id=encode($1,'hex')::uuid
                   AND progress.race_key='oteryn:creature.terrified_elephant'",
            )
            .bind(character.as_bytes().as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(kills, i64::try_from(index)? + 1);
            assert_eq!(awards, i64::from(index >= 4));
        } else {
            // The carrier rejects an unsequenced occurrence containing a NUL byte.
            let occurrence = format!(
                "raid-live-death:{}:{}:{}",
                status.identity.wave, status.identity.entry, status.identity.index
            );
            runtime
                .commit_monster_lab_damage(actor, occurrence.as_bytes(), health)
                .map_err(debug)?;
            assert!(!runtime.contains_live_creature(actor));
        }
        deaths.push(RaidCreatureIdentity {
            wave: status.identity.wave,
            entry: status.identity.entry,
            index: status.identity.index,
        });
    }
    if boss {
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
