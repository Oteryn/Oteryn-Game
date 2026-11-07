//! Source tutorial NPC effects through NPC_TALK, never seeded completed flags.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use std::sync::Arc;

#[test]
fn questpc3_beginning_source_greet_commits_both_initial_flags() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC3_NPC_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| beginning_greet().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Beginning NPC scenario panicked")??;
    Ok(())
}

fn beginning_greet() -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "questpc3_beginning_npc", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("generation")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content_and_capacity(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            8,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let base = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/";
        // Isolate later dialogue lessons with fixture-only prior milestones. This
        // does not stand in for the cockroach hand-in, damage lesson or food grant.
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use crate::foundation::{CommandId, CommandRef};
        use oteryn_gameplay_quests::quest::{
            QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
        };
        let original = owner.quest_catalogue.clone().ok_or("catalogue")?;
        let tracks = ["santiago_npc_greet_storage", "santiago_quest_log"]
            .map(|name| {
                original
                    .track(&format!("{base}{name}"))
                    .cloned()
                    .ok_or("track")
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let mut transitions = oteryn_gameplay_quests::quest::reward_callbacks::load()
            .map_err(debug)?
            .iter()
            .filter(|c| c.npc == "oteryn:npc.santiago")
            .map(|c| original.transition(&c.key).cloned().ok_or("transition"))
            .collect::<Result<Vec<_>, _>>()?;
        for (stage, log) in [(7, 5), (11, 9)] {
            transitions.push(QuestTransition {
                key: format!("oteryn:quest-transition/fixture/beginning-prior-{stage}"),
                quest: "oteryn:quest.the_beginning_quest".into(),
                effects: [
                    ("santiago_npc_greet_storage", stage),
                    ("santiago_quest_log", log),
                ]
                .into_iter()
                .map(|(name, value)| QuestEffect {
                    track: format!("{base}{name}"),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
                completes: false,
                experience: None,
            });
        }
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?,
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let [santiago] =
            super::classic_quests::install(&owner, &admitted, ["oteryn:npc.santiago"]).await?;
        eprintln!(
            "QUESTPC3_NPC POSTGRES_ASSERTION_REACHED phase=before_source_greet duration_ms={}",
            started.elapsed().as_millis()
        );
        super::classic_quests::talk(&mut owner, &admitted, santiago, 3100, "hi").await?;
        let log = super::classic_quests::progress(
            &owner,
            &admitted,
            &authority,
            "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_quest_log",
        )
        .await?;
        let greet = super::classic_quests::progress(
            &owner,
            &admitted,
            &authority,
            "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_npc_greet_storage",
        )
        .await?;
        eprintln!(
            "QUESTPC3_NPC POSTGRES_ASSERTION_REACHED beginning_greet duration_ms={}",
            started.elapsed().as_millis()
        );
        assert_eq!(
            (greet, log),
            (1, 1),
            "Santiago's source greet writes both flags atomically"
        );
        super::classic_quests::talk(&mut owner, &admitted, santiago, 3101, "hi").await?;
        assert_eq!(
            super::classic_quests::progress(
                &owner,
                &admitted,
                &authority,
                "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_quest_log"
            )
            .await?,
            1
        );
        // A yes opens the source offer; only the following confirmation writes flags.
        super::classic_quests::talk(&mut owner, &admitted, santiago, 3102, "yes").await?;
        assert_eq!(
            super::classic_quests::progress(
                &owner,
                &admitted,
                &authority,
                &format!("{base}santiago_npc_greet_storage")
            )
            .await?,
            1
        );
        super::classic_quests::talk(&mut owner, &admitted, santiago, 3103, "yes").await?;
        for track in ["santiago_npc_greet_storage", "santiago_quest_log"] {
            assert_eq!(
                super::classic_quests::progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{base}{track}")
                )
                .await?,
                2
            );
        }
        super::classic_quests::talk(&mut owner, &admitted, santiago, 3104, "yes").await?;
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/beginning/%'")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 2,
            "greet and confirmed introduction only; no repeated grant"
        );
        for (stage, expected, command) in [(7, (9, 7), 3200), (11, (13, 11), 3300)] {
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let result = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: format!(
                            "oteryn:quest-transition/fixture/beginning-prior-{stage}"
                        ),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(command).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(result, QuestTransitionOutcome::Committed(_)),
                "fixture prerequisite: {result:?}"
            );
            drop(slot);
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            super::classic_quests::talk(&mut owner, &admitted, santiago, command + 1, "bye")
                .await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, command + 2, "hi").await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, command + 3, "yes")
                .await?;
            super::classic_quests::talk(&mut owner, &admitted, santiago, command + 4, "yes")
                .await?;
            assert_eq!(
                (
                    super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{base}santiago_npc_greet_storage")
                    )
                    .await?,
                    super::classic_quests::progress(
                        &owner,
                        &admitted,
                        &authority,
                        &format!("{base}santiago_quest_log")
                    )
                    .await?
                ),
                expected
            );
        }
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/beginning/%'")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 6,
            "six source callbacks through the real NPC owner"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    eprintln!(
        "QUESTPC3_NPC POSTGRES_CASE_COMPLETED beginning_greet duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
