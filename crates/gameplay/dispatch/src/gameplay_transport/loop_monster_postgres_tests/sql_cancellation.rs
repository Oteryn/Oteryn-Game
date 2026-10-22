//! Cancellation while the actual scope Item lock is awaited preserves the retained step.
use super::*;

pub(super) async fn cancel_then_retry(
    harness: &Harness,
    deps: &Dependencies,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    rat: ExactActorRef,
) -> TestResult {
    let (position, step, sequence, think) = {
        let runtime = deps.runtime.lock().await;
        let states = owner.spell_states.lock().await;
        let table = &states.monster_melee.creature_ai;
        (
            runtime.read_actor_position(rat).map_err(debug)?,
            table.step_due_for_test(rat).ok_or("pending step")?,
            table.last_report_for_test(rat).ok_or("report")?.sequence,
            table.due(rat),
        )
    };
    let mut blocker = harness.pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'), 33))",
    )
    .bind(deps.admitted.world_id.as_bytes().as_slice())
    .bind(deps.admitted.channel_id.as_bytes().as_slice())
    .execute(&mut *blocker)
    .await?;
    owner.clock_origin -=
        std::time::Duration::from_micros(step.saturating_sub(owner.owner_now().get()) + 100);
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(30),
            owner.drain_monster_melee()
        )
        .await
        .is_err(),
        "the actual Item scope lock held the owner pass"
    );
    {
        let runtime = deps.runtime.lock().await;
        let states = owner.spell_states.lock().await;
        let table = &states.monster_melee.creature_ai;
        assert_eq!(runtime.read_actor_position(rat).map_err(debug)?, position);
        assert_eq!(table.step_due_for_test(rat), Some(step));
        assert_eq!(table.due(rat), think);
        assert_eq!(
            table.last_report_for_test(rat).ok_or("report")?.sequence,
            sequence
        );
    }
    blocker.rollback().await?;
    // The production root's maintenance loop observes this newly released SQL holder.
    harness.root.request_ready();
    harness.root.maintain_ready_once().await?;
    owner.drain_monster_melee().await;
    let runtime = deps.runtime.lock().await;
    assert_ne!(
        runtime.read_actor_position(rat).map_err(debug)?,
        position,
        "retry committed the retained step without another think"
    );
    assert_eq!(
        owner
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .last_report_for_test(rat)
            .ok_or("report")?
            .sequence,
        sequence
    );
    Ok(())
}
