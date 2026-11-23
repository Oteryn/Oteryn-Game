//! Exercise both committed coffin selectors through dispatch and the durable writer.
use super::fixture::{Harness, debug};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks_fixtures as fixture,
};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const RESTART: &str = "inquisition-weakened-coffin";
pub(super) const FOREIGN_GENERATION: &str = "inquisition-weakened-coffin-side";

pub(super) fn matches(name: &str) -> bool {
    matches!(name, RESTART | FOREIGN_GENERATION)
}

pub(super) fn staging(name: &str) -> TestResult<Position> {
    Ok(fixture::callback(name)?
        .effects
        .encounter
        .as_ref()
        .ok_or("coffin encounter")?
        .players[0]
        .from)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    weakened: &Callback,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    owner
        .runtime
        .lock()
        .await
        .install_companion_policies(native.policies().clone())
        .map_err(debug)?;
    let requirements = weakened
        .storage
        .iter()
        .map(|gate| (gate.track.as_str(), gate.minimum))
        .collect::<Vec<_>>();
    super::rottin_rewards_fixtures::seed_tracks(h, owner, admitted, &requirements).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("coffin fence")?;
    for (index, offering) in weakened.offerings.iter().enumerate() {
        fixture::offering(
            owner,
            fence.item_fence(),
            offering,
            80 + u64::try_from(index)?,
        )
        .await?;
    }
    owner.states.loop_ground.invalidate();
    let foreign_generation = weakened.key.ends_with(FOREIGN_GENERATION);
    if foreign_generation {
        // Fixtures own generation 1. Represent carried-over drops with a distinct
        // generation while keeping their location, stack and item identity intact.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let changed = sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=2 WHERE runtime_scope_ownership_generation=1 AND game_item_loop_drop_ref(corpse_ref)")
            .execute(&mut *tx)
            .await?;
        assert_eq!(
            changed.rows_affected(),
            u64::try_from(weakened.offerings.len())?
        );
        tx.commit().await?;
    } else {
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, std::sync::atomic::Ordering::Release);
        assert_eq!(
            use_coffin(owner, admitted, weakened, 61).await?,
            UseDisposition::Rejected
        );
        assert!(owner.has_pending_quest_callback(admitted.game_session_id));
        assert!(
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .is_empty()
        );
        assert_eq!(burns(h).await?, i64::try_from(weakened.offerings.len())?);
        owner.forget_coffin_projection_for_test(admitted.game_session_id)?;
        owner.states.loop_ground.invalidate();
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    assert_eq!(
        use_coffin(owner, admitted, weakened, 62).await?,
        UseDisposition::Committed
    );
    let expected = if foreign_generation {
        let fallback = weakened
            .vampire_coffin_fallback()
            .ok_or("ordinary coffin")?;
        fixture::callback(fallback)?
    } else {
        weakened.clone()
    };
    let runtime = owner.runtime.lock().await;
    let creatures = runtime.visible_entities().creatures;
    assert_eq!(creatures.len(), 1);
    assert_eq!(
        runtime
            .companion_snapshot(creatures[0].actor)
            .map_err(debug)?
            .state
            .policy
            .definition_key,
        expected
            .effects
            .encounter
            .as_ref()
            .ok_or("expected encounter")?
            .monsters[0]
            .creature,
    );
    drop(runtime);
    let expected_burns = if foreign_generation {
        0
    } else {
        i64::try_from(weakened.offerings.len())?
    };
    assert_eq!(burns(h).await?, expected_burns);
    Ok(())
}

async fn use_coffin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    weakened: &Callback,
    command_id: u64,
) -> TestResult<UseDisposition> {
    let Position { x, y, floor } = weakened.selector.position.ok_or("coffin position")?;
    let key = placement_key(TilePos { x, y, floor }, 1).ok_or("coffin placement")?;
    let map = owner.bundle_map().ok_or("coffin map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("coffin fence")?;
    let result = owner
        .use_quest_callback(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            map,
            &WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await
        .ok_or("committed coffin selector was not dispatched")?;
    Ok(result.disposition)
}

async fn burns(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
            .fetch_one(&h.pool)
            .await?,
    )
}
