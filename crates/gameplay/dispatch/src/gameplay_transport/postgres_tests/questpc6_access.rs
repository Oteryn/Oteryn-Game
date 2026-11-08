//! Quest access mechanisms and complete source reward inventories for this lane.
//! A source reward receipt does not certify a chosen storyline or an outfit grant.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, Trigger};

#[test]
fn questpc6_reward_caches_fibula_demon_devil_draconia() -> TestResult {
    let started = std::time::Instant::now();
    super::quest_catalogue_chests::phase_a_cohort(&[
        "deeper_fibula_quest",
        "demon_helmet_quest",
        "devil_helmet_quest",
        "draconia_quest",
    ])?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        println!(
            "POSTGRES_CASE_COMPLETED questpc6_reward_caches_fibula_demon_devil_draconia duration={:?}",
            started.elapsed()
        );
    }
    Ok(())
}

#[test]
fn questpc6_draconia_both_levers_open_and_restore_their_passages() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("questpc6-draconia-levers", || {
        for (key, position) in [
            (
                "draconia-lever-30006",
                Position {
                    x: 32792,
                    y: 31595,
                    floor: -7,
                },
            ),
            (
                "draconia-lever-30035",
                Position {
                    x: 32792,
                    y: 31579,
                    floor: -7,
                },
            ),
        ] {
            let started = std::time::Instant::now();
            super::quest_world_callbacks::scenario(key, position, false)?;
            println!(
                "POSTGRES_CASE_COMPLETED {key} duration={:?}",
                started.elapsed()
            );
        }
        Ok(())
    })
}

// The only non-claim interaction in the original Fibula source is the well's
// action15005. Exercise its selector through transport without fixture callbacks.
#[test]
fn questpc6_deeper_fibula_well_literal_use_preserves_quest_state() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("questpc6-fibula-well", || {
        for x in [32171, 32172] {
            stateless_callback(
                "deeper-fibula-well",
                Position {
                    x,
                    y: 32439,
                    floor: -7,
                },
                Position {
                    x: 32172,
                    y: 32439,
                    floor: -8,
                },
            )?;
        }
        Ok(())
    })
}

fn stateless_callback(name: &str, trigger: Position, destination: Position) -> TestResult {
    use super::fixture::{Harness, debug, id, run};
    use super::quest_world_callbacks::{local, wire, world};
    use super::{Dependencies, prepare_current_session};
    use crate::content::channel_content_pin::ChannelContentPin;
    use crate::foundation::ChannelRuntimeV1;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

    let started = std::time::Instant::now();
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with(name))
        .ok_or("source callback")?;
    if name == "deeper-fibula-well" {
        assert_eq!(callback.selector.action, Some(15005));
        assert!(callback.selector.position.is_none());
    }
    if let Some(position) = callback.selector.position {
        assert_eq!(position, trigger);
    }
    assert!(callback.storage.is_empty());
    assert!(callback.offerings.is_empty());
    assert!(callback.item_on_target.is_none());
    assert_eq!(callback.teleport, Some(destination));
    let world = world(&callback, trigger)?;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "questpc6_fibula_well", false, false)
            .await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {name} server_version={version}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
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
            pin,
        )
        .map_err(debug)?;
        let reserved = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells =
            crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "current quest fence")?
            .ok_or("current quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        let before = owner
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let key =
            oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                .ok_or("well placement")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let (kind, payload) = match callback.trigger {
            Trigger::StepOut => return Err("departure requires the StepOut fixture".into()),
            Trigger::Use => (
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
            ),
            Trigger::StepIn => (
                1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                    oteryn_protocol_oteryn::world_spatial::StepDirection::East,
                ),
            ),
        };
        let command = if callback.trigger == Trigger::Use {
            let stale = WorldObjectTarget {
                expected_revision: 1,
                ..target
            };
            let refused = wire(
                &owner,
                &mut admitted,
                1,
                kind,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&stale).map_err(debug)?,
            )
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
                UseDisposition::StaleState
            );
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            assert_eq!(
                owner
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                before
            );
            // A stale USE still consumes the transport's action cadence. Advance
            // the fixture clock before a distinct command; replay below stays exact.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            2
        } else {
            1
        };
        let reply = wire(&owner, &mut admitted, command, kind, payload.clone()).await?;
        match callback.trigger {
            Trigger::StepOut => return Err("departure requires the StepOut fixture".into()),
            Trigger::Use => assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            ),
            Trigger::StepIn => assert_eq!(
                oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
            ),
        }
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(destination)
        );
        assert_eq!(
            owner
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            before
        );
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            receipts, 0,
            "stateless travel must not fabricate quest progress"
        );
        // Retrying the same client command cannot teleport twice or add receipts.
        let replay = wire(&owner, &mut admitted, command, kind, payload).await?;
        assert_eq!(replay, reply);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(destination)
        );
        assert_eq!(
            owner
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            before
        );
        println!(
            "POSTGRES_CASE_COMPLETED {name} duration={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn questpc6_cults_orc_passages_preserve_progress_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("questpc6-cults-passages", || {
        for (name, origin, destination) in [
            (
                "cults_of_tibia/orc_entry",
                (33167, 31930, -12),
                (33182, 31837, -15),
            ),
            (
                "cults_of_tibia/orc_exit",
                (33183, 31837, -15),
                (33168, 31930, -12),
            ),
        ] {
            stateless_callback(name, pos(origin), pos(destination))?;
        }
        Ok(())
    })
}

#[test]
fn questpc6_dreamers_challenge_exit_and_maze_passages_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("questpc6-dreamers-passages", || {
        for (name, unique, destination) in [
            (
                "dreamers_challenge_quest/hugo_exit",
                2242,
                (32861, 32240, -9),
            ),
            (
                "dreamers_challenge_quest/maze_2250",
                2250,
                (32915, 32263, -14),
            ),
            (
                "dreamers_challenge_quest/maze_2251",
                2251,
                (32946, 32270, -13),
            ),
            (
                "dreamers_challenge_quest/maze_2252",
                2252,
                (32976, 32270, -14),
            ),
            (
                "dreamers_challenge_quest/maze_2253",
                2253,
                (32933, 32282, -13),
            ),
            (
                "dreamers_challenge_quest/maze_2254",
                2254,
                (32753, 32344, -14),
            ),
            (
                "dreamers_challenge_quest/maze_2255",
                2255,
                (32753, 32344, -14),
            ),
        ] {
            let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
                .into_iter()
                .find(|callback| callback.key.ends_with(name))
                .ok_or("maze callback")?;
            assert_eq!(callback.selector.unique, Some(unique));
            assert!(callback.selector.position.is_none());
            // Unique-id selectors intentionally do not assert source-map coordinates.
            stateless_callback(name, pos((32850, 32250, -9)), pos(destination))?;
        }
        Ok(())
    })
}

fn pos((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}

// Qualify the original source contract without silently promoting the separate
// chosen recipe's additional dragon-kill progression to production readiness.
#[test]
fn questpc6_source_deeper_fibula_well_and_complete_reward_inventory() -> TestResult {
    use std::collections::BTreeSet;
    let shard: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/definitions/quests-00000-00099.json"
    ))?;
    let definition = &shard["records"]
        .as_array()
        .ok_or("quest definitions")?
        .iter()
        .find(|row| row["definition"]["identity"]["key"] == "oteryn:quest.deeper_fibula_quest")
        .ok_or("Fibula definition")?["definition"];
    assert_eq!(definition["kind"], "reward_only");
    assert_eq!(definition["shown_in_quest_log"], false);
    assert!(
        definition["source_refs"]["gates"]
            .as_array()
            .ok_or("Fibula gates")?
            .is_empty()
    );
    let features: BTreeSet<_> = definition["reported_source_readiness"]["features"]
        .as_array()
        .ok_or("Fibula features")?
        .iter()
        .map(|value| value.as_str().ok_or("feature key"))
        .collect::<Result<_, _>>()?;
    assert_eq!(
        features,
        BTreeSet::from(["reward_claim", "teleport", "trigger_use"])
    );
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    let callbacks: Vec<_> = callbacks
        .iter()
        .filter(|callback| callback.quest == "oteryn:quest.deeper_fibula_quest")
        .collect();
    assert_eq!(callbacks.len(), 1, "every source interaction must execute");
    assert_eq!(callbacks[0].key, "oteryn:quest-callback/deeper-fibula-well");
    let coverage: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    ))?;
    let actual: BTreeSet<_> = coverage["quests"]
        .as_array()
        .ok_or("source coverage")?
        .iter()
        .find(|row| row["quest"] == definition["source_refs"]["quest"]["key"])
        .ok_or("Fibula coverage")?["chests"]
        .as_array()
        .ok_or("Fibula chests")?
        .iter()
        .map(|chest| chest["claim"].as_str().ok_or("claim key"))
        .collect::<Result<_, _>>()?;
    let expected: BTreeSet<_> = definition["claims"]
        .as_array()
        .ok_or("Fibula claims")?
        .iter()
        .map(|claim| claim["key"].as_str().ok_or("claim key"))
        .collect::<Result<_, _>>()?;
    assert_eq!(actual, expected, "complete original claim inventory");
    assert_eq!(actual.len(), 6);
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    questpc6_deeper_fibula_well_literal_use_preserves_quest_state()?;
    super::quest_catalogue_chests::phase_a_cohort(&["deeper_fibula_quest"])?;
    println!(
        "POSTGRES_CASE_COMPLETED questpc6_source_deeper_fibula_well_and_complete_reward_inventory duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc6_source_narrative_caches_children_cults_dark_trails_dreamer_druid() -> TestResult {
    let started = std::time::Instant::now();
    super::quest_catalogue_chests::phase_a_cohort(&[
        "children_of_the_revolution_quest",
        "cults_of_tibia_quest",
        "dark_trails_quest",
        "dreamer_s_challenge_quest",
        "druid_outfits_quest",
    ])?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        println!(
            "POSTGRES_CASE_COMPLETED questpc6_source_narrative_caches_children_cults_dark_trails_dreamer_druid duration={:?}",
            started.elapsed()
        );
    }
    Ok(())
}
