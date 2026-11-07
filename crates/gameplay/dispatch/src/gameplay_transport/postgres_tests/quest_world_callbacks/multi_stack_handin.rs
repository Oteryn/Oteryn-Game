//! Shared carried-resource composition, using the source's three-tear callback.
//! This proves the mechanism, not qualification of the Gravedigger quest.
use super::*;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};

#[test]
fn multi_stack_handin_atomic_shortfall_partial_stack_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::super::kill_reward_e2e::scenario_thread::run("multi-stack-handin", || {
        scenario(
            "gravedigger-tears",
            Position {
                x: 100,
                y: 100,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED multi_stack_handin duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    // Only the prerequisite is fixture-built. Consumption and its actual source
    // successor always run through literal USE and the production Quest writer.
    let original = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let prepare = QuestTransition {
        key: "oteryn:quest-transition/fixture/multi-stack-prerequisite".into(),
        quest: callback.quest.clone(),
        effects: vec![QuestEffect {
            track: callback.storage[0].track.clone(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(callback.storage[0].minimum),
        }],
        completes: false,
        experience: None,
    };
    let tracks = callback
        .storage
        .iter()
        .map(|storage| original.track(&storage.track).cloned().ok_or("track"))
        .collect::<Result<Vec<_>, _>>()?;
    let transition = original
        .transition(&callback.key)
        .cloned()
        .ok_or("transition")?;
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new("content-1", tracks, vec![prepare.clone(), transition])
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: prepare.key,
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(900).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    drop(slot);
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let required = callback.item_on_target.as_ref().ok_or("required item")?;
    assert_eq!(required.count, 3); // actions_tears.lua:1-16, pinned by the callback.
    let placement_key =
        oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
            .ok_or("placement")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&placement_key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    // Two units cannot satisfy three. Refusal must leave the stack live,
    // with no consumption proof or completion receipt.
    input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: trigger,
            item: required.item.clone(),
            count: 2,
        },
        60,
        true,
    )
    .await?;
    let refused = wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
        UseDisposition::Rejected
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    let completion_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completion_receipts, 0);
    // Four units in two stacks: exactly three are consumed, with one partial
    // survivor, in stable Item order.
    input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: trigger,
            item: required.item.clone(),
            count: 2,
        },
        61,
        true,
    )
    .await?;
    // Pickup autostacks. Use the real split owner to establish two separate
    // carried stacks before exercising the hand-in, without changing Item rows.
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(c.item) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 AND i.quantity=4 LIMIT 1")
        .bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(&required.item).fetch_one(&h.pool).await?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let split = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(980).map_err(debug)?,
            ),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            crate::durability::item_loop_move::LoopMoveRequest {
                item: source.try_into().map_err(|_| "source UUID")?,
                count: Some(2),
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::MainBackpack,
            },
        )
        .await
        .map_err(debug)?;
    drop(permit);
    assert!(split.is_ok(), "split: {split:?}");
    let committed = wire(owner, admitted, 2, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&committed).map_err(debug)?,
        UseDisposition::Committed
    );
    let (consumed, proofs): (i64, i64) = sqlx::query_as("SELECT coalesce(sum(quantity),0)::bigint,count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1")
        .bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(consumed, 3);
    assert_eq!(proofs, 2);
    let remaining: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3")
        .bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(&required.item).fetch_one(&h.pool).await?;
    assert_eq!(remaining, 1);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.storage[1].track), Some(&1));
    let replay = wire(owner, admitted, 2, 2, payload).await?;
    assert_eq!(replay, committed);
    let consumed_after: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1")
        .bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(consumed_after, 3);
    // Re-enter the durable owner with its original world binding, after the
    // stacks have changed. The generic NPC receipt reader has another binding.
    let proof: Vec<u8> = sqlx::query_scalar(
        "SELECT proof FROM game_quest_callback_consumptions WHERE transition_key=$1 \
         ORDER BY item_instance_id LIMIT 1",
    )
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    let proof: serde_json::Value = serde_json::from_slice(&proof)?;
    let context = serde_json::from_value(proof["context"].clone())?;
    let permit = owner.spell_lane_permit().await.ok_or("replay permit")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let reconciled = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Use(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(2).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("replay catalogue")?,
            &permit,
            context,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        reconciled,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    Ok(())
}
