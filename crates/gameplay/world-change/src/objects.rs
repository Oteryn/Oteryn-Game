//! Shared object transitions. The World owner supplies source membership, custody,
//! current authority and the clock, and commits these changes with its receipt.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStatus {
    Dormant,
    Waiting,
    Ready,
    Claimed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedObjectState {
    revision: u64,
    units: u16,
    lit: bool,
    ready_ms: Option<i64>,
    claimed: bool,
}

impl SharedObjectState {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.ready_ms.is_some_and(|at| at < 0) || (self.claimed && self.ready_ms.is_none()) {
            return Err("invalid shared object");
        }
        Ok(())
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn units(&self) -> u16 {
        self.units
    }
    pub fn lit(&self) -> bool {
        self.lit
    }

    fn next_revision(&self) -> Result<u64, &'static str> {
        self.validate()?;
        self.revision
            .checked_add(1)
            .ok_or("object revision exhausted")
    }

    pub fn status(&self, now_ms: i64) -> Result<ObjectStatus, &'static str> {
        self.validate()?;
        if now_ms < 0 {
            return Err("invalid object clock");
        }
        Ok(if self.claimed {
            ObjectStatus::Claimed
        } else {
            match self.ready_ms {
                None => ObjectStatus::Dormant,
                Some(ready) if now_ms < ready => ObjectStatus::Waiting,
                Some(_) => ObjectStatus::Ready,
            }
        })
    }

    /// A source timer is armed once until an explicit reset. Zero delay admits
    /// immediate objects such as a recovered bag using the same claim path.
    pub fn begin(&mut self, now_ms: i64, delay_ms: i64) -> Result<bool, &'static str> {
        self.validate()?;
        if now_ms < 0 || delay_ms < 0 {
            return Err("invalid object clock");
        }
        let ready = now_ms
            .checked_add(delay_ms)
            .ok_or("object timer exhausted")?;
        if self.ready_ms.is_some() {
            return Ok(false);
        }
        let revision = self.next_revision()?;
        self.ready_ms = Some(ready);
        self.revision = revision;
        Ok(true)
    }

    pub fn claim(&mut self, now_ms: i64) -> Result<bool, &'static str> {
        if self.status(now_ms)? != ObjectStatus::Ready {
            return Ok(false);
        }
        let revision = self.next_revision()?;
        self.claimed = true;
        self.revision = revision;
        Ok(true)
    }

    pub fn add(&mut self, amount: u16, capacity: u16) -> Result<bool, &'static str> {
        self.validate()?;
        if amount == 0 || capacity == 0 || self.units > capacity {
            return Err("invalid object capacity");
        }
        let Some(units) = self.units.checked_add(amount).filter(|n| *n <= capacity) else {
            return Ok(false);
        };
        let revision = self.next_revision()?;
        self.units = units;
        self.revision = revision;
        Ok(true)
    }

    pub fn take(&mut self, amount: u16) -> Result<bool, &'static str> {
        self.validate()?;
        if amount == 0 {
            return Err("invalid object quantity");
        }
        let Some(units) = self.units.checked_sub(amount) else {
            return Ok(false);
        };
        let revision = self.next_revision()?;
        self.units = units;
        self.revision = revision;
        Ok(true)
    }

    pub fn ignite(&mut self) -> Result<bool, &'static str> {
        self.set_lit(true)
    }
    pub fn extinguish(&mut self) -> Result<bool, &'static str> {
        self.set_lit(false)
    }

    fn set_lit(&mut self, lit: bool) -> Result<bool, &'static str> {
        self.validate()?;
        if self.lit == lit {
            return Ok(false);
        }
        let revision = self.next_revision()?;
        self.lit = lit;
        self.revision = revision;
        Ok(true)
    }

    /// Reset preserves a monotone revision, so pre-reset object proofs cannot
    /// become current again when the new object happens to look identical.
    pub fn reset(&mut self) -> Result<(), &'static str> {
        let revision = self.next_revision()?;
        *self = Self {
            revision,
            ..Self::default()
        };
        Ok(())
    }
}
