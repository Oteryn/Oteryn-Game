//! Shared physical source pickups: no personal quest claim substitutes for map custody.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[path = "l6_world_pickups/wire.rs"]
mod wire;
#[path = "l6_world_pickups/world.rs"]
mod world;
pub(super) use world::{
    ExpectedTeleport, ExpectedTile, NativeCrop, native_crop, native_scene,
    native_scene_with_appearances, small_ruby,
};
#[path = "l6_world_pickups/reset.rs"]
mod reset;
pub(super) use reset::cycle;

pub(super) const RUBY: Position = Position {
    x: 32437,
    y: 32174,
    floor: -15,
};

#[test]
fn l6_small_ruby_native_pickup_replay_and_shared_custody() -> TestResult {
    run_case("l6-small-ruby")
}

#[test]
fn l6_small_ruby_actual_reset_replenishes_and_refuses_old_epoch() -> TestResult {
    run_case("l6-small-ruby-reset")
}

fn run_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = (|| -> TestResult {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn(move || {
                super::quest_world_callbacks::scenario(name, RUBY, false)
                    .map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "Small Ruby source pickup panicked")??;
        Ok(())
    })();
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    prior_pickups: i64,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_small_ruby_native_pickup version={version}");
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 18],
        )
        .ok_or("physical pickup capabilities")?;
    assert_eq!(ledger(harness).await?, (prior_pickups, 0));
    wire::pickup(owner, admitted).await?;
    assert_eq!(ledger(harness).await?, (prior_pickups + 1, 0));
    let ruby: (i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3030' AND e.character_id=encode($1,'hex')::uuid")
        .bind(super::fixture::id(41).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(
        ruby,
        (1, prior_pickups + 1),
        "one shared physical source earns one carried ruby"
    );
    Ok(())
}

async fn ledger(harness: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_map_item_mint_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)")
        .fetch_one(&harness.pool).await?)
}

pub(super) async fn refuse_old_epoch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::item_move::{
        LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget,
    };
    use crate::gameplay_transport::item_view::ItemKey;
    let map = owner.bundle_map().ok_or("reset map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-15, 32437, 32174, 1).ok_or("ruby key")?;
    let before = ledger(h).await?;
    let result = owner
        .move_loop_item(
            admitted.runtime_actor.ok_or("new reset actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: admitted.continuity.next_command_id,
                item_fence: admitted.item_fence,
            },
            LoopMove {
                item: ItemKey::MapBase {
                    bundle_digest: map.overlay.base().digest(),
                    placement_key: key,
                    reset_epoch: 0,
                },
                count: None,
                target: LoopTarget::MainBackpack,
            },
        )
        .await;
    assert_eq!(result, LoopMoveResult::Refused(LoopMoveRefusal::Stale));
    assert_eq!(ledger(h).await?, before);
    let rows: Vec<(String, Vec<u8>, String, String, i64)> = sqlx::query_as("SELECT reset_epoch::text,base_bundle_digest,placement_key::text,definition_production_key,quantity::bigint FROM game_map_item_mint_receipts ORDER BY reset_epoch")
        .fetch_all(&h.pool).await?;
    assert_eq!(rows.len(), 2);
    assert_eq!((&rows[0].0, &rows[1].0), (&"0".to_owned(), &"1".to_owned()));
    assert_eq!((&rows[0].1, &rows[0].2), (&rows[1].1, &rows[1].2));
    assert!(
        rows.iter()
            .all(|row| row.3 == "oteryn:item.tibia.i3030" && row.4 == 1)
    );
    Ok(())
}
