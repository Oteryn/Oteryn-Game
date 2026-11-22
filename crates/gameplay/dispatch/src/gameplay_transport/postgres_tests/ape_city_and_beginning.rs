//! Production quest dialogue uses the current character, NPC, and quest owners.
use super::fixture::{Harness, debug, id, run};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::foundation::WorldId;
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const APE: &str = "oteryn:quest.the_ape_city_quest";
const APE_TRACK: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/";
const ROOK_TRACK: &str = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/";

#[test]
fn ape_city_and_beginning_production_npc_progress_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("ape-city-and-beginning", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "ape_city_beginning", true, false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    super::prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        7,
    )?;
    deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(controller.active().ok_or("active generation")?);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone(),
    ));
    let mut admitted = deps.admitted;
    owner.refresh_quest_session(admitted.game_session_id, true).await;
    let [santiago, zirella, carlos, hairycles] = super::classic_quests_fixtures::install_with_offsets(
        &owner,
        &admitted,
        [
            "oteryn:npc.santiago", "oteryn:npc.zirella",
            "oteryn:npc.carlos", "oteryn:npc.hairycles",
        ],
        Some([(0, 1), (1, 1), (0, 2), (1, 2)]),
    ).await?;
    super::qd_world_fixtures::select_talk_capabilities(&owner, &mut admitted, &[3, 6, 25])?;
    let mut command = 1000;
    for (npc, phrases, track, value) in [
        (santiago, &["hi", "yes", "yes", "bye"][..], format!("{ROOK_TRACK}santiago_quest_log"), 2),
        (zirella, &["hi", "quest", "yes", "yes", "yes", "yes", "yes", "yes", "bye"][..], format!("{ROOK_TRACK}zirella_quest_log"), 6),
        (carlos, &["hi", "yes", "hi", "help", "yes", "yes", "bye"][..], format!("{ROOK_TRACK}carlos_quest_log"), 5),
        (hairycles, &["hi", "mission", "yes", "yes"][..], format!("{APE_TRACK}dworc_door"), 1),
    ] {
        conversation(&mut owner, &mut admitted, npc, &mut command, phrases).await?;
        assert_eq!(progress(h, &track).await?, value, "{track}");
    }

    seed_monument(h).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    conversation(&mut owner, &mut admitted, hairycles, &mut command, &["mission"]).await?;
    assert_eq!(progress(h, &format!("{APE_TRACK}questline")).await?, 18);
    let complete: bool = sqlx::query_scalar(
        "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$1",
    ).bind(APE).fetch_one(&h.pool).await?;
    assert!(complete);
    let awarded: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind("oteryn:achievement/friend_of_the_apes")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(awarded, 1);
    conversation(&mut owner, &mut admitted, hairycles, &mut command, &["mission"]).await?;
    assert_eq!(progress(h, &format!("{APE_TRACK}questline")).await?, 18);
    Ok(())
}

async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    phrases: &[&str],
) -> TestResult {
    for phrase in phrases {
        *command += 1;
        let outcome = super::qengine_rewards_fixtures::talk(owner, admitted, npc, *command, phrase).await?;
        if *phrase != "bye" {
            assert_eq!(outcome, NpcIntentDisposition::Ok, "{phrase}");
        }
    }
    Ok(())
}

async fn progress(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    ).bind(track).fetch_one(&h.pool).await?)
}

// Prepare the persisted outcome of the monument USE. The actual report must
// read this revision through the production session and grant its own receipt.
async fn seed_monument(h: &Harness) -> TestResult {
    let mut transaction = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *transaction)
        .await?;
    let stage = format!("{APE_TRACK}questline");
    let revision: String = sqlx::query_scalar(
        "UPDATE game_character_quest_tracks SET value=17 WHERE track_key=$1 AND value=1
         RETURNING committed_character_revision::text",
    )
    .bind(stage)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id, track_key, quest_key, value, committed_character_revision)
         VALUES (encode($1,'hex')::uuid,$2,$3,1,$4::text::numeric)",
    )
    .bind(id(41).as_slice())
    .bind(format!("{APE_TRACK}snake_destroyer"))
    .bind(APE)
    .bind(revision)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
