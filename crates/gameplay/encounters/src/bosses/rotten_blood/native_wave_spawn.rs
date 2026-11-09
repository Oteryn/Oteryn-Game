//! Current physical owner for Ichgahal/Murcion source spawn requests.
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
}

struct SpawnMemo {
    effect: SummonWaveEffect,
    result: Result<ExactActorRef, NativeWaveSpawnError>,
}

impl NativeWaveSpawnOwner {
    pub fn for_boss(runtime: &ChannelRuntimeV1, boss: ExactActorRef) -> Option<Self> {
        let snapshot = runtime.companion_snapshot(boss).ok()?;
        let key = &snapshot.state.policy.definition_key;
        let key = match key.as_str() {
            "oteryn:creature.ichgahal" => "oteryn:creature.ichgahal",
            "oteryn:creature.murcion" => "oteryn:creature.murcion",
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
        let binding = runtime.binding();
        if binding.runtime_scope() != self.scope
            || binding.scope_generation() != self.generation
            || runtime.content_pin().server_artifact_digest() != self.content
            || !current.is_current_for_scope(self.scope, self.generation)
            || !current.accepts_stamp(stamp)
        {
            return Err(NativeWaveSpawnError::StaleOwner);
        }
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
        if ordinal >= 12 {
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
            ) if *offset_tiles == mushroom_offset => *offset_tiles,
            ("oteryn:creature.elder_bloodjaw", SummonPlacement::Close { offset_tiles: 2 }) => 2,
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
            runtime
                .admit_native_encounter_masterless(current, stamp, self.boss, &spec, at, |_, _| {
                    Some(allowed)
                })
                .map_err(NativeWaveSpawnError::Native)
        })();
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
