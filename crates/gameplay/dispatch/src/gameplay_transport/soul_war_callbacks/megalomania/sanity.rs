//! Crystal mechanics.lua966–1012: retain the original name lookup and publish
//! the shared defense clock only after the canonical Sanity Item/Quest outcome.
use super::*;
use crate::gameplay_transport::source_names::{SourceNameRole, SourceNamedActor};
use oteryn_gameplay_encounters::bosses::soul_war_cruelty::Cruelty;
use oteryn_gameplay_entities::CompanionSnapshot;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct SanityState {
    pub(super) clock: Cruelty,
    pub(super) pending: Option<[u8; 16]>,
}
#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PreparedSanityUse {
    event: [u8; 16],
    clock: Cruelty,
    boss: Option<(CompanionSnapshot, u32)>,
    affected: Vec<ExactActorRef>,
    missing_type: bool,
}
impl SanityState {
    pub(in crate::gameplay_transport) fn prepare(
        &self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        event: [u8; 16],
        named: Option<&SourceNamedActor>,
    ) -> Result<PreparedSanityUse, ()> {
        runtime.owner_fence().map_err(|_| ())?;
        if self.pending.is_some() || event == [0; 16] || !qualifies_source(runtime, native) {
            return Err(());
        }
        let mut affected = Vec::new();
        let mut missing_type = false;
        let boss = match named {
            Some(named) if named.role == SourceNameRole::Creature => {
                if !source_actor(runtime, native, named.actor)
                    .is_some_and(|key| FORMS.contains(&key.as_str()))
                {
                    return Err(());
                }
                let snapshot = runtime
                    .companion_snapshot_including_dead(named.actor)
                    .map_err(|_| ())?;
                let defense = runtime
                    .effective_creature_defense(&snapshot)
                    .map_err(|_| ())?;
                runtime
                    .validate_soul_war_megalomania_sanity(&snapshot, defense)
                    .map_err(|_| ())?;
                affected.push(named.actor);
                Some((snapshot, defense))
            }
            Some(_) => {
                missing_type = true;
                None
            }
            None => None,
        };
        Ok(PreparedSanityUse {
            event,
            clock: self.clock.clone(),
            boss,
            affected,
            missing_type,
        })
    }
    pub(in crate::gameplay_transport) fn retain(&mut self, prepared: &PreparedSanityUse) {
        self.pending = Some(prepared.event);
    }
    pub(in crate::gameplay_transport) fn completed_clock(
        &self,
        prepared: &PreparedSanityUse,
        committed_second: Option<u64>,
    ) -> Result<Cruelty, ()> {
        if self.pending != Some(prepared.event) || self.clock != prepared.clock {
            return Err(());
        }
        let mut clock = prepared.clock.clone();
        if let Some(second) = committed_second {
            // Sanity sets the timestamp even with no boss, and does not change
            // Cruelty's shared drain DATA (unlike Mortal Essence).
            clock.record_mortal_essence_time(second).ok_or(())?;
        }
        Ok(clock)
    }
    pub(in crate::gameplay_transport) fn publish(&mut self, clock: Cruelty) {
        self.clock = clock;
        self.pending = None;
    }
}
impl PreparedSanityUse {
    pub(in crate::gameplay_transport) fn affected(&self) -> &[ExactActorRef] {
        &self.affected
    }
    pub(in crate::gameplay_transport) fn target(&self) -> Option<(&CompanionSnapshot, u32)> {
        self.boss.as_ref().map(|(boss, defense)| (boss, *defense))
    }
    pub(in crate::gameplay_transport) fn missing_type(&self) -> bool {
        self.missing_type
    }
}
