//! M02 reads real ordered Ground custody in the same fenced quest transaction.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, primary_transition};
use oteryn_gameplay_quests::quest::{QuestEffectKind, QuestStateCatalogue, QuestTrack};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.ground_fixture";
const TRACK: &str = "oteryn:quest-progress/ground_fixture/stage";
const KEY: &str = "oteryn:quest-transition/ground_fixture/window";
const CRATE: &str = "oteryn:item.fixture_crate";
const AMPHORA: &str = "oteryn:item.fixture_amphora";

#[test]
fn engnpc2_m02_ground_stack_is_current_complete_locked_and_replay_bound() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "engnpc2_m02_ground", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGNPC2_M02_PG_EXECUTED ground_stack duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M02 PG thread failed")?
        .map_err(Into::into)
}

fn callback() -> f::TestResult<Callback> {
    let document = serde_json::json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":QUEST,"trigger":"use","selector":{"position":{"x":1,"y":1,"floor":-1}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1,"increment":false}],
        "offerings":[],"return_on_failure":false,"failure_message":null,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13"],
        "requires_world_admission":true,"world_requirements":[{"position":{"x":2,"y":1,"floor":-1},"appearance":1739,"ground_stack":[
            {"item":CRATE,"count":1},
            {"item":CRATE,"count":1},
            {"item":AMPHORA,"count":1}]}]
    }]});
    Ok(serde_json::from_value(document["callbacks"][0].clone())?)
}

async fn seed(h: &f::Harness, correct: bool, extra: bool) -> f::TestResult {
    // Historical fixture custody only. All triggers remain enabled for every
    // tested quest call; no quest progress or predicate receipts are preseeded.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_item_ground_locations")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_item_instances")
        .execute(&mut *tx)
        .await?;
    let order = if correct {
        [CRATE, CRATE, AMPHORA]
    } else {
        [CRATE, AMPHORA, CRATE]
    };
    let mut spatial = 2_i32.to_be_bytes().to_vec();
    spatial.extend_from_slice(&1_i32.to_be_bytes());
    spatial.extend_from_slice(&(-1_i16).to_be_bytes());
    for (index, key) in order.into_iter().chain(extra.then_some(CRATE)).enumerate() {
        let id = f::id(91 + u8::try_from(index)?);
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
            .bind(id.as_slice()).bind(f::id(f::WORLD).as_slice()).bind(key).bind(f::id(120).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,scope_kind,channel_id,instance_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($3,'hex')::uuid,NULL,1,$4,$5,'ground-fixture-map',$6,$7,$8)")
            .bind(id.as_slice()).bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice())
            .bind(&spatial).bind(b"OTERYN_ITEM_LOOP_DROP/v1:fixture".as_slice())
            .bind(format!("sha256:{}", "01".repeat(32))).bind([3_u8;32].as_slice())
            .bind(i64::try_from(index + 1)? * 11).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn no_write(h: &f::Harness) -> f::TestResult {
    assert_eq!(h.root_revision().await?, "1");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let callback = callback()?;
    // The resolved fixture source owns its exact bound operation. The physical
    // Ground checks below execute only after that operation has qualified.
    let transition = primary_transition(&callback).map_err(f::debug)?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![transition],
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
        map_revision: "ground-fixture-map".into(),
        content_revision: format!("sha256:{}", "01".repeat(32)),
        placement_context: vec![3; 32],
        reset_epoch: 0,
        actor_placement: f::id(7),
        movement_revision: 1,
    };
    let request = |id| -> f::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: KEY.into(),
            cause: QuestCause::Use(CommandRef::new(
                GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
                CommandId::new(id).map_err(f::debug)?,
            )),
        })
    };
    seed(h, false, false).await?;
    let refused = h
        .root
        .commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            request(9700)?,
            catalogue.clone(),
            Some((&permit, context.clone())),
            callback.clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(refused, QuestTransitionOutcome::Refused(_)),
        "{refused:?}"
    );
    no_write(h).await?;
    // A weaker generic context cannot execute a Ground-bound progress effect.
    assert!(
        h.root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                fence,
                request(9701)?,
                catalogue.clone(),
                None,
                callback.clone()
            )
            .await
            .is_err()
    );
    no_write(h).await?;
    seed(h, true, true).await?;
    let refused = h
        .root
        .commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            request(9702)?,
            catalogue.clone(),
            Some((&permit, context.clone())),
            callback.clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(refused, QuestTransitionOutcome::Refused(_)),
        "{refused:?}"
    );
    no_write(h).await?;
    seed(h, true, false).await?;
    // Keep every physical prerequisite valid while stripping only the binding.
    let mut stripped = catalogue.transition(KEY).ok_or("bound operation")?.clone();
    let mut changed = false;
    for effect in &mut stripped.effects {
        if let QuestEffectKind::SetWithBinding { value, .. } = effect.effect {
            effect.effect = QuestEffectKind::Set(value);
            changed = true;
        }
    }
    assert!(changed, "fixture must exercise a genuinely bound operation");
    let stripped = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![catalogue.track(TRACK).ok_or("bound track")?.clone()],
            vec![stripped],
        )
        .map_err(f::debug)?,
    );
    let refused = h
        .root
        .commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            request(9707)?,
            stripped,
            Some((&permit, context.clone())),
            callback.clone(),
        )
        .await;
    assert!(matches!(
        refused,
        Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
    ));
    no_write(h).await?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_fixture_ground_callback(
                &authority,
                &h.node,
                stale,
                request(9703)?,
                catalogue.clone(),
                Some((&permit, context.clone())),
                callback.clone()
            )
            .await
            .is_err()
    );
    // A competing Ground mutation owns the same lock; refusal must not wait.
    let mut blocked = h.pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||encode($2,'hex'),33))",
    )
    .bind(f::id(f::WORLD).as_slice())
    .bind(f::id(f::CHANNEL).as_slice())
    .execute(&mut *blocked)
    .await?;
    let refused = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        h.root.commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            request(9704)?,
            catalogue.clone(),
            Some((&permit, context.clone())),
            callback.clone(),
        ),
    )
    .await?
    .map_err(f::debug)?;
    assert!(
        matches!(refused, QuestTransitionOutcome::Refused(_)),
        "{refused:?}"
    );
    blocked.rollback().await?;
    no_write(h).await?;
    // An Item locked before its Ground mutation cannot form a deadlock cycle.
    let mut blocked = h.pool.begin().await?;
    sqlx::query("SELECT item_instance_id FROM game_item_instances ORDER BY item_instance_id LIMIT 1 FOR UPDATE").fetch_all(&mut *blocked).await?;
    let refused = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        h.root.commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            request(9705)?,
            catalogue.clone(),
            Some((&permit, context.clone())),
            callback.clone(),
        ),
    )
    .await?;
    assert!(refused.is_err());
    blocked.rollback().await?;
    no_write(h).await?;
    let good = request(9706)?;
    let committed = h
        .root
        .commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            good.clone(),
            catalogue.clone(),
            Some((&permit, context.clone())),
            callback.clone(),
        )
        .await
        .map_err(f::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = committed else {
        return Err(format!("ordered Ground commit: {committed:?}").into());
    };
    assert_eq!(receipt.changes.len(), 1);
    assert_eq!(receipt.changes[0].after, 1);
    assert_eq!(h.root_revision().await?, "2");
    // The recorded command replays after the physical arrangement changes.
    seed(h, false, false).await?;
    let replay = h
        .root
        .commit_fixture_ground_callback(
            &authority,
            &h.node,
            fence,
            good,
            catalogue,
            Some((&permit, context)),
            callback,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.root_revision().await?, "2");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1);
    Ok(())
}
