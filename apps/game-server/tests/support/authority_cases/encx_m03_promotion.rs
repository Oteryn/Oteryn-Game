//! M03: world admission reads the permanent promotion owner before quest effects.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Storage};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use std::sync::Arc;

#[test]
fn encx_m03_world_promotion_postgres_refusal_purchase_commit_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let h = f::Harness::create(admin, "encx_m03_promotion", true).await?;
                    let start = std::time::Instant::now();
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    eprintln!(
                        "M03 promotion real PostgreSQL refusal/purchase/commit/replay: {:?}",
                        start.elapsed()
                    );
                    result
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M03 PG worker panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    const QUEST: &str = "oteryn:quest.encx_promotion";
    const PHASE: &str = "oteryn:quest-progress/encx-promotion/phase";
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/l6/waterskin-of-mead")
        .ok_or("world sample")?;
    callback.key = "oteryn:quest-callback/encx-promotion".into();
    callback.quest = QUEST.into();
    callback.storage = vec![Storage {
        track: PHASE.into(),
        minimum: 0,
        maximum: 0,
        set: 1,
        preserve: false,
        at_least: false,
        increment: false,
    }];
    callback.effects = Default::default();
    callback.teleport = None;
    callback.lever = None;
    callback.reversible = false;
    callback.opening_only = false;
    callback.requires_world_admission = false;
    callback.passages.clear();
    callback.world_requirements.clear();
    callback.state_ops.clear();
    callback.world_change = None;
    callback.experience = None;
    callback.monk_reward = None;
    callback.offerings.clear();
    callback.item_on_target = None;
    callback.native_rule = None;
    callback.minimum_level = None;
    callback.admission = Some(world_callbacks::Admission {
        promotion: true,
        ..Default::default()
    });
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: PHASE.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![world_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
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
            },
        )
        .await
        .map_err(f::debug)?;
    let weights = super::engitem_reward_text::Weights(native);
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
    let invoke = |revision, command| -> f::TestResult<_> {
        Ok(h.root.commit_fixture_world_reward(
            &authority,
            &h.node,
            f::fence(revision)?,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Use(super::engitem_reward_text::command(command)?),
            },
            catalogue.clone(),
            &lane,
            context.clone(),
            admission,
            callback.clone(),
        ))
    };
    assert!(matches!(
        invoke(1, 21)?
            .await
            .map_err(|e| format!("unpromoted world callback: {e:?}"))?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT character_revision::text FROM game_character_progression_state"
        )
        .fetch_one(&h.pool)
        .await?,
        "1"
    );
    super::character_promotion_fixture::purchase(h, 1)
        .await
        .map_err(|e| format!("real promotion purchase: {e}"))?;
    assert!(matches!(
        invoke(2, 22)?
            .await
            .map_err(|e| format!("promoted world callback: {e:?}"))?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        invoke(3, 22)?
            .await
            .map_err(|e| format!("world callback replay: {e:?}"))?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT character_revision::text FROM game_character_progression_state"
        )
        .fetch_one(&h.pool)
        .await?,
        "3"
    );
    Ok(())
}
