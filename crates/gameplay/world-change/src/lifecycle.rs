//! Reusable seasonal stages and source object population state. Native effects
//! and all item/value writes remain with their existing fenced owners.
use crate::objects::{ObjectStatus, SharedObjectState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleState {
    pub stage: u8,
    pub generation: u64,
    pub mask: u16,
    pub first_defeat_ms: Option<i64>,
    pub failed_attempts: u32,
    pub objects: BTreeMap<u16, SharedObjectState>,
    pub firing_versions: BTreeMap<u16, u64>,
    #[serde(default)]
    pub last_success_ms: BTreeMap<u16, i64>,
    #[serde(default)]
    pub last_firing: BTreeMap<u16, [u8; 16]>,
    pub successful_spawns: BTreeSet<u16>,
    pub failed_spawns: BTreeSet<u16>,
}

impl LifecycleState {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.stage > 8
            || self.last_success_ms.len() > 128
            || self.last_firing.len() > 128
            || self.last_success_ms.values().any(|at| *at < 0)
            || self.objects.len() > 128
            || self.firing_versions.len() > 128
            || self.successful_spawns.len() > 128
            || self.failed_spawns.len() > 128
            || self.first_defeat_ms.is_some_and(|at| at < 0)
        {
            return Err("invalid lifecycle state");
        }
        for object in self.objects.values() {
            object.validate()?;
        }
        Ok(())
    }
    pub fn next_generation(&mut self) -> Result<(), &'static str> {
        let next = self
            .generation
            .checked_add(1)
            .ok_or("event generation exhausted")?;
        self.generation = next;
        self.mask = 0;
        self.first_defeat_ms = None;
        self.firing_versions.clear();
        self.last_firing.clear();
        self.last_success_ms.clear();
        self.successful_spawns.clear();
        self.failed_spawns.clear();
        Ok(())
    }
    pub fn ready(&self, object: u16, now: i64) -> bool {
        self.objects
            .get(&object)
            .is_some_and(|o| o.status(now) == Ok(ObjectStatus::Ready))
    }
    pub fn begin(&mut self, object: u16, now: i64, delay: i64) -> Result<bool, &'static str> {
        let Some(state) = self.objects.get_mut(&object) else {
            return Err("unknown event object");
        };
        state.begin(now, delay)
    }
    /// Only the admitted object revision may acknowledge a hatch. Replays cannot
    /// restart its timer, and later cycles retain the last successful hatch time.
    pub fn complete_hatch(
        &mut self,
        member: u16,
        version: u64,
        now: i64,
        delay: i64,
    ) -> Result<bool, &'static str> {
        let Some(object) = self.objects.get(&member) else {
            return Err("unknown event object");
        };
        if object.revision() != version || !self.ready(member, now) {
            return Ok(false);
        }
        // Work on a clone so invalid clocks/overflow never partially reset state.
        let mut next = object.clone();
        next.reset()?;
        next.begin(now, delay)?;
        self.objects.insert(member, next);
        self.last_success_ms.insert(member, now);
        self.note_spawn(member, true)?;
        Ok(true)
    }
    /// The return value describes a new physical placement, not an admission request.
    pub fn note_spawn(&mut self, member: u16, placed: bool) -> Result<bool, &'static str> {
        if member >= 128 {
            return Err("invalid event member");
        }
        Ok(if placed {
            self.failed_spawns.remove(&member);
            self.successful_spawns.insert(member)
        } else if self.successful_spawns.contains(&member) {
            false
        } else {
            self.failed_spawns.insert(member)
        })
    }
    /// Five distinct members must be credited within one shared five-minute
    /// window in stages 1 and 3. Mutations 6/7 and cocoon 8 precede final member 5.
    pub fn defeat(&mut self, member: u8, now: i64) -> Result<(bool, bool), &'static str> {
        if now < 0 {
            return Err("invalid defeat clock");
        }
        match (self.stage, member) {
            (1 | 3, 0..=4) => {
                let bit = 1_u16 << member;
                if self.mask & bit != 0 {
                    return Ok((false, false));
                }
                self.first_defeat_ms.get_or_insert(now);
                self.mask |= bit;
                if self.mask == 31 {
                    let next_stage = self.stage + 1;
                    self.next_generation()?;
                    self.stage = next_stage;
                }
                Ok((true, false))
            }
            (2, 6..=7) => {
                let bit = 1_u16 << member;
                if self.mask & bit != 0 {
                    return Ok((false, false));
                }
                self.mask |= bit;
                if self.mask & 192 == 192 {
                    self.next_generation()?;
                    self.stage = 3;
                }
                Ok((true, false))
            }
            (4, 8) => {
                self.next_generation()?;
                self.stage = 5;
                Ok((true, false))
            }
            (5, 5) => Ok((true, true)),
            _ => Ok((false, false)),
        }
    }
}
