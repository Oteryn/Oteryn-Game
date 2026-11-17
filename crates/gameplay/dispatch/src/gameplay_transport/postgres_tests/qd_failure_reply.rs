//! A locked reward refusal may only speak in the conversation that requested it.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{MovementLocalPosition, WorldId};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::time::{Duration, Instant};

const FAILURE: &str = "Indeed there is something to be done, but I need someone more experienced. Come back later if you want to.";

#[test]
fn muriel_failure_reply_is_shown_on_refused_callback() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "qd_failure_reply", false, false).await?;
        prepare_current_session(&h).await?;
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES              (encode($1,'hex')::uuid,1,35,999,'profile-1','ruleset-1','content-1',              'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
        )
        .bind(id(41).as_slice())
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            WorldId::decode(&id(42)).map_err(debug)?,
            "content-1",
        )?;
        let generation = controller.active().ok_or("generation")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        let [npc] = install(&owner, &admitted, ["oteryn:npc.muriel"]).await?;
        let session = admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;

        for (command_id, stale) in [(7000, false), (7010, true)] {
            talk(&mut owner, &admitted, npc, command_id, "hi").await?;
            talk(&mut owner, &admitted, npc, command_id + 1, "mission").await?;
            assert_eq!(
                owner
                    .states
                    .npc_conversations
                    .fixture_quest_topic(session)?,
                1
            );

            // With no backpack, the reward writer cannot deliver Muriel's container.
            // Pause it at receipt lookup so the second reply races a reopened conversation.
            let mut tx = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_character_quest_receipts IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *tx)
                .await?;
            let response = owner.npc_talk(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: command_id + 2,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            );
            tokio::pin!(response);
            let advance = async {
                loop {
                    let blocked: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM pg_locks                          WHERE relation='game_character_quest_receipts'::regclass AND NOT granted)",
                    )
                    .fetch_one(&h.pool)
                    .await?;
                    if blocked {
                        break;
                    }
                }
                if stale {
                    let chat = &owner.states.chat;
                    let engaged = chat.engaged(session).ok_or("Muriel engagement")?;
                    let generation = chat.npc_conversation_generation(session);
                    assert!(chat.end_engagement(session, engaged));
                    chat.converse(session, |focus| *focus = Some(engaged))
                        .ok_or("reopened engagement")?;
                    assert_ne!(chat.npc_conversation_generation(session), generation);
                }
                tx.commit().await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            tokio::select! {
                outcome = &mut response => return Err(format!("NPC replied before the writer reached PostgreSQL: {outcome:?}").into()),
                result = tokio::time::timeout(Duration::from_secs(5), advance) => result??,
            }
            let outcome = response.await;
            assert!(!outcome.unknown);
            assert!(!outcome.inventory_changed);
            if stale {
                assert!(outcome.conversation.is_none(), "{outcome:?}");
            } else {
                assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
                assert_eq!(
                    outcome.conversation.ok_or("Muriel refusal line")?.lines,
                    [FAILURE]
                );
            }
            talk(&mut owner, &admitted, npc, command_id + 3, "bye").await?;
        }
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(receipts, 0);
        Ok(())
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED muriel_failure_reply duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
