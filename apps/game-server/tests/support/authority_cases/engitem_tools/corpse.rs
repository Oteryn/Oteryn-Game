//! Sampling uses an actual native death and immutable corpse species provenance.
use super::*;
use crate::durability::reward_claim_mint::{
    QuestCarriedUse, quest_creature::QuestCreatureUseProof,
};
use oteryn_gameplay_content::content::{
    NativeEntryContentPin, native_gameplay::NativeGameplayState,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};

pub(super) async fn sample(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    pin: &NativeEntryContentPin,
    native: &NativeGameplayState,
) -> f::TestResult {
    let all = reward_callbacks::load()?;
    let intro = all
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/questpc7-goblin-intro")
        .ok_or("intro")?
        .clone();
    let callback = all
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/questpc7-goblin-slug-sample")
        .ok_or("slug callback")?
        .clone();
    let callbacks = [callback.clone()];
    let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/quests/npc-callbacks/m13-goblin-capture.json"
    )))?;
    let tracks = document["tracks"]
        .as_array()
        .ok_or("tracks")?
        .iter()
        .map(|t| -> f::TestResult<QuestTrack> {
            Ok(QuestTrack {
                key: t["key"].as_str().ok_or("track key")?.into(),
                quest: intro.quest.clone(),
                initial: -1,
                min: -1,
                max: t["max"].as_i64().ok_or("maximum")?,
            })
        })
        .collect::<f::TestResult<Vec<_>>>()?;
    let transitions = std::iter::once(&intro)
        .chain(callbacks.iter())
        .map(reward_callbacks::primary_transition)
        .collect::<Result<Vec<_>, _>>()?;
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, transitions).map_err(f::debug)?);
    let revision: i64 = sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let weights = Weights(native);
    let result = h
        .root
        .commit_fixture_quest_reward(
            authority,
            &h.node,
            f::fence(u64::try_from(revision)?)?,
            QuestTransitionRequest {
                transition_key: intro.key.clone(),
                cause: QuestCause::Command(command(9600)?),
            },
            catalogue.clone(),
            QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                creature_use: None,
                peer_use: None,
            },
            intro,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "production intro: {result:?}"
    );
    let revision = revision + 1;
    let initial = f::fence(u64::try_from(revision)?)?;
    let (mut runtime, actor) = super::live::runtime(pin, initial)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(f::debug)?
        .position();
    native
        .install_companion_policies(&mut runtime)
        .map_err(f::debug)?;
    let index = native.item_index().ok_or("native index")?;
    let key = "oteryn:item.tibia.i17670";
    let definition = index
        .definition_ref("Item", key, index.revision(key).ok_or("tool revision")?)
        .ok_or("tool reference")?;
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i17670'").fetch_one(&h.pool).await?;
    let source: [u8; 16] = source.try_into().map_err(|_| "source UUID")?;
    let old = mint_body(h, authority, native, &mut runtime, at, 901).await?;
    // Production freshness remains ten seconds. No shortened clock or quest writes.
    tokio::time::sleep(std::time::Duration::from_millis(10050)).await;
    let far = mint_body(
        h,
        authority,
        native,
        &mut runtime,
        crate::foundation::MovementLocalPosition { x: at.x + 3, ..at },
        903,
    )
    .await?;
    let fresh = mint_body(h, authority, native, &mut runtime, at, 902).await?;
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(
            std::num::NonZeroU64::new(1).ok_or("handle")?,
        )),
        use_with_position: None,
    };
    let apply = |body, command| {
        let callback = callback.clone();
        let catalogue = catalogue.clone();
        let runtime = &runtime;
        let intent = &intent;
        async move {
            let proof = QuestCreatureUseProof::read_current_corpse(
                runtime, actor, initial, command, &callback, intent, body,
            )
            .ok_or("corpse proof")?;
            let carried = QuestCarriedUse {
                recipe_target: None,
                source,
                intent: encode_item_use_intent(intent).map_err(f::debug)?,
            };
            let weights = Weights(native);
            h.root
                .commit_fixture_quest_reward(
                    authority,
                    &h.node,
                    initial,
                    QuestTransitionRequest {
                        transition_key: callback.key.clone(),
                        cause: QuestCause::Command(command),
                    },
                    catalogue,
                    QuestRewardAdmission {
                        native,
                        weights: &weights,
                        fee_payment: None,
                        actor_conditions: None,
                        carried_use: Some(&carried),
                        creature_use: Some(&proof),
                        peer_use: None,
                    },
                    callback,
                )
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { f::debug(e).into() })
        }
    };
    for (body, ordinal) in [(f::id(99), 9601), (old, 9602), (far, 9604)] {
        let refused = apply(body, command(ordinal)?).await?;
        assert!(
            matches!(refused, QuestTransitionOutcome::Refused(_)),
            "missing, expired or distant native corpse: {refused:?}"
        );
    }
    let before:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_array(to_jsonb(i),to_jsonb(g)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE item_instance_id=encode($1,'hex')::uuid").bind(fresh.as_slice()).fetch_one(&h.pool).await?;
    let result = apply(fresh, command(9603)?).await?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "fresh native corpse: {result:?}"
    );
    tokio::time::sleep(std::time::Duration::from_millis(10050)).await;
    assert!(
        matches!(
            apply(fresh, command(9603)?).await?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "historical replay needs neither the consumed tool nor a fresh corpse"
    );
    let after:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_array(to_jsonb(i),to_jsonb(g)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE item_instance_id=encode($1,'hex')::uuid").bind(fresh.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        before, after,
        "sampling retains corpse identity, attributes, placement and native decay"
    );
    let rows:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(c.xmin=s.xmin AND c.xmin=q.xmin AND c.xmin=i.xmin AND c.xmin=m.xmin) FROM game_quest_corpse_samples c JOIN game_quest_reward_sources s USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_character_quest_receipts q USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_quest_callback_consumptions i ON (i.character_id,i.cause_id,i.cause_ordinal,i.transition_key,i.item_instance_id)=(c.character_id,c.cause_id,c.cause_ordinal,c.transition_key,c.source_item) JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id").fetch_one(&h.pool).await?;
    assert_eq!(
        rows,
        (1, true),
        "sample proof, exact input, output MINT and progress are atomic"
    );
    let state:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i17670'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i17672')").fetch_one(&h.pool).await?;
    assert_eq!(
        state,
        (0, 1),
        "empty bottle is replaced once by the slug sample"
    );
    let progress: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/questpc7/goblin/odours'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(progress, 1, "production first odour advances exactly once");
    Ok(())
}

async fn mint_body(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &NativeGameplayState,
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    position: crate::foundation::MovementLocalPosition,
    source: u16,
) -> f::TestResult<[u8; 16]> {
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
        ItemMintRequest, TypedDefinitionRef,
    };
    let position = crate::foundation::MovementLocalPosition {
        x: position.x + 1,
        ..position
    };
    let pin = runtime.content_pin().clone();
    let creature = runtime
        .realize_native_qualification_spawn_for_source(
            source,
            pin,
            "oteryn:creature.slug",
            position,
        )
        .map_err(f::debug)?;
    let health = runtime
        .companion_snapshot(creature.actor)
        .map_err(f::debug)?
        .health;
    runtime
        .commit_monster_lab_damage(creature.actor, &source.to_be_bytes(), health)
        .map_err(f::debug)?;
    let death = {
        let mut combat = runtime.borrow_combat_death();
        let projection =
            oteryn_gameplay_entities::project_fixed_one_creature_death(&mut combat, creature.actor)
                .map_err(f::debug)?;
        projection.occurrence().death_key().map_err(f::debug)?
    };
    let row = native
        .reward_table()
        .row("oteryn:creature.slug")
        .map_err(f::debug)?;
    let definition = TypedDefinitionRef {
        family: row.corpse_item.family.clone(),
        production_key: row.corpse_item.production_key.clone(),
        revision_ref: row.corpse_item.revision_ref.clone(),
    };
    assert_eq!(definition.production_key, "oteryn:item.tibia.i12526");
    let cause = ItemMintCause::from_creature_death(
        death,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(f::debug)?
    .with_corpse_race(row.race.clone().ok_or("native slug race")?)
    .map_err(f::debug)?;
    let mut candidate = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            ItemMintRequest {
                cause,
                item: definition.clone(),
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            position.x.try_into()?,
                            position.y.try_into()?,
                            (-position.floor).try_into()?,
                        ),
                    corpse_ref: creature.actor.corpse_identity().to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: vec![6; 16],
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    let id = match h
        .root
        .commit_corpse_mint(
            &permit,
            authority,
            &h.node,
            &mut candidate,
            f::id(f::CHARACTER),
        )
        .await
        .map_err(f::debug)?
    {
        ItemMintOutcome::Committed(v) => v.item_instance_id,
        other => return Err(format!("native corpse MINT {other:?}").into()),
    };
    drop(permit);
    runtime
        .bind_corpse_item(
            creature.actor,
            id,
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )
        .map_err(f::debug)?;
    runtime
        .remove_encounter_actor(creature.actor)
        .map_err(f::debug)?;
    Ok(id)
}
