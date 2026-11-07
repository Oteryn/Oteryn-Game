//! Existing quest receipt owns completed Hive-task credit and personal-cycle reset.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::shared_world_progress;
#[test]
fn worldchg_postgres_hive_completed_task_and_cycle_reset_are_one_receipt() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start=std::time::Instant::now();
            let h=fixture::Harness::create(admin,"worldchg_hive_shared",true).await?;
            let result=exercise(&h).await;h.cleanup().await?;result?;
            assert!(start.elapsed().as_secs_f64()>1.0);
            println!("WORLDCHG-Q PG EXECUTED: native Hive completedtask personalcycle+Worldcredit atomic receipt,replay,incomplete refusal; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"Hive PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let world = scope.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let _day = h.root.world_changes(world, 1).await?.ok_or("current day")?;
    let binding = &shared_world_progress::BINDINGS[0];
    let mut catalogue = std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    );
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    // Materialize independently earned progress through the real Quest receipt writer.
    // The fixture transition represents prior gathering; all deferred receipt/track guards run.
    {
        use oteryn_gameplay_quests::quest::{
            QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
        };
        let fixture_tracks = [binding.track, shared_world_progress::ACTIVE_TRACKS[0]];
        let values = [8, 1];
        let seed_key = "oteryn:quest-transition/worldchg-fixture/earned-progress";
        let seeded = std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                fixture_tracks
                    .iter()
                    .map(|track| catalogue.track(track).cloned().ok_or("source track"))
                    .collect::<Result<Vec<_>, _>>()?,
                vec![
                    catalogue
                        .transition(binding.key)
                        .ok_or("native completion")?
                        .clone(),
                    QuestTransition {
                        key: seed_key.into(),
                        quest: shared_world_progress::HIVE_QUEST.into(),
                        effects: fixture_tracks
                            .iter()
                            .zip(values)
                            .map(|(track, value)| QuestEffect {
                                track: (*track).into(),
                                from: QuestComparison::Between(0, 0),
                                effect: QuestEffectKind::Set(value),
                            })
                            .collect(),
                        completes: false,
                        experience: None,
                    },
                ],
            )
            .map_err(fixture::debug)?,
        );
        assert!(matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fixture::fence(1)?,
                    QuestTransitionRequest {
                        transition_key: seed_key.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            session,
                            CommandId::new(99).map_err(fixture::debug)?
                        )),
                    },
                    seeded.clone()
                )
                .await?,
            QuestTransitionOutcome::Committed(_)
        ));
        // The native completion shares the definition pinned by the earned-progress receipt.
        catalogue = seeded;
    }
    let request = |ordinal| -> fixture::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: binding.key.into(),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            )),
        })
    };
    let completed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(2)?,
            request(1)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(&completed, QuestTransitionOutcome::Committed(_)),
        "native completion: {completed:?}"
    );
    let counts = || async {
        let state:serde_json::Value=sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='hive_born'").bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        Ok::<_, Box<dyn std::error::Error>>((
            state["counts"][0].as_u64().ok_or("points")?,
            state["counts"][1].as_u64().ok_or("actions")?,
        ))
    };
    assert_eq!(counts().await?, (1, 1));
    let personal:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(fixture::id(fixture::CHARACTER).as_slice()).bind(binding.track).fetch_one(&h.pool).await?;
    assert_eq!(personal, 0);
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(3)?,
                request(1)?,
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(3)?,
                request(2)?,
                catalogue
            )
            .await?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(counts().await?, (1, 1));
    Ok(())
}
