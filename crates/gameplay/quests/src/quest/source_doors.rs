//! Source quest doors share the existing quest copy and comparison owner.
use super::QuestComparison;

#[derive(Debug)]
pub struct QuestDoorRule {
    pub action: u16,
    pub track: &'static str,
    pub comparison: QuestComparison,
    pub appearances: [u16; 2],
    pub teleport_on_open: bool,
}

impl QuestDoorRule {
    /// Source storage without a durable row reads as -1.
    pub fn allows(&self, value: Option<i64>) -> bool {
        self.comparison.holds(value.unwrap_or(-1), 0)
    }
}

// OTS_HYPOTHESIS_ONLY: Canary04b83b512114bfd888000d6e1433ed8ecaec7c5b,
// data-otservbr-global/scripts/quests/the_annihilator/door.lua:3-14, SHA256
// 03c6833cf9c9e4c335e2d33716b1180159696eb45cf1772d9e09e6036ef50ca1.
// The custom action takes precedence over the ordinary startup quest-door rule.
const RULES: &[QuestDoorRule] = &[QuestDoorRule {
    action: 10102,
    track: "oteryn:quest-progress/quest/u7_24/the_annihilator/reward",
    comparison: QuestComparison::Ne(1),
    appearances: [5113, 5114],
    teleport_on_open: true,
}];

pub fn rule(action: u16) -> Option<&'static QuestDoorRule> {
    RULES.iter().find(|rule| rule.action == action)
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_custom_door_compares_the_value_instead_of_presence() -> Result<(), &'static str> {
        let rule = super::rule(10102).ok_or("source door rule")?;
        for (value, allowed) in [
            (None, true),
            (Some(-1), true),
            (Some(0), true),
            (Some(1), false),
            (Some(2), true),
        ] {
            assert_eq!(rule.allows(value), allowed);
        }
        assert!(super::rule(10089).is_none());
        Ok(())
    }
}
