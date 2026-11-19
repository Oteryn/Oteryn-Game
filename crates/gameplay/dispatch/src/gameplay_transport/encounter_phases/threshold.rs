//! Crystal 201189f creaturescripts_faceless_bane.lua:11-28, faceless_bane.lua:23-26,153-157.
use super::*;
use oteryn_gameplay_encounters::bosses::phase_controller::gates;
use oteryn_gameplay_quests::quest::world_callbacks::ThresholdRearm;

pub(in crate::gameplay_transport) struct Threshold {
    source: String,
    spec: ThresholdRearm,
    boss: ExactActorRef,
    protected: bool,
    remaining: u8,
}

impl Threshold {
    pub(in crate::gameplay_transport) fn bind(
        source: &str,
        spec: &ThresholdRearm,
        boss: ExactActorRef,
    ) -> Self {
        Self {
            source: source.into(),
            spec: spec.clone(),
            boss,
            protected: false,
            remaining: spec.restores,
        }
    }

    pub(in crate::gameplay_transport) fn release(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: &str,
    ) -> Result<(), CarrierError> {
        if self.source != source || !runtime.contains_live_creature(self.boss) {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        // Crystal movements_faceless_tiles.lua:11-17 finds the boss within a
        // same-floor spectator square centred on the Cathedral, not the player.
        let boss = runtime.read_actor_position(self.boss)?.position();
        let center = self.spec.release_center;
        let radius = u32::from(self.spec.release_radius);
        if boss.floor != -i16::from(center.floor)
            || boss.x.abs_diff(i32::from(center.x)) > radius
            || boss.y.abs_diff(i32::from(center.y)) > radius
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if self.protected {
            gates::release(runtime, self.boss)?;
            self.protected = false;
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) fn tick(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        if self.protected || self.remaining == 0 || !runtime.contains_live_creature(self.boss) {
            return Ok(Vec::new());
        }
        let boss = runtime.companion_snapshot(self.boss)?;
        let creature = &boss.state.policy.definition_key;
        let maximum = boss.state.policy.maximum_health;
        if i128::from(boss.health) * 100
            > i128::from(maximum) * i128::from(self.spec.health_percent)
        {
            return Ok(Vec::new());
        }
        let maximum = u64::try_from(maximum).map_err(|_| CarrierError::InvalidCreatureHealth)?;
        let at = runtime.read_actor_position(self.boss)?.position();
        let content = runtime.content_pin().server_artifact_digest();
        // Crystal creaturescripts_faceless_bane.lua:18-27 restores and
        // protects the boss before separately attempting each force-spawn.
        runtime.commit_quest_creature_spawn_with_owner(&[], |runtime, _| {
            gates::protect(runtime, &[self.boss])?;
            runtime.commit_source_creature_heal_batch(
                content,
                &[(self.boss, creature.clone(), maximum, maximum)],
            )?;
            Ok(())
        })?;
        self.protected = true;
        self.remaining -= 1;
        let mut actors = Vec::with_capacity(self.spec.summons.len());
        for key in &self.spec.summons {
            let Ok(policy) = runtime.companion_policy(key) else {
                continue;
            };
            if let Ok(spawned) =
                runtime.commit_quest_creature_spawn(&[(key.as_str(), policy.maximum_health, at)])
            {
                actors.extend(spawned);
            }
        }
        Ok(actors)
    }
}
