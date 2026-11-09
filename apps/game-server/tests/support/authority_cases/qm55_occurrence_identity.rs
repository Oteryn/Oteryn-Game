//! The WorldCounter owner persists quest lifetimes independently of save epochs.
use crate::bestiary_postgres_harness as f;
use oteryn_gameplay_world_change::counter::CounterKind as K;

#[test]
fn qm55_counter_occurrence_identity_persists_across_saves_and_legacy_reload() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_occurrence_identity", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED occurrence_identity duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 occurrence worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let world = f::scope()?.world_id();
    for epoch in 1..=6_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    h.root
        .world_changes(world, 1)
        .await?
        .ok_or("initialized counters")?;
    // Retain old serialized rows: a rollout cannot require rewriting reset history.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(jsonb_set(state,'{stage}','3'),'{entered_epoch}','1')-'occurrence_generation' WHERE world_id=encode($1,'hex')::uuid AND change_key IN ('hive_born','deeplings')")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let original = h
        .root
        .world_changes(world, 1)
        .await?
        .ok_or("legacy counter projection")?;
    for kind in [K::HiveBorn, K::Deeplings] {
        assert_eq!(original.counter_quest_occurrences[kind.index()], 1);
    }
    assert_eq!(
        original.counter_quest_occurrences[K::DemonWars.index()],
        0,
        "unqualified lifecycles cannot authorize quest progress"
    );
    for epoch in 2..=6_i64 {
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
        let day = h
            .root
            .world_changes(world, epoch as u64)
            .await?
            .ok_or("next day")?;
        let expected = if epoch < 6 { 1 } else { 2 };
        for kind in [K::HiveBorn, K::Deeplings] {
            assert_eq!(day.counter_quest_occurrences[kind.index()], expected);
            assert_eq!(day.counter_generations[kind.index()], 1);
        }
        assert_eq!(h.root.world_changes(world, epoch as u64).await?, Some(day));
    }
    let persisted: Vec<(String, String)> = sqlx::query_as("SELECT change_key,state->>'occurrence_generation' FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key IN ('hive_born','deeplings') ORDER BY change_key LIMIT 3")
        .bind(world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        persisted,
        vec![
            ("deeplings".into(), "2".into()),
            ("hive_born".into(), "2".into())
        ]
    );
    assert!(h.root.world_changes(world, 5).await?.is_none());
    let rewritten: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND decision ? 'counter_quest_occurrences')")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        !rewritten,
        "live quest scopes must not rewrite immutable daily decisions"
    );
    Ok(())
}

#[test]
fn qm55_native_hive_receipt_stays_current_until_counter_cycle_reset() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_personal_cycle", true).await?;
                    let result = personal_cycle(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED personal_cycle duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 personal cycle worker failed")?
        .map_err(Into::into)
}

async fn personal_cycle(h: &f::Harness) -> f::TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef, GameSessionId};
    use oteryn_gameplay_quests::quest::shared_world_progress;
    let world = f::scope()?.world_id();
    for epoch in 1..=6_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    h.root
        .world_changes(world, 1)
        .await?
        .ok_or("initialized counters")?;
    let catalogue = std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(f::debug)?
            .catalogue()
            .clone(),
    );
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
        hive_access,
    };
    let mission_tracks: Vec<_> = shared_world_progress::BINDINGS
        .iter()
        .take(6)
        .map(|binding| binding.track)
        .chain(shared_world_progress::ACTIVE_TRACKS)
        .chain([shared_world_progress::HATCHLINGS_TRACK])
        .collect();
    let seed_key = "oteryn:quest-transition/qm55-prior-mission-progress";
    let objective_key = "oteryn:quest-transition/qm55-observe-mission";
    let seed_tracks: Vec<_> = mission_tracks
        .iter()
        .copied()
        .chain([hive_access::FAVOUR, hive_access::PURCHASED])
        .collect();
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            seed_tracks
                .iter()
                .map(|key| catalogue.track(key).cloned().ok_or("source mission track"))
                .collect::<Result<Vec<_>, _>>()?,
            vec![
                catalogue
                    .transition(&shared_world_progress::accept_key(1))
                    .cloned()
                    .ok_or("second native mission acceptance")?,
                catalogue
                    .transition(shared_world_progress::BINDINGS[0].key)
                    .cloned()
                    .ok_or("native turn-in")?,
                QuestTransition {
                    key: objective_key.into(),
                    quest: shared_world_progress::HIVE_QUEST.into(),
                    effects: vec![
                        QuestEffect {
                            track: shared_world_progress::BINDINGS[1].track.into(),
                            from: QuestComparison::Between(0, 6),
                            effect: QuestEffectKind::Add(1),
                        },
                        QuestEffect {
                            track: shared_world_progress::ACTIVE_TRACKS[1].into(),
                            from: QuestComparison::Between(1, 1),
                            effect: QuestEffectKind::Set(1),
                        },
                    ],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
                catalogue
                    .transition(&shared_world_progress::accept_key(0))
                    .cloned()
                    .ok_or("native acceptance")?,
                QuestTransition {
                    key: seed_key.into(),
                    quest: shared_world_progress::HIVE_QUEST.into(),
                    effects: seed_tracks
                        .iter()
                        .map(|key| QuestEffect {
                            track: (*key).into(),
                            from: QuestComparison::Between(0, 1),
                            effect: QuestEffectKind::Set(if *key == hive_access::FAVOUR {
                                49
                            } else if *key == shared_world_progress::BINDINGS[0].track {
                                8
                            } else {
                                1
                            }),
                        })
                        .collect(),
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            ],
        )
        .map_err(f::debug)?,
    );
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let request = QuestTransitionRequest {
        transition_key: shared_world_progress::accept_key(0),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(1).map_err(f::debug)?,
        )),
    };
    let outcome = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request.clone(),
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "native acceptance: {outcome:?}"
    );
    let character = f::fence(1)?.character_id;
    let copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await?;
    let progress = copy
        .counter_quest_progress(shared_world_progress::HIVE_QUEST)
        .ok_or("current cycle missing from admission")?;
    assert_eq!(progress.occurrence_generation, 1);
    assert!(
        copy.save_event_quest_progress(shared_world_progress::HIVE_QUEST)
            .is_none(),
        "counter cycles cannot fall back to save scope"
    );

    assert_eq!(
        progress.tracks.get(shared_world_progress::ACTIVE_TRACKS[0]),
        Some(&1)
    );
    if let QuestTransitionOutcome::Committed(receipt) = &outcome {
        let mut delta = copy.clone();
        assert!(delta.apply(receipt));
        assert!(
            delta
                .counter_quest_progress(shared_world_progress::HIVE_QUEST)
                .is_none(),
            "an unqualified delta cannot refresh cycle evidence"
        );
    }
    let seed = QuestTransitionRequest {
        transition_key: seed_key.into(),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(10).map_err(f::debug)?,
        )),
    };
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(2)?,
                seed,
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let history: (String, String) = sqlx::query_as("SELECT change_key,occurrence_generation::text FROM game_character_counter_quest_scopes LIMIT 2").fetch_one(&h.pool).await?;
    assert_eq!(history, ("hive_born".into(), "1".into()));
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(jsonb_set(state,'{stage}','3'),'{entered_epoch}','1') WHERE world_id=encode($1,'hex')::uuid AND change_key='hive_born'").bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    for epoch in 2..=6_i64 {
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
        let stale: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_current_counter_quest_progress")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            stale, 0,
            "a stale counter cannot authorize current progress"
        );
        let stale_copy = h
            .root
            .read_character_quest_state(&authority, character)
            .await?;
        assert!(
            stale_copy
                .counter_quest_progress(shared_world_progress::HIVE_QUEST)
                .is_none()
        );
        h.root
            .world_changes(world, epoch as u64)
            .await?
            .ok_or("next day")?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_current_counter_quest_progress")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            count,
            2 * i64::from(epoch < 6),
            "same source cycle survives saves; reset hides old personal progress"
        );
        let copy = h
            .root
            .read_character_quest_state(&authority, character)
            .await?;
        let progress = copy.counter_quest_progress(shared_world_progress::HIVE_QUEST);
        assert_eq!(
            progress.map(|progress| progress.occurrence_generation),
            (epoch < 6).then_some(1)
        );
        assert_eq!(
            copy.tracks().get(shared_world_progress::ACTIVE_TRACKS[0]),
            Some(&1),
            "cycle projection does not rewrite canonical receipt history"
        );
    }
    for (key, ordinal) in [
        (shared_world_progress::BINDINGS[0].key, 11),
        (objective_key, 12),
    ] {
        let stale_progress = QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
                CommandId::new(ordinal).map_err(f::debug)?,
            )),
        };
        let result = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(3)?,
                stale_progress,
                catalogue.clone(),
            )
            .await?;
        assert!(
            matches!(result, QuestTransitionOutcome::Refused(_)),
            "old-cycle action must require fresh acceptance: {key}: {result:?}"
        );
        assert_eq!(h.root_revision().await?, "3");
    }
    let restart_request = QuestTransitionRequest {
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(2).map_err(f::debug)?,
        )),
        ..request.clone()
    };
    let mut stale = f::fence(3)?;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                stale,
                restart_request.clone(),
                catalogue.clone()
            )
            .await
            .is_err()
    );
    assert_eq!(
        h.root_revision().await?,
        "3",
        "stale Character lease cannot reset mission progress"
    );
    let restarted = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(3)?,
            restart_request,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(restarted, QuestTransitionOutcome::Committed(_)),
        "new cycle must restart stale missions: {restarted:?}"
    );
    let restarted_copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await?;
    let progress = restarted_copy
        .counter_quest_progress(shared_world_progress::HIVE_QUEST)
        .ok_or("restarted cycle missing")?;
    assert_eq!(progress.occurrence_generation, 2);
    for track in &mission_tracks {
        let expected = i64::from(*track == shared_world_progress::ACTIVE_TRACKS[0]);
        assert_eq!(
            restarted_copy.tracks().get(*track),
            Some(&expected),
            "stale mission track {track}"
        );
        assert_eq!(progress.tracks.get(*track), Some(&expected));
    }
    assert_eq!(restarted_copy.tracks().get(hive_access::FAVOUR), Some(&49));
    assert_eq!(
        restarted_copy.tracks().get(hive_access::PURCHASED),
        Some(&1)
    );
    let duplicate = QuestTransitionRequest {
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(3).map_err(f::debug)?,
        )),
        ..request.clone()
    };
    assert!(
        matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    f::fence(4)?,
                    duplicate,
                    catalogue.clone()
                )
                .await?,
            QuestTransitionOutcome::Refused(_)
        ),
        "same-cycle acceptance cannot reset progress"
    );
    let second = QuestTransitionRequest {
        transition_key: shared_world_progress::accept_key(1),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(13).map_err(f::debug)?,
        )),
    };
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(4)?,
                second,
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let current_objective = QuestTransitionRequest {
        transition_key: objective_key.into(),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(14).map_err(f::debug)?,
        )),
    };
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(5)?,
                current_objective,
                catalogue.clone()
            )
            .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    let copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await?;
    assert_eq!(
        copy.counter_quest_progress(shared_world_progress::HIVE_QUEST)
            .ok_or("current objectives missing")?
            .tracks
            .get(shared_world_progress::BINDINGS[1].track),
        Some(&1)
    );
    let credits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_change_credit_receipts WHERE world_id=encode($1,'hex')::uuid AND source_kind=2").bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        credits, 0,
        "refused turn-in and objective-only progress must not credit the World"
    );
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(1)?,
                request,
                catalogue
            )
            .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_counter_quest_scopes")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count, 5,
        "replay must not rebind an old receipt to the new cycle"
    );
    let error =
        sqlx::query("UPDATE game_character_counter_quest_scopes SET occurrence_generation=2")
            .execute(&h.pool)
            .await
            .err()
            .ok_or("counter scope mutation unexpectedly succeeded")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    let error = sqlx::query("INSERT INTO game_character_counter_quest_scopes SELECT character_id,committed_character_revision,world_id,change_key,2 FROM game_character_counter_quest_scopes").execute(&h.pool).await.err().ok_or("counter scope mutation unexpectedly succeeded")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    Ok(())
}
//! The WorldCounter owner persists quest lifetimes independently of save epochs.
use crate::bestiary_postgres_harness as f;
use oteryn_gameplay_world_change::counter::CounterKind as K;

#[test]
fn qm55_counter_occurrence_identity_persists_across_saves_and_legacy_reload() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_occurrence_identity", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED occurrence_identity duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 occurrence worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let world = f::scope()?.world_id();
    for epoch in 1..=6_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    h.root
        .world_changes(world, 1)
        .await?
        .ok_or("initialized counters")?;
    // Retain old serialized rows: a rollout cannot require rewriting reset history.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(jsonb_set(state,'{stage}','3'),'{entered_epoch}','1')-'occurrence_generation' WHERE world_id=encode($1,'hex')::uuid AND change_key IN ('hive_born','deeplings')")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let original = h
        .root
        .world_changes(world, 1)
        .await?
        .ok_or("legacy counter projection")?;
    for kind in [K::HiveBorn, K::Deeplings] {
        assert_eq!(original.counter_quest_occurrences[kind.index()], 1);
    }
    assert_eq!(
        original.counter_quest_occurrences[K::DemonWars.index()],
        0,
        "unqualified lifecycles cannot authorize quest progress"
    );
    for epoch in 2..=6_i64 {
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
        let day = h
            .root
            .world_changes(world, epoch as u64)
            .await?
            .ok_or("next day")?;
        let expected = if epoch < 6 { 1 } else { 2 };
        for kind in [K::HiveBorn, K::Deeplings] {
            assert_eq!(day.counter_quest_occurrences[kind.index()], expected);
            assert_eq!(day.counter_generations[kind.index()], 1);
        }
        assert_eq!(h.root.world_changes(world, epoch as u64).await?, Some(day));
    }
    let persisted: Vec<(String, String)> = sqlx::query_as("SELECT change_key,state->>'occurrence_generation' FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key IN ('hive_born','deeplings') ORDER BY change_key LIMIT 3")
        .bind(world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        persisted,
        vec![
            ("deeplings".into(), "2".into()),
            ("hive_born".into(), "2".into())
        ]
    );
    assert!(h.root.world_changes(world, 5).await?.is_none());
    let rewritten: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND decision ? 'counter_quest_occurrences')")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        !rewritten,
        "live quest scopes must not rewrite immutable daily decisions"
    );
    Ok(())
}
