//! A committed live occurrence cannot be captured twice, even before projection.
use super::*;
use crate::durability::reward_claim_mint::{
    QuestCarriedUse, quest_creature::QuestCreatureUseProof,
};
use oteryn_gameplay_content::content::{
    NativeEntryContentPin, native_gameplay::NativeGameplayState,
};
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};

pub(super) async fn capture(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    pin: &NativeEntryContentPin,
    native: &NativeGameplayState,
) -> f::TestResult {
    let all = reward_callbacks::load()?;
    let intro = all
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/questpc7-goblin-intro")
        .ok_or("production intro")?
        .clone();
    let callbacks = (1..=3)
        .map(|n| {
            all.iter()
                .find(|c| c.key == format!("oteryn:quest-reward/questpc7-goblin-capture-{n}"))
                .cloned()
                .ok_or("production capture")
        })
        .collect::<Result<Vec<_>, _>>()?;
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
                cause: QuestCause::Command(command(9505)?),
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
    let mut targets = Vec::new();
    for (dx, dy) in [(1, 0), (0, 1), (1, 1)] {
        let target = runtime
            .admit_source_pinned_lab_creature(
                crate::foundation::MovementLocalPosition {
                    x: at.x + dx,
                    y: at.y + dy,
                    floor: at.floor,
                },
                "oteryn:creature.adventurer",
                100,
            )
            .map_err(f::debug)?;
        let visible = runtime.visible_entities();
        let generation = visible
            .creatures
            .iter()
            .find(|c| c.actor == target)
            .ok_or("native target")?
            .generation;
        targets.push((
            target,
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: target.placement_identity(),
                generation,
            },
        ));
    }
    let index = native.item_index().ok_or("native index")?;
    let key = "oteryn:item.tibia.i17669";
    let definition = index
        .definition_ref("Item", key, index.revision(key).ok_or("tool revision")?)
        .ok_or("native reference")?;
    let source = || {
        sqlx::query_scalar::<_,Vec<u8>>("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i17669'").fetch_one(&h.pool)
    };
    let original: [u8; 16] = source().await?.try_into().map_err(|_| "source UUID")?;
    let apply = |fence, callback: RewardCallback, command, source: [u8; 16], target| {
        let catalogue = catalogue.clone();
        let runtime = &runtime;
        async move {
            let intent = UseIntent {
                target: UseTarget::ItemByDefinition(definition),
                use_with: Some(target),
                use_with_map_item: None,
                use_with_position: None,
            };
            let proof = QuestCreatureUseProof::read_current(
                runtime, actor, fence, command, &callback, &intent,
            )
            .ok_or("current capture target")?;
            let carried = QuestCarriedUse {
                recipe_target: None,
                source,
                intent: encode_item_use_intent(&intent).map_err(f::debug)?,
            };
            let weights = Weights(native);
            h.root
                .commit_fixture_quest_reward(
                    authority,
                    &h.node,
                    fence,
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
    let result = apply(
        initial,
        callbacks[0].clone(),
        command(9510)?,
        original,
        targets[0].1,
    )
    .await?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "{result:?}"
    );
    assert!(matches!(
        apply(
            initial,
            callbacks[0].clone(),
            command(9510)?,
            original,
            targets[0].1
        )
        .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let next: [u8; 16] = source().await?.try_into().map_err(|_| "returned tool")?;
    assert_ne!(next, original, "a returned tool has its own MINT custody");
    let fence = f::fence(u64::try_from(revision + 1)?)?;
    let result = apply(
        fence,
        callbacks[1].clone(),
        command(9511)?,
        next,
        targets[0].1,
    )
    .await?;
    assert!(
        matches!(result, QuestTransitionOutcome::Refused(_)),
        "a second command cannot capture the same live occurrence: {result:?}"
    );
    assert_eq!(
        source().await?,
        next,
        "duplicate target refusal rolls back tool consumption"
    );
    let result = apply(
        fence,
        callbacks[1].clone(),
        command(9512)?,
        next,
        targets[1].1,
    )
    .await?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "{result:?}"
    );
    assert!(matches!(
        apply(
            fence,
            callbacks[1].clone(),
            command(9512)?,
            next,
            targets[1].1
        )
        .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let final_tool: [u8; 16] = source().await?.try_into().map_err(|_| "final tool")?;
    let final_fence = f::fence(u64::try_from(revision + 2)?)?;
    let result = apply(
        final_fence,
        callbacks[2].clone(),
        command(9513)?,
        final_tool,
        targets[2].1,
    )
    .await?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "last capture: {result:?}"
    );
    let rows:(i64,bool)=sqlx::query_as("SELECT count(*),bool_and(c.xmin=s.xmin AND c.xmin=q.xmin AND c.xmin=i.xmin) FROM game_quest_creature_captures c JOIN game_quest_reward_sources s USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_character_quest_receipts q USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_quest_callback_consumptions i ON (i.character_id,i.cause_id,i.cause_ordinal,i.transition_key,i.item_instance_id)=(c.character_id,c.cause_id,c.cause_ordinal,c.transition_key,c.source_item)").fetch_one(&h.pool).await?;
    assert_eq!(
        rows,
        (3, true),
        "captures, exact selected input and quest progress share each transaction"
    );
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i17669'").fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0, "the final authored capture consumes its tool");
    for (target, _) in targets {
        runtime
            .remove_mount_taming_subject(target, "oteryn:creature.adventurer")
            .map_err(f::debug)?;
        assert!(
            runtime
                .check_mount_taming_subject(target, "oteryn:creature.adventurer")
                .is_err()
        );
    }
    Ok(())
}
