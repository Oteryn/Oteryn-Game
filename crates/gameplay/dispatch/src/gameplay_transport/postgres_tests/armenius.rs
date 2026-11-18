//! Drive the source conversation through the normal NPC dispatcher and a durable quest track.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, WorldId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::armenius::{Outcome, RESOLVE, RESOLVED_MISSION};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use std::sync::Arc;

#[test]
fn armenius_formula_progresses_only_after_mission_admission_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("armenius_formula", Outcome::Dismissed)
}

#[test]
fn armenius_confrontation_teleports_spawns_and_grants_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(
        "armenius_confrontation",
        oteryn_gameplay_quests::quest::armenius::outcome(1),
    )
}

fn scenario(name: &'static str, expected: Outcome) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
            let result = exercise(&h, expected).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}

async fn exercise(h: &Harness, expected: Outcome) -> TestResult {
    const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission03";
    const READY: &str = "oteryn:quest-transition/blood-brothers/armenius-test-admission";
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (generation, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = generation.active().ok_or("active generation")?;
    let room = pin.qualified_room().clone();
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let world_map = super::quest_callbacks::source_world_spatial(
        (32854, 31324, -8),
        &[(32858, 31325, -8, 0, 1900)],
    )?;
    let cells = world_map
        .movement_cells(room.movement_cells())
        .map_err(debug)?;
    let position = (32854, 31324, 8);
    let channel_pin = ChannelContentPin::from_activation(
        world,
        1,
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        world_map.map_revision_digest(),
        position,
    );
    let deps =
        Dependencies::with_content_and_capacity(channel_pin, room, native.spell_book().clone(), 8)?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: position.0,
                    y: position.1,
                    floor: position.2,
                },
            )
            .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
    }
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    let canonical = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let track = canonical
        .catalogue()
        .track(MISSION)
        .ok_or("Mission03 track")?
        .clone();
    let transition = QuestTransition {
        world_change: None,
        key: READY.into(),
        quest: track.quest.clone(),
        effects: vec![QuestEffect {
            track: MISSION.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(1),
        }],
        completes: false,
        experience: None,
    };
    let resolve = canonical
        .catalogue()
        .transition(RESOLVE)
        .ok_or("resolution")?
        .clone();
    let catalogue = Arc::new(
        QuestStateCatalogue::new("content-1", vec![track], vec![transition, resolve])
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    admitted.item_fence = Some(fence.item_fence());
    owner.admit_quest_session(&admitted).await;
    let [npc] = install(&owner, &admitted, ["oteryn:npc.armenius"]).await?;
    talk(&mut owner, &admitted, npc, 1000, "hi").await?;
    talk(&mut owner, &admitted, npc, 1002, "alori mort").await?;
    assert_eq!(
        owner
            .states
            .npc_conversations
            .fixture_quest_topic(session)?,
        0
    );
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: READY.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(2000).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    owner.refresh_quest_session(session, false).await;
    for (index, topic) in (2..=5).enumerate() {
        talk(
            &mut owner,
            &admitted,
            npc,
            2010 + 2 * u64::try_from(index)?,
            "AlOrI MoRt",
        )
        .await?;
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(session)?,
            topic
        );
        if topic == 2 {
            talk(&mut owner, &admitted, npc, 2011, "alori").await?;
            assert_eq!(
                owner
                    .states
                    .npc_conversations
                    .fixture_quest_topic(session)?,
                2,
                "an incomplete formula cannot advance Mission03 dialogue"
            );
        }
    }
    let command = {
        let runtime = owner.runtime.lock().await;
        (2030..2100)
            .find(|id| {
                crate::gameplay_transport::npc_command::armenius::final_outcome(
                    &runtime, session, *id,
                ) == Some(expected)
            })
            .ok_or("final decision")?
    };
    talk(&mut owner, &admitted, npc, command, "alori mort").await?;
    let progress = owner
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(progress.tracks().get(MISSION), Some(&RESOLVED_MISSION));
    if let Outcome::Confrontation { destination, spawn } = expected {
        let mut runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            (at.x, at.y, at.floor),
            (destination.x, destination.y, -destination.floor)
        );
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .any(|creature| {
                    runtime
                        .borrow_combat_death()
                        .creature_target_identity(creature)
                        .is_ok_and(|key| {
                            key == oteryn_gameplay_quests::quest::armenius::CREATURE.as_bytes()
                        })
                        && runtime.read_actor_position(creature).is_ok_and(|position| {
                            let position = position.position();
                            (position.x, position.y, position.floor)
                                == (spawn.x, spawn.y, -spawn.floor)
                        })
                })
        );
        drop(runtime);
        let achievements: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key = $1",
        )
        .bind("oteryn:achievement/his_true_face")
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(achievements, 1);
    }
    Ok(())
}
