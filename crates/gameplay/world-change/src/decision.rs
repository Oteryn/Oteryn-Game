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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    awash: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    swamp_epidemic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    steamship_open: bool,
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
            awash: day.awash,
            swamp_epidemic: day.swamp_epidemic,
            steamship_open: day.steamship_open,
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
            awash: self.awash,
            swamp_epidemic: self.swamp_epidemic,
            steamship_open: self.steamship_open,
            stampede: false,
            fire_from_the_earth: false,
            world_counters: false,
            counter_stages: [0; 8],
            counter_generations: [0; 8],
            counter_quest_occurrences: [0; 8],
            counter_progress: [[0; 3]; 8],
            shared_used_sources_mask: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_native_decisions_round_trip_without_redrawing() -> Result<(), String> {
        for epoch in 1..=64 {
            for (month, day) in [(1, 1), (6, 13), (6, 14), (12, 31)] {
                let original = crate::daily::roll_calendar(&[42; 16], epoch, month, day);
                let captured = NativeDecision::capture(&original);
                let bytes = serde_json::to_vec(&captured).map_err(|error| format!("{error:?}"))?;
                let decoded: NativeDecision =
                    serde_json::from_slice(&bytes).map_err(|error| format!("{error:?}"))?;
                let restored = decoded.restore().map_err(|error| format!("{error:?}"))?;
                assert_eq!(NativeDecision::capture(&restored), captured);
            }
        }
        Ok(())
    }

    #[test]
    fn contribution_decisions_round_trip_and_legacy_days_default_to_inactive() -> Result<(), String>
    {
        let mut day = crate::daily::roll_calendar(&[42; 16], 1, 11, 17);
        let legacy = serde_json::to_value(NativeDecision::capture(&day))
            .map_err(|error| error.to_string())?;
        for key in ["awash", "steamship_open", "swamp_epidemic"] {
            assert!(legacy.get(key).is_none());
        }
        let legacy: NativeDecision =
            serde_json::from_value(legacy).map_err(|error| error.to_string())?;
        let legacy = legacy.restore().map_err(|error| format!("{error:?}"))?;
        assert!(!legacy.awash && !legacy.steamship_open && !legacy.swamp_epidemic);

        day.awash = true;
        day.steamship_open = true;
        day.swamp_epidemic = true;
        let encoded = serde_json::to_vec(&NativeDecision::capture(&day))
            .map_err(|error| error.to_string())?;
        let decoded: NativeDecision =
            serde_json::from_slice(&encoded).map_err(|error| error.to_string())?;
        let restored = decoded.restore().map_err(|error| format!("{error:?}"))?;
        assert!(restored.awash && restored.steamship_open && restored.swamp_epidemic);
        Ok(())
    }

    #[test]
    fn persisted_native_decisions_reject_invalid_bindings() -> Result<(), String> {
        let original = crate::daily::roll_calendar(&[42; 16], 1, 6, 13);
        let valid = serde_json::to_value(NativeDecision::capture(&original))
            .map_err(|error| format!("{error:?}"))?;
        for (field, value) in [
            ("epoch", serde_json::json!(0)),
            ("fury_exit", serde_json::json!("unregistered-layer")),
            ("fury_gates", serde_json::json!("unregistered-layer")),
            ("nightmare_isles", serde_json::json!("unregistered-layer")),
            ("oriental_trader", serde_json::json!(3)),
        ] {
            let mut corrupted = valid.clone();
            corrupted[field] = value;
            let decoded: NativeDecision =
                serde_json::from_value(corrupted).map_err(|error| format!("{error:?}"))?;
            assert_eq!(
                decoded.restore().err(),
                Some(crate::rules::RuleError::NativeBinding),
                "accepted corrupt {field}"
            );
        }
        let mut unknown = valid;
        unknown["future_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<NativeDecision>(unknown).is_err());
        Ok(())
    }
}
