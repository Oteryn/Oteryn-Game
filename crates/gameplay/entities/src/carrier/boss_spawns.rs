use super::*;
use oteryn_gameplay_content::content::BossPolicy;

/// Content placement and ephemeral realization; the clock remains in PostgreSQL.
#[derive(Debug, Clone)]
pub struct BossSpawn {
    pub key: String,
    pub creature: String,
    pub position: MovementLocalPosition,
    pub policy: BossPolicy,
    pub actor: Option<ExactActorRef>,
    pub cycle: u64,
    pub pending_claim: bool,
}

impl ChannelRuntimeV1 {
    /// Installed before serving, from the checked bundle's boss-only points.
    pub fn configure_boss_spawns(&mut self, spawns: Vec<BossSpawn>) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !self.boss_spawns.is_empty() || spawns.len() > CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let mut keys = BTreeSet::new();
        for spawn in &spawns {
            if spawn.key.is_empty()
                || spawn.actor.is_some()
                || spawn.pending_claim
                || !keys.insert(&spawn.key)
                || spawn.policy.validate().is_err()
                || self.companion_policy(&spawn.creature).is_err()
            {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
        }
        self.boss_spawns = spawns;
        Ok(())
    }

    pub fn boss_spawns(&self) -> &[BossSpawn] {
        &self.boss_spawns
    }

    /// The caller has committed the exact clock claim and still owns this generation.
    /// These actors are never registered in the ordinary respawn schedule.
    pub fn realize_boss_spawn(
        &mut self,
        key: &str,
        cycle: u64,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.owner_fence()?;
        let index = self
            .boss_spawns
            .iter()
            .position(|s| s.key == key)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let spawn = &self.boss_spawns[index];
        if spawn
            .actor
            .is_some_and(|a| self.carrier.creature_alive(&self.continuity, a))
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let at = spawn.position;
        let creature = spawn.creature.clone();
        let old = spawn.actor;
        let health = self.companion_policy(&creature)?.maximum_health;
        let context = self.pinned_position_context();
        self.boss_spawns[index].cycle = cycle;
        self.boss_spawns[index].pending_claim = true;
        if !self.native_summon_cell_free(at) {
            // A confirmed claim is retained while occupancy delays installation. A new
            // owner can still recover ALIVE if this process ends while waiting.
            return Ok(None);
        }
        if let Some(old) = old {
            let _ = self.carrier.remove(&self.continuity, old.0);
        }
        let actor =
            self.carrier
                .admit_creature(&self.continuity, ActorState(0), &creature, health)?;
        let actor = ExactActorRef(actor);
        let result = self
            .carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .and_then(|_| self.install_creature_policy(actor, &creature));
        if let Err(error) = result {
            let _ = self.carrier.remove(&self.continuity, actor.0);
            return Err(error);
        }
        self.boss_spawns[index].actor = Some(actor);
        self.boss_spawns[index].cycle = cycle;
        self.boss_spawns[index].pending_claim = false;
        Ok(Some(actor))
    }
}
