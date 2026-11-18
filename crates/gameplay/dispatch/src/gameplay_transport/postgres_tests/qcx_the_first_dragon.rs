//! First Dragon knowledge trial through the production NPC command dispatcher.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const ACCESS: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/gelidrazah_access";

#[test]
fn gelidrazah_quiz_requires_all_three_answers_in_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx_first_dragon_quiz", || {
        run(async |admin| {
            super::reset();
            let h =
                Harness::create_with_admission_seed(admin, "qcx_first_dragon_quiz", false, false)
                    .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    &[3, 4, 6, 11, 15, 18, 25],
                    &[3, 4, 6, 11, 15, 18, 25],
                )
                .ok_or("quest capabilities")?;
            let [npc] = super::classic_quests::install(
                &owner,
                &admitted,
                ["oteryn:npc.gelidrazahs_thirst"],
            )
            .await?;
            let read_access = || {
                sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT max(value) FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
                )
                .bind(QUEST)
                .bind(ACCESS)
                .fetch_one(&h.pool)
            };

            // TibiaWiki spoiler oldid 1054086, Gelidrazah's Thirst transcript.
            for (index, (message, expected)) in [
                ("hi", "Have you come to answer Gelidrazah's questions?"),
                ("yes", "There are three questions."),
                ("yes", "So I ask you:"),
                ("Tahmehe", "That's right. Listen to the second question:"),
                ("Svir", "I don't know what you are talking about."),
                ("Ishara", "That's right. Listen to the third question:"),
                ("Svir", "You may pass and enter Gelidrazah's lair."),
            ]
            .into_iter()
            .enumerate()
            {
                let reply = super::classic_quests::talk(
                    &mut owner,
                    &admitted,
                    npc,
                    4800 + u64::try_from(index)?,
                    message,
                )
                .await?;
                assert!(reply.join(" ").contains(expected), "{message}: {reply:?}");
                assert_eq!(read_access().await?, (index == 6).then_some(1));
            }
            Ok(())
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED gelidrazah_quiz_requires_all_three_answers_in_order duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
