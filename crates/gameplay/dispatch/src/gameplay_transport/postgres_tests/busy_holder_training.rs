use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::actor_spell::{TRAINING_READBACK_BARRIER, TrainingSave};
use oteryn_gameplay_progression::hit_training::SkillFamily;
use std::sync::Arc;
use tokio::sync::Notify;

#[test]
fn busy_holder_release_waits_for_forced_checkpoint_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("busy-holder-training", move || {
        crate::bestiary_postgres_harness::runtime()?.block_on(async {
            let h = Harness::create_with_admission_seed(admin, "busy_holder_training", true, false)
                .await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let (generation, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let active = generation.active().ok_or("active generation")?;
    let formula = active
        .native_gameplay()
        .and_then(|native| native.training_formula())
        .ok_or("formula")?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let character = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "current fence")?
        .ok_or("character")?
        .character_id;
    let build = h
        .root
        .read_character_build_state(&authority, character)
        .await
        .map_err(debug)?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::from_key(build.vocation())
                            .ok_or("vocation")?,
                        magic_level: u32::from(build.magic().0),
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        let state = states.get_mut(&runtime, actor, session).ok_or("actor")?;
        state
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
        state
            .train_skill(SkillFamily::Sword, owner.owner_now().get(), formula, |_| 1)
            .map_err(debug)?;
    }

    let count_receipts = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_build_receipts")
            .fetch_one(&h.pool)
            .await
    };
    let before = count_receipts().await?;
    let held = h.root.try_issue_semantic_pass()?;
    assert_eq!(
        owner.save_spell_training(actor, session, false).await,
        TrainingSave::NotApplicable
    );
    assert_eq!(count_receipts().await?, before);
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;

    let barrier = Arc::new((Notify::new(), Notify::new()));
    let budget =
        crate::gameplay_transport::AdmissionRetryBudget::for_foreground_root(&session, &h.root)?;
    let mut forced = Box::pin(TRAINING_READBACK_BARRIER.scope(
        barrier.clone(),
        owner.save_spell_training_with_budget(actor, session, true, &budget),
    ));
    tokio::select! {
        result = forced.as_mut() => return Err(format!("forced checkpoint ended before reconciliation: {result:?}").into()),
        () = barrier.0.notified() => {}
    }
    let held = h.root.try_issue_semantic_pass()?;
    barrier.1.notify_one();
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(std::future::Future::poll(
            forced.as_mut(),
            cx
        )))
        .await
        .is_pending()
    );
    assert!(
        owner.revision_sequencer.try_acquire(character).is_some(),
        "readback holds no revision slot"
    );
    let active: i16 = sqlx::query_scalar("SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        active, 2,
        "the lease still belongs to the active controller"
    );
    assert_eq!(count_receipts().await?, before + 1);
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    assert_eq!(forced.await, TrainingSave::Saved);
    assert_eq!(
        count_receipts().await?,
        before + 1,
        "reconciliation does not write again"
    );
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert!(
            states
                .get(&runtime, actor, session)
                .ok_or("actor")?
                .pending_training_checkpoint()
                .is_none()
        );
    }
    let mut admitted = deps.admitted;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session(session)
            .await?,
    );
    let controller = admitted.controller.ok_or("controller")?;
    let released = tokio::time::timeout(
        crate::durability::ROOT_RECOVERY_WINDOW,
        owner.release_terminal(
            admitted,
            crate::gameplay_transport::TerminalRelease::StaffKick(controller.transport),
        ),
    )
    .await?;
    assert_eq!(
        released,
        crate::gameplay_transport::connection::GraceExpiryResult::Released
    );
    assert_eq!(count_receipts().await?, before + 1);
    Ok(())
}
