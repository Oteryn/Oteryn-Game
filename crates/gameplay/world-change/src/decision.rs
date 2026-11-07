//! Owned durable representation of the existing native daily decision.
use crate::daily::{DailyWorldChanges, FURY_LAYERS, NIGHTMARE_LAYERS, OrientalTraderTown};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDecision {
    epoch: u64,
    fury_gates: Option<String>,
    fury_exit: String,
    nightmare_isles: String,
    thawing: bool,
    their_masters_voice: bool,
    oriental_trader: Option<u8>,
    grimvale_full_moon: bool,
    flower_month: bool,
}

impl NativeDecision {
    pub fn capture(day: &DailyWorldChanges) -> Self {
        Self {
            epoch: day.reset_epoch,
            fury_gates: day.fury_gates.map(str::to_owned),
            fury_exit: day.fury_exit.into(),
            nightmare_isles: day.nightmare_isles.into(),
            thawing: day.thawing,
            their_masters_voice: day.their_masters_voice,
            oriental_trader: day.oriental_trader.map(|town| match town {
                OrientalTraderTown::Ankrahmun => 0,
                OrientalTraderTown::Carlin => 1,
                OrientalTraderTown::LibertyBay => 2,
            }),
            grimvale_full_moon: day.grimvale_full_moon,
            flower_month: day.flower_month,
        }
    }
    pub fn restore(&self) -> Result<DailyWorldChanges, crate::rules::RuleError> {
        let invalid = crate::rules::RuleError::NativeBinding;
        if self.epoch == 0
            || self
                .fury_gates
                .as_ref()
                .is_some_and(|key| key != &self.fury_exit)
        {
            return Err(invalid);
        }
        let fury_exit = FURY_LAYERS
            .iter()
            .copied()
            .find(|key| *key == self.fury_exit)
            .ok_or(invalid)?;
        let nightmare_isles = NIGHTMARE_LAYERS
            .iter()
            .copied()
            .find(|key| *key == self.nightmare_isles)
            .ok_or(invalid)?;
        let oriental_trader = match self.oriental_trader {
            None => None,
            Some(0) => Some(OrientalTraderTown::Ankrahmun),
            Some(1) => Some(OrientalTraderTown::Carlin),
            Some(2) => Some(OrientalTraderTown::LibertyBay),
            _ => return Err(invalid),
        };
        Ok(DailyWorldChanges {
            reset_epoch: self.epoch,
            fury_gates: self.fury_gates.as_ref().map(|_| fury_exit),
            fury_exit,
            nightmare_isles,
            thawing: self.thawing,
            their_masters_voice: self.their_masters_voice,
            oriental_trader,
            grimvale_full_moon: self.grimvale_full_moon,
            flower_month: self.flower_month,
        })
    }
}
