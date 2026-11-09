//! Current physical owner for Ichgahal/Murcion/Vemiath source spawn requests.
//! Qualification of the room, source cadence and subsequent AI registration
//! remain independently owned; a request never substitutes for current custody.
use super::summon_wave::{SummonPlacement, SummonWaveEffect};
use oteryn_gameplay_entities::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, NativeEncounterSpawnSpec,
};
use oteryn_gameplay_foundation::foundation::{
    RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeWaveSpawnError {
    InvalidSource,
    StaleOwner,
    StaleSequence,
    ReplayConflict,
    MissingMap,
    InvalidPosition,
    UnownedChild,
    Capacity,
    Removal(oteryn_gameplay_entities::CarrierError),
    Native(oteryn_gameplay_entities::EncounterSpawnError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeWaveSpawnReceipt {
    pub child: ExactActorRef,
    pub newly_created: bool,
}

/// Resolves and checks a current tile under the same exclusive owner turn.
/// `None` withholds placement; `false` remains an actual refusal.
pub trait NativeWaveSpawnMap {
    fn resolve(
        &mut self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        effect: &SummonWaveEffect,
        sequence: u64,
        ordinal: u8,
        stamp: RuntimeWorkStamp,
    ) -> Option<(MovementLocalPosition, bool)>;
}

pub struct NativeWaveSpawnOwner {
    boss: ExactActorRef,
    key: &'static str,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    content: [u8; 32],
    sequence: u64,
    memos: BTreeMap<u8, SpawnMemo>,
    wave_children: Vec<ExactActorRef>,
}

struct SpawnMemo {
    effect: SummonWaveEffect,
    result: Result<ExactActorRef, NativeWaveSpawnError>,
}

impl NativeWaveSpawnOwner {
    #[allow(
        clippy::too_many_arguments,
        reason = "current owner and source occurrence stay explicit"
    )]
    pub fn spawn_current(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        sequence: u64,
        ordinal: u8,
        effect: &SummonWaveEffect,
        map: &mut dyn NativeWaveSpawnMap,
    ) -> Result<NativeWaveSpawnReceipt, NativeWaveSpawnError> {
        self.spawn_bound(runtime, None, stamp, sequence, ordinal, effect, map)
    }

    pub fn retire_wave_child(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
        child: ExactActorRef,
    ) -> Result<bool, NativeWaveSpawnError> {
        self.assert_owner(runtime, None, stamp)?;
        if !self.wave_children.contains(&child) {
            return Err(NativeWaveSpawnError::UnownedChild);
        }
        if !runtime.contains_live_creature(child) {
            return Ok(false);
        }
        runtime
            .remove_current_native_encounter_subject(
                stamp,
                child,
                "oteryn:creature.mushroom",
                self.content,
            )
            .map_err(NativeWaveSpawnError::Removal)?;
        Ok(true)
    }

    pub fn for_boss(runtime: &ChannelRuntimeV1, boss: ExactActorRef) -> Option<Self> {
        let snapshot = runtime.companion_snapshot(boss).ok()?;
        let key = &snapshot.state.policy.definition_key;
        let key = match key.as_str() {
            "oteryn:creature.ichgahal" => "oteryn:creature.ichgahal",
            "oteryn:creature.murcion" => "oteryn:creature.murcion",
            "oteryn:creature.vemiath" => "oteryn:creature.vemiath",
            _ => return None,
        };
        let binding = runtime.binding();
        Some(Self {
            boss,
            key,
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            content: runtime.content_pin().server_artifact_digest(),
            sequence: 0,
            memos: BTreeMap::new(),
            wave_children: Vec::new(),
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "current owner and source occurrence stay explicit"
    )]
    pub fn spawn(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        sequence: u64,
        ordinal: u8,
        effect: &SummonWaveEffect,
        map: &mut dyn NativeWaveSpawnMap,
    ) -> Result<NativeWaveSpawnReceipt, NativeWaveSpawnError> {
        self.spawn_bound(
            runtime,
            Some(current),
            stamp,
            sequence,
            ordinal,
            effect,
            map,
        )
    }

    fn assert_owner(
        &self,
        runtime: &ChannelRuntimeV1,
        current: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
    ) -> Result<(), NativeWaveSpawnError> {
        let binding = runtime.binding();
        if binding.runtime_scope() != self.scope
            || binding.scope_generation() != self.generation
            || runtime.content_pin().server_artifact_digest() != self.content
            || !current
                .or_else(|| runtime.owner_fence().ok())
                .is_some_and(|fence| {
                    fence.is_current_for_scope(self.scope, self.generation)
                        && fence.accepts_stamp(stamp)
                })
        {
            return Err(NativeWaveSpawnError::StaleOwner);
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one implementation serves explicit and installed current custody"
    )]
    fn spawn_bound(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        sequence: u64,
        ordinal: u8,
        effect: &SummonWaveEffect,
        map: &mut dyn NativeWaveSpawnMap,
    ) -> Result<NativeWaveSpawnReceipt, NativeWaveSpawnError> {
        self.assert_owner(runtime, current, stamp)?;
        if !runtime.matches_live_creature_identity(self.boss, self.key.as_bytes()) {
            return Err(NativeWaveSpawnError::InvalidSource);
        }
        if sequence == 0 || sequence < self.sequence {
            return Err(NativeWaveSpawnError::StaleSequence);
        }
        if sequence == self.sequence
            && let Some(memo) = self.memos.get(&ordinal)
        {
            return if memo.effect == *effect {
                memo.result.map(|child| NativeWaveSpawnReceipt {
                    child,
                    newly_created: false,
                })
            } else {
                Err(NativeWaveSpawnError::ReplayConflict)
            };
        }
        // The source maximum per think is five bloodjaws, four minimum
        // mushrooms and three wave mushrooms. Retain only this occurrence.
        let limit = if self.key == "oteryn:creature.vemiath" {
            9
        } else {
            12
        };
        if ordinal >= limit {
            return Err(NativeWaveSpawnError::InvalidSource);
        }
        let SummonWaveEffect::Spawn {
            creature_key,
            around,
            placement,
        } = effect
        else {
            return Err(NativeWaveSpawnError::InvalidSource);
        };
        let mushroom_offset = if self.key == "oteryn:creature.ichgahal" {
            2
        } else {
            1
        };
        let offset = match (*creature_key, placement) {
            (
                "oteryn:creature.mushroom",
                SummonPlacement::Close { offset_tiles } | SummonPlacement::Random { offset_tiles },
            ) if self.key != "oteryn:creature.vemiath" && *offset_tiles == mushroom_offset => {
                *offset_tiles
            }
            ("oteryn:creature.elder_bloodjaw", SummonPlacement::Close { offset_tiles })
                if *offset_tiles
                    == if self.key == "oteryn:creature.vemiath" {
                        1
                    } else {
                        2
                    } =>
            {
                *offset_tiles
            }
            (
                "oteryn:creature.pillar_of_dark_energy",
                SummonPlacement::Close { offset_tiles: 1 },
            ) if self.key == "oteryn:creature.vemiath" => 1,
            _ => return Err(NativeWaveSpawnError::InvalidSource),
        };
        let boss_position = runtime
            .read_actor_position(self.boss)
            .map_err(|_| NativeWaveSpawnError::InvalidSource)?
            .position();
        if around.x != boss_position.x
            || around.y != boss_position.y
            || around.floor != boss_position.floor
        {
            return Err(NativeWaveSpawnError::InvalidPosition);
        }
        let child_policy = runtime
            .companion_policy(creature_key)
            .map_err(|_| NativeWaveSpawnError::InvalidSource)?;
        if child_policy.definition_key != *creature_key {
            return Err(NativeWaveSpawnError::InvalidSource);
        }
        let spec = NativeEncounterSpawnSpec::qualified(
            self.key,
            creature_key,
            &child_policy.definition_revision,
            self.content,
        )
        .map_err(|_| NativeWaveSpawnError::InvalidSource)?;
        if matches!(placement, SummonPlacement::Random { .. }) {
            self.wave_children
                .retain(|child| runtime.contains_live_creature(*child));
            if self.wave_children.len()
                >= oteryn_gameplay_entities::carrier::CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            {
                return Err(NativeWaveSpawnError::Capacity);
            }
            self.wave_children
                .try_reserve(1)
                .map_err(|_| NativeWaveSpawnError::Capacity)?;
        }
        let result = (|| {
            let (at, allowed) = map
                .resolve(runtime, self.boss, effect, sequence, ordinal, stamp)
                .ok_or(NativeWaveSpawnError::MissingMap)?;
            if at.floor != around.floor
                || at.x.abs_diff(around.x) > u32::from(offset)
                || at.y.abs_diff(around.y) > u32::from(offset)
            {
                return Err(NativeWaveSpawnError::InvalidPosition);
            }
            match current {
                Some(fence) => runtime.admit_native_encounter_masterless(
                    fence,
                    stamp,
                    self.boss,
                    &spec,
                    at,
                    |_, _| Some(allowed),
                ),
                None => runtime.admit_current_native_encounter_masterless(
                    stamp,
                    self.boss,
                    &spec,
                    at,
                    |_, _| Some(allowed),
                ),
            }
            .map_err(NativeWaveSpawnError::Native)
        })();
        if let Ok(child) = result
            && matches!(placement, SummonPlacement::Random { .. })
        {
            self.wave_children.push(child);
        }
        if sequence != self.sequence {
            self.memos.clear();
            self.sequence = sequence;
        }
        self.memos.insert(
            ordinal,
            SpawnMemo {
                effect: *effect,
                result,
            },
        );
        result.map(|child| NativeWaveSpawnReceipt {
            child,
            newly_created: true,
        })
    }
}

#[cfg(test)]
#[path = "native_wave_spawn_tests.rs"]
mod tests;
