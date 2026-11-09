//! Exact live-target admission through the real Root: an empty-bag boundary
//! fixture and the registered Rotworm perfume BURN/MINT exchange.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::{
    QuestCarriedUse, QuestRewardAdmission, quest_creature::QuestCreatureUseProof,
};
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CharacterId, CharacterLease, CommandId,
    CommandRef, GameSessionId, MovementLocalPosition, NodeId, WorldId,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    reward_callbacks::{self, RewardCallback},
};
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::{num::NonZeroU32, sync::Arc};

#[test]
fn engnpc2_live_creature_postgres_exact_intent_burn_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_creature_intent", true).await?;
            let result = exercise(&h, false).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_live_creature_postgres_exact_intent_burn_and_replay duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "live target PG thread failed")?.map_err(Into::into)
}

#[test]
fn engnpc2_rotworm_perfume_postgres_production_exchange() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_rotworm_perfume", true).await?;
            let result = exercise(&h, true).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_rotworm_perfume_postgres_production_exchange duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "perfume PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &f::Harness, production: bool) -> f::TestResult {
    let key = if production {
        "oteryn:item.tibia.i17673"
    } else {
        "oteryn:item.tibia.i2854"
    };
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-reward/engnpc2-live-intent", "quest":"oteryn:quest.live_intent_fixture",
        "npc":"oteryn:npc.fixture", "keywords":["sample"],"topic":0,"next_topic":0,
        "carried_item":key,"creature_target":"oteryn:creature.rotworm",
        "storage":[{"track":"oteryn:quest-progress/engnpc2-live-intent","minimum":0,"maximum":0,"set":1}],
        "take":[{"item":key,"count":1}],"give":null,"outfit":null,"achievement":null,
        "reply":["Sample admitted."],"sources":["engine live-target fixture"]
    }))?;
    let callback = if production {
        reward_callbacks::load()
            .map_err(f::debug)?
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-reward/goblin-merchant/rotworm-sample")
            .ok_or("registered Rotworm callback")?
    } else {
        callback
    };
    // This immutable unregistered fixture owns a private track. Keep the global
    // bound-operation registry closed; the tested live proof and input participant
    // still run unchanged inside the real fenced Root transaction.
    let mut transition = reward_callbacks::primary_transition(&callback).map_err(f::debug)?;
    for effect in transition.effects.iter_mut().filter(|_| !production) {
        if let oteryn_gameplay_quests::quest::QuestEffectKind::SetWithBinding { value, .. } =
            effect.effect
        {
            effect.effect = oteryn_gameplay_quests::quest::QuestEffectKind::Set(value);
        }
    }
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: callback.storage[0].track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![transition],
        )
        .map_err(f::debug)?,
    );
    let world = WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?;
    let channel = ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?;
    let session = GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native content")?;
    let weights = super::engitem_reward_text::Weights(native);
    let index = native.item_index().ok_or("native item index")?;
    let revision = index.revision(key).ok_or("fixture stock revision")?;
    let definition = index
        .definition_ref("Item", key, revision)
        .ok_or("fixture stock reference")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let bag_key = "oteryn:item.tibia.i2854";
    let bag_revision = index.revision(bag_key).ok_or("bag revision")?;
    for seed in [91, 92] {
        let (stock_key, stock_revision) = if seed == 91 {
            (bag_key, bag_revision)
        } else {
            (key, revision)
        };
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(f::id(seed).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(stock_key).bind(stock_revision).bind(f::id(seed+10).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(91).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(f::id(111).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(91).as_slice()).bind(f::id(92).as_slice()).bind(f::id(f::WORLD).as_slice()).bind(f::id(112).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (0, 0, 0),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&f::id(1)).map_err(f::debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        12,
        channel_pin,
    )
    .map_err(f::debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(f::debug)?;
    let actor = runtime
        .commit_fresh_session(reservation)
        .map_err(f::debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(f::debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(&f::id(f::CHARACTER)).map_err(f::debug)?,
                1,
            )
            .map_err(f::debug)?,
        )
        .map_err(f::debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(f::debug)?
        .position();
    let creature = runtime
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x.checked_add(1).ok_or("position overflow")?,
                y: at.y,
                floor: at.floor,
            },
            "oteryn:creature.rotworm",
            100,
        )
        .map_err(f::debug)?;
    let generation = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .find(|entry| entry.actor == creature)
        .ok_or("visible creature")?
        .generation;
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: Some(EntityRef {
            identity: creature.placement_identity(),
            generation,
        }),
        use_with_map_item: None,
        use_with_position: None,
    };
    let fence = f::fence(1)?;
    let command = CommandRef::new(session, CommandId::new(10100).map_err(f::debug)?);
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(command),
    };
    let proof =
        QuestCreatureUseProof::read_current(&runtime, actor, fence, command, &callback, &intent)
            .ok_or("current creature proof")?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    // Same creature and same selected inventory UUID, but another carried-use subject.
    let mut changed = intent.clone();
    changed.target = UseTarget::ItemByDefinition(
        NonZeroU32::new(definition.get() + 1).ok_or("changed definition")?,
    );
    let bad = QuestCarriedUse {
        source: f::id(92),
        intent: encode_item_use_intent(&changed).map_err(f::debug)?,
    };
    let admission = |carried| QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: Some(carried),
        creature_use: Some(&proof),
        peer_use: None,
    };
    let refused = h
        .root
        .commit_fixture_quest_reward(
            &authority,
            &h.node,
            fence,
            request.clone(),
            catalogue.clone(),
            admission(&bad),
            callback.clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(refused, QuestTransitionOutcome::Refused(_)),
        "changed intent reused a target proof: {refused:?}"
    );
    assert_eq!(h.root_revision().await?, "1");
    assert_eq!(h.count("game_character_quest_receipts").await?, 0);
    let good = QuestCarriedUse {
        source: f::id(92),
        intent: encode_item_use_intent(&intent).map_err(f::debug)?,
    };
    let committed = h
        .root
        .commit_fixture_quest_reward(
            &authority,
            &h.node,
            fence,
            request.clone(),
            catalogue.clone(),
            admission(&good),
            callback.clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(committed, QuestTransitionOutcome::Committed(_)),
        "{committed:?}"
    );
    assert_eq!(h.root_revision().await?, "2");
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(f::id(92).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 0);
    if production {
        let output: (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i17674' AND lifecycle=1")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            output,
            (1, 1),
            "production callback must mint exactly one Rotworm-sampled gatherer"
        );
        assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);
        let atomic: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions c JOIN game_character_quest_receipts q USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_reward_claim_mint_receipts m ON m.xmin=q.xmin WHERE c.created_xact_id=q.created_xact_id")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            atomic, 1,
            "perfume BURN, MINT and progress share one transaction"
        );
    }
    drop(proof);
    runtime
        .remove_mount_taming_subject(creature, "oteryn:creature.rotworm")
        .map_err(f::debug)?;
    assert!(
        QuestCreatureUseProof::read_current(
            &runtime,
            actor,
            f::fence(2)?,
            command,
            &callback,
            &intent
        )
        .is_none()
    );
    let replay_admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: Some(&good),
        creature_use: None,
        peer_use: None,
    };
    let replay = h
        .root
        .commit_fixture_quest_reward(
            &authority,
            &h.node,
            f::fence(2)?,
            request,
            catalogue,
            replay_admission,
            callback,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
        "{replay:?}"
    );
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    if production {
        assert_eq!(
            h.count("game_reward_claim_mint_receipts").await?,
            1,
            "replay must not remint perfume"
        );
    }
    Ok(())
}
