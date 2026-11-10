//! M01c native Task Board acceptance is the only source of programme entry.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
#[allow(dead_code, reason = "reuse the canonical fenced PostgreSQL harness")]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
use bestiary_postgres_harness as f;
use durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use durability::task_board::{TaskBoardCatalogue, TaskBoardCommand};
use foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::task_board::full::{TaskBoardAction, TaskBoardCreature};
use std::sync::Arc;

#[test]
fn m01c_task_board_entries_require_committed_acceptance_postgres() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "questpc3_m01c_board", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M01C_BOARD POSTGRES_CASE_COMPLETED duration_ms={} success=true",
                        started.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M01c Task Board PostgreSQL thread")?
        .map_err(Into::into)
}

fn cause(id: u64) -> f::TestResult<QuestCause> {
    Ok(QuestCause::Command(CommandRef::new(
        f::fence(1)?.game_session_id,
        CommandId::new(id).map_err(f::debug)?,
    )))
}
fn request(slug: &str, id: u64) -> f::TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: format!("oteryn:quest-task-board-use/m01c-{slug}"),
        cause: cause(id)?,
    })
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M01C_BOARD POSTGRES_CONNECTED version={version}");
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(f::debug)?
            .catalogue()
            .clone(),
    );
    assert!(
        catalogue
            .transition("oteryn:quest-task-board-use/m01c-feral-trappers")
            .is_some(),
        "production programme entry must be registered"
    );
    // Only the committed calendar baseline is a fixture. Every board acceptance,
    // refusal and quest checkpoint below goes through its production writer.
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2026-11-08 12:00:00+00','2026-11')")
        .bind(f::id(f::WORLD).as_slice()).bind([1_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,0)")
        .bind(f::id(f::WORLD).as_slice()).execute(&h.pool).await?;
    // A bounded rules catalogue supplies enough distinct eligible species. This
    // test qualifies receipt admission, not production board balance or RNG.
    let board = Arc::new(TaskBoardCatalogue {
        creatures: (0..48)
            .map(|i| TaskBoardCreature {
                race: format!("oteryn:creature.m01c_board_{i}"),
                bestiary_stars: (i % 6) as u8,
                experience: 100,
                preyable: true,
                prey_exclusive: false,
            })
            .collect(),
        gameplay_content_revision: "content-1".into(),
        progression: None,
        content_revision: String::new(),
        items: Default::default(),
        cosmetics: None,
    });
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let sequencer = durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(f::fence(1)?.character_id).await;
    let unproven = slot
        .commit_quest_transition(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            request("feral-trappers", 44000)?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(unproven, QuestTransitionOutcome::Refused(_)));
    for (id, action) in [
        (44001, TaskBoardAction::Query),
        (44002, TaskBoardAction::StartWeekly { difficulty: 4 }),
    ] {
        let outcome = slot
            .commit_task_board(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                TaskBoardCommand {
                    cause: cause(id)?,
                    action,
                    payload_binding: vec![id as u8],
                    fee_change: None,
                },
                Arc::clone(&board),
                None,
            )
            .await
            .map_err(f::debug)?;
        assert!(outcome.transition.is_none());
        let observed = slot
            .commit_quest_transition(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                request("feral-trappers", id)?,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(observed, QuestTransitionOutcome::Refused(_)),
            "query/refusal must not prove acceptance"
        );
    }
    let accepted = slot
        .commit_task_board(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            TaskBoardCommand {
                cause: cause(44003)?,
                action: TaskBoardAction::StartWeekly { difficulty: 0 },
                payload_binding: vec![5, 0],
                fee_change: None,
            },
            board,
            None,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        accepted.refusal.is_none(),
        "native weekly outcome: {accepted:?}"
    );
    assert_eq!(
        accepted
            .transition
            .ok_or("weekly acceptance")?
            .committed_character_revision
            .get(),
        2
    );
    let unrelated = slot
        .commit_task_board(
            &h.root,
            &authority,
            &h.node,
            f::fence(2)?,
            TaskBoardCommand {
                cause: cause(44004)?,
                action: TaskBoardAction::SetElite { enabled: true },
                payload_binding: vec![19, 1],
                fee_change: None,
            },
            Arc::new(TaskBoardCatalogue {
                creatures: Vec::new(),
                gameplay_content_revision: "content-1".into(),
                progression: None,
                content_revision: String::new(),
                items: Default::default(),
                cosmetics: None,
            }),
            None,
        )
        .await
        .map_err(f::debug)?;
    assert!(unrelated.refusal.is_none());
    assert!(unrelated.view.data.bounty.elite);
    assert!(
        unrelated.transition.is_some(),
        "a different mutating board action really committed"
    );
    let wrong_action = slot
        .commit_quest_transition(
            &h.root,
            &authority,
            &h.node,
            f::fence(3)?,
            request("feral-trappers", 44004)?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(wrong_action, QuestTransitionOutcome::Refused(_)),
        "committed elite setting is not programme acceptance"
    );
    let below_level = slot
        .commit_quest_transition(
            &h.root,
            &authority,
            &h.node,
            f::fence(3)?,
            request("falconer", 44003)?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(below_level, QuestTransitionOutcome::Refused(_)),
        "Falconer still requires level100"
    );
    let entered = slot
        .commit_quest_transition(
            &h.root,
            &authority,
            &h.node,
            f::fence(3)?,
            request("feral-trappers", 44003)?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(entered, QuestTransitionOutcome::Committed(_)));
    let replay = slot
        .commit_quest_transition(
            &h.root,
            &authority,
            &h.node,
            f::fence(4)?,
            request("feral-trappers", 44003)?,
            catalogue,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let value: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-feral-trappers-route'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1);
    let completed: Option<String> = sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.feral_trappers_quest'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed.is_none());
    eprintln!(
        "M01C_BOARD native_weekly_accepted query_refused failed_start_refused unrelated_committed_action_refused forged_command_refused level100_guard replay_safe completion_held"
    );
    Ok(())
}
