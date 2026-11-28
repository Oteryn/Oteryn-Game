use super::TestResult;
use super::promotion_npc_fixtures::run_atelier_promotion_scenario;

#[test]
fn atelier_v2_query_is_served_after_same_session_promotion_and_readmission() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            drop(admin);
            run_atelier_promotion_scenario().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Atelier promotion PostgreSQL thread panicked")??;
    Ok(())
}

#[test]
fn atelier_post_promotion_initialization_rejects_stale_connection_generation() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(stale_generation_scenario(admin)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Atelier stale generation PostgreSQL thread panicked")??;
    Ok(())
}

async fn stale_generation_scenario(admin: String) -> TestResult {
    use super::atelier_fixtures::fixture;
    use super::fixture::{Harness, debug};
    use super::{Dependencies, prepare_current_session};
    use crate::durability::character_wheel::WheelRuleset;
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::durability::npc_purchase::PurchaseOutcome;
    use crate::foundation::ConnectionGeneration;
    use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
    use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
    use std::sync::Arc;

    let h =
        Harness::create_with_admission_seed(admin, "atelier_stale_promotion", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let dependencies = Dependencies::new()?;
    let gate = SpellPremiumCoordinator::new(Default::default(), None, None, Some(h.root.clone()));
    let data = WheelGemData::embedded().map_err(debug)?;
    let mut owner = dependencies.owner(&h, &authority);
    owner.wheel_ruleset = Some(Arc::new(
        WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?,
    ));
    owner.premium_coordinator = Some(&gate);
    let session = dependencies.admitted.game_session_id;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    dependencies
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            dependencies.actor,
            session,
            current.current_character_lease(),
        )
        .map_err(debug)?;
    fixture::eligible(&h.pool).await?;
    owner.admit_wheel_session(session).await;
    let request = fixture::prepare(&h).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current promotion fence")?
        .ok_or("active session")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    // Stop after the real durable COMMIT so the post-promotion step first sees
    // a mismatched generation while the Character is eligible but uninitialized.
    let promoted = slot
        .commit_npc_purchase(&h.root, &authority, &h.node, fence, request, &gate)
        .await
        .map_err(debug)?;
    assert!(matches!(promoted, PurchaseOutcome::Committed(_)));
    let mut stale = fence.item_fence();
    stale.connection_generation =
        ConnectionGeneration::new(stale.connection_generation.get() + 1).map_err(debug)?;
    owner
        .initialize_atelier_after_promotion(&mut slot, dependencies.actor, stale)
        .await;
    assert!(
        h.root
            .read_character_atelier(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .is_none()
    );
    assert!(
        owner
            .states
            .atelier_sessions
            .lock()
            .map_err(debug)?
            .get(&session)
            .is_none_or(Option::is_none)
    );

    owner
        .initialize_atelier_after_promotion(&mut slot, dependencies.actor, fence.item_fence())
        .await;
    assert!(
        owner
            .states
            .atelier_sessions
            .lock()
            .map_err(debug)?
            .get(&session)
            .is_some_and(Option::is_some)
    );
    drop(slot);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
