//! NEW HAVEN qualification uses the real compiler artifact, production admission, paced
//! movement, vocation selection, XP owner, sealed-door USE and Anna's NPC travel owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, LogoutReply, UseCommand};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::FreshAdmissionAuthority};
use oteryn_protocol_oteryn::newhaven_vocation::NewhavenChoiceResult;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::time::Duration;

#[path = "newhaven/world.rs"]
mod world;

#[test]
fn newhaven_real_world_portal_kit_ferry_and_restart_postgres() -> TestResult {
    // A route-less run is not PostgreSQL qualification, and must not read the artifact.
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    if std::env::var_os("OTERYN_NEWHAVEN_TEST_BUNDLE").is_none() {
        eprintln!("NEW HAVEN NOT_EXECUTED: compiler artifact is not configured");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("newhaven-real-world", scenario)
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    world: &crate::map::boot::BundleWorld,
    actor: ExactActorRef,
    session: GameSessionId,
    target: (i32, i32),
) -> TestResult {
    for direction in world::path(world, position(owner, actor).await?, target)? {
        step(owner, actor, session, direction).await?;
    }
    let at = position(owner, actor).await?;
    assert_eq!((at.x, at.y, at.floor), (target.0, target.1, 7));
    Ok(())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("movement clock")?;
    let (outcome, _) = owner.paced_step(actor, session, direction).await;
    assert_eq!(outcome.disposition, StepDisposition::Moved, "{outcome:?}");
    Ok(())
}

/// Full item identity/quantity ledger catches reminting even when stack totals match.
async fn items(h: &Harness) -> TestResult<Vec<(String, String, i64)>> {
    Ok(sqlx::query_as(
        "SELECT item_instance_id::text,definition_production_key,quantity::bigint \
        FROM game_item_instances WHERE lifecycle=1 ORDER BY item_instance_id",
    )
    .fetch_all(&h.pool)
    .await?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "newhaven_journey", false, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let world = world::load(native)?;
        assert!(
            world.enterable(crate::map::overlay::TilePos {
                x: 32527,
                y: 32430,
                floor: -7
            }),
            "Chief selected source-backed tile must be walkable in the real World"
        );
        assert!(
            !world.enterable(crate::map::overlay::TilePos {
                x: 32526,
                y: 32430,
                floor: -7
            }),
            "the blocked design anchor must retain its source collision"
        );
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(world_id),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 32534,
                    y: 32513,
                    floor: 7,
                },
            )
            .ok_or("real bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (32534, 32513, 7),
        );
        let mut deps = Dependencies::with_content_and_capacity(
            channel_pin.clone(),
            room.clone(),
            native.spell_book().clone(),
            8,
        )?;
        deps.combat_room = Some(room.clone());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let (entry, item_fence, decided) = Box::pin(owner.initialize_first_entry(
            &request,
            session,
            deps.admitted.controller.ok_or("controller")?.transport,
            actor,
        ))
        .await;
        assert_eq!(entry, FirstEntryOutcome::Positioned);
        let budget = crate::gameplay_transport::AdmissionRetryBudget::for_root(&session, &h.root);
        assert!(
            owner
                .consume_admitted_newhaven_arrival(&budget, session, actor, decided.as_ref())
                .await
        );
        let mut admitted = deps.admitted;
        admitted.first_entry = entry;
        admitted.item_fence = item_fence;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_loop_training(&mut admitted).await;
        let at = position(&owner, actor).await?;
        assert_eq!((at.x, at.y, at.floor), (32534, 32513, 7));
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("vocationless state")?
                    .character_facts()
                    .vocation,
                crate::spell::Vocation::None
            );
        }
        walk(&mut owner, &world, actor, session, (32537, 32514)).await?;
        step(&mut owner, actor, session, StepDirection::East).await?;
        let at = position(&owner, actor).await?;
        assert_eq!((at.x, at.y, at.floor), (32544, 32515, 7));
        assert_eq!(
            owner.states.tutorial_sessions.take(session),
            Some(oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::Hint(2))
        );
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: admitted.item_fence,
        };
        let chosen = owner
            .choose_newhaven_vocation(actor, command(1), "knight")
            .await;
        assert_eq!(chosen, Some(NewhavenChoiceResult::Chosen));
        let kit = items(&h).await?;
        for source_id in [
            3355, 3561, 3559, 3552, 34017, 2854, 3272, 3412, 268, 266, 3031, 3578,
        ] {
            assert!(
                kit.iter()
                    .any(|(_, key, _)| key == &format!("oteryn:item.tibia.i{source_id}")),
                "missing starter Item {source_id}"
            );
        }
        assert_eq!(
            owner
                .choose_newhaven_vocation(actor, command(1), "knight")
                .await,
            chosen
        );
        assert_eq!(
            items(&h).await?,
            kit,
            "choice replay preserves Item identities"
        );
        // Production owner XP awards provide level progression; no SQL edits to level/build.
        award_level(&owner, &h, &authority, actor, session, 7, 90).await?;
        walk(&mut owner, &world, actor, session, (32557, 32497)).await?;
        let target = ferry_target(&world, native)?;
        let refused = owner
            .use_bundle_door(
                actor,
                session,
                owner.bundle_map().ok_or("bundle map")?,
                &target,
            )
            .await;
        assert_eq!(
            refused.disposition,
            oteryn_protocol_oteryn::world_object::UseDisposition::Locked
        );
        assert_eq!(
            owner.states.tutorial_sessions.take(session),
            Some(
                oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                    "You need to be at least level 8.".into()
                )
            )
        );
        assert_eq!(
            position(&owner, actor).await?,
            MovementLocalPosition {
                x: 32557,
                y: 32497,
                floor: 7
            }
        );
        award_level(&owner, &h, &authority, actor, session, 8, 91).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("door clock")?;
        let passed = owner
            .use_bundle_door(
                actor,
                session,
                owner.bundle_map().ok_or("bundle map")?,
                &target,
            )
            .await;
        assert_eq!(
            passed.disposition,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        let at = position(&owner, actor).await?;
        assert_eq!((at.x, at.y, at.floor), (32555, 32497, 7));
        anna(&mut owner, &deps, &world, session).await?;
        let at = position(&owner, actor).await?;
        assert_eq!((at.x, at.y, at.floor), (32472, 32288, 7));
        let after = items(&h).await?;
        assert_eq!(after, kit, "travel does not mint another kit");
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(120))
            .ok_or("logout clock")?;
        assert_eq!(
            owner.logout_session(admitted).await,
            LogoutReply::Result(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        );
        let next_session = super::full_world_e2e::relogin(&h).await?;
        // Discard the entire connection owner and open a fresh durable root. A fresh Channel
        // owner must restore state from PostgreSQL rather than retaining tutorial queues.
        drop(owner);
        drop(authority);
        let reopened = super::fixture::tutorial_vitals_fixture::reopen_root(&h)?;
        assert!(reopened.maintain_ready_once().await?);
        let reopened_authority = reopened
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut reboot_deps = Dependencies::with_content_and_capacity(
            channel_pin.clone(),
            room.clone(),
            native.spell_book().clone(),
            8,
        )?;
        reboot_deps.combat_room = Some(room);
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(next_session).map_err(debug)?;
        let next_actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        reboot_deps.actor = next_actor;
        reboot_deps.admitted.game_session_id = next_session;
        reboot_deps.admitted.runtime_actor = Some(next_actor);
        reboot_deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut restarted = reboot_deps.owner(&h, &reopened_authority);
        restarted.root = &reopened;
        restarted.active_generation = Some(active);
        restarted.movement_cells = &cells;
        let fence = restarted
            .current_quest_fence(next_session)
            .await
            .map_err(|()| "restart fence")?
            .ok_or("restart session")?;
        let decided = restarted
            .read_admission_position(fence)
            .await
            .map_err(|()| "restart position")?;
        {
            let mut runtime = reboot_deps.runtime.lock().await;
            restarted
                .initialize_admitted_position(&mut runtime, next_actor, next_session, &decided)
                .await
                .map_err(debug)?;
        }
        let at = position(&restarted, next_actor).await?;
        assert_eq!((at.x, at.y, at.floor), (32472, 32288, 7));
        let progress = reopened
            .character_tutorial_progress(&reopened_authority, &h.node, fence, None)
            .await
            .map_err(debug)?;
        let crate::durability::character_tutorial::TutorialProgressOutcome::Read(Some(stored)) =
            progress
        else {
            return Err("reloaded tutorial progress".into());
        };
        assert_eq!(
            stored.progress.step,
            oteryn_gameplay_quests::tutorial::TutorialStep::Departed
        );
        assert!(!stored.progress.hunting && !stored.progress.corruptor);
        assert_eq!(
            items(&h).await?,
            after,
            "reopened root and successor admission preserve Item identities"
        );
        eprintln!(
            "NEW HAVEN POSTGRES_EXECUTED real World portal, knight kit replay, level7 refusal, level8 ferry, Anna Thais Peninsula, successor admission and reopened durable root"
        );
        drop(restarted);
        drop(reopened_authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn award_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    level: u32,
    occurrence: u8,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceRewardOccurrence,
    };
    let progression = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    let total = progression
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == level)
        .ok_or("level threshold")?
        .minimum_experience
        .get();
    let before: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "XP fence")?
        .ok_or("XP session")?;
    let award = ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(occurrence)).map_err(debug)?,
        amount: oteryn_simulation_determinism::ExactI64::new(total - before),
        context: progression.context.clone(),
        policy_revision: progression.policy_revision.clone(),
        reward_revision: progression.reward_revision.clone(),
        policy: progression.policy.clone(),
    };
    let outcome = h
        .root
        .commit_character_experience(authority, &h.node, fence, award.clone())
        .await
        .map_err(debug)?;
    owner
        .states
        .level_refresh
        .mark(crate::gameplay_transport::level_up_live::leveled_character(
            &outcome,
        ));
    Box::pin(owner.refresh_level_up_vitals(actor, session)).await;
    let replay = h
        .root
        .commit_character_experience(authority, &h.node, fence, award)
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            replay,
            crate::durability::character_progression::ExperienceCommitOutcome::AlreadyCommitted(_)
        ),
        "XP occurrence replay: {replay:?}"
    );
    let current: (i64, i64) =
        sqlx::query_as("SELECT total_experience,level FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(current, (total, i64::from(level)));
    Ok(())
}

fn ferry_target(
    world: &crate::map::boot::BundleWorld,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<oteryn_protocol_oteryn::world_object::WorldObjectTarget> {
    let tile = crate::map::overlay::TilePos {
        x: 32556,
        y: 32497,
        floor: -7,
    };
    let view = crate::map::view::compose_tile(world.overlay(), world.facts(), tile)
        .map_err(debug)?
        .ok_or("ferry tile")?;
    let index = native.item_index().ok_or("Item index")?;
    let key = view
        .entries
        .iter()
        .find_map(|entry| {
            let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                entry.facts.definition
            else {
                return None;
            };
            let definition =
                crate::gameplay_transport::item_look::typed_definition(index, reference)?;
            if !matches!(
                definition.production_key.as_str(),
                "oteryn:item.tibia.i51777" | "oteryn:item.tibia.i51778"
            ) {
                return None;
            }
            match entry.source.key() {
                crate::map::view::MapEntryKey::Base(key) => Some(key),
                _ => None,
            }
        })
        .ok_or("real ferry51777/51778 placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(oteryn_protocol_oteryn::world_object::WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}

async fn anna(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    world: &crate::map::boot::BundleWorld,
    session: GameSessionId,
) -> TestResult {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content")
            .as_path(),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(NpcTalkBook::from_model(&model)));
    owner
        .states
        .chat
        .admit(session, "Newhaven Traveller", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let table = world::anna_placement()?;
    let npc = {
        let mut runtime = owner.runtime.lock().await;
        runtime.place_npcs(&table, |_| true, true).map_err(debug)?;
        let placed = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.anna"))
            .ok_or("Anna from real bundle")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: placed.actor.placement_identity(),
            generation: placed.generation,
        }
    };
    walk(owner, world, deps.actor, session, (32553, 32497)).await?;
    for (index, text) in ["hi", "passage", "yes"].into_iter().enumerate() {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(1))
            .ok_or("NPC clock")?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "Anna fence")?
            .ok_or("Anna session")?;
        let result = owner
            .npc_talk(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 20 + u64::try_from(index)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(
            result.disposition,
            NpcIntentDisposition::Ok,
            "Anna {text}: {result:?}"
        );
        assert!(!result.unknown, "Anna {text}");
    }
    Ok(())
}
