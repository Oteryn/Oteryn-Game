//! Avar Tar's two Demon Outfit prompts through the real NPC command dispatcher.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks::Position};
use std::sync::Arc;

#[test]
fn avar_tar_explains_the_demon_outfit_before_asking_for_a_choice() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-the-annihilator", || {
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
                    super::quest_world_callbacks::local(Position {
                        x: 33294,
                        y: 31677,
                        floor: -15,
                    }),
                )
                .map_err(debug)?;
            let mut admitted = deps.admitted;
            let mut owner = deps.owner(&harness, &authority);
            owner.quest_catalogue = Some(Arc::new(
                load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            owner.admit_quest_session(&admitted).await;
            let [avar] = install(&owner, &admitted, ["oteryn:npc.avar_tar"]).await?;
            talk(&mut owner, &mut admitted, avar, 2600, "hi").await?;
            let explanation = talk(&mut owner, &mut admitted, avar, 2601, "outfit").await?;
            let choice = talk(&mut owner, &mut admitted, avar, 2602, "yes").await?;
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            assert_eq!(
                explanation,
                [
                    "I'm tired of all these young unskilled wannabe heroes. Every Tibian can show his skills or actions by wearing a special outfit. To prove oneself worthy of the demon outfit, this is how it goes: ...",
                    "The base outfit will be granted for completing the annihilator quest, which isn't much of a challenge nowadays, in my opinion. Anyway ...",
                    "The shield however will only be granted to those adventurers who have finished the demon helmet quest. ...",
                    "Well, the helmet is for those who really are tenacious and have hunted down all 6666 demons and finished the demon oak as well. ...",
                    "Are you interested?",
                ],
            );
            assert_eq!(
                choice,
                ["So you want to have the demon outfit, hah! Let's have a look first if you really deserve it. Tell me: {base}, {shield} or {helmet}?"],
            );
            Ok(())
        })
    });
    eprintln!("QCX39_POSTGRES_CASE duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}
