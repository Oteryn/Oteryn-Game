//! Personal native deaths are admitted against the current durable world occurrence.
use super::*;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};

#[test]
fn qworld_native_fury_death_requires_active_world_and_credits_once() -> TestResult {
    run("fury")
}

#[test]
fn qworld_native_werewolf_death_requires_full_moon_and_credits_once() -> TestResult {
    run("werewolf")
}

pub(super) fn run(creature: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenarios(creature).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "native death scenario panicked")??;
    eprintln!(
        "QWORLD_NATIVE_DEATH_PG_EXECUTED creature={creature} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenarios(creature: &'static str) -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    for active in [false, true] {
        let epoch = if creature == "werewolf" {
            1
        } else {
            (1..=32)
                .find(|epoch| {
                    oteryn_gameplay_world_change::daily::roll_calendar(
                        world.as_bytes(),
                        *epoch,
                        1,
                        1,
                    )
                    .fury_gates
                    .is_some()
                        == active
                })
                .ok_or("deterministic Fury occurrence")?
        };
        super::world_event::scenario_with_native_deaths(epoch, creature, active)?;
    }
    Ok(())
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    creature: &str,
    active: bool,
) -> TestResult {
    eprintln!("POSTGRES_DATABASE_READY qworld_native_death creature={creature} active={active}");
    let world_change = if creature == "fury" {
        "fury_gates"
    } else {
        "grimvale"
    };
    assert_eq!(
        h.root
            .world_change_active(admitted.world_id, world_change)
            .await?,
        active
    );
    owner.admit_progression_session(admitted).await;
    let counter = oteryn_gameplay_quests::quest::native_kills::counter(&format!(
        "oteryn:quest-transition/native-kills/qworld-{creature}"
    ))
    .ok_or("Fury counter")?;
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let catalogue = loaded.catalogue();
    let accept = "oteryn:quest-transition/qworld/fixture-accept-hunt";
    let restart = "oteryn:quest-transition/qworld/fixture-restart-hunt";
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![
                catalogue.track(counter.gate).ok_or("hunt gate")?.clone(),
                catalogue.track(counter.track).ok_or("kill track")?.clone(),
            ],
            vec![
                catalogue
                    .transition(counter.transition)
                    .ok_or("native death transition")?
                    .clone(),
                QuestTransition {
                    key: accept.into(),
                    quest: counter.quest.into(),
                    effects: vec![QuestEffect {
                        track: counter.gate.into(),
                        from: QuestComparison::Eq(0),
                        effect: QuestEffectKind::Set(1),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
                QuestTransition {
                    key: restart.into(),
                    quest: counter.quest.into(),
                    effects: vec![QuestEffect {
                        track: counter.gate.into(),
                        from: QuestComparison::Eq(1),
                        effect: QuestEffectKind::Set(1),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            ],
        )
        .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    // Before the personal hunt starts, even an active world cannot award credit.
    super::qengine_kill_stories::death(owner, admitted, counter.creature, 20000).await?;
    assert_eq!(count(h, counter.track).await?, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: accept.into(),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(20001).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    // The shared helper records and drains the same death twice, through real capture.
    super::qengine_kill_stories::death(owner, admitted, counter.creature, 20002).await?;
    assert_eq!(count(h, counter.track).await?, i64::from(active));
    super::qengine_kill_stories::queued_deaths(
        owner,
        admitted,
        counter.creature,
        &[20003, 20004, 20005],
    )
    .await?;
    assert_eq!(
        count(h, counter.track).await?,
        i64::from(active) * 4,
        "every distinct native death captured in the same activation counts exactly once"
    );
    super::qengine_kill_stories::capture_deaths(
        owner,
        admitted,
        counter.creature,
        &[20006, 20007, 20008],
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("restart fence")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: restart.into(),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(20009).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    for _ in 0..2 {
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        );
    }
    assert_eq!(
        count(h, counter.track).await?,
        i64::from(active) * 4,
        "same-value reactivation invalidates every previously captured death"
    );
    Ok(())
}

async fn count(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}
