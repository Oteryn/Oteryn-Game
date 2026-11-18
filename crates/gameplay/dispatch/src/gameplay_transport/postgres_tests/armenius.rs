//! Drive the source conversation through the normal NPC dispatcher and a durable quest track.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, WorldId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
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
    super::kill_reward_e2e::scenario_thread::run("armenius_formula", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "armenius_formula", true, false).await?;
            let result = exercise(&h).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
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
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let deps = Dependencies::with_content(
        channel_pin,
        room,
        active
            .native_gameplay()
            .ok_or("native gameplay")?
            .spell_book()
            .clone(),
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
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
    let catalogue = Arc::new(
        QuestStateCatalogue::new("content-1", vec![track], vec![transition]).map_err(debug)?,
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
    Ok(())
}
