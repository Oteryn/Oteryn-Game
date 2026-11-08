//! Real Root qualification of the three-track seal planner. The immutable fixture
//! starts at the NPC's painting phase; it does not qualify map/tool admission.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, primary_transition};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, ice_islands_seals as seals};
use std::sync::Arc;

#[test]
fn engnpc2_ice_seals_postgres_distinct_atomic_completion_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_ice_seals", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("ENGNPC2_ICE_SEALS_PG_EXECUTED distinct_atomic_completion_and_replay duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "seal PG thread failed")?.map_err(Into::into)
}

fn callback(seal: u16) -> f::TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":format!("oteryn:quest-callback/engnpc2-fixture/seal-{seal}"),
        "quest":seals::QUEST,"trigger":"use","selector":{"position":{"x":1,"y":1,"floor":-1}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":seals::TRACKS.into_iter().zip([(8,8),(0,(1_i64<<48)-1),(-1,2)]).map(|(track,(minimum,maximum))| serde_json::json!({"track":track,"minimum":minimum,"maximum":maximum,"set":0,"increment":false,"preserve":true})).collect::<Vec<_>>(),
        "native_rule":{"kind":"ice_seal_paint","seal":seal},
        "offerings":[],"return_on_failure":false,"failure_message":null,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua:15-21"]
    }))?)
}

fn request(callback: &Callback, ordinal: u64) -> f::TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Use(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(ordinal).map_err(f::debug)?,
        )),
    })
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let callbacks = [callback(183)?, callback(2)?, callback(63)?];
    let tracks = seals::TRACKS
        .into_iter()
        .zip([(8, -1, 9), (0, 0, (1_i64 << 48) - 1), (1, -1, 2)])
        .map(|(key, (initial, min, max))| QuestTrack {
            key: key.into(),
            quest: seals::QUEST.into(),
            initial,
            min,
            max,
        })
        .collect();
    let transitions = callbacks
        .iter()
        .map(primary_transition)
        .collect::<Result<Vec<_>, _>>()
        .map_err(f::debug)?;
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, transitions).map_err(f::debug)?);
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
    let mut fence = f::fence(1)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let context = QuestWorldContext {
        map_revision: "seal-fixture-map".into(),
        content_revision: format!("sha256:{}", "01".repeat(32)),
        placement_context: vec![3; 32],
        reset_epoch: 0,
        actor_placement: f::id(7),
        movement_revision: 1,
    };
    // Native effects cannot be routed through the weaker generic writer.
    assert!(
        h.root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                fence,
                request(&callbacks[0], 9800)?,
                catalogue.clone(),
                None,
                callbacks[0].clone()
            )
            .await
            .is_err()
    );
    assert_eq!(h.root_revision().await?, "1");
    for (index, callback) in callbacks.iter().enumerate() {
        let ordinal = 9801 + u64::try_from(index)?;
        let outcome = h
            .root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                fence,
                request(callback, ordinal)?,
                catalogue.clone(),
                Some((&permit, context.clone())),
                callback.clone(),
            )
            .await
            .map_err(f::debug)?;
        let QuestTransitionOutcome::Committed(receipt) = outcome else {
            return Err(format!("seal commit: {outcome:?}").into());
        };
        let after = |track| {
            receipt
                .changes
                .iter()
                .find(|change| change.track == track)
                .map(|change| change.after)
        };
        assert_eq!(
            after(seals::QUESTLINE),
            Some(if index == 2 { 9 } else { 8 })
        );
        assert_eq!(
            after(seals::MISSION04),
            Some(if index == 2 { 2 } else { 1 })
        );
        let revision = h.root_revision().await?;
        // A fresh command cannot reuse the revision from before this commit.
        let stale = h
            .root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                fence,
                request(callback, 9950 + u64::try_from(index)?)?,
                catalogue.clone(),
                Some((&permit, context.clone())),
                callback.clone(),
            )
            .await;
        assert!(matches!(stale, Err(crate::durability::character_progression::CharacterProgressionError::CharacterRevisionMismatch)), "{stale:?}");
        fence = f::fence(u64::try_from(index)? + 2)?;
        let duplicate = h
            .root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                fence,
                request(callback, 9900 + u64::try_from(index)?)?,
                catalogue.clone(),
                Some((&permit, context.clone())),
                callback.clone(),
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(duplicate, QuestTransitionOutcome::Refused(_)),
            "{duplicate:?}"
        );
        assert_eq!(h.root_revision().await?, revision);
    }
    let replay = h
        .root
        .commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            request(&callbacks[0], 9801)?,
            catalogue,
            Some((&permit, context)),
            callbacks[0].clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
        "{replay:?}"
    );
    assert_eq!(h.root_revision().await?, "4");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 3);
    let packed: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(seals::TARGETS)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(packed, 3 | (64 << 16) | (184 << 32));
    Ok(())
}
