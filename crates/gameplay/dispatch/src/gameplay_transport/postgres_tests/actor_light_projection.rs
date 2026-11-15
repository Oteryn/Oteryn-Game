//! Source-qualified lights are projected through the real Channel and House views.
use super::{TestResult, fixture::debug};
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ExactActorRef, GameSessionId,
};
use crate::gameplay_transport::{ComposedFreshAdmission, world_spatial::EntityLight};
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

pub(super) fn apply_light(state: &mut PlayerSpellState, now: u64, digest: [u8; 32]) -> TestResult {
    let definition = ConditionDefinition::new(
        "test:visibility/light",
        1,
        ConditionValues::SpellLight {
            level: 8,
            color: 215,
            duration_ms: 600_000,
        },
    )
    .ok_or("Light definition")?;
    let root = GameplayDecisionRoot::from_bytes(digest);
    state
        .conditions
        .apply(
            &definition,
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &ApplicationFacts {
                now,
                base_speed: 0,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes(super::fixture::id(187)),
            },
        )
        .map_err(debug)?;
    assert_eq!(state.owned_conditions().light_at(now), Some((8, Some(215))));
    Ok(())
}

pub(super) async fn channel(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    creature: ExactActorRef,
) -> TestResult {
    let view = owner
        .observe_channel_entities(actor)
        .await
        .ok_or("Channel light view")?;
    assert_eq!(
        view.lights.get(&actor.placement_identity()).copied(),
        Some(EntityLight {
            level: 3,
            color: 180
        }),
        "equipped Item presentation supplies light"
    );
    assert_eq!(
        view.lights.get(&creature.placement_identity()).copied(),
        Some(EntityLight {
            level: 3,
            color: 180
        }),
        "native creature presentation supplies light"
    );
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        apply_light(
            states.get_mut(&runtime, actor, session).ok_or("caster")?,
            owner.owner_now().get(),
            runtime.content_pin().server_artifact_digest(),
        )?;
    }
    let view = owner
        .observe_channel_entities(actor)
        .await
        .ok_or("brightest light view")?;
    assert_eq!(
        view.lights.get(&actor.placement_identity()).copied(),
        Some(EntityLight {
            level: 8,
            color: 215
        }),
        "the brighter condition wins over equipment"
    );
    Ok(())
}

pub(super) async fn house(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult {
    let view = owner
        .observe_channel_entities(actor)
        .await
        .ok_or("House light view")?;
    assert_eq!(
        view.lights.get(&actor.placement_identity()).copied(),
        Some(EntityLight {
            level: 8,
            color: 215
        }),
        "entry must preserve the resident's active Light"
    );
    Ok(())
}
