//! Shared source container stock uses real admission and PostgreSQL map-pickup ownership.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{container_view::ContainerObservation, item_view::ItemKey};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn draconia_container_stock_shared_withdrawal() -> TestResult {
    scenario("shared")
}
#[test]
fn draconia_container_stock_replay() -> TestResult {
    scenario("replay")
}
#[test]
fn draconia_container_stock_rejects_forged_origin() -> TestResult {
    scenario("forged")
}
fn scenario(case: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let name = format!("draconia-container-stock-{case}");
    let result = super::kill_reward_e2e::scenario_thread::run(case, move || {
        super::quest_world_callbacks::scenario(
            &name,
            Position {
                x: 32794,
                y: 31572,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_{} draconia_container_stock_{case} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    assert!(version.starts_with("PostgreSQL"));
    eprintln!("POSTGRES_CONTAINER_STOCK_OWNER actual_postgresql=true");
    let base = owner.bundle_map().ok_or("bundle map")?.overlay.base();
    let root = ItemKey::MapBase {
        bundle_digest: base.digest(),
        placement_key: crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: 32794,
                y: 31572,
                floor: -7,
            },
            1,
        )
        .ok_or("placement")?,
        reset_epoch: owner.reset_epoch,
    };
    let view = crate::gameplay_transport::loop_move::observe_container(
        owner,
        actor,
        admitted.item_fence,
        root,
    )
    .await;
    let Some(ContainerObservation::Container {
        parent,
        capacity,
        entries,
    }) = view
    else {
        return Err(format!("source skeleton must open as shared container, got {view:?}").into());
    };
    assert_eq!(parent, None);
    assert_eq!(capacity, 10);
    assert_eq!(entries.len(), 1);
    let (definition, _) = owner
        .active_generation
        .ok_or("generation")?
        .native_gameplay()
        .ok_or("native")?
        .item_index()
        .ok_or("index")?
        .definition_at(entries[0].item_definition_ref)
        .ok_or("child definition")?;
    assert_eq!(definition, "oteryn:item.tibia.i2969");
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 4100,
        item_fence: admitted.item_fence,
    };
    use crate::gameplay_transport::item_move::{
        LoopMove, LoopMoveRefusal, LoopMoveReplay, LoopMoveResult, LoopTarget,
    };
    let request = LoopMove {
        item: entries[0].key,
        count: None,
        target: LoopTarget::MainBackpack,
    };
    if name.ends_with("forged") {
        let ItemKey::MapBase {
            bundle_digest,
            placement_key,
            reset_epoch,
        } = request.item
        else {
            return Err("source child must retain reset ownership".into());
        };
        for item in [
            ItemKey::MapBase {
                bundle_digest,
                placement_key: placement_key - 1,
                reset_epoch,
            },
            ItemKey::MapBase {
                bundle_digest,
                placement_key,
                reset_epoch: reset_epoch + 1,
            },
        ] {
            let refused = crate::gameplay_transport::loop_move::move_loop_item(
                owner,
                actor,
                command,
                LoopMove { item, ..request },
            )
            .await;
            assert_eq!(refused, LoopMoveResult::Refused(LoopMoveRefusal::Stale));
        }
        let keys: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_keys WHERE key_number=3001")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(keys, 0);
    }
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(owner, actor, command, request).await,
        LoopMoveResult::Committed
    );
    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_keys WHERE key_number=3001")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(keys, 1);
    if name.ends_with("replay") {
        assert_eq!(
            crate::gameplay_transport::loop_move::committed_loop_move(owner, command).await,
            Some(LoopMoveReplay::Moved)
        );
    }
    // Force restoration from durable shared placement receipts, not the connection's old pane.
    owner.states.loop_ground.invalidate();
    let rebuilt = crate::gameplay_transport::loop_move::observe_container(
        owner,
        actor,
        admitted.item_fence,
        root,
    )
    .await;
    assert!(
        matches!(rebuilt, Some(ContainerObservation::Container { entries, .. }) if entries.is_empty())
    );
    let again = crate::gameplay_transport::connection::UseCommand {
        command_id: 4101,
        ..command
    };
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(owner, actor, again, request).await,
        LoopMoveResult::Refused(LoopMoveRefusal::Stale)
    );
    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_keys WHERE key_number=3001")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(keys, 1);
    Ok(())
}
