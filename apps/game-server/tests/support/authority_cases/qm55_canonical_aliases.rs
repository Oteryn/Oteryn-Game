//! Canonical aliases inherit only the current counter cycle's completion.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTransition, shared_world_progress};

#[test]
fn qm55_deepling_helper_completes_canonical_quest_only_in_current_counter_cycle() -> f::TestResult {
    run(
        shared_world_progress::DEEPLING_QUEST,
        "oteryn:quest.deeplings_worldchange",
        "deeplings",
    )
}
#[test]
fn qm55_hive_helper_completes_canonical_quest_only_in_current_counter_cycle() -> f::TestResult {
    run(
        shared_world_progress::HIVE_QUEST,
        "oteryn:quest.authored.hive_born_world_change",
        "hive_born",
    )
}
fn run(helper: &'static str, canonical: &'static str, change: &'static str) -> f::TestResult {
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
                    let fixture = if change == "hive_born" {
                        "qm55_canonical_hive"
                    } else {
                        "qm55_canonical_deepling"
                    };
                    let h = f::Harness::create(admin, fixture, true).await?;
                    let result = exercise(&h, helper, canonical, change).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED canonical_{change} duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 alias worker failed")?
        .map_err(Into::into)
}
async fn exercise(
    h: &f::Harness,
    helper: &'static str,
    canonical: &'static str,
    change: &'static str,
) -> f::TestResult {
    let world = f::scope()?.world_id();
    for epoch in [1_i64, 2, 6] {
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
        .ok_or("current counter")?;
    let key = "oteryn:quest-transition/qm55/deepling-completion";
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![],
            vec![QuestTransition {
                key: key.into(),
                quest: helper.into(),
                effects: vec![],
                completes: true,
                experience: None,
                world_change: Some(change.into()),
            }],
        )
        .map_err(f::debug)?
        .with_quest(
            vec![],
            vec![QuestTransition {
                key: "oteryn:quest-transition/qm55/canonical-deepling-declaration".into(),
                quest: canonical.into(),
                effects: vec![],
                completes: true,
                experience: None,
                world_change: None,
            }],
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
        transition_key: key.into(),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(1).map_err(f::debug)?,
        )),
    };
    let result = h
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
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "{result:?}"
    );
    for epoch in [1_i64, 2] {
        if epoch == 2 {
            sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
            h.root
                .world_changes(world, 2)
                .await?
                .ok_or("same source cycle")?;
        }
        let copy = h
            .root
            .read_character_quest_state(&authority, f::fence(2)?.character_id)
            .await?;
        let progress = copy
            .counter_quest_progress(helper)
            .ok_or("trackless source-cycle completion was dropped")?;
        assert_eq!(progress.occurrence_generation, 1);
        assert!(progress.completed);
        assert!(progress.tracks.is_empty());
        assert!(copy.states()[helper].completed);
        assert!(
            copy.states()
                .get(canonical)
                .is_some_and(|state| state.completed),
            "helper must durably complete its canonical quest"
        );
        use oteryn_gameplay_quests::quest::log::QuestLogFacts;
        assert!(
            copy.quest_state(canonical)
                .ok_or("canonical log state")?
                .completed
        );
        assert!(copy.save_event_quest_progress(helper).is_none());
        let mut duplicate = request.clone();
        duplicate.cause = QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(u64::try_from(epoch)? + 1).map_err(f::debug)?,
        ));
        let duplicate = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(2)?,
                duplicate,
                catalogue.clone(),
            )
            .await?;
        assert!(
            matches!(duplicate, QuestTransitionOutcome::Refused(_)),
            "a fresh command must not complete twice in the source cycle: {duplicate:?}"
        );
        assert_eq!(
            h.root_revision().await?,
            "2",
            "refusal must not advance Character state"
        );
    }
    // The source owner, rather than the test, advances the cycle generation.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(jsonb_set(state,'{stage}','3'),'{entered_epoch}','1') WHERE world_id=encode($1,'hex')::uuid AND change_key=$2")
        .bind(world.as_bytes().as_slice()).bind(change).execute(&h.pool).await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=6 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    h.root
        .world_changes(world, 6)
        .await?
        .ok_or("next source cycle")?;
    let next = h
        .root
        .read_character_quest_state(&authority, f::fence(2)?.character_id)
        .await?;
    assert!(next.counter_quest_progress(helper).is_none());
    use oteryn_gameplay_quests::quest::log::QuestLogFacts;
    assert!(
        !next
            .quest_state(canonical)
            .ok_or("canonical log state after reset")?
            .completed
    );
    assert!(
        next.states()
            .get(canonical)
            .is_some_and(|state| state.completed),
        "permanent canonical history survives the source reset"
    );
    assert!(
        next.states()[helper].completed,
        "canonical history is retained"
    );
    let replay = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(2)?,
            request.clone(),
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
        "{replay:?}"
    );
    assert!(
        h.root
            .read_character_quest_state(&authority, f::fence(2)?.character_id)
            .await?
            .counter_quest_progress(helper)
            .is_none(),
        "replay cannot retag old completion"
    );
    // Completion must name this source owner, even when the canonical quest is
    // already complete and a changed definition would otherwise keep its pin.
    for (ordinal, world_change) in [
        (4, None),
        (
            5,
            Some(if change == "hive_born" {
                "deeplings"
            } else {
                "hive_born"
            }),
        ),
    ] {
        let invalid = std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![],
                vec![QuestTransition {
                    key: key.into(),
                    quest: helper.into(),
                    effects: vec![],
                    completes: true,
                    experience: None,
                    world_change: world_change.map(str::to_owned),
                }],
            )
            .map_err(f::debug)?,
        );
        let mut attempt = request.clone();
        attempt.cause = QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(ordinal).map_err(f::debug)?,
        ));
        let result = h
            .root
            .commit_character_quest_transition(&authority, &h.node, f::fence(2)?, attempt, invalid)
            .await?;
        assert!(
            matches!(result, QuestTransitionOutcome::Refused(_)),
            "unbound source completion: {result:?}"
        );
        assert_eq!(h.root_revision().await?, "2");
    }
    let mut fresh = request;
    fresh.cause = QuestCause::Command(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(6).map_err(f::debug)?,
    ));
    let result = h
        .root
        .commit_character_quest_transition(&authority, &h.node, f::fence(2)?, fresh, catalogue)
        .await?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "next source cycle must permit fresh completion: {result:?}"
    );
    let current = h
        .root
        .read_character_quest_state(&authority, f::fence(3)?.character_id)
        .await?;
    let progress = current
        .counter_quest_progress(helper)
        .ok_or("fresh cycle completion")?;
    assert_eq!(progress.occurrence_generation, 2);
    assert!(progress.completed);
    assert!(
        current
            .quest_state(canonical)
            .ok_or("new canonical completion")?
            .completed
    );
    let proofs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_quest_world_completions WHERE quest_key=$1")
            .bind(canonical)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        proofs, 1,
        "new cycles reuse permanent canonical completion history"
    );
    let generations: Vec<String> = sqlx::query_scalar("SELECT occurrence_generation::text FROM game_character_counter_quest_scopes ORDER BY committed_character_revision")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        generations,
        vec!["1", "2"],
        "refusals and replay must not create scope receipts"
    );
    Ok(())
}
