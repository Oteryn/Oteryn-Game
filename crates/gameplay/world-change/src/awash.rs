//! Dronk's three pump placements in Crystal 201189f8 data-global/world/world.otbm.

pub const COAL: &str = "oteryn:item.tibia.i12600";
pub const PUMP_RECIPE: &str = "oteryn:world-change.awash.coal";

const PUMPS: [(&str, u16, u16, i8); 3] = [
    ("oteryn:item.tibia.i12666", 32558, 32007, -9),
    ("oteryn:item.tibia.i12667", 32560, 32014, -9),
    ("oteryn:item.tibia.i12666", 32567, 32024, -9),
];

pub fn is_pump_item(key: &str) -> bool {
    PUMPS.iter().any(|pump| pump.0 == key)
}

pub fn is_dronk_pump(key: &str, x: u16, y: u16, floor: i8) -> bool {
    PUMPS.contains(&(key, x, y, floor))
}

/// TibiaWiki Awash spoiler 54770@1149364. The sixth save forces flooding
/// even when the community meets the requirements to keep the tunnels dry.
pub const MAX_DRY_DAYS: usize = 5;

pub fn must_flood(previous_days: [bool; MAX_DRY_DAYS]) -> bool {
    previous_days.into_iter().all(|dry| dry)
}

pub const SCOUT: &str = "oteryn:creature.deepling_scout";
// The retained completion242 Awash recipe supplies the chosen Oteryn count.
// The wiki identifies a daily quota without giving its numeric value.
const SCOUTS_PER_DAY: i64 = 10;
pub const MAX_SCOUT_CREDIT: i64 = 50;

pub fn next_scout_credit(days_open: usize, last_credit: Option<i64>) -> Option<i64> {
    if !(1..MAX_DRY_DAYS).contains(&days_open) {
        return None;
    }
    let minimum = i64::try_from(days_open).ok()? * SCOUTS_PER_DAY;
    last_credit.filter(|credit| (minimum..=MAX_SCOUT_CREDIT).contains(credit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awash_carries_surplus_scout_credit_until_the_next_save() {
        assert_eq!(next_scout_credit(0, None), None);
        assert_eq!(next_scout_credit(1, Some(9)), None);
        assert_eq!(next_scout_credit(1, Some(10)), Some(10));
        assert_eq!(next_scout_credit(2, Some(19)), None);
        assert_eq!(next_scout_credit(2, Some(20)), Some(20));
        assert_eq!(next_scout_credit(4, Some(50)), Some(50));
        assert_eq!(next_scout_credit(5, Some(50)), None);
    }

    #[test]
    fn awash_forces_flood_after_five_consecutive_dry_days() {
        assert!(!must_flood([true, true, true, true, false]));
        assert!(!must_flood([true, true, false, true, true]));
        assert!(must_flood([true; 5]));
    }

    #[test]
    fn awash_admits_only_dronks_three_exact_pumps() {
        let common = "oteryn:item.tibia.i12666";
        let second = "oteryn:item.tibia.i12667";
        assert!(is_dronk_pump(common, 32558, 32007, -9));
        assert!(is_dronk_pump(second, 32560, 32014, -9));
        assert!(is_dronk_pump(common, 32567, 32024, -9));
        assert!(!is_dronk_pump(common, 32560, 32014, -9));
        assert!(!is_dronk_pump(common, 32558, 32007, -8));
        assert!(!is_dronk_pump(common, 32558, 32008, -9));
        assert!(!is_dronk_pump(COAL, 32558, 32007, -9));
    }
}
