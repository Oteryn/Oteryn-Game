//! The sealed production proof is read from native actors, never a fabricated target.
use super::*;
use crate::durability::reward_claim_mint::{
    QuestCarriedUse, quest_creature::QuestCreatureUseProof,
};
use crate::foundation::{ChannelRuntimeV1, CharacterId, CharacterLease, NodeId};
use oteryn_gameplay_content::content::{
    NativeEntryContentPin, channel_content_pin::ChannelContentPin,
    native_gameplay::NativeGameplayState,
};
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};

pub(super) async fn sample(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    pin: &NativeEntryContentPin,
    native: &NativeGameplayState,
) -> f::TestResult {
    let revision: i64 = sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let fence = f::fence(u64::try_from(revision)?)?;
    let world = fence.runtime_scope.world_id();
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
        f::scope()?.channel_id().ok_or("channel")?,
        NodeId::decode(&f::id(1)).map_err(f::debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        12,
        channel_pin,
    )
    .map_err(f::debug)?;
    let reservation = runtime
        .reserve_fresh_session(fence.game_session_id)
        .map_err(f::debug)?;
    let actor = runtime
        .commit_fresh_session(reservation)
        .map_err(f::debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(f::debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            fence.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(f::debug)?,
                fence.character_lease_generation,
            )
            .map_err(f::debug)?,
        )
        .map_err(f::debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(f::debug)?
        .position();
    let mut targets = Vec::new();
    for (key, dx) in [
        ("oteryn:creature.rat", -1),
        ("oteryn:creature.rotworm", 4),
        ("oteryn:creature.rotworm", 1),
    ] {
        let target = runtime
            .admit_source_pinned_lab_creature(
                crate::foundation::MovementLocalPosition {
                    x: at.x + dx,
                    y: at.y,
                    floor: at.floor,
                },
                key,
                100,
            )
            .map_err(f::debug)?;
        let visible = runtime.visible_entities();
        let generation = visible
            .creatures
            .iter()
            .find(|c| c.actor == target)
            .ok_or("visible target")?
            .generation;
        targets.push(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation,
        });
    }
    let callback = reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/goblin-merchant/rotworm-sample")
        .ok_or("production sample")?;
    let command = command(9500)?;
    let index = native.item_index().ok_or("native item index")?;
    let key = "oteryn:item.tibia.i17673";
    let definition = index
        .definition_ref("Item", key, index.revision(key).ok_or("tool revision")?)
        .ok_or("tool reference")?;
    let intent = |target| UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: Some(target),
        use_with_map_item: None,
        use_with_position: None,
    };
    for target in &targets[..2] {
        assert!(
            QuestCreatureUseProof::read_current(
                &runtime,
                actor,
                fence,
                command,
                &callback,
                &intent(*target)
            )
            .is_none(),
            "wrong species or remote actor cannot authorize a sample"
        );
    }
    let mut stale = targets[2];
    stale.generation += 1;
    assert!(
        QuestCreatureUseProof::read_current(
            &runtime,
            actor,
            fence,
            command,
            &callback,
            &intent(stale)
        )
        .is_none()
    );
    let intent = intent(targets[2]);
    let proof =
        QuestCreatureUseProof::read_current(&runtime, actor, fence, command, &callback, &intent)
            .ok_or("current native Rotworm")?;
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i17673' AND lifecycle=1").fetch_one(&h.pool).await?;
    let carried = QuestCarriedUse {
        source: source.try_into().map_err(|_| "source UUID")?,
        intent: encode_item_use_intent(&intent).map_err(f::debug)?,
    };
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
            vec![reward_callbacks::primary_transition(&callback)?],
        )
        .map_err(f::debug)?,
    );
    let weights = Weights(native);
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: Some(&carried),
        creature_use: Some(&proof),
        peer_use: None,
    };
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(command),
    };
    let without_target = QuestRewardAdmission {
        creature_use: None,
        ..admission
    };
    assert!(
        matches!(
            h.root
                .commit_fixture_quest_reward(
                    authority,
                    &h.node,
                    fence,
                    request.clone(),
                    catalogue.clone(),
                    without_target,
                    callback.clone()
                )
                .await
                .map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "source possession alone cannot authorize a live-target quest action"
    );
    let apply = || {
        h.root.commit_fixture_quest_reward(
            authority,
            &h.node,
            fence,
            request.clone(),
            catalogue.clone(),
            admission,
            callback.clone(),
        )
    };
    assert!(matches!(
        apply().await.map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let atomic: bool = sqlx::query_scalar("SELECT c.xmin=q.xmin AND c.xmin=m.xmin AND c.xmin=s.xmin FROM game_quest_callback_consumptions c JOIN game_quest_reward_sources s USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_character_quest_receipts q USING(character_id,cause_id,cause_ordinal,transition_key) JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id WHERE c.item_instance_id=encode($1,'hex')::uuid").bind(carried.source.as_slice()).fetch_one(&h.pool).await?;
    assert!(
        atomic,
        "exact source consumption, transformed tool MINT and progress commit together"
    );
    let quantities: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i17673' AND lifecycle=1),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i17674' AND lifecycle=1)").fetch_one(&h.pool).await?;
    assert_eq!(quantities, (0, 2));
    assert!(matches!(
        apply().await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE item_instance_id=encode($1,'hex')::uuid").bind(carried.source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        count, 1,
        "a replay never consumes or remints the selected tool again"
    );
    Ok(())
}
