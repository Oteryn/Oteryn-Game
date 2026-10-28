//! ANIMUS-1 durable completion replay, reload and stale-session fencing.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};

#[test]
fn animus_mastery_completion_replay_reload_and_fence_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("animus-mastery", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "animus_mastery", false, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("session")?;
            let dragon = "oteryn:creature.dragon";
            assert!(
                h.root
                    .complete_animus_mastery(&authority, &h.node, fence, dragon)
                    .await
                    .map_err(debug)?
            );
            assert!(
                !h.root
                    .complete_animus_mastery(&authority, &h.node, fence, dragon)
                    .await
                    .map_err(debug)?
            );
            assert_eq!(
                h.root
                    .read_animus_masteries(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                vec![dragon]
            );
            let mut stale = fence;
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
            assert!(
                h.root
                    .complete_animus_mastery(&authority, &h.node, stale, "oteryn:creature.rat")
                    .await
                    .is_err()
            );
            assert_eq!(
                h.root
                    .read_animus_masteries(&authority, fence.character_id)
                    .await
                    .map_err(debug)?,
                vec![dragon]
            );
            eprintln!("ANIMUS PostgreSQL replay, query and stale-session fence verified");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
