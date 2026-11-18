//! TibiaWiki Grave Danger Quest/Spoiler, curid 88737, oldid 1176165.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use oteryn_gameplay_quests::quest::{grave_danger as grave, loader};
use std::sync::Arc;

#[test]
fn jack_starts_twelve_graves_and_answers_an_early_report() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-grave-danger", scenario);
    eprintln!(
        "QCX_GRAVE_DANGER duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx_grave_danger", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let admitted = dependencies.admitted;
        let mut owner = dependencies.owner(&h, &authority);
        let lowered = loader::load_embedded_quest_state("content-1").map_err(debug)?;
        owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
        owner.admit_quest_session(&admitted).await;
        let [jack] = install(&owner, &admitted, [grave::NPC]).await?;
        talk(&mut owner, &admitted, jack, 100, "hi").await?;
        let lines = talk(&mut owner, &admitted, jack, 101, "late").await?;
        assert!(
            lines
                .iter()
                .any(|line| line.contains("very potent holy water"))
        );
        assert_eq!(
            progress(&owner, &admitted, &authority, grave::QUESTLINE).await?,
            1
        );
        let initialized: Vec<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks
             WHERE track_key LIKE 'oteryn:quest-progress/quest/u12_20/grave_danger/graves/%'",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(initialized, vec![1; 12]);

        let lines = talk(&mut owner, &admitted, jack, 102, "report").await?;
        assert_eq!(
            lines,
            [
                "Sadly, I have no news yet. But I can give you information about the {locations} of the graves that we learned about. If you need more holy water just ask me for a {trade}."
            ]
        );
        talk(&mut owner, &admitted, jack, 103, "late").await?;
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key = $1",
        )
        .bind(grave::START)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 1);
        Ok(())
    })
}
