//! A first item move keeps its replay checks intact while the foreground holder is busy.
use super::fixture::{Harness, debug, fence};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_move::{EquipmentMoveReplay, LoopMoveReplay};
use std::{future::Future, task::Poll};

#[test]
fn first_item_move_replay_reads_wait_for_a_busy_holder_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "item_move_receipt_busy", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&harness, &authority);
        let item_fence = fence(1)?.item_fence();
        let command = UseCommand {
            game_session_id: item_fence.game_session_id,
            command_id: 1,
            item_fence: Some(item_fence),
        };

        let transfer =
            during_checkout(&harness, owner.committed_item_move(deps.actor, command)).await?;
        assert!(transfer.map_err(debug)?.is_none());
        let equipment = during_checkout(
            &harness,
            owner.committed_equipment_move(deps.actor, command),
        )
        .await?;
        assert!(matches!(equipment, Some(EquipmentMoveReplay::Absent)));
        let loop_move =
            during_checkout(&harness, owner.committed_loop_move(deps.actor, command)).await?;
        assert!(matches!(loop_move, Some(LoopMoveReplay::Absent)));
        Ok(())
    })
}

async fn during_checkout<T>(harness: &Harness, read: impl Future<Output = T>) -> TestResult<T> {
    let held = harness.root.try_issue_semantic_pass().map_err(debug)?;
    let mut read = std::pin::pin!(read);
    let first = std::future::poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await;
    held.run(|_, _| Box::pin(async { Ok(()) }))
        .await
        .map_err(debug)?;
    assert!(
        first.is_pending(),
        "receipt lookup must wait for the holder instead of closing the command"
    );
    Ok(read.await)
}
