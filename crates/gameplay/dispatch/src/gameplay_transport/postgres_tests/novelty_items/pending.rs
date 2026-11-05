//! Real ordinary USE cancellation before and after its durable commit.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
    source: [u8; 16],
    intent: UseIntent,
) -> TestResult {
    let session = fence.game_session_id;
    let command = || UseCommand {
        game_session_id: session,
        command_id: 1,
        item_fence: Some(fence),
    };
    let before = {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(&runtime, actor, session)
            .ok_or("pending player")?
            .vitals()
    };

    // The actual writer parks the original effect before it waits on this item.
    let mut blocker = harness.pool.begin().await?;
    sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(source.as_slice()).execute(&mut *blocker).await?;
    let mut use_future = Box::pin(owner.use_event_item(
        actor,
        command(),
        intent.clone(),
        Some(ItemKey::Instance(source)),
        true,
    ));
    while_pending(&mut use_future, wait_for_memo(owner, actor, session)).await?;
    drop(use_future);
    assert!(owner.item_use_pending(actor, session).await);
    blocker.rollback().await?;

    // Wait for that cancelled transaction to release the same command lock,
    // then retain it ourselves to make the receipt probe's outcome unknown.
    let mut probe_blocker = harness.pool.begin().await?;
    tokio::time::timeout(Duration::from_secs(10),
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':1',0))")
            .bind(session.as_bytes().as_slice()).execute(&mut *probe_blocker)
    ).await??;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *probe_blocker)
        .await?;
    // Releasing the cancelled writer's SQL lock does not prove its holder has
    // returned to the ready pool. Retry a refused pass until this exact probe
    // is observed waiting on our advisory lock, then test that same future.
    let mut reconciliation = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let mut attempt = Box::pin(owner.reconcile_spells_before_control_loss(actor, session));
            let waiting = async {
                loop {
                    let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks l WHERE l.locktype='advisory' AND NOT l.granted AND $1=ANY(pg_blocking_pids(l.pid)) AND l.database=(SELECT oid FROM pg_database WHERE datname=current_database()))")
                        .bind(blocker_pid).fetch_one(&harness.pool).await?;
                    if blocked {
                        return Ok::<(), sqlx::Error>(());
                    }
                    tokio::task::yield_now().await;
                }
            };
            let mut waiting = Box::pin(waiting);
            let observed = std::future::poll_fn(|cx| {
                if let std::task::Poll::Ready(settled) = attempt.as_mut().poll(cx) {
                    assert!(!settled, "unknown receipt must prevent control loss");
                    return std::task::Poll::Ready(Ok(false));
                }
                waiting.as_mut().poll(cx).map(|result| result.map(|()| true))
            }).await?;
            if observed {
                return Ok::<_, Box<dyn std::error::Error>>(attempt);
            }
            assert!(owner.item_use_pending(actor, session).await);
            if owner.root.has_ready_demand() {
                owner.root.maintain_ready_once().await.map_err(debug)?;
            }
            tokio::task::yield_now().await;
        }
    }).await??;
    assert!(
        tokio::time::timeout(
            Duration::from_millis(50),
            &mut reconciliation,
        )
        .await
        .is_err()
    );
    drop(reconciliation);
    assert!(
        owner.item_use_pending(actor, session).await,
        "unknown receipt must retain the prepared effect"
    );
    probe_blocker.rollback().await?;

    // Cancellation can retire the sole root holder while returning it to the
    // pool. Exercise that outcome deterministically, rather than relying on load.
    owner.root.request_ready();
    assert!(owner.root.maintain_ready_once().await.map_err(debug)?);
    let holder = owner.root.try_acquire_ready().map_err(debug)?;
    holder.close().await?;
    assert!(!owner.root.is_ready());

    reconcile_actor_end(owner, actor, session, "rollback").await?;
    assert!(
        !owner.item_use_pending(actor, session).await,
        "known rollback releases the original memo"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_consume_transform_records")
            .fetch_one(&harness.pool)
            .await?,
        0
    );
    assert_eq!(source_state(harness, source).await?, (1, 1));
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("rollback player")?
                .vitals(),
            before
        );
    }

    // Retry the same uncommitted command. Hold runtime after preparation so the
    // durable receipt commits while its ordinary finish is still unable to run.
    let mut blocker = harness.pool.begin().await?;
    sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(source.as_slice()).execute(&mut *blocker).await?;
    let mut use_future = Box::pin(owner.use_event_item(
        actor,
        command(),
        intent,
        Some(ItemKey::Instance(source)),
        true,
    ));
    while_pending(&mut use_future, wait_for_memo(owner, actor, session)).await?;
    let runtime_hold = owner.runtime.lock().await;
    blocker.rollback().await?;
    while_pending(&mut use_future, async {
        loop {
            if sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_consume_transform_records",
            )
            .fetch_one(&harness.pool)
            .await?
                == 1
            {
                return Ok::<(), sqlx::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await??;
    drop(use_future);
    drop(runtime_hold);
    assert!(owner.item_use_pending(actor, session).await);
    assert_eq!(source_state(harness, source).await?, (0, 2));

    // A committed receipt is history. The actual runtime lease fence prevents
    // installing its retained successor, while the exact pending memo survives.
    let token = {
        let mut runtime = owner.runtime.lock().await;
        let token =
            ChannelRuntimeV1::transition_fence(runtime.mint_transition_fence().map_err(debug)?);
        runtime
            .fence_player_writes(actor, session, token)
            .map_err(debug)?;
        token
    };
    assert!(!owner.reconcile_pending_item_effects(actor, session).await);
    assert!(owner.item_use_pending(actor, session).await);
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("fenced player")?
                .vitals(),
            before
        );
        assert!(
            runtime
                .lift_player_fence(actor, session, token)
                .map_err(debug)?
        );
    }

    reconcile_actor_end(owner, actor, session, "committed").await?;
    assert!(!owner.item_use_pending(actor, session).await);
    let after = {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(&runtime, actor, session)
            .ok_or("committed player")?
            .vitals()
    };
    assert_eq!(after.health, before.health - 10);
    reconcile_actor_end(owner, actor, session, "settled").await?;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("settled player")?
                .vitals(),
            after
        );
        assert!(states.pending_item_effects.is_empty());
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_consume_transform_records")
            .fetch_one(&harness.pool)
            .await?,
        1
    );
    Ok(())
}

async fn wait_for_memo(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) {
    loop {
        if owner.item_use_pending(actor, session).await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn while_pending<F: Future, U: Future>(
    running: &mut Pin<Box<F>>,
    until: U,
) -> Result<U::Output, tokio::time::error::Elapsed> {
    let mut until = Box::pin(until);
    tokio::time::timeout(
        Duration::from_secs(10),
        std::future::poll_fn(|cx| {
            assert!(
                running.as_mut().poll(cx).is_pending(),
                "blocked ordinary USE must remain pending"
            );
            until.as_mut().poll(cx)
        }),
    )
    .await
}

async fn source_state(harness: &Harness, source: [u8; 16]) -> Result<(i64, i16), sqlx::Error> {
    sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&harness.pool).await
}

// The direct fixture has no node maintenance task. Like the real release
// owner's node, service holder demand after a refused pass: cancellation can
// retire the sole connection, so polling alone cannot make the root ready.
// A persistent memo or unrelated save refusal remains a failure.
async fn reconcile_actor_end(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    phase: &str,
) -> TestResult {
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if owner
                .reconcile_spells_before_control_loss(actor, session)
                .await
            {
                return Ok::<(), Box<dyn std::error::Error>>(());
            }
            if owner.root.has_ready_demand() {
                owner.root.maintain_ready_once().await.map_err(debug)?;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    if result.is_err() {
        let item_pending = owner.item_use_pending(actor, session).await;
        let spell_pending = owner
            .states
            .spell_states
            .lock()
            .await
            .has_pending_spell_commit(actor, session);
        let training = owner.save_spell_training(actor, session, true).await;
        return Err(format!(
            "{phase} control-loss reconciliation timed out: item_pending={item_pending} spell_pending={spell_pending} training={training:?} root_ready={} ready_demand={}",
            owner.root.is_ready(),
            owner.root.has_ready_demand()
        ).into());
    }
    result??;
    Ok(())
}
