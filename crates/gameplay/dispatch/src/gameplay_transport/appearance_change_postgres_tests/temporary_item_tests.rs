//! A base selection commits beneath a pinned Item transform; native clear reveals the new base.
use super::*;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use oteryn_gameplay_foundation::foundation::condition::{ApplicationFacts, ConditionDefinition};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    entity: appearance::EntityRef,
    original: &appearance::Change,
) -> TestResult<appearance::Change> {
    let generation = owner.active_generation.ok_or("generation")?;
    let native = generation.native_gameplay().ok_or("native")?;
    let member = native
        .spell_appearances()
        .ok_or("appearances")?
        .for_item("oteryn:item.tibia.i2096", "definition-r1")
        .ok_or("qualified Item")?;
    let reference = native
        .item_index()
        .ok_or("Item index")?
        .definition_ref("Item", &member.key, &member.revision)
        .ok_or("Item reference")?
        .get();
    let definition = ConditionDefinition::new(
        "test.appearance.item",
        1,
        ConditionValues::ItemOutfit {
            duration_ms: 600_000,
        },
    )
    .and_then(|d| d.with_item_appearance(&member.key, &member.revision, native.source_digest()))
    .ok_or("Item condition")?;
    {
        let root = GameplayDecisionRoot::from_bytes([4; 32]);
        let facts = ApplicationFacts {
            now: owner.owner_now().get(),
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([5; 16]),
        };
        let runtime = owner.runtime.lock().await;
        let mut states = owner.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, command.game_session_id)
            .ok_or("player")?;
        *state = state
            .stage_owned_test_conditions(&[definition], &facts)
            .ok_or("Item condition stage")?;
    }
    let mut change = original.clone();
    change.expected_root_revision = 2;
    change.prior = Some(appearance::Selected {
        selection: original.selection.clone(),
        revision: 2,
        provenance: original.content_revision.clone(),
    });
    change.selection.colours = [4, 3, 2, 1];
    assert_eq!(
        crate::gameplay_transport::connection::appearance_change::dispatch(
            owner,
            actor,
            UseCommand {
                command_id: 3,
                ..command
            },
            &change.encode().map_err(debug)?,
        )
        .await,
        Some(appearance::ChangeResult::Committed(3))
    );
    let transformed = owner
        .appearance_observation(actor, command.game_session_id, vec![entity])
        .await
        .ok_or("Item observation")?;
    assert_eq!(
        transformed.bindings[0].look,
        appearance::Look::Item {
            definition_ref: reference
        }
    );
    assert_eq!(transformed.bindings[0].selection_revision, Some(3));
    super::temporary_appearance_tests::clear(owner, actor, command.game_session_id).await?;
    let restored = owner
        .appearance_observation(actor, command.game_session_id, vec![entity])
        .await
        .ok_or("restored Item base")?;
    assert!(matches!(
        restored.bindings[0].look,
        appearance::Look::Outfit {
            colours: [4, 3, 2, 1],
            ..
        }
    ));
    assert_eq!(restored.bindings[0].selection_revision, Some(3));
    Ok(change)
}
