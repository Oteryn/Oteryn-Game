//! Atomic corpse quest participant. Fixture stock is historical; corpse MINT,
//! race provenance, selected tool BURN, reward and quest progress use real owners.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::{
    QuestCarriedUse, QuestRewardAdmission, quest_creature::QuestCreatureUseProof,
};
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CharacterId, CharacterLease, CommandId,
    CommandRef, GameSessionId, NodeId, WorldId,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    reward_callbacks::{self, RewardCallback},
};
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};
use std::{num::NonZeroU32, sync::Arc};

#[test]
fn engnpc2_corpse_sample_postgres_atomic_burn_mint_progress_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_corpse_commit", true).await?;
            let result = exercise(&h, false).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_corpse_sample_postgres_atomic_burn_mint_progress_and_replay duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "live target PG thread failed")?.map_err(Into::into)
}

#[test]
fn engnpc2_slug_sample_postgres_production_burn_mint_progress_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_slug_sample", true).await?;
            let result = exercise(&h, true).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_slug_sample_postgres_production_burn_mint_progress_and_replay duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "Slug sample PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &f::Harness, production: bool) -> f::TestResult {
    let key = if production {
        "oteryn:item.tibia.i17670"
    } else {
        "oteryn:item.tibia.i2854"
    };
    let callback: RewardCallback = if production {
        reward_callbacks::load()
            .map_err(f::debug)?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/goblin-merchant/slug-sample")
            .ok_or("registered Slug sample")?
    } else {
        serde_json::from_value(serde_json::json!({
            "key":"oteryn:quest-reward/engnpc2-corpse-sample", "quest":"oteryn:quest.corpse_sample_fixture",
            "npc":"oteryn:npc.fixture", "keywords":["sample"],"topic":0,"next_topic":0,
            "carried_item":key,"creature_target":"oteryn:creature.rotworm","fresh_corpse":true,
            "storage":[{"track":"oteryn:quest-progress/engnpc2-corpse-sample","minimum":0,"maximum":0,"set":1}],
            "take":[{"item":key,"count":1}],"give":{"item":"oteryn:item.tibia.i2814","count":1,"contents":[]},"outfit":null,"achievement":null,
            "reply":["Sample admitted."],"sources":["engine live-target fixture"]
        }))?
    };
    // Only the unregistered engine fixture uses an unbound private track. The
    // production Slug callback retains its exact global bound operation.
    let mut transition = reward_callbacks::primary_transition(&callback).map_err(f::debug)?;
    for effect in transition.effects.iter_mut().filter(|_| !production) {
        if let oteryn_gameplay_quests::quest::QuestEffectKind::SetWithBinding { value, .. } =
            effect.effect
        {
            effect.effect = oteryn_gameplay_quests::quest::QuestEffectKind::Set(value);
        }
    }
    let catalogue = if production {
        Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(f::debug)?
                .catalogue()
                .clone(),
        )
    } else {
        Arc::new(
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
        )
    };
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
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let (operand, corpse_at) = mint_corpse(
        h,
        &authority,
        native,
        callback.creature_target.as_deref().ok_or("corpse race")?,
    )
    .await?;
    let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
        f::fence(1)?.runtime_scope,
    )
    .await?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        corpse_at,
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
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(
            std::num::NonZeroU64::new(77).ok_or("resolved corpse handle")?,
        )),
        use_with_position: None,
    };
    let fence = f::fence(1)?;
    let command = CommandRef::new(session, CommandId::new(10100).map_err(f::debug)?);
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(command),
    };
    let proof = QuestCreatureUseProof::read_corpse(
        &runtime, actor, fence, command, &callback, &intent, operand, &permit,
    )
    .ok_or("current corpse proof")?;
    // Same creature and same selected inventory UUID, but another carried-use subject.
    let mut changed = intent.clone();
    changed.target = UseTarget::ItemByDefinition(
        NonZeroU32::new(definition.get() + 1).ok_or("changed definition")?,
    );
    let bad = QuestCarriedUse { recipe_target: None,
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
    let good = QuestCarriedUse { recipe_target: None,
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
    {
        let output: (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(&callback.give.as_ref().ok_or("sample output")?.item).fetch_one(&h.pool).await?;
        assert_eq!(
            output,
            (1, 1),
            "corpse sample must mint exactly one qualified output"
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
    drop(runtime);
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
            callback.clone(),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
        "{replay:?}"
    );
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    {
        assert_eq!(
            h.count("game_reward_claim_mint_receipts").await?,
            1,
            "replay must not remint perfume"
        );
        let output: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1 AND quantity=1")
            .bind(&callback.give.as_ref().ok_or("sample output")?.item)
            .fetch_one(&h.pool).await?;
        assert_eq!(output, 1, "replay leaves exactly one physical sample");
    }
    Ok(())
}

async fn mint_corpse(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    creature: &str,
) -> f::TestResult<(
    crate::durability::reward_claim_mint::quest_creature::QuestCorpseOperand,
    (i32, i32, i16),
)> {
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
        ItemMintRequest, TypedDefinitionRef,
    };
    let mut body = oteryn_gameplay_entities::carrier::CombatDeathFixture::new_for_scope(
        f::fence(1)?.runtime_scope,
        crate::foundation::ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
    )
    .map_err(f::debug)?;
    body.strike(
        "fixture:corpse.quest",
        oteryn_gameplay_entities::carrier::CombatDeathFixture::HEALTH,
    )
    .map_err(f::debug)?;
    let projection = body.project_death_projection().map_err(f::debug)?;
    let death = projection.occurrence().death_key().map_err(f::debug)?;
    let at = projection.position();
    let (frame, map, content) = projection.context_markers();
    let context = [
        frame.to_be_bytes(),
        map.to_be_bytes(),
        content.to_be_bytes(),
    ]
    .concat();
    let spatial = [
        at.x.to_be_bytes().as_slice(),
        at.y.to_be_bytes().as_slice(),
        at.floor.to_be_bytes().as_slice(),
    ]
    .concat();
    let ground = GroundPlacement {
        spatial_position: spatial.clone(),
        corpse_ref: death.native_bytes(),
        map_revision: "map-1".into(),
        content_revision: "content-1".into(),
        native_room_placement_context: context.clone(),
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "fixture:corpse.quest".into(),
        revision_ref: "definition-r1".into(),
    };
    let revision = &native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == creature)
        .ok_or("active corpse race")?
        .profile
        .target
        .revision;
    let race = crate::domain::bestiary::BestiaryRace::new(creature, revision, vec![5, 50, 500])
        .map_err(f::debug)?;
    let cause = ItemMintCause::from_creature_death(
        death,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(f::debug)?
    .with_corpse_race(race)
    .map_err(f::debug)?;
    let mut candidate = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            ItemMintRequest {
                cause,
                item: definition,
                quantity: 1,
                ground,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let item = *candidate.item_instance_id();
    let permit = candidate.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &permit,
                authority,
                &h.node,
                &mut candidate,
                f::id(f::CHARACTER)
            )
            .await
            .map_err(f::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    Ok((
        crate::durability::reward_claim_mint::quest_creature::QuestCorpseOperand {
            item,
            ground: crate::durability::spell_items_abi::SpellGroundTarget {
                item_profile_source: None,
                spatial_position: spatial,
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                placement_context: context,
            },
        },
        (at.x, at.y, at.floor.checked_neg().ok_or("runtime floor")?),
    ))
}
