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
            println!("WORLDCHG-Q PG EXECUTED: native Hive Favour+Worldcredit, SAME pinned Quest catalogue seed/completion/purchase/expiry, atomic50 seven-day permit, safe egress, replay/refusal; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"Hive PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M55_POSTGRES_READY version={version}");
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M55_POSTGRES_READY version={version}");
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M55_POSTGRES_READY version={version}");
    let version: String = sqlx::query_scalar("SHOW server_version_num").fetch_one(&h.pool).await?;
    eprintln!("M55_POSTGRES_READY version={version}");
    let scope = fixture::scope()?;
    let world = scope.world_id();
    use oteryn_gameplay_quests::quest::hive_access as access;
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
    // Define the historical scent timestamp BEFORE the first Quest write, so its
    // fixture transition shares the same definition hash as native purchase/completion.
    let old_scent: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint-$1")
            .bind(access::DURATION_SECONDS)
            .fetch_one(&h.pool)
            .await?;
    let old_scent_key = "oteryn:quest-transition/worldchg-fixture/old-scent";
    // Materialize independently earned progress through the real Quest receipt writer.
    // The fixture transition represents prior gathering; all deferred receipt/track guards run.
    {
        use oteryn_gameplay_quests::quest::{
            QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
        };
        let fixture_tracks = [
            binding.track,
            shared_world_progress::ACTIVE_TRACKS[0],
            access::FAVOUR,
            access::PURCHASED,
        ];
        let values = [8, 1, 49, 0];
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
                    catalogue
                        .transition(access::BUY)
                        .ok_or("native scent purchase")?
                        .clone(),
                    QuestTransition {
                        key: old_scent_key.into(),
                        quest: shared_world_progress::HIVE_QUEST.into(),
                        effects: vec![QuestEffect {
                            track: access::PURCHASED.into(),
                            from: QuestComparison::Gt(0),
                            effect: QuestEffectKind::Set(old_scent),
                        }],
                        completes: false,
                        experience: None,
                        world_change: None,
                    },
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
                        world_change: None,
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
    let occurrence: (String, String, Vec<i64>) = sqlx::query_as("SELECT reset_epoch::text,quest_key,values_after FROM game_character_event_quest_progress WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(binding.key).fetch_one(&h.pool).await?;
    assert_eq!(occurrence.0, "1");
    assert_eq!(occurrence.1, shared_world_progress::HIVE_QUEST);
    assert!(
        occurrence.2.contains(&0),
        "task reset is retained in this occurrence"
    );

    let occurrence: (String, String, Vec<i64>) = sqlx::query_as("SELECT reset_epoch::text,quest_key,values_after FROM game_character_event_quest_progress WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(binding.key).fetch_one(&h.pool).await?;
    assert_eq!(occurrence.0, "1");
    assert_eq!(occurrence.1, shared_world_progress::HIVE_QUEST);
    assert!(
        occurrence.2.contains(&0),
        "task reset is retained in this occurrence"
    );

    let occurrence: (String, String, Vec<i64>) = sqlx::query_as("SELECT reset_epoch::text,quest_key,values_after FROM game_character_event_quest_progress WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(binding.key).fetch_one(&h.pool).await?;
    assert_eq!(occurrence.0, "1");
    assert_eq!(occurrence.1, shared_world_progress::HIVE_QUEST);
    assert!(
        occurrence.2.contains(&0),
        "task reset is retained in this occurrence"
    );

    let occurrence: (String, String, Vec<i64>) = sqlx::query_as("SELECT reset_epoch::text,quest_key,values_after FROM game_character_event_quest_progress WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(binding.key).fetch_one(&h.pool).await?;
    assert_eq!(occurrence.0, "1");
    assert_eq!(occurrence.1, shared_world_progress::HIVE_QUEST);
    assert!(occurrence.2.contains(&0), "task reset is retained in this occurrence");

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
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(counts().await?, (1, 1));
    let favour = || async {
        sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(access::FAVOUR).fetch_one(&h.pool).await
    };
    assert_eq!(
        favour().await?,
        50,
        "native task adds one to the 49 prior earned points, replay adds none"
    );
    let purchase = |ordinal| -> fixture::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: access::BUY.into(),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            )),
        })
    };
    assert!(
        matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fixture::fence(3)?,
                    purchase(3)?,
                    catalogue.clone()
                )
                .await?,
            QuestTransitionOutcome::Refused(_)
        ),
        "stage one does not sell inner Hive access"
    );
    assert_eq!(favour().await?, 50);
    // This case isolates the personal access owner. The shared counter/save case
    // independently proves the 200-action server-save advance; install its typed
    // historical stage-two snapshot without bypassing any Quest receipt guard.
    let mut stage:oteryn_gameplay_world_change::counter::CounterState=serde_json::from_value(
        sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='hive_born'")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?)?;
    stage.stage = 2;
    stage.phase_version += 1;
    stage
        .validate(oteryn_gameplay_world_change::counter::CounterKind::HiveBorn)
        .map_err(fixture::debug)?;
    sqlx::query("UPDATE game_world_change_counters SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND change_key='hive_born'")
        .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(stage)?).execute(&h.pool).await?;
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(3)?,
                purchase(4)?,
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(favour().await?, 0);
    let purchased:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(access::PURCHASED).fetch_one(&h.pool).await?;
    assert!(purchased > access::DURATION_SECONDS);
    let east = access::GATES
        .iter()
        .position(|g| g.minimum_stage == 2)
        .ok_or("native east gate")?;
    let west = access::GATES
        .iter()
        .position(|g| g.minimum_stage == 3)
        .ok_or("native west gate")?;
    let character = crate::domain::CharacterId::from_bytes(fixture::id(fixture::CHARACTER))
        .map_err(fixture::debug)?;
    {
        let mut tx = h.pool.begin().await?;
        assert!(
            crate::durability::world_event::changes::callbacks::hive_gate_permitted(
                &mut tx,
                world,
                1,
                character,
                &access::gate_key(east)
            )
            .await?
        );
        assert!(
            !crate::durability::world_event::changes::callbacks::hive_gate_permitted(
                &mut tx,
                world,
                1,
                character,
                &access::gate_key(west)
            )
            .await?
        );
        tx.rollback().await?;
    }
    assert!(
        matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fixture::fence(4)?,
                    purchase(5)?,
                    catalogue.clone()
                )
                .await?,
            QuestTransitionOutcome::Refused(_)
        ),
        "insufficient favour cannot refresh the timer"
    );
    // The old-scent fixture transition was pinned before the first Quest receipt;
    // no second catalogue/hash is introduced after this quest has started.
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(4)?,
                QuestTransitionRequest {
                    transition_key: old_scent_key.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(97).map_err(fixture::debug)?
                    )),
                },
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                fixture::fence(5)?,
                purchase(4)?,
                catalogue
            )
            .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let mut tx = h.pool.begin().await?;
    assert!(
        !crate::durability::world_event::changes::callbacks::hive_gate_permitted(
            &mut tx,
            world,
            1,
            character,
            &access::gate_key(east)
        )
        .await?,
        "expired purchase remains expired after replay"
    );
    tx.rollback().await?;
    assert_eq!(favour().await?, 0);
    // Only a closed, server-derived egress key skips stage and scent; the current
    // World epoch is still required. Real native wire coverage uses the same key derivation.
    stage.stage = 1;
    sqlx::query("UPDATE game_world_change_counters SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND change_key='hive_born'")
        .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(stage)?).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    assert!(
        !crate::durability::world_event::changes::callbacks::hive_gate_permitted(
            &mut tx,
            world,
            1,
            character,
            &access::gate_key(east)
        )
        .await?
    );
    let exit = format!("{}/exit", access::gate_key(east));
    assert!(
        crate::durability::world_event::changes::callbacks::hive_gate_permitted(
            &mut tx, world, 1, character, &exit
        )
        .await?,
        "closed stage and expired scent still allow egress"
    );
    assert!(
        !crate::durability::world_event::changes::callbacks::hive_gate_permitted(
            &mut tx, world, 999, character, &exit
        )
        .await?,
        "egress cannot use a stale World epoch"
    );
    tx.rollback().await?;
    let entries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        entries, 1,
        "replay and refused actions do not add occurrence progress"
    );
    // The view follows independently owned World reset state, preserving history.
    sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,2,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let current: i64 = sqlx::query_scalar("SELECT count(*) FROM game_current_event_quest_progress")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        current, 0,
        "a new occurrence starts without personal history"
    );
    let history: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(history, 1);
    let stale = sqlx::query("INSERT INTO game_character_event_quest_progress SELECT character_id,world_id,2,committed_character_revision,quest_key,transition_key,completes,track_keys,values_after FROM game_character_event_quest_progress").execute(&h.pool).await.err().ok_or("historical receipt accepted in a new occurrence")?;
    assert_eq!(
        stale.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        sqlx::query("UPDATE game_character_event_quest_progress SET completes=true")
            .execute(&h.pool)
            .await
            .is_err()
    );
    let entries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        entries, 1,
        "replay and refused actions do not add occurrence progress"
    );
    // The view follows independently owned World reset state, preserving history.
    sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,2,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let current: i64 = sqlx::query_scalar("SELECT count(*) FROM game_current_event_quest_progress")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        current, 0,
        "a new occurrence starts without personal history"
    );
    let history: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(history, 1);
    let stale = sqlx::query("INSERT INTO game_character_event_quest_progress SELECT character_id,world_id,2,committed_character_revision,quest_key,transition_key,completes,track_keys,values_after FROM game_character_event_quest_progress").execute(&h.pool).await.err().ok_or("historical receipt accepted in a new occurrence")?;
    assert_eq!(
        stale.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        sqlx::query("UPDATE game_character_event_quest_progress SET completes=true")
            .execute(&h.pool)
            .await
            .is_err()
    );
    let entries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        entries, 1,
        "replay and refused actions do not add occurrence progress"
    );
    // The view follows independently owned World reset state, preserving history.
    sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,2,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let current: i64 = sqlx::query_scalar("SELECT count(*) FROM game_current_event_quest_progress")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        current, 0,
        "a new occurrence starts without personal history"
    );
    let history: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(history, 1);
    let stale = sqlx::query("INSERT INTO game_character_event_quest_progress SELECT character_id,world_id,2,committed_character_revision,quest_key,transition_key,completes,track_keys,values_after FROM game_character_event_quest_progress").execute(&h.pool).await.err().ok_or("historical receipt accepted in a new occurrence")?;
    assert_eq!(
        stale.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        sqlx::query("UPDATE game_character_event_quest_progress SET completes=true")
            .execute(&h.pool)
            .await
            .is_err()
    );
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress").fetch_one(&h.pool).await?;
    assert_eq!(entries, 1, "replay and refused actions do not add occurrence progress");
    // The view follows independently owned World reset state, preserving history.
    sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,2,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let current: i64 = sqlx::query_scalar("SELECT count(*) FROM game_current_event_quest_progress").fetch_one(&h.pool).await?;
    assert_eq!(current, 0, "a new occurrence starts without personal history");
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_event_quest_progress").fetch_one(&h.pool).await?;
    assert_eq!(history, 1);
    let stale = sqlx::query("INSERT INTO game_character_event_quest_progress SELECT character_id,world_id,2,committed_character_revision,quest_key,transition_key,completes,track_keys,values_after FROM game_character_event_quest_progress").execute(&h.pool).await.err().ok_or("historical receipt accepted in a new occurrence")?;
    assert_eq!(stale.as_database_error().and_then(|e| e.code()).as_deref(), Some("23514"));
    assert!(sqlx::query("UPDATE game_character_event_quest_progress SET completes=true").execute(&h.pool).await.is_err());
    Ok(())
}
