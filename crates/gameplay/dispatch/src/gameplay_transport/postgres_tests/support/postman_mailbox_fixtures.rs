use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::input_item;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    encode_use_intent,
};

pub(super) const AXE: &str = "oteryn:item.tibia.i3304";
pub(super) const ALTERNATE: &str = "oteryn:item.tibia.i9598";
pub(super) const LETTERBAG: &str = "oteryn:item.tibia.i3217";
const DELIVERED: &str = "oteryn:item.tibia.i2859";

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    callback: &Callback,
    target: WorldObjectTarget,
    predecessor: i64,
    selected: &str,
) -> TestResult {
    let delivery = selected == LETTERBAG;
    let items = if delivery {
        [LETTERBAG; 2]
    } else {
        [AXE, ALTERNATE]
    };
    if delivery {
        // Let the player earn the capacity to carry two full 500-oz letterbags.
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "mailbox progression fence")?
            .ok_or("mailbox progression fence")?;
        let binding = owner
            .progression_binding(admitted.game_session_id)
            .ok_or("mailbox progression binding")?;
        let award = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_experience(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(96)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(7_915_800),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
                None,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(award, ExperienceCommitOutcome::Committed(_)),
            "{award:?}"
        );
    }
    let fence = admitted.item_fence.ok_or("item fence")?;
    let position = callback
        .selector
        .native
        .as_ref()
        .ok_or("mailbox position")?
        .position;
    let track = &callback.storage.first().ok_or("mailbox track")?.track;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("Postman catalogue")?;
    let definition = catalogue
        .definition_hash(&callback.quest)
        .ok_or("Postman definition")?;
    let character = fence.character_id.as_bytes().as_slice();
    // Arrange a save at the handoff from the preceding mission step. The real
    // World USE below must still earn its own fenced receipt and item mutation.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let state = sqlx::query(
        "INSERT INTO game_character_quest_states
         (character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision)
         SELECT character_id,$2,$3,$4,character_revision,character_revision FROM game_character_roots
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .bind(&callback.quest)
    .bind(catalogue.content_revision())
    .bind(definition.as_slice())
    .execute(&mut *tx)
    .await?;
    assert_eq!(state.rows_affected(), 1);
    let stage = sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .bind(track)
    .bind(&callback.quest)
    .bind(predecessor)
    .execute(&mut *tx)
    .await?;
    assert_eq!(stage.rows_affected(), 1);
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(|error| format!("bind mailbox actor lease: {error:?}"))?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("USE capabilities")?;
    for (id, item) in [580, 581].into_iter().zip(items) {
        input_item(
            owner,
            fence,
            &Offering {
                position,
                item: item.into(),
                count: 1,
            },
            id,
            true,
        )
        .await?;
    }
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .map_err(|error| format!("read mailbox inventory: {error:?}"))?;
    let carried: Vec<_> = inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .filter(|entry| items.contains(&entry.item.definition.production_key.as_str()))
        .map(|entry| {
            (
                sqlx::types::Uuid::from_bytes(entry.item.item_instance_id),
                entry.item.definition.production_key.clone(),
            )
        })
        .collect();
    assert_eq!(carried.len(), 2);

    let stage = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
        )
        .bind(track)
        .fetch_one(&h.pool)
        .await
    };
    let item_definition = |id| async move {
        sqlx::query_scalar::<_, String>(
            "SELECT i.definition_production_key FROM game_item_instances i \
             JOIN game_item_container_entries e USING (item_instance_id) \
             WHERE i.item_instance_id=$1 AND i.lifecycle=1 AND i.quantity=1",
        )
        .bind(id)
        .fetch_one(&h.pool)
        .await
    };
    if delivery {
        let payload = encode_use_intent(&target).map_err(debug)?;
        let (next, result, _) = super::item_craft_fixtures::exchange(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            &payload,
            None,
        )
        .await?;
        admitted = next;
        assert_eq!(
            stage().await?,
            2,
            "bare USE changed the delivery stage ({result:?})"
        );
        for (id, definition) in &carried {
            assert_eq!(&item_definition(*id).await?, definition);
        }
        assert_ne!(result, UseDisposition::Committed);
    }

    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", selected, "definition-r1"))
        .ok_or("selected tool definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let (_, result, _) = super::item_craft_fixtures::exchange_selected(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
        usize::from(delivery),
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "{} using {selected}",
        callback.key
    );
    assert_eq!(stage().await?, if delivery { 3 } else { 2 });
    for (index, (id, definition)) in carried.iter().enumerate() {
        let expected = if delivery && index == 1 {
            DELIVERED
        } else {
            definition
        };
        assert_eq!(item_definition(*id).await?, expected);
    }
    eprintln!("POSTGRES_EXECUTED {} using {selected}", callback.key);
    Ok(())
}
