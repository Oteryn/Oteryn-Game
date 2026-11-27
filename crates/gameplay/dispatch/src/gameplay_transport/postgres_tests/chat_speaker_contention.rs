use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;

#[test]
fn chat_speaker_unavailable_keeps_named_visibility_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("chat-speaker-busy", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "chat_speaker_busy", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let session = deps.admitted.game_session_id;
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session(session)
                .await?;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(deps.actor, session, current.current_character_lease())
                    .map_err(debug)?;
            }
            let held = h.root.try_issue_semantic_pass()?;
            let fallback = owner
                .named_channel_entities(deps.actor, false)
                .await
                .map_err(debug)?;
            let own = |channel: crate::gameplay_transport::world_spatial::ChannelEntities| {
                channel
                    .entities
                    .into_iter()
                    .find(|entry| entry.identity == channel.observer)
                    .and_then(|entry| entry.display_name)
                    .ok_or("named observer")
            };
            assert_eq!(own(fallback)?.as_str().map_err(debug)?, "Unknown");
            assert!(!owner.states.chat.knows(session));
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            let restored = owner
                .named_channel_entities(deps.actor, false)
                .await
                .map_err(debug)?;
            assert_ne!(own(restored)?.as_str().map_err(debug)?, "Unknown");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
