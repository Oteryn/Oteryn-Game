//! Read-only native completion admission; fixture state is rolled back, never earned.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::{Bindings, actor_admission::QuestPrerequisite},
};
#[path = "support/m01c_admission_probe.rs"]
mod admission_probe;
#[allow(dead_code)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod fixture;

#[test]
fn m01c_native_prerequisite_uses_its_registered_completion_owner_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let started=std::time::Instant::now();
            let h=fixture::Harness::create(admin,"questpc3_m01c_native_prerequisite",true).await?;
            let version: String=sqlx::query_scalar("SHOW server_version_num").fetch_one(&h.pool).await?;
            eprintln!("M01C_NATIVE_PREREQUISITE POSTGRES_CONNECTED version={version}");
            let loaded=oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("decaying-defender-pg").map_err(fixture::debug)?;
            let production=loaded.catalogue().authored_bindings().and_then(|b|b.quests.iter().find(|q|q.quest=="oteryn:quest.decaying_defender_outfits_quest")).ok_or("production Decaying Defender binding")?;
            let bindings=Bindings { schema:1,quests:vec![production.clone()] };
            let quest=bindings.quests[0].quest.clone();
            let catalogue=bindings.attach(&QuestStateCatalogue::empty("native-prerequisite-fixture").ok_or("catalogue")?)?;
            assert!(!admission_probe::permits_level(&catalogue,&quest,249));
            assert!(admission_probe::permits_level(&catalogue,&quest,250));
            assert!(admission_probe::requires_premium(&catalogue,&quest));
            eprintln!("M01C_DECAYING_DEFENDER production_binding level_boundary_probe249_250 premium_required native_prerequisite query_component_only");
            let character=fixture::id(fixture::CHARACTER);
            let mut tx=h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(character.as_slice()).execute(&mut *tx).await?;
            assert!(!admission_probe::prerequisites_hold(&mut tx,&character,&catalogue,&quest).await?);
            // Uncommitted query fixtures: deferred ownership guards remain enabled.
            // No Native victory, outfit, receipt or playable quest is claimed here.
            sqlx::query("INSERT INTO game_character_rotten_blood(character_id,completed) VALUES(encode($1,'hex')::uuid,false)")
                .bind(character.as_slice()).execute(&mut *tx).await?;
            assert!(!admission_probe::prerequisites_hold(&mut tx,&character,&catalogue,&quest).await?);
            sqlx::query("UPDATE game_character_rotten_blood SET completed=true,final_taint=true WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).execute(&mut *tx).await?;
            assert!(admission_probe::prerequisites_hold(&mut tx,&character,&catalogue,&quest).await?,"native owner completion must not require a synthetic generic quest-state row");
            let canonical: i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
                .bind(character.as_slice()).bind(oteryn_gameplay_quests::quest::rotten_blood::QUEST).fetch_one(&mut *tx).await?;
            assert_eq!(canonical,0,"admission must not copy another owner's state");
            let mut mixed = bindings;
            mixed.quests[0].admission.as_mut().ok_or("admission")?.prerequisites.push(QuestPrerequisite {
                requirement: "another independent prerequisite".into(),
                quest: "oteryn:quest.m01c_missing_dependency".into(),
            });
            let mixed = mixed.attach(&QuestStateCatalogue::empty("mixed-prerequisite-fixture").ok_or("catalogue")?)?;
            assert!(!admission_probe::prerequisites_hold(&mut tx, &character, &mixed, &quest).await?,
                "native completion must not bypass a missing generic prerequisite");
            sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'native-prerequisite-fixture',decode(repeat('11',32),'hex'),2,2,2)")
                .bind(character.as_slice()).bind("oteryn:quest.m01c_missing_dependency").execute(&mut *tx).await?;
            assert!(admission_probe::prerequisites_hold(&mut tx, &character, &mixed, &quest).await?,
                "all native and generic prerequisites together must satisfy admission");
            sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'native-prerequisite-fixture',decode(repeat('22',32),'hex'),2,2,2)")
                .bind(character.as_slice()).bind(oteryn_gameplay_quests::quest::rotten_blood::QUEST).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_character_rotten_blood SET completed=false WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).execute(&mut *tx).await?;
            assert!(!admission_probe::prerequisites_hold(&mut tx, &character, &catalogue, &quest).await?,
                "a stale generic shadow must never override the native owner's refusal");
            tx.rollback().await?;
            let persisted: i64=sqlx::query_scalar("SELECT count(*) FROM game_character_rotten_blood WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(persisted,0,"query fixtures must never commit unearned completion");
            reject_unwitnessed_native_door(&h).await?;
            h.cleanup().await?;
            assert!(started.elapsed().as_secs_f64()>0.01);
            eprintln!("M01C_NATIVE_PREREQUISITE POSTGRES_CASE_COMPLETED duration_ms={} missing_refused incomplete_refused native_completed_accepted mixed_both_required stale_generic_shadow_refused no_foreign_writes rolled_back",started.elapsed().as_millis());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"native prerequisite PG thread")?.map_err(Into::into)
}

async fn reject_unwitnessed_native_door(h: &fixture::Harness) -> fixture::TestResult {
    use durability::character_progression::CharacterProgressionError;
    use durability::quest_state::{QuestCause, QuestTransitionRequest};
    let bindings = Bindings::parse(include_bytes!(
        "../../quests/src/quest/authored_bindings/fixtures/door_use.json"
    ))?;
    let catalogue =
        bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    let command = foundation::CommandRef::new(
        fence.game_session_id,
        foundation::CommandId::new(9910).map_err(fixture::debug)?,
    );
    let outcome = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-door-use/m01c-fixture".into(),
                cause: QuestCause::Command(command),
            },
            std::sync::Arc::new(catalogue),
        )
        .await;
    assert!(
        matches!(outcome, Err(CharacterProgressionError::InvalidInput)),
        "a caller-supplied command is not a captured native opening: {outcome:?}"
    );
    let tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-door-fixture'")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(tracks, 0);
    eprintln!("M01C_DOOR_UNWITNESSED plain_command_refused no_progress_written");
    Ok(())
}
