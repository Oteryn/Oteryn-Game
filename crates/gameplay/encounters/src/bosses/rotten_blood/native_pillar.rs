//! Installed-owner consumption of Crystal's Pillar lifetime and boss-presence callback.
use super::RoomPosition;
use super::pillar_of_dark_energy_think::{
    BOSS_PRESENCE_RANGE_TILES, PILLAR_KEY, PILLAR_LIFETIME_THINKS, PillarThink, PillarThinkOwner,
};
use oteryn_gameplay_entities::{CarrierError, ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_foundation::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};

pub struct NativePillarThinkOwner {
    actor: ExactActorRef,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    content: [u8; 32],
    sequence: u64,
    rules: PillarThinkOwner,
}

impl NativePillarThinkOwner {
    pub fn for_actor(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        runtime.owner_fence()?;
        if content == [0; 32] || content != runtime.content_pin().server_artifact_digest() {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        own_position(runtime, actor)?;
        let binding = runtime.binding();
        Ok(Self {
            actor,
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            content,
            sequence: 0,
            rules: PillarThinkOwner::default(),
        })
    }

    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }

    /// A committed native think, once per sequence. Refusals preserve the counter.
    /// Removal is administrative, without corpse, XP or boss reward consequences.
    pub fn consume(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        sequence: u64,
    ) -> Result<bool, CarrierError> {
        let stamp = runtime.issue_owner_work()?;
        let binding = runtime.binding();
        if binding.runtime_scope() != self.scope
            || binding.scope_generation() != self.generation
            || runtime.content_pin().server_artifact_digest() != self.content
        {
            return Err(CarrierError::WrongScope);
        }
        let own = own_position(runtime, self.actor)?;
        if sequence <= self.sequence {
            return Ok(false);
        }
        let mut monsters = Vec::new();
        // Source expiry precedes the spectator query: an unavailable neighbour
        // cannot hold an otherwise expired Pillar alive on its tenth callback.
        if self.rules.thinks(1).unwrap_or(0) < PILLAR_LIFETIME_THINKS - 1 {
            for (actor, position, session) in runtime.positioned_melee_census()? {
                if session.is_some() {
                    continue;
                }
                let at = position.position();
                let at = RoomPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                };
                if !own.in_range(at, BOSS_PRESENCE_RANGE_TILES) {
                    continue;
                }
                // Missing native identity withholds the absence proof.
                let snapshot = runtime.companion_snapshot(actor)?;
                if snapshot.health > 0 {
                    monsters.push((snapshot.state.policy.definition_key.clone(), at));
                }
            }
        }
        let neighbours = monsters
            .iter()
            .map(|(key, at)| (key.as_str(), *at))
            .collect::<Vec<_>>();
        let mut next = self.rules.clone();
        let removed = next.think(1, own, &neighbours) != PillarThink::Stay;
        if removed {
            runtime.remove_current_native_encounter_subject(
                stamp,
                self.actor,
                PILLAR_KEY,
                self.content,
            )?;
        }
        self.rules = next;
        self.sequence = sequence;
        Ok(removed)
    }
}

fn own_position(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Result<RoomPosition, CarrierError> {
    let snapshot = runtime.companion_snapshot(actor)?;
    if snapshot.state.policy.definition_key != PILLAR_KEY
        || snapshot.state.master.is_some()
        || runtime.native_summon_master(actor).is_some()
        || snapshot.health == 0
    {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    Ok(RoomPosition {
        x: snapshot.position.x,
        y: snapshot.position.y,
        floor: snapshot.position.floor,
    })
}
