//! Owner-defined PARITY_PENDING starting values; official news 8834 supplies the envelope.
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverySpeed {
    qualification: String,
    owner_decision: String,
    minimum: u16,
    maximum: u16,
}
impl DiscoverySpeed {
    pub fn embedded() -> Option<Self> {
        let value: Self = serde_json::from_str(include_str!(
            "../../../../../../content/cyclopedia/discovery-speed.json"
        ))
        .ok()?;
        (value.qualification == "PARITY_PENDING"
            && !value.owner_decision.is_empty()
            && value.minimum > 0
            && value.maximum >= value.minimum)
            .then_some(value)
    }
    pub fn bonus(&self, completed: u32, total: u32, current_complete: bool) -> u16 {
        if !current_complete || completed == 0 || completed > total {
            return 0;
        }
        if total == 1 {
            return self.maximum;
        }
        self.minimum
            + (u64::from(self.maximum - self.minimum) * u64::from(completed - 1)
                / u64::from(total - 1)) as u16
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_speed_bonus_matches_documented_envelope() -> Result<(), &'static str> {
        let rules = DiscoverySpeed::embedded().ok_or("speed content")?;
        // Exercise the envelope with sample totals, independent of the live map's area count.
        for total in [20, 100] {
            assert_eq!(rules.bonus(1, total, true), 100);
            assert_eq!(rules.bonus(total, total, true), 195);
            for completed in 1..=total {
                assert_eq!(rules.bonus(completed, total, false), 0);
                assert!(
                    rules.bonus(completed - 1, total, true) <= rules.bonus(completed, total, true),
                    "bonus decreased at {completed}/{total} completed areas"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn discovery_speed_owner_formula_endpoints_floor_and_single_area() -> Result<(), &'static str> {
        let rules = DiscoverySpeed::embedded().ok_or("speed content")?;
        assert_eq!(rules.bonus(1, 20, true), 100);
        assert_eq!(rules.bonus(20, 20, true), 195);
        assert_eq!(rules.bonus(2, 3, true), 147);
        assert_eq!(rules.bonus(1, 1, true), 195);
        assert_eq!(rules.bonus(0, 20, true), 0);
        assert_eq!(rules.bonus(20, 20, false), 0);
        Ok(())
    }
}
