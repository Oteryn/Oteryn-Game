//! Crystal201189f mechanics.lua:862-911. The exact source USE retains its
//! original name resolution and clock through the fused Item/Quest receipt.
use super::*;
use crate::gameplay_transport::source_names::{SourceNameRole, SourceNamedActor};
use oteryn_gameplay_encounters::bosses::soul_war_cruelty::Cruelty;
use oteryn_gameplay_entities::CompanionSnapshot;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct MawState {
    pub(super) clock: Cruelty,
    pub(super) pending: Option<[u8; 16]>,
}

#[derive(Debug, Clone)]
struct MawBoss {
    snapshot: CompanionSnapshot,
    defense: u32,
    drain: Option<u64>,
}

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PreparedMawUse {
    event: [u8; 16],
    clock: Cruelty,
    boss: Option<MawBoss>,
    affected: Vec<ExactActorRef>,
    missing_type: bool,
}

impl MawState {
    pub(in crate::gameplay_transport) fn prepare(
        &self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        event: [u8; 16],
        named: Option<&SourceNamedActor>,
    ) -> Result<PreparedMawUse, ()> {
        runtime.owner_fence().map_err(|_| ())?;
        if self.pending.is_some() || event == [0; 16] || !qualifies_source(runtime, native) {
            return Err(());
        }
        let mut affected = Vec::new();
        let mut missing_type = false;
        let boss = match named {
            Some(named) if named.role == SourceNameRole::Creature => {
                if source_actor(runtime, native, named.actor).as_deref() != Some(BOSS) {
                    return Err(());
                }
                let snapshot = runtime
                    .companion_snapshot_including_dead(named.actor)
                    .map_err(|_| ())?;
                let defense = runtime
                    .effective_creature_defense(&snapshot)
                    .map_err(|_| ())?;
                let drain = runtime.soul_war_cruelty_drain(&snapshot).map_err(|_| ())?;
                runtime
                    .validate_soul_war_cruelty_maw(&snapshot, defense, drain)
                    .map_err(|_| ())?;
                affected.push(named.actor);
                Some(MawBoss {
                    snapshot,
                    defense,
                    drain,
                })
            }
            Some(_) => {
                // Creature(name) checks Players/NPCs first. A missing Monster
                // type returns false after the timestamp and source burn; it
                // never falls through to a later same-name Monster.
                missing_type = true;
                None
            }
            None => None,
        };
        Ok(PreparedMawUse {
            event,
            clock: self.clock.clone(),
            boss,
            affected,
            missing_type,
        })
    }

    /// Called in the same synchronous turn as successful physical reservation.
    pub(in crate::gameplay_transport) fn retain(&mut self, prepared: &PreparedMawUse) {
        self.pending = Some(prepared.event);
    }

    /// A known receipt supplies the original database second, never retry time.
    /// Refusal releases the reservation without changing this source clock.
    pub(in crate::gameplay_transport) fn completed_clock(
        &self,
        prepared: &PreparedMawUse,
        committed_second: Option<u64>,
    ) -> Result<Cruelty, ()> {
        if self.pending != Some(prepared.event) || self.clock != prepared.clock {
            return Err(());
        }
        let mut clock = prepared.clock.clone();
        if let Some(second) = committed_second {
            if prepared.boss.is_some() {
                clock.mortal_essence_used(second).ok_or(())?;
            } else {
                clock.record_mortal_essence_time(second).ok_or(())?;
            }
        }
        Ok(clock)
    }

    /// Physical installation, source clock and source gate ACK share one turn.
    pub(in crate::gameplay_transport) fn publish(&mut self, clock: Cruelty) {
        self.clock = clock;
        self.pending = None;
    }
}

impl PreparedMawUse {
    pub(in crate::gameplay_transport) fn affected(&self) -> &[ExactActorRef] {
        &self.affected
    }
    pub(in crate::gameplay_transport) fn target(
        &self,
    ) -> Option<(&CompanionSnapshot, u32, Option<u64>)> {
        self.boss
            .as_ref()
            .map(|boss| (&boss.snapshot, boss.defense, boss.drain))
    }
    pub(in crate::gameplay_transport) fn missing_type(&self) -> bool {
        self.missing_type
    }
}
