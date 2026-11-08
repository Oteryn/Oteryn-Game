//! M19: a World reward and its SQL-clock epoch commit together.
use super::engitem_reward_text::{Weights, command, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, world_callbacks};
use std::sync::Arc;
const EPOCH: i64 = 1_700_000_000;
const WAIT: i64 = 60 * 30 * 1000;

#[test]
fn qready1_raven_herb_clock_reward_is_atomic_once_per_interval_and_replay_safe() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "qready1_clock_reward", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "QREADY1_M19_PG_EXECUTED raven_reward duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M19 World clock reward PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/qready1-threatened-raven-herb")
        .ok_or("production Raven Herb callback")?;
    let clock = callback
        .state_ops
        .first()
        .ok_or("production clock")?
        .track
        .clone();
    assert!(callback.storage.is_empty());
    assert!(callback.clock_reward_epoch(&clock));
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: clock.clone(),
                quest: callback.quest.clone(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            }],
            vec![world_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    h.root.set_test_quest_clock(EPOCH)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    assert_eq!(
        native.source_digest(),
        pin.identity().server_artifact_digest()
    );
    // Standard control-fixture grant; the real activation writer still enforces it.
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: f::scope()?.world_id(),
                    channel_id: crate::foundation::ChannelId::decode(&f::id(f::CHANNEL))
                        .map_err(f::debug)?,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: native.source_digest(),
                    client_artifact_digest: pin.identity().client_artifact_digest(),
                    frame_binding_digest: pin.frame_binding().digest(),
                }
            )
            .await
            .map_err(f::debug)?
    );
    let weights = Weights(native);
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let context = crate::durability::quest_world_callback::QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: pin.frame_binding().digest().to_vec(),
        reset_epoch: 0,
        actor_placement: f::id(17),
        movement_revision: 1,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let lane = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let world = |revision, id| -> f::TestResult<_> {
        Ok(h.root.commit_fixture_world_reward(
            &authority,
            &h.node,
            f::fence(revision)?,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Use(command(id)?),
            },
            catalogue.clone(),
            &lane,
            context.clone(),
            admission,
            callback.clone(),
        ))
    };
    assert!(matches!(
        world(1, 9800)?.await?,
        QuestTransitionOutcome::Refused(_)
    ));
    let empty: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1)")
        .bind(&clock).fetch_one(&h.pool).await?;
    assert_eq!(
        empty,
        (0, 0, 0),
        "no backpack: no clock, quest receipt or reward"
    );
    equip_backpack(h, &authority, native, &weights).await?;
    let first = world(1, 9800)?.await?;
    assert!(
        matches!(first, QuestTransitionOutcome::Committed(_)),
        "{first:?}"
    );
    let first_key: String =
        sqlx::query_scalar("SELECT claim_production_key FROM game_reward_claim_mint_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert!(first_key.starts_with("oteryn:quest-cycle/"));
    assert!(matches!(
        world(2, 9801)?.await?,
        QuestTransitionOutcome::Refused(_)
    ));
    h.root.advance_test_quest_clock(WAIT - 1)?;
    assert!(matches!(
        world(2, 9802)?.await?,
        QuestTransitionOutcome::Refused(_)
    ));
    let stored: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(&clock)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stored, EPOCH, "early attempts cannot advance the clock");
    h.root.advance_test_quest_clock(1)?;
    assert!(
        world(1, 9803)?.await.is_err(),
        "a stale character revision cannot claim"
    );
    let second = world(2, 9803)?.await?;
    assert!(
        matches!(second, QuestTransitionOutcome::Committed(_)),
        "{second:?}"
    );
    assert!(matches!(
        world(3, 9800)?.await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let totals: (i64, i64, i64) = sqlx::query_as("SELECT count(*),count(DISTINCT m.claim_production_key),sum(m.quantity)::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i5953'").fetch_one(&h.pool).await?;
    assert_eq!(totals, (2, 2, 2));
    let atomic: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.created_xact_id=m.created_xact_id WHERE q.transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 2,
        "each grant and its clock receipt share the SQL transaction"
    );
    let original: String = sqlx::query_scalar(
        "SELECT claim_production_key FROM game_reward_claim_mint_receipts WHERE command_id=9800",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(original, first_key);
    Ok(())
}
