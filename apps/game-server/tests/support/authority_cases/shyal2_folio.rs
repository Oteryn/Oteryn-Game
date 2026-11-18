//! PostgreSQL qualification of the registered folio USE and its receipt.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, world_callbacks};
use std::sync::Arc;

const HEADACHE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_headache_until";
const OUTCOME: &str = "oteryn:quest-progress/shadows-of-yalahar/folio-outcome";
const STREAK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_books_success";

#[test]
fn shyal2_folio_use_commits_one_outcome_and_replays_its_quest_state() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "shyal2_folio", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    eprintln!(
                        "POSTGRES_CASE_COMPLETED shyal2_folio {:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "folio PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let callback = world_callbacks::load()
        .map_err(f::debug)?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/shadows-of-yalahar-folio")
        .ok_or("folio USE callback absent")?;
    let tracks = callback
        .storage
        .iter()
        .map(|entry| QuestTrack {
            key: entry.track.clone(),
            quest: callback.quest.clone(),
            initial: if entry.track == STREAK {
                7
            } else {
                entry.minimum.max(0)
            },
            min: entry.minimum,
            max: entry.maximum,
        })
        .collect();
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            tracks,
            vec![world_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice())
        .bind([1_u8;32].as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let fence = f::fence(1)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let context = QuestWorldContext {
        map_revision: "folio-use-fixture-map".into(),
        content_revision: format!("sha256:{}", "01".repeat(32)),
        placement_context: vec![3; 32],
        reset_epoch: 0,
        actor_placement: f::id(7),
        movement_revision: 1,
        party: Vec::new(),
    };
    let request = QuestTransitionRequest {
        transition_key: callback.key,
        cause: QuestCause::Use(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(8200).map_err(f::debug)?,
        )),
    };
    let invoke = || {
        h.root.commit_character_world_callback(
            &authority,
            &h.node,
            fence,
            request.clone(),
            catalogue.clone(),
            &permit,
            context.clone(),
            None,
        )
    };
    let now = 1_800_000_000;
    h.root.set_test_quest_clock(now).map_err(f::debug)?;
    let result = invoke().await.map_err(f::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = result else {
        return Err(format!("folio use: {result:?}").into());
    };
    let state = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(f::debug)?;
    let (streak, deadline) = match state.tracks().get(OUTCOME).copied() {
        Some(1) => (0, 0),
        Some(2..=5) => (0, now + 60),
        Some(6) => (8, 0),
        other => return Err(format!("invalid folio outcome: {other:?}").into()),
    };
    assert_eq!(state.tracks().get(STREAK), Some(&streak));
    assert_eq!(state.tracks().get(HEADACHE), Some(&deadline));
    let replay = invoke().await.map_err(f::debug)?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    assert_eq!(h.root_revision().await?, "2");
    Ok(())
}
