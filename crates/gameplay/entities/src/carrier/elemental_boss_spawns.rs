//! M49 projects committed one-shot clocks through the existing native boss owner.
use super::*;
use oteryn_gameplay_content::content::{BossChance, BossPolicy};

const PREFIX: &str = "qn1.elemental:";
const ALTARS: usize = 4;
const POLICY: BossPolicy = BossPolicy::AtReset {
    chance: BossChance {
        numerator: 0,
        denominator: 1,
    },
};

fn valid_key(key: &str) -> bool {
    key.strip_prefix(PREFIX).is_some_and(|id| {
        id.len() == 32
            && id.bytes().any(|byte| byte != b'0')
            && id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

impl ChannelRuntimeV1 {
    /// The caller has committed this arrival clock under its independently current
    /// Channel owner. Replaying it preserves the exact actor, including its movement.
    pub fn install_elemental_boss_spawn(
        &mut self,
        pin: &ChannelContentPin,
        generation: ScopeOwnershipGeneration,
        key: &str,
        creature: &str,
        position: MovementLocalPosition,
        cycle: u64,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.owner_fence()?;
        if self.binding.channel_id().is_none()
            || self.binding.scope_generation() != generation
            || self.content_pin() != pin
            || !valid_key(key)
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        self.companion_policy(creature)?;
        if let Some(spawn) = self.boss_spawns.iter().find(|spawn| spawn.key == key) {
            if spawn.creature != creature
                || spawn.position != position
                || spawn.cycle != cycle
                || spawn.policy != POLICY
            {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
            if let Some(actor) = spawn.actor {
                // Death settlement may still be pending. The same cycle cannot
                // recreate its actor while waiting for the durable lease release.
                return Ok(self
                    .carrier
                    .creature_alive(&self.continuity, actor)
                    .then_some(actor));
            }
        } else {
            if self
                .boss_spawns
                .iter()
                .filter(|spawn| spawn.key.starts_with(PREFIX))
                .count()
                >= ALTARS
                || self.boss_spawns.len() >= CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX
            {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
            self.boss_spawns.push(BossSpawn {
                key: key.into(),
                creature: creature.into(),
                position,
                policy: POLICY,
                actor: None,
                cycle,
                pending_claim: true,
            });
        }
        match self.realize_boss_spawn(key, cycle) {
            // The committed claim stays queued until native actor capacity returns.
            Err(CarrierError::CapacityExceeded) => Ok(None),
            result => result,
        }
    }

    /// Call only after a successful complete recovery set for this binding.
    /// A live actor contradicting that set is a fault, never silent cleanup.
    pub fn retain_elemental_boss_spawns(
        &mut self,
        pin: &ChannelContentPin,
        generation: ScopeOwnershipGeneration,
        confirmed: &[&str],
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let keys: BTreeSet<_> = confirmed.iter().copied().collect();
        if self.binding.channel_id().is_none()
            || self.binding.scope_generation() != generation
            || self.content_pin() != pin
            || confirmed.len() > ALTARS
            || keys.len() != confirmed.len()
            || confirmed.iter().any(|key| !valid_key(key))
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let absent =
            |spawn: &BossSpawn| spawn.key.starts_with(PREFIX) && !keys.contains(spawn.key.as_str());
        if self
            .boss_spawns
            .iter()
            .filter(|spawn| absent(spawn))
            .any(|spawn| {
                spawn
                    .actor
                    .is_some_and(|actor| self.carrier.creature_alive(&self.continuity, actor))
            })
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let retired: Vec<_> = self
            .boss_spawns
            .iter()
            .filter(|spawn| absent(spawn))
            .filter_map(|spawn| spawn.actor)
            .collect();
        for actor in retired {
            // Exact actor generations protect a slot already reused by a new boss.
            let _ = self.carrier.remove(&self.continuity, actor.0);
        }
        self.boss_spawns.retain(|spawn| !absent(spawn));
        Ok(())
    }
}
