#![allow(
    clippy::expect_used,
    reason = "test fixture invariants use named assertions"
)]


use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId,
};

use crate::gameplay_transport::actor_spell::ChannelSpellStates;

pub(super) struct CertainCharms(Vec<oteryn_gameplay_progression::charm_effects::CharmDefinition>);

impl oteryn_gameplay_progression::charm_effects::CharmCatalogueRead for CertainCharms {
    fn charm(
        &self,
        key: &str,
    ) -> Option<&oteryn_gameplay_progression::charm_effects::CharmDefinition> {
        self.0.iter().find(|definition| definition.key() == key)
    }
}

pub(super) fn assign_charms(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    source: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    names: &[&str],
) -> CertainCharms {
    use crate::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};
    use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
    use oteryn_gameplay_progression::charm_effects::{
        CharmCatalogueRead, CharmDefinition, CharmPercent,
    };
    let canonical = crate::content::charm_source::CanonicalCharmCatalogue::embedded()
        .expect("canonical charms");
    let race = runtime
        .companion_snapshot_including_dead(source)
        .expect("source")
        .state
        .policy
        .definition_key
        .clone();
    let mut certain = Vec::new();
    let mut state = CharacterCharmState {
        unlocks: Default::default(),
        assignments: Default::default(),
    };
    for name in names {
        let key = CharmKey::new(format!("oteryn:charm.{name}")).expect("key");
        let definition = canonical.charm(key.as_str()).expect("definition");
        certain.push(
            CharmDefinition::new(
                key.as_str(),
                definition.category(),
                definition.effect().stage_value(),
                [100, 5000, 10000]
                    .map(|chance| CharmPercent::from_hundredths(chance).expect("chance")),
                definition.effect(),
            )
            .expect("certain"),
        );
        state
            .unlocks
            .insert(key.clone(), CharmStage::new(3).expect("stage"));
        state
            .assignments
            .insert(key, BestiaryRaceKey::new(race.clone()).expect("race"));
    }
    let character = *runtime
        .bound_attacker_lease(target, session)
        .expect("principal")
        .character_id()
        .as_bytes();
    assert!(states.install_charm_state(
        runtime,
        target,
        session,
        CombatCharmState::new(
            crate::domain::CharacterId::from_bytes(character).expect("character"),
            state
        )
    ));
    CertainCharms(certain)
}
