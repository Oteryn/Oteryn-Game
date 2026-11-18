//! Publish the source room views from the same World bundle used by the Channel.
use super::TestResult;
use super::fixture::{Harness, debug, run};
use crate::content::bosses::activity_template::captured_boss_levers;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::foundation::MovementLocalPosition;

#[test]
fn ddepths_three_warzone_rooms_publish_with_one_landing_and_individual_capacity_postgres()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run("ddepths-qualified-rooms", || {
        run(async |admin| {
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33461,
                y: 32267,
                floor: 15,
            })?;
            let candidates = captured_boss_levers().map_err(debug)?;
            let rows: Vec<_> = candidates
                .into_iter()
                .filter(|candidate| {
                    candidate
                        .path
                        .starts_with("content/encounters/bosses/warzone-")
                })
                .collect();
            assert_eq!(rows.len(), 3);
            let mut templates = Vec::with_capacity(rows.len());
            for candidate in rows {
                let template = scene
                    .controller
                    .qualify_activity_template(&scene.room, &scene.room, "content-1", &candidate)
                    .map_err(|error| format!("{}: {error:?}", candidate.path))?;
                assert_eq!(template.entry_positions().len(), 1);
                assert_eq!(template.minimum_players(), 1);
                assert!(template.maximum_players() >= 2);
                assert_eq!(template.cooldown_seconds(), 36_000);
                assert_eq!(template.time_limit_seconds(), 1_800);
                assert_eq!(template.after_kill_seconds(), 1_200);
                assert!(template.exit_teleporter().is_some());
                templates.push(template);
            }

            let harness =
                Harness::create_with_admission_seed(admin, "ddepths_qualified_rooms", false, false)
                    .await?;
            let result: TestResult = async {
                ActivityHandoffOwner::new(harness.root.clone())
                    .publish_qualified_boss_rooms(&harness.node, &templates)
                    .await?;
                let published: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_boss_room_publications WHERE boss_key = ANY($1)",
                )
                .bind(
                    templates
                        .iter()
                        .map(|room| room.boss_key())
                        .collect::<Vec<_>>(),
                )
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(published, 3);
                Ok(())
            }
            .await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup
        })
    })
}

#[test]
fn ddepths_warzone_two_independent_players_enter_and_exit_through_paced_step_postgres() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run("ddepths-paced-entry-exit", || {
        run(async |admin| {
            super::reset();
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33461,
                y: 32267,
                floor: 15,
            })?;
            let active = scene.controller.active().ok_or("active Warzone content")?;
            let candidate = captured_boss_levers()
                .map_err(debug)?
                .into_iter()
                .find(|row| {
                    row.creature_key.as_deref() == Some("oteryn:creature.the_baron_from_below")
                })
                .ok_or("Baron room capture")?;
            let template = scene
                .controller
                .qualify_activity_template(&scene.room, &scene.room, "content-1", &candidate)
                .map_err(debug)?;
            let harness =
                Harness::create_with_admission_seed(admin, "ddepths_two_players", false, false)
                    .await?;
            let result = paced_entry_and_exit(&harness, &scene, active, &template).await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup
        })
    })
}

async fn paced_entry_and_exit(
    harness: &Harness,
    scene: &super::srx3_activity_source_fixture::SourceScene,
    active: &crate::content::ActiveGeneration,
    template: &crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
) -> TestResult {
    use crate::foundation::RuntimeScopeRefV1;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    let deps = scene.dependencies()?;
    let evidence = super::house_transfer::RecoverySource::start(harness).await?;
    super::house_transfer::prepare_owned_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(harness, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
    owner
        .activity_runtimes
        .register_template(template)
        .map_err(debug)?;
    super::boss_activity::issue_content_and_reset(harness, &scene.pin).await?;
    ActivityHandoffOwner::new(harness.root.clone())
        .publish_qualified_boss_rooms(&harness.node, std::slice::from_ref(template))
        .await?;

    let supported = [4, 6, 7, 10, 11, 15, 17, 18, 23, 30, 92, 117, 385];
    {
        use crate::gameplay_transport::world_required_capabilities as caps;
        let runtime = owner.runtime.lock().await;
        let offers = caps::with_native_map_presentation_offer(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            Some(active),
            Some(&scene.room),
            runtime.content_pin(),
        );
        owner.required_capabilities = caps::BoundRequiredCapabilities::for_channel_boot(
            &[11, 18, 30],
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation().get(),
            "policy-1",
            offers,
        )
        .map_err(debug)?;
    }
    let mut first = deps.admitted;
    first.continuity.selected_capabilities =
        crate::gameplay_transport::connection::fresh_capability_selection(&owner, &supported)
            .await
            .ok_or("native capability negotiation")?;
    super::srx3_activity_source_fixture::prepare_player(harness, &owner, &mut first, 1999).await?;
    let second =
        super::srx3_activity_source_fixture::prepare_independent_peer(harness, &owner, &first)
            .await?;
    let instance = super::activity_runtime::allocate_ready_instance(harness, 237).await?;
    open_captured_test_entrance(&owner)?;

    let entry_clock: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let mut entrants = Vec::new();
    for (source, arrival) in [(first, entry_clock), (second, entry_clock + 1)] {
        harness.root.set_test_quest_clock(arrival)?;
        let actor = source.runtime_actor.ok_or("current Channel actor")?;
        let (outcome, _) = owner
            .paced_step(actor, source.game_session_id, StepDirection::West)
            .await;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
        let switched = owner.activity_runtimes.current_connection(source);
        assert_ne!(switched.game_session_id, source.game_session_id);
        assert_eq!(
            switched.item_fence.ok_or("Instance fence")?.runtime_scope,
            instance
        );
        owner
            .activity_runtimes
            .finish_switch(source, switched)
            .map_err(debug)?;
        entrants.push(switched);
    }
    let members: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_boss_room_participants WHERE admission =
         (SELECT admission FROM game_boss_room_activities WHERE boss_key = $1)",
    )
    .bind(template.boss_key())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(members, 2);
    let cooldowns: Vec<i64> = sqlx::query_scalar(
        "SELECT c.available_at FROM game_character_boss_cooldowns c
         JOIN game_boss_room_participants p
           ON p.admission = c.admission AND p.character_id = c.character_id
         WHERE c.boss_key = $1 ORDER BY c.available_at",
    )
    .bind(template.boss_key())
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(
        cooldowns,
        vec![
            (entry_clock + 36_000) * 1_000,
            (entry_clock + 36_001) * 1_000
        ],
        "sharing the encounter preserves each character's own entry clock"
    );

    let entry_fence = entrants[0].item_fence.ok_or("first Instance fence")?;
    let registered = owner
        .activity_runtimes
        .get(instance, entry_fence.scope_ownership_generation.get())
        .ok_or("real qualified Instance runtime")?;
    let mut view = owner.runtime_view(&registered);
    let exit = template.exit_teleporter().ok_or("sealed Warzone return")?;
    let trigger = exit.trigger();
    let target = {
        let runtime = view.runtime.lock().await;
        view.wire_observation(
            &runtime,
            MovementLocalPosition {
                x: trigger.x,
                y: trigger.y,
                floor: i16::try_from(trigger.z)?,
            },
        )
        .ok_or("return trigger wire position")?
        .actor_position
    };
    for entrant in entrants {
        let actor = entrant.runtime_actor.ok_or("current Instance actor")?;
        let route = view
            .item_walk_route(
                actor,
                entrant.game_session_id,
                crate::gameplay_transport::paced_movement::AutoWalkTarget::Cell(target),
            )
            .await
            .map_err(debug)?
            .ok_or("authored return is reachable")?;
        assert!(!route.steps.is_empty());
        for direction in route.steps {
            view.clock_origin -= std::time::Duration::from_secs(1);
            let (step, _) = view
                .paced_step(actor, entrant.game_session_id, direction)
                .await;
            assert_eq!(step.disposition, StepDisposition::Moved);
            view.committed_paced_movement(entrant, step.moved_to).await;
        }
        let returned = view.activity_runtimes.current_connection(entrant);
        assert_ne!(returned.game_session_id, entrant.game_session_id);
        assert!(matches!(
            returned
                .item_fence
                .ok_or("returned Channel fence")?
                .runtime_scope,
            RuntimeScopeRefV1::Channel { .. },
        ));
        view.activity_runtimes
            .finish_switch(entrant, returned)
            .map_err(debug)?;
    }
    assert!(
        view.runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    let step_returns: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_activity_runtime_returns WHERE native_step_exit IS NOT NULL",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(step_returns, 2);
    Ok(())
}

fn open_captured_test_entrance(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    use crate::map::facts::QuestCallbackProjection;
    use crate::map::overlay::TilePos;
    use crate::map::view::{MapFacts, placement_key};
    let position = TilePos {
        x: 33460,
        y: 32267,
        floor: -15,
    };
    let landing = TilePos {
        x: 33650,
        y: 32310,
        floor: -15,
    };
    let map = owner.bundle_map().ok_or("published Warzone map")?;
    let tile = map
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .ok_or("authored portal source")?;
    let (key, restore) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(position, ordinal)?;
            let facts = map.facts.base_entry(position, ordinal, *id)?;
            (facts.appearance_id == 388).then_some((key, facts))
        })
        .ok_or("source-bound stalagmite appearance 388")?;
    let portal = map
        .facts
        .movement_item_facts(1949)
        .ok_or("native portal appearance 1949")?;
    assert!(map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        portal,
        &[],
        &[(key, portal, true)],
        false,
        Some(&QuestCallbackProjection::TimedEntrance {
            due: owner.owner_now().get() + 8_000_000,
            restore,
            position,
            destination: landing,
        }),
    ));
    Ok(())
}
