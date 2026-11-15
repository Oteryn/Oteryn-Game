//! Best-effort speed refresh must not retain the channel lane behind a background writer.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};

#[test]
fn movement_speed_refresh_does_not_wait_for_a_busy_character_revision_slot() -> TestResult {
    super::item_look::large_stack(|| {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "movement_revision_slot", false, false)
                    .await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
            let session = deps.admitted.game_session_id;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|_| "current fence")?
                .ok_or("current session")?;
            let slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let permit = owner.spell_lane_permit().await.ok_or("movement lane")?;
            let refresh = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                owner.refresh_cyclopedia_speed(deps.actor, session),
            )
            .await;
            assert!(
                refresh.is_ok(),
                "speed refresh must defer instead of waiting behind the held revision slot"
            );
            drop(permit);
            drop(slot);
            assert_eq!(
                owner.refresh_cyclopedia_speed(deps.actor, session).await,
                Some(0)
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
