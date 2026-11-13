//! A checked fixture native model exercises the production PostgreSQL-to-actor projector.
use super::{f, rules};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_loop_move::ElementalSummonClock;
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, ExactActorRef, ScopeOwnershipGeneration, WorldId,
};
use crate::gameplay_transport::world_respawns::elemental::project;
use oteryn_gameplay_entities::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags, ScopeRuntimeAssignmentBinding,
};

fn runtime(h: &f::Harness, generation: u64) -> f::TestResult<ChannelRuntimeV1> {
    let world = WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?;
    let pin =
        ChannelContentPin::from_activation(world, 1, [1; 32], [2; 32], [3; 32], [7; 32], (0, 0, 0));
    let fact = h.node.fact();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
        fact.node_id(),
        fact.registration_revision(),
        generation,
        generation,
        &format!("runtime-scope-assignment:{generation}"),
        8,
        pin,
    )
    .map_err(f::debug)?;
    // Fixture health and appearance are deliberate test data, not donor-stat qualification.
    let policy = CompiledCreaturePolicy {
        definition_key: rules::RECIPES[0].creature.into(),
        definition_revision: "fixture-r1".into(),
        display_name: "fixture overlord".into(),
        maximum_health: 20,
        forge_difficulty: None,
        base_speed: 220,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: Vec::new(),
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
        defense: Some(1),
        armor: Some(0),
        mitigation: None,
        resistances: Vec::new(),
        damage_immunities: Vec::new(),
        healing_from_damage: Vec::new(),
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    };
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact([1; 32], vec![policy])
                .map_err(f::debug)?,
        )
        .map_err(f::debug)?;
    Ok(runtime)
}

pub(super) struct Scene {
    runtime: ChannelRuntimeV1,
    binding: ScopeRuntimeAssignmentBinding,
    pin: ChannelContentPin,
    actor: ExactActorRef,
}

pub(super) fn start(h: &f::Harness, clock: &ElementalSummonClock) -> f::TestResult<Scene> {
    eprintln!("QN1_ELEMENTAL_ACTOR_PROJECTION_PG_READY");
    let mut runtime = runtime(h, 1)?;
    let binding = runtime.binding();
    let pin = runtime.content_pin().clone();
    let clocks = std::slice::from_ref(clock);
    project(&mut runtime, binding, &pin, clocks, |_| false).map_err(f::debug)?;
    assert!(
        runtime.boss_spawns().is_empty(),
        "blocked geometry keeps the durable claim queued"
    );
    project(&mut runtime, binding, &pin, clocks, |_| true).map_err(f::debug)?;
    let actor = runtime
        .boss_spawns()
        .first()
        .and_then(|spawn| spawn.actor)
        .ok_or("native projected boss")?;
    project(&mut runtime, binding, &pin, clocks, |_| true).map_err(f::debug)?;
    assert_eq!(runtime.boss_spawns().len(), 1);
    assert_eq!(runtime.boss_spawns()[0].actor, Some(actor));
    assert_eq!(
        runtime.positioned_actor_census().map_err(f::debug)?.len(),
        1
    );
    let mut successor = self::runtime(h, 2)?;
    assert!(project(&mut successor, binding, &pin, clocks, |_| true).is_err());
    assert!(
        successor
            .positioned_actor_census()
            .map_err(f::debug)?
            .is_empty()
    );
    let mut stale = clock.clone();
    stale.owner_generation = ScopeOwnershipGeneration::new(2).map_err(f::debug)?;
    assert!(project(&mut runtime, binding, &pin, &[stale], |_| true).is_err());
    assert!(runtime.contains_live_creature(actor));
    Ok(Scene {
        runtime,
        binding,
        pin,
        actor,
    })
}

impl Scene {
    pub(super) fn die(&mut self) -> f::TestResult {
        self.runtime
            .remove_encounter_actor(self.actor)
            .map_err(f::debug)?;
        Ok(())
    }
    pub(super) fn finish(&mut self, clocks: &[ElementalSummonClock]) -> f::TestResult {
        project(&mut self.runtime, self.binding, &self.pin, clocks, |_| true).map_err(f::debug)?;
        assert!(self.runtime.boss_spawns().is_empty());
        assert!(
            self.runtime
                .positioned_actor_census()
                .map_err(f::debug)?
                .is_empty()
        );
        eprintln!("QN1_ELEMENTAL_ACTOR_PROJECTION_PG_EXECUTED");
        Ok(())
    }
}
