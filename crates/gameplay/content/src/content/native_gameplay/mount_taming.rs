//! Source-qualified creature taming. Probabilities are Canary values, not proven Global rates.
//! Rules apply only when their exact mount is present in the active cosmetics generation.
use serde::Deserialize;
use std::sync::OnceLock;

pub const PROVENANCE: &str = "mount-taming-v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TamingFailure {
    pub action: u8,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TamingRule {
    pub item_key: String,
    pub creature_key: String,
    pub mount_key: String,
    pub grant_key: String,
    pub chance: u8,
    pub break_item: bool,
    pub failures: Vec<TamingFailure>,
    pub success_text: String,
    pub achievement_key: String,
    #[serde(default)]
    pub required_equipment: Option<String>,
    pub source: String,
}

/// One retained uniform draw selects success or a uniformly selected failure, as Canary's
/// two independent draws do. Zero is success; 1..=N identify authored failure rows.
impl TamingRule {
    pub fn decide(&self, draw: u64) -> u8 {
        let n = self.failures.len().max(1) as u64;
        let bucket = ((u128::from(draw) * u128::from(100 * n)) >> 64) as u64;
        if bucket < u64::from(self.chance) * n {
            0
        } else {
            (1 + (bucket - u64::from(self.chance) * n) % n) as u8
        }
    }
    pub fn effects(&self, outcome: u8) -> Option<(bool, bool, &str)> {
        if outcome == 0 {
            return Some((true, true, &self.success_text));
        }
        let failure = self.failures.get(usize::from(outcome - 1))?;
        Some((
            self.break_item && matches!(failure.action, 2 | 4),
            matches!(failure.action, 1 | 4),
            &failure.text,
        ))
    }
}

pub fn rules() -> &'static [TamingRule] {
    static RULES: OnceLock<Vec<TamingRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let Ok(rules): Result<Vec<TamingRule>, _> = serde_json::from_str(include_str!(
            "../../../../../../content/cosmetics/taming.json"
        )) else {
            return Vec::new();
        };
        let mut pairs = std::collections::BTreeSet::new();
        if rules.len() > 128 {
            return Vec::new();
        }
        for rule in &rules {
            if !pairs.insert((&rule.item_key, &rule.creature_key))
                || !(1..=100).contains(&rule.chance)
                || rule.failures.len() > 3
                || (rule.chance != 100 && rule.failures.is_empty())
                || rule.failures.iter().any(|f| !(1..=4).contains(&f.action))
                || !rule.source.starts_with("canary:")
            {
                return Vec::new();
            }
        }
        rules
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_taming_bear_threshold_and_reference_effects() -> Result<(), &'static str> {
        assert_eq!(rules().len(), 44);
        let bear = rules()
            .iter()
            .find(|r| r.item_key == "oteryn:item.tibia.i5907")
            .ok_or("slingshot rule")?;
        let threshold = u64::try_from((1_u128 << 64).div_ceil(5)).map_err(|_| "threshold")?;
        assert_eq!(bear.decide(0), 0);
        assert_eq!(bear.decide(threshold - 1), 0);
        assert_eq!(bear.decide(threshold), 1);
        assert_eq!(
            bear.effects(0),
            Some((true, true, "You have tamed the war bear."))
        );
        assert_eq!(bear.effects(1), Some((false, true, "The bear ran away.")));
        assert_eq!(
            bear.effects(2),
            Some((true, false, "Oh no! The slingshot broke."))
        );
        assert_eq!(
            bear.effects(3),
            Some((
                false,
                false,
                "The bear is trying to hit you with its claws."
            ))
        );
        assert!(bear.effects(4).is_none());
        Ok(())
    }
}
