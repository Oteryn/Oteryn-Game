//! A fenced, preserved quest epoch scopes World MINTs; commands do not create cycles.
use super::engitem_reward_text::{Weights, command, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_gameplay_quests::quest::{
    reward_callbacks::{self, RewardCallback},
    world_callbacks::{self, Callback, Storage},
};
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.engitem_cycle";
const PHASE: &str = "oteryn:quest-progress/engitem-cycle/phase";
const CYCLE: &str = "oteryn:quest-progress/engitem-cycle/epoch";
const CLOCK: &str = "oteryn:quest-progress/engitem-cycle/restarted-at";

#[test]
fn engitem_world_cycle_mint_is_once_per_epoch_atomic_and_replay_safe() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "engitem_world_cycle", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGITEM_PG_EXECUTED world_cycle duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "World cycle PG thread failed")?
        .map_err(Into::into)
}

fn world() -> f::TestResult<Callback> {
    let effects = serde_json::json!({"item_reward":{"item":"oteryn:item.tibia.i3031","count":2,"contents":[],"additional":[{"item":"oteryn:item.tibia.i3035","count":1}]},"item_reward_cycle":CYCLE,
        "transform":null,"facing":null,"magic":null,"portals":null,"item_cycle":null,"encounter":null});
    // Parse only after the real PG harness exists, so RED executes the database case.
    let _: world_callbacks::CallbackEffects = serde_json::from_value(effects.clone())?;
    let mut c = world_callbacks::load()?
        .into_iter()
        .find(|c| c.effects.item_reward.is_some())
        .ok_or("World item fixture")?;
    c.key = "oteryn:quest-callback/engitem-cycle".into();
    c.quest = QUEST.into();
    c.storage = vec![
        Storage {
            add: None,
            read_only: false,
            set_if: None,
            track: PHASE.into(),
            minimum: 0,
            maximum: 0,
            set: 1,
            preserve: false,
            at_least: false,
            increment: false,
        },
        Storage {
            add: None,
            read_only: false,
            set_if: None,
            track: CYCLE.into(),
            minimum: 0,
            maximum: 2,
            set: 0,
            preserve: true,
            at_least: false,
            increment: false,
        },
    ];
    c.teleport = None;
    c.lever = None;
    c.reversible = false;
    c.passages.clear();
    c.world_change = None;
    c.experience = None;
    c.monk_reward = None;
    c.offerings.clear();
    c.item_on_target = None;
    c.return_on_failure = false;
    c.success_message = None;
    c.failure_message = None;
    c.native_rule = None;
    c.minimum_level = None;
    c.facing = None;
    c.effects = serde_json::from_value(effects)?;
    Ok(c)
}
fn npc(next: bool) -> f::TestResult<RewardCallback> {
    let mut value = serde_json::json!({"key":if next{"oteryn:quest-reward/engitem-next-cycle"}else{"oteryn:quest-reward/engitem-same-cycle"},
        "quest":QUEST,"npc":"oteryn:npc.fixture","keywords":["yes"],"topic":0,"next_topic":0,
        "storage":[{"track":PHASE,"minimum":if next{0}else{1},"maximum":1,"set":0}],
        "take":[],"give":null,"outfit":null,"achievement":null,"reply":["Restarted."],"sources":["engine fixture"]});
    if next {
        value["state_ops"] = serde_json::json!([
            {"track":CYCLE,"from":{"kind":"between","minimum":0,"maximum":1},"effect":{"kind":"add","value":1}},
            {"track":CLOCK,"from":{"kind":"any"},"effect":{"kind":"set_now"}}
        ]);
    }
    Ok(serde_json::from_value(value)?)
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let callback = world()?;
    let same = npc(false)?;
    let next = npc(true)?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![
                QuestTrack {
                    key: PHASE.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 1,
                },
                QuestTrack {
                    key: CYCLE.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 2,
                },
                QuestTrack {
                    key: CLOCK.into(),
                    quest: QUEST.into(),
                    initial: -1,
                    min: -1,
                    max: i64::MAX,
                },
            ],
            vec![
                world_callbacks::primary_transition(&callback).map_err(f::debug)?,
                reward_callbacks::primary_transition(&same).map_err(f::debug)?,
                reward_callbacks::primary_transition(&next).map_err(f::debug)?,
            ],
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
    let npc = |revision, id, callback: RewardCallback| -> f::TestResult<_> {
        Ok(h.root.commit_fixture_quest_reward(
            &authority,
            &h.node,
            f::fence(revision)?,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Command(command(id)?),
            },
            catalogue.clone(),
            admission,
            callback,
        ))
    };
    assert!(matches!(
        world(1, 9500)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    let empty:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(
        empty,
        (0, 0),
        "no backpack means no partial cycle/progress/MINT"
    );
    equip_backpack(h, &authority, native, &weights).await?;
    assert!(matches!(
        world(1, 9500)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let first: (String, String) = sqlx::query_as(
        "SELECT item_instance_id::text,claim_production_key FROM game_reward_claim_mint_receipts",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(
        first.1.starts_with("oteryn:quest-cycle/"),
        "World cycle identity must not be a command or permanent callback key"
    );
    assert!(matches!(
        world(1, 9500)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    // Reopening a phase does not create a new reward cycle.
    assert!(matches!(
        npc(2, 9600, same)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        world(3, 9501)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert!(matches!(
        npc(3, 9601, next.clone())?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        world(4, 9502)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(
        matches!(
            world(5, 9500)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "old command returns its original receipt after the cycle advances"
    );
    assert!(matches!(
        npc(5, 9603, next.clone())?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        world(6, 9503)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let before = h
        .root
        .read_character_quest_state(&authority, f::fence(7)?.character_id)
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(
            npc(7, 9604, next)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "an unadmitted epoch cannot create another reward cycle"
    );
    assert_eq!(
        before,
        h.root
            .read_character_quest_state(&authority, f::fence(7)?.character_id)
            .await
            .map_err(f::debug)?,
        "refused cycle admission leaves epoch, clock and phase unchanged"
    );
    let minted:(i64,i64,i64,i64)=sqlx::query_as("SELECT count(*),count(DISTINCT item_instance_id),count(DISTINCT claim_production_key),sum(quantity)::bigint FROM game_reward_claim_mint_receipts").fetch_one(&h.pool).await?;
    assert_eq!(minted, (3, 3, 3, 6));
    let siblings:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts r CROSS JOIN LATERAL jsonb_array_elements(r.additional_plan) p JOIN game_item_instances i ON i.item_instance_id=(p->>'item_instance_id')::uuid WHERE p->>'production_key'='oteryn:item.tibia.i3035' AND i.quantity=1 AND i.lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(
        siblings, 3,
        "each admitted epoch has exactly one independent sibling; replay cannot duplicate it"
    );

    let atomic:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.created_xact_id=m.created_xact_id WHERE q.transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 3,
        "each cycle's MINT and progress share the same transaction"
    );
    let original:(String,String)=sqlx::query_as("SELECT item_instance_id::text,claim_production_key FROM game_reward_claim_mint_receipts WHERE command_id=9500").fetch_one(&h.pool).await?;
    assert_eq!(
        first, original,
        "later cycles never replace the original cycle's root"
    );
    Ok(())
}
