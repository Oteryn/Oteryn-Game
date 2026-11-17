//! Demon Oak compatibility through the composed dispatcher and PostgreSQL quest owner.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.demon_oak";
const DONE: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";

#[test]
fn oldrak_records_defeat_and_unlocks_the_grave_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx_demon_oak_oldrak", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?.catalogue().clone(),
        ));
        let [oldrak] = super::classic_quests::install(&owner, &admitted, ["oteryn:npc.oldrak"]).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(DONE).bind(QUEST).execute(&mut *tx).await?;
        tx.commit().await?;
        super::classic_quests::talk(&mut owner, &admitted, oldrak, 1, "hi").await?;
        for command_id in [2, 3] {
            super::classic_quests::talk(&mut owner, &admitted, oldrak, command_id, "demon oak").await?;
            assert_eq!(super::classic_quests::progress(&owner, &admitted, &authority, DONE).await?, 2);
        }
        Ok(())
    });
    eprintln!("POSTGRES_CASE_COMPLETED oldrak_records_defeat_and_unlocks_the_grave_once {:?} success={}", started.elapsed(), result.is_ok());
    result
}
