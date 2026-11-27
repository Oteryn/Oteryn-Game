use std::time::Duration;

use super::{
    TestResult,
    fixture::{Harness, debug},
    kill_reward_e2e_fixtures, task_board,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::{
    COMMAND_HOLDER_WINDOW, ComposedFreshAdmission, actor_spell::TrainingSave,
    connection::FreshAdmissionAuthority, kill_reward::DrainOutcome,
};
use oteryn_gameplay_progression::hit_training::SkillFamily;
use oteryn_gameplay_quests::quest::task_board::full::{
    BountyBoard, BountyStatus, BountyTask, TaskBoardData,
};
use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, world_spatial_entities::EntityRef};

#[test]
fn p04_training_holder_busy_does_not_block_combat_or_lose_bounty_credit() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    kill_reward_e2e_fixtures::scenario_thread::run(
        "p04-training-holder",
        task_board::training_holder_contention_scenario,
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    (actor, session): (ExactActorRef, GameSessionId),
    (rat, rat_actor): (EntityRef, ExactActorRef),
) -> TestResult {
    let data = TaskBoardData {
        bounty: BountyBoard {
            state: BountyStatus::Active,
            active: Some(BountyTask {
                elite: false,
                dust: 0,
                race: "oteryn:creature.rat".into(),
                required: 30,
                current: 0,
                experience: 1850,
                points: 25,
                grade: 0,
                difficulty: 0,
                slot: 0,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("active fence")?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_task_boards(character_id,world_id,body,committed_character_revision,cause_id,cause_ordinal) SELECT character_id,encode($2,'hex')::uuid,$3,character_revision,encode($4,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(serde_json::to_value(data)?)
        .bind(session.as_bytes().as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    let formula = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("active training formula")?;
    let now = owner.owner_now();
    {
        // Ensure the kill's swing meets the same eager checkpoint condition that
        // used to await the training-session holder inline.
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("live player")?;
        let mut skills = [(10, 0); 7];
        skills[0] = (1_000, 0);
        let build =
            crate::durability::character_build::DurableBuildState::new("monk", (0, 0), skills)
                .map_err(debug)?;
        state
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
        let banked = state
            .train_skill(SkillFamily::Club, now.get(), formula, |_| 10_000)
            .map_err(debug)?;
        assert!(banked.checkpoint_due);
    }
    assert_eq!(
        owner.attack_target(actor, session, 1, Some(rat)).await,
        AttackIntentDisposition::Ok
    );
    owner.refresh_attack_weapon(actor, session).await;
    let mut busy = owner.root.try_acquire_ready()?;
    let attempt = owner.run_training_checkpoint_attempt(actor, session);
    let combat = async {
        for swing in 1..=8 {
            // The optional Talisman read has its own existing holder budget.
            // Allow that wait while detecting a second, inline training wait.
            tokio::time::timeout(
                COMMAND_HOLDER_WINDOW + Duration::from_millis(500),
                owner.drain_auto_attacks_at(now.checked_add(swing * 3_000_000).map_err(debug)?),
            )
            .await?;
            if !owner.runtime.lock().await.contains_live_creature(rat_actor) {
                break;
            }
        }
        assert!(!owner.runtime.lock().await.contains_live_creature(rat_actor));
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Deferred
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    // No SQL can run while the only holder is checked out. Keep the deadline
    // comparison independent of the machine's wall-clock scheduling.
    tokio::time::pause();
    let (saved, combat) = tokio::join!(attempt, combat);
    tokio::time::resume();
    combat?;
    assert_eq!(
        saved,
        TrainingSave::Unknown,
        "training_session must exhaust its holder wait"
    );
    assert_eq!(
        busy.return_to_pool_observed().await,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    );
    for _ in 0..8 {
        if owner.drain_kill_rewards(session).await == DrainOutcome::Drained {
            break;
        }
        owner.root.await_ready().await?;
    }
    let (current, receipts): (i32, i64) = sqlx::query_as(
        "SELECT (body #>> '{bounty,active,current}')::integer, \
         (SELECT count(*) FROM game_task_board_receipts WHERE cause_kind='creature_death' AND refusal = 'null'::jsonb) \
         FROM game_character_task_boards"
    ).fetch_one(&harness.pool).await?;
    // The setup exercised one inactive death. Only the contested physical kill moved this board.
    assert_eq!((current, receipts), (1, 2));
    println!("POSTGRES_CASE_COMPLETED p04_training_holder_busy");
    Ok(())
}
