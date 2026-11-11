//! Source-qualified creature taming. Probabilities are Canary/Crystal values, not proven Global rates.
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
    /// Item/object target for Crystal's construction-style mount rows. Exactly one of this and
    /// `creature_key` is populated. Donor: `data-global/scripts/actions/mounts/mounts.lua:16,31,48,71`.
    #[serde(default)]
    pub target_item_key: Option<String>,
    /// Complete construction recipe, executed by the existing atomic item-recipe owner.
    /// Empty retains the ordinary selected-item taming contract.
    #[serde(default)]
    pub required_items: Vec<String>,
    #[serde(default)]
    pub creature_key: String,
    pub mount_key: String,
    pub grant_key: String,
    pub chance: u8,
    pub break_item: bool,
    pub failures: Vec<TamingFailure>,
    pub success_text: String,
    #[serde(default)]
    pub achievement_key: Option<String>,
    #[serde(default)]
    pub required_equipment: Option<String>,
    pub source: String,
}

/// One retained uniform draw selects success or a uniformly selected failure, as Canary's
/// two independent draws do. Zero is success; 1..=N identify authored failure rows.
impl TamingRule {
    /// Multi-piece construction must never enter the one-item taming transaction.
    pub fn single_item_use(&self) -> bool {
        self.required_items.len() <= 1
    }

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
            "../../../../../../content/cosmetics/taming-native.json"
        )) else {
            return Vec::new();
        };
        let mut pairs = std::collections::BTreeSet::new();
        if rules.len() > 128 {
            return Vec::new();
        }
        for rule in &rules {
            let target = rule
                .target_item_key
                .as_deref()
                .unwrap_or(&rule.creature_key);
            if !pairs.insert((&rule.item_key, target))
                || (rule.creature_key.is_empty() == rule.target_item_key.is_none())
                || rule.required_items.len() > 8
                || (!rule.required_items.is_empty()
                    && (rule.target_item_key.is_none()
                        || rule.chance != 100
                        || !rule.required_items.contains(&rule.item_key)
                        || rule
                            .required_items
                            .iter()
                            .collect::<std::collections::BTreeSet<_>>()
                            .len()
                            != rule.required_items.len()
                        || rule.required_items.iter().any(|key| {
                            !key.starts_with("oteryn:item.") || key.len() > 128 || !key.is_ascii()
                        })))
                || !(1..=100).contains(&rule.chance)
                || rule.failures.len() > 3
                || (rule.chance != 100 && rule.failures.is_empty())
                || rule.failures.iter().any(|f| !(1..=4).contains(&f.action))
                || !(rule.source.starts_with("canary:") || rule.source.starts_with("crystal:"))
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
        assert_eq!(rules().len(), 51);
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

    /// Crystal `201189f8` `data-global/scripts/actions/mounts/mounts.lua` rows shared with
    /// Oteryn: (item id, CHANCE, BREAK, FAIL_MSG actions). Recorded source data.
    const CRYSTAL_SHARED: &[(u32, u8, bool, &[u8])] = &[
        (5907, 20, true, &[1, 2, 3]),
        (12308, 25, true, &[1, 2, 3]),
        (12260, 40, true, &[1, 3]),
        (12318, 40, true, &[1, 2]),
        (12547, 40, true, &[1]),
        (12548, 40, true, &[1, 2, 3]),
        (16155, 30, true, &[1, 2]),
        (12307, 40, true, &[1, 3]),
        (12546, 40, true, &[1]),
        (12550, 40, true, &[2, 3]),
        (28791, 20, false, &[1]),
        (19136, 40, false, &[1, 3]),
        (31576, 30, false, &[1]),
        (30171, 20, false, &[2, 4]),
        (16153, 30, true, &[1, 2, 3]),
        (30170, 20, false, &[2, 4]),
        (14143, 30, true, &[1, 3]),
        (16154, 30, true, &[1, 2, 3]),
        (14142, 30, true, &[1, 3]),
        (12306, 20, true, &[1, 2]),
        (16251, 100, false, &[]),
        (27605, 30, true, &[2, 4]),
        (21439, 30, false, &[2, 4]),
        (12549, 40, true, &[4]),
        (12509, 40, true, &[1, 2]),
        (20274, 30, true, &[1, 3]),
        (24960, 30, false, &[1, 3]),
        (12519, 40, true, &[1, 3]),
        (12311, 15, true, &[1, 3]),
        (12304, 40, true, &[1]),
        (12320, 40, true, &[1, 2]),
        (21186, 30, true, &[2, 4]),
        (17858, 30, true, &[1, 3]),
        (34258, 50, true, &[1, 2]),
    ];

    fn rule_for(item: u32) -> Vec<&'static TamingRule> {
        let key = format!("oteryn:item.tibia.i{item}");
        rules().iter().filter(|r| r.item_key == key).collect()
    }

    /// S06: every shared Crystal row has the same chance, break flag and failure actions, and
    /// `effects` reproduces Crystal `doFailAction` (RUN removes the creature, BREAK and ALL
    /// lose the item only when BREAK is set, success always consumes the item).
    #[test]
    fn s06_crystal_taming_rows_and_fail_actions() -> Result<(), &'static str> {
        for &(item, chance, brk, actions) in CRYSTAL_SHARED {
            let found = rule_for(item);
            assert_eq!(found.len(), 1, "item {item}");
            let rule = found[0];
            assert_eq!(rule.chance, chance, "item {item}");
            assert_eq!(rule.break_item, brk, "item {item}");
            let ours: Vec<u8> = rule.failures.iter().map(|f| f.action).collect();
            assert_eq!(ours, actions, "item {item}");
            // Crystal: success iff math.random(100) <= CHANCE.
            let edge = u64::try_from((u128::from(chance) << 64).div_ceil(100)).unwrap_or(u64::MAX);
            assert_eq!(rule.decide(0), 0, "item {item}");
            if chance < 100 {
                assert_eq!(rule.decide(edge - 1), 0, "item {item}");
                assert_ne!(rule.decide(edge), 0, "item {item}");
            } else {
                assert_eq!(rule.decide(u64::MAX), 0, "item {item}");
            }
            assert_eq!(rule.effects(0).map(|e| (e.0, e.1)), Some((true, true)));
            for (index, &action) in actions.iter().enumerate() {
                let outcome = u8::try_from(index + 1).map_err(|_| "outcome")?;
                let (lose_item, remove_creature, _) = rule.effects(outcome).ok_or("failure row")?;
                assert_eq!(lose_item, brk && matches!(action, 2 | 4), "item {item}");
                assert_eq!(remove_creature, matches!(action, 1 | 4), "item {item}");
            }
        }
        Ok(())
    }

    fn assert_s06_rule(item: u32, chance: u8, success: &str, failure: Option<&str>) {
        let found = rule_for(item);
        assert_eq!(found.len(), 1, "item {item}");
        let rule = found[0];
        let mount = match item {
            52638 => "oteryn:content.mount.battlefrazzle",
            39548 => "oteryn:content.mount.giant_beaver_mount",
            43901 => "oteryn:content.mount.foxmouse_mount",
            _ => unreachable!("only the three admitted creature rows"),
        };
        assert_eq!(rule.mount_key, mount);
        if chance == 100 {
            assert_eq!(rule.decide(u64::MAX), 0);
        }
        assert_eq!(rule.chance, chance);
        assert_eq!(rule.effects(0).map(|effect| effect.2), Some(success));
        assert_eq!(rule.failures.first().map(|row| row.text.as_str()), failure);
    }

    macro_rules! s06_rule_test {
        ($name:ident, $item:literal, $chance:literal, $success:literal, $failure:expr) => {
            #[test]
            fn $name() {
                assert_s06_rule($item, $chance, $success, $failure);
            }
        };
    }

    s06_rule_test!(
        battlefrazzle_taming_rule,
        52638,
        40,
        "The tamed frazzlemaw hungers for fish and this one is a particular rare treat. You command this one now, take good care of it!",
        Some(
            "The tamed frazzlemaw devours the rare fish but otherwise takes no notice of your advancement."
        )
    );
    s06_rule_test!(
        giant_beaver_taming_rule,
        39548,
        40,
        "You tamed the giant beaver.",
        Some("The giant beaver ran away.")
    );
    s06_rule_test!(
        foxmouse_taming_rule,
        43901,
        100,
        "You have tamed the foxmouse.",
        Some("The foxmouse ran away.")
    );

    fn assert_item_target(item: u32, target: u32, mount: &str) {
        let found = rule_for(item);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].target_item_key.as_deref(),
            Some(format!("oteryn:item.tibia.i{target}").as_str())
        );
        assert_eq!(found[0].mount_key, mount);
    }

    // Crystal 201189f8, data-global/scripts/actions/mounts/mounts.lua:16,31,48,71.
    #[test]
    fn tin_lizzard_item_target_rule() {
        assert_item_target(12305, 12305, "oteryn:content.mount.tin_lizzard");
    }
    #[test]
    fn uniwheel_item_target_rule() {
        assert_item_target(12801, 12801, "oteryn:content.mount.uniwheel");
    }
    #[test]
    fn wind_up_loco_item_target_rule() {
        assert_item_target(37397, 37397, "oteryn:content.mount.wind_up_loco");
    }
    #[test]
    fn glooth_glider_item_target_rule() {
        assert_item_target(21897, 21897, "oteryn:content.mount.glooth_glider");
        let rule = rule_for(21897)[0];
        let expected: Vec<_> = [21897, 21898, 21899, 21901, 21902, 21905, 21906]
            .into_iter()
            .map(|id| format!("oteryn:item.tibia.i{id}"))
            .collect();
        assert_eq!(rule.required_items, expected);
        assert!(
            !rule.single_item_use(),
            "construction cannot fall through to a one-piece grant"
        );
        assert_eq!(
            rules().iter().filter(|rule| rule.single_item_use()).count(),
            50
        );
    }

    /// S06 keeps Global's 15% Wild Horse rule and admits every donor row. Crystal has 51 rows;
    /// no row is excluded (the duplicate Wild Horse behaviour is represented by Global's rule).
    #[test]
    fn s06_missing_crystal_taming_rows_are_admitted_and_global_horse_wins() {
        let horse = rule_for(12802);
        let oats: Vec<_> = horse
            .iter()
            .filter(|r| r.creature_key == "oteryn:creature.wild_horse")
            .collect();
        assert_eq!(oats.len(), 1);
        assert_eq!(oats[0].chance, 15);
        assert!(oats[0].break_item);
        for item in [52638, 39548, 43901] {
            assert_eq!(rule_for(item).len(), 1, "item {item}");
        }
        for item in [12305, 12801, 37397, 21897] {
            let found = rule_for(item);
            assert_eq!(found.len(), 1, "item-only target {item}");
            assert!(found[0].creature_key.is_empty());
            assert!(found[0].target_item_key.is_some());
        }
        assert_eq!(
            rules().len(),
            51,
            "Crystal rows (no exclusions) == Oteryn rules"
        );
    }
}
