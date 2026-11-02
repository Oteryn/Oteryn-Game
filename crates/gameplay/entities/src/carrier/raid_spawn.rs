//! Raid realization and receipts in the existing Channel creature owner.
//! A durable raid claim supplies identities; the current owner still proves
//! its generation, active creature revision and present map/occupancy facts.
use super::*;
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RaidSpawnIdentity {
    pub firing_id: [u8; 16],
    pub wave: u16,
    pub entry: u16,
    pub index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRaidSpawnSpec {
    creature: Box<str>,
    revision: Box<str>,
    content: [u8; 32],
    boss: bool,
}

impl NativeRaidSpawnSpec {
    pub fn qualified(
        creature: &str,
        revision: &str,
        content: [u8; 32],
        boss: bool,
    ) -> Result<Self, CarrierError> {
        if content == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(Self {
            creature: copy_native_summon_key(creature)?,
            revision: copy_native_summon_key(revision)?,
            content,
            boss,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidSpawnOutcome {
    Placed(ExactActorRef),
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidSpawnReceipt {
    pub identity: RaidSpawnIdentity,
    pub outcome: RaidSpawnOutcome,
    pub replayed: bool,
    spec: NativeRaidSpawnSpec,
    position: Option<MovementLocalPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidCreatureStatus {
    pub identity: RaidSpawnIdentity,
    pub outcome: RaidSpawnOutcome,
    pub alive: bool,
    pub boss: bool,
}

#[derive(Debug, Default)]
pub(super) struct RaidSpawnLedger {
    receipts: BTreeMap<RaidSpawnIdentity, RaidSpawnReceipt>,
    // A closing marker is reserved before the first actor admission. Closing
    // an existing run never needs more capacity than that run already owns.
    runs: BTreeMap<[u8; 16], bool>,
    // If an unknown LOST run arrives at the hard limit, reject all future new
    // firing identities until reset; never prevent cleanup of existing runs.
    sealed: bool,
    #[cfg(test)]
    test_limit: Option<usize>,
}

impl RaidSpawnLedger {
    fn limit(&self) -> usize {
        #[cfg(test)]
        if let Some(limit) = self.test_limit {
            return limit;
        }
        CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
    }

    fn size(&self) -> usize {
        self.receipts.len() + self.runs.len()
    }
}

impl ChannelRuntimeV1 {
    fn check_raid_stamp(&self, stamp: RuntimeWorkStamp) -> Result<(), CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    /// Called only after the durable writer commits the exact step. A repeated
    /// identity returns its original result even after death or failed placement.
    /// An old ownership generation is never restored into a new runtime.
    pub fn realize_raid_spawn(
        &mut self,
        stamp: RuntimeWorkStamp,
        identity: RaidSpawnIdentity,
        spec: &NativeRaidSpawnSpec,
        position: Option<MovementLocalPosition>,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<RaidSpawnReceipt, EncounterSpawnError> {
        use EncounterSpawnError as E;
        self.check_raid_stamp(stamp).map_err(E::Native)?;
        if WorldId::decode(&identity.firing_id).is_err() {
            return Err(E::Native(CarrierError::InvalidSpawnDefinition));
        }
        if let Some(previous) = self.raid_spawns.receipts.get(&identity) {
            if &previous.spec != spec {
                return Err(E::Native(CarrierError::InvalidSpawnDefinition));
            }
            let mut replay = previous.clone();
            replay.replayed = true;
            return Ok(replay);
        }
        if self.raid_spawns.runs.get(&identity.firing_id) == Some(&true) {
            return Err(E::Native(CarrierError::InvalidSpawnDefinition));
        }
        // Reuse the existing Channel creature resource envelope for retained
        // raid outcomes (including failed requests); reset drops this ledger.
        let new_run = !self.raid_spawns.runs.contains_key(&identity.firing_id);
        let required = 1 + usize::from(new_run);
        if (new_run && self.raid_spawns.sealed)
            || self.raid_spawns.size().saturating_add(required) > self.raid_spawns.limit()
        {
            return Err(E::Native(CarrierError::CapacityExceeded));
        }
        if spec.content != self.content_pin().server_artifact_digest() {
            return Err(E::Native(CarrierError::ContentPinWorldMismatch));
        }
        let policy = self.companion_policy(&spec.creature).map_err(E::Native)?;
        if policy.definition_key.as_str() != spec.creature.as_ref()
            || policy.definition_revision.as_str() != spec.revision.as_ref()
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        let outcome = if let Some(at) = position {
            match self.admit_native_masterless_at(
                &spec.creature,
                &spec.revision,
                spec.content,
                at,
                map_admits,
            ) {
                Ok(actor) => RaidSpawnOutcome::Placed(actor),
                Err(E::RefusedMap | E::Native(CarrierError::CapacityExceeded)) => {
                    RaidSpawnOutcome::Failed
                }
                Err(error) => return Err(error),
            }
        } else {
            RaidSpawnOutcome::Failed
        };
        let receipt = RaidSpawnReceipt {
            identity,
            outcome,
            replayed: false,
            spec: spec.clone(),
            position,
        };
        self.raid_spawns
            .runs
            .entry(identity.firing_id)
            .or_insert(false);
        self.raid_spawns.receipts.insert(identity, receipt.clone());
        Ok(receipt)
    }

    /// Current liveness joins retained raid identities to the actual actor
    /// carrier; recycled slots never make an old raid creature alive again.
    pub fn raid_creatures(
        &self,
        firing_id: [u8; 16],
    ) -> Result<Vec<RaidCreatureStatus>, CarrierError> {
        self.owner_fence()?;
        let first = RaidSpawnIdentity {
            firing_id,
            wave: 0,
            entry: 0,
            index: 0,
        };
        let last = RaidSpawnIdentity {
            firing_id,
            wave: u16::MAX,
            entry: u16::MAX,
            index: u16::MAX,
        };
        Ok(self.raid_spawns.receipts.range(first..=last).map(|(_, receipt)| {
            RaidCreatureStatus {
                identity: receipt.identity,
                outcome: receipt.outcome,
                alive: matches!(receipt.outcome, RaidSpawnOutcome::Placed(actor) if self.contains_live_creature(actor)),
                boss: receipt.spec.boss,
            }
        }).collect())
    }

    /// Closing a persistent run keeps its actors. Explicit despawn and LOST
    /// cleanup remove them through the same administrative removal as native
    /// encounters, with no death, corpse, loot or reward mint.
    pub fn finish_raid_run(
        &mut self,
        stamp: RuntimeWorkStamp,
        firing_id: [u8; 16],
        cleanup: bool,
    ) -> Result<usize, CarrierError> {
        self.check_raid_stamp(stamp)?;
        if WorldId::decode(&firing_id).is_err() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        // Close before removal: a reservation can defer cleanup but cannot
        // permit another wave to appear after the end decision.
        if let Some(closed) = self.raid_spawns.runs.get_mut(&firing_id) {
            *closed = true;
        } else if self.raid_spawns.size() < self.raid_spawns.limit() {
            self.raid_spawns.runs.insert(firing_id, true);
        } else {
            // Every firing with any local actor already owns a marker, so this
            // is a LOST run with no local effects. Fail closed for new firings
            // without allocating or obstructing cleanup of a tracked firing.
            self.raid_spawns.sealed = true;
            return Ok(0);
        }
        if !cleanup {
            return Ok(0);
        }
        let identities = self.raid_creatures(firing_id)?;
        let mut removed = 0;
        for status in identities {
            let RaidSpawnOutcome::Placed(actor) = status.outcome else {
                continue;
            };
            if !status.alive {
                continue;
            }
            let receipt = self
                .raid_spawns
                .receipts
                .remove(&status.identity)
                .ok_or(CarrierError::InvalidSpawnDefinition)?;
            let result = self.remove_native_masterless_subject(
                actor,
                &receipt.spec.creature,
                receipt.spec.content,
            );
            self.raid_spawns.receipts.insert(status.identity, receipt);
            result?;
            removed += 1;
        }
        Ok(removed)
    }

    /// After the durable end effect is acknowledged, per-creature failures and
    /// dead/removed actor receipts need not be replayed. Keep the closing marker
    /// and every surviving persistent actor, so neither replay nor later LOST
    /// cleanup can create or lose an actor. Never call this after an uncertain ack.
    pub fn acknowledge_raid_run_end(
        &mut self,
        stamp: RuntimeWorkStamp,
        firing_id: [u8; 16],
    ) -> Result<usize, CarrierError> {
        self.check_raid_stamp(stamp)?;
        if WorldId::decode(&firing_id).is_err() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        match self.raid_spawns.runs.get(&firing_id) {
            Some(true) => {}
            None if self.raid_spawns.sealed => return Ok(0),
            _ => return Err(CarrierError::InvalidSpawnDefinition),
        }
        let terminal = self
            .raid_creatures(firing_id)?
            .into_iter()
            .filter(|status| !status.alive)
            .map(|status| status.identity)
            .collect::<Vec<_>>();
        for identity in &terminal {
            self.raid_spawns.receipts.remove(identity);
        }
        Ok(terminal.len())
    }
}

#[cfg(test)]
#[path = "raid_spawn_tests.rs"]
mod tests;
