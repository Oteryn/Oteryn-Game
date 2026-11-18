//! Avar Tar's Demon Outfit phrase and confirmation through the real NPC command dispatcher.
use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks::Position};
use std::sync::Arc;

#[test]
fn avar_tar_accepts_demon_outfit_and_advances_to_the_choice() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qcx-the-annihilator", || {
        run(|admin| async move {
            let harness =
                Harness::create_with_admission_seed(admin, "qcx_annihilator", false, false).await?;
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime
                .lock()
                .await
                .initialize_pinned_test_position(
                    deps.actor,
                    super::quest_world_callbacks_fixtures::local(Position {
                        x: 33294,
                        y: 31677,
                        floor: -15,
                    }),
                )
                .map_err(debug)?;
            let admitted = deps.admitted;
            let mut owner = deps.owner(&harness, &authority);
            owner.quest_catalogue = Some(Arc::new(
                load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            owner.admit_quest_session(&admitted).await;
            let [avar] = install(&owner, &admitted, ["oteryn:npc.avar_tar"]).await?;
            talk(&mut owner, &admitted, avar, 2600, "hi").await?;
            let explanation = talk(&mut owner, &admitted, avar, 2601, "demon outfit").await?;
            let choice = talk(&mut owner, &admitted, avar, 2602, "yes").await?;
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            assert_eq!(
                (explanation, choice),
                (
                    vec!["Choose the base, shield or helmet.".to_owned()],
                    vec!["Choose the base, shield or helmet.".to_owned()],
                ),
                "the full outfit phrase must open the offer and advance to the confirmation"
            );
            Ok(())
        })
    });
    eprintln!(
        "QCX39_POSTGRES_CASE duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
