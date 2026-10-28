//! The three admitted action classes exercise literal production USE commands.
use super::*;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};

#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &Callback,
    trigger: Position,
    origin: MovementLocalPosition,
) -> TestResult {
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Callback", owner.owner_now());
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let reply = wire(owner, admitted, 1, 2, payload.clone())
        .await
        .map_err(|e| format!("{} first refusal: {e}", callback.key))?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );
    if let Some(message) = &callback.failure_message {
        assert!(owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
            matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == message)));
    }
    let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions), (SELECT count(*) FROM game_character_quest_receipts)")
        .fetch_one(&h.pool).await?;
    assert_eq!(counts, (0, 0));
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let track = &callback.storage[0];
    let steps: &[&str] = if callback.item_on_target.is_some() {
        &["npc_3"]
    } else {
        &["npc_1", "npc_6", "npc_6"]
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    for (n, step) in steps.iter().enumerate() {
        let transition_key = format!(
            "oteryn:quest-transition/{}/{step}",
            track
                .track
                .strip_prefix("oteryn:quest-progress/")
                .ok_or("track prefix")?
        );
        let seeded = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: transition_key.clone(),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(90 + u64::try_from(n)?).map_err(debug)?,
                    )),
                },
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(seeded, QuestTransitionOutcome::Committed(_)),
            "{transition_key:?}: {seeded:?}"
        );
    }
    drop(slot);
    if callback.item_on_target.is_some() {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(matches!(
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(99).map_err(debug)?
                    ))
                },
                catalogue.clone()
            )
            .await,
            Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
        ));
        drop(slot);
        // A valid stage with no key still refuses without a receipt, consumption or teleport.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let reply = wire(owner, admitted, 2, 2, payload.clone()).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Rejected
        );
        let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1)")
            .bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(counts, (0, 0));
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position(),
            origin
        );
        let state = h
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(&track.track), Some(&track.minimum));
        let item = callback.item_on_target.as_ref().ok_or("item")?;
        input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: trigger,
                item: item.item.clone(),
                count: item.count,
            },
            60,
            true,
        )
        .await?;
        if item.target_replacement.is_some() {
            nest_key(owner, h, fence.item_fence(), trigger, &item.item).await?;
        }
    }
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let next = if callback.item_on_target.is_some() {
        3
    } else {
        2
    };
    let reply = wire(owner, admitted, next, 2, payload)
        .await
        .map_err(|e| format!("{} success: {e}", callback.key))?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    {
        use crate::map::view::MapFacts;
        let facts = &owner.bundle_map().ok_or("map")?.facts;
        assert_eq!(facts.object_revision(key), 1);
        if let Some(appearance) = callback
            .item_on_target
            .as_ref()
            .and_then(|i| i.target_replacement)
            .or_else(|| callback.lever.map(|p| p[1]))
        {
            assert_eq!(
                facts
                    .base_entry(
                        crate::map::overlay::TilePos {
                            x: trigger.x,
                            y: trigger.y,
                            floor: trigger.floor
                        },
                        1,
                        1
                    )
                    .ok_or("after")?
                    .appearance_id,
                appearance
            );
        }
    }
    let copy = h
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(copy.tracks().get(&track.track), Some(&track.set));
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        callback.teleport.map(local).unwrap_or(origin)
    );
    if let Some(item) = &callback.item_on_target {
        let proof: (i64,i64,bool) = sqlx::query_as("SELECT i.quantity,i.lifecycle::bigint,sha256(c.proof)=c.proof_sha256 AND c.before_location IS NOT NULL FROM game_quest_callback_consumptions c JOIN game_item_instances i USING(item_instance_id) WHERE c.transition_key=$1")
            .bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(proof, (0, 2, true));
        let kind: String = sqlx::query_scalar(
            "SELECT location_kind FROM game_quest_callback_consumptions WHERE transition_key=$1",
        )
        .bind(&callback.key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            kind,
            if item.target_replacement.is_some() {
                "nested"
            } else {
                "container"
            }
        );
        // The same database after-proof admits exactly a one-for-one identity-preserving
        // replacement and rejects quantity or unrelated-field drift.
        let replacement: (bool, bool, bool) = sqlx::query_as(
            "WITH evidence AS (SELECT to_jsonb(c)||jsonb_build_object('replacement','oteryn:item.tibia.i3114') AS receipt, \
             c.before_item||jsonb_build_object('definition_production_key','oteryn:item.tibia.i3114', \
             'last_transaction_id',c.transaction_id) AS after FROM game_quest_callback_consumptions c \
             WHERE c.transition_key=$1 LIMIT 1), typed AS (SELECT \
             jsonb_populate_record(NULL::game_quest_callback_consumptions,receipt) AS r,after FROM evidence) \
             SELECT game_quest_callback_after(r,jsonb_populate_record(NULL::game_item_instances,after)), \
             game_quest_callback_after(r,jsonb_populate_record(NULL::game_item_instances,after||'{\"quantity\":2}'::jsonb)), \
             game_quest_callback_after(r,jsonb_populate_record(NULL::game_item_instances,after||'{\"definition_revision_ref\":\"wrong\"}'::jsonb)) FROM typed")
            .bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(replacement, (true, false, false));
        let carried: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON c.item=i.item_instance_id WHERE i.definition_production_key=$3")
            .bind(fence.character_id.as_bytes().as_slice()).bind(id(42).as_slice()).bind(&item.item).fetch_one(&h.pool).await?;
        assert_eq!(carried, 0);
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT proof FROM game_quest_callback_consumptions WHERE transition_key=$1",
        )
        .bind(&callback.key)
        .fetch_one(&h.pool)
        .await?;
        let proof: serde_json::Value = serde_json::from_slice(&bytes)?;
        let context = serde_json::from_value(proof["context"].clone())?;
        let permit = owner.spell_lane_permit().await.ok_or("lane")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        assert!(matches!(
            slot.commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(3).map_err(debug)?
                    ))
                },
                catalogue.clone(),
                &permit,
                context
            )
            .await
            .map_err(debug)?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);
    }
    Ok(())
}

#[test]
fn quest_world_callbacks_qcb3_real_dispatch_actions() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quest-qcb3-actions", || {
        for name in [
            "outlaw-camp-door",
            "travelling-trader-delivery",
            "fish-napping-key",
        ] {
            scenario(
                name,
                Position {
                    x: 32358,
                    y: 32788,
                    floor: -6,
                },
                false,
            )?;
        }
        Ok(())
    })
}

#[test]
fn quest_world_callbacks_qcb3_target_swap_with_consumption_and_teleport() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quest-qcb3-target-swap", || {
        scenario(
            "fish-napping-key-target-swap",
            Position {
                x: 32358,
                y: 32788,
                floor: -6,
            },
            false,
        )
    })
}

async fn nest_key(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    trigger: Position,
    key: &str,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    input_item(
        owner,
        fence,
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: trigger,
            item: "oteryn:item.tibia.i2854".into(),
            count: 1,
        },
        61,
        true,
    )
    .await?;
    let find = |definition: String| async move {
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(c.item) FROM game_quest_callback_carried(encode($1,'hex')::uuid, \
             encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item \
             WHERE c.kind='container' AND i.definition_production_key=$3 LIMIT 1",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(id(42).as_slice())
        .bind(definition)
        .fetch_one(&h.pool)
        .await?;
        TestResult::<[u8; 16]>::Ok(bytes.try_into().map_err(|_| "item id")?)
    };
    let item = find(key.into()).await?;
    let bag = find("oteryn:item.tibia.i2854".into()).await?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            crate::foundation::CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(1700).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Container(bag),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "nest: {moved:?}");
    Ok(())
}
